//! The session writer's tests (M5d, the owner's rulings of 2026-11-20). Rust-only checks of
//! doom-play's file handling; each was planted to fail (see the M5d record).

use super::*;
use crate::audio::{Discard, Output};
use doom_game::app::{App, GameData, SaveStatus, SessionConfig};
use doom_render::GameMode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

pub(crate) fn load() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    let files: Vec<Vec<u8>> = ["m5a_wads/amp.wad", "m5b_gfx.wad", "m5c_sfx.wad"]
        .iter()
        .map(|w| std::fs::read(dir.join(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    GameData::load_with_sound(&refs, GameMode::Registered, 1, 2).expect("game data")
}

pub(crate) fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

pub(crate) fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("doom-play-writer-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub(crate) fn fs(dir: &Path) -> FsHost {
    FsHost {
        dir: dir.to_path_buf(),
        audio: Output::Discard(Discard::default()),
    }
}

/// A gate the test opens: every file operation waits for it (a stalled disk).
#[derive(Default)]
pub(crate) struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
    /// How many callers wait at the closed gate now ([`Gate::await_waiter`]).
    waiting: Mutex<usize>,
    arrived: Condvar,
}

impl Gate {
    pub(crate) fn closed() -> Arc<Self> {
        Arc::new(Self::default())
    }
    pub(crate) fn set(&self, open: bool) {
        *self.open.lock().unwrap() = open;
        self.changed.notify_all();
    }
    /// A guard that opens the gate when dropped, so that a failing assertion with the disk
    /// stalled ends the test instead of leaving the host's drain waiting on the gate.
    pub(crate) fn opens_on_drop(self: &Arc<Self>) -> Opens {
        Opens(Arc::clone(self))
    }
    pub(crate) fn pass(&self) {
        let mut open = self.open.lock().unwrap();
        if *open {
            return;
        }
        *self.waiting.lock().unwrap() += 1;
        self.arrived.notify_all();
        while !*open {
            open = self.changed.wait(open).unwrap();
        }
        *self.waiting.lock().unwrap() -= 1;
    }
    /// Waits (at most 10 s) until a caller waits at the closed gate: the writer has taken an
    /// operation and is held in it.
    pub(crate) fn await_waiter(&self) {
        let waiting = self.waiting.lock().unwrap();
        let (_waiting, wait) = self
            .arrived
            .wait_timeout_while(waiting, Duration::from_secs(10), |n| *n == 0)
            .unwrap();
        assert!(!wait.timed_out(), "nothing came to the closed gate in 10 s");
    }
}

pub(crate) struct Opens(Arc<Gate>);

impl Drop for Opens {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

/// One file operation as the writer asked for it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Op {
    Claim,
    Rewrite { synced: bool, tics: u64 },
    Append { at: u64, tics: usize },
}

/// The disk ([`RealIo`]) with what a test needs: a delay before each operation, a gate, a record
/// of the operations, planted failures, another process taking the name before the claim, and a
/// claim reported as the fallback.
#[derive(Default)]
pub(crate) struct TestIo {
    pub(crate) delay: Duration,
    pub(crate) gate: Option<Arc<Gate>>,
    pub(crate) ops: Arc<Mutex<Vec<Op>>>,
    pub(crate) fail_appends: Arc<AtomicBool>,
    pub(crate) fail_rewrites: Arc<AtomicBool>,
    /// The next this many whole rewrites fail; later ones succeed (unless `fail_rewrites`).
    pub(crate) fail_next_rewrites: Arc<AtomicUsize>,
    /// A whole rewrite panics (the writer thread dies).
    pub(crate) panic_rewrites: Arc<AtomicBool>,
    pub(crate) race: bool,
    pub(crate) fallback: bool,
}

impl TestIo {
    fn before(&self, op: Op) {
        if let Some(g) = &self.gate {
            g.pass();
        }
        std::thread::sleep(self.delay);
        self.ops.lock().unwrap().push(op);
    }
}

impl SessionIo for TestIo {
    fn claim(&mut self, dir: &Path, name: &str, bytes: &[u8]) -> Result<Claim, String> {
        self.before(Op::Claim);
        if self.race {
            std::fs::write(dir.join(name), b"another process's file").unwrap();
        }
        let c = RealIo.claim(dir, name, bytes)?;
        Ok(if self.fallback && c == Claim::Linked {
            Claim::Created
        } else {
            c
        })
    }
    fn rewrite(
        &mut self,
        dir: &Path,
        name: &str,
        bytes: &[u8],
        synced: bool,
    ) -> Result<(), String> {
        self.before(Op::Rewrite {
            synced,
            tics: tics_in(bytes.len()),
        });
        let next = || {
            let mut n = self.fail_next_rewrites.load(Ordering::SeqCst);
            while n > 0 {
                match self.fail_next_rewrites.compare_exchange(
                    n,
                    n - 1,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => return true,
                    Err(now) => n = now,
                }
            }
            false
        };
        if self.fail_rewrites.load(Ordering::SeqCst) || next() {
            return Err("planted: disk full".into());
        }
        assert!(
            !self.panic_rewrites.load(Ordering::SeqCst),
            "planted: the writer panics"
        );
        RealIo.rewrite(dir, name, bytes, synced)
    }
    fn append(
        &mut self,
        dir: &Path,
        name: &str,
        at: u64,
        tails: &[[u8; 5]],
    ) -> Result<bool, String> {
        self.before(Op::Append {
            at,
            tics: tails.len(),
        });
        if self.fail_appends.load(Ordering::SeqCst) {
            return Err("planted: disk full".into());
        }
        RealIo.append(dir, name, at, tails)
    }
}

/// A host that times every call the app makes to the writer's host.
struct Timing<'a> {
    host: &'a mut WriterHost,
    longest: Duration,
    calls: u64,
}

impl Timing<'_> {
    fn time<T>(&mut self, f: impl FnOnce(&mut WriterHost) -> T) -> T {
        let t0 = Instant::now();
        let r = f(self.host);
        self.longest = self.longest.max(t0.elapsed());
        self.calls += 1;
        r
    }
}

impl Host for Timing<'_> {
    fn timestamp(&mut self) -> Timestamp {
        self.host.timestamp()
    }
    fn exists(&mut self, name: &str) -> bool {
        self.time(|h| h.exists(name))
    }
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        self.time(|h| h.claim(name, bytes))
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.time(|h| h.save(name, bytes))
    }
    fn appends(&self) -> bool {
        self.host.appends()
    }
    fn append(&mut self, name: &str, at: u64, tail: &[u8; 5]) -> Result<bool, String> {
        self.time(|h| h.append(name, at, tail))
    }
}

pub(crate) fn writer(dir: &Path, io: TestIo) -> WriterHost {
    WriterHost::with_io(fs(dir), Box::new(io), Box::new(|_| {})).unwrap()
}

