//! Shared helpers of the doom-sound integration tests: reference paths, the synthetic sound
//! WADs, and a replay of DOOM-M5C-STARTS-v1 start lists that writes the rows of the genuine
//! mixer driver (tools/reference/m5c_mixer_driver.c, schema DOOM-M5C-MIXER-v1) from the Rust
//! mixer, so that they can be compared with the committed genuine goldens line for line.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use doom_render::LumpDirectory;
use doom_sound::lumps::{SoundCaps, SoundSet};
use doom_sound::mixer::{Buffer, MixRefusal, Mixer, buffer_bytes};
use doom_sound::sounds::{NUMSFX, S_SFX, SfxId};
use doom_sound::tables::table_bytes;
use doom_test_evidence::sha256;

/// `tests/reference/<name>`.
pub fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

/// A lump directory of the given WADs in load order (`W_InitMultipleFiles`).
pub fn directory(wads: &[&str]) -> LumpDirectory {
    let mut dir = LumpDirectory::new();
    for w in wads {
        let bytes = std::fs::read(reference(w)).expect("synthetic sound WAD");
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("WAD");
        dir.add_wad(&wad).expect("directory");
    }
    dir
}

/// The sound set of one of the two committed synthetic WADs.
pub fn sound_set(wad: &str) -> Arc<SoundSet> {
    Arc::new(SoundSet::load(&directory(&[wad]), SoundCaps::default()).expect("sound set"))
}

/// One directive of a start list.
#[derive(Clone, Debug)]
pub enum Row {
    /// `s TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO HANDLE`.
    S {
        tic: i64,
        seq: i64,
        gametic: i32,
        sfx: i64,
        vol: i32,
        sep: i32,
        pitch: i32,
        prio: i32,
        handle: Option<i32>,
    },
    /// `n COUNT TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO`.
    N {
        count: i64,
        tic: i64,
        seq: i64,
        gametic: i32,
        sfx: i64,
        vol: i32,
        sep: i32,
        pitch: i32,
        prio: i32,
    },
    /// `g TIC SEQ SFX`.
    G { tic: i64, seq: i64, sfx: i64 },
}

impl Row {
    fn tic(&self) -> i64 {
        match self {
            Row::S { tic, .. } | Row::N { tic, .. } | Row::G { tic, .. } => *tic,
        }
    }
}

/// A parsed start list: `tics N` and the rows.
pub struct StartList {
    pub mintics: i64,
    pub rows: Vec<Row>,
}

fn ints(f: &[&str], n: usize, line: &str) -> Vec<i64> {
    assert_eq!(f.len(), n, "field count in {line:?}");
    f.iter()
        .map(|x| {
            x.parse::<i64>()
                .unwrap_or_else(|_| panic!("integer {x:?} in {line:?}"))
        })
        .collect()
}

/// Parses a DOOM-M5C-STARTS-v1 list (tests/reference/m5c_mixer/README.md).
pub fn parse_starts(text: &str) -> StartList {
    let mut schema = false;
    let mut config = false;
    let mut mintics = 0;
    let mut rows = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.is_empty() || f[0].starts_with('#') {
            continue;
        }
        if !schema {
            assert_eq!(f, ["schema", "DOOM-M5C-STARTS-v1"]);
            schema = true;
            continue;
        }
        match f[0] {
            "config" => {
                assert_eq!(f, ["config", "integrated"]);
                config = true;
            }
            "tics" => mintics = ints(&f[1..], 1, line)[0],
            "s" => {
                let handle = (f.get(9) != Some(&"-")).then(|| f[9].parse().expect("handle"));
                let v = ints(&f[1..9], 8, line);
                rows.push(Row::S {
                    tic: v[0],
                    seq: v[1],
                    gametic: v[2] as i32,
                    sfx: v[3],
                    vol: v[4] as i32,
                    sep: v[5] as i32,
                    pitch: v[6] as i32,
                    prio: v[7] as i32,
                    handle,
                });
            }
            "n" => {
                let v = ints(&f[1..], 9, line);
                rows.push(Row::N {
                    count: v[0],
                    tic: v[1],
                    seq: v[2],
                    gametic: v[3] as i32,
                    sfx: v[4],
                    vol: v[5] as i32,
                    sep: v[6] as i32,
                    pitch: v[7] as i32,
                    prio: v[8] as i32,
                });
            }
            "g" => {
                let v = ints(&f[1..], 3, line);
                rows.push(Row::G {
                    tic: v[0],
                    seq: v[1],
                    sfx: v[2],
                });
            }
            other => panic!("unknown directive {other}"),
        }
    }
    assert!(schema && config, "schema and config directives");
    StartList { mintics, rows }
}

