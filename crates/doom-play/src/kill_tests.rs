//! M5d: the session file survives `SIGKILL` at any instant as a valid demo of every tic run, or
//! of every tic but the last (the owner's requirement for Ctrl+C, a closed console window and a
//! lost display connection, all of which end the process without the app's save on quit).
//!
//! [`sigkill_at_random_points_leaves_valid_demos`] spawns this test executable again running the
//! ignored [`child_session`] (selected by name, with `DOOM_PLAY_KILL_CHILD_DIR` naming a fresh
//! directory under the system temporary directory and `DOOM_PLAY_KILL_SEED` a seed). The child
//! plays the real [`App`] with the real [`FsHost`] (sound discarded) on the synthetic WADs of the
//! M5c session check (`tests/reference/m5c_wads/sndmonst.wad` with the committed status bar, sound
//! and music WADs, E1M1 skill 2) as fast as it can on a fake clock, with a seeded pseudo-random
//! input script (keys, mouse motion and buttons, pauses, restarts; a death or another ending is
//! followed by a restart), and prints one line per event to its standard output, unbuffered:
//! `@start` before the first session, `@claim NAME` when a session's file is created,
//! `@tic NAME M HASH` from `Host::tic` for each committed tic (M the tics committed in that
//! session, HASH a running digest of the executed commands' demo bytes), `@end NAME LEN` after a
//! session ended and `@restart` before a restart. The parent kills it (`Child::kill`, `SIGKILL`)
//! after a seeded random delay counted from `@start` (a tenth of the kills within 0.3 ms and a
//! tenth within 3 ms, which land in or near the first claim) and checks every `.lmp` left: not empty; parsed by doom-sim's
//! reader through [`DemoCommands::parse`]; replayed headless by [`replay`] to its last command with
//! no refusal; its commands' digest equal to the one printed at its tic count; the file of the
//! session running at the kill holds M − 1 or M tics, where M is the last count printed for it
//! (the app prints a tic in `Host::tic` and then writes it); every earlier session's file holds
//! exactly its last printed count; a file whose claim line was not printed holds 0 tics and is
//! the only such file; every claimed file exists. Leftover `*.tmp` files are counted (they are
//! not demos); any other file fails the test.
//!
//! Environment: `DOOM_PLAY_KILLS` (default 50), `DOOM_PLAY_KILL_MAX_MS` (the longest ordinary
//! delay, default 400), `DOOM_PLAY_KILL_LONG_MS` (a tenth of the kills wait 1 s plus up to this,
//! default 1600), `DOOM_PLAY_KILL_SEED` (the first seed, default 1; kill k uses seed + k),
//! `DOOM_PLAY_KILL_WADS` (newline-separated WAD paths, the first decides the game mode as
//! doom-play's `--iwad` does; default the synthetic set), `DOOM_PLAY_KILL_MAP` (default 1),
//! `DOOM_PLAY_KILL_AT_TIC` (aimed kills: wait for the first `@tic` line of this count, in any
//! session, then up to `DOOM_PLAY_KILL_AT_US` microseconds, default 3000; for example 1021, the
//! first page-boundary rewrite, or 1050, the first periodic rewrite) and
//! `DOOM_PLAY_KILL_KEEP` (a directory: each kill's files are kept under it as `kNNNN/`, for
//! `tools/reference/m5d_kill_check.py`; by default they are deleted). One summary line goes to
//! standard output (seen with `--nocapture`).

use crate::FsHost;
use crate::audio::{Discard, Output};
use doom_game::app::{
    App, AudioSink, DemoCommands, GameData, Host, Mode, SessionConfig, TicReport, Timestamp, replay,
};
use doom_game::input::{MouseButton, keys, write_ticcmd_bytes};
use doom_render::GameMode;
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const CHILD_DIR: &str = "DOOM_PLAY_KILL_CHILD_DIR";
const SEED: &str = "DOOM_PLAY_KILL_SEED";
const WADS: &str = "DOOM_PLAY_KILL_WADS";
const MAP: &str = "DOOM_PLAY_KILL_MAP";
/// The child's name as the test harness selects it.
const CHILD_TEST: &str = "kill_tests::child_session";
/// The longest a child plays before it stops by itself (it is normally killed long before).
const CHILD_LIMIT: Duration = Duration::from_secs(120);

/// splitmix64: a small seeded generator for the input script and the delays.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n` (n > 0).
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    /// True with probability 1/n.
    fn one_in(&mut self, n: u64) -> bool {
        self.below(n) == 0
    }
}

