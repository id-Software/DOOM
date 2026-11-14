//! M4b wire 24 (`PlayerMonstersEpisode1V2`, DSIM40), plan step 3: its identity
//! and predicates, the constructor's map number and skill, the load refusals of
//! what later steps admit (special 97, doomednums 14 and 3003, skill 3), the
//! DSIM40 writer and reader, and that wire 24 runs every wire-23 workload exactly
//! as wire 23 does, capture for capture, outside the four DSIM40 differences
//! (version word, `profile`, `context.schema`, `level.map`; STATE_FORMAT "DSIM
//! v40" compatibility rule). Rust-only policy tests: the C comparison of the M4a
//! synthetic scripts under wire 24 is tools/reference/check_player_episode1v2.py.
use crate::barrels_replay::{cases, room};
use crate::diagonal_room::Lumps;
use crate::movers::PlatType;
use crate::*;

const V1: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V1;
const V2: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V2;

/// Every policy, in wire order.
pub(crate) const ALL: [OriginalExecutionProfile; 24] = {
    use OriginalExecutionProfile as P;
    [
        P::DemonNormalV1,
        P::PlayerBodyLifecycleV1,
        P::PlayerBodyKnockbackV1,
        P::PlayerCommandMovementV1,
        P::PlayerDemonAcquisitionV1,
        P::PlayerDemonMeleeV1,
        P::PlayerDemonPistolV1,
        P::PlayerDemonWeaponsV1,
        P::PlayerDemonPickupsV1,
        P::PlayerDemonArmorV1,
        P::PlayerDemonBonusesV1,
        P::PlayerMonstersV1,
        P::PlayerMonstersImpV1,
        P::PlayerMonstersMapScaleV1,
        P::PlayerMonstersHeightsV1,
        P::PlayerMonstersDiagonalV1,
        P::PlayerMonstersRealMapV1,
        P::PlayerMonstersSpecialsV1,
        P::PlayerMonstersMoversV1,
        P::PlayerMonstersE1M1V1,
        P::PlayerMonstersBarrelsV1,
        P::PlayerMonstersLevelV1,
        P::PlayerMonstersEpisode1V1,
        P::PlayerMonstersEpisode1V2,
    ]
};

fn lump<'a>(l: &'a mut Lumps, name: &str) -> &'a mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == name).unwrap().1
}
fn record(x: i16, y: i16, angle: i16, kind: i16, options: i16) -> Vec<u8> {
    [x, y, angle, kind, options]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
fn with_start(mut l: Lumps, at: usize, position: [i16; 2], angle: i16) -> Lumps {
    let t = lump(&mut l, "THINGS");
    let at = at.min(t.len() / 10);
    t.splice(
        at * 10..at * 10,
        record(position[0], position[1], angle, 1, 7),
    );
    l
}
fn set_line(l: &mut Lumps, line: usize, special: i16, tag: i16) {
    let v = lump(l, "LINEDEFS");
    v[14 * line + 6..14 * line + 8].copy_from_slice(&special.to_le_bytes());
    v[14 * line + 8..14 * line + 10].copy_from_slice(&tag.to_le_bytes());
}
fn set_heights(l: &mut Lumps, sector: usize, floor: i16, ceiling: i16) {
    let s = lump(l, "SECTORS");
    s[26 * sector..26 * sector + 2].copy_from_slice(&floor.to_le_bytes());
    s[26 * sector + 2..26 * sector + 4].copy_from_slice(&ceiling.to_le_bytes());
}
fn set_floor_flat(l: &mut Lumps, sector: usize, name: &[u8; 8]) {
    lump(l, "SECTORS")[26 * sector + 4..26 * sector + 12].copy_from_slice(name);
}
fn set_sector_special(l: &mut Lumps, sector: usize, special: i16, tag: i16) {
    let s = lump(l, "SECTORS");
    s[26 * sector + 22..26 * sector + 24].copy_from_slice(&special.to_le_bytes());
    s[26 * sector + 24..26 * sector + 26].copy_from_slice(&tag.to_le_bytes());
}
fn set_front_texture(l: &mut Lumps, line: usize, name: &[u8; 8]) {
    let v = lump(l, "LINEDEFS");
    let front = usize::from(u16::from_le_bytes([v[14 * line + 10], v[14 * line + 11]]));
    lump(l, "SIDEDEFS")[30 * front + 20..30 * front + 28].copy_from_slice(name);
}
fn options(seed: u8) -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(seed, 0),
        respawn_monsters: false,
    }
}
fn load(
    l: &Lumps,
    policy: OriginalExecutionProfile,
    map_number: u8,
    seed: u8,
) -> Result<Simulation, SimError> {
    load_at(l, policy, map_number, seed, Skill::Medium)
}
/// [`load`] with the wire-24 skill given (wire 23 has none and ignores it).
fn load_at(
    l: &Lumps,
    policy: OriginalExecutionProfile,
    map_number: u8,
    seed: u8,
    skill: Skill,
) -> Result<Simulation, SimError> {
    let result = if policy == V2 {
        Simulation::original_player_episode1_v2_map(l.map(), map_number, skill, options(seed))
    } else {
        Simulation::original_player_episode1_map(l.map(), options(seed))
    };
    result.map_err(|e| match e {
        OriginalRegistryError::Runtime(e) => e,
        other => panic!("{other:?}"),
    })
}
fn setup(l: &Lumps, policy: OriginalExecutionProfile, map_number: u8, seed: u8) -> Simulation {
    setup_at(l, policy, map_number, seed, Skill::Medium)
}
fn setup_at(
    l: &Lumps,
    policy: OriginalExecutionProfile,
    map_number: u8,
    seed: u8,
    skill: Skill,
) -> Simulation {
    let mut s = load_at(l, policy, map_number, seed, skill).unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
/// A field edit for re-encoded captures: `None` drops the field.
type Edit = dyn Fn(&str, Vec<u8>) -> Option<Vec<u8>>;

fn zero() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}
fn fields(s: &Simulation) -> Vec<(String, Vec<u8>)> {
    s.snapshot()
        .fields()
        .map(|f| (f.path().to_string(), f.value().to_vec()))
        .collect()
}

