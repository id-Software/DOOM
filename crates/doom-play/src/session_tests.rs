//! The sound options change nothing in the game (decisions T8, T9): the same scripted session,
//! headless (no device, no thread, the fake clock), run with `--no-sound`, with a device sink at
//! volume 100, 0 and 37, and with the mute key pressed, gives the same tics, frames, sound
//! reports, `M_Random` indices, unscaled stream and demo bytes. The device sinks are headless
//! queues pulled by the test at scripted moments with different sizes, standing in for the
//! device's callback. Rust-only checks of doom-play's sink and key paths (the stream itself is
//! compared with the genuine engine in doom-game's tests/m5c_frames.rs).

use crate::audio::{Counters, DeviceSink, Output, Trial, Volume};
use crate::keys::{MUTE_KEY, Mute, key_event};
use doom_game::app::{
    App, AudioSink, DemoCommands, GameData, Host, RecordingSink, SessionConfig, TicReport,
    Timestamp, replay,
};
use doom_game::input::MouseButton;
use doom_render::GameMode;
use miniquad::KeyCode;
use std::collections::BTreeMap;
use std::path::Path;

fn load() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    let files: Vec<Vec<u8>> = ["m5a_wads/amp.wad", "m5b_gfx.wad", "m5c_sfx.wad"]
        .iter()
        .map(|w| std::fs::read(dir.join(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    GameData::load_with_sound(&refs, GameMode::Registered, 1, 2).expect("game data with sound")
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
    })
}

fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

/// What one tic reported: everything the game produced in it.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    tic: u64,
    /// The executed command, as the demo re-reads it.
    executed: String,
    frame: u64,
    rndindex: u8,
    /// A digest of the tic's whole sound report (records, update, draws, index, buffer).
    sound: u64,
    /// A digest of the sound logic's channel table after the tic.
    channels: u64,
    layout_dependent_from: Option<u64>,
}

/// The output under test, with a recording of exactly what the app handed to it.
struct Tee {
    out: Output,
    rec: RecordingSink,
}

impl AudioSink for Tee {
    fn buffer(&mut self, frames: &[[i16; 2]; 512]) {
        self.rec.buffer(frames);
        self.out.buffer(frames);
    }
    fn pause(&mut self) {
        self.rec.pause();
        self.out.pause();
    }
    fn resume(&mut self) {
        self.rec.resume();
        self.out.resume();
    }
    fn session_start(&mut self) {
        self.rec.session_start();
        self.out.session_start();
    }
}

struct TestHost {
    files: BTreeMap<String, Vec<u8>>,
    tee: Tee,
    seen: Vec<Seen>,
}

impl TestHost {
    fn new(out: Output) -> Self {
        Self {
            files: BTreeMap::new(),
            tee: Tee {
                out,
                rec: RecordingSink::default(),
            },
            seen: Vec::new(),
        }
    }
}

impl Host for TestHost {
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
        Some(&mut self.tee)
    }
    fn tic(&mut self, r: &TicReport<'_>) {
        let s = r.sound.expect("a session with sound reports sound");
        self.seen.push(Seen {
            tic: r.tic,
            executed: format!("{:?}", r.executed),
            frame: fnv(r.frame.pixels()),
            rndindex: r.rndindex,
            sound: fnv(format!("{:?}", s.tic).as_bytes()),
            channels: fnv(format!("{:?}", s.session.logic().channels()).as_bytes()),
            layout_dependent_from: s.session.detector().layout_dependent_from(),
        });
    }
}

impl Mute for TestHost {
    fn toggle_mute(&mut self) {
        self.tee.out.toggle_mute();
    }
}

/// One scripted input, as the window delivers it.
#[derive(Clone, Copy, Debug)]
enum In {
    Down(KeyCode),
    Repeat(KeyCode),
    Up(KeyCode),
    Motion(i32, i32),
    Button(MouseButton, bool),
}