fn fnv_step(h: u64, bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(h, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3))
}

const FNV0: u64 = 0xcbf2_9ce4_8422_2325;

fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

/// The WAD paths: `DOOM_PLAY_KILL_WADS`, or the synthetic set of the M5c session check.
fn wad_paths() -> Vec<PathBuf> {
    match std::env::var(WADS) {
        Ok(list) if !list.trim().is_empty() => list
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect(),
        _ => {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
            [
                "m5c_wads/sndmonst.wad",
                "m5b_gfx.wad",
                "m5c_sfx.wad",
                "m5c_music.wad",
            ]
            .iter()
            .map(|w| root.join(w))
            .collect()
        }
    }
}

fn map() -> u8 {
    std::env::var(MAP)
        .ok()
        .and_then(|m| m.parse().ok())
        .unwrap_or(1)
}

/// The game data as doom-play loads it (wire 25 with sound; the mode from the first file's name).
fn load() -> GameData {
    let paths = wad_paths();
    let files: Vec<Vec<u8>> = paths
        .iter()
        .map(|p| std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let mode = match std::env::var(WADS) {
        Ok(l) if !l.trim().is_empty() => crate::game_mode(&paths[0]),
        _ => GameMode::Registered,
    };
    GameData::load_with_sound(&refs, mode, map(), 2).expect("game data")
}

/// The child's host: doom-play's [`FsHost`], with the protocol lines.
struct ChildHost {
    fs: FsHost,
    out: std::io::Stdout,
    name: String,
    hash: u64,
}

impl ChildHost {
    /// One protocol line in one write; a closed pipe (the parent is gone) ends the child.
    fn line(&mut self, text: String) {
        let mut out = self.out.lock();
        if out
            .write_all(text.as_bytes())
            .and_then(|()| out.flush())
            .is_err()
        {
            std::process::exit(0);
        }
    }
}

impl Host for ChildHost {
    fn timestamp(&mut self) -> Timestamp {
        self.fs.timestamp()
    }
    fn exists(&mut self, name: &str) -> bool {
        self.fs.exists(name)
    }
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        let claimed = self.fs.claim(name, bytes);
        if claimed == Ok(true) {
            self.name = name.to_string();
            self.hash = FNV0;
            self.line(format!("@claim {name}\n"));
        }
        claimed
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.fs.save(name, bytes)
    }
    fn appends(&self) -> bool {
        self.fs.appends()
    }
    fn append(&mut self, name: &str, at: u64, tail: &[u8; 5]) -> Result<bool, String> {
        self.fs.append(name, at, tail)
    }
    fn tic(&mut self, r: &TicReport<'_>) {
        self.hash = fnv_step(self.hash, &write_ticcmd_bytes(&r.executed));
        let line = format!("@tic {} {} {:016x}\n", self.name, r.tic + 1, self.hash);
        self.line(line);
    }
    fn audio(&mut self) -> Option<&mut dyn AudioSink> {
        self.fs.audio()
    }
}

/// The keys the script presses: movement, turning, fire, use, run, strafe and weapons.
const KEYS: [u8; 14] = [
    keys::KEY_UPARROW,
    keys::KEY_DOWNARROW,
    keys::KEY_LEFTARROW,
    keys::KEY_RIGHTARROW,
    keys::KEY_RCTRL,
    keys::KEY_RSHIFT,
    keys::KEY_RALT,
    b' ',
    b',',
    b'.',
    b'1',
    b'2',
    b'3',
    b'4',
];

