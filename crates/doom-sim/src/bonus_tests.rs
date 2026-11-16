use super::*;
use crate::{lifecycle::ActorAccess, movement::MotionAccess};

fn component(
    doomed: i16,
    health: i32,
    kind: u8,
    points: i32,
) -> (Simulation, bonus_evidence::Actors, ActorId) {
    // Exactly the declared C component's initial state and spawn-call order.
    let bytes = bonus_evidence::COMPONENT_MAPS
        .iter()
        .find(|(k, _)| *k == doomed)
        .unwrap()
        .1;
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s =
        Simulation::original_player_bonuses_map(map, 64, MapContextOptions::default()).unwrap();
    let ids = s.spawn_map_things().unwrap().spawned;
    let body = s
        .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    s.initialize_player_weapons(
        PlayerSlot::One,
        WeaponInventory {
            ready: Weapon::Pistol,
            owned: 3,
            ammo: [50, 0, 0, 0],
        },
    )
    .unwrap();
    s.original_player.as_mut().unwrap().health = health;
    s.original_player.as_mut().unwrap().armor = Some(PlayerArmorState {
        kind: ArmorType::try_from(kind).unwrap(),
        points,
    });
    s.actors
        .get_mut(body)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .health = health;
    (
        s,
        bonus_evidence::Actors {
            body,
            origin: Some(ids[0]),
        },
        ids[1],
    )
}

fn touch(s: &mut Simulation, a: bonus_evidence::Actors) {
    touch_one(s, a, None);
}
fn touch_one(s: &mut Simulation, a: bonus_evidence::Actors, only: Option<ActorId>) {
    // Direct Touch component, not movement: use the actual contact eligibility
    // query at x1, retain the original x0 body, as in the unticked C invocation.
    s.checked_demon_operation(|host| {
        let mut body = *host.access.get(a.body).unwrap();
        let mut attempt = movement::check(
            host.world,
            a.body,
            &body,
            [Fixed::from_map_units(1), Fixed::ZERO],
            host.spatial,
            &|id| host.access.get(id).copied(),
        );
        if let Some(item) = only {
            attempt.actors.retain(|id| *id == item);
        }
        let mut events = Vec::new();
        world_pickups::Contacts {
            access: &mut host.access,
            player: host.player.as_deref_mut().unwrap(),
            sectors: host.sectors,
            boundary: None,
        }
        .contacts(a.body, &mut body, &attempt, host.spatial, &mut events)
        .unwrap();
        *host.access.get_mut(a.body).unwrap() = body;
        host.events.extend(events.into_iter().map(|e| match e {
            movement::MotionEvent::Contact(c) => c.event(),
            _ => panic!("direct contact component"),
        }));
    })
    .unwrap();
}

fn component_observations(s: &Simulation, a: bonus_evidence::Actors, item: ActorId) -> Vec<i64> {
    let p = s.original_player(PlayerSlot::One).unwrap();
    let armor = s.original_player_armor(PlayerSlot::One).unwrap();
    let pickup = s.original_player_pickups(PlayerSlot::One).unwrap();
    let body = s.actor(a.body).unwrap();
    let l = body.life().unwrap();
    let birth = |id: Option<ActorId>| {
        id.map_or(-1, |id| {
            if Some(id) == a.origin {
                0
            } else if id == item {
                1
            } else if id == a.body {
                2
            } else {
                panic!("component identity")
            }
        })
    };
    vec![
        i64::from(p.health),
        i64::from(l.health),
        i64::from(armor.kind.wire_id()),
        i64::from(armor.points),
        i64::from(pickup.item_count),
        i64::from(s.level_statistics().unwrap().total_items),
        i64::from(pickup.bonus_count),
        pickup.message as i64,
        i64::from(s.actors.is_pending(item) == Some(true)),
        i64::from(s.rng.indices()[0]),
        i64::from(p.damage_count),
        birth(p.attacker),
        i64::from(body.momentum()[0].bits()),
        i64::from(body.momentum()[1].bits()),
        i64::from(l.state.unwrap().0),
        i64::from(l.tics),
        i64::from(l.flags),
        i64::from(l.reaction_time),
        i64::from(l.threshold),
        birth(l.target),
        i64::from(body.height().bits()),
        i64::from(p.state == PlayerBodyState::Dead),
        i64::from(p.frags),
    ]
}

