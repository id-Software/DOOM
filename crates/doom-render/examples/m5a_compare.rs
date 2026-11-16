//! M5a first-difference reporter (local only). Runs the pixel comparison harness of
//! `tests/frames.rs` on the synthetic items and, for each item and view configuration, reports
//! the counts and the first frame that does not match: its dumped tic, the Rust refusal or the
//! first differing pixel outside the genuine tracker's flagged columns, and the renderer stage
//! that wrote that pixel.
//!
//! With `--c-runner`, it reruns the genuine M5a runner with `--frames-raw` for every item and
//! configuration and also compares every pixel of every comparable frame against the genuine
//! raw frame, except the genuine tracker's flagged columns (decision R4). For the first
//! difference of each item and configuration it names the feature in play (the renderer
//! stage and its object: the seg, visplane, vissprite with its sprite name and frame, weapon
//! sprite, or the fuzz column's owner; [`doom_render::Feature`]) and writes a PNG pair
//! (genuine, Rust; palette 0 of the synthetic PLAYPAL) to the diff directory (default
//! `~/scratch/m5a/diff`).
//!
//! ```sh
//! python3 tools/reference/m5a_build.py --variant strict --out ~/scratch/m5a/build
//! cargo run -p doom-render --release --example m5a_compare -- \
//!     --c-runner ~/scratch/m5a/build/strict/crunner [--only M5PEG,TELE1] [--screenblocks 11,10,9] [--diff DIR]
//! ```
#[path = "../tests/common/mod.rs"]
mod common;

use common::{Outcome, harness, items, reference};
use doom_render::{Feature, Frame, SCREENHEIGHT, SCREENWIDTH};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The first non-matching frame of an item and configuration.
struct First {
    tic: i64,
    what: String,
    rust: Frame,
    genuine: Option<Vec<u8>>,
}

/// Raw-frame comparison counts of one configuration.
#[derive(Default, Clone)]
struct World {
    frames: usize,
    matched: usize,
    differing: usize,
    differing_pixels: usize,
    /// Pixels compared over all compared frames.
    compared_pixels: usize,
    /// Planted one-pixel changes the comparison detected (self-test, first compared frame).
    selftests: usize,
}

/// The pixels of `rust` that differ from the genuine frame `c` outside the genuine flagged
/// columns: their count, the first, and the pixels compared.
fn world_diff(c: &[u8], rust: &[u8], columns: &[u16]) -> (usize, Option<(usize, usize)>, usize) {
    let mut first = None;
    let mut n = 0;
    let mut compared = 0;
    for p in 0..c.len() {
        let x = p % SCREENWIDTH;
        if columns.contains(&(x as u16)) {
            continue;
        }
        compared += 1;
        if c[p] != rust[p] {
            n += 1;
            first.get_or_insert((x, p / SCREENWIDTH));
        }
    }
    (n, first, compared)
}

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

