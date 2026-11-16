#![no_main]
//! Rust software renderer (`doom-render`, M5a) on random but valid states of the committed
//! synthetic maps: the 23 M5a rooms (`tests/reference/m5a_wads/*.wad`, map E1M1) and the 18 M4b
//! mechanism WADs (`tests/reference/m4b_wads/*.wad`, map E1M1, BOSS8 in the E1M8 slot), each
//! with the committed synthetic graphics set `tests/reference/m5a_gfx.wad` (no IWAD, no id art).
//! Each input loads one WAD as the renderer tests do (`Simulation::original_player_episode1_v2_map`,
//! `spawn_map_things`, `initialize_player_weapons`, `spawn_level_specials`; `Resources::load` with
//! the map WAD then the graphics WAD; `Level::load`), runs the byte-derived slot-One commands, and
//! observes and renders every committed tic (and tic 0).
//!
//! Input: a four-byte header, then four-byte command records. Byte 0 picks the WAD
//! (`data[0] % 41`, order of [`WADS`]); byte 1 the view and skill (`9 + data[1] % 3` screenblocks;
//! bit 2 skill 3 when set, skill 2 when clear; bits 3-7 the gameplay RNG index of the start, 0 the
//! default context as the committed scripts run); byte 2 the low byte of every command's
//! `angleturn`; byte 3 the tic budget (`16 << (data[3] % 4)` tics). A record is a version-110 demo
//! ticcmd (forward, side, `angleturn` high byte, buttons), so the command bytes of the committed
//! `.lmp` scripts are records as they stand; buttons bits 0x40/0x80 are the record's repeat count
//! (none 1, 0x40 4, 0x80 16, both 64 tics). Commands are clamped into the simulation's admitted
//! domain (movement within +-50, weapon 0-3 only with `BT_CHANGE`) so that refusals of the
//! simulation are the rare declared witnesses, not the common case; a refused or non-observable
//! tic ends the run. `-max_len=516` (4 + 4 x 128 tics, the largest budget).
//!
//! Properties: no panic; every refused frame is a typed `RenderRefusal` (never `NotObserved`,
//! because the harness observes every tic) that leaves the render state byte-identical and
//! repeats on a cloned renderer and simulation; every drawn frame is deterministic (a cloned
//! renderer and simulation produce the identical frame, report and render state); the render
//! state round-trips through its `DRST` encoding (decode then encode gives the same bytes),
//! and a mutated or truncated encoding never panics; rendering never changes the simulation
//! (its snapshot digest is the same before and after every frame, and the clone's canonical bytes
//! equal the original's). The clone, the `DRST` round trip and the canonical-bytes comparison run
//! on tic 0, every fourth tic and the last, to keep an input's cost bounded. No
//! property compares pixels with the genuine renderer: that is the committed golden harness
//! (`crates/doom-render/tests/frames.rs`), not a fuzz oracle.
use doom_render::{
    Detail, Frame, GameMode, Level, LumpDirectory, RenderConfig, RenderRefusal, RenderState,
    Renderer, Resources,
};
use doom_sim::*;
use doom_types::DoomRng;
use libfuzzer_sys::fuzz_target;
use std::sync::{Arc, OnceLock};

