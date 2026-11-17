//! M5a local viewer (decision R16): replays a demo script through the Rust simulation and the
//! Rust renderer and writes an animated GIF of the 3D view, one frame per tic, in the palette 0
//! of the WAD in use (palette flashes belong to M5b). No window, no input, no sound.
//!
//! ```sh
//! # a real map (local only; the output is IWAD-derived and must never be committed)
//! cargo run -p doom-render --release --example m5a_view -- --iwad ~/scratch/m5a/iwad/DOOM.WAD \
//!     --lmp tests/reference/e1m1_scripts/WALK.lmp --out /mnt/d/DoomRenders/e1m1_walk.gif
//! # a committed synthetic room (no IWAD)
//! cargo run -p doom-render --release --example m5a_view -- --room M5AMP --out ~/scratch/m5a/m5amp.gif
//! ```
//!
//! Options: `--screenblocks 9|10|11` (default 11), `--from TIC` and `--to TIC` (inclusive
//! frame range of the tic numbers the harness uses, default the whole script), `--scale N`
//! (integer nearest-neighbour enlargement, default 1), `--fps N` (default 35; GIF delays are
//! whole hundredths of a second, so the rounding alternates 2, 3, 3, 3, 3, 3, 3 cs at 35 fps
//! and a second of tics lasts exactly a second). The replay stops at the end of the script,
//! when the simulation stops advancing (the exit), or at a renderer refusal (reported).
//! `--raw DIR` also writes each rendered frame as `DIR/frame_NNNNN.raw` (tic number NNNNN, the
//! 64,000 palette indices row by row, the genuine runner's `--frames-raw` form); with `--raw`
//! the GIF `--out` is optional. `tools/media/m5a_readme_visual.py` uses it.
#[path = "../tests/common/mod.rs"]
mod common;

use doom_render::gif::{GifWriter, delay_cs, scale_nearest};
use doom_render::{Detail, Frame, RenderConfig, Renderer, SCREENHEIGHT, SCREENWIDTH};
use std::path::PathBuf;

fn main() {
    let (mut iwad, mut lmp, mut out, mut raw): (
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
    ) = (None, None, None, None);
    let mut room: Option<String> = None;
    let (mut screenblocks, mut from, mut to, mut scale, mut fps) =
        (11u8, 0i64, i64::MAX, 1usize, 35u32);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{a}: missing value"));
        match a.as_str() {
            "--iwad" => iwad = Some(PathBuf::from(value())),
            "--lmp" => lmp = Some(PathBuf::from(value())),
            "--room" => room = Some(value().to_uppercase()),
            "--out" => out = Some(PathBuf::from(value())),
            "--raw" => raw = Some(PathBuf::from(value())),
            "--screenblocks" => screenblocks = value().parse().expect("screenblocks"),
            "--from" => from = value().parse().expect("from"),
            "--to" => to = value().parse().expect("to"),
            "--scale" => scale = value().parse().expect("scale"),
            "--fps" => fps = value().parse().expect("fps"),
            _ => usage(),
        }
    }
    if out.is_none() && raw.is_none() {
        usage()
    }
    if let Some(dir) = &raw {
        std::fs::create_dir_all(dir).expect("raw directory");
    }
    if scale == 0 || fps == 0 || scale > 8 {
        eprintln!("--scale must be 1-8 and --fps at least 1");
        std::process::exit(2);
    }
    let item = match (iwad, lmp, room) {
        (Some(wad), Some(lmp), None) => {
            let lump = lmp
                .file_stem()
                .expect("demo name")
                .to_string_lossy()
                .to_uppercase();
            common::Item {
                lump,
                wad,
                lmp,
                room: false,
                gfx: false,
            }
        }
        (None, None, Some(name)) => common::items()
            .into_iter()
            .find(|i| i.lump == name)
            .unwrap_or_else(|| panic!("{name}: no such synthetic item")),
        _ => usage(),
    };
    let mut run = common::load(&item);
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
    let palette = renderer
        .resources()
        .palette
        .clone()
        .expect("the WAD has no PLAYPAL");
    let (w, h) = (SCREENWIDTH * scale, SCREENHEIGHT * scale);
    let mut gif = GifWriter::new(w as u16, h as u16, &palette);
    let mut frame = Frame::new();
    let (mut written, mut stop) = (0u64, String::from("the end of the script"));
    for index in 0..run.commands.len() {
        let tic = index as i64;
        if tic > to {
            stop = format!("--to {to}");
            break;
        }
        if let Err(e) = common::advance(&mut run, index) {
            stop = format!("the simulation stopped advancing at tic {tic}: {e:?}");
            break;
        }
        // Every tic is observed, rendered or not (the render state follows the simulation).
        if let Err(e) = renderer.observe(&run.sim) {
            stop = format!("observe failed at tic {tic}: {e:?}");
            break;
        }
        if tic < from {
            continue;
        }
        if let Err(e) = renderer.render(&run.sim, &mut frame) {
            stop = format!("the renderer refused tic {tic}: {e:?}");
            break;
        }
        if let Some(dir) = &raw {
            std::fs::write(dir.join(format!("frame_{tic:05}.raw")), frame.pixels())
                .expect("write raw frame");
        }
        let delay = delay_cs(written, fps);
        if scale == 1 {
            gif.frame(frame.pixels(), delay);
        } else {
            gif.frame(
                &scale_nearest(SCREENWIDTH, SCREENHEIGHT, frame.pixels(), scale),
                delay,
            );
        }
        written += 1;
    }
    let bytes = gif.finish();
    let name = out.as_ref().map_or_else(
        || String::from("(no GIF)"),
        |out| {
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir).expect("output directory");
            }
            std::fs::write(out, &bytes).expect("write GIF");
            out.display().to_string()
        },
    );
    eprintln!(
        "{name}: {} frames, {w}x{h}, screenblocks {screenblocks}, {} bytes, stopped at {stop}",
        written,
        bytes.len()
    );
}

fn usage() -> ! {
    eprintln!(
        "usage: m5a_view (--iwad WAD --lmp DEMO | --room NAME) [--out FILE.gif] [--raw DIR] [--screenblocks 9|10|11] [--from TIC] [--to TIC] [--scale 1-8] [--fps N]"
    );
    std::process::exit(2);
}
