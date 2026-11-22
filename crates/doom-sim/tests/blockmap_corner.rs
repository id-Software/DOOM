//! Linux DOOM 1.10 P_PathTraverse corner evidence and historical endpoint policy.
//! The genuine-engine observations are recorded in docs/validation/BLOCKMAP_CORNER.md.

use doom_map::{Map, MapLimits};
use doom_sim::{
    ActorSpec, ActorTypeId, AttackResult, CombatError, CombatEvent, Event, LineAttack,
    MapContextOptions, PlayerSlot, SimError, Simulation, Skill, SpawnHeight, TicCommand,
    TraceObject, WeaponInventory,
};
use doom_types::Fixed;
use doom_wad::{Limits, Wad};

const TURN: i16 = (45 * 65536 / 360 + 7 * 256) as i16;
const ATTACK: LineAttack = LineAttack {
    angle: (TURN as u32) << 16,
    range: Fixed::from_map_units(2048),
    slope: Fixed::ZERO,
    damage: 0,
};

fn map() -> Map {
    let bytes = include_bytes!("../../../tests/reference/blockmap_corner.wad");
    let wad = Wad::parse(bytes, Limits::default()).unwrap();
    Map::from_wad(&wad, "E1M1", MapLimits::default()).unwrap()
}

fn position(x: i16) -> [Fixed; 2] {
    [Fixed::from_map_units(x), Fixed::from_map_units(276)]
}

fn commands(command: TicCommand) -> [TicCommand; 4] {
    [
        command,
        TicCommand::default(),
        TicCommand::default(),
        TicCommand::default(),
    ]
}

fn fire(mut sim: Simulation, turn: i16) -> AttackResult {
    for tic in 0..71 {
        sim.advance(commands(TicCommand {
            angle_turn: if tic == 0 { turn } else { 0 },
            buttons: u8::from(tic == 30),
            ..TicCommand::default()
        }))
        .unwrap();

        if let Some(result) = sim.events().iter().find_map(|event| match event {
            Event::Combat(CombatEvent::Trace { result }) => Some(result),
            _ => None,
        }) {
            assert_eq!(tic, 34);
            return (**result).clone();
        }
    }
    panic!("pistol trace never ran");
}

fn wire_seven_shot(x: i16) -> AttackResult {
    let mut sim =
        Simulation::original_player_demon_pistol_map(map(), 64, MapContextOptions::default())
            .unwrap();
    sim.spawn_player_body(PlayerSlot::One, position(x)).unwrap();
    sim.initialize_player_pistol(PlayerSlot::One, 50).unwrap();
    fire(sim, TURN)
}

#[test]
fn source_budget_collects_and_resolves_after_64_cell_visits() {
    let mut sim = Simulation::original_player_episode1_v3_map(
        map(),
        1,
        Skill::Medium,
        MapContextOptions::default(),
    )
    .unwrap();
    sim.spawn_map_things().unwrap();
    sim.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    sim.spawn_level_specials().unwrap();

    // S3, genuine P_PathTraverse shot on tic 34. The initial cell takes one visit
    // and the stalled side cell takes the remaining 63. Line 0 still produces a hit.
    for shot in [wire_seven_shot(308), fire(sim, 7 * 256)] {
        assert_eq!(shot.trace.cells.len(), 64);
        assert_eq!(shot.trace.cells[0], [2, 2]);
        assert!(shot.trace.cells[1..].iter().all(|&cell| cell == [3, 2]));
        assert_eq!(shot.hit, Some(TraceObject::Line(0)));
        assert_eq!(shot.trace.intercepts.len(), 2);
        assert_eq!(shot.trace.intercepts[1].fraction.bits(), 19_251);
    }

    // C10S, the same geometry and angle with the player one map unit west.
    let control = wire_seven_shot(307);
    assert_eq!(control.trace.cells.len(), 23);
    assert_eq!(control.trace.cells.last(), Some(&[11, 15]));
    assert_eq!(control.hit, Some(TraceObject::Line(5)));
    assert_eq!(control.trace.intercepts.len(), 4);
    assert_eq!(control.trace.intercepts[1].fraction.bits(), 6_573);
}

#[test]
fn historical_endpoint_refusal_preserves_canonical_state() {
    for make in [Simulation::combat, Simulation::player_weapons] {
        let mut sim = make(map()).unwrap();
        let shooter = sim
            .spawn_type(ActorTypeId(0), position(308), SpawnHeight::Floor)
            .unwrap();
        let before = sim.snapshot().canonical_bytes().to_vec();
        let refusal = SimError::Combat(CombatError::TraversalLimit);
        assert_eq!(
            sim.aim_line_attack(shooter, ATTACK.angle, ATTACK.range),
            Err(refusal)
        );
        assert_eq!(sim.snapshot().canonical_bytes(), before);
        assert_eq!(sim.line_attack(shooter, ATTACK), Err(refusal));
        assert_eq!(sim.snapshot().canonical_bytes(), before);

        let control = sim
            .spawn_type(ActorTypeId(0), position(307), SpawnHeight::Floor)
            .unwrap();
        let result = sim
            .aim_line_attack(control, ATTACK.angle, ATTACK.range)
            .unwrap();
        assert_eq!(result.trace.cells.last(), Some(&[11, 15]));
    }
}

#[test]
fn original_wires_before_pistol_refuse_attack_commands_and_explicit_shots() {
    let context = MapContextOptions::default();
    for make in [
        Simulation::original_player_map,
        Simulation::original_player_knockback_map,
        Simulation::original_player_command_map,
        Simulation::original_player_demon_map,
        Simulation::original_player_demon_melee_map,
    ] {
        let mut sim = make(map(), 64, context).unwrap();
        let shooter = sim
            .spawn_player_body(PlayerSlot::One, position(308))
            .unwrap();
        let before = sim.snapshot().canonical_bytes().to_vec();
        assert_eq!(
            sim.advance(commands(TicCommand {
                buttons: 1,
                ..TicCommand::default()
            })),
            Err(SimError::UnsupportedButtons)
        );
        assert_eq!(
            sim.aim_line_attack(shooter, ATTACK.angle, ATTACK.range),
            Err(SimError::Combat(CombatError::Profile))
        );
        assert_eq!(
            sim.line_attack(shooter, ATTACK),
            Err(SimError::Combat(CombatError::Profile))
        );
        assert_eq!(sim.snapshot().canonical_bytes(), before);
    }
}

#[test]
fn historical_profiles_before_combat_refuse_shot_queries() {
    let constructors: [fn(Map) -> Result<Simulation, SimError>; 3] = [
        Simulation::player_collision,
        Simulation::multisector,
        Simulation::actor_states,
    ];
    for (index, make) in constructors.into_iter().enumerate() {
        let mut sim = make(map()).unwrap();
        let shooter = if index == 2 {
            sim.spawn_type(ActorTypeId(0), position(308), SpawnHeight::Floor)
        } else {
            sim.spawn(ActorSpec {
                position: position(308),
                ..ActorSpec::default()
            })
        }
        .unwrap();
        let before = sim.snapshot().canonical_bytes().to_vec();
        let refusal = SimError::Combat(CombatError::Profile);
        assert_eq!(
            sim.aim_line_attack(shooter, ATTACK.angle, ATTACK.range),
            Err(refusal)
        );
        assert_eq!(sim.line_attack(shooter, ATTACK), Err(refusal));
        assert_eq!(sim.snapshot().canonical_bytes(), before);
    }
}
