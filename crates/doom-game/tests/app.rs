//! Headless sessions of the app on the repository's synthetic maps (with
//! `tests/reference/m5b_gfx.wad`, the M5a graphics with the status bar and HUD lumps), driven
//! by scripted events and a fake monotonic clock: no window, no file system inside the app, no
//! wall clock.

use doom_game::app::FrameRefusal;
use doom_game::app::{
    App, DemoCommands, Ending, GAMMA, GameData, Host, MAX_NAME_SUFFIX, MAX_TICS_PER_UPDATE, Mode,
    PICTURE_BYTES, SAVE_FAILED_LABEL, SAVE_INTERVAL, SaveStatus, Screen, SessionConfig, TIC_CAP,
    TicReport, Timestamp, UNRECORDED_LABEL, Witness, replay, replay_full_resolution, session_name,
    wire24_admits,
};
use doom_game::input::{MouseButton, keys};
use doom_render::{DisplayRefusal, GameMode, RenderRefusal};
use doom_sim::demo::parse_demo_episode1_v2;
use doom_sim::{SimError, TicCommand};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

fn load(map_wad: &str, map: u8, skill: u8) -> GameData {
    let wad = std::fs::read(reference(map_wad)).expect("map WAD");
    let gfx = std::fs::read(reference("m5b_gfx.wad")).expect("gfx WAD");
    GameData::load(&[&wad, &gfx], GameMode::Registered, map, skill).expect("game data")
}

/// The start of tic `k` on the fake clock (µs): the first instant at which 35·t/10⁶ ≥ k.
fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

/// A fake host: an in-memory sessions directory and an injected date.
#[derive(Default)]
struct FakeHost {
    stamp: Timestamp,
    files: BTreeMap<String, Vec<u8>>,
    writes: usize,
    fail: bool,
    reports: Vec<(u64, TicCommand, u64, u8, u8)>,
}

impl Host for FakeHost {
    fn timestamp(&mut self) -> Timestamp {
        self.stamp
    }
    fn exists(&mut self, name: &str) -> bool {
        self.files.contains_key(name)
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        if self.fail {
            return Err("disk full".into());
        }
        self.writes += 1;
        self.files.insert(name.to_string(), bytes.to_vec());
        Ok(())
    }
    fn tic(&mut self, r: &TicReport<'_>) {
        let f = fnv(r.frame.pixels());
        let e = (r.tic, r.executed, f, r.display.palette, r.rndindex);
        self.reports.push(e);
    }
}

/// One scripted input.
#[derive(Clone, Copy, Debug)]
enum In {
    Key(u8, bool),
    Motion(i32, i32),
    Button(MouseButton, bool),
}

/// A session driven one tic per update, with per-tic observations.
struct Driver {
    app: App,
    host: FakeHost,
    clock: u64,
    frames: Vec<(u64, u8)>,
    digests: Vec<u64>,
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
    })
}

impl Driver {
    fn new(data: GameData, cfg: SessionConfig) -> Self {
        let mut host = FakeHost {
            stamp: Timestamp {
                year: 2026,
                month: 11,
                day: 17,
                hour: 9,
                minute: 5,
                second: 7,
            },
            ..FakeHost::default()
        };
        let mut app = App::new(data, cfg, &mut host);
        assert_eq!(
            app.update(0, &mut host).ran,
            0,
            "the first update starts the clock"
        );
        assert!(app.frame().is_none(), "nothing is drawn before tic 1");
        Self {
            app,
            host,
            clock: 0,
            frames: Vec::new(),
            digests: Vec::new(),
        }
    }

    fn feed(&mut self, input: In) {
        match input {
            In::Key(k, d) => self.app.key(k, d, &mut self.host),
            In::Motion(x, y) => self.app.mouse_motion(x, y),
            In::Button(b, d) => self.app.mouse_button(b, d),
        }
    }

    /// Advances the clock to the next tic boundary and updates: exactly one tic while playing.
    fn tic(&mut self) {
        self.clock += 1;
        let before = self.app.tic();
        let u = self.app.update(at(self.clock), &mut self.host);
        assert_eq!(u.dropped, 0);
        if self.app.tic() > before {
            assert_eq!(u.ran, 1);
            let (frame, palette) = self.app.frame().expect("a drawn frame");
            self.frames.push((fnv(frame), palette));
            self.digests.push(self.app.simulation().snapshot().digest());
        }
    }

    /// Runs `script`: (tic count, inputs fed before the first of those tics).
    fn run(&mut self, script: &[(u64, &[In])]) {
        for (tics, inputs) in script {
            for i in *inputs {
                self.feed(*i);
            }
            for _ in 0..*tics {
                self.tic();
            }
        }
    }

    fn demo(&self) -> &[u8] {
        &self.host.files[self.app.session_name()]
    }
}

const UP: u8 = keys::KEY_UPARROW;
const FIRE: u8 = keys::KEY_RCTRL;

/// A walk with keyboard turns, mouse motion, a mouse fire and the use key on the M5a AMP room.
fn walk_script() -> Vec<(u64, &'static [In])> {
    vec![
        (3, &[]),
        (20, &[In::Key(UP, true)]),
        (8, &[In::Key(keys::KEY_LEFTARROW, true)]),
        (
            4,
            &[In::Key(keys::KEY_LEFTARROW, false), In::Motion(-37, 5)],
        ),
        (
            2,
            &[In::Button(MouseButton::Left, true), In::Motion(90, -12)],
        ),
        (
            6,
            &[In::Button(MouseButton::Left, false), In::Key(UP, false)],
        ),
        (
            5,
            &[
                In::Key(b' ', true),
                In::Key(keys::KEY_RSHIFT, true),
                In::Key(UP, true),
            ],
        ),
        (10, &[In::Key(b' ', false), In::Motion(3, 0)]),
    ]
}

/// The demo's commands.
fn commands(demo: &[u8], map: u8) -> Vec<TicCommand> {
    parse_demo_episode1_v2(demo, map)
        .expect("a wire-24 demo")
        .1
        .commands
}

/// The commands of a committed M4b demo.
fn lmp_commands(name: &str, map: u8) -> Vec<TicCommand> {
    let bytes = std::fs::read(reference(&format!("m4b_scripts/{name}.lmp"))).expect("lmp");
    commands(&bytes, map)
}

