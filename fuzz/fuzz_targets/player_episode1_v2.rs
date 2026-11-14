#![no_main]
//! Bounded public command streams under `PlayerMonstersEpisode1V2` (wire 24,
//! DSIM40) on the eighteen committed M4b synthetic mechanism WADs
//! (`tests/reference/m4b_wads/*.wad`, map E1M1, BOSS8 in the E1M8 slot; no IWAD):
//! teleporters, telefrags, monster and sliding teleports, the Baron of Hell and its
//! fireball, `A_BossDeath`, the ninth special line, the skill-3 THING filter and the
//! saw bite on a flagged body. Each input loads one WAD with
//! `Simulation::original_player_episode1_v2_map` (then `spawn_map_things`,
//! `initialize_player_weapons` with the default inventory and `spawn_level_specials`,
//! as the `player_level_run` example does under `--policy episode1v2`) and drives
//! `advance` with a byte-derived slot-One command stream.
//!
//! Input: a four-byte header, then four-byte command records. Byte 0 picks the WAD
//! (`data[0] % 18`, order of [`WADS`]; the map number is 8 for BOSS8 and 1 for the
//! others, as the committed scripts run them); byte 1 the skill and the start (bit 0:
//! skill 2 when clear, skill 3 when set; the other seven bits: 0 the default context
//! as the committed scripts run, any other value gameplay RNG index `data[1] >> 1`);
//! byte 2 the low byte of every command's `angleturn`; byte 3 the capture stride
//! (`16 << (data[3] % 4)` admitted tics between DSIM40 round trips). A record is a
//! version-110 demo ticcmd (forward, side, `angleturn` high byte, buttons), so the
//! command bytes of the committed `.lmp` scripts are records as they stand. Buttons
//! bits 0x40/0x80 are never sent: they are the record's repeat count (none 1, 0x40
//! 4, 0x80 16, both 64 tics). Forward and side -128 are sent as -127. The other
//! button bits are sent as they are (`BT_ATTACK`, `BT_USE`, `BT_CHANGE`, weapon
//! 0-7). At most 600 tics run per input; `-max_len=2404` (4 + 4 x 600).
//!
//! Properties: no panic; every refused tic leaves `snapshot().canonical_bytes()`
//! and the RNG of the state before it; a command inside the admitted command domain
//! (movement -50..50, weapon 0-3 with `BT_CHANGE`) is either executed or refused by
//! a declared witness, which ends the run (each later tic must repeat a sticky
//! `LevelExited` / `PlayerReborn`); a command outside it may also be refused by the
//! command preflight, the state kept, and the run continues; the DSIM40 capture
//! round-trips through `Diagnostic::decode_episode1_v2` every stride-th admitted tic
//! and at the end while `decode_episode1` rejects it with `Version(40)`, and a
//! mutated copy never panics; a second simulation loaded afresh and fed the same
//! bytes yields the same per-tic results and the same digest sequence.
use doom_sim::*;
use doom_types::{DoomRng, Fixed};
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;

/// The eighteen M4b synthetic mechanism WADs (`tests/reference/m4b_manifest.json`, `m4b_manifest2.json`).
const WADS: [&[u8]; 18] = [
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
const MAP_NUMBERS: [u8; 18] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 1, 1, 1, 1, 1, 1, 1, 1];
const HEADER: usize = 4;
const RECORD: usize = 4;
const TIC_BUDGET: usize = 600;
/// The command bound of the admitted domain at this step (`MAX_COMMAND_MOVE`).
const MAX_MOVE: i8 = 50;

fn maps() -> &'static [doom_map::Map] {
    static MAPS: OnceLock<Vec<doom_map::Map>> = OnceLock::new();
    MAPS.get_or_init(|| {
        WADS.iter()
            .map(|bytes| {
                let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
                doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default())
                    .or_else(|_| {
                        doom_map::Map::from_wad(&wad, "E1M8", doom_map::MapLimits::default())
                    })
                    .unwrap()
            })
            .collect()
    })
}