/// The ruling "no file write of any kind may block the tic loop", and a normal quit drains: with
/// every file operation 200 ms slow, every call the app makes to the host returns at once (well
/// under the 200 ms one operation takes), and after the quit and the drain the file is the
/// recorder's demo byte for byte, every tic in it.
#[test]
fn a_slow_disk_never_holds_up_the_app_and_a_quit_drains_everything() {
    let dir = scratch("slow");
    let ops = Arc::new(Mutex::new(Vec::new()));
    let mut host = writer(
        &dir,
        TestIo {
            delay: Duration::from_millis(200),
            ops: Arc::clone(&ops),
            ..TestIo::default()
        },
    );
    let mut t = Timing {
        host: &mut host,
        longest: Duration::ZERO,
        calls: 0,
    };
    let mut app = App::new(load(), SessionConfig::default(), &mut t);
    app.update(0, &mut t);
    for k in 1..=60 {
        assert_eq!(app.update(at(k), &mut t).ran, 1);
    }
    app.quit(&mut t);
    let (longest, calls) = (t.longest, t.calls);
    assert!(calls > 60, "{calls} calls");
    assert!(
        longest < Duration::from_millis(50),
        "a call took {longest:?} while each operation takes 200 ms"
    );
    assert!(
        (ops.lock().unwrap().len() as u64) < calls,
        "the writer is behind when the app quits"
    );
    // the window's end: drain, then the app's save on quit with the writer waiting
    host.finish();
    app.quit(&mut host);
    let report = host.close();
    let file = std::fs::read(dir.join(app.session_name())).unwrap();
    assert_eq!(
        file,
        app.recorder().bytes().unwrap(),
        "complete after the drain"
    );
    assert_eq!(tics_in(file.len()), 60);
    assert_eq!(app.save_status(), &SaveStatus::Saved(60));
    assert!(report.unreported.is_empty() && !report.panicked);
    assert_eq!(report.times.failed, 0);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Dropping the host also drains: the file is complete once the host is gone.
#[test]
fn dropping_the_host_drains_the_queue() {
    let dir = scratch("drop");
    let mut host = writer(
        &dir,
        TestIo {
            delay: Duration::from_millis(30),
            ..TestIo::default()
        },
    );
    let mut app = App::new(load(), SessionConfig::default(), &mut host);
    app.update(0, &mut host);
    for k in 1..=20 {
        app.update(at(k), &mut host);
    }
    let want = app.recorder().bytes().unwrap();
    let name = app.session_name().to_string();
    drop(host);
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), want);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The owner's bounded-queue ruling: with the disk stalled, the app plays on (every call prompt),
/// at most [`QUEUE_TICS`] tics are ever queued and at most one snapshot is pending; past the bound
/// the app's appends are declined and it hands the whole demo over, replacing the pending
/// snapshot. When the disk answers, the file catches up and after the drain is the recorder's
/// demo byte for byte (no tic dropped); the catch-up snapshot is written unsynced.
#[test]
fn a_stalled_disk_keeps_the_queue_bounded_and_loses_no_tic() {
    let dir = scratch("stall");
    let gate = Gate::closed();
    gate.set(true);
    let ops = Arc::new(Mutex::new(Vec::new()));
    let mut host = writer(
        &dir,
        TestIo {
            gate: Some(Arc::clone(&gate)),
            ops: Arc::clone(&ops),
            ..TestIo::default()
        },
    );
    let _open = gate.opens_on_drop();
    let mut app = App::new(load(), SessionConfig::default(), &mut host);
    app.update(0, &mut host);
    host.wait_idle(); // the claim is made
    gate.set(false); // the disk stalls
    let ticks = QUEUE_TICS * 3;
    let mut longest = Duration::ZERO;
    for k in 1..=ticks {
        let t0 = Instant::now();
        assert_eq!(app.update(at(k), &mut host).ran, 1);
        longest = longest.max(t0.elapsed());
        let (queued, snapshots, _, _) = host.queue_depth();
        assert!(
            queued <= QUEUE_TICS && snapshots <= 1,
            "{queued} {snapshots}"
        );
    }
    let (queued, snapshots, most_queued, most_snapshots) = host.queue_depth();
    assert_eq!((most_queued, most_snapshots), (QUEUE_TICS, 1));
    // at most one operation started before the gate closed; the rest wait
    assert!(
        queued >= QUEUE_TICS - 1 && snapshots == 1,
        "{queued} {snapshots}"
    );
    assert!(
        longest < Duration::from_millis(500),
        "an update took {longest:?} with the disk stalled"
    );
    assert_eq!(app.save_status(), &SaveStatus::Saved(ticks as usize));
    gate.set(true); // the disk answers
    host.finish();
    let report = host.close();
    let file = std::fs::read(dir.join(app.session_name())).unwrap();
    assert_eq!(
        file,
        app.recorder().bytes().unwrap(),
        "every tic, after the stall"
    );
    assert_eq!(tics_in(file.len()), ticks);
    assert!(report.counts.most_waiting >= QUEUE_TICS);
    let ops = ops.lock().unwrap();
    assert!(
        ops.contains(&Op::Rewrite {
            synced: false,
            tics: ticks
        }),
        "the catch-up snapshot, unsynced: {:?}",
        &ops[ops.len().saturating_sub(4)..]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A demo of `n` tics as the app's recorder lays it out (13 header bytes, 4 per tic, the
/// marker), with tic k's bytes `[k, k >> 8, 1, 0]`.
pub(crate) fn demo(n: u64) -> Vec<u8> {
    let mut d = vec![109, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0];
    for k in 1..=n {
        d.extend([k as u8, (k >> 8) as u8, 1, 0]);
    }
    d.push(0x80);
    d
}

/// Plays `n` tics into `host` as the app does: an append per tic (at 13 + 4(k − 1), the new
/// marker after the tic's bytes), a whole save every 1,050 tics and whenever an append is
/// declined; returns the calls' longest time.
fn drive(
    host: &mut WriterHost,
    name: &str,
    from: u64,
    n: u64,
    mut pause: impl FnMut(u64),
) -> Duration {
    let mut longest = Duration::ZERO;
    for k in from..from + n {
        let t0 = Instant::now();
        let d = demo(k);
        if k % 1_050 == 0 {
            host.save(name, &d).unwrap();
        } else {
            let tail: [u8; 5] = d[d.len() - 5..].try_into().unwrap();
            if !host.append(name, 13 + 4 * (k - 1), &tail).unwrap() {
                host.save(name, &d).unwrap();
            }
        }
        longest = longest.max(t0.elapsed());
        pause(k);
    }
    longest
}

/// Review of the atomicity argument under a backlog: across 2,200 tics (the page boundaries
/// after 1,020 and 2,044 tics, the saves at 1,050 and 2,100) with a slow disk, every operation the
/// writer makes is a claim, a whole rewrite (a temporary file and a rename) or an append whose
/// every five-byte write stays inside one page; appends of several waiting tics happen; and the
/// file ends equal to the demo.
#[test]
fn under_a_backlog_every_write_is_an_in_page_append_or_a_whole_rewrite() {
    let dir = scratch("backlog");
    let ops = Arc::new(Mutex::new(Vec::new()));
    let mut host = writer(
        &dir,
        TestIo {
            delay: Duration::from_micros(800),
            ops: Arc::clone(&ops),
            ..TestIo::default()
        },
    );
    let name = "E1M1-skill2-20261120-000000.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    // bursts of 25 tics, then a pause, so the writer meets both one tic and many
    drive(&mut host, name, 1, 2_200, |k| {
        if k % 25 == 0 {
            std::thread::sleep(Duration::from_millis(30));
        }
    });
    host.finish();
    let report = host.close();
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), demo(2_200));
    let ops = ops.lock().unwrap();
    let mut many = 0;
    for op in ops.iter() {
        if let Op::Append { at, tics } = *op {
            assert!(tics >= 1);
            many += u64::from(tics > 1);
            for i in 0..tics as u64 {
                assert!(
                    append_fits_one_page(at + 4 * i, TAIL),
                    "a write across a page: {op:?}"
                );
            }
        }
    }
    assert!(many > 0, "no append of several tics");
    // the app's saves stay synced; the page-boundary rewrites are the writer's, unsynced
    assert!(
        ops.iter()
            .any(|o| matches!(o, Op::Rewrite { synced: true, tics } if *tics >= 1_050))
    );
    assert!(
        ops.iter()
            .any(|o| matches!(o, Op::Rewrite { synced: false, .. }))
    );
    assert_eq!(report.times.failed, 0);
    assert_eq!(report.counts.claims, 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Errors travel back: a failed write is returned by the next call about the session; while it
/// stands, appends are refused and a save is queued (the retry) but answers with the failure; once
/// a retry succeeds, the next save answers `Ok` and appends resume. The writer writes nothing
/// for the session between the failure and the retry.
#[test]
fn a_failure_reaches_the_next_call_and_a_retry_clears_it() {
    let dir = scratch("errors");
    let fail_appends = Arc::new(AtomicBool::new(false));
    let fail_rewrites = Arc::new(AtomicBool::new(false));
    let ops = Arc::new(Mutex::new(Vec::new()));
    let mut host = writer(
        &dir,
        TestIo {
            ops: Arc::clone(&ops),
            fail_appends: Arc::clone(&fail_appends),
            fail_rewrites: Arc::clone(&fail_rewrites),
            ..TestIo::default()
        },
    );
    let name = "E1M1-skill2-20261120-000001.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    let tail = |k: u64| -> [u8; 5] { demo(k)[demo(k).len() - 5..].try_into().unwrap() };
    assert_eq!(host.append(name, 13, &tail(1)), Ok(true));
    host.wait_idle();
    fail_appends.store(true, Ordering::SeqCst);
    fail_rewrites.store(true, Ordering::SeqCst);
    assert_eq!(
        host.append(name, 17, &tail(2)),
        Ok(true),
        "queued before the failure"
    );
    host.wait_idle();
    let e = host.append(name, 21, &tail(3)).unwrap_err();
    assert!(e.ends_with("planted: disk full"), "{e}");
    let before = ops.lock().unwrap().len();
    // the app's retry: queued, answered with the failure while its outcome is unknown
    assert!(host.save(name, &demo(2)).is_err());
    host.wait_idle();
    assert_eq!(
        ops.lock().unwrap().len(),
        before + 1,
        "one attempt per retry"
    );
    assert!(host.save(name, &demo(2)).is_err(), "the retry failed");
    host.wait_idle();
    fail_rewrites.store(false, Ordering::SeqCst);
    fail_appends.store(false, Ordering::SeqCst);
    assert!(host.save(name, &demo(2)).is_err(), "not known yet");
    host.wait_idle();
    assert_eq!(host.save(name, &demo(2)), Ok(()), "the retry succeeded");
    assert_eq!(host.append(name, 21, &tail(3)), Ok(true));
    host.finish();
    let report = host.close();
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), demo(3));
    assert!(report.times.failed >= 3, "{:?}", report.times);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Tics queued behind a failed write are not written (not even as a whole rewrite) until the
/// app's retry hands a whole demo over; the writer's copy keeps them, so appends resume in place
/// after the retry.
#[test]
fn tics_queued_behind_a_failure_wait_for_the_retry() {
    let dir = scratch("behind");
    let fail_appends = Arc::new(AtomicBool::new(false));
    let ops = Arc::new(Mutex::new(Vec::new()));
    let gate = Gate::closed();
    gate.set(true);
    let mut host = writer(
        &dir,
        TestIo {
            gate: Some(Arc::clone(&gate)),
            ops: Arc::clone(&ops),
            fail_appends: Arc::clone(&fail_appends),
            ..TestIo::default()
        },
    );
    let _open = gate.opens_on_drop();
    let name = "E1M1-skill2-20261120-000002.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    host.wait_idle();
    fail_appends.store(true, Ordering::SeqCst);
    let tail = |k: u64| -> [u8; 5] { demo(k)[demo(k).len() - 5..].try_into().unwrap() };
    gate.set(false);
    assert_eq!(host.append(name, 13, &tail(1)), Ok(true));
    // the writer is held inside the first append when the second tic is queued (a gate, not a
    // sleep: a slow machine once let the first append fail before the second was queued)
    gate.await_waiter();
    assert_eq!(host.append(name, 17, &tail(2)), Ok(true));
    gate.set(true);
    host.wait_idle();
    assert_eq!(
        *ops.lock().unwrap(),
        [Op::Claim, Op::Append { at: 13, tics: 1 }],
        "nothing written for the tic queued behind the failure"
    );
    assert!(host.append(name, 21, &tail(3)).is_err());
    fail_appends.store(false, Ordering::SeqCst);
    assert!(
        host.save(name, &demo(2)).is_err(),
        "the retry, its outcome unknown"
    );
    host.wait_idle();
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), demo(2));
    assert_eq!(host.save(name, &demo(2)), Ok(()));
    assert_eq!(host.append(name, 21, &tail(3)), Ok(true));
    host.finish();
    host.close();
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), demo(3));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Ruling 4 kept: under a persistent failure the real app's retries reach the disk at its own
/// cadence (the next tic, then once every 35 tics), not every other tic.
#[test]
fn the_apps_retry_cadence_reaches_the_disk_unchanged() {
    let dir = scratch("cadence");
    let fail = Arc::new(AtomicBool::new(false));
    let ops = Arc::new(Mutex::new(Vec::new()));
    let mut host = writer(
        &dir,
        TestIo {
            ops: Arc::clone(&ops),
            fail_appends: Arc::clone(&fail),
            fail_rewrites: Arc::clone(&fail),
            ..TestIo::default()
        },
    );
    let mut app = App::new(load(), SessionConfig::default(), &mut host);
    app.update(0, &mut host);
    host.wait_idle();
    fail.store(true, Ordering::SeqCst);
    for k in 1..=110 {
        app.update(at(k), &mut host);
        host.wait_idle(); // the writer keeps up: the cadence is the app's
    }
    assert!(matches!(app.save_status(), SaveStatus::Failed(_)));
    let ops = ops.lock().unwrap();
    let rewrites = ops
        .iter()
        .filter(|o| matches!(o, Op::Rewrite { .. }))
        .count();
    // the failed append at tic 1, then retries at tics 3, 38, 73 and 108
    assert_eq!(rewrites, 4, "{ops:?}");
    drop(ops);
    drop(host);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A claim that loses the race: another process creates the name after the app was answered
/// and before the writer's claim. The writer never replaces or writes that file; the session's
/// later calls fail visibly.
#[test]
fn a_claim_that_loses_the_race_never_touches_the_other_file() {
    let dir = scratch("race");
    let mut host = writer(
        &dir,
        TestIo {
            race: true,
            ..TestIo::default()
        },
    );
    let mut app = App::new(load(), SessionConfig::default(), &mut host);
    app.update(0, &mut host);
    for k in 1..=80 {
        app.update(at(k), &mut host);
        host.wait_idle();
    }
    let name = app.session_name().to_string();
    match app.save_status() {
        SaveStatus::Failed(e) => assert!(e.contains("another file took the name"), "{e}"),
        s => panic!("{s:?}"),
    }
    host.finish();
    app.quit(&mut host);
    let report = host.close();
    assert_eq!(
        std::fs::read(dir.join(&name)).unwrap(),
        b"another process's file",
        "never replaced"
    );
    assert!(matches!(app.save_status(), SaveStatus::Failed(_)));
    assert_eq!(report.counts.claims, 1);
    assert_eq!(
        report.counts.appends + report.counts.synced_rewrites + report.counts.rewrites,
        0
    );
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, [name], "no temporary file is left");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A restart within the same second: the second session's name is the first's, handed over but
/// perhaps not created yet; the host answers that it exists, so the app takes the `-2` name.
#[test]
fn a_name_handed_over_is_taken_even_before_its_file_exists() {
    let dir = scratch("reserved");
    let gate = Gate::closed();
    let mut host = writer(
        &dir,
        TestIo {
            gate: Some(Arc::clone(&gate)),
            ..TestIo::default()
        },
    );
    let _open = gate.opens_on_drop();
    assert_eq!(host.claim("a.lmp", &demo(0)), Ok(true));
    assert!(!dir.join("a.lmp").exists());
    assert!(host.exists("a.lmp"));
    assert_eq!(host.claim("a.lmp", &demo(0)), Ok(false));
    gate.set(true);
    host.finish();
    host.close();
    assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), demo(0));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Ruling 5: a claim that takes the fallback prints exactly one line, the fallback's.
