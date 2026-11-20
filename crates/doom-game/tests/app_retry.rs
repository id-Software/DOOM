//! M5d review MIN-1: the retry cadence after a failed write. A host that appends
//! ([`Host::appends`]) gets, after a failed write, one whole rewrite at the next recorded tic and,
//! while those retries keep failing, at most one every `FAILED_WRITE_RETRY_TICS` (35) recorded
//! tics; the whole rewrite every `SAVE_INTERVAL` tics, at the session's end and on restart and
//! quit is made as before, and appends resume only after a whole rewrite succeeded. A host that
//! does not opt in keeps M5b's cadence exactly, failures or not. Headless, on the M5a AMP room
//! with `m5b_gfx.wad` and a fake clock; Rust-only checks of the app's write schedule.

use doom_game::app::{
    App, FAILED_WRITE_RETRY_TICS, GameData, Host, SAVE_INTERVAL, SaveStatus, SessionConfig,
    Timestamp,
};
use doom_render::GameMode;
use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::Path;

fn load() -> GameData {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    let wad = std::fs::read(root.join("m5a_wads/amp.wad")).expect("map WAD");
    let gfx = std::fs::read(root.join("m5b_gfx.wad")).expect("gfx WAD");
    GameData::load(&[&wad, &gfx], GameMode::Registered, 1, 2).expect("game data")
}

fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Call {
    Claim,
    Save,
    Append,
}

/// An in-memory sessions directory in which every write fails while the test's tic is inside
/// one of the ranges of `broken` (tic 0 is the session's beginning, before the first tic).
struct FailingHost {
    opts_in: bool,
    broken: Vec<RangeInclusive<u64>>,
    /// The tic the test is running (set before each update).
    tic: u64,
    files: BTreeMap<String, Vec<u8>>,
    /// Every call, with the tic at which it was made.
    calls: Vec<(u64, Call)>,
}

impl FailingHost {
    /// `broken` lists the failing ranges as (first, last) tics.
    fn new(opts_in: bool, broken: &[(u64, u64)]) -> Self {
        Self {
            opts_in,
            broken: broken.iter().map(|&(a, b)| a..=b).collect(),
            tic: 0,
            files: BTreeMap::new(),
            calls: Vec::new(),
        }
    }
    fn fails(&mut self, call: Call) -> bool {
        self.calls.push((self.tic, call));
        self.broken.iter().any(|r| r.contains(&self.tic))
    }
}

impl Host for FailingHost {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, name: &str) -> bool {
        self.files.contains_key(name)
    }
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        if self.fails(Call::Claim) {
            return Err("read-only directory".into());
        }
        if self.files.contains_key(name) {
            return Ok(false);
        }
        self.files.insert(name.into(), bytes.to_vec());
        Ok(true)
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        if self.fails(Call::Save) {
            return Err("rename refused".into());
        }
        assert!(self.files.contains_key(name), "saved before claimed");
        self.files.insert(name.into(), bytes.to_vec());
        Ok(())
    }
    fn appends(&self) -> bool {
        self.opts_in
    }
    fn append(&mut self, name: &str, at: u64, tail: &[u8; 5]) -> Result<bool, String> {
        assert!(self.opts_in, "append called on a host that did not opt in");
        if self.fails(Call::Append) {
            return Err("disk full".into());
        }
        let file = self.files.get_mut(name).expect("append to a claimed file");
        let at = usize::try_from(at).unwrap();
        assert_eq!(
            file.len(),
            at + 1,
            "the file holds exactly the earlier tics"
        );
        file.truncate(at);
        file.extend_from_slice(tail);
        Ok(true)
    }
}

/// Runs `tics` tics with no input; after each, `check` sees the tic, the app and the host.
fn run(host: &mut FailingHost, tics: u64, mut check: impl FnMut(u64, &App, &FailingHost)) -> App {
    host.tic = 0;
    let mut app = App::new(load(), SessionConfig::default(), host);
    check(0, &app, host);
    assert_eq!(app.update(0, host).ran, 0);
    for k in 1..=tics {
        host.tic = k;
        assert_eq!(app.update(at(k), host).ran, 1, "tic {k}");
        check(k, &app, host);
    }
    app
}

/// The tics of `calls` that are not appends, with their kind.
fn writes(calls: &[(u64, Call)]) -> Vec<(u64, Call)> {
    calls
        .iter()
        .copied()
        .filter(|(_, c)| *c != Call::Append)
        .collect()
}

