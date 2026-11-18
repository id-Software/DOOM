//! `DSND` version 1 (doom_sound::state; M5c note §4.8): restore-and-continue on every one of the
//! 64 items, and every validation rule refusing a forged state.
//!
//! Restore-and-continue (note §4.8.3): each item is run unbroken through the session
//! (tests/common/gate.rs); at several tics (a quarter, a half and three quarters of the way, and
//! the first two tics after which a sound-logic channel or a mixer channel is busy) the session is
//! saved to bytes next to a clone of the simulation; each save is decoded, re-encoded to the same
//! bytes, restored into a fresh `Sound` against the clone, and continued to the end. Every stream
//! row (the genuine S, T, K, L, C and R rows as the gate writes them), every buffer digest and the
//! final state's bytes must equal the unbroken run's, and the continued rows must equal the
//! genuine stream's from that tic on.
//!
//! The forged states are Rust-only safety tests from the checks of note §4.8.2.

mod common;

use common::gate::{Carry, Checkpoint, RowCounts, compare_stream, genuine_stream, load_item, run};
use common::{sound_set, wire25};
use doom_sim::ActorId;
use doom_sound::logic::{ChannelEntry, LogicConfig, SoundOrigin};
use doom_sound::mixer::{MixChannelState, MixStateError};
use doom_sound::schedule::{Schedule, ScheduleError};
use doom_sound::session::{Sound, SoundConfig};
use doom_sound::sounds::{NUMSFX, SfxId};
use doom_sound::state::{MAX_BYTES, SoundState, StateError};
use doom_sound::tables::STEPTABLE;

fn busy(s: &Sound) -> bool {
    s.logic().channels().iter().any(Option::is_some) || s.mixer().mask() != 0
}

#[test]
fn every_item_restores_and_continues_exactly() {
    let set = sound_set("m5c_sfx.wad");
    let (mut checkpoints, mut continued_tics, mut busy_saves) = (0u64, 0u64, 0u64);
    let mut counts = RowCounts::default();
    for i in 0..wire25::ITEMS.len() {
        let item = load_item(i);
        let genuine = genuine_stream(&item.name);
        let n = genuine.len() as u64;
        let fixed = [n / 4, n / 2, 3 * n / 4];
        let mut saves: Vec<Checkpoint> = Vec::new();
        let mut busy_taken = 0;
        let sound = Sound::new(set.clone(), item.config).unwrap();
        let unbroken = run(
            &item.commands,
            item.sim.clone(),
            sound,
            Carry::default(),
            genuine.len(),
            None,
            |tic, sim, sound, carry| {
                let is_busy = busy(sound);
                if fixed.contains(&tic) || (is_busy && busy_taken < 2) {
                    if is_busy {
                        busy_taken += 1;
                    }
                    saves.push(Checkpoint {
                        tic,
                        sim: sim.clone(),
                        sound: sound.state().encode(),
                        carry,
                    });
                }
            },
        );
        let end = unbroken.sound.state().encode();
        for cp in saves {
            let state = SoundState::decode(&cp.sound)
                .unwrap_or_else(|e| panic!("{} tic {}: decode {e:?}", item.name, cp.tic));
            assert_eq!(state.encode(), cp.sound, "{}: re-encoding", item.name);
            let restored = Sound::restore(set.clone(), &state, &cp.sim, item.config)
                .unwrap_or_else(|e| panic!("{} tic {}: restore {e:?}", item.name, cp.tic));
            busy_saves += u64::from(busy(&restored));
            let rest = run(
                &item.commands,
                cp.sim,
                restored,
                cp.carry,
                genuine.len(),
                None,
                |_, _, _, _| {},
            );
            let from = cp.tic as usize;
            assert_eq!(
                rest.stream,
                unbroken.stream[from..],
                "{} restored at tic {}: stream rows",
                item.name,
                cp.tic
            );
            assert_eq!(
                rest.buffers,
                unbroken.buffers[from..],
                "{} restored at tic {}: buffers",
                item.name,
                cp.tic
            );
            assert_eq!(
                rest.sound.state().encode(),
                end,
                "{}: final state",
                item.name
            );
            compare_stream(&item.name, &rest.stream, &genuine[from..], &mut counts)
                .unwrap_or_else(|e| panic!("restored at tic {}: {e}", cp.tic));
            checkpoints += 1;
            continued_tics += rest.stream.len() as u64;
        }
    }
    eprintln!(
        "restore and continue: 64 items, {checkpoints} saves ({busy_saves} with a busy channel), \
         {continued_tics} tics continued; genuine rows matched after restore: S {}, T {}, K {}, \
         C {}, L {}, R {}",
        counts.s, counts.t, counts.k, counts.c, counts.l, counts.r
    );
    assert!(busy_saves > 0);
}

