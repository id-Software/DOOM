//! M5a: level loading against the genuine startup (investigation §7 items 41 and 45). The
//! episode selects the sky (g_game.c:1464-1478) and only Episode 1 is admitted; a two-sided
//! line without a back side is refused by doom-map's parser before the renderer sees it. The
//! room is the committed synthetic `amp.wad` with the committed
//! graphics set (no id art); the variants are rebuilt in memory.
use doom_render::{GameMode, Level, LumpDirectory, ResourceError, Resources};
use std::path::{Path, PathBuf};

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

/// The room's lumps (name, bytes) in directory order.
fn room_lumps() -> Vec<([u8; 8], Vec<u8>)> {
    let bytes = std::fs::read(reference("m5a_wads/amp.wad")).expect("room WAD");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("room");
    wad.lumps()
        .iter()
        .map(|l| (*l.name().as_bytes(), bytes[l.range()].to_vec()))
        .collect()
}

/// A PWAD of `lumps`.
fn pwad(lumps: &[([u8; 8], Vec<u8>)]) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();
    let mut dir: Vec<u8> = Vec::new();
    for (name, bytes) in lumps {
        dir.extend(((12 + data.len()) as i32).to_le_bytes());
        dir.extend((bytes.len() as i32).to_le_bytes());
        dir.extend(name);
        data.extend(bytes);
    }
    let mut o = b"PWAD".to_vec();
    o.extend((lumps.len() as i32).to_le_bytes());
    o.extend(((12 + data.len()) as i32).to_le_bytes());
    o.extend(data);
    o.extend(dir);
    o
}

fn lump_mut<'a>(lumps: &'a mut [([u8; 8], Vec<u8>)], name: &str) -> &'a mut Vec<u8> {
    let mut want = [0u8; 8];
    want[..name.len()].copy_from_slice(name.as_bytes());
    &mut lumps.iter_mut().find(|(n, _)| *n == want).expect(name).1
}

/// Resources and the map of a room WAD (the room's lumps, then the graphics set).
fn load(wad_bytes: &[u8]) -> (Resources, Result<doom_map::Map, doom_map::MapError>) {
    let gfx_bytes = std::fs::read(reference("m5a_gfx.wad")).expect("graphics WAD");
    let wad = doom_wad::Wad::parse(wad_bytes, doom_wad::Limits::default()).expect("room");
    let gfx = doom_wad::Wad::parse(&gfx_bytes, doom_wad::Limits::default()).expect("gfx");
    let mut dir = LumpDirectory::new();
    dir.add_wad(&wad).expect("room directory");
    dir.add_wad(&gfx).expect("graphics directory");
    let res = Resources::load(dir, GameMode::Registered).expect("resources");
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default());
    (res, map)
}

#[test]
fn only_episode_one_is_admitted() {
    // G_InitNew: episode 1 SKY1, 2 SKY2, 3 SKY3, 4 SKY4 (g_game.c:1464-1478).
    let (res, map) = load(&pwad(&room_lumps()));
    let map = map.expect("map");
    assert!(Level::load(&res, &map, 1).is_ok());
    for episode in [0, 2, 3, 4, 255] {
        assert_eq!(
            Level::load(&res, &map, episode).err(),
            Some(ResourceError::SkyEpisode(episode)),
            "{episode}"
        );
    }
}

#[test]
fn a_two_sided_line_without_a_back_side_is_refused_by_doom_map_first() {
    // LINEDEFS: 14 bytes (v1, v2, flags, special, tag, sidenum[2]). The genuine load reads
    // `sides[-1]` only through a seg (p_setup.c:189-190; P_LoadLineDefs sets
    // `backsector = 0`, p_setup.c:425-428), so the renderer refuses only at a seg
    // (`Geometry::load`). doom-map's parser, outside M5a, refuses every two-sided line
    // without a back side before the renderer sees the map (investigation §7 item 41).
    let mut lumps = room_lumps();
    let linedefs = lump_mut(&mut lumps, "LINEDEFS");
    let mut extra = linedefs[..14].to_vec();
    extra[4..6].copy_from_slice(&4i16.to_le_bytes());
    extra[12..14].copy_from_slice(&(-1i16).to_le_bytes());
    let record = linedefs.len() / 14;
    linedefs.extend(extra);
    let (_, map) = load(&pwad(&lumps));
    let e = map.expect_err("doom-map refuses the line");
    assert_eq!(e.lump, "LINEDEFS");
    assert_eq!(e.record, Some(record));
}
