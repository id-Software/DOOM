//! M5c: the composed display under the new random contract (`RandomContract::WithSound`) against
//! the genuine full-frame goldens of `tests/reference/m5c_frames` (DOOM-M5C-FRAMES-v1, written by
//! the C only: `tools/reference/m5c_reference.py`, linuxdoom-1.10 built with its in-process sound
//! code, one define removed).
//!
//! This is a display-level check with the sound draws taken from the goldens (an explicit fixture
//! projection: the genuine Q row's cumulative `snd_draws`, differenced per tic, is what
//! `Display::observe_after` receives); the end-to-end gate, with the draws from the Rust sound
//! session, is crates/doom-game/tests/m5c_frames.rs. Every item of the 64 runs once through the
//! wire-25 simulation; after every committed `advance` one display per screen size observes the
//! tic with its sound draws and draws. Per frame: the whole-frame digest (the masked digest where
//! the genuine tracker flagged columns, as M5a and M5b), the face-zeroed digest of the Q row, the
//! renderer counters, `fuzzpos` and the newly mapped lines, and the P row's palette, palette set,
//! face, `message_on` and `M_Random` index (which includes the wipe's 320 draws after the frame of
//! tic 0, f_wipe.c:159-166); per tic, the message taken against the X rows.
mod common;
mod m5c;

use doom_render::{Display, DisplayConfig, DisplayRefusal, Frame, RandomContract};
use m5c::{Golden, cheats, genuine_end, load, noface, read_stream, wire25};

/// What was compared.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Frames drawn and compared with a golden frame.
    pub frames: usize,
    /// Compared on the whole-frame digest.
    pub digests: usize,
    /// Compared on the masked digest (tracker-flagged frames).
    pub masked: usize,
    /// Not digest-compared (the renderer reports the switch parity unknown).
    pub non_comparable: usize,
    /// Frames whose face index (P row) agrees: every frame.
    pub face_index: usize,
    /// Digest-compared frames (whole or masked) whose face-zeroed digest (Q row) also agrees, or
    /// whose Q digest is `undefined` (tracker-flagged).
    pub face_zeroed: usize,
    /// Ends checked (a render refusal where the genuine run ended at a renderer limit).
    pub ends: usize,
    /// Messages compared.
    pub messages: usize,
}

const PARTS: usize = 4;

