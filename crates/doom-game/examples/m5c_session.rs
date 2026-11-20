//! M5c step 5c: one scripted session through the app's own code path under the new contract
//! (wire 25, the sound session, the display with sound), headless, for
//! `tools/reference/m5c_session.py`. The M5b example `m5b_session.rs` is left as it is.
//!
//! The data is loaded with [`GameData::load_with_sound`]; the script runs through [`App`] exactly
//! as the window drives it (platform-level events with timestamps on a fake monotonic clock,
//! delivered in frames of a fixed period, each frame followed by [`App::update`]), with a host
//! whose [`AudioSink`] records every buffer and every pause, resume and session start. Unlike the
//! M5b example the script does not stop at the session's end: a restart (`r` while paused or on
//! the end screen, decision S11/T15) begins a new session, a fresh process, which becomes a new
//! *part*. Each part is one session file: the host refuses a name it has already handed out, so
//! the second part is saved as `-2`, as on a disk.
//!
//! Writes to `--out`:
//!
//! - `partN/session.lmp`: the demo the app saved for part N;
//! - `partN/rows.txt`: per tic `E` (the events posted) and `T` (the executed command and the
//!   simulation's state digest), as the M5b example writes them (the input driver's script is
//!   built from these);
//! - `partN/frames.txt`: the frame rows in the genuine M5c runner's golden format (DOOM-M5C-FRAMES-v1
//!   as `tools/reference/m5c_reference.py` normalises it): F, U (`tic,x,-`), M, P, X, Q, exactly as
//!   `m5c_real_rows.rs` writes them (switch parity unknown: `parity` in place of the digests);
//! - `partN/sound.txt`: the sound-logic rows S, T, K, L, C, R (DOOM-M5C-SOUND-v1, the M5c note
//!   §3.1.4), by the doom-sound gate's row writer (`crates/doom-sound/tests/common/rows.rs`); actor
//!   origins are `A<slot>:<generation>` and a start without a placement has `-` listener columns;
//! - `partN/mixer.txt`: the genuine mixer driver's rows (DOOM-M5C-MIXER-v1 without its device rows),
//!   with the drain after the last tic, by the same writer;
//! - `partN/buffers.txt`: `b,K,TIC,SHA256` for every buffer the session handed to the sink;
//! - `log.txt`: every sink call and every reported tic in order (`tic,PART,TIC`, `buffer,K,SHA256`,
//!   `pause`, `resume`, `session_start`), the evidence that a pause runs no tic and mixes no buffer;
//! - `summary.txt`: `key,value` lines: parts, the ending, per part its tics, buffers, ending and the
//!   T12 detector's first tic and firings.
//!
//! Live equals recorded: each part's saved demo is replayed through [`replay`] and must give the
//! same E-less rows, frames, sound rows, mixer rows and buffers, or the example fails.
//!
//! Script lines (`#` comments) as `m5b_session.rs`: `TIME_MS key K down|up`, `TIME_MS motion DX
//! DY`, `TIME_MS button left|middle|right down|up`, `TIME_MS focus`, `TIME_MS end`; `frame US`;
//! `wad`, `map`, `skill` and `screenblocks` lines are read by the tool and skipped here.
#[path = "../../doom-sound/tests/common/rows.rs"]
mod rows;

use doom_game::app::{
    App, AudioSink, DemoCommands, GameData, Host, SessionConfig, TicReport, Timestamp, replay,
};
use doom_game::input::{Event, MouseButton, keys};
use doom_render::st_stuff::StatusGraphics;
use doom_render::{GameMode, LumpDirectory, Resources, SCREENHEIGHT, SCREENWIDTH, WIPE_DRAWS};
use doom_sound::logic::SoundRecord;
use doom_sound::lumps::SoundSet;
use doom_sound::mixer::{Buffer, buffer_bytes};
use doom_sound::session::Sound;
use doom_test_evidence::sha256;
use rows::{Carry, GoldenWriter, drain, stream_rows_with};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

/// The face rectangle of the genuine driver's Q row (x0, y0, x1, y1, upper bounds exclusive):
/// the union of the 42 face patches (ST_NUMFACES, st_stuff.c:110) at ST_FACESX, ST_FACESY = 143,
/// 168 (st_stuff.c:1342-1347), as tools/reference/m5c_driver.c:984-993 computes it.
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

/// The frame with the face rectangle zeroed, clipped to the screen
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

