//! The miniquad window (decisions S2, S13, S14): the app's 320×200 RGBA picture uploaded as a
//! texture and drawn as one quad with nearest-neighbour filtering, letterboxed to 4:3 (the
//! picture shown as 320×240) or square pixels; input translated to DOOM key codes and raw
//! mouse motion; the mouse captured only while playing. The loop sleeps until the next tic
//! falls due, runs it and presents the result.

use crate::FsHost;
use crate::closing::Play;
use crate::input_log::{InputLog, LoggedHost};
use crate::keys::key_event;
use crate::mouse_forward::Forward;
use crate::session_writer::WriterHost;
use crate::tic_stats::TicStats;
use doom_game::app::{App, Mode, PICTURE_BYTES, SaveStatus, TICRATE, UNRECORDED_LABEL};
use doom_game::input::MouseButton;
use doom_render::{SCREENHEIGHT, SCREENWIDTH};
use miniquad::conf::{Conf, LinuxBackend, Platform};
use miniquad::{
    Bindings, BufferLayout, BufferSource, BufferType, BufferUsage, EventHandler, FilterMode,
    KeyCode, KeyMods, PassAction, Pipeline, PipelineParams, RenderingBackend, ShaderMeta,
    ShaderSource, TextureAccess, TextureFormat, TextureParams, TextureSource, UniformBlockLayout,
    VertexAttribute, VertexFormat, window,
};
use std::time::{Duration, Instant};

/// The presentation choices, fixed for the run.
#[derive(Clone, Copy, Debug)]
pub struct Presentation {
    /// The gamma table, 0–4.
    pub gamma: usize,
    /// Square pixels instead of 4:3.
    pub square: bool,
    /// Borderless fullscreen.
    pub fullscreen: bool,
    /// The measurement aid: save and quit after this many tics or at a session end.
    pub quit_after: Option<u64>,
    /// The measurement aid: print the tic interval summary to stderr on exit.
    pub tic_stats: bool,
}

#[repr(C)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

const VERTEX: &str = "#version 100
attribute vec2 in_pos; attribute vec2 in_uv; varying lowp vec2 uv;
void main() { gl_Position = vec4(in_pos, 0.0, 1.0); uv = in_uv; }";
const FRAGMENT: &str = "#version 100
varying lowp vec2 uv; uniform sampler2D tex;
void main() { gl_FragColor = texture2D(tex, uv); }";

/// The largest rectangle of the picture's aspect (4:3, or 320:200 with square pixels) that
/// fits a `width`×`height` window, centred: x, y, width, height in pixels.
pub fn fit(width: f32, height: f32, square: bool) -> (i32, i32, i32, i32) {
    let aspect = if square { 320.0 / 200.0 } else { 4.0 / 3.0 };
    let (w, h) = if width > height * aspect {
        (height * aspect, height)
    } else {
        (width, width / aspect)
    };
    (
        ((width - w) / 2.0).round() as i32,
        ((height - h) / 2.0).round() as i32,
        w.round() as i32,
        h.round() as i32,
    )
}

/// The Linux display backend: X11 when `DISPLAY` is set (miniquad's default; its fallback to
/// Wayland happens only when the X11 libraries cannot be loaded, not when no X server
/// answers), otherwise Wayland, falling back to X11 if the Wayland libraries are missing.
pub fn linux_backend(display: bool) -> LinuxBackend {
    if display {
        LinuxBackend::X11WithWaylandFallback
    } else {
        LinuxBackend::WaylandWithX11Fallback
    }
}

