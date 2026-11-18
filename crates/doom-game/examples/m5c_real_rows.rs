//! M5c step 5b: one demo on one map through the app's real pipeline under the new contract,
//! writing every row the genuine runners write, for `tools/local/m5c_real_full.py` (the 61 real
//! Episode 1 runs, local only) and for any later session tool.
//!
//! [`GameData::load_with_sound`] over the given lump list (wire 25, the sound session, the display
//! under `RandomContract::WithSound`), then [`replay`] of the demo at each `--screenblocks`: per
//! tic the simulation, the sound session and the display run exactly as a played session runs
//! them (`FrameSource::tic`). From each [`TicReport`] it writes, in the genuine formats:
//!
//! - `--sound-out`: the sound-logic stream (DOOM-M5C-SOUND-v1, the M5c note §3.1.4, as
//!   `tools/reference/m5c_driver.c` writes it): S, T, K, L, C and R rows, built by the row writer
//!   the doom-sound gate uses (`crates/doom-sound/tests/common/rows.rs`, included by path). Actor
//!   origins are written `A<slot>:<generation>` (the genuine stream writes thinker serials; a
//!   comparator matches the two by one bijection). The R row's index after `P_Ticker` is the
//!   session's (`TicSound::rndindex`), after `D_Display` the app's reported index; the index after
//!   `ST_Ticker` is the reported one less the wipe's 320 on the first frame
//!   (`doom_render::WIPE_DRAWS`, f_wipe.c:159-166), the only draw between them (d_main.c:327-340).
//!   For a start without a placement (a NULL origin or the listener's own body) the record has no
//!   listener position, so the three listener columns are `-` (the step 4b report's SIM NEEDS);
//! - `--frames SB=PATH`: the frame stream's golden rows (DOOM-M5C-FRAMES-v1 as
//!   `tools/reference/m5c_reference.py` normalises it): F (the whole-frame SHA-256, or `undefined`
//!   and the digest with the flagged columns zeroed where the renderer flagged columns; the last
//!   column `0`), U (`tic,x,-`: the Rust tracker has no origin bits), M, P, Q (the digest with the
//!   face rectangle zeroed, the union of the 42 face patches as the genuine driver computes it,
//!   tools/reference/m5c_driver.c:984-993 and 1555-1556; cumulative sound draws; wipe draws 320)
//!   and X rows
//!   (written before the Q row, as the genuine driver orders them);
//!   a frame whose switch parity the renderer reports unknown carries `parity` in place of its
//!   whole digest;
//! - `--mixer-out`: the genuine mixer driver's rows (DOOM-M5C-MIXER-v1,
//!   `tools/reference/m5c_mixer_driver.c`, without its device rows `d` and `q`): header, table
//!   and lump digests, one `s` row per started sound (the start echo), buffer rows with the
//!   driver's repeat compression, the drain after the last tic while a mixer channel is busy, and
//!   the stream digest;
//! - `--summary`: `key,value` lines: tics, ending, the T12 detector's first tic and firing count,
//!   the release starts, those whose next slot was busy, the later `S_UpdateSounds` reads of a
//!   release origin and those with a busy next slot, buffers, and whether every screen size gave
//!   the same sound rows and buffers.
//!
//! The sound rows, the mixer rows and the T12 counts are those of the first `--screenblocks`; the
//! other screen sizes' rows are compared with them (`sound_screens_identical`).
//!
//! ```sh
//! cargo run -p doom-game --release --example m5c_real_rows -- --wad DOOM.WAD --demo DEMO.lmp \
//!     --screenblocks 10 --screenblocks 11 --frames 10=a.sb10.frames --frames 11=a.sb11.frames \
//!     --sound-out a.sound --mixer-out a.mixer --summary a.summary
//! ```
#[path = "../../doom-sound/tests/common/rows.rs"]
mod rows;

