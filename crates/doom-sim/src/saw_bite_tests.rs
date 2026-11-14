//! A Demon bite on a body that carries `A_Saw`'s `MF_JUSTATTACKED` (M4b step 7, the
//! E1M5 `M5WALK3` refusal). Expected values come from the C source (cited) and from
//! the genuine engine's run of the SAWBITE script recorded in
//! `tests/reference/m4b_synthetic_results2.json` ("C-observed"); never from Rust
//! output. The tic-for-tic comparison of SAWBITE against the genuine goldens is
//! `tests/episode1v2_mechanism_goldens.rs`.
//!
//! C facts used (linuxdoom-1.10): `P_Ticker` runs `P_PlayerThink` before
//! `P_RunThinkers` (p_tick.c:150–152); `A_Saw` sets `MF_JUSTATTACKED` on the player
//! only after a hit (p_pspr.c:519–523, 542); the next `P_PlayerThink` clears it
//! (p_user.c:249–255). `A_SargAttack` (p_enemy.c:935–948), `A_FaceTarget` (782–796),
//! `P_CheckMeleeRange` (174–192) and `P_DamageMobj` (p_inter.c:775–918) never read
//! `MF_JUSTATTACKED`, so the bite proceeds as on any live body.
//!
//! C-observed in SAWBITE (dump tic N is DSIM tic N + 1): A_Saw hits at dump tics 45,
//! 49, …, 97; the east Demon bites at dump tic 73 (20 health, the body flagged,
//! health 80 after it) and 97 (4 health, flagged, 76 after it).
use crate::ActorId;
use crate::diagonal_room::Lumps;
use crate::identity::{OriginalActorType, OriginalState};
use crate::original_player::{MF_SHADOW, MF_SKULLFLY};
use crate::player_commands::MF_JUSTATTACKED;
use crate::{
    MapContextOptions, OriginalExecutionProfile as P, OriginalRegistryError, PlayerBodyError,
    PlayerSlot, SimError, Simulation, Skill, StateId, TicCommand, WeaponInventory,
    demo::parse_demo_episode1_v2,
};

const V1: P = P::PlayerMonstersEpisode1V1;
const V2: P = P::PlayerMonstersEpisode1V2;
/// DSIM tics of the two bites on the flagged body (C dump tics 73 and 97).
const FIRST_BITE: usize = 74;
const SECOND_BITE: usize = 98;

fn reference(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name);
    std::fs::read(path).expect("reference file")
}
fn map() -> doom_map::Map {
    // The generator writes IWAD-headed files; the lump editor reads the same layout.
    let mut bytes = reference("m4b_wads/sawbite.wad");
    bytes[0] = b'P';
    let wad_bytes = Lumps::parse(&bytes).wad();
    let wad = doom_wad::Wad::parse(&wad_bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).unwrap()
}
fn setup(policy: P) -> Simulation {
    let loaded = if policy == V2 {
        Simulation::original_player_episode1_v2_map(
            map(),
            1,
            Skill::Medium,
            MapContextOptions::default(),
        )
    } else {
        Simulation::original_player_episode1_map(map(), MapContextOptions::default())
    };
    let mut s = loaded.unwrap_or_else(|e: OriginalRegistryError| panic!("{e:?}"));
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn demo() -> Vec<TicCommand> {
    parse_demo_episode1_v2(&reference("m4b_scripts/SAWBITE.lmp"), 1)
        .unwrap()
        .1
        .commands
}
fn step(s: &mut Simulation, command: TicCommand) -> Result<(), SimError> {
    s.advance([
        command,
        TicCommand::default(),
        TicCommand::default(),
        TicCommand::default(),
    ])
}
/// Runs DSIM tics 1..=tics.
fn run_to(s: &mut Simulation, commands: &[TicCommand], tics: usize) {
    for (i, c) in commands[..tics].iter().enumerate() {
        step(s, *c).unwrap_or_else(|e| panic!("tic {}: {e:?}", i + 1));
    }
}
fn body(s: &Simulation) -> (ActorId, crate::Actor) {
    let id = s.original_player.unwrap().mo.unwrap();
    (id, *s.actor(id).unwrap())
}
/// The east Demon (the second Demon THING): the one that bites.
fn biter(s: &Simulation) -> ActorId {
    let demons: Vec<ActorId> = s
        .actors
        .order()
        .filter(|id| s.actors.is_pending(*id) == Some(false))
        .filter(|id| {
            s.actor(*id)
                .and_then(|a| a.life)
                .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_SERGEANT.ordinal())
        })
        .collect();
    assert_eq!(demons.len(), 2);
    demons
        .into_iter()
        .max_by_key(|id| s.actor(*id).unwrap().position()[0])
        .unwrap()
}

