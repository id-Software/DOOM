//! M5a real-map frame comparison (local only: it reads the IWAD). Replays one script through
//! the simulation, renders every tic at each requested view configuration and compares every
//! frame with the genuine M5a runner's frame stream (`--frames` of
//! `tools/reference/m5a_driver.c`, DOOM-M5A-FRAMES-v1) by the rules of `tests/frames.rs`: the
//! full SHA-256, or the digest with the genuine tracker's flagged columns zeroed (owner ruling
//! on R4); a frame the tracker could not attribute, or whose switch parity is unknown
//! (decision R7), is non-comparable. It writes the counts and the digest of the Rust frame
//! stream as JSON; `tools/local/m5a_real.py` drives it and commits counts and digests only.
//!
//! With `--raw SB=DIR` (the genuine runner's `--frames-raw`) it also compares every pixel of
//! every comparable frame outside the flagged columns and names the feature in play at the
//! first difference, writing a PNG pair (genuine, Rust; the IWAD's palette 0) to `--diff`
//! (default `~/scratch/m5a/diff`). Those images are IWAD-derived and must never be committed.
//!
//! ```sh
//! cargo run -p doom-render --release --example m5a_real -- --iwad ~/scratch/m5a/iwad/DOOM.WAD \
//!     --lmp tests/reference/e1m1_scripts/WALK.lmp --frames 10=walk.sb10.frames [--raw 10=DIR] \
//!     [--digests 10=FILE] [--diff DIR] --json OUT
//! ```
//!
//! The stream digest is the SHA-256 of one line `tic,digest\n` per genuine frame, where the
//! digest is the frame's SHA-256 (the masked one where the genuine tracker flagged columns),
//! `whole` where the genuine frame is non-comparable as a whole, and `refused` where the Rust
//! renderer refused; `m5a_real.py` computes the genuine stream's digest by the same rule.
#[path = "../tests/common/mod.rs"]
mod common;

use common::{FrameStream, Outcome, harness_streams, masked, parse_frames};
use doom_render::{Feature, SCREENHEIGHT, SCREENWIDTH};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

/// The feature in play at a pixel, with sprite names (`sprnames` order).
fn describe(f: Feature) -> String {
    let name = |sprite: u16| {
        doom_sim::identity::OriginalSprite::all()
            .nth(usize::from(sprite))
            .map_or("?", |s| s.resource_name())
    };
    match f {
        Feature::Sprite {
            vissprite,
            sprite,
            frame,
        } => format!(
            "vissprite {vissprite} (sprite {} frame {})",
            name(sprite),
            char::from(b'A' + frame as u8)
        ),
        Feature::PSprite {
            psprite,
            sprite,
            frame,
        } => format!(
            "{} psprite (sprite {} frame {})",
            if psprite == 0 { "weapon" } else { "flash" },
            name(sprite),
            char::from(b'A' + frame as u8)
        ),
        other => format!("{other:?}"),
    }
}

/// `SB=VALUE` pairs.
fn pair(v: &str) -> (u8, String) {
    let (sb, rest) = v.split_once('=').expect("SB=VALUE");
    (sb.parse().expect("screenblocks"), rest.to_string())
}

/// Per-configuration state collected while the harness runs.
#[derive(Default)]
struct Side {
    lines: String,
    raw_frames: usize,
    raw_matched: usize,
    raw_differing: usize,
    raw_pixels: usize,
    selftests: usize,
    first: Option<Value>,
}