/// Everything one session (one part) collects.
struct Part {
    name: String,
    demo: Vec<u8>,
    rows: String,
    frames: String,
    sound: String,
    buffers: String,
    golden: GoldenWriter,
    carry: Carry,
    snd_draws: u64,
    last: Option<Sound>,
    tics: u64,
    nbuf: u64,
    error: Option<String>,
}

impl Part {
    fn new(name: &str, set: &SoundSet) -> Self {
        Self {
            name: name.to_string(),
            demo: Vec::new(),
            rows: String::new(),
            frames: String::new(),
            sound: String::new(),
            buffers: String::new(),
            golden: GoldenWriter::new(set),
            carry: Carry::default(),
            snd_draws: 0,
            last: None,
            tics: 0,
            nbuf: 0,
            error: None,
        }
    }

    /// The rows the replay of the part's demo must give again (no `E` rows: a replay posts no event).
    fn replayable(&self) -> Vec<String> {
        let rows: Vec<&str> = self.rows.lines().filter(|l| !l.starts_with("E,")).collect();
        vec![
            rows.join("\n"),
            self.frames.clone(),
            self.sound.clone(),
            self.buffers.clone(),
        ]
    }

    fn record(&mut self, r: &TicReport<'_>, face: [i32; 4]) {
        let (t, c) = (r.tic, r.executed);
        self.tics = t + 1;
        let w = &mut self.rows;
        for e in r.events {
            let _ = match *e {
                Event::KeyDown(k) => writeln!(w, "E,{t},keydown,{k},0,0"),
                Event::KeyUp(k) => writeln!(w, "E,{t},keyup,{k},0,0"),
                Event::Mouse { buttons, dx, dy } => writeln!(w, "E,{t},mouse,{buttons},{dx},{dy}"),
            };
        }
        let _ = writeln!(
            w,
            "T,{t},{},{},{},{},{:016x}",
            c.forward,
            c.side,
            c.angle_turn,
            c.buttons,
            r.simulation.snapshot().digest()
        );
        let Some(s) = r.sound else {
            self.error = Some(format!("tic {t}: no sound report"));
            return;
        };
        // The sound-logic stream; the R row's index after ST_Ticker is the reported one less the
        // wipe's draws on the first frame (f_wipe.c:159-166, d_main.c:327-340).
        let records: Vec<SoundRecord> = r.simulation.sound_records().to_vec();
        let listener = r.simulation.sound_listener().map(|l| l.actor);
        let after_display = r.rndindex;
        let after_st = if t == 0 {
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
        self.golden.starts(s.tic, first_seq);
        if let Some(b) = &s.tic.buffer {
            self.golden.buffer(t, b, s.session.mixer().mask());
            let _ = writeln!(
                self.buffers,
                "b,{},{t},{}",
                self.nbuf,
                sha256(&buffer_bytes(b))
            );
            self.nbuf += 1;
        }
        self.last = Some(s.session.clone());
        // The frame rows, as m5c_real_rows.rs writes them.
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
            let _ = writeln!(w, "U,{t},{x},-");
        }
        let k = &v.counters;
        let _ = writeln!(
            w,
            "F,{t},{whole},{masked},{},{},{},{},{},{},{},{},{},{},0",
            v.fuzzpos,
            k.mapped,
            v.flagged_columns.len(),
            k.visplanes,
            k.drawsegs,
            k.vissprites,
            k.openings,
            k.solidsegs_peak,
            k.drawseg_drops,
            u8::from(k.vissprite_drop)
        );
        if !v.newly_mapped.is_empty() {
            let lines: Vec<String> = v.newly_mapped.iter().map(u16::to_string).collect();
            let _ = writeln!(w, "M,{t},{}", lines.join(" "));
        }
        let _ = writeln!(
            w,
            "P,{t},{},{},{},{},{}",
            d.palette,
            d.palette_set.map_or(-1, i32::from),
            d.face,
            u8::from(d.message_on),
            r.rndindex
        );
        if let Some(m) = r.message {
            let _ = writeln!(w, "X,{t},{},{}", m.code, m.text);
        }
        let nf = if v.switch_parity_unknown {
            "parity".to_string()
        } else if v.flagged_columns.is_empty() {
            sha256(&noface(px, face))
        } else {
            "undefined".to_string()
        };
        let [x0, y0, x1, y1] = face;
        let _ = writeln!(
            w,
            "Q,{t},{nf},{},{WIPE_DRAWS},{x0},{y0},{x1},{y1}",
            self.snd_draws
        );
    }

