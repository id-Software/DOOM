//! Wire 24 (`PlayerMonstersEpisode1V2`, DSIM40) on the M4b mechanism WADs
//! (`tests/reference/m4b_wads/`), tic for tic against the genuine engine's goldens
//! (M4b step 4: teleportation; step 5: the Baron of Hell and `A_BossDeath`; step
//! 6: hard skill).
//!
//! `tools/reference/check_player_episode1v2.py` performs the same comparison on the
//! release example's captures; this test makes it part of `cargo test`, through the
//! public API only (`Simulation::original_player_episode1_v2_map`, `advance`,
//! `snapshot`, `Diagnostic::decode_episode1_v2`). Every expected value is read from
//! `tests/reference/m4b_synthetic_goldens.json` and the committed fixtures
//! `tests/reference/m4b_episode1v2_*.bin`, which `m4b_snapshot_goldens.py` projected
//! from the dumps of the genuine linuxdoom-1.10 engine with the independent DSIM40
//! encoder `level_snapshot_m4b.py`: never from Rust output. For each script admitted
//! so far:
//!
//! - every capture, stripped as the comparator strips it (no `map.*`, no
//!   `events.*`), has the golden per-tic SHA-256 prefix, and the sequence hash and
//!   final digest match;
//! - every capture decodes with the DSIM40 reader;
//! - at each committed fixture's tic the capture, with its event lists emptied (the
//!   fixtures carry `events.*.count` 0 and no event), equals the fixture byte for
//!   byte;
//! - SPECH9 (declared `after_last_capture`): the captures up to DSIM tic 75 match,
//!   and DSIM tic 76 of the probe demo (`SPECH8.lmp`, whose commands begin with
//!   SPECH9's) is refused with `MapScaleError::SpecialCross`, the tic in which the
//!   genuine engine writes a ninth `spechit` entry (p_map.c:242; the address
//!   variant's probe).
//!
//! - BOSS1 (declared end `exit`): every capture matches through the exit tic, and
//!   the next tic is refused with `SimError::LevelExited`.
//!
//! TELE2X, of the second M4b set (`m4b_synthetic_goldens2.json`, step 5), makes the
//! player teleport twice in one tic. SKILL3 (step 6) loads SKILL2's WAD at hard
//! skill (demo skill byte 3): the THING filter takes option bit 4 instead of 2
//! (p_mobj.c:743–751), and its tic-0 fixture holds the hard-only things.
//!
//! SAWBITE, of the second set (step 7), has a Demon bite the player in a tic in
//! which `A_Saw` hit the other Demon and set `MF_JUSTATTACKED` on the player
//! (p_pspr.c:542; DSIM tics 74 and 98): wire 24 admits that bite, every older
//! policy refuses it with `PlayerBodyError::Invariant`. Its fixture is the capture
//! of the first such bite.
//!
//! BTUNNEL, of the second set (step 9), has a Baron fireball (speed 15, radius 6,
//! info.c:1524–1548) step wholly past a 1-unit inner one-sided wall in one
//! unsplit step, because `P_CheckPosition` tests only the destination box
//! (p_map.c:191–195, 374–442); it flies on and explodes on the far wall (dump tics
//! 161–162 and 170). Wire 24 admits it, as the genuine engine does: the fireball
//! stays inside the vertex rectangle. Its fixture is the capture one step past the
//! wall (DSIM tic 163).
//!
//! TELEZ, of the second set (step 14, review finding F1), has a Demon whose spawn
//! box reaches a floor 24 above its z cross a 97 line on its first step: `P_Move`
//! sets `z = floorz` only after `P_TryMove`'s crossing loop (p_enemy.c:293,
//! 327–328; p_map.c:500–514), so `EV_Teleport` spawns fog 1 at the pre-move z 0
//! (p_telept.c:100–111). Its fixture is the capture of the teleport (DSIM tic 11).
use doom_sim::{demo::parse_demo_episode1_v2, *};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

