//! M5c headless app sessions with sound (decision T15; M5c note §4.7.1): `GameData::load_with_sound`
//! on the M5a AMP room with `m5b_gfx.wad` and the synthetic sound lumps `m5c_sfx.wad`, driven by
//! scripted events and the fake monotonic clock, the mixed stream taken by a `RecordingSink`. No
//! device, no thread, no wall clock. Rust-only checks of the app's sequence and seam (the frames
//! and the sound under the same sequence are compared with the genuine engine in
//! tests/m5c_frames.rs).
//!
//! - The same scripted session twice gives identical buffers, starts (the tic's whole sound
//!   report), channel tables, `M_Random` indices and frames; `replay` of its recorded demo gives the
//!   same again.
//! - The app's pause sends `pause` to the sink, no tic or buffer runs while paused, resuming sends
//!   `resume`; a restart is a fresh process (decision T15): `session_start` (after `resume` when
//!   restarted from the pause), a fresh session, and the same script then gives the same stream.
//! - A sound refusal (a start of an 8-byte sound lump, decision T11) ends the session through the
//!   app's typed refusal path, the sound session left as before the tic; the demo replays to the
//!   same refusal.

use doom_game::app::{
    App, AudioSink, DemoCommands, Ending, FrameSource, GameData, Host, Mode, RecordingSink,
    SessionConfig, SinkCall, TicReport, Timestamp, Witness, replay,
};
use doom_game::input::{MouseButton, keys};
use doom_render::GameMode;
use doom_sound::logic::{ChannelEntry, SoundRefusal};
use doom_sound::mixer::MixRefusal;
use doom_sound::session::{RefusedAt, SessionRefusal, Sound, TicSound};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

/// A one-lump PWAD.
fn pwad(name: &[u8], data: &[u8]) -> Vec<u8> {
    let mut w = b"PWAD".to_vec();
    w.extend(1u32.to_le_bytes());
    w.extend((12 + data.len() as u32).to_le_bytes());
    w.extend(data);
    w.extend(12u32.to_le_bytes());
    w.extend((data.len() as u32).to_le_bytes());
    let mut n = [0u8; 8];
    n[..name.len()].copy_from_slice(name);
    w.extend(n);
    w
}

fn load(extra: Option<Vec<u8>>) -> GameData {
    let mut files: Vec<Vec<u8>> = ["m5a_wads/amp.wad", "m5b_gfx.wad", "m5c_sfx.wad"]
        .iter()
        .map(|w| std::fs::read(reference(w)).expect("WAD"))
        .collect();
    files.extend(extra);
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    GameData::load_with_sound(&refs, GameMode::Registered, 1, 2).expect("game data with sound")
}

fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
    })
}

/// What one tic reported.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    tic: u64,
    frame: u64,
    rndindex: u8,
    sound: TicSound,
    channels: Vec<Option<ChannelEntry>>,
    layout_dependent_from: Option<u64>,
}

#[derive(Default)]
struct SoundHost {
    files: BTreeMap<String, Vec<u8>>,
    sink: RecordingSink,
    seen: Vec<Seen>,
    last_session: Option<Sound>,
}

impl Host for SoundHost {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp {
            year: 2026,
            month: 11,
            day: 18,
            hour: 12,
            minute: 0,
            second: 0,
        }
    }
    fn exists(&mut self, name: &str) -> bool {
        self.files.contains_key(name)
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.files.insert(name.to_string(), bytes.to_vec());
        Ok(())
    }
    fn audio(&mut self) -> Option<&mut dyn AudioSink> {
        Some(&mut self.sink)
    }
    fn tic(&mut self, r: &TicReport<'_>) {
        let s = r.sound.expect("a session with sound reports sound");
        self.seen.push(Seen {
            tic: r.tic,
            frame: fnv(r.frame.pixels()),
            rndindex: r.rndindex,
            sound: s.tic.clone(),
            channels: s.session.logic().channels().to_vec(),
            layout_dependent_from: s.session.detector().layout_dependent_from(),
        });
        self.last_session = Some(s.session.clone());
    }
}

#[derive(Clone, Copy, Debug)]
enum In {
    Key(u8, bool),
    Motion(i32, i32),
    Button(MouseButton, bool),
}

const UP: u8 = keys::KEY_UPARROW;

