//! M5d mutation survivors of `App::append`, `Host::append`'s default and `App::begin`'s reset of
//! the failed-write retry counter (the second review's note N-A). Rust-only checks of the app's
//! write schedule through the public `Host` trait, headless on the M5a AMP room with
//! `m5b_gfx.wad` and a fake clock. Each test names the mutant it pins.
//!
//! The expectations come from the documented `Host` contract (`Host::append`: the app calls it
//! "only while the file holds exactly the demo of every earlier tic", otherwise it rewrites the
//! file whole; the default declines) and from `FAILED_WRITE_RETRY_TICS`; no new C citation is
//! needed (the marker is `DEMOMARKER`, g_game.c:1488, as in `app_append.rs`).

use doom_game::app::{
    App, DEMO_HEADER_BYTES, FAILED_WRITE_RETRY_TICS, GameData, Host, SaveStatus, SessionConfig,
    Timestamp,
};
use doom_render::GameMode;
use std::collections::BTreeMap;
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

fn marker(tics: usize) -> u64 {
    (DEMO_HEADER_BYTES + 4 * tics) as u64
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Call {
    Claim,
    Save,
    Append { at: u64 },
}

/// An in-memory directory whose `appends` answer the test can change between tics, and whose
/// `append` panics unless the file holds exactly `at + 1` bytes (the contract of
/// `Host::append`).
#[derive(Default)]
struct SwitchHost {
    opts_in: bool,
    files: BTreeMap<String, Vec<u8>>,
    calls: Vec<Call>,
}

impl Host for SwitchHost {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, name: &str) -> bool {
        self.files.contains_key(name)
    }
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        self.calls.push(Call::Claim);
        if self.files.contains_key(name) {
            return Ok(false);
        }
        self.files.insert(name.into(), bytes.to_vec());
        Ok(true)
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.calls.push(Call::Save);
        self.files.insert(name.into(), bytes.to_vec());
        Ok(())
    }
    fn appends(&self) -> bool {
        self.opts_in
    }
    fn append(&mut self, name: &str, at: u64, tail: &[u8; 5]) -> Result<bool, String> {
        self.calls.push(Call::Append { at });
        let file = self.files.get_mut(name).expect("append to a claimed file");
        let at = usize::try_from(at).unwrap();
        assert_eq!(
            file.len(),
            at + 1,
            "append offered while the file does not hold exactly the earlier tics"
        );
        file.truncate(at);
        file.extend_from_slice(tail);
        Ok(true)
    }
}

/// Pins `App::append`'s `current` condition (`claimed && n > 0 && save == Saved(n - 1)`): when
/// the host first opts in after tic 5, the file is behind (the status is `Saved(5)` and the
/// recorder holds 6 tics), so the app must rewrite the file whole at tic 6 and only then
/// append. Kills `&&` to `||` (both) and the match guard `current` to `true`. The public API
/// reaches the stale state only through a host whose `appends` changes, which the `Host`
/// contract allows ("only when [`Host::appends`] says so" is asked at every tic).
#[test]
fn a_host_that_opts_in_late_gets_a_whole_rewrite_before_its_first_append() {
    let mut host = SwitchHost::default();
    let mut app = App::new(load(), SessionConfig::default(), &mut host);
    assert_eq!(app.update(0, &mut host).ran, 0);
    for k in 1..=10u64 {
        host.opts_in = k >= 6;
        assert_eq!(app.update(at(k), &mut host).ran, 1, "tic {k}");
        if k >= 6 {
            assert_eq!(
                host.files[app.session_name()],
                app.recorder().bytes().unwrap(),
                "tic {k}: the file is current"
            );
        }
    }
    assert_eq!(
        host.calls,
        [
            Call::Claim,
            Call::Save,
            Call::Append { at: marker(6) },
            Call::Append { at: marker(7) },
            Call::Append { at: marker(8) },
            Call::Append { at: marker(9) },
        ]
    );
}