/// The synthetic graphics WAD every run's lump list holds after the map WAD.
const GFX: &[u8] = include_bytes!("../../tests/reference/m5a_gfx.wad");
/// The 23 M5a rooms (`tests/reference/m5a_manifest.json`, by file name) and the 18 M4b WADs
/// (`m4b_manifest.json`, `m4b_manifest2.json`; the order of `player_episode1_v2.rs`).
const WADS: [&[u8]; 41] = [
    include_bytes!("../../tests/reference/m5a_wads/amp.wad"),
    include_bytes!("../../tests/reference/m5a_wads/ampflick.wad"),
    include_bytes!("../../tests/reference/m5a_wads/clip.wad"),
    include_bytes!("../../tests/reference/m5a_wads/contrast.wad"),
    include_bytes!("../../tests/reference/m5a_wads/dist.wad"),
    include_bytes!("../../tests/reference/m5a_wads/dsegs.wad"),
    include_bytes!("../../tests/reference/m5a_wads/fuzz.wad"),
    include_bytes!("../../tests/reference/m5a_wads/hom.wad"),
    include_bytes!("../../tests/reference/m5a_wads/light.wad"),
    include_bytes!("../../tests/reference/m5a_wads/mask.wad"),
    include_bytes!("../../tests/reference/m5a_wads/openings.wad"),
    include_bytes!("../../tests/reference/m5a_wads/peg.wad"),
    include_bytes!("../../tests/reference/m5a_wads/planes.wad"),
    include_bytes!("../../tests/reference/m5a_wads/short.wad"),
    include_bytes!("../../tests/reference/m5a_wads/sky.wad"),
    include_bytes!("../../tests/reference/m5a_wads/solidsegs.wad"),
    include_bytes!("../../tests/reference/m5a_wads/sprites.wad"),
    include_bytes!("../../tests/reference/m5a_wads/switch.wad"),
    include_bytes!("../../tests/reference/m5a_wads/view.wad"),
    include_bytes!("../../tests/reference/m5a_wads/viewz.wad"),
    include_bytes!("../../tests/reference/m5a_wads/vplanes.wad"),
    include_bytes!("../../tests/reference/m5a_wads/vsprites.wad"),
    include_bytes!("../../tests/reference/m5a_wads/weapon.wad"),
    include_bytes!("../../tests/reference/m4b_wads/tele.wad"),
    include_bytes!("../../tests/reference/m4b_wads/telemove.wad"),
    include_bytes!("../../tests/reference/m4b_wads/tfrag.wad"),
    include_bytes!("../../tests/reference/m4b_wads/montele.wad"),
    include_bytes!("../../tests/reference/m4b_wads/slideb.wad"),
    include_bytes!("../../tests/reference/m4b_wads/slidec.wad"),
    include_bytes!("../../tests/reference/m4b_wads/baron.wad"),
    include_bytes!("../../tests/reference/m4b_wads/infight.wad"),
    include_bytes!("../../tests/reference/m4b_wads/bvb.wad"),
    include_bytes!("../../tests/reference/m4b_wads/boss8.wad"),
    include_bytes!("../../tests/reference/m4b_wads/bossm1.wad"),
    include_bytes!("../../tests/reference/m4b_wads/spech8.wad"),
    include_bytes!("../../tests/reference/m4b_wads/spech9.wad"),
    include_bytes!("../../tests/reference/m4b_wads/skill.wad"),
    include_bytes!("../../tests/reference/m4b_wads/tele2x.wad"),
    include_bytes!("../../tests/reference/m4b_wads/sawbite.wad"),
    include_bytes!("../../tests/reference/m4b_wads/btunnel.wad"),
    include_bytes!("../../tests/reference/m4b_wads/telez.wad"),
];
/// The map number each WAD runs under (BOSS8 is the E1M8 slot).
fn map_number(index: usize) -> u8 {
    if index == 23 + 9 { 8 } else { 1 }
}
const HEADER: usize = 4;
const RECORD: usize = 4;
const MAX_MOVE: i8 = 50;

struct Setup {
    map: doom_map::Map,
    res: Arc<Resources>,
    level: Arc<Level>,
}

fn setup(index: usize) -> &'static Setup {
    static SETUPS: [OnceLock<Setup>; 41] = [const { OnceLock::new() }; 41];
    SETUPS[index].get_or_init(|| {
        let wad = doom_wad::Wad::parse(WADS[index], doom_wad::Limits::default()).unwrap();
        let gfx = doom_wad::Wad::parse(GFX, doom_wad::Limits::default()).unwrap();
        let mut dir = LumpDirectory::new();
        dir.add_wad(&wad).unwrap();
        dir.add_wad(&gfx).unwrap();
        let res = Resources::load(dir, GameMode::Registered)
            .expect("a committed WAD loads with the committed graphics");
        let slot = format!("E1M{}", map_number(index));
        let map = doom_map::Map::from_wad(&wad, &slot, doom_map::MapLimits::default()).unwrap();
        let level = Level::load(&res, &map).expect("a committed map resolves its textures");
        Setup {
            map,
            res: Arc::new(res),
            level: Arc::new(level),
        }
    })
}

fn start(index: usize, byte: u8) -> Simulation {
    let skill = if byte & 4 == 0 {
        Skill::Medium
    } else {
        Skill::Hard
    };
    let rng = byte >> 3;
    let options = if rng == 0 {
        MapContextOptions::default()
    } else {
        MapContextOptions {
            initial_rng: DoomRng::from_indices(rng, 0),
            respawn_monsters: false,
        }
    };
    let mut s = Simulation::original_player_episode1_v2_map(
        setup(index).map.clone(),
        map_number(index),
        skill,
        options,
    )
    .expect("a committed WAD loads under wire 24");
    s.spawn_map_things().expect("its THINGs spawn");
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("the default inventory");
    s.spawn_level_specials().expect("its specials start");
    s
}

