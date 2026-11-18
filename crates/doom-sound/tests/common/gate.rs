//! The end-to-end session harness of M5c step 4b: one of the 64 committed items run through the
//! Rust wire-25 simulation (doom-sim's `tests/wire25/mod.rs`: the item table and the setup calls
//! of every level run), each tic's records fed to `doom_sound::session::Sound`, and every row the
//! genuine runners write rebuilt from what the session reports:
//!
//! - the sound-logic stream (DOOM-M5C-SOUND-v1, M5c note §3.1.4): S, T, K, L, C and R rows, in the
//!   genuine order (per record: L, then S, then the K rows it caused; a stop's K row, then T; after the
//!   records the update's K rows, then the C rows of the table, then R). Actor origins are written
//!   as `A<slot>:<generation>` and matched to the genuine thinker serials by a bijection that must
//!   hold across the whole stream;
//! - the mixer session golden (DOOM-M5C-MIXER-v1, tests/reference/m5c_mixer/README.md): the
//!   header, table and lump digests, one `s` row per started sound, the buffer rows with the
//!   driver's repeat compression, the drain after the last tic while a mixer channel is busy (the
//!   genuine driver's loop, tools/reference/m5c_mixer_driver.c), and the stream digest.
//!
//! The R rows need the `M_Random` index, which the status bar owns (M5c note §4.3.2 (i)); here it
//! is advanced by the declared rule (the real status-bar integration is the next step's):
//! the sound draws of the tic (from the session), then `ST_Ticker`'s one draw
//! (`st_randomnumber = M_Random()`, st_stuff.c:992, called after `P_Ticker`, g_game.c:731-732),
//! then after the frame of tic 0 only the wipe's 320 (`wipe_initMelt`, f_wipe.c:159-162: one draw
//! for column 0 and one for each of the other 319 columns; decision T6, M5c note §1.4.5).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use doom_sim::demo::parse_demo_episode1_v2;
use doom_sim::{ActorId, Simulation, Skill, TicCommand};
use doom_sound::logic::SoundRecord;
use doom_sound::lumps::SoundSet;
use doom_sound::mixer::buffer_bytes;
use doom_sound::session::{Sound, SoundConfig, TicSound};
use doom_test_evidence::sha256;

#[allow(unused_imports, reason = "each test target uses a subset")]
pub use super::rows::{Carry, GoldenWriter, drain, stream_rows_with};
use super::{directory, reference, texture_count, wire25};

/// One loaded item: its simulation after load, its commands and its session configuration.
pub struct Item {
    /// The lump name (the stream's file stem).
    pub name: String,
    /// The simulation after the setup calls, before tic 0.
    pub sim: Simulation,
    /// The demo's commands, one per tic.
    pub commands: Vec<TicCommand>,
    /// The session's configuration (map number, texture count of the item's WAD).
    pub config: SoundConfig,
}

/// Loads item `i` of doom-sim's table under wire 25.
pub fn load_item(i: usize) -> Item {
    let (lump, file, lmp) = wire25::ITEMS[i];
    let bytes = std::fs::read(reference(file)).expect("item WAD");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("WAD");
    let demo = std::fs::read(reference(lmp)).expect("demo");
    // Byte 3 of a version-110 header is the map (g_game.c G_BeginRecording).
    let number = demo[3];
    let map =
        doom_map::Map::from_wad(&wad, &format!("E1M{number}"), Default::default()).expect("map");
    let (header, demo) = parse_demo_episode1_v2(&demo, number).expect("demo");
    let skill = if header.skill == 3 {
        Skill::Hard
    } else {
        Skill::Medium
    };
    let sim = wire25::build(true, map, number, skill).expect("wire-25 simulation");
    let textures = texture_count(&directory(&[file]));
    Item {
        name: lump.to_string(),
        sim,
        commands: demo.commands,
        config: SoundConfig::fresh(i32::from(number), textures),
    }
}

/// Writes the genuine stream's rows for one tic from the session's report, with the R row's
/// indices by the declared `M_Random` rule of this harness (module documentation): after
/// `ST_Ticker` one draw more than the session's index, after the frame of tic 0 the wipe's 320.
pub fn stream_rows(
    t: &TicSound,
    records: &[SoundRecord],
    sound: &Sound,
    listener: Option<ActorId>,
    carry: &mut Carry,
) -> Vec<String> {
    let after_st = t.rndindex.wrapping_add(1);
    let after_display = if t.tic == 0 {
        after_st.wrapping_add((320 % 256) as u8)
    } else {
        after_st
    };
    stream_rows_with(t, records, sound, listener, carry, after_st, after_display)
}

