//! Shared support for the M5c display tests: the 64 items of the genuine M5c full-frame goldens
//! (`tests/reference/m5c_frames`, DOOM-M5C-FRAMES-v1, written by the C only:
//! `tools/reference/m5c_reference.py`), their genuine lump list, the wire-25 simulation and a
//! reader of the goldens. The item table and the level setup are doom-sim's
//! (`crates/doom-sim/tests/wire25/mod.rs`, included by path, as doom-sound's tests do).
#![allow(dead_code, reason = "each test target uses a subset")]

use crate::common::reference;
use doom_render::{GameMode, Level, LumpDirectory, Resources};
use doom_sim::demo::parse_demo_episode1_v2;
use doom_sim::{Simulation, Skill, TicCommand};
use std::collections::BTreeMap;
use std::sync::Arc;

/// doom-sim's item table and level setup of the 64 M5c items.
#[path = "../../../doom-sim/tests/wire25/mod.rs"]
pub mod wire25;

/// One genuine frame: the F row (with its U and M rows), the P row and the Q row.
#[derive(Clone, Debug, Default)]
pub struct Golden {
    /// The whole-frame digest, or `undefined` for a tracker-flagged frame.
    pub sha256: String,
    /// The digest with the flagged columns zeroed (`-` when nothing was flagged).
    pub masked: String,
    /// The tracker-flagged columns (U rows).
    pub columns: Vec<u16>,
    /// The newly mapped lines (M row).
    pub mapped: Vec<u16>,
    /// `fuzzpos`.
    pub fuzzpos: u32,
    /// The renderer counters.
    pub counters: doom_render::FrameCounters,
    /// The P row: palette, palette set, face, `message_on`, `M_Random` index.
    pub status: [i64; 5],
    /// The Q row: the digest of the frame with the face rectangle zeroed (`undefined` when
    /// flagged).
    pub noface: String,
    /// The Q row: the cumulative sound draws and wipe draws after the tic.
    pub snd_draws: u64,
    /// See [`Golden::snd_draws`].
    pub wipe_draws: u64,
    /// The Q row: the face rectangle x0, y0, x1, y1 (x1 and y1 exclusive).
    pub face_rect: [usize; 4],
}

/// One golden file.
pub struct Stream {
    /// Frames by tic.
    pub frames: BTreeMap<i64, Golden>,
    /// The X rows: (tic, code, text).
    pub messages: Vec<(i64, i32, String)>,
}

/// Reads `tests/reference/m5c_frames/<lump>.sb<sb>.frames`.
pub fn read_stream(lump: &str, sb: u8) -> Stream {
    let text = std::fs::read_to_string(reference(&format!("m5c_frames/{lump}.sb{sb}.frames")))
        .expect("frames");
    let mut s = Stream {
        frames: BTreeMap::new(),
        messages: Vec::new(),
    };
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("#schema,DOOM-M5C-FRAMES-v1"));
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
                assert_eq!(c[14], "0", "{lump}: no whole-frame exclusion in M5c: {l}");
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
            "Q" => {
                let g = s.frames.get_mut(&tic()).expect("Q");
                g.noface = c[2].into();
                g.snd_draws = n(3) as u64;
                g.wipe_draws = n(4) as u64;
                g.face_rect = std::array::from_fn(|i| n(5 + i) as usize);
            }
            "X" => s.messages.push((tic(), n(2) as i32, c[3..].join(","))),
            "O" | "E" | "D" | "W" | "Z" => panic!("{lump}: unexpected row {l}"),
            _ => {}
        }
    }
    s
}

/// The frame with the face rectangle zeroed, as the genuine driver digests it for the Q row
/// (tools/reference/m5c_driver.c: rows y0..y1 and columns x0..x1, upper bounds exclusive).
pub fn noface(pixels: &[u8], r: [usize; 4]) -> Vec<u8> {
    let mut m = pixels.to_vec();
    for y in r[1]..r[3].min(doom_render::SCREENHEIGHT) {
        for x in r[0]..r[2].min(doom_render::SCREENWIDTH) {
            m[y * doom_render::SCREENWIDTH + x] = 0;
        }
    }
    m
}

