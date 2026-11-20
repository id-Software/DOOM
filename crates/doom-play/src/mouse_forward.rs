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
