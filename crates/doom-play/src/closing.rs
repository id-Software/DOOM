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
//! earlier session the writer left behind ([`earlier_lines`]), and the writers' lines. If the
//! session writer's thread panicked, its lines ([`panic_lines`]) say so and name every session
//! whose file does not hold its whole demo, with at least how many tics the file holds (those of
//! the writer's last completed write of it); the current session's line then comes from the
//! writer's report, not the app's status ([`after_panic`]): "saved" if its file holds every tic,
//! otherwise the panic's line about it.

use crate::input_log::LoggedHost;
use crate::mouse_forward::Forward;
use crate::session_writer::{Earlier, OnDisk, Report, Settled, WriterHost, lost};
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

    /// The app's update at monotonic time `now_us`, noted for `--tic-stats`. When the app's tic
    /// clock is not running as the update begins (the app is paused or on an end screen, or was
    /// resumed or restarted since the update before: [`App::next_tic_at`] gives no time), the
    /// statistics are told so first, so that the time since the last tic, which was the
    /// player's, is not taken for an interval between tics ([`TicStats::clock_stopped`]).
    pub(crate) fn update(&mut self, now_us: u64) -> Update {
        let stopped = self.app.next_tic_at().is_none();
        let u = self.app.update(now_us, &mut self.host);
        if let Some(s) = &mut self.stats {
            if stopped {
                s.clock_stopped();
            }
            s.record(now_us, u.ran, u.dropped);
        }
        u
    }

    /// After the app's update: when it is not playing (and the run has not ended), settles the
    /// session file into the app, waiting up to [`END_WAIT`] if the session ended in this update
    /// (`ended_before` is whether it had ended before it).
    ///
    /// First, in every update, it tells the sound queue the session's committed tics and whether
    /// the app is playing, which the queue keeps only to say where the game was at an underrun
    /// ([`crate::audio::Queue::note_tic`]).
    pub(crate) fn settle_after_update(&mut self, ended_before: bool) {
        let playing = self.app.mode() == &Mode::Playing;
        self.host.host.fs.audio.note_tic(self.app.tic(), playing);
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
        let current = current_session(report, self.app.session_name());
        let stats = self.stats.as_ref();
        let mut lines = match after_panic(report, &self.app, current, &fs.dir) {
            Current::App => crate::window::exit_lines(&self.app, fs, stats),
            Current::Saved(line) => {
                let mut lines = crate::window::exit_lines_before_session(&self.app, fs, stats);
                lines.push(line);
                lines
            }
            Current::Named => crate::window::exit_lines_before_session(&self.app, fs, stats),
        };
        lines.extend(earlier_lines(&report.earlier, &fs.dir));
        lines.extend(writer_lines(report, log_times.as_ref(), self.tic_stats));
        lines.extend(panic_lines(report, &fs.dir, current));
        lines
    }
}

/// Where the current session's exit line comes from.
enum Current {
    /// The app's status ([`crate::window::session_line`]).
    App,
    /// The writer's report after a panic: the file holds every tic recorded.
    Saved(String),
    /// [`panic_lines`] names it.
    Named,
}

/// After a panic of the session writer, the current session's exit line comes from the writer's
/// report instead of the app's status (one line per session): "saved PATH (N tics)" when its
/// file holds every tic recorded (the app's status then says the writer stopped before writes
/// handed over were made, which added nothing to the file); [`panic_lines`]'s line when it names
/// it, or when a claim was refused and the session has no name (every claim after the stop
/// fails, so it is a session begun after it). The app's line otherwise, and always for an
/// unrecorded run. `current` is the session's name ([`current_session`]).
fn after_panic(w: &Report, app: &App, current: &str, dir: &Path) -> Current {
    if !w.panicked || !app.recorded() {
        return Current::App;
    }
    if let Some((_, tics)) = w.whole.iter().find(|(n, _)| n == current) {
        return Current::Saved(format!(
            "saved {} ({tics} tics)",
            dir.join(current).display()
        ));
    }
    if w.not_written.iter().any(|n| n.name == current) || (w.refused && current.is_empty()) {
        return Current::Named;
    }
    Current::App
}

/// The current session's name for the exit lines: the app's (`app_name`), except after a panic
/// when the app has none and its last claim was handed over and failed
/// ([`Report::failed_claim`]): that claim was the current session's, so it gets one line, as "the
/// session" (review 6, n1).
fn current_session<'a>(w: &'a Report, app_name: &'a str) -> &'a str {
    match (&w.failed_claim, app_name) {
        (Some(name), "") if w.panicked => name,
        (_, name) => name,
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

/// The writers' lines at exit ([`write_stats::exit_lines`]) and one line per failure the app
/// never heard of, except, after a panic, for a session handed over to the writer: either
/// [`panic_lines`] names it (one line per session) or its file holds every tic recorded, so a
/// failed write of it lost nothing (review 6, m1).
pub(crate) fn writer_lines(w: &Report, log: Option<&WriteTimes>, tic_stats: bool) -> Vec<String> {
    let mut lines = write_stats::exit_lines(Some((&w.counts, &w.times)), log, tic_stats);
    for (name, error) in &w.unreported {
        if w.not_written.iter().any(|n| &n.name == name) || w.whole.iter().any(|(n, _)| n == name) {
            continue;
        }
        lines.push(format!(
            "the session file {name} was not fully written: {error}"
        ));
    }
    lines
}

/// If the session writer's thread panicked: one line saying so, then one line per session whose
/// file does not hold its whole demo ([`crate::session_writer::NotWritten`]), in the order of
/// their claims, saying what its file holds (`current` is the current session at exit, "the
/// session"; the others are "an earlier session"), and one line if a session was begun after the
/// writer stopped (no file was created for it). None of the sessions named was saved by a
/// rewrite at exit: the writes still queued, and the copies of the earlier sessions not yet
/// rewritten when the writer died, died with the thread (a session rewritten at exit before the
/// panic holds every tic and is not named).
pub(crate) fn panic_lines(w: &Report, dir: &Path, current: &str) -> Vec<String> {
    if !w.panicked {
        return Vec::new();
    }
    let mut lines = vec![if w.not_written.is_empty() && !w.refused {
        "the session writer stopped unexpectedly (a panic); every session file it was handed \
         holds its whole demo"
            .to_string()
    } else {
        "the session writer stopped unexpectedly (a panic); the sessions below were not saved, \
         and no rewrite at exit saved them"
            .to_string()
    }];
    for n in &w.not_written {
        let who = if n.name == current {
            "the session"
        } else {
            "an earlier session"
        };
        let path = dir.join(&n.name);
        let recorded = n.recorded;
        lines.push(match n.file {
            Some(OnDisk::Holds(t)) => format!(
                "{who} was not saved: the session writer panicked; {} holds at least {t} of \
                 its {recorded} tics (its last completed write)",
                path.display()
            ),
            None => format!(
                "{who} was not saved: the session writer panicked; {} was not created (no \
                 write of it completed)",
                path.display()
            ),
            Some(OnDisk::Taken) => format!("{who} was not written: {}", lost(dir, &n.name)),
        });
    }
    if w.refused {
        lines.push(if current.is_empty() {
            "the session was not saved, nor was any other session begun after the session \
             writer stopped: no file was created for them"
                .to_string()
        } else {
            "no session begun after the session writer stopped was saved: no file was created \
             for them"
                .to_string()
        });
    }
    lines
}

#[cfg(test)]
#[path = "closing_tests.rs"]
mod tests;
