//! `doom-play`: the playable port's window around doom-game's platform-free app (milestone
//! M5b, decisions S2, S13, S14, S19–S21), with sound (milestone M5c, decisions T7–T9, T15).
//! This crate is the only one that depends on the window library (miniquad) and the audio
//! library (maudio); it reads the files, keeps the monotonic clock, translates input, presents
//! the picture and hands the mixed sound stream to the device. Nothing here reaches the
//! simulation except through [`doom_game::app::App`]'s input calls. No `unsafe` (forbidden by
//! the workspace lints).

mod audio;
mod keys;
#[cfg(test)]
mod session_tests;
mod tic_stats;
mod window;

use audio::{Output, Volume};
use doom_game::app::{App, AudioSink, GameData, Host, SessionConfig, Timestamp, UNRECORDED_LABEL};
use doom_game::input::Preset;
use doom_render::GameMode;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const HELP: &str = "\
doom-play: play one Episode 1 map of DOOM in a window, with the Linux DOOM 1.10 game rules
and sound effects (no cheats, menu or music; see Sound and Limits).

usage: doom-play --iwad PATH [options]
       doom-play --audio-probe

  --iwad PATH          the IWAD (required; read only, never copied)
  --wad PATH           an extra WAD loaded after the IWAD (repeatable; later files win)
  --map N              E1M<N>, 1-9 (default 1); E1M1..E1M9 is also accepted
  --skill N            2 or hmp: Hurt Me Plenty (default); 3 or uv: Ultra-Violence.
                       N is the skill value stored in the demo header and the file
                       name. Vanilla's own -skill option counts from 1, so these are
                       its -skill 3 and -skill 4.
  --sessions DIR       where session demos are written (default doom-sessions/)
  --bindings NAME      vanilla (default) or modern (WASD, E use, always run)
  --gamma N            gamma table 0-4 (default 0, as the original)
  --aspect NAME        4:3 (default, the picture shown as 320x240) or square
  --fullscreen         borderless fullscreen instead of a window
  --screenblocks N     10 (default, status bar row) or 11 (full view)
  --quit-after-tics N  measurement aid: save and quit after N tics (or at a session end)
  --tic-stats          measurement aid: print, on exit, the intervals between executed tics
                       (taken from the loop's monotonic clock) to stderr; no other effect
  --unrecorded-full-turning
                       opt-in, never implied by another option: mouse and keyboard turns
                       are executed at full resolution instead of being rounded to demo
                       resolution (multiples of 256), but nothing is recorded: no demo is
                       written, so the session cannot be replayed or verified against the
                       genuine engine afterwards. The window title, the pause overlay
                       and the end screens say \"not recorded: no demo, cannot be replayed\",
                       and so does a band over the top of the picture for the first five
                       seconds (175 tics) of every session, in a window or fullscreen.
  --volume N           sound volume at the speakers, 0-100 percent (default 100). It
                       scales the finished sound only; 0 is silent. The game's sound
                       logic always runs at the original's default volume, so the game,
                       the status bar face and the demo do not depend on this setting.
  --no-sound           open no audio device. The sound logic and the mixer still run, so
                       the picture, the face and the demo are the same as with sound.
  --help               this text
  --audio-probe        diagnostic, taking no other option: open the audio device from the
                       listed real backends only, play digital silence (no sound) for about
                       one second, print which backend opened and exit 0; or print why
                       none opened and exit 1.

Keys: Esc or Pause pauses (then Esc or Pause resumes, R restarts, Q quits); on an end screen
R restarts and Q or Esc quits. End mutes and unmutes the sound (the player's own key, never
seen by the game); pausing also silences it. When the window library reports that the window
was minimised or lost focus, the game pauses and every held key is released. Every session
(unless --unrecorded-full-turning) is recorded as a version-110 demo named
E1M<map>-skill<n>-<yyyymmdd-hhmmss>.lmp (the time is UTC) in the sessions directory; an
existing file is never replaced (a suffix -2, -3, ... is added). If writing the demo fails,
the pause box says so, the end screen and the exit message give the reason, and the write
is tried again on quit and restart.

Sound: sound effects play from the first audio backend that opens (Linux: PulseAudio, then
ALSA, which need the system's libpulse.so.0 or libasound.so.2; Windows: WASAPI, DirectSound,
then WinMM; macOS: Core Audio). If none opens, one line on standard error says so and names
the backends tried, and the game runs exactly the same without sound output. The sound logic
and mixer are matched to linuxdoom-1.10 built with its in-process sound code (one define
removed), under the declared schedule; what reaches the speakers after that (the device,
resampling, latency and volume) is presentation and is not part of any claim.

Limits: an OpenGL 2.1 (or OpenGL ES 2) driver is required. There is no music, menu,
automap, intermission or next level; the session ends at the level's exit, or when the
dead player presses use. Cheat codes have no effect: their keys reach the game as ordinary
keys. The status bar face's idle glance follows the sound's random draws as the genuine
engine replaying the session's demo does; a vanilla player's menu keys (Esc, -, =, F1-F10),
which start menu sounds, are not part of a demo and are not modelled.";

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
    tic_stats: bool,
    unrecorded: bool,
    volume: Volume,
    no_sound: bool,
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
        tic_stats: false,
        unrecorded: false,
        volume: Volume::FULL,
        no_sound: false,
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
            "--skill" => {
                o.skill = match value()?.as_str() {
                    "hmp" => 2,
                    "uv" => 3,
                    v => number(&flag, v, |s| (2..=3).contains(s))?,
                }
            }
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
            "--tic-stats" => o.tic_stats = true,
            "--unrecorded-full-turning" => o.unrecorded = true,
            "--no-sound" => o.no_sound = true,
            "--volume" => {
                let percent = number(&flag, &value()?, |v: &u8| *v <= 100)?;
                o.volume = Volume::new(percent).ok_or("--volume: above 100")?;
            }
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

/// Whether the arguments ask for the audio probe: `Ok(true)` for `--audio-probe` alone,
/// `Ok(false)` when it is absent, an error when it is mixed with other arguments.
fn audio_probe_requested(args: &[String]) -> Result<bool, String> {
    match args {
        [only] if only == "--audio-probe" => Ok(true),
        _ if args.iter().any(|a| a == "--audio-probe") => {
            Err("--audio-probe takes no other argument (see --help)".into())
        }
        _ => Ok(false),
    }
}

/// The window title: the session's label when nothing is recorded (decision S3 (ii)).
fn title(recorded: bool) -> String {
    if recorded {
        "doom-play".into()
    } else {
        format!("doom-play ({UNRECORDED_LABEL})")
    }
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

/// The sessions directory on disk and the sound output.
struct FsHost {
    dir: PathBuf,
    audio: Output,
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
    /// Creates the file with `create_new`, so a name another session (or process) took is
    /// never replaced; see [`claim_new`].
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        claim_new(&self.dir, name, bytes, |f, b| {
            f.write_all(b).and_then(|()| f.sync_all())
        })
    }
    /// The device sink, or the sink that discards (no device, or `--no-sound`).
    fn audio(&mut self) -> Option<&mut dyn AudioSink> {
        Some(&mut self.audio)
    }
    /// Writes a temporary file whose name holds the process id (two processes never share
    /// it), flushes it to the disk and renames it over the session's file; a failed write
    /// removes the temporary file.
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let path = self.dir.join(name);
        let tmp = self.dir.join(format!("{name}.{}.tmp", std::process::id()));
        let written = std::fs::File::create(&tmp)
            .and_then(|mut f| f.write_all(bytes).and_then(|()| f.sync_all()))
            .and_then(|()| std::fs::rename(&tmp, &path));
        if written.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        written.map_err(|e| format!("{}: {e}", path.display()))
    }
}

