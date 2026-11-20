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
    /// The longest operation.
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
    /// Append operations (one open each).
    pub(crate) appends: u64,
    /// Tics appended (one five-byte write each).
    pub(crate) appended_tics: u64,
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

/// The session writer's exit line.
pub(crate) fn session_line(c: &SessionCounts, t: &WriteTimes) -> String {
    format!(
        "session file writes: {} (claims {}, appends {} of {} tics, whole rewrites {} synced and \
         {} unsynced), {} failed; {}; at most {} tics waiting",
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
