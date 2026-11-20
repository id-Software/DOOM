//! Wire 25's sound records of one demo run, with wire 24 alongside (M5c step 3a).
//!
//! `sound_records_run --wad <path> --map E1Mn --demo <lmp> [--out <file>]` builds
//! `original_player_episode1_v2_map` (wire 24) and `original_player_episode1_v3_map`
//! (wire 25) for the map with the demo header's skill, runs the demo under both
//! and, at every capture (tic 0 and every admitted tic), checks 1 and 2 of M5c note
//! §4.4.4 (see `tests/wire25/mod.rs`): the stripped DSIM41 capture equals the
//! stripped DSIM40 capture, and wire 25's events are wire 24's with `sfx_pdiehi`
//! read as `sfx_pldeth`. Any difference, or a refusal under one wire only, ends
//! the run with a non-zero status. With `--out` it writes wire 25's records, one
//! row per record in the call order:
//!
//! - `S,tic,seq,sfx,okind,oid,placed,ox,oy,lx,ly,langle`: a start; okind `none`
//!   (oid -1), `player` (the listener's body), `mobj`, `sector` or `button`; oid
//!   `slot:generation` for an actor, the index or slot otherwise; the five
//!   positions as raw 16.16 bits and the angle, `-` when not placed;
//! - `T,tic,seq,slot:generation`: `P_RemoveMobj`'s stop;
//! - `Z,...`: the totals, also printed.
//!
//! `tic` is the 0-based number of the tic that made the record (the genuine
//! stream's numbering); records of explicit operations do not occur in a demo
//! run.
//!
//! `--captures <dir>` (M5c step 3b, for the whole-engine comparison of wire 25
//! against the genuine engine, tools/reference/level_compare_m5c.py) also writes
//! every wire-25 capture as `<dir>/tic_NNNNN.bin` (the canonical DSIM41 bytes,
//! capture N after N tics) and, when the run is refused, `<dir>/refused.txt`
//! (`<tic> <error>`), the layout of `player_level_run --captures`.
//! `--capture-stream <path>` writes the same captures to one stream instead (a
//! pipe for the real-map comparison, which then needs no disk for them): per
//! capture u32 tic, u64 length and the bytes, then a trailer u32 0xffffffff, u64
//! length and the UTF-8 text `end` or `refused <tic> <error>`; little endian.
//! `--alone` runs
//! wire 25 by itself, without wire 24 and checks 1 and 2: for a map the two wires
//! do not load alike (the THING-loading truncation, M5c decision T4). Nothing is
//! written but `--out` and `--captures`; frozen tools and examples are untouched.
//! Without `--wad` it prints its usage and exits successfully.

#[path = "../tests/wire25/mod.rs"]
mod wire25;

use doom_sim::{
    demo::parse_demo_episode1_v2,
    sound::{SoundOrigin, SoundRecord},
    *,
};
use std::{error::Error, fmt::Write as _, fs, io::Write as _, path::PathBuf};