/// Asserts the STATE_FORMAT "DSIM v40" compatibility rule on one pair of
/// captures of the same state: the wire-24 capture is the wire-23 capture with
/// `profile` 24, the v40 context and `level.map` (`map`) inserted right after
/// `level.secret_exit`, and nothing else changed; the version words are 39 and 40.
fn v40_is_v39_plus_map(s23: &Simulation, s24: &Simulation, map: u8) {
    assert_eq!(s23.snapshot().version(), 39);
    assert_eq!(s24.snapshot().version(), 40);
    let (w23, mut w24) = (fields(s23), fields(s24));
    let at = w24.iter().position(|(p, _)| p == "level.map").unwrap();
    assert_eq!(w24[at - 1].0, "level.secret_exit");
    assert_eq!(w24.remove(at).1, [map]);
    assert_eq!(w23.len(), w24.len());
    for ((p23, v23), (p24, v24)) in w23.iter().zip(&w24) {
        assert_eq!(p23, p24);
        match p23.as_str() {
            "profile" => {
                assert_eq!(v23, &23u32.to_le_bytes());
                assert_eq!(v24, &24u32.to_le_bytes());
            }
            "context.schema" => {
                assert_eq!(v23, PLAYER_EPISODE1_CONTEXT_SCHEMA);
                assert_eq!(v24, PLAYER_EPISODE1_V2_CONTEXT_SCHEMA);
            }
            // The one field the wire-24 skill may change (M4b step 6): wire 23 is
            // always medium (2); wire 24 holds its run's skill.
            "actors.options" => {
                assert_eq!(v23, &[2, 0, 0]);
                assert_eq!(v24, &[s24.actor_options.skill as u8, 0, 0]);
            }
            _ => assert_eq!(v23, v24, "{p23}"),
        }
    }
}

/// Runs `l` under wire 23 and wire 24 (map `map`) for `tics` tics of `command`,
/// asserting the same tic results and, at every capture, the compatibility rule;
/// every 35th wire-24 capture decodes with `decode_episode1_v2`. Returns the two
/// simulations and whether a tic was refused (both refused it with the same error
/// and kept their captures).
fn twin(
    l: &Lumps,
    map: u8,
    seed: u8,
    tics: u64,
    command: impl Fn(u64) -> [TicCommand; 4],
) -> (Simulation, Simulation, Option<SimError>) {
    let limits = DiagnosticLimits::default();
    let mut a = setup(l, V1, map, seed);
    let mut b = setup(l, V2, map, seed);
    assert_eq!(a.episode1_bounds(), b.episode1_bounds());
    v40_is_v39_plus_map(&a, &b, map);
    for tic in 1..=tics {
        let (before_a, before_b) = (
            a.snapshot().canonical_bytes(),
            b.snapshot().canonical_bytes(),
        );
        let ra = a.advance(command(tic));
        let rb = b.advance(command(tic));
        assert_eq!(ra, rb, "tic {tic}");
        if let Err(e) = rb {
            assert_eq!(a.snapshot().canonical_bytes(), before_a);
            assert_eq!(b.snapshot().canonical_bytes(), before_b);
            return (a, b, Some(e));
        }
        v40_is_v39_plus_map(&a, &b, map);
        if tic % 35 == 0 {
            Diagnostic::decode_episode1_v2(&b.snapshot().canonical_bytes(), limits)
                .unwrap_or_else(|e| panic!("tic {tic}: {e:?}"));
        }
    }
    (a, b, None)
}

