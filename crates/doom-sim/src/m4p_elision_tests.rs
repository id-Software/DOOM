//! M4p decision P8 (docs/M4P_REFERENCE_INVESTIGATION.md §4 P8): Rust-only tests, one per
//! elision of O1, O3 and O4, each failing if the remaining check named in the "as implemented"
//! proofs of §3 is removed. They restate behaviour that existed before the elisions and need no
//! new C citation; the existing goldens remain the equivalence evidence. Each refusal leaves
//! the canonical state bytes, the RNG and the tic exactly as before.
use crate::*;

/// Canonical bytes, RNG and tic: the whole observable state a refused tic must preserve.
fn witness(s: &Simulation) -> (Vec<u8>, doom_types::DoomRng, u64) {
    (s.snapshot().canonical_bytes(), s.rng(), s.tic())
}

/// A refused `step` and `advance`: exact error, nothing changed, and the same error again
/// on the next tic (the refusal tic does not move).
fn refuses_with(s: &mut Simulation, expected: SimError) {
    let before = witness(s);
    assert_eq!(s.step([TicCommand::default(); 4]).err(), Some(expected));
    assert_eq!(witness(s), before);
    assert_eq!(s.advance([TicCommand::default(); 4]).err(), Some(expected));
    assert_eq!(witness(s), before);
}