/// The child: plays until killed (or [`CHILD_LIMIT`]). Does nothing unless the parent named its
/// directory.
#[test]
#[ignore = "the child of sigkill_at_random_points_leaves_valid_demos; runs only when spawned by it"]
fn child_session() {
    let Some(dir) = std::env::var_os(CHILD_DIR) else {
        return;
    };
    let seed: u64 = std::env::var(SEED)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let mut rng = Rng(seed);
    let data = load();
    let mut host = ChildHost {
        fs: FsHost {
            dir: PathBuf::from(dir),
            audio: Output::Discard(Discard::default()),
        },
        out: std::io::stdout(),
        name: String::new(),
        hash: FNV0,
    };
    // after a newline: the harness has printed "test kill_tests::child_session ... " unended
    host.line("\n@start\n".into());
    let started = Instant::now();
    let mut app = App::new(data, SessionConfig::default(), &mut host);
    let mut k = 0u64;
    app.update(0, &mut host);
    while started.elapsed() < CHILD_LIMIT {
        k += 1;
        match app.mode() {
            Mode::Playing => {
                if rng.one_in(3) {
                    let key = KEYS[rng.below(KEYS.len() as u64) as usize];
                    app.key(key, rng.one_in(2), &mut host);
                }
                if rng.one_in(3) {
                    let (dx, dy) = (rng.below(81) as i32 - 40, rng.below(21) as i32 - 10);
                    app.mouse_motion(dx, dy);
                }
                if rng.one_in(12) {
                    let b = [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
                        [rng.below(3) as usize];
                    app.mouse_button(b, rng.one_in(2));
                }
                if rng.one_in(1500) {
                    app.key(keys::KEY_ESCAPE, true, &mut host);
                    app.key(keys::KEY_ESCAPE, false, &mut host);
                }
                if rng.one_in(2500) {
                    host.line("@restart\n".into());
                    app.restart(&mut host);
                }
            }
            Mode::Paused => {
                // resume, or restart from the pause
                if rng.one_in(4) {
                    host.line("@restart\n".into());
                    app.key(b'r', true, &mut host);
                } else {
                    app.key(keys::KEY_ESCAPE, true, &mut host);
                }
            }
            Mode::Ended(_) => {
                let len = app.recorder().len();
                let name = app.session_name().to_string();
                host.line(format!("@end {name} {len}\n"));
                host.line("@restart\n".into());
                app.restart(&mut host);
            }
        }
        app.update(at(k), &mut host);
    }
}

/// What the child printed about one session.
#[derive(Debug, Default)]
struct Logged {
    /// The last tic count printed, 0 before the first tic.
    last: u64,
    /// The digest printed with each count.
    hashes: BTreeMap<u64, u64>,
    /// The recorded length printed after the session ended.
    ended: Option<u64>,
}

/// Where the kill landed, judged from the child's last line.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Landed {
    /// Before or during the claim of a session's file (no claim line after `@start` or
    /// `@restart`).
    Claim,
    /// After the print of a tic whose write is a whole rewrite (a page-boundary tic or a
    /// `SAVE_INTERVAL` tic): before, during or after that rewrite.
    Rewrite,
    /// After the print of any other tic, or of a claim.
    Session,
    /// After a session's end.
    Ended,
}

#[derive(Debug, Default)]
struct Stats {
    kills: u64,
    files: u64,
    tics: u64,
    tmp: u64,
    no_file: u64,
    landed: BTreeMap<String, u64>,
    current_behind: u64,
    current_even: u64,
    longest: u64,
    past_page_rewrite: u64,
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Whether the write of the tic that makes `m` tics is a whole rewrite: a page-boundary tic
/// (`session_file::append_fits_one_page` refuses the offset of m − 1 tics) or a `SAVE_INTERVAL`
/// tic.
fn rewrite_tic(m: u64) -> bool {
    m > 0
        && (m % doom_game::app::SAVE_INTERVAL as u64 == 0
            || !crate::session_file::append_fits_one_page(13 + 4 * (m - 1), 5))
}

/// A host for the headless replay: hears nothing.
struct Quiet;

impl Host for Quiet {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
        Ok(())
    }
}

