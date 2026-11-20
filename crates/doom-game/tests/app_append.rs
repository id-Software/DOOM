//! M5d: the session file kept current tic by tic. A host that opts in ([`Host::appends`]) is
//! offered every committed, recorded tic as one five-byte write at the old marker's offset (the
//! tic's four demo bytes and a new `DEMOMARKER`, g_game.c:1488, which `G_ReadDemoTiccmd` tests at
//! the start of each tic, g_game.c:1493); the app falls back to a whole rewrite when the host
//! declines or an earlier write failed, keeps the whole rewrite every `SAVE_INTERVAL` tics, and
//! never calls `append` on a host that did not opt in (which keeps M5b's cadence exactly).
//! Headless, on the M5a AMP room with `m5b_gfx.wad` and a fake clock; Rust-only checks of the app's
//! write schedule (the file's validity at every instant on a real file system is the kill test in
//! doom-play's `kill_tests`).

use doom_game::app::{
    App, DEMO_HEADER_BYTES, DemoCommands, GameData, Host, SAVE_INTERVAL, SaveStatus, SessionConfig,
    Timestamp, replay,
};
use doom_game::input::keys;
use doom_render::GameMode;
use std::collections::{BTreeMap, BTreeSet};
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

/// One call the app made, with the app's recorded tic count at the call.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Call {
    Claim { bytes: usize },
    Save { bytes: usize },
    Append { at: u64 },
}

/// An in-memory sessions directory whose `append` writes at the offset as a file would, checking
/// the app's promise that the file holds exactly `at + 1` bytes ending in the marker.
#[derive(Default)]
struct AppendHost {
    opts_in: bool,
    files: BTreeMap<String, Vec<u8>>,
    calls: Vec<Call>,
    /// Claims that fail before one succeeds.
    failing_claims: usize,
    /// Byte offsets at which `append` declines.
    decline: BTreeSet<u64>,
    /// Byte offsets at which `append` fails.
    fail: BTreeSet<u64>,
}

impl Host for AppendHost {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, name: &str) -> bool {
        self.files.contains_key(name)
    }
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        self.calls.push(Call::Claim { bytes: bytes.len() });
        if self.failing_claims > 0 {
            self.failing_claims -= 1;
            return Err("read-only directory".into());
        }
        if self.files.contains_key(name) {
            return Ok(false);
        }
        self.files.insert(name.into(), bytes.to_vec());
        Ok(true)
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.calls.push(Call::Save { bytes: bytes.len() });
        assert!(self.files.contains_key(name), "saved before claimed");
        self.files.insert(name.into(), bytes.to_vec());
        Ok(())
    }
    fn appends(&self) -> bool {
        self.opts_in
    }
    fn append(&mut self, name: &str, at: u64, tail: &[u8; 5]) -> Result<bool, String> {
        assert!(self.opts_in, "append called on a host that did not opt in");
        self.calls.push(Call::Append { at });
        if self.decline.contains(&at) {
            return Ok(false);
        }
        if self.fail.contains(&at) {
            return Err("disk full".into());
        }
        let file = self.files.get_mut(name).expect("append to a claimed file");
        let at = usize::try_from(at).unwrap();
        assert_eq!(
            file.len(),
            at + 1,
            "the file holds exactly the earlier tics"
        );
        assert_eq!(file[at], 0x80, "the old marker is at the offset");
        assert_eq!(tail[4], 0x80, "the tail ends with the marker");
        assert_ne!(tail[0], 0x80, "a tic's forward byte is never the marker");
        file.truncate(at);
        file.extend_from_slice(tail);
        Ok(true)
    }
}

/// The byte offset of the marker of a file holding `tics` tics.
fn marker(tics: usize) -> u64 {
    (DEMO_HEADER_BYTES + 4 * tics) as u64
}

/// Runs `tics` tics with a little input (forward held, turning, firing now and then) and returns
/// the app; after every tic, `check` sees the app and the host.
fn run(
    host: &mut AppendHost,
    cfg: SessionConfig,
    tics: u64,
    mut check: impl FnMut(&App, &AppendHost),
) -> App {
    let mut app = App::new(load(), cfg, host);
    check(&app, host);
    assert_eq!(app.update(0, host).ran, 0);
    for k in 1..=tics {
        match k % 70 {
            1 => app.key(keys::KEY_UPARROW, true, host),
            20 => app.key(keys::KEY_RIGHTARROW, true, host),
            30 => app.key(keys::KEY_RIGHTARROW, false, host),
            40 => app.key(keys::KEY_RCTRL, true, host),
            45 => app.key(keys::KEY_RCTRL, false, host),
            _ => {}
        }
        assert_eq!(app.update(at(k), host).ran, 1, "tic {k}");
        check(&app, host);
    }
    app
}

