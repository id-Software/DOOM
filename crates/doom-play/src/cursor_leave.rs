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

/// Drives a [`Leave`] from the text of an `--input-log` file the way the window drives it: a
/// `capture on window WxH` line starts it and `capture off` stops it, `raw ... cursor` and
/// (not ignored) `raw ... motion` lines feed it, and a change of pump ends the previous pump.
/// Pumps with no line are skipped; with no input in them they cannot change its answer. Returns
/// each report with the pump it came at the end of.
#[cfg(test)]
pub(crate) fn replay(text: &str) -> Vec<(u64, Left)> {
    let mut leave = Leave::default();
    let mut fires = Vec::new();
    let mut pump = None;
    let pair = |s: &str, a: &str, b: &str| -> Option<(f32, f32)> {
        let rest = s.strip_prefix(a)?;
        let (x, y) = rest.split_once(b)?;
        Some((x.parse().ok()?, y.split(' ').next()?.parse().ok()?))
    };
    for line in text.lines() {
        let mut w = line.splitn(4, ' ');
        let (Some(kind), Some(p), Some(_tic), Some(rest)) =
            (w.next(), w.next(), w.next(), w.next())
        else {
            continue;
        };
        let Ok(p) = p.parse::<u64>() else {
            continue;
        };
        if kind == "#" {
            continue;
        }
        if let Some(prev) = pump.filter(|&prev| prev != p) {
            fires.extend(leave.pump_end().map(|l| (prev, l)));
        }
        pump = Some(p);
        match kind {
            "app" if rest.starts_with("capture on window ") => {
                if let Some((w, h)) = pair(rest, "capture on window ", "x") {
                    leave.start(w, h);
                }
            }
            "app" if rest.starts_with("capture off") => leave.stop(),
            "raw" if rest.starts_with("cursor ") => {
                if let Some((x, y)) = pair(rest, "cursor x=", " y=") {
                    leave.cursor(x, y);
                }
            }
            "raw" if rest.starts_with("motion ") && !rest.ends_with(" ignored") => {
                if let Some((dx, dy)) = pair(rest, "motion dx=", " dy=") {
                    leave.motion(dx, dy);
                }
            }
            _ => {}
        }
    }
    if let Some(prev) = pump {
        fires.extend(leave.pump_end().map(|l| (prev, l)));
    }
    fires
}

