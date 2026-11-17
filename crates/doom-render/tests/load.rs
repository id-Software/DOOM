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

#[test]
fn the_scrolling_line_budget_is_maxlineanims() {
    // P_SpawnSpecials registers every special-48 line in `linespeciallist[MAXLINEANIMS]` and
    // `I_Error`s on the 65th (p_spec.c:1336-1345; MAXLINEANIMS is 64): 64 lines load, 65 are
    // refused. Pins `Level::load`'s count and its bound. Extra lines are copies of the room's
    // first linedef with special 48 (bytes 6-8 of the 14-byte record); no seg refers to them.
    let base = room_lumps();
    let records = base
        .iter()
        .find(|(n, _)| n.starts_with(b"LINEDEFS"))
        .expect("LINEDEFS")
        .1
        .chunks(14)
        .filter(|r| i16::from_le_bytes([r[6], r[7]]) == 48)
        .count();
    assert_eq!(records, 0, "the room has no scrolling line of its own");
    let with_scrollers = |n: usize| {
        let mut lumps = base.clone();
        let linedefs = lump_mut(&mut lumps, "LINEDEFS");
        let mut extra = linedefs[..14].to_vec();
        extra[6..8].copy_from_slice(&48i16.to_le_bytes());
        for _ in 0..n {
            linedefs.extend(&extra);
        }
        let (res, map) = load(&pwad(&lumps));
        Level::load(&res, &map.expect("map"), 1)
    };
    // Zero: the genuine count is not touched.
    assert!(with_scrollers(0).is_ok());
    assert!(with_scrollers(1).is_ok());
    assert!(with_scrollers(64).is_ok());
    for n in [65, 66] {
        assert_eq!(
            with_scrollers(n).err(),
            Some(ResourceError::Map(
                "more than MAXLINEANIMS (64) scrolling lines (p_spec.c:1336-1345)".into()
            )),
            "{n}"
        );
    }
}

#[test]
fn scrolling_lines_move_their_front_side_one_unit_a_tic() {
    // P_UpdateSpecials (p_spec.c:1113-1124): every special-48 line adds FRACUNIT to its front
    // side's `textureoffset` each tic (a side with two such lines moves twice as fast; the
    // 16.16 value wraps in an `int`). The room's first linedef is copied twice with special
    // 48, so its front side has two scrolling lines.
    let mut lumps = room_lumps();
    let linedefs = lump_mut(&mut lumps, "LINEDEFS");
    let first = linedefs[..14].to_vec();
    let front = usize::from(u16::from_le_bytes([first[10], first[11]]));
    let mut extra = first;
    extra[6..8].copy_from_slice(&48i16.to_le_bytes());
    linedefs.extend(&extra);
    linedefs.extend(&extra);
    let (_, map) = load(&pwad(&lumps));
    let map = map.expect("map");
    let scrolling = |side: usize| {
        map.linedefs()
            .iter()
            .filter(|l| l.special == 48 && l.sides[0].map(usize::from) == Some(side))
            .count() as i32
    };
    assert_eq!(scrolling(front), 2);
    for tic in [0u64, 1, 3, 40] {
        let got = doom_render::p_spec::texture_offsets(&map, tic);
        assert_eq!(got.len(), map.sidedefs().len());
        for (side, s) in map.sidedefs().iter().enumerate() {
            let want = (i32::from(s.offsets[0]) << 16) + tic as i32 * scrolling(side) * 65536;
            assert_eq!(got[side], want, "tic {tic} side {side}");
        }
    }
    assert_eq!(
        doom_render::p_spec::texture_offsets(&map, 3)[front]
            - doom_render::p_spec::texture_offsets(&map, 0)[front],
        6 * 65536
    );
}
