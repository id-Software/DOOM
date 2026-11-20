//! doom-play's forwarding of the mouse to the app (moved unchanged out of `input_log.rs` by the
//! owner's ruling of 2026-11-19): whole units of raw motion while the mouse is captured, the
//! fraction kept for the next motion, and the buttons always. Every input and what was passed on
//! is written to the `--input-log` file when it is on ([`InputLog`]).

use crate::cursor_leave::Leave;
use crate::input_log::{BUTTONS, InputLog};
use doom_game::app::App;
use doom_game::input::MouseButton;

/// What happened since the last pump that makes doom-play ask for the cursor grab again (owner
/// ruling 1 of the M5d Windows session, 2026-11-20).
///
/// By miniquad 0.4.11's source (`src/native/windows.rs`), the grab is one `ClipCursor` call whose
/// result is discarded (`set_cursor_grab`, 124-133; `update_clip_rect`, 327-353, the call at
/// 352), made again only on `WM_SIZE` and `WM_MOVE` (439-442, 522-524), never on `WM_ACTIVATE`
/// or `WM_SETFOCUS` (763-769, 770-797), while the library's `cursor_grabbed` flag stays set; and
/// the window is shown (1222) before the event handler and the window's user data are set (1239,
/// 1242), so the activation at launch reaches only `DefWindowProcW` (392-394). Nothing in the
/// library puts the clip back if Windows or another program clears or replaces it. Asking again
/// costs one `ClipCursor` of the same rectangle, sent through the library's request channel and
/// made at the top of the next loop iteration (1248-1250, `process_request`, 1126).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Again {
    /// A mouse button went down.
    pub(crate) button_down: bool,
    /// The window gained focus (`window_restored_event`).
    pub(crate) focus_gained: bool,
    /// The cursor-left detector reported ([`crate::cursor_leave`]).
    pub(crate) cursor_left: bool,
}

impl Again {
    /// Whether anything happened.
    pub(crate) fn any(self) -> bool {
        self.button_down || self.focus_gained || self.cursor_left
    }

    /// The reasons for the log, in a fixed order, separated by spaces.
    pub(crate) fn names(self) -> String {
        [
            (self.button_down, "button-down"),
            (self.focus_gained, "focus-gained"),
            (self.cursor_left, "cursor-left"),
        ]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(" ")
    }
}

/// The decision, pure and the same on every system: ask for the grab again when asking is
/// enabled (`enabled`: on Windows only, `window::GRAB_AGAIN`), doom-play holds the grab
/// (`captured`, after the pump's capture change) and something in `why` happened.
pub(crate) fn grab_again(enabled: bool, captured: bool, why: Again) -> bool {
    enabled && captured && why.any()
}

/// doom-play's forwarding of the mouse to the app, shared by the window and the test: whole
/// units of raw motion while captured (the fraction waits for the next motion), and the
/// buttons always (the app ignores them unless playing). With asking again enabled
/// ([`Forward::new`]) it also runs the cursor-left detector while captured and decides, at the
/// end of each pump, whether to ask for the grab again ([`Forward::grab_again`]); what reaches
/// the app is the same either way.
#[derive(Debug, Default)]
pub(crate) struct Forward {
    captured: bool,
    motion: (f32, f32),
    /// Asking for the grab again (and the cursor-left detector) is enabled.
    enabled: bool,
    /// What happened since the last pump.
    pending: Again,
    /// The cursor-left detector, active while captured and enabled.
    leave: Leave,
}

