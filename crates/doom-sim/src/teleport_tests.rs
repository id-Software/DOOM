//! Teleportation under wire 24 (M4b step 4). Expected values come from the C
//! source (cited), from the genuine engine's runs recorded in
//! docs/validation/M4B.md ("C-observed"), or from written derivations; never from
//! Rust output. The tic-for-tic comparison of the M4b scripts against the genuine
//! goldens is `tests/episode1v2_mechanism_goldens.rs`.
use super::*;
use crate::PursuitError;
use crate::diagonal_room::Lumps;
use crate::identity::OriginalState;
use crate::{
    MapContextOptions, OriginalExecutionProfile as P, OriginalRegistryError, PlayerSlot,
    Simulation, Skill, TicCommand, WeaponInventory, demo::parse_demo_episode1_v2,
};

const V1: P = P::PlayerMonstersEpisode1V1;
const V2: P = P::PlayerMonstersEpisode1V2;

fn reference(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name);
    std::fs::read(path).expect("reference file")
}
fn wad(name: &str) -> Lumps {
    // The generator writes IWAD-headed files; the lump editor reads the same layout.
    let mut bytes = reference(&format!("m4b_wads/{name}.wad"));
    bytes[0] = b'P';
    Lumps::parse(&bytes)
}
fn map_of(l: &Lumps) -> doom_map::Map {
    let bytes = l.wad();
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).unwrap()
}
fn lines(l: &mut Lumps) -> &mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == "LINEDEFS").unwrap().1
}
fn things(l: &mut Lumps) -> &mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == "THINGS").unwrap().1
}
fn load(l: &Lumps, policy: P) -> Result<Simulation, SimError> {
    let result = if policy == V2 {
        Simulation::original_player_episode1_v2_map(
            map_of(l),
            1,
            Skill::Medium,
            MapContextOptions::default(),
        )
    } else {
        Simulation::original_player_episode1_map(map_of(l), MapContextOptions::default())
    };
    result.map_err(|e| match e {
        OriginalRegistryError::Runtime(e) => e,
        other => panic!("{other:?}"),
    })
}
fn setup(l: &Lumps) -> Simulation {
    let mut s = load(l, V2).unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn demo(script: &str) -> Vec<TicCommand> {
    parse_demo_episode1_v2(&reference(&format!("m4b_scripts/{script}.lmp")), 1)
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
/// Runs `commands[..tics]` (DSIM tics 1..=tics).
fn run_to(s: &mut Simulation, commands: &[TicCommand], tics: usize) {
    for (i, c) in commands[..tics].iter().enumerate() {
        step(s, *c).unwrap_or_else(|e| panic!("tic {}: {e:?}", i + 1));
    }
}
fn body(s: &Simulation) -> crate::Actor {
    *s.actor(s.original_player.unwrap().mo.unwrap()).unwrap()
}
fn of_type(s: &Simulation, ty: OriginalActorType) -> Vec<(ActorId, crate::Actor)> {
    s.actors
        .order()
        .filter(|id| s.actors.is_pending(*id) == Some(false))
        .map(|id| (id, *s.actor(id).unwrap()))
        .filter(|(_, a)| a.life.is_some_and(|l| l.actor_type.0 == ty.ordinal()))
        .collect()
}
fn teleport_events(s: &Simulation) -> Vec<MoverEvent> {
    s.events()
        .iter()
        .filter_map(|e| match e {
            Event::Mover(
                m @ (MoverEvent::Teleported { .. } | MoverEvent::TeleportNotTaken { .. }),
            ) => Some(*m),
            _ => None,
        })
        .collect()
}

/// p_inter.c:817 with damage 10,000 (decision I3): the product 8,192,000,000 wraps
/// to −397,934,592 and the signed division truncates toward zero. The values are
/// the M4b note's T8.5 (observed in the strict and strict32 builds).
#[test]
fn telefrag_thrust_is_the_wrapped_c_value() {
    use crate::combat::telefrag_thrust;
    assert_eq!(telefrag_thrust(10_000, 100).unwrap(), -3_979_345);
    assert_eq!(telefrag_thrust(10_000, 400).unwrap(), -994_836);
    assert_eq!(telefrag_thrust(10_000, 1000).unwrap(), -397_934);
    // Only the stomp's amount is computed this way.
    assert!(telefrag_thrust(9_999, 100).is_err());
    assert!(telefrag_thrust(1_000, 100).is_err());
    assert!(telefrag_thrust(10_000, 0).is_err());
}

/// Fog 2's offset, p_telept.c:113–115, against the M4b note's T2.9 (observed from
/// tables.c): angle 0 gives (+1,310,700, +500), 45° (+926,460, +927,160), 90°
/// (−500, +1,310,700); 180° and 270° follow by the table's symmetry.
#[test]
fn fog_offset_is_the_table_value() {
    let at = |octant: u32| fog_offset(octant.wrapping_mul(0x2000_0000)).map(Fixed::bits);
    assert_eq!(at(0), [1_310_700, 500]);
    assert_eq!(at(1), [926_460, 927_160]);
    assert_eq!(at(2), [-500, 1_310_700]);
    assert_eq!(at(4), [-1_310_700, -500]);
    assert_eq!(at(6), [500, -1_310_700]);
}

/// The crossing loop ends exactly when `P_TeleportMove` was called (p_map.c:152).
#[test]
fn only_a_reached_teleport_move_ends_the_crossing_loop() {
    assert!(!Outcome::BackSide.ends_crossings());
    assert!(!Outcome::NoLanding.ends_crossings());
    assert!(Outcome::Blocked.ends_crossings());
    assert!(Outcome::Teleported.ends_crossings());
}

/// Wire 24 admits line special 97 and the landing; wire 23 keeps refusing both
/// with its errors (`PursuitError::Specials` at load, `UnsupportedType(14)` at the
/// THING batch, state unchanged); 39, 125 and 126 stay refused under both wires
/// (decision I1). The landing spawns in state 0 (`S_NULL`) with tics −1, in
/// `spatial.link_order` (p_mobj.c:517) but in no sector list (NOSECTOR).
#[test]
fn wire_24_admits_97_and_the_landing() {
    let tele = wad("tele");
    let mut s = setup(&tele);
    let landings = of_type(&s, OriginalActorType::MT_TELEPORTMAN);
    assert!(!landings.is_empty());
    for (id, a) in &landings {
        let l = a.life.unwrap();
        assert_eq!((l.state, l.tics), (Some(crate::StateId(0)), -1));
        assert!(s.spatial.order.contains(id));
        assert!(!crate::lifecycle::in_sector_list(a));
        assert!(!crate::lifecycle::blocks_cell_index(a));
    }
    step(&mut s, TicCommand::default()).unwrap();
    assert_eq!(
        of_type(&s, OriginalActorType::MT_TELEPORTMAN).len(),
        landings.len()
    );
    // Wire 23.
    assert_eq!(
        load(&tele, V1).err(),
        Some(SimError::Pursuit(PursuitError::Specials))
    );
    // 39, 125 and 126 instead of 97, under both wires.
    for special in [39_i16, 125, 126] {
        let mut l = tele.clone();
        let v = lines(&mut l);
        for line in 0..v.len() / 14 {
            if i16::from_le_bytes([v[14 * line + 6], v[14 * line + 7]]) == LINE_TELEPORT {
                v[14 * line + 6..14 * line + 8].copy_from_slice(&special.to_le_bytes());
            }
        }
        for policy in [V1, V2] {
            assert_eq!(
                load(&l, policy).err(),
                Some(SimError::Pursuit(PursuitError::Specials)),
                "{special} {policy:?}"
            );
        }
    }
    // The landing alone (no 97 line) under wire 23.
    let mut l = tele.clone();
    let v = lines(&mut l);
    for line in 0..v.len() / 14 {
        if i16::from_le_bytes([v[14 * line + 6], v[14 * line + 7]]) == LINE_TELEPORT {
            v[14 * line + 6..14 * line + 8].copy_from_slice(&0_i16.to_le_bytes());
        }
    }
    let mut s = load(&l, V1).unwrap();
    let before = s.snapshot().canonical_bytes();
    assert_eq!(
        s.spawn_map_things().err(),
        Some(SimError::MapSpawn(crate::MapSpawnError::UnsupportedType(
            14
        )))
    );
    assert_eq!(s.snapshot().canonical_bytes(), before);
}

/// TELE1 (C-observed, validation/M4B.md): the back-side crossing at dump tic 34
/// does nothing (DSIM tic 35: sub-tag 16 reason 0); the front-side crossing at dump
/// tic 88 teleports (DSIM tic 89): `reactiontime` 18, momentum 0, two fogs and two
/// draws (one `lastlook` per fog, p_mobj.c:506). The next 18 `P_PlayerThink` calls
/// (DSIM tics 90–107) count it down instead of `P_MovePlayer` (p_user.c:267–270):
/// forward and turn commands are ignored and the body stays put; it moves again at
/// DSIM tic 108 (dump tic 107).
#[test]
fn player_teleport_and_the_freeze_boundary() {
    let commands = demo("TELE1");
    let mut s = setup(&wad("tele"));
    run_to(&mut s, &commands, 34);
    step(&mut s, commands[34]).unwrap();
    assert!(matches!(
        teleport_events(&s)[..],
        [MoverEvent::TeleportNotTaken { reason: 0, .. }]
    ));
    run_to(&mut s, &commands[35..], 88 - 35);
    let rng = s.rng.indices()[0];
    let fogs = of_type(&s, OriginalActorType::MT_TFOG).len();
    step(&mut s, commands[88]).unwrap();
    assert!(matches!(
        teleport_events(&s)[..],
        [MoverEvent::Teleported { .. }]
    ));
    let b = body(&s);
    assert_eq!(b.life.unwrap().reaction_time, PLAYER_TELEPORT_FREEZE);
    assert_eq!(b.momentum(), [Fixed::ZERO; 2]);
    assert_eq!(b.z, b.floor);
    assert_eq!(of_type(&s, OriginalActorType::MT_TFOG).len(), fogs + 2);
    assert_eq!(s.rng.indices()[0].wrapping_sub(rng), 2);
    let landed = (b.position(), b.angle());
    for k in 1..=18 {
        step(&mut s, commands[88 + k]).unwrap();
        let b = body(&s);
        assert_eq!(
            b.life.unwrap().reaction_time,
            18 - k as i32,
            "tic {}",
            89 + k
        );
        assert_eq!((b.position(), b.angle()), landed, "tic {}", 89 + k);
    }
    assert!(
        commands[89..=106]
            .iter()
            .any(|c| c.forward != 0 || c.angle_turn != 0),
        "the frozen tics receive movement commands"
    );
    step(&mut s, commands[107]).unwrap();
    assert_ne!((body(&s).position(), body(&s).angle()), landed, "tic 108");
}

/// MTELE1 (C-observed): the east Demon crosses its 97 line at dump tic 117 with a
/// live barrel on the landing. `PIT_StompThing` refuses a non-player's move on maps
/// 1–9 (p_map.c:102–103): no teleport, no fog, no draw from the teleport, and the
/// Demon stays where its walk took it (sub-tag 16 reason 2).
#[test]
fn blocked_monster_landing_changes_nothing() {
    let commands = demo("MTELE1");
    let mut s = setup(&wad("montele"));
    run_to(&mut s, &commands, 117);
    let fogs = of_type(&s, OriginalActorType::MT_TFOG);
    step(&mut s, commands[117]).unwrap();
    let events = teleport_events(&s);
    let [
        MoverEvent::TeleportNotTaken {
            actor, reason: 2, ..
        },
    ] = events[..]
    else {
        panic!("{events:?}");
    };
    let demon = *s.actor(actor).unwrap();
    assert_eq!(of_type(&s, OriginalActorType::MT_TFOG).len(), fogs.len());
    assert!(
        !s.events().iter().any(|e| matches!(e, Event::Spawned(_))),
        "a refused teleport spawns nothing"
    );
    // Not on any landing.
    for (_, landing) in of_type(&s, OriginalActorType::MT_TELEPORTMAN) {
        assert_ne!(landing.position(), demon.position());
    }
}

/// The store-full witness (decision I7): TELE1's teleport needs two free slots for
/// its fogs (`P_SpawnMobj` twice, p_telept.c:111–117). With one free slot the tic is
/// refused with `StoreError::Capacity` before any mutation of the teleport, the
/// canonical state is unchanged, and a retry refuses again; with two the tic runs.
#[test]
fn store_full_at_the_fogs_refuses_the_tic() {
    let commands = demo("TELE1");
    let mut base = setup(&wad("tele"));
    run_to(&mut base, &commands, 88);
    let fill = |s: &mut Simulation, free: usize| {
        let registry = s.registry.clone().unwrap();
        let fog = ActorTypeId(OriginalActorType::MT_TFOG.ordinal());
        let position = body(s).position();
        let live = s.actors.order().count();
        for _ in 0..(crate::player_map_scale::EPISODE1_ACTOR_SLOTS as usize - live - free) {
            let actor = crate::lifecycle::make_actor(
                &s.world,
                crate::staging::Sectors::committed(&s.sectors),
                &registry,
                s.actor_options,
                fog,
                position,
                SpawnHeight::Floor,
            );
            let id = s.actors.spawn(actor).unwrap();
            s.spatial.link(id);
        }
    };
    let mut full = base.clone();
    fill(&mut full, 1);
    let before = full.snapshot().canonical_bytes();
    assert_eq!(
        step(&mut full, commands[88]),
        Err(StoreError::Capacity.into())
    );
    assert_eq!(full.snapshot().canonical_bytes(), before);
    assert_eq!(
        step(&mut full, commands[88]),
        Err(StoreError::Capacity.into())
    );
    assert_eq!(full.snapshot().canonical_bytes(), before);
    let mut room = base.clone();
    fill(&mut room, 2);
    step(&mut room, commands[88]).unwrap();
    assert!(matches!(
        teleport_events(&room)[..],
        [MoverEvent::Teleported { .. }]
    ));
}

/// The transient allowance of wire 24: `slots − (1 + K + R + P + B + D + L)` from
/// the THINGS lump; the load refuses a map that leaves fewer than two slots.
#[test]
fn transient_allowance_formula() {
    let b = crate::Episode1Bounds {
        monsters: 10,
        droppers: 4,
        pickups: 20,
        keycards: 1,
        barrels: 3,
        statics: 5,
        countitems: 2,
    };
    assert_eq!(
        b.transient_allowance(2, 1024),
        Some(1024 - (1 + 10 + 4 + 20 + 3 + 5 + 2))
    );
    assert_eq!(b.transient_allowance(2, 45), Some(0));
    assert_eq!(b.transient_allowance(2, 44), None);
    let tele = wad("tele");
    let map = map_of(&tele);
    // Counted independently from the lump: doomednum 14 with the medium bit (2)
    // and without MTF_NOTSINGLE (16), p_mobj.c:740–751.
    let mut l = tele.clone();
    let expected = things(&mut l)
        .chunks_exact(10)
        .filter(|r| {
            let options = u16::from_le_bytes([r[8], r[9]]);
            i16::from_le_bytes([r[6], r[7]]) == 14 && options & 16 == 0 && options & 2 != 0
        })
        .count();
    assert!(expected > 0);
    assert_eq!(
        crate::Episode1Bounds::landings(&map, Skill::Medium),
        expected
    );
}

/// TFRAG (C-observed, validation/M4B.md) without its Baron (step 5): the
/// zombieman telefragged at dump tic 63 (DSIM 64) has health −9,980, its xdeath
/// state and momentum (−3,979,285, −1,518), the wrapped thrust applied through
/// `FixedMul` toward the player's pre-teleport position; the Demon at dump tic 139
/// (DSIM 140) has health −9,850 and momentum (−994,821, −380). Neither value reads
/// the RNG, so removing the Baron's THING changes neither. With the chainsaw ready
/// no thrust applies (p_inter.c:806–810): the zombieman keeps momentum 0.
#[test]
fn telefrag_without_the_baron() {
    let mut l = wad("tfrag");
    let t = things(&mut l);
    let kept: Vec<u8> = t
        .chunks_exact(10)
        .filter(|r| i16::from_le_bytes([r[6], r[7]]) != 3003)
        .flatten()
        .copied()
        .collect();
    assert!(kept.len() < t.len(), "TFRAG places a Baron");
    *t = kept;
    let commands = demo("TFRAG1");
    let victim = |s: &Simulation, ty| of_type(s, ty)[0].1;
    let victim_id = |s: &Simulation, ty| of_type(s, ty)[0].0;
    let mut s = setup(&l);
    run_to(&mut s, &commands, 63);
    let before_stomp = s.clone();
    step(&mut s, commands[63]).unwrap();
    let z = victim(&s, OriginalActorType::MT_POSSESSED);
    let zl = z.life.unwrap();
    assert_eq!(zl.health, -9_980);
    assert_eq!(
        zl.state,
        Some(crate::StateId(OriginalState::S_POSS_XDIE1.ordinal()))
    );
    assert_eq!(z.momentum().map(Fixed::bits), [-3_979_285, -1_518]);
    // The DSIM40 reader admits the capture: the damage record of 10,000 with
    // inflictor and source the player, the knockback and the health −9,980.
    let bytes = s.snapshot().canonical_bytes();
    crate::Diagnostic::decode_episode1_v2(&bytes, crate::DiagnosticLimits::default())
        .expect("telefrag capture");
    // The health domain's boundary: −9,999 is admitted, −10,000 is not.
    let slot = of_type(&s, OriginalActorType::MT_POSSESSED)[0].0.slot();
    let snap = s.snapshot();
    let with_health = |health: i32| -> Vec<u8> {
        let path = format!("actor.{slot}.health");
        let fields: Vec<(String, Vec<u8>)> = snap
            .fields()
            .map(|f| {
                let value = if f.path() == path {
                    health.to_le_bytes().to_vec()
                } else {
                    f.value().to_vec()
                };
                (f.path().to_string(), value)
            })
            .collect();
        let mut out = b"DSIM".to_vec();
        out.extend(snap.version().to_le_bytes());
        out.extend((fields.len() as u64).to_le_bytes());
        for (p, v) in fields {
            out.extend((p.len() as u32).to_le_bytes());
            out.extend(p.as_bytes());
            out.extend((v.len() as u64).to_le_bytes());
            out.extend(v);
        }
        out
    };
    let decode = |bytes: &[u8]| {
        crate::Diagnostic::decode_episode1_v2(bytes, crate::DiagnosticLimits::default()).map(|_| ())
    };
    assert_eq!(decode(&with_health(-9_999)), Ok(()));
    assert_eq!(
        decode(&with_health(-10_000)),
        Err(crate::DiagnosticError::Identity)
    );
    run_to(&mut s, &commands[64..], 139 - 64);
    step(&mut s, commands[139]).unwrap();
    let d = victim(&s, OriginalActorType::MT_SERGEANT);
    assert_eq!(d.life.unwrap().health, -9_850);
    assert_eq!(d.momentum().map(Fixed::bits), [-994_821, -380]);
    // The chainsaw ready: the same damage, planned and applied in the state before
    // the stomp with only the player's ready weapon changed (the plan reads nothing
    // else of the weapon), gives no knockback.
    let after = |chainsaw: bool| {
        let mut t = before_stomp.clone();
        let target = victim_id(&t, OriginalActorType::MT_POSSESSED);
        let mo = t.original_player.unwrap().mo.unwrap();
        t.checked_demon_operation(|host| {
            if chainsaw {
                if let Some(crate::player_weapons::PlayerWeaponRuntime::Pack(w)) =
                    host.player.as_deref_mut().and_then(|p| p.weapon.as_mut())
                {
                    w.ready = crate::Weapon::Chainsaw;
                }
            }
            let request = crate::DamageRequest {
                target,
                inflictor: Some(mo),
                source: Some(mo),
                amount: TELEFRAG_DAMAGE,
            };
            let plan = crate::combat::DamagePlan::telefrag(host, request).unwrap();
            host.apply_damage(plan);
        })
        .unwrap();
        t
    };
    let saw = after(true);
    assert_ne!(
        victim(&after(false), OriginalActorType::MT_POSSESSED).momentum(),
        [Fixed::ZERO; 2]
    );
    let z = victim(&saw, OriginalActorType::MT_POSSESSED);
    assert_eq!(z.life.unwrap().health, -9_980);
    assert_eq!(z.momentum(), [Fixed::ZERO; 2]);
}

/// Damage above 1,000 stays refused outside the stomp (decision I3), under wire
/// 24 and under wire 23: `damage_actor` with 10,000 is `CombatError::Damage`, and
/// the state is unchanged.
#[test]
fn damage_above_1000_stays_refused_outside_the_stomp() {
    let mut room = crate::player_movers_tests::room(13);
    room.add_thing([1000, 64, 180, 1, 7]);
    let mut wire23 =
        Simulation::original_player_episode1_map(room.map(), MapContextOptions::default()).unwrap();
    wire23.spawn_map_things().unwrap();
    wire23
        .initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    wire23.spawn_level_specials().unwrap();
    for mut s in [setup(&wad("tele")), wire23] {
        let mo = s.original_player.unwrap().mo.unwrap();
        let before = s.snapshot().canonical_bytes();
        for amount in [1_001, TELEFRAG_DAMAGE] {
            assert_eq!(
                s.damage_actor(crate::DamageRequest {
                    target: mo,
                    inflictor: None,
                    source: None,
                    amount,
                }),
                Err(crate::CombatError::Damage.into())
            );
            assert_eq!(s.snapshot().canonical_bytes(), before);
        }
    }
}

/// TELE2X (second M4b set), C-observed at dump tic 36 (DSIM 37): the split move's
/// first half teleports the player, the second half continues from the landing
/// across the second 97 line and teleports it again (p_mobj.c:147–191): four fogs
/// and four draws in one tic, the player on the second landing at angle 90 with
/// `reaction_time` 18 and momentum 0.
#[test]
fn two_teleports_in_one_tic() {
    let commands = demo("TELE2X");
    let mut s = setup(&wad("tele2x"));
    run_to(&mut s, &commands, 36);
    let rng = s.rng.indices()[0];
    step(&mut s, commands[36]).unwrap();
    assert!(matches!(
        teleport_events(&s)[..],
        [MoverEvent::Teleported { .. }, MoverEvent::Teleported { .. }]
    ));
    assert_eq!(of_type(&s, OriginalActorType::MT_TFOG).len(), 4);
    assert_eq!(usize::from(s.rng.indices()[0].wrapping_sub(rng)), 4);
    let b = body(&s);
    assert_eq!(b.position().map(Fixed::bits), [1152 << 16, 640 << 16]);
    assert_eq!(b.angle(), 0x4000_0000);
    assert_eq!(b.life.unwrap().reaction_time, 18);
    assert_eq!(b.momentum(), [Fixed::ZERO; 2]);
}

/// TELEZ (M4b step 14, review finding F1): a monster's fog 1 has its pre-move z.
/// `P_Move` calls `P_TryMove` (p_enemy.c:293), whose crossing loop runs
/// `EV_Teleport` (p_map.c:500–514) before `P_Move` sets `z = floorz`
/// (p_enemy.c:327–328), and `EV_Teleport` spawns fog 1 at `oldz = thing->z`
/// (p_telept.c:100–111). The Demon spawned at its centre sector's floor 0
/// (p_mobj.c:517–525) while its box reaches HIGH's floor 24, so its first step's
/// `tmfloorz` is 24 and its z is still 0 when it teleports; fog 1 lands in MID
/// (floor 0) and its own `P_ZMovement` leaves it there (p_mobj.c:284–311).
/// C-observed (tests/reference/m4b_synthetic_results2.json, TELEZ facts): the
/// teleport in dump tic 10 (DSIM tic 11), the Demon before it at x −8, z 0,
/// floorz 0; fog 1 at (2, 0, 0); the Demon on the landing at z 0. Before the
/// fix the move set z to 24 ahead of the crossing and fog 1 stood at 24.
#[test]
fn a_monster_fog_keeps_the_pre_move_z() {
    let commands = demo("TELEZ");
    let mut s = setup(&wad("telez"));
    run_to(&mut s, &commands, 10);
    let [(demon, before)] = of_type(&s, OriginalActorType::MT_SERGEANT)[..] else {
        panic!("one Demon");
    };
    assert_eq!(before.position().map(Fixed::bits), [-8 << 16, 0]);
    assert_eq!((before.z, before.floor), (Fixed::ZERO, Fixed::ZERO));
    step(&mut s, commands[10]).unwrap();
    let events = teleport_events(&s);
    let [MoverEvent::Teleported { actor, old, .. }] = events[..] else {
        panic!("{events:?}");
    };
    assert_eq!(actor, demon);
    assert_eq!(old.map(Fixed::bits), [2 << 16, 0, 0]);
    let fogs: Vec<_> = of_type(&s, OriginalActorType::MT_TFOG)
        .into_iter()
        .map(|(_, f)| (f.position().map(Fixed::bits), f.z.bits()))
        .collect();
    assert_eq!(fogs.len(), 2);
    assert!(fogs.contains(&([2 << 16, 0], 0)), "{fogs:?}");
    let after = *s.actor(demon).unwrap();
    assert_eq!(after.position().map(Fixed::bits), [1152 << 16, 0]);
    assert_eq!((after.z, after.floor), (Fixed::ZERO, Fixed::ZERO));
}

// --- Mutation-survivor tests (M4b step 11, Mutants run 43543402068) -----------
//
// Every expectation below comes from a citation already in the code comments or the
// M4b note (§2 T3, T8) or from the written contract of the function under test;
// none comes from Rust output. Tests that call below the public API name the guard
// and why a whole-tic run cannot reach it.

type Host<'a, 'b> = crate::lifecycle::LifeHost<'a, &'b mut crate::ThinkerStore<crate::Actor>>;

/// `tfrag` after its load, with an extra barrel (doomednum 2035, all skills) at an
/// open spot of the first room, as a victim for the plans below.
fn tfrag_with_barrel() -> Simulation {
    let mut l = wad("tfrag");
    let mut record = Vec::new();
    for v in [-300_i16, 60, 0, 2035, 7] {
        record.extend(v.to_le_bytes());
    }
    things(&mut l).extend(record);
    setup(&l)
}
fn id_of(s: &Simulation, ty: OriginalActorType) -> ActorId {
    of_type(s, ty)[0].0
}
fn request(
    target: ActorId,
    inflictor: Option<ActorId>,
    source: Option<ActorId>,
    amount: i32,
) -> DamageRequest {
    DamageRequest {
        target,
        inflictor,
        source,
        amount,
    }
}
/// `DamagePlan::telefrag` of `request` on a copy of `s` after `prepare` changed the
/// copy's staged host. Only the decision is returned.
fn telefrag_decision(
    s: &Simulation,
    prepare: impl FnOnce(&mut Host<'_, '_>),
    request: DamageRequest,
) -> Result<(), SimError> {
    let mut t = s.clone();
    t.checked_demon_operation(|host| {
        prepare(host);
        crate::combat::DamagePlan::telefrag(host, request).map(|_| ())
    })
    .unwrap()
}
fn ready_chainsaw(host: &mut Host<'_, '_>) {
    if let Some(crate::player_weapons::PlayerWeaponRuntime::Pack(w)) =
        host.player.as_deref_mut().and_then(|p| p.weapon.as_mut())
    {
        w.ready = crate::Weapon::Chainsaw;
    }
}

/// Pins the six-way request guard of `DamagePlan::telefrag` (combat.rs, the first
/// `if`): the stomp's damage is exactly 10,000 (p_map.c:105), its inflictor and
/// source are the same present actor (`tmthing` twice) and that actor is the player
/// body (p_map.c:102, `tmthing->player`), which is never the victim (T3.9). Each
/// case breaks exactly one clause of an otherwise accepted request (the first is
/// the accepted request itself) and expects `CombatError::Damage`. Below the public
/// API because the public stomp only ever builds sound requests; a refusal here is
/// the defence in depth of decision I3 ("every other damage keeps the 1,000 bound").
#[test]
fn telefrag_plan_refuses_each_unsound_request() {
    let s = setup(&wad("tfrag"));
    let mo = s.original_player.unwrap().mo.unwrap();
    let zombie = id_of(&s, OriginalActorType::MT_POSSESSED);
    let baron = id_of(&s, OriginalActorType::MT_BRUISER);
    let landing = id_of(&s, OriginalActorType::MT_TELEPORTMAN);
    let refused = Err(crate::CombatError::Damage.into());
    let sound = request(zombie, Some(mo), Some(mo), TELEFRAG_DAMAGE);
    assert_eq!(telefrag_decision(&s, |_| {}, sound), Ok(()));
    // Amount 9,999: the thrust is skipped (chainsaw ready, p_inter.c:806–811) so
    // the later `telefrag_thrust` check cannot supply the same refusal.
    assert_eq!(
        telefrag_decision(
            &s,
            ready_chainsaw,
            request(zombie, Some(mo), Some(mo), TELEFRAG_DAMAGE - 1)
        ),
        refused
    );
    // No inflictor and no source, with the player having no body: only the
    // `inflictor.is_none()` clause holds.
    assert_eq!(
        telefrag_decision(
            &s,
            |host| host.player.as_deref_mut().unwrap().mo = None,
            request(zombie, None, None, TELEFRAG_DAMAGE)
        ),
        refused
    );
    // Inflictor and source differ, the source being the player body.
    assert_eq!(
        telefrag_decision(
            &s,
            |_| {},
            request(zombie, Some(baron), Some(mo), TELEFRAG_DAMAGE)
        ),
        refused
    );
    // Inflictor and source are the same actor but not the player body.
    assert_eq!(
        telefrag_decision(
            &s,
            |_| {},
            request(zombie, Some(baron), Some(baron), TELEFRAG_DAMAGE)
        ),
        refused
    );
    // The victim is the player body itself.
    assert_eq!(
        telefrag_decision(&s, |_| {}, request(mo, Some(mo), Some(mo), TELEFRAG_DAMAGE)),
        refused
    );
    // A victim that is neither a monster nor a barrel (the landing is shootable by
    // neither): `MonsterError::Target`, not a plan.
    assert_eq!(
        telefrag_decision(
            &s,
            |_| {},
            request(landing, Some(mo), Some(mo), TELEFRAG_DAMAGE)
        ),
        Err(crate::MonsterError::Target.into())
    );
}

/// Pins the victim classes of `DamagePlan::telefrag`: a barrel is a legal victim
/// (`PIT_StompThing` stomps any `MF_SHOOTABLE` thing, p_map.c:85–86, and a barrel
/// is one, T3.4) and, not being `MF_COUNTKILL` (info.c, MT_BARREL flags), earns no
/// kill credit (p_inter.c:683–691), whereas a zombieman earns one.
#[test]
fn telefrag_credits_a_kill_only_for_a_counted_monster() {
    let s = tfrag_with_barrel();
    let mo = s.original_player.unwrap().mo.unwrap();
    let kills = |ty: OriginalActorType| {
        let target = id_of(&s, ty);
        let mut t = s.clone();
        t.checked_demon_operation(|host| {
            let before = host.statistics.as_deref().unwrap().single_player_kill_count;
            let plan = crate::combat::DamagePlan::telefrag(
                host,
                request(target, Some(mo), Some(mo), TELEFRAG_DAMAGE),
            )
            .unwrap();
            host.apply_damage(plan);
            host.statistics.as_deref().unwrap().single_player_kill_count - before
        })
        .unwrap()
    };
    assert_eq!(kills(OriginalActorType::MT_BARREL), 0);
    assert_eq!(kills(OriginalActorType::MT_POSSESSED), 1);
    assert_eq!(kills(OriginalActorType::MT_BRUISER), 1);
}

/// Pins the thrust preflight of `DamagePlan::telefrag` (its `if life.health > 0 &&
/// life.flags & MF_SHOOTABLE != 0 && thrust`): `P_DamageMobj` returns before the
/// thrust block for a victim that is not shootable (p_inter.c:787–788) or has
/// `health <= 0` (790–791), and the block is skipped with the chainsaw ready
/// (806–811). The preflight is the Rust-only refusal of a knockback whose momentum
/// would leave the admitted travel bound, so a victim whose momentum already
/// exceeds every bound is refused exactly when the thrust block runs: live and
/// shootable, saw not ready. Below the public API because no admitted capture holds
/// a monster with such a momentum; the plan's preflight is the guard.
#[test]
fn telefrag_thrust_preflight_runs_only_where_the_thrust_runs() {
    let s = setup(&wad("tfrag"));
    let mo = s.original_player.unwrap().mo.unwrap();
    let zombie = id_of(&s, OriginalActorType::MT_POSSESSED);
    let stomp = request(zombie, Some(mo), Some(mo), TELEFRAG_DAMAGE);
    let overweight = |host: &mut Host<'_, '_>, health: Option<i32>, shootable: bool| {
        let a = host.access.get_mut(zombie).unwrap();
        a.spec.momentum = [Fixed::from_bits(i32::MIN); 2];
        let l = a.life.as_mut().unwrap();
        if let Some(health) = health {
            l.health = health;
        }
        if !shootable {
            l.flags &= !MF_SHOOTABLE;
        }
    };
    // Live, shootable, no saw: the preflight runs and refuses the momentum.
    assert!(telefrag_decision(&s, |h| overweight(h, None, true), stomp).is_err());
    // Health exactly 0, below 0, and not shootable: `P_DamageMobj` ignores the hit
    // before any thrust, so the plan is accepted (as an ignored plan).
    for health in [0, -1] {
        assert_eq!(
            telefrag_decision(&s, |h| overweight(h, Some(health), true), stomp),
            Ok(()),
            "health {health}"
        );
    }
    assert_eq!(
        telefrag_decision(&s, |h| overweight(h, None, false), stomp),
        Ok(())
    );
    // Chainsaw ready: no thrust block, no preflight.
    assert_eq!(
        telefrag_decision(
            &s,
            |h| {
                overweight(h, None, true);
                ready_chainsaw(h);
            },
            stomp
        ),
        Ok(())
    );
}

/// Pins `stomped`'s reach test (p_map.c:88–95, T3.4): a shootable thing is stomped
/// exactly when `|thing.x − tmx| < blockdist` and `|thing.y − tmy| < blockdist`,
/// `blockdist` being the SUM of the two radii (a zombieman's 20 and the player's
/// 16, info.c), strictly. Below the public API: the whole-tic tests land on the
/// landings' exact spots, which overlap fully; the boundary of the reach is not
/// otherwise visited.
#[test]
fn stomp_reach_is_the_strict_sum_of_the_radii() {
    let s = setup(&wad("tfrag"));
    let mo = s.original_player.unwrap().mo.unwrap();
    let thing = *s.actor(mo).unwrap();
    let zombie = id_of(&s, OriginalActorType::MT_POSSESSED);
    let z = s.actor(zombie).unwrap().position();
    let reach = Fixed::from_map_units(20 + 16).bits();
    let at = |dx: i32, dy: i32| {
        let mut t = s.clone();
        let dest = [
            Fixed::from_bits(z[0].bits() + dx),
            Fixed::from_bits(z[1].bits() + dy),
        ];
        t.checked_demon_operation(|host| stomped(host, mo, &thing, dest))
            .unwrap()
    };
    assert_eq!(at(0, 0), [zombie]);
    assert_eq!(at(reach - 1, 0), [zombie]);
    assert_eq!(at(0, -(reach - 1)), [zombie]);
    assert_eq!(at(reach - 1, reach - 1), [zombie]);
    // Exactly the summed radii on either axis, and beyond, on each side: not stomped.
    for (dx, dy) in [
        (reach, 0),
        (-reach, 0),
        (0, reach),
        (0, -reach),
        (reach, reach),
    ] {
        assert_eq!(at(dx, dy), [], "({dx}, {dy})");
    }
    // A product of the radii in place of the sum would reach everything near.
    assert_eq!(at(reach * 2, 0), []);
}

/// Pins `ev_teleport`'s check of the latched failure after each victim's damage
/// (`latched`): a failure latched while a telefrag's death state ran (a refused
/// state entry) aborts the teleport before the relink instead of letting the thing
/// land. Below the public API because every Episode 1 victim's first death state has
/// nonzero tics and no action (M4b note §2 T3.8), so no whole-tic run latches there;
/// the failure is latched here by hand, before the call.
#[test]
fn a_failure_latched_during_a_stomp_aborts_the_teleport() {
    let l = wad("tfrag");
    let commands = demo("TFRAG1");
    let mut s = setup(&l);
    run_to(&mut s, &commands, 63);
    let mo = s.original_player.unwrap().mo.unwrap();
    let line = map_of(&l)
        .linedefs()
        .iter()
        .position(|d| d.special == LINE_TELEPORT)
        .expect("a 97 line") as u16;
    let planted: SimError = crate::CombatError::Reference.into();
    let mut inner = None;
    let outer = s.clone().checked_demon_operation(|host| {
        host.fail_sight(planted);
        inner = Some(ev_teleport(host, mo, line, 0));
    });
    // The call itself refuses with the planted failure (not `Teleported`), and the
    // staged tic is discarded, as for every latched refusal.
    assert_eq!(inner, Some(Err(planted)));
    assert_eq!(outer, Err(planted));
    // Control: without the planted failure the same call teleports.
    let control = s
        .clone()
        .checked_demon_operation(|host| ev_teleport(host, mo, line, 0))
        .unwrap();
    assert_eq!(control, Ok(Outcome::Teleported));
}

/// Pins `cross`'s defence in depth: only special 97 under a teleport world is
/// serviced here (the dispatcher admits nothing else, rule 1); a crossing of any
/// other special is `PursuitError::Specials`, not a teleport. Below the public API
/// because the dispatcher in `Motion::cross` sends only 97 to this function.
#[test]
fn teleport_cross_refuses_any_other_special() {
    let l = wad("tfrag");
    let s = setup(&l);
    let mo = s.original_player.unwrap().mo.unwrap();
    let line = map_of(&l)
        .linedefs()
        .iter()
        .position(|d| d.special == LINE_TELEPORT)
        .expect("a 97 line") as u16;
    let mut t = s.clone();
    let result = t
        .checked_demon_operation(|host| {
            cross(
                host,
                mo,
                crate::line_triggers::Crosser::Player,
                crate::line_triggers::Crossing {
                    line,
                    side: 0,
                    special: LINE_TELEPORT + 1,
                },
            )
        })
        .unwrap();
    assert_eq!(result, Err(PursuitError::Specials.into()));
}

/// Pins `validate_landing` (the tic-boundary witness of a landing): the record is
/// exactly what `P_SpawnMapThing` left (state `S_NULL`, tics −1, health 1,000, the
/// info flags plus at most `MF_AMBUSH`, no target, no momentum, inside the room;
/// info.c:2174–2198, p_mobj.c:479–534, 708–797). Each case breaks exactly one clause
/// of an accepted record. Below the public API because the reader of whole-tic
/// states never presents a landing that was edited this way.
#[test]
fn a_landing_must_be_exactly_as_spawned() {
    let s = setup(&wad("tele"));
    let room = s.original_player.unwrap().room;
    let landing = of_type(&s, OriginalActorType::MT_TELEPORTMAN)[0].1;
    assert_eq!(validate_landing(room, landing), Ok(()));
    let invariant = Err(PlayerBodyError::Invariant.into());
    // `MF_AMBUSH` (p_mobj.h:131) is the one flag a THING may add.
    let mut ambush = landing;
    ambush.life.as_mut().unwrap().flags |= crate::world_pickups::MF_AMBUSH;
    assert_eq!(validate_landing(room, ambush), Ok(()));
    type Edit = fn(&mut crate::Actor);
    let edits: [(&str, Edit); 8] = [
        ("state", |a| {
            a.life.as_mut().unwrap().state = Some(crate::StateId(1))
        }),
        ("tics", |a| a.life.as_mut().unwrap().tics = 1),
        ("health", |a| a.life.as_mut().unwrap().health = 999),
        ("flags", |a| a.life.as_mut().unwrap().flags |= MF_SHOOTABLE),
        ("target", |a| {
            a.life.as_mut().unwrap().target = Some(a_handle())
        }),
        ("momentum", |a| {
            a.spec.momentum = [Fixed::from_bits(1), Fixed::ZERO]
        }),
        ("momz", |a| a.momz = Fixed::from_bits(1)),
        ("position", |a| {
            a.spec.position = [Fixed::from_map_units(20_000), Fixed::ZERO];
        }),
    ];
    for (name, edit) in edits {
        let mut broken = landing;
        edit(&mut broken);
        assert_eq!(validate_landing(room, broken), invariant, "{name}");
    }
    assert_eq!(
        validate_landing(
            room,
            crate::Actor {
                life: None,
                ..landing
            }
        ),
        invariant
    );
}
fn a_handle() -> ActorId {
    ActorId::from_parts(0, 1)
}

/// Runs `f` on a [`Motion`] of `mover` over the committed state of `s`, inside a
/// real thinker traversal (the only way to hold the `ThinkerContext` it borrows).
fn with_motion<R>(
    s: &mut Simulation,
    mover: Mover,
    f: impl FnOnce(&mut Motion<'_, '_, '_>) -> R,
) -> R {
    let mut actors = s.actors.clone();
    let budget = actors.visit_bound();
    let mut f = Some(f);
    let mut out = None;
    let registry = s.registry.clone().unwrap();
    actors
        .run(budget, |visit, context| {
            if matches!(visit, crate::Visit::Think(_)) {
                if let Some(f) = f.take() {
                    let mut motion = Motion {
                        mover,
                        access: context,
                        player: s.original_player.as_mut(),
                        statistics: s.statistics.as_mut(),
                        perception: None,
                        registry: &registry,
                        world: &s.world,
                        sectors: crate::staging::Sectors::committed(&s.sectors),
                        options: s.actor_options,
                        rng: &mut s.rng,
                        bindings: &mut s.bindings,
                        specials: None,
                        boundary: s.boundary.as_deref(),
                    };
                    out = Some(f(&mut motion));
                }
            }
        })
        .unwrap();
    out.expect("a live thinker")
}

/// Pins `Motion`'s three witnesses of wire 24 (teleport.rs): the slide escape
/// witness (the player only: M3h5 note §4.5, `Contacts::after_slide`), the
/// post-move escape witness (a barrel on a map with a one-sided boundary only:
/// `BarrelMotion::after_move`, decision F3 of M3h9b) and the real-map intercept
/// witness of the player's slide (`MAX_INTERCEPTS`, M3h6 note §4.4). Below the public
/// API because each witness guards a position the world's walls cannot produce
/// (Theorem B of M3h5 §4.5; a thing outside the boundary), so a whole-tic run never
/// presents it; the actor handed to the witness here is placed outside by hand.
#[test]
fn motion_witnesses_belong_to_their_movers() {
    use crate::movement::MotionAccess;
    let s = setup(&wad("tele"));
    assert!(s.boundary.is_some(), "the TELE map has a closed boundary");
    let mo = s.original_player.unwrap().mo.unwrap();
    let inside = *s.actor(mo).unwrap();
    let mut outside = inside;
    outside.spec.position = [Fixed::from_map_units(20_000), Fixed::from_map_units(20_000)];
    let escape = Err(crate::MapScaleError::Escape.into());
    for (mover, slide, step) in [
        (Mover::Player, escape, Ok(())),
        (Mover::Barrel, Ok(()), escape),
        (Mover::Other, Ok(()), Ok(())),
    ] {
        let mut t = s.clone();
        let (a, b, c, d) = with_motion(&mut t, mover, |m| {
            (
                m.after_slide(&outside),
                m.after_slide(&inside),
                m.after_move(&outside),
                m.after_move(&inside),
            )
        });
        assert_eq!(a, slide, "{mover:?} after_slide outside");
        assert_eq!(b, Ok(()), "{mover:?} after_slide inside");
        assert_eq!(c, step, "{mover:?} after_move outside");
        assert_eq!(d, Ok(()), "{mover:?} after_move inside");
    }
    for (mover, limit) in [
        (Mover::Player, Some(crate::player_map_scale::MAX_INTERCEPTS)),
        (Mover::Barrel, None),
        (Mover::Other, None),
    ] {
        let mut t = s.clone();
        assert_eq!(
            with_motion(&mut t, mover, |m| m.intercept_limit()),
            limit,
            "{mover:?}"
        );
    }
    assert_eq!(crate::player_map_scale::MAX_INTERCEPTS, 128);
}

/// Pins the load rule of wire 24 (`with_original_registry`): the map is admitted
/// when its transient allowance, `1024 − (1 + K + R + P + B + D + L)`, is at least 2
/// (the two fogs of one teleport), and refused with `StoreError::Capacity` below it
/// (M4b note §4.1 on the allowance; `transient_allowance`'s own contract). The map
/// is the TELE map with its THINGS replaced by one player start and `L` landings
/// (doomednum 14, all skills), so the allowance is `1023 − L` by the formula.
#[test]
fn the_load_admits_an_allowance_of_exactly_two() {
    let with_landings = |landings: usize| {
        let mut l = wad("tele");
        let mut records = Vec::new();
        for (kind, n) in [(1_i16, 1), (14, landings)] {
            for _ in 0..n {
                for v in [0_i16, 0, 0, kind, 7] {
                    records.extend(v.to_le_bytes());
                }
            }
        }
        *things(&mut l) = records;
        load(&l, V2).map(|_| ())
    };
    assert_eq!(with_landings(1020), Ok(()), "allowance 3");
    assert_eq!(with_landings(1021), Ok(()), "allowance 2");
    assert_eq!(
        with_landings(1022),
        Err(StoreError::Capacity.into()),
        "allowance 1"
    );
    assert_eq!(
        with_landings(1023),
        Err(StoreError::Capacity.into()),
        "allowance 0"
    );
    assert_eq!(
        with_landings(1024),
        Err(StoreError::Capacity.into()),
        "placed things alone exceed the slots"
    );
}

/// Pins the default `MotionAccess::cross` (movement.rs): an access that does not run
/// crossings inside the move (`crosses_in_move()` false, the default of every access
/// but wire 24's `Motion`) is never asked to, and its `cross` is the explicit
/// `unreachable!`, not a quiet `Ok`, so a future access that claims `crosses_in_move`
/// without implementing `cross` fails loudly instead of skipping a teleport. Below
/// the public API because the only caller is guarded by `crosses_in_move`.
#[test]
#[should_panic(expected = "no crossing inside the move without the teleport policy")]
fn the_default_access_never_crosses_inside_the_move() {
    use crate::movement::MotionAccess;
    let mut s = setup(&wad("tele"));
    let mo = s.original_player.unwrap().mo.unwrap();
    let mut actor = *s.actor(mo).unwrap();
    let mut access = |_: ActorId| -> Option<crate::Actor> { None };
    assert!(!access.crosses_in_move());
    let mut events = Vec::new();
    let _ = access.cross(
        mo,
        &mut actor,
        crate::line_triggers::Crosser::Player,
        crate::line_triggers::Crossing {
            line: 0,
            side: 0,
            special: LINE_TELEPORT,
        },
        crate::staging::Sectors::committed(&s.sectors),
        &mut s.spatial,
        &mut events,
    );
}

/// Pins which movers carry the escape witness under wire 24 (`Mover` of
/// `thinkers::run`: a barrel on a map with a one-sided boundary is `Mover::Barrel`;
/// a monster, a corpse or a barrel without a boundary is `Mover::Other`, "no
/// witness"; teleport.rs). The witness refuses a tic whose accepted move leaves
/// the boundary's inside (`BarrelMotion::after_move`, M3h9b decision F3). On the TFRAG
/// map, which has a boundary, a zombieman and a barrel are each put by hand into the
/// void above the first room (outside the boundary, inside the vertex
/// rectangle: nothing the walls would let a thing reach) with a one-unit momentum:
/// the zombieman's tic is accepted and it slides on; the barrel's is refused with
/// `MapScaleError::Escape`. Below the public API because no admitted capture holds a
/// thing outside the boundary; the surgery is the only way to show which witness each
/// mover has.
#[test]
fn only_a_barrel_carries_the_escape_witness() {
    let in_the_void = |ty: OriginalActorType| {
        let mut s = tfrag_with_barrel();
        assert!(s.boundary.is_some());
        let victim = id_of(&s, ty);
        let at = [Fixed::from_map_units(-400), Fixed::from_map_units(200)];
        assert!(!s.boundary.as_ref().unwrap().inside(at), "the void");
        let location = s.world.locate_validated(at);
        let sector = s.sectors[usize::from(location.sector)];
        {
            let a = s.actors.get_mut(victim).unwrap();
            a.spec.position = at;
            a.spec.momentum = [Fixed::from_map_units(1), Fixed::ZERO];
            a.floor = sector.floor;
            a.ceiling = sector.ceiling;
            a.z = sector.floor;
            a.location = Some(location);
        }
        s.spatial.link(victim);
        let result = step(&mut s, TicCommand::default());
        (result, s.actor(victim).map(|a| a.position()))
    };
    let (zombie, moved) = in_the_void(OriginalActorType::MT_POSSESSED);
    assert_eq!(zombie, Ok(()));
    assert_eq!(moved.unwrap()[0], Fixed::from_map_units(-399));
    let (barrel, _) = in_the_void(OriginalActorType::MT_BARREL);
    assert_eq!(barrel, Err(crate::MapScaleError::Escape.into()));
}