/// A host that opts in and keeps `Host::append`'s default: the default declines, so the app
/// rewrites the file whole at every tic and the file is current after each. Kills
/// `Host::append` returning `Ok(true)` (the app would believe a write that never happened).
#[test]
fn the_default_append_declines_so_the_file_is_rewritten_whole() {
    struct DefaultAppend {
        files: BTreeMap<String, Vec<u8>>,
        saves: usize,
    }
    impl Host for DefaultAppend {
        fn timestamp(&mut self) -> Timestamp {
            Timestamp::default()
        }
        fn exists(&mut self, name: &str) -> bool {
            self.files.contains_key(name)
        }
        fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
            self.saves += 1;
            self.files.insert(name.into(), bytes.to_vec());
            Ok(())
        }
        fn appends(&self) -> bool {
            true
        }
    }
    let mut host = DefaultAppend {
        files: BTreeMap::new(),
        saves: 0,
    };
    let mut app = App::new(load(), SessionConfig::default(), &mut host);
    assert_eq!(app.update(0, &mut host).ran, 0);
    let after_claim = host.saves;
    for k in 1..=5u64 {
        assert_eq!(app.update(at(k), &mut host).ran, 1, "tic {k}");
        assert_eq!(
            host.files[app.session_name()],
            app.recorder().bytes().unwrap(),
            "tic {k}: the file is current"
        );
        assert_eq!(host.saves, after_claim + k as usize, "one rewrite per tic");
    }
}

/// A host whose writes fail on demand (`fail` for every call, `fail_next` for the next calls).
#[derive(Default)]
struct DemandHost {
    fail: bool,
    fail_next: usize,
    files: BTreeMap<String, Vec<u8>>,
    calls: Vec<Call>,
}

impl DemandHost {
    fn fails(&mut self, call: Call) -> bool {
        self.calls.push(call);
        if self.fail_next > 0 {
            self.fail_next -= 1;
            return true;
        }
        self.fail
    }
}

impl Host for DemandHost {
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
        self.files.insert(name.into(), bytes.to_vec());
        Ok(())
    }
    fn appends(&self) -> bool {
        true
    }
    fn append(&mut self, name: &str, at: u64, tail: &[u8; 5]) -> Result<bool, String> {
        if self.fails(Call::Append { at }) {
            return Err("disk full".into());
        }
        let file = self.files.get_mut(name).expect("append to a claimed file");
        let at = usize::try_from(at).unwrap();
        assert_eq!(file.len(), at + 1);
        file.truncate(at);
        file.extend_from_slice(tail);
        Ok(true)
    }
}

/// Review 2 note N-A: `App::begin` resets the failed-write retry counter. A session that ends
/// under a persistent failure holds `retry_from` at tic 71 (the claim fails at its beginning,
/// is retried at tics 1 and 36, so the next retry is due at 71). After a restart whose old
/// write and new claim both fail (a successful write would clear the counter by itself), the new
/// session's first tic must retry at once (the documented "next tic" retry; the new session's
/// recorder holds 1 tic), not wait for tic 71. Pins `self.retry_from = None` in `begin`.
#[test]
fn a_restart_resets_the_failed_write_retry_counter() {
    let mut host = DemandHost {
        fail: true,
        ..DemandHost::default()
    };
    let mut app = App::new(load(), SessionConfig::default(), &mut host);
    assert_eq!(app.update(0, &mut host).ran, 0);
    for k in 1..=40u64 {
        assert_eq!(app.update(at(k), &mut host).ran, 1, "tic {k}");
    }
    assert!(matches!(app.save_status(), SaveStatus::Failed(_)));
    const { assert!(FAILED_WRITE_RETRY_TICS > 2) };
    // restart: the old session's write and the new session's claim both fail
    host.calls.clear();
    app.restart(&mut host);
    assert_eq!(host.calls, [Call::Claim, Call::Claim]);
    assert!(matches!(app.save_status(), SaveStatus::Failed(_)));
    host.calls.clear();
    let base = at(100);
    assert_eq!(app.update(base, &mut host).ran, 0, "the clock restarted");
    assert_eq!(app.update(base + at(1), &mut host).ran, 1);
    assert_eq!(
        host.calls,
        [Call::Claim],
        "the new session's first tic retries at once, not at the old session's tic 71"
    );
}
