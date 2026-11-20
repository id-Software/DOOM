//! M5c README visual: one DOOM-M5C-STARTS-v1 start list through the Rust mixer under the
//! declared schedule, writing the raw sample stream, for `tools/media/m5c_readme_visual.py`.
//!
//! The start list is a committed session list (`tests/reference/m5c_mixer/sessions/starts/`),
//! whose rows are `s` rows only. Each tic t from 0 runs its starts in list order
//! ([`Mixer::start`], `I_StartSound` with `addsfx`, i_sound.c:470-501 and 264-381) and then mixes
//! every buffer due under decision T5 ([`buffers_due`]: buffer k of 512 frames after tic
//! floor(512k/315)), until the list's `tics` have run, every start is made and no mixer channel
//! is busy: the loop of the genuine mixer driver (`tools/reference/m5c_mixer_driver.c`, its
//! `for (t = 0; ; t++)` loop). The stream is written as the fake device received it in the
//! genuine run: each buffer's 2,048 bytes ([`buffer_bytes`], little-endian signed 16-bit, left
//! first). Nothing is played and no audio device is opened.
//!
//! The list's `n` and `g` rows (harness only, never in a session list) are refused, as is a
//! handle different from the one the list records, a refused start, and a list that does not
//! end within the buffer limit. One line is printed on success:
//! `starts S buffers B stream SHA256`.
//!
//! ```sh
//! cargo run -p doom-sound --release --example m5c_mixer_raw -- \
//!     --wad tests/reference/m5c_sfx.wad --starts LIST.txt --raw OUT.raw
//! ```

use std::process::ExitCode;
use std::sync::Arc;

use doom_render::LumpDirectory;
use doom_sound::lumps::{SoundCaps, SoundSet};
use doom_sound::mixer::{Buffer, Mixer, buffer_bytes, buffers_due};
use doom_sound::sounds::SfxId;
use doom_test_evidence::sha256;

/// The most buffers one run may mix (the genuine driver's default limit is far above any session).
const MAX_BUFFERS: u64 = 2_000_000;

/// One `s TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO HANDLE` row.
struct Start {
    tic: u64,
    seq: i64,
    gametic: i32,
    sfx: SfxId,
    vol: i32,
    sep: i32,
    pitch: i32,
    handle: Option<i32>,
}

fn int<T: std::str::FromStr>(s: &str, line: &str) -> Result<T, String> {
    s.parse()
        .map_err(|_| format!("not an integer in range: {s:?} in {line:?}"))
}

/// Parses the list: (`tics`, starts), with the schema, config and order checks.
fn parse(text: &str) -> Result<(u64, Vec<Start>), String> {
    let (mut schema, mut config, mut tics) = (false, false, 0u64);
    let mut starts: Vec<Start> = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.is_empty() || f[0].starts_with('#') {
            continue;
        }
        if !schema {
            if f != ["schema", "DOOM-M5C-STARTS-v1"] {
                return Err(format!("first directive is not the schema: {line:?}"));
            }
            schema = true;
            continue;
        }
        match f[0] {
            "config" if f == ["config", "integrated"] => config = true,
            "tics" if f.len() == 2 => tics = int(f[1], line)?,
            "s" if f.len() == 10 => {
                let sfx: i64 = int(f[4], line)?;
                let s = Start {
                    tic: int(f[1], line)?,
                    seq: int(f[2], line)?,
                    gametic: int(f[3], line)?,
                    sfx: SfxId::new(sfx).ok_or(format!("sfx {sfx} out of 1..108 in {line:?}"))?,
                    vol: int(f[5], line)?,
                    sep: int(f[6], line)?,
                    pitch: int(f[7], line)?,
                    handle: if f[9] == "-" {
                        None
                    } else {
                        Some(int(f[9], line)?)
                    },
                };
                let _prio: i32 = int(f[8], line)?;
                if let Some(p) = starts.last() {
                    if (s.tic, s.seq) <= (p.tic, p.seq) {
                        return Err(format!("TIC SEQ does not strictly increase at {line:?}"));
                    }
                }
                starts.push(s);
            }
            _ => return Err(format!("not a session-list directive: {line:?}")),
        }
    }
    if !(schema && config) {
        return Err("missing schema or config directive".to_string());
    }
    Ok((tics, starts))
}

fn run() -> Result<(), String> {
    let mut wads = Vec::new();
    let (mut starts_path, mut raw_path) = (None, None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wads.push(value()?),
            "--starts" => starts_path = Some(value()?),
            "--raw" => raw_path = Some(value()?),
            _ => return Err(format!("unknown argument {a}")),
        }
    }
    let starts_path = starts_path.ok_or("--starts is required")?;
    let raw_path = raw_path.ok_or("--raw is required")?;
    if wads.is_empty() {
        return Err("at least one --wad is required".to_string());
    }
    let mut dir = LumpDirectory::new();
    for w in &wads {
        let bytes = std::fs::read(w).map_err(|e| format!("{w}: {e}"))?;
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default())
            .map_err(|e| format!("{w}: {e:?}"))?;
        dir.add_wad(&wad).map_err(|e| format!("{w}: {e:?}"))?;
    }
    let set = Arc::new(SoundSet::load(&dir, SoundCaps::default()).map_err(|e| format!("{e:?}"))?);
    let text = std::fs::read_to_string(&starts_path).map_err(|e| format!("{starts_path}: {e}"))?;
    let (tics, list) = parse(&text)?;

    let mut mixer = Mixer::new(set);
    let mut buf: Buffer = [[0; 2]; 512];
    let mut stream: Vec<u8> = Vec::new();
    let (mut nbuf, mut made, mut ri) = (0u64, 0u64, 0usize);
    let mut t: u64 = 0;
    loop {
        if t >= tics && ri >= list.len() && mixer.mask() == 0 {
            break;
        }
        if nbuf >= MAX_BUFFERS {
            return Err(format!("{nbuf} buffers mixed and a channel is still busy"));
        }
        while ri < list.len() && list[ri].tic == t {
            let s = &list[ri];
            let m = mixer
                .start(s.sfx, s.vol, s.sep, s.pitch, s.gametic)
                .map_err(|r| {
                    format!(
                        "start at tic {} seq {} refused: {}",
                        s.tic,
                        s.seq,
                        r.message()
                    )
                })?;
            if let Some(h) = s.handle {
                if h != m.handle {
                    return Err(format!(
                        "start at tic {} seq {}: the list records handle {h}, the mixer gave {}",
                        s.tic, s.seq, m.handle
                    ));
                }
            }
            made += 1;
            ri += 1;
        }
        for _ in 0..buffers_due(t, nbuf) {
            mixer.mix_buffer(&mut buf);
            stream.extend_from_slice(&buffer_bytes(&buf));
            nbuf += 1;
        }
        t += 1;
    }
    std::fs::write(&raw_path, &stream).map_err(|e| format!("{raw_path}: {e}"))?;
    println!("starts {made} buffers {nbuf} stream {}", sha256(&stream));
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("m5c_mixer_raw: {e}");
            ExitCode::FAILURE
        }
    }
}
