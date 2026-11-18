//! The mixer gate of M5c Phase 2 step 4: every committed start list of
//! tests/reference/m5c_mixer/ replayed through the Rust mixer under the declared schedule
//! (decision T5: buffer k of 512 frames after tic floor(512k/315)) must give exactly the rows of
//! the genuine linuxdoom-1.10 in-process mixer (built with its in-process sound code, one define
//! removed): table digests, lump rows, every start echo, every buffer digest and the stream
//! digest. The goldens come from the genuine C only (tools/reference/m5c_mixer_reference.py);
//! the device rows `d` and `q` are fake-device glue and are not compared.
//!
//! Equivalence class: source equivalence against genuine traces (the goldens), with the two
//! declared exceptions of decision T11 compared as refusals (the short-lump witnesses) and the
//! out-of-domain pitch probe as a refusal (tests/reference/m5c_mixer/README.md).

mod common;

use common::{Stop, first_difference, golden_rows, parse_starts, reference, replay, sound_set};
use doom_sound::mixer::MixRefusal;

/// The 23 goldened scenarios: (name, synthetic WAD, `--lumps`), as
/// tools/reference/m5c_mixer_reference.py `scenarios()` runs them.
const SCENARIOS: [(&str, &str, bool); 23] = [
    ("lumps_main", "m5c_sfx.wad", true),
    ("lumps_edge", "m5c_sfx_edge.wad", true),
    ("basic", "m5c_sfx.wad", false),
    ("volume", "m5c_sfx.wad", false),
    ("separation", "m5c_sfx.wad", false),
    ("separation_wide", "m5c_sfx.wad", false),
    ("pitch", "m5c_sfx.wad", false),
    ("single_instance", "m5c_sfx.wad", false),
    ("nine", "m5c_sfx.wad", false),
    ("oldest", "m5c_sfx.wad", false),
    ("boundary", "m5c_sfx_edge.wad", false),
    ("clip", "m5c_sfx_edge.wad", false),
    ("chgun", "m5c_sfx.wad", false),
    ("header", "m5c_sfx_edge.wad", false),
    ("padding", "m5c_sfx_edge.wad", false),
    ("missing", "m5c_sfx_edge.wad", false),
    ("handle_wrap", "m5c_sfx.wad", false),
    ("phase", "m5c_sfx.wad", false),
    ("busy", "m5c_sfx.wad", false),
    ("x_vol128_left", "m5c_sfx_edge.wad", false),
    ("x_vol128_right", "m5c_sfx_edge.wad", false),
    ("x_sep257", "m5c_sfx_edge.wad", false),
    ("x_sepm2", "m5c_sfx_edge.wad", false),
];

fn listed(dir: &str, ext: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(reference(dir))
        .expect("reference directory")
        .map(|e| e.expect("entry").file_name().into_string().expect("name"))
        .filter_map(|n| n.strip_suffix(ext).map(str::to_string))
        .collect();
    names.sort();
    names
}

