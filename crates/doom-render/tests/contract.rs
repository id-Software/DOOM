//! M5a: the renderer's public contract on the committed synthetic rooms, from facts already
//! recorded (`docs/M5A_REFERENCE_INVESTIGATION.md` §1.9 and §2.7, decisions R2, R3, R5 and R8;
//! `docs/validation/M5A.md`): rendering never writes the simulation, a refused frame changes
//! nothing and repeats, the call order is enforced with typed errors, and a configuration
//! outside screenblocks 9-11 at high detail is refused.
mod common;

use common::{advance, items, load};
use doom_render::{
    ConfigError, Detail, Feature, Frame, ObserveError, RenderConfig, RenderRefusal, Renderer,
    view::ShadowError,
};

fn cfg(screenblocks: u8) -> RenderConfig {
    RenderConfig {
        screenblocks,
        detail: Detail::High,
    }
}

fn item(lump: &str) -> common::Item {
    items()
        .into_iter()
        .find(|i| i.lump == lump)
        .unwrap_or_else(|| panic!("{lump}"))
}

#[test]
fn configurations_outside_the_admitted_domain_are_refused() {
    // Decision R2: screenblocks 9, 10 and 11 at high detail; low detail is refused (the genuine
    // `R_DrawColumnLow` doubles `dc_x` in place, investigation §2.2.3, §2.9).
    let run = load(&item("M5AMP"));
    for screenblocks in [0, 1, 8, 12, 13, 255] {
        assert_eq!(
            Renderer::new(run.res.clone(), run.level.clone(), cfg(screenblocks)).err(),
            Some(ConfigError::ScreenBlocks(screenblocks))
        );
    }
    for screenblocks in [9, 10, 11] {
        assert_eq!(
            Renderer::new(
                run.res.clone(),
                run.level.clone(),
                RenderConfig {
                    screenblocks,
                    detail: Detail::Low
                }
            )
            .err(),
            Some(ConfigError::LowDetail)
        );
        assert!(Renderer::new(run.res.clone(), run.level.clone(), cfg(screenblocks)).is_ok());
    }
}

#[test]
fn render_and_observe_follow_the_documented_call_order() {
    // lib.rs: `begin_level` at tic 0, then `observe` after every committed `advance`, then
    // `render`; a frame of an unobserved tic is refused and observing out of order is a gap.
    let mut run = load(&item("M5AMP"));
    let mut r = Renderer::new(run.res.clone(), run.level.clone(), cfg(11)).unwrap();
    let mut frame = Frame::new();
    assert_eq!(
        r.render(&run.sim, &mut frame),
        Err(RenderRefusal::NotObserved {
            observed: None,
            simulation: 0
        })
    );
    assert_eq!(
        r.observe(&run.sim),
        Err(ObserveError::Shadow(ShadowError::Gap {
            expected: 0,
            got: 0
        }))
    );
    assert!(r.state().is_none(), "a failed observe leaves no state");
    r.begin_level(&run.sim).unwrap();
    assert!(r.render(&run.sim, &mut frame).is_ok());
    advance(&mut run, 0).unwrap();
    // The simulation moved to tic 1; the renderer last observed tic 0.
    let before = r.state().cloned();
    assert_eq!(
        r.render(&run.sim, &mut frame),
        Err(RenderRefusal::NotObserved {
            observed: Some(0),
            simulation: 1
        })
    );
    assert_eq!(
        r.state().cloned(),
        before,
        "a refusal leaves the state unchanged"
    );
    // Observing the same tic twice is a gap, and keeps the state.
    r.observe(&run.sim).unwrap();
    let observed = r.state().cloned();
    assert_eq!(
        r.observe(&run.sim),
        Err(ObserveError::Shadow(ShadowError::Gap {
            expected: 2,
            got: 1
        }))
    );
    assert_eq!(r.state().cloned(), observed);
    assert!(r.render(&run.sim, &mut frame).is_ok());
}