#[test]
fn wire_24_identity_and_predicates() {
    for (i, p) in ALL.iter().enumerate() {
        assert_eq!(p.wire_id(), i as u32 + 1, "{p:?}");
    }
    assert_eq!(V2.wire_id(), 24);
    assert_eq!(V2.context_schema(), PLAYER_EPISODE1_V2_CONTEXT_SCHEMA);
    assert_eq!(
        PLAYER_EPISODE1_V2_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-EPISODE1-2-v1"
    );
    let contexts: std::collections::BTreeSet<_> =
        ALL[1..].iter().map(|p| p.context_schema()).collect();
    assert_eq!(
        contexts.len(),
        ALL.len() - 1,
        "every player context is distinct"
    );
    // A strict superset of wire 23: every predicate wire 23 holds, wire 24 holds.
    type Predicate = fn(OriginalExecutionProfile) -> bool;
    let inherited: [(&str, Predicate); 21] = [
        ("armor", OriginalExecutionProfile::has_player_armor),
        ("bonuses", OriginalExecutionProfile::has_player_bonuses),
        ("monsters", OriginalExecutionProfile::has_monsters),
        ("imps", OriginalExecutionProfile::has_imps),
        ("map_scale", OriginalExecutionProfile::has_map_scale),
        ("heights", OriginalExecutionProfile::has_heights),
        ("diagonals", OriginalExecutionProfile::has_diagonals),
        ("real_map", OriginalExecutionProfile::has_real_map),
        ("specials", OriginalExecutionProfile::has_specials),
        ("movers", OriginalExecutionProfile::has_movers),
        ("e1m1", OriginalExecutionProfile::has_e1m1),
        ("barrels", OriginalExecutionProfile::has_barrels),
        ("level", OriginalExecutionProfile::has_level),
        ("episode1", OriginalExecutionProfile::has_episode1),
        ("world_pickups", OriginalExecutionProfile::has_world_pickups),
        ("player_body", OriginalExecutionProfile::has_player_body),
        ("physical", OriginalExecutionProfile::has_physical_player),
        ("weapons", OriginalExecutionProfile::has_player_weapons),
        ("pack", OriginalExecutionProfile::has_weapon_pack),
        ("commands", OriginalExecutionProfile::has_player_commands),
        ("level+", |p| p.has_level() && p.has_episode1()),
    ];
    for (name, f) in inherited {
        assert!(f(V1) && f(V2), "{name}");
    }
    // `has_episode1` is wires 23 and 24 only; the new predicates are wire 24 only.
    let new: [(&str, Predicate); 5] = [
        ("episode1_v2", OriginalExecutionProfile::has_episode1_v2),
        ("teleport", OriginalExecutionProfile::has_teleport),
        ("baron", OriginalExecutionProfile::has_baron),
        ("boss_death", OriginalExecutionProfile::has_boss_death),
        ("hard_skill", OriginalExecutionProfile::has_hard_skill),
    ];
    for p in ALL {
        assert_eq!(p.has_episode1(), matches!(p, V1 | V2), "{p:?}");
        for (name, f) in new {
            assert_eq!(f(p), p == V2, "{name} {p:?}");
        }
    }
    assert_eq!(player_map_scale::actor_slots(V2), EPISODE1_ACTOR_SLOTS);
    assert_eq!(player_map_scale::live_projectile_bound(V2), 1024);
    OriginalExecutionRegistry::new(V2).unwrap();
}

