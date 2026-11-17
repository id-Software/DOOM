//! M5b step 10: raw frames and palette numbers of a recorded demo, for the README animation
//! (`tools/media/m5b_readme_visual.py`).
//!
//! `--wad FILE`... `--demo LMP --screenblocks N --out DIR` replays the demo through the app's own
//! frame source ([`replay`]: the simulation and the composed `Display`) and writes, per tic,
//! `DIR/frame_NNNNN.raw` (320x200 palette indices, row-major, the same name and layout as the
//! genuine runner's `--frames-raw`) and one line `tic,palette` per tic in `DIR/palettes.txt`.
//! Frames are identical to the genuine engine under the declared sound-free, wipe-free contract
//! (decision S15).

use doom_game::app::{GameData, Host, TicReport, Timestamp, replay};
use doom_render::GameMode;
use doom_sim::demo::parse_demo_episode1_v2;
use std::fmt::Write as _;
use std::path::PathBuf;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

struct Dump {
    out: PathBuf,
    palettes: String,
    error: Option<std::io::Error>,
}

impl Host for Dump {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn tic(&mut self, r: &TicReport<'_>) {
        let path = self.out.join(format!("frame_{:05}.raw", r.tic));
        if let Err(e) = std::fs::write(path, r.frame.pixels()) {
            self.error.get_or_insert(e);
        }
        let _ = writeln!(self.palettes, "{},{}", r.tic, r.display.palette);
    }
}

fn main() -> Res<()> {
    let (mut wads, mut out, mut demo, mut sb) = (Vec::new(), None, None, 10u8);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut v = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wads.push(std::fs::read(v()?)?),
            "--demo" => demo = Some(std::fs::read(v()?)?),
            "--out" => out = Some(PathBuf::from(v()?)),
            "--screenblocks" => sb = v()?.parse()?,
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let out = out.ok_or("--out DIR is required")?;
    let d = demo.ok_or("--demo LMP is required")?;
    let (skill, map) = (
        *d.get(1).ok_or("short demo")?,
        *d.get(3).ok_or("short demo")?,
    );
    std::fs::create_dir_all(&out)?;
    let refs: Vec<&[u8]> = wads.iter().map(Vec::as_slice).collect();
    let data = GameData::load(&refs, GameMode::Registered, map, skill)?;
    let commands = parse_demo_episode1_v2(&d, map)
        .map_err(|e| format!("{e:?}"))?
        .1
        .commands;
    let mut host = Dump {
        out: out.clone(),
        palettes: String::new(),
        error: None,
    };
    let ending = replay(&data, sb, &commands, &mut host);
    if let Some(e) = host.error {
        return Err(e.into());
    }
    std::fs::write(out.join("palettes.txt"), &host.palettes)?;
    println!(
        "frames: {} ending {ending:?}",
        host.palettes.lines().count()
    );
    Ok(())
}