fn main() {
    let mut iwad = None;
    let mut lmp: Option<PathBuf> = None;
    let mut frames: BTreeMap<u8, PathBuf> = BTreeMap::new();
    let mut raw: BTreeMap<u8, PathBuf> = BTreeMap::new();
    let mut digests: BTreeMap<u8, PathBuf> = BTreeMap::new();
    let mut json_out = None;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let mut diff = home.join("scratch/m5a/diff");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{a}: missing value"));
        match a.as_str() {
            "--iwad" => iwad = Some(PathBuf::from(value())),
            "--lmp" => lmp = Some(PathBuf::from(value())),
            "--frames" => {
                let (sb, p) = pair(&value());
                frames.insert(sb, p.into());
            }
            "--raw" => {
                let (sb, p) = pair(&value());
                raw.insert(sb, p.into());
            }
            "--digests" => {
                let (sb, p) = pair(&value());
                digests.insert(sb, p.into());
            }
            "--diff" => diff = PathBuf::from(value()),
            "--json" => json_out = Some(PathBuf::from(value())),
            _ => {
                eprintln!(
                    "usage: m5a_real --iwad WAD --lmp DEMO --frames SB=FILE... [--raw SB=DIR] [--digests SB=FILE] [--diff DIR] --json OUT"
                );
                std::process::exit(2);
            }
        }
    }
    let (Some(iwad), Some(lmp), Some(json_out)) = (iwad, lmp, json_out) else {
        eprintln!("--iwad, --lmp and --json are required");
        std::process::exit(2);
    };
    let lump = lmp
        .file_stem()
        .expect("demo name")
        .to_string_lossy()
        .to_uppercase();
    let item = common::Item {
        lump: lump.clone(),
        wad: iwad,
        lmp,
        room: false,
        gfx: false,
    };
    let configs: Vec<u8> = frames.keys().copied().collect();
    let mut streams: Vec<FrameStream> = Vec::new();
    let mut end_checks: Vec<Option<(i64, String)>> = Vec::new();
    for sb in &configs {
        let text = std::fs::read_to_string(&frames[sb]).expect("frame stream");
        let mut s = parse_frames(&text, &lump);
        // A genuine limit ending (decision R3): the first O row (a write past a renderer
        // array; that frame's pixels follow the overwrite) or E row (I_Error during the frame,
        // which writes no F row). The Rust renderer must refuse that tic.
        let end = match (&s.witness, &s.error) {
            (Some((t, w)), _) => Some((*t, format!("refused:{w}"))),
            (None, Some((t, e))) => Some((*t, format!("error:{e}"))),
            _ => None,
        };
        if let Some((t, _)) = &end {
            s.frames.retain(|tic, _| tic < t);
        }
        // Every tic from 0 to the last genuine frame must have its F row.
        if let Some(problem) = common::stream_gaps(&s, s.frames.len()) {
            eprintln!("{lump} sb{sb}: genuine frame stream: {problem}");
            std::process::exit(1);
        }
        end_checks.push(end);
        streams.push(s);
    }
    let trace = !raw.is_empty();
    let mut sides: Vec<Side> = configs.iter().map(|_| Side::default()).collect();
    let counts = harness_streams(&item, &configs, &streams, &end_checks, trace, |s| {
        let k = configs
            .iter()
            .position(|c| *c == s.screenblocks)
            .expect("config");
        let side = &mut sides[k];
        let digest = match s.result {
            Err(_) => "refused".to_string(),
            Ok(_) if s.row.whole => "whole".to_string(),
            Ok(_) if s.row.columns.is_empty() => doom_test_evidence::sha256(s.frame.pixels()),
            Ok(_) => doom_test_evidence::sha256(&masked(s.frame.pixels(), &s.row.columns)),
        };
        let _ = writeln!(side.lines, "{},{digest}", s.tic);
        if side.first.is_none() {
            match s.outcome {
                Outcome::Refused(e) => {
                    side.first =
                        Some(json!({"tic": s.tic, "what": format!("Rust refused: {e:?}")}));
                }
                Outcome::Mismatch if raw.is_empty() => {
                    side.first = Some(json!({"tic": s.tic, "what": "the digest differs"}));
                }
                _ => {}
            }
        }
        let Some(dir) = raw.get(&s.screenblocks) else {
            return;
        };
        if !matches!(s.outcome, Outcome::Match { .. } | Outcome::Mismatch) {
            return;
        }
        let Ok(c) = std::fs::read(dir.join(format!("frame_{:05}.raw", s.tic))) else {
            return;
        };
        assert_eq!(c.len(), SCREENWIDTH * SCREENHEIGHT);
        let rust = s.frame.pixels();
        let excluded = |p: usize| s.row.columns.contains(&((p % SCREENWIDTH) as u16));
        let diff_of = |px: &[u8]| {
            (0..c.len())
                .filter(|&p| !excluded(p) && c[p] != px[p])
                .count()
        };
        let n = diff_of(rust);
        side.raw_frames += 1;
        if side.raw_frames == 1 {
            // Self-test: a planted change to one compared pixel must change the count by one.
            if let Some(p) = (0..c.len()).find(|&p| !excluded(p)) {
                let mut planted = rust.to_vec();
                planted[p] ^= 1;
                let m = diff_of(&planted);
                assert_eq!(m.abs_diff(n), 1, "self-test");
                side.selftests += 1;
            }
        }
        if n == 0 {
            side.raw_matched += 1;
            return;
        }
        side.raw_differing += 1;
        side.raw_pixels += n;
        if side.first.is_none() {
            let p = (0..c.len())
                .find(|&p| !excluded(p) && c[p] != rust[p])
                .expect("a difference");
            let (x, y) = (p % SCREENWIDTH, p / SCREENWIDTH);
            let feature = s.renderer.trace().map_or("-".into(), |t| describe(t[p]));
            let palette = s.renderer.resources().palette.clone().unwrap_or_default();
            let mut written = Vec::new();
            for (name, px) in [("c", c.as_slice()), ("rust", rust)] {
                let path = diff.join(format!("{lump}_sb{}_{}_{name}.png", s.screenblocks, s.tic));
                std::fs::create_dir_all(&diff).expect("diff directory");
                std::fs::write(
                    &path,
                    doom_render::png::encode_indexed(320, 200, px, &palette),
                )
                .expect("png");
                written.push(path.display().to_string());
            }
            side.first = Some(json!({
                "tic": s.tic,
                "what": format!("{n} pixels differ; first ({x}, {y}): genuine {} rust {}; written by {feature}", c[p], rust[p]),
                "pngs": written,
            }));
        }
    });
    let mut out = serde_json::Map::new();
    for (((sb, c), side), stream) in counts.into_iter().zip(sides).zip(&streams) {
        // Per configuration, every genuine frame was compared or counted as not reached.
        if c.missing != 0 || c.frames + c.not_reached != stream.frames.len() {
            eprintln!(
                "{lump} sb{sb}: {} frames compared and {} not reached of {} genuine frames ({} tics without a row)",
                c.frames,
                c.not_reached,
                stream.frames.len(),
                c.missing
            );
            std::process::exit(1);
        }
        if let Some(p) = digests.get(&sb) {
            std::fs::write(p, &side.lines).expect("digest list");
        }
        let refusals: BTreeMap<String, usize> = c.refusals.clone();
        let mut v = json!({
            "frames": c.frames + c.not_reached,
            "rendered": c.frames,
            "matched": c.matched,
            "mismatched": c.mismatched,
            "non_comparable": c.non_comparable,
            "parity_unknown": c.parity_unknown,
            "whole": c.whole,
            "refused": c.refused,
            "refusals": refusals,
            "not_reached": c.not_reached,
            "counter_mismatches": c.counter_mismatches,
            "flag_disagreements": c.flag_disagreements,
            "observe_error": c.observe_error.as_ref().map(|(t, e)| json!({"tic": t, "error": e})),
            "end": c.end.as_ref().map(|(t, e, r)| json!({"tic": t, "genuine": e, "rust": format!("{r:?}")})),
            "rust_stream_sha256": doom_test_evidence::sha256(side.lines.as_bytes()),
            "first": side.first,
        });
        if !raw.is_empty() {
            v["raw"] = json!({
                "compared": side.raw_frames,
                "matched": side.raw_matched,
                "differing": side.raw_differing,
                "differing_pixels": side.raw_pixels,
                "selftests": side.selftests,
            });
        }
        eprintln!(
            "{lump} sb{sb}: frames {} matched {} mismatched {} non-comparable {} (parity {} whole {}) refused {} {:?} not-reached {} counter-mismatches {} flag-disagreements {}{}",
            c.frames + c.not_reached,
            c.matched,
            c.mismatched,
            c.non_comparable,
            c.parity_unknown,
            c.whole,
            c.refused,
            c.refusals,
            c.not_reached,
            c.counter_mismatches,
            c.flag_disagreements,
            v["first"]
                .as_object()
                .map(|f| format!("; first: tic {} {}", f["tic"], f["what"]))
                .unwrap_or_default()
        );
        out.insert(sb.to_string(), v);
    }
    std::fs::write(
        &json_out,
        serde_json::to_string_pretty(&Value::Object(out)).expect("json") + "\n",
    )
    .expect("json");
}
