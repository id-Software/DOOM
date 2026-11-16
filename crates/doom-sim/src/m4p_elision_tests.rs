//! M4p decision P8 (docs/M4P_REFERENCE_INVESTIGATION.md §4 P8): Rust-only tests, one per
//! elision of O1, O2, O3 and O4 and per check named for O7, each failing if the remaining check named in the "as implemented"
//! proofs of §3 is removed. They restate behaviour that existed before the elisions and need no
//! new C citation; the existing goldens remain the equivalence evidence. Each refusal leaves
//! the canonical state bytes, the RNG and the tic exactly as before. Module `o6` is not an
//! elision test: it compares O6's single-pass `for_things` with the per-cell scan it replaced.
use crate::*;
#[allow(clippy::duplicate_mod)]
#[path = "../../../tests/support/m3f3.rs"]
mod fixtures;

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

// ---------------------------------------------------------------------------------------
// O2 (note §3 "O2 as implemented"): the sound-target and owner-target reference checks.
// ---------------------------------------------------------------------------------------

/// `demon_map` with a good id (the owners' Demon target) and a bad one (an effect).
fn map_ids() -> (Simulation, ActorId, ActorId) {
    let mut s = demon_map();
    let good = s.actor(OWNER).unwrap().life().unwrap().target.unwrap();
    let bad = effect(&mut s);
    assert!(s.validate_perception_reference(good).is_ok());
    assert!(s.validate_perception_reference(bad).is_err());
    (s, good, bad)
}

/// Pins the kept `validate_perception_reference` call of each run of equal sound targets in
/// `preflight_map_demons` (O2 (a), last-accepted-id skip). A below-API guard: the public API
/// refuses a bad sound target at write time, so the table is written from inside the crate.
/// Cases: a different bad id after an accepted run, a bad id first, a bad id repeated (it must
/// fail at its first occurrence and never count as accepted), and a bad id between good runs.
/// Each must refuse with the error the function gives for the bad id, on the same tic, with
/// canonical bytes, RNG and tic unchanged.
#[test]
fn sound_target_loop_refuses_a_bad_id_after_and_inside_runs_of_equal_ids() {
    let (s, good, bad) = map_ids();
    let expected = s.validate_perception_reference(bad).unwrap_err();
    // The map has one sector; `preflight_map_demons` walks whatever the table holds, so the
    // table is widened (below the API) to give a loop of several entries.
    let mut s = s;
    s.sound_targets.resize(4, None);
    // Control: the all-good table passes the pass.
    let mut ok = s.clone();
    ok.sound_targets.iter_mut().for_each(|t| *t = Some(good));
    assert_eq!(ok.preflight_map_demons(), Ok(()));
    let (g, b) = (Some(good), Some(bad));
    for table in [
        [g, g, b, None],
        [b, g, None, None],
        [b, b, g, None],
        [g, b, b, g],
        [g, b, g, g],
        [g, g, g, b],
    ] {
        let mut s = s.clone();
        s.sound_targets.iter_mut().for_each(|t| *t = None);
        s.sound_targets[..4].copy_from_slice(&table);
        assert_eq!(s.preflight_map_demons(), Err(expected));
        refuses_with(&mut s, expected);
        assert_eq!(s.tic(), 0);
    }
}

/// Pins O2 (b), the first live owner's walk of the sound targets in `preflight_monster_owner`
/// (`monster_callback_references(a, true)`: the walk-once flag starts unset). In `DemonLifecycle`
/// `preflight_map_demons` is a no-op, so this walk is the only check of a sound target there;
/// the direct call of the owner pass proves it is the pass that refuses. Below the API: the
/// public `step` refuses without saying which pass did, and the corrupt table is injected.
#[test]
fn first_live_owner_still_walks_the_sound_targets() {
    let mut s = lifecycle();
    assert_eq!(s.preflight_map_demons(), Ok(()));
    assert_eq!(s.preflight_monster_owner().map(|_| ()), Ok(()));
    let e = effect(&mut s);
    let expected = s.monster_reference(e).unwrap_err();
    s.sound_targets[0] = Some(e);
    assert_eq!(s.preflight_map_demons(), Ok(()));
    assert_eq!(s.preflight_monster_owner().map(|_| ()), Err(expected));
    refuses_with(&mut s, expected);
    assert_eq!(s.tic(), 0);
}

