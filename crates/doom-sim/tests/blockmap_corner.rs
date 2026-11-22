//! Linux DOOM 1.10 P_PathTraverse corner evidence and historical endpoint policy.
//! The genuine-engine observations are recorded in docs/validation/BLOCKMAP_CORNER.md.

use doom_map::{Map, MapLimits};
use doom_sim::{
    ActorSpec, ActorTypeId, AttackResult, CombatError, CombatEvent, Event, LineAttack,
    MapContextOptions, OriginalRegistryError, PlayerSlot, SimError, Simulation, Skill, SpawnHeight,
    TicCommand, TraceObject, WeaponInventory,
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
    scene_map(308)
}

fn scene_map(x: i16) -> Map {
    let mut bytes = include_bytes!("../../../tests/reference/blockmap_corner.wad").to_vec();
    let wad = Wad::parse(&bytes, Limits::default()).unwrap();
    let things = wad.lumps()[wad.find("THINGS").unwrap()].range();
    assert_eq!(things.len(), 10);
    bytes[things.start..things.start + 2].copy_from_slice(&x.to_le_bytes());
    let wad = Wad::parse(&bytes, Limits::default()).unwrap();
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

fn pistol_world(wire: u32, x: i16) -> Simulation {
    type WithLimit = fn(Map, u32, MapContextOptions) -> Result<Simulation, OriginalRegistryError>;
    type FromMap = fn(Map, MapContextOptions) -> Result<Simulation, OriginalRegistryError>;
    let limited: [WithLimit; 7] = [
        Simulation::original_player_demon_pistol_map,
        Simulation::original_player_demon_weapons_map,
        Simulation::original_player_pickups_map,
        Simulation::original_player_armor_map,
        Simulation::original_player_bonuses_map,
        Simulation::original_player_monsters_map,
        Simulation::original_player_imps_map,
    ];
    let from_map: [FromMap; 10] = [
        Simulation::original_player_map_scale_map,
        Simulation::original_player_heights_map,
        Simulation::original_player_diagonal_map,
        Simulation::original_player_real_map_map,
        Simulation::original_player_specials_map,
        Simulation::original_player_movers_map,
        Simulation::original_player_e1m1_map,
        Simulation::original_player_barrels_map,
        Simulation::original_player_level_map,
        Simulation::original_player_episode1_map,
    ];
    let map = scene_map(x);
    let context = MapContextOptions::default();
    let mut sim = match wire {
        7..=13 => limited[(wire - 7) as usize](map, 64, context),
        14..=23 => from_map[(wire - 14) as usize](map, context),
        24 => Simulation::original_player_episode1_v2_map(map, 1, Skill::Medium, context),
        25 => Simulation::original_player_episode1_v3_map(map, 1, Skill::Medium, context),
        _ => unreachable!("unexpected wire {wire}"),
    }
    .unwrap_or_else(|error| panic!("wire {wire} setup: {error:?}"));
    assert_eq!(sim.original_execution_profile().unwrap().wire_id(), wire);

    if wire >= 17 {
        let report = sim.spawn_map_things().unwrap();
        assert!(report.spawned.is_empty());
        assert_eq!(report.player_body.is_some(), wire >= 22);
    }
    if wire < 22 {
        sim.spawn_player_body(PlayerSlot::One, position(x)).unwrap();
    }
    if wire == 7 {
        sim.initialize_player_pistol(PlayerSlot::One, 50).unwrap();
    } else {
        sim.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
            .unwrap();
    }
    if wire >= 18 {
        sim.spawn_level_specials().unwrap();
    }
    sim
}

fn assert_pistol_walk(
    wire: u32,
    x: i16,
    shot: AttackResult,
    cells: &[[i32; 2]],
    lines: &[(u16, i32)],
) {
    assert_eq!(shot.trace.origin, position(x), "wire {wire}, x {x}");
    assert_eq!(
        shot.trace.delta.map(Fixed::bits),
        [77_240_320, 109_762_560],
        "wire {wire}, x {x}"
    );
    assert_eq!(shot.trace.cells, cells, "wire {wire}, x {x}");
    assert_eq!(
        shot.hit,
        Some(TraceObject::Line(lines[0].0)),
        "wire {wire}, x {x}"
    );
    assert_eq!(shot.trace.intercepts.len(), lines.len() + 1);
    assert!(matches!(
        shot.trace.intercepts[0].object,
        TraceObject::Actor(_)
    ));
    for (intercept, &(line, fraction)) in shot.trace.intercepts[1..].iter().zip(lines) {
        assert_eq!(
            intercept.object,
            TraceObject::Line(line),
            "wire {wire}, x {x}"
        );
        assert_eq!(intercept.fraction.bits(), fraction, "wire {wire}, x {x}");
    }
}

#[test]
fn source_budget_collects_and_resolves_after_64_cell_visits() {
    let mut stalled = [[3, 2]; 64];
    stalled[0] = [2, 2];
    let control = [
        [2, 2],
        [2, 3],
        [3, 3],
        [3, 4],
        [4, 4],
        [4, 5],
        [5, 5],
        [5, 6],
        [5, 7],
        [6, 7],
        [6, 8],
        [7, 8],
        [7, 9],
        [7, 10],
        [8, 10],
        [8, 11],
        [9, 11],
        [9, 12],
        [10, 12],
        [10, 13],
        [10, 14],
        [11, 14],
        [11, 15],
    ];
    for wire in 7..=25 {
        let turn = if wire < 22 { TURN } else { 7 * 256 };

        // The genuine shot on tic 34 still hits line 0 after the S3 walk stalls.
        assert_pistol_walk(
            wire,
            308,
            fire(pistol_world(wire, 308), turn),
            &stalled,
            &[(0, 19_251)],
        );

        // C10S, the same geometry and angle with the player one map unit west.
        assert_pistol_walk(
            wire,
            307,
            fire(pistol_world(wire, 307), turn),
            &control,
            &[(5, 6_573), (7, 6_886), (0, 19_251)],
        );
    }
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
    for (wire, make) in (2..=6).zip([
        Simulation::original_player_map,
        Simulation::original_player_knockback_map,
        Simulation::original_player_command_map,
        Simulation::original_player_demon_map,
        Simulation::original_player_demon_melee_map,
    ]) {
        let mut sim = make(map(), 64, context).unwrap();
        assert_eq!(sim.original_execution_profile().unwrap().wire_id(), wire);
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
        assert_eq!(sim.snapshot().canonical_bytes(), before);
        assert_eq!(
            sim.aim_line_attack(shooter, ATTACK.angle, ATTACK.range),
            Err(SimError::Combat(CombatError::Profile))
        );
        assert_eq!(sim.snapshot().canonical_bytes(), before);
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