/// The constructor holds the map number for the run and captures it as
/// `level.map`; wire 23 holds none. Map numbers outside 1–9 and every skill but
/// medium and hard (hard admitted in M4b step 6, decision I4) are refused before
/// anything is built.
#[test]
fn constructor_map_number_and_skill() {
    let c = &cases()[0];
    let l = with_start(room(c.map), 0, c.position, 0);
    for map in EPISODE1_MAP_NUMBERS {
        let s = load(&l, V2, map, c.seed).unwrap();
        assert_eq!(s.level_map, Some(map));
        assert_eq!(
            s.snapshot()
                .fields()
                .find(|f| f.path() == "level.map")
                .unwrap()
                .value(),
            [map]
        );
        assert_eq!(s.actor_options.skill, Skill::Medium);
    }
    let old = load(&l, V1, 1, c.seed).unwrap();
    assert_eq!(old.level_map, None);
    assert!(old.snapshot().fields().all(|f| f.path() != "level.map"));
    let unsupported = SimError::PlayerBody(PlayerBodyError::Unsupported);
    for map in [0, 10, 255] {
        assert_eq!(load(&l, V2, map, c.seed).err(), Some(unsupported));
    }
    for skill in [Skill::Baby, Skill::Easy, Skill::Nightmare] {
        assert_eq!(
            Simulation::original_player_episode1_v2_map(l.map(), 1, skill, options(c.seed)).err(),
            Some(OriginalRegistryError::Runtime(unsupported)),
            "{skill:?}"
        );
    }
    let hard =
        Simulation::original_player_episode1_v2_map(l.map(), 1, Skill::Hard, options(c.seed))
            .unwrap();
    assert_eq!(hard.actor_options.skill, Skill::Hard);
}

/// What M4b admits under wire 24 stays refused at load under wire 23: line
/// special 97, the landing (14) and the Baron of Hell (3003, by the THING batch,
/// which changes nothing when it refuses), each with its unchanged error. M4b
/// step 4 admits 97 and 14 under wire 24
/// (`teleport_tests::wire_24_admits_97_and_the_landing`) and step 5 admits 3003
/// (`baron_tests::wire_24_admits_the_baron_wire_23_refuses_it`), so this test,
/// written in step 3 for both wires, keeps only its wire-23 cases: step 5 removed
/// wire 24 from the 3003 case and changed nothing else.
#[test]
fn unadmitted_specials_and_things_refuse_at_load() {
    let mut l = crate::player_movers_tests::room(13);
    l.add_thing([1000, 64, 180, 1, 7]);
    load(&l, V2, 1, 6).unwrap();
    let mut tele = l.clone();
    set_line(&mut tele, 4, 97, 5);
    assert_eq!(
        load(&tele, V1, 5, 6).err(),
        Some(SimError::Pursuit(PursuitError::Specials)),
        "{V1:?}"
    );
    for (doomednum, policies) in [(14, &[V1][..]), (3003, &[V1][..])] {
        let mut l = l.clone();
        l.add_thing([1000, 128, 0, doomednum, 7]);
        for &policy in policies {
            let mut s = load(&l, policy, 8, 6).unwrap();
            let before = s.snapshot().canonical_bytes();
            assert_eq!(
                s.spawn_map_things().err(),
                Some(SimError::MapSpawn(MapSpawnError::UnsupportedType(
                    doomednum
                ))),
                "{doomednum} {policy:?}"
            );
            assert_eq!(s.snapshot().canonical_bytes(), before);
        }
    }
}

/// Every M3h9b room with a start runs under wire 24 exactly as under wire 23
/// for 70 idle tics (wire 23 itself is checked against wire 22 by
/// `player_episode1_tests`), on two map numbers.
#[test]
fn every_m3h9b_room_runs_under_wire_24_as_under_wire_23() {
    let mut seen = std::collections::BTreeSet::new();
    for c in cases() {
        if !seen.insert(c.map) {
            continue;
        }
        let l = with_start(room(c.map), 1, c.position, 90);
        for map in [1, 9] {
            twin(&l, map, c.seed, 70, |_| zero());
        }
    }
}

