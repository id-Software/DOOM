//! `doom-play`: the playable port's window around doom-game's platform-free app (milestone
//! M5b, decisions S2, S13, S14, S19–S21). This crate is the only one that depends on the
//! window library (miniquad); it reads the files, keeps the monotonic clock, translates input
//! and presents the picture. Nothing here reaches the simulation except through
//! [`doom_game::app::App`]'s input calls. No `unsafe` (forbidden by the workspace lints).

mod keys;
mod window;

use doom_game::app::{App, GameData, Host, SessionConfig, Timestamp};
use doom_game::input::Preset;
use doom_render::GameMode;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const HELP: &str = "\
doom-play: play one Episode 1 map of DOOM (Linux DOOM 1.10 rules) in a window.

usage: doom-play --iwad PATH [options]

  --iwad PATH          the IWAD (required; read only, never copied)
  --wad PATH           an extra WAD loaded after the IWAD (repeatable; later files win)
  --map N              E1M<N>, 1-9 (default 1); E1M1..E1M9 is also accepted
  --skill N            2 or 3 (default 2)
  --sessions DIR       where session demos are written (default doom-sessions/)
  --bindings NAME      vanilla (default) or modern (WASD, E use, always run)
  --gamma N            gamma table 0-4 (default 0, as the original)
  --aspect NAME        4:3 (default, the picture shown as 320x240) or square
  --fullscreen         borderless fullscreen instead of a window
  --screenblocks N     10 (default, status bar row) or 11 (full view)
  --quit-after-tics N  measurement aid: save and quit after N tics (or at a session end)
  --help               this text

Keys: Esc or Pause pauses (then Esc resumes, R restarts, Q quits); on an end screen
R restarts and Q or Esc quits. Every session is recorded as a version-110 demo named
E1M<map>-skill<n>-<yyyymmdd-hhmmss>.lmp (the time is UTC) in the sessions directory.

Limits: an OpenGL 2.1 (or OpenGL ES 2) driver is required. The window library reports
no focus loss: a key held while you switch away stays held until pressed again; press
Esc first to pause, which releases every key. There is no sound, menu, automap,
intermission or next level; the session ends at the level's exit or the player's death.";

/// The parsed command line.
#[derive(Debug)]
struct Options {
    iwad: PathBuf,
    wads: Vec<PathBuf>,
    map: u8,
    skill: u8,
    sessions: PathBuf,
    preset: Preset,
    gamma: usize,
    square: bool,
    fullscreen: bool,
    screenblocks: u8,
    quit_after: Option<u64>,
}

fn number<T: std::str::FromStr>(
    flag: &str,
    value: &str,
    ok: impl Fn(&T) -> bool,
) -> Result<T, String> {
    value
        .parse()
        .ok()
        .filter(|v| ok(v))
        .ok_or_else(|| format!("{flag}: {value:?} is not an accepted value (see --help)"))
}

/// Parses the arguments; `Ok(None)` means `--help`.
fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut iwad = None;
    let mut o = Options {
        iwad: PathBuf::new(),
        wads: Vec::new(),
        map: 1,
        skill: 2,
        sessions: PathBuf::from("doom-sessions"),
        preset: Preset::Vanilla,
        gamma: 0,
        square: false,
        fullscreen: false,
        screenblocks: 10,
        quit_after: None,
    };
    let mut it = args.into_iter();
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--help" | "-h" => return Ok(None),
            "--iwad" => iwad = Some(PathBuf::from(value()?)),
            "--wad" => o.wads.push(PathBuf::from(value()?)),
            "--map" => {
                let v = value()?;
                let n = v
                    .strip_prefix("E1M")
                    .or(v.strip_prefix("e1m"))
                    .unwrap_or(&v);
                o.map = number(&flag, n, |m| (1..=9).contains(m))?;
            }
            "--skill" => o.skill = number(&flag, &value()?, |s| (2..=3).contains(s))?,
            "--sessions" => o.sessions = PathBuf::from(value()?),
            "--bindings" => {
                o.preset = match value()?.as_str() {
                    "vanilla" => Preset::Vanilla,
                    "modern" => Preset::Modern,
                    v => return Err(format!("--bindings: {v:?} is not vanilla or modern")),
                }
            }
            "--gamma" => o.gamma = number(&flag, &value()?, |g| *g <= 4)?,
            "--aspect" => {
                o.square = match value()?.as_str() {
                    "4:3" => false,
                    "square" => true,
                    v => return Err(format!("--aspect: {v:?} is not 4:3 or square")),
                }
            }
            "--fullscreen" => o.fullscreen = true,
            "--screenblocks" => {
                o.screenblocks = number(&flag, &value()?, |s| (10..=11).contains(s))?;
            }
            "--quit-after-tics" => o.quit_after = Some(number(&flag, &value()?, |n| *n > 0)?),
            other => return Err(format!("unknown argument {other:?} (see --help)")),
        }
    }
    o.iwad = iwad.ok_or("--iwad PATH is required (see --help)")?;
    Ok(Some(o))
}

/// The game mode linuxdoom's `IdentifyVersion` gives the IWAD's file name (d_main.c:618-632,
/// 690-709): `doom1.wad` shareware, `doomu.wad` retail, anything else registered (the mode
/// of the M5a evidence runs).
fn game_mode(iwad: &Path) -> GameMode {
    let name = iwad
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase());
    match name.as_deref() {
        Some("doom1.wad") => GameMode::Shareware,
        Some("doomu.wad") => GameMode::Retail,
        _ => GameMode::Registered,
    }
}