/// How the genuine run of an item ends at both screen sizes, from
/// `tests/reference/m5c_results.json` (`views.<sb>.end`): every other item ends at its demo's
/// marker. BOSS1 ends where the level is exited (the simulation refuses the next tic).
pub fn genuine_end(lump: &str) -> Option<doom_render::RenderRefusal> {
    match lump {
        "M5VPLAN" => Some(doom_render::RenderRefusal::VisplaneOverflow),
        "M5OPEN" => Some(doom_render::RenderRefusal::OpeningOverflow),
        "M5SSEG" => Some(doom_render::RenderRefusal::SolidSegOverflow),
        _ => None,
    }
}

/// One loaded item.
pub struct Loaded {
    /// The lump name.
    pub lump: String,
    /// The resources of the genuine lump list.
    pub res: Arc<Resources>,
    /// The level.
    pub level: Arc<Level>,
    /// The wire-25 simulation after the level setup, before tic 0.
    pub sim: Simulation,
    /// The demo's commands.
    pub commands: Vec<TicCommand>,
    /// The map number.
    pub map: u8,
    /// The demo's skill byte.
    pub skill: u8,
    /// The lump directory's WAD files (item, gfx, sfx, music), as paths under tests/reference.
    pub wads: [String; 4],
}

/// Loads item `i` of the table with the genuine lump list of `m5c_reference.py`: the item WAD,
/// `m5b_gfx.wad`, `m5c_sfx.wad`, `m5c_music.wad` and the demo as a single lump.
pub fn load(i: usize) -> Loaded {
    let (lump, file, lmp_path) = wire25::ITEMS[i];
    let wads = [
        file.to_string(),
        "m5b_gfx.wad".to_string(),
        "m5c_sfx.wad".to_string(),
        "m5c_music.wad".to_string(),
    ];
    let mut dir = LumpDirectory::new();
    let files: Vec<Vec<u8>> = wads
        .iter()
        .map(|w| std::fs::read(reference(w)).expect("WAD"))
        .collect();
    let parsed: Vec<doom_wad::Wad> = files
        .iter()
        .map(|b| doom_wad::Wad::parse(b, doom_wad::Limits::default()).expect("WAD"))
        .collect();
    for wad in &parsed {
        dir.add_wad(wad).expect("WAD lumps");
    }
    let item_wad = parsed.first();
    let lmp = std::fs::read(reference(lmp_path)).expect("demo");
    dir.add_single_lump(lmp_path, &lmp).expect("demo lump");
    let res = Resources::load(dir, GameMode::Registered).expect("resources");
    let map_number = lmp[3];
    let map = doom_map::Map::from_wad(
        item_wad.expect("item WAD"),
        &format!("E{}M{map_number}", lmp[2]),
        Default::default(),
    )
    .expect("map");
    let level = Level::load(&res, &map, lmp[2]).expect("level");
    let (header, demo) = parse_demo_episode1_v2(&lmp, map_number).expect("demo");
    let skill = [Skill::Baby, Skill::Easy, Skill::Medium, Skill::Hard][usize::from(header.skill)];
    let sim = wire25::build(true, map, map_number, skill).expect("wire-25 simulation");
    Loaded {
        lump: lump.to_string(),
        res: Arc::new(res),
        level: Arc::new(level),
        sim,
        commands: demo.commands,
        map: map_number,
        skill: header.skill,
        wads,
    }
}

/// The bespoke scenario's cheats, as `m5b::cheats` (M5CHEAT's keys are outside every wire).
pub fn cheats(
    tic: i64,
    inputs: &mut doom_render::st_stuff::StatusInputs,
    m: &mut Option<doom_render::hu_stuff::Message>,
) {
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
    *m = Some(doom_render::hu_stuff::Message { code: -1, text });
}