#[test]
fn independent_bonus_resources_ordinary_thresholds_and_shared_damage() {
    let mut count = 0;
    for line in include_str!("../../../tests/reference/m3g11_components.csv")
        .lines()
        .filter(|l| l.starts_with("BCOMP,T,"))
    {
        let v: Vec<_> = line.split(',').collect();
        let n = |i: usize| v[i].parse::<i32>().unwrap();
        let (mut s, a, item) = component(n(3) as i16, n(4), n(5) as u8, n(6));
        touch(&mut s, a);
        if n(2) == 40 {
            // Source-only raw300 physical component exceeds the retained30-unit
            // momentum admission. Its pickup equals source componentT4; the
            // invalid thrust rejects atomically, without widening old bounds.
            let pickup_row = include_str!("../../../tests/reference/m3g11_components.csv")
                .lines()
                .find(|l| l.starts_with("BCOMP,T,4,"))
                .unwrap();
            let expected: Vec<i64> = pickup_row
                .split(',')
                .skip(8)
                .map(|n| n.parse().unwrap())
                .collect();
            let got = component_observations(&s, a, item);
            for (field, (got, expected)) in got.iter().zip(&expected).enumerate() {
                assert_eq!(got, expected, "source pickupT40 field{field}");
            }
            let before = s.snapshot();
            assert_eq!(
                s.damage_actor(DamageRequest {
                    target: a.body,
                    source: a.origin,
                    inflictor: a.origin,
                    amount: n(7)
                }),
                Err(PlayerBodyError::Unsupported.into())
            );
            assert_eq!(s.snapshot(), before);
            count += 1;
            continue;
        }
        if n(7) != 0 {
            s.rng = DoomRng::from_indices(0, 0);
            s.damage_actor(DamageRequest {
                target: a.body,
                source: a.origin,
                inflictor: a.origin,
                amount: n(7),
            })
            .unwrap_or_else(|e| panic!("{line}: {e}"));
        }
        let got = component_observations(&s, a, item);
        let expected: Vec<i64> = v[8..].iter().map(|n| n.parse().unwrap()).collect();
        for (field, (got, expected)) in got.iter().zip(&expected).enumerate() {
            assert_eq!(got, expected, "{line} field{field}");
        }
        count += 1;
    }
    assert_eq!(count, 48);
}

#[test]
fn bonus_armor_invariant_does_not_redefine_ordinary_green_armor() {
    for points in [101, 120, 199, 200] {
        let armor = PlayerArmorState {
            kind: ArmorType::Green,
            points,
        };
        assert!(armor.validate().is_err());
        assert!(
            armor
                .validate_for(OriginalExecutionProfile::PlayerDemonArmorV1)
                .is_err()
        );
        armor
            .validate_for(OriginalExecutionProfile::PlayerDemonBonusesV1)
            .unwrap();
    }
    for kind in [ArmorType::None, ArmorType::Green, ArmorType::Blue] {
        for points in [-1, 201, i32::MAX] {
            assert!(
                PlayerArmorState { kind, points }
                    .validate_for(OriginalExecutionProfile::PlayerDemonBonusesV1)
                    .is_err()
            );
        }
    }
}

fn initial_cap(s: &mut Simulation, c: &bonus_evidence::Case, a: bonus_evidence::Actors) {
    assert!(matches!(c.component, 30 | 31));
    s.original_player.as_mut().unwrap().health = 200;
    s.actors
        .get_mut(a.body)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .health = 200;
    if c.component == 31 {
        s.original_player.as_mut().unwrap().armor = Some(PlayerArmorState {
            kind: ArmorType::Green,
            points: 200,
        });
    }
}
#[test]
fn independent_counted_items_and_integrated_bonus_combat() {
    assert_ne!(bonus_evidence::verify(), 0);
}
#[test]
fn independent_capped_resources_collect_through_actual_movement() {
    assert_ne!(bonus_evidence::verify_with_setup(28..30, initial_cap), 0);
}

