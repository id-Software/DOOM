//! The DSIM39/DSIM40 readers' refusal domains for what M4b added or widened,
//! through the public API (M4b step 11, informational Mutants run 43543402068: the
//! survivors in `diagnostic.rs`).
//!
//! Each test takes a committed independent capture, changes one field (or pushes
//! one event payload onto `events.last`), re-encodes it with the canonical framing
//! and asserts the reader's error class (or acceptance) for it. The base captures
//! are the fixtures `tests/reference/m4a_episode1_*.bin` (DSIM39) and
//! `m4b_episode1v2_*.bin` (DSIM40), projected from the genuine engine's dumps by
//! the independent encoders, never from Rust output. The event layouts and the
//! accept/refuse decisions come from docs/STATE_FORMAT.md ("DSIM v40": "Events" and
//! "What the v40 reader checks", and the sections it inherits); each test names the
//! rule it pins. `DiagnosticError` carries only the class, so every case changes
//! exactly one thing of an otherwise accepted capture.
use doom_sim::identity::{OriginalActorType, OriginalState};
use doom_sim::*;
use std::path::{Path, PathBuf};

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}
/// A DSIM capture as (path, value) pairs, re-encoded with the canonical framing
/// (`DSIM`, u32 version, u64 count, then per field u32 path length, path, u64 value
/// length, value: STATE_FORMAT "Bounded encoding and reading").
#[derive(Clone)]
struct Capture {
    version: u32,
    fields: Vec<(String, Vec<u8>)>,
}
impl Capture {
    fn fixture(name: &str) -> Self {
        let bytes = std::fs::read(reference(name)).expect("fixture");
        assert_eq!(&bytes[..4], b"DSIM");
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let count = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let (mut at, mut fields) = (16, Vec::new());
        for _ in 0..count {
            let len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
            let path = String::from_utf8(bytes[at + 4..at + 4 + len].to_vec()).unwrap();
            at += 4 + len;
            let len = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
            fields.push((path, bytes[at + 8..at + 8 + len].to_vec()));
            at += 8 + len;
        }
        assert_eq!(at, bytes.len());
        let c = Self { version, fields };
        assert_eq!(c.decode(), Ok(()), "{name}");
        c
    }
    fn encode(&self) -> Vec<u8> {
        let mut out = b"DSIM".to_vec();
        out.extend(self.version.to_le_bytes());
        out.extend((self.fields.len() as u64).to_le_bytes());
        for (path, value) in &self.fields {
            out.extend((path.len() as u32).to_le_bytes());
            out.extend(path.as_bytes());
            out.extend((value.len() as u64).to_le_bytes());
            out.extend(value);
        }
        out
    }
    fn decode(&self) -> Result<(), DiagnosticError> {
        let bytes = self.encode();
        let limits = DiagnosticLimits::default();
        match self.version {
            39 => Diagnostic::decode_episode1(&bytes, limits).map(|_| ()),
            40 => Diagnostic::decode_episode1_v2(&bytes, limits).map(|_| ()),
            v => panic!("version {v}"),
        }
    }
    fn position(&self, path: &str) -> usize {
        self.fields
            .iter()
            .position(|(p, _)| p == path)
            .unwrap_or_else(|| panic!("missing field {path}"))
    }
    fn get(&self, path: &str) -> &[u8] {
        &self.fields[self.position(path)].1
    }
    fn u64(&self, path: &str) -> u64 {
        u64::from_le_bytes(self.get(path).try_into().unwrap())
    }
    fn set(&mut self, path: &str, value: impl Into<Vec<u8>>) -> &mut Self {
        let at = self.position(path);
        self.fields[at].1 = value.into();
        self
    }
    /// Append `event` to `events.last` (count and the next index).
    fn push_event(&mut self, event: &[u8]) -> &mut Self {
        let n = self.u64("events.last.count");
        let after = if n == 0 {
            "events.last.count".to_string()
        } else {
            format!("events.last.{}", n - 1)
        };
        self.set("events.last.count", (n + 1).to_le_bytes());
        let at = self.position(&after) + 1;
        self.fields
            .insert(at, (format!("events.last.{n}"), event.to_vec()));
        self
    }
    /// The decision on this capture with `event` appended to `events.last`.
    fn with_pushed(&self, event: &[u8]) -> Result<(), DiagnosticError> {
        let mut c = self.clone();
        c.push_event(event);
        c.decode()
    }
    /// The decision on this capture with field `path` replaced by `value`.
    fn with(&self, path: &str, value: impl Into<Vec<u8>>) -> Result<(), DiagnosticError> {
        let mut c = self.clone();
        c.set(path, value);
        c.decode()
    }
    /// The `i`-th 12-byte actor identity of `thinker.order`.
    fn handle(&self, i: usize) -> [u8; 12] {
        self.get("thinker.order")[12 * i..12 * i + 12]
            .try_into()
            .unwrap()
    }
    fn lines(&self) -> usize {
        self.get("map.linedefs").len() / 14
    }
    /// Occupied actor slots of type `ty`.
    fn actors(&self, ty: OriginalActorType) -> Vec<u32> {
        (0..self.u64("store.slot_count") as u32)
            .filter(|i| {
                self.get(&format!("actor.{i}.occupied")) == [1]
                    && self.get(&format!("actor.{i}.type")) == ty.ordinal().to_le_bytes()
            })
            .collect()
    }
}
/// The event `tag, sub` followed by `parts`.
fn event(tag: u8, sub: u8, parts: &[&[u8]]) -> Vec<u8> {
    let mut e = vec![tag, sub];
    for p in parts {
        e.extend_from_slice(p);
    }
    e
}
const V39: &str = "m4a_episode1_SAW1_78.bin";
const V40: &str = "m4b_episode1v2_TELE1_89.bin";

