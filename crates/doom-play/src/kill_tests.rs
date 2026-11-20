//! M5d: the session file survives `SIGKILL` at any instant as a valid demo of a prefix of the
//! session (the owner's requirement for Ctrl+C, a closed console window and a lost display
//! connection, all of which end the process without the app's save on quit). Since the session
//! writer thread (`session_writer.rs`; the owner's rulings of 2026-11-20) the file lags the game:
//! it holds at least every tic of the writer's last completed write and at most every tic run, a
//! declared weakening of the earlier "every tic run, or every tic but the last" that the owner
//! accepted; the test measures the lag.
//!
//! [`sigkill_at_random_points_leaves_valid_demos`] spawns this test executable again running the
//! ignored [`child_session`] (selected by name, with `DOOM_PLAY_KILL_CHILD_DIR` naming a fresh
//! directory and `DOOM_PLAY_KILL_SEED` a seed). The child plays the real [`App`] with doom-play's
//! own [`WriterHost`] over [`FsHost`]'s directory (sound discarded) on the synthetic WADs of the
//! M5c session check (`tests/reference/m5c_wads/sndmonst.wad` with the committed status bar, sound
//! and music WADs, E1M1 skill 2) as fast as it can on a fake clock, with a seeded pseudo-random
//! input script (keys, mouse motion and buttons, pauses, restarts; a death or another ending is
//! followed by a restart), and prints one line per event to its standard output, unbuffered:
//! `@start` before the first session, `@claim NAME` when the host answers a session's claim
//! (the writer creates the file afterwards), `@tic NAME M HASH W` from `Host::tic` for each
//! committed tic (M the tics committed in that session, HASH a running digest of the executed
//! commands' demo bytes, W the tics the file held after the writer's last completed write or `-`
//! before its claim completed), `@end NAME LEN` after a session ended and `@restart` before a
//! restart. The parent kills it (`Child::kill`, `SIGKILL`) after a seeded random delay counted
//! from `@start` (a tenth of the kills within 0.3 ms and a tenth within 3 ms, aimed at the first
//! claim), requires that the child died by that `SIGKILL` (any other end, such as a panic, fails
//! the test and shows the child's standard error), and checks every `.lmp` left by the rules of
//! [`check`]: not empty; parsed by doom-sim's reader through [`DemoCommands::parse`]; replayed
//! headless by [`replay`] to its last command with no refusal; its commands' digest equal to the
//! one printed at its tic count; between the last W and the last M printed for its session
//! (exactly M once a later session's claim completed); missing only before its claim completed;
//! a file whose claim line was not printed holds 0 tics and is the only such file; and the **lag
//! bound** (review 3, m1): the session running at the kill, once its claim completed, has lost at
//! most [`MAX_LOST`] tics (M minus the tics in its file). Leftover `*.tmp` files are counted
//! (they are not demos); any other file fails the test. The summary gives the tics lost by the
//! session running at each kill as a distribution, and the bound applied. The child runs far
//! faster than 35 tics a second, so its lag is larger than in play: the distribution is the worst
//! case of this machine's disk, not the player's.
//!
//! **The lag bound.** [`MAX_LOST`] is three times the writer's queue bound
//! ([`crate::session_writer::QUEUE_TICS`], 175 tics). While the writer keeps up, M minus the tics
//! in the file is at most the tics queued, so at most 176 (the queue's 175 and the tic printed
//! before the app hands it over); the bound is reached only after the queue has filled and the app
//! has fallen back to whole demos for 350 more tics, that is when the writer has made no progress
//! for as long as the child takes to play 525 tics (about 0.75 s for this machine's debug child,
//! about 0.2 to 0.3 s for its release child). Observed at `3957cf0` on this machine's ext4, in
//! release: at most 77 tics lost over 2,000 random kills and at most 63 in kills aimed within
//! 60 ms of the first page-boundary or periodic rewrite, so a healthy temporary directory meets it
//! more than six times over; a writer that has stopped exceeds it as soon as the child has played
//! 525 tics past the stop (the `stall` self-test). Kills before the session's claim completed are
//! not bound by it (the claim's flushed temporary file can take long on a slow disk, and the file
//! does not exist yet). On a slow disk (WSL's `/mnt/d` over 9p, observed up to hundreds of tics)
//! give `DOOM_PLAY_KILL_MAX_LOST`.
//!
//! Environment: `DOOM_PLAY_KILLS` (default 50), `DOOM_PLAY_KILL_MAX_MS` (the longest ordinary
//! delay, default 400), `DOOM_PLAY_KILL_LONG_MS` (a tenth of the kills wait 1 s plus up to this,
//! default 1600), `DOOM_PLAY_KILL_SEED` (the first seed, default 1; kill k uses seed + k),
//! `DOOM_PLAY_KILL_WADS` (newline-separated WAD paths, the first decides the game mode as
//! doom-play's `--iwad` does; default the synthetic set), `DOOM_PLAY_KILL_MAP` (default 1),
//! `DOOM_PLAY_KILL_AT_TIC` (aimed kills: wait for the first `@tic` line of this count, in any
//! session, then up to `DOOM_PLAY_KILL_AT_US` microseconds, default 3000; for example 1021, the
//! first page-boundary rewrite, or 1050, the first periodic rewrite),
//! `DOOM_PLAY_KILL_KEEP` (a directory: each kill's files are kept under it as `kNNNN/`, for
//! `tools/reference/m5d_kill_check.py`; an existing `kNNNN/` there is replaced; by default the
//! files go under the system temporary directory and are deleted), `DOOM_PLAY_KILL_DIR` (a
//! directory, for example on another disk: the kills' directories `kNNNN/` are made there and
//! kept, and the test deletes nothing; a `kNNNN/` that already exists fails the run, so give it a
//! new, empty directory each time; it takes precedence over `DOOM_PLAY_KILL_KEEP`) and
//! `DOOM_PLAY_KILL_MAX_LOST` (the lag bound in tics, default [`MAX_LOST`], or `none` to lift it;
//! the summary line says which) and `DOOM_PLAY_KILL_PLANT` (a planted fault in the child, for the
//! self-tests only, after which the test must fail: `torn` makes the writer's appends two writes
//! per tic, the tic's four bytes and then the marker, [`TORN_GAP`] apart, instead of doom-play's
//! one write; `early` makes every file operation [`EARLY_DELAY`] slow and the child print W
//! [`EARLY_BY`] tics too high, capped at M − 1; `stall` makes the writer stop for good before
//! the first file operation that would leave the file holding more than [`STALL_PAST`] tics). One
//! summary line goes to standard output (seen with
//! `--nocapture`).
//!
//! **Self-tests** (each run by default; each stops at its first failure, which it requires):
//! - [`a_torn_append_fails_the_kill_test`]: aimed kills with the `torn` plant must leave a `.lmp`
//!   that is not a valid demo, so the harness is shown to fail when the file can be torn. It shows
//!   that the checks catch a torn file when a kill lands in a gap of a millisecond; a kill cannot
//!   be aimed inside the microseconds of a real two-write append, which is why the one write is
//!   pinned by the deterministic tests in `session_file` instead.
//! - [`a_w_printed_too_high_fails_the_kill_test`]: with the `early` plant a file must hold fewer
//!   tics than the W printed for it (the W rule; on a healthy disk the writer keeps up so closely
//!   that a W a few tics too high, capped below M, is rarely contradicted, hence the slow
//!   operations).
//! - [`a_stopped_writer_fails_the_lag_bound`]: with the `stall` plant, kills aimed at tic 1,000
//!   must exceed the lag bound.
//!
//! **What a run reaches depends on where and how it runs.** Whether kills land inside the claim
//! of a session's file or inside a whole rewrite depends on the temporary directory's file system
//! (on tmpfs a claim takes microseconds and kills rarely or never land in it; on a disk-backed
//! file system they do) and on the build profile (a debug child runs slower, and the default
//! delays rarely carry it past the first page-boundary rewrite at 1,021 tics or the first
//! periodic one at 1,050). A default run in CI's configuration (debug, tmpfs) therefore exercises
//! mainly the steady-state append. The claim and the whole rewrite are covered by runs with
//! `TMPDIR` on a disk-backed file system and by aimed runs (`DOOM_PLAY_KILL_AT_TIC`), as the local
//! tool and the aimed runs of the M5d record do.