#[test]
fn the_fallback_claim_prints_one_line() {
    let dir = scratch("fallback");
    let lines = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&lines);
    let mut host = WriterHost::with_io(
        fs(&dir),
        Box::new(TestIo {
            fallback: true,
            ..TestIo::default()
        }),
        Box::new(move |l| seen.lock().unwrap().push(l)),
    )
    .unwrap();
    assert_eq!(host.claim("a.lmp", &demo(0)), Ok(true));
    host.finish();
    host.close();
    assert_eq!(
        *lines.lock().unwrap(),
        [session_file::fallback_line(&dir.join("a.lmp"))]
    );
    assert!(lines.lock().unwrap()[0].contains("refused a hard link"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 3, P25: after `finish` (the run's end) the host is synchronous: every call waits for its
/// write and returns its real outcome, so the app's save on quit reports the truth, and the file
/// holds the write when the call returns (before `close`).
#[test]
fn after_finish_every_call_waits_and_returns_its_real_outcome() {
    let dir = scratch("sync");
    let fail = Arc::new(AtomicBool::new(false));
    let mut host = writer(
        &dir,
        TestIo {
            delay: Duration::from_millis(20),
            fail_rewrites: Arc::clone(&fail),
            ..TestIo::default()
        },
    );
    let name = "E1M1-skill2-20261120-000003.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    let tail: [u8; 5] = demo(1)[demo(1).len() - 5..].try_into().unwrap();
    assert_eq!(host.append(name, 13, &tail), Ok(true));
    host.finish();
    assert_eq!(host.queue_depth().0, 0, "drained");
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), demo(1));
    fail.store(true, Ordering::SeqCst);
    assert_eq!(
        host.save(name, &demo(2)),
        Err("planted: disk full".to_string())
    );
    fail.store(false, Ordering::SeqCst);
    assert_eq!(host.save(name, &demo(2)), Ok(()));
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), demo(2), "on return");
    host.close();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 3: a writer thread that dies (a panic) before making a write handed over never lets a
/// waiting call or a settle report it written: both report that the writer stopped.
#[test]
fn a_writer_that_died_is_never_reported_as_having_written() {
    let dir = scratch("died");
    let panic = Arc::new(AtomicBool::new(false));
    let mut host = writer(
        &dir,
        TestIo {
            panic_rewrites: Arc::clone(&panic),
            ..TestIo::default()
        },
    );
    let name = "E1M1-skill2-20261120-000004.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    host.finish();
    panic.store(true, Ordering::SeqCst);
    assert_eq!(host.save(name, &demo(1)), Err(STOPPED.to_string()));
    assert_eq!(
        host.settle(name, Duration::ZERO),
        Settled::Failed(STOPPED.to_string())
    );
    assert!(host.close().panicked);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 3, P8: a failure of the current session that the app never heard of (no call came
/// after it and nothing settled it) is in the closed writer's report; once settled (reported to
/// the app) it is not reported twice.
#[test]
fn a_failure_the_app_never_heard_of_is_in_the_report() {
    for settled in [false, true] {
        let dir = scratch(&format!("unheard-{settled}"));
        let fail = Arc::new(AtomicBool::new(false));
        let mut host = writer(
            &dir,
            TestIo {
                fail_appends: Arc::clone(&fail),
                ..TestIo::default()
            },
        );
        let name = "E1M1-skill2-20261120-000005.lmp";
        assert_eq!(host.claim(name, &demo(0)), Ok(true));
        host.wait_idle();
        fail.store(true, Ordering::SeqCst);
        let tail: [u8; 5] = demo(1)[demo(1).len() - 5..].try_into().unwrap();
        assert_eq!(host.append(name, 13, &tail), Ok(true));
        host.wait_idle();
        if settled {
            assert_eq!(
                host.settle(name, Duration::ZERO),
                Settled::Failed("planted: disk full".into())
            );
        }
        let report = host.close();
        let want = if settled {
            vec![]
        } else {
            vec![(name.to_string(), "planted: disk full".to_string())]
        };
        assert_eq!(report.unreported, want);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// A writer made directly, with its shared state, for the branches the host cannot reach.
fn bare(dir: &Path, ops: &Arc<Mutex<Vec<Op>>>, model: Vec<u8>) -> (Writer, Arc<Shared>) {
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            alive: true,
            ..State::default()
        }),
        changed: Condvar::new(),
    });
    let w = Writer {
        dir: dir.to_path_buf(),
        io: Box::new(TestIo {
            ops: Arc::clone(ops),
            ..TestIo::default()
        }),
        notify: Box::new(|_| {}),
        shared: Arc::clone(&shared),
        session: Some(Session {
            name: "a.lmp".into(),
            model,
            disk: Disk::Current,
            retried: false,
        }),
        left: Vec::new(),
    };
    (w, shared)
}

