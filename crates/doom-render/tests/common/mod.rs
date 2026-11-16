//! Shared support for the M5a renderer tests and the comparison example: the 42 items (the
//! 20 M5a rooms and the 22 M4b synthetic scripts), the genuine global lump list of each run
//! (map WAD, `m5a_gfx.wad`, the demo as a single lump: the M5a runner's `--wad` order,
//! `tools/reference/m5a_reference.py` `run`), the simulation set-up of the existing M4b script
//! runner (`crates/doom-sim/tests/episode1v2_mechanism_goldens.rs`), and readers of the
//! committed genuine goldens (`tests/reference/m5a_views/*.view`, DOOM-M5A-VIEW-v1, and
//! `tests/reference/m5a_frames/*.frames`, DOOM-M5A-FRAMES-v1).
#![allow(dead_code, reason = "each test target uses a subset")]

use doom_render::{GameMode, Level, LumpDirectory, Resources};
use doom_sim::{
    MapContextOptions, PlayerSlot, Simulation, Skill, TicCommand, WeaponInventory,
    demo::parse_demo_episode1_v2,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// `tests/reference/<name>`.
pub fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

/// One item: a demo lump and its map WAD.
#[derive(Clone, Debug)]
pub struct Item {
    /// The demo lump name (the goldens' file stem).
    pub lump: String,
    /// The map WAD.
    pub wad: PathBuf,
    /// The demo.
    pub lmp: PathBuf,
    /// Whether it is an M5a room (else an M4b script).
    pub room: bool,
}

/// The `"file"` entry preceding `"lump": "<lump>"` in a manifest's text.
fn manifest_file(text: &str, lump: &str) -> Option<String> {
    let at = text.find(&format!("\"lump\": \"{lump}\""))?;
    let before = &text[..at];
    let f = before.rfind("\"file\": \"")? + "\"file\": \"".len();
    let end = before[f..].find('"')? + f;
    Some(before[f..end].to_string())
}

/// Every item with committed view goldens, in name order.
pub fn items() -> Vec<Item> {
    let rooms = std::fs::read_to_string(reference("m5a_manifest.json")).expect("manifest");
    let m4b: Vec<String> = ["m4b_manifest.json", "m4b_manifest2.json"]
        .iter()
        .map(|n| std::fs::read_to_string(reference(n)).expect("manifest"))
        .collect();
    let mut names: Vec<String> = std::fs::read_dir(reference("m5a_views"))
        .expect("views")
        .map(|e| {
            e.expect("entry")
                .path()
                .file_stem()
                .expect("stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|lump| {
            if let Some(file) = manifest_file(&rooms, &lump) {
                Item {
                    wad: reference(file.trim_start_matches("tests/reference/")),
                    lmp: reference(&format!("m5a_scripts/{lump}.lmp")),
                    room: true,
                    lump,
                }
            } else {
                let file = m4b
                    .iter()
                    .find_map(|t| manifest_file(t, &lump))
                    .unwrap_or_else(|| panic!("{lump}: in no manifest"));
                Item {
                    wad: reference(file.trim_start_matches("tests/reference/")),
                    lmp: reference(&format!("m4b_scripts/{lump}.lmp")),
                    room: false,
                    lump,
                }
            }
        })
        .collect()
}

/// A loaded item: its simulation at tic 0, the demo's commands, and the renderer resources.
pub struct Run {
    /// The simulation (tic 0).
    pub sim: Simulation,
    /// The demo's commands.
    pub commands: Vec<TicCommand>,
    /// Renderer resources of the run's global lump list.
    pub res: Arc<Resources>,
    /// The level tables.
    pub level: Arc<Level>,
}

/// Builds the genuine lump list, the resources, the level and the simulation of `item`.
pub fn load(item: &Item) -> Run {
    let wad_bytes = std::fs::read(&item.wad).expect("WAD");
    let gfx_bytes = std::fs::read(reference("m5a_gfx.wad")).expect("gfx");
    let lmp = std::fs::read(&item.lmp).expect("demo");
    let wad = doom_wad::Wad::parse(&wad_bytes, doom_wad::Limits::default()).expect("WAD");
    let gfx = doom_wad::Wad::parse(&gfx_bytes, doom_wad::Limits::default()).expect("gfx");
    let mut dir = LumpDirectory::new();
    dir.add_wad(&wad).expect("map WAD directory");
    dir.add_wad(&gfx).expect("gfx directory");
    dir.add_single_lump(&item.lmp.to_string_lossy(), &lmp)
        .expect("demo lump");
    let res = Resources::load(dir, GameMode::Registered)
        .unwrap_or_else(|e| panic!("{}: resources: {e:?}", item.lump));
    // The demo header selects the map (G_DoPlayDemo, g_game.c:1596-1598).
    let (episode, map_number) = (lmp[2], lmp[3]);
    let slot = format!("E{episode}M{map_number}");
    let map = doom_map::Map::from_wad(&wad, &slot, doom_map::MapLimits::default()).expect("map");
    let level = Level::load(&res, &map).unwrap_or_else(|e| panic!("{}: level: {e:?}", item.lump));
    let (header, demo) = parse_demo_episode1_v2(&lmp, map_number).expect("demo");
    let skill = match header.skill {
        0 => Skill::Baby,
        1 => Skill::Easy,
        2 => Skill::Medium,
        3 => Skill::Hard,
        _ => Skill::Nightmare,
    };
    let mut sim = Simulation::original_player_episode1_v2_map(
        map,
        map_number,
        skill,
        MapContextOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{}: admission: {e:?}", item.lump));
    sim.spawn_map_things().expect("THINGs");
    sim.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("weapons");
    sim.spawn_level_specials().expect("specials");
    Run {
        sim,
        commands: demo.commands,
        res: Arc::new(res),
        level: Arc::new(level),
    }
}

/// Advances the run by its `index`-th command (player 1; the other slots default).
pub fn advance(run: &mut Run, index: usize) -> Result<(), doom_sim::SimError> {
    run.sim.advance([
        run.commands[index],
        TicCommand::default(),
        TicCommand::default(),
        TicCommand::default(),
    ])
}

/// The columns of a DOOM-M5A-VIEW-v1 V row, after `V,tic`.
pub const VIEW_FIELDS: [&str; 21] = [
    "viewx",
    "viewy",
    "viewz",
    "viewangle",
    "viewheight",
    "deltaviewheight",
    "bob",
    "onground",
    "extralight",
    "fixedcolormap",
    "invisibility",
    "ps0state",
    "ps0tics",
    "ps0sx",
    "ps0sy",
    "ps1state",
    "ps1tics",
    "ps1sx",
    "ps1sy",
    "sector",
    "lightlevel",
];

/// One tic of a view record: the V row's values and the non-identity translations
/// (`kind`, index, translation).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ViewTic {
    /// The V values in [`VIEW_FIELDS`] order.
    pub values: Vec<i64>,
    /// A rows.
    pub anims: Vec<(String, usize, usize)>,
}

/// Reads `tests/reference/m5a_views/<lump>.view`: tic (-1 is the load) to its record.
pub fn read_view(lump: &str) -> BTreeMap<i64, ViewTic> {
    let text = std::fs::read_to_string(reference(&format!("m5a_views/{lump}.view"))).expect("view");
    let mut out: BTreeMap<i64, ViewTic> = BTreeMap::new();
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    assert_eq!(lines.next(), Some("#schema,DOOM-M5A-VIEW-v1"));
    for l in lines {
        let cols: Vec<&str> = l.split(',').collect();
        match cols[0] {
            "V" => {
                assert_eq!(cols.len(), 2 + VIEW_FIELDS.len(), "{lump}: {l}");
                let tic = cols[1].parse().expect("tic");
                out.entry(tic).or_default().values = cols[2..]
                    .iter()
                    .map(|v| v.parse().expect("value"))
                    .collect();
            }
            "A" => {
                let tic = cols[1].parse().expect("tic");
                out.entry(tic).or_default().anims.push((
                    cols[2].to_string(),
                    cols[3].parse().expect("index"),
                    cols[4].parse().expect("translation"),
                ));
            }
            _ => assert!(l.starts_with('#'), "{lump}: {l}"),
        }
    }
    out
}

/// The F row of one frame and its flagged columns.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FrameRow {
    /// SHA-256 of the 64,000 bytes.
    pub sha256: String,
    /// SHA-256 with the flagged columns zeroed (`-` when none).
    pub masked_sha256: String,
    /// `fuzzpos` after the frame.
    pub fuzzpos: u32,
    /// The tracker could not identify the columns: the whole frame is non-comparable.
    pub whole: bool,
    /// Flagged screen columns (U rows).
    pub columns: Vec<u16>,
    /// Newly mapped lines (M row).
    pub mapped: Vec<u16>,
}

/// A frame stream: per tic its F row, and the run's ending witness or error (O/E rows).
#[derive(Clone, Debug, Default)]
pub struct FrameStream {
    /// Frames by tic.
    pub frames: BTreeMap<i64, FrameRow>,
    /// The first O row: (tic, witness).
    pub witness: Option<(i64, String)>,
    /// The first E row: (tic, message).
    pub error: Option<(i64, String)>,
}

/// Reads `tests/reference/m5a_frames/<lump>.sb<screenblocks>.frames`.
pub fn read_frames(lump: &str, screenblocks: u8) -> FrameStream {
    let text = std::fs::read_to_string(reference(&format!(
        "m5a_frames/{lump}.sb{screenblocks}.frames"
    )))
    .expect("frames");
    let mut s = FrameStream::default();
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    assert_eq!(lines.next(), Some("#schema,DOOM-M5A-FRAMES-v1"));
    for l in lines {
        let cols: Vec<&str> = l.split(',').collect();
        let tic = || cols[1].parse::<i64>().expect("tic");
        match cols[0] {
            "F" => {
                s.frames.insert(
                    tic(),
                    FrameRow {
                        sha256: cols[2].to_string(),
                        masked_sha256: cols[3].to_string(),
                        fuzzpos: cols[4].parse().expect("fuzzpos"),
                        whole: cols[14] == "1",
                        ..FrameRow::default()
                    },
                );
            }
            "U" => s
                .frames
                .get_mut(&tic())
                .expect("U after F")
                .columns
                .push(cols[2].parse().expect("x")),
            "M" => {
                s.frames.get_mut(&tic()).expect("M after F").mapped = cols[2]
                    .split(' ')
                    .map(|v| v.parse().expect("line"))
                    .collect();
            }
            "O" => {
                s.witness.get_or_insert((tic(), cols[2].to_string()));
            }
            "E" => {
                s.error.get_or_insert((tic(), cols[2..].join(",")));
            }
            _ => assert!(l.starts_with('#') || l.starts_with("N,"), "{lump}: {l}"),
        }
    }
    s
}

/// The outcome of one frame's comparison with its genuine F row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// Every comparable pixel equals the genuine frame; `flags_agree` says whether the Rust
    /// renderer flagged exactly the genuine tracker's columns.
    Match {
        /// Rust's flagged columns equal the genuine U rows.
        flags_agree: bool,
    },
    /// Some comparable pixel differs.
    Mismatch,
    /// Non-comparable as a whole: the genuine tracker could not identify the columns, or the
    /// switch parity is unknown (decisions R4, R7).
    NonComparable,
    /// The Rust renderer refused the frame.
    Refused(doom_render::RenderRefusal),
}

