//! `--tic-stats`: a measurement aid. Keeps the monotonic time of each executed tic in memory
//! and summarises the intervals on exit. Nothing on the simulation path reads it.
//!
//! The intervals are those between tics of one stretch of play: a stretch ends when the app's
//! tic clock stops (a pause, a session's end screen) and the next begins at the first tic after
//! the clock was started again. The time between two stretches is the player's, not a stall of
//! the loop, so it is no interval and no wall time here; the summary says how many stretches
//! there were and how long the breaks between them took. (Before 2026-11-21 that time was
//! counted: the owner's 19 s pause showed as the longest interval and as extra wall time.)
//!
//! A limit of that rule: a break runs from the last tic before the clock stopped to the first
//! tic after it started again, so a stall of the loop at either edge of a break is inside the
//! break and is not seen as an interval. If the loop freezes and the player presses Esc during
//! the freeze, the pause is handled before the next update, the late tics never fall due, and
//! the freeze shows only as part of the break's time.

/// Executed-tic times (microseconds on the loop's monotonic clock) and the dropped count.
#[derive(Default)]
pub struct TicStats {
    times: Vec<u64>,
    /// The index in `times` of the first tic of every stretch of play after the first.
    starts: Vec<usize>,
    /// The tic clock stopped since the last recorded tic: the next tic begins a stretch.
    stopped: bool,
    dropped: u64,
}

impl TicStats {
    /// Notes one `App::update` at `now_us`: `ran` tics ran and `dropped` were dropped.
    pub fn record(&mut self, now_us: u64, ran: u64, dropped: u64) {
        if ran > 0 && std::mem::take(&mut self.stopped) && !self.times.is_empty() {
            self.starts.push(self.times.len());
        }
        self.times.extend((0..ran).map(|_| now_us));
        self.dropped += dropped;
    }

    /// Notes that the app's tic clock was not running when an update began (the app was paused
    /// or on an end screen, or was resumed or restarted since the update before): the next tic
    /// recorded begins a new stretch of play.
    pub fn clock_stopped(&mut self) {
        self.stopped = true;
    }

    pub fn summary(&self) -> String {
        if self.starts.is_empty() {
            summarize(&self.times, self.dropped)
        } else {
            summarize_stretches(&self.times, &self.starts, self.dropped)
        }
    }
}

/// The summary text of a run without a break: tics, wall time against tics/35, intervals in
/// microseconds. The signature and the text are those from before breaks were told apart
/// ([`summarize_stretches`] with no stretch after the first).
pub fn summarize(times: &[u64], dropped: u64) -> String {
    summarize_stretches(times, &[], dropped)
}

