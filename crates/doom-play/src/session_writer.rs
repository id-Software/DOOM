//! The session writer (M5d, the owner's rulings of 2026-11-20: no file write of any kind may block
//! the tic loop; the queue is bounded in memory; no recorded tic is ever dropped from the session).
//!
//! [`WriterHost`] is the host the window runs: it answers the app at once and hands every write of
//! the session's demo to one writer thread, which makes them in order with the same operations as
//! the synchronous [`crate::FsHost`] ([`crate::session_file`]):
//!
//! - **The claim** ([`Host::claim`]) is answered from the directory (`Path::exists`, a read) and the
//!   names this process has already handed over; the writer then creates the file with
//!   [`session_file::claim_kind`] (a flushed temporary file hard-linked to the name, or the
//!   fallback, ruling 5). If another process took the name in between, the writer never writes it:
//!   the session's later writes fail visibly (the app's pause box and exit line), and the other
//!   file is left alone.
//! - **A tic** ([`Host::append`]) is queued as its five bytes. The writer keeps its own copy of the
//!   demo, and for the tics waiting when it gets to them makes one open of the file and one
//!   five-byte `write` per tic, each inside one page ([`session_file::append_tics`]); when one of
//!   them would straddle a page, it rewrites the file whole instead, unsynced, through
//!   `NAME.PID.tmp` and a rename ([`session_file::save_with`]).
//! - **A whole demo** ([`Host::save`], the app's every-1,050-tics, end, restart and quit writes and
//!   its retries after a failure) is kept as the newest *snapshot* of that session (a newer one
//!   replaces it in place), and the writer rewrites the file whole from it, flushed to the disk
//!   before the rename, as before. Synced rewrites remain only there (and in the claim): on the
//!   writer thread they no longer stop the loop.
//!
//! **The bound** ([`QUEUE_TICS`]): at most that many tics are queued. When the disk stalls and the
//! bound is reached, [`Host::append`] declines (`Ok(false)`) and the app hands the whole demo over
//! through [`Host::save`] instead, which replaces the session's pending snapshot; so the memory held
//! is at most [`QUEUE_TICS`] queued tics plus one demo-sized snapshot for each session whose last
//! snapshot is not written yet (one, unless the player restarts while the disk is stalled), and no
//! tic is ever dropped: the file lags, and the first write after the stall is one whole rewrite
//! holding every tic. Such a catch-up snapshot (one that follows a decline) is written unsynced.
//!
//! **Errors** travel back to the app by two roads. While tics run, a failed write is returned as
//! `Err` from the next call about that session, so the app's existing failed-save handling (the
//! pause box, the exit line, the retry once every 35 tics, ruling 4) works unchanged. After a
//! failure the writer writes nothing more for the session until the app hands a whole demo over
//! (its retry), so a failing disk sees one attempt per retry, not one per tic. While a retry's
//! outcome is not known yet, [`Host::save`] returns the failure it is retrying, so the app keeps
//! its own retry cadence; the outcome is learnt at the next retry. When no tic runs (the session
//! paused or ended, and at quit) there may be no next call, so the window asks
//! ([`WriterHost::settle`]: written, failed or not yet, after a wait it bounds) and tells the app
//! (`App::settle_save`), whose status, pause box, end screen and retries at restart and quit then
//! follow the truth (review 3, M1).
//!
//! **A session the writer moves on from** (the next claim) whose file does not hold its copy is
//! rewritten whole from that copy once more first, unless its last write was already a retry of
//! that same copy: a whole demo written after an earlier failure of the session (such as the
//! app's retry at restart) that failed too, with no tic added to the copy since (review 4, R2).
//! So a session whose last write was its first failure (an append, a periodic, catch-up or end
//! save, the claim) is rewritten once at the next claim whether or not the app heard of the
//! failure, and one the app has just retried is not rewritten again at once. If that fails too,
//! the session is kept, copy and failure, and rewritten whole once more when the writer stops
//! ([`WriterHost::close`]); the outcome is in the [`Report`] ([`Earlier`]), so an earlier
//! session's file is never left behind without an exit line saying so. A session whose name
//! another file took is never written.
//!
//! **If the writer thread panics** (a fault: no write fails that way), the writes still queued
//! are never made, and the earlier sessions left behind that the rewrite at exit had not reached
//! are not rewritten (their copies died with the thread). The exit report still names every
//! session whose file is short: the loop side keeps the names it handed over as claims, in order,
//! with the tics the app recorded in each (the most tics any of its calls about it carried), and
//! the writer notes in the shared state what its last completed write of each session left in the
//! file ([`OnDisk`]); [`WriterHost::close`] compares the two ([`NotWritten`]). A session begun
//! after the writer stopped has no file (its claim cannot be handed over), and the report says so
//! ([`Report::refused`]).
//!
//! **The atomicity argument** is unchanged in kind: every write the writer makes is one of the
//! all-or-nothing operations of [`crate::session_file`] (a five-byte write inside one page, or a
//! temporary file and a rename), made in order on one thread, so at every instant the name holds a
//! valid demo that is a prefix of the session. What changed is the lag: the file holds at least
//! every tic of the writer's last completed write and at most every tic the app ran; a kill (or a
//! lost display connection) loses the tics still queued. A normal quit drains the queue
//! ([`WriterHost::finish`], [`WriterHost::close`]), so the file is then complete.