/// The walk of crates/doom-game/tests/app.rs on the AMP room: keyboard turns, mouse motion, a
/// mouse fire (the pistol) and the use key.
fn walk_script() -> Vec<(u64, Vec<In>)> {
    vec![
        (3, vec![]),
        (20, vec![In::Key(UP, true)]),
        (8, vec![In::Key(keys::KEY_LEFTARROW, true)]),
        (
            4,
            vec![In::Key(keys::KEY_LEFTARROW, false), In::Motion(-37, 5)],
        ),
        (
            2,
            vec![In::Button(MouseButton::Left, true), In::Motion(90, -12)],
        ),
        (
            6,
            vec![In::Button(MouseButton::Left, false), In::Key(UP, false)],
        ),
        (
            5,
            vec![
                In::Key(b' ', true),
                In::Key(keys::KEY_RSHIFT, true),
                In::Key(UP, true),
            ],
        ),
        (10, vec![In::Key(b' ', false), In::Motion(3, 0)]),
        (
            30,
            vec![In::Key(UP, false), In::Key(keys::KEY_RSHIFT, false)],
        ),
    ]
}

struct Driver {
    app: App,
    host: SoundHost,
    clock: u64,
    base: u64,
}

impl Driver {
    fn new(data: GameData) -> Self {
        let mut host = SoundHost::default();
        let mut app = App::new(data, SessionConfig::default(), &mut host);
        assert_eq!(app.update(0, &mut host).ran, 0);
        Self {
            app,
            host,
            clock: 0,
            base: 0,
        }
    }

    fn feed(&mut self, i: In) {
        match i {
            In::Key(k, d) => self.app.key(k, d, &mut self.host),
            In::Motion(x, y) => self.app.mouse_motion(x, y),
            In::Button(b, d) => self.app.mouse_button(b, d),
        }
    }

    fn tic(&mut self) -> u64 {
        self.clock += 1;
        self.app
            .update(self.base + at(self.clock), &mut self.host)
            .ran
    }

    fn run(&mut self, script: &[(u64, Vec<In>)]) {
        for (tics, inputs) in script {
            for i in inputs {
                self.feed(*i);
            }
            for _ in 0..*tics {
                self.tic();
            }
        }
    }

    fn demo(&self) -> Vec<u8> {
        self.host.files[self.app.session_name()].clone()
    }
}

fn starts(seen: &[Seen]) -> usize {
    seen.iter().map(|s| s.sound.records.len()).sum()
}

#[test]
fn a_scripted_session_twice_is_identical_and_its_demo_replays_the_same() {
    let data = load(None);
    let mut a = Driver::new(data.clone());
    a.run(&walk_script());
    a.app.quit(&mut a.host);
    let mut b = Driver::new(data.clone());
    b.run(&walk_script());
    let n = walk_script().iter().map(|s| s.0).sum::<u64>() as usize;
    assert_eq!(a.host.seen.len(), n);
    assert_eq!(a.host.seen, b.host.seen);
    assert_eq!(a.host.sink, b.host.sink);
    assert!(starts(&a.host.seen) > 0, "the walk starts sounds");
    eprintln!(
        "walk: {n} tics, {} sound records, {} buffers",
        starts(&a.host.seen),
        a.host.sink.buffers.len()
    );
    let buffers = a.host.sink.buffers.len();
    // the declared schedule: ceil(315 (t + 1) / 512) buffers after tic t (decision T5)
    assert_eq!(buffers, (315 * n).div_ceil(512));
    assert!(
        a.host.sink.stream_bytes().iter().any(|b| *b != 0),
        "not silence"
    );
    assert_eq!(a.host.sink.calls[0], SinkCall::SessionStart);
    // under the M5c contract the index is not the tic count: tic 0 adds the wipe's 320
    let s0 = &a.host.seen[0];
    assert_eq!(
        u16::from(s0.rndindex),
        (s0.sound.draws + 1 + 320) % 256,
        "sound draws, ST_Ticker's, the wipe's"
    );
    assert!(
        a.host
            .seen
            .iter()
            .all(|s| s.layout_dependent_from.is_none())
    );
    // the recorded demo replays the same
    let commands = DemoCommands::parse(&a.demo(), 1).expect("demo");
    assert_eq!(commands.commands().len(), n);
    let mut again = SoundHost::default();
    assert_eq!(replay(&data, 10, &commands, &mut again), None);
    assert_eq!(again.seen, a.host.seen);
    assert_eq!(again.sink.buffers, a.host.sink.buffers);
}

