//! M5b real-map display comparison (local only: it reads the IWAD). Replays one script through
//! the simulation and, at each requested screenblocks, observes every tic and draws it through
//! the composed [`Display`] (status bar, 3D view, HUD message, palette choice), comparing every
//! frame with the genuine M5b runner's frame stream (`--frames` of
//! `tools/reference/m5b_driver.c`, DOOM-M5B-FRAMES-v1) by the rules of
//! `tests/m5b_display.rs`: the whole-frame SHA-256, or the digest with the genuine tracker's
//! flagged columns zeroed where it flagged columns; a frame the tracker could not attribute, or
//! whose switch parity is unknown, is non-comparable. Per frame it also compares the renderer
//! counters, `fuzzpos`, the newly mapped lines and the P row (palette, palette set during the
//! frame, face, `message_on`, `M_Random` index), and per tic the message `HU_Ticker` took
//! (none on most tics) against the X rows. Frames are identical to the genuine engine under the
//! declared sound-free, wipe-free contract (decision S15).
//!
//! It writes counts and the digest of the Rust stream as JSON; `tools/local/m5b_real.py`
//! drives it and commits counts and digests only. A refusal of any kind (an observe error such
//! as `MessageUnderivable` or `AttackerRemoved`, a refused frame, resources the display
//! refuses) is reported with its tic and witness, never worked around (decision S17).
//!
//! ```sh
//! cargo run -p doom-render --release --example m5b_real -- --iwad ~/scratch/m5b/iwad/DOOM.WAD \
//!     --lmp tests/reference/e1m1_scripts/WALK.lmp --frames 10=walk.sb10.frames \
//!     [--digests 10=FILE] --json OUT
//! ```
//!
//! Stream digest: the SHA-256 of, per genuine frame, one line
//! `tic,digest,palette,palette_set,face,message_on,rndindex\n` (the digest is the frame's
//! SHA-256, the masked one where the genuine tracker flagged columns, `whole` where the genuine
//! frame is non-comparable as a whole, `refused` where the Rust display refused), followed by
//! one line `X,tic,code,text\n` per message taken; `m5b_real.py` computes the genuine stream's
//! digest by the same rule.
#[path = "../tests/common/mod.rs"]
mod common;

use doom_render::{Display, DisplayConfig, DisplayRefusal, Frame, FrameCounters, RenderRefusal};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

/// One genuine frame: its F row (with U and M rows) and P row.
#[derive(Default)]
struct Golden {
    sha256: String,
    masked: String,
    whole: bool,
    columns: Vec<u16>,
    mapped: Vec<u16>,
    fuzzpos: u32,
    counters: FrameCounters,
    status: Option<[i64; 5]>,
}

/// A genuine DOOM-M5B-FRAMES-v1 stream.
#[derive(Default)]
struct Stream {
    frames: BTreeMap<i64, Golden>,
    messages: BTreeMap<i64, (i32, String)>,
    end: Option<(i64, String)>,
}

fn parse(text: &str, lump: &str) -> Stream {
    let mut s = Stream::default();
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("#schema,DOOM-M5B-FRAMES-v1"), "{lump}");
    for l in lines {
        let c: Vec<&str> = l.split(',').collect();
        let tic = || c[1].parse::<i64>().expect("tic");
        let n = |i: usize| c[i].parse::<i64>().expect("number");
        match c[0] {
            "F" => {
                let g = Golden {
                    sha256: c[2].into(),
                    masked: c[3].into(),
                    fuzzpos: n(4) as u32,
                    whole: c[14] == "1",
                    counters: FrameCounters {
                        mapped: n(5) as u32,
                        visplanes: n(7) as u32,
                        drawsegs: n(8) as u32,
                        vissprites: n(9) as u32,
                        openings: n(10) as u32,
                        solidsegs_peak: n(11) as u32,
                        drawseg_drops: n(12) as u32,
                        vissprite_drop: c[13] == "1",
                    },
                    ..Golden::default()
                };
                assert!(s.frames.insert(tic(), g).is_none(), "{lump}: {l}");
            }
            "U" => s
                .frames
                .get_mut(&tic())
                .expect("U after F")
                .columns
                .push(n(2) as u16),
            "M" => {
                s.frames.get_mut(&tic()).expect("M after F").mapped =
                    c[2].split(' ').map(|v| v.parse().expect("line")).collect();
            }
            "P" => {
                s.frames.get_mut(&tic()).expect("P after F").status =
                    Some(std::array::from_fn(|i| n(i + 2)));
            }
            "X" => {
                let prev = s.messages.insert(tic(), (n(2) as i32, c[3..].join(",")));
                assert!(prev.is_none(), "{lump}: two messages at {l}");
            }
            "O" => {
                s.end.get_or_insert((tic(), format!("refused:{}", c[2])));
            }
            "E" => {
                s.end
                    .get_or_insert((tic(), format!("error:{}", c[2..].join(","))));
            }
            _ => {}
        }
    }
    s
}

