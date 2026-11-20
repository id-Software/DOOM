//! The session writer's tests (M5d, the owner's rulings of 2026-11-20). Rust-only checks of
//! doom-play's file handling; each was planted to fail (see the M5d record).

use super::*;
use crate::audio::{Discard, Output};
use doom_game::app::{App, GameData, SaveStatus, SessionConfig};
use doom_render::GameMode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

fn load() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    let files: Vec<Vec<u8>> = ["m5a_wads/amp.wad", "m5b_gfx.wad", "m5c_sfx.wad"]
        .iter()
        .map(|w| std::fs::read(dir.join(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    GameData::load_with_sound(&refs, GameMode::Registered, 1, 2).expect("game data")
}

fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("doom-play-writer-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fs(dir: &Path) -> FsHost {
    FsHost {
        dir: dir.to_path_buf(),
        audio: Output::Discard(Discard::default()),
    }
}

/// A gate the test opens: every file operation waits for it (a stalled disk).
#[derive(Default)]
struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    fn closed() -> Arc<Self> {
        Arc::new(Self::default())
    }
    fn set(&self, open: bool) {
        *self.open.lock().unwrap() = open;
        self.changed.notify_all();
    }
    /// A guard that opens the gate when dropped, so that a failing assertion with the disk
    /// stalled ends the test instead of leaving the host's drain waiting on the gate.
    fn opens_on_drop(self: &Arc<Self>) -> Opens {
        Opens(Arc::clone(self))
    }
    fn pass(&self) {
        let mut open = self.open.lock().unwrap();
        while !*open {
            open = self.changed.wait(open).unwrap();
        }
    }
}

struct Opens(Arc<Gate>);

impl Drop for Opens {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

/// One file operation as the writer asked for it.
#[derive(Clone, Debug, PartialEq)]
enum Op {
    Claim,
    Rewrite { synced: bool, tics: u64 },
    Append { at: u64, tics: usize },
}

/// The disk ([`RealIo`]) with what a test needs: a delay before each operation, a gate, a record
/// of the operations, planted failures, another process taking the name before the claim, and a
/// claim reported as the fallback.
#[derive(Default)]
struct TestIo {
    delay: Duration,
    gate: Option<Arc<Gate>>,
    ops: Arc<Mutex<Vec<Op>>>,
    fail_appends: Arc<AtomicBool>,
    fail_rewrites: Arc<AtomicBool>,
    race: bool,
    fallback: bool,
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
        if self.fail_rewrites.load(Ordering::SeqCst) {
            return Err("planted: disk full".into());
        }
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

fn writer(dir: &Path, io: TestIo) -> WriterHost {
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
fn demo(n: u64) -> Vec<u8> {
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
    let mut host = writer(
        &dir,
        TestIo {
            delay: Duration::from_millis(60),
            ops: Arc::clone(&ops),
            fail_appends: Arc::clone(&fail_appends),
            ..TestIo::default()
        },
    );
    let name = "E1M1-skill2-20261120-000002.lmp";
    assert_eq!(host.claim(name, &demo(0)), Ok(true));
    host.wait_idle();
    fail_appends.store(true, Ordering::SeqCst);
    let tail = |k: u64| -> [u8; 5] { demo(k)[demo(k).len() - 5..].try_into().unwrap() };
    assert_eq!(host.append(name, 13, &tail(1)), Ok(true));
    // the writer is inside the first append's 60 ms when the second tic is queued
    std::thread::sleep(Duration::from_millis(15));
    assert_eq!(host.append(name, 17, &tail(2)), Ok(true));
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
