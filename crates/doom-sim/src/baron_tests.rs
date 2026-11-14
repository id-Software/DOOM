//! The Baron of Hell and `A_BossDeath` under wire 24 (M4b step 5). Expected values
//! come from the C source (cited), from the genuine engine's runs recorded in
//! docs/validation/M4B.md ("C-observed", dump tic N is DSIM tic N + 1), or from
//! written derivations; never from Rust output. The tic-for-tic comparison of the
//! Baron scripts against the genuine goldens is
//! `tests/episode1v2_mechanism_goldens.rs`; these tests pin the individual
//! mechanisms on those runs and on edited copies of their WADs.
use super::*;
use crate::diagonal_room::Lumps;
use crate::identity::OriginalState;
use crate::{
    ActorTypeId, CombatEvent, DamageRequest, Fixed, MapContextOptions, MapSpawnError, MonsterEvent,
    OriginalExecutionProfile as P, OriginalRegistryError, PlayerSlot, ProjectileEvent, Simulation,
    Skill, TicCommand, WeaponInventory, demo::parse_demo_episode1_v2, episode1::Episode1Bounds,
    original_registry,
};

const V1: P = P::PlayerMonstersEpisode1V1;
const V2: P = P::PlayerMonstersEpisode1V2;
const BARON: OriginalActorType = OriginalActorType::MT_BRUISER;
const BALL: OriginalActorType = OriginalActorType::MT_BRUISERSHOT;

