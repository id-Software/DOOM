//! A whole level under `PlayerMonstersLevelV1` (wire 22) or, with `--policy
//! episode1`, `PlayerMonstersEpisode1V1` (wire 23, DSIM39) driven by a version-110
//! demo, for a tic-for-tic comparison with the genuine engine (M3h9c, M4a).
//!
//! `player_level_run --wad <path> --map E1M1 --demo <lmp> --out <dir> [--captures]
//! [--policy level|episode1]` loads the map, builds `original_player_level_map`
//! (or `original_player_episode1_map`) with default options
//! (medium skill, single player, RNG indices 0), spawns the THINGs (the body at
//! its record), the default weapons and the level specials, then runs one tic
//! per demo command. It writes `digests.txt` (one line per capture: tic, SHA-256
//! of the canonical capture without its `map.*` fields, the FNV-1a digest of the
//! whole capture, the gameplay RNG index, the actor and thinker counts), with
//! `--captures` every capture as `tic_NNNNN.bin`, `stats.txt` (work maxima) and,
//! on a refused tic, `refused.txt` (the tic and the error) before stopping.
//! Captures of a commercial IWAD hold its map bytes: keep them out of Git.
//! Without `--wad` it prints its usage and exits successfully.
//! `player_level_run --write-idle <tics> --demo <lmp>` instead writes a demo of
//! `tics` zero commands with `write_demo` and exits.
//!
//! With `--policy episode1` the map must be `E1M1`..`E1M9`; the demo is read
//! with `parse_demo_episode1`, whose header map must be the `--map` one
//! (decision H10), `--write-idle` writes that map's header with
//! `write_demo_episode1`, and `stats.txt` adds the exit tic (`exit_tic`, `none`
//! without one), whether the exit was the secret one (`secret_exit`) and the
//! actor-store high-water mark (`store_high_water`: one past the highest slot
//! `thinker.order` named on any capture).
//!
//! With `--policy episode1v2` it builds `original_player_episode1_v2_map`
//! (`PlayerMonstersEpisode1V2`, wire 24, DSIM40; M4b decision I2) as for
//! `episode1`, with the map number of `--map` and the skill of the demo header,
//! read with `parse_demo_episode1_v2` (skill 2 or 3; the header map must be the
//! `--map` one); a skill-3 demo runs at hard skill (M4b step 6, decision I4).
//! `--write-idle` writes a skill-2
//! header with `write_demo_episode1_v2`; `stats.txt` is `episode1`'s.
use doom_sim::{
    demo::{
        parse_demo, parse_demo_episode1, parse_demo_episode1_v2, write_demo, write_demo_episode1,
        write_demo_episode1_v2,
    },
    *,
};
use std::{error::Error, fmt::Write as _, fs, path::PathBuf};

type ResultHost<T> = Result<T, Box<dyn Error>>;

const USAGE: &str = "usage: player_level_run --wad <path> --map E1M1 --demo <lmp> --out <dir> \
     [--captures] [--policy level|episode1|episode1v2 (wire 22, default | wire 23 | wire 24)] \
     | --write-idle <tics> --demo <lmp> [--policy episode1|episode1v2 --map E1Mn]";

/// The policy `--policy` selects.
#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum Policy {
    /// `PlayerMonstersLevelV1`, wire 22 (the default).
    #[default]
    Level,
    /// `PlayerMonstersEpisode1V1`, wire 23.
    Episode1,
    /// `PlayerMonstersEpisode1V2`, wire 24.
    Episode1V2,
}

/// `n` of `E1M<n>`, `n` in 1..=9, for the Episode 1 policies.
fn episode1_map(name: &str, policy: &str) -> ResultHost<u8> {
    match name.as_bytes() {
        [b'E', b'1', b'M', n @ b'1'..=b'9'] => Ok(n - b'0'),
        _ => Err(format!("--policy {policy} needs --map E1M1..E1M9, not {name}").into()),
    }
}

