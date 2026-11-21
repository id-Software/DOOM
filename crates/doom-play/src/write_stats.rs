//! What the two background writers did (M5d, the owner's rulings of 2026-11-20): every file
//! operation of the session writer ([`crate::session_writer`]) and of the input log's writer
//! ([`crate::input_log`]) is counted and timed on the writer's own thread, and the exit lines
//! below report it, so that a retest on a slow disk measures which writer took how long instead
//! of inferring it from the tic statistics. Diagnostics only: nothing here reaches the game, and
//! the times come from the monotonic clock (`Instant`) of the writer threads, never of the loop.

use std::time::{Duration, Instant};

/// A write slower than this is listed with `--tic-stats`.
pub(crate) const SLOW: Duration = Duration::from_millis(20);

/// At most this many slow writes are listed (the rest are counted).
pub(crate) const SLOW_LISTED: usize = 40;

/// What a timed file operation was.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    /// A session file's claim (a flushed temporary file hard-linked to the name).
    Claim,
    /// One open of the session file and one five-byte write per tic waiting.
    Append,
    /// A whole rewrite flushed to the disk before its rename (the app's own saves).
    SyncedRewrite,
    /// A whole rewrite not flushed (the writer's catch-up and page-boundary rewrites).
    Rewrite,
    /// One pump's lines of the input log written and flushed.
    Log,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Claim => "claim",
            Self::Append => "append",
            Self::SyncedRewrite => "synced rewrite",
            Self::Rewrite => "unsynced rewrite",
            Self::Log => "input log",
        }
    }
}

/// One timed operation: its kind, the tic it wrote (for the session file the tics the file holds
/// after it; for the log the app's tic at the pump), how long it took and when it began.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Timed {
    pub(crate) kind: Kind,
    pub(crate) tic: u64,
    pub(crate) took: Duration,
    pub(crate) began: Instant,
}

/// The timing of one writer's operations.
#[derive(Clone, Debug, Default)]
pub(crate) struct WriteTimes {
    /// Operations made (failed ones included).
    pub(crate) count: u64,
    /// Operations that failed.
    pub(crate) failed: u64,
    /// The longest operation. The tie rule: of operations that took exactly as long, the one
    /// recorded first stays the longest ([`WriteTimes::record`] replaces it only for a strictly
    /// longer one), so the exit line names the earliest of them. The test
    /// `of_two_equally_long_writes_the_first_is_the_longest` pins this.
    pub(crate) longest: Option<Timed>,
    /// The operations slower than [`SLOW`], in order, at most [`SLOW_LISTED`].
    pub(crate) slow: Vec<Timed>,
    /// Slow operations beyond [`SLOW_LISTED`], counted only.
    pub(crate) slow_omitted: u64,
}

impl WriteTimes {
    /// Counts one operation.
    pub(crate) fn record(&mut self, t: Timed, ok: bool) {
        self.count += 1;
        self.failed += u64::from(!ok);
        // The tie rule: strictly longer, so of two operations that took exactly as long the
        // first recorded is kept as the longest (pinned by the test
        // `of_two_equally_long_writes_the_first_is_the_longest`).
        if self.longest.is_none_or(|l| t.took > l.took) {
            self.longest = Some(t);
        }
        if t.took > SLOW {
            if self.slow.len() < SLOW_LISTED {
                self.slow.push(t);
            } else {
                self.slow_omitted += 1;
            }
        }
    }
}

/// The session writer's operations by kind, and the most tics that were ever waiting for it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SessionCounts {
    pub(crate) claims: u64,
    /// Append operations (one open each), failed and declined ones included.
    pub(crate) appends: u64,
    /// Tics written by the append operations that completed (one five-byte write each).
    pub(crate) appended_tics: u64,
    /// Append operations that wrote nothing because the file was not the length the writer's
    /// copy expected (each repaired by a whole rewrite).
    pub(crate) declined_appends: u64,
    pub(crate) synced_rewrites: u64,
    pub(crate) rewrites: u64,
    /// The most tics the app had handed over that the file did not hold yet, at a hand-over.
    pub(crate) most_waiting: u64,
}

/// Milliseconds with one decimal, truncated (integer arithmetic).
fn ms(d: Duration) -> String {
    let us = d.as_micros();
    format!("{}.{}", us / 1_000, us % 1_000 / 100)
}

fn longest(t: Option<&Timed>) -> String {
    match t {
        Some(t) => format!(
            "longest {} ms ({} at tic {})",
            ms(t.took),
            t.kind.name(),
            t.tic
        ),
        None => "longest none".to_string(),
    }
}