fn reference(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name);
    std::fs::read(path).expect("reference file")
}
fn wad(name: &str) -> Lumps {
    let mut bytes = reference(&format!("m4b_wads/{name}.wad"));
    bytes[0] = b'P';
    Lumps::parse(&bytes)
}
fn lump<'a>(l: &'a mut Lumps, name: &str) -> &'a mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == name).unwrap().1
}
fn map_of(l: &Lumps, slot: &str) -> doom_map::Map {
    let bytes = l.wad();
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, slot, doom_map::MapLimits::default()).unwrap()
}
fn setup(l: &Lumps, slot: &str, map_number: u8) -> Simulation {
    let mut s = Simulation::original_player_episode1_v2_map(
        map_of(l, slot),
        map_number,
        Skill::Medium,
        MapContextOptions::default(),
    )
    .unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn demo(script: &str, map_number: u8) -> Vec<TicCommand> {
    parse_demo_episode1_v2(&reference(&format!("m4b_scripts/{script}.lmp")), map_number)
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
fn of_type(s: &Simulation, ty: OriginalActorType) -> Vec<(ActorId, crate::Actor)> {
    s.actors
        .order()
        .filter(|id| s.actors.is_pending(*id) == Some(false))
        .map(|id| (id, *s.actor(id).unwrap()))
        .filter(|(_, a)| a.life.is_some_and(|l| l.actor_type.0 == ty.ordinal()))
        .collect()
}
fn is_type(s: &Simulation, id: ActorId, ty: OriginalActorType) -> bool {
    s.actor(id)
        .and_then(|a| a.life)
        .is_some_and(|l| l.actor_type.0 == ty.ordinal())
}
fn boss_events(s: &Simulation) -> Vec<(ActorId, u8)> {
    s.events()
        .iter()
        .filter_map(|e| match e {
            Event::Mover(MoverEvent::BossDeath { actor, outcome }) => Some((*actor, *outcome)),
            _ => None,
        })
        .collect()
}
/// Every `A_BossDeath` of a script: (DSIM tic, outcome), with the tic's floor calls.
fn boss_run(
    l: &Lumps,
    slot: &str,
    map_number: u8,
    script: &str,
) -> Vec<(usize, u8, Vec<MoverEvent>)> {
    let commands = demo(script, map_number);
    let mut s = setup(l, slot, map_number);
    let mut out = Vec::new();
    for (i, c) in commands.iter().enumerate() {
        match step(&mut s, *c) {
            Ok(()) => {}
            Err(SimError::LevelExited) => break,
            Err(e) => panic!("{script} tic {}: {e:?}", i + 1),
        }
        for (actor, outcome) in boss_events(&s) {
            assert!(is_type(&s, actor, BARON));
            let calls = s
                .events()
                .iter()
                .filter_map(|e| match e {
                    Event::Mover(m @ (MoverEvent::Call { .. } | MoverEvent::Started { .. })) => {
                        Some(*m)
                    }
                    _ => None,
                })
                .collect();
            out.push((i + 1, outcome, calls));
        }
    }
    out
}

/// `snap`'s canonical bytes with one field's value replaced.
fn with_field(snap: &crate::Snapshot, path: &str, value: &[u8]) -> Vec<u8> {
    let mut out = b"DSIM".to_vec();
    out.extend(snap.version().to_le_bytes());
    out.extend((snap.fields().count() as u64).to_le_bytes());
    for f in snap.fields() {
        let v = if f.path() == path { value } else { f.value() };
        out.extend((f.path().len() as u32).to_le_bytes());
        out.extend(f.path().as_bytes());
        out.extend((v.len() as u64).to_le_bytes());
        out.extend(v);
    }
    out
}

/// Wire 24 admits doomednum 3003 with its closure, raise states excluded; wire 23
/// keeps refusing it with the unchanged error and state (the THING batch changes
/// nothing when it refuses). `K` counts it under wire 24 only.
#[test]
fn wire_24_admits_the_baron_wire_23_refuses_it() {
    let l = wad("baron");
    let map = map_of(&l, "E1M1");
    let v1 = Episode1Bounds::from_map(&map);
    let v2 = Episode1Bounds::from_map_v2(&map, Skill::Medium);
    let placed = map.things().iter().filter(|t| t.kind == 3003).count();
    assert_eq!(placed, 1);
    assert_eq!(v2.monsters, v1.monsters + 1);
    assert_eq!(v2.droppers, v1.droppers);
    let mut s =
        Simulation::original_player_episode1_map(map_of(&l, "E1M1"), MapContextOptions::default())
            .unwrap();
    let before = s.snapshot().canonical_bytes();
    assert_eq!(
        s.spawn_map_things().err(),
        Some(SimError::MapSpawn(MapSpawnError::UnsupportedType(3003)))
    );
    assert_eq!(s.snapshot().canonical_bytes(), before);
    let s = setup(&l, "E1M1", 1);
    assert_eq!(of_type(&s, BARON).len(), 1);
    assert_eq!(s.episode1_bounds(), Some(v2));
    let baron = ActorTypeId(BARON.ordinal());
    let ball = ActorTypeId(BALL.ordinal());
    for state in OriginalState::all() {
        let id = crate::StateId(state.ordinal());
        let n = state.ordinal();
        // `S_BOSS_STND` (527) … `S_BOSS_DIE7` (548); `S_BOSS_RAISE1`–`7` are
        // 549–555; `S_BRBALL1` (522) … `S_BRBALLX3` (526) (info.h).
        let own = (527..=548).contains(&n);
        assert_eq!(
            original_registry::state_role_for(V2, baron, id).is_ok(),
            own,
            "{n}"
        );
        assert_eq!(
            original_registry::state_role_for(V2, ball, id).is_ok(),
            (522..=526).contains(&n),
            "{n}"
        );
        assert!(original_registry::state_role_for(V1, baron, id).is_err());
        assert!(original_registry::state_role_for(V1, ball, id).is_err());
    }
    assert!(original_registry::is_policy_monster(V2, baron));
    assert!(!original_registry::is_policy_monster(V1, baron));
}

/// The DSIM40 reader refuses a Baron in a raise state (`S_BOSS_RAISE1`, info.c:685;
/// reached only through the arch-vile) and admits its live states.
#[test]
fn reader_refuses_baron_raise_states() {
    let l = wad("baron");
    let s = setup(&l, "E1M1", 1);
    let slot = of_type(&s, BARON)[0].0.slot();
    let snap = s.snapshot();
    let bytes = snap.canonical_bytes();
    crate::Diagnostic::decode_episode1_v2(&bytes, crate::DiagnosticLimits::default())
        .expect("load capture");
    let path = format!("actor.{slot}.state");
    for (state, ok) in [
        (OriginalState::S_BOSS_RUN1, true),
        (OriginalState::S_BOSS_DIE7, true),
        (OriginalState::S_BOSS_RAISE1, false),
        (OriginalState::S_BOSS_RAISE7, false),
        (OriginalState::S_BOS2_RUN1, false),
    ] {
        let edited = with_field(&snap, &path, &state.ordinal().to_le_bytes());
        assert_eq!(
            crate::Diagnostic::decode_episode1_v2(&edited, crate::DiagnosticLimits::default())
                .is_ok(),
            ok,
            "{state:?}"
        );
    }
}

/// The Baron fireball's record (info.c:1524–1548): speed 15·FRACUNIT, damage 8,
/// radius 6, height 8; the Imp's is speed 10, damage 3 (info.c:1914–1938). No
/// fireball step splits: `P_XYMovement` splits only above `MAXMOVE/2`
/// (p_mobj.c:152; 30·FRACUNIT/2 = 983,040), and the largest component is
/// `FixedMul(15·FRACUNIT, 65535)` = 983,025 (the largest `finesine` entry,
/// tables.c), strictly below.
#[test]
fn baron_fireball_record_and_no_split() {
    let d = BALL.descriptor();
    assert_eq!(d.speed, 15 * 65536);
    assert_eq!(d.damage, 8);
    assert_eq!(d.radius, 6 * 65536);
    assert_eq!(d.height, 8 * 65536);
    assert_eq!(
        OriginalActorType::MT_TROOPSHOT.descriptor().speed,
        10 * 65536
    );
    assert_eq!(OriginalActorType::MT_TROOPSHOT.descriptor().damage, 3);
    let largest = crate::FINE_SINE.iter().map(|v| v.abs()).max().unwrap();
    assert_eq!(largest, 65535);
    let step = (Fixed::from_bits(15 * 65536) * Fixed::from_bits(largest)).bits();
    assert_eq!(step, 983_025);
    assert!(step < 30 * 65536 / 2);
}

/// BARON1, C-observed: a fireball at dump tic 64 (DSIM 65). `A_BruisAttack` at
/// `S_BOSS_ATK3` does not face (no `A_FaceTarget` record in that call,
/// p_enemy.c:979–996); `P_SpawnMissile` launches `MT_BRUISERSHOT` with the Baron as
/// its target, momentum `FixedMul(15·FRACUNIT, finecosine/finesine[angle >> 19])`
/// (p_mobj.c:912–915), and its only draws besides `lastlook` are the spawn tics
/// (the player is not `MF_SHADOW`: no pair, p_mobj.c:909–910).
#[test]
fn baron_fireball_launch() {
    let commands = demo("BARON1", 1);
    let mut s = setup(&wad("baron"), "E1M1", 1);
    run_to(&mut s, &commands, 64);
    let baron = of_type(&s, BARON)[0].0;
    step(&mut s, commands[64]).unwrap();
    let mut launched = None;
    let mut draws = Vec::new();
    let mut after_launch = false;
    for e in s.events() {
        match e {
            Event::Projectile(ProjectileEvent::Launch(r)) => {
                assert_eq!(r.source, baron);
                assert_eq!(r.actor_type, ActorTypeId(BALL.ordinal()));
                launched = Some(r.source);
                after_launch = true;
            }
            Event::Monster(MonsterEvent::Face { actor, .. }) if after_launch => {
                assert_ne!(*actor, baron, "A_BruisAttack faced");
            }
            Event::Projectile(ProjectileEvent::Random { kind, .. }) if after_launch => {
                draws.push(*kind)
            }
            Event::Projectile(ProjectileEvent::Velocity {
                angle, momentum, ..
            }) if after_launch => {
                let phase = (*angle >> 19) as usize;
                let speed = Fixed::from_bits(15 * 65536);
                assert_eq!(
                    momentum[0],
                    speed * Fixed::from_bits(crate::FINE_SINE[phase + 2048])
                );
                assert_eq!(
                    momentum[1],
                    speed * Fixed::from_bits(crate::FINE_SINE[phase])
                );
            }
            _ => {}
        }
    }
    assert_eq!(launched, Some(baron));
    assert_eq!(draws.first(), Some(&0), "spawn tics draw");
    assert!(!draws.contains(&1) && !draws.contains(&2), "no shadow pair");
    let ball = of_type(&s, BALL);
    assert_eq!(ball.len(), 1);
    assert_eq!(ball[0].1.life.unwrap().target, Some(baron));
}

/// BARON1, C-observed: the claw at dump tic 505 (DSIM 506), no fireball spawned.
/// `S_StartSound (actor, sfx_claw)`, one draw, `P_DamageMobj (target, actor, actor,
/// (P_Random()%8+1)*10)` (p_enemy.c:986–991).
#[test]
fn baron_claw() {
    let commands = demo("BARON1", 1);
    let mut s = setup(&wad("baron"), "E1M1", 1);
    run_to(&mut s, &commands, 505);
    let baron = of_type(&s, BARON)[0].0;
    let player = s.original_player.unwrap().mo.unwrap();
    let balls_before = of_type(&s, BALL).len();
    step(&mut s, commands[505]).unwrap();
    let claw = crate::lifecycle::source_sound("sfx_claw").unwrap();
    let events = s.events();
    let sound = events
        .iter()
        .position(|e| {
            matches!(e, Event::ActorState(crate::ActorStateEvent::Sound { actor, sound })
                if *actor == Some(baron) && *sound == claw)
        })
        .expect("claw sound");
    let value = events[sound..]
        .iter()
        .find_map(|e| match e {
            Event::Monster(MonsterEvent::Random { actor, value, .. }) if *actor == baron => {
                Some(*value)
            }
            _ => None,
        })
        .expect("damage draw");
    let damage = events[sound..]
        .iter()
        .find_map(|e| match e {
            Event::Combat(CombatEvent::Damage(r)) => Some(*r),
            _ => None,
        })
        .expect("damage");
    assert_eq!(
        damage,
        DamageRequest {
            target: player,
            inflictor: Some(baron),
            source: Some(baron),
            amount: (i32::from(value) % 8 + 1) * 10,
        }
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::Projectile(ProjectileEvent::Launch(_))))
    );
    assert!(of_type(&s, BALL).len() <= balls_before);
}