/// A valid state with every kind of entry: a NULL, an actor, a sector and a button channel, two
/// busy mixer channels, a label and two looked-up sounds.
fn sample() -> SoundState {
    let set = sound_set("m5c_sfx.wad");
    let pistol = SfxId::new(1).unwrap();
    let doropn = SfxId::new(20).unwrap();
    let data = |id: SfxId| set.sfx(id).len;
    let mut looked_up = [false; NUMSFX];
    looked_up[1] = true;
    looked_up[20] = true;
    let ch = |sfx, origin, handle| {
        Some(ChannelEntry {
            sfx,
            origin,
            handle,
        })
    };
    SoundState {
        config: SoundConfig {
            logic: LogicConfig {
                gamemap: 1,
                sfx_volume: 8,
                channels: 4,
            },
            textures: 80,
        },
        schedule: Schedule::at(40, Schedule::buffers_before(40)).unwrap(),
        detector: doom_sound::detector::LayoutDetector::from_parts(80, Some(12), 3).unwrap(),
        looked_up,
        channels: vec![
            ch(pistol, SoundOrigin::None, 100),
            ch(doropn, SoundOrigin::Actor(ActorId::from_parts(3, 7)), 101),
            ch(doropn, SoundOrigin::Sector(2), 102),
            ch(doropn, SoundOrigin::Button(14), 103),
        ],
        handlenums: 104,
        mixer: [
            Some(MixChannelState {
                sfx: pistol,
                pos: 10,
                end: data(pistol),
                step: STEPTABLE[128],
                frac: 0xffff,
                start: 39,
                handle: 100,
                left: 127,
                right: 0,
            }),
            None,
            Some(MixChannelState {
                sfx: doropn,
                pos: 0,
                end: data(doropn),
                step: STEPTABLE[0],
                frac: 0,
                start: 0,
                handle: 65535,
                left: 3,
                right: 4,
            }),
            None,
            None,
            None,
            None,
            None,
        ],
    }
}

/// The byte offset of the first sound-logic channel, and of each field after the header.
const CHANNELS_AT: usize = 63;