/// The session writer's exit line: the operations by kind ("appends A of T tics": A append
/// operations, failed and declined ones included, and the T tics written by those that
/// completed; the declined ones are named only when there were any), the failed operations, the
/// longest and the most tics waiting.
pub(crate) fn session_line(c: &SessionCounts, t: &WriteTimes) -> String {
    let declined = match c.declined_appends {
        0 => String::new(),
        n => format!(", {n} appends declined"),
    };
    format!(
        "session file writes: {} (claims {}, appends {} of {} tics{declined}, whole rewrites {} \
         synced and {} unsynced), {} failed; {}; at most {} tics waiting",
        t.count,
        c.claims,
        c.appends,
        c.appended_tics,
        c.synced_rewrites,
        c.rewrites,
        t.failed,
        longest(t.longest.as_ref()),
        c.most_waiting
    )
}

/// The input log writer's exit line.
pub(crate) fn log_line(t: &WriteTimes) -> String {
    format!(
        "input log writes: {}, {} failed; {}",
        t.count,
        t.failed,
        longest(t.longest.as_ref())
    )
}

/// The `--tic-stats` line listing the writes of both writers slower than [`SLOW`], in the order
/// they began, at most [`SLOW_LISTED`].
pub(crate) fn slow_line(writers: &[&WriteTimes]) -> String {
    let mut all: Vec<&Timed> = writers.iter().flat_map(|w| &w.slow).collect();
    all.sort_by_key(|t| t.began);
    let mut omitted: u64 = writers.iter().map(|w| w.slow_omitted).sum();
    if all.len() > SLOW_LISTED {
        omitted += (all.len() - SLOW_LISTED) as u64;
        all.truncate(SLOW_LISTED);
    }
    if all.is_empty() && omitted == 0 {
        return format!("writes over {} ms: none", SLOW.as_millis());
    }
    let mut text: Vec<String> = all
        .iter()
        .map(|t| format!("{} at tic {} {} ms", t.kind.name(), t.tic, ms(t.took)))
        .collect();
    if omitted > 0 {
        text.push(format!("{omitted} more not listed"));
    }
    format!("writes over {} ms: {}", SLOW.as_millis(), text.join("; "))
}