/// Why a replay stopped before draining.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stop {
    /// A mixer refusal at (tic, seq), with the mixer state unchanged by the refused call.
    Mix {
        tic: i64,
        seq: i64,
        refusal: MixRefusal,
    },
    /// `I_GetSfxLumpNum` found no lump at (tic, seq).
    Lump { tic: i64, seq: i64, sfx: i64 },
}

/// The rows the Rust mixer produces for a start list, in the genuine driver's format, without the
/// two device rows `d` and `q` (fake-device glue, not mixer output).
pub struct Replay {
    pub rows: Vec<String>,
    pub stop: Option<Stop>,
    pub starts: u64,
    pub buffers: u64,
}

struct Rep {
    last: Option<(String, u8)>,
    rep: u64,
    rep_k: u64,
    rep_tic: i64,
}

impl Rep {
    fn brk(&mut self, out: &mut Vec<String>) {
        if self.rep > 0 {
            out.push(format!("r {} {} {}", self.rep, self.rep_k, self.rep_tic));
        }
        self.rep = 0;
        self.last = None;
    }
}

/// The `l`, `L` and `P` rows of a loaded set (the driver's `lump_rows`).
fn lump_rows(set: &SoundSet, full: bool, out: &mut Vec<String>) {
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

/// Replays a start list through the Rust mixer under the declared schedule `ticbuffers`
/// (decision T5: buffer k after tic floor(512k/315)), writing the driver's rows.
pub fn replay(set: &Arc<SoundSet>, list: &StartList, lumps: bool, max_buffers: u64) -> Replay {
    let mut out = vec!["h DOOM-M5C-MIXER-v1 schedule ticbuffers frames 512".to_string()];
    let (st, vl) = table_bytes();
    out.push(format!("T steptable {}", sha256(&st)));
    out.push(format!("T vol_lookup {}", sha256(&vl)));
    lump_rows(set, lumps, &mut out);
    let mut mixer = Mixer::new(set.clone());
    let mut rep = Rep {
        last: None,
        rep: 0,
        rep_k: 0,
        rep_tic: 0,
    };
    let mut stream: Vec<u8> = Vec::new();
    let mut nbuf: u64 = 0;
    let mut starts = 0u64;
    let mut ri = 0;
    let mut stop = None;
    let mut buf: Buffer = [[0; 2]; 512];
    let mut t: i64 = 0;
    'run: loop {
        if t >= list.mintics && ri >= list.rows.len() && mixer.mask() == 0 {
            break;
        }
        if nbuf >= max_buffers {
            rep.brk(&mut out);
            out.push(format!(
                "x limit - {nbuf} buffers mixed and a channel is still busy"
            ));
            break;
        }
        while ri < list.rows.len() && list.rows[ri].tic() == t {
            match &list.rows[ri] {
                Row::G { tic, seq, sfx } => {
                    let id = SfxId::new(*sfx).expect("g sfx 1..108");
                    match set.lump_num(id) {
                        Some(l) => {
                            rep.brk(&mut out);
                            out.push(format!("g {tic} {seq} {sfx} > {l}"));
                        }
                        None => {
                            rep.brk(&mut out);
                            out.push(format!(
                                "x {tic} {seq} W_GetNumForName: ds{} not found!",
                                S_SFX[id.index()].name
                            ));
                            stop = Some(Stop::Lump {
                                tic: *tic,
                                seq: *seq,
                                sfx: *sfx,
                            });
                            break 'run;
                        }
                    }
                }
                Row::S {
                    tic,
                    seq,
                    gametic,
                    sfx,
                    vol,
                    sep,
                    pitch,
                    prio,
                    handle,
                } => {
                    let id = SfxId::new(*sfx).expect("s sfx 1..108");
                    let before = mixer.clone();
                    match mixer.start(id, *vol, *sep, *pitch, *gametic) {
                        Ok(m) => {
                            starts += 1;
                            rep.brk(&mut out);
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
                            out.push(format!(
                                "s {tic} {seq} {gametic} {sfx} {vol} {sep} {pitch} {prio} > {} {} {} {} {} {:02x} {prev} {freed}",
                                m.handle, m.slot, m.step, m.left, m.right, m.before
                            ));
                            if let Some(h) = handle {
                                if *h != m.handle {
                                    out.push(format!("m {tic} {seq} {h} {}", m.handle));
                                }
                            }
                        }
                        Err(refusal) => {
                            assert_eq!(mixer, before, "a refused start changed the mixer");
                            rep.brk(&mut out);
                            out.push(format!("x {tic} {seq} {}", refusal.message()));
                            stop = Some(Stop::Mix {
                                tic: *tic,
                                seq: *seq,
                                refusal,
                            });
                            break 'run;
                        }
                    }
                }
                Row::N {
                    count,
                    tic,
                    seq,
                    gametic,
                    sfx,
                    vol,
                    sep,
                    pitch,
                    prio,
                } => {
                    let id = SfxId::new(*sfx).expect("n sfx 1..108");
                    let mut hs = String::new();
                    let mut ss = String::new();
                    let (mut first, mut last) = (0, 0);
                    for c in 0..*count {
                        let m = mixer
                            .start(id, *vol, *sep, *pitch, *gametic)
                            .unwrap_or_else(|r| panic!("n row start {} refused: {r:?}", seq + c));
                        starts += 1;
                        if c == 0 {
                            first = m.handle;
                        }
                        last = m.handle;
                        hs.push_str(&format!("{}\n", m.handle));
                        ss.push_str(&format!("{}\n", m.slot));
                    }
                    rep.brk(&mut out);
                    out.push(format!(
                        "sn {tic} {seq} {count} {gametic} {sfx} {vol} {sep} {pitch} {prio} > {first} {last} {} {} {:02x}",
                        sha256(hs.as_bytes()),
                        sha256(ss.as_bytes()),
                        mixer.mask()
                    ));
                }
            }
            ri += 1;
        }
        let due = doom_sound::mixer::buffers_due(t as u64, nbuf);
        for _ in 0..due {
            mixer.mix_buffer(&mut buf);
            let bytes = buffer_bytes(&buf);
            stream.extend_from_slice(&bytes);
            let digest = sha256(&bytes);
            let mask = mixer.mask();
            if rep.last.as_ref() == Some(&(digest.clone(), mask)) {
                rep.rep += 1;
                rep.rep_k = nbuf;
                rep.rep_tic = t;
            } else {
                rep.brk(&mut out);
                out.push(format!("b {nbuf} {t} {digest} {mask:02x}"));
                rep.last = Some((digest, mask));
            }
            nbuf += 1;
        }
        t += 1;
    }
    rep.brk(&mut out);
    out.push(format!("z {nbuf} {}", sha256(&stream)));
    Replay {
        rows: out,
        stop,
        starts,
        buffers: nbuf,
    }
}

/// The golden's rows without the device rows `d` and `q`.
pub fn golden_rows(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.starts_with("d ") && !l.starts_with("q "))
        .map(str::to_string)
        .collect()
}

/// The first differing row, if any (index, expected, got).
pub fn first_difference(want: &[String], got: &[String]) -> Option<(usize, String, String)> {
    let n = want.len().max(got.len());
    (0..n).find_map(|i| {
        let w = want.get(i).cloned().unwrap_or_else(|| "<end>".to_string());
        let g = got.get(i).cloned().unwrap_or_else(|| "<end>".to_string());
        (w != g).then_some((i, w, g))
    })
}