/// Every write fails from tic 10 to tic 200: tic 10's append fails, the whole rewrite is retried
/// at tic 11 and then every 35 tics while it fails (46, 81, 116, 151, 186), nothing is written
/// in between, the retry at tic 221 succeeds, and appends resume at tic 222. Quit while current
/// writes nothing.
#[test]
fn a_failing_write_is_retried_at_the_next_tic_then_once_a_second() {
    assert_eq!(FAILED_WRITE_RETRY_TICS, 35);
    let mut host = FailingHost::new(true, &[(10, 200)]);
    let mut app = run(&mut host, 260, |k, app, host| {
        let failed = matches!(app.save_status(), SaveStatus::Failed(_));
        assert_eq!(
            failed,
            (10..221).contains(&k),
            "tic {k}: {:?}",
            app.save_status()
        );
        if let SaveStatus::Saved(n) = app.save_status() {
            assert_eq!(*n, app.recorder().len(), "tic {k}");
            assert_eq!(
                host.files[app.session_name()],
                app.recorder().bytes().unwrap()
            );
        }
    });
    let retries = [11, 46, 81, 116, 151, 186, 221];
    let mut want = vec![(0, Call::Claim)];
    want.extend(retries.iter().map(|&k| (k, Call::Save)));
    assert_eq!(writes(&host.calls), want);
    let appends: Vec<u64> = host
        .calls
        .iter()
        .filter(|(_, c)| *c == Call::Append)
        .map(|(k, _)| *k)
        .collect();
    let want_appends: Vec<u64> = (1..=10).chain(222..=260).collect();
    assert_eq!(
        appends, want_appends,
        "appends stop at the failure and resume after a rewrite"
    );
    let before = host.calls.len();
    app.quit(&mut host);
    assert_eq!(
        host.calls.len(),
        before,
        "the file is current: quit writes nothing"
    );
}

/// A failed claim when the session begins, failing on: the claim is retried at tic 1, then at
/// tics 36 and 71; the `SAVE_INTERVAL` write is made as before; restart and quit each make
/// their write (failing) at once.
#[test]
fn a_failing_claim_is_retried_once_a_second_and_the_periodic_write_stays() {
    let tics = SAVE_INTERVAL as u64 + 40;
    let mut host = FailingHost::new(true, &[(0, u64::MAX)]);
    let mut app = run(&mut host, tics, |k, app, _| {
        assert!(
            matches!(app.save_status(), SaveStatus::Failed(_)),
            "tic {k}"
        );
    });
    let mut want = vec![(0, Call::Claim)];
    let mut next = 1;
    for k in 1..=tics {
        if k % SAVE_INTERVAL as u64 == 0 {
            want.push((k, Call::Claim));
        } else if k == next {
            want.push((k, Call::Claim));
            next = k + FAILED_WRITE_RETRY_TICS as u64;
        }
    }
    assert_eq!(
        &want[..4],
        [
            (0, Call::Claim),
            (1, Call::Claim),
            (36, Call::Claim),
            (71, Call::Claim)
        ]
    );
    assert!(want.contains(&(SAVE_INTERVAL as u64, Call::Claim)));
    assert_eq!(
        host.calls, want,
        "no append is ever offered: the file was never claimed"
    );
    let before = host.calls.len();
    app.restart(&mut host);
    // the old session's write on restart, then the new session's claim
    assert_eq!(
        host.calls[before..],
        [(tics, Call::Claim), (tics, Call::Claim)]
    );
    let before = host.calls.len();
    app.quit(&mut host);
    assert_eq!(
        host.calls[before..],
        [(tics, Call::Claim)],
        "quit writes when behind"
    );
}

/// A success of the periodic whole rewrite ends the failure's cadence too: writes fail from tic
/// 1030 to 1049 (the append of 1030 fails, the retry at 1031 fails, the next would be at 1066),
/// the `SAVE_INTERVAL` rewrite at 1050 succeeds and appends resume; the append of 1055 fails, and
/// its retry comes at once, at 1056 (not at 1066).
#[test]
fn a_periodic_rewrite_that_succeeds_ends_the_retry_cadence() {
    let interval = SAVE_INTERVAL as u64;
    assert_eq!(interval, 1050);
    let mut host = FailingHost::new(true, &[(1030, 1049), (1055, 1055)]);
    run(&mut host, 1070, |_, _, _| {});
    assert_eq!(
        writes(&host.calls),
        [
            (0, Call::Claim),
            (1031, Call::Save),
            (1050, Call::Save),
            (1056, Call::Save)
        ]
    );
    let appends: Vec<u64> = host
        .calls
        .iter()
        .filter(|(_, c)| *c == Call::Append)
        .map(|(k, _)| *k)
        .collect();
    let want: Vec<u64> = (1..=1030).chain(1051..=1055).chain(1057..=1070).collect();
    assert_eq!(appends, want);
}

/// A host that does not opt in keeps M5b's cadence exactly under a persistent failure: the
/// claim when the session begins, a whole write every `SAVE_INTERVAL` tics, no per-tic retry
/// and never an append.
#[test]
fn a_host_that_does_not_opt_in_keeps_the_m5b_cadence_when_writes_fail() {
    let tics = SAVE_INTERVAL as u64 + 40;
    let mut host = FailingHost::new(false, &[(0, u64::MAX)]);
    let mut app = run(&mut host, tics, |_, _, _| {});
    assert_eq!(
        host.calls,
        [(0, Call::Claim), (SAVE_INTERVAL as u64, Call::Claim)]
    );
    app.quit(&mut host);
    assert_eq!(host.calls.last(), Some(&(tics, Call::Claim)));
    assert_eq!(host.calls.len(), 3);
}