/// BARON1, C-observed: the fireball hits the player at dump tic 119 (DSIM 120):
/// contact damage `((P_Random()%8)+1)*8` (p_map.c:323) with inflictor the ball and
/// source the Baron, then the explosion tics draw (p_mobj.c:96).
#[test]
fn baron_fireball_contact_damage() {
    let commands = demo("BARON1", 1);
    let mut s = setup(&wad("baron"), "E1M1", 1);
    run_to(&mut s, &commands, 119);
    let baron = of_type(&s, BARON)[0].0;
    let player = s.original_player.unwrap().mo.unwrap();
    step(&mut s, commands[119]).unwrap();
    let events = s.events();
    let contact = events
        .iter()
        .position(|e| {
            matches!(
                e,
                Event::Projectile(ProjectileEvent::Random { kind: 3, .. })
            )
        })
        .expect("contact draw");
    let Event::Projectile(ProjectileEvent::Random { value, .. }) = events[contact] else {
        unreachable!()
    };
    let damage = events[contact..]
        .iter()
        .find_map(|e| match e {
            Event::Combat(CombatEvent::Damage(r)) => Some(*r),
            _ => None,
        })
        .expect("damage");
    assert_eq!(damage.target, player);
    assert_eq!(damage.source, Some(baron));
    assert!(is_type(&s, damage.inflictor.unwrap(), BALL));
    assert_eq!(damage.amount, (i32::from(value) % 8 + 1) * 8);
    assert!(events[contact..].iter().any(|e| matches!(
        e,
        Event::Projectile(ProjectileEvent::Random { kind: 4, .. })
    )));
}

