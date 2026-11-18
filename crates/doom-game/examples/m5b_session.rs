//! M5b step 7: one session through the app's own code path, headless, for
//! `tools/reference/m5b_session.py` (M5b investigation §2.3).
//!
//! With `--script`, a scripted session runs through [`App`] exactly as the window drives it:
//! platform-level events with timestamps on a fake monotonic clock, delivered in frames of a
//! fixed period, each frame followed by [`App::update`]; the event layer, `G_BuildTiccmd`, the
//! demo round trip, the recorder, the simulation and the composed display all run as in play.
//! The saved demo is then replayed through [`replay`] and must give the same rows (the state
//! the app reached equals the replay of its own demo). With `--demo`, a recorded session (for
//! example one written by doom-play) is replayed through [`replay`] only. With `--commands`,
//! tic commands at full resolution (one line `forward side angleturn buttons` per tic, step 7b)
//! are fed to [`replay_full_resolution`] as they are, for `tools/reference/m5b_inject.py`; no demo is written.
//! `--unrecorded` plays `--script` with [`SessionConfig::unrecorded_full_turning`] (S3 (ii)): the
//! app must save nothing, and the replay of its executed commands ([`replay_full_resolution`])
//! must give the same rows.
//!
//! Writes to `--out`: `session.lmp` (the demo the app saved), `rows.txt` (per tic `E` the events
//! posted, `T` the executed command and the state digest, `F` the frame digest, the digest with
//! the flagged columns zeroed, those columns and the parity flag, `P` and `X` as the genuine
//! runner's frame stream) and, with `--captures`, `captures/tic_NNNNN.bin`. Frames are
//! identical to the genuine engine under the declared sound-free, wipe-free contract (decision S15),
//! with the evidence's two exclusions: 300 frames compared only on a masked digest and 70 not
//! compared (counts asserted in crates/doom-render/tests/m5b_display.rs).
//!
//! Script lines (`#` comments): `TIME_MS key K down|up` (K: up, down, left, right, fire, use,
//! speed, strafe, esc, pause, one character or 0xNN), `TIME_MS motion DX DY`, `TIME_MS button
//! left|middle|right down|up`, `TIME_MS focus`, `TIME_MS end`; `frame US` sets the frame period
//! (default 16,667 µs).

use doom_game::app::{
    App, DemoCommands, GameData, Host, Mode, SessionConfig, TicReport, Timestamp, replay,
    replay_full_resolution,
};
use doom_game::input::{Event, MouseButton, keys};
use doom_render::{GameMode, SCREENWIDTH};
use doom_sim::TicCommand;
use std::fmt::Write as _;
use std::path::PathBuf;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

/// Collects the rows (and captures) of every reported tic, and the saved demo.
#[derive(Default)]
struct Rows {
    text: String,
    captures: Option<PathBuf>,
    demo: Vec<u8>,
    executed: Vec<TicCommand>,
}

impl Host for Rows {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, _: &str, bytes: &[u8]) -> Result<(), String> {
        self.demo = bytes.to_vec();
        Ok(())
    }
    fn tic(&mut self, r: &TicReport<'_>) {
        let (t, c, d) = (r.tic, r.executed, r.display);
        self.executed.push(c);
        let w = &mut self.text;
        for e in r.events {
            let _ = match *e {
                Event::KeyDown(k) => writeln!(w, "E,{t},keydown,{k},0,0"),
                Event::KeyUp(k) => writeln!(w, "E,{t},keyup,{k},0,0"),
                Event::Mouse { buttons, dx, dy } => writeln!(w, "E,{t},mouse,{buttons},{dx},{dy}"),
            };
        }
        let snap = r.simulation.snapshot();
        let _ = writeln!(
            w,
            "T,{t},{},{},{},{},{:016x}",
            c.forward,
            c.side,
            c.angle_turn,
            c.buttons,
            snap.digest()
        );
        let px = r.frame.pixels();
        let mut masked = px.to_vec();
        for &x in &d.view.flagged_columns {
            masked
                .iter_mut()
                .skip(usize::from(x))
                .step_by(SCREENWIDTH)
                .for_each(|p| *p = 0);
        }
        let cols: Vec<String> = d.view.flagged_columns.iter().map(u16::to_string).collect();
        let _ = writeln!(
            w,
            "F,{t},{},{},{},{}",
            doom_test_evidence::sha256(px),
            doom_test_evidence::sha256(&masked),
            cols.join(" "),
            u8::from(d.view.switch_parity_unknown)
        );
        let set = d.palette_set.map_or(-1, i32::from);
        let _ = writeln!(
            w,
            "P,{t},{},{set},{},{},{}",
            d.palette,
            d.face,
            u8::from(d.message_on),
            r.rndindex
        );
        if let Some(m) = r.message {
            let _ = writeln!(w, "X,{t},{},{}", m.code, m.text);
        }
        if let Some(dir) = &self.captures {
            let path = dir.join(format!("tic_{:05}.bin", t + 1));
            std::fs::write(path, snap.canonical_bytes()).expect("capture");
        }
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

/// Runs a script through the app; returns the session's ending.
fn run_script(app: &mut App, host: &mut Rows, text: &str) -> Res<String> {
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
        if matches!(app.mode(), Mode::Ended(_)) {
            break;
        }
    }
    let ending = format!("{:?}", app.mode());
    app.quit(host);
    Ok(ending)
}