#[test]
fn a_valid_state_round_trips_and_every_forgery_is_refused() {
    let set = sound_set("m5c_sfx.wad");
    let good = sample();
    let bytes = good.encode();
    assert!(bytes.len() <= MAX_BYTES);
    assert_eq!(SoundState::decode(&bytes), Ok(good.clone()));
    assert!(Sound::from_state(set.clone(), &good, good.config).is_ok());
    let refuse = |b: &[u8], want: StateError| {
        assert_eq!(SoundState::decode(b), Err(want), "{want:?}");
    };
    // Every prefix is truncated; one byte more is trailing; far too long.
    for len in 0..bytes.len() {
        assert!(SoundState::decode(&bytes[..len]).is_err(), "prefix {len}");
    }
    refuse(&bytes[..10], StateError::Truncated);
    let mut b = bytes.clone();
    b.push(0);
    refuse(&b, StateError::Trailing);
    refuse(&vec![0; MAX_BYTES + 1], StateError::TooLong);
    // Magic and version.
    let patch = |at: usize, v: &[u8]| {
        let mut b = bytes.clone();
        b[at..at + v.len()].copy_from_slice(v);
        b
    };
    refuse(&patch(0, b"DSNE"), StateError::Magic);
    refuse(&patch(4, &2u16.to_le_bytes()), StateError::Version(2));
    refuse(&patch(4, &0u16.to_le_bytes()), StateError::Version(0));
    // Schedule: a buffer count the rule does not give; too many tics.
    refuse(
        &patch(14, &(Schedule::buffers_before(40) + 1).to_le_bytes()),
        StateError::Schedule(ScheduleError::Buffers { got: 26, want: 25 }),
    );
    refuse(
        &patch(6, &u64::MAX.to_le_bytes()),
        StateError::Schedule(ScheduleError::TicLimit),
    );
    // Configuration.
    refuse(&patch(22, &[128]), StateError::Config);
    refuse(&patch(23, &[65]), StateError::Config);
    // Detector: label byte 2, a label without firings, a first tic not before the tics run,
    // too many textures, firings without a label.
    refuse(&patch(32, &[2]), StateError::Detector);
    refuse(&patch(41, &0u64.to_le_bytes()), StateError::Detector);
    refuse(&patch(33, &40u64.to_le_bytes()), StateError::Detector);
    refuse(&patch(28, &(65_537u32).to_le_bytes()), StateError::Detector);
    let mut unlabelled = patch(32, &[0]);
    unlabelled[33..41].copy_from_slice(&0u64.to_le_bytes());
    refuse(&unlabelled, StateError::Detector);
    // Looked-up bits 0 and 109.
    refuse(&patch(49, &[0x03]), StateError::LookedUp { sfx: 0 });
    let mut b = bytes.clone();
    b[49 + 13] |= 1 << (109 % 8);
    refuse(&b, StateError::LookedUp { sfx: 109 });
    // Channel 0 (NULL origin): tag, sfx, origin kind, handle.
    refuse(&patch(CHANNELS_AT, &[2]), StateError::Tag);
    refuse(&patch(CHANNELS_AT + 1, &[0]), StateError::Sfx { sfx: 0 });
    refuse(
        &patch(CHANNELS_AT + 1, &[109]),
        StateError::Sfx { sfx: 109 },
    );
    refuse(
        &patch(CHANNELS_AT + 2, &[4]),
        StateError::OriginKind { kind: 4 },
    );
    refuse(
        &patch(CHANNELS_AT + 3, &99i32.to_le_bytes()),
        StateError::Handle { cnum: 0 },
    );
    refuse(
        &patch(CHANNELS_AT + 3, &65_536i32.to_le_bytes()),
        StateError::Handle { cnum: 0 },
    );
    // Structured forgeries (encode writes whatever it is given).
    let forged = |f: &dyn Fn(&mut SoundState)| {
        let mut s = good.clone();
        f(&mut s);
        s.encode()
    };
    refuse(
        &forged(&|s| {
            s.channels[3] = Some(ChannelEntry {
                origin: SoundOrigin::Button(15),
                ..s.channels[3].unwrap()
            })
        }),
        StateError::Button { slot: 15 },
    );
    refuse(
        &forged(&|s| {
            s.channels[2] = Some(ChannelEntry {
                origin: SoundOrigin::Actor(ActorId::from_parts(3, 7)),
                ..s.channels[2].unwrap()
            })
        }),
        StateError::DuplicateOrigin { cnum: 2 },
    );
    refuse(
        &forged(&|s| {
            s.channels[0] = Some(ChannelEntry {
                origin: SoundOrigin::Sector(2),
                ..s.channels[0].unwrap()
            })
        }),
        StateError::DuplicateOrigin { cnum: 2 },
    );
    refuse(
        &forged(&|s| s.handlenums = 50),
        StateError::Mixer(MixStateError::Counter),
    );
    let mix = |f: &dyn Fn(&mut MixChannelState)| {
        forged(&|s| {
            let mut m = s.mixer[2].unwrap();
            f(&mut m);
            s.mixer[2] = Some(m);
        })
    };
    refuse(
        &mix(&|m| m.pos = m.end),
        StateError::Mixer(MixStateError::Position { slot: 2 }),
    );
    refuse(
        &mix(&|m| m.frac = 0x10000),
        StateError::Mixer(MixStateError::Remainder { slot: 2 }),
    );
    refuse(
        &mix(&|m| m.step = STEPTABLE[0] + 1),
        StateError::Mixer(MixStateError::Step { slot: 2 }),
    );
    refuse(
        &mix(&|m| m.right = 128),
        StateError::Mixer(MixStateError::Volume { slot: 2 }),
    );
    refuse(
        &mix(&|m| m.handle = 99),
        StateError::Mixer(MixStateError::Handle { slot: 2 }),
    );
    refuse(&mix(&|m| m.start = 40), StateError::StartTic { slot: 2 });
    refuse(&mix(&|m| m.start = -1), StateError::StartTic { slot: 2 });
    refuse(
        &forged(&|s| {
            s.schedule = Schedule::new();
            s.detector = doom_sound::detector::LayoutDetector::new(80).unwrap();
            s.mixer = [None; 8];
        }),
        StateError::BusyAtStart,
    );
    // Against the set: a wrong padded end, and a looked-up sound whose lump is missing.
    let mut wrong_end = good.clone();
    let mut m = wrong_end.mixer[2].unwrap();
    m.end += 512;
    m.pos = 0;
    wrong_end.mixer[2] = Some(m);
    let decoded = SoundState::decode(&wrong_end.encode()).unwrap();
    assert_eq!(
        Sound::from_state(set.clone(), &decoded, good.config),
        Err(StateError::Mixer(MixStateError::End { slot: 2 }))
    );
    let missing = (1..NUMSFX)
        .map(|n| SfxId::new(n as i64).unwrap())
        .find(|id| set.lump_num(*id).is_none())
        .expect("a sound without a lump in m5c_sfx.wad");
    let mut ghost = good.clone();
    ghost.looked_up[missing.index()] = true;
    assert_eq!(
        Sound::from_state(set.clone(), &ghost, good.config),
        Err(StateError::LookedUp { sfx: missing.get() })
    );
}

