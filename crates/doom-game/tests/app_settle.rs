//! M5d review 3, M1: [`App::settle_save`], the host's later word on the session file. A host that
//! answers before it writes (doom-play's writer thread) tells the app afterwards, at a moment no
//! tic runs, whether the write landed. The status, the end screen and the pause overlay follow
//! it, and a failure is retried by restart, quit and the next tic exactly as any failed save is.
//! Headless, on the M5a AMP room with `m5b_gfx.wad` and a fake clock; Rust-only checks of the
//! app's save status and write schedule.

use doom_game::app::{
    App, GameData, Host, Mode, PICTURE_BYTES, SAVE_FAILED_LABEL, SaveStatus, Screen, SessionConfig,
    Timestamp,
};
use doom_render::GameMode;
use std::collections::BTreeMap;
use std::path::Path;

fn load() -> GameData {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    let wad = std::fs::read(root.join("m5a_wads/amp.wad")).expect("map WAD");
    let gfx = std::fs::read(root.join("m5b_gfx.wad")).expect("gfx WAD");
    GameData::load(&[&wad, &gfx], GameMode::Registered, 1, 2).expect("game data")
}

fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Call {
    Claim,
    Save,
    Append,
}

/// An in-memory directory that answers every write `Ok` (as a writer thread answers before it
/// writes) unless `fail` is set, and records the calls.
#[derive(Default)]
struct Answering {
    appends: bool,
    fail: bool,
    claims_fail: bool,
    files: BTreeMap<String, Vec<u8>>,
    calls: Vec<Call>,
}

impl Host for Answering {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, name: &str) -> bool {
        self.files.contains_key(name)
    }
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        self.calls.push(Call::Claim);
        if self.claims_fail {
            return Err("read-only directory".into());
        }
        if self.files.contains_key(name) {
            return Ok(false);
        }
        self.files.insert(name.into(), bytes.to_vec());
        Ok(true)
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.calls.push(Call::Save);
        if self.fail {
            return Err("disk full".into());
        }
        self.files.insert(name.into(), bytes.to_vec());
        Ok(())
    }
    fn appends(&self) -> bool {
        self.appends
    }
    fn append(&mut self, name: &str, at: u64, tail: &[u8; 5]) -> Result<bool, String> {
        self.calls.push(Call::Append);
        if self.fail {
            return Err("disk full".into());
        }
        let file = self.files.get_mut(name).expect("a claimed file");
        file.truncate(usize::try_from(at).unwrap());
        file.extend_from_slice(tail);
        Ok(true)
    }
}

/// Whether `text` is drawn in the picture (the app's font, any ink colour; as in `app.rs`).
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

/// Plays `tics` tics with no input on a session capped at `cap` tics.
fn play(host: &mut Answering, cap: usize, tics: u64) -> App {
    let cfg = SessionConfig {
        tic_cap: cap,
        ..SessionConfig::default()
    };
    let mut app = App::new(load(), cfg, host);
    app.update(0, host);
    for k in 1..=tics {
        app.update(at(k), host);
    }
    app
}

/// After a resume (the clock re-based at the first update), runs `n` tics, one per update.
fn resume_tics(app: &mut App, host: &mut Answering, n: u64) {
    let t0 = at(1_000);
    assert_eq!(app.update(t0, host).ran, 0);
    for k in 1..=n {
        assert_eq!(app.update(t0 + at(k), host).ran, 1);
    }
}

/// The reviewer's case: the session ends at its cap with the end save answered `Ok`, so the end
/// screen says DEMO SAVED; the host then learns that the write failed. The status becomes
/// Failed, the end screen is drawn again and says DEMO NOT SAVED with the reason, and a restart
/// rewrites the file whole before it claims the next session's file (M5b's retry at restart).
#[test]
fn a_failure_learnt_after_the_end_redraws_the_end_screen_and_is_retried_at_restart() {
    let mut host = Answering {
        appends: true,
        ..Answering::default()
    };
    let mut app = play(&mut host, 30, 31);
    assert!(matches!(app.mode(), Mode::Ended(_)));
    assert_eq!(app.save_status(), &SaveStatus::Saved(30));
    assert!(shows(&picture(&app), "DEMO SAVED:"));
    app.settle_save(Err("disk full".into()));
    assert_eq!(app.save_status(), &SaveStatus::Failed("disk full".into()));
    let pic = picture(&app);
    assert!(shows(&pic, "DEMO NOT SAVED: DISK FULL"));
    assert!(!shows(&pic, "DEMO SAVED:"));
    host.calls.clear();
    app.restart(&mut host);
    // the retry, then the next session's claim (its first name is taken: the same second)
    assert_eq!(host.calls.first(), Some(&Call::Save), "{:?}", host.calls);
    assert!(host.calls.len() > 1 && host.calls[1..].iter().all(|c| *c == Call::Claim));
}

