//! What the window's loop does that is not the window (M5d review 3, M1, m2 and m3): the app, its
//! host (the session writer and the input log) and the mouse forwarding, the session file
//! settled into the app whenever no tic runs, and the run's end with its exit report.
//!
//! **Settling the session file.** The session writer answers the app before it writes
//! ([`crate::session_writer`]), so while tics run a failure reaches the app through its next
//! call. When no tic runs there is no next call, so after every `update` in which the app is not
//! playing the window asks the writer what became of the session's file and tells the app
//! ([`settle`], `App::settle_save`): its status, pause box and end screen then say what is true,
//! and its own save on restart and quit retries a failed write as it did before the writer
//! existed (M5b). At the update in which a session ends, the window waits up to [`END_WAIT`] for
//! the writer first: no tic runs then (the session has ended), so the end screen usually shows
//! the outcome at once. If the disk has not answered by then, the end screen says the demo is
//! not written yet ([`NOT_YET`]); the writer goes on, and the screen changes to DEMO SAVED (or to
//! the failure) at the first update after the writer finishes. Polls while paused or ended do
//! not wait. While tics run nothing here waits.
//!
//! **The run's end** ([`Play::end_run`], once, at quit or when the window closes): the input log's
//! summary line; the session writer drained (every write handed over is made, waiting for the
//! disk, as M5b's synchronous quit did) and settled into the app; the app's own save on quit,
//! which, with the writer now waiting for each write, rewrites the file whole if it is behind or
//! its last write failed and reports the real outcome; the writer closed, which rewrites once
//! more any earlier session it had left behind. The log is ended after the update's last pump
//! line ([`Play::pump_end`]), so the log's bytes are those of b374aaa for the same events (its
//! final `pump` line follows the summary). The exit report then says one thing per session: the
//! current session's line from the app's status ([`crate::window::exit_lines`]), one line per
//! earlier session the writer left behind ([`earlier_lines`]), and the writers' lines.

use crate::input_log::LoggedHost;
use crate::mouse_forward::Forward;
use crate::session_writer::{Earlier, Report, Settled, WriterHost};
use crate::tic_stats::TicStats;
use crate::write_stats::{self, WriteTimes};
use doom_game::app::{App, Mode, SaveStatus, Update};
use std::path::Path;
use std::time::Duration;

/// How long the window waits for the session writer in the update in which a session ends,
/// before the end screen is shown with what is known then (no tic runs during the wait).
pub(crate) const END_WAIT: Duration = Duration::from_secs(2);

/// The failure the end screen shows while the writer has not finished the session's file after
/// [`END_WAIT`] (the app shows it as "DEMO NOT SAVED: ..."); replaced by the outcome once known.
pub(crate) const NOT_YET: &str = "not written yet: the disk has not answered in 2 s; still trying";

/// Tells the app what became of its session file, waiting at most `wait` for the writer (call
/// only when no tic runs): a failure makes the app's status Failed (once per distinct failure);
/// a file holding every recorded tic makes it Saved; writes still being made after a wait that
/// was not zero make a status that says Saved say [`NOT_YET`] instead. Nothing for a session
/// whose file was never created (no name).
pub(crate) fn settle(app: &mut App, host: &WriterHost, wait: Duration) {
    if app.session_name().is_empty() {
        return;
    }
    let len = app.recorder().len();
    match host.settle(app.session_name(), wait) {
        Settled::Failed(e) => {
            if app.save_status() != &SaveStatus::Failed(e.clone()) {
                app.settle_save(Err(e));
            }
        }
        Settled::Written(t) => {
            if t == len as u64 && app.save_status() != &SaveStatus::Saved(len) {
                app.settle_save(Ok(()));
            }
        }
        Settled::Pending => {
            if !wait.is_zero() && matches!(app.save_status(), SaveStatus::Saved(_)) {
                app.settle_save(Err(NOT_YET.to_string()));
            }
        }
    }
}

/// The app, its host and the mouse forwarding, with the run's end (see the module
/// documentation).
pub(crate) struct Play {
    pub(crate) app: App,
    pub(crate) host: LoggedHost<WriterHost>,
    pub(crate) fwd: Forward,
    pub(crate) stats: Option<TicStats>,
    tic_stats: bool,
    /// The session writer's report, once the run's end has closed it.
    closed: Option<Report>,
    /// The exit report was made.
    concluded: bool,
}

impl Play {
    /// `tic_stats`: the `--tic-stats` measurement aid (the tic interval summary and the slow
    /// writes).
    pub(crate) fn new(
        app: App,
        host: LoggedHost<WriterHost>,
        fwd: Forward,
        tic_stats: bool,
    ) -> Self {
        Self {
            app,
            host,
            fwd,
            stats: tic_stats.then(TicStats::default),
            tic_stats,
            closed: None,
            concluded: false,
        }
    }