/// Wire 24 runs both C-observed bites on the flagged body: after each the body still
/// carries `MF_JUSTATTACKED` (nothing on the bite path clears it), the health is the
/// C value, and the biter has just entered `S_SARG_ATK3`.
#[test]
fn wire_24_admits_the_bite_on_a_body_a_saw_flagged_in_the_same_tic() {
    let commands = demo();
    let mut s = setup(V2);
    run_to(&mut s, &commands, FIRST_BITE - 1);
    let (_, before) = body(&s);
    assert_eq!(before.life.unwrap().flags & MF_JUSTATTACKED, 0);
    assert_eq!(before.life.unwrap().health, 100);
    step(&mut s, commands[FIRST_BITE - 1]).unwrap();
    let (id, after) = body(&s);
    let life = after.life.unwrap();
    assert_ne!(life.flags & MF_JUSTATTACKED, 0);
    assert_eq!(life.health, 80);
    assert_eq!(s.original_player.unwrap().health, 80);
    let demon = s.actor(biter(&s)).unwrap().life.unwrap();
    assert_eq!(
        demon.state,
        Some(StateId(OriginalState::S_SARG_ATK3.ordinal()))
    );
    assert_eq!(demon.tics, 8);
    assert_eq!(demon.target, Some(id));
    run_to(&mut s, &commands[FIRST_BITE..], SECOND_BITE - FIRST_BITE);
    let life = body(&s).1.life.unwrap();
    assert_ne!(life.flags & MF_JUSTATTACKED, 0);
    assert_eq!(life.health, 76);
}

/// Every older policy keeps its admitted surface: wire 23 runs SAWBITE as wire 24 does
/// up to the first bite and refuses that tic with `PlayerBodyError::Invariant`,
/// leaving the complete canonical state unchanged.
#[test]
fn wire_23_still_refuses_the_bite_with_the_invariant() {
    let commands = demo();
    let mut s = setup(V1);
    run_to(&mut s, &commands, FIRST_BITE - 1);
    let snapshot = s.snapshot();
    let player = s.original_player;
    assert_eq!(
        step(&mut s, commands[FIRST_BITE - 1]),
        Err(PlayerBodyError::Invariant.into())
    );
    assert_eq!(s.snapshot(), snapshot);
    assert_eq!(s.original_player, player);
}

/// Only `MF_JUSTATTACKED` is added under wire 24: the same bite, entered directly in
/// a checked operation on the body as it stood before DSIM tic 74, lands with that
/// flag alone and is refused with `PlayerBodyError::Invariant` when any other
/// unadmitted flag is set too (`MF_SHADOW`, which `A_FaceTarget` would read,
/// p_enemy.c:794–795; `MF_SKULLFLY`, which `P_DamageMobj` reads, p_inter.c:793), with
/// the state unchanged.
#[test]
fn wire_24_still_refuses_any_other_unadmitted_flag_on_the_bitten_body() {
    let commands = demo();
    for extra in [0, MF_SHADOW, MF_SKULLFLY, MF_SHADOW | MF_SKULLFLY] {
        let mut s = setup(V2);
        run_to(&mut s, &commands, FIRST_BITE - 1);
        let (id, _) = body(&s);
        let demon = biter(&s);
        let snapshot = s.snapshot();
        let result = s.checked_demon_operation(|host| {
            host.access
                .get_mut(id)
                .unwrap()
                .life
                .as_mut()
                .unwrap()
                .flags |= MF_JUSTATTACKED | extra;
            host.enter(demon, StateId(OriginalState::S_SARG_ATK3.ordinal()));
            host.player.as_deref().unwrap().health
        });
        if extra == 0 {
            let health = result.unwrap();
            assert!(health < 100, "the bite lands: {health}");
            assert_eq!(body(&s).1.life.unwrap().health, health);
        } else {
            assert_eq!(result, Err(PlayerBodyError::Invariant.into()), "{extra:#x}");
            assert_eq!(s.snapshot(), snapshot, "{extra:#x}");
        }
    }
}

/// Pins the disjointness `strike_player`'s `allowed` mask relies on: MT_PLAYER's own
/// flags (info.c:1130: `MF_SOLID|MF_SHOOTABLE|MF_DROPOFF|MF_PICKUP|MF_NOTDMATCH`),
/// `MF_JUSTHIT` (p_mobj.h:133, 64), `MF_JUSTATTACKED` (135, 128) and `MF_CORPSE`
/// (173, 0x100000) share no bit, so OR and XOR of them agree. The expected bit
/// values are the header's, written out.
#[test]
fn the_allowed_flag_terms_are_disjoint() {
    let player = OriginalActorType::MT_PLAYER.descriptor().flags;
    assert_eq!(player, 0x2 | 0x4 | 0x400 | 0x800 | 0x200_0000);
    let terms = [
        player,
        crate::original_player::MF_JUSTHIT,
        MF_JUSTATTACKED,
        crate::original_player::MF_CORPSE,
    ];
    assert_eq!(terms[1..], [0x40, 0x80, 0x10_0000]);
    for (i, a) in terms.iter().enumerate() {
        for b in &terms[i + 1..] {
            assert_eq!(a & b, 0);
        }
    }
    assert_eq!(
        terms.iter().fold(0, |m, t| m | t),
        terms.iter().fold(0, |m, t| m ^ t)
    );
}