/// The host's word that the file now holds every tic: an end screen that said DEMO NOT SAVED
/// says DEMO SAVED, the status is Saved with every recorded tic, and a restart or quit then
/// writes nothing for the session.
#[test]
fn a_write_learnt_to_have_landed_redraws_the_end_screen_as_saved() {
    let mut host = Answering {
        appends: true,
        ..Answering::default()
    };
    let mut app = play(&mut host, 30, 29);
    host.fail = true;
    app.update(at(30), &mut host);
    app.update(at(31), &mut host);
    assert!(matches!(app.mode(), Mode::Ended(_)));
    assert!(matches!(app.save_status(), SaveStatus::Failed(_)));
    assert!(shows(&picture(&app), "DEMO NOT SAVED: DISK FULL"));
    host.fail = false;
    app.settle_save(Ok(()));
    assert_eq!(app.save_status(), &SaveStatus::Saved(30));
    assert!(shows(&picture(&app), "DEMO SAVED:"));
    host.calls.clear();
    app.quit(&mut host);
    assert_eq!(host.calls, [], "the file holds every tic: nothing to write");
}

/// While paused: a failure learnt then shows on the pause overlay; a quit retries it; and if
/// play resumes instead, the next tic rewrites the file whole (not an append), as after a
/// failed append.
#[test]
fn a_failure_learnt_while_paused_shows_and_is_retried_by_quit_or_the_next_tic() {
    for quit in [true, false] {
        let mut host = Answering {
            appends: true,
            ..Answering::default()
        };
        let mut app = play(&mut host, 1_000, 20);
        app.key(doom_game::input::keys::KEY_ESCAPE, true, &mut host);
        assert_eq!(app.mode(), &Mode::Paused);
        assert!(!shows(&picture(&app), SAVE_FAILED_LABEL));
        app.settle_save(Err("disk full".into()));
        assert!(shows(&picture(&app), SAVE_FAILED_LABEL));
        host.calls.clear();
        if quit {
            app.quit(&mut host);
            assert_eq!(host.calls, [Call::Save]);
            assert_eq!(app.save_status(), &SaveStatus::Saved(20));
        } else {
            app.key(doom_game::input::keys::KEY_ESCAPE, true, &mut host);
            resume_tics(&mut app, &mut host, 2);
            assert_eq!(host.calls, [Call::Save, Call::Append]);
            assert_eq!(app.save_status(), &SaveStatus::Saved(22));
        }
    }
}

/// The retry cadence restarts from the host's word: after failed retries wait 35 tics, the
/// host's `Ok` makes the next tic append (the file is current), and its `Err` makes the next tic
/// retry at once.
#[test]
fn the_hosts_word_resets_the_retry_cadence() {
    for ok in [true, false] {
        let mut host = Answering {
            appends: true,
            ..Answering::default()
        };
        let mut app = play(&mut host, 1_000, 5);
        host.fail = true;
        app.update(at(6), &mut host); // the append fails
        app.update(at(7), &mut host); // the retry fails: the next waits 35 tics
        host.calls.clear();
        app.update(at(8), &mut host);
        assert_eq!(host.calls, [], "waiting for the cadence");
        host.fail = false;
        app.key(doom_game::input::keys::KEY_ESCAPE, true, &mut host);
        app.settle_save(if ok { Ok(()) } else { Err("late".into()) });
        app.key(doom_game::input::keys::KEY_ESCAPE, true, &mut host);
        resume_tics(&mut app, &mut host, 1);
        let want: &[Call] = if ok { &[Call::Append] } else { &[Call::Save] };
        assert_eq!(host.calls, want, "ok {ok}");
    }
}

/// Nothing changes for an unrecorded session (no file) or for a session whose file was never
/// created (every claim failed): the status stays as it was and no end screen is redrawn.
#[test]
fn an_unrecorded_or_never_created_session_ignores_the_hosts_word() {
    let cfg = SessionConfig {
        unrecorded_full_turning: true,
        tic_cap: 10,
        ..SessionConfig::default()
    };
    let mut host = Answering::default();
    let mut app = App::new(load(), cfg, &mut host);
    app.update(0, &mut host);
    app.settle_save(Err("x".into()));
    assert_eq!(app.save_status(), &SaveStatus::NotSaved);
    app.settle_save(Ok(()));
    assert_eq!(app.save_status(), &SaveStatus::NotSaved);

    let mut host = Answering {
        claims_fail: true,
        ..Answering::default()
    };
    let mut app = play(&mut host, 10, 11);
    assert!(matches!(app.mode(), Mode::Ended(_)));
    let before = app.save_status().clone();
    assert!(matches!(before, SaveStatus::Failed(_)));
    let pic = picture(&app);
    app.settle_save(Ok(()));
    assert_eq!(app.save_status(), &before);
    assert_eq!(picture(&app), pic);
}