/// BVB1, C-observed: a Baron fireball bursts on the other Baron at dump tic 322
/// (DSIM 323) without damage (p_map.c:299–313, "Explode, but do no damage"): an
/// explosion, no contact draw and no damage record in that tic, the ledge
/// Baron's health unchanged.
#[test]
fn baron_fireball_never_hurts_a_baron() {
    let commands = demo("BVB1", 1);
    let mut s = setup(&wad("bvb"), "E1M1", 1);
    run_to(&mut s, &commands, 322);
    let health = |s: &Simulation| {
        of_type(s, BARON)
            .iter()
            .map(|(_, a)| a.life.unwrap().health)
            .collect::<Vec<_>>()
    };
    let before = health(&s);
    step(&mut s, commands[322]).unwrap();
    assert_eq!(health(&s), before);
    let exploded: Vec<ActorId> = s
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Projectile(ProjectileEvent::Exploded(id)) => Some(*id),
            _ => None,
        })
        .collect();
    assert!(exploded.iter().any(|id| is_type(&s, *id, BALL)));
    assert!(!s.events().iter().any(|e| matches!(
        e,
        Event::Projectile(ProjectileEvent::Random { kind: 3, .. })
            | Event::Combat(CombatEvent::Damage(_))
    )));
}

/// BINF1, C-observed: a fireball aimed at the player hits the Spectre at dump tic
/// 81 (DSIM 82), which then targets the Baron (infighting: `P_DamageMobj` retargets
/// a surviving victim with threshold 0 to the source, p_inter.c:904–915).
#[test]
fn baron_fireball_hits_a_spectre_which_retargets() {
    let commands = demo("BINF1", 1);
    let mut s = setup(&wad("infight"), "E1M1", 1);
    run_to(&mut s, &commands, 81);
    let baron = of_type(&s, BARON)[0].0;
    let spectre = of_type(&s, OriginalActorType::MT_SHADOWS)[0].0;
    step(&mut s, commands[81]).unwrap();
    let damage = s
        .events()
        .iter()
        .find_map(|e| match e {
            Event::Combat(CombatEvent::Damage(r)) => Some(*r),
            _ => None,
        })
        .expect("damage");
    assert_eq!(damage.target, spectre);
    assert_eq!(damage.source, Some(baron));
    let life = s.actor(spectre).unwrap().life.unwrap();
    assert_eq!(life.target, Some(baron));
    // `threshold = BASETHRESHOLD` (100, p_local.h:61); the Spectre's own A_Chase
    // later in the tic may count it down once (p_enemy.c:682–690).
    assert!((99..=100).contains(&life.threshold), "{}", life.threshold);
}

/// `A_Chase` (p_enemy.c:725–749): the melee branch needs no `movecount == 0` and
/// sets no `MF_JUSTATTACKED`; the missile branch needs `movecount == 0` and sets
/// it. Over the three Baron scripts every Baron selection is checked against the
/// Baron's `movecount` at the end of the previous tic (A_Chase does not change it
/// before the branches) and the flag record of the same call.
#[test]
fn chase_melee_and_missile_branches_differ() {
    let mut kinds = [0; 2];
    for (script, name) in [("BARON1", "baron"), ("BINF1", "infight"), ("BVB1", "bvb")] {
        let commands = demo(script, 1);
        let mut s = setup(&wad(name), "E1M1", 1);
        for c in commands {
            let before: Vec<(ActorId, i32)> = of_type(&s, BARON)
                .iter()
                .map(|(id, a)| (*id, a.life.unwrap().movecount))
                .collect();
            step(&mut s, c).unwrap();
            let events = s.events();
            for (i, e) in events.iter().enumerate() {
                let Event::Monster(MonsterEvent::Selection { actor, kind, .. }) = e else {
                    continue;
                };
                let Some(&(_, movecount)) = before.iter().find(|(id, _)| id == actor) else {
                    continue;
                };
                let flagged = events[i + 1..].iter().any(|e| {
                    matches!(e,
                        Event::Monster(MonsterEvent::JustAttacked { actor: a, .. }) if a == actor)
                });
                kinds[usize::from(*kind)] += 1;
                if *kind == 1 {
                    assert_eq!(movecount, 0, "{script}");
                    assert!(flagged, "{script}");
                } else {
                    assert!(!flagged, "{script}");
                }
            }
        }
    }
    assert!(kinds[0] > 0 && kinds[1] > 0, "{kinds:?}");
}

/// `A_BossDeath`'s checks in order (p_enemy.c:1629–1705): the map first, then the
/// live player, then the thinker walk, which runs only when both pass.
#[test]
fn boss_death_checks_in_order() {
    let never = || -> Result<bool, SimError> { panic!("walk ran") };
    assert_eq!(decide(1, 100, never).unwrap(), Outcome::NotBossMap);
    assert_eq!(decide(1, 0, never).unwrap(), Outcome::NotBossMap);
    assert_eq!(decide(9, 100, never).unwrap(), Outcome::NotBossMap);
    assert_eq!(decide(8, 0, never).unwrap(), Outcome::NoLivePlayer);
    assert_eq!(decide(8, -5, never).unwrap(), Outcome::NoLivePlayer);
    assert_eq!(decide(8, 1, || Ok(true)).unwrap(), Outcome::OtherBossAlive);
    assert_eq!(decide(8, 1, || Ok(false)).unwrap(), Outcome::FloorCalled);
    assert!(decide(8, 1, || Err(StoreError::UnsupportedThinker.into())).is_err());
}

