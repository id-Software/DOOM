//! M5c wire 25 (`PlayerMonstersEpisode1V3`, DSIM41), step 3a: unit tests of what
//! wire 25 adds, each from a cited C fact (M5c note §2, §4.4.4, §4.4.5). Rust-only
//! tests: the genuine comparison of the records is M5c step 3b's.
use crate::barrels_replay::{cases, room};
use crate::diagonal_room::Lumps;
use crate::sound::{SoundError, SoundOrigin, SoundRecord};
use crate::*;

const V2: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V2;
const V3: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V3;
/// sounds.h:236-237, counting from `sfx_None` 0.
const SFX_PLDETH: u16 = 57;
const SFX_PDIEHI: u16 = 58;

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
fn load(l: &Lumps, policy: OriginalExecutionProfile) -> Result<Simulation, SimError> {
    let options = MapContextOptions::default();
    let result = if policy == V3 {
        Simulation::original_player_episode1_v3_map(l.map(), 1, Skill::Medium, options)
    } else {
        Simulation::original_player_episode1_v2_map(l.map(), 1, Skill::Medium, options)
    };
    result.map_err(|e| match e {
        OriginalRegistryError::Runtime(e) => e,
        other => panic!("{other:?}"),
    })
}
fn setup(l: &Lumps, policy: OriginalExecutionProfile) -> Simulation {
    let mut s = load(l, policy).unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn base() -> Lumps {
    let c = &cases()[0];
    with_start(room(c.map), 0, c.position, 0)
}
fn body(s: &Simulation) -> ActorId {
    s.original_player.as_ref().unwrap().mo.unwrap()
}

/// The sound starts of a list of events, as (origin, sfx).
fn sounds(events: &[Event]) -> Vec<(Option<ActorId>, u16)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::ActorState(ActorStateEvent::Sound { actor, sound }) => Some((*actor, *sound)),
            _ => None,
        })
        .collect()
}

/// `A_PlayerScream` (p_enemy.c:1994-2007) plays `sfx_pdiehi` only in commercial
/// mode with the health below -50: wire 25 (non-commercial) plays `sfx_pldeth` at
/// any health, with the listener's own body as its unplaced origin; wire 24 keeps
/// its frozen `sfx_pdiehi` below -50. The player is killed by an explicit
/// unsourced damage of 100 - health (no gib: the health stays at or above -100,
/// the player's spawn health, p_inter.c:726-729), and the scream comes with
/// `S_PLAY_DIE2` (info.c) a few tics later.
#[test]
fn hard_death_scream_is_pldeth_under_wire_25_only() {
    for (policy, health, expected) in [
        (V2, -51, SFX_PDIEHI),
        (V2, -50, SFX_PLDETH),
        (V3, -51, SFX_PLDETH),
        (V3, -100, SFX_PLDETH),
        (V3, -50, SFX_PLDETH),
    ] {
        let mut s = setup(&base(), policy);
        let mo = body(&s);
        s.damage_actor(DamageRequest {
            target: mo,
            inflictor: None,
            source: None,
            amount: 100 - health,
        })
        .unwrap();
        let mut heard = Vec::new();
        let mut records = Vec::new();
        for _ in 0..20 {
            s.advance([TicCommand::default(); 4]).unwrap();
            heard.extend(sounds(s.events()));
            records.extend_from_slice(s.sound_records());
        }
        let screams: Vec<_> = heard
            .iter()
            .filter(|(_, sfx)| matches!(*sfx, SFX_PLDETH | SFX_PDIEHI))
            .copied()
            .collect();
        assert_eq!(screams, [(Some(mo), expected)], "{policy:?} {health}");
        if policy == V3 {
            assert!(records.contains(&SoundRecord::Start {
                sfx: SFX_PLDETH,
                origin: SoundOrigin::Actor(mo),
                at: None,
            }));
        } else {
            assert!(records.is_empty());
        }
    }
}

/// `M_AddToBox`'s `else if` (m_bbox.c:51-58): a first point lowers only the
/// left and bottom edges, so an outline whose x only decreases keeps
/// `BOXRIGHT == MININT`, and the origin is `(MININT + left) / 2`, truncated
/// toward zero (p_setup.c:556). A closed outline gives the box's centre.
#[test]
fn sector_origin_box_arithmetic() {
    let mut b = sound::CLEAR_BOX;
    sound::add_to_box(&mut b, [10, 0]);
    sound::add_to_box(&mut b, [5, 0]);
    assert_eq!(b, [0, 0, 5, i32::MIN]);
    // (-2147483648 + 5) / 2 = -1073741821.5, truncated toward zero.
    assert_eq!(sound::box_origin(&b), Some([-1_073_741_821, 0]));
    let mut b = sound::CLEAR_BOX;
    for p in [[0, 0], [7, 0], [7, -3], [0, -3]] {
        sound::add_to_box(&mut b, p);
    }
    // x (7 + 0) / 2 = 3; y (0 + -3) / 2 = -1 (toward zero, not -2).
    assert_eq!(sound::box_origin(&b), Some([3, -1]));
    // A box with no point: (MININT + MAXINT) / 2 = -1 / 2 = 0.
    assert_eq!(sound::box_origin(&sound::CLEAR_BOX), Some([0, 0]));
    // Sums that leave `int` (undefined in C) are refused.
    let mut b = sound::CLEAR_BOX;
    sound::add_to_box(&mut b, [i32::MAX - 1, 0]);
    sound::add_to_box(&mut b, [i32::MAX, 0]);
    assert_eq!(sound::box_origin(&b), None);
}