use doom_game::app::{DemoCommands, GameData, Host, TicReport, Timestamp, replay};
use doom_render::st_stuff::StatusGraphics;
use doom_render::{GameMode, LumpDirectory, Resources, SCREENHEIGHT, SCREENWIDTH, WIPE_DRAWS};
use doom_sim::Simulation;
use doom_sound::logic::{SoundOrigin, SoundRecord};
use doom_sound::mixer::buffer_bytes;
use doom_sound::session::Sound;
use doom_test_evidence::sha256;
use rows::{Carry, GoldenWriter, drain, stream_rows_with};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

/// The face rectangle of the genuine driver's Q row (x0, y0, x1, y1, upper bounds exclusive):
/// the union of the 42 face patches (ST_NUMFACES, st_stuff.c:110) drawn at ST_FACESX,
/// ST_FACESY = 143, 168 (st_stuff.c:1342-1347), each placed by its own offsets as
/// `V_DrawPatch` places it, computed as tools/reference/m5c_driver.c:984-993 computes it and
/// zeroed as m5c_driver.c:1555-1556 zeroes it.
fn face_rect(wads: &[&[u8]]) -> Res<[i32; 4]> {
    let mut dir = LumpDirectory::new();
    for w in wads {
        dir.add_wad(&doom_wad::Wad::parse(w, doom_wad::Limits::default())?)?;
    }
    let res = Resources::load(dir, GameMode::Registered)?;
    let gfx = StatusGraphics::load(&res)?;
    let mut r = [9999, 9999, -1, -1];
    for p in &gfx.faces {
        let h = &p.header;
        let (x0, y0) = (143 - i32::from(h.leftoffset), 168 - i32::from(h.topoffset));
        r[0] = r[0].min(x0);
        r[1] = r[1].min(y0);
        r[2] = r[2].max(x0 + i32::from(h.width));
        r[3] = r[3].max(y0 + i32::from(h.height));
    }
    Ok(r)
}

/// The frame with the face rectangle zeroed, rows and columns clipped to the screen
/// (tools/reference/m5c_driver.c:1555-1556).
fn noface(px: &[u8], r: [i32; 4]) -> Vec<u8> {
    let mut m = px.to_vec();
    let clip = |v: i32, hi: usize| usize::try_from(v.max(0)).unwrap_or(0).min(hi);
    if r[2] > 0 {
        for y in clip(r[1], SCREENHEIGHT)..clip(r[3], SCREENHEIGHT) {
            for x in clip(r[0], SCREENWIDTH)..clip(r[2], SCREENWIDTH) {
                m[y * SCREENWIDTH + x] = 0;
            }
        }
    }
    m
}

/// The T12 counts of a run (decision T12, the owner's ruling of 2026-11-18).
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
struct T12 {
    release_starts: u64,
    release_next_busy: u64,
    release_reads: u64,
    release_reads_next_busy: u64,
}

/// Everything one screen size's run collects.
struct Run {
    sound: String,
    frames: String,
    buffers: Vec<String>,
    golden: Option<GoldenWriter>,
    carry: Carry,
    snd_draws: u64,
    t12: T12,
    last: Option<Sound>,
    tics: u64,
    error: Option<String>,
    face: [i32; 4],
}

impl Run {
    fn new(golden: Option<GoldenWriter>, face: [i32; 4]) -> Self {
        Self {
            sound: String::new(),
            frames: String::new(),
            buffers: Vec::new(),
            golden,
            carry: Carry::default(),
            snd_draws: 0,
            t12: T12::default(),
            last: None,
            tics: 0,
            error: None,
            face,
        }
    }

    fn release_reads(&mut self, sim: &Simulation, session: &Sound) {
        for e in session.logic().channels().iter().flatten() {
            if let SoundOrigin::Button(slot) = e.origin {
                self.t12.release_reads += 1;
                match sim.button_release_xy(slot) {
                    Ok(xy) => self.t12.release_reads_next_busy += u64::from(xy != [0, 0]),
                    Err(e) => self.error = Some(format!("button_release_xy({slot}): {e:?}")),
                }
            }
        }
    }
}

