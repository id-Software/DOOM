//! M5a: the rewind test (owner ruling on R5, R6 and R7, 2026-11-15). The renderer's own
//! history (the view shadow, the switch tracker, the mapped-line set, `fuzzpos` and the
//! persistent framebuffer with its taint) is a self-contained [`RenderState`]; saved next to a
//! simulation snapshot of the same tic and restored, it must render the following tics exactly
//! as an uninterrupted run.
//!
//! doom-sim has no API that restores a simulation from its canonical snapshot (its
//! `Simulation::snapshot` is a read-only projection), and this milestone does not edit
//! doom-sim. `Simulation` implements `Clone`, so the simulation's restore point is a clone
//! taken at tic N-k. The render state goes through its byte encoding: at tic N-k the
//! uninterrupted renderer's state is encoded (`DRST` v2, docs/STATE_FORMAT.md), decoded and
//! restored into a fresh renderer, which then observes and renders tics N-k+1..=N from the
//! cloned simulation. Every one of those frames and reports, and the encoded render state at
//! tic N, must equal the uninterrupted run's; every rewound frame must also match its genuine
//! golden (decision R7's non-comparable M5SWTCH frames excepted).
mod common;

use common::{Outcome, compare, items, load, read_frames};
use doom_render::{Detail, Frame, FrameReport, RenderConfig, RenderState, Renderer};
use doom_sim::TicCommand;

/// One rewind: the item, the restore tic N-k and the target tic N (simulation tics).
struct Case {
    lump: &'static str,
    restore: u64,
    target: u64,
    why: &'static str,
}

const CASES: &[Case] = &[
    Case {
        lump: "M5FUZZ",
        restore: 40,
        target: 120,
        why: "spectres: fuzz columns and the persistent fuzzpos",
    },
    Case {
        lump: "BINF1",
        restore: 150,
        target: 260,
        why: "partial invisibility: fuzzed weapon sprites",
    },
    Case {
        lump: "M5SWTCH",
        restore: 20,
        target: 160,
        why: "restored inside the parity-unknown window after a re-press (decision R7)",
    },
    Case {
        lump: "M5VIEWZ",
        restore: 39,
        target: 380,
        why: "restored inside a step-up dip; landing and death descent after it (decision R6)",
    },
    Case {
        lump: "M5DSEG",
        restore: 30,
        target: 82,
        why: "mapped-lines-heavy (515 lines mapped; decision R5) and the MAXDRAWSEGS drop",
    },
];

fn renderer(run: &common::Run, screenblocks: u8) -> Renderer {
    Renderer::new(
        run.res.clone(),
        run.level.clone(),
        RenderConfig {
            screenblocks,
            detail: Detail::High,
        },
    )
    .expect("configuration")
}

fn commands(cmd: TicCommand) -> [TicCommand; 4] {
    [
        cmd,
        TicCommand::default(),
        TicCommand::default(),
        TicCommand::default(),
    ]
}