/// Whether a session's run continues after a tic.
pub struct RunOutput {
    /// Per tic, the stream rows.
    pub stream: Vec<Vec<String>>,
    /// Per tic, the digest of the buffer mixed after it (if any).
    pub buffers: Vec<Option<String>>,
    /// The mixer golden rows (with the drain).
    pub golden: Vec<String>,
    /// Started sounds.
    pub starts: u64,
    /// The session after the last tic.
    pub sound: Sound,
    /// The simulation's refusal, if it refused a tic: (tic, error).
    pub refused: Option<(u64, String)>,
    /// The simulation after the last tic run.
    pub sim: Simulation,
}

/// A point at which a run can be resumed: the simulation after `tic` tics, the session's saved
/// bytes, and the harness carry.
#[derive(Clone)]
pub struct Checkpoint {
    /// Tics run.
    pub tic: u64,
    /// The simulation then.
    pub sim: Simulation,
    /// The session's saved state then.
    pub sound: Vec<u8>,
    /// The harness carry then.
    pub carry: Carry,
}

/// Runs the commands from `sim` and `sound` (which have run the same number of tics) up to
/// `tics` tics in all, writing rows; before each tic, `before` sees the tics run, the
/// simulation, the session and the carry (to take checkpoints).
pub fn run(
    commands: &[TicCommand],
    mut sim: Simulation,
    mut sound: Sound,
    mut carry: Carry,
    tics: usize,
    golden: Option<&mut GoldenWriter>,
    mut before: impl FnMut(u64, &Simulation, &Sound, Carry),
) -> RunOutput {
    let mut out = RunOutput {
        stream: Vec::new(),
        buffers: Vec::new(),
        golden: Vec::new(),
        starts: 0,
        sound: sound.clone(),
        refused: None,
        sim: sim.clone(),
    };
    let mut golden = golden;
    let start = sound.schedule().tic() as usize;
    for (i, c) in commands.iter().enumerate().take(tics).skip(start) {
        before(i as u64, &sim, &sound, carry);
        let cmd = [
            *c,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ];
        if let Err(e) = sim.advance(cmd) {
            out.refused = Some((i as u64, format!("{e:?}")));
            break;
        }
        let records = sim.sound_records().to_vec();
        let listener = sim.sound_listener().map(|l| l.actor);
        let first_seq = carry.seq;
        let t = sound
            .tic_from_simulation(&sim, carry.index)
            .unwrap_or_else(|e| panic!("tic {i}: the session refused: {e:?}"));
        let rows = stream_rows(&t, &records, &sound, listener, &mut carry);
        if let Some(g) = golden.as_deref_mut() {
            g.starts(&t, first_seq);
            if let Some(b) = &t.buffer {
                g.buffer(t.tic, b, sound.mixer().mask());
            }
        }
        out.buffers
            .push(t.buffer.as_ref().map(|b| sha256(&buffer_bytes(b))));
        out.stream.push(rows);
    }
    out.sound = sound;
    out.sim = sim;
    out
}

/// The genuine stream of an item, split by tic (the S, T, K, L, C and R rows; `#`, W and Z rows
/// are left out) and checked to hold no E row.
pub fn genuine_stream(name: &str) -> Vec<Vec<String>> {
    let path: PathBuf = reference(&format!("m5c_sound/{name}.sound"));
    let text = std::fs::read_to_string(path).expect("stream");
    let mut by_tic: Vec<Vec<String>> = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("W,") || line.starts_with("Z,") {
            continue;
        }
        assert!(!line.starts_with("E,"), "{name}: an I_Error row: {line}");
        let tic: usize = line
            .split(',')
            .nth(1)
            .and_then(|t| t.parse().ok())
            .unwrap_or_else(|| panic!("{name}: {line}"));
        if by_tic.len() <= tic {
            by_tic.resize(tic + 1, Vec::new());
        }
        by_tic[tic].push(line.to_string());
    }
    by_tic
}

/// The committed mixer session golden of an item, without the device rows `d` and `q`.
pub fn genuine_golden(name: &str) -> Vec<String> {
    let path = reference(&format!("m5c_mixer/sessions/goldens/{name}.golden"));
    std::fs::read_to_string(path)
        .expect("golden")
        .lines()
        .filter(|l| !l.starts_with("d ") && !l.starts_with("q "))
        .map(str::to_string)
        .collect()
}

/// The bijection between genuine thinker serials and Rust handles.
#[derive(Default)]
pub struct Serials {
    to_rust: BTreeMap<String, String>,
    to_c: BTreeMap<String, String>,
}