/// The opted-in host: a failed claim when the session begins is retried by the first tic's whole
/// write; then every tic is one append at the old marker's offset, except at `SAVE_INTERVAL`
/// (a whole rewrite instead), where the host declines (the append offered, then a whole
/// rewrite) and after a failed append (the failure visible, then a whole rewrite at the next tic,
/// with no append offered). After every successful write the file is exactly the recorder's
/// demo; restart and quit write nothing more, because the file is current.
#[test]
fn an_appending_host_gets_every_tic_as_one_write_at_the_old_marker() {
    let tics = SAVE_INTERVAL as u64 + 12;
    let mut host = AppendHost {
        opts_in: true,
        failing_claims: 1,
        decline: [marker(9), marker(500)].into(),
        fail: [marker(20), marker(21), marker(700)].into(),
        ..AppendHost::default()
    };
    let mut expected: Vec<Call> = vec![Call::Claim { bytes: 14 }];
    let mut behind = true; // the claim fails
    for n in 1..=tics as usize {
        let at = marker(n - 1);
        let whole = Call::Save {
            bytes: DEMO_HEADER_BYTES + 4 * n + 1,
        };
        if n % SAVE_INTERVAL == 0 {
            expected.push(whole);
            behind = false;
        } else if behind && n == 1 {
            expected.push(Call::Claim { bytes: 18 });
            behind = false;
        } else if behind {
            expected.push(whole);
            behind = false;
        } else if host.decline.contains(&at) {
            expected.extend([Call::Append { at }, whole]);
        } else {
            expected.push(Call::Append { at });
            behind = host.fail.contains(&at);
        }
    }
    let mut statuses = Vec::new();
    let mut app = run(&mut host, SessionConfig::default(), tics, |app, host| {
        let n = app.recorder().len();
        statuses.push(app.save_status().clone());
        if let SaveStatus::Saved(k) = app.save_status() {
            assert_eq!(*k, n, "a successful write leaves the file current");
            let file = &host.files[app.session_name()];
            assert_eq!(*file, app.recorder().bytes().unwrap(), "tic {n}");
            DemoCommands::parse(file, 1).expect("every saved state is a valid demo");
        }
    });
    assert_eq!(
        statuses[0],
        SaveStatus::Failed("read-only directory".into())
    );
    for n in [20usize, 21, 22, 700, 701] {
        // after tic n (statuses[n]): the appends of tics 21 and 22 (offsets of 20 and 21 tics)
        // and of tic 701 fail; tic 22's write follows a failure, so it is a whole rewrite
        let want_failed = [21, 701].contains(&n);
        let failed = matches!(statuses[n], SaveStatus::Failed(ref e) if e == "disk full");
        assert_eq!(failed, want_failed, "tic {n}: {:?}", statuses[n]);
    }
    assert_eq!(host.calls, expected);
    let before = host.calls.len();
    app.restart(&mut host);
    // only the new session's claim: the base name (the same timestamp) is taken, then `-2`
    assert_eq!(
        host.calls[before..],
        [Call::Claim { bytes: 14 }, Call::Claim { bytes: 14 }]
    );
    let before = host.calls.len();
    let first = host
        .files
        .values()
        .find(|f| f.len() > 14)
        .expect("the session")
        .clone();
    let commands = DemoCommands::parse(&first, 1).expect("parsed");
    assert_eq!(commands.commands().len(), tics as usize);
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
    assert_eq!(replay(&load(), 10, &commands, &mut Quiet), None);
    app.quit(&mut host);
    assert_eq!(
        host.calls.len(),
        before,
        "quit writes nothing: the file is current"
    );
}

/// A host that does not opt in keeps M5b's cadence exactly: the claim when the session begins,
/// a whole rewrite every `SAVE_INTERVAL` tics and on quit when behind, and never an append.
#[test]
fn a_host_that_does_not_opt_in_keeps_the_periodic_whole_rewrite() {
    let tics = SAVE_INTERVAL as u64 + 3;
    let mut host = AppendHost::default();
    let mut app = run(&mut host, SessionConfig::default(), tics, |_, _| {});
    let interval = DEMO_HEADER_BYTES + 4 * SAVE_INTERVAL + 1;
    assert_eq!(
        host.calls,
        [Call::Claim { bytes: 14 }, Call::Save { bytes: interval }]
    );
    app.quit(&mut host);
    let end = DEMO_HEADER_BYTES + 4 * tics as usize + 1;
    assert_eq!(host.calls.last(), Some(&Call::Save { bytes: end }));
    assert_eq!(host.calls.len(), 3);
}

/// An unrecorded session (decision S3 (ii)) writes nothing, even to a host that appends.
#[test]
fn an_unrecorded_session_offers_nothing_to_an_appending_host() {
    let mut host = AppendHost {
        opts_in: true,
        ..AppendHost::default()
    };
    let cfg = SessionConfig {
        unrecorded_full_turning: true,
        ..SessionConfig::default()
    };
    let mut app = run(&mut host, cfg, 40, |_, _| {});
    app.quit(&mut host);
    assert!(host.calls.is_empty(), "{:?}", host.calls);
    assert!(host.files.is_empty());
}