#[test]
fn same_events_and_clock_give_the_same_session_twice() {
    let run = || {
        let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
        d.run(&walk_script());
        d.app.quit(&mut d.host);
        (d.frames.clone(), d.digests.clone(), d.demo().to_vec())
    };
    let (a, b) = (run(), run());
    assert_eq!(a, b);
    let (frames, _, demo) = a;
    assert_eq!(frames.len(), 58);
    // the pickup of tic 18 shows the first bonus palette for six frames (st_stuff.c:1036-1043)
    let palettes: Vec<u8> = frames.iter().map(|f| f.1).collect();
    assert_eq!(palettes[17..23], [10; 6]);
    assert!(
        palettes
            .iter()
            .enumerate()
            .all(|(i, p)| *p == 0 || (17..23).contains(&i))
    );
    // the walk is visible: the frames are not all the same
    assert!(frames.windows(2).any(|w| w[0].0 != w[1].0));
    assert_eq!(demo.len(), 13 + 4 * 58 + 1);
}

#[test]
fn the_written_demo_replays_to_the_same_state_at_every_tic() {
    let data = load("m5a_wads/amp.wad", 1, 2);
    let mut sim = data.start().clone();
    let mut d = Driver::new(data, SessionConfig::default());
    d.run(&walk_script());
    d.app.quit(&mut d.host);
    assert!(d.app.quit_requested());
    let cmds = commands(d.demo(), 1);
    assert_eq!(cmds.len(), d.digests.len());
    // the mouse turn and the keyboard turn were rounded to demo resolution and executed so
    assert!(cmds.iter().any(|c| c.angle_turn != 0));
    assert!(
        cmds.iter()
            .all(|c| c.angle_turn & 0xff == 0 && wire24_admits(c))
    );
    assert!(cmds.iter().any(|c| c.buttons & 1 != 0) && cmds.iter().any(|c| c.buttons & 2 != 0));
    let none = TicCommand::default();
    for (i, c) in cmds.iter().enumerate() {
        sim.advance([*c, none, none, none]).expect("replay");
        assert_eq!(sim.snapshot().digest(), d.digests[i], "tic {i}");
    }
    assert_eq!(d.app.save_status(), &SaveStatus::Saved(cmds.len()));
    // every played tic was reported once, and the replay of the demo reports the same tics,
    // commands, frames, palettes and M_Random indices
    let live = &d.host.reports;
    assert_eq!(
        live.iter().map(|r| r.0).collect::<Vec<_>>(),
        (0..58).collect::<Vec<_>>()
    );
    let mut again = FakeHost::default();
    assert_eq!(
        replay(
            &load("m5a_wads/amp.wad", 1, 2),
            10,
            &DemoCommands::parse(d.demo(), 1).expect("demo"),
            &mut again
        ),
        None
    );
    assert_eq!(&again.reports, live);
}

#[test]
fn a_slow_frame_runs_five_tics_and_drops_the_surplus() {
    let mut host = FakeHost::default();
    let mut app = App::new(
        load("m5a_wads/amp.wad", 1, 2),
        SessionConfig::default(),
        &mut host,
    );
    app.key(UP, true, &mut host);
    assert_eq!(app.update(1_000, &mut host).ran, 0); // the clock starts at 1 ms
    let u = app.update(1_000 + at(1) - 1, &mut host);
    assert_eq!((u.ran, u.dropped), (0, 0), "1 µs before the first tic");
    let u = app.update(1_000 + at(1), &mut host);
    assert_eq!((u.ran, u.dropped, app.tic()), (1, 0, 1));
    assert!(app.frame().is_some(), "the first frame appears after tic 1");
    // exactly 5 due: all run
    let u = app.update(1_000 + at(6), &mut host);
    assert_eq!((u.ran, u.dropped, app.tic()), (MAX_TICS_PER_UPDATE, 0, 6));
    // 6 due: 5 run, 1 dropped for good
    let u = app.update(1_000 + at(12), &mut host);
    assert_eq!((u.ran, u.dropped, app.tic()), (5, 1, 11));
    // 30 tics late: 5 run, 25 dropped; the next tic is due one tic after that frame's tic
    let u = app.update(1_000 + at(42), &mut host);
    assert_eq!((u.ran, u.dropped, app.tic()), (5, 25, 16));
    assert_eq!(app.next_tic_at(), Some(1_000 + at(43)));
    let u = app.update(1_000 + at(43), &mut host);
    assert_eq!((u.ran, u.dropped, app.tic()), (1, 0, 17));
    // every executed tic was built with the key held
    assert!(app.recorder().commands().iter().all(|c| c.forward == 25));
}

#[test]
fn the_tic_clock_has_no_drift() {
    let mut host = FakeHost::default();
    let mut app = App::new(
        load("m5a_wads/amp.wad", 1, 2),
        SessionConfig::default(),
        &mut host,
    );
    let origin = 123_456_789;
    app.update(origin, &mut host);
    // updates every 7 ms for 10 s: exactly 350 tics, never more than one per update
    let mut t = origin;
    while t < origin + 10_000_000 {
        t += 7_000;
        assert!(app.update(t.min(origin + 10_000_000), &mut host).ran <= 1);
    }
    assert_eq!(app.tic(), 350);
    for k in 351..=360u64 {
        assert_eq!(app.update(origin + at(k) - 1, &mut host).ran, 0, "tic {k}");
        assert_eq!(app.update(origin + at(k), &mut host).ran, 1, "tic {k}");
        assert_eq!(app.tic(), k);
    }
}