/// The refusal a genuine limit ending requires of the Rust renderer (decision R3).
fn expected(end: &str) -> Option<RenderRefusal> {
    match end {
        "error:R_FindPlane: no more visplanes" | "refused:VisplaneOverflow" => {
            Some(RenderRefusal::VisplaneOverflow)
        }
        "refused:OpeningOverflow" => Some(RenderRefusal::OpeningOverflow),
        "refused:SolidSegOverflow" => Some(RenderRefusal::SolidSegOverflow),
        _ => None,
    }
}

const FIELDS: [&str; 5] = ["palette", "palette_set", "face", "message_on", "rndindex"];

/// Per-configuration counts.
#[derive(Default)]
struct Side {
    frames: usize,
    compared_whole: usize,
    compared_masked: usize,
    non_comparable_whole: usize,
    non_comparable_parity: usize,
    mismatched: usize,
    flag_disagreements: usize,
    counter_mismatches: usize,
    field_mismatches: BTreeMap<&'static str, usize>,
    fields_compared: usize,
    message_tics_compared: usize,
    message_mismatches: usize,
    messages_rust: usize,
    refused: BTreeMap<String, usize>,
    observe_error: Option<Value>,
    not_reached: usize,
    end: Option<Value>,
    lines: String,
    xlines: String,
    first: Option<Value>,
    done: bool,
}

impl Side {
    fn note(&mut self, tic: i64, what: String) {
        if self.first.is_none() {
            self.first = Some(json!({"tic": tic, "what": what}));
        }
    }
}

fn pair(v: &str) -> (u8, String) {
    let (sb, rest) = v.split_once('=').expect("SB=VALUE");
    (sb.parse().expect("screenblocks"), rest.to_string())
}