    /// After the app's update: when it is not playing (and the run has not ended), settles the
    /// session file into the app, waiting up to [`END_WAIT`] if the session ended in this update
    /// (`ended_before` is whether it had ended before it).
    pub(crate) fn settle_after_update(&mut self, ended_before: bool) {
        if self.closed.is_some() || self.app.mode() == &Mode::Playing {
            return;
        }
        let ended_now = !ended_before && matches!(self.app.mode(), Mode::Ended(_));
        let wait = if ended_now { END_WAIT } else { Duration::ZERO };
        settle(&mut self.app, &self.host.host, wait);
    }

    /// The player quits (or the window closes): the app's save on quit, then the run's end.
    /// Nothing once the run has ended (review 4, R1): the writer is closed then, so a second
    /// save on quit (the window's Drop after its quit event, or `--quit-after` in a later
    /// update) could only fail with "the session writer was closed" and replace the outcome
    /// the run's end settled into the app.
    pub(crate) fn quit(&mut self) {
        if self.closed.is_some() {
            return;
        }
        self.app.quit(&mut self.host);
        self.end_run();
    }

    /// The run's end, once (see the module documentation): the log's summary line, the writer
    /// drained and settled into the app, the app's save on quit with the writer waiting, the
    /// writer closed. The exit report follows at [`Play::pump_end`] or [`Play::conclude`].
    pub(crate) fn end_run(&mut self) {
        if self.closed.is_some() {
            return;
        }
        self.host.log.summary(self.app.tic());
        self.host.host.finish();
        settle(&mut self.app, &self.host.host, Duration::ZERO);
        self.app.quit(&mut self.host);
        self.closed = Some(self.host.host.close());
    }

    /// The end of an `update` call: the log's pump line; after the run's end, the exit report
    /// ([`Play::conclude`]), whose lines are returned (once).
    pub(crate) fn pump_end(&mut self, u: Update, captured: bool) -> Vec<String> {
        self.host
            .log
            .pump_end(&self.app, u.ran, u.dropped, captured);
        self.conclude()
    }

    /// After the run's end, once: ends the input log (its thread writes everything handed over)
    /// and returns the exit lines; otherwise nothing.
    pub(crate) fn conclude(&mut self) -> Vec<String> {
        let Some(report) = self.closed.as_ref().filter(|_| !self.concluded) else {
            return Vec::new();
        };
        self.concluded = true;
        let log_times = self.host.log.finish();
        let fs = &self.host.host.fs;
        let mut lines = crate::window::exit_lines(&self.app, fs, self.stats.as_ref());
        lines.extend(earlier_lines(&report.earlier, &fs.dir));
        lines.extend(writer_lines(report, log_times.as_ref(), self.tic_stats));
        lines
    }
}

/// One exit line per earlier session the writer left behind ([`Earlier`]): saved by the rewrite
/// at exit, not saved, or never written (another file took its name).
pub(crate) fn earlier_lines(earlier: &[Earlier], dir: &Path) -> Vec<String> {
    earlier
        .iter()
        .map(|e| {
            let path = dir.join(&e.name);
            match &e.retry {
                None => format!("an earlier session was not written: {}", e.error),
                Some(Ok(tics)) => format!(
                    "an earlier session was saved at exit: {} ({tics} tics), rewritten whole \
                     after its last write failed ({})",
                    path.display(),
                    e.error
                ),
                Some(Err(again)) => format!(
                    "an earlier session was not saved: its last write to {} failed ({}), and the \
                     rewrite at exit failed too ({again})",
                    path.display(),
                    e.error
                ),
            }
        })
        .collect()
}

/// The writers' lines at exit ([`write_stats::exit_lines`]), one line per failure the app never
/// heard of, and one line if the session writer's thread panicked.
pub(crate) fn writer_lines(w: &Report, log: Option<&WriteTimes>, tic_stats: bool) -> Vec<String> {
    let mut lines = write_stats::exit_lines(Some((&w.counts, &w.times)), log, tic_stats);
    for (name, error) in &w.unreported {
        lines.push(format!(
            "the session file {name} was not fully written: {error}"
        ));
    }
    if w.panicked {
        lines.push(
            "the session writer stopped unexpectedly (a panic); the session file holds its last \
             completed write"
                .to_string(),
        );
    }
    lines
}

#[cfg(test)]
#[path = "closing_tests.rs"]
mod tests;