impl Serials {
    /// Compares a Rust row with a genuine row field by field; an `A<slot>:<generation>` field
    /// must stand for the same serial wherever it occurs, and no two handles for one serial.
    pub fn compare(&mut self, rust: &str, genuine: &str) -> Result<(), String> {
        let r: Vec<&str> = rust.split(',').collect();
        let g: Vec<&str> = genuine.split(',').collect();
        if r.len() != g.len() {
            return Err("field count".into());
        }
        for (i, (a, b)) in r.iter().zip(&g).enumerate() {
            if let Some(handle) = a.strip_prefix('A') {
                let (h, s) = (handle.to_string(), b.to_string());
                let known_c = self.to_c.get(&h);
                let known_r = self.to_rust.get(&s);
                match (known_c, known_r) {
                    (None, None) => {
                        self.to_c.insert(h.clone(), s.clone());
                        self.to_rust.insert(s, h);
                    }
                    (Some(c), Some(r)) if *c == s && *r == h => {}
                    _ => return Err(format!("field {i}: handle {h} against serial {s}")),
                }
            } else if a != b {
                return Err(format!("field {i}"));
            }
        }
        Ok(())
    }
}

/// Counts of compared rows by kind.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RowCounts {
    /// S rows.
    pub s: u64,
    /// T rows.
    pub t: u64,
    /// K rows.
    pub k: u64,
    /// C rows.
    pub c: u64,
    /// L rows.
    pub l: u64,
    /// R rows.
    pub r: u64,
    /// S rows of starts without a placement, whose three listener columns were not compared
    /// (the record does not carry them).
    pub s_listener_unchecked: u64,
}

impl RowCounts {
    /// Counts one row by its kind.
    pub fn add(&mut self, row: &str) {
        match row.as_bytes().first() {
            Some(b'S') => self.s += 1,
            Some(b'T') => self.t += 1,
            Some(b'K') => self.k += 1,
            Some(b'C') => self.c += 1,
            Some(b'L') => self.l += 1,
            Some(b'R') => self.r += 1,
            _ => panic!("row kind {row}"),
        }
    }
}

/// For an S row whose Rust form has no placement (`-` in all five position columns) and whose
/// genuine form has none for the origin (ox, oy `-`), the genuine row with its listener columns
/// replaced by `-`.
fn unplaced_listener(rust: &str, genuine: &str) -> Option<String> {
    let r: Vec<&str> = rust.split(',').collect();
    let g: Vec<&str> = genuine.split(',').collect();
    if r.first() != Some(&"S") || g.first() != Some(&"S") || r.len() != 22 || g.len() != 22 {
        return None;
    }
    if r[17..22].iter().any(|f| *f != "-") || g[17] != "-" || g[18] != "-" {
        return None;
    }
    let mut out: Vec<&str> = g[..19].to_vec();
    out.extend(["-", "-", "-"]);
    Some(out.join(","))
}

/// Compares an item's Rust stream rows with its genuine stream; returns the first difference.
pub fn compare_stream(
    name: &str,
    rust: &[Vec<String>],
    genuine: &[Vec<String>],
    counts: &mut RowCounts,
) -> Result<(), String> {
    let mut serials = Serials::default();
    if rust.len() != genuine.len() {
        return Err(format!(
            "{name}: {} tics run, the genuine stream has {}",
            rust.len(),
            genuine.len()
        ));
    }
    for (tic, (r, g)) in rust.iter().zip(genuine).enumerate() {
        for i in 0..r.len().max(g.len()) {
            let rr = r.get(i).map_or("<none>", String::as_str);
            let gg = g.get(i).map_or("<none>", String::as_str);
            // SIM NEEDS (step 4b report): the genuine runner writes the listener's x, y and
            // angle at the call for every start; a wire-25 record holds them only for a
            // positional start (SoundPlacement). For a start without a placement the three
            // columns are not compared (nothing the sound logic does reads them there,
            // s_sound.c:304-324), and each such row is counted.
            let patched;
            let gg = match unplaced_listener(rr, gg) {
                Some(g2) => {
                    counts.s_listener_unchecked += 1;
                    patched = g2;
                    patched.as_str()
                }
                None => gg,
            };
            serials.compare(rr, gg).map_err(|e| {
                format!("{name}: tic {tic} row {i} ({e}):\n  genuine: {gg}\n  rust:    {rr}")
            })?;
            counts.add(gg);
        }
    }
    Ok(())
}

/// Shared handle to a set for the whole gate.
pub fn session_set() -> Arc<SoundSet> {
    super::sound_set("m5c_sfx.wad")
}