/// The sessions directory on disk.
struct FsHost {
    dir: PathBuf,
}

impl Host for FsHost {
    fn timestamp(&mut self) -> Timestamp {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        utc(secs)
    }
    fn exists(&mut self, name: &str) -> bool {
        self.dir.join(name).exists()
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let path = self.dir.join(name);
        let tmp = self.dir.join(format!("{name}.tmp"));
        std::fs::create_dir_all(&self.dir)
            .and_then(|()| std::fs::write(&tmp, bytes))
            .and_then(|()| std::fs::rename(&tmp, &path))
            .map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// The UTC date and time of `secs` since 1970 (the civil-from-days algorithm).
fn utc(secs: u64) -> Timestamp {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Timestamp {
        year: u16::try_from(year).unwrap_or(0),
        month: month as u8,
        day: day as u8,
        hour: (rem / 3_600) as u8,
        minute: (rem / 60 % 60) as u8,
        second: (rem % 60) as u8,
    }
}

/// Whether a display server is named (Linux and the other X11/Wayland systems; miniquad
/// panics without one).
#[cfg(all(unix, not(target_os = "macos")))]
fn display_available() -> bool {
    std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
fn display_available() -> bool {
    true
}

fn run(o: Options) -> Result<(), String> {
    let mut files = Vec::with_capacity(1 + o.wads.len());
    for path in std::iter::once(&o.iwad).chain(&o.wads) {
        files.push(std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?);
    }
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let data =
        GameData::load(&refs, game_mode(&o.iwad), o.map, o.skill).map_err(|e| e.to_string())?;
    drop(files);
    if !display_available() {
        return Err("no display: neither DISPLAY nor WAYLAND_DISPLAY is set".into());
    }
    let cfg = SessionConfig {
        screenblocks: o.screenblocks,
        preset: o.preset,
        ..SessionConfig::default()
    };
    let mut host = FsHost { dir: o.sessions };
    let app = App::new(data, cfg, &mut host);
    window::play(
        app,
        host,
        window::Presentation {
            gamma: o.gamma,
            square: o.square,
            fullscreen: o.fullscreen,
            quit_after: o.quit_after,
        },
    );
    Ok(())
}

fn main() {
    let code = match parse(std::env::args().skip(1)) {
        Ok(None) => {
            println!("{HELP}");
            0
        }
        Ok(Some(o)) => match run(o) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("doom-play: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("doom-play: {e}");
            2
        }
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn arguments() {
        let o = parse(args("--iwad a.wad")).unwrap().unwrap();
        assert_eq!((o.map, o.skill, o.gamma, o.screenblocks), (1, 2, 0, 10));
        assert_eq!(
            (o.preset, o.square, o.fullscreen),
            (Preset::Vanilla, false, false)
        );
        assert_eq!(o.sessions, PathBuf::from("doom-sessions"));
        let o = parse(args(
            "--iwad a.wad --wad b.wad --wad c.wad --map E1M3 --skill 3 --sessions s \
             --bindings modern --gamma 4 --aspect square --fullscreen --screenblocks 11 \
             --quit-after-tics 70",
        ))
        .unwrap()
        .unwrap();
        assert_eq!(o.wads, [PathBuf::from("b.wad"), PathBuf::from("c.wad")]);
        assert_eq!((o.map, o.skill, o.gamma, o.screenblocks), (3, 3, 4, 11));
        assert_eq!(
            (o.preset, o.square, o.fullscreen, o.quit_after),
            (Preset::Modern, true, true, Some(70))
        );
        assert!(parse(args("--help")).unwrap().is_none());
        for bad in [
            "",
            "--map 2",
            "--iwad a --map 0",
            "--iwad a --map E1M10",
            "--iwad a --skill 1",
            "--iwad a --skill 4",
            "--iwad a --gamma 5",
            "--iwad a --screenblocks 9",
            "--iwad a --aspect wide",
            "--iwad a --bindings other",
            "--iwad a --quit-after-tics 0",
            "--iwad a --nope",
            "--iwad",
        ] {
            assert!(parse(args(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn game_mode_follows_identify_version() {
        assert_eq!(game_mode(Path::new("/x/DOOM1.WAD")), GameMode::Shareware);
        assert_eq!(game_mode(Path::new("doomu.wad")), GameMode::Retail);
        assert_eq!(game_mode(Path::new("DOOM.WAD")), GameMode::Registered);
        assert_eq!(game_mode(Path::new("other.wad")), GameMode::Registered);
    }

    #[test]
    fn utc_dates() {
        assert_eq!(
            utc(0),
            Timestamp {
                year: 1970,
                month: 1,
                day: 1,
                ..Timestamp::default()
            }
        );
        // 2026-11-17 21:05:09 UTC; 2024-02-29 23:59:59 UTC
        let t = utc(1_794_949_509);
        assert_eq!(
            (t.year, t.month, t.day, t.hour, t.minute, t.second),
            (2026, 11, 17, 21, 5, 9)
        );
        let t = utc(1_709_251_199);
        assert_eq!(
            (t.year, t.month, t.day, t.hour, t.minute, t.second),
            (2024, 2, 29, 23, 59, 59)
        );
    }
}