/// The canonical bytes of `snap` without its static `map.*` fields.
fn without_map(snap: &Snapshot) -> Vec<u8> {
    let fields: Vec<_> = snap
        .fields()
        .filter(|f| !f.path().starts_with("map."))
        .collect();
    let mut out = b"DSIM".to_vec();
    out.extend(snap.version().to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for f in fields {
        out.extend((f.path().len() as u32).to_le_bytes());
        out.extend(f.path().as_bytes());
        out.extend((f.value().len() as u64).to_le_bytes());
        out.extend(f.value());
    }
    out
}

fn field<'a>(snap: &'a Snapshot, path: &str) -> &'a [u8] {
    snap.fields()
        .find(|f| f.path() == path)
        .map_or(&[], StateField::value)
}

/// One past the highest actor-store slot `thinker.order` names (each entry a
/// little-endian `u32` slot then the generation): the store high-water mark.
fn store_high_water(snap: &Snapshot) -> usize {
    field(snap, "thinker.order")
        .chunks_exact(12)
        .map(|id| u32::from_le_bytes([id[0], id[1], id[2], id[3]]) as usize + 1)
        .max()
        .unwrap_or(0)
}

/// One `digests.txt` line, plus the actor and thinker counts.
fn line(snap: &Snapshot) -> (String, usize, usize) {
    let actors = field(snap, "thinker.order").len() / 12;
    let thinkers = field(snap, "thinker.shared_order").len() / 13;
    let rng = field(snap, "rng.gameplay").first().copied().unwrap_or(0);
    (
        format!(
            "{} {} {:016x} {rng} {actors} {thinkers}",
            snap.tic(),
            doom_test_evidence::sha256(&without_map(snap)),
            snap.digest()
        ),
        actors,
        thinkers,
    )
}

#[derive(Default)]
struct Args {
    wad: Option<PathBuf>,
    map: String,
    demo: Option<PathBuf>,
    out: Option<PathBuf>,
    captures: bool,
    write_idle: Option<usize>,
    policy: Policy,
    help: bool,
}
fn args() -> ResultHost<Args> {
    let mut a = Args {
        map: "E1M1".into(),
        ..Args::default()
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--wad" => a.wad = it.next().map(PathBuf::from),
            "--map" => a.map = it.next().ok_or(USAGE)?,
            "--demo" => a.demo = it.next().map(PathBuf::from),
            "--out" => a.out = it.next().map(PathBuf::from),
            "--captures" => a.captures = true,
            "--help" => a.help = true,
            "--policy" => {
                a.policy = match it.next().as_deref() {
                    Some("level") => Policy::Level,
                    Some("episode1") => Policy::Episode1,
                    Some("episode1v2") => Policy::Episode1V2,
                    _ => return Err(USAGE.into()),
                }
            }
            "--write-idle" => a.write_idle = Some(it.next().ok_or(USAGE)?.parse()?),
            _ => return Err(USAGE.into()),
        }
    }
    Ok(a)
}