fn map(path: &[u8]) -> doom_map::Map {
    let wad = doom_wad::Wad::parse(path, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}

/// DemonMapSpawn (uses_demon_map, no room) with two map owners and a Demon target actor.
fn demon_map() -> Simulation {
    let mut s = Simulation::with_demon_map_spawn_registry(
        map(include_bytes!("../../../tests/reference/m3d3i_map_1.bin")),
        64,
        ActorRegistry::demon_stand_run_fixture(),
        ActorOptions::default(),
    )
    .unwrap();
    s.spawn_map_things().unwrap();
    let target = s
        .spawn_type(ActorTypeId(135), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    for id in [ActorId::from_parts(0, 1), ActorId::from_parts(1, 1)] {
        s.set_actor_target(id, Some(target)).unwrap();
        s.enter_actor_state(id, StateId(5)).unwrap();
        s.actors.get_mut(id).unwrap().life.as_mut().unwrap().tics = 1;
    }
    s
}

/// DemonLifecycle (not uses_demon_map): `preflight_map_demons` is a no-op, so the owner loop's
/// own reference step is the only check of a target that `monster_reference` refuses.
fn lifecycle() -> Simulation {
    let mut s = Simulation::with_demon_lifecycle_registry(
        map(include_bytes!("../../../tests/reference/m3d3g_map_0.bin")),
        64,
        ActorRegistry::demon_lifecycle_fixture(),
        ActorOptions::default(),
    )
    .unwrap();
    s.spawn_type(
        ActorTypeId(136),
        [Fixed::from_map_units(-64), Fixed::ZERO],
        SpawnHeight::Floor,
    )
    .unwrap();
    s.spawn_type(
        ActorTypeId(135),
        [Fixed::from_map_units(96), Fixed::ZERO],
        SpawnHeight::Floor,
    )
    .unwrap();
    s
}

const OWNER: ActorId = ActorId::from_parts(0, 1);

fn effect(s: &mut Simulation) -> ActorId {
    // Type 37 is an effect: a live owner may never hold it as a target (`monster_reference`).
    s.spawn_type(ActorTypeId(37), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap()
}

/// Pins the owner-loop reference step `monster_callback_references` (M4p note §3 O1 and O4,
/// P8): it is the only check of the owner's target under a policy where
/// `preflight_map_demons` is a no-op. A below-API guard: the public `set_actor_target`
/// refuses such a target at write time, so the corrupt record is injected into the field.
#[test]
fn owner_loop_refuses_a_live_owner_target_that_monster_reference_rejects() {
    let mut s = lifecycle();
    let e = effect(&mut s);
    s.actors
        .get_mut(OWNER)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .target = Some(e);
    assert_eq!(s.tic(), 0);
    refuses_with(&mut s, MonsterError::Target.into());
    assert_eq!(s.tic(), 0);
}

/// Same guard for a sector sound target (M4p note §3 O1 and O4, P8): the references step runs
/// the sound targets after the owner's target, so a bad sound target alone must still refuse.
#[test]
fn owner_loop_refuses_a_sector_sound_target_that_monster_reference_rejects() {
    let mut s = lifecycle();
    let e = effect(&mut s);
    s.sound_targets[0] = Some(e);
    refuses_with(&mut s, MonsterError::Target.into());
    assert_eq!(s.tic(), 0);
}

fn owner_at(units_bits: i32) -> Simulation {
    let mut s = demon_map();
    s.actors.get_mut(OWNER).unwrap().spec.position[0] = Fixed::from_bits(units_bits);
    s
}

/// Pins the residue `position_admitted(pos, 978)` of the owner loop under `uses_demon_map`
/// (M4p note §3 O4 proof item 3 and 4(a), P8). Under `DemonMapSpawn` there is no room, so
/// `preflight_map_demons` admits the owner at margin 1008 and only the residue refuses it at
/// 978: an owner at 1000 map units (between 978 and 1008) must refuse with
/// `MonsterError::Arithmetic` on the same tic. Not reachable by the public API (it builds
/// owners only inside the admitted bounds), so the position is injected into the actor.
#[test]
fn demon_map_owner_between_978_and_1008_refuses_with_arithmetic() {
    let mut s = owner_at(1000 * 65536);
    // The 1008 pass of `preflight_map_demons` alone admits this state.
    assert_eq!(s.preflight_map_demons(), Ok(()));
    refuses_with(&mut s, MonsterError::Arithmetic.into());
    assert_eq!(s.tic(), 0);
}

/// The 978 boundary itself (M4p note §3 O4, P8): exactly 978 units is admitted by the owner
/// loop, one raw unit beyond is refused with `MonsterError::Arithmetic`; the 1008 boundary of
/// `preflight_map_demons` likewise.
#[test]
fn demon_map_owner_position_boundaries_978_and_1008() {
    let mut s = owner_at(978 * 65536);
    assert_eq!(s.preflight_monster_owner().map(|_| ()), Ok(()));
    let before = witness(&s);
    assert!(s.step([TicCommand::default(); 4]).is_ok());
    assert_eq!(witness(&s).2, before.2 + 1);
    let mut s = owner_at(978 * 65536 + 1);
    assert_eq!(s.preflight_map_demons(), Ok(()));
    refuses_with(&mut s, MonsterError::Arithmetic.into());
    let mut s = owner_at(1008 * 65536 + 1);
    assert_eq!(
        s.preflight_map_demons(),
        Err(MonsterError::Arithmetic.into())
    );
    refuses_with(&mut s, MonsterError::Arithmetic.into());
}

/// Pins `preflight_world_pickups` and its `validate_item` call (`world_pickups.rs` loop,
/// reached from `demon_map_spawn.rs` `preflight_map_demons`), the load-bearing check once O3
/// removed the second `validate_item` of the demon-map pass (M4p note §3 O3 proof item 4, P8).
/// Under a `uses_demon_map` policy with world pickups, a pickup whose `health` alone is
/// corrupted must still refuse with `PlayerBodyError::Invariant` before any command, with state bytes, RNG and tic unchanged.
/// A below-API guard: the public API cannot write a pickup record, so the field is injected.
#[test]
fn demon_map_pickup_corruption_refuses_through_preflight_world_pickups() {
    use crate::pickup_evidence as source;
    let case = &source::cases()[1];
    let expected = SimError::from(PlayerBodyError::Invariant);
    // Control: the uncorrupted state advances, so the refusal below comes from the corruption.
    let (mut control, _) = source::setup(case);
    assert!(control.step(source::commands(case, 1)).is_ok());
    // `health` is the field the proof names; `validate_item` demands exactly 1000.
    for health in [999, 1001] {
        let (mut s, _) = source::setup(case);
        assert!(s.profile.uses_demon_map());
        assert!(s.player_policy().has_world_pickups());
        // The pickup farthest from every other actor: the player's touch path (`validate_item`
        // at the touch, only for items within reach) must not be what refuses it.
        let item = s
            .actor_order()
            .filter(|id| {
                s.actor(*id)
                    .and_then(Actor::life)
                    .is_some_and(|l| world_pickups::kind(l.actor_type).is_some())
            })
            .max_by_key(|id| {
                let p = s.actor(*id).unwrap().position();
                s.actor_order()
                    .filter(|o| o != id)
                    .map(|o| {
                        let q = s.actor(o).unwrap().position();
                        (i64::from(p[0].bits() - q[0].bits()).abs())
                            .max(i64::from(p[1].bits() - q[1].bits()).abs())
                    })
                    .min()
                    .unwrap_or(0)
            })
            .unwrap();
        let l = s.actors.get_mut(item).unwrap().life.as_mut().unwrap();
        l.health = health;
        let before = witness(&s);
        let commands = [TicCommand::default(); 4];
        assert_eq!(s.step(commands).err(), Some(expected));
        assert_eq!(witness(&s), before);
        assert_eq!(s.advance(commands).err(), Some(expected));
        assert_eq!(witness(&s), before);
        assert_eq!(s.tic(), 0);
    }
}