/// The 64,000 bytes with `columns` zeroed (the genuine runner's masked digest input).
pub fn masked(pixels: &[u8], columns: &[u16]) -> Vec<u8> {
    let mut m = pixels.to_vec();
    for x in columns {
        for y in 0..doom_render::SCREENHEIGHT {
            m[y * doom_render::SCREENWIDTH + usize::from(*x)] = 0;
        }
    }
    m
}

/// Compares one rendered frame with its genuine F row: the full SHA-256 when the genuine
/// tracker flagged nothing, otherwise the digest with the genuine flagged columns zeroed
/// (owner ruling on R4: every other pixel is compared).
pub fn compare(
    row: &FrameRow,
    result: &Result<doom_render::FrameReport, doom_render::RenderRefusal>,
    frame: &doom_render::Frame,
) -> Outcome {
    let report = match result {
        Ok(r) => r,
        Err(e) => return Outcome::Refused(e.clone()),
    };
    if row.whole || report.switch_parity_unknown {
        return Outcome::NonComparable;
    }
    let digest = if row.columns.is_empty() {
        doom_test_evidence::sha256(frame.pixels()) == row.sha256
    } else {
        doom_test_evidence::sha256(&masked(frame.pixels(), &row.columns)) == row.masked_sha256
    };
    if digest {
        Outcome::Match {
            flags_agree: report.flagged_columns == row.columns,
        }
    } else {
        Outcome::Mismatch
    }
}