/// Runs `lump` twice at `screenblocks`: once as the bare simulation, once with a renderer
/// observing and rendering every tic. Returns the number of rendered tics.
fn render_never_writes_the_simulation(lump: &str, screenblocks: u8) -> usize {
    let item = item(lump);
    let mut plain = load(&item);
    let mut seen = load(&item);
    let mut r = Renderer::new(seen.res.clone(), seen.level.clone(), cfg(screenblocks)).unwrap();
    r.begin_level(&seen.sim).unwrap();
    let mut frame = Frame::new();
    let mut rendered = 0;
    for t in 0..seen.commands.len() {
        advance(&mut plain, t).unwrap();
        advance(&mut seen, t).unwrap();
        r.observe(&seen.sim).unwrap();
        let digest = seen.sim.snapshot().digest();
        match r.render(&seen.sim, &mut frame) {
            Ok(_) => rendered += 1,
            Err(_) => break, // a limit room's refusal ends the run (decision R3)
        }
        // §1.9, §2.7: the renderer reads the simulation only; ML_MAPPED lives in the render
        // state (decision R5), so even the line flags are those of the never-rendered run.
        assert_eq!(
            seen.sim.snapshot().digest(),
            digest,
            "{lump} tic {t}: render wrote"
        );
        assert_eq!(
            seen.sim.snapshot().digest(),
            plain.sim.snapshot().digest(),
            "{lump} tic {t}: the rendered run diverged from the bare run"
        );
    }
    assert_eq!(
        seen.sim.snapshot().canonical_bytes(),
        plain.sim.snapshot().canonical_bytes(),
        "{lump}"
    );
    rendered
}

/// Rooms that exercise the stages with the most shared state: masked walls and sprites, fuzz
/// (the persistent `fuzzpos`), a reusable switch, the drawseg drop and the weapon. Each room is
/// its own test (`rendering_never_changes_the_simulation_in_room_*`), so the eight run on
/// parallel threads; `the_eight_rooms_are_all_tested` fails if one is dropped from the list or
/// if a room's generated test is deleted from the `rooms!` invocation.
const SHARED_STATE_ROOMS: [&str; 8] = [
    "M5AMP", "M5MASK", "M5SPRIT", "M5FUZZ", "M5SWTCH", "M5DSEG", "M5WEAP", "TELE1",
];

fn rendering_never_changes_the_simulation_in(room: usize) {
    let lump = SHARED_STATE_ROOMS[room];
    let mut total = 0;
    for screenblocks in [11, 10, 9] {
        let n = render_never_writes_the_simulation(lump, screenblocks);
        assert!(n > 0, "{lump} sb{screenblocks}: no frame rendered");
        total += n;
    }
    eprintln!("{lump}: {total} frames rendered without a simulation change");
}

