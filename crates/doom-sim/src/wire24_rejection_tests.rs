//! Wire 24 (`PlayerMonstersEpisode1V2`, DSIM40) rejections, rollback and older-wire
//! isolation (M4b plan step 9). Rust-only safety tests: every expectation comes from
//! a citation or written admission rule that already exists in
//! docs/M4B_REFERENCE_INVESTIGATION.md (named at each test), from the committed
//! synthetic WADs read as data, or from the genuine-engine observations recorded in
//! docs/validation/M4B.md; none is captured Rust output. The refusals the other M4b
//! test files already pin (store full at the fogs and at the fireball, the ninth
//! `spechit`, the damage bound, the skill and map-number domains, the demo skill byte,
//! the boss-floor height bound) are not repeated here.
use crate::PursuitError;
use crate::diagonal_room::Lumps;
use crate::identity::OriginalActorType;
use crate::*;

const V1: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V1;
const V2: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V2;

fn reference(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(name),
    )
    .expect("reference file")
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
fn lump<'a>(l: &'a mut Lumps, name: &str) -> &'a mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == name).unwrap().1
}
fn load(l: &Lumps, policy: OriginalExecutionProfile) -> Result<Simulation, SimError> {
    let map = || l.try_map().unwrap_or_else(|_| map_of(l));
    let result = if policy == V2 {
        Simulation::original_player_episode1_v2_map(
            map(),
            1,
            Skill::Medium,
            MapContextOptions::default(),
        )
    } else {
        Simulation::original_player_episode1_map(map(), MapContextOptions::default())
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
fn step(s: &mut Simulation, command: TicCommand) -> Result<(), SimError> {
    s.advance([
        command,
        TicCommand::default(),
        TicCommand::default(),
        TicCommand::default(),
    ])
}
fn demo(script: &str) -> Vec<TicCommand> {
    crate::demo::parse_demo_episode1_v2(&reference(&format!("m4b_scripts/{script}.lmp")), 1)
        .unwrap()
        .1
        .commands
}
fn body(s: &Simulation) -> Actor {
    *s.actor(s.original_player.unwrap().mo.unwrap()).unwrap()
}
/// What a refused tic must leave: the canonical bytes, the RNG and the tic.
fn state(s: &Simulation) -> (Vec<u8>, DoomRng, u64) {
    (s.snapshot().canonical_bytes(), s.rng(), s.tic())
}

/// The Rust-only witness hall (`tests/reference/m3h6_witness.bin`, 200 lines): a
/// 2,048-unit pistol shot east from (-1000, 100) straddles 130 teeth lines, so
/// `P_LineAttack` would write the 129th entry of `intercepts[128]` (p_maputl.c:
/// 603-606, 668-671; M4b note §2 "Hazards and witnesses": "129th intercept ...
/// inherited"; decision I7 lists the intercept bound among wire 24's witnesses).
/// Wire 24 inherits wire 23's witness: the shot's tic is refused with
/// `MapScaleError::Intercepts`, every byte, the RNG and the tic unchanged, and a
/// retry is refused again. The same hall under wire 23 refuses the same tic, so the
/// bound did not move; before that tic every tic is admitted.
///
/// Public API cannot build this: the hall is a raw map lump set edited with the
/// crate-private `Lumps`.
#[test]
fn wire_24_refuses_the_129th_intercept_and_keeps_the_state() {
    let mut hall = Lumps::parse(include_bytes!("../../../tests/reference/m3h6_witness.bin"));
    assert_eq!(hall.map().linedefs().len(), 200);
    hall.add_thing([-1000, 100, 0, 1, 7]);
    let fire = TicCommand {
        buttons: BT_ATTACK,
        ..TicCommand::default()
    };
    let mut first_refusal = [None, None];
    for (slot, policy) in [V1, V2].into_iter().enumerate() {
        let mut s = setup(&hall, policy);
        for tic in 1..=40 {
            let before = state(&s);
            match step(&mut s, fire) {
                Ok(()) => {}
                Err(e) => {
                    assert_eq!(
                        e,
                        SimError::MapScale(MapScaleError::Intercepts),
                        "{policy:?}"
                    );
                    assert_eq!(state(&s), before, "{policy:?} tic {tic}");
                    assert_eq!(step(&mut s, fire), Err(e), "{policy:?} retry");
                    assert_eq!(state(&s), before, "{policy:?} retry");
                    first_refusal[slot] = Some(tic);
                    break;
                }
            }
        }
    }
    assert!(first_refusal[1].is_some(), "wire 24 never refused the shot");
    assert!(first_refusal[1].unwrap() > 1, "the first tic fires nothing");
    assert_eq!(first_refusal[0], first_refusal[1]);
}

/// Decision I7 / finding 16 ("the voodoo-doll case cannot arise"): the level policy,
/// inherited by wire 24, refuses a second type-1 start at spawn with
/// `PlayerBodyError::Binding`, and the refused batch changes nothing.
#[test]
fn wire_24_refuses_a_second_player_one_start_and_changes_nothing() {
    let mut l = wad("tele");
    l.add_thing([320, 64, 0, 1, 7]);
    let mut s = load(&l, V2).unwrap();
    let before = state(&s);
    assert_eq!(
        s.spawn_map_things().err(),
        Some(SimError::PlayerBody(PlayerBodyError::Binding))
    );
    assert_eq!(state(&s), before);
    assert_eq!(s.actor_order().count(), 0);
}

/// §2 T2.3-T2.4 (p_telept.c:74-97): `EV_Teleport` has no tag-0 guard, so a tag-0
/// line searches the tag-0 sectors in index order and the first landing wins. TELE2
/// (C-observed, validation/M4B.md: "the player lands at tic 25 on the first (THINGS
/// order) landing of the second sector, angle 0") crosses line 5 (tag 2; sectors
/// 4-6 are tagged 2 and each holds a landing: the first in THINGS order sits at
/// (1600, 640), angle 0). With the line and sectors 4-6 retagged 0, sectors 0, 1 and
/// 3 are also tag 0 but hold no landing, so the walk must land on that same landing:
/// a port that guarded tag 0 would not teleport at all.
#[test]
fn a_tag_zero_teleport_searches_the_tag_zero_sectors() {
    let commands = demo("TELE2");
    let land = |l: &Lumps| {
        let mut s = setup(l, V2);
        let mut at = None;
        for (i, c) in commands.iter().enumerate() {
            step(&mut s, *c).unwrap();
            if s.events()
                .iter()
                .any(|e| matches!(e, Event::Mover(MoverEvent::Teleported { .. })))
            {
                at = Some((
                    i + 1,
                    body(&s).position().map(Fixed::bits),
                    body(&s).angle(),
                ));
                break;
            }
        }
        at
    };
    let base = wad("tele");
    let tagged = land(&base).expect("TELE2 teleports");
    // C-observed: the player lands at dump tic 25 (DSIM tic 26) on (1600, 640), angle 0.
    assert_eq!(tagged, (26, [1600 << 16, 640 << 16], 0));
    let mut zero = base.clone();
    {
        let v = lump(&mut zero, "LINEDEFS");
        assert_eq!(i16::from_le_bytes([v[14 * 5 + 8], v[14 * 5 + 9]]), 2);
        v[14 * 5 + 8..14 * 5 + 10].copy_from_slice(&0_i16.to_le_bytes());
        let s = lump(&mut zero, "SECTORS");
        for sector in [4, 5, 6] {
            assert_eq!(
                i16::from_le_bytes([s[26 * sector + 24], s[26 * sector + 25]]),
                2
            );
            s[26 * sector + 24..26 * sector + 26].copy_from_slice(&0_i16.to_le_bytes());
        }
    }
    assert_eq!(land(&zero), Some(tagged));
}

/// The wire-24-only content of a DSIM40 capture is outside the frozen readers'
/// domain (STATE_FORMAT "DSIM v40": "every earlier version-specific reader, the v39
/// reader included, keeps its historical contract"; the new actor types are listed
/// there as new in v40). A capture of a wire-24 state holding a landing
/// (`MT_TELEPORTMAN`, doomednum 14) or a Baron (`MT_BRUISER`, 3003; a landing-free
/// room), relabelled as a DSIM39 capture (version 39, profile 23, the v39
/// context, no `level.map`), is refused by the DSIM39 reader, while the same
/// relabelling of a state with none of them is read (control: the relabelling itself
/// is a valid DSIM39 capture).
///
/// Public API cannot build this: it needs the crate-private `Lumps` editor for the
/// control room.
///
/// Not covered: a fog (`MT_TFOG`) alone, because a fog exists only after a teleport,
/// which needs a landing in the same capture.
#[test]
fn the_dsim39_reader_refuses_wire_24_only_actor_types() {
    let limits = DiagnosticLimits::default();
    let relabel = |s: &Simulation| {
        let snap = s.snapshot();
        let fields: Vec<(String, Vec<u8>)> = snap
            .fields()
            .filter(|f| f.path() != "level.map")
            .map(|f| {
                let v = match f.path() {
                    "profile" => 23u32.to_le_bytes().to_vec(),
                    "context.schema" => PLAYER_EPISODE1_CONTEXT_SCHEMA.to_vec(),
                    _ => f.value().to_vec(),
                };
                (f.path().to_string(), v)
            })
            .collect();
        let mut out = b"DSIM".to_vec();
        out.extend(39u32.to_le_bytes());
        out.extend((fields.len() as u64).to_le_bytes());
        for (path, value) in fields {
            out.extend((path.len() as u32).to_le_bytes());
            out.extend(path.as_bytes());
            out.extend((value.len() as u64).to_le_bytes());
            out.extend(value);
        }
        out
    };
    let read = |b: &[u8]| Diagnostic::decode_episode1(b, limits).map(|_| ());
    // Control: a wire-24 state with only wire-23 content (the M3h9b room 13).
    let mut room = crate::player_movers_tests::room(13);
    room.add_thing([1000, 64, 180, 1, 7]);
    let control = setup(&room, V2);
    read(&relabel(&control)).expect("the relabelled control is a DSIM39 capture");
    // Landings at spawn.
    let landing = setup(&wad("tele"), V2);
    assert!(landing.actor_order().any(|id| {
        landing
            .actor(id)
            .unwrap()
            .life
            .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_TELEPORTMAN.ordinal())
    }));
    assert!(read(&relabel(&landing)).is_err(), "landing");
    // A Baron alone (no landing, no fog).
    let mut baron = wad("baron");
    {
        let t = lump(&mut baron, "THINGS");
        assert!(
            t.chunks_exact(10)
                .any(|r| i16::from_le_bytes([r[6], r[7]]) == 3003)
        );
    }
    let s = setup(&baron, V2);
    assert!(s.actor_order().any(|id| {
        s.actor(id)
            .unwrap()
            .life
            .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_BRUISER.ordinal())
    }));
    // No landing in it: the refusal below is the Baron's alone.
    assert!(!s.actor_order().any(|id| {
        s.actor(id)
            .unwrap()
            .life
            .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_TELEPORTMAN.ordinal())
    }));
    assert!(read(&relabel(&s)).is_err(), "baron");
    // The wire-24 reader reads the unrelabelled captures of both.
    for s in [&landing, &s] {
        Diagnostic::decode_episode1_v2(&s.snapshot().canonical_bytes(), limits).unwrap();
    }
}