/// The single pass over the lines gives, for every sector of the room, what the
/// source's per-sector pass over every line gives (p_setup.c:536-557, written
/// here as the C loop nest), and the constructor holds them only under wire 25.
#[test]
fn sector_origins_follow_p_group_lines() {
    let l = base();
    let map = l.map();
    let side = |s: Option<u16>| s.map(|s| map.sidedefs()[usize::from(s)].sector);
    let mut expected = Vec::new();
    for sector in 0..map.sectors().len() as u16 {
        let mut b = sound::CLEAR_BOX;
        for line in map.linedefs() {
            if side(line.sides[0]) == Some(sector) || side(line.sides[1]) == Some(sector) {
                for v in line.vertices {
                    let v = map.vertices()[usize::from(v)];
                    sound::add_to_box(&mut b, [i32::from(v.x) << 16, i32::from(v.y) << 16]);
                }
            }
        }
        expected.push(sound::box_origin(&b).unwrap());
    }
    let s = load(&l, V3).unwrap();
    for (sector, xy) in expected.iter().enumerate() {
        assert_eq!(s.sector_sound_origin(sector as u16), Some(*xy));
    }
    assert_eq!(s.sector_sound_origin(expected.len() as u16), None);
    let s = load(&l, V2).unwrap();
    assert_eq!(s.sector_sound_origin(0), None);
}

/// The room moved `dx` units east: every vertex, THING, node partition and node
/// box, and the BLOCKMAP origin (doomdata.h `mapvertex_t`, `mapthing_t`,
/// `mapnode_t`; the BLOCKMAP header's first short).
fn shifted(mut l: Lumps, dx: i16) -> Lumps {
    let mut add = |name: &str, stride: usize, offsets: &[usize]| {
        let data = lump(&mut l, name);
        for chunk in data.chunks_exact_mut(stride) {
            for &o in offsets {
                let v = i16::from_le_bytes([chunk[o], chunk[o + 1]]) + dx;
                chunk[o..o + 2].copy_from_slice(&v.to_le_bytes());
            }
        }
    };
    add("VERTEXES", 4, &[0]);
    add("THINGS", 10, &[0]);
    add("NODES", 28, &[0, 12, 14, 20, 22]);
    let b = lump(&mut l, "BLOCKMAP");
    let v = i16::from_le_bytes([b[0], b[1]]) + dx;
    b[..2].copy_from_slice(&v.to_le_bytes());
    l
}

/// The same room 16,000 units east loads under wires 24 and 25 with every
/// sector origin moved by exactly that much; 17,000 units east every sector's
/// left and right edges pass 16,384 units, so `bbox[BOXRIGHT]+bbox[BOXLEFT]`
/// leaves `int` (p_setup.c:556, undefined in C): wire 25 refuses the map at load
/// with the first such sector, and wire 24, which computes no origin, loads it.
#[test]
fn sector_origin_overflow_refuses_the_map_under_wire_25() {
    let near = load(&base(), V3).unwrap();
    let far = load(&shifted(base(), 16_000), V3).unwrap();
    for sector in 0..near.world.map().sectors().len() as u16 {
        let [x, y] = near.sector_sound_origin(sector).unwrap();
        assert_eq!(
            far.sector_sound_origin(sector),
            Some([x + (16_000 << 16), y])
        );
    }
    let l = shifted(base(), 17_000);
    assert_eq!(
        load(&l, V3).err(),
        Some(SimError::Sound(SoundError::SectorOriginOverflow {
            sector: 0
        }))
    );
    assert_eq!(load(&l, V2).unwrap().sector_sound_origin(0), None);
}