#[test]
fn counted_map_batch_rolls_back_after_positive_timer_rng_and_tentative_statistics() {
    let cases = bonus_evidence::cases();
    let c = &cases[7];
    for after in [1, 2, 16] {
        let mut s = Simulation::original_player_bonuses_map(
            bonus_evidence::case_map(c.map),
            64,
            MapContextOptions::default(),
        )
        .unwrap();
        let before = s.snapshot();
        let spatial = s.spatial.order.clone();
        spawning::BONUS_MAP_FAILURE_OBSERVATION.with(|o| o.set(None));
        spawning::BONUS_MAP_FAIL_AFTER.with(|f| f.set(Some(after)));
        assert_eq!(s.spawn_map_things(), Err(MapSpawnError::WorkLimit.into()));
        let (rng, total, actors, tics) = spawning::BONUS_MAP_FAILURE_OBSERVATION
            .with(|o| o.take())
            .unwrap();
        assert_eq!(rng, 2 * after as u8);
        assert_eq!(total, after as i32);
        assert_eq!(actors, after);
        assert!((1..=6).contains(&tics));
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.spatial.order, spatial);
        assert_eq!(s.actor_order().count(), 0);
        let spawned = s.spawn_map_things().unwrap();
        assert_eq!(spawned.spawned.len(), 16);
        assert_eq!(s.level_statistics().unwrap().total_items, 16);
    }
}

fn rejected_tic(s: &mut Simulation, commands: [TicCommand; 4]) {
    let before = s.snapshot();
    let spatial = s.spatial.order.clone();
    let roster = s.original_player_roster();
    let captures = snapshot::capture_count();
    assert!(s.advance(commands).is_err());
    assert_eq!(snapshot::capture_count(), captures);
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.spatial.order, spatial);
    assert_eq!(s.original_player_roster(), roster);
    assert!(s.step(commands).is_err());
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.spatial.order, spatial);
}

#[test]
fn counted_collection_failure_rolls_back_resources_messages_removal_and_credit() {
    for which in [2, 5, 11, 15] {
        let cases = bonus_evidence::cases();
        let c = &cases[which];
        let (mut s, _) = bonus_evidence::setup(c);
        // Range-valid but causally artificial, matching the historical late-error
        // technique:95+6 fails after the source effect/count/removal have run.
        s.original_player
            .as_mut()
            .unwrap()
            .pickups
            .as_mut()
            .unwrap()
            .bonus_count = 96;
        rejected_tic(&mut s, bonus_evidence::commands(c, 1));
        assert_eq!(
            s.original_player_pickups(PlayerSlot::One)
                .unwrap()
                .item_count,
            0
        );
    }
    let cases = bonus_evidence::cases();
    let c = &cases[11];
    let (mut s, _) = bonus_evidence::setup(c);
    // First contact reaches94; second reaches100 and rejects the entire tic.
    s.original_player
        .as_mut()
        .unwrap()
        .pickups
        .as_mut()
        .unwrap()
        .bonus_count = 89;
    rejected_tic(&mut s, bonus_evidence::commands(c, 1));
}

#[test]
fn retirement_before_second_counted_collection_is_part_of_the_same_transaction() {
    let cases = bonus_evidence::cases();
    let c = &cases[30];
    let (mut s, _) = bonus_evidence::setup(c);
    let item = s
        .actor_order()
        .find(|id| s.actor(*id).unwrap().life().unwrap().actor_type.0 == 45)
        .unwrap();
    s.advance(bonus_evidence::commands(c, 1)).unwrap();
    assert_eq!(s.actors.is_pending(item), Some(true));
    assert_eq!(
        s.original_player_pickups(PlayerSlot::One)
            .unwrap()
            .item_count,
        1
    );
    s.original_player
        .as_mut()
        .unwrap()
        .pickups
        .as_mut()
        .unwrap()
        .bonus_count = 96;
    rejected_tic(&mut s, bonus_evidence::commands(c, 2));
    assert_eq!(s.actors.is_pending(item), Some(true));
    assert_eq!(
        s.original_player_pickups(PlayerSlot::One)
            .unwrap()
            .item_count,
        1
    );
    s.original_player
        .as_mut()
        .unwrap()
        .pickups
        .as_mut()
        .unwrap()
        .bonus_count = 6;
    s.advance(bonus_evidence::commands(c, 2)).unwrap();
    assert!(s.actor(item).is_none());
    assert_eq!(
        s.original_player_pickups(PlayerSlot::One)
            .unwrap()
            .item_count,
        2
    );
    assert_eq!(s.level_statistics().unwrap().total_items, 2);
}

