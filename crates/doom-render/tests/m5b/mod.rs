//! Shared support for the M5b display tests: the 59 items of `tools/reference/m5b_reference.py`,
//! their genuine lump lists with `m5b_gfx.wad`, readers of the DOOM-M5B-FRAMES-v1 goldens and the
//! bespoke scenario's cheat effects.
#![allow(dead_code, reason = "each test target uses a subset")]

use crate::common::{self, reference};
use doom_render::hu_stuff::Message;
use doom_render::{GameMode, Level, LumpDirectory, Resources};
use doom_sim::demo::parse_demo_episode1_v2;
use doom_sim::{MapContextOptions, PlayerSlot, Simulation, Skill, TicCommand, WeaponInventory};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// One item: its demo lump name, map WAD and demo.
pub struct Item {
    pub lump: String,
    pub wad: PathBuf,
    pub lmp: PathBuf,
}

/// The 59 items of `m5b_reference.py`.
pub fn items() -> Vec<Item> {
    let mut out: Vec<Item> = common::items()
        .into_iter()
        .map(|i| Item {
            lump: i.lump,
            wad: i.wad,
            lmp: i.lmp,
        })
        .collect();
    let text = std::fs::read_to_string(reference("m5b_manifest.json")).expect("manifest");
    let m: serde_json::Value = serde_json::from_str(&text).expect("manifest JSON");
    for room in m["rooms"].as_array().expect("rooms") {
        let rel = |v: &serde_json::Value| {
            reference(
                v.as_str()
                    .expect("path")
                    .trim_start_matches("tests/reference/"),
            )
        };
        out.push(Item {
            lump: room["script"]["lump"].as_str().expect("lump").into(),
            wad: rel(&room["file"]),
            lmp: rel(&room["script"]["lmp"]),
        });
    }
    out
}

/// One genuine frame: the F row (with its U and M rows) and the P row.
#[derive(Default)]
pub struct Golden {
    pub sha256: String,
    pub masked: String,
    pub whole: bool,
    pub columns: Vec<u16>,
    pub mapped: Vec<u16>,
    pub fuzzpos: u32,
    pub counters: doom_render::FrameCounters,
    pub status: [i64; 5],
}

pub struct Stream {
    pub frames: BTreeMap<i64, Golden>,
    pub messages: Vec<(i64, i32, String)>,
}

pub fn read_stream(lump: &str, sb: u8) -> Stream {
    let text = std::fs::read_to_string(reference(&format!("m5b_frames/{lump}.sb{sb}.frames")))
        .expect("frames");
    let mut s = Stream {
        frames: BTreeMap::new(),
        messages: Vec::new(),
    };
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("#schema,DOOM-M5B-FRAMES-v1"));
    for l in lines {
        let c: Vec<&str> = l.split(',').collect();
        let tic = || c[1].parse::<i64>().expect("tic");
        let n = |i: usize| c[i].parse::<i64>().expect("number");
        match c[0] {
            "F" => {
                let g = Golden {
                    sha256: c[2].into(),
                    masked: c[3].into(),
                    fuzzpos: n(4) as u32,
                    whole: c[14] == "1",
                    counters: doom_render::FrameCounters {
                        mapped: n(5) as u32,
                        visplanes: n(7) as u32,
                        drawsegs: n(8) as u32,
                        vissprites: n(9) as u32,
                        openings: n(10) as u32,
                        solidsegs_peak: n(11) as u32,
                        drawseg_drops: n(12) as u32,
                        vissprite_drop: c[13] == "1",
                    },
                    ..Golden::default()
                };
                assert!(s.frames.insert(tic(), g).is_none(), "{lump}: {l}");
            }
            "U" => s
                .frames
                .get_mut(&tic())
                .expect("U")
                .columns
                .push(n(2) as u16),
            "M" => {
                s.frames.get_mut(&tic()).expect("M").mapped =
                    c[2].split(' ').map(|v| v.parse().expect("line")).collect();
            }
            "P" => s.frames.get_mut(&tic()).expect("P").status = std::array::from_fn(|i| n(i + 2)),
            "X" => s.messages.push((tic(), n(2) as i32, c[3..].join(","))),
            _ => {}
        }
    }
    s
}

pub fn results() -> serde_json::Value {
    let text = std::fs::read_to_string(reference("m5b_results.json")).expect("results");
    serde_json::from_str(&text).expect("results JSON")
}

/// The genuine lump list (map WAD, `m5b_gfx.wad`, the demo as a single lump), the resources,
/// the level, the simulation at tic 0 and the demo's commands.
pub fn load(item: &Item) -> (Arc<Resources>, Arc<Level>, Simulation, Vec<TicCommand>) {
    let wad_bytes = std::fs::read(&item.wad).expect("WAD");
    let gfx_bytes = std::fs::read(reference("m5b_gfx.wad")).expect("gfx");
    let lmp = std::fs::read(&item.lmp).expect("demo");
    let wad = doom_wad::Wad::parse(&wad_bytes, doom_wad::Limits::default()).expect("WAD");
    let gfx = doom_wad::Wad::parse(&gfx_bytes, doom_wad::Limits::default()).expect("gfx");
    let mut dir = LumpDirectory::new();
    dir.add_wad(&wad).expect("map WAD");
    dir.add_wad(&gfx).expect("gfx");
    dir.add_single_lump(&item.lmp.to_string_lossy(), &lmp)
        .expect("demo lump");
    let res = Resources::load(dir, GameMode::Registered).expect("resources");
    let map_number = lmp[3];
    let map = doom_map::Map::from_wad(
        &wad,
        &format!("E{}M{map_number}", lmp[2]),
        Default::default(),
    )
    .expect("map");
    let level = Level::load(&res, &map, lmp[2]).expect("level");
    let (header, demo) = parse_demo_episode1_v2(&lmp, map_number).expect("demo");
    let skill = [Skill::Baby, Skill::Easy, Skill::Medium, Skill::Hard][usize::from(header.skill)];
    let mut sim = Simulation::original_player_episode1_v2_map(
        map,
        map_number,
        skill,
        MapContextOptions::default(),
    )
    .expect("admission");
    sim.spawn_map_things().expect("THINGs");
    sim.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("weapons");
    sim.spawn_level_specials().expect("specials");
    (Arc::new(res), Arc::new(level), sim, demo.commands)
}

/// The bespoke scenario's cheats (m5b_wad.py BESPOKE: `idbeholds` before tic 10, `iddqd` before
/// tic 40), applied to the status bar inputs and the message of the tic. `idbeholds` gives
/// `powers[pw_strength] = 1` (st_stuff.c:636-645, p_inter.c:319-323), which `P_PlayerThink`
/// increments every tic from that tic on (p_user.c:339-340), so `ST_Ticker` of tic `t` reads
/// `t - 8`; message STSTR_BEHOLDX (st_stuff.c:647, d_englsh.h:351). `iddqd` sets `CF_GODMODE`
/// (st_stuff.c:549-558; health already 100), message STSTR_DQDON (d_englsh.h:341).
pub fn cheats(tic: i64, inputs: &mut doom_render::st_stuff::StatusInputs, m: &mut Option<Message>) {
    if tic >= 10 {
        inputs.powers[1] = (tic - 8) as i32;
    }
    if tic >= 40 {
        inputs.godmode = true;
    }
    let text = match tic {
        10 => "Power-up Toggled",
        40 => "Degreelessness Mode On",
        _ => return,
    };
    *m = Some(Message { code: -1, text });
}