    /// The mixer rows with the genuine driver's drain after the last tic; the writer is spent
    /// (a fresh one for `set` takes its place).
    fn mixer(&mut self, set: &SoundSet) -> String {
        let mut g = std::mem::replace(&mut self.golden, GoldenWriter::new(set));
        if let Some(s) = &self.last {
            drain(s, &mut g);
        }
        let mut text = g.finish().join("\n");
        text.push('\n');
        text
    }
}

/// The sink: every call in order, into the host's log.
#[derive(Default)]
struct LogSink {
    log: String,
    buffers: u64,
}

impl AudioSink for LogSink {
    fn buffer(&mut self, frames: &Buffer) {
        let _ = writeln!(
            self.log,
            "buffer,{},{}",
            self.buffers,
            sha256(&buffer_bytes(frames))
        );
        self.buffers += 1;
    }
    fn pause(&mut self) {
        self.log.push_str("pause\n");
    }
    fn resume(&mut self) {
        self.log.push_str("resume\n");
    }
    fn session_start(&mut self) {
        self.log.push_str("session_start\n");
    }
}

/// The headless host: one part per claimed session file.
struct Session {
    set: Arc<SoundSet>,
    face: [i32; 4],
    parts: Vec<Part>,
    sink: LogSink,
}

impl Host for Session {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, name: &str) -> bool {
        self.parts.iter().any(|p| p.name == name)
    }
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        if self.exists(name) {
            return Ok(false);
        }
        let mut p = Part::new(name, &self.set);
        p.demo = bytes.to_vec();
        self.parts.push(p);
        Ok(true)
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        match self.parts.iter_mut().find(|p| p.name == name) {
            Some(p) => {
                p.demo = bytes.to_vec();
                Ok(())
            }
            None => Err(format!("{name} was never claimed")),
        }
    }
    fn tic(&mut self, r: &TicReport<'_>) {
        let n = self.parts.len();
        let _ = writeln!(self.sink.log, "tic,{},{}", n.saturating_sub(1), r.tic);
        match self.parts.last_mut() {
            Some(p) => p.record(r, self.face),
            None => self
                .sink
                .log
                .push_str("error,a tic before any session file\n"),
        }
    }
    fn audio(&mut self) -> Option<&mut dyn AudioSink> {
        Some(&mut self.sink)
    }
}

fn key(name: &str) -> Res<u8> {
    Ok(match name {
        "up" => keys::KEY_UPARROW,
        "down" => keys::KEY_DOWNARROW,
        "left" => keys::KEY_LEFTARROW,
        "right" => keys::KEY_RIGHTARROW,
        "fire" => keys::KEY_RCTRL,
        "use" => b' ',
        "speed" => keys::KEY_RSHIFT,
        "strafe" => keys::KEY_RALT,
        "esc" => keys::KEY_ESCAPE,
        "pause" => keys::KEY_PAUSE,
        hex if hex.starts_with("0x") => u8::from_str_radix(&hex[2..], 16)?,
        c if c.len() == 1 => c.as_bytes()[0],
        _ => return Err(format!("unknown key {name}").into()),
    })
}

/// Runs a script through the app to its `end` line (restarts included); returns the ending.
fn run_script(app: &mut App, host: &mut Session, text: &str) -> Res<String> {
    let mut frame_us = 16_667u64;
    let mut lines: Vec<(u64, Vec<&str>)> = Vec::new();
    for l in text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
    {
        let w: Vec<&str> = l.split_whitespace().collect();
        match w.as_slice() {
            [] | ["wad" | "map" | "skill" | "screenblocks", ..] => {}
            ["frame", us] => frame_us = us.parse()?,
            [ms, rest @ ..] => lines.push((ms.parse::<u64>()? * 1000, rest.to_vec())),
        }
    }
    let end = lines.iter().find(|(_, w)| w == &["end"]).map(|l| l.0);
    let end = end.ok_or("the script has no end line")?;
    app.update(0, host);
    let mut next = lines.iter().peekable();
    let mut now = 0;
    while now < end {
        now = (now + frame_us).min(end);
        while let Some((_, w)) = next.next_if(|(at, _)| *at <= now) {
            let down = |s: &str| -> Res<bool> {
                match s {
                    "down" => Ok(true),
                    "up" => Ok(false),
                    _ => Err(format!("{s} is neither down nor up").into()),
                }
            };
            match w.as_slice() {
                ["key", k, d] => app.key(key(k)?, down(d)?, host),
                ["motion", x, y] => app.mouse_motion(x.parse()?, y.parse()?),
                ["button", b, d] => {
                    let b = match *b {
                        "left" => MouseButton::Left,
                        "middle" => MouseButton::Middle,
                        "right" => MouseButton::Right,
                        _ => return Err(format!("unknown button {b}").into()),
                    };
                    app.mouse_button(b, down(d)?);
                }
                ["focus"] => app.focus_lost(),
                ["end"] => {}
                _ => return Err(format!("bad script line {w:?}").into()),
            }
        }
        app.update(now, host);
        if app.quit_requested() {
            break;
        }
    }
    let ending = format!("{:?}", app.mode());
    if !app.quit_requested() {
        app.quit(host);
    }
    Ok(ending)
}