/// The summary text: tics, wall time against tics/35, intervals in microseconds. `starts` are
/// the indices in `times` of the first tic of every stretch of play after the first (ascending,
/// each above 0 and below `times.len()`); the interval that ends at such a tic is a break (a
/// pause or an end screen), which is left out of the intervals and of the wall time and is
/// reported separately. Without a break the text is as it was before breaks were told apart.
pub fn summarize_stretches(times: &[u64], starts: &[usize], dropped: u64) -> String {
    let n = times.len();
    let (mut d, mut breaks) = (Vec::with_capacity(n), 0u64);
    for (i, w) in times.windows(2).enumerate() {
        if starts.contains(&(i + 1)) {
            breaks += w[1] - w[0];
        } else {
            d.push(w[1] - w[0]);
        }
    }
    if d.is_empty() {
        return format!("tic-stats: executed {n}, dropped {dropped}, too few tics for intervals");
    }
    let wall = d.iter().sum::<u64>() as f64 / 1e6;
    let mean = d.iter().sum::<u64>() as f64 / d.len() as f64;
    let sd = (d.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / d.len() as f64).sqrt();
    let expected = d.len() as f64 / 35.0;
    d.sort_unstable();
    let at = |p: usize| d[((d.len() - 1) * p).div_ceil(100)];
    let ideal = 1e6 / 35.0;
    let off = |us: f64| d.iter().filter(|&&x| (x as f64 - ideal).abs() > us).count();
    let span = match starts.len() {
        0 => "first to last tic".to_string(),
        b => format!(
            "{} stretches of play, each first to last tic; the {b} break{} between them, pauses \
             and end screens, {:.3} s, not counted",
            b + 1,
            if b == 1 { "" } else { "s" },
            breaks as f64 / 1e6
        ),
    };
    format!(
        "tic-stats: executed {n}, dropped {dropped}, wall {wall:.3} s ({span}) against {expected:.3} s expected; \
         intervals us: mean {mean:.1}, sd {sd:.1}, min {}, median {}, p95 {}, p99 {}, max {}; \
         more than 1 ms from 1/35 s: {}, more than 5 ms: {}",
        d[0],
        at(50),
        at(95),
        at(99),
        d[d.len() - 1],
        off(1000.0),
        off(5000.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_arithmetic() {
        // intervals 28000, 29000, 30000 us; ideal 28571.4
        let s = summarize(&[0, 28_000, 57_000, 87_000], 2);
        for want in [
            "executed 4, dropped 2",
            "wall 0.087 s",
            "against 0.086 s",
            "mean 29000.0, sd 816.5, min 28000, median 29000, p95 30000, p99 30000, max 30000",
            "more than 1 ms from 1/35 s: 1, more than 5 ms: 0",
        ] {
            assert!(s.contains(want), "{want} not in {s}");
        }
        assert!(summarize(&[5], 0).contains("too few"));
        let mut t = TicStats::default();
        t.record(10, 2, 1);
        t.record(20, 0, 3);
        assert_eq!((t.times.clone(), t.dropped), (vec![10, 10], 4));
    }

    /// A pause of 19 s and an end screen of 5 s between three stretches of play are no
    /// intervals: the same tics without the breaks give the same statistics, and the line says
    /// how many breaks there were and how long they took.
    #[test]
    fn a_pause_and_an_end_screen_are_not_intervals() {
        // stretch 1: tics at 0, 28000, 57000; a 19 s pause; stretch 2: two tics 30000 apart; a
        // 5 s end screen; stretch 3: two tics 28000 apart
        let times = [
            0, 28_000, 57_000, 19_057_000, 19_087_000, 24_087_000, 24_115_000,
        ];
        let s = summarize_stretches(&times, &[3, 5], 0);
        for want in [
            "executed 7, dropped 0",
            "wall 0.115 s (3 stretches of play, each first to last tic; the 2 breaks between \
             them, pauses and end screens, 24.000 s, not counted)",
            "against 0.114 s expected",
            "mean 28750.0, sd 829.2, min 28000, median 29000, p95 30000, p99 30000, max 30000",
            "more than 1 ms from 1/35 s: 1, more than 5 ms: 0",
        ] {
            assert!(s.contains(want), "{want} not in {s}");
        }
        // counted as before, the breaks are the two longest intervals and 24 s of wall time
        let before = summarize_stretches(&times, &[], 0);
        assert!(
            before.contains("wall 24.115 s (first to last tic)"),
            "{before}"
        );
        assert!(before.contains("max 19000000"), "{before}");
        // one break: singular; a stretch of one tic gives no interval
        let one = summarize_stretches(&[0, 28_000, 9_000_000], &[2], 0);
        assert!(
            one.contains(
                "wall 0.028 s (2 stretches of play, each first to last tic; the 1 break between \
                 them, pauses and end screens, 8.972 s, not counted) against 0.029 s expected"
            ),
            "{one}"
        );
        assert!(summarize_stretches(&[0, 9_000_000], &[1], 0).contains("too few"));
    }

    /// A stretch begins at the first tic recorded after the clock stopped, however often the
    /// stop was noted and whether or not updates without a tic came between; a stop before the
    /// first tic of the run begins nothing, and a stop with no tic after it ends nothing.
    #[test]
    fn a_stretch_begins_at_the_first_tic_after_the_clock_stopped() {
        let mut t = TicStats::default();
        t.clock_stopped(); // the launch: the clock starts at the first update
        t.record(0, 0, 0);
        t.record(10, 1, 0);
        t.record(20, 2, 0);
        assert!(t.starts.is_empty());
        t.clock_stopped();
        t.record(30, 0, 0);
        t.clock_stopped();
        t.record(40, 0, 0); // the update that starts the clock again runs no tic
        t.record(50, 1, 0);
        t.record(60, 1, 0);
        assert_eq!(
            (t.times.clone(), t.starts.clone()),
            (vec![10, 20, 20, 50, 60], vec![3])
        );
        t.clock_stopped();
        t.record(70, 0, 0);
        assert_eq!(t.starts, vec![3], "no tic after the last stop");
        assert!(
            t.summary().contains("2 stretches of play"),
            "{}",
            t.summary()
        );
    }
}