/// The first pixel (x, y) where `a` and `b` differ outside `excluded` columns.
pub fn first_difference(a: &[u8], b: &[u8], excluded: &[u16]) -> Option<(usize, usize)> {
    (0..a.len())
        .find(|&i| a[i] != b[i] && !excluded.contains(&((i % doom_render::SCREENWIDTH) as u16)))
        .map(|i| (i % doom_render::SCREENWIDTH, i / doom_render::SCREENWIDTH))
}

/// Per-configuration counts of a harness run.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    /// Genuine frames compared.
    pub frames: usize,
    /// Matching frames.
    pub matched: usize,
    /// Matching frames whose flagged columns differ from the genuine tracker's.
    pub flag_disagreements: usize,
    /// Mismatching frames.
    pub mismatched: usize,
    /// Non-comparable frames.
    pub non_comparable: usize,
    /// Frames the Rust renderer refused.
    pub refused: usize,
    /// Genuine frames after the Rust run ended (simulation refusal or demo end).
    pub not_reached: usize,
}

/// One frame as the harness saw it, for reporters.
pub struct Seen<'a> {
    /// The item.
    pub item: &'a Item,
    /// The configuration's screenblocks.
    pub screenblocks: u8,
    /// The genuine dumped tic.
    pub tic: i64,
    /// The genuine F row.
    pub row: &'a FrameRow,
    /// The Rust result.
    pub result: &'a Result<doom_render::FrameReport, doom_render::RenderRefusal>,
    /// The Rust frame.
    pub frame: &'a doom_render::Frame,
    /// The renderer (for its trace).
    pub renderer: &'a doom_render::Renderer,
    /// The outcome.
    pub outcome: &'a Outcome,
}