fn run_genuine(bin: &Path, item: &common::Item, sb: u8, dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).expect("raw directory");
    let status = Command::new(bin)
        .arg("--wad")
        .arg(&item.wad)
        .arg("--wad")
        .arg(reference("m5a_gfx.wad"))
        .arg("--wad")
        .arg(&item.lmp)
        .args(["--demo", &item.lump, "--no-dump", "--render", "every"])
        .args(["--screenblocks", &sb.to_string(), "--frames-raw"])
        .arg(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("genuine runner");
    // A run that ends at a renderer limit exits nonzero; its frames up to there are written.
    let _ = status;
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
    std::fs::create_dir_all(&diff).expect("diff directory");
    let mut totals: BTreeMap<u8, (common::Counts, World)> = BTreeMap::new();
    let mut reports: Vec<String> = Vec::new();
    for item in items() {
        if only.as_ref().is_some_and(|o| !o.contains(&item.lump)) {
            continue;
        }
        let raw_dirs: Vec<PathBuf> = configs
            .iter()
            .map(|sb| diff.join(format!("raw_{}_sb{sb}", item.lump)))
            .collect();
        if let Some(bin) = &runner {
            for (sb, dir) in configs.iter().zip(&raw_dirs) {
                run_genuine(bin, &item, *sb, dir);
            }
        }
        let mut world: Vec<World> = vec![World::default(); configs.len()];
        let mut firsts: Vec<Option<First>> = configs.iter().map(|_| None).collect();
        let counts = harness(&item, &configs, true, |s| {
            let k = configs
                .iter()
                .position(|c| *c == s.screenblocks)
                .expect("config");
            let (Ok(_), Some(trace)) = (s.result, s.renderer.trace()) else {
                if firsts[k].is_none() {
                    if let Outcome::Refused(e) = s.outcome {
                        firsts[k] = Some(First {
                            tic: s.tic,
                            what: format!("Rust refused: {e:?}"),
                            rust: s.frame.clone(),
                            genuine: None,
                        });
                    }
                }
                return;
            };
            if runner.is_none() {
                if firsts[k].is_none() && matches!(s.outcome, Outcome::Mismatch) {
                    firsts[k] = Some(First {
                        tic: s.tic,
                        what: "the digest differs (rerun with --c-runner for the pixel)".into(),
                        rust: s.frame.clone(),
                        genuine: None,
                    });
                }
                return;
            }
            if matches!(s.outcome, Outcome::NonComparable) {
                return;
            }
            let path = raw_dirs[k].join(format!("frame_{:05}.raw", s.tic));
            let Ok(c) = std::fs::read(&path) else {
                return;
            };
            assert_eq!(c.len(), SCREENWIDTH * SCREENHEIGHT);
            let w = &mut world[k];
            w.frames += 1;
            let rust = s.frame.pixels();
            let (n, first, compared) = world_diff(&c, rust, &s.row.columns);
            w.compared_pixels += compared;
            if w.frames == 1 {
                // Self-test: a planted change to one compared pixel must be detected.
                if let Some(p) =
                    (0..c.len()).find(|&p| !s.row.columns.contains(&((p % SCREENWIDTH) as u16)))
                {
                    let mut planted = rust.to_vec();
                    planted[p] ^= 1;
                    let (m, _, _) = world_diff(&c, &planted, &s.row.columns);
                    // The planted pixel turns from equal to differing, or the reverse.
                    let before = usize::from(c[p] != rust[p]);
                    let after = usize::from(c[p] != planted[p]);
                    assert_eq!(m + before, n + after, "self-test");
                    w.selftests += 1;
                }
            }
            if n == 0 {
                w.matched += 1;
                return;
            }
            w.differing += 1;
            w.differing_pixels += n;
            if firsts[k].is_none() {
                let (x, y) = first.expect("a difference");
                let p = y * SCREENWIDTH + x;
                firsts[k] = Some(First {
                    tic: s.tic,
                    what: format!(
                        "{n} pixels differ; first ({x}, {y}): genuine {} rust {}; written by {}",
                        c[p],
                        rust[p],
                        describe(trace[p])
                    ),
                    rust: s.frame.clone(),
                    genuine: Some(c),
                });
            }
        });
        for dir in &raw_dirs {
            let _ = std::fs::remove_dir_all(dir);
        }
        let palette = common::load(&item).res.palette.clone().unwrap_or_default();
        for (k, (sb, c)) in counts.into_iter().enumerate() {
            let w = &world[k];
            let end = c
                .end
                .as_ref()
                .map(|(t, e, r)| format!(" end tic {t} genuine {e} rust {r:?}"))
                .unwrap_or_default();
            println!(
                "{} sb{sb}: frames {} matched {} mismatched {} non-comparable {} refused {} {:?} counter-mismatches {} flag-disagreements {} raw: compared {} matched {} differing {} ({} pixels; {} pixels compared; self-tests {}){end}",
                item.lump,
                c.frames,
                c.matched,
                c.mismatched,
                c.non_comparable,
                c.refused,
                c.refusals,
                c.counter_mismatches,
                c.flag_disagreements,
                w.frames,
                w.matched,
                w.differing,
                w.differing_pixels,
                w.compared_pixels,
                w.selftests
            );
            if let Some(f) = &firsts[k] {
                let mut line = format!(
                    "  first difference: {} sb{sb} tic {}: {}",
                    item.lump, f.tic, f.what
                );
                if let Some(g) = &f.genuine {
                    for (side, px) in [("c", g.as_slice()), ("rust", f.rust.pixels())] {
                        let path = diff.join(format!("{}_sb{sb}_{}_{side}.png", item.lump, f.tic));
                        std::fs::write(
                            &path,
                            doom_render::png::encode_indexed(320, 200, px, &palette),
                        )
                        .expect("png");
                        line.push_str(&format!("\n  wrote {}", path.display()));
                    }
                }
                println!("{line}");
                reports.push(line);
            }
            let t = totals.entry(sb).or_default();
            t.0.frames += c.frames;
            t.0.matched += c.matched;
            t.0.mismatched += c.mismatched;
            t.0.non_comparable += c.non_comparable;
            t.0.refused += c.refused;
            t.0.counter_mismatches += c.counter_mismatches;
            for (r, n) in c.refusals {
                *t.0.refusals.entry(r).or_default() += n;
            }
            t.1.frames += w.frames;
            t.1.matched += w.matched;
            t.1.differing += w.differing;
            t.1.differing_pixels += w.differing_pixels;
            t.1.compared_pixels += w.compared_pixels;
            t.1.selftests += w.selftests;
            t.0.flag_disagreements += c.flag_disagreements;
        }
    }
    for (sb, (c, w)) in &totals {
        println!(
            "total sb{sb}: frames {} matched {} mismatched {} non-comparable {} refused {} {:?} counter-mismatches {} flag-disagreements {} raw: compared {} matched {} differing {} ({} pixels; {} pixels compared, {:.1}% of the 64,000 per frame; self-tests {})",
            c.frames,
            c.matched,
            c.mismatched,
            c.non_comparable,
            c.refused,
            c.refusals,
            c.counter_mismatches,
            c.flag_disagreements,
            w.frames,
            w.matched,
            w.differing,
            w.differing_pixels,
            w.compared_pixels,
            100.0 * w.compared_pixels as f64 / (w.frames.max(1) as f64 * 64_000.0),
            w.selftests
        );
    }
}