/// DSIM40 added mover sub-tags 15 (teleported, 40 bytes), 16 (teleport not taken,
/// 17) and 17 (`A_BossDeath` ran, 15) to tag 26 (STATE_FORMAT "DSIM v40", "Events":
/// "admitted by the v40 reader only"). Each well-formed payload is accepted by the
/// v40 reader and refused by the v39 reader as `Structure`, an unknown sub-tag.
/// Survivors `diagnostic.rs:1986:19`, `1987:19` and `1988:19`.
#[test]
fn dsim40_mover_sub_tags_exist_in_dsim40_only() {
    for (name, expect) in [(V39, Err(DiagnosticError::Structure)), (V40, Ok(()))] {
        let c = Capture::fixture(name);
        let (a, b) = (c.handle(0), c.handle(1));
        let teleported = event(26, 15, &[&a, &0u16.to_le_bytes(), &b, &[0; 12]]);
        let not_taken = event(26, 16, &[&a, &0u16.to_le_bytes(), &[2]]);
        let boss_death = event(26, 17, &[&a, &[3]]);
        for (what, e) in [
            ("sub 15", teleported),
            ("sub 16", not_taken),
            ("sub 17", boss_death),
        ] {
            assert_eq!(c.with_pushed(&e), expect, "{name} {what}");
        }
    }
}

/// The sub-tag 6 record of `A_BossDeath`'s floor call (function 4 `EV_DoFloor`,
/// line 65535, argument 1) is admitted by the v40 reader only (STATE_FORMAT "DSIM
/// v40", sub-tag 17 row: "line 65535 (no line…), argument 1 (`lowerFloorToLowest`)").
/// Any other sub-tag 6 record must name a real line. So: the exact record is
/// accepted under v40 and refused under v39 (`Identity`, an unknown line); under v40
/// a record with a line past the table, or with the right line but another function
/// or another argument, is refused. Survivors `diagnostic.rs:2048:21`, `2049:21`
/// and `2050:21`.
#[test]
fn the_boss_floor_call_record_is_one_exact_form_under_dsim40() {
    let call = |function: u8, line: u16, argument: i32| {
        event(
            26,
            6,
            &[
                &[function],
                &line.to_le_bytes(),
                &argument.to_le_bytes(),
                &1i32.to_le_bytes(),
                &[1],
            ],
        )
    };
    let v40 = Capture::fixture(V40);
    let v39 = Capture::fixture(V39);
    assert_eq!(v40.with_pushed(&call(4, u16::MAX, 1)), Ok(()));
    assert_eq!(
        v39.with_pushed(&call(4, u16::MAX, 1)),
        Err(DiagnosticError::Identity)
    );
    // A real line is accepted either way (control).
    assert_eq!(v40.with_pushed(&call(4, 0, 1)), Ok(()));
    assert_eq!(v39.with_pushed(&call(4, 0, 1)), Ok(()));
    let past = v40.lines() as u16;
    for (what, e) in [
        ("a line past the table", call(4, past, 1)),
        ("65535 with function 3", call(3, u16::MAX, 1)),
        ("65535 with argument 2", call(4, u16::MAX, 2)),
        ("a line past the table, argument 1", call(3, past, 1)),
    ] {
        assert_eq!(
            v40.with_pushed(&e),
            Err(DiagnosticError::Identity),
            "{what}"
        );
    }
}