/// BOSS1 and BOSSM1, C-observed: both Barons enter DIE7 at dump tics 112 and 223
/// (DSIM 113 and 224; 48 tics after the telefrags at 64 and 175, §7 finding 6).
/// On E1M8 the first finds the other alive and the second starts the tag-666
/// `lowerFloorToLowest`: one floor (kind 7), the call record naming no line
/// (65535), function 4, argument 1, result 1. In the E1M1 slot nothing happens.
#[test]
fn boss_death_on_e1m8_and_elsewhere() {
    let boss = boss_run(&wad("boss8"), "E1M8", 8, "BOSS1");
    assert_eq!(
        boss.iter().map(|(t, o, _)| (*t, *o)).collect::<Vec<_>>(),
        [(113, 2), (224, 3)]
    );
    assert!(boss[0].2.is_empty());
    let calls = &boss[1].2;
    assert!(matches!(
        calls[..],
        [
            MoverEvent::Started { kind: 7, .. },
            MoverEvent::Call {
                function: crate::planes::MoverFunction::DoFloor,
                line: u16::MAX,
                argument: 1,
                result: 1,
                created: 1,
            }
        ]
    ));
    let other = boss_run(&wad("bossm1"), "E1M1", 1, "BOSSM1");
    assert_eq!(
        other.iter().map(|(t, o, _)| (*t, *o)).collect::<Vec<_>>(),
        [(113, 0), (224, 0)]
    );
    assert!(other.iter().all(|(_, _, c)| c.is_empty()));
    // BARON1 (death at dump tic 688, DIE7 at 736) and TFRAG1 (DIE7 at 271), E1M1.
    let baron = boss_run(&wad("baron"), "E1M1", 1, "BARON1");
    assert_eq!(
        baron.iter().map(|(t, o, _)| (*t, *o)).collect::<Vec<_>>(),
        [(737, 0)]
    );
    let tfrag = boss_run(&wad("tfrag"), "E1M1", 1, "TFRAG1");
    assert_eq!(
        tfrag.iter().map(|(t, o, _)| (*t, *o)).collect::<Vec<_>>(),
        [(272, 0)]
    );
}

/// BOSS2, C-observed: the W1 raiseFloor on tag 666 runs when the second Baron's
/// DIE7 comes, so `EV_DoFloor` skips the busy sector (p_floor.c:277–278): the call
/// is made and creates nothing (result 0).
#[test]
fn boss_death_skips_a_busy_sector() {
    let boss = boss_run(&wad("boss8"), "E1M8", 8, "BOSS2");
    assert_eq!(
        boss.iter().map(|(t, o, _)| (*t, *o)).collect::<Vec<_>>(),
        [(113, 2), (224, 3)]
    );
    assert!(matches!(
        boss[1].2[..],
        [MoverEvent::Call {
            line: u16::MAX,
            result: 0,
            created: 0,
            ..
        }]
    ));
}

/// BOSS1 with the tag-666 sector's tag cleared: no sector carries tag 666, so
/// `P_FindSectorFromLineTag` finds none and the call creates nothing
/// (p_spec.c:434–445, p_floor.c:270–283). Rust-only edit of the C-run WAD.
#[test]
fn boss_death_without_a_tagged_sector() {
    let mut l = wad("boss8");
    let sectors = lump(&mut l, "SECTORS");
    let mut cleared = 0;
    for r in sectors.chunks_exact_mut(26) {
        if i16::from_le_bytes([r[24], r[25]]) == 666 {
            r[24..26].copy_from_slice(&0_i16.to_le_bytes());
            cleared += 1;
        }
    }
    assert_eq!(cleared, 1);
    let boss = boss_run(&l, "E1M8", 8, "BOSS1");
    assert_eq!(boss.last().map(|(t, o, _)| (*t, *o)), Some((224, 3)));
    assert!(matches!(
        boss.last().unwrap().2[..],
        [MoverEvent::Call {
            result: 0,
            created: 0,
            ..
        }]
    ));
}

/// The ±1,024 sector-height bound (decision I6) on the boss floor's path. The
/// floor's destination is `P_FindLowestFloorSurrounding`, the least current floor
/// of the sector and its neighbours (p_floor.c:299–305; p_spec.c:270–288), and it
/// moves down by `FLOORSPEED` (one unit, p_spec.h:600) per step, the last step
/// landing on the destination exactly (p_floor.c:68–84). Every height it writes
/// therefore lies between its start and a floor that already exists, so the only
/// way to give it an out-of-bound destination is a static floor outside ±1,024,
/// which the format envelope refuses at load (`map_extent::validate_real_map`).
/// Rust-only edit of the C-run BOSS1 WAD: the `END` sector (special 11), the
/// tag-666 `BLOCK` step's lowest neighbour, at −1,024 and at −1,025. At −1,024
/// the floor called at DSIM tic 224 descends one unit per tic from 64 to −1,024,
/// unrefused, reaching it at tic 224 + 1,087 and finishing with the next step; at
/// −1,025 the map is refused before any state exists.
#[test]
fn the_boss_floor_stays_inside_the_height_bound() {
    let edited = |floor: i16| {
        let mut l = wad("boss8");
        let sectors = lump(&mut l, "SECTORS");
        let mut set = 0;
        for r in sectors.chunks_exact_mut(26) {
            if i16::from_le_bytes([r[22], r[23]]) == 11 {
                r[0..2].copy_from_slice(&floor.to_le_bytes());
                set += 1;
            }
        }
        assert_eq!(set, 1);
        l
    };
    assert_eq!(
        Simulation::original_player_episode1_v2_map(
            map_of(&edited(-1025), "E1M8"),
            8,
            Skill::Medium,
            MapContextOptions::default(),
        )
        .err(),
        Some(OriginalRegistryError::Runtime(
            crate::GeometryError::CoordinateOverflow.into()
        ))
    );
    let l = edited(-1024);
    let block = {
        let map = map_of(&l, "E1M8");
        let tagged: Vec<usize> = crate::mover_rules::tagged(&map, 666).collect();
        assert_eq!(tagged.len(), 1);
        tagged[0]
    };
    let commands = demo("BOSS1", 8);
    let mut s = setup(&l, "E1M8", 8);
    run_to(&mut s, &commands, 223);
    assert_eq!(s.sectors[block].floor, Fixed::from_map_units(64));
    step(&mut s, commands[223]).unwrap();
    assert_eq!(boss_events(&s).last().map(|e| e.1), Some(3));
    let mut floor = 63;
    assert_eq!(s.sectors[block].floor, Fixed::from_map_units(floor));
    while floor > -1024 {
        step(&mut s, TicCommand::default()).unwrap();
        floor -= 1;
        assert_eq!(s.sectors[block].floor, Fixed::from_map_units(floor));
    }
    assert!(s.events().iter().all(|e| !matches!(
        e,
        Event::Mover(MoverEvent::PlaneStep {
            result: crate::planes::MoveResult::PastDest,
            ..
        })
    )));
    step(&mut s, TicCommand::default()).unwrap();
    assert_eq!(s.sectors[block].floor, Fixed::from_map_units(-1024));
    assert!(s.events().iter().any(|e| matches!(
        e,
        Event::Mover(MoverEvent::PlaneStep {
            result: crate::planes::MoveResult::PastDest,
            ..
        })
    )));
}