/// The button release (p_spec.c:1129-1152): slot 0 expiring reads slot 1's
/// `btimer` before the loop decrements it as x and 0 as y (the declared 64-bit
/// reading, M5c decision T12), with the listener at the end of the tic; a free
/// slot 1 reads 0, 0. Slot 15 expiring refuses the tic under wire 25 alone and
/// leaves the state untouched.
#[test]
fn button_release_placement_and_slot_15() {
    for (next, x) in [(Some(20_u8), 20), (None, 0)] {
        let mut s = setup(&base(), V3);
        s.buttons[0] = Some(crate::use_lines::Button {
            line: 0,
            timer: 1,
            sector: 0,
        });
        s.buttons[1] = next.map(|timer| crate::use_lines::Button {
            line: 1,
            timer,
            sector: 0,
        });
        assert_eq!(s.button_release_xy(0), Ok([x, 0]));
        s.advance([TicCommand::default(); 4]).unwrap();
        let listener = s.sound_listener().unwrap();
        let release: Vec<_> = s
            .sound_records()
            .iter()
            .filter(|r| {
                matches!(
                    r,
                    SoundRecord::Start {
                        origin: SoundOrigin::Button(_),
                        ..
                    }
                )
            })
            .copied()
            .collect();
        assert_eq!(
            release,
            [SoundRecord::Start {
                sfx: 23,
                origin: SoundOrigin::Button(0),
                at: Some(sound::SoundPlacement {
                    origin: [x, 0],
                    listener: listener.xy,
                    listener_angle: listener.angle,
                }),
            }]
        );
        assert_eq!(s.sound_buttons()[1], next.map(|t| t - 1));
    }
    for policy in [V2, V3] {
        let mut s = setup(&base(), policy);
        s.buttons[15] = Some(crate::use_lines::Button {
            line: 0,
            timer: 1,
            sector: 0,
        });
        let before = s.snapshot();
        let result = s.advance([TicCommand::default(); 4]);
        if policy == V3 {
            assert_eq!(result, Err(SimError::Sound(SoundError::ButtonSlot15)));
            assert_eq!(s.snapshot(), before);
        } else {
            result.unwrap();
        }
    }
    let s = setup(&base(), V3);
    assert_eq!(s.button_release_xy(14), Ok([0, 0]));
    assert_eq!(s.button_release_xy(15), Err(SoundError::ButtonSlot15));
}

/// `P_RemoveMobj`'s `S_StopSound` (p_mobj.c:565-566): an explicit removal under
/// wire 25 holds a stop record with the pending events, published first with the
/// next tic's records; wire 24 holds none.
#[test]
fn removal_makes_a_stop_record() {
    for policy in [V2, V3] {
        // The first M3h9b room holding an actor an explicit removal admits.
        let (mut s, victim) = cases()
            .iter()
            .find_map(|c| {
                let mut s = setup(&with_start(room(c.map), 0, c.position, 0), policy);
                let me = body(&s);
                let order: Vec<ActorId> = s.actor_order().collect();
                let victim = order
                    .into_iter()
                    .find(|id| *id != me && s.mark_remove(*id).is_ok())?;
                Some((s, victim))
            })
            .expect("a room holds a removable actor");
        let stop = SoundRecord::Stop { actor: victim };
        if policy == V3 {
            assert_eq!(s.pending_sound_records(), [stop]);
            s.advance([TicCommand::default(); 4]).unwrap();
            assert_eq!(s.sound_records().first(), Some(&stop));
            assert!(s.pending_sound_records().is_empty());
        } else {
            assert!(s.pending_sound_records().is_empty());
        }
    }
}

/// The records follow the events: starts in event order, each positional start
/// with its placement right after it, a NULL or listener origin unplaced, every
/// `RemovalMarked` a stop; a positional start without its placement, or a
/// placement without its start, is refused.
#[test]
fn split_keeps_the_call_order_and_refuses_an_unplaced_start() {
    let (me, other) = (ActorId::from_parts(1, 1), ActorId::from_parts(2, 1));
    let at = sound::SoundPlacement {
        origin: [1, 2],
        listener: [3, 4],
        listener_angle: 5,
    };
    let sound = |actor, sound| Event::ActorState(ActorStateEvent::Sound { actor, sound });
    let placed = |sfx, origin| {
        Event::SoundRecord(SoundRecord::Start {
            sfx,
            origin,
            at: Some(at),
        })
    };
    let events = vec![
        sound(Some(other), 7),
        placed(7, SoundOrigin::Actor(other)),
        Event::RemovalMarked(other),
        sound(None, 8),
        sound(Some(me), 9),
        Event::Mover(MoverEvent::SectorSound {
            origin: planes::SoundOrigin::Sector,
            index: 3,
            sound: 10,
        }),
        placed(10, SoundOrigin::Sector(3)),
    ];
    let mut out = Vec::new();
    sound::split(&events, Some(me), &mut out, true).unwrap();
    assert_eq!(
        out,
        [
            SoundRecord::Start {
                sfx: 7,
                origin: SoundOrigin::Actor(other),
                at: Some(at)
            },
            SoundRecord::Stop { actor: other },
            SoundRecord::Start {
                sfx: 8,
                origin: SoundOrigin::None,
                at: None
            },
            SoundRecord::Start {
                sfx: 9,
                origin: SoundOrigin::Actor(me),
                at: None
            },
            SoundRecord::Start {
                sfx: 10,
                origin: SoundOrigin::Sector(3),
                at: Some(at)
            },
        ]
    );
    let mut bad = events.clone();
    bad.remove(1);
    assert_eq!(
        sound::split(&bad, Some(me), &mut Vec::new(), true),
        Err(SoundError::Unplaced)
    );
    let mut bad = events.clone();
    bad.push(placed(11, SoundOrigin::None));
    assert_eq!(
        sound::split(&bad, Some(me), &mut Vec::new(), true),
        Err(SoundError::Unplaced)
    );
    let mut bad = events;
    bad[1] = placed(7, SoundOrigin::Actor(me));
    assert_eq!(
        sound::split(&bad, Some(me), &mut Vec::new(), true),
        Err(SoundError::Unplaced)
    );
}