fn map_with_things(things: &[[i16; 5]]) -> doom_map::Map {
    // Test-input assembly only. Retain every synthetic geometry lump in order.
    let bytes = bonus_evidence::MAP_BYTES[0];
    let u32at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let count = u32at(4);
    let directory = u32at(8);
    let mut output = vec![0; 12];
    output[..4].copy_from_slice(b"PWAD");
    let mut entries = Vec::new();
    for i in 0..count {
        let at = directory + i * 16;
        let name = &bytes[at + 8..at + 16];
        let offset = output.len();
        if name == b"THINGS\0\0" {
            for t in things {
                for v in t {
                    output.extend(v.to_le_bytes());
                }
            }
        } else {
            output.extend_from_slice(&bytes[u32at(at)..u32at(at) + u32at(at + 4)]);
        }
        entries.extend((offset as u32).to_le_bytes());
        entries.extend(((output.len() - offset) as u32).to_le_bytes());
        entries.extend_from_slice(name);
    }
    output[4..8].copy_from_slice(&(count as u32).to_le_bytes());
    let directory = output.len() as u32;
    output[8..12].copy_from_slice(&directory.to_le_bytes());
    output.extend(entries);
    let wad = doom_wad::Wad::parse(&output, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}

#[test]
fn item_limits_and_partial_map_selection_errors_preserve_setup() {
    for things in [
        vec![[36, 0, 0, 2014, 7]; 17],
        vec![[-64, 0, 0, 3002, 7], [-128, 0, 0, 3002, 7]],
        vec![[36, 0, 0, 2014, 7], [72, 0, 0, 32000, 7]],
    ] {
        let mut s = Simulation::original_player_bonuses_map(
            map_with_things(&things),
            64,
            MapContextOptions::default(),
        )
        .unwrap();
        let before = s.snapshot();
        assert!(s.spawn_map_things().is_err());
        assert_eq!(s.snapshot(), before);
    }
}

#[test]
fn above100_health_and_green_armor_damage_failures_restore_all_state() {
    for (health, points, raw) in [(200, 101, 60), (151, 200, 60), (108, 1, 200), (1, 0, 12)] {
        let (mut s, a, _) = component(2015, health, if points == 0 { 0 } else { 1 }, points);
        let request = DamageRequest {
            target: a.body,
            source: a.origin,
            // The lethal200 component is nonphysical: raw thrust is bounded
            // independently of armor, as in the declared source components.
            inflictor: if raw == 200 { None } else { a.origin },
            amount: raw,
        };
        let plan = s.plan_damage(request).unwrap();
        let before = s.snapshot();
        let spatial = s.spatial.order.clone();
        let result = s.checked_demon_operation(|host| {
            host.apply_damage(plan);
            host.fail_sight(PerceptionError::WorkLimit.into());
        });
        assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.spatial.order, spatial);
    }
}

