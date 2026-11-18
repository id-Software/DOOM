//! M5c timing mode (recorded, never gated), beside M5b's `m5b_bench`. Replays committed rooms
//! through the app core's own frame source and, per tic, times four series, each in its own
//! replay of a freshly loaded simulation (one unmeasured warm-up replay of each kind first):
//!
//! 1. `sound tic`: [`Sound::tic_from_simulation`] alone (the tic's records, `S_UpdateSounds` and
//!    the buffer when one is due), on a session of its own that is fed the `M_Random` index of
//!    the replay's real frame source; the frame source runs between the timers, untimed;
//! 2. `mix buffer`: [`Mixer::mix_buffer`] alone, per buffer the schedule makes due. The mixer
//!    state the C would mix is rebuilt outside the timer (the mixer after the previous tic plus
//!    this tic's started sounds) and checked to give the buffer the session itself mixed; a
//!    mismatch stops the example;
//! 3. `frame step, sound`: `FrameSource::tic` under the M5c contract (the sound session, the
//!    display's observe and draw, with its per-tic staging copies of both);
//! 4. `frame step, M5b`: `FrameSource::tic` over the same room loaded without sound (the M5b
//!    contract: the display's observe and draw).
//!
//! The simulation advance is outside every timer; `Instant` is a monotonic clock that never
//! touches simulation, sound or rendering outcomes. No device is opened and no sound is played.
//!
//! ```sh
//! cargo run -p doom-game --release --example m5c_bench -- [--rooms SNDBUSY,SNDMONST] [--screenblocks 10,11] [--warmup 1]
//! ```
//! One line per case and series: tics (or buffers), total, mean, median, 95th percentile and
//! maximum, in microseconds; and one `workload` line per room with its counts.
mod common {
    use std::path::{Path, PathBuf};
    pub fn reference(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(name)
    }
}
#[path = "../../doom-render/tests/m5c/mod.rs"]
mod m5c;

use doom_game::app::{FrameSource, GameData};
use doom_render::{Frame, GameMode};
use doom_sim::{Simulation, TicCommand};
use doom_sound::logic::RecordEcho;
use doom_sound::mixer::Buffer;
use doom_sound::session::Sound;
use m5c::wire25;
use std::time::Instant;