/// Against the simulation: the tic, an actor that does not resolve, a sector past the map.
#[test]
fn restore_checks_the_simulation() {
    let set = sound_set("m5c_sfx.wad");
    let item = load_item(0);
    let sound = Sound::new(set.clone(), item.config).unwrap();
    let out = run(
        &item.commands,
        item.sim.clone(),
        sound,
        Carry::default(),
        3,
        None,
        |_, _, _, _| {},
    );
    let sim = out.sim;
    let state = out.sound.state();
    assert!(Sound::restore(set.clone(), &state, &sim, item.config).is_ok());
    assert_eq!(
        Sound::restore(set.clone(), &state, &item.sim, item.config),
        Err(StateError::TicMismatch {
            state: 3,
            simulation: 0
        })
    );
    let mut s = state.clone();
    s.channels[0] = Some(ChannelEntry {
        sfx: SfxId::new(1).unwrap(),
        origin: SoundOrigin::Actor(ActorId::from_parts(60_000, 9)),
        handle: 100,
    });
    assert_eq!(
        Sound::restore(set.clone(), &s, &sim, item.config),
        Err(StateError::ActorGone { cnum: 0 })
    );
    s.channels[0] = Some(ChannelEntry {
        sfx: SfxId::new(1).unwrap(),
        origin: SoundOrigin::Sector(u16::MAX),
        handle: 100,
    });
    assert_eq!(
        Sound::restore(set.clone(), &s, &sim, item.config),
        Err(StateError::Sector { cnum: 0 })
    );
}

/// Review finding M2 (review-sound): the bytes never choose the session's configuration. A
/// state whose map number, volume, channel count or texture count differs from the caller's
/// decodes (each value is in range) but is refused on restore with the caller's configuration
/// named, and a hand-built state whose detector counts other textures than its configuration
/// is refused.
#[test]
fn restore_refuses_a_configuration_other_than_the_callers() {
    let set = sound_set("m5c_sfx.wad");
    let good = sample();
    let expected = good.config;
    let edits: [&dyn Fn(&mut SoundState); 4] = [
        &|s| s.config.logic.gamemap = 8,
        &|s| s.config.logic.sfx_volume = 0,
        &|s| {
            s.config.logic.channels = 64;
            s.channels.resize(64, None);
        },
        &|s| {
            s.config.textures = 0;
            s.detector = doom_sound::detector::LayoutDetector::new(0).unwrap();
        },
    ];
    for (i, edit) in edits.iter().enumerate() {
        let mut s = good.clone();
        edit(&mut s);
        let decoded = SoundState::decode(&s.encode()).unwrap_or_else(|e| panic!("{i}: {e:?}"));
        assert_eq!(
            Sound::from_state(set.clone(), &decoded, expected),
            Err(StateError::ConfigMismatch {
                saved: decoded.config,
                expected
            }),
            "edit {i}"
        );
        // The same state restores into a session that runs with its configuration.
        assert!(
            Sound::from_state(set.clone(), &decoded, decoded.config).is_ok(),
            "edit {i}"
        );
    }
    let mut skew = good.clone();
    skew.detector = doom_sound::detector::LayoutDetector::from_parts(81, Some(12), 3).unwrap();
    assert_eq!(
        Sound::from_state(set.clone(), &skew, expected),
        Err(StateError::Detector)
    );
}