/// BOSS1, C-observed: the exit at dump tic 544 completes (DSIM 545) through sector
/// special 11 and every later tic is refused with `LevelExited`, state preserved
/// (as in M4a).
#[test]
fn e1m8_ends_through_sector_11() {
    let commands = demo("BOSS1", 8);
    let mut s = setup(&wad("boss8"), "E1M8", 8);
    run_to(&mut s, &commands, 545);
    let before = s.snapshot().canonical_bytes();
    assert_eq!(step(&mut s, commands[545]), Err(SimError::LevelExited));
    assert_eq!(s.snapshot().canonical_bytes(), before);
}

/// The store-full witness on the Baron's fireball: BARON1's launch tic (DSIM 65)
/// with no free slot is refused with `StoreError::Capacity` before the launch
/// changes anything, the canonical state is unchanged, and a retry refuses again;
/// with one free slot the tic runs.
#[test]
fn store_full_at_the_fireball_refuses_the_tic() {
    let commands = demo("BARON1", 1);
    let mut base = setup(&wad("baron"), "E1M1", 1);
    run_to(&mut base, &commands, 64);
    let fill = |s: &mut Simulation, free: usize| {
        let registry = s.registry.clone().unwrap();
        let fog = ActorTypeId(OriginalActorType::MT_TFOG.ordinal());
        let position = s
            .actor(s.original_player.unwrap().mo.unwrap())
            .unwrap()
            .position();
        let live = s.actors.order().count();
        for _ in 0..(crate::player_map_scale::EPISODE1_ACTOR_SLOTS as usize - live - free) {
            let actor = crate::lifecycle::make_actor(
                &s.world,
                crate::staging::Sectors::committed(&s.sectors),
                &registry,
                s.actor_options,
                fog,
                position,
                crate::SpawnHeight::Floor,
            );
            let id = s.actors.spawn(actor).unwrap();
            s.spatial.link(id);
        }
    };
    let mut full = base.clone();
    fill(&mut full, 0);
    let before = full.snapshot().canonical_bytes();
    for _ in 0..2 {
        assert_eq!(
            step(&mut full, commands[64]),
            Err(StoreError::Capacity.into())
        );
        assert_eq!(full.snapshot().canonical_bytes(), before);
    }
    let mut room = base.clone();
    fill(&mut room, 1);
    step(&mut room, commands[64]).unwrap();
    assert_eq!(of_type(&room, BALL).len(), 1);
}