#[test]
fn source_capped_repeat_components_credit_each_actor_once() {
    for which in 0..2 {
        let kind = if which == 0 { 2014 } else { 2015 };
        let mut s = Simulation::original_player_bonuses_map(
            map_with_things(&[[36, 0, 0, kind, 7], [36, 0, 0, kind, 7]]),
            64,
            MapContextOptions::default(),
        )
        .unwrap();
        let ids = s.spawn_map_things().unwrap().spawned;
        let body = s
            .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
            .unwrap();
        s.initialize_player_weapons(
            PlayerSlot::One,
            WeaponInventory {
                ready: Weapon::Pistol,
                owned: 3,
                ammo: [50, 0, 0, 0],
            },
        )
        .unwrap();
        s.original_player.as_mut().unwrap().health = 200;
        s.actors
            .get_mut(body)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .health = 200;
        if which == 1 {
            s.original_player.as_mut().unwrap().armor = Some(PlayerArmorState {
                kind: ArmorType::Green,
                points: 200,
            });
        }
        let a = bonus_evidence::Actors { body, origin: None };
        for (j, item) in ids.iter().copied().enumerate() {
            touch_one(&mut s, a, Some(item));
            let p = s.original_player(PlayerSlot::One).unwrap();
            let armor = s.original_player_armor(PlayerSlot::One).unwrap();
            let pickup = s.original_player_pickups(PlayerSlot::One).unwrap();
            let got = [
                i64::from(p.health),
                i64::from(s.actor(body).unwrap().life().unwrap().health),
                i64::from(armor.kind.wire_id()),
                i64::from(armor.points),
                i64::from(pickup.item_count),
                i64::from(s.level_statistics().unwrap().total_items),
                i64::from(pickup.bonus_count),
                pickup.message as i64,
                i64::from(s.actors.is_pending(item) == Some(true)),
            ];
            let prefix = format!("BCOMP,C,{which},{j},");
            let line = include_str!("../../../tests/reference/m3g11_components.csv")
                .lines()
                .find(|l| l.starts_with(&prefix))
                .unwrap();
            let expected: Vec<i64> = line
                .split(',')
                .skip(4)
                .map(|n| n.parse().unwrap())
                .collect();
            assert_eq!(got.as_slice(), expected);
        }
    }
}