/// Older-wire isolation (decision I2: "every new behaviour gets its own predicate,
/// so wires <=23 cannot change a byte"; M4b note §1.2): under every older
/// player-body policy that admits the control room (the M3h9b room 13 with a start;
/// the policies below wire 18 refuse the room itself for lack of movers, which says
/// nothing about wire-24 content, and are skipped), the same room with the teleport
/// special 97, a landing (14) or a Baron (3003) is refused: special 97 at load with
/// `Pursuit(Specials)`, a landing or a Baron at the THING batch with
/// `UnsupportedType`, the refused batch leaving the canonical state as it was. The
/// control: wire 24 loads and spawns `tele.wad` (landings) and `baron.wad` (a Baron).
#[test]
fn every_older_policy_refuses_the_wire_24_content() {
    let mut room = crate::player_movers_tests::room(13);
    room.add_thing([1000, 64, 180, 1, 7]);
    let build = |l: &Lumps, policy: OriginalExecutionProfile| {
        Simulation::with_original_registry(
            l.map(),
            crate::player_map_scale::actor_slots(policy),
            OriginalExecutionRegistry::new(policy).unwrap(),
            ActorOptions::default(),
            MapContextOptions::default(),
        )
    };
    let mut checked = 0;
    for policy in crate::player_episode1_v2_tests::ALL {
        if policy == V2 || !policy.has_player_body() {
            continue;
        }
        // Control: the room itself loads and spawns under this policy.
        let Ok(mut control) = build(&room, policy) else {
            continue;
        };
        if control.spawn_map_things().is_err() {
            continue;
        }
        checked += 1;
        // Line 97 (tagged): refused at load.
        let mut tele = room.clone();
        {
            let v = lump(&mut tele, "LINEDEFS");
            v[14 * 4 + 6..14 * 4 + 8].copy_from_slice(&97_i16.to_le_bytes());
            v[14 * 4 + 8..14 * 4 + 10].copy_from_slice(&5_i16.to_le_bytes());
        }
        assert_eq!(
            build(&tele, policy).err(),
            Some(SimError::Pursuit(PursuitError::Specials)),
            "{policy:?}: special 97"
        );
        // The landing and the Baron: refused at the batch, state unchanged.
        for doomednum in [14, 3003] {
            let mut l = room.clone();
            l.add_thing([1000, 128, 0, doomednum, 7]);
            let mut s = build(&l, policy).unwrap();
            let before = state(&s);
            assert_eq!(
                s.spawn_map_things().err(),
                Some(SimError::MapSpawn(MapSpawnError::UnsupportedType(
                    doomednum
                ))),
                "{policy:?} {doomednum}"
            );
            assert_eq!(state(&s), before, "{policy:?} {doomednum}");
        }
    }
    // Wires 18-23: movers, E1M1, barrels, level and Episode 1 part 1.
    assert_eq!(checked, 5);
    // Wire 24 admits a landing and a Baron where it loads them (the control WADs).
    for name in ["tele", "baron"] {
        setup(&wad(name), V2);
    }
}