/// O2 (c), the target check of every later owner (`monster_callback_references(a, false)`), and
/// the nearest reachable boundary. Why (c) has no reachable witness: a second live owner exists
/// only under a `uses_demon_map` profile (`monster_actor` and `spawn_type` refuse a second owner
/// elsewhere: `monsters.rs` "actors.order().next() != Some(id)" and the `monster_is_owner_type`
/// spawn test), and there `preflight_map_demons`, which runs first in `preflight_monster_owner`
/// on the same `&self`, already calls `validate_perception_reference` on the target of every
/// live roster monster, and that function ends in `monster_reference` (for these policies,
/// `perception.rs` `validate_perception_reference`), so it refuses every target that the
/// owner loop's own check would refuse, earlier and at the same tic. So the test pins the
/// reachable boundary: a second owner whose target (distinct from the first owner's) is bad
/// refuses on the same tic, through `preflight_map_demons`, and a second owner with a good
/// distinct target passes. The injection is below the API (`set_actor_target` refuses it).
#[test]
fn later_map_owner_with_a_bad_distinct_target_refuses_before_the_owner_loop() {
    let (mut s, good, bad) = map_ids();
    let second = ActorId::from_parts(1, 1);
    assert!(s.monster_is_owner(OWNER) && s.monster_is_owner(second));
    // Another good target, distinct from the first owner's.
    let other = s
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(32), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert!(s.validate_perception_reference(other).is_ok());
    set_target(&mut s, OWNER, Some(good));
    set_target(&mut s, second, Some(other));
    assert_eq!(s.preflight_monster_owner().map(|_| ()), Ok(()));
    let expected = s.validate_perception_reference(bad).unwrap_err();
    set_target(&mut s, second, Some(bad));
    assert_eq!(s.preflight_map_demons(), Err(expected));
    assert_eq!(s.preflight_monster_owner().map(|_| ()), Err(expected));
    refuses_with(&mut s, expected);
    assert_eq!(s.tic(), 0);
}

fn set_target(s: &mut Simulation, owner: ActorId, t: Option<ActorId>) {
    s.actors
        .get_mut(owner)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .target = t;
}

// ---------------------------------------------------------------------------------------
// O7 (note §3 "O7 as implemented"): the slot bitmap of `preflight_expanded_demons_for`.
// ---------------------------------------------------------------------------------------

mod expanded {
    use super::fixtures;
    use super::*;

    fn setup() -> Simulation {
        let s = fixtures::setup(&fixtures::cases()[0]);
        // Control: the uncorrupted fixture advances, so a refusal below comes from the corruption.
        let mut c = s.clone();
        assert!(c.step([TicCommand::default(); 4]).is_ok());
        assert!(s.spatial.order.len() >= 2);
        s
    }

    /// Pins the duplicate test of the first loop (`linked[word] & bit != 0`): an id twice in
    /// `spatial.order` must refuse with `MapSpawnError::Ownership` (the `BTreeSet::insert`
    /// returning false before O7) from `step` and `advance` on the same tic (the preflight is
    /// private to its module, so the public step is the witness).
    /// Every id stays live and in range, so no other clause refuses; below the API because
    /// the spatial order is only ever built without repeats. Checked at the start, middle and
    /// end of the order and for a distinct slot pair that must not count as a duplicate.
    #[test]
    fn an_id_twice_in_the_spatial_order_is_refused() {
        let expected = SimError::MapSpawn(MapSpawnError::Ownership);
        let n = setup().spatial.order.len();
        for (at, dup) in [(0, 0), (0, n - 1), (n - 1, 0), (n - 1, n - 1)] {
            let mut s = setup();
            let id = s.spatial.order[dup];
            s.spatial.order.insert(at, id);
            refuses_with(&mut s, expected);
            assert_eq!(s.tic(), 0);
        }
        // Control: distinct slots are not duplicates (the bitmap keys by slot, not by position).
        let mut s = setup();
        s.spatial.order.reverse();
        assert!(s.step([TicCommand::default(); 4]).is_ok());
    }