fn options(rng: u8) -> MapContextOptions {
    if rng == 0 {
        MapContextOptions::default()
    } else {
        MapContextOptions {
            initial_rng: DoomRng::from_indices(rng, 0),
            respawn_monsters: false,
        }
    }
}

/// The level start of WAD `index` at the skill and start `byte` selects. Every
/// committed WAD is admitted under wire 24 at both skills (the soak test and the
/// golden tests load each), so a refusal here is a finding, not an input to skip.
fn start(index: usize, byte: u8) -> Simulation {
    let skill = if byte & 1 == 0 {
        Skill::Medium
    } else {
        Skill::Hard
    };
    let map = maps()[index].clone();
    let mut s = Simulation::original_player_episode1_v2_map(
        map,
        MAP_NUMBERS[index],
        skill,
        options(byte >> 1),
    )
    .expect("a committed M4b WAD loads under wire 24");
    s.spawn_map_things().expect("its THINGs spawn");
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("the default inventory");
    s.spawn_level_specials().expect("its specials start");
    s
}

/// The DSIM40 capture of an admitted state: `decode_episode1_v2` round-trips it, the
/// DSIM39 reader rejects it with `Version(40)`, and a mutated copy never panics
/// (a mutated copy that decodes re-encodes to itself).
fn capture(s: &Simulation, flip: [u8; 3]) {
    let limits = DiagnosticLimits::default();
    let snapshot = s.snapshot();
    assert_eq!(snapshot.version(), 40);
    let mut bytes = snapshot.diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_episode1_v2(&bytes, limits).unwrap();
    assert_eq!(view.encode(limits).unwrap(), bytes);
    assert_eq!(
        Diagnostic::decode_episode1(&bytes, limits).err(),
        Some(DiagnosticError::Version(40))
    );
    let at = usize::from(u16::from_le_bytes([flip[0], flip[1]])) % bytes.len();
    bytes[at] ^= flip[2] | 1;
    if let Ok(v) = Diagnostic::decode_episode1_v2(&bytes, limits) {
        assert_eq!(v.encode(limits).unwrap(), bytes);
    }
}

/// A command the step's command preflight admits: movement inside ±50, weapon
/// bits only with `BT_CHANGE` and then a weapon 0–3.
fn admitted(c: &TicCommand) -> bool {
    let weapon = (c.buttons & BT_WEAPONMASK) >> 3;
    (-MAX_MOVE..=MAX_MOVE).contains(&c.forward)
        && (-MAX_MOVE..=MAX_MOVE).contains(&c.side)
        && if c.buttons & BT_CHANGE != 0 {
            weapon <= 3
        } else {
            weapon == 0
        }
}

/// The preflight refusals of a command outside this step's admitted domain but
/// inside the wire-23 domain (until the command-domain lift of plan step 7).
fn domain_refusal(e: &SimError) -> bool {
    matches!(
        e,
        SimError::PlayerBody(PlayerBodyError::Unsupported)
            | SimError::Weapon(WeaponError::UnsupportedWeapon)
            | SimError::UnsupportedButtons
    )
}

/// The witnesses an admitted tic may be refused with, state preserved, ending the
/// run: the Episode 1 mover witnesses (H7, H8, D1) and the per-tic mover family,
/// checked arithmetic (H3), the store-full witness (H4), the barrels/level
/// policy's witnesses (the 129th intercept, the corner jump, `SpecialCross`, the
/// vertical witness, the per-explosion sight budget, the unresolved dead-player
/// attacker) and the exit and reborn tics. The Baron fireball tunnel
/// ([`bruiser_tunnel`]) is declared separately: it needs the kept state.
fn witness(e: &SimError) -> bool {
    matches!(
        e,
        SimError::MapScale(
            MapScaleError::PlatBlocked
                | MapScaleError::StairBlocked
                | MapScaleError::SecondDoor
                | MapScaleError::DonutBusy
                | MapScaleError::MoverFamily
                | MapScaleError::Arithmetic
                | MapScaleError::Escape
                | MapScaleError::Intercepts
                | MapScaleError::SpecialCross
                | MapScaleError::Vertical
        ) | SimError::Store(StoreError::Capacity)
            | SimError::Perception(PerceptionError::WorkLimit)
            | SimError::PlayerBody(PlayerBodyError::Invariant)
            | SimError::LevelExited
            | SimError::PlayerReborn
    )
}