#[test]
fn pause_resume_and_restart() {
    let data = load(None);
    let mut d = Driver::new(data.clone());
    d.run(&walk_script()[..3]);
    let ran = d.host.seen.len();
    let buffers = d.host.sink.buffers.len();
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    assert_eq!(d.app.mode(), &Mode::Paused);
    assert_eq!(d.host.sink.calls.last(), Some(&SinkCall::Pause));
    for _ in 0..20 {
        assert_eq!(d.tic(), 0, "no tic while paused");
    }
    assert_eq!(d.host.sink.buffers.len(), buffers, "no buffer while paused");
    d.feed(In::Key(keys::KEY_ESCAPE, false));
    d.feed(In::Key(keys::KEY_ESCAPE, true));
    assert_eq!(d.app.mode(), &Mode::Playing);
    assert_eq!(d.host.sink.calls.last(), Some(&SinkCall::Resume));
    d.tic(); // re-bases the clock
    while d.host.seen.len() < ran + 10 {
        d.tic();
    }
    // focus loss pauses; the sink hears it at the next call with the host
    let calls = d.host.sink.calls.len();
    d.app.focus_lost();
    assert_eq!(d.host.sink.calls.len(), calls);
    assert_eq!(d.tic(), 0);
    assert_eq!(d.host.sink.calls.last(), Some(&SinkCall::Pause));
    // restart from the pause: resume, then session_start, and a fresh process
    d.feed(In::Key(b'r', true));
    let calls = &d.host.sink.calls;
    assert_eq!(
        &calls[calls.len() - 2..],
        &[SinkCall::Resume, SinkCall::SessionStart]
    );
    let fresh = d.app.sound().expect("sound");
    assert_eq!(fresh.schedule().tic(), 0);
    assert!(fresh.logic().channels().iter().all(Option::is_none));
    assert_eq!(fresh.mixer().mask(), 0);
    assert_eq!(fresh.mixer().handle_counter(), 0);
    // the same script after the restart gives the first session's stream
    let mut first = Driver::new(data);
    first.run(&walk_script());
    let (seen0, buf0) = (d.host.seen.len(), d.host.sink.buffers.len());
    // the restarted session's clock starts at its first update, as the first session's did
    d.base += at(d.clock) + 1;
    d.clock = 0;
    assert_eq!(d.app.update(d.base, &mut d.host).ran, 0);
    d.run(&walk_script());
    assert_eq!(&d.host.seen[seen0..], &first.host.seen[..]);
    assert_eq!(&d.host.sink.buffers[buf0..], &first.host.sink.buffers[..]);
}

#[test]
fn a_sound_refusal_ends_the_session_with_the_sound_state_preserved() {
    // DSPISTOL of 8 bytes (format 3, 11,025 Hz, 0 samples): its start would read one byte past
    // its block (i_sound.c:230-247, 591), refused at the exact start (decision T11).
    let short = pwad(b"DSPISTOL", &[3, 0, 0x11, 0x2b, 0, 0, 0, 0]);
    let data = load(Some(short));
    let mut d = Driver::new(data.clone());
    d.run(&[(5, vec![])]);
    d.feed(In::Key(keys::KEY_RCTRL, true));
    for _ in 0..60 {
        d.tic();
    }
    let Mode::Ended(Ending::Refused { tic, witness }) = d.app.mode().clone() else {
        panic!("refused: {:?}", d.app.mode());
    };
    eprintln!("refused at tic {tic}: {witness}");
    assert!(
        matches!(
            witness,
            Witness::Sound(SessionRefusal::Sound {
                at: RefusedAt::Record(_),
                refusal: SoundRefusal::Mix(MixRefusal::ShortLump { sfx: 1 }),
                ..
            })
        ),
        "{witness:?}"
    );
    // review-app N6: the end screen's witness text is a plain message, not Debug text
    let text = witness.to_string();
    let SessionRefusal::Sound {
        at: RefusedAt::Record(i),
        ..
    } = (match &witness {
        Witness::Sound(r) => *r,
        _ => unreachable!(),
    })
    else {
        unreachable!()
    };
    assert_eq!(
        text,
        format!(
            "sound: tic {tic}, sound call {i}: mixer: refused: sfx 1 has no sample byte at its \
             start (i_sound.c:230-247, 591)"
        )
    );
    assert!(!text.contains('{') && !text.contains("Sound"), "{text}");
    // the sound session is exactly as after the last reported tic, the one before the refusal
    let after = d.app.sound().expect("sound");
    assert_eq!(after.schedule().tic(), tic);
    assert_eq!(
        d.host.seen.len() as u64,
        tic,
        "the refused tic is not reported"
    );
    assert_eq!(Some(after), d.host.last_session.as_ref());
    // the simulation committed the tic (decision S9) and the demo replays to the same refusal
    assert_eq!(d.app.tic(), tic + 1);
    let commands = DemoCommands::parse(&d.demo(), 1).expect("demo");
    let mut again = SoundHost::default();
    assert_eq!(
        replay(&data, 10, &commands, &mut again),
        Some(Ending::Refused { tic, witness })
    );
    assert_eq!(again.seen, d.host.seen);
    // the frame source's whole saved state (DDSP and DSND) is unchanged by the refused tic
    let mut sim = data.start().clone();
    let mut fs = FrameSource::start(&data, 10, &sim).expect("start");
    let mut frame = doom_render::Frame::new();
    for (t, cmd) in commands.commands().iter().enumerate() {
        let before = fs.save().expect("save");
        sim.advance([
            *cmd,
            Default::default(),
            Default::default(),
            Default::default(),
        ])
        .expect("the simulation admits every tic");
        match fs.tic(&data, &sim, cmd, &mut frame) {
            Ok(_) => assert!((t as u64) < tic),
            Err(w) => {
                assert_eq!(t as u64, tic);
                assert!(matches!(w, Witness::Sound(_)));
                assert_eq!(fs.save().expect("save"), before);
                break;
            }
        }
    }
}