/// Every committed M3h8 mover room loads under wire 24 with wire 23's mover
/// admission and runs 105 tics of use and forward commands identically.
#[test]
fn every_mover_room_runs_under_wire_24_as_under_wire_23() {
    for index in 0..crate::player_movers_tests::ROOM_COUNT {
        let mut l = crate::player_movers_tests::room(index);
        let has_start = lump(&mut l, "THINGS")
            .chunks(10)
            .any(|t| i16::from_le_bytes([t[6], t[7]]) == 1);
        if !has_start {
            continue;
        }
        let a = load(&l, V1, 3, 6).unwrap();
        let b = load(&l, V2, 3, 6).unwrap();
        assert_eq!(a.movers, b.movers, "room {index}");
        assert_eq!(a.actors.max_sector_slots, b.actors.max_sector_slots);
        twin(&l, 3, 6, 105, |tic| {
            let mut c = zero();
            c[0].forward = 25;
            c[0].angle_turn = if tic % 20 < 10 { 256 } else { -256 };
            if tic % 7 == 0 {
                c[0].buttons = weapons::BT_USE;
            }
            c
        });
    }
}

/// A refused tic under wire 24 keeps the complete canonical state, as under wire
/// 23: the blocked `raiseToNearestAndChange` rise of decision H7 (a barrel that
/// does not fit under the ceiling) is `PlatBlocked` on the same tic for both.
#[test]
fn refused_tic_keeps_the_state_under_wire_24() {
    refused_plat_keeps_the_state(Skill::Medium);
}

/// The same refusal at hard skill (M4b step 6): the skill changes only the THING
/// filter's bit (p_mobj.c:743-751; note §3 part C item 29), so the refused tic is
/// the same tic with the same error, and it keeps the canonical state. The room's
/// things are all-skill (options 7), so wire 23 at medium is the twin.
#[test]
fn refused_tic_keeps_the_state_under_wire_24_at_hard_skill() {
    refused_plat_keeps_the_state(Skill::Hard);
}

fn refused_plat_keeps_the_state(skill: Skill) {
    // Lift room 7: sector 3 (tag 1) between sectors 1 and 2; line 12 a 20 switch.
    let mut l = crate::player_movers_tests::room(7);
    l.clear_things();
    set_heights(&mut l, 3, 2, 46);
    set_floor_flat(&mut l, 2, b"FLAT20\0\0");
    set_sector_special(&mut l, 3, 5, 1);
    set_line(&mut l, 12, 20, 1);
    set_front_texture(&mut l, 12, b"SW1COMM\0");
    set_line(&mut l, 0, 0, 0);
    l.add_thing([256, 64, 90, 1, 7]);
    l.add_thing([256, 320, 0, 2035, 7]);
    let mut a = setup(&l, V1, 2, 6);
    let mut b = setup_at(&l, V2, 2, 6, skill);
    assert_eq!(b.actor_options.skill, skill);
    for s in [&mut a, &mut b] {
        assert!(
            s.start_movers(|m| m.do_plat(12, PlatType::RaiseToNearestAndChange))
                .unwrap()
        );
    }
    let mut refused = None;
    for tic in 1..=40 {
        let before = b.snapshot().canonical_bytes();
        let ra = a.advance(zero());
        let rb = b.advance(zero());
        assert_eq!(ra, rb, "tic {tic}");
        if let Err(e) = rb {
            assert_eq!(e, SimError::MapScale(MapScaleError::PlatBlocked));
            assert_eq!(b.snapshot().canonical_bytes(), before);
            // Refused again on retry, still without a change.
            assert_eq!(b.advance(zero()), Err(e));
            assert_eq!(b.snapshot().canonical_bytes(), before);
            refused = Some(tic);
            break;
        }
        v40_is_v39_plus_map(&a, &b, 2);
    }
    assert!(refused.is_some(), "the blocked plat was not refused");
}