#[test]
fn restored_render_state_renders_like_an_uninterrupted_run() {
    let all = items();
    let mut rewound_frames = 0usize;
    for case in CASES {
        let item = all
            .iter()
            .find(|i| i.lump == case.lump)
            .unwrap_or_else(|| panic!("{}: no such item", case.lump));
        for screenblocks in [11u8, 10, 9] {
            let golden = read_frames(case.lump, screenblocks);
            let mut run = load(item);
            assert!(
                case.restore < case.target && case.target as usize <= run.commands.len(),
                "{}: bad case",
                case.lump
            );
            // The uninterrupted run, keeping the frames after the restore point.
            let mut straight = renderer(&run, screenblocks);
            straight.begin_level(&run.sim).expect("level start");
            let mut frame = Frame::new();
            let mut saved: Option<(doom_sim::Simulation, Vec<u8>)> = None;
            let mut expected: Vec<(Vec<u8>, FrameReport)> = Vec::new();
            for index in 0..case.target as usize {
                common::advance(&mut run, index).expect("advance");
                straight.observe(&run.sim).expect("observe");
                let report = straight.render(&run.sim, &mut frame).expect("render");
                let tic = run.sim.tic();
                if tic == case.restore {
                    let state = straight.state().expect("state").encode();
                    saved = Some((run.sim.clone(), state));
                } else if tic > case.restore {
                    expected.push((frame.pixels().to_vec(), report));
                }
            }
            let at_target = straight.state().expect("state").encode();
            let (mut sim, bytes) = saved.expect("restore point reached");

            // The rewind: a fresh renderer from the decoded state, the cloned simulation.
            let state = RenderState::decode(&bytes).expect("decode");
            assert_eq!(state.encode(), bytes, "{}: re-encoding", case.lump);
            assert_eq!(state.tic, case.restore);
            let mut rewound = renderer(&run, screenblocks);
            rewound.restore(state).expect("restore");
            let mut frame = Frame::new();
            for (k, index) in (case.restore as usize..case.target as usize).enumerate() {
                sim.advance(commands(run.commands[index])).expect("advance");
                rewound.observe(&sim).expect("observe");
                let report = rewound.render(&sim, &mut frame).expect("render");
                let (pixels, want) = &expected[k];
                assert!(
                    frame.pixels() == pixels.as_slice() && &report == want,
                    "{} sb{screenblocks}: rewound frame of tic {} differs ({})",
                    case.lump,
                    sim.tic(),
                    case.why
                );
                // The genuine dumped tic of simulation tic t is t-1 (tests/common `harness`).
                let row = &golden.frames[&(sim.tic() as i64 - 1)];
                let outcome = compare(row, &Ok(report), &frame);
                assert!(
                    matches!(outcome, Outcome::Match { flags_agree: true })
                        || (case.lump == "M5SWTCH" && outcome == Outcome::NonComparable),
                    "{} sb{screenblocks} tic {}: {outcome:?}",
                    case.lump,
                    sim.tic()
                );
                rewound_frames += 1;
            }
            assert_eq!(
                rewound.state().expect("state").encode(),
                at_target,
                "{} sb{screenblocks}: render state at tic {}",
                case.lump,
                case.target
            );
        }
    }
    eprintln!("rewound frames compared: {rewound_frames}");
    let want: u64 = CASES.iter().map(|c| 3 * (c.target - c.restore)).sum();
    assert_eq!(rewound_frames as u64, want);
}

/// The test can fail: a render state restored with a different `fuzzpos` renders M5FUZZ's
/// next fuzzed frame differently.
#[test]
fn a_changed_render_state_is_detected() {
    let item = items()
        .into_iter()
        .find(|i| i.lump == "M5FUZZ")
        .expect("M5FUZZ");
    let mut run = load(&item);
    let mut a = renderer(&run, 10);
    a.begin_level(&run.sim).expect("level start");
    let mut frame = Frame::new();
    let mut index = 0;
    // Advance to the first frame whose fuzz moves fuzzpos.
    let (mut b, before) = loop {
        let before = a.state().expect("state").clone();
        let sim_before = run.sim.clone();
        common::advance(&mut run, index).expect("advance");
        index += 1;
        a.observe(&run.sim).expect("observe");
        let r = a.render(&run.sim, &mut frame).expect("render");
        if r.fuzzpos != before.fuzzpos {
            break (sim_before, before);
        }
    };
    let mut changed = before;
    changed.fuzzpos = (changed.fuzzpos + 1) % 50;
    let mut r = renderer(&run, 10);
    r.restore(changed).expect("restore");
    b.advance(commands(run.commands[index - 1]))
        .expect("advance");
    r.observe(&b).expect("observe");
    let mut other = Frame::new();
    r.render(&b, &mut other).expect("render");
    assert_ne!(other.pixels(), frame.pixels());
}