/// The room with one THING of `kind` inserted at lump position `at` (the start
/// is record 0 of [`base`]), 64 units east of the start.
fn with_thing(at: usize, kind: i16) -> Lumps {
    let c = &cases()[0];
    let mut l = base();
    let t = lump(&mut l, "THINGS");
    t.splice(
        at * 10..at * 10,
        record(c.position[0] + 64, c.position[1], 0, kind, 7),
    );
    l
}

/// `P_LoadThings` outside commercial mode ends the whole loop at the first of the
/// ten Doom II types (p_setup.c:317-337), before `P_SpawnMapThing`: under wire 25
/// a type-65 record after the start leaves the player alone (every later record
/// unspawned), and a type-66 record, which wire 24 refuses, is skipped in the
/// same way; placed before the start, it leaves no player (`players[0].mo` NULL),
/// which the level policy refuses as it refuses a map without a start. Wire 24
/// spawns the chaingunner (type 65) and the records after it.
#[test]
fn thing_loading_truncation_under_wire_25() {
    let counts = |l: &Lumps, policy| {
        let mut s = load(l, policy).unwrap();
        s.spawn_map_things()
            .map(|r| (s.actor_order().count(), r.player_starts.len()))
    };
    let all = counts(&base(), V2).unwrap();
    assert_eq!(counts(&base(), V3).unwrap(), all);
    assert_eq!(counts(&with_thing(1, 65), V2).unwrap(), (all.0 + 1, 1));
    for kind in [68, 64, 88, 89, 69, 67, 71, 65, 66, 84] {
        assert_eq!(counts(&with_thing(1, kind), V3).unwrap(), (1, 1), "{kind}");
        assert_eq!(
            counts(&with_thing(0, kind), V3),
            Err(SimError::PlayerBody(PlayerBodyError::Binding)),
            "{kind}"
        );
    }
    assert!(matches!(
        counts(&with_thing(1, 66), V2),
        Err(SimError::MapSpawn(MapSpawnError::UnsupportedType(66)))
    ));
}

/// M5c review, MINOR 2: the admission bounds of wire 25 (`Episode1Bounds`, the
/// landings and the transient allowance `slots - (1+K+R+P+B+D+L)`, M4b step 4)
/// and the item total's cross-check count only the records `P_LoadThings` loads,
/// the prefix before the first of the ten Doom II types (p_setup.c:317-337), as
/// `totalkills` and `totalitems` count only spawned things (p_mobj.c:790, 792).
/// The map: the room, a health bonus (2014, `MF_COUNTITEM`) after the start, then
/// a type-65 record and a tail of 1,100 zombiemen, 8 health bonuses and 4
/// landings that the original never loads. Wire 25 loads and plays it with the
/// tail uncounted; wire 24, which loads every record, refuses it at load for want
/// of actor slots (`StoreError::Capacity`).
#[test]
fn an_unloaded_thing_tail_is_not_counted_under_wire_25() {
    let c = &cases()[0];
    let near = |kind| record(c.position[0] + 64, c.position[1], 0, kind, 7);
    let mut l = base();
    let t = lump(&mut l, "THINGS");
    t.splice(10..10, near(2014));
    t.extend(near(65));
    for _ in 0..1100 {
        t.extend(near(3004));
    }
    for _ in 0..8 {
        t.extend(near(2014));
    }
    for _ in 0..4 {
        t.extend(near(14));
    }
    assert!(matches!(
        load(&l, V2),
        Err(SimError::Store(StoreError::Capacity))
    ));
    let mut s = setup(&l, V3);
    let reference = setup(&base(), V3);
    let total = |s: &Simulation| s.level_statistics().unwrap().total_items;
    assert_eq!(total(&s), total(&reference) + 1);
    assert_eq!(
        s.level_statistics().unwrap().total_kills,
        reference.level_statistics().unwrap().total_kills
    );
    assert_eq!(s.actor_order().count(), reference.actor_order().count() + 1);
    for _ in 0..4 {
        s.advance([TicCommand::default(); 4]).unwrap();
        Diagnostic::decode_episode1_v3(
            &s.snapshot().canonical_bytes(),
            DiagnosticLimits::default(),
        )
        .unwrap();
    }
}