impl Forward {
    /// A follower that asks for the grab again when `enabled` (the window passes
    /// `window::GRAB_AGAIN`); [`Forward::default`] never does.
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ..Self::default()
        }
    }

    /// Whether the mouse is captured.
    pub(crate) fn captured(&self) -> bool {
        self.captured
    }

    /// Follows the app's wish for the capture: `Some(new)` when it changed (the caller grabs
    /// or frees the cursor), with the fraction of motion dropped.
    pub(crate) fn sync_capture(
        &mut self,
        app: &App,
        log: &mut InputLog,
        size: impl FnOnce() -> (f32, f32),
    ) -> Option<bool> {
        let capture = app.wants_mouse_capture();
        if capture == self.captured {
            return None;
        }
        self.captured = capture;
        self.motion = (0.0, 0.0);
        let what = if capture { "on" } else { "off" };
        let (w, h) = size();
        if capture && self.enabled {
            self.leave.start(w, h);
        } else {
            self.leave.stop();
        }
        log.line(
            "app",
            app.tic(),
            format_args!("capture {what} window {w}x{h}"),
        );
        Some(capture)
    }

    /// Raw device motion from the window library.
    pub(crate) fn raw_motion(&mut self, app: &mut App, log: &mut InputLog, dx: f32, dy: f32) {
        if !self.captured {
            log.line(
                "raw",
                app.tic(),
                format_args!("motion dx={dx} dy={dy} ignored"),
            );
            return;
        }
        log.line("raw", app.tic(), format_args!("motion dx={dx} dy={dy}"));
        self.leave.motion(dx, dy);
        self.motion = (self.motion.0 + dx, self.motion.1 + dy);
        let (ix, iy) = (self.motion.0.trunc(), self.motion.1.trunc());
        self.motion = (self.motion.0 - ix, self.motion.1 - iy);
        let (ix, iy) = (ix as i32, iy as i32);
        log.line("app", app.tic(), format_args!("mouse-motion {ix} {iy}"));
        app.mouse_motion(ix, iy);
    }

    /// A button change from the window library (`index` 0, 1, 2 for left, middle, right;
    /// `None` for a button the game has no bit for).
    pub(crate) fn button(
        &mut self,
        app: &mut App,
        log: &mut InputLog,
        button: Option<MouseButton>,
        down: bool,
        at: (f32, f32),
    ) {
        self.pending.button_down |= down;
        let index = match button {
            Some(MouseButton::Left) => 0,
            Some(MouseButton::Middle) => 1,
            Some(MouseButton::Right) => 2,
            None => {
                let what = if down { "button-down" } else { "button-up" };
                log.line("raw", app.tic(), format_args!("{what} Unknown"));
                return;
            }
        };
        log.raw_button(app.tic(), index, down, at.0, at.1);
        if let Some(b) = button {
            let what = if down { "down" } else { "up" };
            log.line(
                "app",
                app.tic(),
                format_args!("mouse-button {} {what}", BUTTONS[index]),
            );
            app.mouse_button(b, down);
        }
    }

    /// A cursor event from the window library: logged (never forwarded) and given to the
    /// cursor-left detector.
    pub(crate) fn cursor(&mut self, app: &App, log: &mut InputLog, x: f32, y: f32) {
        log.line("raw", app.tic(), format_args!("cursor x={x:.1} y={y:.1}"));
        self.leave.cursor(x, y);
    }

    /// The window gained focus.
    pub(crate) fn focus_gained(&mut self) {
        self.pending.focus_gained = true;
    }

    /// The end of a pump, after the capture change ([`Forward::sync_capture`]): the cursor-left
    /// check (an `app ... cursor-left` line when it reports), then whether to ask for the grab
    /// again ([`grab_again`]; an `app ... capture again (REASONS)` line when it does). The
    /// caller makes the request.
    pub(crate) fn grab_again(&mut self, app: &App, log: &mut InputLog) -> bool {
        if let Some(left) = self.leave.pump_end() {
            let ((x, y), (dx, dy)) = (left.at, left.net);
            log.line(
                "app",
                app.tic(),
                format_args!(
                    "cursor-left from x={x:.1} y={y:.1} dx={dx} dy={dy} pumps={}",
                    left.pumps
                ),
            );
            self.pending.cursor_left = true;
        }
        let why = std::mem::take(&mut self.pending);
        let again = grab_again(self.enabled, self.captured, why);
        if again {
            log.line(
                "app",
                app.tic(),
                format_args!("capture again ({})", why.names()),
            );
        }
        again
    }
}