/// `launch_missile` refuses any type but the Imp's and, under wire 24, the
/// Baron's, and `A_BruisAttack`'s callback is wire 24's alone.
#[test]
fn baron_paths_are_wire_24_only() {
    assert!(V2.has_baron() && V2.has_boss_death());
    for p in [V1, P::PlayerMonstersLevelV1, P::PlayerMonstersImpV1] {
        assert!(!p.has_baron() && !p.has_boss_death(), "{p:?}");
        assert!(original_registry::require_role_for(p, ActorTypeId(BARON.ordinal())).is_err());
        assert!(original_registry::require_role_for(p, ActorTypeId(BALL.ordinal())).is_err());
    }
    let _ = OriginalRegistryError::Graph;
}
/// A contact during `P_CheckMissileSpawn`'s half step (p_mobj.c:868–881; M4b note
/// §3 fact 8, correction 2): the contact draws the damage (p_map.c:323), runs
/// `P_DamageMobj` (with its pain draw for a surviving victim, p_inter.c:894) and
/// fails the move, so `P_ExplodeMissile` draws as well (p_mobj.c:96), in that order,
/// all in the launch tic. Rust-only setup: BINF1 just before the Baron's first
/// fireball, with the Spectre moved 30 units in front of the Baron on the aim line
/// (the player stays out of melee range, so `A_BruisAttack` fires).
#[test]
fn spawn_step_hit_draws_damage_pain_and_explosion() {
    let commands = demo("BINF1", 1);
    let mut s = setup(&wad("infight"), "E1M1", 1);
    let mut tic = 0;
    loop {
        let mut probe = s.clone();
        step(&mut probe, commands[tic]).unwrap();
        if probe
            .events()
            .iter()
            .any(|e| matches!(e, Event::Projectile(ProjectileEvent::Launch(_))))
        {
            break;
        }
        s = probe;
        tic += 1;
    }
    let (baron, b) = of_type(&s, BARON)[0];
    let (spectre, _) = of_type(&s, OriginalActorType::MT_SHADOWS)[0];
    let target = s.actor(b.life.unwrap().target.unwrap()).unwrap().position();
    let angle =
        crate::movement::point_angle([target[0] - b.position()[0], target[1] - b.position()[1]]);
    let phase = (angle >> 19) as usize;
    let thirty = Fixed::from_bits(30 * 65536);
    let position = [
        b.position()[0] + thirty * Fixed::from_bits(crate::FINE_SINE[phase + 2048]),
        b.position()[1] + thirty * Fixed::from_bits(crate::FINE_SINE[phase]),
    ];
    {
        let a = s.actors.get_mut(spectre).unwrap();
        a.spec.position = position;
        a.z = b.z;
    }
    s.spatial.link(spectre);
    step(&mut s, commands[tic]).unwrap();
    let events = s.events();
    let at = |f: &dyn Fn(&Event) -> bool| events.iter().position(f);
    let launch =
        at(&|e| matches!(e, Event::Projectile(ProjectileEvent::Launch(r)) if r.source == baron))
            .expect("launch");
    let contact = at(&|e| {
        matches!(
            e,
            Event::Projectile(ProjectileEvent::Random { kind: 3, .. })
        )
    })
    .expect("contact draw");
    let damage = at(&|e| {
        matches!(e, Event::Combat(CombatEvent::Damage(r))
        if r.target == spectre && r.source == Some(baron))
    })
    .expect("damage");
    let pain = at(&|e| matches!(e, Event::Combat(CombatEvent::Random { kind: 1, .. })))
        .expect("pain draw");
    let explode = at(&|e| {
        matches!(
            e,
            Event::Projectile(ProjectileEvent::Random { kind: 4, .. })
        )
    })
    .expect("explosion draw");
    assert!(launch < contact && contact < damage && damage < pain && pain < explode);
}

// --- Mutation-survivor tests (M4b step 11, Mutants run 43543402068) -----------
//
// Expectations come from the citations at each test (linuxdoom-1.10 and the M4b
// note) and from the written contracts of the functions; none from Rust output.

type Host<'a, 'b> = crate::lifecycle::LifeHost<'a, &'b mut crate::ThinkerStore<crate::Actor>>;

fn two_barons() -> (Simulation, ActorId, ActorId) {
    let s = setup(&wad("boss8"), "E1M8", 8);
    let barons = of_type(&s, BARON);
    assert_eq!(barons.len(), 2, "BOSS1's map places two Barons");
    let (a, b) = (barons[0].0, barons[1].0);
    (s, a, b)
}

/// Pins `other_alive` (boss.rs): `A_BossDeath`'s thinker walk counts another mobj of
/// the Baron's type only with `health > 0` (p_enemy.c:1692–1705: `if (mo2 != mo &&
/// mo2->type == mo->type && mo2->health > 0) return`). A Baron at exactly 0 health
/// (a thinker still listed, e.g. one killed by 1,000 damage) is not alive; 1 is.
/// Below the public API because a whole-tic run reaches the walk only with the
/// other Baron's health far from the boundary (−9,980 after a telefrag, or
/// 1,000).
#[test]
fn another_baron_counts_only_with_positive_health() {
    let (s, a, b) = two_barons();
    let ty = ActorTypeId(BARON.ordinal());
    let alive_with = |health: i32| {
        let mut t = s.clone();
        t.checked_demon_operation(|host| {
            host.access
                .get_mut(b)
                .unwrap()
                .life
                .as_mut()
                .unwrap()
                .health = health;
            other_alive(host, a, ty)
        })
        .unwrap()
        .unwrap()
    };
    assert!(alive_with(1000));
    assert!(alive_with(1));
    assert!(!alive_with(0));
    assert!(!alive_with(-1));
    assert!(!alive_with(-9_980));
}

/// Pins `boss_death`'s latching (boss.rs): a refusal of `A_BossDeath` is latched
/// for the whole-tic rollback unless a failure is already latched, which stays (the
/// first refusal of a tic is the one reported, as for every action callback). Here
/// the action is run on a non-Baron actor, which `boss_death_checked` refuses
/// (`PlayerBodyError::Unsupported`, "only `S_BOSS_DIE7` names the action"). Below the
/// public API because the state machine only runs the action on a Baron.
#[test]
fn boss_death_latches_its_first_refusal() {
    let (s, _, _) = two_barons();
    let mo = s.original_player.unwrap().mo.unwrap();
    let refused = |planted: Option<SimError>| {
        s.clone().checked_demon_operation(|host| {
            if let Some(error) = planted {
                host.fail_sight(error);
            }
            boss_death(host, mo);
        })
    };
    assert_eq!(refused(None), Err(PlayerBodyError::Unsupported.into()));
    let first: SimError = crate::CombatError::Reference.into();
    assert_eq!(refused(Some(first)), Err(first));
}