/// A sky removal (tag 25 sub-tag 1) names an occupied fireball: the Imp's
/// `MT_TROOPSHOT`, from DSIM39 a rocket and from DSIM40 a Baron fireball
/// (`MT_BRUISERSHOT`), and nothing else (STATE_FORMAT "DSIM v34" tag 25, "DSIM v40":
/// `MT_BRUISERSHOT` "in the projectile launch record and the sky removal"). In
/// BARON1's capture after the launch, the Baron fireball is accepted, and the
/// Baron and the player body are refused (`Identity`). Survivors
/// `diagnostic.rs:2138:38`, `2138:44` and `2138:50`.
#[test]
fn a_sky_removal_names_a_fireball_only() {
    let c = Capture::fixture("m4b_episode1v2_BARON1_67.bin");
    let removal = |slot: u32| {
        let generation = c.get(&format!("actor.{slot}.generation")).to_vec();
        event(
            25,
            1,
            &[&slot.to_le_bytes(), &generation, &0u16.to_le_bytes()],
        )
    };
    let ball = c.actors(OriginalActorType::MT_BRUISERSHOT);
    let baron = c.actors(OriginalActorType::MT_BRUISER);
    let player = c.actors(OriginalActorType::MT_PLAYER);
    assert_eq!((ball.len(), baron.len(), player.len()), (1, 1, 1));
    assert_eq!(c.with_pushed(&removal(ball[0])), Ok(()));
    assert_eq!(
        c.with_pushed(&removal(baron[0])),
        Err(DiagnosticError::Identity)
    );
    assert_eq!(
        c.with_pushed(&removal(player[0])),
        Err(DiagnosticError::Identity)
    );
}

/// A monster's projectile launch record (tag 15 sub-tag 5 with aim kind 0) names the
/// Imp fireball `MT_TROOPSHOT`, and under DSIM40 also `MT_BRUISERSHOT` (STATE_FORMAT
/// "DSIM v40": "The Baron's attacks use the existing combat and projectile records
/// (tags 13 and 15) with `MT_BRUISERSHOT` (16) wherever the v39 forms name
/// `MT_TROOPSHOT` (31)"). No other type, and the Baron's under DSIM39 is not
/// admitted. Survivor `diagnostic.rs:2920:42`.
#[test]
fn a_launch_record_names_the_fireball_or_the_baron_ball() {
    let launch = |c: &Capture, ty: OriginalActorType| {
        event(
            15,
            5,
            &[
                &c.handle(0),
                &[0],
                &c.handle(1),
                &ty.ordinal().to_le_bytes(),
            ],
        )
    };
    let fireball = OriginalActorType::MT_TROOPSHOT;
    let baron_ball = OriginalActorType::MT_BRUISERSHOT;
    let rocket = OriginalActorType::MT_ROCKET;
    let v39 = Capture::fixture(V39);
    let v40 = Capture::fixture(V40);
    assert_eq!(v39.with_pushed(&launch(&v39, fireball)), Ok(()));
    assert_eq!(
        v39.with_pushed(&launch(&v39, baron_ball)),
        Err(DiagnosticError::Identity)
    );
    assert_eq!(
        v39.with_pushed(&launch(&v39, rocket)),
        Err(DiagnosticError::Identity)
    );
    assert_eq!(v40.with_pushed(&launch(&v40, fireball)), Ok(()));
    assert_eq!(v40.with_pushed(&launch(&v40, baron_ball)), Ok(()));
    assert_eq!(
        v40.with_pushed(&launch(&v40, rocket)),
        Err(DiagnosticError::Identity)
    );
}