    /// Pins the second loop's membership test (`linked[word] & bit == 0`): a live actor of
    /// `actors.order()` that is absent from `spatial.order` must refuse with
    /// `MapSpawnError::Ownership`, whichever entry is removed. Below the API: removal from the
    /// spatial order is only done together with the actor.
    #[test]
    fn a_live_actor_missing_from_the_spatial_order_is_refused() {
        let expected = SimError::MapSpawn(MapSpawnError::Ownership);
        for at in 0..setup().spatial.order.len() {
            let mut s = setup();
            s.spatial.order.remove(at);
            refuses_with(&mut s, expected);
            assert_eq!(s.tic(), 0);
        }
    }
}

/// O6 (docs/M4P_REFERENCE_INVESTIGATION.md §3 O6): the single-pass `for_things` against
/// the per-cell scan it replaced, kept here verbatim as the oracle. Crowded synthetic
/// layouts with ids out of slot order, ids repeated in the spatial order, actors outside
/// the blockmap and `MF_NOBLOCKMAP` actors; every box clamp; every early exit; the mover's
/// own `actor` substituted for its stored copy; and the missing-handle panic.
mod o6 {
    use super::*;
    use crate::movement::{SpatialIndex, cell, for_things};

    /// The pre-O6 body of `movement::for_things` (master `4021d60`, movement.rs:254–277).
    fn per_cell_scan(
        world: &StaticWorld,
        id: ActorId,
        actor: &Actor,
        target: [Fixed; 2],
        spatial: &SpatialIndex,
        get: &impl Fn(ActorId) -> Option<Actor>,
        mut visit: impl FnMut(ActorId, Actor) -> bool,
    ) {
        let margin = actor.spec.radius + Fixed::from_map_units(32);
        let [xl, yl] = cell(world, [target[0] - margin, target[1] - margin]);
        let [xh, yh] = cell(world, [target[0] + margin, target[1] + margin]);
        let [w, h] = world.map().blockmap().dimensions().map(i32::from);
        for x in xl.max(0)..=xh.min(w - 1) {
            for y in yl.max(0)..=yh.min(h - 1) {
                for candidate in &spatial.order {
                    let other = if *candidate == id {
                        *actor
                    } else {
                        get(*candidate).expect("valid spatial handle")
                    };
                    if !crate::lifecycle::blocks_cell_index(&other)
                        || cell(world, other.spec.position) != [x, y]
                    {
                        continue;
                    }
                    if !visit(*candidate, other) {
                        return;
                    }
                }
            }
        }
    }