fn run_part(part: usize) -> Counts {
    let mut total = Counts::default();
    let mut failures = Vec::new();
    for i in (0..wire25::ITEMS.len()).filter(|i| i % PARTS == part) {
        let mut it = load(i);
        let configs = [10u8, 11];
        let streams: Vec<_> = configs
            .iter()
            .map(|&sb| read_stream(&it.lump, sb))
            .collect();
        let mut displays: Vec<Display> = configs
            .iter()
            .map(|&screenblocks| {
                let mut d = Display::new(
                    it.res.clone(),
                    it.level.clone(),
                    DisplayConfig { screenblocks },
                )
                .expect("display");
                d.begin_level(&it.sim).expect("level start");
                d.set_contract(RandomContract::WithSound).expect("contract");
                d
            })
            .collect();
        let mut frame = Frame::new();
        let mut done = [false; 2];
        let mut taken: Vec<Vec<(i64, i32, String)>> = vec![Vec::new(); 2];
        let mut drawn = [0usize; 2];
        for (t, cmd) in it.commands.iter().enumerate() {
            let tic = t as i64;
            if done.iter().all(|d| *d) {
                break;
            }
            if it
                .sim
                .advance([
                    *cmd,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                ])
                .is_err()
            {
                break;
            }
            for (k, d) in displays.iter_mut().enumerate() {
                if done[k] {
                    continue;
                }
                let ctx = format!("{} sb{} tic {tic}", it.lump, configs[k]);
                let g: Option<&Golden> = streams[k].frames.get(&tic);
                let before = if tic == 0 {
                    0
                } else {
                    streams[k].frames.get(&(tic - 1)).map_or(0, |p| p.snd_draws)
                };
                let draws = g.map_or(0, |g| g.snd_draws - before) as u16;
                let observed = if it.lump == "M5CHEAT" {
                    d.observe_after_with(&it.sim, cmd, draws, |i, m| cheats(tic, i, m))
                } else {
                    d.observe_after(&it.sim, cmd, draws)
                };
                match observed {
                    Ok(Some(m)) => taken[k].push((tic, m.code, m.text.into())),
                    Ok(None) => {}
                    Err(e) => {
                        failures.push(format!("{ctx}: observe {e:?}"));
                        done[k] = true;
                        continue;
                    }
                }
                let result = d.draw(&it.sim, &mut frame);
                let Some(g) = g else {
                    let want = genuine_end(&it.lump).map(DisplayRefusal::Render);
                    if want.is_none() || want != result.clone().err() {
                        failures.push(format!("{ctx}: no genuine frame, Rust {result:?}"));
                    }
                    total.ends += 1;
                    done[k] = true;
                    continue;
                };
                let f = match result {
                    Ok(f) => f,
                    Err(e) => {
                        failures.push(format!("{ctx}: refused {e:?}"));
                        done[k] = true;
                        continue;
                    }
                };
                total.frames += 1;
                drawn[k] += 1;
                if let Err(e) = d.state().expect("state").check(&it.sim) {
                    failures.push(format!("{ctx}: the restore checks refuse the state: {e:?}"));
                }
                let px = frame.pixels();
                let face_zeroed = doom_test_evidence::sha256(&noface(px, g.face_rect));
                if f.view.switch_parity_unknown {
                    total.non_comparable += 1;
                } else if g.columns.is_empty() {
                    total.digests += 1;
                    if doom_test_evidence::sha256(px) != g.sha256 {
                        let where_ = if face_zeroed == g.noface {
                            "inside the face rectangle only"
                        } else {
                            "outside the face rectangle"
                        };
                        failures.push(format!("{ctx}: frame differs ({where_})"));
                    }
                } else {
                    total.masked += 1;
                    let m = common::masked(px, &g.columns);
                    if doom_test_evidence::sha256(&m) != g.masked
                        || f.view.flagged_columns != g.columns
                    {
                        failures.push(format!("{ctx}: masked frame or flagged columns differ"));
                    }
                }
                total.face_index += usize::from(i64::from(f.face) == g.status[2]);
                if !f.view.switch_parity_unknown {
                    if g.noface == "undefined" || face_zeroed == g.noface {
                        total.face_zeroed += 1;
                    } else {
                        failures.push(format!("{ctx}: face-zeroed digest differs"));
                    }
                }
                if f.view.counters != g.counters
                    || f.view.fuzzpos != g.fuzzpos
                    || f.view.newly_mapped != g.mapped
                {
                    failures.push(format!("{ctx}: counters"));
                }
                let got = [
                    i64::from(f.palette),
                    f.palette_set.map_or(-1, i64::from),
                    i64::from(f.face),
                    i64::from(f.message_on),
                    i64::from(d.status().rndindex),
                ];
                if got != g.status {
                    failures.push(format!("{ctx}: P {got:?} genuine {:?}", g.status));
                }
            }
        }
        for (k, &sb) in configs.iter().enumerate() {
            if drawn[k] != streams[k].frames.len() {
                failures.push(format!(
                    "{} sb{sb}: {} frames drawn, {} genuine",
                    it.lump,
                    drawn[k],
                    streams[k].frames.len()
                ));
            }
            total.messages += streams[k].messages.len();
            if taken[k] != streams[k].messages {
                failures.push(format!("{} sb{sb}: messages differ", it.lump));
            }
        }
    }
    eprintln!("m5c display part {part}: {total:?}");
    failures.truncate(40);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    total
}

const fn counts(c: [usize; 8]) -> Counts {
    Counts {
        frames: c[0],
        digests: c[1],
        masked: c[2],
        non_comparable: c[3],
        face_index: c[4],
        face_zeroed: c[5],
        ends: c[6],
        messages: c[7],
    }
}

/// The counts of each part (frames, whole digests, masked, not compared, face index agreements,
/// face-zeroed agreements, ends, messages), so that a change in what is compared or skipped fails;
/// `the_parts_sum_to_the_totals` checks their sum.
const PART_COUNTS: [Counts; PARTS] = [
    counts([20548, 20548, 0, 0, 20548, 20548, 0, 38]),
    counts([8968, 8668, 300, 0, 8968, 8968, 0, 112]),
    counts([9760, 9690, 0, 70, 9760, 9690, 2, 32]),
    counts([6264, 6264, 0, 0, 6264, 6264, 4, 18]),
];

/// The 45,540 golden frames of the 64 items at both screen sizes (F rows of
/// tests/reference/m5c_frames, `m5c_results.json` totals), every face index equal, the 70 frames
/// whose switch parity the renderer reports unknown not digest-compared (the same 70 as M5b's
/// tests/m5b_display.rs), the 300 tracker-flagged frames on the masked digest, and the 6 ends of
/// M5VPLAN, M5OPEN and M5SSEG at both sizes.
#[test]
fn the_parts_sum_to_the_totals() {
    let sum = PART_COUNTS.iter().fold([0; 8], |a, c| {
        [
            a[0] + c.frames,
            a[1] + c.digests,
            a[2] + c.masked,
            a[3] + c.non_comparable,
            a[4] + c.face_index,
            a[5] + c.face_zeroed,
            a[6] + c.ends,
            a[7] + c.messages,
        ]
    });
    assert_eq!(sum, [45_540, 45_170, 300, 70, 45_540, 45_470, 6, 200]);
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
