//! Hard skill (skill 3) under wire 24 (M4b step 6, decision I4). Expected values
//! come from the C source cited in docs/M4B_REFERENCE_INVESTIGATION.md (§3 part C
//! items 28-30, §4.4, decision I4, findings 10, 11 and 23), from the THING options
//! written into the committed synthetic WAD `tests/reference/m4b_wads/skill.wad`
//! (read here as data) and from the committed genuine-engine fixtures; never from
//! Rust output. The tic-for-tic comparison of SKILL2 and SKILL3 against the
//! genuine goldens is `tests/episode1v2_mechanism_goldens.rs`.
//!
//! C facts used (linuxdoom-1.10, `P_SpawnMapThing`, p_mobj.c): types <= 4 and 11
//! return before any skill test (718-737); `if (!netgame && (options & 16))
//! return` (740-741); `bit` is 1 for baby, 4 for nightmare, else `1<<(gameskill-1)`,
//! so 2 for medium and 4 for hard (743-748); `if (!(options & bit)) return`
//! (750-751). Nothing else differs between skills 2 and 3 in single player
//! (note §3 part C item 29).
use crate::barrels_replay::{cases, room};
use crate::diagonal_room::Lumps;
use crate::episode1::Episode1Bounds;
use crate::identity::OriginalActorType;
use crate::player_episode1_v2_tests::ALL;
use crate::world_pickups::{counted_things, counted_things_with_bit};
use crate::*;

const V2: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V2;

