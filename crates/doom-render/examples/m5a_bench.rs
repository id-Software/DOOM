//! M5a timing mode (decision R15: performance is recorded, never gated). Replays one script
//! through the Rust simulation and renderer and times, per tic, the simulation advance and the
//! render call separately. Observe, hashing and I/O are outside both timers; `Instant` only
//! reads the clock and never touches simulation or rendering outcomes.
//!
//! ```sh
//! cargo run -p doom-render --release --example m5a_bench -- --room M5PLANES [--screenblocks 11,10] [--reps 5] [--warmup 1]
//! cargo run -p doom-render --release --example m5a_bench -- --iwad ~/scratch/m5a/iwad/DOOM.WAD --lmp tests/reference/e1m1_scripts/WALK.lmp
//! ```
//!
//! Each configuration runs `--warmup` unmeasured replays, then `--reps` measured replays on a
//! freshly loaded simulation. It prints one line per configuration: frames per replay, the
//! render median, p95 and max in microseconds over all measured frames of all replays, the
//! advance median, p95 and max per tic, and the per-replay render medians (the run-to-run
//! variation). A replay stops at the end of the script or at a refusal (reported; refused tics
//! are not timed).
#[path = "../tests/common/mod.rs"]
mod common;

use doom_render::{Detail, Frame, RenderConfig, Renderer};
use std::path::PathBuf;
use std::time::Instant;

fn stats(v: &mut [u64]) -> (u64, u64, u64) {
    v.sort_unstable();
    if v.is_empty() {
        return (0, 0, 0);
    }
    let at = |p: usize| v[((v.len() - 1) * p).div_ceil(100)];
    (at(50), at(95), v[v.len() - 1])
}

/// One replay: (render ns per frame, advance ns per tic, stop reason).
fn replay(item: &common::Item, screenblocks: u8) -> (Vec<u64>, Vec<u64>, String) {
    let mut run = common::load(item);
    let mut renderer = Renderer::new(
        run.res.clone(),
        run.level.clone(),
        RenderConfig {
            screenblocks,
            detail: Detail::High,
        },
    )
    .unwrap_or_else(|e| panic!("configuration: {e:?}"));
    renderer.begin_level(&run.sim).expect("level start");
    let mut frame = Frame::new();
    let (mut render, mut advance) = (Vec::new(), Vec::new());
    let mut stop = String::from("the end of the script");
    for index in 0..run.commands.len() {
        let t = Instant::now();
        let r = common::advance(&mut run, index);
        let adv = t.elapsed().as_nanos() as u64;
        if let Err(e) = r {
            stop = format!("the simulation stopped advancing at tic {index}: {e:?}");
            break;
        }
        advance.push(adv);
        if let Err(e) = renderer.observe(&run.sim) {
            stop = format!("observe failed at tic {index}: {e:?}");
            break;
        }
        let t = Instant::now();
        let r = renderer.render(&run.sim, &mut frame);
        let ns = t.elapsed().as_nanos() as u64;
        match r {
            Ok(_) => render.push(ns),
            Err(e) => {
                stop = format!("the renderer refused tic {index}: {e:?}");
                break;
            }
        }
    }
    (render, advance, stop)
}

fn main() {
    let (mut iwad, mut lmp, mut room): (Option<PathBuf>, Option<PathBuf>, Option<String>) =
        (None, None, None);
    let (mut blocks, mut reps, mut warmup) = (vec![11u8, 10], 5usize, 1usize);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{a}: missing value"));
        match a.as_str() {
            "--iwad" => iwad = Some(PathBuf::from(value())),
            "--lmp" => lmp = Some(PathBuf::from(value())),
            "--room" => room = Some(value().to_uppercase()),
            "--screenblocks" => {
                blocks = value()
                    .split(',')
                    .map(|s| s.parse().expect("screenblocks"))
                    .collect()
            }
            "--reps" => reps = value().parse().expect("reps"),
            "--warmup" => warmup = value().parse().expect("warmup"),
            _ => usage(),
        }
    }
    let item = match (iwad, lmp, room) {
        (Some(wad), Some(lmp), None) => common::Item {
            lump: lmp
                .file_stem()
                .expect("demo name")
                .to_string_lossy()
                .to_uppercase(),
            wad,
            lmp,
            room: false,
            gfx: false,
        },
        (None, None, Some(name)) => common::items()
            .into_iter()
            .find(|i| i.lump == name)
            .unwrap_or_else(|| panic!("{name}: no such synthetic item")),
        _ => usage(),
    };
    if reps == 0 {
        usage()
    }
    for sb in blocks {
        for _ in 0..warmup {
            replay(&item, sb);
        }
        let (mut all_r, mut all_a, mut medians) = (Vec::new(), Vec::new(), Vec::new());
        let (mut frames, mut stop) = (0, String::new());
        for _ in 0..reps {
            let (r, a, s) = replay(&item, sb);
            frames = r.len();
            stop = s;
            medians.push(stats(&mut r.clone()).0 / 1000);
            all_r.extend(r);
            all_a.extend(a);
        }
        let (rm, rp, rx) = stats(&mut all_r);
        let (am, ap, ax) = stats(&mut all_a);
        println!(
            "{} sb{sb}: frames {frames}, reps {reps}, render us median {rm_} p95 {rp_} max {rx_}, advance us median {am_} p95 {ap_} max {ax_}, per-rep render medians us {medians:?}, stopped at {stop}",
            item.lump,
            rm_ = rm / 1000,
            rp_ = rp / 1000,
            rx_ = rx / 1000,
            am_ = am / 1000,
            ap_ = ap / 1000,
            ax_ = ax / 1000,
        );
    }
}

fn usage() -> ! {
    eprintln!(
        "usage: m5a_bench (--iwad WAD --lmp DEMO | --room NAME) [--screenblocks 11,10] [--reps N] [--warmup N]"
    );
    std::process::exit(2);
}