/// Whether doom-play asks the window library for DPI awareness: on Windows only (milestone M5d,
/// the stuck-fire fix proposed in `docs/validation/M5C.md`, untested until the owner's Windows
/// session). By miniquad 0.4.11's source (`src/native/windows.rs`): the process calls
/// `SetProcessDPIAware` before the window exists (`run`, 1158-1160), so the client rectangle,
/// the `ClipCursor` rectangle built from it (`update_clip_rect`, 327-353) and the cursor are
/// all in physical pixels, which is meant to keep the hidden cursor inside the client area
/// instead of letting it rest on the border, where a button release goes to the frame and is
/// lost (no `SetCapture` in the library). The requested 960×720 (or 960×600) is then taken as
/// physical pixels (`create_window`, 894-964, does not scale it), so at 125% or 150% the window
/// is smaller on screen than before (about 768×576 or 640×480 in Windows' scaled units, where
/// it was 960×720 scaled up); `screen_size` is the physical client size after two truncations
/// through the scale (`update_dimensions`, 1059-1066, with both scales the monitor's DPI / 96,
/// `init_dpi`, 1109 and 1112), so it can be short of the client by less than the scale plus one
/// pixel: worked through in `f32` for client widths up to 8,000, none at 100%, at most one pixel
/// at 125%, 150% and 200%, at most two at 175%, 225%, 250% and 300% (three at 275%); the picture
/// is fitted to it at every draw. Raw mouse motion is scaled by 1 either way (`init_dpi`, 1093-1116), so
/// turning speed does not change. On Linux and macOS nothing is asked: there the flag would
/// change only what the library reports (X11), or render at the backing resolution (Wayland
/// with a scale, macOS Retina), and no fault was seen there.
const HIGH_DPI: bool = cfg!(target_os = "windows");

/// Whether doom-play asks for the cursor grab again while it holds it, and runs the cursor-left
/// detector: on Windows only (owner ruling 1 of the M5d Windows session, 2026-11-20). At 100%
/// the owner's `--input-log` showed the grab asked for at the first pump and the cursor still
/// leaving the window, with presses whose releases never came. miniquad 0.4.11 applies the clip
/// once, discards `ClipCursor`'s result and puts it back only on `WM_SIZE` or `WM_MOVE`, and its
/// grab flag stays set ([`crate::mouse_forward::Again`] has the source lines); what cleared the
/// clip on the owner's machine is Windows' behaviour, not readable in the library's source. So
/// doom-play asks again on every button-down, on focus gained and when the detector reports
/// ([`crate::cursor_leave`]); the decision is [`crate::mouse_forward::grab_again`]. Not on Linux
/// or macOS: there the library's grab is a different mechanism (X11 ungrabs and grabs the pointer
/// on every call, `src/native/linux_x11.rs`, 345-370; macOS ignores a repeated request and freezes
/// the cursor while grabbed, `src/native/macos.rs`, 51-65, so the detector's silence rule would
/// misread it), no fault was seen there and nothing changes, by construction.
const GRAB_AGAIN: bool = cfg!(target_os = "windows");

/// Opens the window and runs the app until it quits, writing the session through `host`'s writer
/// thread and `log` (`--input-log`, or off) through its own.
/// Returns when the window is closed.
pub(crate) fn play(app: App, host: WriterHost, log: InputLog, title: String, p: Presentation) {
    let conf = window_conf(
        title,
        p.square,
        p.fullscreen,
        std::env::var_os("DISPLAY").is_some(),
    );
    let host = LoggedHost { host, log };
    miniquad::start(conf, move || Box::new(Stage::new(app, host, p)));
}

/// The window library's configuration: 960×720 (960×600 with `square`), resizable, DPI awareness
/// as [`HIGH_DPI`] says, the Linux backend from whether `DISPLAY` is set ([`linux_backend`]).
fn window_conf(title: String, square: bool, fullscreen: bool, display: bool) -> Conf {
    Conf {
        window_title: title,
        window_width: 960,
        window_height: if square { 600 } else { 720 },
        fullscreen,
        window_resizable: true,
        high_dpi: HIGH_DPI,
        platform: Platform {
            linux_backend: linux_backend(display),
            ..Platform::default()
        },
        ..Conf::default()
    }
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    bindings: Bindings,
    rgba: Vec<u8>,
    /// The app, its host, the mouse forwarding and the run's end ([`crate::closing`]).
    play: Play,
    p: Presentation,
    start: Instant,
}