/// Review finding M2 (review-app): decision T12 counts a layout-dependent session "separately in
/// every report". A session whose detector fires (a switch release with its next slot busy, at
/// the edge of audibility, where texture number 0 read as x by a 32-bit build would be inaudible:
/// s_sound.c:796-815 with p_spec.c:1151) gives one plain line naming the first tic and the count;
/// an unlabelled session gives none unless an earlier session of the run was labelled; the app's
/// scripted session (no release) gives none.
#[test]
fn a_layout_dependent_session_gives_one_plain_line() {
    use doom_game::app::layout_dependent_line;
    use doom_sound::logic::{Listener, OriginPositions, S_CLIPPING_DIST, SoundOrigin};
    use doom_sound::logic::{SoundPlacement, SoundRecord};
    use doom_sound::session::SoundConfig;
    struct Quiet;
    impl OriginPositions for Quiet {
        fn actor_xy(&self, _: doom_sim::ActorId) -> Option<[i32; 2]> {
            None
        }
        fn sector_xy(&self, _: u16) -> Option<[i32; 2]> {
            None
        }
        fn button_xy(&self, _: u8) -> Option<[i32; 2]> {
            Some([0, 0])
        }
    }
    let data = load(None);
    let set = data.sound().expect("sound").set().clone();
    // The declared reading (34, 0) is audible with volume 1; (0, 34) is not.
    let edge = S_CLIPPING_DIST - (130 << 16);
    let listener = Listener {
        actor: doom_sim::ActorId::from_parts(0, 0),
        xy: [edge + 34, 0],
        angle: 0,
    };
    let release = SoundRecord::Start {
        sfx: 23,
        origin: SoundOrigin::Button(0),
        at: Some(SoundPlacement {
            origin: [34, 0],
            listener: listener.xy,
            listener_angle: 0,
        }),
    };
    let mut s = Sound::new(set, SoundConfig::fresh(1, 1)).expect("session");
    s.tic(&[], 0, &listener, &Quiet).expect("tic 0");
    s.tic(&[release], 0, &listener, &Quiet).expect("tic 1");
    let d = s.detector();
    assert_eq!((d.layout_dependent_from(), d.firings()), (Some(1), 1));
    assert_eq!(
        layout_dependent_line(Some(d), 0).as_deref(),
        Some(
            "layout-dependent session from tic 1 (1 detector firing): a 32-bit build could have \
             played a switch-release sound differently; this one used the 64-bit reading"
        )
    );
    assert!(
        layout_dependent_line(Some(d), 2)
            .is_some_and(|l| l.ends_with("; 2 earlier sessions of this run were labelled too"))
    );
    let fresh = Sound::new(
        data.sound().expect("sound").set().clone(),
        SoundConfig::fresh(1, 1),
    )
    .expect("session")
    .detector();
    assert_eq!(layout_dependent_line(Some(fresh), 0), None);
    assert_eq!(layout_dependent_line(None, 0), None);
    assert_eq!(
        layout_dependent_line(Some(fresh), 1).as_deref(),
        Some(
            "this session is not layout-dependent; 1 earlier session of this run was labelled too"
        )
    );
    let mut a = Driver::new(data.clone());
    a.run(&walk_script());
    assert!(starts(&a.host.seen) > 0);
    assert_eq!(a.app.layout_dependent_line(), None);
    // a restart keeps the run's count (here 0 labelled)
    a.app.restart(&mut a.host);
    assert_eq!(a.app.layout_dependent_line(), None);
}
