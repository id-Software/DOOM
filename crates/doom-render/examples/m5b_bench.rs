//! M5b timing mode (recorded, never gated). Replays committed rooms through the simulation and,
//! per frame, times `Display::observe` plus `Display::draw`, `Display::draw` alone, and M5a's
//! `Renderer::observe` plus `render` (the 3D view without status bar and HUD) on the same
//! frames. The simulation advance is outside every timer; `Instant` is a monotonic clock that
//! never touches simulation or rendering outcomes.
//!
//! ```sh
//! cargo run -p doom-render --release --example m5b_bench -- [--rooms M5MSG,M5PALRD] [--screenblocks 10,11] [--warmup 1]
//! ```
//! One warm-up replay, then one measured replay per room and screenblocks. One line per case
//! and series: frames, total, mean, median, 95th percentile and maximum, in microseconds.
#[path = "../tests/common/mod.rs"]
mod common;
#[path = "../tests/m5b/mod.rs"]
mod m5b;

use doom_render::{Detail, Display, DisplayConfig, Frame, RenderConfig, Renderer};
use std::time::Instant;

fn us(ns: u64) -> f64 {
    ns as f64 / 1000.0
}

fn report(room: &str, sb: u8, series: &str, mut v: Vec<u64>) {
    v.sort_unstable();
    let n = v.len();
    let at = |p: usize| v[((n - 1) * p).div_ceil(100)];
    let total: u64 = v.iter().sum();
    println!(
        "{room} sb{sb} {series}: frames {n}, total us {:.0}, mean {:.1}, median {:.1}, p95 {:.1}, max {:.1}",
        us(total),
        us(total) / n as f64,
        us(at(50)),
        us(at(95)),
        us(v[n - 1]),
    );
}

fn replay(item: &m5b::Item, sb: u8, print: bool) {
    let (res, level, mut sim, commands) = m5b::load(item);
    let mut display = Display::new(
        res.clone(),
        level.clone(),
        DisplayConfig { screenblocks: sb },
    )
    .expect("display");
    display.begin_level(&sim).expect("level start");
    let cfg = RenderConfig {
        screenblocks: sb,
        detail: Detail::High,
    };
    let mut renderer = Renderer::new(res, level, cfg).expect("renderer");
    renderer.begin_level(&sim).expect("level start");
    let (mut frame, mut frame2) = (Frame::new(), Frame::new());
    let (mut obs_draw, mut draw, mut render) = (Vec::new(), Vec::new(), Vec::new());
    for (tic, cmd) in commands.iter().enumerate() {
        let mut inputs: [_; 4] = Default::default();
        inputs[0] = *cmd;
        if sim.advance(inputs).is_err() {
            break;
        }
        let t = Instant::now();
        let observed = display.observe(&sim, cmd);
        let t1 = t.elapsed().as_nanos() as u64;
        if observed.is_err() {
            eprintln!("{}: observe refused at tic {tic}", item.lump);
            break;
        }
        let t = Instant::now();
        let drawn = display.draw(&sim, &mut frame);
        let t2 = t.elapsed().as_nanos() as u64;
        let t = Instant::now();
        let observed = renderer.observe(&sim);
        let rendered = observed.map(|_| renderer.render(&sim, &mut frame2));
        let t3 = t.elapsed().as_nanos() as u64;
        if drawn.is_err() || !matches!(rendered, Ok(Ok(_))) {
            eprintln!("{}: refused at tic {tic}", item.lump);
            break;
        }
        obs_draw.push(t1 + t2);
        draw.push(t2);
        render.push(t3);
    }
    if print && !draw.is_empty() {
        report(&item.lump, sb, "display observe+draw", obs_draw);
        report(&item.lump, sb, "display draw", draw);
        report(&item.lump, sb, "renderer observe+render", render);
    }
}

fn main() {
    let mut rooms: Vec<String> = ["M5MSG", "M5PALRD", "M5PALBN", "M5STDIG", "M5EVIL"]
        .map(String::from)
        .into();
    let (mut blocks, mut warmup) = (vec![10u8, 11], 1usize);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let v = args.next().unwrap_or_else(|| panic!("{a}: missing value"));
        match a.as_str() {
            "--rooms" => rooms = v.split(',').map(str::to_uppercase).collect(),
            "--screenblocks" => blocks = v.split(',').map(|s| s.parse().expect("sb")).collect(),
            "--warmup" => warmup = v.parse().expect("warmup"),
            _ => panic!("usage: m5b_bench [--rooms A,B] [--screenblocks 10,11] [--warmup N]"),
        }
    }
    let items = m5b::items();
    for room in &rooms {
        let item = items
            .iter()
            .find(|i| &i.lump == room)
            .unwrap_or_else(|| panic!("{room}: no such item"));
        for &sb in &blocks {
            for _ in 0..warmup {
                replay(item, sb, false);
            }
            replay(item, sb, true);
        }
    }
}