/// The closed vertex rectangle (minimum and maximum x and y over all VERTEXES, in
/// fixed point): the room every actor centre is checked against under wire 24.
type Room = [[i32; 2]; 2];

fn room_of(m: &doom_map::Map) -> Room {
    let v = m.vertices();
    let fixed = |f: fn(&doom_map::Vertex) -> i16| -> [i32; 2] {
        let (lo, hi) = (v.iter().map(f).min(), v.iter().map(f).max());
        [lo.unwrap(), hi.unwrap()].map(|u| Fixed::from_map_units(u).bits())
    };
    let (x, y) = (fixed(|v| v.x), fixed(|v| v.y));
    [[x[0], y[0]], [x[1], y[1]]]
}

/// The one declared case of `Monster(Arithmetic)` (the same predicate as the wire-24
/// soak's): the state the refused tic kept holds at least one actor centre outside
/// the vertex rectangle, and every such actor is a Baron fireball (`MT_BRUISERSHOT`,
/// speed 15, radius 6: info.c:1524–1548) that is still a flying missile
/// (`MF_MISSILE`, cleared by `P_ExplodeMissile`, p_mobj.c:101), whose momentum is at
/// most 15 per axis (one unsplit `P_XYMovement` step: p_mobj.c:150–164), whose
/// centre one momentum step back lay inside the rectangle, and whose box now lies
/// wholly beyond a rectangle edge: the genuine engine's wall tunnel (`PIT_CheckLine`
/// ignores a line whose bbox the destination box does not strictly overlap,
/// p_map.c:191–195; `P_CheckPosition` tests only the destination, p_map.c:374–442).
/// Rust admits the tunnelling tic, as C does, and the next tic's preflight position
/// check refuses with the state kept.
fn bruiser_tunnel(s: &Simulation, room: Room) -> bool {
    let ball = ActorTypeId(identity::OriginalActorType::MT_BRUISERSHOT.ordinal());
    let inside = |p: [i32; 2]| (0..2).all(|i| room[0][i] <= p[i] && p[i] <= room[1][i]);
    let mut tunnels = 0;
    for id in s.actor_order() {
        let a = s.actor(id).unwrap();
        let p = a.position().map(Fixed::bits);
        if inside(p) {
            continue;
        }
        let m = a.momentum().map(Fixed::bits);
        let r = a.radius().bits();
        let flying = a
            .life()
            .is_some_and(|l| l.actor_type == ball && l.flags & 0x1_0000 != 0);
        let step = m.iter().all(|v| v.unsigned_abs() <= 15 << 16);
        let before = [0, 1].map(|i| p[i].checked_sub(m[i]));
        let came_from_inside = before[0].zip(before[1]).is_some_and(|b| inside([b.0, b.1]));
        let clear = (0..2).any(|i| {
            p[i].checked_add(r).is_some_and(|e| e <= room[0][i])
                || p[i].checked_sub(r).is_some_and(|e| e >= room[1][i])
        });
        if !(flying && step && came_from_inside && clear) {
            return false;
        }
        tunnels += 1;
    }
    tunnels > 0
}

/// A declared witness of a tic refused from the kept state `s`.
fn declared(e: &SimError, s: &Simulation, room: Room) -> bool {
    witness(e) || *e == SimError::Monster(MonsterError::Arithmetic) && bruiser_tunnel(s, room)
}