/// Checks one kill's directory against the child's lines; returns where the kill landed or the
/// first failure.
fn check(dir: &Path, lines: &[String], data: &GameData, stats: &mut Stats) -> Result<(), String> {
    let mut sessions: Vec<(String, Logged)> = Vec::new();
    let mut last_event = "";
    let mut claim_pending = false;
    for l in lines.iter().filter(|l| l.starts_with('@')) {
        let w: Vec<&str> = l.split(' ').collect();
        match w.as_slice() {
            ["@start"] | ["@restart"] => claim_pending = true,
            ["@claim", name] => {
                sessions.push((name.to_string(), Logged::default()));
                claim_pending = false;
            }
            ["@tic", name, m, h] => {
                let (n, s) = sessions.last_mut().ok_or("a tic before any claim")?;
                if n != name {
                    return Err(format!("tic of {name} while {n} is current"));
                }
                let m: u64 = m.parse().map_err(|_| format!("bad line {l:?}"))?;
                let h = u64::from_str_radix(h, 16).map_err(|_| format!("bad line {l:?}"))?;
                if m != s.last + 1 {
                    return Err(format!("{name}: tic {m} after {}", s.last));
                }
                s.last = m;
                s.hashes.insert(m, h);
            }
            ["@end", name, len] => {
                let (n, s) = sessions.last_mut().ok_or("an end before any claim")?;
                if n != name {
                    return Err(format!("end of {name} while {n} is current"));
                }
                s.ended = Some(len.parse().map_err(|_| format!("bad line {l:?}"))?);
            }
            _ => return Err(format!("unexpected line {l:?}")),
        }
        last_event = w[0];
    }
    if last_event.is_empty() {
        return Err("the child printed nothing (not even @start)".into());
    }
    let landed = match (claim_pending, last_event, sessions.last()) {
        (true, _, _) => Landed::Claim,
        (_, "@end", _) => Landed::Ended,
        (_, "@tic", Some((_, s))) if rewrite_tic(s.last) => Landed::Rewrite,
        _ => Landed::Session,
    };
    *stats.landed.entry(format!("{landed:?}")).or_default() += 1;

    let mut lmps = BTreeMap::new();
    for e in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name().into_string().map_err(|n| format!("{n:?}"))?;
        if name.ends_with(".lmp") {
            lmps.insert(name, std::fs::read(e.path()).map_err(|e| e.to_string())?);
        } else if name.ends_with(".tmp") {
            stats.tmp += 1;
        } else {
            return Err(format!("unexpected file {name}"));
        }
    }
    stats.no_file += u64::from(lmps.is_empty());
    let current = sessions.last().map(|(n, _)| n.clone());
    let mut unlogged = 0;
    for (name, bytes) in &lmps {
        if bytes.is_empty() {
            return Err(format!("{name} is empty"));
        }
        let commands = DemoCommands::parse(bytes, data.map())
            .map_err(|e| format!("{name} ({} bytes) is not a valid demo: {e:?}", bytes.len()))?;
        if let Some(end) = replay(data, 10, &commands, &mut Quiet) {
            return Err(format!("{name}: the replay ended early: {end:?}"));
        }
        let tics = commands.commands().len() as u64;
        let hash = commands
            .commands()
            .iter()
            .fold(FNV0, |h, c| fnv_step(h, &write_ticcmd_bytes(c)));
        stats.files += 1;
        stats.tics += tics;
        stats.longest = stats.longest.max(tics);
        stats.past_page_rewrite += u64::from(tics > 1021);
        let Some((_, s)) = sessions.iter().find(|(n, _)| n == name) else {
            unlogged += 1;
            if tics != 0 || !claim_pending {
                return Err(format!(
                    "{name}: {tics} tics, but its claim was never printed (claim pending \
                     {claim_pending})"
                ));
            }
            continue;
        };
        let allowed: Vec<u64> = match s.ended {
            Some(len) => vec![len],
            None if current.as_deref() == Some(name) => {
                if tics + 1 == s.last {
                    stats.current_behind += 1;
                } else if tics == s.last {
                    stats.current_even += 1;
                }
                vec![s.last.saturating_sub(1), s.last]
            }
            None => vec![s.last],
        };
        if !allowed.contains(&tics) {
            return Err(format!(
                "{name}: {tics} tics, allowed {allowed:?} (last printed {}, ended {:?}, current {})",
                s.last,
                s.ended,
                current.as_deref() == Some(name)
            ));
        }
        if tics > 0 && s.hashes.get(&tics) != Some(&hash) {
            return Err(format!(
                "{name}: the commands' digest at {tics} tics differs from the child's"
            ));
        }
    }
    if unlogged > 1 {
        return Err(format!("{unlogged} files whose claim was never printed"));
    }
    for (name, _) in &sessions {
        if !lmps.contains_key(name) {
            return Err(format!("{name} was claimed and is gone"));
        }
    }
    Ok(())
}