#[test]
fn source_depletion_then_bonus_reacquires_green_without_an_ordinary_grant() {
    let mut s = Simulation::original_player_bonuses_map(
        map_with_things(&[
            [-64, 0, 0, 3002, 7],
            [36, 0, 0, 2015, 7],
            [36, 0, 0, 2015, 7],
        ]),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    let ids = s.spawn_map_things().unwrap().spawned;
    let body = s
        .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    s.initialize_player_weapons(
        PlayerSlot::One,
        WeaponInventory {
            ready: Weapon::Pistol,
            owned: 3,
            ammo: [50, 0, 0, 0],
        },
    )
    .unwrap();
    let a = bonus_evidence::Actors {
        body,
        origin: Some(ids[0]),
    };
    for j in 0..2 {
        for phase in 0..2 {
            if phase == 0 {
                touch_one(&mut s, a, Some(ids[j + 1]));
            } else {
                s.rng = DoomRng::from_indices(0, 0);
                s.damage_actor(DamageRequest {
                    target: body,
                    source: a.origin,
                    inflictor: None,
                    amount: 12,
                })
                .unwrap();
            }
            let p = s.original_player(PlayerSlot::One).unwrap();
            let armor = s.original_player_armor(PlayerSlot::One).unwrap();
            let pickup = s.original_player_pickups(PlayerSlot::One).unwrap();
            let got = [
                i64::from(p.health),
                i64::from(s.actor(body).unwrap().life().unwrap().health),
                i64::from(armor.kind.wire_id()),
                i64::from(armor.points),
                i64::from(pickup.item_count),
                i64::from(s.level_statistics().unwrap().total_items),
                i64::from(pickup.bonus_count),
            ];
            let prefix = format!("BCOMP,R,{j},{phase},");
            let line = include_str!("../../../tests/reference/m3g11_components.csv")
                .lines()
                .find(|l| l.starts_with(&prefix))
                .unwrap();
            let expected: Vec<i64> = line
                .split(',')
                .skip(4)
                .map(|n| n.parse().unwrap())
                .collect();
            assert_eq!(got.as_slice(), expected);
        }
    }
}

#[test]
fn paused_bonus_world_does_not_animate_collect_consume_rng_or_capture_implicitly() {
    let (mut s, _) = bonus_evidence::setup(&bonus_evidence::cases()[2]);
    s.set_paused(true);
    let mut stepped = s.clone();
    let armor = s.original_player_armor(PlayerSlot::One).unwrap();
    let rng = s.rng;
    let captures = snapshot::capture_count();
    let c = bonus_evidence::commands(&bonus_evidence::cases()[2], 1);
    s.advance(c).unwrap();
    assert_eq!(snapshot::capture_count(), captures);
    assert_eq!(s.snapshot(), stepped.step(c).unwrap());
    assert_eq!(s.rng, rng);
    assert_eq!(s.original_player_armor(PlayerSlot::One).unwrap(), armor);
}
#[test]
fn bonus_item_instance_references_do_not_cross_game_instances() {
    use identity::*;
    let (s, a) = bonus_evidence::setup(&bonus_evidence::cases()[1]);
    let item = s.actor_order().find(|id| *id != a.body).unwrap();
    let mut left = GameInstance::new(s.clone());
    let mut right = GameInstance::new(s);
    let reference = left
        .with_execution_context(|ctx| {
            let item = ctx.admit_local(
                item,
                DefinitionIdentity::Original(OriginalActorType::MT_MISC2),
            )?;
            ctx.export_actor(item)
        })
        .unwrap();
    let before = right.simulation().snapshot();
    assert!(
        right
            .with_execution_context(|ctx| {
                ctx.import_actor(
                    &reference,
                    DefinitionIdentity::Original(OriginalActorType::MT_MISC2),
                )?;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(right.simulation().snapshot(), before);
}
fn replace_field(bytes: &mut [u8], name: &str, value: &[u8]) {
    let mut at = 16;
    while at < bytes.len() {
        let n = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        at += 4;
        let yes = &bytes[at..at + n] == name.as_bytes();
        at += n;
        let n = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
        at += 8;
        if yes {
            assert_eq!(n, value.len());
            bytes[at..at + n].copy_from_slice(value);
            return;
        }
        at += n;
    }
    panic!("missing test field {name}");
}
#[test]
fn bonus_diagnostic_is_explicit_and_preserves_historical_readers() {
    let (s, _) = bonus_evidence::setup(&bonus_evidence::cases()[1]);
    let bytes = s
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    assert_eq!(s.snapshot().version(), 27);
    assert!(Diagnostic::decode(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_pistol(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_weapons(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_pickups(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_armor(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_bonuses(&bytes, DiagnosticLimits::default()).is_ok());
}

#[test]
fn bonus_retirement_slot_reuse_and_no_implicit_capture() {
    let c = &bonus_evidence::cases()[19];
    let (mut s, _) = bonus_evidence::setup(c);
    let items: Vec<_> = s
        .actor_order()
        .filter(|id| s.actor(*id).unwrap().life().unwrap().actor_type.0 == 45)
        .collect();
    let mut observed_reuse = false;
    for op in 1..=c.tics {
        let captures = snapshot::capture_count();
        s.advance(bonus_evidence::commands(c, op)).unwrap();
        assert_eq!(snapshot::capture_count(), captures);
        observed_reuse |= items.iter().any(|old| {
            s.actor_order()
                .any(|id| id.slot() == old.slot() && id.generation() != old.generation())
        });
    }
    assert_eq!(
        s.original_player_pickups(PlayerSlot::One)
            .unwrap()
            .item_count,
        8
    );
    assert_eq!(s.level_statistics().unwrap().total_items, 8);
    assert!(observed_reuse);
    for item in items {
        assert!(matches!(
            s.actor_availability(item),
            ActorAvailability::StaleGeneration | ActorAvailability::Retired
        ));
        let before = s.snapshot();
        assert!(s.set_actor_target(item, None).is_err());
        assert_eq!(s.snapshot(), before);
    }
}

#[test]
fn animation_transition_then_late_damage_error_preserves_clock_and_rng() {
    let (mut s, a, item) = component(2015, 200, 1, 120);
    s.actors.get_mut(item).unwrap().life.as_mut().unwrap().tics = 1;
    let plan = s
        .plan_damage(DamageRequest {
            target: a.body,
            source: a.origin,
            inflictor: None,
            amount: 60,
        })
        .unwrap();
    let before = s.snapshot();
    let result = s.checked_demon_operation(|host| {
        host.tick(item);
        assert_eq!(
            host.access.get(item).unwrap().life().unwrap().state,
            Some(StateId(823))
        );
        host.apply_damage(plan);
        host.fail_sight(PerceptionError::WorkLimit.into());
    });
    assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
    assert_eq!(s.snapshot(), before);
}

#[test]
fn malformed_bonus_diagnostic_resources_statistics_and_events_reject() {
    for which in [28, 29] {
        let c = &bonus_evidence::cases()[which];
        let (mut s, a) = bonus_evidence::setup(c);
        initial_cap(&mut s, c, a);
        s.advance(bonus_evidence::commands(c, 1)).unwrap();
        let snap = s.snapshot();
        let bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
        Diagnostic::decode_bonuses(&bytes, DiagnosticLimits::default()).unwrap();
        for (name, value) in [
            ("profile", 10u32.to_le_bytes().to_vec()),
            ("original_player.0.health", 201i32.to_le_bytes().to_vec()),
            ("original_player.0.armor.type", vec![3]),
            (
                "original_player.0.armor.points",
                201i32.to_le_bytes().to_vec(),
            ),
            (
                "original_player.0.pickups.item_count",
                17i32.to_le_bytes().to_vec(),
            ),
            (
                "original_player.0.pickups.item_count",
                (-1i32).to_le_bytes().to_vec(),
            ),
            ("level.total_items", 17i32.to_le_bytes().to_vec()),
            ("level.total_items", 0i32.to_le_bytes().to_vec()),
            ("original_player.0.pickups.message", vec![14]),
        ] {
            let mut bad = bytes.clone();
            replace_field(&mut bad, name, &value);
            assert!(
                Diagnostic::decode_bonuses(&bad, DiagnosticLimits::default()).is_err(),
                "{name}"
            );
        }
        let mut checked = 0;
        for f in snap.fields().filter(|f| {
            f.path().starts_with("events.last.") && matches!(f.value().get(..2), Some([23, 8..=10]))
        }) {
            let mut value = f.value().to_vec();
            *value.last_mut().unwrap() = 255;
            let mut bad = bytes.clone();
            replace_field(&mut bad, f.path(), &value);
            assert!(
                Diagnostic::decode_bonuses(&bad, DiagnosticLimits::default()).is_err(),
                "{}",
                f.path()
            );
            checked += 1;
        }
        assert!(checked >= 2);
        for n in [0, 4, 16, bytes.len() - 1] {
            assert!(Diagnostic::decode_bonuses(&bytes[..n], DiagnosticLimits::default()).is_err());
        }
    }
}

/// Pins the `validate_item` call of `Contacts::contacts` (`world_pickups.rs`, the touch path):
/// an item within reach whose `health` is not 1000 must refuse the contact with
/// `PlayerBodyError::Invariant` (the error `validate_item` gave before M4p O10 made it a wrapper
/// of `validate_item_ref`) and give no pickup. A below-API guard: the tic-boundary
/// `preflight_world_pickups` refuses the same corruption first, so the public API cannot reach
/// this call with a bad item; the item is corrupted inside `checked_demon_operation`, after its
/// preflight, and the contact component is called directly, as `touch_one` does.
#[test]
fn contact_refuses_a_corrupt_item_with_the_invariant_error() {
    let (mut s, a, item) = component(2015, 100, 0, 0);
    let mut result = None;
    let outcome = s.checked_demon_operation(|host| {
        // Corrupted after the operation's own preflight, which would refuse it first.
        host.access
            .get_mut(item)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .health = 999;
        let mut body = *host.access.get(a.body).unwrap();
        let attempt = movement::check(
            host.world,
            a.body,
            &body,
            [Fixed::from_map_units(1), Fixed::ZERO],
            host.spatial,
            &|id| host.access.get(id).copied(),
        );
        assert!(attempt.actors.contains(&item), "the item is in reach");
        let mut events = Vec::new();
        result = Some(
            world_pickups::Contacts {
                access: &mut host.access,
                player: host.player.as_deref_mut().unwrap(),
                sectors: host.sectors,
                boundary: None,
            }
            .contacts(a.body, &mut body, &attempt, host.spatial, &mut events),
        );
        assert!(events.is_empty(), "no touch event before the refusal");
    });
    assert!(
        outcome.is_ok(),
        "the preflight passed the uncorrupted state"
    );
    assert_eq!(result, Some(Err(PlayerBodyError::Invariant.into())));
}