fn read(path: &str) -> String {
    std::fs::read_to_string(reference(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn every_scenario_golden_is_matched_row_for_row() {
    let mut want_names: Vec<String> = SCENARIOS.iter().map(|s| s.0.to_string()).collect();
    want_names.sort();
    assert_eq!(
        listed("m5c_mixer/starts", ".txt"),
        want_names,
        "scenario start lists"
    );
    assert_eq!(
        listed("m5c_mixer/goldens", ".golden"),
        want_names,
        "scenario goldens"
    );
    let (mut starts, mut buffers, mut rows) = (0, 0, 0);
    for (name, wad, lumps) in SCENARIOS {
        let set = sound_set(wad);
        let list = parse_starts(&read(&format!("m5c_mixer/starts/{name}.txt")));
        let got = replay(&set, &list, lumps, 2_000_000);
        let want = golden_rows(&read(&format!("m5c_mixer/goldens/{name}.golden")));
        if let Some((i, w, g)) = first_difference(&want, &got.rows) {
            panic!("scenario {name}: first differing row {i}:\n  golden: {w}\n  rust:   {g}");
        }
        starts += got.starts;
        buffers += got.buffers;
        rows += want.len();
    }
    // tests/reference/m5c_mixer_results.json: 65882 listed starts and 490 buffers over the 23
    // scenarios; 4 of the listed starts (one per x_ scenario) end in the genuine I_Error and are
    // refused here, so 65878 are mixed.
    assert_eq!(
        (starts, buffers),
        (65_878, 490),
        "starts mixed and buffers over the 23 scenarios"
    );
    eprintln!(
        "mixer gate: 23 scenarios, {starts} starts mixed (4 refused), {buffers} buffers, {rows} rows matched"
    );
}

#[test]
fn the_refused_scenarios_stop_where_the_genuine_code_ends_in_i_error() {
    let cases: [(&str, i64, i64, MixRefusal); 4] = [
        (
            "x_vol128_left",
            2,
            0,
            MixRefusal::LeftVolume { vol: 128, sep: 0 },
        ),
        (
            "x_vol128_right",
            2,
            0,
            MixRefusal::RightVolume { vol: 128, sep: 255 },
        ),
        (
            "x_sep257",
            2,
            0,
            MixRefusal::LeftVolume { vol: 127, sep: 257 },
        ),
        (
            "x_sepm2",
            2,
            0,
            MixRefusal::RightVolume { vol: 127, sep: -2 },
        ),
    ];
    for (name, tic, seq, refusal) in cases {
        let list = parse_starts(&read(&format!("m5c_mixer/starts/{name}.txt")));
        let got = replay(&sound_set("m5c_sfx_edge.wad"), &list, false, 2_000_000);
        assert_eq!(got.stop, Some(Stop::Mix { tic, seq, refusal }), "{name}");
    }
    let list = parse_starts(&read("m5c_mixer/starts/missing.txt"));
    let got = replay(&sound_set("m5c_sfx_edge.wad"), &list, false, 2_000_000);
    assert_eq!(
        got.stop,
        Some(Stop::Lump {
            tic: 4,
            seq: 1,
            sfx: 18
        })
    );
}

/// Decision T11: the three short-lump witnesses (lumps of 0, 5 and 8 bytes) are refused at the
/// start the genuine driver flags with an `o` row, (2, 0) in each
/// (tests/reference/m5c_mixer_results.json, `refusal_witnesses_T11`), with the mixer unchanged.
#[test]
fn the_short_lump_witnesses_are_refused_at_the_flagged_start() {
    assert_eq!(
        listed("m5c_mixer/witness", ".txt"),
        ["w_short0", "w_short5", "w_short8"]
    );
    for (name, sfx) in [("w_short0", 5), ("w_short5", 6), ("w_short8", 7)] {
        let list = parse_starts(&read(&format!("m5c_mixer/witness/{name}.txt")));
        let got = replay(&sound_set("m5c_sfx_edge.wad"), &list, false, 2_000_000);
        assert_eq!(
            got.stop,
            Some(Stop::Mix {
                tic: 2,
                seq: 0,
                refusal: MixRefusal::ShortLump { sfx }
            }),
            "{name}"
        );
        // The first start (shotgn at tic 0) and buffers 0 and 1 run before the refusal.
        assert_eq!((got.starts, got.buffers), (1, 2), "{name}");
    }
}

/// The pitch-256 probe (outside what s_sound.c can pass; the genuine address build reports
/// i_sound.c:495) is refused before any change.
#[test]
fn the_pitch_probe_is_refused() {
    assert_eq!(listed("m5c_mixer/probes", ".txt"), ["p_pitch256"]);
    let list = parse_starts(&read("m5c_mixer/probes/p_pitch256.txt"));
    let got = replay(&sound_set("m5c_sfx_edge.wad"), &list, false, 64);
    assert_eq!(
        got.stop,
        Some(Stop::Mix {
            tic: 0,
            seq: 0,
            refusal: MixRefusal::Pitch { pitch: 256 }
        })
    );
}

/// The 64 session goldens: the start lists of the committed sound-logic streams
/// (tests/reference/m5c_sound/, converted by tools/reference/m5c_starts.py) through the mixer.
/// Every handle the sound-logic run recorded must be the mixer's (no `m` row).
#[test]
fn every_session_golden_is_matched_row_for_row() {
    let names = listed("m5c_mixer/sessions/starts", ".txt");
    assert_eq!(names.len(), 64, "session start lists");
    assert_eq!(
        listed("m5c_mixer/sessions/goldens", ".golden"),
        names,
        "session goldens"
    );
    let set = sound_set("m5c_sfx.wad");
    let (mut starts, mut buffers, mut rows) = (0, 0, 0);
    for name in &names {
        let list = parse_starts(&read(&format!("m5c_mixer/sessions/starts/{name}.txt")));
        let got = replay(&set, &list, false, 2_000_000);
        let want = golden_rows(&read(&format!("m5c_mixer/sessions/goldens/{name}.golden")));
        if let Some((i, w, g)) = first_difference(&want, &got.rows) {
            panic!("session {name}: first differing row {i}:\n  golden: {w}\n  rust:   {g}");
        }
        assert!(got.stop.is_none(), "session {name} stopped");
        starts += got.starts;
        buffers += got.buffers;
        rows += want.len();
    }
    // tests/reference/m5c_mixer_sessions_results.json: 64 items, 741 starts, 14043 buffers.
    assert_eq!(
        (starts, buffers),
        (741, 14_043),
        "starts and buffers over the 64 sessions"
    );
    eprintln!("mixer gate: 64 sessions, {starts} starts, {buffers} buffers, {rows} rows matched");
}