fn load(lump: &str, with_sound: bool) -> (GameData, Vec<TicCommand>) {
    let (_, file, lmp) = *wire25::ITEMS
        .iter()
        .find(|i| i.0 == lump)
        .unwrap_or_else(|| panic!("{lump}: no such item"));
    let files: Vec<Vec<u8>> = [file, "m5b_gfx.wad", "m5c_sfx.wad", "m5c_music.wad"]
        .iter()
        .map(|w| std::fs::read(common::reference(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let demo = std::fs::read(common::reference(lmp)).expect("demo");
    let (map, skill) = (demo[3], demo[1]);
    let (_, parsed) = doom_sim::demo::parse_demo_episode1_v2(&demo, map).expect("demo");
    let data = if with_sound {
        GameData::load_with_sound(&refs, GameMode::Registered, map, skill)
    } else {
        GameData::load(&refs, GameMode::Registered, map, skill)
    }
    .expect("game data");
    (data, parsed.commands)
}

fn advance(sim: &mut Simulation, cmd: TicCommand) -> bool {
    sim.advance([
        cmd,
        Default::default(),
        Default::default(),
        Default::default(),
    ])
    .is_ok()
}

fn us(ns: u64) -> f64 {
    ns as f64 / 1000.0
}

fn report(room: &str, sb: u8, series: &str, unit: &str, mut v: Vec<u64>) {
    if v.is_empty() {
        println!("{room} sb{sb} {series}: no samples");
        return;
    }
    v.sort_unstable();
    let n = v.len();
    let at = |p: usize| v[((n - 1) * p).div_ceil(100)];
    let total: u64 = v.iter().sum();
    println!(
        "{room} sb{sb} {series}: {unit} {n}, total us {:.0}, mean {:.1}, median {:.1}, p95 {:.1}, max {:.1}",
        us(total),
        us(total) / n as f64,
        us(at(50)),
        us(at(95)),
        us(v[n - 1]),
    );
}

/// What a replay of the sound and mixer series counted, for the `workload` line.
#[derive(Default)]
struct Workload {
    tics: usize,
    starts: u64,
    stops: u64,
    buffers: usize,
    busy_sum: u64,
    busy_max: u32,
}

/// The sound-only series: the session's tic and the mixer's buffer, with the real frame source
/// running untimed between the timers.
fn replay_sound(lump: &str, sb: u8, print: bool) {
    let (data, commands) = load(lump, true);
    let mut sim = data.start().clone();
    let mut fs = FrameSource::start(&data, sb, &sim).expect("start");
    let mut snd: Sound = fs.sound().expect("sound").clone();
    let mut frame = Frame::new();
    let (mut tics, mut mixes) = (Vec::new(), Vec::new());
    let mut w = Workload::default();
    for cmd in &commands {
        let before = fs.rndindex();
        if !advance(&mut sim, *cmd) {
            break;
        }
        let gametic = snd.schedule().gametics().expect("schedule").0;
        let mixer_before = snd.mixer().clone();
        let t = Instant::now();
        let ticked = snd.tic_from_simulation(&sim, before);
        let ns = t.elapsed().as_nanos() as u64;
        let ticked = match ticked {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{lump}: sound refused at tic {}: {e}", w.tics);
                break;
            }
        };
        tics.push(ns);
        w.tics += 1;
        for echo in &ticked.records {
            match echo {
                RecordEcho::Start(s) => w.starts += u64::from(s.mix.is_some()),
                RecordEcho::Stop(s) => w.stops += u64::from(s.is_some()),
            }
        }
        if let Some(buffer) = &ticked.buffer {
            // The mixer the buffer was mixed from: the one before the tic plus its started sounds.
            let mut mixer = mixer_before;
            for echo in &ticked.records {
                if let RecordEcho::Start(s) = echo {
                    if let (Some(mix), Some([vol, sep, pitch, _])) = (s.mix, s.args) {
                        let got = mixer
                            .start(s.sfx, vol, sep, pitch, gametic)
                            .expect("rebuilt start");
                        assert_eq!(got, mix, "{lump}: rebuilt start differs");
                    }
                }
            }
            let busy = mixer.mask().count_ones();
            w.busy_sum += u64::from(busy);
            w.busy_max = w.busy_max.max(busy);
            let mut out: Buffer = [[0; 2]; doom_sound::lumps::SAMPLECOUNT];
            let t = Instant::now();
            mixer.mix_buffer(&mut out);
            mixes.push(t.elapsed().as_nanos() as u64);
            assert_eq!(&out, buffer, "{lump}: rebuilt buffer differs");
            w.buffers += 1;
        }
        // The frame source (untimed) supplies the next tic's index and must agree with the session.
        let p = fs.tic(&data, &sim, cmd, &mut frame).expect("frame tic");
        let s = p.sound.expect("sound report");
        assert_eq!(
            s, ticked,
            "{lump}: the two sessions differ at tic {}",
            w.tics
        );
    }
    if print {
        report(lump, sb, "sound tic", "tics", tics);
        report(lump, sb, "mix buffer", "buffers", mixes);
        println!(
            "{lump} sb{sb} workload: tics {}, started sounds {}, channels freed by stops {}, buffers {}, busy channels at a mix: mean {:.2}, max {}",
            w.tics,
            w.starts,
            w.stops,
            w.buffers,
            w.busy_sum as f64 / w.buffers.max(1) as f64,
            w.busy_max,
        );
    }
}

/// The frame step series: `FrameSource::tic` with sound, or over the same room without it.
fn replay_frames(lump: &str, sb: u8, with_sound: bool, print: bool) {
    let (data, commands) = load(lump, with_sound);
    let mut sim = data.start().clone();
    let mut fs = FrameSource::start(&data, sb, &sim).expect("start");
    let mut frame = Frame::new();
    let mut steps = Vec::new();
    for (tic, cmd) in commands.iter().enumerate() {
        if !advance(&mut sim, *cmd) {
            break;
        }
        let t = Instant::now();
        let presented = fs.tic(&data, &sim, cmd, &mut frame);
        let ns = t.elapsed().as_nanos() as u64;
        if presented.is_err() {
            eprintln!("{lump}: frame step refused at tic {tic}");
            break;
        }
        steps.push(ns);
    }
    if print {
        let series = if with_sound {
            "frame step, sound"
        } else {
            "frame step, M5b"
        };
        report(lump, sb, series, "tics", steps);
    }
}

fn main() {
    let mut rooms: Vec<String> = [
        "SNDBUSY", "SNDMONST", "SNDBTN2", "SLIDEB", "M5AMP", "M5STDIG",
    ]
    .map(String::from)
    .into();
    let (mut blocks, mut warmup) = (vec![10u8, 11], 1usize);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let v = args.next().unwrap_or_else(|| panic!("{a}: missing value"));
        match a.as_str() {
            "--rooms" => rooms = v.split(',').map(str::to_uppercase).collect(),
            "--screenblocks" => blocks = v.split(',').map(|s| s.parse().expect("sb")).collect(),
            "--warmup" => warmup = v.parse().expect("warmup"),
            _ => panic!("usage: m5c_bench [--rooms A,B] [--screenblocks 10,11] [--warmup N]"),
        }
    }
    for room in &rooms {
        for &sb in &blocks {
            for _ in 0..warmup {
                replay_sound(room, sb, false);
                replay_frames(room, sb, true, false);
                replay_frames(room, sb, false, false);
            }
            replay_sound(room, sb, true);
            replay_frames(room, sb, true, true);
            replay_frames(room, sb, false, true);
        }
    }
}