/// Generates one test per room and `GENERATED_ROOMS`, the rooms it generated tests for, in the
/// order given.
macro_rules! rooms {
    ($($name:ident: $room:expr),*) => {
        $(#[test] fn $name() { rendering_never_changes_the_simulation_in($room); })*
        const GENERATED_ROOMS: &[usize] = &[$($room),*];
    };
}

rooms!(
    rendering_never_changes_the_simulation_in_room_m5amp: 0,
    rendering_never_changes_the_simulation_in_room_m5mask: 1,
    rendering_never_changes_the_simulation_in_room_m5sprit: 2,
    rendering_never_changes_the_simulation_in_room_m5fuzz: 3,
    rendering_never_changes_the_simulation_in_room_m5swtch: 4,
    rendering_never_changes_the_simulation_in_room_m5dseg: 5,
    rendering_never_changes_the_simulation_in_room_m5weap: 6,
    rendering_never_changes_the_simulation_in_room_tele1: 7
);

#[test]
fn the_eight_rooms_are_all_tested() {
    assert_eq!(
        SHARED_STATE_ROOMS,
        [
            "M5AMP", "M5MASK", "M5SPRIT", "M5FUZZ", "M5SWTCH", "M5DSEG", "M5WEAP", "TELE1"
        ]
    );
    assert_eq!(
        GENERATED_ROOMS,
        (0..SHARED_STATE_ROOMS.len()).collect::<Vec<_>>(),
        "a test per room"
    );
}

#[test]
fn a_limit_refusal_is_typed_repeatable_and_changes_nothing() {
    // Decision R3 (docs/validation/M5A.md, "Refusals by kind"): the three limit rooms refuse
    // the tic after the genuine run ends, in every configuration.
    for (lump, want) in [
        ("M5VPLAN", RenderRefusal::VisplaneOverflow),
        ("M5OPEN", RenderRefusal::OpeningOverflow),
        ("M5SSEG", RenderRefusal::SolidSegOverflow),
    ] {
        for screenblocks in [11, 10, 9] {
            let mut run = load(&item(lump));
            let mut r =
                Renderer::new(run.res.clone(), run.level.clone(), cfg(screenblocks)).unwrap();
            r.begin_level(&run.sim).unwrap();
            let mut frame = Frame::new();
            let mut refusal = None;
            for t in 0..run.commands.len() {
                advance(&mut run, t).unwrap();
                r.observe(&run.sim).unwrap();
                let (state, digest) = (r.state().cloned(), run.sim.snapshot().digest());
                match r.render(&run.sim, &mut frame) {
                    Ok(_) => {}
                    Err(e) => {
                        // The render state and the simulation are untouched, and the same
                        // tic refuses again identically.
                        assert_eq!(r.state().cloned(), state, "{lump} sb{screenblocks}");
                        assert_eq!(run.sim.snapshot().digest(), digest);
                        assert_eq!(r.render(&run.sim, &mut Frame::new()), Err(e.clone()));
                        refusal = Some(e);
                        break;
                    }
                }
            }
            assert_eq!(refusal, Some(want.clone()), "{lump} sb{screenblocks}");
        }
    }
}

#[test]
fn the_per_pixel_trace_is_off_by_default_and_covers_the_view() {
    // `set_trace` enables a record of which stage wrote each of the 320x200 pixels of the last
    // frame (lib.rs, `Feature`); it is first-difference reporting only.
    let run = load(&item("M5AMP"));
    let mut r = Renderer::new(run.res.clone(), run.level.clone(), cfg(11)).unwrap();
    assert!(r.trace().is_none());
    r.set_trace(true);
    assert_eq!(r.trace().map(<[Feature]>::len), Some(320 * 200));
    assert!(r.trace().unwrap().iter().all(|f| *f == Feature::Untouched));
    r.begin_level(&run.sim).unwrap();
    let mut frame = Frame::new();
    r.render(&run.sim, &mut frame).unwrap();
    let t = r.trace().unwrap();
    assert_eq!(t.len(), 320 * 200);
    assert!(
        t.iter().any(|f| *f != Feature::Untouched),
        "a rendered frame writes pixels"
    );
    r.set_trace(false);
    assert!(r.trace().is_none());
}

#[test]
fn refusals_and_errors_display_their_cause() {
    // The `Display` of each public error names the failure and the variant (diagnostics for
    // the viewer tools; no caller parses it).
    assert_eq!(
        ConfigError::LowDetail.to_string(),
        "render configuration refused: LowDetail"
    );
    assert_eq!(
        ConfigError::ScreenBlocks(8).to_string(),
        "render configuration refused: ScreenBlocks(8)"
    );
    assert_eq!(
        RenderRefusal::VisplaneOverflow.to_string(),
        "frame refused: VisplaneOverflow"
    );
    assert_eq!(
        RenderRefusal::RangeCheck("x").to_string(),
        "frame refused: RangeCheck(\"x\")"
    );
    assert_eq!(
        ObserveError::SwitchInconsistency.to_string(),
        "tic not observed: SwitchInconsistency"
    );
}
