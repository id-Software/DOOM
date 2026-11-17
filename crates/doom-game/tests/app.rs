//! Headless sessions of the app on the repository's synthetic maps (with
//! `tests/reference/m5b_gfx.wad`, the M5a graphics with the status bar and HUD lumps), driven
//! by scripted events and a fake monotonic clock: no window, no file system inside the app, no
//! wall clock.

use doom_game::app::{
    App, Ending, GAMMA, GameData, Host, MAX_TICS_PER_UPDATE, Mode, PICTURE_BYTES, SAVE_INTERVAL,
    SaveStatus, SessionConfig, TIC_CAP, TicReport, Timestamp, Witness, replay, session_name,
    wire24_admits,
};
use doom_game::input::{MouseButton, keys};
use doom_render::GameMode;
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
        replay(&load("m5a_wads/amp.wad", 1, 2), 10, &cmds, &mut again),
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
    for _ in 0..SAVE_INTERVAL - 1 {
        d.tic();
    }
    assert_eq!(d.host.writes, 0);
    d.tic();
    assert_eq!(d.host.writes, 1);
    assert_eq!(commands(d.demo(), 1).len(), SAVE_INTERVAL);
    d.host.fail = true;
    d.app.quit(&mut d.host);
    assert_eq!(d.app.save_status(), &SaveStatus::Failed("disk full".into()));
}

#[test]
fn file_names_come_from_the_injected_date_and_never_overwrite() {
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