/// The slot-One commands of an input, expanded and bounded by [`TIC_BUDGET`].
fn commands(data: &[u8]) -> Vec<TicCommand> {
    let low = data[2];
    let mut out = Vec::new();
    for r in data[HEADER..].as_chunks::<RECORD>().0 {
        let repeat = [1, 4, 16, 64][usize::from(r[3] >> 6)];
        let c = TicCommand {
            forward: i8::from_le_bytes([r[0]]).max(-127),
            side: i8::from_le_bytes([r[1]]).max(-127),
            angle_turn: i16::from_le_bytes([low, r[2]]),
            buttons: r[3] & 0x3f,
        };
        for _ in 0..repeat {
            if out.len() == TIC_BUDGET {
                return out;
            }
            out.push(c);
        }
    }
    out
}

/// One tic's observable outcome: its result and, at a check point, the digest.
type Trace = Vec<(Result<(), SimError>, Option<u64>)>;

/// Drives `s` with `commands`. With `checks`, asserts the properties of the
/// module documentation; the digest is recorded at every stride-th admitted tic,
/// at every refusal and at the end, in both runs.
fn run(
    s: &mut Simulation,
    room: Room,
    commands: &[TicCommand],
    stride: usize,
    checks: bool,
) -> Trace {
    let mut trace = Trace::new();
    let mut ended: Option<SimError> = None;
    let mut admitted_tics = 0;
    for (t, &c) in commands.iter().enumerate() {
        let before = checks.then(|| s.clone());
        let dead =
            checks && s.original_player(PlayerSlot::One).unwrap().state != PlayerBodyState::Live;
        let mut tic = [TicCommand::default(); 4];
        tic[0] = c;
        let result = s.advance(tic);
        if let Some(before) = &before {
            if let Err(e) = result {
                // Preflight before mutation: the refused tic keeps every byte.
                assert_eq!(
                    s.snapshot().canonical_bytes(),
                    before.snapshot().canonical_bytes(),
                    "tic {t}: {e:?} changed the state"
                );
                assert_eq!(s.rng(), before.rng());
                assert_eq!(s.tic(), before.tic());
                if let Some(sticky) = ended {
                    assert_eq!(e, sticky, "tic {t}");
                } else if admitted(&c) {
                    assert!(
                        declared(&e, s, room),
                        "tic {t}: admitted {c:?} refused: {e:?}"
                    );
                } else {
                    assert!(
                        declared(&e, s, room) || domain_refusal(&e),
                        "tic {t}: {c:?} refused: {e:?}"
                    );
                }
            } else {
                assert_eq!(ended, None, "tic {t}: ran after the run ended");
                assert_eq!(s.tic(), before.tic() + 1);
            }
        }
        match result {
            Ok(()) => {
                admitted_tics += 1;
                let point = admitted_tics % stride == 0;
                if checks {
                    if s.events()
                        .iter()
                        .any(|e| matches!(e, Event::Mover(MoverEvent::Exit { .. })))
                    {
                        ended = Some(SimError::LevelExited);
                    }
                    // P_DeathThink reads BT_USE on an unpaused tic that starts dead.
                    if dead && c.buttons & BT_USE != 0 && ended.is_none() {
                        ended = Some(SimError::PlayerReborn);
                    }
                    if point {
                        capture(s, [c.forward as u8, c.side as u8, c.buttons]);
                    }
                }
                trace.push((result, point.then(|| s.snapshot().digest())));
            }
            Err(e) => {
                trace.push((result, Some(s.snapshot().digest())));
                if declared(&e, s, room) {
                    break;
                }
            }
        }
    }
    trace.push((Ok(()), Some(s.snapshot().digest())));
    trace
}

fuzz_target!(|data: &[u8]| {
    if data.len() < HEADER {
        return;
    }
    let index = usize::from(data[0]) % WADS.len();
    let mut s = start(index, data[1]);
    let stride = 16 << (data[3] % 4);
    let commands = commands(data);
    capture(&s, [data[1], data[2], data[3]]);
    let room = room_of(&maps()[index]);
    let trace = run(&mut s, room, &commands, stride, true);
    capture(&s, [data[0], data[3], 0x80]);
    // Determinism: a fresh load fed the same bytes repeats every result and digest.
    let mut again = start(index, data[1]);
    assert_eq!(run(&mut again, room, &commands, stride, false), trace);
    assert_eq!(again.snapshot(), s.snapshot());
});