/// A restored render state must fit the renderer's level (Rust-only safety, investigation §7
/// item 47). The independent review reproduced two panics with states the decoder accepted:
/// a mapped-line set of the wrong length (an out-of-bounds index in `R_StoreWallRange`'s
/// `ML_MAPPED` write) and `powers[0] = i32::MIN` (an overflow in the view shadow's
/// countdown). Both are now typed errors, the renderer keeps its state, and the good state
/// still restores and continues exactly.
#[test]
fn a_restored_state_must_fit_the_level() {
    use doom_render::{RenderRefusal, RestoreError, StateDecodeError};
    let item = items()
        .into_iter()
        .find(|i| i.lump == "M5DSEG")
        .expect("M5DSEG");
    let mut run = load(&item);
    let mut a = renderer(&run, 11);
    a.begin_level(&run.sim).expect("level start");
    let mut frame = Frame::new();
    common::advance(&mut run, 0).expect("advance");
    a.observe(&run.sim).expect("observe");
    a.render(&run.sim, &mut frame).expect("render");
    let good = a.state().expect("state").clone();
    let lines = run.level.map.linedefs().len();
    let sectors = run.level.map.sectors().len();

    let mut fresh = renderer(&run, 11);
    // The reviewer's first case: the mapped set cleared; the level-independent decoder
    // accepts it, the restore does not.
    let mut bad = good.clone();
    bad.mapped.clear();
    let decoded = RenderState::decode(&bad.encode()).expect("the decoder does not know the level");
    assert_eq!(
        fresh.restore(decoded),
        Err(RestoreError::Count {
            field: "mapped",
            expected: lines,
            got: 0
        })
    );
    assert!(fresh.state().is_none(), "a refused restore keeps the state");
    assert!(matches!(
        fresh.render(&run.sim, &mut frame),
        Err(RenderRefusal::NotObserved { observed: None, .. })
    ));
    // The reviewer's second case: powers[0] = i32::MIN, refused by the decoder and by the
    // restore of a state built by hand.
    let mut bad = good.clone();
    bad.view.powers[0] = i32::MIN;
    assert_eq!(
        RenderState::decode(&bad.encode()),
        Err(StateDecodeError::Invalid("powers"))
    );
    assert_eq!(fresh.restore(bad), Err(RestoreError::Value("powers")));
    // The other level checks.
    let mut bad = good.clone();
    bad.view.sector_floors.push(0);
    assert_eq!(
        fresh.restore(bad),
        Err(RestoreError::Count {
            field: "sector_floors",
            expected: sectors,
            got: sectors + 1
        })
    );
    let mut bad = good.clone();
    bad.screen.pop();
    assert!(matches!(
        fresh.restore(bad),
        Err(RestoreError::Count {
            field: "screen",
            ..
        })
    ));
    let mut bad = good.clone();
    bad.switches.buttons[0] = doom_render::p_spec::ButtonSlot {
        timer: 5,
        line: lines as u16,
    };
    assert_eq!(
        fresh.restore(bad),
        Err(RestoreError::Line {
            field: "button",
            line: lines as u16
        })
    );
    let mut bad = good.clone();
    bad.switches.unknown = vec![u16::MAX];
    assert_eq!(
        fresh.restore(bad),
        Err(RestoreError::Line {
            field: "unknown",
            line: u16::MAX
        })
    );
    let mut bad = good.clone();
    bad.switches.cleared = vec![u16::MAX];
    assert_eq!(
        fresh.restore(bad),
        Err(RestoreError::Line {
            field: "cleared",
            line: u16::MAX
        })
    );
    let mut bad = good.clone();
    bad.fuzzpos = 50;
    assert_eq!(fresh.restore(bad), Err(RestoreError::Value("fuzzpos")));
    assert!(fresh.state().is_none());

    // The good state restores and continues as the uninterrupted renderer.
    fresh.restore(good).expect("restore");
    common::advance(&mut run, 1).expect("advance");
    a.observe(&run.sim).expect("observe");
    fresh.observe(&run.sim).expect("observe");
    let mut other = Frame::new();
    assert_eq!(
        a.render(&run.sim, &mut frame),
        fresh.render(&run.sim, &mut other)
    );
    assert_eq!(frame, other);
    assert_eq!(a.state(), fresh.state());
}
