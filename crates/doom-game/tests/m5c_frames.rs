//! M5c THE FRAME GATE: every frame of the 64 genuine items of `tests/reference/m5c_frames`
//! (DOOM-M5C-FRAMES-v1, written by the C only: `tools/reference/m5c_reference.py`, linuxdoom-1.10
//! built with its in-process sound code, one define removed) reproduced end to end by the app
//! core: `GameData::load_with_sound` over the genuine lump list (the item WAD, `m5b_gfx.wad`,
//! `m5c_sfx.wad`, `m5c_music.wad`), then `replay` of the item's demo at screenblocks 10 and 11,
//! which runs per tic the wire-25 simulation, the Rust sound session and the display under
//! `RandomContract::WithSound` exactly as a played session does (`FrameSource::tic`). No draw count
//! comes from a golden here (doom-render's tests/m5c_display.rs is the fixture projection).
//!
//! Per frame: the whole-frame digest (the masked digest and flagged columns for the 300
//! tracker-flagged frames; the 70 frames whose switch parity the renderer reports unknown are not
//! digest-compared, as in M5b), the face-zeroed digest of the Q row, the renderer counters,
//! `fuzzpos` and the newly mapped lines, and the P row (palette, palette set, face, `message_on`,
//! `M_Random` index); per tic the message taken against the X rows, and the Q row's cumulative
//! sound draws against the session's. A run must end where the genuine one ends: a renderer
//! refusal for M5VPLAN, M5OPEN and M5SSEG, the exit for BOSS1, the demo's end otherwise. The
//! layout detector of decision T12 must not fire on these items (its firings are counted).
//! M5CHEAT's two cheats reach the status bar through `replay_adjusted` (decision S1), since no
//! wire carries cheats.
#[allow(dead_code)]
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

use doom_game::app::{
    AudioSink, Ending, FrameRefusal, GameData, Host, RecordingSink, TicReport, Timestamp, Witness,
    replay_adjusted, replay_full_resolution,
};
use doom_render::{DisplayRefusal, GameMode};
use doom_test_evidence::sha256;
use m5c::{Stream, cheats, genuine_end, noface, read_stream, wire25};

/// What was compared.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Counts {
    frames: usize,
    digests: usize,
    masked: usize,
    non_comparable: usize,
    face_index: usize,
    face_zeroed: usize,
    ends: usize,
    messages: usize,
    buffers: usize,
    detector_firings: u64,
}

struct Gate<'a> {
    golden: &'a Stream,
    ctx: String,
    failures: Vec<String>,
    counts: Counts,
    taken: Vec<(i64, i32, String)>,
    snd_draws: u64,
    sink: RecordingSink,
}

impl Host for Gate<'_> {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn audio(&mut self) -> Option<&mut dyn AudioSink> {
        Some(&mut self.sink)
    }
    fn tic(&mut self, r: &TicReport<'_>) {
        let tic = r.tic as i64;
        let ctx = format!("{} tic {tic}", self.ctx);
        let Some(g) = self.golden.frames.get(&tic) else {
            self.failures
                .push(format!("{ctx}: a frame the genuine run does not have"));
            return;
        };
        let c = &mut self.counts;
        c.frames += 1;
        if let Some(m) = r.message {
            self.taken.push((tic, m.code, m.text.into()));
        }
        let s = r.sound.expect("a session with sound reports sound");
        self.snd_draws += u64::from(s.tic.draws);
        c.detector_firings = s.session.detector().firings();
        if self.snd_draws != g.snd_draws {
            self.failures.push(format!(
                "{ctx}: sound draws {} genuine {}",
                self.snd_draws, g.snd_draws
            ));
        }
        let px = r.frame.pixels();
        let f = r.display;
        let face_zeroed = sha256(&noface(px, g.face_rect));
        if f.view.switch_parity_unknown {
            c.non_comparable += 1;
        } else {
            if g.columns.is_empty() {
                c.digests += 1;
                if sha256(px) != g.sha256 {
                    let where_ = if face_zeroed == g.noface {
                        "inside the face rectangle only"
                    } else {
                        "outside the face rectangle"
                    };
                    self.failures
                        .push(format!("{ctx}: frame differs ({where_})"));
                }
            } else {
                c.masked += 1;
                let mut m = px.to_vec();
                for x in &g.columns {
                    for y in 0..doom_render::SCREENHEIGHT {
                        m[y * doom_render::SCREENWIDTH + usize::from(*x)] = 0;
                    }
                }
                if sha256(&m) != g.masked || f.view.flagged_columns != g.columns {
                    self.failures
                        .push(format!("{ctx}: masked frame or flagged columns differ"));
                }
            }
            if g.noface == "undefined" || face_zeroed == g.noface {
                c.face_zeroed += 1;
            } else {
                self.failures
                    .push(format!("{ctx}: face-zeroed digest differs"));
            }
        }
        c.face_index += usize::from(i64::from(f.face) == g.status[2]);
        if f.view.counters != g.counters
            || f.view.fuzzpos != g.fuzzpos
            || f.view.newly_mapped != g.mapped
        {
            self.failures.push(format!("{ctx}: counters"));
        }
        let got = [
            i64::from(f.palette),
            f.palette_set.map_or(-1, i64::from),
            i64::from(f.face),
            i64::from(f.message_on),
            i64::from(r.rndindex),
        ];
        if got != g.status {
            self.failures
                .push(format!("{ctx}: P {got:?} genuine {:?}", g.status));
        }
    }
}