/// A field edit for re-encoded captures: `None` drops the field.
type Edit = dyn Fn(&str, &[u8]) -> Option<Vec<u8>>;

/// A DSIM41 capture with records is read by `decode_episode1_v3` alone; a record
/// whose sound is 0, whose placement flag disagrees with its origin, whose button
/// y is not 0 or whose tag is unknown is refused, as is an `events.sounds` field
/// in a DSIM40 frame.
#[test]
fn dsim41_reader_round_trip_and_refusals() {
    let mut s = setup(&base(), V3);
    s.buttons[0] = Some(crate::use_lines::Button {
        line: 0,
        timer: 1,
        sector: 0,
    });
    s.advance([TicCommand::default(); 4]).unwrap();
    assert!(!s.sound_records().is_empty());
    let snap = s.snapshot();
    let limits = DiagnosticLimits::default();
    let bytes = snap.canonical_bytes();
    assert_eq!(
        Diagnostic::decode_episode1_v3(&bytes, limits)
            .unwrap()
            .version(),
        41
    );
    assert_eq!(
        Diagnostic::decode_episode1_v2(&bytes, limits).err(),
        Some(DiagnosticError::Version(41))
    );
    let v2 = setup(&base(), V2).snapshot().canonical_bytes();
    assert_eq!(
        Diagnostic::decode_episode1_v3(&v2, limits).err(),
        Some(DiagnosticError::Version(40))
    );
    let encode = |version: u32, edit: &Edit| {
        let fields: Vec<(String, Vec<u8>)> = snap
            .fields()
            .filter_map(|f| edit(f.path(), f.value()).map(|v| (f.path().to_owned(), v)))
            .collect();
        let mut out = b"DSIM".to_vec();
        out.extend(version.to_le_bytes());
        out.extend((fields.len() as u64).to_le_bytes());
        for (p, v) in fields {
            out.extend((p.len() as u32).to_le_bytes());
            out.extend(p.as_bytes());
            out.extend((v.len() as u64).to_le_bytes());
            out.extend(v);
        }
        out
    };
    assert_eq!(encode(41, &|_, v| Some(v.to_vec())), bytes);
    let record = |edit: fn(&mut Vec<u8>)| {
        move |p: &str, v: &[u8]| {
            let mut v = v.to_vec();
            if p == "events.sounds.last.0" {
                edit(&mut v);
            }
            Some(v)
        }
    };
    let first = snap
        .fields()
        .find(|f| f.path() == "events.sounds.last.0")
        .unwrap()
        .value()
        .to_vec();
    // The tic's only start is the release of slot 0: tag 1, sfx 23, kind 3,
    // slot 0, placed, origin (0, 0).
    assert_eq!(first[..7], [1, 23, 0, 3, 0, 1, 0]);
    for (name, edit) in [
        ("sfx 0", record(|v| v[1] = 0)),
        // Placed 0 for a button origin, which is always positional
        // (s_sound.c:304): the record is whole, so only the placed/positional
        // rule refuses it.
        (
            "unplaced button",
            record(|v| {
                v.truncate(5);
                v.push(0)
            }),
        ),
        // Placed 1 without the 20 bytes of the placement: the length check.
        ("short placed record", record(|v| v.truncate(6))),
        ("button y", record(|v| v[11] = 1)),
        ("tag", record(|v| v[0] = 3)),
    ] {
        assert!(
            Diagnostic::decode_episode1_v3(&encode(41, &edit), limits).is_err(),
            "{name}"
        );
    }
    let v40_with_sounds = encode(40, &|p, v| {
        let v = v.to_vec();
        Some(match p {
            "profile" => 24_u32.to_le_bytes().to_vec(),
            "context.schema" => PLAYER_EPISODE1_V2_CONTEXT_SCHEMA.to_vec(),
            _ => v,
        })
    });
    assert_eq!(
        Diagnostic::decode_episode1_v2(&v40_with_sounds, limits).err(),
        Some(DiagnosticError::Structure)
    );
}