/// Review 3, P21 (and P7's successor): a whole demo handed over for a session that is not the one
/// being written (unreachable through the host: snapshots are taken in order) is never written
/// and never dropped silently: its failure goes to the report.
#[test]
fn a_demo_of_another_session_is_not_written_and_is_reported() {
    let dir = scratch("stray");
    let ops = Arc::new(Mutex::new(Vec::new()));
    let (mut w, shared) = bare(&dir, &ops, demo(1));
    shared.lock().snapshots.push_back(Snapshot {
        name: "b.lmp".into(),
        bytes: demo(2),
        synced: true,
    });
    w.process(vec![Msg::Snapshot]);
    assert_eq!(*ops.lock().unwrap(), []);
    let st = shared.lock();
    assert_eq!(
        st.strays,
        [(
            "b.lmp".to_string(),
            "b.lmp: not the session being written".to_string()
        )]
    );
    assert!(
        st.failure.is_none(),
        "the session being written is unaffected"
    );
    drop(st);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 3, P24: a tic that does not follow the writer's copy (a gap) marks the file stale, so a
/// later tic of the same batch that does follow the copy is not appended to a file the writer no
/// longer vouches for; the failure is recorded for the app.
#[test]
fn a_tic_out_of_step_marks_the_file_stale() {
    let dir = scratch("gap");
    let ops = Arc::new(Mutex::new(Vec::new()));
    let (mut w, shared) = bare(&dir, &ops, demo(1));
    let tail = |k: u64| -> [u8; 5] { demo(k)[demo(k).len() - 5..].try_into().unwrap() };
    w.process(vec![
        Msg::Append {
            at: 21,
            tail: tail(3),
        },
        Msg::Append {
            at: 17,
            tail: tail(2),
        },
    ]);
    assert_eq!(*ops.lock().unwrap(), [], "nothing appended after the gap");
    assert_eq!(w.session.as_ref().map(|s| s.disk), Some(Disk::Stale));
    let failure = shared.lock().failure.as_ref().map(|f| f.error.clone());
    assert_eq!(
        failure.as_deref(),
        Some("a.lmp: a tic at byte 21, but the writer's copy holds 18 bytes")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 3, P23: a pending synced snapshot (the app's own save) replaced in place by a catch-up
/// snapshot (after a decline at the bound, unsynced) stays synced: the one rewrite made is
/// flushed.
#[test]
fn a_synced_snapshot_replaced_in_place_stays_synced() {
    let dir = scratch("synced");
    let gate = Gate::closed();
    let ops = Arc::new(Mutex::new(Vec::new()));
    let mut host = writer(
        &dir,
        TestIo {
            gate: Some(Arc::clone(&gate)),
            ops: Arc::clone(&ops),
            ..TestIo::default()
        },
    );
    let _open = gate.opens_on_drop();
    let name = "E1M1-skill2-20261120-000006.lmp";
    // the claim's file operation waits at the gate, so both saves are pending behind it
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    assert_eq!(host.save(name, &demo(1)), Ok(()));
    host.declined = true; // as after an append declined at the bound
    assert_eq!(host.save(name, &demo(1)), Ok(()));
    assert_eq!(host.queue_depth().1, 1, "replaced in place");
    gate.set(true);
    host.finish();
    host.close();
    assert_eq!(
        *ops.lock().unwrap(),
        [
            Op::Claim,
            Op::Rewrite {
                synced: true,
                tics: 1
            }
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The tail (five bytes) of tic `k` of [`demo`], as the app appends it.
fn tail_of(k: u64) -> [u8; 5] {
    let d = demo(k);
    d[d.len() - 5..].try_into().unwrap()
}

/// The owner's ruling on a writer-thread panic: the closed writer's report names every session
/// whose file does not hold its whole demo, with what its file holds, and no other. Session A is
/// written whole (3 tics). B's last append fails and the rewrite when the writer moves on to C
/// fails too, so B is left behind holding 1 of its 2 tics (rewritten at exit only by a writer
/// that did not die). C holds 2 tics; then the app hands over a whole demo of 5 tics, whose
/// rewrite panics, while the writer is held at the gate inside it the app queues tics 6 and 7
/// and claims D. So C's file holds 2 of its 7 tics and D's claim never completed (no file).
/// The exit lines ([`crate::closing::panic_lines`]) say so, by path, with D as the current
/// session, and say nothing about A.
#[test]
fn a_writer_panic_names_every_session_whose_file_is_short() {
    let dir = scratch("panic-names");
    let gate = Gate::closed();
    gate.set(true);
    let (fail_appends, fail_rewrites, panic) = (
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let mut host = writer(
        &dir,
        TestIo {
            gate: Some(Arc::clone(&gate)),
            fail_appends: Arc::clone(&fail_appends),
            fail_rewrites: Arc::clone(&fail_rewrites),
            panic_rewrites: Arc::clone(&panic),
            ..TestIo::default()
        },
    );
    let _open = gate.opens_on_drop();
    let (a, b, c, d) = ("a.lmp", "b.lmp", "c.lmp", "d.lmp");
    assert_eq!(host.claim(a, &demo(0)), Ok(true));
    for k in 1..=3 {
        assert_eq!(host.append(a, at_tic(k), &tail_of(k)), Ok(true));
    }
    host.wait_idle();
    assert_eq!(host.claim(b, &demo(0)), Ok(true));
    assert_eq!(host.append(b, at_tic(1), &tail_of(1)), Ok(true));
    host.wait_idle();
    fail_appends.store(true, Ordering::SeqCst);
    fail_rewrites.store(true, Ordering::SeqCst);
    assert_eq!(host.append(b, at_tic(2), &tail_of(2)), Ok(true));
    host.wait_idle();
    assert_eq!(host.claim(c, &demo(0)), Ok(true)); // B's rewrite fails: left behind
    host.wait_idle();
    fail_appends.store(false, Ordering::SeqCst);
    fail_rewrites.store(false, Ordering::SeqCst);
    for k in 1..=2 {
        assert_eq!(host.append(c, at_tic(k), &tail_of(k)), Ok(true));
    }
    host.wait_idle();
    assert_eq!(host.written(c), Some(2));
    gate.set(false);
    panic.store(true, Ordering::SeqCst);
    assert_eq!(host.save(c, &demo(5)), Ok(()));
    gate.await_waiter(); // the writer is held inside C's rewrite, which will panic
    for k in 6..=7 {
        assert_eq!(host.append(c, at_tic(k), &tail_of(k)), Ok(true));
    }
    assert_eq!(host.claim(d, &demo(0)), Ok(true));
    gate.set(true);
    host.wait_idle();
    let report = host.close();
    assert!(report.panicked);
    assert!(!report.refused);
    let short = |name: &str, recorded, file| NotWritten {
        name: name.to_string(),
        recorded,
        file,
    };
    assert_eq!(
        report.not_written,
        [
            short(b, 2, Some(OnDisk::Holds(1))),
            short(c, 7, Some(OnDisk::Holds(2))),
            short(d, 0, None),
        ]
    );
    assert_eq!(report.whole, [(a.to_string(), 3)]);
    // what the report says the files hold is what they hold
    assert_eq!(std::fs::read(dir.join(a)).unwrap(), demo(3));
    assert_eq!(std::fs::read(dir.join(b)).unwrap(), demo(1));
    assert_eq!(std::fs::read(dir.join(c)).unwrap(), demo(2));
    assert!(!dir.join(d).exists());
    let path = |name: &str| dir.join(name).display().to_string();
    assert_eq!(
        crate::closing::panic_lines(&report, &dir, d),
        [
            "the session writer stopped unexpectedly (a panic); the sessions below were not \
             saved, and no rewrite at exit saved them"
                .to_string(),
            format!(
                "an earlier session was not saved: the session writer panicked; {} holds at \
                 least 1 of its 2 tics (its last completed write)",
                path(b)
            ),
            format!(
                "an earlier session was not saved: the session writer panicked; {} holds at \
                 least 2 of its 7 tics (its last completed write)",
                path(c)
            ),
            format!(
                "the session was not saved: the session writer panicked; {} was not created (no \
                 write of it completed)",
                path(d)
            ),
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The byte offset of tic `k`'s append (13 + 4(k − 1)).
fn at_tic(k: u64) -> u64 {
    13 + 4 * (k - 1)
}

/// The owner's ruling, the other side: a panic that lost nothing (the rewrite that panics would
/// have written the 3 tics the file already holds) names no session as not saved, and the exit
/// still says the writer stopped. A claim after the panic is refused (no file), and only then
/// does the report say a session begun after the stop has no file.
#[test]
fn a_writer_panic_that_lost_nothing_names_no_session() {
    for claim_after in [false, true] {
        let dir = scratch(&format!("panic-nothing-{claim_after}"));
        let panic = Arc::new(AtomicBool::new(false));
        let mut host = writer(
            &dir,
            TestIo {
                panic_rewrites: Arc::clone(&panic),
                ..TestIo::default()
            },
        );
        let a = "a.lmp";
        assert_eq!(host.claim(a, &demo(0)), Ok(true));
        for k in 1..=3 {
            assert_eq!(host.append(a, at_tic(k), &tail_of(k)), Ok(true));
        }
        host.wait_idle();
        panic.store(true, Ordering::SeqCst);
        assert_eq!(host.save(a, &demo(3)), Ok(()));
        host.wait_idle(); // returns once the writer has died
        if claim_after {
            assert!(host.claim("b.lmp", &demo(0)).is_err());
        }
        let report = host.close();
        assert!(report.panicked);
        assert_eq!(report.not_written, []);
        assert_eq!(report.whole, [(a.to_string(), 3)]);
        assert_eq!(report.refused, claim_after);
        assert_eq!(std::fs::read(dir.join(a)).unwrap(), demo(3));
        let lines = crate::closing::panic_lines(&report, &dir, a);
        if claim_after {
            assert_eq!(
                lines,
                [
                    "the session writer stopped unexpectedly (a panic); the sessions below were \
                     not saved, and no rewrite at exit saved them",
                    "no session begun after the session writer stopped was saved: no file was \
                     created for them",
                ]
            );
        } else {
            assert_eq!(
                lines,
                [
                    "the session writer stopped unexpectedly (a panic); every session file it was \
                  handed holds its whole demo"
                ]
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// A panic in the rewrite the writer makes when it moves on to the next session: B's last append
/// failed and no call about B came after it (the app never heard of it), so the failure is in the
/// report as unreported; C's claim was handed over and never made. Each session gets one line
/// at exit: B's says what its file holds (no second "not fully written" line about it), C's
/// that its file was not created.
#[test]
fn a_writer_panic_while_moving_on_names_each_session_once() {
    let dir = scratch("panic-moving-on");
    let (fail_appends, panic) = (
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let mut host = writer(
        &dir,
        TestIo {
            fail_appends: Arc::clone(&fail_appends),
            panic_rewrites: Arc::clone(&panic),
            ..TestIo::default()
        },
    );
    let (b, c) = ("b.lmp", "c.lmp");
    assert_eq!(host.claim(b, &demo(0)), Ok(true));
    assert_eq!(host.append(b, at_tic(1), &tail_of(1)), Ok(true));
    host.wait_idle();
    fail_appends.store(true, Ordering::SeqCst);
    assert_eq!(host.append(b, at_tic(2), &tail_of(2)), Ok(true));
    host.wait_idle();
    fail_appends.store(false, Ordering::SeqCst);
    panic.store(true, Ordering::SeqCst);
    assert_eq!(host.claim(c, &demo(0)), Ok(true)); // B's rewrite panics
    host.wait_idle();
    let report = host.close();
    assert_eq!(
        report.unreported,
        [(b.to_string(), "planted: disk full".to_string())]
    );
    let path = |name: &str| dir.join(name).display().to_string();
    let mut lines = crate::closing::writer_lines(&report, None, false);
    lines.extend(crate::closing::panic_lines(&report, &dir, c));
    lines.retain(|l| !l.starts_with("session file writes: "));
    assert_eq!(
        lines,
        [
            "the session writer stopped unexpectedly (a panic); the sessions below were not \
             saved, and no rewrite at exit saved them"
                .to_string(),
            format!(
                "an earlier session was not saved: the session writer panicked; {} holds at \
                 least 1 of its 2 tics (its last completed write)",
                path(b)
            ),
            format!(
                "the session was not saved: the session writer panicked; {} was not created (no \
                 write of it completed)",
                path(c)
            ),
        ]
    );
    assert_eq!(std::fs::read(dir.join(b)).unwrap(), demo(1));
    assert!(!dir.join(c).exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Faults aimed at one name or one kind of batch, set by the test while the writer runs (review
/// 6, m2, m3 and n1), on top of [`TestIo`].
#[derive(Default)]
struct Aims {
    /// A claim of this name finds another process's file there (the name was taken).
    take: Option<String>,
    /// A claim of this name fails.
    fail_claims_of: Option<String>,
    /// A whole rewrite of this name panics (the writer thread dies).
    panic_rewrites_of: Option<String>,
    /// An append of more than one tic makes its first write, then fails.
    fail_after_first: bool,
}

/// [`TestIo`] with the faults of [`Aims`].
struct AimedIo {
    io: TestIo,
    aims: Arc<Mutex<Aims>>,
}

impl SessionIo for AimedIo {
    fn claim(&mut self, dir: &Path, name: &str, bytes: &[u8]) -> Result<Claim, String> {
        let (take, fail) = {
            let a = self.aims.lock().unwrap();
            (
                a.take.as_deref() == Some(name),
                a.fail_claims_of.as_deref() == Some(name),
            )
        };
        if fail {
            return Err("planted: the claim failed".into());
        }
        if take {
            std::fs::write(dir.join(name), b"another process's file").unwrap();
        }
        self.io.claim(dir, name, bytes)
    }
    fn rewrite(
        &mut self,
        dir: &Path,
        name: &str,
        bytes: &[u8],
        synced: bool,
    ) -> Result<(), String> {
        let panic = self.aims.lock().unwrap().panic_rewrites_of.as_deref() == Some(name);
        assert!(
            !panic,
            "planted: the writer panics in the rewrite of {name}"
        );
        self.io.rewrite(dir, name, bytes, synced)
    }
    fn append(
        &mut self,
        dir: &Path,
        name: &str,
        at: u64,
        tails: &[[u8; 5]],
    ) -> Result<bool, String> {
        let partly = self.aims.lock().unwrap().fail_after_first && tails.len() > 1;
        if partly {
            assert_eq!(self.io.append(dir, name, at, &tails[..1]), Ok(true));
            return Err("planted: disk full after the first tic".into());
        }
        self.io.append(dir, name, at, tails)
    }
}

/// A writer over [`AimedIo`].
fn aimed_writer(dir: &Path, io: TestIo, aims: &Arc<Mutex<Aims>>) -> WriterHost {
    let io = AimedIo {
        io,
        aims: Arc::clone(aims),
    };
    WriterHost::with_io(fs(dir), Box::new(io), Box::new(|_| {})).unwrap()
}

/// The exit lines from a closed writer's report after a panic (the writers' lines, then the
/// panic's), without the counters line (its times vary).
fn panic_exit_lines(report: &Report, dir: &Path, current: &str) -> Vec<String> {
    let mut lines = crate::closing::writer_lines(report, None, false);
    lines.extend(crate::closing::panic_lines(report, dir, current));
    lines.retain(|l| !l.starts_with("session file writes: "));
    lines
}

/// The heading of the panic's lines when a session is named.
const NAMED: &str = "the session writer stopped unexpectedly (a panic); the sessions below were \
                     not saved, and no rewrite at exit saved them";

/// Review 6, m1: after a panic, a failure the app never heard of, of a session whose file holds
/// every tic recorded, gets no "not fully written" line. A holds tic 1; a whole save of that same
/// tic fails and no call about A comes after it; the writer then panics in A's rewrite when it
/// moves on to B, whose claim is never made. The report keeps A's failure as unreported and A as
/// whole; the exit names B only.
#[test]
fn after_a_panic_a_whole_session_with_an_unheard_failure_gets_no_line() {
    let dir = scratch("panic-whole-unheard");
    let (fail_rewrites, panic) = (
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let mut host = writer(
        &dir,
        TestIo {
            fail_rewrites: Arc::clone(&fail_rewrites),
            panic_rewrites: Arc::clone(&panic),
            ..TestIo::default()
        },
    );
    let (a, b) = ("a.lmp", "b.lmp");
    assert_eq!(host.claim(a, &demo(0)), Ok(true));
    assert_eq!(host.append(a, at_tic(1), &tail_of(1)), Ok(true));
    host.wait_idle();
    fail_rewrites.store(true, Ordering::SeqCst);
    assert_eq!(host.save(a, &demo(1)), Ok(()));
    host.wait_idle();
    fail_rewrites.store(false, Ordering::SeqCst);
    panic.store(true, Ordering::SeqCst);
    assert_eq!(host.claim(b, &demo(0)), Ok(true)); // A's rewrite panics
    host.wait_idle();
    let report = host.close();
    assert!(report.panicked);
    assert_eq!(
        report.unreported,
        [(a.to_string(), "planted: disk full".to_string())]
    );
    assert_eq!(report.whole, [(a.to_string(), 1)]);
    assert_eq!(std::fs::read(dir.join(a)).unwrap(), demo(1));
    assert_eq!(
        panic_exit_lines(&report, &dir, b),
        [
            NAMED.to_string(),
            format!(
                "the session was not saved: the session writer panicked; {} was not created (no \
                 write of it completed)",
                dir.join(b).display()
            ),
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 6, m2: the tics a short file's line gives are a lower bound. A holds 2 tics; tics 3 and
/// 4 go over in one append, whose first write is made and whose second fails, so the file holds
/// 3 tics; then the rewrite of A's whole demo panics. The line says A's file holds at least 2 of
/// its 4 tics, and it holds 3.
#[test]
fn after_a_panic_a_partly_failed_append_leaves_at_least_the_tics_said() {
    let dir = scratch("panic-partly");
    let gate = Gate::closed();
    gate.set(true);
    let aims = Arc::new(Mutex::new(Aims::default()));
    let mut host = aimed_writer(
        &dir,
        TestIo {
            gate: Some(Arc::clone(&gate)),
            ..TestIo::default()
        },
        &aims,
    );
    let _open = gate.opens_on_drop();
    let a = "a.lmp";
    assert_eq!(host.claim(a, &demo(0)), Ok(true));
    assert_eq!(host.append(a, at_tic(1), &tail_of(1)), Ok(true));
    host.wait_idle();
    gate.set(false);
    assert_eq!(host.append(a, at_tic(2), &tail_of(2)), Ok(true));
    gate.await_waiter(); // the writer is held in tic 2's append: tics 3 and 4 make one batch
    for k in 3..=4 {
        assert_eq!(host.append(a, at_tic(k), &tail_of(k)), Ok(true));
    }
    aims.lock().unwrap().fail_after_first = true;
    gate.set(true);
    host.wait_idle();
    assert_eq!(host.written(a), Some(2));
    assert_eq!(std::fs::read(dir.join(a)).unwrap(), demo(3));
    aims.lock().unwrap().panic_rewrites_of = Some(a.to_string());
    assert!(host.save(a, &demo(4)).is_err()); // the append's failure, now heard; its retry panics
    host.wait_idle();
    let report = host.close();
    assert!(report.panicked);
    assert_eq!(
        report.not_written,
        [NotWritten {
            name: a.to_string(),
            recorded: 4,
            file: Some(OnDisk::Holds(2)),
        }]
    );
    assert_eq!(std::fs::read(dir.join(a)).unwrap(), demo(3));
    assert_eq!(
        crate::closing::panic_lines(&report, &dir, a),
        [
            NAMED.to_string(),
            format!(
                "the session was not saved: the session writer panicked; {} holds at least 2 of \
                 its 4 tics (its last completed write)",
                dir.join(a).display()
            ),
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 6, m3 (the claim's note that another file took the name): T's claim finds another
/// process's file under its name, so T is never written; the writer moves on to B and panics in
/// the rewrite of B's whole demo. After the panic T is named as not written, with the same words
/// as without a panic (the other file is there; it is not "not created"), and B as short.
#[test]
fn after_a_panic_a_taken_name_is_named_as_taken() {
    let dir = scratch("panic-taken");
    let aims = Arc::new(Mutex::new(Aims::default()));
    let mut host = aimed_writer(&dir, TestIo::default(), &aims);
    let (t, b) = ("t.lmp", "b.lmp");
    aims.lock().unwrap().take = Some(t.to_string());
    assert_eq!(host.claim(t, &demo(0)), Ok(true));
    host.wait_idle();
    assert_eq!(host.claim(b, &demo(0)), Ok(true));
    assert_eq!(host.append(b, at_tic(1), &tail_of(1)), Ok(true));
    host.wait_idle();
    aims.lock().unwrap().panic_rewrites_of = Some(b.to_string());
    assert_eq!(host.save(b, &demo(2)), Ok(()));
    host.wait_idle();
    let report = host.close();
    assert!(report.panicked);
    assert_eq!(
        std::fs::read(dir.join(t)).unwrap(),
        b"another process's file",
        "left alone"
    );
    assert_eq!(
        panic_exit_lines(&report, &dir, b),
        [
            NAMED.to_string(),
            format!("an earlier session was not written: {}", lost(&dir, t)),
            format!(
                "the session was not saved: the session writer panicked; {} holds at least 1 of \
                 its 2 tics (its last completed write)",
                dir.join(b).display()
            ),
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 6, m3 (the notes of the rewrite at exit): a panic in the rewrite at exit, after the
/// rewrite has dealt with two earlier sessions. T's claims failed, so T was left behind with no
/// file; at exit its claim finds another process's file under its name. A's and C's last appends
/// and their rewrites when the writer moved on failed; at exit A is rewritten whole, and the
/// rewrite of C panics. So T is named as taken, A (now whole) is not named, and C is named with
/// what its file holds; D, the current session, is whole.
#[test]
fn a_panic_in_the_rewrite_at_exit_names_what_the_rewrites_before_it_left() {
    let dir = scratch("panic-at-exit");
    let (fail_appends, fail_rewrites) = (
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let aims = Arc::new(Mutex::new(Aims::default()));
    let mut host = aimed_writer(
        &dir,
        TestIo {
            fail_appends: Arc::clone(&fail_appends),
            fail_rewrites: Arc::clone(&fail_rewrites),
            ..TestIo::default()
        },
        &aims,
    );
    let (t, a, c, d) = ("t.lmp", "a.lmp", "c.lmp", "d.lmp");
    aims.lock().unwrap().fail_claims_of = Some(t.to_string());
    assert_eq!(host.claim(t, &demo(0)), Ok(true));
    host.wait_idle();
    // S holds tic 1 and its append of tic 2 fails
    let short = |host: &mut WriterHost, s: &str| {
        assert_eq!(host.append(s, at_tic(1), &tail_of(1)), Ok(true));
        host.wait_idle();
        fail_appends.store(true, Ordering::SeqCst);
        assert_eq!(host.append(s, at_tic(2), &tail_of(2)), Ok(true));
        host.wait_idle();
        fail_appends.store(false, Ordering::SeqCst);
    };
    // the claim of T fails again when the writer moves on to A: T is left behind with no file
    assert_eq!(host.claim(a, &demo(0)), Ok(true));
    short(&mut host, a);
    // the rewrites of A and C when the writer moves on fail: each is left behind holding 1 of
    // its 2 tics
    fail_rewrites.store(true, Ordering::SeqCst);
    assert_eq!(host.claim(c, &demo(0)), Ok(true));
    short(&mut host, c);
    assert_eq!(host.claim(d, &demo(0)), Ok(true));
    host.wait_idle();
    fail_rewrites.store(false, Ordering::SeqCst);
    assert_eq!(host.append(d, at_tic(1), &tail_of(1)), Ok(true));
    host.wait_idle();
    {
        let mut aims = aims.lock().unwrap();
        aims.fail_claims_of = None;
        aims.take = Some(t.to_string());
        aims.panic_rewrites_of = Some(c.to_string());
    }
    host.finish();
    let report = host.close();
    assert!(report.panicked);
    assert_eq!(report.unreported, []);
    assert_eq!(
        report.whole,
        [(a.to_string(), 2), (d.to_string(), 1)],
        "A was rewritten at exit"
    );
    assert_eq!(
        std::fs::read(dir.join(t)).unwrap(),
        b"another process's file"
    );
    assert_eq!(std::fs::read(dir.join(a)).unwrap(), demo(2));
    assert_eq!(std::fs::read(dir.join(c)).unwrap(), demo(1));
    assert_eq!(std::fs::read(dir.join(d)).unwrap(), demo(1));
    assert_eq!(
        panic_exit_lines(&report, &dir, d),
        [
            NAMED.to_string(),
            format!("an earlier session was not written: {}", lost(&dir, t)),
            format!(
                "an earlier session was not saved: the session writer panicked; {} holds at \
                 least 1 of its 2 tics (its last completed write)",
                dir.join(c).display()
            ),
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 6, n1: a claim handed over after [`WriterHost::finish`] whose outcome is an error (the
/// writer dies in the rewrite of A when it moves on to B, before B's claim is made) is in the
/// report as the failed claim, so the exit lines can treat B, which the app keeps no name for,
/// as the current session. A later claim asked for clears it, and without a panic it is not in
/// the report.
#[test]
fn a_claim_that_fails_after_finish_is_the_failed_claim() {
    for panics in [true, false] {
        let dir = scratch(&format!("failed-claim-{panics}"));
        let (fail_appends, fail_rewrites, panic) = (
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        );
        let mut host = writer(
            &dir,
            TestIo {
                fail_appends: Arc::clone(&fail_appends),
                fail_rewrites: Arc::clone(&fail_rewrites),
                panic_rewrites: Arc::clone(&panic),
                ..TestIo::default()
            },
        );
        let (a, b) = ("a.lmp", "b.lmp");
        assert_eq!(host.claim(a, &demo(0)), Ok(true));
        assert_eq!(host.append(a, at_tic(1), &tail_of(1)), Ok(true));
        host.wait_idle();
        fail_appends.store(true, Ordering::SeqCst);
        assert_eq!(host.append(a, at_tic(2), &tail_of(2)), Ok(true));
        host.finish();
        // A's rewrite when the writer moves on to B panics, or fails (then B's claim is made)
        panic.store(panics, Ordering::SeqCst);
        fail_rewrites.store(!panics, Ordering::SeqCst);
        let claimed = host.claim(b, &demo(0));
        assert_eq!(claimed.is_err(), panics, "{claimed:?}");
        let report = host.close();
        assert_eq!(report.panicked, panics);
        if panics {
            assert_eq!(claimed, Err(STOPPED.to_string()));
            assert_eq!(report.failed_claim.as_deref(), Some(b));
            assert_eq!(
                report.not_written,
                [
                    NotWritten {
                        name: a.to_string(),
                        recorded: 2,
                        file: Some(OnDisk::Holds(1)),
                    },
                    NotWritten {
                        name: b.to_string(),
                        recorded: 0,
                        file: None,
                    },
                ]
            );
        } else {
            assert_eq!(report.failed_claim, None);
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
    // a later claim asked for (here one that finds the name handed over already) clears it
    let dir = scratch("failed-claim-cleared");
    let panic = Arc::new(AtomicBool::new(false));
    let fail_appends = Arc::new(AtomicBool::new(false));
    let mut host = writer(
        &dir,
        TestIo {
            fail_appends: Arc::clone(&fail_appends),
            panic_rewrites: Arc::clone(&panic),
            ..TestIo::default()
        },
    );
    assert_eq!(host.claim("a.lmp", &demo(0)), Ok(true));
    assert_eq!(host.append("a.lmp", at_tic(1), &tail_of(1)), Ok(true));
    host.wait_idle();
    fail_appends.store(true, Ordering::SeqCst);
    assert_eq!(host.append("a.lmp", at_tic(2), &tail_of(2)), Ok(true));
    host.finish();
    panic.store(true, Ordering::SeqCst);
    assert!(host.claim("b.lmp", &demo(0)).is_err());
    assert_eq!(host.claim("b.lmp", &demo(0)), Ok(false));
    assert_eq!(host.close().failed_claim, None);
    std::fs::remove_dir_all(&dir).unwrap();
    // a claim after finish that fails on the disk with no panic: not in the report
    let dir = scratch("failed-claim-no-panic");
    let aims = Arc::new(Mutex::new(Aims::default()));
    let mut host = aimed_writer(&dir, TestIo::default(), &aims);
    host.finish();
    aims.lock().unwrap().fail_claims_of = Some("a.lmp".into());
    assert_eq!(
        host.claim("a.lmp", &demo(0)),
        Err("planted: the claim failed".to_string())
    );
    let report = host.close();
    assert!(!report.panicked);
    assert_eq!(report.failed_claim, None);
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---- M5d mutation-survivor tests (mut6): each pins the behaviour in its comment ----

/// `take_append`: "skipped when a later snapshot already holds it". Tic 2 is offered after a whole
/// demo of 2 tics, so the file stays that demo and no failure is recorded. Pins `tic_at` (the
/// tic count after the append whose five bytes go at 13 + 4(k - 1) is k).
#[test]
fn a_tic_a_later_snapshot_already_holds_is_skipped() {
    let dir = scratch("mut6-skip");
    let ops = Arc::new(Mutex::new(Vec::new()));
    let mut host = writer(
        &dir,
        TestIo {
            ops: Arc::clone(&ops),
            ..TestIo::default()
        },
    );
    let name = "a.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    assert_eq!(host.save(name, &demo(2)), Ok(()));
    assert_eq!(host.append(name, at_tic(2), &tail_of(2)), Ok(true));
    host.finish();
    let report = host.close();
    assert!(!report.panicked);
    assert_eq!(report.unreported, []);
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), demo(2));
    assert_eq!(
        *ops.lock().unwrap(),
        [
            Op::Claim,
            Op::Rewrite {
                synced: true,
                tics: 2
            }
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `WriterHost::note_waiting`: "how many tics are waiting once `name` will hold `tics`", counted
/// from what the writer's last completed write left in that session's own file (the field's
/// documentation: "the most tics the app had handed over that the file did not hold yet"). A
/// whole demo of 5 tics handed over while the file holds 3 has 2 waiting; a session whose claim
/// has not completed has nothing in its file, whatever another session's file holds.
#[test]
fn the_most_tics_waiting_counts_from_the_sessions_own_file() {
    let dir = scratch("mut6-waiting");
    let gate = Gate::closed();
    gate.set(true);
    let mut host = writer(
        &dir,
        TestIo {
            gate: Some(Arc::clone(&gate)),
            ..TestIo::default()
        },
    );
    let _open = gate.opens_on_drop();
    let (a, b) = ("a.lmp", "b.lmp");
    assert_eq!(host.claim(a, &demo(0)), Ok(true));
    for k in 1..=3 {
        assert_eq!(host.append(a, at_tic(k), &tail_of(k)), Ok(true));
        host.wait_idle();
    }
    assert_eq!(host.shared.lock().counts.most_waiting, 1);
    assert_eq!(host.save(a, &demo(5)), Ok(()));
    assert_eq!(
        host.shared.lock().counts.most_waiting,
        2,
        "5 tics handed over, the file holds 3"
    );
    host.wait_idle();
    assert_eq!(host.written(a), Some(5));
    gate.set(false); // B's claim cannot complete: the file of A is the last one written
    assert_eq!(host.claim(b, &demo(0)), Ok(true));
    assert_eq!(host.save(b, &demo(8)), Ok(()));
    assert_eq!(
        host.shared.lock().counts.most_waiting,
        8,
        "B's file holds nothing, A's 5 tics do not count for it"
    );
    gate.set(true);
    host.finish();
    host.close();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `WriterHost::stopped_short`: "whether the writer thread ended before finishing every message
/// sent". A writer closed after it made every write handed over did not stop short, so the
/// synchronous outcome of a call is `Ok`, not "the session writer stopped before writing it".
/// Below the public API: after `close` every call that sends fails with "closed" before it
/// reaches this check, and a writer panic leaves `done` behind `sent`; only a direct call sees
/// the stopped writer with nothing left over.
#[test]
fn a_writer_closed_after_every_write_did_not_stop_short() {
    let dir = scratch("mut6-short");
    let mut host = writer(&dir, TestIo::default());
    assert_eq!(host.claim("a.lmp", &demo(0)), Ok(true));
    host.finish();
    let report = host.close();
    assert!(!report.panicked);
    assert!(!host.stopped_short());
    assert_eq!(host.outcome("a.lmp", Ok(())), Ok(()));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `WriterHost::settle` "waits at most `wait` for the writer to finish every write handed over":
/// with nothing left to write it answers at once, however long it may wait (an hour here; a watchdog
/// frees it after 10 s and the test fails), and says what the file holds. A session never handed over has
/// no file of this run: nothing is known, `Pending` (the `held` session is another one).
#[test]
fn settle_answers_at_once_when_the_writer_has_nothing_left() {
    let dir = scratch("mut6-settle");
    let mut host = writer(&dir, TestIo::default());
    assert_eq!(host.claim("a.lmp", &demo(0)), Ok(true));
    assert_eq!(host.append("a.lmp", at_tic(1), &tail_of(1)), Ok(true));
    host.wait_idle();
    // a watchdog frees a settle that keeps waiting (by marking the writer stopped), so that a
    // fault ends the test with a failure instead of an hour's wait
    let shared = Arc::clone(&host.shared);
    let (answered, listen) = mpsc::channel::<()>();
    let watchdog = std::thread::spawn(move || {
        let stuck = listen.recv_timeout(Duration::from_secs(10)).is_err();
        if stuck {
            shared.lock().alive = false;
            shared.changed.notify_all();
        }
        stuck
    });
    let settled = host.settle("a.lmp", Duration::from_secs(3600));
    answered.send(()).unwrap();
    assert!(
        !watchdog.join().unwrap(),
        "settle kept waiting for a writer with nothing left to write"
    );
    assert_eq!(settled, Settled::Written(1));
    assert_eq!(host.settle("b.lmp", Duration::ZERO), Settled::Pending);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The writer host answers the clock, the sound output and the mute key through the file host
/// ([`FsHost`] for "the clock, the directory and the sound"): a real date (month 1-12), a sound
/// sink, and a mute key press that reaches the device (headless sink: no real device is opened).
#[test]
fn the_writer_host_forwards_the_clock_the_sound_and_the_mute_key() {
    use crate::audio::{DeviceSink, Volume};
    use crate::keys::Mute;
    let dir = scratch("mut6-forward");
    let mut file_host = fs(&dir);
    file_host.audio = Output::Device(DeviceSink::headless(Volume::FULL));
    let mut host =
        WriterHost::with_io(file_host, Box::new(TestIo::default()), Box::new(|_| {})).unwrap();
    let t = host.timestamp();
    assert!(t.year >= 2024, "{t:?}");
    assert!((1..=12).contains(&t.month), "{t:?}");
    assert!((1..=31).contains(&t.day), "{t:?}");
    assert!(host.audio().is_some());
    host.toggle_mute(); // muted now
    assert_eq!(
        host.fs.audio.toggle_mute(),
        Some(false),
        "the key press reached the device: this second toggle unmutes"
    );
    host.close();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `WriterHost::save` keeps the newest snapshot "of that session (a newer one replaces it in
/// place)": a whole demo of another session does not replace a pending one of this session; both
/// wait, and each file ends as its own demo handed over.
#[test]
fn a_snapshot_of_another_session_does_not_replace_a_pending_one() {
    let dir = scratch("mut6-two-snapshots");
    let gate = Gate::closed();
    let mut host = writer(
        &dir,
        TestIo {
            gate: Some(Arc::clone(&gate)),
            ..TestIo::default()
        },
    );
    let _open = gate.opens_on_drop();
    let (a, b) = ("a.lmp", "b.lmp");
    assert_eq!(host.claim(a, &demo(0)), Ok(true));
    gate.await_waiter(); // the writer is held inside A's claim
    assert_eq!(host.save(a, &demo(1)), Ok(()));
    assert_eq!(host.claim(b, &demo(0)), Ok(true));
    assert_eq!(host.save(b, &demo(2)), Ok(()));
    assert_eq!(host.queue_depth().1, 2, "one pending snapshot per session");
    gate.set(true);
    host.finish();
    let report = host.close();
    assert_eq!(report.unreported, []);
    assert_eq!(std::fs::read(dir.join(a)).unwrap(), demo(1));
    assert_eq!(std::fs::read(dir.join(b)).unwrap(), demo(2));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `Writer::take_append`: a tic is "queued for the file when the file is current (or a whole
/// rewrite is pending anyway)". The session's file was taken by another process (never written),
/// then a tic out of step and a tic in step are offered in one batch: the in-step tic is not
/// queued for the file (that would make the flush report the loss again, replacing the failure
/// of the tic out of step that the app was to hear).
#[test]
fn a_tic_is_not_queued_for_a_file_that_another_process_took() {
    let dir = scratch("mut6-lost-append");
    let gate = Gate::closed();
    let mut host = writer(
        &dir,
        TestIo {
            gate: Some(Arc::clone(&gate)),
            race: true,
            ..TestIo::default()
        },
    );
    let _open = gate.opens_on_drop();
    let name = "a.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    gate.await_waiter(); // the writer is held inside the claim, which will find the name taken
    assert_eq!(host.append(name, 29, &tail_of(5)), Ok(true)); // out of step
    assert_eq!(host.append(name, 13, &tail_of(1)), Ok(true)); // in step with the copy
    gate.set(true);
    host.wait_idle();
    let failure = host.shared.lock().failure.as_ref().map(|f| f.error.clone());
    assert_eq!(
        failure.as_deref(),
        Some("a.lmp: a tic at byte 29, but the writer's copy holds 14 bytes")
    );
    assert_eq!(
        std::fs::read(dir.join(name)).unwrap(),
        b"another process's file"
    );
    host.close();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `Writer::flush`: "nothing until the app hands a whole demo over (its retry cadence, ruling
/// 4)" for a file the writer no longer vouches for. A tic in step is queued and then a tic out
/// of step marks the file stale, in one batch: nothing is written for either, and the file
/// stays as the claim left it.
#[test]
fn nothing_is_written_to_a_file_marked_stale_until_a_whole_demo_comes() {
    let dir = scratch("mut6-stale-batch");
    let gate = Gate::closed();
    let ops = Arc::new(Mutex::new(Vec::new()));
    let mut host = writer(
        &dir,
        TestIo {
            gate: Some(Arc::clone(&gate)),
            ops: Arc::clone(&ops),
            ..TestIo::default()
        },
    );
    let _open = gate.opens_on_drop();
    let name = "a.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    gate.await_waiter();
    assert_eq!(host.append(name, 13, &tail_of(1)), Ok(true)); // in step
    assert_eq!(host.append(name, 29, &tail_of(5)), Ok(true)); // out of step: stale
    gate.set(true);
    host.wait_idle();
    assert_eq!(*ops.lock().unwrap(), [Op::Claim]);
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), demo(0));
    let failure = host.shared.lock().failure.as_ref().map(|f| f.error.clone());
    assert_eq!(
        failure.as_deref(),
        Some("a.lmp: a tic at byte 29, but the writer's copy holds 18 bytes")
    );
    host.close();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The exit line's counts ([`crate::write_stats::session_line`], the field documentation of
/// [`SessionCounts`]): one claim, three append operations (two wrote a tic each; one found the
/// file not the length the copy expected, wrote nothing and was repaired by an unsynced whole
/// rewrite), two synced whole rewrites (the app's saves) and one more unsynced (a catch-up).
#[test]
fn the_exit_counts_tell_each_kind_of_operation_apart() {
    let dir = scratch("mut6-counts");
    let mut host = writer(&dir, TestIo::default());
    let name = "a.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    host.wait_idle();
    for k in 1..=2 {
        assert_eq!(host.append(name, at_tic(k), &tail_of(k)), Ok(true));
        host.wait_idle();
    }
    // another process changed the file: the next append is declined and repaired
    std::fs::write(dir.join(name), vec![0u8; 100]).unwrap();
    assert_eq!(host.append(name, at_tic(3), &tail_of(3)), Ok(true));
    host.wait_idle();
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), demo(3));
    for _ in 0..2 {
        assert_eq!(host.save(name, &demo(3)), Ok(()));
        host.wait_idle();
    }
    host.declined = true; // as after an append declined at the bound: the next snapshot is unsynced
    assert_eq!(host.save(name, &demo(3)), Ok(()));
    host.wait_idle();
    let report = host.close();
    assert!(!report.panicked);
    let c = &report.counts;
    assert_eq!(
        (
            c.claims,
            c.appends,
            c.appended_tics,
            c.declined_appends,
            c.synced_rewrites,
            c.rewrites
        ),
        (1, 3, 2, 1, 2, 2)
    );
    assert_eq!((report.times.count, report.times.failed), (8, 0));
    let line = crate::write_stats::session_line(&report.counts, &report.times);
    assert!(
        line.starts_with(
            "session file writes: 8 (claims 1, appends 3 of 2 tics, 1 appends declined, whole \
             rewrites 2 synced and 2 unsynced), 0 failed;"
        ),
        "{line}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `Writer::fail`: "a repeat of the standing one changes nothing". A whole rewrite that fails
/// again with the failure the app has already been told about does not make it unreported again:
/// the exit report lists no failure the app never heard of.
#[test]
fn a_repeated_failure_stays_reported() {
    let dir = scratch("mut6-repeat");
    let fail_rewrites = Arc::new(AtomicBool::new(false));
    let mut host = writer(
        &dir,
        TestIo {
            fail_rewrites: Arc::clone(&fail_rewrites),
            ..TestIo::default()
        },
    );
    let name = "a.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    host.wait_idle();
    fail_rewrites.store(true, Ordering::SeqCst);
    assert_eq!(host.save(name, &demo(1)), Ok(()));
    host.wait_idle(); // the rewrite failed: a failure the app has not heard of
    // this call tells the app (the retry is this snapshot), and the retry fails the same way
    assert_eq!(
        host.save(name, &demo(1)),
        Err("planted: disk full".to_string())
    );
    host.wait_idle();
    let report = host.close();
    assert_eq!(report.unreported, []);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A disk whose every whole rewrite fails with a different message (`planted 1`, `planted 2`,
/// ...); the other operations are the disk's.
#[derive(Default)]
struct Numbered(usize);

impl SessionIo for Numbered {
    fn claim(&mut self, dir: &Path, name: &str, bytes: &[u8]) -> Result<Claim, String> {
        RealIo.claim(dir, name, bytes)
    }
    fn rewrite(&mut self, _: &Path, _: &str, _: &[u8], _: bool) -> Result<(), String> {
        self.0 += 1;
        Err(format!("planted {}", self.0))
    }
    fn append(
        &mut self,
        dir: &Path,
        name: &str,
        at: u64,
        tails: &[[u8; 5]],
    ) -> Result<bool, String> {
        RealIo.append(dir, name, at, tails)
    }
}

/// `Writer::fail`, the other side: a different failure of the same session replaces the standing
/// one, which the app had been told about, so the new one is unreported until the app hears it
/// (the exit report lists it).
#[test]
fn a_different_failure_replaces_the_standing_one() {
    let dir = scratch("mut6-different");
    let mut host =
        WriterHost::with_io(fs(&dir), Box::new(Numbered::default()), Box::new(|_| {})).unwrap();
    let name = "a.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    host.wait_idle();
    assert_eq!(host.save(name, &demo(1)), Ok(()));
    host.wait_idle(); // "planted 1"
    assert_eq!(host.save(name, &demo(1)), Err("planted 1".to_string()));
    host.wait_idle(); // the retry fails with "planted 2"
    let report = host.close();
    assert_eq!(
        report.unreported,
        [(name.to_string(), "planted 2".to_string())]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
