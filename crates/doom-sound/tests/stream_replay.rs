//! The sound logic replayed against the committed genuine sound-logic streams
//! (tests/reference/m5c_sound/*.sound, DOOM-M5C-SOUND-v1, M5c note §3.1.4) as far as each stream
//! can be replayed from what it holds.
//!
//! A stream holds every start's origin identity and, for positional starts, the positions read at
//! the call (S rows), every removal stop (T rows) and the `rndindex` around each tic (R rows). It
//! does not hold the positions `S_UpdateSounds` reads after a tic. So each stream is replayed tic
//! by tic, under the declared schedule (decision T5: `S_UpdateSounds` after every tic with
//! `gametic = tic + 1`; the mixer's buffers after tic floor(512k/315)), until the first update at
//! which a still-playing channel has a positional origin (an actor other than the listener, a
//! sector or a button): that tic's records are still compared, the replay stops there. Compared
//! on every replayed row:
//!
//! - S: outcome, draws, value drawn, `I_StartSound`'s vol, sep, pitch and priority, the channel,
//!   the handle, samexy;
//! - K: every channel freed (by a start, a removal stop or an update), in order: channel, sfx,
//!   origin, handle, reason, playing;
//! - T: the number of channels a removal stop freed;
//! - L: the sounds whose lump was looked up in the tic;
//! - C: the channel table after each update (the `I_UpdateSoundParams` columns are "-" on every
//!   replayed tic, no positional channel being examined);
//! - R: `rndindex` after `P_Ticker` (the sound draws of the tic, starting from the index after the
//!   previous tic's display), the cumulative sound draws and the occupied channel count.
//!
//! Equivalence class: source equivalence against genuine traces.

mod common;

use std::collections::BTreeSet;

use common::{reference, sound_set};
use doom_sim::ActorId;
use doom_sound::logic::{
    ChannelStop, Listener, LogicConfig, OriginPositions, SoundLogic, SoundOrigin, SoundPlacement,
    StartOutcome, StopReason,
};
use doom_sound::mixer::{Buffer, Mixer, buffers_due};
use doom_types::DoomRng;

struct NoPositions;
impl OriginPositions for NoPositions {
    fn actor_xy(&self, _: ActorId) -> Option<[i32; 2]> {
        None
    }
    fn sector_xy(&self, _: u16) -> Option<[i32; 2]> {
        None
    }
    fn button_xy(&self, _: u8) -> Option<[i32; 2]> {
        None
    }
}

/// Origin identities: thinker serials become actor slots (generation 0); the player's serial is
/// the listener.
fn origin(kind: &str, id: &str) -> SoundOrigin {
    let n: i64 = id.parse().expect("origin id");
    match kind {
        "none" => SoundOrigin::None,
        "player" | "mobj" => SoundOrigin::Actor(ActorId::from_parts(n as u32, 0)),
        "sector" => SoundOrigin::Sector(n as u16),
        "button" => SoundOrigin::Button(n as u8),
        other => panic!("origin kind {other}"),
    }
}

fn reason(s: &str) -> StopReason {
    match s {
        "same_origin" => StopReason::SameOrigin,
        "priority" => StopReason::Priority,
        "remove" => StopReason::Remove,
        "level_start" => StopReason::LevelStart,
        "finished" => StopReason::Finished,
        "inaudible" => StopReason::Inaudible,
        other => panic!("reason {other}"),
    }
}

fn outcome(s: &str) -> StartOutcome {
    match s {
        "started" => StartOutcome::Started,
        "inaudible" => StartOutcome::Inaudible,
        "no_channel" => StartOutcome::NoChannel,
        "link_volume" => StartOutcome::LinkVolume,
        other => panic!("outcome {other}"),
    }
}

fn stop_key(s: &ChannelStop) -> (usize, u16, SoundOrigin, i32, StopReason, bool) {
    (
        s.cnum,
        s.entry.sfx.get(),
        s.entry.origin,
        s.entry.handle,
        s.reason,
        s.playing,
    )
}

#[derive(Default)]
struct Totals {
    streams_full: u64,
    tics: u64,
    stream_tics: u64,
    s: u64,
    k: u64,
    t: u64,
    l: u64,
    c: u64,
    r: u64,
}

fn origin_text(o: SoundOrigin, listener: ActorId) -> String {
    match o {
        SoundOrigin::None => "none,-1".to_string(),
        SoundOrigin::Actor(a) if a == listener => format!("player,{}", a.slot()),
        SoundOrigin::Actor(a) => format!("mobj,{}", a.slot()),
        SoundOrigin::Sector(s) => format!("sector,{s}"),
        SoundOrigin::Button(b) => format!("button,{b}"),
    }
}