use crate::FsHost;
use crate::audio::{Discard, Output};
use crate::session_file::Claim;
use crate::session_writer::{RealIo, SessionIo, WriterHost};
use doom_game::app::{
    App, AudioSink, DemoCommands, GameData, Host, Mode, SessionConfig, TicReport, Timestamp, replay,
};
use doom_game::input::{MouseButton, keys, write_ticcmd_bytes};
use doom_render::GameMode;
use std::collections::BTreeMap;
use std::io::{BufRead, Read, Seek, SeekFrom, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const CHILD_DIR: &str = "DOOM_PLAY_KILL_CHILD_DIR";
const SEED: &str = "DOOM_PLAY_KILL_SEED";
const WADS: &str = "DOOM_PLAY_KILL_WADS";
const MAP: &str = "DOOM_PLAY_KILL_MAP";
const PLANT: &str = "DOOM_PLAY_KILL_PLANT";
const MAX_LOST_VAR: &str = "DOOM_PLAY_KILL_MAX_LOST";
/// The pause between the two writes of the `torn` plant's append: wide enough that aimed kills
/// land in it.
const TORN_GAP: Duration = Duration::from_millis(1);
/// The `early` plant's delay before each file operation, so that the writer lags by a few tics.
const EARLY_DELAY: Duration = Duration::from_millis(5);
/// How many tics too high the `early` plant prints W (capped at M − 1).
const EARLY_BY: u64 = 3;
/// The `stall` plant's writer stops for good before the first file operation that would leave
/// the file holding more than this many tics (review 4, N2: counted in tics, not operations, so
/// that a slow first operation, after which one catch-up rewrite holds hundreds of tics, cannot
/// carry the file close to the aimed tic). Kills aimed at tic 1,000 then find at least 800 tics
/// lost, 275 more than the lag bound, however few or slow the operations were.
const STALL_PAST: u64 = 200;
/// The lag bound by default (see the module documentation): three times the writer's queue bound.
const MAX_LOST: u64 = 3 * crate::session_writer::QUEUE_TICS;
/// The signal `Child::kill` sends on Unix (`SIGKILL`, 9 on Linux and macOS).
const SIGKILL: i32 = 9;
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

/// The child's host: doom-play's own [`WriterHost`] (the writer thread over [`FsHost`]'s
/// directory), with the protocol lines.
struct ChildHost {
    fs: WriterHost,
    out: std::io::Stdout,
    name: String,
    hash: u64,
    /// The `early` plant: W printed [`EARLY_BY`] too high, capped at M − 1.
    early: bool,
}

/// The `early` and `stall` plants' file operations (the self-tests only, never doom-play's):
/// doom-play's own, each after `delay`; the writer stops for good before an operation that would
/// leave the file holding more than `stall_past` tics.
struct PlantIo {
    delay: Duration,
    stall_past: Option<u64>,
}

/// The tics a demo of `len` bytes holds (13 header bytes, 4 a tic, the end marker).
fn demo_tics(len: usize) -> u64 {
    (len as u64).saturating_sub(14) / 4
}

impl PlantIo {
    /// Before an operation after which the file holds `tics` tics.
    fn before(&mut self, tics: u64) {
        if self.stall_past.is_some_and(|n| tics > n) {
            loop {
                std::thread::sleep(Duration::from_secs(3_600));
            }
        }
        std::thread::sleep(self.delay);
    }
}

impl SessionIo for PlantIo {
    fn claim(&mut self, dir: &Path, name: &str, bytes: &[u8]) -> Result<Claim, String> {
        self.before(demo_tics(bytes.len()));
        RealIo.claim(dir, name, bytes)
    }
    fn rewrite(
        &mut self,
        dir: &Path,
        name: &str,
        bytes: &[u8],
        synced: bool,
    ) -> Result<(), String> {
        self.before(demo_tics(bytes.len()));
        RealIo.rewrite(dir, name, bytes, synced)
    }
    fn append(
        &mut self,
        dir: &Path,
        name: &str,
        at: u64,
        tails: &[[u8; 5]],
    ) -> Result<bool, String> {
        // the first tail is tic (at − 13) / 4 + 1, the last that plus the others
        self.before(at.saturating_sub(13) / 4 + tails.len() as u64);
        RealIo.append(dir, name, at, tails)
    }
}

/// The `torn` plant's file operations (the self-test only, never doom-play's): the claim and the
/// whole rewrite as doom-play makes them, but each tic of an append as two writes, the tic's four
/// bytes and then the marker, [`TORN_GAP`] apart, so that a kill in the gap leaves a file with no
/// marker.
struct TornIo;

impl SessionIo for TornIo {
    fn claim(&mut self, dir: &Path, name: &str, bytes: &[u8]) -> Result<Claim, String> {
        RealIo.claim(dir, name, bytes)
    }
    fn rewrite(
        &mut self,
        dir: &Path,
        name: &str,
        bytes: &[u8],
        synced: bool,
    ) -> Result<(), String> {
        RealIo.rewrite(dir, name, bytes, synced)
    }
    fn append(
        &mut self,
        dir: &Path,
        name: &str,
        at: u64,
        tails: &[[u8; 5]],
    ) -> Result<bool, String> {
        let offsets = (0..tails.len() as u64).map(|i| at + 4 * i);
        if offsets
            .clone()
            .any(|o| !crate::session_file::append_fits_one_page(o, 5))
        {
            return Ok(false);
        }
        let path = dir.join(name);
        let fail = |e: std::io::Error| format!("{}: {e}", path.display());
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .map_err(fail)?;
        if f.metadata().map_err(fail)?.len() != at + 1 {
            return Ok(false);
        }
        for (o, tail) in offsets.zip(tails) {
            f.seek(SeekFrom::Start(o)).map_err(fail)?;
            f.write_all(&tail[..4]).map_err(fail)?;
            std::thread::sleep(TORN_GAP);
            f.write_all(&tail[4..]).map_err(fail)?;
        }
        Ok(true)
    }
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
    /// `@tic NAME M HASH W`: W the tics the file held after the writer's last completed write,
    /// `-` before its claim completed. Printed before the app hands tic M to the writer.
    fn tic(&mut self, r: &TicReport<'_>) {
        self.hash = fnv_step(self.hash, &write_ticcmd_bytes(&r.executed));
        let m = r.tic + 1;
        let early = self.early;
        let w = self
            .fs
            .written(&self.name)
            .map(|w| if early { (w + EARLY_BY).min(m - 1) } else { w })
            .map_or_else(|| "-".to_string(), |w| w.to_string());
        let line = format!("@tic {} {m} {:016x} {w}\n", self.name, self.hash);
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
    let plant = std::env::var(PLANT).ok();
    let io: Box<dyn SessionIo> = match plant.as_deref() {
        None => Box::new(RealIo),
        Some("torn") => Box::new(TornIo),
        Some("early") => Box::new(PlantIo {
            delay: EARLY_DELAY,
            stall_past: None,
        }),
        Some("stall") => Box::new(PlantIo {
            delay: Duration::ZERO,
            stall_past: Some(STALL_PAST),
        }),
        Some(p) => panic!("{PLANT}={p}: the plants are torn, early and stall"),
    };
    let mut rng = Rng(seed);
    let data = load();
    let fs = FsHost {
        dir: PathBuf::from(dir),
        audio: Output::Discard(Discard::default()),
    };
    let mut host = ChildHost {
        fs: WriterHost::with_io(fs, io, Box::new(|line| eprintln!("{line}"))).expect("the writer"),
        out: std::io::stdout(),
        name: String::new(),
        hash: FNV0,
        early: plant.as_deref() == Some("early"),
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
    /// The last W printed (the tics the file held after the writer's last completed write);
    /// `None` while every W printed was `-` (the claim not completed).
    written: Option<u64>,
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
    /// The tics lost by the session running at each kill (its last printed count M minus the
    /// tics its file holds; a missing file counts as M lost), by count.
    lost: BTreeMap<u64, u64>,
    longest: u64,
    past_page_rewrite: u64,
}

/// The lag bound: `DOOM_PLAY_KILL_MAX_LOST` (tics, or `none` to lift it), else [`MAX_LOST`].
fn max_lost() -> Option<u64> {
    match std::env::var(MAX_LOST_VAR) {
        Err(_) => Some(MAX_LOST),
        Ok(v) if v.trim() == "none" => None,
        Ok(v) => Some(
            v.trim()
                .parse()
                .unwrap_or_else(|_| panic!("{MAX_LOST_VAR}={v}: give a number of tics, or none")),
        ),
    }
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
/// first failure. The rules (the writer thread's guarantee, which the owner accepted on
/// 2026-11-20 as a declared weakening): every `.lmp` is a valid demo of a prefix of its session:
/// it is not empty, parses, replays with no refusal and its digest is the child's at its tic
/// count. A session's file holds at least the last W printed for it (0 when none) and at most its
/// last printed count M (its `@end` length once ended); once a later session printed a W (its
/// claim completed, so the writer, which works in order, had finished every write of the earlier
/// sessions), exactly M. A session's file may be missing only while no W was printed for it or
/// for a later session (its claim had not completed). A file whose claim line was not printed
/// holds 0 tics and is the only one. The session running at the kill, if its claim completed
/// (a W was printed for it) and it had not ended, has lost at most `max_lost` tics (its last
/// printed count minus the tics in its file), unless the bound is lifted (`None`).
fn check(
    dir: &Path,
    lines: &[String],
    data: &GameData,
    max_lost: Option<u64>,
    stats: &mut Stats,
) -> Result<(), String> {
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
            ["@tic", name, m, h, written] => {
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
                if *written != "-" {
                    let w: u64 = written.parse().map_err(|_| format!("bad line {l:?}"))?;
                    if w >= m || s.written.is_some_and(|before| w < before) {
                        return Err(format!("{name}: W {w} at tic {m} after {:?}", s.written));
                    }
                    s.written = Some(w);
                }
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
    // whether a later session's claim completed (then the earlier sessions' writes had)
    let completed: Vec<bool> = (0..sessions.len())
        .map(|i| sessions[i + 1..].iter().any(|(_, s)| s.written.is_some()))
        .collect();
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
        let Some(i) = sessions.iter().position(|(n, _)| n == name) else {
            unlogged += 1;
            if tics != 0 || !claim_pending {
                return Err(format!(
                    "{name}: {tics} tics, but its claim was never printed (claim pending \
                     {claim_pending})"
                ));
            }
            continue;
        };
        let s = &sessions[i].1;
        let most = s.ended.unwrap_or(s.last);
        let least = if completed[i] {
            most
        } else {
            s.written.unwrap_or(0)
        };
        if tics < least {
            return Err(format!(
                "{name}: {tics} tics, fewer than the {least} the child reported written (the W \
                 rule; allowed {least} to {most}; last printed {}, W {:?}, ended {:?}, a later \
                 claim completed {})",
                s.last, s.written, s.ended, completed[i]
            ));
        }
        if tics > most {
            return Err(format!(
                "{name}: {tics} tics, more than the {most} the child ran (allowed {least} to \
                 {most}; last printed {}, W {:?}, ended {:?}, a later claim completed {})",
                s.last, s.written, s.ended, completed[i]
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
    for (i, (name, s)) in sessions.iter().enumerate() {
        if !lmps.contains_key(name) && (s.written.is_some() || completed[i]) {
            return Err(format!(
                "{name} is gone, but its claim had completed (W {:?})",
                s.written
            ));
        }
    }
    if let Some((name, s)) = sessions.last() {
        if s.ended.is_none() {
            let held = lmps
                .get(name)
                .map_or(0, |b| (b.len() as u64).saturating_sub(14) / 4);
            let lost = s.last.saturating_sub(held);
            *stats.lost.entry(lost).or_default() += 1;
            if let Some(bound) = max_lost.filter(|&b| s.written.is_some() && lost > b) {
                return Err(format!(
                    "{name}: {lost} tics lost at the kill (last printed tic {}, {held} in its \
                     file), more than the lag bound of {bound} (DOOM_PLAY_KILL_MAX_LOST sets it \
                     or lifts it with none)",
                    s.last
                ));
            }
        }
    }
    Ok(())
}

/// What one kill returns: the child's lines and its standard error.
struct Killed {
    lines: Vec<String>,
    stderr: String,
}

/// The last part of a child's standard error, for a failure message.
fn stderr_tail(stderr: &str) -> String {
    const KEEP: usize = 4000;
    let mut start = stderr.len().saturating_sub(KEEP);
    while !stderr.is_char_boundary(start) {
        start += 1;
    }
    if stderr[start..].trim().is_empty() {
        "the child's standard error was empty".into()
    } else {
        format!("the child's standard error ends:\n{}", &stderr[start..])
    }
}

/// One kill: spawns the child (with `plant` in its environment, or none), waits for `@start`
/// (and, with `at_tic`, then for the first `@tic` line of that count), sleeps `delay`, kills it,
/// requires that it died by that `SIGKILL` and returns its lines and its standard error.
fn kill_one(
    exe: &Path,
    dir: &Path,
    seed: u64,
    plant: Option<&str>,
    at_tic: Option<u64>,
    delay: Duration,
) -> Result<Killed, String> {
    let mut command = Command::new(exe);
    command
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
        .stderr(Stdio::piped());
    match plant {
        Some(p) => command.env(PLANT, p),
        None => command.env_remove(PLANT),
    };
    let mut child = command
        .spawn()
        .map_err(|e| format!("spawn the child: {e}"))?;
    let stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    // a thread drains each pipe, so the child never blocks on a full pipe while it plays
    let (tx, rx) = mpsc::channel::<String>();
    let reader = std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let errors = std::thread::spawn(move || {
        let mut text = Vec::new();
        let _ = stderr.read_to_end(&mut text);
        String::from_utf8_lossy(&text).into_owned()
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
    let stderr = errors
        .join()
        .map_err(|_| "the standard error reader panicked")?;
    lines.extend(rx.try_iter());
    if !started {
        let what = match (seen_start, at_tic) {
            (true, Some(n)) => format!("the child never printed tic {n} within 120 s"),
            _ => "the child never started".into(),
        };
        return Err(format!("{what} ({status}); {}", stderr_tail(&stderr)));
    }
    if status.signal() != Some(SIGKILL) {
        return Err(format!(
            "the child ended by itself ({status}), not by the SIGKILL; {}",
            stderr_tail(&stderr)
        ));
    }
    Ok(Killed { lines, stderr })
}

/// One run of kills (the parent's work, shared by the test and its self-test).
struct Kills {
    kills: u64,
    max_ms: u64,
    long_ms: u64,
    at_tic: Option<u64>,
    at_us: u64,
    seed0: u64,
    /// Keep each kill's files under this directory (else they go under `base` and are deleted).
    keep: Option<PathBuf>,
    /// Never delete anything (`DOOM_PLAY_KILL_DIR`): each kill's directory must not exist yet.
    never_delete: bool,
    base: PathBuf,
    plant: Option<String>,
    /// The lag bound in tics, `None` when lifted.
    max_lost: Option<u64>,
}

/// How a run of kills failed.
#[derive(Debug)]
enum Failure {
    /// A kill could not be made as asked (the child did not start, did not reach the aimed tic,
    /// or ended other than by the `SIGKILL`).
    Harness(String),
    /// A kill's files failed the check.
    Check(String),
}

/// Kills the child at random points and checks what it left (see the module documentation).
/// The check of one kill's files runs on a second thread while the next child plays; the run
/// stops at the first failure (whose files are kept).
fn run_kills(k: &Kills, data: &GameData) -> Result<Stats, Failure> {
    let exe = std::env::current_exe().expect("the test executable");
    let mut rng = Rng(k.seed0 ^ 0x5eed);
    let base = k.keep.clone().unwrap_or_else(|| k.base.clone());
    let (jobs, queue) = mpsc::channel::<(u64, PathBuf, Killed)>();
    let result = std::thread::scope(|scope| {
        let checker = scope.spawn(|| -> Result<Stats, Failure> {
            let mut stats = Stats::default();
            for (seed, dir, killed) in queue {
                stats.kills += 1;
                check(&dir, &killed.lines, data, k.max_lost, &mut stats).map_err(|e| {
                    Failure::Check(format!(
                        "seed {seed} (files kept in {}): {e}; {}",
                        dir.display(),
                        stderr_tail(&killed.stderr)
                    ))
                })?;
                if k.keep.is_none() {
                    std::fs::remove_dir_all(&dir)
                        .map_err(|e| Failure::Harness(format!("{}: {e}", dir.display())))?;
                }
            }
            Ok(stats)
        });
        let mut made = Ok(());
        for n in 0..k.kills {
            let seed = k.seed0 + n;
            let dir = base.join(format!("k{n:04}"));
            if k.never_delete {
                if dir.exists() {
                    made = Err(Failure::Harness(format!(
                        "{} exists: DOOM_PLAY_KILL_DIR needs a directory with no kNNNN in it (the \
                         test deletes nothing there)",
                        dir.display()
                    )));
                    break;
                }
            } else {
                let _ = std::fs::remove_dir_all(&dir);
            }
            std::fs::create_dir_all(&dir).expect("the kill's directory");
            // a tenth within 0.3 ms and a tenth within 3 ms of `@start` (aimed at the first
            // claim), a tenth late (past the first page-boundary rewrite in an optimised build),
            // the rest anywhere up to `max_ms`
            let delay = match rng.below(10) {
                _ if k.at_tic.is_some() => Duration::from_micros(rng.below(k.at_us)),
                0 => Duration::from_micros(rng.below(300)),
                1 => Duration::from_micros(rng.below(3_000)),
                2 => Duration::from_millis(1_000 + rng.below(k.long_ms)),
                _ => Duration::from_micros(rng.below(k.max_ms * 1_000)),
            };
            match kill_one(&exe, &dir, seed, k.plant.as_deref(), k.at_tic, delay) {
                Ok(killed) => {
                    if jobs.send((seed, dir, killed)).is_err() {
                        break; // the checker stopped at a failure
                    }
                }
                Err(e) => {
                    made = Err(Failure::Harness(format!("kill {n} (seed {seed}): {e}")));
                    break;
                }
            }
        }
        drop(jobs);
        let checked = checker.join().expect("the checker");
        // a check failure is reported before a harness failure that followed it
        let stats = checked?;
        made.map(|()| stats)
    });
    let stats = result?;
    if k.keep.is_none() {
        let _ = std::fs::remove_dir(&base);
    }
    Ok(stats)
}

/// The parent: kills the child at random points and checks what it left (see the module
/// documentation), configured from the environment.
#[test]
fn sigkill_at_random_points_leaves_valid_demos() {
    let fixed = std::env::var_os("DOOM_PLAY_KILL_DIR").map(PathBuf::from);
    let k = Kills {
        kills: env_u64("DOOM_PLAY_KILLS", 50),
        max_ms: env_u64("DOOM_PLAY_KILL_MAX_MS", 400).max(1),
        long_ms: env_u64("DOOM_PLAY_KILL_LONG_MS", 1600).max(1),
        at_tic: std::env::var("DOOM_PLAY_KILL_AT_TIC")
            .ok()
            .and_then(|v| v.parse::<u64>().ok()),
        at_us: env_u64("DOOM_PLAY_KILL_AT_US", 3_000).max(1),
        seed0: env_u64(SEED, 1),
        keep: fixed
            .clone()
            .or_else(|| std::env::var_os("DOOM_PLAY_KILL_KEEP").map(PathBuf::from)),
        never_delete: fixed.is_some(),
        base: std::env::temp_dir().join(format!("doom-play-kill-{}", std::process::id())),
        plant: std::env::var(PLANT).ok(),
        max_lost: max_lost(),
    };
    let bound = match (k.max_lost, std::env::var_os(MAX_LOST_VAR).is_some()) {
        (None, _) => "lifted (DOOM_PLAY_KILL_MAX_LOST=none)".to_string(),
        (Some(b), true) => format!("{b} tics (DOOM_PLAY_KILL_MAX_LOST)"),
        (Some(b), false) => format!("{b} tics (the default)"),
    };
    let data = load();
    let clock = Instant::now();
    let stats = run_kills(&k, &data).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(stats.kills, k.kills);
    println!(
        "kill test: {} kills (seeds {}..{}), {} files checked holding {} tics (longest {}, \
         {} past the first page-boundary rewrite), landed {:?}, tics lost by the session running \
         at the kill (M minus the tics in its file: kills) {:?}, lag bound {bound}, kills that \
         left no .lmp {}, leftover .tmp {}, aimed at {}, {:.1} s",
        stats.kills,
        k.seed0,
        k.seed0 + k.kills,
        stats.files,
        stats.tics,
        stats.longest,
        stats.past_page_rewrite,
        stats.landed,
        stats.lost,
        stats.no_file,
        stats.tmp,
        k.at_tic.map_or_else(
            || "no tic (random delays)".to_string(),
            |t| format!("tic {t} (0 to {} us after its print)", k.at_us)
        ),
        clock.elapsed().as_secs_f64()
    );
}

/// The kill test's self-test (review MAJ-1; AGENTS.md: "self-tests must prove the harness can
/// fail"): with the `torn` plant, the child appends each tic as two writes [`TORN_GAP`] apart,
/// and kills aimed just after the print of tic 30 must leave a file that the check refuses as
/// not a valid demo. The run stops at the first such failure; up to 40 kills are made.
#[test]
fn a_torn_append_fails_the_kill_test() {
    let base = std::env::temp_dir().join(format!("doom-play-kill-selftest-{}", std::process::id()));
    let k = Kills {
        kills: 40,
        max_ms: 1,
        long_ms: 1,
        at_tic: Some(30),
        at_us: 3_000,
        seed0: 1,
        keep: None,
        never_delete: false,
        base: base.clone(),
        plant: Some("torn".into()),
        max_lost: Some(MAX_LOST),
    };
    let data = load();
    match run_kills(&k, &data) {
        Err(Failure::Check(e)) => {
            assert!(
                e.contains("is not a valid demo"),
                "the torn file was refused for another reason: {e}"
            );
            println!("kill test self-test: the torn append was caught: {e}");
            std::fs::remove_dir_all(&base).expect("the self-test's files");
        }
        Err(Failure::Harness(e)) => panic!("the self-test could not run its kills: {e}"),
        Ok(stats) => panic!(
            "the torn append was not caught in {} kills ({} files checked, landed {:?})",
            stats.kills, stats.files, stats.landed
        ),
    }
}

/// The second self-test (review 3, m1): with the `early` plant every file operation is
/// [`EARLY_DELAY`] slow, so the writer lags by a few tics, and the child prints W [`EARLY_BY`]
/// tics too high (capped at M − 1). Kills aimed just after the print of tic 300 must find a file
/// holding fewer tics than the W printed for it. Up to 40 kills; the run stops at the first
/// failure, which must be the W rule's.
#[test]
fn a_w_printed_too_high_fails_the_kill_test() {
    let base = std::env::temp_dir().join(format!("doom-play-kill-early-{}", std::process::id()));
    let k = Kills {
        kills: 40,
        max_ms: 1,
        long_ms: 1,
        at_tic: Some(300),
        at_us: 500,
        seed0: 1,
        keep: None,
        never_delete: false,
        base: base.clone(),
        plant: Some("early".into()),
        max_lost: Some(MAX_LOST),
    };
    let data = load();
    match run_kills(&k, &data) {
        Err(Failure::Check(e)) => {
            assert!(
                e.contains("the child reported written (the W rule"),
                "the early W was refused for another reason: {e}"
            );
            println!("kill test self-test: the early W was caught: {e}");
            std::fs::remove_dir_all(&base).expect("the self-test's files");
        }
        Err(Failure::Harness(e)) => panic!("the self-test could not run its kills: {e}"),
        Ok(stats) => panic!(
            "the early W was not caught in {} kills ({} files checked, lost {:?})",
            stats.kills, stats.files, stats.lost
        ),
    }
}

/// The third self-test (review 3, m1): with the `stall` plant the writer stops for good before
/// the file would hold more than [`STALL_PAST`] tics while the child plays on; kills aimed just
/// after the print of tic 1,000 must exceed the lag bound ([`MAX_LOST`]). Up to 10 kills (a kill
/// can land in a session begun after the stall, whose claim never completes and which the bound
/// does not cover); the run stops at the first failure, which must be the lag bound's.
#[test]
fn a_stopped_writer_fails_the_lag_bound() {
    let base = std::env::temp_dir().join(format!("doom-play-kill-stall-{}", std::process::id()));
    let k = Kills {
        kills: 10,
        max_ms: 1,
        long_ms: 1,
        at_tic: Some(1_000),
        at_us: 500,
        seed0: 1,
        keep: None,
        never_delete: false,
        base: base.clone(),
        plant: Some("stall".into()),
        max_lost: Some(MAX_LOST),
    };
    let data = load();
    match run_kills(&k, &data) {
        Err(Failure::Check(e)) => {
            assert!(
                e.contains("more than the lag bound of 525"),
                "the stopped writer was refused for another reason: {e}"
            );
            println!("kill test self-test: the stopped writer was caught: {e}");
            std::fs::remove_dir_all(&base).expect("the self-test's files");
        }
        Err(Failure::Harness(e)) => panic!("the self-test could not run its kills: {e}"),
        Ok(stats) => panic!(
            "the stopped writer was not caught in {} kills ({} files checked, lost {:?})",
            stats.kills, stats.files, stats.lost
        ),
    }
}