use crate::FsHost;
use crate::session_file::{self, Claim, TAIL, append_fits_one_page};
use crate::write_stats::{Kind, SessionCounts, Timed, WriteTimes};
use doom_game::app::{AudioSink, Host, Timestamp};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The most tics queued for the writer (five seconds of play). Past it, [`Host::append`] declines
/// and the app hands the whole demo over instead (see the module documentation).
pub(crate) const QUEUE_TICS: u64 = 175;

/// A demo of n tics holds 13 header bytes, 4n command bytes and the end marker.
fn tics_in(len: usize) -> u64 {
    (len as u64).saturating_sub(14) / 4
}

/// The tic count after the append whose five bytes go at `at` (13 + 4(n − 1) for tic n).
fn tic_at(at: u64) -> u64 {
    at.saturating_sub(13) / 4 + 1
}

/// The file operations the writer makes; [`RealIo`] makes them on the disk, tests wrap it (a slow
/// disk, a recording, a planted failure).
pub(crate) trait SessionIo: Send {
    /// Creates `dir/name` with `bytes`, never replacing a file ([`session_file::claim_kind`]).
    fn claim(&mut self, dir: &Path, name: &str, bytes: &[u8]) -> Result<Claim, String>;
    /// Rewrites `dir/name` whole through its temporary file and a rename
    /// ([`session_file::save_with`]).
    fn rewrite(&mut self, dir: &Path, name: &str, bytes: &[u8], synced: bool)
    -> Result<(), String>;
    /// Writes `tails` from byte `at`, one write per tic ([`session_file::append_tics`]).
    fn append(
        &mut self,
        dir: &Path,
        name: &str,
        at: u64,
        tails: &[[u8; 5]],
    ) -> Result<bool, String>;
}

/// The disk.
pub(crate) struct RealIo;

impl SessionIo for RealIo {
    fn claim(&mut self, dir: &Path, name: &str, bytes: &[u8]) -> Result<Claim, String> {
        session_file::claim_kind(dir, name, bytes)
    }
    fn rewrite(
        &mut self,
        dir: &Path,
        name: &str,
        bytes: &[u8],
        synced: bool,
    ) -> Result<(), String> {
        session_file::save_with(dir, name, bytes, synced)
    }
    fn append(
        &mut self,
        dir: &Path,
        name: &str,
        at: u64,
        tails: &[[u8; 5]],
    ) -> Result<bool, String> {
        session_file::append_tics(dir, name, at, tails)
    }
}

/// What the loop hands over.
enum Msg {
    /// A new session's first content.
    Claim { name: String, bytes: Vec<u8> },
    /// Take the oldest pending snapshot ([`State::snapshots`]).
    Snapshot,
    /// One tic of the current session: its five bytes at `at`.
    Append { at: u64, tail: [u8; 5] },
}

/// A whole demo waiting for the writer.
struct Snapshot {
    name: String,
    bytes: Vec<u8>,
    synced: bool,
}

/// The current session's failed write, which the app may not have heard of yet.
struct Failure {
    name: String,
    error: String,
    reported: bool,
}