#[test]
fn pause_freezes_tics_releases_keys_and_draws_only_on_the_picture() {
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    d.run(&[(10, &[In::Key(UP, true), In::Key(FIRE, true)])]);
    let frame = d.app.frame().expect("frame").0.to_vec();
    let mut playing = vec![0; PICTURE_BYTES];
    d.app.picture(0, &mut playing);
    d.feed(In::Key(keys::KEY_PAUSE, true));
    assert_eq!(d.app.mode(), &Mode::Paused);
    assert!(!d.app.wants_mouse_capture());
    // input while paused is ignored; a minute of clock runs no tic
    d.feed(In::Key(keys::KEY_DOWNARROW, true));
    d.feed(In::Motion(100, 0));
    for k in 0..35 * 60 {
        assert_eq!(d.app.update(at(d.clock + k), &mut d.host).ran, 0);
    }
    d.clock += 35 * 60;
    assert_eq!(d.app.tic(), 10);
    assert_eq!(d.app.frame().expect("frame").0, frame.as_slice());
    let mut paused = vec![0; PICTURE_BYTES];
    d.app.picture(0, &mut paused);
    assert_ne!(paused, playing, "the overlay is on the presented picture");
    // resume with Esc: the clock is re-based, the first tic is due a tic later
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    assert_eq!(d.app.mode(), &Mode::Playing);
    let mut again = vec![0; PICTURE_BYTES];
    d.app.picture(0, &mut again);
    assert_eq!(again, playing, "the game frame was untouched by the pause");
    let resumed = at(d.clock) + 5;
    assert_eq!(d.app.update(resumed, &mut d.host).ran, 0);
    let next = d.app.next_tic_at().expect("clock running");
    assert_eq!(next, resumed + at(1), "1/35 s after the resume, rounded up");
    assert_eq!(d.app.update(next - 1, &mut d.host).ran, 0);
    assert_eq!(d.app.update(next, &mut d.host).ran, 1);
    assert_eq!(d.app.tic(), 11);
    let cmds = d.app.recorder().commands();
    assert_eq!((cmds[9].forward, cmds[9].buttons), (25, 1));
    assert_eq!(
        (cmds[10].forward, cmds[10].buttons),
        (0, 0),
        "keys were released"
    );
    // a minimise also pauses
    d.app.focus_lost();
    assert_eq!(d.app.mode(), &Mode::Paused);
}

#[test]
fn the_level_end_reaches_its_screen_with_the_demo_saved_to_the_exit_tic() {
    // BOSS1's authored commands (tests/reference/m4b_scripts/boss1.txt) as key presses: the
    // up arrow held for 60 tics is `cmd 60 25 0 0 0`.
    let up: &[In] = &[In::Key(UP, true)];
    let release: &[In] = &[In::Key(UP, false)];
    let mut d = Driver::new(load("m4b_wads/boss8.wad", 8, 2), SessionConfig::default());
    d.run(&[
        (60, up),
        (60, release),
        (60, up),
        (30, release),
        (60, up),
        (60, release),
        (70, up),
        (200, release),
    ]);
    assert_eq!(d.app.mode(), &Mode::Ended(Ending::Exit { secret: false }));
    assert!(!d.app.wants_mouse_capture());
    let saved = commands(d.demo(), 8);
    assert_eq!(saved.len(), d.app.tic() as usize);
    assert_eq!(
        saved.len(),
        545,
        "the exit tic (dump tic 544) is the last one"
    );
    assert_eq!(saved, lmp_commands("BOSS1", 8)[..545].to_vec());
    let mut pic = vec![0; PICTURE_BYTES];
    d.app.picture(0, &mut pic);
    assert_eq!(&pic[..4], &[150, 160, 190, 255], "the app's own screen");
    // no more tics, and the screen offers restart
    assert_eq!(d.app.update(at(d.clock + 100), &mut d.host).ran, 0);
    d.feed(In::Key(b'r', true));
    assert_eq!((d.app.mode(), d.app.tic()), (&Mode::Playing, 0));
}

#[test]
fn a_refused_tic_reaches_its_screen_with_the_demo_saved_to_the_last_good_tic() {
    // SPECH9 (tests/reference/m4b_scripts/spech9.txt): the walk whose tic 75 the genuine
    // engine overruns spechit in; the simulation refuses it (M4b manifest).
    let mut d = Driver::new(load("m4b_wads/spech9.wad", 1, 2), SessionConfig::default());
    d.run(&[(68, &[In::Key(UP, true)]), (20, &[In::Key(UP, false)])]);
    let Mode::Ended(Ending::Refused { tic, witness }) = d.app.mode().clone() else {
        panic!("not refused: {:?}", d.app.mode());
    };
    assert_eq!(tic, 75);
    assert!(matches!(witness, Witness::Simulation(_)), "{witness:?}");
    let saved = commands(d.demo(), 1);
    assert_eq!(saved.len(), 75);
    assert_eq!(saved, lmp_commands("SPECH9", 1));
    assert_eq!(d.app.save_status(), &SaveStatus::Saved(75));
}

#[test]
fn death_reaches_its_screen_with_the_demo_saved_to_the_last_good_tic() {
    // BARON1's first walk (tests/reference/m4b_scripts/baron1.txt), then standing still in
    // front of the Baron until dead; then the use key (the reborn).
    let mut d = Driver::new(load("m4b_wads/baron.wad", 1, 2), SessionConfig::default());
    d.run(&[(42, &[In::Key(UP, true)]), (1, &[In::Key(UP, false)])]);
    let mut n = 0;
    while d
        .app
        .simulation()
        .original_player(doom_sim::PlayerSlot::One)
        .expect("player")
        .health
        > 0
    {
        d.tic();
        n += 1;
        assert!(n < 35 * 120, "the player did not die");
    }
    let dead_at = d.app.tic();
    d.run(&[(40, &[]), (3, &[In::Key(b' ', true)])]);
    assert_eq!(d.app.mode(), &Mode::Ended(Ending::Died));
    let saved = commands(d.demo(), 1);
    assert_eq!(saved.len() as u64, d.app.tic());
    assert!(d.app.tic() > dead_at + 40);
    assert_eq!(
        saved.last().map(|c| c.buttons),
        Some(2),
        "the use press is the last tic"
    );
}

#[test]
fn restart_is_a_fresh_session_identical_to_a_fresh_start() {
    let script = walk_script();
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    d.run(&script[..4]);
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    d.host.stamp.second += 1;
    let first = d.app.session_name().to_string();
    d.feed(In::Key(b'r', true));
    assert_eq!(
        commands(&d.host.files[&first], 1).len(),
        35,
        "the old session was saved"
    );
    assert_ne!(d.app.session_name(), first);
    assert_eq!((d.app.mode(), d.app.tic()), (&Mode::Playing, 0));
    assert!(d.app.frame().is_none());
    d.frames.clear();
    d.digests.clear();
    assert_eq!(d.app.update(at(d.clock), &mut d.host).ran, 0);
    d.run(&script);
    d.app.quit(&mut d.host);
    let mut fresh = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    fresh.run(&script);
    fresh.app.quit(&mut fresh.host);
    assert_eq!(d.frames, fresh.frames);
    assert_eq!(d.digests, fresh.digests);
    assert_eq!(d.demo(), fresh.demo());
}

