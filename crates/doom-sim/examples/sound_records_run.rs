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
//! run. Nothing is written but `--out`; frozen tools and examples are untouched.
//! Without `--wad` it prints its usage and exits successfully.

#[path = "../tests/wire25/mod.rs"]
mod wire25;

use doom_sim::{
    demo::parse_demo_episode1_v2,
    sound::{SoundOrigin, SoundRecord},
    *,
};
use std::{error::Error, fmt::Write as _, fs, path::PathBuf};

const USAGE: &str = "usage: sound_records_run --wad <path> --map E1Mn --demo <lmp> [--out <file>]";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let (mut wad, mut map, mut demo, mut out) = (None, None, None, None);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wad = Some(PathBuf::from(value()?)),
            "--map" => map = Some(value()?),
            "--demo" => demo = Some(PathBuf::from(value()?)),
            "--out" => out = Some(PathBuf::from(value()?)),
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
    let mut rows = String::new();
    let mut seq = 0_u64;
    let outcome = wire25::run_pair(&level, number, skill, &demo.commands, |tic, s| {
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
    })?;
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