impl Host for Run {
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
        let tic = r.tic;
        self.tics = tic + 1;
        let Some(s) = r.sound else {
            self.error = Some(format!("tic {tic}: no sound report"));
            return;
        };
        // The sound-logic stream.
        let records = r.simulation.sound_records().to_vec();
        let listener = r.simulation.sound_listener().map(|l| l.actor);
        for rec in &records {
            if let SoundRecord::Start {
                origin: SoundOrigin::Button(_),
                at,
                ..
            } = rec
            {
                self.t12.release_starts += 1;
                self.t12.release_next_busy += u64::from(at.is_some_and(|p| p.origin != [0, 0]));
            }
        }
        self.release_reads(r.simulation, s.session);
        let after_display = r.rndindex;
        let after_st = if tic == 0 {
            after_display.wrapping_sub((WIPE_DRAWS % 256) as u8)
        } else {
            after_display
        };
        let first_seq = self.carry.seq;
        for row in stream_rows_with(
            s.tic,
            &records,
            s.session,
            listener,
            &mut self.carry,
            after_st,
            after_display,
        ) {
            self.sound.push_str(&row);
            self.sound.push('\n');
        }
        self.snd_draws += u64::from(s.tic.draws);
        // The mixer rows.
        if let Some(g) = self.golden.as_mut() {
            g.starts(s.tic, first_seq);
            if let Some(b) = &s.tic.buffer {
                g.buffer(tic, b, s.session.mixer().mask());
            }
        }
        if let Some(b) = &s.tic.buffer {
            self.buffers.push(sha256(&buffer_bytes(b)));
        }
        self.last = Some(s.session.clone());
        // The frame rows.
        let (px, d) = (r.frame.pixels(), r.display);
        let v = &d.view;
        let w = &mut self.frames;
        let (whole, masked) = if v.switch_parity_unknown {
            ("parity".to_string(), "-".to_string())
        } else if v.flagged_columns.is_empty() {
            (sha256(px), "-".to_string())
        } else {
            let mut m = px.to_vec();
            for x in &v.flagged_columns {
                for y in 0..SCREENHEIGHT {
                    m[y * SCREENWIDTH + usize::from(*x)] = 0;
                }
            }
            ("undefined".to_string(), sha256(&m))
        };
        for x in &v.flagged_columns {
            let _ = writeln!(w, "U,{tic},{x},-");
        }
        let c = &v.counters;
        let _ = writeln!(
            w,
            "F,{tic},{whole},{masked},{},{},{},{},{},{},{},{},{},{},0",
            v.fuzzpos,
            c.mapped,
            v.flagged_columns.len(),
            c.visplanes,
            c.drawsegs,
            c.vissprites,
            c.openings,
            c.solidsegs_peak,
            c.drawseg_drops,
            u8::from(c.vissprite_drop)
        );
        if !v.newly_mapped.is_empty() {
            let lines: Vec<String> = v.newly_mapped.iter().map(u16::to_string).collect();
            let _ = writeln!(w, "M,{tic},{}", lines.join(" "));
        }
        let _ = writeln!(
            w,
            "P,{tic},{},{},{},{},{}",
            d.palette,
            d.palette_set.map_or(-1, i32::from),
            d.face,
            u8::from(d.message_on),
            r.rndindex
        );
        // The genuine driver writes the message taken before the Q row.
        if let Some(m) = r.message {
            let _ = writeln!(w, "X,{tic},{},{}", m.code, m.text);
        }
        let noface = if v.flagged_columns.is_empty() && !v.switch_parity_unknown {
            sha256(&noface(px, self.face))
        } else if v.switch_parity_unknown {
            "parity".to_string()
        } else {
            "undefined".to_string()
        };
        let [x0, y0, x1, y1] = self.face;
        let _ = writeln!(
            w,
            "Q,{tic},{noface},{},{WIPE_DRAWS},{x0},{y0},{x1},{y1}",
            self.snd_draws
        );
    }
}