/// Encodes `fields` as a DSIM41 frame (the layout of `Snapshot::canonical_bytes`).
fn encode41(fields: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"DSIM".to_vec();
    out.extend(41_u32.to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for (p, v) in fields {
        out.extend((p.len() as u32).to_le_bytes());
        out.extend(p.as_bytes());
        out.extend((v.len() as u64).to_le_bytes());
        out.extend(v);
    }
    out
}

/// The fields of a snapshot, in order.
fn owned_fields(s: &Simulation) -> Vec<(String, Vec<u8>)> {
    s.snapshot()
        .fields()
        .map(|f| (f.path().to_owned(), f.value().to_vec()))
        .collect()
}

/// `fields` with the value at `path` replaced.
fn set(fields: &mut [(String, Vec<u8>)], path: &str, value: Vec<u8>) {
    fields.iter_mut().find(|(p, _)| p == path).unwrap().1 = value;
}

/// M5c review, MINOR 3: the DSIM41 reader refuses frames the writer never
/// produces: a record beyond `events.sounds.last.count` or
/// `events.sounds.pending.count`, a field of the group outside its documented
/// names, a button start in slot 15 (whose release reads past `buttonlist`,
/// MAXBUTTONS 16, p_spec.h:244; p_spec.c:1151; the simulation refuses that tic),
/// and pending records that are not the calls of `events.pending` (a stop
/// without its `RemovalMarked`, a removal without its stop, a stop of another
/// actor), which the simulation would refuse at the next tic
/// (`sound::same_calls`). A consistent pending stop (an explicit removal,
/// p_mobj.c:565-566) and a release in slot 14 are read.
#[test]
fn dsim41_reader_refuses_forged_frames() {
    let limits = DiagnosticLimits::default();
    let read = |f: &[(String, Vec<u8>)]| Diagnostic::decode_episode1_v3(&encode41(f), limits).err();
    // A pending stop: the first M3h9b room holding a removable actor.
    let (s, victim) = cases()
        .iter()
        .find_map(|c| {
            let mut s = setup(&with_start(room(c.map), 0, c.position, 0), V3);
            let me = body(&s);
            let order: Vec<ActorId> = s.actor_order().collect();
            let victim = order
                .into_iter()
                .find(|id| *id != me && s.mark_remove(*id).is_ok())?;
            Some((s, victim))
        })
        .expect("a room holds a removable actor");
    assert_eq!(
        s.pending_sound_records(),
        [SoundRecord::Stop { actor: victim }]
    );
    let frame = owned_fields(&s);
    assert_eq!(read(&frame), None);
    let stop = crate::sound::record_bytes(&SoundRecord::Stop { actor: victim });
    let other = crate::sound::record_bytes(&SoundRecord::Stop { actor: body(&s) });
    let count = |n: u64| n.to_le_bytes().to_vec();
    let with = |extra: &[(&str, &[u8])]| {
        let mut f = frame.clone();
        f.extend(extra.iter().map(|(p, v)| ((*p).to_owned(), v.to_vec())));
        f
    };
    let last = u64::from_le_bytes(
        frame
            .iter()
            .find(|(p, _)| p == "events.sounds.last.count")
            .unwrap()
            .1
            .clone()
            .try_into()
            .unwrap(),
    );
    let surplus = format!("events.sounds.last.{last}");
    assert_eq!(
        read(&with(&[(surplus.as_str(), &stop)])),
        Some(DiagnosticError::Structure),
        "surplus last record"
    );
    assert_eq!(
        read(&with(&[("events.sounds.pending.1", &stop)])),
        Some(DiagnosticError::Structure),
        "surplus pending record"
    );
    assert_eq!(
        read(&with(&[("events.sounds.zzz", &[0])])),
        Some(DiagnosticError::Structure),
        "unknown field"
    );
    // A second stop with no second removal.
    let mut f = with(&[("events.sounds.pending.1", &other)]);
    set(&mut f, "events.sounds.pending.count", count(2));
    assert_eq!(
        read(&f),
        Some(DiagnosticError::Identity),
        "stop without removal"
    );
    // The removal without its stop.
    let mut f: Vec<_> = frame
        .iter()
        .filter(|(p, _)| p != "events.sounds.pending.0")
        .cloned()
        .collect();
    set(&mut f, "events.sounds.pending.count", count(0));
    assert_eq!(
        read(&f),
        Some(DiagnosticError::Identity),
        "removal without stop"
    );
    // The stop of another actor than the removed one.
    let mut f = frame.clone();
    set(&mut f, "events.sounds.pending.0", other);
    assert_eq!(
        read(&f),
        Some(DiagnosticError::Identity),
        "stop of another actor"
    );

    // A button release: tag 1, sfx 23, kind 3, slot 0, placed.
    let mut s = setup(&base(), V3);
    s.buttons[0] = Some(crate::use_lines::Button {
        line: 0,
        timer: 1,
        sector: 0,
    });
    s.advance([TicCommand::default(); 4]).unwrap();
    let frame = owned_fields(&s);
    let release = frame
        .iter()
        .find(|(p, _)| p == "events.sounds.last.0")
        .unwrap()
        .1
        .clone();
    assert_eq!(release[..6], [1, 23, 0, 3, 0, 1]);
    for (slot, expected) in [(14, None), (15, Some(DiagnosticError::Identity))] {
        let mut f = frame.clone();
        let mut r = release.clone();
        r[4] = slot;
        set(&mut f, "events.sounds.last.0", r);
        assert_eq!(read(&f), expected, "slot {slot}");
    }
}

// ---------------------------------------------------------------------------------------------
// Tests for the mutants the first M5c mutation run missed (step 10). Each names the C fact or
// documented rule it rests on and the mutants it kills.
// ---------------------------------------------------------------------------------------------

/// `sound::same_calls` ("whether `records` are exactly the calls of `events`: sound, origin and
/// order; placements are not compared"): the records of the events themselves match, a
/// different sound, a missing record, an extra event or events that cannot be split (a
/// transient placement with no start before it) do not, and a different placement does not
/// matter.
/// Kills sound.rs `same_calls` -> `true` and `||` -> `&&` in it.
#[test]
fn same_calls_compares_sound_origin_and_order_but_not_placements() {
    let (me, other) = (ActorId::from_parts(1, 1), ActorId::from_parts(2, 1));
    let at = sound::SoundPlacement {
        origin: [1, 2],
        listener: [3, 4],
        listener_angle: 5,
    };
    let sound_event = |actor, sound| Event::ActorState(ActorStateEvent::Sound { actor, sound });
    let placed = |sfx, origin, at| {
        Event::SoundRecord(SoundRecord::Start {
            sfx,
            origin,
            at: Some(at),
        })
    };
    let events = vec![
        sound_event(Some(other), 7),
        placed(7, SoundOrigin::Actor(other), at),
        Event::RemovalMarked(other),
        sound_event(None, 8),
    ];
    let start = |sfx, origin, at| SoundRecord::Start { sfx, origin, at };
    let records = vec![
        start(7, SoundOrigin::Actor(other), Some(at)),
        SoundRecord::Stop { actor: other },
        start(8, SoundOrigin::None, None),
    ];
    assert!(sound::same_calls(&events, Some(me), &records));
    // The placement is not compared.
    let moved = sound::SoundPlacement {
        origin: [9, 9],
        ..at
    };
    let mut r = records.clone();
    r[0] = start(7, SoundOrigin::Actor(other), Some(moved));
    assert!(sound::same_calls(&events, Some(me), &r));
    // A different sound, origin or kind of record, the order, one record too few or too many.
    let mut r = records.clone();
    r[2] = start(9, SoundOrigin::None, None);
    assert!(!sound::same_calls(&events, Some(me), &r));
    let mut r = records.clone();
    r[0] = start(7, SoundOrigin::Actor(me), Some(at));
    assert!(!sound::same_calls(&events, Some(me), &r));
    let mut r = records.clone();
    r.swap(1, 2);
    assert!(!sound::same_calls(&events, Some(me), &r));
    assert!(!sound::same_calls(&events, Some(me), &records[..2]));
    let mut r = records.clone();
    r.push(SoundRecord::Stop { actor: me });
    assert!(!sound::same_calls(&events, Some(me), &r));
    assert!(sound::same_calls(&[], Some(me), &[]));
    // Events that cannot be split: a transient placement with no start before it. The calls it
    // leaves before the error are as many as the records, which must not make them agree.
    let torn = vec![sound_event(None, 8), placed(8, SoundOrigin::None, at)];
    let one = [start(8, SoundOrigin::None, None)];
    assert!(sound::split(&torn, Some(me), &mut Vec::new(), false).is_err());
    assert!(!sound::same_calls(&torn, Some(me), &one));
}

/// `M_AddToBox` (m_bbox.c:51-58) tests `x < left` first and only otherwise `x > right`: a point
/// whose x equals the current left edge is not below it, so it can raise the right edge, which
/// is still `MININT` after a first point. Likewise for y. Two equal points give a box of one
/// point: left = right and bottom = top.
/// Kills sound.rs `add_to_box` `x < b[2]` -> `<=`.
#[test]
fn add_to_box_second_point_with_the_same_x_sets_the_right_edge() {
    let mut b = sound::CLEAR_BOX;
    sound::add_to_box(&mut b, [10, 0]);
    assert_eq!(b, [i32::MIN, 0, 10, i32::MIN]);
    sound::add_to_box(&mut b, [10, 0]);
    assert_eq!(b, [0, 0, 10, 10]);
}

/// A removed-or-not actor's current x, y are readable by handle (`Simulation::sound_origin_xy`):
/// the player's body stands where its THING put it (`P_SpawnPlayer`, p_mobj.c:662-665: x, y =
/// the thing's x, y in 16.16) and is the listener's own position; a handle that does not
/// resolve gives `None`.
/// Kills sound.rs `Simulation::sound_origin_xy` -> `None`, `Some([-1; 2])`, `Some([0; 2])` and
/// `Some([1; 2])`.
#[test]
fn sound_origin_xy_reads_the_actor_position() {
    let c = &cases()[0];
    let s = setup(&with_start(room(c.map), 0, c.position, 0), V3);
    let me = body(&s);
    let at = [
        i32::from(c.position[0]) << 16,
        i32::from(c.position[1]) << 16,
    ];
    assert!(at[0] > 1 || at[0] < -1 || at[1] > 1 || at[1] < -1);
    assert_eq!(s.sound_origin_xy(me), Some(at));
    assert_eq!(s.sound_listener().unwrap().xy, at);
    // Every other actor reads its own position (the loop checks equality with the actor's
    // record; it does not check that the position differs from the player's).
    let mut others = 0;
    for id in s.actor_order().filter(|id| *id != me) {
        let xy = s.sound_origin_xy(id).unwrap();
        assert_eq!(
            xy,
            s.actor(id).unwrap().position().map(doom_types::Fixed::bits)
        );
        others += 1;
    }
    assert!(others > 0);
    assert_eq!(s.sound_origin_xy(ActorId::from_parts(60_000, 1)), None);
}

/// An explicit operation that plays a sound (here the entry of a barrel into its explosion
/// state, whose action `A_Scream` starts the barrel's death sound `sfx_barexp`: info.c:945,
/// 1903; p_enemy.c:1535-1560; `P_SetMobjState` runs the action at once) leaves its record,
/// with the placement read at the call, in the pending list, beside the event that made it;
/// the next tic's preflight requires the two to agree (`sound::same_calls`) and publishes the
/// record first.
/// Kills demon_expanded_map.rs `Simulation::take_operation_sounds` -> `Ok(vec![])`.
#[test]
fn an_explicit_operations_sound_is_kept_as_a_pending_record() {
    // The barrel's explosion state (`S_BEXP2`) in this simulation's numbering: the state after
    // the one a kill enters (808 in the M3h9b trace of a kill).
    const BEXP2: u16 = 809;
    let barexp = lifecycle::source_sound("sfx_barexp").unwrap();
    let mut s = setup(&base(), V3);
    let me = body(&s);
    let listener = s.sound_listener().unwrap();
    let barrel = s
        .actor_order()
        .find(|id| *id != me)
        .expect("the room holds a barrel");
    s.enter_demon_state(barrel, StateId(BEXP2)).unwrap();
    assert_eq!(sounds(s.pending_events()), [(Some(barrel), barexp)]);
    let want = SoundRecord::Start {
        sfx: barexp,
        origin: SoundOrigin::Actor(barrel),
        at: Some(sound::SoundPlacement {
            origin: s.sound_origin_xy(barrel).unwrap(),
            listener: listener.xy,
            listener_angle: listener.angle,
        }),
    };
    assert_eq!(s.pending_sound_records(), [want]);
    // The transient placement left the pending events, and what is kept is exactly the calls
    // of the pending events: the check the next tic makes (`sound::same_calls`).
    assert!(
        !s.pending_events()
            .iter()
            .any(|e| matches!(e, Event::SoundRecord(_)))
    );
    assert!(sound::same_calls(
        s.pending_events(),
        Some(me),
        s.pending_sound_records()
    ));
}

/// The DSIM41 reader bounds a record count by the number of fields in the frame (a Rust-only
/// safety rule of the reader, `Limit`): a count above that number is refused as `Limit`, a count
/// equal to it is not (the records it names are then missing, which is another refusal).
/// Kills diagnostic.rs `validate_sound_records` `count > self.fields.len()` -> `==` and `>=`.
#[test]
fn dsim41_reader_bounds_the_record_count_by_the_field_count() {
    let limits = DiagnosticLimits::default();
    let read = |f: &[(String, Vec<u8>)]| Diagnostic::decode_episode1_v3(&encode41(f), limits).err();
    let count = |n: u64| n.to_le_bytes().to_vec();
    let s = setup(&base(), V3);
    let frame = owned_fields(&s);
    assert_eq!(read(&frame), None);
    let n = frame.len() as u64;
    for prefix in ["events.sounds.last", "events.sounds.pending"] {
        let path = format!("{prefix}.count");
        let mut f = frame.clone();
        set(&mut f, &path, count(n + 1));
        assert_eq!(read(&f), Some(DiagnosticError::Limit), "{prefix} above");
        let mut f = frame.clone();
        set(&mut f, &path, count(n));
        let e = read(&f);
        assert!(
            e.is_some() && e != Some(DiagnosticError::Limit),
            "{prefix} equal: {e:?}"
        );
    }
}