/// Pins the state closure admitted for wire 24's two Baron callbacks
/// (`validate_state_with`, the clause "each at the one state that names it"):
/// `A_BruisAttack` is admitted at `S_BOSS_ATK3` only, not at the Hell Knight's
/// `S_BOS2_ATK3`, and `A_BossDeath` at `S_BOSS_DIE7` only, not at the other bosses'
/// `S_FATT_DIE10`, `S_SPID_DIE11`, `S_BSPI_DIE7` and `S_CYBER_DIE10` (info.c:675,
/// 684, 704, 533, 767, 795, 836: the same callbacks sit at those states).
#[test]
fn baron_callbacks_are_admitted_at_their_own_states_only() {
    let s = setup(&wad("baron"), "E1M1", 1);
    let registry = s.registry.as_deref().unwrap();
    let admitted = |name: &str| {
        let state = OriginalState::from_name(name).expect(name);
        registry
            .validate_state_original_monsters(crate::StateId(state.ordinal()))
            .is_ok()
    };
    assert!(admitted("S_BOSS_ATK3"));
    assert!(admitted("S_BOSS_DIE7"));
    for other in [
        "S_BOS2_ATK3",
        "S_FATT_DIE10",
        "S_SPID_DIE11",
        "S_BSPI_DIE7",
        "S_CYBER_DIE10",
    ] {
        assert!(!admitted(other), "{other}");
    }
}

/// Pins `launch_missile`'s type gate (projectiles.rs): the Imp fireball under every
/// policy, the Baron's `MT_BRUISERSHOT` under wire 24 only (M4b note §3 fact 7:
/// `P_SpawnMissile` for `A_BruisAttack`); every other type, and the Baron's under
/// wire 23, is `ProjectileError::UnsupportedType`. Below the public API because the
/// state machine launches only the type its own action names.
#[test]
fn launch_missile_admits_the_baron_ball_under_wire_24_only() {
    let unsupported = Err(crate::ProjectileError::UnsupportedType.into());
    let ball = ActorTypeId(BALL.ordinal());
    let launch = |s: &Simulation, ty: ActorTypeId| {
        let mo = s.original_player.unwrap().mo.unwrap();
        let source = of_type(s, BARON).first().map_or(mo, |(id, _)| *id);
        s.clone()
            .checked_demon_operation(|host: &mut Host<'_, '_>| {
                crate::projectiles::launch_missile(host, source, mo, ty)
            })
            .unwrap()
    };
    let s = setup(&wad("baron"), "E1M1", 1);
    assert_eq!(launch(&s, ball), Ok(()));
    // The rocket (the player's missile) and the Baron ball's explosion-free kin are
    // not launched by a monster.
    assert_eq!(
        launch(&s, ActorTypeId(OriginalActorType::MT_ROCKET.ordinal())),
        unsupported
    );
    let mut room = crate::player_movers_tests::room(13);
    room.add_thing([1000, 64, 180, 1, 7]);
    let mut wire23 =
        Simulation::original_player_episode1_map(room.map(), MapContextOptions::default()).unwrap();
    wire23.spawn_map_things().unwrap();
    wire23
        .initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    wire23.spawn_level_specials().unwrap();
    assert_eq!(launch(&wire23, ball), unsupported);
}

/// Pins `ActorAccess::live_order` of the staged store (lifecycle.rs): the actors
/// that are not pending removal, in thinker order (`thinkercap.next` onward, with a
/// removed thinker's function −1 skipping it: p_tick.c:80–84), which is what
/// `EV_Teleport`'s landing search (p_telept.c:74–97) and `A_BossDeath`'s walk
/// (p_enemy.c:1692–1705) iterate. Below the public API because a whole-tic run
/// reaches the staged store's order only through those two services run from a
/// state entry, whose iteration the access type fixes.
#[test]
fn live_order_is_the_thinker_order_without_pending_removals() {
    use crate::lifecycle::ActorAccess;
    let (s, a, _) = two_barons();
    let all: Vec<ActorId> = s.actors.order().collect();
    assert!(all.len() > 3 && all.contains(&a));
    let mut t = s.clone();
    let (before, after) = t
        .checked_demon_operation(|host| {
            let before = host.access.live_order();
            assert!(host.access.mark(a).unwrap());
            (before, host.access.live_order())
        })
        .unwrap();
    assert_eq!(before, Some(all.clone()));
    let without: Vec<ActorId> = all.iter().copied().filter(|id| *id != a).collect();
    assert_eq!(after, Some(without));
}

/// Pins the default `ActorAccess::live_order`: an access with no thinker list
/// reports `None`, which `EV_Teleport`'s landing search and `A_BossDeath` refuse
/// (`StoreError::UnsupportedThinker`), rather than an empty list, which would read as
/// "no landing" and "no other Baron". The historical blast preview's projection is the
/// one such access in the crate and is private to its module, so a stand-in access
/// that overrides nothing but the required methods is used.
#[test]
fn an_access_without_a_thinker_list_has_no_live_order() {
    use crate::lifecycle::ActorAccess;
    struct Bare;
    impl ActorAccess for &mut Bare {
        fn get(&self, _: ActorId) -> Option<&crate::Actor> {
            None
        }
        fn get_mut(&mut self, _: ActorId) -> Option<&mut crate::Actor> {
            None
        }
        fn spawn(&mut self, _: crate::Actor) -> Result<ActorId, StoreError> {
            unreachable!()
        }
        fn mark(&mut self, _: ActorId) -> Result<bool, StoreError> {
            unreachable!()
        }
        fn reserve(&mut self, _: usize) -> Result<(), StoreError> {
            unreachable!()
        }
        fn live_of_type(&self, _: ActorTypeId) -> usize {
            unreachable!()
        }
    }
    let mut bare = Bare;
    let access = &mut bare;
    assert_eq!(access.live_order(), None);
}