/// The pumps of left-button presses with no release before the next press or the end of the
/// log (`raw ... button-down Left` and `raw ... button-up Left`).
#[cfg(test)]
pub(crate) fn downs_without_ups(text: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let mut held = None;
    for line in text.lines() {
        let mut w = line.splitn(4, ' ');
        let (Some("raw"), Some(p), Some(_), Some(rest)) = (w.next(), w.next(), w.next(), w.next())
        else {
            continue;
        };
        if rest.starts_with("button-down Left") {
            out.extend(held.replace(p.parse::<u64>().unwrap_or(0)));
        } else if rest.starts_with("button-up Left") {
            held = None;
        }
    }
    out.extend(held);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ends `n` pumps, each with raw motion `(dx, dy)` and no cursor event; the reports.
    fn silent(l: &mut Leave, n: u32, dx: f32, dy: f32) -> Vec<Option<Left>> {
        (0..n)
            .map(|_| {
                l.motion(dx, dy);
                l.pump_end()
            })
            .collect()
    }

    fn started_at(x: f32, y: f32) -> Leave {
        let mut l = Leave::default();
        l.start(960.0, 720.0);
        l.cursor(x, y);
        assert_eq!(l.pump_end(), None);
        l
    }

    /// The 100% pattern of the owner's log: the last cursor event 23 px from the right edge,
    /// then raw motion to the right over pumps with no cursor event. Reported at the end of the
    /// second silent pump ([`SILENT_PUMPS`]), once.
    #[test]
    fn the_cursor_running_off_an_edge_is_reported_once() {
        let mut l = started_at(936.0, 490.0);
        let r = silent(&mut l, 6, 60.0, -3.0);
        assert_eq!(r[0], None, "one silent pump is not enough");
        assert_eq!(
            r[1],
            Some(Left {
                at: (936.0, 490.0),
                net: (120.0, -6.0),
                pumps: 2
            })
        );
        assert!(r[2..].iter().all(Option::is_none), "{r:?}");
        // the left edge and the top and bottom edges likewise
        for (at, d) in [
            ((90.0, 367.0), (-60.0, 0.0)),
            ((400.0, 10.0), (0.0, -8.0)),
            ((400.0, 700.0), (0.0, 12.0)),
        ] {
            let mut l = started_at(at.0, at.1);
            let r = silent(&mut l, 2, d.0, d.1);
            assert!(r[1].is_some(), "{at:?} {d:?}");
        }
    }

    /// A cursor resting at an edge (within [`EDGE`] pixels of `x = 0`, `x = w - 1`, `y = 0` or
    /// `y = h - 1`) with raw motion into that edge posts no cursor event while the clip holds:
    /// not reported, however long it lasts.
    #[test]
    fn a_cursor_resting_at_an_edge_is_not_reported() {
        for (at, d) in [
            ((959.0, 490.0), (60.0, 0.0)),
            ((957.0, 490.0), (60.0, 0.0)),
            ((0.0, 300.0), (-60.0, 0.0)),
            ((2.0, 300.0), (-60.0, 0.0)),
            ((400.0, 719.0), (0.0, 30.0)),
            ((400.0, 0.0), (0.0, -30.0)),
            ((959.0, 719.0), (60.0, 30.0)),
            // one past the reported size: `screen_size` short of the client area (M5D.md, A1)
            ((960.0, 490.0), (60.0, 0.0)),
        ] {
            let mut l = started_at(at.0, at.1);
            let r = silent(&mut l, 50, d.0, d.1);
            assert!(r.iter().all(Option::is_none), "{at:?} {d:?}");
        }
    }

    /// Motion short of the distance to the edge, motion in a pump that also brings a cursor
    /// event (before it or after it), and no motion at all are not reported.
    #[test]
    fn short_motion_or_a_cursor_event_is_not_reported() {
        let mut l = started_at(500.0, 360.0);
        assert!(silent(&mut l, 20, 10.0, 0.0).iter().all(Option::is_none));
        let mut l = started_at(936.0, 490.0);
        for k in 0..20 {
            l.motion(60.0, 0.0);
            l.cursor(936.0, 490.0 - k as f32);
            assert_eq!(l.pump_end(), None);
        }
        let mut l = started_at(936.0, 490.0);
        for _ in 0..20 {
            assert_eq!(l.pump_end(), None);
        }
        // motion after the cursor event of its pump does not make that pump silent: one whole
        // silent pump follows, not two
        let mut l = Leave::default();
        l.start(960.0, 720.0);
        l.cursor(936.0, 490.0);
        l.motion(60.0, 0.0);
        assert_eq!(l.pump_end(), None);
        assert_eq!(silent(&mut l, 1, 60.0, 0.0), [None]);
        assert!(
            silent(&mut l, 1, 60.0, 0.0)[0].is_some(),
            "the second whole one"
        );
    }

    /// Review 3, P27: a pump with no raw motion and no cursor event (idle) is not silent: idle
    /// pumps between two motion pumps do not count, so the report comes at the second pump that
    /// moved, however many idle pumps came between.
    #[test]
    fn idle_pumps_do_not_count_as_silent() {
        let mut l = started_at(936.0, 490.0);
        assert_eq!(silent(&mut l, 1, 60.0, 0.0), [None]);
        for _ in 0..10 {
            assert_eq!(l.pump_end(), None, "an idle pump");
        }
        let r = silent(&mut l, 1, 60.0, 0.0);
        assert_eq!(
            r[0].map(|f| (f.net, f.pumps)),
            Some(((120.0, 0.0), 2)),
            "{r:?}"
        );
    }

    /// The next cursor event re-arms the detector: a second run off an edge is reported again.
    #[test]
    fn a_cursor_event_re_arms_the_detector() {
        let mut l = started_at(936.0, 490.0);
        assert!(silent(&mut l, 3, 60.0, 0.0)[1].is_some());
        l.cursor(950.0, 423.0);
        assert_eq!(l.pump_end(), None);
        let r = silent(&mut l, 3, 30.0, 0.0);
        assert_eq!(r[0], None);
        assert_eq!(r[1].map(|f| (f.at, f.pumps)), Some(((950.0, 423.0), 2)));
        assert_eq!(r[2], None);
    }

    /// Inactive before the grab starts and after it ends; no report before the first cursor
    /// event of a grab.
    #[test]
    fn inactive_without_the_grab_or_a_cursor_position() {
        let mut l = Leave::default();
        l.cursor(936.0, 490.0);
        assert!(silent(&mut l, 5, 60.0, 0.0).iter().all(Option::is_none));
        let mut l = started_at(936.0, 490.0);
        l.stop();
        assert!(silent(&mut l, 5, 60.0, 0.0).iter().all(Option::is_none));
        let mut l = Leave::default();
        l.start(960.0, 720.0);
        assert!(silent(&mut l, 5, 600.0, 0.0).iter().all(Option::is_none));
    }

    /// The replay of a log file finds the 100% pattern at the pump it ends, and stops at
    /// `capture off`.
    #[test]
    fn the_replay_reads_the_log_grammar() {
        let text = "# 1 0 doom-play input log 1: KIND PUMP TIC TEXT\n\
            app 1 0 capture on window 960x720\n\
            raw 47 45 cursor x=936.0 y=490.0\n\
            raw 47 45 button-down Left x=936.0 y=490.0\n\
            raw 48 46 motion dx=60 dy=-3\n\
            app 48 46 mouse-motion 60 -3\n\
            pump 48 46 ran=1 dropped=0 mode=playing capture=on\n\
            raw 49 47 motion dx=53.5 dy=-3\n\
            raw 50 48 motion dx=57 dy=-4\n\
            raw 60 50 cursor x=950.0 y=423.0\n\
            raw 60 50 button-down Left x=950.0 y=423.0\n\
            raw 61 51 motion dx=30 dy=0\n\
            app 62 52 capture off window 960x720\n\
            raw 63 52 motion dx=30 dy=0 ignored\n\
            raw 64 52 motion dx=30 dy=0 ignored\n";
        let fires = replay(text);
        assert_eq!(fires.len(), 1, "{fires:?}");
        assert_eq!(fires[0].0, 49);
        assert_eq!(fires[0].1.net, (113.5, -6.0));
        assert_eq!(downs_without_ups(text), vec![47, 60]);
    }

    /// Runs the detector over the `--input-log` files named in `DOOM_PLAY_INPUT_LOGS` (a path
    /// list), printing per file the reports and the pumps of presses without a release. Local
    /// only: the owner's logs never enter the repository.
    #[test]
    #[ignore = "reads local input logs named in DOOM_PLAY_INPUT_LOGS"]
    fn replay_local_input_logs() {
        let Some(paths) = std::env::var_os("DOOM_PLAY_INPUT_LOGS") else {
            println!("DOOM_PLAY_INPUT_LOGS is not set");
            return;
        };
        for path in std::env::split_paths(&paths) {
            let text = std::fs::read_to_string(&path).expect("an input log");
            let fires = replay(&text);
            let pumps: Vec<u64> = fires.iter().map(|f| f.0).collect();
            println!(
                "{}: cursor-left {} at pumps {pumps:?}; presses without a release at pumps {:?}",
                path.display(),
                fires.len(),
                downs_without_ups(&text)
            );
            for (p, f) in &fires {
                println!("  pump {p}: {f:?}");
            }
        }
    }
}