impl keys::Mute for FsHost {
    fn toggle_mute(&mut self) {
        self.audio.toggle_mute();
    }
}

/// Creates `dir/name` exclusively (`create_new`) and writes `bytes` into it with `write`:
/// `Ok(false)` when the name is taken (that file is left alone). When the first write fails
/// after the create (a full disk, say), the file this call created is removed before the error
/// is returned, so no empty or partial file that looks like a session demo is left behind and
/// the retry can take the same name.
fn claim_new(
    dir: &Path,
    name: &str,
    bytes: &[u8],
    write: impl FnOnce(&mut std::fs::File, &[u8]) -> std::io::Result<()>,
) -> Result<bool, String> {
    let path = dir.join(name);
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    std::fs::create_dir_all(dir).map_err(fail)?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path);
    match file {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(fail(e)),
        Ok(mut f) => match write(&mut f, bytes) {
            Ok(()) => Ok(true),
            Err(e) => {
                drop(f);
                let _ = std::fs::remove_file(&path);
                Err(fail(e))
            }
        },
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
    // wire 25 with sound (decision T15)
    let data = GameData::load_with_sound(&refs, game_mode(&o.iwad), o.map, o.skill)
        .map_err(|e| e.to_string())?;
    drop(files);
    if !display_available() {
        return Err("no display: neither DISPLAY nor WAYLAND_DISPLAY is set".into());
    }
    let cfg = SessionConfig {
        screenblocks: o.screenblocks,
        preset: o.preset,
        unrecorded_full_turning: o.unrecorded,
        ..SessionConfig::default()
    };
    // after the checks that can still refuse the launch; never waits during play
    let (audio, line) = Output::for_launch(o.no_sound, o.volume);
    if let Some(line) = line {
        eprintln!("{line}");
    }
    let mut host = FsHost {
        dir: o.sessions,
        audio,
    };
    let app = App::new(data, cfg, &mut host);
    window::play(
        app,
        host,
        title(!o.unrecorded),
        window::Presentation {
            gamma: o.gamma,
            square: o.square,
            fullscreen: o.fullscreen,
            quit_after: o.quit_after,
            tic_stats: o.tic_stats,
        },
    );
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match audio_probe_requested(&args) {
        Ok(true) => match audio::probe() {
            Ok(line) => {
                println!("{line}");
                0
            }
            Err(e) => {
                eprintln!("doom-play: audio probe: {e}");
                1
            }
        },
        Ok(false) => play(args),
        Err(e) => {
            eprintln!("doom-play: {e}");
            2
        }
    };
    std::process::exit(code);
}