    /// A small deterministic generator (test-only; not the game RNG).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self, n: u64) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) % n
        }
    }

    type Visits = Vec<(ActorId, Actor)>;

    /// Runs one implementation, stopping after `stop` visits, and returns its visits and
    /// whether it panicked.
    fn run(
        new: bool,
        s: &Simulation,
        id: ActorId,
        actor: &Actor,
        target: [Fixed; 2],
        stop: usize,
    ) -> (Visits, bool) {
        let mut visits = Vec::new();
        let get = |i| s.actors.get(i).copied();
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let visit = |candidate, other| {
                visits.push((candidate, other));
                visits.len() < stop
            };
            if new {
                for_things(&s.world, id, actor, target, &s.spatial, &get, visit);
            } else {
                per_cell_scan(&s.world, id, actor, target, &s.spatial, &get, visit);
            }
        }))
        .is_err();
        (visits, panicked)
    }

    /// A crowd of copies of one spawned actor over and around the blockmap.
    fn crowd(seed: u64, count: usize) -> (Simulation, Vec<ActorId>) {
        // The `lifecycle` fixture's map, actor and 64-slot store.
        let mut s = lifecycle();
        let template = *s.actors.get(OWNER).unwrap();
        let blockmap = s.world.map().blockmap();
        let origin = blockmap.origin();
        let [w, h] = blockmap.dimensions().map(i32::from);
        assert!(w >= 2 && h >= 2, "the layout needs several cells");
        let mut rng = Lcg(seed);
        let mut ids = s.spatial.order.clone();
        for _ in 0..count {
            let mut a = template;
            // From one cell before the blockmap to one cell past it, in map units.
            let x = i32::from(origin.x) - 128 + rng.next((w as u64 + 2) * 128) as i32;
            let y = i32::from(origin.y) - 128 + rng.next((h as u64 + 2) * 128) as i32;
            a.spec.position = [x, y].map(|v| Fixed::from_bits(v << 16));
            if rng.next(6) == 0 {
                a.life.as_mut().unwrap().flags |= 16; // MF_NOBLOCKMAP
            }
            ids.push(s.actors.spawn(a).unwrap());
        }
        // Spatial order unrelated to slot order, with some ids repeated.
        for i in (1..ids.len()).rev() {
            ids.swap(i, rng.next(i as u64 + 1) as usize);
        }
        for _ in 0..count / 8 {
            let i = rng.next(ids.len() as u64) as usize;
            ids.insert(rng.next(ids.len() as u64 + 1) as usize, ids[i]);
        }
        s.spatial.order = ids.clone();
        (s, ids)
    }

    fn compare(s: &Simulation, ids: &[ActorId], rng: &mut Lcg, rounds: usize) -> usize {
        let blockmap = s.world.map().blockmap();
        let origin = blockmap.origin();
        let [w, h] = blockmap.dimensions().map(i32::from);
        let mut visited = 0;
        for _ in 0..rounds {
            let id = ids[rng.next(ids.len() as u64) as usize];
            let mut actor = *s.actors.get(id).unwrap();
            let x = i32::from(origin.x) - 256 + rng.next((w as u64 + 4) * 128) as i32;
            let y = i32::from(origin.y) - 256 + rng.next((h as u64 + 4) * 128) as i32;
            let mut target = [x, y].map(|v| Fixed::from_bits(v << 16));
            if rng.next(2) == 0 {
                // Into the crowd: another listed actor's position.
                let other = ids[rng.next(ids.len() as u64) as usize];
                target = s.actors.get(other).unwrap().spec.position;
            }
            if rng.next(2) == 0 {
                // The mover's own copy differs from the stored one.
                actor.spec.position = target;
            }
            let full = run(false, s, id, &actor, target, usize::MAX);
            assert_eq!(run(true, s, id, &actor, target, usize::MAX), full);
            for stop in 1..=full.0.len() {
                assert_eq!(
                    run(true, s, id, &actor, target, stop),
                    run(false, s, id, &actor, target, stop)
                );
            }
            visited += full.0.len();
        }
        visited
    }

    #[test]
    fn single_pass_reproduces_the_per_cell_visit_sequence() {
        let mut total = 0;
        for seed in 1..=6 {
            let (s, ids) = crowd(seed, [12, 24, 36, 48, 60, 62][seed as usize - 1]);
            total += compare(&s, &ids, &mut Lcg(seed ^ 0x5eed), 200);
        }
        assert!(total > 1000, "layouts too sparse: {total} visits");
    }

    #[test]
    fn a_missing_handle_panics_after_the_same_first_cell_visits() {
        let mut checked = 0;
        for seed in 1..=4 {
            let (mut s, ids) = crowd(seed, 60);
            let mut rng = Lcg(seed ^ 0xdead);
            // A stale handle (a later generation of a used slot) at a random position.
            let stale = ActorId::from_parts(ids[0].slot(), 99);
            let at = rng.next(s.spatial.order.len() as u64 + 1) as usize;
            s.spatial.order.insert(at, stale);
            let ids: Vec<_> = ids.into_iter().filter(|i| *i != stale).collect();
            for _ in 0..40 {
                let id = ids[rng.next(ids.len() as u64) as usize];
                let actor = *s.actors.get(id).unwrap();
                let target = actor.spec.position;
                let full = run(false, &s, id, &actor, target, usize::MAX);
                assert_eq!(run(true, &s, id, &actor, target, usize::MAX), full);
                checked += usize::from(full.1);
                for stop in 1..=full.0.len() {
                    assert_eq!(
                        run(true, &s, id, &actor, target, stop),
                        run(false, &s, id, &actor, target, stop)
                    );
                }
            }
            // The scratch is usable again after a panic inside the walk.
            s.spatial.order.retain(|i| *i != stale);
            compare(&s, &ids, &mut rng, 4);
        }
        assert!(checked > 0, "no walk reached the missing handle");
    }
}