fn main() -> ResultHost<()> {
    let a = args()?;
    if a.help {
        println!("{USAGE}");
        return Ok(());
    }
    if let Some(tics) = a.write_idle {
        let idle = vec![TicCommand::default(); tics];
        let demo = match a.policy {
            Policy::Level => write_demo(&idle)?,
            Policy::Episode1 => write_demo_episode1(episode1_map(&a.map, "episode1")?, &idle)?,
            Policy::Episode1V2 => {
                write_demo_episode1_v2(episode1_map(&a.map, "episode1v2")?, 2, &idle)?
            }
        };
        fs::write(a.demo.ok_or(USAGE)?, &demo)?;
        println!("wrote an idle demo of {tics} tics ({} bytes)", demo.len());
        return Ok(());
    }
    let Some(wad_path) = a.wad else {
        println!("{USAGE}");
        return Ok(());
    };
    let (demo_path, out) = (a.demo.ok_or(USAGE)?, a.out.ok_or(USAGE)?);
    let demo_bytes = fs::read(demo_path)?;
    // The wire-24 map number and skill (decisions I2 and I4).
    let mut v2 = None;
    let demo = match a.policy {
        Policy::Level => parse_demo(&demo_bytes)?,
        Policy::Episode1 => parse_demo_episode1(&demo_bytes, episode1_map(&a.map, "episode1")?)?.1,
        Policy::Episode1V2 => {
            let (header, demo) =
                parse_demo_episode1_v2(&demo_bytes, episode1_map(&a.map, "episode1v2")?)?;
            // The reader admits skill bytes 2 and 3 only (decision I4).
            let skill = match header.skill {
                2 => Skill::Medium,
                3 => Skill::Hard,
                other => {
                    return Err(format!(
                        "--policy episode1v2: the demo header's skill {other} is not admitted; \
                         wire 24 runs skills 2 and 3"
                    )
                    .into());
                }
            };
            v2 = Some((header.map, skill));
            demo
        }
    };
    let bytes = fs::read(wad_path)?;
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default())?;
    let map = doom_map::Map::from_wad(&wad, &a.map, doom_map::MapLimits::default())?;
    let mut s = match (a.policy, v2) {
        (Policy::Episode1V2, Some((n, skill))) => Simulation::original_player_episode1_v2_map(
            map,
            n,
            skill,
            MapContextOptions::default(),
        )?,
        (Policy::Episode1, _) => {
            Simulation::original_player_episode1_map(map, MapContextOptions::default())?
        }
        _ => Simulation::original_player_level_map(map, MapContextOptions::default())?,
    };
    let report = s.spawn_map_things()?;
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())?;
    s.spawn_level_specials()?;
    fs::create_dir_all(&out)?;
    // Load work is not a tic's: the maxima cover the tics only.
    take_level_run_stats();
    let mut digests = String::new();
    let snap = s.snapshot();
    let (text, mut peak_actors, mut peak_thinkers) = line(&snap);
    let mut high_water = store_high_water(&snap);
    writeln!(digests, "{text}")?;
    if a.captures {
        fs::write(out.join("tic_00000.bin"), snap.canonical_bytes())?;
    }
    let (mut changes, mut clips, mut admitted) = (0usize, 0usize, 0usize);
    let mut refused = None;
    let mut exit = None;
    let mut last = snap.digest();
    for (i, command) in demo.commands.iter().enumerate() {
        let tic = i + 1;
        if let Err(error) = s.advance([
            *command,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ]) {
            fs::write(out.join("refused.txt"), format!("{tic} {error:?}\n"))?;
            refused = Some(tic);
            break;
        }
        admitted += 1;
        let (c, t) = s.events().iter().fold((0, 0), |(c, t), e| match e {
            Event::Mover(MoverEvent::SectorChanged { .. }) => (c + 1, t),
            Event::Mover(MoverEvent::ThingClipped { .. }) => (c, t + 1),
            _ => (c, t),
        });
        changes = changes.max(c);
        clips = clips.max(t);
        let snap = s.snapshot();
        let (text, actors, thinkers) = line(&snap);
        peak_actors = peak_actors.max(actors);
        peak_thinkers = peak_thinkers.max(thinkers);
        high_water = high_water.max(store_high_water(&snap));
        if exit.is_none() && field(&snap, "level.exit") == [1] {
            exit = Some((tic, field(&snap, "level.secret_exit") == [1]));
        }
        writeln!(digests, "{text}")?;
        if a.captures {
            fs::write(
                out.join(format!("tic_{tic:05}.bin")),
                snap.canonical_bytes(),
            )?;
        }
        last = snap.digest();
    }
    fs::write(out.join("digests.txt"), digests)?;
    let stats = take_level_run_stats();
    let mut stats_text = format!(
        "max_intercepts {}\nmax_spechit {}\nmax_change_sector_calls_per_tic {changes}\n\
         max_change_sector_thing_clips_per_tic {clips}\npeak_actors {peak_actors}\n\
         peak_thinkers {peak_thinkers}\n",
        stats.max_intercepts, stats.max_spechit
    );
    if a.policy != Policy::Level {
        match exit {
            Some((tic, secret)) => writeln!(stats_text, "exit_tic {tic}\nsecret_exit {secret}")?,
            None => writeln!(stats_text, "exit_tic none\nsecret_exit false")?,
        }
        writeln!(stats_text, "store_high_water {high_water}")?;
    }
    fs::write(out.join("stats.txt"), stats_text)?;
    println!(
        "spawned {} body {} commands {} admitted {admitted} refused {} final_fnv {last:016x}",
        report.spawned.len(),
        report.player_body.is_some(),
        demo.commands.len(),
        refused.map_or("no".into(), |t| format!("at tic {t}")),
    );
    Ok(())
}