/// A minimal JSON value: enough for the committed goldens file.
#[derive(Debug)]
enum Json {
    Null,
    Bool,
    Number(i64),
    Text(String),
    List(Vec<Json>),
    Object(BTreeMap<String, Json>),
}
impl Json {
    fn get(&self, key: &str) -> &Json {
        match self {
            Json::Object(m) => m.get(key).unwrap_or_else(|| panic!("missing key {key}")),
            other => panic!("not an object: {other:?}"),
        }
    }
    fn text(&self) -> &str {
        match self {
            Json::Text(s) => s,
            other => panic!("not a string: {other:?}"),
        }
    }
    fn number(&self) -> usize {
        match self {
            Json::Number(n) => usize::try_from(*n).expect("non-negative"),
            other => panic!("not a number: {other:?}"),
        }
    }
    fn list(&self) -> &[Json] {
        match self {
            Json::List(v) => v,
            other => panic!("not a list: {other:?}"),
        }
    }
}
struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Parser<'_> {
    fn skip(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn eat(&mut self, b: u8) {
        self.skip();
        assert_eq!(self.bytes[self.at], b, "JSON at byte {}", self.at);
        self.at += 1;
    }
    fn peek(&mut self) -> u8 {
        self.skip();
        self.bytes[self.at]
    }
    fn string(&mut self) -> String {
        self.eat(b'"');
        let mut out = String::new();
        loop {
            let b = self.bytes[self.at];
            self.at += 1;
            match b {
                b'"' => return out,
                b'\\' => {
                    let e = self.bytes[self.at];
                    self.at += 1;
                    match e {
                        b'"' | b'\\' | b'/' => out.push(char::from(e)),
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = std::str::from_utf8(&self.bytes[self.at..self.at + 4])
                                .expect("hex");
                            self.at += 4;
                            out.push(
                                char::from_u32(u32::from_str_radix(hex, 16).expect("hex"))
                                    .unwrap_or('?'),
                            );
                        }
                        other => panic!("escape {other}"),
                    }
                }
                _ => {
                    // Multi-byte UTF-8 passes through byte by byte.
                    let start = self.at - 1;
                    let len = match b {
                        0..=0x7f => 1,
                        0xc0..=0xdf => 2,
                        0xe0..=0xef => 3,
                        _ => 4,
                    };
                    self.at = start + len;
                    out.push_str(std::str::from_utf8(&self.bytes[start..self.at]).expect("utf8"));
                }
            }
        }
    }
    fn value(&mut self) -> Json {
        match self.peek() {
            b'{' => {
                self.eat(b'{');
                let mut m = BTreeMap::new();
                if self.peek() == b'}' {
                    self.eat(b'}');
                    return Json::Object(m);
                }
                loop {
                    let k = self.string();
                    self.eat(b':');
                    m.insert(k, self.value());
                    if self.peek() == b',' {
                        self.eat(b',');
                    } else {
                        self.eat(b'}');
                        return Json::Object(m);
                    }
                }
            }
            b'[' => {
                self.eat(b'[');
                let mut v = Vec::new();
                if self.peek() == b']' {
                    self.eat(b']');
                    return Json::List(v);
                }
                loop {
                    v.push(self.value());
                    if self.peek() == b',' {
                        self.eat(b',');
                    } else {
                        self.eat(b']');
                        return Json::List(v);
                    }
                }
            }
            b'"' => Json::Text(self.string()),
            b'n' => {
                self.at += 4;
                Json::Null
            }
            b't' => {
                self.at += 4;
                Json::Bool
            }
            b'f' => {
                self.at += 5;
                Json::Bool
            }
            _ => {
                let start = self.at;
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| b.is_ascii_digit() || *b == b'-')
                {
                    self.at += 1;
                }
                Json::Number(
                    std::str::from_utf8(&self.bytes[start..self.at])
                        .expect("number")
                        .parse()
                        .expect("integer"),
                )
            }
        }
    }
}
fn goldens_file(name: &str) -> Json {
    let bytes = std::fs::read(reference(name)).expect("goldens");
    Parser {
        bytes: &bytes,
        at: 0,
    }
    .value()
}
/// The goldens holding `script`: the first M4b set's, or the second set's
/// (`m4b_synthetic_goldens2.json`, M4b step 5: TELE2X; step 7: SAWBITE; step 9: BTUNNEL;
/// step 14: TELEZ).
fn goldens(script: &str) -> Json {
    let first = goldens_file("m4b_synthetic_goldens.json");
    match first.get("scripts") {
        Json::Object(m) if m.contains_key(script) => first,
        _ => goldens_file("m4b_synthetic_goldens2.json"),
    }
}

/// The canonical bytes of `snap` without its `map.*` and `events.*` fields
/// (`level_compare.stripped`).
fn stripped(snap: &Snapshot) -> Vec<u8> {
    let fields: Vec<_> = snap
        .fields()
        .filter(|f| !f.path().starts_with("map.") && !f.path().starts_with("events."))
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

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
        .collect()
}

