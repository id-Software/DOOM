//! The row writers shared by doom-sound's end-to-end session gate (`gate.rs`) and doom-game's
//! real-run example `m5c_real_rows` (included there by path): the genuine sound-logic stream's
//! rows (DOOM-M5C-SOUND-v1, M5c note §3.1.4) rebuilt from a session's per-tic report, and the
//! genuine mixer driver's rows (DOOM-M5C-MIXER-v1, `tools/reference/m5c_mixer_driver.c`) rebuilt
//! from the session's start echoes and buffers. Moved here unchanged from `gate.rs` and `mod.rs`
//! (M5c step 5b), except that the R row's indices after `ST_Ticker` and `D_Display` are now the
//! caller's (`gate.rs` keeps its declared rule in its own `stream_rows`).
#![allow(dead_code, reason = "each includer uses a subset")]

use doom_sim::ActorId;
use doom_sound::logic::{
    Adjusted, ChannelStop, NORM_SEP, RecordEcho, SoundOrigin, SoundRecord, StartOutcome, StopReason,
};
use doom_sound::lumps::SoundSet;
use doom_sound::mixer::{Buffer, buffer_bytes, buffers_due};
use doom_sound::session::{Sound, TicSound};
use doom_sound::sounds::{NUMSFX, S_SFX, SfxId};
use doom_sound::tables::table_bytes;
use doom_test_evidence::sha256;

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

/// Writes the genuine stream's rows for one tic from the session's report. The R row's three
/// `M_Random` indices are the session's index after `P_Ticker` (`t.rndindex`) and the caller's
/// `after_st` (after `ST_Ticker`) and `after_display` (after `D_Display`); `carry.index` becomes
/// `after_display`.
pub fn stream_rows_with(
    t: &TicSound,
    records: &[SoundRecord],
    sound: &Sound,
    listener: Option<ActorId>,
    carry: &mut Carry,
    after_st: u8,
    after_display: u8,
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
    let after_p = t.rndindex;
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
        lump_rows(set, false, &mut rows);
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

/// The `l`, `L` and `P` rows of a loaded set (the driver's `lump_rows`).
pub fn lump_rows(set: &SoundSet, full: bool, out: &mut Vec<String>) {
    let mut all = String::new();
    let mut lines = Vec::new();
    for i in 0..NUMSFX {
        let line = match SfxId::new(i as i64) {
            None => format!("l {i} 0 -"),
            Some(id) => {
                let d = set.sfx(id);
                format!("l {i} {} {}", d.len, sha256(&d.data[..d.len as usize]))
            }
        };
        all.push_str(&line);
        all.push('\n');
        lines.push(line);
    }
    if full {
        out.extend(lines);
    }
    out.push(format!("L {NUMSFX} {}", sha256(all.as_bytes())));
    if !full {
        return;
    }
    for (i, info) in S_SFX.iter().enumerate().skip(1) {
        if info.link.is_some() {
            continue;
        }
        let d = set.sfx(SfxId::new(i as i64).unwrap());
        let own = d.source == doom_sound::lumps::SfxSource::Own;
        let size = d.lump_size as i64;
        let real = (size - 8).max(0);
        let len = i64::from(d.len);
        let n128 = d.data[real as usize..d.len as usize]
            .iter()
            .filter(|b| **b == 128)
            .count();
        if size - 8 != len || !own {
            out.push(format!(
                "P {i} {size} {len} {} {n128} {}",
                len - real,
                if own { "own" } else { "fallback" }
            ));
        }
    }
}
