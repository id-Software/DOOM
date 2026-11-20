//! The session file settled into the app and the run's end (M5d review 3, M1, m2, m3), with a
//! real [`App`] and the real [`WriterHost`] over the session writer's test disk
//! ([`crate::session_writer::tests::TestIo`]). Rust-only checks of doom-play's file handling.

use super::*;
use crate::input_log::InputLog;
use crate::session_writer::tests::{Gate, Op, TestIo, at, load, scratch, writer};
use doom_game::app::{PICTURE_BYTES, SAVE_FAILED_LABEL, Screen, SessionConfig};
use doom_game::input::keys::KEY_ESCAPE;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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
            gate: self.gate.clone(),
            ..TestIo::default()
        }
    }
    fn fail(&self, appends: bool, rewrites: bool) {
        self.fail_appends.store(appends, Ordering::SeqCst);
        self.fail_rewrites.store(rewrites, Ordering::SeqCst);
    }
    fn gate(&self, open: bool) {
        self.gate.as_ref().expect("a gated disk").set(open);
    }
    fn synced_rewrites_of(&self, tics: u64) -> usize {
        let want = Op::Rewrite { synced: true, tics };
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
    }
    assert!(longest < Duration::from_millis(100), "{longest:?}");
    gate.set(true);
    // the thread empties the queue: wait until the file stops growing (at most 5 s)
    let mut seen = 0;
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(50));
        let now = text.bytes.lock().unwrap().len();
        if now > 0 && now == seen {
            break;
        }
        seen = now;
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
    // 1 to 20 went over with the file stalled (the thread holds at most one, four wait), so at
    // least 15 of them are dropped and said so; line 20, after the file answered, is kept.
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
    let before_20: u64 = said
        .iter()
        .filter(|(from, _)| *from < 20)
        .map(|(_, n)| n)
        .sum();
    assert!(before_20 >= 15, "{said:?}");
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
    assert_eq!(
        earlier_lines(&[e(None), e(Some(Ok(7))), e(Some(Err("e2".into())))], dir),
        [
            "an earlier session was not written: e1".to_string(),
            "an earlier session was saved at exit: /s/a.lmp (7 tics), rewritten whole after its \
             last write failed (e1)"
                .to_string(),
            "an earlier session was not saved: its last write to /s/a.lmp failed (e1), and the \
             rewrite at exit failed too (e2)"
                .to_string(),
        ]
    );
}

/// Review 3, m3: the log as doom-play makes it ([`InputLog::threaded`]) keeps at most
/// [`crate::input_log::LOG_QUEUE_PUMPS`] pumps waiting (1,024) besides the one being written:
/// with the file stalled for 1,100 pumps, the rest are dropped and counted.
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
    for _ in 0..1_100 {
        log.pump_end(&p.app, 1, 0, false);
    }
    gate.set(true);
    log.summary(0);
    log.finish();
    let got = String::from_utf8(text.bytes.lock().unwrap().clone()).unwrap();
    let kept = got.lines().filter(|l| l.starts_with("pump ")).count();
    assert!(kept == 1_024 || kept == 1_025, "{kept}");
    let dropped = 1_100 - kept;
    assert!(
        got.contains(&format!(
            "{dropped} lines of {dropped} pumps were not written"
        )),
        "{}",
        got.lines()
            .find(|l| l.starts_with("dropped"))
            .unwrap_or("no dropped line")
    );
    p.quit();
    std::fs::remove_dir_all(&dir).unwrap();
}
