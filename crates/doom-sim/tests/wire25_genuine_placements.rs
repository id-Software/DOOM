//! Wire 25's sound placements and sector origins against the genuine C, on every committed M5c
//! item (the 64 items of tests/reference/m5c_sound, `wire25::ITEMS`), through doom-sim alone.
//!
//! - Every placed start record of the Rust simulation (`Simulation::sound_records`: origin x, y,
//!   listener x, y and angle, the positions `S_AdjustSoundParams` reads at the call,
//!   s_sound.c:304-312, 761-768) equals the placed S row of the genuine stream at the same tic,
//!   in order (columns `sfx`, `ox`, `oy`, `lx`, `ly`, `langle` of DOOM-M5C-SOUND-v1). This
//!   covers the listener read at a line crossing in the middle of a move (the mover's current
//!   record, not the store's, p_map.c:494-512).
//! - Every sector's sound origin equals `P_GroupLines`'s (p_setup.c:536-557) computed here
//!   from the map by the C's own loop nest and `M_AddToBox` (m_bbox.c:46-58), written again
//!   independently of the crate.
//!
//! Equivalence class: source equivalence against genuine traces (placements), and an independent
//! recomputation of the C arithmetic (sector origins). These tests exist because the mutation
//! tester runs only doom-sim's own tests against a doom-sim mutant; doom-sound's gate
//! (crates/doom-sound/tests/session_gate.rs) checks the same rows end to end.

mod wire25;

use doom_sim::sound::{SoundOrigin, SoundRecord};
use doom_sim::{demo::parse_demo_episode1_v2, *};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn reference() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference")
}

struct Item {
    map: doom_map::Map,
    number: u8,
    skill: Skill,
    commands: Vec<TicCommand>,
}

fn load(file: &str, lmp: &str) -> Item {
    let bytes = std::fs::read(reference().join(file)).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    let demo = std::fs::read(reference().join(lmp)).expect("demo");
    // Byte 3 of a version-110 header is the map (g_game.c G_BeginRecording).
    let number = demo[3];
    let map = doom_map::Map::from_wad(
        &wad,
        &format!("E1M{number}"),
        doom_map::MapLimits::default(),
    )
    .expect("map");
    let (header, demo) = parse_demo_episode1_v2(&demo, number).expect("demo parse");
    let skill = if header.skill == 3 {
        Skill::Hard
    } else {
        Skill::Medium
    };
    Item {
        map,
        number,
        skill,
        commands: demo.commands,
    }
}

type Placed = (u16, [i32; 2], [i32; 2], u32);

/// The placed S rows of a genuine stream by tic, and the number of tics the stream covers (the
/// R row of each tic).
fn genuine(name: &str) -> (BTreeMap<usize, Vec<Placed>>, usize) {
    let text = std::fs::read_to_string(reference().join(format!("m5c_sound/{name}.sound")))
        .expect("stream");
    let mut by_tic: BTreeMap<usize, Vec<Placed>> = BTreeMap::new();
    let mut tics = 0;
    for line in text.lines() {
        let c: Vec<&str> = line.split(',').collect();
        match c[0] {
            "R" => tics = tics.max(c[1].parse::<usize>().expect("tic") + 1),
            // S,tic,seq,sfx,okind,oid,outcome,draws,rnd,vol,sep,pitch,priority,cnum,handle,
            // samexy,gametic,ox,oy,lx,ly,langle
            "S" if c[17] != "-" => {
                let n = |i: usize| c[i].parse::<i64>().expect("number");
                by_tic.entry(c[1].parse().expect("tic")).or_default().push((
                    n(3) as u16,
                    [n(17) as i32, n(18) as i32],
                    [n(19) as i32, n(20) as i32],
                    n(21) as u32,
                ));
            }
            _ => {}
        }
    }
    (by_tic, tics)
}

