//! The session file settled into the app and the run's end (M5d review 3, M1, m2, m3), with a
//! real [`App`] and the real [`WriterHost`] over the session writer's test disk
//! ([`crate::session_writer::tests::TestIo`]). Rust-only checks of doom-play's file handling.

use super::*;
use crate::input_log::InputLog;
use crate::session_writer::tests::{Gate, Op, TestIo, at, load, scratch, writer};
use doom_game::app::{PICTURE_BYTES, SAVE_FAILED_LABEL, Screen, SessionConfig};
use doom_game::input::keys::KEY_ESCAPE;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Whether `text` is drawn in the picture (the app's font, any ink colour; as doom-game's tests
/// read it).
fn shows(pic: &[u8], text: &str) -> bool {
    let mut glyphs = Screen::filled(0);
    glyphs.text(0, 0, text, 1);
    let (w, h) = (text.chars().count() * 6 - 1, 7);
    let px = |x: usize, y: usize| &pic[(y * 320 + x) * 4..(y * 320 + x) * 4 + 4];
    let first = glyphs.pixels().iter().position(|&p| p == 1).unwrap_or(0);
    (0..=200 - h).any(|y0| {
        (0..=320 - w).any(|x0| {
            let ink = px(x0 + first % 320, y0 + first / 320);
            (0..h).all(|y| {
                (0..w).all(|x| (glyphs.pixels()[y * 320 + x] == 1) == (px(x0 + x, y0 + y) == ink))
            })
        })
    })
}

fn picture(app: &App) -> Vec<u8> {
    let mut pic = vec![0; PICTURE_BYTES];
    app.picture(0, &mut pic);
    pic
}

/// The test disk's switches and record.
#[derive(Clone, Default)]
struct Disk {
    ops: Arc<Mutex<Vec<Op>>>,
    fail_appends: Arc<AtomicBool>,
    fail_rewrites: Arc<AtomicBool>,
    fail_next_rewrites: Arc<AtomicUsize>,
    gate: Option<Arc<Gate>>,
}

impl Disk {
    fn gated() -> Self {
        let gate = Gate::closed();
        gate.set(true);
        Self {
            gate: Some(gate),
            ..Self::default()
        }
    }
    fn io(&self) -> TestIo {
        TestIo {
            ops: Arc::clone(&self.ops),
            fail_appends: Arc::clone(&self.fail_appends),
            fail_rewrites: Arc::clone(&self.fail_rewrites),
            fail_next_rewrites: Arc::clone(&self.fail_next_rewrites),
            gate: self.gate.clone(),
            ..TestIo::default()
        }
    }
    fn fail(&self, appends: bool, rewrites: bool) {
        self.fail_appends.store(appends, Ordering::SeqCst);
        self.fail_rewrites.store(rewrites, Ordering::SeqCst);
    }
    /// The next `n` whole rewrites fail, later ones succeed.
    fn fail_next_rewrites(&self, n: usize) {
        self.fail_next_rewrites.store(n, Ordering::SeqCst);
    }
    fn gate(&self, open: bool) {
        self.gate.as_ref().expect("a gated disk").set(open);
    }
    fn synced_rewrites_of(&self, tics: u64) -> usize {
        self.rewrites_of(true, tics)
    }
    fn rewrites_of(&self, synced: bool, tics: u64) -> usize {
        let want = Op::Rewrite { synced, tics };
        self.ops
            .lock()
            .unwrap()
            .iter()
            .filter(|o| **o == want)
            .count()
    }
}

/// A run as the window makes it: the app capped at `cap` tics on the session writer over `disk`
/// in `dir`, the log off.
fn play(dir: &std::path::Path, disk: &Disk, cap: usize) -> Play {
    play_with_log(dir, disk, cap, InputLog::off())
}

fn play_with_log(dir: &std::path::Path, disk: &Disk, cap: usize, log: InputLog) -> Play {
    let mut host = LoggedHost {
        host: writer(dir, disk.io()),
        log,
    };
    let cfg = SessionConfig {
        tic_cap: cap,
        ..SessionConfig::default()
    };
    let app = App::new(load(), cfg, &mut host);
    let mut p = Play::new(app, host, Forward::default(), false);
    p.app.update(0, &mut p.host);
    p
}

/// One `update` as the window makes it (the tic loop, the settle, the pump's end); the exit
/// report's lines if the run ended.
fn step(p: &mut Play, k: u64) -> Vec<String> {
    let ended_before = matches!(p.app.mode(), Mode::Ended(_));
    let u = p.app.update(at(k), &mut p.host);
    p.settle_after_update(ended_before);
    p.pump_end(u, false)
}

/// Plays tics `from..=to`, letting the writer keep up after each.
fn steady(p: &mut Play, from: u64, to: u64) {
    for k in from..=to {
        assert!(step(p, k).is_empty());
        p.host.host.wait_idle();
    }
}

fn session(p: &Play, dir: &std::path::Path) -> (PathBuf, Vec<u8>) {
    (
        dir.join(p.app.session_name()),
        p.app.recorder().bytes().unwrap(),
    )
}

/// Writes that fail from the session's last tic, which the app never hears of while tics run
/// (the disk is stalled at the end, so the end save is answered `Ok`): the end screen says DEMO
/// SAVED until the window settles the file at the session's end, then DEMO NOT SAVED with the
/// reason. A restart with the disk recovered rewrites the first file whole (the app's retry),
/// and it then holds every tic.
#[test]
fn writes_that_fail_from_the_last_tic_show_on_the_end_screen_and_are_retried_at_restart() {
    let dir = scratch("close-last");
    let disk = Disk::gated();
    let mut p = play(&dir, &disk, 30);
    // after the play: dropped first, so a failing assertion opens the gate before the writer joins
    let _open = disk.gate.as_ref().unwrap().opens_on_drop();
    steady(&mut p, 1, 29);
    disk.gate(false);
    disk.fail(true, true);
    p.app.update(at(30), &mut p.host);
    p.app.update(at(31), &mut p.host);
    assert!(matches!(p.app.mode(), Mode::Ended(_)));
    assert_eq!(p.app.save_status(), &SaveStatus::Saved(30), "answered Ok");
    assert!(shows(&picture(&p.app), "DEMO SAVED:"));
    disk.gate(true);
    p.settle_after_update(false);
    assert_eq!(
        p.app.save_status(),
        &SaveStatus::Failed("planted: disk full".into())
    );
    let pic = picture(&p.app);
    assert!(shows(&pic, "DEMO NOT SAVED: PLANTED: DISK FULL"));
    assert!(!shows(&pic, "DEMO SAVED:"));
    let (path, want) = session(&p, &dir);
    disk.fail(false, false);
    p.app.key(b'r', true, &mut p.host);
    p.host.host.wait_idle();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        want,
        "every tic, after the retry"
    );
    p.quit();
    let lines = p.conclude();
    assert!(
        !lines.iter().any(|l| l.contains("earlier session")),
        "{lines:#?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The reviewer's probe (review 3, M1): whole rewrites fail from tic 30 and the session is capped
/// at 30 tics, so the failure is found only at the end (tic 30's append went with the end save).
/// The app's status becomes Failed at the session's end and the restart retries it: once (the
/// writer does not rewrite it again at once when it moves on), and with the disk recovered the
/// first file then holds all 30 tics. With the disk still failing at the restart, the session is
/// rewritten once more at exit: the exit report says it was saved then (disk recovered) or that
/// it was not (still failing), and nothing else about it.
#[test]
fn a_failure_found_only_at_the_end_is_retried_at_restart_and_at_exit() {
    for case in ["restart", "exit", "never"] {
        let dir = scratch(&format!("close-probe-{case}"));
        let disk = Disk::default();
        let mut p = play(&dir, &disk, 30);
        steady(&mut p, 1, 29);
        disk.fail(false, true);
        step(&mut p, 30);
        step(&mut p, 31);
        assert!(matches!(p.app.mode(), Mode::Ended(_)), "{case}");
        assert_eq!(
            p.app.save_status(),
            &SaveStatus::Failed("planted: disk full".into()),
            "{case}"
        );
        assert!(shows(
            &picture(&p.app),
            "DEMO NOT SAVED: PLANTED: DISK FULL"
        ));
        let (path, want) = session(&p, &dir);
        let before = disk.synced_rewrites_of(30);
        disk.fail(false, case != "restart");
        p.app.key(b'r', true, &mut p.host);
        p.host.host.wait_idle();
        assert_eq!(disk.synced_rewrites_of(30), before + 1, "one retry: {case}");
        if case == "restart" {
            assert_eq!(std::fs::read(&path).unwrap(), want, "{case}");
        }
        disk.fail(false, case == "never");
        step(&mut p, 32);
        p.quit();
        let lines = p.conclude();
        let about: Vec<&String> = lines
            .iter()
            .filter(|l| l.contains(path.to_str().unwrap()))
            .collect();
        match case {
            "restart" => assert!(about.is_empty(), "{lines:#?}"),
            "exit" => {
                assert_eq!(about.len(), 1, "{lines:#?}");
                assert!(
                    about[0].starts_with(&format!(
                        "an earlier session was saved at exit: {} (30 tics), rewritten whole \
                         after its last write failed (planted: disk full)",
                        path.display()
                    )),
                    "{lines:#?}"
                );
                assert_eq!(std::fs::read(&path).unwrap(), want);
            }
            _ => {
                assert_eq!(about.len(), 1, "{lines:#?}");
                assert_eq!(
                    about[0],
                    &format!(
                        "an earlier session was not saved: its last write to {} failed (planted: \
                         disk full), and the rewrite at exit failed too (planted: disk full)",
                        path.display()
                    )
                );
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// The disk stalled at a session's end: the window waits [`END_WAIT`] (2 s) for the writer in
/// the update in which the session ended, then the end screen says the demo is not written yet
/// ([`NOT_YET`]); when the disk answers, the next update's poll (which does not wait) makes it
/// say DEMO SAVED.
#[test]
fn a_stalled_disk_at_the_end_waits_two_seconds_then_says_not_yet_until_written() {
    let dir = scratch("close-stall");
    let disk = Disk::gated();
    let mut p = play(&dir, &disk, 30);
    // after the play: dropped first, so a failing assertion opens the gate before the writer joins
    let _open = disk.gate.as_ref().unwrap().opens_on_drop();
    steady(&mut p, 1, 29);
    disk.gate(false);
    step(&mut p, 30);
    let t0 = Instant::now();
    step(&mut p, 31);
    let waited = t0.elapsed();
    assert!(matches!(p.app.mode(), Mode::Ended(_)));
    assert!(
        waited >= END_WAIT && waited < END_WAIT + Duration::from_secs(1),
        "{waited:?}"
    );
    assert_eq!(p.app.save_status(), &SaveStatus::Failed(NOT_YET.into()));
    assert!(shows(&picture(&p.app), "DEMO NOT SAVED: NOT WRITTEN YET"));
    // a poll while the disk is still stalled neither waits nor changes it
    let t0 = Instant::now();
    step(&mut p, 32);
    assert!(t0.elapsed() < Duration::from_millis(500));
    assert_eq!(p.app.save_status(), &SaveStatus::Failed(NOT_YET.into()));
    disk.gate(true);
    p.host.host.wait_idle();
    step(&mut p, 33);
    assert_eq!(p.app.save_status(), &SaveStatus::Saved(30));
    assert!(shows(&picture(&p.app), "DEMO SAVED:"));
    let (path, want) = session(&p, &dir);
    assert_eq!(std::fs::read(path).unwrap(), want);
    p.quit();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A paused session whose last appends fail after the pause: a poll shows the failure on the
/// pause box. And with no poll before the restart (the failure comes after the player pressed
/// R), the writer rewrites the session whole from its own copy before it claims the next file,
/// so with the disk recovered the first file holds every tic.
#[test]
fn a_failure_after_the_pause_shows_on_the_pause_box_or_is_retried_by_the_writer() {
    for poll in [true, false] {
        let dir = scratch(&format!("close-pause-{poll}"));
        let disk = Disk::gated();
        let mut p = play(&dir, &disk, 1_000);
        let _open = disk.gate.as_ref().unwrap().opens_on_drop();
        steady(&mut p, 1, 20);
        disk.gate(false);
        disk.fail(true, false);
        step(&mut p, 21);
        p.app.key(KEY_ESCAPE, true, &mut p.host);
        assert_eq!(p.app.mode(), &Mode::Paused);
        let (path, want) = session(&p, &dir);
        if poll {
            disk.gate(true);
            p.host.host.wait_idle();
            step(&mut p, 22);
            assert!(matches!(p.app.save_status(), SaveStatus::Failed(_)));
            assert!(shows(&picture(&p.app), SAVE_FAILED_LABEL));
        } else {
            assert_eq!(p.app.save_status(), &SaveStatus::Saved(21));
            p.app.key(b'r', true, &mut p.host);
            disk.gate(true);
            p.host.host.wait_idle();
            assert_eq!(disk.synced_rewrites_of(21), 1, "the writer's rewrite");
            assert_eq!(std::fs::read(&path).unwrap(), want);
        }
        p.quit();
        let lines = p.conclude();
        assert!(
            !lines.iter().any(|l| l.contains("earlier session")),
            "{lines:#?}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Review 4, R2 (the reviewer's probe C): the disk stalls (the writer is held in tic 21's
/// append), the queue fills at [`crate::session_writer::QUEUE_TICS`] tics and the app falls back
/// to a catch-up snapshot, answered `Ok` (its status says Saved(260)). The player pauses and
/// presses R in the same pump, so no poll runs between and the app does not save again; the
/// catch-up rewrite then fails, a first failure the app never hears of. The writer rewrites the
/// first session whole once more before it claims the next file: with the disk working again
/// ("claim"), the first file then holds all 260 tics and nothing is said about it at exit; with
/// that rewrite failing too ("exit"), the session is kept and rewritten at exit, and the one line
/// about it says it was saved then (no second line about the same file).
#[test]
fn a_first_failure_after_the_restart_is_rewritten_at_the_next_claim() {
    for case in ["claim", "exit"] {
        let dir = scratch(&format!("close-catch-up-{case}"));
        let disk = Disk::gated();
        let mut p = play(&dir, &disk, 1_000);
        let _open = disk.gate.as_ref().unwrap().opens_on_drop();
        steady(&mut p, 1, 20);
        disk.gate(false);
        assert!(step(&mut p, 21).is_empty());
        disk.gate.as_ref().unwrap().await_waiter(); // the writer holds tic 21's append
        for k in 22..=260 {
            assert!(step(&mut p, k).is_empty());
        }
        let (queued, snapshots, most, _) = p.host.host.queue_depth();
        let bound = crate::session_writer::QUEUE_TICS;
        assert_eq!((queued, snapshots, most), (bound, 1, bound), "{case}");
        assert_eq!(p.app.save_status(), &SaveStatus::Saved(260), "{case}");
        disk.fail_next_rewrites(if case == "claim" { 1 } else { 2 });
        p.app.key(KEY_ESCAPE, true, &mut p.host);
        let (path, want) = session(&p, &dir);
        let name = p.app.session_name().to_string();
        p.app.key(b'r', true, &mut p.host);
        assert_ne!(p.app.session_name(), name, "{case}: restarted");
        disk.gate(true);
        p.host.host.wait_idle();
        assert_eq!(
            disk.rewrites_of(false, 260),
            1,
            "{case}: the failed catch-up"
        );
        assert_eq!(
            disk.synced_rewrites_of(260),
            1,
            "{case}: the writer's rewrite"
        );
        if case == "claim" {
            assert_eq!(std::fs::read(&path).unwrap(), want, "{case}");
        } else {
            assert_ne!(std::fs::read(&path).unwrap(), want, "{case}");
        }
        p.quit();
        let lines = p.conclude();
        let about: Vec<&String> = lines.iter().filter(|l| l.contains(&name)).collect();
        if case == "claim" {
            assert!(about.is_empty(), "{lines:#?}");
        } else {
            assert_eq!(
                about,
                [&format!(
                    "an earlier session was saved at exit: {} (260 tics), rewritten whole after \
                     its last write failed (planted: disk full)",
                    path.display()
                )],
                "{lines:#?}"
            );
            assert_eq!(disk.synced_rewrites_of(260), 2, "{case}: and at exit");
        }
        assert_eq!(std::fs::read(&path).unwrap(), want, "{case}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Review 4, R3 (plant C1; the reviewer's probe B2): one append fails (tic 13), the app hears
/// of it at tic 14 and retries at tic 15, which the writer makes (the file then holds 15 tics);
/// the app's next retry falls 35 tics later, so tics 16 to 30 are not handed over and its status
/// stays Failed. Paused at tic 30, the poll finds every write handed over made, the file holding
/// 15 tics: the status stays Failed (the pause box says so), never Saved(30). The quit then
/// rewrites the file whole, and the exit line says it was saved with its 30 tics, which the file
/// holds.
#[test]
fn a_file_behind_the_recorder_is_never_called_saved() {
    let dir = scratch("close-behind");
    let disk = Disk::default();
    let mut p = play(&dir, &disk, 1_000);
    steady(&mut p, 1, 12);
    disk.fail(true, false);
    steady(&mut p, 13, 13);
    disk.fail(false, false);
    steady(&mut p, 14, 30);
    assert_eq!(p.host.host.written(p.app.session_name()), Some(15));
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    assert!(step(&mut p, 31).is_empty());
    assert_eq!(p.app.mode(), &Mode::Paused);
    assert_eq!(
        p.app.save_status(),
        &SaveStatus::Failed("planted: disk full".into())
    );
    assert!(shows(&picture(&p.app), SAVE_FAILED_LABEL));
    let (path, want) = session(&p, &dir);
    assert_eq!(p.app.recorder().len(), 30);
    p.quit();
    let lines = p.conclude();
    let about: Vec<&String> = lines
        .iter()
        .filter(|l| l.contains("saved") || l.contains("written"))
        .collect();
    assert_eq!(
        about,
        [&format!("saved {} (30 tics)", path.display())],
        "{lines:#?}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), want);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 4, R3 (plant D): a poll while paused does not wait, so a write still being made is
/// not a failure. The disk stalls at tic 20 (the writer is held in its append) and the player
/// pauses: the poll leaves the status Saved(20) and the pause box does not say the save failed
/// (only the poll at a session's end, after its wait, says the demo is not written yet). When
/// the disk answers, the next poll finds the file holding the 20 tics.
#[test]
fn a_paused_poll_with_a_write_in_flight_is_not_a_failure() {
    let dir = scratch("close-pause-pending");
    let disk = Disk::gated();
    let mut p = play(&dir, &disk, 1_000);
    let _open = disk.gate.as_ref().unwrap().opens_on_drop();
    steady(&mut p, 1, 19);
    disk.gate(false);
    assert!(step(&mut p, 20).is_empty());
    disk.gate.as_ref().unwrap().await_waiter(); // the writer holds tic 20's append
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    let t0 = Instant::now();
    assert!(step(&mut p, 21).is_empty());
    assert!(
        t0.elapsed() < Duration::from_millis(500),
        "the poll did not wait"
    );
    assert_eq!(p.app.mode(), &Mode::Paused);
    assert_eq!(p.app.save_status(), &SaveStatus::Saved(20));
    assert!(!shows(&picture(&p.app), SAVE_FAILED_LABEL));
    assert_eq!(
        p.host.host.settle(p.app.session_name(), Duration::ZERO),
        Settled::Pending
    );
    disk.gate(true);
    p.host.host.wait_idle();
    assert!(step(&mut p, 22).is_empty());
    assert_eq!(p.app.save_status(), &SaveStatus::Saved(20));
    let (path, want) = session(&p, &dir);
    assert_eq!(std::fs::read(path).unwrap(), want);
    p.quit();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 3, m2: the exit report says one thing about the current session. Appends fail from
/// tic 10 (the app hears of it and retries every 35 tics); at quit the writer is drained, the
/// failure settled into the app and the app's save on quit made with the writer waiting: with
/// the disk recovered the one line is "saved PATH (N tics)" and the file holds every tic; with it
/// still failing the one line is "the session was not saved: ..." and no line says saved.
#[test]
fn the_exit_report_says_one_true_thing_about_the_current_session() {
    for recovered in [true, false] {
        let dir = scratch(&format!("close-exit-{recovered}"));
        let disk = Disk::default();
        let mut p = play(&dir, &disk, 1_000);
        steady(&mut p, 1, 9);
        disk.fail(true, true);
        steady(&mut p, 10, 40);
        assert!(matches!(p.app.save_status(), SaveStatus::Failed(_)));
        disk.fail(!recovered, !recovered);
        let (path, want) = session(&p, &dir);
        p.quit();
        let u = doom_game::app::Update::default();
        let lines = p.pump_end(u, false);
        assert!(p.pump_end(u, false).is_empty(), "once");
        let saved = format!("saved {} (40 tics)", path.display());
        let session_lines: Vec<&String> = lines
            .iter()
            .filter(|l| l.contains("saved") || l.contains("written"))
            .collect();
        if recovered {
            assert_eq!(session_lines, [&saved], "{lines:#?}");
            assert_eq!(std::fs::read(&path).unwrap(), want);
        } else {
            assert_eq!(
                session_lines,
                [&"the session was not saved: planted: disk full".to_string()],
                "{lines:#?}"
            );
        }
        assert!(
            lines.iter().any(|l| l.starts_with("session file writes: ")),
            "{lines:#?}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Review 3, M1 at quit: a failure of the last writes that the app never heard of (the disk was
/// stalled when the player quit, so the app's save on quit was answered before it failed) is
/// settled into the app after the drain, so the app's save on quit, with the writer waiting,
/// retries it: with the disk recovered the one line about the session is "saved" and the file
/// holds every tic; with it still failing the line says it was not saved. Nothing else is said
/// about the session.
#[test]
fn a_failure_the_app_never_heard_of_is_retried_at_quit() {
    for recovered in [true, false] {
        let dir = scratch(&format!("close-quit-{recovered}"));
        let disk = Disk::gated();
        let mut p = play(&dir, &disk, 1_000);
        let _open = disk.gate.as_ref().unwrap().opens_on_drop();
        steady(&mut p, 1, 19);
        disk.gate(false);
        disk.fail(true, true);
        step(&mut p, 20);
        p.app.quit(&mut p.host);
        assert_eq!(p.app.save_status(), &SaveStatus::Saved(20), "answered Ok");
        let (path, want) = session(&p, &dir);
        let opener = {
            let disk = disk.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                disk.gate(true); // the failing append is made, then the disk recovers or not
                std::thread::sleep(Duration::from_millis(100));
                disk.fail(!recovered, !recovered);
            })
        };
        // the window: the append fails while the run's end waits for the writer
        std::thread::sleep(Duration::from_millis(150));
        opener.join().unwrap();
        p.end_run();
        let lines = p.conclude();
        let about: Vec<&String> = lines
            .iter()
            .filter(|l| l.contains("saved") || l.contains("written"))
            .collect();
        if recovered {
            assert_eq!(
                about,
                [&format!("saved {} (20 tics)", path.display())],
                "{lines:#?}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), want);
        } else {
            assert_eq!(
                about,
                [&"the session was not saved: planted: disk full".to_string()],
                "{lines:#?}"
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Review 4, R1 (the reviewer's probe A): appends and whole rewrites fail from tic 10 and keep
/// failing; the window's quit event ends the run, then its Drop quits again with no update
/// between. The second quit changes nothing: the exit line keeps the real reason, never "the
/// session writer was closed", and the app's status keeps it too.
#[test]
fn a_quit_after_the_runs_end_keeps_the_real_failure() {
    let dir = scratch("close-quit-twice");
    let disk = Disk::default();
    let mut p = play(&dir, &disk, 1_000);
    steady(&mut p, 1, 9);
    disk.fail(true, true);
    steady(&mut p, 10, 20);
    p.quit(); // the window's quit event
    p.quit(); // its Drop, no update between
    let failed = SaveStatus::Failed("planted: disk full".into());
    assert_eq!(p.app.save_status(), &failed);
    let lines = p.conclude();
    let about: Vec<&String> = lines
        .iter()
        .filter(|l| l.contains("saved") || l.contains("written"))
        .collect();
    assert_eq!(
        about,
        [&"the session was not saved: planted: disk full".to_string()],
        "{lines:#?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("writer was closed")),
        "{lines:#?}"
    );
    assert!(p.conclude().is_empty(), "once");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A writer the log's thread can own, read back by the test; it waits at `gate` if one is set.
#[derive(Clone, Default)]
struct Text {
    bytes: Arc<Mutex<Vec<u8>>>,
    gate: Option<Arc<Gate>>,
}

impl std::io::Write for Text {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if let Some(g) = &self.gate {
            g.pass();
        }
        self.bytes.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Review 3, m3: the run's end keeps the log's last pump line. A session played through
/// [`Play`] (the threaded log, ended after the update's pump line) and the same events written
/// in b374aaa's order to the direct writer (summary, then the pump line, at quit) give the same
/// bytes, the last line a `pump` line after the summary; and the window-close order (the run's
/// end in the quit event, the pump line in the next update) gives them too.
#[test]
fn the_runs_end_keeps_the_logs_last_pump_line() {
    let events = |log: &mut InputLog, app: &App, k: u64| {
        log.line("raw", app.tic(), format_args!("key-down Up {k}"));
    };
    // b374aaa's order with the direct writer and the synchronous host
    let dir = scratch("close-log-a");
    let direct = Text::default();
    let mut h = LoggedHost {
        host: crate::session_writer::tests::fs(&dir),
        log: InputLog::to_writer(Box::new(direct.clone())),
    };
    let mut app = App::new(load(), SessionConfig::default(), &mut h);
    let u = app.update(0, &mut h);
    h.log.pump_end(&app, u.ran, u.dropped, false);
    for k in 1..=12 {
        events(&mut h.log, &app, k);
        let u = app.update(at(k), &mut h);
        h.log.pump_end(&app, u.ran, u.dropped, false);
    }
    app.quit(&mut h);
    h.log.summary(app.tic());
    let u = app.update(at(13), &mut h);
    h.log.pump_end(&app, u.ran, u.dropped, false);
    drop(h);
    let want = String::from_utf8(direct.bytes.lock().unwrap().clone()).unwrap();
    let want = want
        .split_once('\n')
        .map(|(_, rest)| rest.to_string())
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        want.lines().last().unwrap().starts_with("pump 14 12 ran=0"),
        "{want}"
    );
    for close_event in [false, true] {
        let dir = scratch(&format!("close-log-b-{close_event}"));
        let text = Text::default();
        let log = InputLog::threaded(Box::new(text.clone())).unwrap();
        let mut p = play_with_log(&dir, &Disk::default(), 1_000, log);
        let u = doom_game::app::Update::default();
        p.host.log.pump_end(&p.app, u.ran, u.dropped, false);
        for k in 1..=12 {
            events(&mut p.host.log, &p.app, k);
            assert!(step(&mut p, k).is_empty());
        }
        if close_event {
            p.quit(); // the window's quit event; the next update ends the report
        } else {
            p.app.quit(&mut p.host); // the player's q
        }
        let ended_before = matches!(p.app.mode(), Mode::Ended(_));
        let u = p.app.update(at(13), &mut p.host);
        p.settle_after_update(ended_before);
        if p.app.quit_requested() {
            p.end_run();
        }
        let lines = p.pump_end(u, false);
        assert!(!lines.is_empty(), "the exit report");
        let got = String::from_utf8(text.bytes.lock().unwrap().clone()).unwrap();
        let unnamed = |t: &str| -> Vec<String> {
            t.lines()
                .map(|l| {
                    if l.starts_with("session ") {
                        "session".to_string()
                    } else {
                        l.to_string()
                    }
                })
                .collect()
        };
        assert_eq!(unnamed(&got), unnamed(&want), "close event {close_event}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Review 3, m3: the log's queue is bounded and the loop never waits for it while play runs. With
/// the log's file stalled, at most the bound of pumps wait; later pumps' lines are dropped whole
/// and every pump's end returns at once; when the file answers, one `dropped` line saying how
/// many lines of how many pumps are missing comes before the next pump's lines; after the
/// summary the loop waits for room instead of dropping, so the summary is written.
#[test]
fn a_stalled_log_drops_whole_pumps_and_says_how_many() {
    let gate = Gate::closed();
    let text = Text {
        gate: Some(Arc::clone(&gate)),
        ..Text::default()
    };
    let dir = scratch("close-log-bound");
    let mut p = play(&dir, &Disk::default(), 1_000);
    let mut log = InputLog::threaded_bounded(Box::new(text.clone()), 4).unwrap();
    // after the log: dropped first, so a failing assertion opens the gate before the log joins
    let _open = gate.opens_on_drop();
    let mut longest = Duration::ZERO;
    for k in 0..20u64 {
        let t0 = Instant::now();
        log.line("raw", k, format_args!("line {k}"));
        log.pump_end(&p.app, 1, 0, false);
        longest = longest.max(t0.elapsed());
        if k == 0 {
            // the thread has taken pump 0 and is held in its write: pumps 1 to 4 wait, the
            // other 15 are dropped (review 4, N5: no dependence on scheduling)
            gate.await_waiter();
        }
    }
    assert!(longest < Duration::from_millis(100), "{longest:?}");
    gate.set(true);
    // the thread empties the queue: it has taken the last waiting pump once it has written the
    // fifth (at most 10 s)
    let pumps_written = || {
        let bytes = text.bytes.lock().unwrap();
        bytes
            .split(|&b| b == b'\n')
            .filter(|l| l.starts_with(b"pump "))
            .count()
    };
    let t0 = Instant::now();
    while pumps_written() < 5 {
        assert!(
            t0.elapsed() < Duration::from_secs(10),
            "the log's thread did not drain"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    log.line("raw", 20, format_args!("line 20"));
    log.pump_end(&p.app, 1, 0, false);
    // the summary waits for room: the file stalls again and answers 200 ms later
    gate.set(false);
    for k in 21..30u64 {
        log.line("raw", k, format_args!("line {k}"));
        log.pump_end(&p.app, 1, 0, false);
    }
    let opener = {
        let gate = Arc::clone(&gate);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            gate.set(true);
        })
    };
    log.summary(30);
    log.pump_end(&p.app, 0, 0, false);
    log.finish();
    opener.join().unwrap();
    let got = String::from_utf8(text.bytes.lock().unwrap().clone()).unwrap();
    let lines: Vec<&str> = got.lines().collect();
    // Read in order: each pump kept is whole (its raw line, then its pump line); each `dropped`
    // line stands where its pumps are missing and counts them right (two lines a pump); pumps
    // 0 to 19 went over with the file stalled (the thread held pump 0, pumps 1 to 4 waited), so
    // pumps 5 to 19 are dropped and said so in one line; line 20, after the file answered and
    // the queue emptied, is kept.
    let mut next = 0u64;
    let mut said = Vec::new();
    let mut k = 0;
    while k < lines.len() && !lines[k].starts_with("summary ") {
        let l = lines[k];
        if let Some(rest) = l.strip_prefix("dropped ") {
            let w: Vec<&str> = rest.split(' ').collect();
            let (n, pumps): (u64, u64) = (w[2].parse().unwrap(), w[5].parse().unwrap());
            assert_eq!(
                rest.split_once(' ').unwrap().1.split_once(' ').unwrap().1,
                format!("{n} lines of {pumps} pumps were not written: the log's writer was behind")
            );
            assert_eq!(n, 2 * pumps, "{l}");
            said.push((next, pumps));
            next += pumps;
            k += 1;
            continue;
        }
        let want = format!("raw {} {next} line {next}", next + 1);
        assert_eq!(l, want, "{got}");
        assert!(lines[k + 1].starts_with(&format!("pump {} 0 ran=1", next + 1)));
        next += 1;
        k += 2;
    }
    assert_eq!(next, 30, "every pump kept or counted: {got}");
    assert_eq!(said.first(), Some(&(5, 15)), "{said:?}");
    assert!(got.contains("raw 21 20 line 20\n"));
    let tail = &lines[lines.len() - 2..];
    assert!(
        tail[0].starts_with("summary 31 30 raw downs/ups"),
        "{tail:?}"
    );
    assert!(tail[1].starts_with("pump 31 0 ran=0"), "{tail:?}");
    p.quit();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The earlier sessions' exit lines, one per session: never written (another file took the
/// name), saved by the rewrite at exit, or not saved.
#[test]
fn each_earlier_session_left_behind_gets_one_line() {
    let dir = std::path::Path::new("/s");
    let e = |retry| crate::session_writer::Earlier {
        name: "a.lmp".into(),
        error: "e1".into(),
        retry,
    };
    // the separator is the platform's (a backslash on Windows)
    let path = dir.join("a.lmp").display().to_string();
    assert_eq!(
        earlier_lines(&[e(None), e(Some(Ok(7))), e(Some(Err("e2".into())))], dir),
        [
            "an earlier session was not written: e1".to_string(),
            format!(
                "an earlier session was saved at exit: {path} (7 tics), rewritten whole after \
                 its last write failed (e1)"
            ),
            format!(
                "an earlier session was not saved: its last write to {path} failed (e1), and \
                 the rewrite at exit failed too (e2)"
            ),
        ]
    );
}

/// Review 3, m3: the log as doom-play makes it ([`InputLog::threaded`]) keeps at most
/// [`crate::input_log::LOG_QUEUE_PUMPS`] pumps waiting (1,024) besides the one being written:
/// with the file stalled for 1,100 pumps, the thread holds the first in its write, the next
/// 1,024 wait and the last 75 are dropped, counted in one line before the summary's (pump
/// 1,101). Review 6, X1: the test waits until the thread is held in the first pump's write
/// before it hands the others over, so the counts do not depend on when the thread first runs.
#[test]
fn the_logs_queue_holds_1024_pumps() {
    assert_eq!(crate::input_log::LOG_QUEUE_PUMPS, 1_024);
    let gate = Gate::closed();
    let text = Text {
        gate: Some(Arc::clone(&gate)),
        ..Text::default()
    };
    let dir = scratch("close-log-default");
    let mut p = play(&dir, &Disk::default(), 1_000);
    let mut log = InputLog::threaded(Box::new(text.clone())).unwrap();
    let _open = gate.opens_on_drop();
    for k in 0..1_100 {
        log.pump_end(&p.app, 1, 0, false);
        if k == 0 {
            // the thread has taken the first pump and is held in its write
            gate.await_waiter();
        }
    }
    gate.set(true);
    log.summary(0);
    log.finish();
    let got = String::from_utf8(text.bytes.lock().unwrap().clone()).unwrap();
    let kept = got.lines().filter(|l| l.starts_with("pump ")).count();
    assert_eq!(kept, 1_025);
    let pumps: Vec<&str> = got
        .lines()
        .filter(|l| l.starts_with("pump "))
        .filter_map(|l| l.split(' ').nth(1))
        .collect();
    let first: Vec<String> = (1..=1_025).map(|n: u32| n.to_string()).collect();
    assert_eq!(pumps, first, "the first 1,025 pumps, in order");
    let dropped: Vec<&str> = got.lines().filter(|l| l.starts_with("dropped")).collect();
    assert_eq!(
        dropped,
        ["dropped 1101 0 75 lines of 75 pumps were not written: the log's writer was behind"]
    );
    p.quit();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A run as [`play`] makes it, over `disk` with whole rewrites panicking while `panic` is set.
fn play_panicking(dir: &std::path::Path, disk: &Disk, panic: &Arc<AtomicBool>) -> Play {
    let mut host = LoggedHost {
        host: writer(
            dir,
            TestIo {
                panic_rewrites: Arc::clone(panic),
                ..disk.io()
            },
        ),
        log: InputLog::off(),
    };
    let cfg = SessionConfig {
        tic_cap: 1_000,
        ..SessionConfig::default()
    };
    let app = App::new(load(), cfg, &mut host);
    let mut p = Play::new(app, host, Forward::default(), false);
    p.app.update(0, &mut p.host);
    p
}

/// One `update` as the window makes it ([`step`]) at monotonic time `now` (microseconds).
fn step_at(p: &mut Play, now: u64) -> Vec<String> {
    let ended_before = matches!(p.app.mode(), Mode::Ended(_));
    let u = p.app.update(now, &mut p.host);
    p.settle_after_update(ended_before);
    p.pump_end(u, false)
}

/// Plays until the session has recorded `n` tics, one tic per update (each at the time the app
/// says the next tic falls due, after a first update that starts a re-based clock; `now` the
/// time, continued), letting the writer keep up after each update.
fn play_to(p: &mut Play, now: &mut u64, n: usize) {
    while p.app.recorder().len() < n {
        let before = p.app.recorder().len();
        *now = p.app.next_tic_at().unwrap_or(*now + 1).max(*now);
        assert!(step_at(p, *now).is_empty());
        p.host.host.wait_idle();
        assert!(p.app.recorder().len() <= before + 1, "one tic per update");
    }
}

/// The tics a demo file holds (13 header bytes, 4 per tic, the marker).
fn tics_on_disk(path: &std::path::Path) -> u64 {
    (std::fs::read(path).unwrap().len() as u64 - 14) / 4
}

/// The exit lines without the session writer's counters line (its times vary).
fn without_counters(lines: &[String]) -> Vec<&str> {
    lines
        .iter()
        .filter(|l| !l.starts_with("session file writes: "))
        .map(String::as_str)
        .collect()
}

/// The owner's ruling on a writer-thread panic, through the window's run with a real app. A
/// (5 tics) is written whole and restarted from. B's append of tic 9 fails, the pause shows it,
/// and the app's retry at the restart fails too, so the writer leaves B behind holding 8 of its
/// 9 tics. C's append of tic 11 fails; the app hears of it at tic 12 and its retry at tic 13
/// hands a whole demo over, whose rewrite panics: C's file holds 10 of its 13 tics. The exit
/// report as a whole: the panic line, one line for B and one for C (the current session; it
/// replaces the app's own line), each by path with what its file holds, no "saved" line for
/// either, and nothing about A.
#[test]
fn a_writer_panic_at_exit_names_the_earlier_and_the_current_session() {
    let dir = scratch("close-panic-names");
    let disk = Disk::default();
    let panic = Arc::new(AtomicBool::new(false));
    let mut p = play_panicking(&dir, &disk, &panic);
    let mut now = 0;
    play_to(&mut p, &mut now, 5);
    let a = dir.join(p.app.session_name());
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    p.app.key(b'r', true, &mut p.host);
    p.host.host.wait_idle();
    play_to(&mut p, &mut now, 8);
    disk.fail(true, true);
    play_to(&mut p, &mut now, 9);
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    now += 1;
    assert!(step_at(&mut p, now).is_empty()); // the paused poll
    assert_eq!(
        p.app.save_status(),
        &SaveStatus::Failed("planted: disk full".into())
    );
    let b = dir.join(p.app.session_name());
    p.app.key(b'r', true, &mut p.host); // the retry fails; the writer leaves B behind
    p.host.host.wait_idle();
    disk.fail(false, false);
    play_to(&mut p, &mut now, 10);
    disk.fail(true, false);
    play_to(&mut p, &mut now, 11);
    disk.fail(false, false);
    panic.store(true, Ordering::SeqCst);
    play_to(&mut p, &mut now, 13); // tic 13's retry panics the writer
    let c = dir.join(p.app.session_name());
    p.quit();
    let lines = p.conclude();
    assert_eq!(
        without_counters(&lines),
        [
            "the session writer stopped unexpectedly (a panic); the sessions below were not \
             saved, and no rewrite at exit saved them"
                .to_string(),
            format!(
                "an earlier session was not saved: the session writer panicked; {} holds at \
                 least 8 of its 9 tics (its last completed write)",
                b.display()
            ),
            format!(
                "the session was not saved: the session writer panicked; {} holds at least 10 \
                 of its 13 tics (its last completed write)",
                c.display()
            ),
        ],
        "{lines:#?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("session file writes: ")),
        "{lines:#?}"
    );
    let a_name = a.file_name().unwrap().to_str().unwrap();
    assert!(!lines.iter().any(|l| l.contains(a_name)), "{lines:#?}");
    assert_eq!(
        (tics_on_disk(&a), tics_on_disk(&b), tics_on_disk(&c)),
        (5, 8, 10)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The owner's ruling, the other side, through the window's run: a panic that lost nothing (the
/// whole demo handed over, here by the test as the app's own save would, is what the file
/// already holds) names no session as not saved and still says the writer stopped. The app's
/// status says the writer stopped before writing it (writes handed over were not made); the
/// exit line about the session comes from the writer's report instead, which knows the file
/// holds every tic: "saved", and no "not saved" line.
#[test]
fn a_writer_panic_that_lost_nothing_says_saved_and_that_the_writer_stopped() {
    let dir = scratch("close-panic-nothing");
    let disk = Disk::default();
    let panic = Arc::new(AtomicBool::new(false));
    let mut p = play_panicking(&dir, &disk, &panic);
    let mut now = 0;
    play_to(&mut p, &mut now, 5);
    let (path, want) = session(&p, &dir);
    let name = p.app.session_name().to_string();
    panic.store(true, Ordering::SeqCst);
    assert_eq!(
        doom_game::app::Host::save(&mut p.host.host, &name, &want),
        Ok(())
    );
    p.host.host.wait_idle(); // returns once the writer has died
    p.quit();
    assert!(
        matches!(p.app.save_status(), SaveStatus::Failed(_)),
        "{:?}",
        p.app.save_status()
    );
    let lines = p.conclude();
    assert_eq!(
        without_counters(&lines),
        [
            format!("saved {} (5 tics)", path.display()),
            "the session writer stopped unexpectedly (a panic); every session file it was handed \
             holds its whole demo"
                .to_string(),
        ],
        "{lines:#?}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), want);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A session begun after the writer died has no file: its claim cannot be handed over. The exit
/// says so in one line, instead of the app's own line about it ("not saved: the session writer
/// stopped"), and says nothing about the earlier session, whose file holds every tic.
#[test]
fn a_session_begun_after_a_writer_panic_is_named_as_having_no_file() {
    let dir = scratch("close-panic-after");
    let disk = Disk::default();
    let panic = Arc::new(AtomicBool::new(false));
    let mut p = play_panicking(&dir, &disk, &panic);
    let mut now = 0;
    play_to(&mut p, &mut now, 5);
    let (path, want) = session(&p, &dir);
    let name = p.app.session_name().to_string();
    panic.store(true, Ordering::SeqCst);
    assert_eq!(
        doom_game::app::Host::save(&mut p.host.host, &name, &want),
        Ok(())
    );
    p.host.host.wait_idle(); // returns once the writer has died
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    p.app.key(b'r', true, &mut p.host);
    assert_eq!(p.app.session_name(), "", "the claim was refused");
    play_to(&mut p, &mut now, 3);
    p.quit();
    let lines = p.conclude();
    assert_eq!(
        without_counters(&lines),
        [
            "the session writer stopped unexpectedly (a panic); the sessions below were not \
             saved, and no rewrite at exit saved them",
            "the session was not saved, nor was any other session begun after the session writer \
             stopped: no file was created for them",
        ],
        "{lines:#?}"
    );
    assert!(!lines.iter().any(|l| l.contains(&name)), "{lines:#?}");
    assert_eq!(std::fs::read(&path).unwrap(), want);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The panic's lines for the cases the runs above do not reach: a session whose name another
/// file took (never written; the same words as its line when the writer does not die), as the
/// current or an earlier session, and a refused claim, worded for a current session with no
/// name (begun after the stop) or, defensively, with one.
#[test]
fn the_panic_lines_for_a_taken_name_and_a_refused_claim() {
    let dir = scratch("close-panic-taken");
    let report = Report {
        panicked: true,
        not_written: vec![crate::session_writer::NotWritten {
            name: "t.lmp".into(),
            recorded: 4,
            file: Some(OnDisk::Taken),
        }],
        refused: true,
        ..Report::default()
    };
    let taken = crate::session_writer::lost(&dir, "t.lmp");
    assert!(taken.starts_with(&dir.join("t.lmp").display().to_string()));
    let head = "the session writer stopped unexpectedly (a panic); the sessions below were not \
                saved, and no rewrite at exit saved them";
    assert_eq!(
        panic_lines(&report, &dir, ""),
        [
            head.to_string(),
            format!("an earlier session was not written: {taken}"),
            "the session was not saved, nor was any other session begun after the session writer \
             stopped: no file was created for them"
                .to_string(),
        ]
    );
    assert_eq!(
        panic_lines(&report, &dir, "t.lmp"),
        [
            head.to_string(),
            format!("the session was not written: {taken}"),
            "no session begun after the session writer stopped was saved: no file was created \
             for them"
                .to_string(),
        ]
    );
    assert!(
        panic_lines(&Report::default(), &dir, "").is_empty(),
        "no panic"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// [`play_panicking`] with `--tic-stats` on (the tic interval summary is an exit line before the
/// session's).
fn play_panicking_with_stats(dir: &std::path::Path, panic: &Arc<AtomicBool>) -> Play {
    let mut host = LoggedHost {
        host: writer(
            dir,
            TestIo {
                panic_rewrites: Arc::clone(panic),
                ..TestIo::default()
            },
        ),
        log: InputLog::off(),
    };
    let cfg = SessionConfig {
        tic_cap: 1_000,
        ..SessionConfig::default()
    };
    let app = App::new(load(), cfg, &mut host);
    let mut p = Play::new(app, host, Forward::default(), true);
    p.app.update(0, &mut p.host);
    p
}

/// Review 6, m3: after a panic, the exit lines before the session's (the tic-stats summary, the
/// device's lines, the layout line) are still printed first, whether the current session's line
/// is "saved" from the writer's report (its file holds every tic) or the panic's line names it.
/// Both runs record 5 tics and hand the whole demo over again, and its rewrite panics; in the
/// second the app then records 2 more tics, which the stopped writer cannot take.
#[test]
fn after_a_panic_the_lines_before_the_sessions_are_kept() {
    for more in [false, true] {
        let dir = scratch(&format!("close-panic-head-{more}"));
        let panic = Arc::new(AtomicBool::new(false));
        let mut p = play_panicking_with_stats(&dir, &panic);
        let mut now = 0;
        play_to(&mut p, &mut now, 5);
        let (path, want) = session(&p, &dir);
        let name = p.app.session_name().to_string();
        panic.store(true, Ordering::SeqCst);
        assert_eq!(
            doom_game::app::Host::save(&mut p.host.host, &name, &want),
            Ok(())
        );
        p.host.host.wait_idle(); // returns once the writer has died
        if more {
            play_to(&mut p, &mut now, 7);
        }
        p.quit();
        let head =
            crate::window::exit_lines_before_session(&p.app, &p.host.host.fs, p.stats.as_ref());
        assert!(
            head.first().is_some_and(|l| l.starts_with("tic-stats: ")),
            "{head:#?}"
        );
        let mut want_lines = head.clone();
        if more {
            want_lines.extend([
                "the session writer stopped unexpectedly (a panic); the sessions below were not \
                 saved, and no rewrite at exit saved them"
                    .to_string(),
                format!(
                    "the session was not saved: the session writer panicked; {} holds at least 5 \
                     of its 7 tics (its last completed write)",
                    path.display()
                ),
            ]);
        } else {
            want_lines.extend([
                format!("saved {} (5 tics)", path.display()),
                "the session writer stopped unexpectedly (a panic); every session file it was \
                 handed holds its whole demo"
                    .to_string(),
            ]);
        }
        let lines = p.conclude();
        let got: Vec<&str> = without_counters(&lines)
            .into_iter()
            .filter(|l| !l.starts_with("writes over "))
            .collect();
        assert_eq!(got, want_lines, "{lines:#?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Review 6, n1: after a panic, a session whose claim was handed over after the writer's drain
/// and failed (the app keeps no name for it) is the current session: [`panic_lines`] calls it
/// "the session" and the app's own line about it is not printed (one line per session). With a
/// name of its own, or without a panic, the app's name stands.
#[test]
fn after_a_panic_a_failed_last_claim_is_the_current_session() {
    let dir = scratch("close-panic-failed-claim");
    let p = play(&dir, &Disk::default(), 1_000);
    let mut report = Report {
        panicked: true,
        not_written: vec![
            crate::session_writer::NotWritten {
                name: "a.lmp".into(),
                recorded: 2,
                file: Some(OnDisk::Holds(1)),
            },
            crate::session_writer::NotWritten {
                name: "b.lmp".into(),
                recorded: 0,
                file: None,
            },
        ],
        failed_claim: Some("b.lmp".into()),
        ..Report::default()
    };
    let current = current_session(&report, "");
    assert_eq!(current, "b.lmp");
    assert!(matches!(
        after_panic(&report, &p.app, current, &dir),
        Current::Named
    ));
    let path = |name: &str| dir.join(name).display().to_string();
    assert_eq!(
        panic_lines(&report, &dir, current),
        [
            "the session writer stopped unexpectedly (a panic); the sessions below were not \
             saved, and no rewrite at exit saved them"
                .to_string(),
            format!(
                "an earlier session was not saved: the session writer panicked; {} holds at least \
                 1 of its 2 tics (its last completed write)",
                path("a.lmp")
            ),
            format!(
                "the session was not saved: the session writer panicked; {} was not created (no \
                 write of it completed)",
                path("b.lmp")
            ),
        ]
    );
    assert_eq!(current_session(&report, "c.lmp"), "c.lmp");
    report.panicked = false;
    assert_eq!(current_session(&report, ""), "");
    drop(p);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review 6, n1, through the exit report ([`Play::conclude`]): an app whose session has no name
/// (its claim failed: here the writer was closed before the app's first claim) and a report,
/// made by hand, of a panic whose last claim was handed over and failed. The exit names that
/// claim as "the session", once, and the app's own line about the session is not printed.
#[test]
fn the_exit_report_names_a_failed_last_claim_as_the_session() {
    let dir = scratch("close-panic-failed-claim-exit");
    let mut host = LoggedHost {
        host: writer(&dir, TestIo::default()),
        log: InputLog::off(),
    };
    host.host.close();
    let app = App::new(load(), SessionConfig::default(), &mut host);
    let mut p = Play::new(app, host, Forward::default(), false);
    p.app.update(0, &mut p.host);
    assert_eq!(p.app.session_name(), "", "the claim failed");
    assert!(p.app.recorded());
    p.closed = Some(Report {
        panicked: true,
        not_written: vec![crate::session_writer::NotWritten {
            name: "b.lmp".into(),
            recorded: 0,
            file: None,
        }],
        failed_claim: Some("b.lmp".into()),
        ..Report::default()
    });
    assert_eq!(
        without_counters(&p.conclude()),
        [
            "the session writer stopped unexpectedly (a panic); the sessions below were not \
             saved, and no rewrite at exit saved them"
                .to_string(),
            format!(
                "the session was not saved: the session writer panicked; {} was not created (no \
                 write of it completed)",
                dir.join("b.lmp").display()
            ),
        ]
    );
    drop(p);
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---- mut7: tests for the M5d mutation survivors in closing.rs ----

/// Pins `Play::settle_after_update`'s early return while the app is playing. Its doc comment:
/// "After the app's update: when it is not playing (and the run has not ended), settles the
/// session file into the app". The writer holds a failure the app never heard of; the run is
/// still open and the app is playing, so no settle: the status stays as it was.
#[test]
fn nothing_is_settled_into_an_app_that_is_playing() {
    let dir = scratch("close-mut7-playing");
    let disk = Disk::default();
    let mut p = play(&dir, &disk, 1_000);
    steady(&mut p, 1, 9);
    disk.fail(true, true);
    for k in 10..=40 {
        p.app.update(at(k), &mut p.host);
        p.host.host.wait_idle();
    }
    assert_eq!(p.app.mode(), &Mode::Playing);
    let before = p.app.save_status().clone();
    p.settle_after_update(false);
    assert_eq!(p.app.save_status(), &before, "playing: nothing settled");
    drop(p);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The underrun diagnostic (2026-11-21): after every update the loop tells the sound queue the
/// session's committed tics and whether the app is playing, also in the update in which the
/// session ends, so an underrun is recorded with the tic the loop last noted and an ended
/// session's queue running dry is told apart. A headless device sink stands in for the device;
/// Rust-only.
#[test]
fn every_update_tells_the_sound_queue_the_sessions_tic() {
    use crate::audio::{BUFFER_FRAMES, DeviceSink, Output, Volume};
    use doom_game::app::AudioSink as _;
    /// Two buffers in, far more asked: one underrun; what it recorded.
    fn underrun(p: &mut Play) -> (u64, bool) {
        let Output::Device(d) = &mut p.host.host.fs.audio else {
            unreachable!()
        };
        d.buffer(&[[1, 1]; BUFFER_FRAMES]);
        d.buffer(&[[1, 1]; BUFFER_FRAMES]);
        d.pull(&mut vec![0; 2 * 100_000]);
        let u = *d.underruns().last().expect("an underrun");
        (u.tic, u.playing)
    }
    let dir = scratch("underrun-tic");
    let disk = Disk::default();
    let mut p = play(&dir, &disk, 8);
    p.host.host.fs.audio = Output::Device(DeviceSink::headless(Volume::FULL));
    steady(&mut p, 1, 5);
    let tic = p.app.tic();
    assert!(tic > 0 && p.app.mode() == &Mode::Playing, "tics ran");
    assert_eq!(underrun(&mut p), (tic, true));
    // a later update moves the number on
    steady(&mut p, 6, 7);
    assert!(p.app.tic() > tic);
    assert_eq!(underrun(&mut p), (p.app.tic(), true));
    // the update in which the session ends (the tic cap) is noted too, as not playing
    let mut k = 8;
    while p.app.mode() == &Mode::Playing {
        assert!(k < 20, "the cap of 8 tics ends the session");
        step(&mut p, k);
        k += 1;
    }
    assert!(matches!(p.app.mode(), Mode::Ended(_)));
    assert_eq!(underrun(&mut p), (p.app.tic(), false));
    drop(p);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Pins the first guard of `after_panic`: "The app's line ... always for an unrecorded run", and
/// the app's line when the writer did not panic. A report whose `whole` holds the current
/// session would otherwise give the "saved" line.
#[test]
fn after_panic_gives_the_apps_line_unless_both_panicked_and_recorded() {
    let dir = scratch("close-mut7-after-panic-guard");
    let p = play(&dir, &Disk::default(), 1_000);
    let whole = vec![("a.lmp".to_string(), 3)];
    let mut report = Report {
        panicked: true,
        whole: whole.clone(),
        ..Report::default()
    };
    assert!(matches!(
        after_panic(&report, &p.app, "a.lmp", &dir),
        Current::Saved(_)
    ));
    report.panicked = false;
    assert!(matches!(
        after_panic(&report, &p.app, "a.lmp", &dir),
        Current::App
    ));
    drop(p);
    // an unrecorded run
    let mut host = LoggedHost {
        host: writer(&dir, TestIo::default()),
        log: InputLog::off(),
    };
    let cfg = SessionConfig {
        unrecorded_full_turning: true,
        ..SessionConfig::default()
    };
    let app = App::new(load(), cfg, &mut host);
    assert!(!app.recorded());
    let report = Report {
        panicked: true,
        whole,
        ..Report::default()
    };
    assert!(matches!(
        after_panic(&report, &app, "a.lmp", &dir),
        Current::App
    ));
    drop(host);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Pins the last test of `after_panic`: "[`panic_lines`]'s line when it names it, or when a claim
/// was refused and the session has no name". A refused claim with a named current session that
/// the report does not list, or a nameless current session without a refused claim, gets the
/// app's line.
#[test]
fn a_refused_claim_names_only_a_session_without_a_name() {
    let dir = scratch("close-mut7-refused");
    let p = play(&dir, &Disk::default(), 1_000);
    let mut report = Report {
        panicked: true,
        refused: true,
        ..Report::default()
    };
    assert!(matches!(
        after_panic(&report, &p.app, "", &dir),
        Current::Named
    ));
    assert!(matches!(
        after_panic(&report, &p.app, "x.lmp", &dir),
        Current::App
    ));
    report.refused = false;
    assert!(matches!(
        after_panic(&report, &p.app, "", &dir),
        Current::App
    ));
    drop(p);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Pins `writer_lines`: "one line per failure the app never heard of, except, after a panic, for
/// a session handed over to the writer: either [`panic_lines`] names it ... or its file holds
/// every tic recorded". A failure of a session the report lists neither way keeps its line; one
/// the report lists in `not_written` or `whole` has none.
#[test]
fn an_unreported_failure_has_a_line_unless_its_session_is_listed() {
    let listed = |name: &str| crate::session_writer::NotWritten {
        name: name.into(),
        recorded: 1,
        file: None,
    };
    let lines = |w: &Report| -> Vec<String> {
        writer_lines(w, None, false)
            .into_iter()
            .filter(|l| l.contains("was not fully written"))
            .collect()
    };
    let line = "the session file a.lmp was not fully written: disk full".to_string();
    let base = Report {
        unreported: vec![("a.lmp".into(), "disk full".into())],
        ..Report::default()
    };
    assert_eq!(lines(&base), std::slice::from_ref(&line));
    let other = Report {
        not_written: vec![listed("b.lmp")],
        whole: vec![("c.lmp".into(), 2)],
        ..base.clone_for_mut7()
    };
    assert_eq!(lines(&other), [line]);
    let named = Report {
        not_written: vec![listed("a.lmp")],
        ..base.clone_for_mut7()
    };
    assert!(lines(&named).is_empty());
    let whole = Report {
        whole: vec![("a.lmp".into(), 2)],
        ..base.clone_for_mut7()
    };
    assert!(lines(&whole).is_empty());
}

impl Report {
    /// A report with only the failures the app never heard of (`Report` is not `Clone`).
    fn clone_for_mut7(&self) -> Report {
        Report {
            unreported: self.unreported.clone(),
            ..Report::default()
        }
    }
}

/// `--tic-stats` ([`Play::update`]): a pause and the end screen between two sessions are not
/// intervals between tics. Five tics, a pause of 19 s, five more, a restart after 7 s on the
/// pause box and three more: three stretches of play and two breaks, 26 s and a little more
/// (each break also holds the 1/35 s from the update that starts the clock again to its first
/// tic); no interval is longer than a tic's, and a pause pressed and released between two
/// updates (the loop never sees the app paused) is a break as well.
#[test]
fn a_pause_and_a_restart_are_breaks_in_the_tic_statistics_not_intervals() {
    let dir = scratch("close-stats-breaks");
    let panic = Arc::new(AtomicBool::new(false));
    let mut p = play_panicking_with_stats(&dir, &panic);
    let tic = 1_000_000 / 35 + 1;
    let run = |p: &mut Play, now: &mut u64, tics: usize| {
        // the update that starts the clock runs no tic; then one tic per update
        assert_eq!(p.update(*now).ran, 0, "the clock starts");
        for _ in 0..tics {
            *now += tic;
            assert_eq!(p.update(*now).ran, 1);
            p.host.host.wait_idle();
        }
    };
    let mut now = 1_000;
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    assert_eq!(p.app.mode(), &Mode::Playing, "paused and resumed: re-based");
    run(&mut p, &mut now, 5);
    let s = p.stats.as_ref().unwrap().summary();
    assert!(
        s.contains("executed 5, dropped 0, wall 0.114 s (first to last tic)"),
        "{s}"
    );
    // a pause the loop sees: updates while paused run nothing
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    for _ in 0..3 {
        now += 6_000_000;
        assert_eq!(p.update(now).ran, 0, "paused");
    }
    now += 1_000_000;
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    run(&mut p, &mut now, 5);
    // a pause the loop never sees, between two updates
    now += 7_000_000;
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    p.app.key(b'r', true, &mut p.host);
    assert_eq!(p.app.mode(), &Mode::Playing, "restarted from the pause box");
    run(&mut p, &mut now, 3);
    let s = p.stats.as_ref().unwrap().summary();
    for want in [
        "executed 13, dropped 0, wall 0.286 s (3 stretches of play, each first to last tic; \
         the 2 breaks between them, pauses and end screens, 26.057 s, not counted) against \
         0.286 s expected",
        "min 28572, median 28572, p95 28572, p99 28572, max 28572",
        "more than 1 ms from 1/35 s: 0, more than 5 ms: 0",
    ] {
        assert!(s.contains(want), "{want} not in {s}");
    }
    p.quit();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `--tic-stats` ([`Play::update`]) at the two breaks the test above does not make: a pause
/// pressed and released between two updates in the middle of play (the loop never sees the app
/// paused; the clock is re-based), and the end screen between two sessions (the session ends
/// at its tic cap inside an update, updates on the end screen run nothing, R restarts). Four
/// tics, the unseen pause 3 s long, four more to the cap, 5 s on the end screen, three tics of
/// the next session: three stretches, two breaks, and no interval longer than a tic's.
#[test]
fn an_unseen_pause_and_an_end_screen_are_breaks_in_the_tic_statistics() {
    let dir = scratch("close-stats-end-screen");
    let mut p = play(&dir, &Disk::default(), 8);
    p.stats = Some(crate::tic_stats::TicStats::default());
    let tic = 1_000_000 / 35 + 1;
    let mut now = 0; // `play` made the update that starts the clock, at 0
    let run = |p: &mut Play, now: &mut u64, tics: usize| {
        for _ in 0..tics {
            *now += tic;
            assert_eq!(p.update(*now).ran, 1);
            p.host.host.wait_idle();
        }
    };
    run(&mut p, &mut now, 4);
    now += 3_000_000;
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    p.app.key(KEY_ESCAPE, true, &mut p.host);
    assert_eq!(p.app.mode(), &Mode::Playing, "paused and resumed unseen");
    assert_eq!(p.update(now).ran, 0, "the clock starts again");
    run(&mut p, &mut now, 4);
    assert_eq!(
        p.app.mode(),
        &Mode::Playing,
        "eight tics: at the cap, not over"
    );
    // the next update finds the cap: the session ends in it, on its end screen
    now += tic;
    let ended = p.update(now);
    p.settle_after_update(false);
    assert!(matches!(p.app.mode(), Mode::Ended(_)), "{:?}", p.app.mode());
    for _ in 0..3 {
        now += 1_000_000;
        assert_eq!(p.update(now).ran, 0, "the end screen");
        p.settle_after_update(true);
    }
    now += 2_000_000;
    p.app.key(b'r', true, &mut p.host);
    assert_eq!(p.app.mode(), &Mode::Playing, "restarted");
    assert_eq!(p.update(now).ran, 0, "the clock starts");
    run(&mut p, &mut now, 3);
    let s = p.stats.as_ref().unwrap().summary();
    let executed = 11 + ended.ran;
    let want = format!("executed {executed}, dropped 0, wall ");
    assert!(s.contains(&want), "{want} not in {s}");
    for want in [
        "(3 stretches of play, each first to last tic; the 2 breaks between them, pauses and end \
         screens, ",
        "min 28572, median 28572, p95 28572, p99 28572, max 28572",
        "more than 1 ms from 1/35 s: 0, more than 5 ms: 0",
    ] {
        assert!(s.contains(want), "{want} not in {s}");
    }
    p.quit();
    std::fs::remove_dir_all(&dir).unwrap();
}