fn skill_wad() -> Lumps {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference/m4b_wads/skill.wad");
    let mut bytes = std::fs::read(path).expect("skill.wad");
    // The generator writes IWAD-headed files; the lump editor reads the same layout.
    bytes[0] = b'P';
    Lumps::parse(&bytes)
}
fn map_of(l: &Lumps) -> doom_map::Map {
    let bytes = l.wad();
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).unwrap()
}
fn load(l: &Lumps, skill: Skill) -> Result<Simulation, OriginalRegistryError> {
    Simulation::original_player_episode1_v2_map(map_of(l), 1, skill, MapContextOptions::default())
}
fn setup(l: &Lumps, skill: Skill) -> Simulation {
    let mut s = load(l, skill).unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn count_of(s: &Simulation, ty: OriginalActorType) -> usize {
    s.actors
        .order()
        .filter(|id| s.actors.is_pending(*id) == Some(false))
        .filter_map(|id| s.actor(id).copied())
        .filter(|a| a.life.is_some_and(|l| l.actor_type.0 == ty.ordinal()))
        .count()
}
fn zero() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}
/// `snap`'s canonical bytes with one field's value replaced.
fn with_field(snap: &Snapshot, path: &str, value: &[u8]) -> Vec<u8> {
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
fn record(x: i16, y: i16, kind: i16, options: i16) -> Vec<u8> {
    [x, y, 0, kind, options]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
fn field_i32(s: &Simulation, path: &str) -> i32 {
    let snap = s.snapshot();
    let f = snap.fields().find(|f| f.path() == path).unwrap();
    i32::from_le_bytes(f.value().try_into().unwrap())
}

/// Decision I4: the wire-24 constructor admits medium and hard and refuses every
/// other skill, with the explicit `PlayerBody(Unsupported)`; `no_monsters` is not
/// a skill and is refused too (only default options or `{skill: Hard}` pass). The
/// skill is what the world keeps (`actors.options[0]`).
#[test]
fn wire_24_constructor_admits_only_medium_and_hard() {
    let c = &cases()[0];
    let l = room(c.map);
    let unsupported =
        OriginalRegistryError::Runtime(SimError::PlayerBody(PlayerBodyError::Unsupported));
    let build = |options: ActorOptions| {
        Simulation::with_original_registry(
            l.map(),
            crate::player_map_scale::actor_slots(V2),
            OriginalExecutionRegistry::new(V2).unwrap(),
            options,
            MapContextOptions::default(),
        )
    };
    for skill in [Skill::Baby, Skill::Easy, Skill::Nightmare] {
        let options = ActorOptions {
            skill,
            ..ActorOptions::default()
        };
        assert_eq!(
            build(options).err(),
            Some(SimError::PlayerBody(PlayerBodyError::Unsupported)),
            "{skill:?}"
        );
        assert_eq!(
            Simulation::original_player_episode1_v2_map(
                l.map(),
                1,
                skill,
                MapContextOptions::default()
            )
            .err(),
            Some(unsupported),
            "{skill:?}"
        );
    }
    // `no_monsters` at either admitted skill is no admitted option set.
    for skill in [Skill::Medium, Skill::Hard] {
        let options = ActorOptions {
            skill,
            no_monsters: true,
            ..ActorOptions::default()
        };
        assert_eq!(
            build(options).err(),
            Some(SimError::PlayerBody(PlayerBodyError::Unsupported)),
            "{skill:?} no_monsters"
        );
    }
    for skill in [Skill::Medium, Skill::Hard] {
        let s = Simulation::original_player_episode1_v2_map(
            l.map(),
            1,
            skill,
            MapContextOptions::default(),
        )
        .unwrap();
        assert_eq!(s.actor_options.skill, skill);
        let snap = s.snapshot();
        let options = snap
            .fields()
            .find(|f| f.path() == "actors.options")
            .unwrap();
        assert_eq!(options.value(), [skill as u8, 0, 0]);
    }
}

/// `Skill::Hard` is refused with `Unsupported` under wire 23 and every older
/// player-body policy (note §3 part C; `has_hard_skill` is wire 24 only), and
/// accepted exactly under wire 24. The registry entry point is below the public
/// per-policy constructors, which take no skill; it is the one door that does.
#[test]
fn hard_skill_is_refused_by_every_policy_but_wire_24() {
    let c = &cases()[0];
    let l = room(c.map);
    let hard = ActorOptions {
        skill: Skill::Hard,
        ..ActorOptions::default()
    };
    let mut player_policies = 0;
    for policy in ALL {
        if !policy.has_player_body() {
            continue;
        }
        player_policies += 1;
        let built = Simulation::with_original_registry(
            l.map(),
            crate::player_map_scale::actor_slots(policy),
            OriginalExecutionRegistry::new(policy).unwrap(),
            hard,
            MapContextOptions::default(),
        );
        if policy == V2 {
            assert!(built.is_ok(), "{policy:?}");
            assert!(policy.has_hard_skill());
        } else {
            assert_eq!(
                built.err(),
                Some(SimError::PlayerBody(PlayerBodyError::Unsupported)),
                "{policy:?}"
            );
            assert!(!policy.has_hard_skill(), "{policy:?}");
        }
    }
    // Every player-body policy was visited, wire 24 among them.
    assert!(player_policies >= 14, "{player_policies}");
}

/// A frame whose `actors.options[0]` is 3 is refused by the DSIM36-39 readers
/// (`validate_decorations` admits skill 2 only for them; hard is DSIM40 only,
/// decision I4), each fed a capture of its own wire with only that byte changed.
#[test]
fn frozen_readers_refuse_skill_3() {
    let limits = DiagnosticLimits::default();
    let c = &cases()[0];
    let hard = [3u8, 0, 0];
    let medium = [2u8, 0, 0];
    let with_start = |at: usize| {
        let mut l = room(c.map);
        let t = &mut l.lumps.iter_mut().find(|(n, _)| n == "THINGS").unwrap().1;
        let at = at.min(t.len() / 10);
        t.splice(at * 10..at * 10, record(c.position[0], c.position[1], 1, 7));
        l
    };
    // (reader, version, capture).
    type Reader = Box<dyn Fn(&[u8]) -> Result<(), DiagnosticError>>;
    let mut frames: Vec<(Reader, u32, Snapshot)> = Vec::new();
    {
        let (s, _, _) = crate::player_e1m1_tests::setup(0, c.position);
        frames.push((
            Box::new(move |b| Diagnostic::decode_e1m1(b, limits).map(|_| ())),
            36,
            s.snapshot(),
        ));
    }
    {
        let l = room(c.map);
        let mut s = Simulation::original_player_barrels_map(
            l.map(),
            MapContextOptions {
                initial_rng: DoomRng::from_indices(c.seed, 0),
                respawn_monsters: false,
            },
        )
        .unwrap();
        s.spawn_map_things().unwrap();
        s.spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
            .unwrap();
        s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
            .unwrap();
        s.spawn_level_specials().unwrap();
        frames.push((
            Box::new(move |b| Diagnostic::decode_barrels(b, limits).map(|_| ())),
            37,
            s.snapshot(),
        ));
    }
    {
        let l = with_start(0);
        let mut s = Simulation::original_player_level_map(
            l.map(),
            MapContextOptions {
                initial_rng: DoomRng::from_indices(c.seed, 0),
                respawn_monsters: false,
            },
        )
        .unwrap();
        s.spawn_map_things().unwrap();
        s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
            .unwrap();
        s.spawn_level_specials().unwrap();
        frames.push((
            Box::new(move |b| Diagnostic::decode_level(b, limits).map(|_| ())),
            38,
            s.snapshot(),
        ));
    }
    {
        let l = with_start(0);
        let mut s = Simulation::original_player_episode1_map(
            l.map(),
            MapContextOptions {
                initial_rng: DoomRng::from_indices(c.seed, 0),
                respawn_monsters: false,
            },
        )
        .unwrap();
        s.spawn_map_things().unwrap();
        s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
            .unwrap();
        s.spawn_level_specials().unwrap();
        frames.push((
            Box::new(move |b| Diagnostic::decode_episode1(b, limits).map(|_| ())),
            39,
            s.snapshot(),
        ));
    }
    for (read, version, snap) in &frames {
        assert_eq!(snap.version(), *version);
        // The unmodified capture and a re-encoded skill-2 capture are read.
        read(&snap.canonical_bytes()).unwrap();
        read(&with_field(snap, "actors.options", &medium)).unwrap();
        for skill in [hard, [1, 0, 0], [4, 0, 0]] {
            assert_eq!(
                read(&with_field(snap, "actors.options", &skill)).err(),
                Some(DiagnosticError::Identity),
                "DSIM{version} skill {}",
                skill[0]
            );
        }
    }
}

/// The DSIM40 reader admits `actors.options[0]` 2 and 3 on captures of their own
/// skill and refuses 0, 1, 4 and 5; a skill-3 capture round-trips byte for byte,
/// and so does the committed genuine-engine SKILL3 fixture (the C run's load of
/// the same WAD at skill 3, tests/reference/m4b_episode1v2_SKILL3_0.bin).
#[test]
fn dsim40_reader_skill_domain_and_hard_round_trip() {
    let limits = DiagnosticLimits::default();
    let l = skill_wad();
    for skill in [Skill::Medium, Skill::Hard] {
        let snap = setup(&l, skill).snapshot();
        let bytes = snap.canonical_bytes();
        let d = Diagnostic::decode_episode1_v2(&bytes, limits).unwrap();
        assert_eq!(d.field("actors.options"), Some(&[skill as u8, 0, 0][..]));
        assert_eq!(d.encode(limits).unwrap(), bytes, "{skill:?}");
        for bad in [0u8, 1, 4, 5] {
            assert!(
                Diagnostic::decode_episode1_v2(
                    &with_field(&snap, "actors.options", &[bad, 0, 0]),
                    limits
                )
                .is_err(),
                "{skill:?} relabelled {bad}"
            );
        }
    }
    for (name, skill) in [("SKILL2_0", 2u8), ("SKILL3_0", 3)] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../tests/reference/m4b_episode1v2_{name}.bin"));
        let bytes = std::fs::read(path).expect("fixture");
        let d = Diagnostic::decode_episode1_v2(&bytes, limits).unwrap();
        assert_eq!(
            d.field("actors.options"),
            Some(&[skill, 0, 0][..]),
            "{name}"
        );
        assert_eq!(d.encode(limits).unwrap(), bytes, "{name}");
    }
}

/// A hard snapshot read as medium (and the reverse) is refused by the DSIM40
/// reader, because the placed totals are re-derived from `map.things` with the
/// bit of the frame's own skill (diagnostic.rs `v40_skill`): the skill-3 frame's
/// `level.total_kills` (3, the hard-only Baron included, finding 23) is not the
/// skill-2 count (2) of the same THINGS lump, and vice versa. Skill.wad's
/// options are 2 (medium-only 3004), 4 (hard-only 3001 and 3003), 15 (3002, all).
#[test]
fn a_snapshot_relabelled_with_the_other_skill_is_refused() {
    let limits = DiagnosticLimits::default();
    let l = skill_wad();
    let medium = setup(&l, Skill::Medium);
    let hard = setup(&l, Skill::Hard);
    let (m, h) = (medium.snapshot(), hard.snapshot());
    assert_ne!(m.canonical_bytes(), h.canonical_bytes());
    assert_eq!(field_i32(&medium, "level.total_kills"), 2);
    assert_eq!(field_i32(&hard, "level.total_kills"), 3);
    Diagnostic::decode_episode1_v2(&m.canonical_bytes(), limits).unwrap();
    Diagnostic::decode_episode1_v2(&h.canonical_bytes(), limits).unwrap();
    assert!(
        Diagnostic::decode_episode1_v2(&with_field(&h, "actors.options", &[2, 0, 0]), limits)
            .is_err()
    );
    assert!(
        Diagnostic::decode_episode1_v2(&with_field(&m, "actors.options", &[3, 0, 0]), limits)
            .is_err()
    );
}

/// Bounds, landings and the pre-wire-24 helpers on skill.wad (options: 3004 = 2,
/// 3001 = 4|8, 3003 = 4|8, 3002 = 15, 9 = 1|8, 58 = 31, landing 14 = 4): medium
/// counts the Possessed and the Sergeant (K 2), hard the Imp, the Baron and the
/// Sergeant (K 3, finding 23); the landing exists at hard only; `from_map`
/// (wire 23 and the DSIM39 reader) keeps the medium bit.
#[test]
fn bounds_and_landings_differ_by_skill() {
    let l = skill_wad();
    let map = map_of(&l);
    let medium = Episode1Bounds::from_map_v2(&map, Skill::Medium);
    let hard = Episode1Bounds::from_map_v2(&map, Skill::Hard);
    assert_eq!(medium.monsters, 2);
    assert_eq!(hard.monsters, 3);
    // The Baron is a monster that drops nothing: only 3004's drop counts at
    // medium, none of 3001, 3003, 3002 at hard (p_inter.c:737-754 per finding 23).
    assert_eq!(hard.droppers, 0);
    assert_ne!(medium, hard);
    assert_eq!(Episode1Bounds::landings(&map, Skill::Medium), 0);
    assert_eq!(Episode1Bounds::landings(&map, Skill::Hard), 1);
    // Wire 23's helper ignores the skill and the Baron: the medium numbers, which
    // here are wire 24's medium numbers (no medium Baron on this map).
    assert_eq!(Episode1Bounds::from_map(&map), medium);
    // The same through the DSIM40 reader's record form.
    let things: Vec<u8> = map
        .things()
        .iter()
        .flat_map(|t| record(0, 0, t.kind, t.options as i16))
        .collect();
    assert_eq!(
        Episode1Bounds::from_thing_records_v2(&things, Skill::Medium),
        Some(medium)
    );
    assert_eq!(
        Episode1Bounds::from_thing_records_v2(&things, Skill::Hard),
        Some(hard)
    );
    assert_eq!(Episode1Bounds::from_thing_records(&things), Some(medium));
}

/// The skill bit in the record-form bounds (a pure function of the options, so a
/// hand-built lump pins each case): Barons with options 2, 4, 6, 1, 20 (4|16),
/// 18 (2|16) and 8 (ambush only), and landings likewise. Medium counts 2 and 6,
/// hard counts 4 and 6; bit 16 excludes at both skills (p_mobj.c:740-741), bit 1
/// and bit 8 alone satisfy neither skill (743-751).
#[test]
fn skill_bit_and_the_single_player_bit_in_record_bounds() {
    let options = [2i16, 4, 6, 1, 20, 18, 8];
    let barons: Vec<u8> = options
        .iter()
        .flat_map(|&o| record(0, 0, 3003, o))
        .collect();
    let landings: Vec<u8> = options.iter().flat_map(|&o| record(0, 0, 14, o)).collect();
    let k = |records: &[u8], skill| {
        Episode1Bounds::from_thing_records_v2(records, skill)
            .unwrap()
            .monsters
    };
    assert_eq!(k(&barons, Skill::Medium), 2);
    assert_eq!(k(&barons, Skill::Hard), 2);
    // Baby and easy would use bit 1 and nightmare bit 4 (the spawner's mapping;
    // wire 24 refuses them, but the helper is the same function the spawner shares).
    assert_eq!(k(&barons, Skill::Baby), 1);
    assert_eq!(k(&barons, Skill::Nightmare), 2);
    // Landings through a map are `landings`; the records form has no landing count,
    // so the Baron lump stands for the filter and the landing lump must not count
    // as monsters.
    assert_eq!(k(&landings, Skill::Hard), 0);
    // Bit 16 alone-with-skill excluded: only 20 and 18 carry it.
    let only_16: Vec<u8> = [20i16, 18, 22, 30]
        .iter()
        .flat_map(|&o| record(0, 0, 3003, o))
        .collect();
    for skill in [Skill::Medium, Skill::Hard] {
        assert_eq!(k(&only_16, skill), 0, "{skill:?}");
    }
}

/// `counted_things` (every older policy, medium bit 2) against
/// `counted_things_with_bit` (wire 24 at its skill's bit), on the item doomednums
/// 2014, 2015, 2013, 2024, 2026, 2045 (`MF_COUNTITEM`) with options 2, 2, 4, 6,
/// 22 (6|16, excluded) and 1. Guard: `counted_things_with_bit` is `pub(crate)`
/// and is what the `totalitems` cross-check (world_pickups.rs) and the DSIM40
/// reader call; the public API reaches it only through a full simulation, which
/// `hard_only_items_are_counted_at_hard` below also covers.
#[test]
fn counted_things_take_the_skill_bit() {
    let kinds = crate::world_pickups::COUNTED_ITEM_DOOMED_EPISODE1;
    let things = [
        (2014i16, 2u16),
        (2015, 2),
        (2013, 4),
        (2024, 6),
        (2026, 22),
        (2045, 1),
    ];
    assert_eq!(counted_things_with_bit(things, &kinds, 2), 3);
    assert_eq!(counted_things_with_bit(things, &kinds, 4), 2);
    assert_eq!(counted_things_with_bit(things, &kinds, 1), 1);
    // The older helper keeps the medium bit.
    assert_eq!(counted_things(things, &kinds), 3);
}

/// The total of `MF_COUNTITEM` things through a world: on skill.wad plus three
/// added items (2013 with options 2; 2014 and 2015 with options 4; a fourth, 2024
/// with 20 = 4|16, is excluded), `level.total_items` is 1 at medium and 2 at hard,
/// and the cross-check (`counted_map_things` with the run's bit) lets both worlds
/// tic: it would refuse a hard world as an Invariant if it still used bit 2.
#[test]
fn hard_only_items_are_counted_at_hard() {
    let mut l = skill_wad();
    for thing in [
        [-64, 96, 0, 2013, 2],
        [0, 96, 0, 2014, 4],
        [64, 96, 0, 2015, 4],
        [128, 96, 0, 2024, 20],
    ] {
        l.add_thing(thing);
    }
    let mut medium = setup(&l, Skill::Medium);
    let mut hard = setup(&l, Skill::Hard);
    assert_eq!(field_i32(&medium, "level.total_items"), 1);
    assert_eq!(field_i32(&hard, "level.total_items"), 2);
    for s in [&mut medium, &mut hard] {
        s.advance(zero()).unwrap();
        s.advance(zero()).unwrap();
    }
    // The DSIM40 reader's own count agrees at each frame's skill.
    let limits = DiagnosticLimits::default();
    Diagnostic::decode_episode1_v2(&medium.snapshot().canonical_bytes(), limits).unwrap();
    Diagnostic::decode_episode1_v2(&hard.snapshot().canonical_bytes(), limits).unwrap();
}

/// Hard-only things spawn at hard and not at medium in a wire-24 world, medium-only
/// things the other way round, all-skill things at both, and things with bit 16
/// (MTF multiplayer-only, p_mobj.c:740-741) or without bit 2 or 4 (the baby-only
/// 9 with options 9) at neither; the hard-only landing (14, options 4) is a
/// `MT_TELEPORTMAN` at hard only.
#[test]
fn things_spawn_by_skill_bit() {
    let l = skill_wad();
    let medium = setup(&l, Skill::Medium);
    let hard = setup(&l, Skill::Hard);
    let n = |s: &Simulation, ty| count_of(s, ty);
    use OriginalActorType as T;
    // (type, at medium, at hard)
    for (ty, at_medium, at_hard) in [
        (T::MT_POSSESSED, 1, 0),   // 3004, options 10 = 2|8
        (T::MT_TROOP, 0, 1),       // 3001, options 12 = 4|8
        (T::MT_BRUISER, 0, 1),     // 3003, options 12 = 4|8
        (T::MT_SERGEANT, 1, 1),    // 3002, options 15
        (T::MT_SHOTGUY, 0, 0),     // 9, options 9 = 1|8: baby-only
        (T::MT_SHADOWS, 0, 0),     // 58, options 31 has bit 16
        (T::MT_TELEPORTMAN, 0, 1), // 14, options 4
    ] {
        assert_eq!(n(&medium, ty), at_medium, "medium {ty:?}");
        assert_eq!(n(&hard, ty), at_hard, "hard {ty:?}");
    }
}

/// SKILL3's demo header carries skill 3 (note §3 part C; demo byte, g_game.c per
/// item 29), and the same WAD loaded at skill 2 and at skill 3 gives different
/// canonical states at tic 0 (the hard-only Baron, Imp and landing), the premise of
/// the integration self-test `harness_detects_the_wrong_skill` in
/// `tests/episode1v2_mechanism_goldens.rs`.
#[test]
fn skill3_demo_header_is_hard_and_the_worlds_differ_at_tic_0() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference/m4b_scripts/SKILL3.lmp");
    let bytes = std::fs::read(path).expect("SKILL3.lmp");
    let (header, _) = crate::demo::parse_demo_episode1_v2(&bytes, 1).unwrap();
    assert_eq!(header.skill, 3);
    let l = skill_wad();
    assert_ne!(
        setup(&l, Skill::Medium).snapshot().canonical_bytes(),
        setup(&l, Skill::Hard).snapshot().canonical_bytes()
    );
}