fn main() -> Res<()> {
    let (mut wads, mut out, mut script) = (Vec::new(), None, None);
    let (mut map, mut skill, mut sb) = (1u8, 2u8, 10u8);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut v = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wads.push(std::fs::read(v()?)?),
            "--out" => out = Some(PathBuf::from(v()?)),
            "--script" => script = Some(std::fs::read_to_string(v()?)?),
            "--map" => map = v()?.parse()?,
            "--skill" => skill = v()?.parse()?,
            "--screenblocks" => sb = v()?.parse()?,
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let out = out.ok_or("--out DIR is required")?;
    let text = script.ok_or("--script FILE is required")?;
    let refs: Vec<&[u8]> = wads.iter().map(Vec::as_slice).collect();
    let data = GameData::load_with_sound(&refs, GameMode::Registered, map, skill)?;
    let set = data.sound().ok_or("no sound data")?.set().clone();
    let face = face_rect(&refs)?;
    let mut host = Session {
        set: set.clone(),
        face,
        parts: Vec::new(),
        sink: LogSink::default(),
    };
    let cfg = SessionConfig {
        screenblocks: sb,
        ..SessionConfig::default()
    };
    let mut app = App::new(data.clone(), cfg, &mut host);
    let ending = run_script(&mut app, &mut host, &text)?;
    let mut summary = format!("parts,{}\nending,{ending}\n", host.parts.len());
    for (n, p) in host.parts.iter_mut().enumerate() {
        if let Some(e) = &p.error {
            return Err(format!("part {n}: {e}").into());
        }
        // live equals recorded: the replay of the part's saved demo gives the same rows
        let commands = DemoCommands::parse(&p.demo, map).map_err(|e| format!("{e:?}"))?;
        let mut again = Session {
            set: set.clone(),
            face,
            parts: vec![Part::new("replay", &set)],
            sink: LogSink::default(),
        };
        let replay_ending = replay(&data, sb, &commands, &mut again);
        let mixer = p.mixer(&set);
        let a = &mut again.parts[0];
        if a.replayable() != p.replayable() || a.mixer(&set) != mixer {
            return Err(format!(
                "part {n}: the replay of the saved demo differs from the live session"
            )
            .into());
        }
        let det = p.last.as_ref().map(Sound::detector);
        let _ = writeln!(summary, "part{n}.tics,{}", p.tics);
        let _ = writeln!(summary, "part{n}.buffers,{}", p.nbuf);
        let _ = writeln!(summary, "part{n}.replay_ending,{replay_ending:?}");
        let _ = writeln!(
            summary,
            "part{n}.detector_first_tic,{}",
            det.and_then(|d| d.layout_dependent_from())
                .map_or("-".to_string(), |t| t.to_string())
        );
        let _ = writeln!(
            summary,
            "part{n}.detector_firings,{}",
            det.map_or(0, |d| d.firings())
        );
        let dir = out.join(format!("part{n}"));
        std::fs::create_dir_all(&dir)?;
        write(&dir, "session.lmp", &p.demo)?;
        write(&dir, "rows.txt", p.rows.as_bytes())?;
        write(&dir, "frames.txt", p.frames.as_bytes())?;
        write(&dir, "sound.txt", p.sound.as_bytes())?;
        write(&dir, "mixer.txt", mixer.as_bytes())?;
        write(&dir, "buffers.txt", p.buffers.as_bytes())?;
    }
    write(&out, "log.txt", host.sink.log.as_bytes())?;
    write(&out, "summary.txt", summary.as_bytes())?;
    let tics: u64 = host.parts.iter().map(|p| p.tics).sum();
    println!(
        "session: parts {} tics {tics} ending {ending}",
        host.parts.len()
    );
    Ok(())
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> Res<()> {
    std::fs::write(dir.join(name), bytes)?;
    Ok(())
}