/// What the loop and the writer share. The writer holds the lock only to read or note a result,
/// never across a file operation, so the loop never waits for the disk through it.
#[derive(Default)]
struct State {
    /// Messages the writer has finished with.
    done: u64,
    /// Whether the writer thread runs.
    alive: bool,
    /// Append messages the writer has finished with.
    appends_done: u64,
    /// The pending snapshots, oldest first, at most one per session at the back.
    snapshots: VecDeque<Snapshot>,
    /// The session file of the writer's last completed write and the tics it holds.
    held: Option<(String, u64)>,
    failure: Option<Failure>,
    /// Failures of a session that is not the one being written (unreachable by construction:
    /// a whole demo handed over for another session).
    strays: Vec<(String, String)>,
    /// The earlier sessions left behind and what their rewrite at the writer's end did.
    earlier: Vec<Earlier>,
    /// What the writer's last completed write of each session left in its file (or that another
    /// file took its name), for the report after a panic; a session absent here has no file of
    /// this run (no claim of it completed).
    files: BTreeMap<String, OnDisk>,
    counts: SessionCounts,
    times: WriteTimes,
    /// The most tics ever queued at a hand-over, and the most snapshots pending.
    most_queued: u64,
    most_snapshots: usize,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What a closed writer did, for the exit lines.
#[derive(Debug, Default)]
pub(crate) struct Report {
    pub(crate) counts: SessionCounts,
    pub(crate) times: WriteTimes,
    /// Failures the app never heard of: (file name, error).
    pub(crate) unreported: Vec<(String, String)>,
    /// The earlier sessions whose files the writer left behind, in order.
    pub(crate) earlier: Vec<Earlier>,
    /// The writer thread panicked.
    pub(crate) panicked: bool,
    /// After a panic: every session handed over whose file does not hold every tic the app
    /// recorded in it, in the order of their claims; empty when the writer did not panic.
    pub(crate) not_written: Vec<NotWritten>,
    /// After a panic: every other session handed over (its file holds every tic the app recorded
    /// in it), with that count; empty when the writer did not panic.
    pub(crate) whole: Vec<(String, u64)>,
    /// After a panic: a claim could not be handed over because the writer had stopped, so a
    /// session begun after it has no file.
    pub(crate) refused: bool,
    /// After a panic: the last claim asked for was handed over and its outcome was an error, so
    /// the app keeps no name for that session (review 6, n1). A claim's outcome can be an error
    /// only after [`WriterHost::finish`], whose one call from the app (its save on quit) is the
    /// last, so in the window's run this is the app's current session. `None` when the writer
    /// did not panic.
    pub(crate) failed_claim: Option<String>,
}

/// What the writer's last completed write of a session left in its file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OnDisk {
    /// The writer's last completed write left this many tics in the file. The file holds at
    /// least this many: a later append of several tics that failed after its first write
    /// ([`session_file::append_tics`] makes one write per tic) may have added some, and no write
    /// shortens the file (review 6, m2).
    Holds(u64),
    /// Another file took the name before the claim: never written.
    Taken,
}

/// A session whose file does not hold its whole demo after the writer panicked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NotWritten {
    /// The file's name.
    pub(crate) name: String,
    /// The tics the app recorded in the session (the most any of its calls about it carried).
    pub(crate) recorded: u64,
    /// What the writer's last completed write left in the file; `None` when no write of it
    /// completed (no file of this run was created).
    pub(crate) file: Option<OnDisk>,
}

/// An earlier session whose file did not hold its whole demo when the writer moved on to the
/// next session (its last write failed, or another file took its name), and what became of it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Earlier {
    /// The file's name.
    pub(crate) name: String,
    /// The failure it was left with.
    pub(crate) error: String,
    /// The whole rewrite when the writer stopped: the tics then written, or its failure;
    /// `None` when another file took the name (never written).
    pub(crate) retry: Option<Result<u64, String>>,
}

/// What became of a session's file, as far as the writer knows ([`WriterHost::settle`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Settled {
    /// Every write handed over is done; the file holds this many tics.
    Written(u64),
    /// The session's last write failed (or the writer stopped before making it).
    Failed(String),
    /// Writes handed over are still being made (the wait ended first).
    Pending,
}

/// The failure when the writer thread ended before making every write handed over (a panic).
const STOPPED: &str = "the session writer stopped before writing it";

/// The host the window runs (see the module documentation): [`FsHost`] for the clock, the
/// directory and the sound, and the writer thread for every write.
pub(crate) struct WriterHost {
    /// The sessions directory and the sound output.
    pub(crate) fs: FsHost,
    tx: Option<mpsc::Sender<Msg>>,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
    /// Every name this process handed over as a claim.
    reserved: BTreeSet<String>,
    /// The session of the last claim.
    current: Option<String>,
    /// Every session handed over as a claim, in order, with the most tics the app has recorded
    /// in it (the tics its calls about it carried), for the report after a panic.
    handed: Vec<(String, u64)>,
    /// A claim could not be handed over (the writer had stopped).
    refused: bool,
    /// The last claim asked for was handed over and its outcome was an error ([`Report`]).
    failed_claim: Option<String>,
    sent: u64,
    appends_sent: u64,
    /// The last append declined at the bound: the next snapshot is a catch-up (unsynced).
    declined: bool,
    /// After [`WriterHost::finish`]: every call waits for its write and returns its outcome.
    synchronous: bool,
}

impl WriterHost {
    /// Starts the writer on the disk; its notices (the fallback claim's line) go to standard
    /// error.
    pub(crate) fn new(fs: FsHost) -> Result<Self, String> {
        Self::with_io(fs, Box::new(RealIo), Box::new(|line| eprintln!("{line}")))
    }

