//! The miniquad window (decisions S2, S13, S14): the app's 320×200 RGBA picture uploaded as a
//! texture and drawn as one quad with nearest-neighbour filtering, letterboxed to 4:3 (the
//! picture shown as 320×240) or square pixels; input translated to DOOM key codes and raw
//! mouse motion; the mouse captured only while playing. The loop sleeps until the next tic
//! falls due, runs it and presents the result.

use crate::FsHost;
use crate::keys::doom_key;
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

/// Opens the window and runs the app until it quits. Returns when the window is closed.
pub fn play(app: App, host: FsHost, title: String, p: Presentation) {
    let height = if p.square { 600 } else { 720 };
    let conf = Conf {
        window_title: title,
        window_width: 960,
        window_height: height,
        fullscreen: p.fullscreen,
        window_resizable: true,
        platform: Platform {
            linux_backend: linux_backend(std::env::var_os("DISPLAY").is_some()),
            ..Platform::default()
        },
        ..Conf::default()
    };
    miniquad::start(conf, move || Box::new(Stage::new(app, host, p)));
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    bindings: Bindings,
    rgba: Vec<u8>,
    app: App,
    host: FsHost,
    p: Presentation,
    start: Instant,
    captured: bool,
    motion: (f32, f32),
    reported: bool,
}

impl Stage {
    fn new(app: App, host: FsHost, p: Presentation) -> Self {
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
            app,
            host,
            p,
            start: Instant::now(),
            captured: false,
            motion: (0.0, 0.0),
            reported: false,
        }
    }

    fn now(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    /// Prints where the session went, once.
    fn report(&mut self) {
        if self.reported {
            return;
        }
        self.reported = true;
        let path = self.host.dir.join(self.app.session_name());
        match self.app.save_status() {
            _ if !self.app.recorded() => eprintln!("doom-play: {UNRECORDED_LABEL}"),
            SaveStatus::Saved(tics) => {
                eprintln!("doom-play: saved {} ({tics} tics)", path.display())
            }
            SaveStatus::Failed(e) => eprintln!("doom-play: the session was not saved: {e}"),
            SaveStatus::NotSaved => eprintln!("doom-play: the session was not saved"),
        }
    }

    fn quit(&mut self) {
        self.app.quit(&mut self.host);
        self.report();
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {
        let tic = Duration::from_micros(1_000_000 / TICRATE);
        let now = self.now();
        let wait = match self.app.next_tic_at() {
            Some(at) => Duration::from_micros(at.saturating_sub(now)).min(tic),
            None if self.app.mode() != &Mode::Playing => tic,
            None => Duration::ZERO,
        };
        std::thread::sleep(wait);
        let now = self.now();
        self.app.update(now, &mut self.host);
        let capture = self.app.wants_mouse_capture();
        if capture != self.captured {
            self.captured = capture;
            self.motion = (0.0, 0.0);
            window::set_cursor_grab(capture);
            window::show_mouse(!capture);
        }
        if let Some(n) = self.p.quit_after {
            if self.app.tic() >= n || matches!(self.app.mode(), Mode::Ended(_)) {
                self.quit();
            }
        }
        if self.app.quit_requested() {
            self.report();
            window::order_quit();
        }
    }

    fn draw(&mut self) {
        self.app.picture(self.p.gamma, &mut self.rgba);
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
        if let (false, Some(code)) = (repeat, doom_key(key)) {
            self.app.key(code, true, &mut self.host);
        }
    }

    fn key_up_event(&mut self, key: KeyCode, _mods: KeyMods) {
        if let Some(code) = doom_key(key) {
            self.app.key(code, false, &mut self.host);
        }
    }

    fn raw_mouse_motion(&mut self, dx: f32, dy: f32) {
        if !self.captured {
            return;
        }
        // whole pixels go to the app; the fraction waits for the next motion
        self.motion = (self.motion.0 + dx, self.motion.1 + dy);
        let (ix, iy) = (self.motion.0.trunc(), self.motion.1.trunc());
        self.motion = (self.motion.0 - ix, self.motion.1 - iy);
        self.app.mouse_motion(ix as i32, iy as i32);
    }

    fn mouse_button_down_event(&mut self, button: miniquad::MouseButton, _x: f32, _y: f32) {
        if let Some(b) = button_of(button) {
            self.app.mouse_button(b, true);
        }
    }

    fn mouse_button_up_event(&mut self, button: miniquad::MouseButton, _x: f32, _y: f32) {
        if let Some(b) = button_of(button) {
            self.app.mouse_button(b, false);
        }
    }

    fn window_minimized_event(&mut self) {
        self.app.focus_lost();
    }

    fn quit_requested_event(&mut self) {
        self.quit();
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
}