fn load_data(i: usize) -> (GameData, Vec<doom_sim::TicCommand>, u8) {
    let (_, file, lmp) = wire25::ITEMS[i];
    let files: Vec<Vec<u8>> = [file, "m5b_gfx.wad", "m5c_sfx.wad", "m5c_music.wad"]
        .iter()
        .map(|w| std::fs::read(common::reference(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let demo = std::fs::read(common::reference(lmp)).expect("demo");
    let (map, skill) = (demo[3], demo[1]);
    let (_, parsed) = doom_sim::demo::parse_demo_episode1_v2(&demo, map).expect("demo");
    let data = GameData::load_with_sound(&refs, GameMode::Registered, map, skill)
        .expect("game data with sound");
    (data, parsed.commands, map)
}

fn run_part(part: usize) -> Counts {
    let mut total = Counts::default();
    let mut failures = Vec::new();
    for i in (0..wire25::ITEMS.len()).filter(|i| i % PARTS == part) {
        let lump = wire25::ITEMS[i].0;
        let (data, commands, _) = load_data(i);
        for sb in [10u8, 11] {
            let golden = read_stream(lump, sb);
            let mut gate = Gate {
                golden: &golden,
                ctx: format!("{lump} sb{sb}"),
                failures: Vec::new(),
                counts: Counts::default(),
                taken: Vec::new(),
                snd_draws: 0,
                sink: RecordingSink::default(),
            };
            let ending = if lump == "M5CHEAT" {
                let adjust = |t: u64, i: &mut _, m: &mut _| cheats(t as i64, i, m);
                replay_adjusted(&data, sb, &commands, &mut gate, &adjust)
            } else {
                replay_full_resolution(&data, sb, &commands, &mut gate)
            };
            let frames = golden.frames.len();
            let want_end = match (lump, genuine_end(lump)) {
                ("BOSS1", _) => Some(Ending::Exit { secret: false }),
                (_, Some(r)) => Some(Ending::Refused {
                    tic: frames as u64,
                    witness: Witness::Frame(FrameRefusal::Draw(DisplayRefusal::Render(r))),
                }),
                _ => None,
            };
            if ending != want_end {
                gate.failures.push(format!(
                    "{lump} sb{sb}: ending {ending:?}, genuine {want_end:?}"
                ));
            }
            gate.counts.ends += usize::from(genuine_end(lump).is_some());
            if gate.counts.frames != frames {
                gate.failures.push(format!(
                    "{lump} sb{sb}: {} frames, genuine {frames}",
                    gate.counts.frames
                ));
            }
            if gate.taken != golden.messages {
                gate.failures
                    .push(format!("{lump} sb{sb}: messages differ"));
            }
            gate.counts.messages = golden.messages.len();
            gate.counts.buffers = gate.sink.buffers.len();
            let c = gate.counts;
            total.frames += c.frames;
            total.digests += c.digests;
            total.masked += c.masked;
            total.non_comparable += c.non_comparable;
            total.face_index += c.face_index;
            total.face_zeroed += c.face_zeroed;
            total.ends += c.ends;
            total.messages += c.messages;
            total.buffers += c.buffers;
            total.detector_firings += c.detector_firings;
            failures.extend(gate.failures);
        }
    }
    eprintln!("m5c frame gate part {part}: {total:?}");
    failures.truncate(40);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    total
}

const PARTS: usize = 4;

const fn counts(c: [usize; 9]) -> Counts {
    Counts {
        frames: c[0],
        digests: c[1],
        masked: c[2],
        non_comparable: c[3],
        face_index: c[4],
        face_zeroed: c[5],
        ends: c[6],
        messages: c[7],
        buffers: c[8],
        detector_firings: 0,
    }
}

/// The counts of each part (frames, whole digests, masked, not compared, face index agreements,
/// face-zeroed agreements, renderer ends, messages, buffers handed to the sink), so that a change
/// in what is compared or skipped fails; the detector never fires on these items.
const PART_COUNTS: [Counts; PARTS] = [
    counts([20548, 20548, 0, 0, 20548, 20548, 0, 38, 12656]),
    counts([8968, 8668, 300, 0, 8968, 8968, 0, 112, 5534]),
    counts([9760, 9690, 0, 70, 9760, 9690, 2, 32, 6020]),
    counts([6264, 6264, 0, 0, 6264, 6264, 4, 18, 3872]),
];

/// The 45,540 golden frames (22,770 per screen size), every face index equal, 6 renderer ends,
/// 200 messages, and 28,082 buffers (14,041 per screen size: the sound does not depend on the
/// screen size). Of the 45,470 frames that are not switch-parity frames (the 70 are not
/// digest-compared), 45,170 have their whole-frame and face-zeroed digests compared; the other
/// 300 are tracker-flagged frames, compared on the masked digest and flagged columns, whose
/// face-zeroed digest is `undefined` in the goldens and is counted without a comparison.
#[test]
fn the_parts_sum_to_the_totals() {
    let sum = PART_COUNTS.iter().fold([0; 9], |a, c| {
        [
            a[0] + c.frames,
            a[1] + c.digests,
            a[2] + c.masked,
            a[3] + c.non_comparable,
            a[4] + c.face_index,
            a[5] + c.face_zeroed,
            a[6] + c.ends,
            a[7] + c.messages,
            a[8] + c.buffers,
        ]
    });
    assert_eq!(
        sum,
        [45_540, 45_170, 300, 70, 45_540, 45_470, 6, 200, 28_082]
    );
}

macro_rules! parts {
    ($($name:ident = $n:expr),*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(run_part($n), PART_COUNTS[$n]);
            }
        )*
    };
}

parts!(
    items_part_0 = 0,
    items_part_1 = 1,
    items_part_2 = 2,
    items_part_3 = 3
);