const USAGE: &str = "usage: sound_records_run --wad <path> --map E1Mn --demo <lmp> [--out <file>] \
     [--captures <dir> | --capture-stream <path>] [--alone]";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let (mut wad, mut map, mut demo, mut out) = (None, None, None, None);
    let (mut captures, mut alone, mut stream) = (None::<PathBuf>, false, None::<PathBuf>);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wad = Some(PathBuf::from(value()?)),
            "--map" => map = Some(value()?),
            "--demo" => demo = Some(PathBuf::from(value()?)),
            "--out" => out = Some(PathBuf::from(value()?)),
            "--captures" => captures = Some(PathBuf::from(value()?)),
            "--alone" => alone = true,
            "--capture-stream" => stream = Some(PathBuf::from(value()?)),
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("unknown option {other}\n{USAGE}").into()),
        }
    }
    let Some(wad) = wad else {
        println!("{USAGE}");
        return Ok(());
    };
    let map = map.ok_or("--map is required")?;
    let number = map
        .strip_prefix("E1M")
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| EPISODE1_MAP_NUMBERS.contains(n))
        .ok_or("--map must be E1M1..E1M9")?;
    let demo_bytes = fs::read(demo.ok_or("--demo is required")?)?;
    let (header, demo) = parse_demo_episode1_v2(&demo_bytes, number)?;
    let skill = match header.skill {
        2 => Skill::Medium,
        3 => Skill::Hard,
        other => return Err(format!("demo skill {other} is not admitted").into()),
    };
    let bytes = fs::read(wad)?;
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default())?;
    let level = doom_map::Map::from_wad(&wad, &map, doom_map::MapLimits::default())?;
    if let Some(dir) = &captures {
        fs::create_dir_all(dir)?;
    }
    if captures.is_some() && stream.is_some() {
        return Err("--captures and --capture-stream exclude each other".into());
    }
    let mut sink = match &stream {
        Some(path) => Some(std::io::BufWriter::new(fs::File::create(path)?)),
        None => None,
    };
    let mut rows = String::new();
    let mut seq = 0_u64;
    let mut written: Result<(), std::io::Error> = Ok(());
    let each = |tic: usize, s: &Simulation| {
        if let (Some(dir), true) = (&captures, written.is_ok()) {
            written = fs::write(
                dir.join(format!("tic_{tic:05}.bin")),
                s.snapshot().canonical_bytes(),
            );
        }
        if let (Some(w), true) = (sink.as_mut(), written.is_ok()) {
            written = frame(
                w,
                u32::try_from(tic).unwrap_or(u32::MAX - 1),
                &s.snapshot().canonical_bytes(),
            );
        }
        if tic == 0 {
            return;
        }
        let listener = s.sound_listener().map(|l| l.actor);
        for r in s.sound_records() {
            seq += 1;
            let made = tic - 1;
            let id = |a: ActorId| format!("{}:{}", a.slot(), a.generation());
            match *r {
                SoundRecord::Start { sfx, origin, at } => {
                    let (kind, oid) = match origin {
                        SoundOrigin::None => ("none", "-1".to_owned()),
                        SoundOrigin::Actor(a) if Some(a) == listener => ("player", id(a)),
                        SoundOrigin::Actor(a) => ("mobj", id(a)),
                        SoundOrigin::Sector(i) => ("sector", i.to_string()),
                        SoundOrigin::Button(i) => ("button", i.to_string()),
                    };
                    let place = at.map_or_else(
                        || "0,-,-,-,-,-".to_owned(),
                        |p| {
                            format!(
                                "1,{},{},{},{},{}",
                                p.origin[0],
                                p.origin[1],
                                p.listener[0],
                                p.listener[1],
                                p.listener_angle
                            )
                        },
                    );
                    let _ = writeln!(rows, "S,{made},{seq},{sfx},{kind},{oid},{place}");
                }
                SoundRecord::Stop { actor } => {
                    let _ = writeln!(rows, "T,{made},{seq},{}", id(actor));
                }
            }
        }
    };
    let outcome = if alone {
        wire25::run_alone(&level, number, skill, &demo.commands, each)?
    } else {
        wire25::run_pair(&level, number, skill, &demo.commands, each)?
    };
    written?;
    if let (Some(dir), Some((tic, error))) = (&captures, &outcome.refused) {
        fs::write(dir.join("refused.txt"), format!("{tic} {error}\n"))?;
    }
    if let Some(mut w) = sink {
        let trailer = outcome
            .refused
            .as_ref()
            .map_or_else(|| "end".to_owned(), |(t, e)| format!("refused {t} {e}"));
        frame(&mut w, u32::MAX, trailer.as_bytes())?;
        w.flush()?;
    }
    let summary = format!(
        "Z,captures,{},tics,{},starts,{},placed,{},stops,{},screams_replaced,{},refused,{}",
        outcome.captures,
        outcome.tics,
        outcome.starts,
        outcome.placed,
        outcome.stops,
        outcome.screams,
        outcome
            .refused
            .as_ref()
            .map_or_else(|| "none".to_owned(), |(t, e)| format!("{t}:{e}"))
    );
    println!("{summary}");
    if let Some(out) = out {
        rows.push_str(&summary);
        rows.push('\n');
        fs::write(out, rows)?;
    }
    Ok(())
}

/// One `--capture-stream` frame: u32 tic, u64 length, then the bytes.
fn frame(w: &mut impl std::io::Write, tic: u32, bytes: &[u8]) -> std::io::Result<()> {
    w.write_all(&tic.to_le_bytes())?;
    w.write_all(&(bytes.len() as u64).to_le_bytes())?;
    w.write_all(bytes)
}