#[test]
fn the_tic_cap_ends_the_session_with_the_demo_saved() {
    let cfg = SessionConfig {
        tic_cap: 12,
        ..SessionConfig::default()
    };
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), cfg);
    d.run(&[(12, &[In::Key(UP, true)])]);
    assert_eq!(d.app.mode(), &Mode::Playing);
    d.tic();
    assert_eq!(d.app.mode(), &Mode::Ended(Ending::TicCap));
    assert_eq!(commands(d.demo(), 1).len(), 12);
    assert_eq!(TIC_CAP, 1_048_576);
    assert_eq!(SessionConfig::default().tic_cap, TIC_CAP);
}

#[test]
fn the_demo_is_rewritten_every_thirty_seconds_and_a_failed_write_is_reported() {
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    assert_eq!(
        d.host.writes, 1,
        "the file is created when the session begins"
    );
    assert_eq!(commands(d.demo(), 1).len(), 0);
    for _ in 0..SAVE_INTERVAL - 1 {
        d.tic();
    }
    assert_eq!(d.host.writes, 1);
    d.tic();
    assert_eq!(d.host.writes, 2);
    assert_eq!(commands(d.demo(), 1).len(), SAVE_INTERVAL);
    d.host.fail = true;
    d.tic();
    d.app.quit(&mut d.host);
    assert_eq!(d.app.save_status(), &SaveStatus::Failed("disk full".into()));
    let before = d.app.tic();
    assert_eq!(d.app.update(at(d.clock + 3), &mut d.host).ran, 0);
    assert_eq!(d.app.tic(), before, "no tic runs after quit");
    d.host.fail = false;
    d.app.quit(&mut d.host);
    assert_eq!(d.host.writes, 3, "a failed save is tried again on quit");
    assert_eq!(d.app.save_status(), &SaveStatus::Saved(SAVE_INTERVAL + 1));
    assert_eq!(commands(d.demo(), 1).len(), SAVE_INTERVAL + 1);
    d.app.quit(&mut d.host);
    assert_eq!(d.host.writes, 3, "a saved session is not written again");
}

#[test]
fn a_failed_save_shows_on_the_pause_box_and_the_end_screen_and_is_retried_on_restart() {
    let cfg = SessionConfig {
        tic_cap: 3,
        ..SessionConfig::default()
    };
    let mut host = FakeHost {
        fail: true,
        ..FakeHost::default()
    };
    let mut ok = Driver::new(load("m5a_wads/amp.wad", 1, 2), cfg);
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), cfg);
    std::mem::swap(&mut d.host, &mut host);
    d.app = App::new(load("m5a_wads/amp.wad", 1, 2), cfg, &mut d.host);
    d.app.update(0, &mut d.host);
    assert!(matches!(d.app.save_status(), SaveStatus::Failed(_)));
    assert_eq!(d.app.session_name(), "", "no file was created");
    d.run(&[(1, &[])]);
    ok.run(&[(1, &[])]);
    for x in [&mut d, &mut ok] {
        x.feed(In::Key(keys::KEY_ESCAPE, true));
    }
    assert!(shows(&picture(&d.app), SAVE_FAILED_LABEL));
    assert!(!shows(&picture(&ok.app), SAVE_FAILED_LABEL));
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    d.run(&[(5, &[])]);
    assert_eq!(d.app.mode(), &Mode::Ended(Ending::TicCap));
    d.host.fail = false;
    d.feed(In::Key(b'r', true));
    let first = session_name(1, 2, Timestamp::default());
    assert_eq!(
        commands(&d.host.files[&first], 1).len(),
        3,
        "saved on restart"
    );
    assert_eq!(
        d.app.session_name(),
        "E1M1-skill2-00000000-000000-2.lmp",
        "the new session never takes the old file's name"
    );
}

#[test]
fn file_names_come_from_the_injected_date_and_an_existing_file_is_never_replaced() {
    let t = Timestamp {
        year: 2026,
        month: 1,
        day: 2,
        hour: 3,
        minute: 4,
        second: 5,
    };
    assert_eq!(session_name(3, 2, t), "E1M3-skill2-20260102-030405.lmp");
    let mut host = FakeHost {
        stamp: t,
        ..FakeHost::default()
    };
    host.files
        .insert("E1M1-skill3-20260102-030405.lmp".into(), Vec::new());
    host.files
        .insert("E1M1-skill3-20260102-030405-2.lmp".into(), Vec::new());
    let app = App::new(
        load("m5a_wads/amp.wad", 1, 3),
        SessionConfig::default(),
        &mut host,
    );
    assert_eq!(app.session_name(), "E1M1-skill3-20260102-030405-3.lmp");
    assert_eq!(host.files["E1M1-skill3-20260102-030405-2.lmp"], b"");
    // every name taken (as with a clock stuck before 1970, which gives every session one
    // date): nothing is written, no file changes, and the failure is reported
    let all: BTreeMap<String, Vec<u8>> = (1..=MAX_NAME_SUFFIX)
        .map(|n| match n {
            1 => "E1M1-skill3-20260102-030405.lmp".to_string(),
            n => format!("E1M1-skill3-20260102-030405-{n}.lmp"),
        })
        .map(|name| (name, vec![7]))
        .collect();
    host.files = all.clone();
    let mut app = App::new(
        load("m5a_wads/amp.wad", 1, 3),
        SessionConfig::default(),
        &mut host,
    );
    app.quit(&mut host);
    assert_eq!(host.files, all);
    assert_eq!(app.session_name(), "");
    assert!(matches!(app.save_status(), SaveStatus::Failed(e) if e.contains("-9999.lmp exists")));
}