fn main() -> Res<()> {
    let (mut wads, mut demo) = (Vec::new(), None);
    let (mut sbs, mut frames) = (Vec::<u8>::new(), BTreeMap::<u8, PathBuf>::new());
    let (mut sound_out, mut mixer_out, mut summary) = (None, None, None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut v = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wads.push(std::fs::read(v()?)?),
            "--demo" => demo = Some(std::fs::read(v()?)?),
            "--screenblocks" => sbs.push(v()?.parse()?),
            "--frames" => {
                let s = v()?;
                let (sb, p) = s.split_once('=').ok_or("--frames SB=PATH")?;
                frames.insert(sb.parse()?, PathBuf::from(p));
            }
            "--sound-out" => sound_out = Some(PathBuf::from(v()?)),
            "--mixer-out" => mixer_out = Some(PathBuf::from(v()?)),
            "--summary" => summary = Some(PathBuf::from(v()?)),
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    if sbs.is_empty() {
        sbs.push(10);
    }
    let d = demo.ok_or("--demo LMP is required")?;
    // A version-110 demo header: skill at byte 1, map at byte 3 (g_game.c G_BeginRecording).
    let (skill, map) = (
        *d.get(1).ok_or("short demo")?,
        *d.get(3).ok_or("short demo")?,
    );
    let refs: Vec<&[u8]> = wads.iter().map(Vec::as_slice).collect();
    let data = GameData::load_with_sound(&refs, GameMode::Registered, map, skill)?;
    let face = face_rect(&refs)?;
    let set = data.sound().ok_or("no sound data")?.set().clone();
    let commands = DemoCommands::parse(&d, map).map_err(|e| format!("{e:?}"))?;
    let mut out = String::new();
    let mut first: Option<Run> = None;
    let mut identical = true;
    for (k, &sb) in sbs.iter().enumerate() {
        let mut run = Run::new((k == 0).then(|| GoldenWriter::new(&set)), face);
        let ending = replay(&data, sb, &commands, &mut run);
        if let Some(e) = &run.error {
            return Err(format!("sb{sb}: {e}").into());
        }
        if let Some(p) = frames.get(&sb) {
            std::fs::write(p, &run.frames)?;
        }
        let _ = writeln!(out, "ending.{sb},{ending:?}");
        let _ = writeln!(out, "tics.{sb},{}", run.tics);
        match &first {
            None => first = Some(run),
            Some(f) => {
                identical &= f.sound == run.sound && f.buffers == run.buffers && f.t12 == run.t12;
            }
        }
    }
    let mut f = first.ok_or("no run")?;
    let det = f.last.as_ref().map(Sound::detector);
    let mut golden = f.golden.take().ok_or("no mixer writer")?;
    if let Some(s) = &f.last {
        drain(s, &mut golden);
    }
    let _ = writeln!(out, "tics,{}", f.tics);
    let _ = writeln!(out, "buffers,{}", f.buffers.len());
    let _ = writeln!(
        out,
        "detector_first_tic,{}",
        det.as_ref()
            .and_then(|d| d.layout_dependent_from())
            .map_or("-".to_string(), |t| t.to_string())
    );
    let _ = writeln!(
        out,
        "detector_firings,{}",
        det.as_ref().map_or(0, |d| d.firings())
    );
    let _ = writeln!(
        out,
        "detector_textures,{}",
        det.as_ref().map_or(0, |d| d.textures())
    );
    let _ = writeln!(out, "release_starts,{}", f.t12.release_starts);
    let _ = writeln!(out, "release_next_busy,{}", f.t12.release_next_busy);
    let _ = writeln!(out, "release_reads,{}", f.t12.release_reads);
    let _ = writeln!(
        out,
        "release_reads_next_busy,{}",
        f.t12.release_reads_next_busy
    );
    let _ = writeln!(out, "sound_draws,{}", f.snd_draws);
    let _ = writeln!(out, "sound_screens_identical,{}", u8::from(identical));
    if let Some(p) = sound_out {
        std::fs::write(p, &f.sound)?;
    }
    if let Some(p) = mixer_out {
        let mut text = golden.finish().join("\n");
        text.push('\n');
        std::fs::write(p, text)?;
    }
    match summary {
        Some(p) => std::fs::write(p, &out)?,
        None => print!("{out}"),
    }
    Ok(())
}
