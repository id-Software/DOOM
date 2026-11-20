//! Wire 25 (`PlayerMonstersEpisode1V3`, DSIM41) against wire 24 (DSIM40), Rust
//! against Rust, on every committed M5c sound item (the 64 items of the genuine streams in
//! tests/reference/m5c_sound, `wire25::ITEMS`): checks 1 and 2 of M5c note §4.4.4 at
//! every capture (see `wire25/mod.rs`). These prove that wire 25 changes nothing
//! but the `events.sounds` group and the corrected scream; they are not genuine
//! evidence for the records themselves (the genuine sound comparator is M5c step
//! 3b's). Every wire-25 capture is also read back by `Diagnostic::decode_episode1_v3`
//! and refused by `decode_episode1_v2`.

mod wire25;

use doom_sim::{demo::parse_demo_episode1_v2, *};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every committed genuine sound stream has its item in the table, and only those.
#[test]
fn the_item_table_is_the_genuine_stream_set() {
    let mut streams: Vec<String> = std::fs::read_dir(root().join("tests/reference/m5c_sound"))
        .expect("streams")
        .map(|e| e.expect("entry").file_name().into_string().expect("name"))
        .filter_map(|n| n.strip_suffix(".sound").map(str::to_owned))
        .collect();
    streams.sort();
    let mut lumps: Vec<String> = wire25::ITEMS.iter().map(|i| i.0.to_owned()).collect();
    lumps.sort();
    assert_eq!(streams, lumps);
}

#[test]
fn wire25_projects_to_wire24_on_every_m5c_item() {
    let items = wire25::ITEMS;
    let (mut captures, mut screams, mut starts, mut placed, mut stops, mut refused) =
        (0, 0, 0, 0, 0, 0);
    for (lump, file, lmp) in items {
        let reference = root().join("tests/reference");
        let bytes = std::fs::read(reference.join(file)).expect("wad");
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
        let demo = std::fs::read(reference.join(lmp)).expect("demo");
        // Byte 3 of a version-110 header is the map (g_game.c G_BeginRecording).
        let number = demo[3];
        let slot = format!("E1M{number}");
        let map =
            doom_map::Map::from_wad(&wad, &slot, doom_map::MapLimits::default()).expect("map");
        let (header, demo) = parse_demo_episode1_v2(&demo, number).expect("demo parse");
        let skill = if header.skill == 3 {
            Skill::Hard
        } else {
            Skill::Medium
        };
        let mut decoded = 0;
        let out = wire25::run_pair(&map, number, skill, &demo.commands, |_, s| {
            let bytes = s.snapshot().canonical_bytes();
            let limits = DiagnosticLimits::default();
            Diagnostic::decode_episode1_v3(&bytes, limits)
                .unwrap_or_else(|e| panic!("{lump}: DSIM41 reader: {e:?}"));
            assert!(Diagnostic::decode_episode1_v2(&bytes, limits).is_err());
            decoded += 1;
        })
        .unwrap_or_else(|e| panic!("{lump}: {e}"));
        assert_eq!(decoded, out.captures);
        captures += out.captures;
        screams += out.screams;
        starts += out.starts;
        placed += out.placed;
        stops += out.stops;
        refused += usize::from(out.refused.is_some());
        println!(
            "{lump}: {} captures, {} starts ({} placed), {} stops, {} screams, refused {:?}",
            out.captures, out.starts, out.placed, out.stops, out.screams, out.refused
        );
    }
    println!(
        "wire25_projection: {} items, {captures} captures, {starts} starts ({placed} placed), \
         {stops} stops, {screams} screams replaced, {refused} refused runs",
        items.len()
    );
}

/// M5c review, MAJOR 1: under wire 25 the DSIM41 reader counts its per-map bounds
/// (`K`, the counted items, the decorations and barrels) over the THINGS records
/// `P_LoadThings` loads, the prefix before the first of the ten Doom II types
/// (p_setup.c:317-337), since `totalkills` and `totalitems` count spawned things
/// only (p_mobj.c:790, 792). Every capture of the committed T4 map and script
/// (tests/reference/m5c_t4: a zombieman, type 65, a second zombieman and a
/// stimpack after the start) is read back by `decode_episode1_v3`; wire 25 spawns
/// the player and the first zombieman only, so `level.total_kills` is 1 although
/// the lump places three monsters.
#[test]
fn every_t4_capture_decodes_with_the_dsim41_reader() {
    let reference = root().join("tests/reference/m5c_t4");
    let bytes = std::fs::read(reference.join("truncate.wad")).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    let demo = std::fs::read(reference.join("TRUNC.lmp")).expect("demo");
    let number = demo[3];
    let map = doom_map::Map::from_wad(
        &wad,
        &format!("E1M{number}"),
        doom_map::MapLimits::default(),
    )
    .expect("map");
    let (header, demo) = parse_demo_episode1_v2(&demo, number).expect("demo parse");
    assert_eq!(header.skill, 2);
    let mut decoded = 0;
    let out = wire25::run_alone(&map, number, Skill::Medium, &demo.commands, |tic, s| {
        if tic == 0 {
            assert_eq!(s.actor_order().count(), 2, "the player and one zombieman");
            assert_eq!(s.level_statistics().expect("statistics").total_kills, 1);
        }
        let bytes = s.snapshot().canonical_bytes();
        Diagnostic::decode_episode1_v3(&bytes, DiagnosticLimits::default())
            .unwrap_or_else(|e| panic!("tic {tic}: DSIM41 reader: {e:?}"));
        decoded += 1;
    })
    .expect("run");
    assert_eq!(out.refused, None);
    assert_eq!(decoded, out.captures);
    assert_eq!(decoded, demo.commands.len() + 1);
    println!("m5c_t4: {decoded} captures decoded by decode_episode1_v3");
}