/// The walk of doom-game's tests on the AMP room in window key codes (turns, mouse motion, a
/// pistol shot, the use key, running), with a pause and resume in the middle (after step 5).
/// `mute` lists the steps at which the mute key is pressed (down, an auto-repeat, up) before
/// the step's other inputs.
fn script(mute: &[usize]) -> Vec<(u64, Vec<In>)> {
    let mut steps = vec![
        (3, vec![]),
        (20, vec![In::Down(KeyCode::Up)]),
        (8, vec![In::Down(KeyCode::Left)]),
        (4, vec![In::Up(KeyCode::Left), In::Motion(-37, 5)]),
        (
            2,
            vec![In::Button(MouseButton::Left, true), In::Motion(90, -12)],
        ),
        (
            6,
            vec![In::Button(MouseButton::Left, false), In::Up(KeyCode::Up)],
        ),
        (
            5,
            vec![
                In::Down(KeyCode::Space),
                In::Down(KeyCode::LeftShift),
                In::Down(KeyCode::Up),
            ],
        ),
        (10, vec![In::Up(KeyCode::Space), In::Motion(3, 0)]),
        (30, vec![In::Up(KeyCode::Up), In::Up(KeyCode::LeftShift)]),
    ];
    for &i in mute.iter().rev() {
        let keys = [In::Down(MUTE_KEY), In::Repeat(MUTE_KEY), In::Up(MUTE_KEY)];
        steps[i].1.splice(0..0, keys);
    }
    steps
}

/// The script's whole run on `out`, pulling `pull` frames from a device sink after every
/// update; returns the host and the frames pulled.
fn run(data: &GameData, out: Output, pull: usize, mute: &[usize]) -> (TestHost, App, Vec<i16>) {
    let mut host = TestHost::new(out);
    let mut app = App::new(data.clone(), SessionConfig::default(), &mut host);
    let mut pulled = Vec::new();
    let (mut clock, mut base) = (0u64, 0u64);
    let mut update = |app: &mut App, host: &mut TestHost, clock: &mut u64, base: u64| {
        let ran = app.update(base + at(*clock), host).ran;
        *clock += 1;
        if let Output::Device(d) = &host.tee.out {
            let mut buf = vec![0i16; 2 * pull];
            d.pull(&mut buf);
            pulled.extend(buf);
        }
        ran
    };
    assert_eq!(update(&mut app, &mut host, &mut clock, base), 0);
    for (n, (tics, inputs)) in script(mute).into_iter().enumerate() {
        for i in inputs {
            match i {
                In::Down(k) => key_event(&mut app, &mut host, k, true, false),
                In::Repeat(k) => key_event(&mut app, &mut host, k, true, true),
                In::Up(k) => key_event(&mut app, &mut host, k, false, false),
                In::Motion(x, y) => app.mouse_motion(x, y),
                In::Button(b, d) => app.mouse_button(b, d),
            }
        }
        if n == 5 {
            // the app's pause, 12 paused updates, then resume
            key_event(&mut app, &mut host, KeyCode::Escape, true, false);
            key_event(&mut app, &mut host, KeyCode::Escape, false, false);
            for _ in 0..12 {
                assert_eq!(update(&mut app, &mut host, &mut clock, base), 0);
            }
            key_event(&mut app, &mut host, KeyCode::Escape, true, false);
            key_event(&mut app, &mut host, KeyCode::Escape, false, false);
            base += at(clock) + 1;
            clock = 0;
            assert_eq!(update(&mut app, &mut host, &mut clock, base), 0);
        }
        let mut ran = 0;
        while ran < tics {
            ran += update(&mut app, &mut host, &mut clock, base);
        }
    }
    app.quit(&mut host);
    (host, app, pulled)
}