#[test]
fn every_placed_start_equals_the_genuine_s_row() {
    let (mut items, mut compared, mut tics_compared) = (0, 0, 0);
    for (lump, file, lmp) in wire25::ITEMS {
        let item = load(file, lmp);
        let (want, covered) = genuine(lump);
        let mut got: BTreeMap<usize, Vec<Placed>> = BTreeMap::new();
        wire25::run_alone(
            &item.map,
            item.number,
            item.skill,
            &item.commands,
            |tic, sim| {
                // Capture `tic` (1-based after the tic's advance) holds the records the session
                // runs at gametic `tic - 1`.
                for r in sim.sound_records() {
                    if let SoundRecord::Start {
                        sfx,
                        origin,
                        at: Some(at),
                    } = *r
                    {
                        assert!(
                            !matches!(origin, SoundOrigin::None),
                            "{lump}: a NULL origin is never placed"
                        );
                        got.entry(tic - 1).or_default().push((
                            sfx,
                            at.origin,
                            at.listener,
                            at.listener_angle,
                        ));
                    }
                }
            },
        )
        .unwrap_or_else(|e| panic!("{lump}: {e}"));
        got.retain(|t, _| *t < covered);
        assert_eq!(got, want, "{lump}");
        compared += want.values().map(Vec::len).sum::<usize>();
        tics_compared += covered;
        items += 1;
    }
    eprintln!(
        "{items} items, {tics_compared} tics, {compared} placed starts equal the genuine S rows"
    );
    assert_eq!(items, 64);
    assert!(compared > 300, "{compared}");
}

/// `P_GroupLines` (p_setup.c:536-557) from the map, as the C writes it: for every sector, over
/// every line in order whose front or back sector is it, `M_AddToBox` of v1 and v2, then
/// `(right + left) / 2` and `(top + bottom) / 2`.
fn group_lines(map: &doom_map::Map) -> Vec<[i64; 2]> {
    // [top, bottom, left, right]: m_bbox.h BOXTOP 0, BOXBOTTOM 1, BOXLEFT 2, BOXRIGHT 3.
    const MININT: i64 = i32::MIN as i64;
    const MAXINT: i64 = i32::MAX as i64;
    let sector_of = |side: Option<u16>| side.map(|s| map.sidedefs()[usize::from(s)].sector);
    let mut out = Vec::new();
    for sector in 0..map.sectors().len() as u16 {
        let mut b = [MININT, MAXINT, MAXINT, MININT];
        for line in map.linedefs() {
            if sector_of(line.sides[0]) == Some(sector) || sector_of(line.sides[1]) == Some(sector)
            {
                for v in line.vertices {
                    let v = map.vertices()[usize::from(v)];
                    let (x, y) = (i64::from(v.x) << 16, i64::from(v.y) << 16);
                    if x < b[2] {
                        b[2] = x;
                    } else if x > b[3] {
                        b[3] = x;
                    }
                    if y < b[1] {
                        b[1] = y;
                    } else if y > b[0] {
                        b[0] = y;
                    }
                }
            }
        }
        out.push([(b[3] + b[2]) / 2, (b[0] + b[1]) / 2]);
    }
    out
}

#[test]
fn every_sector_origin_of_every_item_equals_p_group_lines() {
    let (mut sectors, mut two_sided_between_sectors) = (0, 0);
    for (lump, file, lmp) in wire25::ITEMS {
        let item = load(file, lmp);
        let sim = wire25::build(true, item.map.clone(), item.number, item.skill)
            .unwrap_or_else(|e| panic!("{lump}: {e}"));
        let want = group_lines(&item.map);
        assert_eq!(want.len(), item.map.sectors().len());
        for (s, xy) in want.iter().enumerate() {
            let got = sim.sector_sound_origin(s as u16).map(|o| o.map(i64::from));
            assert_eq!(got, Some(*xy), "{lump} sector {s}");
        }
        sectors += want.len();
        let side = |s: Option<u16>| s.map(|s| item.map.sidedefs()[usize::from(s)].sector);
        two_sided_between_sectors += item
            .map
            .linedefs()
            .iter()
            .filter(|l| {
                let (f, b) = (side(l.sides[0]), side(l.sides[1]));
                f.is_some() && b.is_some() && f != b
            })
            .count();
    }
    eprintln!("{sectors} sector origins over 64 items equal P_GroupLines");
    // The check means something only if lines between two sectors are present.
    assert!(two_sided_between_sectors > 0);
}
