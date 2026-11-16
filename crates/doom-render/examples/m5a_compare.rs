//! M5a first-difference reporter (local only). Runs the pixel comparison harness of
//! `tests/frames.rs` on the synthetic items and, for each item and view configuration, reports
//! the counts and the first frame that does not match: its dumped tic, the Rust refusal or the
//! first differing pixel outside the genuine tracker's flagged columns, and the renderer stage
//! that wrote that pixel. With `--c-runner`, it reruns the genuine M5a runner with
//! `--frames-raw` for that item and configuration and writes a PNG pair (genuine, Rust; palette
//! 0 of the synthetic PLAYPAL) to the diff directory (default `~/scratch/m5a/diff`).
//!
//! ```sh
//! python3 tools/reference/m5a_build.py --variant strict --out ~/scratch/m5a/build
//! cargo run -p doom-render --release --example m5a_compare -- \
//!     --c-runner ~/scratch/m5a/build/strict/crunner [--only M5PEG,TELE1] [--screenblocks 11,10,9] [--diff DIR]
//! ```
#[path = "../tests/common/mod.rs"]
mod common;

use common::{Outcome, harness, items, reference};
use doom_render::{Frame, SCREENHEIGHT, SCREENWIDTH};
use std::path::PathBuf;
use std::process::Command;

struct First {
    lump: String,
    screenblocks: u8,
    tic: i64,
    outcome: Outcome,
    frame: Frame,
    columns: Vec<u16>,
    features: Option<Vec<doom_render::Feature>>,
}

fn main() {
    let mut only: Option<Vec<String>> = None;
    let mut configs: Vec<u8> = vec![11, 10, 9];
    let mut runner: Option<PathBuf> = None;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let mut diff = home.join("scratch/m5a/diff");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{a}: missing value"));
        match a.as_str() {
            "--only" => only = Some(value().split(',').map(str::to_uppercase).collect()),
            "--screenblocks" => {
                configs = value()
                    .split(',')
                    .map(|s| s.parse().expect("screenblocks"))
                    .collect()
            }
            "--c-runner" => runner = Some(PathBuf::from(value())),
            "--diff" => diff = PathBuf::from(value()),
            _ => {
                eprintln!(
                    "usage: m5a_compare [--only LUMPS] [--screenblocks 11,10,9] [--c-runner BIN] [--diff DIR]"
                );
                std::process::exit(2);
            }
        }
    }
    let mut firsts: Vec<(common::Item, First)> = Vec::new();
    let mut total = common::Counts::default();
    for item in items() {
        if only.as_ref().is_some_and(|o| !o.contains(&item.lump)) {
            continue;
        }
        let mut found: Vec<First> = Vec::new();
        let counts = harness(&item, &configs, true, |s| {
            if matches!(s.outcome, Outcome::Match { .. } | Outcome::NonComparable)
                || found.iter().any(|f| f.screenblocks == s.screenblocks)
            {
                return;
            }
            found.push(First {
                lump: s.item.lump.clone(),
                screenblocks: s.screenblocks,
                tic: s.tic,
                outcome: s.outcome.clone(),
                frame: s.frame.clone(),
                columns: s.row.columns.clone(),
                features: s.renderer.trace().map(<[_]>::to_vec),
            });
        });
        for (sb, c) in &counts {
            println!(
                "{} sb{sb}: frames {} matched {} mismatched {} non-comparable {} refused {} flag-disagreements {} not-reached {}",
                item.lump,
                c.frames,
                c.matched,
                c.mismatched,
                c.non_comparable,
                c.refused,
                c.flag_disagreements,
                c.not_reached
            );
            total.frames += c.frames;
            total.matched += c.matched;
            total.mismatched += c.mismatched;
            total.non_comparable += c.non_comparable;
            total.refused += c.refused;
        }
        firsts.extend(found.into_iter().map(|f| (item.clone(), f)));
    }
    println!(
        "total: frames {} matched {} mismatched {} non-comparable {} refused {}",
        total.frames, total.matched, total.mismatched, total.non_comparable, total.refused
    );
    for (item, f) in &firsts {
        let what = match &f.outcome {
            Outcome::Refused(e) => format!("Rust refused: {e:?}"),
            Outcome::Mismatch => "pixels differ".to_string(),
            o => format!("{o:?}"),
        };
        println!(
            "first difference: {} sb{} tic {}: {what}",
            f.lump, f.screenblocks, f.tic
        );
        let Some(bin) = &runner else { continue };
        let raw = diff.join(format!("raw_{}_sb{}", f.lump, f.screenblocks));
        std::fs::create_dir_all(&raw).expect("raw directory");
        let status = Command::new(bin)
            .args(["--wad"])
            .arg(&item.wad)
            .arg("--wad")
            .arg(reference("m5a_gfx.wad"))
            .arg("--wad")
            .arg(&item.lmp)
            .args(["--demo", &item.lump, "--no-dump", "--render", "every"])
            .args([
                "--screenblocks",
                &f.screenblocks.to_string(),
                "--frames-raw",
            ])
            .arg(&raw)
            .stdout(std::process::Stdio::null())
            .status()
            .expect("genuine runner");
        assert!(status.success(), "genuine runner failed: {status}");
        let c = std::fs::read(raw.join(format!("frame_{:05}.raw", f.tic))).expect("raw frame");
        assert_eq!(c.len(), SCREENWIDTH * SCREENHEIGHT);
        let _ = std::fs::remove_dir_all(&raw);
        match common::first_difference(&c, f.frame.pixels(), &f.columns) {
            Some((x, y)) => {
                let stage = f
                    .features
                    .as_ref()
                    .map(|t| format!("{:?}", t[y * SCREENWIDTH + x]))
                    .unwrap_or_default();
                println!(
                    "  first differing pixel ({x}, {y}): genuine {} rust {} (Rust stage {stage})",
                    c[y * SCREENWIDTH + x],
                    f.frame.pixels()[y * SCREENWIDTH + x]
                );
            }
            None => println!("  no differing pixel outside the flagged columns"),
        }
        let palette = common::load(item).res.palette.clone().unwrap_or_default();
        for (side, px) in [("c", c.as_slice()), ("rust", f.frame.pixels())] {
            let path = diff.join(format!(
                "{}_sb{}_{}_{side}.png",
                f.lump, f.screenblocks, f.tic
            ));
            std::fs::write(
                &path,
                doom_render::png::encode_indexed(320, 200, px, &palette),
            )
            .expect("png");
            println!("  wrote {}", path.display());
        }
    }
}