#[test]
fn only_admitted_commands_reach_the_simulation_and_each_equals_its_demo_bytes() {
    // a long pseudo-random stream of every kind of input, including withheld keys (not Esc
    // or Pause, which pause), through the app; every tic's executed command is the demo's
    let mut d = Driver::new(
        load("m5a_wads/amp.wad", 1, 2),
        SessionConfig {
            preset: doom_game::input::Preset::Modern,
            ..SessionConfig::default()
        },
    );
    let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = |n: u64| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x % n
    };
    let mut sim = d.app.simulation().clone();
    let none = TicCommand::default();
    for _ in 0..400 {
        for _ in 0..next(4) {
            let input = match next(4) {
                0 => In::Key(next(256) as u8, next(2) == 0),
                1 => In::Motion(next(2001) as i32 - 1000, next(2001) as i32 - 1000),
                2 => In::Button(
                    [MouseButton::Left, MouseButton::Middle, MouseButton::Right][next(3) as usize],
                    next(2) == 0,
                ),
                _ => In::Key(b"wasde1234567 ,."[next(15) as usize], next(2) == 0),
            };
            if !matches!(input, In::Key(keys::KEY_ESCAPE | keys::KEY_PAUSE, _)) {
                d.feed(input);
            }
        }
        if d.app.mode() != &Mode::Playing {
            break;
        }
        d.tic();
        let c = *d.app.recorder().commands().last().expect("a recorded tic");
        assert!(wire24_admits(&c) && c.angle_turn & 0xff == 0, "{c:?}");
        sim.advance([c, none, none, none]).expect("replay");
        assert_eq!(sim.snapshot().digest(), *d.digests.last().expect("digest"));
    }
    d.app.quit(&mut d.host);
    assert_eq!(commands(d.demo(), 1), d.app.recorder().commands());
    assert!(d.app.tic() > 100);
}

#[test]
fn keys_on_the_end_screen_and_quit() {
    let cfg = SessionConfig {
        tic_cap: 1,
        ..SessionConfig::default()
    };
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), cfg);
    d.run(&[(2, &[])]);
    assert_eq!(d.app.mode(), &Mode::Ended(Ending::TicCap));
    let writes = d.host.writes;
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    assert!(d.app.quit_requested());
    assert_eq!(
        d.host.writes, writes,
        "an ended session is not written again"
    );
}

#[test]
fn gamma_tables_are_the_c_tables() {
    let c = include_str!("../../../linuxdoom-1.10/v_video.c");
    let at = c.find("gammatable[5][256] =").expect("table");
    let body = &c[at..at + c[at..].find("};").expect("end")];
    let nums: Vec<u8> = body
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .skip(2)
        .map(|s| s.parse().expect("byte"))
        .collect();
    assert_eq!(nums, GAMMA.concat());
    assert_eq!((GAMMA[0][0], GAMMA[0][127], GAMMA[0][128]), (1, 128, 128));
}

#[test]
fn the_picture_uses_the_palette_through_the_gamma_table() {
    let data = load("m5a_wads/amp.wad", 1, 2);
    let pal = data.assets().palette(0).expect("palette 0").to_vec();
    let mut d = Driver::new(data, SessionConfig::default());
    let mut pic = vec![7; PICTURE_BYTES];
    d.app.picture(0, &mut pic);
    assert!(
        pic.chunks(4).all(|p| p == [0, 0, 0, 255]),
        "black before the first tic"
    );
    d.tic();
    let frame = d.app.frame().expect("frame").0.to_vec();
    for g in [0, 4] {
        d.app.picture(g, &mut pic);
        for i in [0, 31_999, 63_999] {
            let idx = usize::from(frame[i]) * 3;
            let want = [0, 1, 2].map(|k| GAMMA[g][usize::from(pal[idx + k])]);
            assert_eq!(&pic[i * 4..i * 4 + 3], &want);
        }
    }
}

#[test]
fn the_status_bar_and_a_bonus_palette_reach_the_presented_picture() {
    let data = load("m5a_wads/amp.wad", 1, 2);
    let (pal0, pal10) = (
        data.assets().palette(0).expect("palette 0").to_vec(),
        data.assets().palette(10).expect("palette 10").to_vec(),
    );
    let mut d = Driver::new(data, SessionConfig::default());
    d.run(&walk_script()[..2]);
    let (frame, palette) = d.app.frame().expect("frame");
    assert_eq!(
        (d.app.tic(), palette),
        (23, 10),
        "the pickup's bonus palette"
    );
    // the status bar fills rows 168-199 at screenblocks 10 (ST_Y, st_stuff.h:32-34); M5a's
    // view-only frame left them 0
    let bar = &frame[168 * 320..];
    assert!(bar.iter().filter(|&&c| c != 0).count() > 320 * 16);
    let frame = frame.to_vec();
    let mut pic = vec![0; PICTURE_BYTES];
    d.app.picture(0, &mut pic);
    let rgb = |pal: &[u8], i: usize| {
        let at = usize::from(frame[i]) * 3;
        [0, 1, 2].map(|k| GAMMA[0][usize::from(pal[at + k])])
    };
    assert!((0..64_000).all(|i| pic[i * 4..i * 4 + 3] == rgb(&pal10, i)));
    assert!((0..64_000).any(|i| rgb(&pal10, i) != rgb(&pal0, i)));
    // at screenblocks 11 the same tic has no status bar: the view covers the rows
    let mut full = Driver::new(
        load("m5a_wads/amp.wad", 1, 2),
        SessionConfig {
            screenblocks: 11,
            ..SessionConfig::default()
        },
    );
    full.run(&walk_script()[..2]);
    let (other, palette) = full.app.frame().expect("frame");
    assert_eq!(palette, 10);
    assert_ne!(&other[168 * 320..], &frame[168 * 320..]);
}

#[test]
fn the_load_refuses_outside_the_wire_24_domain() {
    let wad = std::fs::read(reference("m5a_wads/amp.wad")).expect("map WAD");
    let gfx = std::fs::read(reference("m5b_gfx.wad")).expect("gfx");
    for (map, skill) in [(1, 1), (1, 4), (0, 2), (10, 2)] {
        let e = GameData::load(&[&wad, &gfx], GameMode::Registered, map, skill).unwrap_err();
        assert!(e.to_string().contains("not playable"), "{e}");
    }
    assert!(
        GameData::load(&[&wad, &gfx], GameMode::Registered, 2, 2).is_err(),
        "no E1M2"
    );
    assert!(GameData::load(&[&wad[..20]], GameMode::Registered, 1, 2).is_err());
    let _ = SimError::LevelExited;
}

/// Decision S3 (ii): the opt-in unrecorded session at full turning resolution.
fn unrecorded() -> SessionConfig {
    SessionConfig {
        unrecorded_full_turning: true,
        ..SessionConfig::default()
    }
}

/// Whether `text` is drawn (in any one colour on a differently coloured box) in the RGBA
/// picture `pic`.
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

#[test]
fn the_default_session_is_recorded_and_unlabelled() {
    assert!(!SessionConfig::default().unrecorded_full_turning);
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    d.run(&walk_script());
    assert!(d.app.recorded());
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    assert!(!shows(&picture(&d.app), UNRECORDED_LABEL));
    d.app.quit(&mut d.host);
    assert_eq!(d.host.writes, 2, "created at the start, saved at quit");
}