fn main() {
    let (mut iwad, mut lmp, mut json_out) = (None, None::<PathBuf>, None);
    let mut frames: BTreeMap<u8, PathBuf> = BTreeMap::new();
    let mut digests: BTreeMap<u8, PathBuf> = BTreeMap::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{a}: missing value"));
        match a.as_str() {
            "--iwad" => iwad = Some(PathBuf::from(value())),
            "--lmp" => lmp = Some(PathBuf::from(value())),
            "--frames" => {
                let (sb, p) = pair(&value());
                frames.insert(sb, p.into());
            }
            "--digests" => {
                let (sb, p) = pair(&value());
                digests.insert(sb, p.into());
            }
            "--json" => json_out = Some(PathBuf::from(value())),
            _ => {
                eprintln!(
                    "usage: m5b_real --iwad WAD --lmp DEMO --frames SB=FILE... [--digests SB=FILE] --json OUT"
                );
                std::process::exit(2);
            }
        }
    }
    let (Some(iwad), Some(lmp), Some(json_out)) = (iwad, lmp, json_out) else {
        eprintln!("--iwad, --lmp and --json are required");
        std::process::exit(2);
    };
    let lump = lmp
        .file_stem()
        .expect("demo name")
        .to_string_lossy()
        .to_uppercase();
    let item = common::Item {
        lump: lump.clone(),
        wad: iwad,
        lmp,
        room: false,
        gfx: false,
    };
    let configs: Vec<u8> = frames.keys().copied().collect();
    let streams: Vec<Stream> = configs
        .iter()
        .map(|sb| {
            parse(
                &std::fs::read_to_string(&frames[sb]).expect("frames"),
                &lump,
            )
        })
        .collect();
    for (sb, s) in configs.iter().zip(&streams) {
        let first_gap = s.frames.keys().zip(0i64..).find(|(t, k)| **t != *k);
        if first_gap.is_some() || s.frames.values().any(|g| g.status.is_none()) {
            eprintln!("{lump} sb{sb}: genuine stream has a gap or a frame without a P row");
            std::process::exit(1);
        }
    }
    let mut run = common::load(&item);
    let mut sides: Vec<Side> = configs.iter().map(|_| Side::default()).collect();
    let mut displays: Vec<Option<Display>> = Vec::new();
    for (k, &screenblocks) in configs.iter().enumerate() {
        let d = Display::new(
            run.res.clone(),
            run.level.clone(),
            DisplayConfig { screenblocks },
        )
        .map_err(|e| format!("Display::new: {e:?}"))
        .and_then(|mut d| {
            d.begin_level(&run.sim)
                .map(|()| d)
                .map_err(|e| format!("begin_level: {e:?}"))
        });
        match d {
            Ok(d) => displays.push(Some(d)),
            Err(e) => {
                sides[k].observe_error = Some(json!({"tic": 0, "error": e}));
                sides[k].not_reached = streams[k].frames.len();
                sides[k].done = true;
                displays.push(None);
            }
        }
    }
    let mut frame = Frame::new();
    let mut tic: i64 = 0;
    loop {
        let index = tic as usize;
        if sides.iter().all(|s| s.done) {
            break;
        }
        let ok = index < run.commands.len() && common::advance(&mut run, index).is_ok();
        if !ok {
            for (s, st) in sides.iter_mut().zip(&streams) {
                if !s.done {
                    s.not_reached += st.frames.range(tic..).count();
                    s.done = true;
                }
            }
            break;
        }
        let cmd = run.commands[index];
        for (k, slot) in displays.iter_mut().enumerate() {
            let side = &mut sides[k];
            let stream = &streams[k];
            let Some(d) = slot.as_mut().filter(|_| !side.done) else {
                continue;
            };
            let message = match d.observe(&run.sim, &cmd) {
                Ok(m) => m,
                Err(e) => {
                    side.observe_error = Some(json!({"tic": tic, "error": format!("{e:?}")}));
                    side.not_reached += stream.frames.range(tic..).count();
                    side.done = true;
                    continue;
                }
            };
            let result = d.draw(&run.sim, &mut frame);
            let Some(g) = stream.frames.get(&tic) else {
                // The genuine stream ended: at a renderer limit the Rust display must refuse.
                if let Some((t, end)) = &stream.end {
                    let want = expected(end).map(DisplayRefusal::Render);
                    side.end = Some(json!({
                        "tic": t, "genuine": end, "rust": format!("{:?}", result.as_ref().err()),
                        "agree": *t == tic && want.is_some() && want == result.clone().err(),
                    }));
                }
                side.done = true;
                continue;
            };
            side.frames += 1;
            // The message taken on this tic, compared on every tic (none on most).
            side.message_tics_compared += 1;
            let rust_msg = message.map(|m| (m.code, m.text.to_string()));
            if rust_msg.is_some() {
                side.messages_rust += 1;
            }
            if let Some((code, text)) = &rust_msg {
                let _ = writeln!(side.xlines, "X,{tic},{code},{text}");
            }
            if rust_msg.as_ref() != stream.messages.get(&tic) {
                side.message_mismatches += 1;
                side.note(
                    tic,
                    format!(
                        "message: rust code {:?}, genuine code {:?}",
                        rust_msg.as_ref().map(|m| m.0),
                        stream.messages.get(&tic).map(|m| m.0)
                    ),
                );
            }
            let f = match result {
                Ok(f) => f,
                Err(e) => {
                    *side.refused.entry(format!("{e:?}")).or_default() += 1;
                    let _ = writeln!(side.lines, "{tic},refused,-,-,-,-,-");
                    side.note(tic, format!("Rust refused: {e:?}"));
                    side.not_reached += stream.frames.range(tic + 1..).count();
                    side.done = true;
                    continue;
                }
            };
            let px = frame.pixels();
            let digest = if g.whole {
                side.non_comparable_whole += 1;
                "whole".to_string()
            } else if g.columns.is_empty() {
                let d = doom_test_evidence::sha256(px);
                if f.view.switch_parity_unknown {
                    side.non_comparable_parity += 1;
                } else {
                    side.compared_whole += 1;
                    if d != g.sha256 {
                        side.mismatched += 1;
                        side.note(tic, "the whole-frame digest differs".into());
                    }
                }
                d
            } else {
                let d = doom_test_evidence::sha256(&common::masked(px, &g.columns));
                if f.view.switch_parity_unknown {
                    side.non_comparable_parity += 1;
                } else {
                    side.compared_masked += 1;
                    if d != g.masked {
                        side.mismatched += 1;
                        side.note(tic, "the masked digest differs".into());
                    }
                    if f.view.flagged_columns != g.columns {
                        side.flag_disagreements += 1;
                        side.note(tic, "flagged columns differ".into());
                    }
                }
                d
            };
            if f.view.counters != g.counters
                || f.view.fuzzpos != g.fuzzpos
                || f.view.newly_mapped != g.mapped
            {
                side.counter_mismatches += 1;
                side.note(
                    tic,
                    "renderer counters, fuzzpos or mapped lines differ".into(),
                );
            }
            let got = [
                i64::from(f.palette),
                f.palette_set.map_or(-1, i64::from),
                i64::from(f.face),
                i64::from(f.message_on),
                i64::from(d.status().rndindex),
            ];
            let want = g.status.expect("P row");
            side.fields_compared += FIELDS.len();
            for (i, name) in FIELDS.iter().enumerate() {
                if got[i] != want[i] {
                    *side.field_mismatches.entry(name).or_default() += 1;
                    side.note(tic, format!("{name}: rust {} genuine {}", got[i], want[i]));
                }
            }
            let _ = writeln!(
                side.lines,
                "{tic},{digest},{},{},{},{},{}",
                got[0], got[1], got[2], got[3], got[4]
            );
        }
        tic += 1;
    }
    let mut out = serde_json::Map::new();
    for ((sb, side), stream) in configs.iter().zip(sides).zip(&streams) {
        let text = side.lines.clone() + &side.xlines;
        if let Some(p) = digests.get(sb) {
            std::fs::write(p, &text).expect("digest list");
        }
        let fm: BTreeMap<String, usize> = side
            .field_mismatches
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect();
        let v = json!({
            "genuine_frames": stream.frames.len(),
            "frames": side.frames,
            "compared_whole": side.compared_whole,
            "compared_masked": side.compared_masked,
            "non_comparable_whole": side.non_comparable_whole,
            "non_comparable_parity": side.non_comparable_parity,
            "mismatched": side.mismatched,
            "flag_disagreements": side.flag_disagreements,
            "counter_mismatches": side.counter_mismatches,
            "fields_compared": side.fields_compared,
            "field_mismatches": fm,
            "message_tics_compared": side.message_tics_compared,
            "message_mismatches": side.message_mismatches,
            "messages_genuine": stream.messages.len(),
            "messages_rust": side.messages_rust,
            "refused": side.refused,
            "observe_error": side.observe_error,
            "not_reached": side.not_reached,
            "end": side.end,
            "rust_stream_sha256": doom_test_evidence::sha256(text.as_bytes()),
            "first": side.first,
        });
        eprintln!(
            "{lump} sb{sb}: frames {} whole {} masked {} non-comparable {}+{} mismatched {} fields {:?} messages {}/{} mismatches {} refused {:?} observe {} not-reached {}{}",
            side.frames,
            side.compared_whole,
            side.compared_masked,
            side.non_comparable_whole,
            side.non_comparable_parity,
            side.mismatched,
            side.field_mismatches,
            side.messages_rust,
            stream.messages.len(),
            side.message_mismatches,
            side.refused,
            v["observe_error"],
            side.not_reached,
            v["first"]
                .as_object()
                .map(|f| format!("; first: tic {} {}", f["tic"], f["what"]))
                .unwrap_or_default()
        );
        out.insert(sb.to_string(), v);
    }
    std::fs::write(
        &json_out,
        serde_json::to_string_pretty(&Value::Object(out)).expect("json") + "\n",
    )
    .expect("json");
}
