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
use doom_sound::logic::{
    Adjusted, ChannelStop, NORM_SEP, RecordEcho, SoundOrigin, SoundRecord, StartOutcome, StopReason,
};
use doom_sound::lumps::SoundSet;
use doom_sound::mixer::{Buffer, buffer_bytes, buffers_due};
use doom_sound::session::{Sound, SoundConfig, TicSound};
use doom_sound::tables::table_bytes;
use doom_test_evidence::sha256;

use super::{directory, lump_rows_pub, reference, texture_count, wire25};

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

/// Test-side state carried from tic to tic: the genuine row counters and the random index.
#[derive(Clone, Copy, Debug, Default)]
pub struct Carry {
    /// The S and T row counter (`seq`).
    pub seq: u64,
    /// The `M_Random` index at the start of the next tic.
    pub index: u8,
    /// Cumulative sound draws.
    pub snd_draws: u64,
}

/// Writes the genuine stream's rows for one tic from the session's report.
pub fn stream_rows(
    t: &TicSound,
    records: &[SoundRecord],
    sound: &Sound,
    listener: Option<ActorId>,
    carry: &mut Carry,
) -> Vec<String> {
    let tic = t.tic;
    let origin = |o: SoundOrigin| match o {
        SoundOrigin::None => "none,-1".to_string(),
        SoundOrigin::Actor(a) => {
            let kind = if Some(a) == listener {
                "player"
            } else {
                "mobj"
            };
            format!("{kind},A{}:{}", a.slot(), a.generation())
        }
        SoundOrigin::Sector(s) => format!("sector,{s}"),
        SoundOrigin::Button(b) => format!("button,{b}"),
    };
    let reason = |r: StopReason| match r {
        StopReason::SameOrigin => "same_origin",
        StopReason::SameOriginGetChannel => "same_origin_getchannel",
        StopReason::Priority => "priority",
        StopReason::Remove => "remove",
        StopReason::LevelStart => "level_start",
        StopReason::Finished => "finished",
        StopReason::Inaudible => "inaudible",
        StopReason::LinkVolume => "link_volume",
    };
    let k_row = |seq: u64, s: &ChannelStop| {
        format!(
            "K,{tic},{seq},{},{},{},{},{},{}",
            s.cnum,
            s.entry.sfx.get(),
            origin(s.entry.origin),
            s.entry.handle,
            reason(s.reason),
            u8::from(s.playing)
        )
    };
    let mut rows = Vec::new();
    for (record, echo) in records.iter().zip(&t.records) {
        carry.seq += 1;
        let seq = carry.seq;
        match (record, echo) {
            (SoundRecord::Start { at, .. }, RecordEcho::Start(e)) => {
                if e.lump_lookup {
                    rows.push(format!("L,{tic},{}", e.sfx.get()));
                }
                let outcome = match e.outcome {
                    StartOutcome::Started => "started",
                    StartOutcome::Inaudible => "inaudible",
                    StartOutcome::NoChannel => "no_channel",
                    StartOutcome::LinkVolume => "link_volume",
                };
                let (vol, sep, pitch, prio) = match (e.args, e.adjusted) {
                    (Some([v, s, p, q]), _) => {
                        (v.to_string(), s.to_string(), p.to_string(), q.to_string())
                    }
                    // A start that drew but found no channel: the runner writes no argument.
                    (None, _) if e.outcome == StartOutcome::NoChannel => {
                        ("-".into(), "-".into(), "-".into(), "-".into())
                    }
                    // s_sound.c:306-319: an inaudible start's probe values; clipped leaves
                    // *vol at snd_SfxVolume and *sep unwritten (the runner's NORM_SEP).
                    (None, Some(Adjusted::Computed { vol, sep })) => (
                        vol.to_string(),
                        if e.samexy { NORM_SEP } else { sep }.to_string(),
                        "-".into(),
                        "-".into(),
                    ),
                    (None, _) => ("8".into(), NORM_SEP.to_string(), "-".into(), "-".into()),
                };
                let place = at.map_or_else(
                    || "-,-,-,-,-".to_string(),
                    |p| {
                        format!(
                            "{},{},{},{},{}",
                            p.origin[0],
                            p.origin[1],
                            p.listener[0],
                            p.listener[1],
                            p.listener_angle
                        )
                    },
                );
                rows.push(format!(
                    "S,{tic},{seq},{},{},{outcome},{},{},{vol},{sep},{pitch},{prio},{},{},{},{tic},{place}",
                    e.sfx.get(),
                    origin(e.origin),
                    e.draws(),
                    e.rnd.map_or(-1, i32::from),
                    e.cnum.map_or(-1, |c| c as i64),
                    e.handle.map_or(-1, i64::from),
                    u8::from(e.samexy),
                ));
                for s in e.stops.iter().flatten() {
                    rows.push(k_row(seq, s));
                }
                carry.snd_draws += u64::from(e.draws());
            }
            (SoundRecord::Stop { actor }, RecordEcho::Stop(s)) => {
                // The genuine runner writes the K row inside S_StopChannel and the T row when
                // S_StopSound returns.
                if let Some(s) = s {
                    rows.push(k_row(seq, s));
                }
                rows.push(format!(
                    "T,{tic},{seq},{},{}",
                    origin(SoundOrigin::Actor(*actor)),
                    u8::from(s.is_some())
                ));
            }
            _ => panic!("record and echo kinds differ"),
        }
    }
    for s in &t.update.stops {
        rows.push(k_row(0, s));
    }
    let mut occupied = 0;
    for (c, e) in sound.logic().channels().iter().enumerate() {
        let Some(e) = e else { continue };
        occupied += 1;
        let params = t
            .update
            .params
            .iter()
            .find(|p| p.0 == c)
            .map_or("-,-,-".to_string(), |p| format!("{},{},{}", p.1, p.2, p.3));
        rows.push(format!(
            "C,{tic},{c},{},{},{},{params}",
            e.sfx.get(),
            origin(e.origin),
            e.handle
        ));
    }
    // The declared M_Random rule of this harness (module documentation).
    let after_p = t.rndindex;
    let after_st = after_p.wrapping_add(1);
    let after_display = if tic == 0 {
        after_st.wrapping_add((320 % 256) as u8)
    } else {
        after_st
    };
    rows.push(format!(
        "R,{tic},{after_p},{after_st},{after_display},{},{},320,1,{occupied}",
        carry.snd_draws,
        tic + 1
    ));
    carry.index = after_display;
    rows
}