impl Stage {
    fn new(app: App, host: LoggedHost<WriterHost>, p: Presentation) -> Self {
        let mut ctx = window::new_rendering_backend();
        let quad = [
            ([-1.0, -1.0], [0.0, 1.0]),
            ([1.0, -1.0], [1.0, 1.0]),
            ([1.0, 1.0], [1.0, 0.0]),
            ([-1.0, 1.0], [0.0, 0.0]),
        ]
        .map(|(pos, uv)| Vertex { pos, uv });
        let vertices = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&quad),
        );
        let indices = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&[0u16, 1, 2, 0, 2, 3]),
        );
        let texture = ctx.new_texture(
            TextureAccess::Static,
            TextureSource::Empty,
            TextureParams {
                format: TextureFormat::RGBA8,
                width: SCREENWIDTH as u32,
                height: SCREENHEIGHT as u32,
                min_filter: FilterMode::Nearest,
                mag_filter: FilterMode::Nearest,
                ..TextureParams::default()
            },
        );
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: FRAGMENT,
                },
                ShaderMeta {
                    images: vec!["tex".to_string()],
                    uniforms: UniformBlockLayout { uniforms: vec![] },
                },
            )
            .unwrap_or_else(|e| fail(&format!("the OpenGL shader was refused: {e:?}")));
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            shader,
            PipelineParams::default(),
        );
        Self {
            ctx,
            pipeline,
            bindings: Bindings {
                vertex_buffers: vec![vertices],
                index_buffer: indices,
                images: vec![texture],
            },
            rgba: vec![0; PICTURE_BYTES],
            play: Play::new(app, host, Forward::new(GRAB_AGAIN), p.tic_stats),
            p,
            start: Instant::now(),
        }
    }

    fn now(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

/// Prints the exit report's lines ([`Play::conclude`]), each after `doom-play: `.
fn print(lines: Vec<String>) {
    for line in lines {
        eprintln!("doom-play: {line}");
    }
}

impl Drop for Stage {
    /// The run's end and its exit report if no update made them after a quit (the window library
    /// stopped without another `update`): the session file is then complete all the same.
    fn drop(&mut self) {
        self.play.quit();
        print(self.play.conclude());
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {
        let tic = Duration::from_micros(1_000_000 / TICRATE);
        let now = self.now();
        let wait = match self.play.app.next_tic_at() {
            Some(at) => Duration::from_micros(at.saturating_sub(now)).min(tic),
            None if self.play.app.mode() != &Mode::Playing => tic,
            None => Duration::ZERO,
        };
        std::thread::sleep(wait);
        let now = self.now();
        let play = &mut self.play;
        let ended_before = matches!(play.app.mode(), Mode::Ended(_));
        let u = play.update(now);
        // no tic runs from here to the next update: the session file is settled into the app
        // when it is not playing (waiting up to closing::END_WAIT at a session's end)
        play.settle_after_update(ended_before);
        let log = &mut play.host.log;
        if let Some(capture) = play.fwd.sync_capture(&play.app, log, window::screen_size) {
            window::set_cursor_grab(capture);
            window::show_mouse(!capture);
        }
        if play.fwd.grab_again(&play.app, log) {
            window::set_cursor_grab(true);
        }
        if let Some(n) = self.p.quit_after {
            if play.app.tic() >= n || matches!(play.app.mode(), Mode::Ended(_)) {
                play.quit();
            }
        }
        if play.app.quit_requested() {
            play.end_run();
            window::order_quit();
        }
        let captured = play.fwd.captured();
        print(play.pump_end(u, captured));
    }

    fn draw(&mut self) {
        self.play.app.picture(self.p.gamma, &mut self.rgba);
        self.ctx.texture_update(self.bindings.images[0], &self.rgba);
        let (w, h) = window::screen_size();
        let (x, y, vw, vh) = fit(w, h, self.p.square);
        self.ctx
            .begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.ctx.apply_viewport(x, y, vw, vh);
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);
        self.ctx.draw(0, 6, 1);
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    fn key_down_event(&mut self, key: KeyCode, _mods: KeyMods, repeat: bool) {
        let play = &mut self.play;
        play.host.log.line(
            "raw",
            play.app.tic(),
            format_args!("key-down {key:?} repeat={repeat}"),
        );
        key_event(&mut play.app, &mut play.host, key, true, repeat);
    }

    fn key_up_event(&mut self, key: KeyCode, _mods: KeyMods) {
        let play = &mut self.play;
        play.host
            .log
            .line("raw", play.app.tic(), format_args!("key-up {key:?}"));
        key_event(&mut play.app, &mut play.host, key, false, false);
    }

    /// Whole units go to the app; the fraction waits for the next motion ([`Forward`]).
    fn raw_mouse_motion(&mut self, dx: f32, dy: f32) {
        let play = &mut self.play;
        play.fwd
            .raw_motion(&mut play.app, &mut play.host.log, dx, dy);
    }

    /// Logged only (`--input-log`), never forwarded; the cursor-left detector reads it.
    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let play = &mut self.play;
        play.fwd.cursor(&play.app, &mut play.host.log, x, y);
    }

    fn mouse_button_down_event(&mut self, button: miniquad::MouseButton, x: f32, y: f32) {
        let b = button_of(button);
        let play = &mut self.play;
        play.fwd
            .button(&mut play.app, &mut play.host.log, b, true, (x, y));
    }

    fn mouse_button_up_event(&mut self, button: miniquad::MouseButton, x: f32, y: f32) {
        let b = button_of(button);
        let play = &mut self.play;
        play.fwd
            .button(&mut play.app, &mut play.host.log, b, false, (x, y));
    }

    /// miniquad 0.4.11 sends this on minimising and, by its source, on focus loss too (X11
    /// FocusOut, Wayland keyboard leave, Windows `WM_ACTIVATE` inactive, macOS resign-key and
    /// window move): the app pauses and releases every held key.
    fn window_minimized_event(&mut self) {
        let play = &mut self.play;
        play.host
            .log
            .line("raw", play.app.tic(), format_args!("focus-lost"));
        play.app.focus_lost();
    }

    /// Logged (`--input-log`), and a reason to ask for the cursor grab again on Windows if it is
    /// held ([`GRAB_AGAIN`]): the app resumes on Esc or Pause, never by itself.
    fn window_restored_event(&mut self) {
        let play = &mut self.play;
        play.host
            .log
            .line("raw", play.app.tic(), format_args!("restored"));
        play.fwd.focus_gained();
    }

    /// Logged only (`--input-log`): the picture is fitted at every draw.
    fn resize_event(&mut self, width: f32, height: f32) {
        let play = &mut self.play;
        play.host.log.line(
            "raw",
            play.app.tic(),
            format_args!("resize {width}x{height}"),
        );
    }

    /// The window is closing: the app's save on quit and the run's end; the exit report follows
    /// the pump line of the update the window library runs next (as the log's last lines did in
    /// b374aaa), or the stage's drop.
    fn quit_requested_event(&mut self) {
        self.play.quit();
    }
}

/// The lines printed on standard error at exit, each after `doom-play: `: with `--tic-stats`
/// the tic interval summary; whenever a device is open (with or without `--tic-stats`;
/// milestone M5d) the device's lines ([`crate::audio::DeviceSink::exit_lines`]: what it
/// reported, the settings asked for, the callbacks' sizes, the queue's depth, underruns and
/// overruns), which replace the counters line `--tic-stats` printed before; the run's
/// layout-dependence line when a session of the run was labelled by the detector of decision
/// T12 (always, with or without `--tic-stats`); then where the session's demo went.
pub(crate) fn exit_lines(app: &App, host: &FsHost, stats: Option<&TicStats>) -> Vec<String> {
    let mut lines = exit_lines_before_session(app, host, stats);
    lines.push(session_line(app, host));
    lines
}

/// [`exit_lines`] without the last one, where the session's demo went ([`session_line`]).
pub(crate) fn exit_lines_before_session(
    app: &App,
    host: &FsHost,
    stats: Option<&TicStats>,
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(s) = stats {
        lines.push(s.summary());
    }
    lines.extend(host.audio.device_exit_lines());
    lines.extend(app.layout_dependent_line());
    lines
}

/// The last of [`exit_lines`]: where the session's demo went, from the app's status.
pub(crate) fn session_line(app: &App, host: &FsHost) -> String {
    let path = host.dir.join(app.session_name());
    match app.save_status() {
        _ if !app.recorded() => UNRECORDED_LABEL.to_string(),
        SaveStatus::Saved(tics) => format!("saved {} ({tics} tics)", path.display()),
        SaveStatus::Failed(e) => format!("the session was not saved: {e}"),
        SaveStatus::NotSaved => "the session was not saved".to_string(),
    }
}

fn button_of(button: miniquad::MouseButton) -> Option<MouseButton> {
    match button {
        miniquad::MouseButton::Left => Some(MouseButton::Left),
        miniquad::MouseButton::Middle => Some(MouseButton::Middle),
        miniquad::MouseButton::Right => Some(MouseButton::Right),
        miniquad::MouseButton::Unknown => None,
    }
}

fn fail(message: &str) -> ! {
    eprintln!("doom-play: {message}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_is_letterboxed_to_its_aspect() {
        assert_eq!(fit(960.0, 720.0, false), (0, 0, 960, 720));
        assert_eq!(fit(960.0, 600.0, true), (0, 0, 960, 600));
        assert_eq!(fit(1920.0, 1080.0, false), (240, 0, 1440, 1080));
        assert_eq!(fit(1920.0, 1080.0, true), (96, 0, 1728, 1080));
        assert_eq!(fit(800.0, 1000.0, false), (0, 200, 800, 600));
        assert_eq!(fit(640.0, 480.0, true), (0, 40, 640, 400));
        assert!(matches!(
            linux_backend(true),
            LinuxBackend::X11WithWaylandFallback
        ));
        assert!(matches!(
            linux_backend(false),
            LinuxBackend::WaylandWithX11Fallback
        ));
    }

    #[test]
    fn dpi_awareness_is_asked_for_on_windows_only() {
        assert_eq!(HIGH_DPI, cfg!(target_os = "windows"));
    }

    /// Owner ruling 1 of the M5d Windows session: the grab is asked for again on Windows only.
    #[test]
    fn the_grab_is_asked_for_again_on_windows_only() {
        assert_eq!(GRAB_AGAIN, cfg!(target_os = "windows"));
    }

    /// Review note N2: the window's configuration itself asks for DPI awareness on Windows only
    /// (not just the constant), with the window size and backend as before.
    #[test]
    fn the_window_configuration_asks_for_dpi_awareness_on_windows_only() {
        for (square, fullscreen, display) in [(false, false, true), (true, true, false)] {
            let conf = window_conf("t".into(), square, fullscreen, display);
            assert_eq!(conf.high_dpi, cfg!(target_os = "windows"));
            assert_eq!(conf.window_title, "t");
            assert_eq!(
                (conf.window_width, conf.window_height),
                (960, if square { 600 } else { 720 })
            );
            assert_eq!(conf.fullscreen, fullscreen);
            assert!(conf.window_resizable);
            let want = linux_backend(display);
            assert_eq!(
                std::mem::discriminant(&conf.platform.linux_backend),
                std::mem::discriminant(&want)
            );
        }
    }
}

/// Mutation-survivor tests of M5d (mut6).
#[cfg(test)]
mod mut6_tests {
    use super::*;

    const CHILD: &str = "DOOM_PLAY_TEST_PRINT_CHILD";
    const NAME: &str = "window::mut6_tests::the_exit_report_goes_to_standard_error_after_the_name";

    /// [`print`]: "the exit report's lines, each after `doom-play: `" on standard error. The
    /// test runs itself as a child process (the harness captures what a test prints, so the
    /// parent could not read it) and reads the child's standard error. No window is opened.
    #[test]
    fn the_exit_report_goes_to_standard_error_after_the_name() {
        if std::env::var_os(CHILD).is_some() {
            print(vec!["first line".to_string(), "second line".to_string()]);
            return;
        }
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([NAME, "--exact", "--nocapture", "--test-threads=1"])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("doom-play: first line\ndoom-play: second line\n"),
            "{err}"
        );
    }
}