#[test]
fn an_unrecorded_session_executes_the_built_command_and_writes_nothing() {
    let mut u = Driver::new(load("m5a_wads/amp.wad", 1, 2), unrecorded());
    u.run(&walk_script());
    assert!(!u.app.recorded());
    let mut r = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    r.run(&walk_script());
    let (full, demo): (Vec<_>, Vec<_>) = u
        .host
        .reports
        .iter()
        .zip(&r.host.reports)
        .map(|(a, b)| (a.1, b.1))
        .unzip();
    // the built turns, each executed whole: the left key held 8 tics turns 320 while
    // turnheld < SLOWTURNTICS (5 tics) and 640 after (g_game.c:177, 264-270, 298-305); the
    // mouse motions of -37, 90 and 3 pixels post data2 = 4·dx and turn -8·data2 at
    // sensitivity 5 (g_game.c:409, 579): +1184, -2880 and -96. Recorded, the same session
    // executes 256, 768, 1280, -2816 and 0 (g_game.c:1500, 1512).
    let turns: Vec<i16> = full
        .iter()
        .map(|c| c.angle_turn)
        .filter(|&a| a != 0)
        .collect();
    assert_eq!(
        turns,
        [320, 320, 320, 320, 320, 640, 640, 640, 1184, -2880, -96]
    );
    for (f, d) in full.iter().zip(&demo) {
        let round = ((((i32::from(f.angle_turn) + 128) >> 8) as u8 as u16) << 8) as i16;
        assert_eq!(
            (f.forward, f.side, round, f.buttons),
            (d.forward, d.side, d.angle_turn, d.buttons)
        );
        assert!(wire24_admits(f));
    }
    assert_ne!(u.digests, r.digests, "the rounding matters on this script");
    // a plain simulation fed the executed commands reaches the same state on every tic
    let mut again = FakeHost::default();
    assert_eq!(
        replay_full_resolution(&load("m5a_wads/amp.wad", 1, 2), 10, &full, &mut again),
        None
    );
    assert_eq!(again.reports, u.host.reports);
    // determinism
    let mut twice = Driver::new(load("m5a_wads/amp.wad", 1, 2), unrecorded());
    twice.run(&walk_script());
    assert_eq!(
        (twice.frames, twice.digests),
        (u.frames.clone(), u.digests.clone())
    );
    // the pause overlay carries the label; quit and restart write nothing
    u.feed(In::Key(keys::KEY_ESCAPE, true));
    assert!(shows(&picture(&u.app), UNRECORDED_LABEL));
    u.feed(In::Key(b'r', true));
    assert_eq!((u.app.mode(), u.app.tic()), (&Mode::Playing, 0));
    u.app.quit(&mut u.host);
    assert_eq!(
        (u.host.writes, u.host.files.len(), u.app.save_status()),
        (0, 0, &SaveStatus::NotSaved)
    );
}