/// Parses `--commands` lines: `forward side angleturn buttons`, `#` comments.
fn parse_commands(text: &str) -> Res<Vec<TicCommand>> {
    let mut out = Vec::new();
    for l in text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
    {
        let w: Vec<&str> = l.split_whitespace().collect();
        match w.as_slice() {
            [] => {}
            [f, s, a, b] => out.push(TicCommand {
                forward: f.parse()?,
                side: s.parse()?,
                angle_turn: a.parse()?,
                buttons: b.parse()?,
            }),
            _ => return Err(format!("bad command line {l:?}").into()),
        }
    }
    Ok(out)
}

fn main() -> Res<()> {
    let (mut wads, mut out, mut script, mut demo) = (Vec::new(), None, None, None);
    let mut commands: Option<Vec<TicCommand>> = None;
    let (mut map, mut skill, mut sb, mut captures) = (1u8, 2u8, 10u8, false);
    let mut unrecorded = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut v = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wads.push(std::fs::read(v()?)?),
            "--out" => out = Some(PathBuf::from(v()?)),
            "--script" => script = Some(std::fs::read_to_string(v()?)?),
            "--demo" => demo = Some(std::fs::read(v()?)?),
            "--map" => map = v()?.parse()?,
            "--skill" => skill = v()?.parse()?,
            "--screenblocks" => sb = v()?.parse()?,
            "--captures" => captures = true,
            "--unrecorded" => unrecorded = true,
            "--commands" => commands = Some(parse_commands(&std::fs::read_to_string(v()?)?)?),
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let out = out.ok_or("--out DIR is required")?;
    std::fs::create_dir_all(out.join("captures"))?;
    let refs: Vec<&[u8]> = wads.iter().map(Vec::as_slice).collect();
    if let Some(d) = &demo {
        (skill, map) = (
            *d.get(1).ok_or("short demo")?,
            *d.get(3).ok_or("short demo")?,
        );
    }
    let data = GameData::load(&refs, GameMode::Registered, map, skill)?;
    let mut host = Rows {
        captures: captures.then(|| out.join("captures")),
        ..Rows::default()
    };
    if captures {
        std::fs::write(
            out.join("captures/tic_00000.bin"),
            data.start().snapshot().canonical_bytes(),
        )?;
    }
    let ending = if let Some(commands) = &commands {
        format!(
            "{:?}",
            replay_full_resolution(&data, sb, commands, &mut host)
        )
    } else if let Some(text) = &script {
        let cfg = SessionConfig {
            screenblocks: sb,
            unrecorded_full_turning: unrecorded,
            ..SessionConfig::default()
        };
        let mut app = App::new(data.clone(), cfg, &mut host);
        let ending = run_script(&mut app, &mut host, text)?;
        // live equals recorded: the replay of the saved demo (unrecorded: of the executed
        // commands, and nothing was saved) gives the same rows
        let mut again = Rows::default();
        if unrecorded {
            if !host.demo.is_empty() {
                return Err("an unrecorded session saved a file".into());
            }
            replay_full_resolution(&data, sb, &host.executed, &mut again);
        } else {
            let commands = DemoCommands::parse(&host.demo, map).map_err(|e| format!("{e:?}"))?;
            replay(&data, sb, &commands, &mut again);
        }
        let live: Vec<&str> = host.text.lines().filter(|l| !l.starts_with("E,")).collect();
        if live != again.text.lines().collect::<Vec<_>>() {
            return Err("the replay of the saved demo differs from the live session".into());
        }
        ending
    } else {
        let d = demo.ok_or("--script FILE or --demo LMP is required")?;
        let commands = DemoCommands::parse(&d, map).map_err(|e| format!("{e:?}"))?;
        host.demo = d;
        format!("{:?}", replay(&data, sb, &commands, &mut host))
    };
    std::fs::write(out.join("session.lmp"), &host.demo)?;
    std::fs::write(out.join("rows.txt"), &host.text)?;
    let tics = host.text.lines().filter(|l| l.starts_with("T,")).count();
    println!("session: tics {tics} ending {ending}");
    Ok(())
}