/// The mixer session golden's rows, written as the genuine driver writes them.
pub struct GoldenWriter {
    rows: Vec<String>,
    stream: Vec<u8>,
    buffers: u64,
    last: Option<(String, u8)>,
    rep: u64,
    rep_k: u64,
    rep_tic: u64,
    /// Started sounds written.
    pub starts: u64,
}

impl GoldenWriter {
    /// The header, the table digests and the lump-table digest of `set`.
    pub fn new(set: &SoundSet) -> Self {
        let mut rows = vec!["h DOOM-M5C-MIXER-v1 schedule ticbuffers frames 512".to_string()];
        let (st, vl) = table_bytes();
        rows.push(format!("T steptable {}", sha256(&st)));
        rows.push(format!("T vol_lookup {}", sha256(&vl)));
        lump_rows_pub(set, false, &mut rows);
        Self {
            rows,
            stream: Vec::new(),
            buffers: 0,
            last: None,
            rep: 0,
            rep_k: 0,
            rep_tic: 0,
            starts: 0,
        }
    }

    fn brk(&mut self) {
        if self.rep > 0 {
            self.rows
                .push(format!("r {} {} {}", self.rep, self.rep_k, self.rep_tic));
        }
        self.rep = 0;
        self.last = None;
    }

    /// The `s` rows of a tic's started sounds, with the stream's `seq` of each record.
    pub fn starts(&mut self, t: &TicSound, first_seq: u64) {
        for (i, echo) in t.records.iter().enumerate() {
            let RecordEcho::Start(e) = echo else { continue };
            let (Some(m), Some([vol, sep, pitch, prio])) = (e.mix, e.args) else {
                continue;
            };
            self.brk();
            let prev = m.prev.map_or("-".to_string(), |p| p.to_string());
            let freed: Vec<String> = (0..8)
                .filter(|k| m.freed & (1 << k) != 0)
                .map(|k| k.to_string())
                .collect();
            let freed = if freed.is_empty() {
                "-".to_string()
            } else {
                freed.join(",")
            };
            self.rows.push(format!(
                "s {} {} {} {} {vol} {sep} {pitch} {prio} > {} {} {} {} {} {:02x} {prev} {freed}",
                t.tic,
                first_seq + i as u64 + 1,
                t.tic,
                e.sfx.get(),
                m.handle,
                m.slot,
                m.step,
                m.left,
                m.right,
                m.before
            ));
            self.starts += 1;
        }
    }

    /// One mixed buffer after tic `tic`, with the occupancy mask after it.
    pub fn buffer(&mut self, tic: u64, buf: &Buffer, mask: u8) {
        let bytes = buffer_bytes(buf);
        self.stream.extend_from_slice(&bytes);
        let digest = sha256(&bytes);
        if self.last.as_ref() == Some(&(digest.clone(), mask)) {
            self.rep += 1;
            self.rep_k = self.buffers;
            self.rep_tic = tic;
        } else {
            self.brk();
            self.rows
                .push(format!("b {} {tic} {digest} {mask:02x}", self.buffers));
            self.last = Some((digest, mask));
        }
        self.buffers += 1;
    }

    /// The buffers mixed.
    pub fn buffers(&self) -> u64 {
        self.buffers
    }

    /// The final `z` row; returns every row.
    pub fn finish(mut self) -> Vec<String> {
        self.brk();
        self.rows
            .push(format!("z {} {}", self.buffers, sha256(&self.stream)));
        self.rows
    }
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

/// Runs `item` from `sim` (after `start` tics) with `sound`, for at most `tics` tics in all,
/// writing rows; at each tic in `save_at` (tics run), `save` is called before that tic runs.
#[allow(clippy::too_many_arguments)]
pub fn run(
    commands: &[TicCommand],
    mut sim: Simulation,
    mut sound: Sound,
    mut carry: Carry,
    tics: usize,
    golden: Option<&mut GoldenWriter>,
    save_at: &[u64],
    mut save: impl FnMut(&Simulation, &Sound, Carry),
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
        if save_at.contains(&(i as u64)) {
            save(&sim, &sound, carry);
        }
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

/// The genuine driver's drain: after the last tic, tics without starts while a mixer channel is
/// busy, each mixing its due buffers.
pub fn drain(sound: &Sound, golden: &mut GoldenWriter) {
    let mut mixer = sound.mixer().clone();
    let mut t = sound.schedule().tic();
    let mut buf: Buffer = [[0; 2]; 512];
    while mixer.mask() != 0 {
        for _ in 0..buffers_due(t, golden.buffers()) {
            mixer.mix_buffer(&mut buf);
            golden.buffer(t, &buf, mixer.mask());
        }
        t += 1;
    }
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