#[test]
fn every_ending_of_an_unrecorded_session_writes_nothing_and_shows_the_label() {
    let up: &[In] = &[In::Key(UP, true)];
    let release: &[In] = &[In::Key(UP, false)];
    let cap = SessionConfig {
        tic_cap: 12,
        ..unrecorded()
    };
    type Run<'a> = (&'a str, u8, SessionConfig, Vec<(u64, &'a [In])>);
    let runs: [Run<'_>; 3] = [
        (
            "m4b_wads/boss8.wad",
            8,
            unrecorded(),
            vec![
                (60, up),
                (60, release),
                (60, up),
                (30, release),
                (60, up),
                (60, release),
                (70, up),
                (200, release),
            ],
        ),
        (
            "m4b_wads/spech9.wad",
            1,
            unrecorded(),
            vec![(68, up), (20, release)],
        ),
        ("m5a_wads/amp.wad", 1, cap, vec![(13, up)]),
    ];
    let mut endings = Vec::new();
    for (wad, map, cfg, script) in runs {
        let mut d = Driver::new(load(wad, map, 2), cfg);
        d.run(&script);
        endings.push(d.app.mode().clone());
        assert!(shows(&picture(&d.app), UNRECORDED_LABEL), "{wad}");
        d.feed(In::Key(b'q', true));
        assert!(d.app.quit_requested());
        assert_eq!((d.host.writes, d.host.files.len()), (0, 0), "{wad}");
    }
    assert_eq!(endings[0], Mode::Ended(Ending::Exit { secret: false }));
    assert!(matches!(
        endings[1],
        Mode::Ended(Ending::Refused {
            tic: 75,
            witness: Witness::Simulation(_)
        })
    ));
    assert_eq!(endings[2], Mode::Ended(Ending::TicCap));
    // death: BARON1's walk, then waiting until dead, then use
    let mut d = Driver::new(load("m4b_wads/baron.wad", 1, 2), unrecorded());
    d.run(&[(42, up), (1, release)]);
    for _ in 0..35 * 120 {
        if d.app
            .simulation()
            .original_player(doom_sim::PlayerSlot::One)
            .expect("player")
            .health
            <= 0
        {
            break;
        }
        d.tic();
    }
    d.run(&[(40, &[]), (3, &[In::Key(b' ', true)])]);
    assert_eq!(d.app.mode(), &Mode::Ended(Ending::Died));
    assert!(shows(&picture(&d.app), UNRECORDED_LABEL));
    assert_eq!((d.host.writes, d.host.files.len()), (0, 0));
}

#[test]
fn a_frame_refused_after_the_simulation_committed_its_tic_stays_in_the_demo() {
    // The M5a solidsegs room turned left 256 a tic (its demo M5SSEG): the genuine run ends at
    // tic 15 with SolidSegOverflow (tests/reference/m5b_results.json). A mouse motion of -8 is
    // a built turn of 256 at sensitivity 5 (g_game.c:579, 409), the same executed command.
    let data = load("m5a_wads/solidsegs.wad", 1, 2);
    let mut d = Driver::new(data.clone(), SessionConfig::default());
    d.run(&[(2, &[])]);
    let mut fed = 0;
    while d.app.mode() == &Mode::Playing {
        // bounded, so that a session that never refuses fails here and does not run to its cap
        assert!(fed < 60, "no refusal after {fed} tics");
        fed += 1;
        d.feed(In::Motion(-8, 0));
        d.tic();
    }
    let ending = Ending::Refused {
        tic: 15,
        witness: Witness::Frame(FrameRefusal::Draw(DisplayRefusal::Render(
            RenderRefusal::SolidSegOverflow,
        ))),
    };
    assert_eq!(d.app.mode(), &Mode::Ended(ending.clone()));
    let pic = picture(&d.app);
    assert!(shows(&pic, "TIC REFUSED") && shows(&pic, "TIC 15, WITNESS:"));
    assert!(shows(&pic, "16 TICS"));
    let demo = DemoCommands::parse(d.demo(), 1).expect("demo");
    assert_eq!(demo.commands().len(), 16, "the refused tic is in the demo");
    assert_eq!(demo.commands()[15].angle_turn, 256);
    assert_eq!(d.app.save_status(), &SaveStatus::Saved(16));
    let mut again = FakeHost::default();
    assert_eq!(replay(&data, 10, &demo, &mut again), Some(ending));
    assert_eq!(again.reports, d.host.reports);
}

#[test]
fn the_picture_fills_any_buffer_length_without_a_panic() {
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    d.run(&[(2, &[])]);
    let whole = picture(&d.app);
    let mut long = vec![9; PICTURE_BYTES + 6];
    d.app.picture(0, &mut long);
    assert_eq!(
        (&long[..PICTURE_BYTES], &long[PICTURE_BYTES..]),
        (&whole[..], &[9; 6][..])
    );
    let mut short = vec![0; 10];
    d.app.picture(0, &mut short);
    assert_eq!((&short[..8], &short[8..]), (&whole[..8], &[0, 0][..]));
}

// ---------------------------------------------------------------------------------------
// Tests added for the mutants of M5b step 11 (cargo-mutants survivors in the app). Each names
// what it pins.

/// Pins `GameData::map` and `GameData::skill` (values other than 0 and 1) and `PICTURE_BYTES`.
#[test]
fn the_game_data_reports_its_map_and_skill_and_the_picture_is_320_by_200_rgba() {
    let data = load("m4b_wads/boss8.wad", 8, 3);
    assert_eq!((data.map(), data.skill()), (8, 3));
    let data = load("m5a_wads/amp.wad", 1, 2);
    assert_eq!((data.map(), data.skill()), (1, 2));
    assert_eq!(PICTURE_BYTES, 320 * 200 * 4);
}

/// Pins `SAVE_INTERVAL`: the demo is rewritten every 30 seconds, 30 x 35 = 1050 tics at
/// `TICRATE` 35 (doomdef.h:122).
#[test]
fn the_save_interval_is_thirty_seconds_of_tics() {
    assert_eq!(SAVE_INTERVAL, 1050);
}

/// Pins `FrameAssets::palette`: palette n is bytes 768n..768(n+1) of the PLAYPAL lump of the
/// loaded WADs (the graphics WAD is loaded last and wins), for 0..=13, and nothing beyond.
#[test]
fn palette_n_is_the_nth_768_byte_block_of_playpal() {
    let gfx = std::fs::read(reference("m5b_gfx.wad")).expect("gfx WAD");
    let wad = doom_wad::Wad::parse(&gfx, doom_wad::Limits::default()).expect("WAD");
    let playpal = wad
        .data(wad.find("PLAYPAL").expect("PLAYPAL"))
        .expect("data");
    assert_eq!(playpal.len(), 14 * 768);
    let data = load("m5a_wads/amp.wad", 1, 2);
    for n in 0..14u8 {
        let at = usize::from(n) * 768;
        assert_eq!(
            data.assets().palette(n),
            Some(&playpal[at..at + 768]),
            "{n}"
        );
    }
    assert_eq!(data.assets().palette(14), None);
    assert_eq!(data.assets().palette(255), None);
}

/// Pins `FrameSource::rndindex`, through `Host::tic`: with no sound and no wipe the `ST_Ticker`
/// of tic t has seen `rndindex = (t + 1) & 0xff` (M5b investigation §1.5 item 5: `M_Random`
/// pre-increments `rndindex`, m_random.c:63-66, and `ST_Ticker` draws once per tic,
/// st_stuff.c:992), through the wrap at 256.
#[test]
fn the_reported_rndindex_after_tic_t_is_t_plus_one_modulo_256() {
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    for _ in 0..300 {
        d.tic();
    }
    assert_eq!(d.host.reports.len(), 300);
    for (tic, _, _, _, rndindex) in &d.host.reports {
        assert_eq!(u64::from(*rndindex), (tic + 1) & 0xff, "tic {tic}");
    }
}

/// Pins `App::wants_mouse_capture` (true while playing) and `App::quit_requested` (false until
/// quit).
#[test]
fn the_mouse_is_captured_while_playing_and_quit_is_off_until_asked() {
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    assert!(d.app.wants_mouse_capture());
    assert!(!d.app.quit_requested());
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    assert!(!d.app.wants_mouse_capture());
    assert!(!d.app.quit_requested());
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    assert!(d.app.wants_mouse_capture());
}

/// Pins `App::key` while paused: Esc or Pause (down) resumes, `q` (down) quits, `r` (down)
/// restarts; a release or any other key does nothing.
#[test]
fn keys_while_paused_act_only_on_their_presses() {
    let paused = || {
        let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
        d.run(&[(3, &[])]);
        d.feed(In::Key(keys::KEY_ESCAPE, true));
        assert_eq!(d.app.mode(), &Mode::Paused);
        d
    };
    for (code, down) in [
        (b'q', false),
        (b'x', true),
        (b'x', false),
        (b'Q', true),
        (b'r', false),
        (keys::KEY_ESCAPE, false),
    ] {
        let mut d = paused();
        d.feed(In::Key(code, down));
        assert_eq!(d.app.mode(), &Mode::Paused, "{code:#x} {down}");
        assert!(!d.app.quit_requested(), "{code:#x} {down}");
        assert_eq!(d.app.tic(), 3, "{code:#x} {down}");
    }
    let mut d = paused();
    d.feed(In::Key(b'q', true));
    assert!(d.app.quit_requested());
    assert_eq!(d.app.mode(), &Mode::Paused);
    let mut d = paused();
    d.feed(In::Key(b'r', true));
    assert_eq!((d.app.mode(), d.app.tic()), (&Mode::Playing, 0));
    let mut d = paused();
    d.feed(In::Key(keys::KEY_PAUSE, true));
    assert_eq!(d.app.mode(), &Mode::Playing);
}

/// Pins `App::key` on an end screen: `q` or Esc (down) quits, `r` (down) restarts; a release,
/// or any other key, does nothing.
#[test]
fn keys_on_the_end_screen_act_only_on_their_presses() {
    let ended = || {
        let cfg = SessionConfig {
            tic_cap: 1,
            ..SessionConfig::default()
        };
        let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), cfg);
        d.run(&[(2, &[])]);
        assert_eq!(d.app.mode(), &Mode::Ended(Ending::TicCap));
        d
    };
    for (code, down) in [
        (b'q', false),
        (keys::KEY_ESCAPE, false),
        (b'x', true),
        (b'x', false),
        (keys::KEY_PAUSE, true),
        (b'r', false),
    ] {
        let mut d = ended();
        d.feed(In::Key(code, down));
        assert!(!d.app.quit_requested(), "{code:#x} {down}");
        assert_eq!(
            d.app.mode(),
            &Mode::Ended(Ending::TicCap),
            "{code:#x} {down}"
        );
    }
    for code in [b'q', keys::KEY_ESCAPE] {
        let mut d = ended();
        d.feed(In::Key(code, true));
        assert!(d.app.quit_requested(), "{code:#x}");
    }
    let mut d = ended();
    d.feed(In::Key(b'r', true));
    assert_eq!(d.app.mode(), &Mode::Playing);
    assert!(!d.app.quit_requested());
}