#[test]
fn sound_options_change_no_tic_frame_index_sound_report_stream_or_demo() {
    let data = load();
    let tics: u64 = script(&[]).iter().map(|s| s.0).sum();
    let headless = |percent| Output::Device(DeviceSink::headless(Volume::new(percent).unwrap()));
    let no_sound = || {
        let (out, line) = Output::for_launch(true, Volume::FULL);
        assert!(line.is_none(), "--no-sound prints nothing");
        out
    };
    let variants: [(&str, Output, usize, &[usize]); 8] = [
        ("--no-sound", no_sound(), 0, &[]),
        ("volume 100", headless(100), 315, &[]),
        ("volume 0", headless(0), 315, &[]),
        ("volume 37", headless(37), 315, &[]),
        ("volume 100, pulls of 100 frames", headless(100), 100, &[]),
        ("volume 100, muted", headless(100), 315, &[1]),
        ("volume 100, muted and unmuted", headless(100), 315, &[1, 1]),
        ("--no-sound, mute key", no_sound(), 0, &[1, 4, 6]),
    ];
    let mut base: Option<(Vec<Seen>, RecordingSink, Vec<u8>)> = None;
    let mut pulled_at: BTreeMap<&str, Vec<i16>> = BTreeMap::new();
    for (name, out, pull, mute) in variants {
        let (host, app, pulled) = run(&data, out, pull, mute);
        let demo = host.files[app.session_name()].clone();
        assert_eq!(host.seen.len() as u64, tics, "{name}: every tic ran");
        match &host.tee.out {
            Output::Device(d) => {
                let c: Counters = d.counters();
                let loud = pulled.iter().filter(|s| **s != 0).count();
                eprintln!(
                    "{name}: {} frames pulled, {loud} non-zero samples, {c:?}",
                    pulled.len() / 2
                );
                if name == "volume 0" || mute == [1] {
                    assert_eq!(loud, 0, "{name}: silent");
                } else {
                    assert!(loud > 0, "{name}: audible");
                }
            }
            Output::Discard(d) => {
                assert_eq!(d.buffers as usize, host.tee.rec.buffers.len(), "{name}");
                eprintln!("{name}: {} buffers discarded", d.buffers);
            }
        }
        pulled_at.insert(name, pulled);
        match &base {
            None => {
                assert!(
                    host.tee.rec.stream_bytes().iter().any(|b| *b != 0),
                    "the walk is not silent"
                );
                base = Some((host.seen, host.tee.rec, demo));
            }
            Some((seen, rec, bytes)) => {
                assert_eq!(
                    &host.seen, seen,
                    "{name}: tics, frames, indices, sound reports"
                );
                assert_eq!(&host.tee.rec, rec, "{name}: the stream and the sink calls");
                assert_eq!(&demo, bytes, "{name}: the demo bytes");
            }
        }
    }
    // at the device: volume 37 is volume 100 scaled sample by sample; muting twice is no mute
    let full = &pulled_at["volume 100"];
    let scaled: Vec<i16> = full
        .iter()
        .map(|s| crate::audio::scale(*s, 37, 100))
        .collect();
    assert_eq!(pulled_at["volume 37"], scaled);
    assert_eq!(&pulled_at["volume 100, muted and unmuted"], full);
    // the demo replays to the same tics with a device sink as the host's output
    let (seen, rec, demo) = base.expect("ran");
    let commands = DemoCommands::parse(&demo, 1).expect("demo");
    assert_eq!(commands.commands().len() as u64, tics);
    let mut again = TestHost::new(headless(100));
    assert_eq!(replay(&data, 10, &commands, &mut again), None);
    assert_eq!(again.seen, seen);
    assert_eq!(again.tee.rec.buffers, rec.buffers);
}

/// Review finding M2 (review-app): the exit lines (`window::exit_lines`) carry the run's
/// layout-dependence line between the tic statistics and the save line; a session the detector of
/// decision T12 never labelled (this script holds no switch release) adds none. The line itself,
/// for a session whose detector fires, is tested in doom-game's tests/m5c_app.rs.
#[test]
fn the_exit_lines_hold_no_layout_line_for_an_unlabelled_session() {
    let data = load();
    let (_, app, _) = run(&data, Output::for_launch(true, Volume::FULL).0, 0, &[]);
    assert_eq!(app.layout_dependent_line(), None);
    let fs = crate::FsHost {
        dir: std::path::PathBuf::from("sessions"),
        audio: Output::Discard(crate::audio::Discard::default()),
    };
    let lines = crate::window::exit_lines(&app, &fs, None);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("saved "), "{lines:?}");
    let stats = crate::tic_stats::TicStats::default();
    let lines = crate::window::exit_lines(&app, &fs, Some(&stats));
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].starts_with("tic-stats: "), "{lines:?}");
    assert!(!lines.iter().any(|l| l.contains("layout-dependent")));
}