/// A sector's `special` is its map special, or 0 once something finished with it
/// (STATE_FORMAT "DSIM v39": "a raiseToNearestAndChange plat and a finished donut
/// ring set the special to 0", kept by v40). Any other nonzero value is refused with
/// `Identity`, under both readers (the rule of `validate_movers`; the copy in
/// `validate_specials`, survivor `diagnostic.rs:1153:68`, runs for DSIM34 only and is
/// dead for these versions, so that mutant is equivalent and is not reachable by any
/// test).
#[test]
fn a_sector_special_is_its_map_value_or_zero() {
    let mut checked = 0;
    for name in [
        V39,
        V40,
        "m4a_episode1_SECT11_180.bin",
        "m4b_episode1v2_SLIDEC_74.bin",
    ] {
        let c = Capture::fixture(name);
        let map = c.get("map.sectors").to_vec();
        for i in 0..map.len() / 26 {
            let original = i16::from_le_bytes([map[26 * i + 22], map[26 * i + 23]]);
            if original == 0 {
                continue;
            }
            let path = format!("sector.{i}.special");
            let junk = (original + 100).to_le_bytes();
            assert_eq!(
                c.with(&path, junk),
                Err(DiagnosticError::Identity),
                "{name} {path}"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no sector with a map special in the fixtures");
}

/// A decoration (E1M1's seven, and from DSIM39 the candle and the firestick) holds
/// a state of its own spawn loop (`e1m1_rules::decoration_states`: the firestick
/// loops, the others hold their one state), with the sprite, frame and countdown of
/// that state (STATE_FORMAT "DSIM v36": "the decoration checks"; "DSIM v39": "the
/// firestick may hold any state of its spawn loop"). Moving the candle and then the
/// firestick through a sweep of the states (below), each with that state's sprite, frame and a
/// countdown in its range (so that no other check of the record fails), the reader
/// accepts exactly the spawn loop: the candle's one state and the firestick's four.
///
/// This pins the behaviour, and it is also why the survivor
/// `diagnostic.rs:1548:17` (`||` → `&&` in `validate_decorations`) is equivalent:
/// the state test there is the same predicate as the generic per-actor check that
/// runs for every occupied actor first (`Diagnostic::identity` →
/// `state_role_for` → `e1m1_rules::spawn_state` = `decoration_states`), so no state
/// outside the loop reaches it (the sweep shows the generic check alone refuses
/// each state it visits).
#[test]
fn a_decoration_holds_a_state_of_its_own_loop_only() {
    let c = Capture::fixture("m4a_episode1_SCALE2_0.bin");
    for (ty, loop_states) in [
        (OriginalActorType::MT_MISC49, vec![911]),
        (OriginalActorType::MT_MISC43, vec![934, 935, 936, 937]),
    ] {
        let actor = c.actors(ty)[0];
        let p = format!("actor.{actor}");
        // The loops, their neighbours, `S_NULL` and a stride across the table (a full
        // sweep of the table takes ~45 s in a debug build).
        let sweep = (0..OriginalState::all().count() as u16)
            .filter(|n| n % 41 == 0 || loop_states.iter().any(|l| n.abs_diff(*l) <= 4));
        let accepted: Vec<u16> = sweep
            .filter(|&n| {
                let d = OriginalState::try_from(n).unwrap().descriptor();
                let mut moved = c.clone();
                moved
                    .set(&format!("{p}.state"), n.to_le_bytes())
                    .set(&format!("{p}.sprite"), d.sprite.ordinal().to_le_bytes())
                    .set(&format!("{p}.frame"), d.frame.to_le_bytes())
                    .set(&format!("{p}.tics"), d.tics.to_le_bytes());
                moved.decode().is_ok()
            })
            .collect();
        assert_eq!(accepted, loop_states, "{ty:?}");
    }
}