/// Pins `App::mouse_motion`: motion while playing reaches the command and is ignored while paused.
/// A motion of dx = 30 posts data2 = 30 << 2 = 120 (i_video.c:250), `G_Responder` scales it by
/// (mouseSensitivity + 5) / 10 = 1 at the default 5 (g_game.c:579), `G_BuildTiccmd` subtracts
/// mousex * 8 = 960 from `angleturn` (g_game.c), and the demo round trip stores
/// (-960 + 128) >> 8 = -4 as the byte, which reads back as -4 << 8 = -1024 (g_game.c:1501, 1512).
#[test]
fn mouse_motion_turns_while_playing_and_is_ignored_while_paused() {
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    d.feed(In::Motion(30, 0));
    d.tic();
    assert_eq!(d.app.recorder().commands()[0].angle_turn, -1024);
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    d.feed(In::Motion(500, 0));
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    // the resume re-bases the clock: this update only starts it
    d.clock += 1;
    assert_eq!(d.app.update(at(d.clock), &mut d.host).ran, 0);
    d.tic();
    assert_eq!(d.app.recorder().commands()[1].angle_turn, 0);
}

/// Pins the loop of `App::update`: the update that runs the tic reaching the cap runs one more
/// call that ends the session (4 for a cap of 3), then stops (the loop does not go on with an
/// ended session) and reports the surplus due tics as dropped; the end writes the demo once.
#[test]
fn an_update_stops_at_the_tic_that_ends_the_session() {
    let cfg = SessionConfig {
        tic_cap: 3,
        ..SessionConfig::default()
    };
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), cfg);
    assert_eq!(d.host.writes, 1);
    let u = d.app.update(at(5), &mut d.host);
    assert_eq!(d.app.mode(), &Mode::Ended(Ending::TicCap));
    assert_eq!(d.app.tic(), 3);
    assert_eq!((u.ran, u.dropped), (4, 1));
    assert_eq!(d.host.writes, 2, "the end saved once");
    assert_eq!(d.app.save_status(), &SaveStatus::Saved(3));
}

/// Pins `App::end_screen` on E1M8: the exit of the last map of the episode says so.
#[test]
fn the_exit_of_e1m8_shows_the_episode_complete_heading() {
    let up: &[In] = &[In::Key(UP, true)];
    let release: &[In] = &[In::Key(UP, false)];
    let mut d = Driver::new(load("m4b_wads/boss8.wad", 8, 2), SessionConfig::default());
    d.run(&[
        (60, up),
        (60, release),
        (60, up),
        (30, release),
        (60, up),
        (60, release),
        (70, up),
        (200, release),
    ]);
    assert_eq!(d.app.mode(), &Mode::Ended(Ending::Exit { secret: false }));
    let pic = picture(&d.app);
    assert!(shows(&pic, "EPISODE 1 COMPLETE"));
    assert!(!shows(&pic, "LEVEL COMPLETE"));
}

/// Pins `App::picture`'s gamma clamp: levels above 4 use level 4 (decision S13), and level 3
/// differs from level 4 on a lit frame.
#[test]
fn gamma_levels_above_four_use_four() {
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    d.run(&[(2, &[])]);
    let at = |g: usize| {
        let mut pic = vec![0; PICTURE_BYTES];
        d.app.picture(g, &mut pic);
        pic
    };
    let four = at(4);
    assert_eq!(at(5), four);
    assert_eq!(at(6), four);
    assert_eq!(at(usize::MAX), four);
    assert_ne!(at(3), four);
}

/// Pins the pause overlay on the presented picture: the box's grey frame, dark inside and
/// yellow heading are in the app's palette, and the game picture around the box is shown at
/// half brightness (each colour channel halved, rounding down).
#[test]
fn the_pause_overlay_is_composed_on_a_half_bright_game_picture() {
    let mut d = Driver::new(load("m5a_wads/amp.wad", 1, 2), SessionConfig::default());
    d.run(&[(2, &[])]);
    let playing = picture(&d.app);
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    let paused = picture(&d.app);
    let px = |p: &[u8], x: usize, y: usize| p[(y * 320 + x) * 4..(y * 320 + x) * 4 + 4].to_vec();
    assert_eq!(px(&paused, 40, 76), [150, 160, 190, 255]);
    assert_eq!(px(&paused, 279, 123), [150, 160, 190, 255]);
    assert_eq!(px(&paused, 50, 80), [16, 24, 64, 255]);
    // 'P' of "PAUSED" starts at column 142 on row 86 with "####."
    assert_eq!(px(&paused, 142, 86), [250, 210, 60, 255]);
    let mut lit = 0;
    for y in 0..200 {
        for x in 0..320 {
            if (40..280).contains(&x) && (76..124).contains(&y) {
                continue;
            }
            let was = px(&playing, x, y);
            let want = [was[0] / 2, was[1] / 2, was[2] / 2, 255];
            assert_eq!(px(&paused, x, y), want, "({x},{y})");
            lit += usize::from(was[0] > 1 || was[1] > 1 || was[2] > 1);
        }
    }
    assert!(lit > 1000, "the frame is not black");
}