/// Replays one stream; returns the number of tics replayed and whether it ran to its end.
fn replay(name: &str, text: &str, t: &mut Totals) -> (u64, bool) {
    let map: i32 = text
        .lines()
        .find(|l| l.starts_with("#level,"))
        .and_then(|l| l.split(',').nth(4))
        .expect("#level")
        .parse()
        .expect("map");
    // The player's serial is the listener (players[consoleplayer].mo, d_main.c:392).
    let listener = text
        .lines()
        .filter(|l| l.starts_with("S,"))
        .find_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            (f[4] == "player").then(|| ActorId::from_parts(f[5].parse().unwrap(), 0))
        })
        .unwrap_or(ActorId::from_parts(u32::MAX, 0));
    let mut by_tic: Vec<Vec<Vec<&str>>> = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        if matches!(f[0], "Z" | "W") {
            continue;
        }
        assert_ne!(f[0], "E", "{name}: an I_Error row");
        let tic: usize = f[1].parse().unwrap_or_else(|_| panic!("{name}: {line}"));
        if by_tic.len() <= tic {
            by_tic.resize(tic + 1, Vec::new());
        }
        by_tic[tic].push(f);
    }
    let mut logic = SoundLogic::new(LogicConfig::fresh(map)).unwrap();
    let mut mixer = Mixer::new(sound_set("m5c_sfx.wad"));
    let mut buf: Buffer = [[0; 2]; 512];
    let mut nbuf = 0u64;
    let mut index = 0u8; // rndindex at the start of the tic
    let mut snd_draws = 0u64;
    let n = by_tic.len() as u64;
    t.stream_tics += n;
    for (tic, rows) in by_tic.iter().enumerate() {
        let ctx = |f: &[&str]| format!("{name}: {}", f.join(","));
        let mut rng = DoomRng::from_indices(0, index);
        let mut pending: Vec<ChannelStop> = Vec::new();
        let mut lookups = BTreeSet::new();
        let mut l_rows = BTreeSet::new();
        let mut update_rows: Vec<&Vec<&str>> = Vec::new();
        let mut r_row = None;
        // Record phase: S, T, L and the K rows of starts and removal stops (seq > 0).
        for f in rows {
            match f[0] {
                "S" => {
                    assert!(pending.is_empty(), "{}: unmatched stops", ctx(f));
                    let at = (f[17] != "-").then(|| SoundPlacement {
                        origin: [f[17].parse().unwrap(), f[18].parse().unwrap()],
                        listener: [f[19].parse().unwrap(), f[20].parse().unwrap()],
                        listener_angle: f[21].parse().unwrap(),
                    });
                    let gametic: i32 = f[16].parse().unwrap();
                    assert_eq!(gametic as usize, tic, "{}", ctx(f));
                    let e = logic
                        .start(
                            f[3].parse().unwrap(),
                            origin(f[4], f[5]),
                            at,
                            &mut rng,
                            &mut mixer,
                            gametic,
                        )
                        .unwrap_or_else(|r| panic!("{}: refused {r:?}", ctx(f)));
                    assert_eq!(e.outcome, outcome(f[6]), "{}", ctx(f));
                    assert_eq!(e.draws().to_string(), f[7], "{}", ctx(f));
                    assert_eq!(e.rnd.map_or(-1, i32::from).to_string(), f[8], "{}", ctx(f));
                    snd_draws += u64::from(e.draws());
                    if e.outcome == StartOutcome::Started {
                        let want = [f[9], f[10], f[11], f[12]].map(|x| x.parse::<i32>().unwrap());
                        assert_eq!(e.args.unwrap(), want, "{}", ctx(f));
                        assert_eq!(e.cnum.unwrap().to_string(), f[13], "{}", ctx(f));
                        assert_eq!(e.handle.unwrap().to_string(), f[14], "{}", ctx(f));
                        if e.lump_lookup {
                            lookups.insert(e.sfx.get());
                        }
                    } else {
                        assert_eq!((f[13], f[14]), ("-1", "-1"), "{}", ctx(f));
                    }
                    assert_eq!(u8::from(e.samexy).to_string(), f[15], "{}", ctx(f));
                    pending.extend(e.stops.iter().flatten().copied());
                    t.s += 1;
                }
                "T" => {
                    assert!(pending.is_empty(), "{}: unmatched stops", ctx(f));
                    let s = logic.stop(origin(f[3], f[4]), tic as i32);
                    assert_eq!(u8::from(s.is_some()).to_string(), f[5], "{}", ctx(f));
                    pending.extend(s);
                    t.t += 1;
                }
                "L" => {
                    l_rows.insert(f[2].parse::<u16>().unwrap());
                    t.l += 1;
                }
                "K" if f[2] != "0" => {
                    assert!(!pending.is_empty(), "{}: no stop to match", ctx(f));
                    let want = (
                        f[3].parse::<usize>().unwrap(),
                        f[4].parse::<u16>().unwrap(),
                        origin(f[5], f[6]),
                        f[7].parse::<i32>().unwrap(),
                        reason(f[8]),
                        f[9] == "1",
                    );
                    assert_eq!(stop_key(&pending.remove(0)), want, "{}", ctx(f));
                    t.k += 1;
                }
                "K" | "C" => update_rows.push(f),
                "R" => r_row = Some(f),
                other => panic!("{name}: row kind {other}"),
            }
        }
        let r = r_row.unwrap_or_else(|| panic!("{name}: tic {tic} has no R row"));
        assert!(
            pending.is_empty(),
            "{}: unmatched stops {pending:?}",
            ctx(r)
        );
        assert_eq!(lookups, l_rows, "{}: lump lookups of the tic", ctx(r));
        assert_eq!(
            rng.indices()[1].to_string(),
            r[2],
            "{}: rndindex after P_Ticker",
            ctx(r)
        );
        assert_eq!(
            snd_draws.to_string(),
            r[5],
            "{}: cumulative sound draws",
            ctx(r)
        );
        // S_UpdateSounds after the tic with gametic = tic + 1 (decision T5), unless a playing
        // channel needs a position the stream does not hold.
        let gametic = tic as i32 + 1;
        let needs_position = logic.channels().iter().flatten().any(|e| {
            gametic < e.handle
                && !matches!(e.origin, SoundOrigin::None)
                && e.origin != SoundOrigin::Actor(listener)
        });
        if needs_position {
            t.tics += tic as u64;
            return (tic as u64, false);
        }
        let l = Listener {
            actor: listener,
            xy: [0, 0],
            angle: 0,
        };
        let u = logic
            .update(&l, &NoPositions, gametic)
            .unwrap_or_else(|e| panic!("{}: update refused {e:?}", ctx(r)));
        assert!(u.params.is_empty());
        let mut stops = u.stops.iter();
        let mut table = Vec::new();
        for (c, e) in logic.channels().iter().enumerate() {
            if let Some(e) = e {
                table.push(format!(
                    "C,{tic},{c},{},{},{},-,-,-",
                    e.sfx.get(),
                    origin_text(e.origin, listener),
                    e.handle
                ));
            }
        }
        let mut c_rows = Vec::new();
        for f in update_rows {
            if f[0] == "K" {
                let s = stops
                    .next()
                    .unwrap_or_else(|| panic!("{}: no update stop", ctx(f)));
                let want = (
                    f[3].parse::<usize>().unwrap(),
                    f[4].parse::<u16>().unwrap(),
                    origin(f[5], f[6]),
                    f[7].parse::<i32>().unwrap(),
                    reason(f[8]),
                    f[9] == "1",
                );
                assert_eq!(stop_key(s), want, "{}", ctx(f));
                t.k += 1;
            } else {
                c_rows.push(f.join(","));
            }
        }
        assert!(
            stops.next().is_none(),
            "{}: an update stop the stream lacks",
            ctx(r)
        );
        assert_eq!(table, c_rows, "{}: channel table after the update", ctx(r));
        assert_eq!(
            (r[8], r[9].parse::<usize>().unwrap()),
            ("1", table.len()),
            "{}",
            ctx(r)
        );
        t.c += table.len() as u64;
        t.r += 1;
        for _ in 0..buffers_due(tic as u64, nbuf) {
            mixer.mix_buffer(&mut buf);
            nbuf += 1;
        }
        // The status bar's draw and (tic 0) the wipe's 320: the index after the display.
        index = r[4].parse().unwrap();
    }
    t.tics += n;
    t.streams_full += 1;
    (n, true)
}

#[test]
fn every_stream_is_replayed_as_far_as_it_holds_positions() {
    let mut paths: Vec<_> = std::fs::read_dir(reference("m5c_sound"))
        .expect("m5c_sound")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "sound"))
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 64);
    let mut t = Totals::default();
    let mut partial = Vec::new();
    for p in &paths {
        let name = p.file_stem().unwrap().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(p).expect("stream");
        let (tics, full) = replay(&name, &text, &mut t);
        if !full {
            partial.push(format!("{name}:{tics}"));
        }
    }
    eprintln!(
        "stream replay: {} of 64 streams to their end, {} tics of {} replayed; rows compared: \
         S {}, K {}, T {}, L {}, C {}, R {}; stopped at the first update needing a position: {}",
        t.streams_full,
        t.tics,
        t.stream_tics,
        t.s,
        t.k,
        t.t,
        t.l,
        t.c,
        t.r,
        partial.join(" ")
    );
    assert_eq!(t.streams_full, 37, "streams replayed to their end");
}