/// Runs `item` through the simulation once, tic by tic, with one renderer per configuration
/// (observe after every committed `advance`, render every dumped tic: decision R8), and
/// compares every frame with the committed goldens. `seen` is called for every frame.
pub fn harness(
    item: &Item,
    configs: &[u8],
    trace: bool,
    mut seen: impl FnMut(&Seen<'_>),
) -> Vec<(u8, Counts)> {
    let mut run = load(item);
    let mut renderers: Vec<doom_render::Renderer> = configs
        .iter()
        .map(|&sb| {
            let mut r = doom_render::Renderer::new(
                run.res.clone(),
                run.level.clone(),
                doom_render::RenderConfig {
                    screenblocks: sb,
                    detail: doom_render::Detail::High,
                },
            )
            .expect("configuration");
            r.set_trace(trace);
            r.begin_level(&run.sim).expect("level start");
            r
        })
        .collect();
    let streams: Vec<FrameStream> = configs
        .iter()
        .map(|&sb| read_frames(&item.lump, sb))
        .collect();
    let mut counts = vec![Counts::default(); configs.len()];
    let mut frames: Vec<doom_render::Frame> =
        configs.iter().map(|_| doom_render::Frame::new()).collect();
    let mut tic: i64 = 0;
    loop {
        let index = tic as usize;
        let ok = index < run.commands.len() && advance(&mut run, index).is_ok();
        if !ok {
            for (c, s) in counts.iter_mut().zip(&streams) {
                c.not_reached += s.frames.range(tic..).count();
            }
            break;
        }
        let mut any = false;
        for (k, r) in renderers.iter_mut().enumerate() {
            r.observe(&run.sim).expect("observe");
            let Some(row) = streams[k].frames.get(&tic) else {
                continue;
            };
            any = true;
            let result = r.render(&run.sim, &mut frames[k]);
            let outcome = compare(row, &result, &frames[k]);
            let c = &mut counts[k];
            c.frames += 1;
            match &outcome {
                Outcome::Match { flags_agree } => {
                    c.matched += 1;
                    c.flag_disagreements += usize::from(!flags_agree);
                }
                Outcome::Mismatch => c.mismatched += 1,
                Outcome::NonComparable => c.non_comparable += 1,
                Outcome::Refused(_) => c.refused += 1,
            }
            seen(&Seen {
                item,
                screenblocks: configs[k],
                tic,
                row,
                result: &result,
                frame: &frames[k],
                renderer: r,
                outcome: &outcome,
            });
        }
        if !any {
            break;
        }
        tic += 1;
    }
    configs.iter().copied().zip(counts).collect()
}