/// Milestone M5d step A2: whenever a device is open the exit lines carry the device's lines, with
/// or without `--tic-stats` (they replace the counters line `--tic-stats` printed before), between
/// the tic statistics and the save line; their figures are the sink's own.
#[test]
fn the_exit_lines_give_the_device_lines_with_or_without_tic_stats() {
    let data = load();
    let (host, app, pulled) = run(
        &data,
        Output::Device(DeviceSink::headless(Volume::FULL)),
        315,
        &[],
    );
    assert!(pulled.iter().any(|s| *s != 0), "the walk is audible");
    let fs = crate::FsHost {
        dir: std::path::PathBuf::from("sessions"),
        audio: host.tee.out,
    };
    let Output::Device(d) = &fs.audio else {
        panic!("a device sink")
    };
    let (p, c) = (d.pull_stats(), d.counters());
    assert_eq!((p.min_frames, p.max_frames), (315, 315));
    assert!(
        p.pulls > 0 && p.depth_samples > 0 && p.depth_samples < p.pulls,
        "{p:?}"
    );
    let lines = crate::window::exit_lines(&app, &fs, None);
    assert_eq!(lines.len(), 5, "{lines:?}");
    assert_eq!(
        lines[0],
        "audio device: backend none (headless); name unavailable (no device (headless)); \
         native sample rates unavailable (no device (headless)); stream 11025 Hz"
    );
    assert_eq!(
        lines[1],
        "audio settings: period asked 128 frames; start threshold 1024 frames (2 buffers); \
         miniaudio's [INFO] report unavailable (no device (headless))"
    );
    assert_eq!(
        lines[2],
        format!(
            "audio callbacks: {} calls; frames per call min 315, max 315",
            p.pulls
        )
    );
    assert_eq!(
        lines[3],
        format!(
            "audio queue: depth in frames when a callback arrived (not paused) min {}, mean {}, \
             max {} ({} samples); underruns {} ({} frames of silence); overruns {} ({} frames \
             dropped)",
            p.depth_min,
            p.depth_mean().unwrap(),
            p.depth_max,
            p.depth_samples,
            c.underruns,
            c.underrun_frames,
            c.overruns,
            c.dropped_frames
        )
    );
    assert!(lines[4].starts_with("saved "), "{lines:?}");
    assert!(
        !lines.iter().any(|l| l.contains("EXPERIMENTAL")),
        "defaults"
    );
    let stats = crate::tic_stats::TicStats::default();
    let with = crate::window::exit_lines(&app, &fs, Some(&stats));
    assert_eq!(with.len(), 6, "{with:?}");
    assert!(with[0].starts_with("tic-stats: "), "{with:?}");
    assert_eq!(
        with[1..],
        lines[..],
        "the same lines after the tic statistics"
    );
    assert_eq!(
        with.iter().filter(|l| l.contains("underruns")).count(),
        1,
        "no second counters line"
    );
}

/// Milestone M5d step A2: the experimental trial settings (a one-buffer start; a period of 128
/// frames, the default since 2026-11-21, or of 256, which is not the default) change nothing in
/// the game: the same script gives the same tics, frames, indices, sound reports, stream and demo
/// as `--no-sound`; at the sink the pulls are all of the period's size and the exit lines say
/// which settings are experimental (the one-buffer start itself is tested on the queue in
/// `audio.rs`: here 128-frame or 256-frame pulls against 315 frames made per tic fill the queue to
/// its cap, so the start threshold leaves no trace in what is pulled later).
#[test]
fn the_trial_settings_change_no_tic_frame_index_sound_report_stream_or_demo() {
    let data = load();
    let (base, base_app, _) = run(&data, Output::for_launch(true, Volume::FULL).0, 0, &[]);
    for (period, described) in [
        (128usize, "period asked 128 frames"),
        (256, "period asked 256 frames (EXPERIMENTAL; default 128)"),
    ] {
        the_trial_changes_nothing(&data, &base, &base_app, period, described);
    }
}

fn the_trial_changes_nothing(
    data: &GameData,
    base: &TestHost,
    base_app: &App,
    period: usize,
    described: &str,
) {
    let trial = Trial::DEFAULT
        .with_start_buffers(1)
        .and_then(|t| t.with_period_frames(period as u32))
        .unwrap();
    let (host, app, pulled) = run(
        data,
        Output::Device(DeviceSink::headless_with(Volume::FULL, trial)),
        period,
        &[],
    );
    assert_eq!(host.seen, base.seen, "tics, frames, indices, sound reports");
    assert_eq!(host.tee.rec, base.tee.rec, "the stream and the sink calls");
    assert_eq!(
        host.files[app.session_name()],
        base.files[base_app.session_name()],
        "the demo bytes"
    );
    assert!(pulled.iter().any(|s| *s != 0), "audible");
    let Output::Device(d) = &host.tee.out else {
        panic!("a device sink")
    };
    let p = d.pull_stats();
    assert_eq!(
        (p.min_frames, p.max_frames),
        (period as u64, period as u64),
        "{period}"
    );
    assert!(d.counters().overruns > 0, "the queue reached its cap");
    let lines = d.exit_lines();
    assert_eq!(
        lines[1],
        format!(
            "audio settings: {described}; start threshold 512 frames (1 buffer; EXPERIMENTAL; \
             default 1024 frames); miniaudio's [INFO] report unavailable (no device (headless))"
        )
    );
}