/// The writers' exit lines: the session writer's line when it made any operation, the input log
/// writer's line when the log ran on its thread, and with `--tic-stats` the slow writes of both.
pub(crate) fn exit_lines(
    session: Option<(&SessionCounts, &WriteTimes)>,
    log: Option<&WriteTimes>,
    tic_stats: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut timed = Vec::new();
    if let Some((c, t)) = session.filter(|(_, t)| t.count > 0) {
        lines.push(session_line(c, t));
        timed.push(t);
    }
    if let Some(t) = log {
        lines.push(log_line(t));
        timed.push(t);
    }
    if tic_stats && !timed.is_empty() {
        lines.push(slow_line(&timed));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timed(kind: Kind, tic: u64, us: u64, began: Instant) -> Timed {
        Timed {
            kind,
            tic,
            took: Duration::from_micros(us),
            began,
        }
    }

    /// The exit lines' wording and arithmetic: counts by kind, the longest with its kind and tic,
    /// milliseconds truncated to a tenth, the slow list in the order the writes began, capped.
    #[test]
    fn the_exit_lines_give_counts_the_longest_and_the_slow_writes() {
        let t0 = Instant::now();
        let mut s = WriteTimes::default();
        s.record(timed(Kind::Claim, 0, 1_234, t0), true);
        s.record(timed(Kind::Append, 1, 80, t0), true);
        s.record(timed(Kind::SyncedRewrite, 1050, 45_678, t0), true);
        s.record(timed(Kind::Rewrite, 1052, 20_000, t0), false);
        let c = SessionCounts {
            claims: 1,
            appends: 1,
            appended_tics: 1,
            declined_appends: 0,
            synced_rewrites: 1,
            rewrites: 1,
            most_waiting: 3,
        };
        assert_eq!(
            session_line(&c, &s),
            "session file writes: 4 (claims 1, appends 1 of 1 tics, whole rewrites 1 synced and 1 \
             unsynced), 1 failed; longest 45.6 ms (synced rewrite at tic 1050); at most 3 tics \
             waiting"
        );
        // review 3, n8: a declined append is named, and its tics are not counted
        let declined = SessionCounts {
            appends: 3,
            appended_tics: 1,
            declined_appends: 2,
            ..c.clone()
        };
        assert!(
            session_line(&declined, &s)
                .contains("(claims 1, appends 3 of 1 tics, 2 appends declined, whole rewrites 1")
        );
        assert_eq!(s.slow.len(), 1, "exactly 20 ms is not over 20 ms");
        let mut l = WriteTimes::default();
        assert_eq!(log_line(&l), "input log writes: 0, 0 failed; longest none");
        assert_eq!(
            slow_line(&[&s, &l]).as_str(),
            "writes over 20 ms: synced rewrite at tic 1050 45.6 ms"
        );
        l.record(
            timed(Kind::Log, 7, 20_001, t0 - Duration::from_millis(1)),
            true,
        );
        assert_eq!(
            log_line(&l),
            "input log writes: 1, 0 failed; longest 20.0 ms (input log at tic 7)"
        );
        assert_eq!(
            slow_line(&[&s, &l]),
            "writes over 20 ms: input log at tic 7 20.0 ms; synced rewrite at tic 1050 45.6 ms"
        );
        assert_eq!(
            slow_line(&[&WriteTimes::default()]),
            "writes over 20 ms: none"
        );
        // the cap: at most SLOW_LISTED listed, the rest counted, across both writers
        let mut many = WriteTimes::default();
        for k in 0..(SLOW_LISTED as u64 + 5) {
            many.record(
                timed(Kind::Append, k, 30_000, t0 + Duration::from_millis(k)),
                true,
            );
        }
        assert_eq!((many.slow.len(), many.slow_omitted), (SLOW_LISTED, 5));
        let line = slow_line(&[&many, &l]);
        assert_eq!(line.matches(" ms;").count() + 1, SLOW_LISTED + 1, "{line}");
        assert!(line.ends_with("; 6 more not listed"), "{line}");
        assert!(
            line.starts_with(
                "writes over 20 ms: input log at tic 7 20.0 ms; append at tic 0 30.0 ms"
            ),
            "{line}"
        );
    }
}

/// Mutation-survivor tests of M5d (mut6).
#[cfg(test)]
mod mut6_tests {
    use super::*;

    fn timed(kind: Kind, tic: u64, us: u64, began: Instant) -> Timed {
        Timed {
            kind,
            tic,
            took: Duration::from_micros(us),
            began,
        }
    }

    /// [`WriteTimes::longest`] is "the longest operation". The tie rule: of two that took exactly
    /// as long, the first recorded is named, because [`WriteTimes::record`] replaces the longest
    /// only for a strictly longer operation (the same strictness as the slow list's "over").
    /// The rule is written on the field and beside the comparison in `record`; this test pins
    /// it: with `>=` there, the rewrite at tic 9 would be named and this test would fail.
    #[test]
    fn of_two_equally_long_writes_the_first_is_the_longest() {
        let t0 = Instant::now();
        let mut w = WriteTimes::default();
        w.record(timed(Kind::Append, 5, 3_000, t0), true);
        w.record(timed(Kind::Rewrite, 9, 3_000, t0), true);
        assert_eq!(w.longest.map(|l| (l.kind, l.tic)), Some((Kind::Append, 5)));
    }

    /// [`slow_line`] lists at most [`SLOW_LISTED`] and counts "the rest" ("N more not listed"):
    /// two writers with 21 slow writes each have 42, so 40 are listed and 2 are not.
    #[test]
    fn the_slow_list_counts_exactly_the_writes_it_leaves_out() {
        let t0 = Instant::now();
        let (mut a, mut b) = (WriteTimes::default(), WriteTimes::default());
        for k in 0..21u64 {
            a.record(
                timed(Kind::Append, k, 30_000, t0 + Duration::from_millis(2 * k)),
                true,
            );
            b.record(
                timed(Kind::Log, k, 30_000, t0 + Duration::from_millis(2 * k + 1)),
                true,
            );
        }
        assert_eq!((a.slow.len(), b.slow.len()), (21, 21));
        let line = slow_line(&[&a, &b]);
        assert!(line.ends_with("; 2 more not listed"), "{line}");
        assert_eq!(line.matches(" at tic ").count(), SLOW_LISTED, "{line}");
    }

    /// [`exit_lines`]: "the session writer's line when it made any operation" (none made: no
    /// line), and the slow-write line only "with `--tic-stats`" and when some writer was timed.
    #[test]
    fn the_exit_lines_follow_what_ran_and_what_was_asked_for() {
        let t0 = Instant::now();
        let counts = SessionCounts::default();
        let none = WriteTimes::default();
        // no operation made, no log: nothing, with or without --tic-stats
        assert_eq!(
            exit_lines(Some((&counts, &none)), None, false),
            Vec::<String>::new()
        );
        assert_eq!(
            exit_lines(Some((&counts, &none)), None, true),
            Vec::<String>::new()
        );
        assert_eq!(exit_lines(None, None, true), Vec::<String>::new());
        let mut some = WriteTimes::default();
        some.record(timed(Kind::Claim, 0, 1_000, t0), true);
        // one operation made: its line, and the slow line only with --tic-stats
        let plain = exit_lines(Some((&counts, &some)), None, false);
        assert_eq!(plain.len(), 1, "{plain:?}");
        assert!(
            plain[0].starts_with("session file writes: 1 ("),
            "{plain:?}"
        );
        let stats = exit_lines(Some((&counts, &some)), None, true);
        assert_eq!(stats.len(), 2, "{stats:?}");
        assert_eq!(stats[0], plain[0]);
        assert_eq!(stats[1], "writes over 20 ms: none");
        // the log alone is timed too
        assert_eq!(exit_lines(None, Some(&some), false).len(), 1);
        assert_eq!(exit_lines(None, Some(&some), true).len(), 2);
    }
}