/// Review survivors of M5d: the forwarding of the mouse, pinned directly (the window and the
/// input-log test only compare frames and demo bytes). Each test names the mutant it kills.
#[cfg(test)]
mod tests {
    use super::*;
    use doom_game::app::{GameData, SessionConfig};
    use doom_render::GameMode;
    use std::io::Write;
    use std::path::Path;

    #[derive(Clone, Default)]
    struct Shared(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct Quiet;
    impl doom_game::app::Host for Quiet {
        fn timestamp(&mut self) -> doom_game::app::Timestamp {
            doom_game::app::Timestamp::default()
        }
        fn exists(&mut self, _: &str) -> bool {
            false
        }
        fn save(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
            Ok(())
        }
    }

    fn app() -> App {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
        let wad = std::fs::read(root.join("m5a_wads/amp.wad")).expect("map WAD");
        let gfx = std::fs::read(root.join("m5b_gfx.wad")).expect("gfx WAD");
        let data = GameData::load(&[&wad, &gfx], GameMode::Registered, 1, 2).expect("game data");
        App::new(data, SessionConfig::default(), &mut Quiet)
    }

    fn app_lines(text: &Shared) -> Vec<String> {
        let text = String::from_utf8(text.0.borrow().clone()).unwrap();
        text.lines()
            .filter(|l| l.contains("mouse-motion"))
            .map(|l| l.rsplit(' ').take(2).collect::<Vec<_>>().join(" "))
            .collect()
    }

    /// Kills `Forward::captured` returning `false`: after the app asks for the capture, the
    /// follower reports it, and reports the release.
    #[test]
    fn captured_follows_the_apps_wish() {
        let app = app();
        assert!(app.wants_mouse_capture(), "a session starts playing");
        let mut fwd = Forward::default();
        assert!(!fwd.captured());
        let mut log = InputLog::off();
        assert_eq!(fwd.sync_capture(&app, &mut log, || (1.0, 1.0)), Some(true));
        assert!(fwd.captured());
    }

    /// Kills `+` to `-` (y, mouse_forward.rs:60) and `-` to `+` (y, :62) in `raw_motion`, and by
    /// the x components the same swaps there: the whole units go to the app and the fraction
    /// stays. Motion of (0.75, -0.75), then (0.5, -0.5) is 1.25 in x: one unit and a quarter
    /// kept; (0.5, -0.5) again is 0.75 kept: no unit. The sign flips of either operator change
    /// the units passed on.
    #[test]
    fn whole_units_are_passed_on_and_the_fraction_is_kept() {
        let mut app = app();
        let text = Shared::default();
        let mut log = InputLog::to_writer(Box::new(text.clone()));
        let mut fwd = Forward::default();
        fwd.sync_capture(&app, &mut log, || (1.0, 1.0));
        fwd.raw_motion(&mut app, &mut log, 0.75, -0.75);
        fwd.raw_motion(&mut app, &mut log, 0.5, -0.5);
        fwd.raw_motion(&mut app, &mut log, 0.5, -0.5);
        fwd.raw_motion(&mut app, &mut log, 0.25, 0.25);
        // the lines read "y x"; the units (x, y) are (0, 0), (1, -1) leaving (0.25, -0.25), (0, 0) leaving (0.75, -0.75),
        // (1, 0) leaving (0.0, -0.5)
        assert_eq!(
            app_lines(&text),
            ["0 0", "-1 1", "0 0", "0 1"]
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
        );
    }

    /// Motion while the mouse is not captured is ignored and reaches nothing.
    #[test]
    fn motion_is_ignored_while_not_captured() {
        let mut app = app();
        let text = Shared::default();
        let mut log = InputLog::to_writer(Box::new(text.clone()));
        let mut fwd = Forward::default();
        fwd.raw_motion(&mut app, &mut log, 5.0, 5.0);
        assert!(app_lines(&text).is_empty());
        let text = String::from_utf8(text.0.borrow().clone()).unwrap();
        assert!(text.contains("motion dx=5 dy=5 ignored"), "{text}");
    }
}