/// The DSIM40 writer and reader: round trip; the frozen readers (DSIM39
/// included) refuse 40 and the DSIM40 reader refuses 39 and every earlier
/// version; a DSIM39 capture relabelled 40 lacks `level.map`; `level.map` is
/// 1–9; the context, `profile` and the skill (2 or 3 since M4b step 6) are checked.
#[test]
fn dsim40_round_trip_and_refusals() {
    let limits = DiagnosticLimits::default();
    let c = &cases()[0];
    let l = with_start(room(c.map), 0, c.position, 0);
    let s = setup(&l, V2, 7, c.seed);
    let snap = s.snapshot();
    let bytes = snap.canonical_bytes();
    assert_eq!(&bytes[4..8], &40u32.to_le_bytes());
    let d = Diagnostic::decode_episode1_v2(&bytes, limits).unwrap();
    assert_eq!(d.version(), 40);
    assert_eq!(d.field("level.map"), Some(&[7u8][..]));
    assert_eq!(d.encode(limits).unwrap(), bytes);
    assert_eq!(snap.diagnostic_bytes(limits).unwrap(), bytes);
    for read in [
        Diagnostic::decode_episode1,
        Diagnostic::decode_level,
        Diagnostic::decode_barrels,
        Diagnostic::decode_e1m1,
        Diagnostic::decode,
    ] {
        assert_eq!(
            read(&bytes, limits).err(),
            Some(DiagnosticError::Version(40))
        );
    }
    let old = setup(&l, V1, 7, c.seed);
    let old_bytes = old.snapshot().canonical_bytes();
    Diagnostic::decode_episode1(&old_bytes, limits).unwrap();
    assert_eq!(
        Diagnostic::decode_episode1_v2(&old_bytes, limits).err(),
        Some(DiagnosticError::Version(39))
    );
    // Re-encode `snap`'s fields with `version` and `edit` applied.
    let rebuild = |version: u32, edit: &Edit| {
        let fields: Vec<(String, Vec<u8>)> = snap
            .fields()
            .filter_map(|f| edit(f.path(), f.value().to_vec()).map(|v| (f.path().to_string(), v)))
            .collect();
        let mut out = b"DSIM".to_vec();
        out.extend(version.to_le_bytes());
        out.extend((fields.len() as u64).to_le_bytes());
        for (path, value) in fields {
            out.extend((path.len() as u32).to_le_bytes());
            out.extend(path.as_bytes());
            out.extend((value.len() as u64).to_le_bytes());
            out.extend(value);
        }
        out
    };
    assert_eq!(rebuild(40, &|_, v| Some(v)), bytes);
    let read = |b: &[u8]| Diagnostic::decode_episode1_v2(b, limits).map(|_| ());
    // The DSIM40 capture labelled 39 is refused by the DSIM39 reader's schema.
    assert_eq!(
        Diagnostic::decode_episode1(&rebuild(39, &|_, v| Some(v)), limits).err(),
        Some(DiagnosticError::Schema)
    );
    // The DSIM39 capture relabelled as DSIM40 (version, profile and context) has
    // no `level.map`.
    let relabelled = {
        let fields: Vec<(String, Vec<u8>)> = old
            .snapshot()
            .fields()
            .map(|f| {
                let v = match f.path() {
                    "profile" => 24u32.to_le_bytes().to_vec(),
                    "context.schema" => PLAYER_EPISODE1_V2_CONTEXT_SCHEMA.to_vec(),
                    _ => f.value().to_vec(),
                };
                (f.path().to_string(), v)
            })
            .collect();
        let mut out = b"DSIM".to_vec();
        out.extend(40u32.to_le_bytes());
        out.extend((fields.len() as u64).to_le_bytes());
        for (path, value) in fields {
            out.extend((path.len() as u32).to_le_bytes());
            out.extend(path.as_bytes());
            out.extend((value.len() as u64).to_le_bytes());
            out.extend(value);
        }
        out
    };
    assert_eq!(read(&relabelled).err(), Some(DiagnosticError::Structure));
    let with = |path: &'static str, value: Vec<u8>| {
        rebuild(40, &move |p, v| {
            Some(if p == path { value.clone() } else { v })
        })
    };
    for map in [0u8, 10, 255] {
        assert_eq!(
            read(&with("level.map", vec![map])).err(),
            Some(DiagnosticError::Structure),
            "{map}"
        );
    }
    for map in EPISODE1_MAP_NUMBERS {
        read(&with("level.map", vec![map])).unwrap();
    }
    assert!(read(&with("level.map", vec![1, 0])).is_err());
    assert!(read(&rebuild(40, &|p, v| (p != "level.map").then_some(v))).is_err());
    assert_eq!(
        read(&with(
            "context.schema",
            PLAYER_EPISODE1_CONTEXT_SCHEMA.to_vec()
        ))
        .err(),
        Some(DiagnosticError::Schema)
    );
    assert_eq!(
        read(&with("profile", 23u32.to_le_bytes().to_vec())).err(),
        Some(DiagnosticError::Schema)
    );
    // Skills 2 and 3 alone are admitted (skill 3 since M4b step 6).
    for skill in [0u8, 1, 4, 5] {
        assert!(
            read(&with("actors.options", vec![skill, 0, 0])).is_err(),
            "{skill}"
        );
    }
    // The v39 field domains still apply.
    assert!(read(&with("level.secret_exit", vec![1])).is_err());
    assert!(read(&with("original_player.0.cards.3", vec![1])).is_err());
}
