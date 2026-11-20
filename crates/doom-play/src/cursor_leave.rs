//! The cursor-left detector (owner ruling 1 of the M5d Windows session, 2026-11-20): an inference,
//! from the window library's events alone, that the cursor has left the window's client area
//! while doom-play holds the cursor grab. It changes nothing the game sees: it only decides when
//! doom-play asks for the grab again on Windows ([`crate::mouse_forward::Forward`]) and writes an
//! `app ... cursor-left` line to the `--input-log` file.
//!
//! **Why it is inferred.** miniquad 0.4.11 gives no event when the cursor leaves the client area:
//! its `WM_MOUSELEAVE` arm is empty and the `TrackMouseEvent` call that would ask for one is
//! commented out (`src/native/windows.rs`, 558-565 and 500-514), and it offers no call that reads
//! the cursor position or the clip rectangle. What it does give: a cursor event (`WM_MOUSEMOVE`,
//! 497-520) only while the cursor is over the client area and moves, and raw device motion
//! (`WM_INPUT`, 526-556) whether the cursor is inside or not.
//!
//! **The rule.** While the grab is held: after a cursor event at `(x, y)`, raw motion is summed
//! per axis. At the end of a pump (one `update` call), the pump counts as *silent* if raw motion
//! arrived in it and no cursor event did. When at least [`SILENT_PUMPS`] silent pumps have passed
//! since the last cursor event and, on some axis, the summed raw motion points at an edge the
//! cursor was more than [`EDGE`] pixels from and is at least that distance in raw units, the
//! detector reports `cursor-left`, once; the next cursor event re-arms it. Before the first cursor
//! event after the grab starts it reports nothing.
//!
//! **The basis, from the owner's input logs of 2026-11-20 (Windows, 100%, 125% and 150%; local
//! only, counts in the M5d record).** With the clip working, a cursor driven at an edge comes to
//! rest at `x = 0`, `x = w - 1`, `y = 0` or `y = h - 1` (`w`×`h` the size on the `capture on`
//! line), the cursor event that takes it there is posted, and further raw motion into the edge
//! posts none: so a last position at an edge explains silence. [`EDGE`] (2 px) also covers
//! `screen_size` falling short of the client area by up to a pixel at 125% and 150%
//! (`docs/validation/M5D.md`, A1). A cursor that moves inside the client area posts at least one
//! cursor event in the pump it moves, but the raw lines of a pump are written before its cursor
//! event, so silence is counted over whole pumps only. Over the five logs read, every count of
//! silent pumps from 1 to 4 and every factor from 0.5 to 3 on the distance gave the same
//! episodes (only the pump they were reported in moved) and none in a log whose clip held; the
//! values chosen (2 pumps, a factor of 1) sit inside that range. The cursor's speed relative to
//! raw motion varied in those logs (about 0.6 to 1.2 pixels per raw unit), which is why the
//! factor is not trusted alone and the silence carries the inference. It is an inference, not a
//! measurement: a cursor resting at an edge whose clip was then cleared, or one that left through
//! an edge it already rested on, is not reported.

/// Whole pumps with raw motion and no cursor event before `cursor-left` can be reported.
pub(crate) const SILENT_PUMPS: u32 = 2;

/// Pixels from an edge within which the last cursor position counts as resting on it.
pub(crate) const EDGE: f32 = 2.0;

/// The detector's state; inactive until [`Leave::start`].
#[derive(Debug, Default)]
pub(crate) struct Leave {
    /// The window size at the start of the grab; `None` while inactive.
    size: Option<(f32, f32)>,
    /// The last cursor position since the grab started.
    last: Option<(f32, f32)>,
    /// Raw motion summed since that cursor event.
    net: (f32, f32),
    /// Whole silent pumps since that cursor event.
    silent: u32,
    /// Raw motion arrived in the current pump.
    moved: bool,
    /// A cursor event arrived in the current pump.
    seen: bool,
    /// Reported since the last cursor event.
    fired: bool,
}

/// A `cursor-left` report: where the cursor was last seen, the raw motion since and the silent
/// pumps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Left {
    /// The last cursor position.
    pub(crate) at: (f32, f32),
    /// The raw motion summed since.
    pub(crate) net: (f32, f32),
    /// The silent pumps since.
    pub(crate) pumps: u32,
}

impl Leave {
    /// The grab starts with the window `width`×`height`: the detector is active and knows no
    /// cursor position yet.
    pub(crate) fn start(&mut self, width: f32, height: f32) {
        *self = Self {
            size: Some((width, height)),
            ..Self::default()
        };
    }

    /// The grab ends: inactive.
    pub(crate) fn stop(&mut self) {
        *self = Self::default();
    }

    /// A cursor event: the position is known again and the detector re-armed.
    pub(crate) fn cursor(&mut self, x: f32, y: f32) {
        if self.size.is_some() {
            self.last = Some((x, y));
            self.net = (0.0, 0.0);
            self.silent = 0;
            self.fired = false;
            self.seen = true;
        }
    }

    /// Raw motion while the grab is held.
    pub(crate) fn motion(&mut self, dx: f32, dy: f32) {
        if self.size.is_some() {
            self.net = (self.net.0 + dx, self.net.1 + dy);
            self.moved = true;
        }
    }

    /// The end of a pump: `Some` when the cursor is inferred to have left the client area.
    pub(crate) fn pump_end(&mut self) -> Option<Left> {
        let (moved, seen) = (self.moved, self.seen);
        self.moved = false;
        self.seen = false;
        let (Some((w, h)), Some(at)) = (self.size, self.last) else {
            return None;
        };
        if moved && !seen {
            self.silent += 1;
        }
        if self.fired || self.silent < SILENT_PUMPS {
            return None;
        }
        if crossed(at.0, w, self.net.0) || crossed(at.1, h, self.net.1) {
            self.fired = true;
            return Some(Left {
                at,
                net: self.net,
                pumps: self.silent,
            });
        }
        None
    }
}

/// Whether raw motion `d` along an axis of `len` pixels, from `pos`, points at an edge more than
/// [`EDGE`] pixels away and covers that distance.
fn crossed(pos: f32, len: f32, d: f32) -> bool {
    let rest = if d > 0.0 {
        len - 1.0 - pos
    } else if d < 0.0 {
        pos
    } else {
        return false;
    };
    rest > EDGE && d.abs() >= rest
}