/// The capture with its event lists emptied: the committed fixtures' form
/// (`events.last.count` and `events.pending.count` 0, no event fields).
fn without_events(snap: &Snapshot) -> Vec<u8> {
    let fields: Vec<(String, Vec<u8>)> = snap
        .fields()
        .filter(|f| !f.path().starts_with("events.") || f.path().ends_with(".count"))
        .map(|f| {
            if f.path().starts_with("events.") {
                (f.path().to_string(), 0_u64.to_le_bytes().to_vec())
            } else {
                (f.path().to_string(), f.value().to_vec())
            }
        })
        .collect();
    let mut out = b"DSIM".to_vec();
    out.extend(snap.version().to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for (path, value) in fields {
        out.extend((path.len() as u32).to_le_bytes());
        out.extend(path.as_bytes());
        out.extend((value.len() as u64).to_le_bytes());
        out.extend(value);
    }
    out
}

fn run(script: &str) {
    run_with(script, |_| {});
}

/// [`run`] with the demo's commands changed by `tweak` first (the self-test).
fn run_with(script: &str, tweak: impl FnOnce(&mut Vec<TicCommand>)) {
    run_at(script, None, tweak);
}

/// [`run_with`] with the skill forced to `skill` instead of the demo header's
/// (the skill self-test); `None` keeps the header's.
fn run_at(script: &str, forced: Option<Skill>, tweak: impl FnOnce(&mut Vec<TicCommand>)) {
    let all = goldens(script);
    assert_eq!(all.get("dsim").get("version").number(), 40);
    let g = all.get("scripts").get(script);
    let wad_file = format!("m4b_wads/{}.wad", g.get("wad").text().to_ascii_lowercase());
    let wad_bytes = std::fs::read(reference(&wad_file)).expect("WAD");
    assert_eq!(
        doom_test_evidence::sha256(&wad_bytes),
        g.get("wad_sha256").text(),
        "{script}: the WAD differs from the goldens'"
    );
    let lmp = std::fs::read(reference(&format!("m4b_scripts/{script}.lmp"))).expect("demo");
    assert_eq!(
        doom_test_evidence::sha256(&lmp),
        g.get("lmp_sha256").text(),
        "{script}: the demo differs from the goldens'"
    );
    let slot = g.get("slot").text();
    let map_number = u8::try_from(g.get("map").number()).expect("map");
    assert!(matches!(g.get("projection_stop"), Json::Null), "{script}");
    let digests: Vec<&str> = g.get("tic_digests").list().iter().map(Json::text).collect();
    let snapshots = g.get("snapshots").number();
    assert_eq!(digests.len(), snapshots);
    // The refused DSIM tic of a declared `after_last_capture` and the demo it runs on.
    let (refused, demo_bytes) = match g.get("after_last_capture") {
        Json::Null => (None, lmp),
        after => {
            assert_eq!(after.get("refuse").text(), "SpecialCross");
            let tic = after.get("dsim_tic").number();
            assert_eq!(tic, snapshots);
            let probe = std::fs::read(reference("m4b_scripts/SPECH8.lmp")).expect("probe");
            (Some(tic), probe)
        }
    };
    let end = g.get("end");
    let exit_tic = (end.get("kind").text() == "exit").then(|| end.get("tic").number());
    let fixtures: Vec<(usize, Vec<u8>)> = all
        .get("fixtures")
        .list()
        .iter()
        .filter(|f| f.get("script").text() == script)
        .map(|f| {
            let path = f.get("path").text();
            let bytes = std::fs::read(reference(path.trim_start_matches("tests/reference/")))
                .expect("fixture");
            assert_eq!(doom_test_evidence::sha256(&bytes), f.get("sha256").text());
            (f.get("tic").number(), bytes)
        })
        .collect();

    let wad = doom_wad::Wad::parse(&wad_bytes, doom_wad::Limits::default()).expect("WAD");
    let map = doom_map::Map::from_wad(&wad, slot, doom_map::MapLimits::default()).expect("map");
    let (header, mut demo) = parse_demo_episode1_v2(&demo_bytes, map_number).expect("demo");
    // The goldens' skill is the genuine run's `gameskill`; the demo header carries it.
    assert_eq!(
        usize::from(header.skill),
        g.get("skill").number(),
        "{script}"
    );
    let skill = match header.skill {
        2 => Skill::Medium,
        3 => Skill::Hard,
        other => panic!("{script}: skill {other}"),
    };
    let skill = forced.unwrap_or(skill);
    tweak(&mut demo.commands);
    let mut s = Simulation::original_player_episode1_v2_map(
        map,
        map_number,
        skill,
        MapContextOptions::default(),
    )
    .expect("admission");
    s.spawn_map_things().expect("THINGs");
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("weapons");
    s.spawn_level_specials().expect("specials");

    let mut sequence = Vec::new();
    let mut fixtures_seen = 0;
    let mut check = |tic: usize, s: &Simulation| {
        assert!(
            tic < snapshots,
            "{script}: capture {tic} beyond the goldens"
        );
        let snap = s.snapshot();
        assert_eq!(snap.version(), 40);
        let bytes = snap.canonical_bytes();
        Diagnostic::decode_episode1_v2(&bytes, DiagnosticLimits::default())
            .unwrap_or_else(|e| panic!("{script}: capture {tic} refused by the reader: {e:?}"));
        let digest = doom_test_evidence::sha256(&stripped(&snap));
        assert_eq!(&digest[..16], digests[tic], "{script}: capture {tic}");
        sequence.extend(hex_to_bytes(&digest));
        if tic == snapshots - 1 {
            assert_eq!(digest, g.get("final_sha256").text(), "{script}: final");
        }
        for (_, fixture) in fixtures.iter().filter(|(t, _)| *t == tic) {
            assert!(
                without_events(&snap) == *fixture,
                "{script}: capture {tic} differs from its fixture"
            );
            fixtures_seen += 1;
        }
    };
    check(0, &s);
    let mut ended = None;
    for (i, command) in demo.commands.iter().enumerate() {
        let tic = i + 1;
        let commands = [
            *command,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ];
        match s.advance(commands) {
            Ok(()) => check(tic, &s),
            Err(e) => {
                // A refused tic keeps the canonical state, and a retry refuses again.
                let before = s.snapshot().canonical_bytes();
                assert_eq!(s.advance(commands).err(), Some(e), "{script}: retry");
                assert_eq!(
                    s.snapshot().canonical_bytes(),
                    before,
                    "{script}: retry state"
                );
                ended = Some((tic, e));
                break;
            }
        }
    }
    match (refused, ended) {
        (Some(tic), Some(got)) => assert_eq!(
            got,
            (tic, MapScaleError::SpecialCross.into()),
            "{script}: refusal"
        ),
        (None, Some((tic, SimError::LevelExited))) if exit_tic.map(|t| t + 1) == Some(tic) => {}
        (None, None) => {}
        (want, got) => panic!("{script}: ending {got:?}, expected refusal at {want:?}"),
    }
    assert_eq!(fixtures_seen, fixtures.len(), "{script}: fixtures compared");
    assert_eq!(
        sequence.len() / 32,
        snapshots,
        "{script}: captures compared"
    );
    assert_eq!(
        doom_test_evidence::sha256(&sequence),
        g.get("sequence_sha256").text(),
        "{script}: sequence"
    );
}

/// The harness can fail: TELE1 with its first forward command removed no longer
/// matches the goldens.
#[test]
fn harness_detects_a_changed_command() {
    let changed = std::panic::catch_unwind(|| {
        run_with("TELE1", |commands| {
            let first = commands
                .iter_mut()
                .find(|c| c.forward != 0)
                .expect("a moving command");
            first.forward = 0;
        })
    });
    assert!(changed.is_err());
}

/// The harness can fail on the skill: SKILL3's WAD, demo and goldens replayed at
/// medium no longer match (the hard-only Baron, Imp and landing are not spawned and
/// the demo's teleport never happens), while the same call at hard passes.
#[test]
fn harness_detects_the_wrong_skill() {
    run_at("SKILL3", Some(Skill::Hard), |_| {});
    let wrong = std::panic::catch_unwind(|| run_at("SKILL3", Some(Skill::Medium), |_| {}));
    assert!(wrong.is_err());
}

#[test]
fn tele1() {
    run("TELE1");
}
#[test]
fn tele2() {
    run("TELE2");
}
#[test]
fn tsplit() {
    run("TSPLIT");
}
#[test]
fn tslide() {
    run("TSLIDE");
}
#[test]
fn mtele1() {
    run("MTELE1");
}
#[test]
fn slideb() {
    run("SLIDEB");
}
#[test]
fn slidec() {
    run("SLIDEC");
}
#[test]
fn skill2() {
    run("SKILL2");
}
#[test]
fn skill3() {
    run("SKILL3");
}
#[test]
fn spech8() {
    run("SPECH8");
}
#[test]
fn spech9() {
    run("SPECH9");
}
#[test]
fn tfrag1() {
    run("TFRAG1");
}
#[test]
fn baron1() {
    run("BARON1");
}
#[test]
fn binf1() {
    run("BINF1");
}
#[test]
fn bvb1() {
    run("BVB1");
}
#[test]
fn boss1() {
    run("BOSS1");
}
#[test]
fn boss2() {
    run("BOSS2");
}
#[test]
fn bossm1() {
    run("BOSSM1");
}
#[test]
fn tele2x() {
    run("TELE2X");
}
#[test]
fn sawbite() {
    run("SAWBITE");
}
#[test]
fn btunnel() {
    run("BTUNNEL");
}
#[test]
fn telez() {
    run("TELEZ");
}