/// The slot-One commands of an input, expanded, clamped into the admitted domain and bounded.
fn commands(data: &[u8], budget: usize) -> Vec<TicCommand> {
    let low = data[2];
    let mut out = Vec::new();
    for r in data[HEADER..].as_chunks::<RECORD>().0 {
        let repeat = [1, 4, 16, 64][usize::from(r[3] >> 6)];
        let mut buttons = r[3] & 0x3f;
        let weapon = (buttons & BT_WEAPONMASK) >> 3;
        buttons &= !BT_WEAPONMASK;
        if buttons & BT_CHANGE != 0 {
            buttons |= (weapon & 3) << 3;
        }
        let c = TicCommand {
            forward: i8::from_le_bytes([r[0]]).clamp(-MAX_MOVE, MAX_MOVE),
            side: i8::from_le_bytes([r[1]]).clamp(-MAX_MOVE, MAX_MOVE),
            angle_turn: i16::from_le_bytes([low, r[2]]),
            buttons,
        };
        for _ in 0..repeat {
            if out.len() == budget {
                return out;
            }
            out.push(c);
        }
    }
    out
}

/// The frame of the observed tic, checked against the properties of the module documentation.
/// Every frame is rendered and the simulation digest compared before and after; with `full`
/// (tic 0, every fourth tic and the last) the frame is also rendered on a cloned renderer and
/// simulation and the render state is round-tripped through `DRST`. Returns false when the
/// frame was refused (the run ends).
fn frame(renderer: &mut Renderer, sim: &Simulation, flip: [u8; 3], full: bool) -> bool {
    let sim_digest = sim.snapshot().digest();
    let before = renderer.state().unwrap().clone();
    let mut twin = full.then(|| (renderer.clone(), sim.clone()));
    let mut a = Frame::new();
    let result = renderer.render(sim, &mut a);
    assert_eq!(
        sim.snapshot().digest(),
        sim_digest,
        "rendering changed the simulation"
    );
    if let Some((twin, sim_twin)) = twin.as_mut() {
        let mut b = Frame::new();
        let result_twin = twin.render(sim_twin, &mut b);
        assert_eq!(result, result_twin, "the same state rendered differently");
        assert_eq!(
            sim.snapshot().canonical_bytes(),
            sim_twin.snapshot().canonical_bytes()
        );
        if result.is_ok() {
            assert_eq!(a, b);
            assert_eq!(renderer.state(), twin.state());
        }
    }
    match result {
        Ok(_) => {
            if !full {
                return true;
            }
            let state = renderer.state().unwrap();
            let bytes = state.encode();
            let back = RenderState::decode(&bytes).expect("an encoded render state decodes");
            assert_eq!(&back, state);
            assert_eq!(back.encode(), bytes);
            let at = usize::from(u16::from_le_bytes([flip[0], flip[1]])) % bytes.len();
            let mut mutated = bytes.clone();
            mutated[at] ^= flip[2] | 1;
            let _ = RenderState::decode(&mutated);
            let _ = RenderState::decode(&bytes[..at]);
            true
        }
        Err(e) => {
            assert!(
                !matches!(e, RenderRefusal::NotObserved { .. }),
                "an observed tic was refused as not observed: {e:?}"
            );
            assert_eq!(
                renderer.state(),
                Some(&before),
                "a refusal changed the render state"
            );
            false
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < HEADER {
        return;
    }
    let index = usize::from(data[0]) % WADS.len();
    let cfg = RenderConfig {
        screenblocks: 9 + data[1] % 3,
        detail: Detail::High,
    };
    let budget = 16usize << (data[3] % 4);
    let setup = setup(index);
    let mut sim = start(index, data[1]);
    let mut renderer = Renderer::new(setup.res.clone(), setup.level.clone(), cfg).unwrap();
    renderer.begin_level(&sim).expect("tic 0 is observable");
    let flip = [data[1], data[2], data[3]];
    if !frame(&mut renderer, &sim, flip, true) {
        return;
    }
    let commands = commands(data, budget);
    let last = commands.len().saturating_sub(1);
    for (t, c) in commands.into_iter().enumerate() {
        let mut tic = [TicCommand::default(); 4];
        tic[0] = c;
        if sim.advance(tic).is_err() {
            return;
        }
        let before = renderer.state().cloned();
        if renderer.observe(&sim).is_err() {
            assert_eq!(
                renderer.state(),
                before.as_ref(),
                "tic {t}: a failed observe changed the state"
            );
            return;
        }
        if !frame(
            &mut renderer,
            &sim,
            [flip[0] ^ t as u8, flip[1], flip[2]],
            t % 4 == 3 || t == last,
        ) {
            return;
        }
    }
});