/// One kill: spawns the child, waits for `@start` (and, with `at_tic`, then for the first `@tic`
/// line of that count), sleeps `delay`, kills it and returns its lines.
fn kill_one(
    exe: &Path,
    dir: &Path,
    seed: u64,
    at_tic: Option<u64>,
    delay: Duration,
) -> Result<Vec<String>, String> {
    let mut child = Command::new(exe)
        .args([
            CHILD_TEST,
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_DIR, dir)
        .env(SEED, seed.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn the child: {e}"))?;
    let stdout = child.stdout.take().expect("piped");
    // a thread drains the pipe, so the child never blocks on a full pipe while it plays
    let (tx, rx) = mpsc::channel::<String>();
    let reader = std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut seen_start = false;
    let started = loop {
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(l) => {
                seen_start |= l == "@start";
                let due = match at_tic {
                    None => l == "@start",
                    Some(n) => {
                        l.starts_with("@tic ") && l.split(' ').nth(2) == Some(&n.to_string())
                    }
                };
                lines.push(l);
                if due {
                    break true;
                }
            }
            Err(_) => break false,
        }
    };
    if started {
        std::thread::sleep(delay);
    }
    let _ = child.kill();
    let status = child.wait().map_err(|e| format!("reap the child: {e}"))?;
    reader.join().map_err(|_| "the reader panicked")?;
    lines.extend(rx.try_iter());
    if !started {
        return Err(match (seen_start, at_tic) {
            (true, Some(n)) => format!("the child never printed tic {n} within 120 s"),
            _ => "the child never started".into(),
        });
    }
    if status.success() {
        return Err(format!("the child exited by itself ({status}), not killed"));
    }
    Ok(lines)
}

/// The parent: kills the child at random points and checks what it left (see the module
/// documentation). The check of one kill's files runs on a second thread while the next child
/// plays.
#[test]
fn sigkill_at_random_points_leaves_valid_demos() {
    let kills = env_u64("DOOM_PLAY_KILLS", 50);
    let max_ms = env_u64("DOOM_PLAY_KILL_MAX_MS", 400).max(1);
    let long_ms = env_u64("DOOM_PLAY_KILL_LONG_MS", 1600).max(1);
    let at_tic = std::env::var("DOOM_PLAY_KILL_AT_TIC")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());
    let at_us = env_u64("DOOM_PLAY_KILL_AT_US", 3_000).max(1);
    let seed0 = env_u64(SEED, 1);
    let keep = std::env::var_os("DOOM_PLAY_KILL_KEEP").map(PathBuf::from);
    let base = keep.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("doom-play-kill-{}", std::process::id()))
    });
    let exe = std::env::current_exe().expect("the test executable");
    let data = load();
    let mut rng = Rng(seed0 ^ 0x5eed);
    let clock = Instant::now();
    let (jobs, queue) = mpsc::channel::<(u64, PathBuf, Vec<String>)>();
    let result = std::thread::scope(|scope| {
        let checker = scope.spawn(|| -> Result<Stats, String> {
            let mut stats = Stats::default();
            for (seed, dir, lines) in queue {
                stats.kills += 1;
                check(&dir, &lines, &data, &mut stats)
                    .map_err(|e| format!("seed {seed} (files kept in {}): {e}", dir.display()))?;
                if keep.is_none() {
                    std::fs::remove_dir_all(&dir).map_err(|e| format!("{e}"))?;
                }
            }
            Ok(stats)
        });
        for k in 0..kills {
            let seed = seed0 + k;
            let dir = base.join(format!("k{k:04}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("the kill's directory");
            // a tenth within 0.3 ms and a tenth within 3 ms of `@start` (in or near the first
            // claim), a tenth late (past the first page-boundary rewrite in an optimised build),
            // the rest anywhere up to `max_ms`
            let delay = match rng.below(10) {
                _ if at_tic.is_some() => Duration::from_micros(rng.below(at_us)),
                0 => Duration::from_micros(rng.below(300)),
                1 => Duration::from_micros(rng.below(3_000)),
                2 => Duration::from_millis(1_000 + rng.below(long_ms)),
                _ => Duration::from_micros(rng.below(max_ms * 1_000)),
            };
            let lines = kill_one(&exe, &dir, seed, at_tic, delay)
                .unwrap_or_else(|e| panic!("kill {k} (seed {seed}): {e}"));
            if jobs.send((seed, dir, lines)).is_err() {
                break; // the checker stopped at a failure
            }
        }
        drop(jobs);
        checker.join().expect("the checker")
    });
    let stats = result.unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(stats.kills, kills);
    if keep.is_none() {
        let _ = std::fs::remove_dir(&base);
    }
    println!(
        "kill test: {} kills (seeds {seed0}..{}), {} files checked holding {} tics (longest {}, \
         {} past the first page-boundary rewrite), landed {:?}, current file one tic behind {} \
         and even {}, kills that left no .lmp {}, leftover .tmp {}, {:.1} s",
        stats.kills,
        seed0 + kills,
        stats.files,
        stats.tics,
        stats.longest,
        stats.past_page_rewrite,
        stats.landed,
        stats.current_behind,
        stats.current_even,
        stats.no_file,
        stats.tmp,
        clock.elapsed().as_secs_f64()
    );
}