    /// Starts the writer with the file operations `io` and the notice sink `notify`.
    pub(crate) fn with_io(
        fs: FsHost,
        io: Box<dyn SessionIo>,
        notify: Box<dyn FnMut(String) + Send>,
    ) -> Result<Self, String> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                alive: true,
                ..State::default()
            }),
            changed: Condvar::new(),
        });
        let (tx, rx) = mpsc::channel();
        let writer = Writer {
            dir: fs.dir.clone(),
            io,
            notify,
            shared: Arc::clone(&shared),
            session: None,
            left: Vec::new(),
        };
        let thread = std::thread::Builder::new()
            .name("session-writer".into())
            .spawn(move || writer.run(rx))
            .map_err(|e| format!("the session writer could not start: {e}"))?;
        Ok(Self {
            fs,
            tx: Some(tx),
            shared,
            thread: Some(thread),
            reserved: BTreeSet::new(),
            current: None,
            handed: Vec::new(),
            refused: false,
            failed_claim: None,
            sent: 0,
            appends_sent: 0,
            declined: false,
            synchronous: false,
        })
    }

    fn send(&mut self, msg: Msg) -> Result<(), String> {
        let tx = self.tx.as_ref().ok_or("the session writer was closed")?;
        tx.send(msg)
            .map_err(|_| "the session writer stopped".to_string())?;
        self.sent += 1;
        Ok(())
    }

    /// Waits until the writer has finished with every message sent (or has stopped).
    pub(crate) fn wait_idle(&self) {
        let mut st = self.shared.lock();
        while st.alive && st.done < self.sent {
            st = self
                .shared
                .changed
                .wait(st)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Notes that the app has recorded `tics` tics in `name` (a session handed over).
    fn note_recorded(&mut self, name: &str, tics: u64) {
        if let Some((_, t)) = self.handed.iter_mut().rev().find(|(n, _)| n == name) {
            *t = (*t).max(tics);
        }
    }

    /// The failure of `name`'s writes, now reported to the app.
    fn failure_of(&self, name: &str) -> Option<String> {
        let mut st = self.shared.lock();
        let f = st.failure.as_mut().filter(|f| f.name == name)?;
        f.reported = true;
        Some(f.error.clone())
    }

    /// Notes how many tics are waiting once `name` will hold `tics`.
    fn note_waiting(st: &mut State, name: &str, tics: u64) {
        let held = match &st.held {
            Some((n, t)) if n == name => *t,
            _ => 0,
        };
        st.counts.most_waiting = st.counts.most_waiting.max(tics.saturating_sub(held));
    }

    /// Whether the writer thread ended before finishing every message sent.
    fn stopped_short(&self) -> bool {
        let st = self.shared.lock();
        !st.alive && st.done < self.sent
    }

    /// In synchronous mode, waits and returns the outcome for `name`; otherwise `queued`.
    fn outcome(&self, name: &str, queued: Result<(), String>) -> Result<(), String> {
        if self.synchronous {
            self.wait_idle();
            if let Some(e) = self.failure_of(name) {
                return Err(e);
            }
            if self.stopped_short() {
                return Err(STOPPED.to_string());
            }
            return Ok(());
        }
        queued
    }

    /// What became of `name`'s file: waits at most `wait` for the writer to finish every write
    /// handed over (the window calls this only when no tic runs: a session paused or ended, or
    /// at quit), then says whether the session's last write failed, what the file holds, or
    /// that writes are still being made. A failure answered here counts as reported to the app.
    pub(crate) fn settle(&self, name: &str, wait: Duration) -> Settled {
        let deadline = Instant::now().checked_add(wait);
        let mut st = self.shared.lock();
        while st.alive && st.done < self.sent {
            let left = deadline.map_or(Duration::MAX, |d| {
                d.saturating_duration_since(Instant::now())
            });
            if left.is_zero() {
                break;
            }
            st = self
                .shared
                .changed
                .wait_timeout(st, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        if let Some(f) = st.failure.as_mut().filter(|f| f.name == name) {
            f.reported = true;
            return Settled::Failed(f.error.clone());
        }
        if st.done < self.sent {
            return if st.alive {
                Settled::Pending
            } else {
                Settled::Failed(STOPPED.to_string())
            };
        }
        match &st.held {
            Some((n, t)) if n == name => Settled::Written(*t),
            _ => Settled::Pending,
        }
    }

    /// The end of play: drains the queue, and from now on every call waits for its write and
    /// returns its real outcome (the app's save on quit then reports the truth).
    pub(crate) fn finish(&mut self) {
        self.synchronous = true;
        self.wait_idle();
    }

    /// The tics `name`'s file held after the writer's last completed write, `None` before its
    /// claim completed (for the kill test).
    #[cfg(test)]
    pub(crate) fn written(&self, name: &str) -> Option<u64> {
        match &self.shared.lock().held {
            Some((n, t)) if n == name => Some(*t),
            _ => None,
        }
    }

    /// The tics queued and the snapshots pending now, and the most of each ever seen.
    #[cfg(test)]
    pub(crate) fn queue_depth(&self) -> (u64, usize, u64, usize) {
        let st = self.shared.lock();
        (
            self.appends_sent - st.appends_done,
            st.snapshots.len(),
            st.most_queued,
            st.most_snapshots,
        )
    }

    /// Stops the writer after it has finished every write handed over and rewritten the earlier
    /// sessions it left behind once more ([`Earlier`]), and reports what it did. Later calls
    /// fail (`Err`); closing again reports the same counts and nothing as unreported. If the
    /// writer thread panicked, the report names every session handed over whose file does not
    /// hold every tic the app recorded in it ([`NotWritten`]) and whether a claim was refused.
    pub(crate) fn close(&mut self) -> Report {
        self.tx = None;
        let panicked = self.thread.take().is_some_and(|t| t.join().is_err());
        let mut st = self.shared.lock();
        let mut unreported = std::mem::take(&mut st.strays);
        if let Some(f) = st.failure.as_mut().filter(|f| !f.reported) {
            f.reported = true;
            unreported.push((f.name.clone(), f.error.clone()));
        }
        let (mut not_written, mut whole) = (Vec::new(), Vec::new());
        if panicked {
            for (name, recorded) in &self.handed {
                let file = st.files.get(name).copied();
                if matches!(file, Some(OnDisk::Holds(t)) if t >= *recorded) {
                    whole.push((name.clone(), *recorded));
                } else {
                    not_written.push(NotWritten {
                        name: name.clone(),
                        recorded: *recorded,
                        file,
                    });
                }
            }
        }
        Report {
            counts: st.counts.clone(),
            times: st.times.clone(),
            unreported,
            earlier: std::mem::take(&mut st.earlier),
            panicked,
            not_written,
            whole,
            refused: panicked && self.refused,
            failed_claim: self.failed_claim.clone().filter(|_| panicked),
        }
    }
}

impl Drop for WriterHost {
    /// Drains and joins the writer, so the file is complete when the host is gone.
    fn drop(&mut self) {
        self.tx = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Host for WriterHost {
    fn timestamp(&mut self) -> Timestamp {
        self.fs.timestamp()
    }
    /// A name exists when the directory holds it or this process handed it over as a claim.
    fn exists(&mut self, name: &str) -> bool {
        self.reserved.contains(name) || self.fs.exists(name)
    }
    /// Answered at once (see the module documentation): `Ok(false)` for a taken or handed-over
    /// name; otherwise the claim is queued and `Ok(true)`.
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        self.failed_claim = None;
        if self.exists(name) {
            return Ok(false);
        }
        self.reserved.insert(name.to_string());
        self.current = Some(name.to_string());
        self.declined = false;
        if let Err(e) = self.send(Msg::Claim {
            name: name.to_string(),
            bytes: bytes.to_vec(),
        }) {
            self.refused = true;
            return Err(e);
        }
        self.handed.push((name.to_string(), tics_in(bytes.len())));
        let outcome = self.outcome(name, Ok(()));
        if outcome.is_err() {
            self.failed_claim = Some(name.to_string());
        }
        outcome.map(|()| true)
    }
    /// Keeps `bytes` as the session's newest snapshot (replacing a pending one) and returns at
    /// once: `Ok`, or the session's failure while one stands (its retry is this snapshot).
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.note_recorded(name, tics_in(bytes.len()));
        let failed = self.failure_of(name);
        let synced = !std::mem::take(&mut self.declined);
        let wake = {
            let mut st = self.shared.lock();
            Self::note_waiting(&mut st, name, tics_in(bytes.len()));
            let wake = match st.snapshots.back_mut() {
                Some(s) if s.name == name => {
                    s.bytes.clear();
                    s.bytes.extend_from_slice(bytes);
                    s.synced |= synced;
                    false
                }
                _ => {
                    st.snapshots.push_back(Snapshot {
                        name: name.to_string(),
                        bytes: bytes.to_vec(),
                        synced,
                    });
                    true
                }
            };
            st.most_snapshots = st.most_snapshots.max(st.snapshots.len());
            wake
        };
        if wake {
            self.send(Msg::Snapshot)?;
        }
        self.outcome(name, failed.map_or(Ok(()), Err))
    }
    fn appends(&self) -> bool {
        true
    }
    /// Queues the tic and returns `Ok(true)` at once; `Ok(false)` at the bound ([`QUEUE_TICS`]),
    /// so that the app hands the whole demo over instead; the session's failure while one stands.
    fn append(&mut self, name: &str, at: u64, tail: &[u8; 5]) -> Result<bool, String> {
        self.note_recorded(name, tic_at(at));
        if self.current.as_deref() != Some(name) {
            return Err(format!("{name}: not the session this process claimed last"));
        }
        if let Some(e) = self.failure_of(name) {
            return Err(e);
        }
        {
            let mut st = self.shared.lock();
            let queued = self.appends_sent - st.appends_done;
            if queued >= QUEUE_TICS {
                self.declined = true;
                return Ok(false);
            }
            Self::note_waiting(&mut st, name, tic_at(at));
            st.most_queued = st.most_queued.max(queued + 1);
        }
        self.send(Msg::Append { at, tail: *tail })?;
        self.appends_sent += 1;
        self.outcome(name, Ok(())).map(|()| true)
    }
    fn audio(&mut self) -> Option<&mut dyn AudioSink> {
        self.fs.audio()
    }
}

impl crate::keys::Mute for WriterHost {
    fn toggle_mute(&mut self) {
        self.fs.toggle_mute();
    }
}

/// Whether the session's file holds the writer's copy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Disk {
    /// The claim failed with an error: no file of ours; the next snapshot claims again.
    Unclaimed,
    /// Another file took the name: never written.
    Lost,
    /// The file holds the copy as it was before the pending changes.
    Current,
    /// The last write failed: nothing is written until the next snapshot.
    Stale,
}

/// The writer's session: the file's name, its own copy of the demo (always every tic handed
/// over), and the file's state.
struct Session {
    name: String,
    model: Vec<u8>,
    disk: Disk,
    /// The last write attempted was a retry of the copy as it stands: a whole demo the app
    /// handed over, written while the file did not hold the copy (an earlier write had failed),
    /// that failed too, and no tic was added to the copy since. Moving on to the next session
    /// then does not rewrite it once more at once (review 4, R2).
    retried: bool,
}

/// An earlier session whose file did not hold its copy when the writer moved on.
struct Left {
    session: Session,
    error: String,
}

/// The changes of one batch not written yet.
#[derive(Default)]
struct Pending {
    whole: bool,
    synced: bool,
    first_at: u64,
    tails: Vec<[u8; 5]>,
}

struct Writer {
    dir: PathBuf,
    io: Box<dyn SessionIo>,
    notify: Box<dyn FnMut(String) + Send>,
    shared: Arc<Shared>,
    session: Option<Session>,
    /// The earlier sessions left behind, rewritten once more when the writer stops.
    left: Vec<Left>,
}

/// Marks the writer stopped when its thread ends (also by a panic), so no wait hangs; by then
/// the receiving end is gone ([`Writer::run`]).
struct Alive(Arc<Shared>);

impl Drop for Alive {
    fn drop(&mut self) {
        self.0.lock().alive = false;
        self.0.changed.notify_all();
    }
}

impl Writer {
    /// Takes what is queued in batches until the loop's side is gone and the queue is empty.
    fn run(mut self, rx: mpsc::Receiver<Msg>) {
        let _alive = Alive(Arc::clone(&self.shared));
        // dropped before `_alive` (locals drop in reverse order, also when unwinding): once the
        // writer is marked stopped, no message can be handed over any more, so a claim after it
        // is refused, never queued for no one
        let rx = rx;
        while let Ok(first) = rx.recv() {
            let mut batch = vec![first];
            batch.extend(rx.try_iter());
            let n = batch.len() as u64;
            let appends = batch
                .iter()
                .filter(|m| matches!(m, Msg::Append { .. }))
                .count() as u64;
            self.process(batch);
            let mut st = self.shared.lock();
            st.done += n;
            st.appends_done += appends;
            drop(st);
            self.shared.changed.notify_all();
        }
        self.rewrite_left();
    }

    /// The writer's end: each earlier session left behind is rewritten whole once more from its
    /// copy (a claim if its file was never created); the outcomes go to the report.
    fn rewrite_left(&mut self) {
        let mut earlier = Vec::new();
        for l in std::mem::take(&mut self.left) {
            let s = &l.session;
            let tics = tics_in(s.model.len());
            let retry = match s.disk {
                Disk::Lost | Disk::Current => None,
                Disk::Unclaimed => {
                    let began = Instant::now();
                    let r = self.io.claim(&self.dir, &s.name, &s.model);
                    let ok = matches!(r, Ok(Claim::Linked | Claim::Created));
                    self.record(Kind::Claim, tics, began, ok);
                    match r {
                        Ok(Claim::Linked) => Some(Ok(tics)),
                        Ok(Claim::Created) => {
                            (self.notify)(session_file::fallback_line(&self.dir.join(&s.name)));
                            Some(Ok(tics))
                        }
                        Ok(Claim::Taken) => {
                            self.note_file(&s.name, OnDisk::Taken);
                            Some(Err(lost(&self.dir, &s.name)))
                        }
                        Err(e) => Some(Err(e)),
                    }
                }
                Disk::Stale => {
                    let began = Instant::now();
                    let r = self.io.rewrite(&self.dir, &s.name, &s.model, true);
                    self.record(Kind::SyncedRewrite, tics, began, r.is_ok());
                    Some(r.map(|()| tics))
                }
            };
            if let Some(Ok(tics)) = &retry {
                self.note_file(&s.name, OnDisk::Holds(*tics));
            }
            earlier.push(Earlier {
                name: l.session.name,
                error: l.error,
                retry,
            });
        }
        self.shared.lock().earlier = earlier;
    }

    /// The writer moves on from the current session (a new claim): if its file does not hold its
    /// copy, it is rewritten whole once more (claimed, if it never was), unless its last write
    /// was already a failed retry of that same copy ([`Session::retried`]); if it still does not,
    /// the session is kept for
    /// [`Writer::rewrite_left`], with its failure, which leaves the shared state.
    fn retire(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        if !s.retried {
            match s.disk {
                Disk::Unclaimed => self.claim(),
                Disk::Stale => self.rewrite(true),
                Disk::Lost | Disk::Current => {}
            }
        }
        let Some(session) = self.session.take() else {
            return;
        };
        let failure = {
            let mut st = self.shared.lock();
            st.failure.take_if(|f| f.name == session.name)
        };
        if session.disk != Disk::Current {
            let error = failure.map_or_else(|| "its last write failed".to_string(), |f| f.error);
            self.left.push(Left { session, error });
        }
    }

    fn process(&mut self, batch: Vec<Msg>) {
        let mut p = Pending::default();
        for msg in batch {
            match msg {
                Msg::Claim { name, bytes } => {
                    self.flush(&mut p);
                    self.retire();
                    self.session = Some(Session {
                        name,
                        model: bytes,
                        disk: Disk::Unclaimed,
                        retried: false,
                    });
                    self.claim();
                }
                Msg::Snapshot => {
                    let Some(snap) = self.shared.lock().snapshots.pop_front() else {
                        continue;
                    };
                    match self.session.as_mut() {
                        Some(s) if s.name == snap.name => {
                            s.model = snap.bytes;
                            p.whole = true;
                            p.synced |= snap.synced;
                            p.tails.clear();
                        }
                        _ => self.fail(
                            &snap.name,
                            format!("{}: not the session being written", snap.name),
                        ),
                    }
                }
                Msg::Append { at, tail } => self.take_append(&mut p, at, tail),
            }
        }
        self.flush(&mut p);
    }

    /// Applies one queued tic to the copy: skipped when a later snapshot already holds it;
    /// queued for the file when the file is current (or a whole rewrite is pending anyway).
    fn take_append(&mut self, p: &mut Pending, at: u64, tail: [u8; 5]) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        if tic_at(at) <= tics_in(s.model.len()) {
            return; // a snapshot handed over after it holds it
        }
        if s.model.len() as u64 != at + 1 {
            let e = format!(
                "{}: a tic at byte {at}, but the writer's copy holds {} bytes",
                s.name,
                s.model.len()
            );
            if s.disk == Disk::Current {
                s.disk = Disk::Stale;
            }
            let name = s.name.clone();
            return self.fail(&name, e);
        }
        s.model.truncate(at as usize);
        s.model.extend_from_slice(&tail);
        s.retried = false; // the copy changed: no write has tried it yet
        if !p.whole && s.disk == Disk::Current {
            if p.tails.is_empty() {
                p.first_at = at;
            }
            p.tails.push(tail);
        }
    }

    /// Writes what the batch changed: the whole file when a snapshot came (synced unless it was a
    /// catch-up) or when the tics would straddle a page; otherwise one append operation.
    fn flush(&mut self, p: &mut Pending) {
        let p = std::mem::take(p);
        let Some(s) = self.session.as_ref() else {
            return;
        };
        if !p.whole && p.tails.is_empty() {
            return;
        }
        // a whole demo written while the file does not hold the copy is a retry
        let retry = p.whole && matches!(s.disk, Disk::Unclaimed | Disk::Stale);
        match s.disk {
            Disk::Lost => {
                let name = s.name.clone();
                let e = lost(&self.dir, &name);
                self.fail(&name, e);
                return;
            }
            // nothing until the app hands a whole demo over (its retry cadence, ruling 4)
            Disk::Unclaimed | Disk::Stale if !p.whole => return,
            Disk::Unclaimed => self.claim(),
            Disk::Current if !p.whole => {
                let fits = (0..p.tails.len() as u64)
                    .all(|i| append_fits_one_page(p.first_at + 4 * i, TAIL));
                if fits {
                    self.append(p.first_at, &p.tails);
                } else {
                    self.rewrite(false);
                }
            }
            Disk::Current | Disk::Stale => self.rewrite(p.synced),
        }
        if let Some(s) = self.session.as_mut() {
            s.retried = retry && s.disk != Disk::Current;
        }
    }

    /// Notes a completed operation's time and kind.
    fn record(&self, kind: Kind, tic: u64, began: Instant, ok: bool) {
        let took = began.elapsed();
        let mut st = self.shared.lock();
        match kind {
            Kind::Claim => st.counts.claims += 1,
            Kind::Append => st.counts.appends += 1,
            Kind::SyncedRewrite => st.counts.synced_rewrites += 1,
            Kind::Rewrite | Kind::Log => st.counts.rewrites += 1,
        }
        st.times.record(
            Timed {
                kind,
                tic,
                took,
                began,
            },
            ok,
        );
    }

    /// A write that left the file holding the copy: current, held, its failure cleared.
    fn written(&mut self) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        s.disk = Disk::Current;
        let (name, tics) = (s.name.clone(), tics_in(s.model.len()));
        let mut st = self.shared.lock();
        if st.failure.as_ref().is_some_and(|f| f.name == name) {
            st.failure = None;
        }
        st.files.insert(name.clone(), OnDisk::Holds(tics));
        st.held = Some((name, tics));
    }

    /// Notes what a completed write (or a claim that found the name taken) left in `name`'s
    /// file, for the report after a panic.
    fn note_file(&self, name: &str, on_disk: OnDisk) {
        self.shared.lock().files.insert(name.to_string(), on_disk);
    }

    fn claim(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let (name, tics) = (s.name.clone(), tics_in(s.model.len()));
        let began = Instant::now();
        let r = self.io.claim(&self.dir, &name, &s.model);
        self.record(
            Kind::Claim,
            tics,
            began,
            matches!(r, Ok(Claim::Linked | Claim::Created)),
        );
        match r {
            Ok(Claim::Linked) => self.written(),
            Ok(Claim::Created) => {
                (self.notify)(session_file::fallback_line(&self.dir.join(&name)));
                self.written();
            }
            Ok(Claim::Taken) => {
                self.set_disk(Disk::Lost);
                self.note_file(&name, OnDisk::Taken);
                let e = lost(&self.dir, &name);
                self.fail(&name, e);
            }
            Err(e) => {
                self.set_disk(Disk::Unclaimed);
                self.fail(&name, e);
            }
        }
    }

    fn rewrite(&mut self, synced: bool) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let (name, tics) = (s.name.clone(), tics_in(s.model.len()));
        let kind = if synced {
            Kind::SyncedRewrite
        } else {
            Kind::Rewrite
        };
        let began = Instant::now();
        let r = self.io.rewrite(&self.dir, &name, &s.model, synced);
        self.record(kind, tics, began, r.is_ok());
        match r {
            Ok(()) => self.written(),
            Err(e) => {
                self.set_disk(Disk::Stale);
                self.fail(&name, e);
            }
        }
    }

    fn append(&mut self, at: u64, tails: &[[u8; 5]]) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let (name, tics) = (s.name.clone(), tics_in(s.model.len()));
        let began = Instant::now();
        let r = self.io.append(&self.dir, &name, at, tails);
        self.record(Kind::Append, tics, began, r.is_ok());
        {
            let mut st = self.shared.lock();
            match r {
                Ok(true) => st.counts.appended_tics += tails.len() as u64,
                Ok(false) => st.counts.declined_appends += 1,
                Err(_) => {}
            }
        }
        match r {
            Ok(true) => self.written(),
            // the file is not the length the copy says: repaired by a whole rewrite
            Ok(false) => self.rewrite(false),
            Err(e) => {
                self.set_disk(Disk::Stale);
                self.fail(&name, e);
            }
        }
    }

    fn set_disk(&mut self, disk: Disk) {
        if let Some(s) = self.session.as_mut() {
            s.disk = disk;
        }
    }

    /// Records a failure of the current session's writes for the app's next call about it (a
    /// repeat of the standing one changes nothing); a failure of any other session goes to the
    /// report as unreported.
    fn fail(&mut self, name: &str, error: String) {
        let current = self.session.as_ref().is_some_and(|s| s.name == name);
        let mut st = self.shared.lock();
        if !current {
            st.strays.push((name.to_string(), error));
            return;
        }
        if st
            .failure
            .as_ref()
            .is_some_and(|f| f.name == name && f.error == error)
        {
            return;
        }
        st.failure = Some(Failure {
            name: name.to_string(),
            error,
            reported: false,
        });
    }
}

/// The error of a session whose name another file took before the claim.
pub(crate) fn lost(dir: &Path, name: &str) -> String {
    format!(
        "{}: another file took the name before this session's file was created; it was left \
         alone and this session is not written",
        dir.join(name).display()
    )
}

#[cfg(test)]
#[path = "session_writer_tests.rs"]
pub(crate) mod tests;
