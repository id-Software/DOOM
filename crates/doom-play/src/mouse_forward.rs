//! doom-play's forwarding of the mouse to the app (moved unchanged out of `input_log.rs` by the
//! owner's ruling of 2026-11-19): whole units of raw motion while the mouse is captured, the
//! fraction kept for the next motion, and the buttons always. Every input and what was passed on
//! is written to the `--input-log` file when it is on ([`InputLog`]).

use crate::input_log::{BUTTONS, InputLog};
use doom_game::app::App;
use doom_game::input::MouseButton;

/// doom-play's forwarding of the mouse to the app, shared by the window and the test: whole
/// units of raw motion while captured (the fraction waits for the next motion), and the
/// buttons always (the app ignores them unless playing).
#[derive(Debug, Default)]
pub(crate) struct Forward {
    captured: bool,
    motion: (f32, f32),
}

impl Forward {
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
