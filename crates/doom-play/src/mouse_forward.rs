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

/// Owner ruling 1 of the M5d Windows session (2026-11-20): when doom-play asks for the cursor
/// grab again. The decision and the detector are the same on every system; only the window's
/// `GRAB_AGAIN` constant turns them on (Windows).
#[cfg(test)]
mod grab_again_tests {
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

    impl Shared {
        /// The text of the lines whose `TEXT` starts with `capture again` or `cursor-left`.
        fn again_lines(&self) -> Vec<String> {
            let text = String::from_utf8(self.0.borrow().clone()).unwrap();
            text.lines()
                .filter_map(|l| l.splitn(4, ' ').nth(3))
                .filter(|t| t.starts_with("capture again") || t.starts_with("cursor-left"))
                .map(String::from)
                .collect()
        }

        /// The lines of what reached the app (`mouse-motion` and `mouse-button`).
        fn app_lines(&self) -> Vec<String> {
            let text = String::from_utf8(self.0.borrow().clone()).unwrap();
            text.lines()
                .filter(|l| l.contains("mouse-motion") || l.contains("mouse-button"))
                .map(String::from)
                .collect()
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

    /// An app, a log read back and a follower (`enabled` as given) holding the grab of a
    /// 960x720 window.
    fn captured(enabled: bool) -> (App, Shared, InputLog, Forward) {
        let app = app();
        let text = Shared::default();
        let mut log = InputLog::to_writer(Box::new(text.clone()));
        let mut fwd = Forward::new(enabled);
        assert_eq!(
            fwd.sync_capture(&app, &mut log, || (960.0, 720.0)),
            Some(true)
        );
        (app, text, log, fwd)
    }

    const LEFT: Option<MouseButton> = Some(MouseButton::Left);

    /// The decision's truth table: asked again only when enabled, captured and with a reason.
    #[test]
    fn the_decision_needs_enabled_captured_and_a_reason() {
        for bits in 0..32u8 {
            let [enabled, captured, b, f, c] = [0, 1, 2, 3, 4].map(|k| bits & (1 << k) != 0);
            let why = Again {
                button_down: b,
                focus_gained: f,
                cursor_left: c,
            };
            assert_eq!(why.any(), b || f || c);
            assert_eq!(
                grab_again(enabled, captured, why),
                enabled && captured && (b || f || c),
                "{bits:05b}"
            );
        }
        let all = Again {
            button_down: true,
            focus_gained: true,
            cursor_left: true,
        };
        assert_eq!(all.names(), "button-down focus-gained cursor-left");
        assert_eq!(Again::default().names(), "");
    }

    /// Every button-down while the grab is held asks again, once, at the end of its pump: the
    /// left, the right and a button the game has no bit for; a release alone does not.
    #[test]
    fn a_button_down_asks_again_while_captured() {
        let (mut app, text, mut log, mut fwd) = captured(true);
        assert!(!fwd.grab_again(&app, &mut log), "nothing happened yet");
        for b in [LEFT, Some(MouseButton::Right), None] {
            fwd.button(&mut app, &mut log, b, true, (480.0, 360.0));
            assert!(fwd.grab_again(&app, &mut log), "{b:?}");
            assert!(!fwd.grab_again(&app, &mut log), "once per pump: {b:?}");
            fwd.button(&mut app, &mut log, b, false, (480.0, 360.0));
            assert!(!fwd.grab_again(&app, &mut log), "a release: {b:?}");
        }
        assert_eq!(text.again_lines(), vec!["capture again (button-down)"; 3]);
    }

    /// Focus gained while the grab is held asks again; with a press in the same pump the one
    /// request names both.
    #[test]
    fn focus_gained_asks_again_while_captured() {
        let (mut app, text, mut log, mut fwd) = captured(true);
        fwd.focus_gained();
        assert!(fwd.grab_again(&app, &mut log));
        assert!(!fwd.grab_again(&app, &mut log));
        fwd.focus_gained();
        fwd.button(&mut app, &mut log, LEFT, true, (480.0, 360.0));
        assert!(fwd.grab_again(&app, &mut log));
        assert_eq!(
            text.again_lines(),
            [
                "capture again (focus-gained)",
                "capture again (button-down focus-gained)"
            ]
        );
    }

    /// The cursor-left detector, fed by the follower's cursor and motion, reports the 100%
    /// pattern at the end of the second silent pump, logs it and asks again; the next cursor
    /// event re-arms it.
    #[test]
    fn the_cursor_leaving_asks_again_while_captured() {
        let (mut app, text, mut log, mut fwd) = captured(true);
        fwd.cursor(&app, &mut log, 936.0, 490.0);
        assert!(!fwd.grab_again(&app, &mut log));
        fwd.raw_motion(&mut app, &mut log, 60.0, -3.0);
        assert!(!fwd.grab_again(&app, &mut log), "one silent pump");
        fwd.raw_motion(&mut app, &mut log, 60.0, -3.0);
        assert!(fwd.grab_again(&app, &mut log), "two silent pumps");
        fwd.raw_motion(&mut app, &mut log, 60.0, -3.0);
        assert!(!fwd.grab_again(&app, &mut log), "once until a cursor event");
        fwd.cursor(&app, &mut log, 950.0, 423.0);
        assert!(!fwd.grab_again(&app, &mut log));
        for _ in 0..2 {
            fwd.raw_motion(&mut app, &mut log, 30.0, 0.0);
        }
        assert!(!fwd.grab_again(&app, &mut log), "one silent pump");
        fwd.raw_motion(&mut app, &mut log, 30.0, 0.0);
        assert!(fwd.grab_again(&app, &mut log), "re-armed");
        assert_eq!(
            text.again_lines(),
            [
                "cursor-left from x=936.0 y=490.0 dx=120 dy=-6 pumps=2",
                "capture again (cursor-left)",
                "cursor-left from x=950.0 y=423.0 dx=90 dy=0 pumps=2",
                "capture again (cursor-left)"
            ]
        );
    }

    /// Feeds every reason in one pump and the detector's pattern over the next three.
    fn every_reason(app: &mut App, log: &mut InputLog, fwd: &mut Forward) -> Vec<bool> {
        let mut asked = Vec::new();
        fwd.button(app, log, LEFT, true, (480.0, 360.0));
        fwd.focus_gained();
        fwd.cursor(app, log, 936.0, 490.0);
        asked.push(fwd.grab_again(app, log));
        for _ in 0..3 {
            fwd.raw_motion(app, log, 60.0, -3.0);
            asked.push(fwd.grab_again(app, log));
        }
        asked
    }

    /// Nothing is asked and no cursor-left is reported while the grab is not held: before the
    /// first capture and after the capture is released (the app paused on focus loss).
    #[test]
    fn nothing_is_asked_while_not_captured() {
        let mut app = app();
        let text = Shared::default();
        let mut log = InputLog::to_writer(Box::new(text.clone()));
        let mut fwd = Forward::new(true);
        assert!(!fwd.captured());
        assert!(
            every_reason(&mut app, &mut log, &mut fwd)
                .iter()
                .all(|a| !a)
        );
        assert_eq!(
            fwd.sync_capture(&app, &mut log, || (960.0, 720.0)),
            Some(true)
        );
        app.focus_lost();
        assert_eq!(
            fwd.sync_capture(&app, &mut log, || (960.0, 720.0)),
            Some(false)
        );
        assert!(
            every_reason(&mut app, &mut log, &mut fwd)
                .iter()
                .all(|a| !a)
        );
        assert!(text.again_lines().is_empty(), "{:?}", text.again_lines());
    }

    /// With asking again not enabled (Linux and macOS; [`Forward::default`]) nothing is asked
    /// and no cursor-left is reported, though the grab is held.
    #[test]
    fn nothing_is_asked_when_not_enabled() {
        let (mut app, text, mut log, mut fwd) = captured(false);
        assert!(
            every_reason(&mut app, &mut log, &mut fwd)
                .iter()
                .all(|a| !a)
        );
        assert!(text.again_lines().is_empty(), "{:?}", text.again_lines());
        let mut fwd = Forward::default();
        assert_eq!(
            fwd.sync_capture(&app, &mut log, || (960.0, 720.0)),
            Some(true)
        );
        assert!(
            every_reason(&mut app, &mut log, &mut fwd)
                .iter()
                .all(|a| !a)
        );
        assert!(text.again_lines().is_empty(), "{:?}", text.again_lines());
    }

    /// What reaches the app is the same with asking again enabled or not: only the log's
    /// `capture again` and `cursor-left` lines differ.
    #[test]
    fn asking_again_changes_nothing_the_app_receives() {
        let run = |enabled: bool| {
            let (mut app, text, mut log, mut fwd) = captured(enabled);
            let asked = every_reason(&mut app, &mut log, &mut fwd);
            fwd.button(&mut app, &mut log, LEFT, false, (480.0, 360.0));
            fwd.raw_motion(&mut app, &mut log, -0.5, 0.75);
            (asked, text.app_lines())
        };
        let (on, on_lines) = run(true);
        let (off, off_lines) = run(false);
        assert_eq!(on, [true, false, true, false]);
        assert_eq!(off, [false; 4]);
        assert_eq!(on_lines, off_lines);
        assert_eq!(on_lines.len(), 6, "{on_lines:?}");
    }
}