/// Parses the arguments and plays; the process exit code.
fn play(args: Vec<String>) -> i32 {
    match parse(args) {
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
    }
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
        assert!(!o.unrecorded, "recorded unless asked");
        for other in [
            "--bindings modern",
            "--fullscreen",
            "--screenblocks 11",
            "--gamma 4",
        ] {
            let o = parse(args(&format!("--iwad a.wad {other}")))
                .unwrap()
                .unwrap();
            assert!(!o.unrecorded, "{other} does not imply the unrecorded mode");
        }
        let o = parse(args("--iwad a.wad --unrecorded-full-turning"))
            .unwrap()
            .unwrap();
        assert!(o.unrecorded);
        assert_eq!(title(true), "doom-play");
        assert!(title(false).contains(UNRECORDED_LABEL));
        assert!(HELP.contains(UNRECORDED_LABEL) && HELP.contains("cannot be replayed or verified"));
        let o = parse(args(
            "--iwad a.wad --wad b.wad --wad c.wad --map E1M3 --skill 3 --sessions s \
             --bindings modern --gamma 4 --aspect square --fullscreen --screenblocks 11 \
             --quit-after-tics 70",
        ))
        .unwrap()
        .unwrap();
        assert_eq!(o.wads, [PathBuf::from("b.wad"), PathBuf::from("c.wad")]);
        assert_eq!((o.map, o.skill, o.gamma, o.screenblocks), (3, 3, 4, 11));
        for (named, value) in [("hmp", 2), ("uv", 3), ("2", 2), ("3", 3)] {
            let o = parse(args(&format!("--iwad a.wad --skill {named}")))
                .unwrap()
                .unwrap();
            assert_eq!(o.skill, value, "{named}");
        }
        assert!(HELP.contains("Hurt Me Plenty") && HELP.contains("-skill 3 and -skill 4"));
        assert!(HELP.contains("dead player presses use") && HELP.contains("(175 tics)"));
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
            "--iwad a --skill nightmare",
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
    fn sound_options() {
        let o = parse(args("--iwad a.wad")).unwrap().unwrap();
        assert_eq!(
            (o.volume, o.no_sound),
            (Volume::FULL, false),
            "sound by default"
        );
        let o = parse(args("--iwad a.wad --no-sound --volume 0"))
            .unwrap()
            .unwrap();
        assert_eq!((o.volume, o.no_sound), (Volume::new(0).unwrap(), true));
        for (v, want) in [("0", 0), ("1", 1), ("37", 37), ("100", 100)] {
            let o = parse(args(&format!("--iwad a.wad --volume {v}")))
                .unwrap()
                .unwrap();
            assert_eq!(o.volume, Volume::new(want).unwrap(), "{v}");
        }
        for bad in [
            "--iwad a --volume 101",
            "--iwad a --volume 255",
            "--iwad a --volume 256",
            "--iwad a --volume -1",
            "--iwad a --volume 50%",
            "--iwad a --volume",
            "--iwad a --no-sound 1",
        ] {
            assert!(parse(args(bad)).is_err(), "{bad}");
        }
        for (other, unrecorded) in [("--no-sound", false), ("--volume 0", false)] {
            let o = parse(args(&format!("--iwad a.wad {other}")))
                .unwrap()
                .unwrap();
            assert_eq!(o.unrecorded, unrecorded, "{other} changes nothing else");
            assert_eq!((o.map, o.skill, o.screenblocks), (1, 2, 10));
        }
        assert!(HELP.contains("--volume N") && HELP.contains("--no-sound"));
        assert!(HELP.contains("End mutes and unmutes the sound"));
        assert!(HELP.contains(
            "The sound logic\nand mixer are matched to linuxdoom-1.10 built with its \
             in-process sound code (one define\nremoved), under the declared schedule"
        ));
        assert!(HELP.contains("is presentation and is not part of any claim"));
        assert!(HELP.contains("There is no music, menu"));
        assert!(!HELP.contains("There is no sound") && !HELP.contains("menu or sound"));
        assert!(!HELP.contains("does not use the device yet"));
    }

    #[test]
    fn the_audio_probe_is_asked_for_alone() {
        assert_eq!(audio_probe_requested(&args("--audio-probe")), Ok(true));
        for absent in ["", "--iwad a.wad", "--help"] {
            assert_eq!(audio_probe_requested(&args(absent)), Ok(false), "{absent}");
        }
        for mixed in ["--audio-probe --iwad a.wad", "--iwad a.wad --audio-probe"] {
            assert!(audio_probe_requested(&args(mixed)).is_err(), "{mixed}");
        }
        assert!(parse(args("--iwad a.wad --audio-probe")).is_err());
        assert!(HELP.contains("--audio-probe") && HELP.contains("digital silence"));
    }

    #[test]
    fn session_files_are_created_once_and_rewritten_through_a_synced_temporary() {
        let dir = std::env::temp_dir().join(format!("doom-play-test-{}", std::process::id()));
        let mut host = FsHost {
            dir: dir.clone(),
            audio: Output::Discard(audio::Discard::default()),
        };
        assert_eq!(host.claim("a.lmp", b"one"), Ok(true));
        assert_eq!(
            host.claim("a.lmp", b"two"),
            Ok(false),
            "an existing file is kept"
        );
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), b"one");
        host.save("a.lmp", b"three").unwrap();
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), b"three");
        // a failed rename (the target is a directory) leaves no temporary file
        std::fs::create_dir(dir.join("d.lmp")).unwrap();
        assert!(host.save("d.lmp", b"x").is_err());
        let mut names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        assert_eq!(names, ["a.lmp", "d.lmp"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Review finding 10: a first write that fails after the exclusive create removes the
    /// file it created (and only that one), so the next claim of the name succeeds.
    #[test]
    fn a_failed_first_write_removes_the_file_it_created() {
        let dir = std::env::temp_dir().join(format!("doom-play-claim-{}", std::process::id()));
        let full = |f: &mut std::fs::File, b: &[u8]| {
            f.write_all(&b[..2])?;
            Err(std::io::Error::other("disk full"))
        };
        let r = claim_new(&dir, "a.lmp", b"demo", full);
        assert!(r.as_ref().is_err_and(|e| e.ends_with("disk full")), "{r:?}");
        assert!(!dir.join("a.lmp").exists(), "the partial file is removed");
        // a file another claim made is never removed by a failing one
        let mut host = FsHost {
            dir: dir.clone(),
            audio: Output::Discard(audio::Discard::default()),
        };
        assert_eq!(host.claim("b.lmp", b"kept"), Ok(true));
        assert_eq!(claim_new(&dir, "b.lmp", b"x", full), Ok(false));
        assert_eq!(std::fs::read(dir.join("b.lmp")).unwrap(), b"kept");
        // the retry takes the freed name
        assert_eq!(host.claim("a.lmp", b"demo"), Ok(true));
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), b"demo");
        let mut names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        assert_eq!(names, ["a.lmp", "b.lmp"]);
        std::fs::remove_dir_all(&dir).unwrap();
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
