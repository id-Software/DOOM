//! Tests written for the mutants the first M5c mutation run missed (cargo-mutants 27.1.0 at
//! 216f976; docs/validation/M5C.md "Step 10"). Each test names the behaviour it pins, the C line
//! or the written rule its expectation rests on, and the mutants of the run it kills. The
//! expectations come from the C text and the documented rules, with three exceptions that say so
//! where they stand: the caps test takes its bounds from the loaded set's own sizes, the
//! provider test transcribes one genuine row and otherwise compares the provider with the
//! simulation's own reading, and the saved-state size test pins the `DSND` layout.
//!
//! A test below the public API says why the public API cannot reach what it checks; here every
//! test goes through public items.

mod common;

use common::sound_set;
use doom_sim::ActorId;
use doom_sound::detector::{DetectorError, LayoutDetector, MAX_TEXTURES};
use doom_sound::logic::{
    ChannelEntry, Listener, LogicConfig, OriginPositions, SoundLogic, SoundOrigin, SoundPlacement,
    SoundRecord, SoundRefusal, StartOutcome, StopReason,
};
use doom_sound::lumps::{SoundCaps, SoundLoadError, SoundSet};
use doom_sound::mixer::{Buffer, MixChannelState, MixStateError, Mixer};
use doom_sound::schedule::{MAX_TIC, Schedule, ScheduleError};
use doom_sound::session::{Sound, SoundConfig};
use doom_sound::sounds::{NUMSFX, SfxId};
use doom_sound::state::{MAX_BYTES, SoundState, StateError};
use doom_sound::tables::STEPTABLE;
use doom_types::DoomRng;

const PISTOL: u16 = 1;
const CHGUN: u16 = 86;

fn sfx(n: u16) -> SfxId {
    SfxId::new(i64::from(n)).unwrap()
}

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

fn listener() -> Listener {
    Listener {
        actor: ActorId::from_parts(1, 1),
        xy: [0, 0],
        angle: 0,
    }
}

// ---------------------------------------------------------------------------------------------
// doom-sound logic.rs
// ---------------------------------------------------------------------------------------------

/// s_sound.c:283-299: `sfx_chgun` is the one linked sound (sounds.c:204, volume 0): its volume is
/// `snd_SfxVolume + 0`, and only a result below 1 ends the start (s_sound.c:287-290), so at
/// `snd_SfxVolume` 1 the start is kept with volume 1; `S_UpdateSounds` applies the same test to a
/// playing channel (s_sound.c:567-575): kept at volumes 1 and 8, freed at 0.
/// Kills logic.rs `volume < 1` -> `<=` in `SoundLogic::start`, and `snd + info.link_volume < 1`
/// with `+` -> `*`, `<` -> `<=`, `<` -> `==` and `<` -> `>` in `SoundLogic::update`.
#[test]
fn a_linked_sound_is_kept_at_volume_one_and_freed_by_update_only_below_one() {
    let edge = sound_set("m5c_sfx_edge.wad");
    for volume in [1, 8] {
        let config = LogicConfig {
            sfx_volume: volume,
            ..LogicConfig::fresh(1)
        };
        let mut logic = SoundLogic::new(config).unwrap();
        let mut mixer = Mixer::new(edge.clone());
        let mut rng = DoomRng::default();
        let e = logic
            .start(CHGUN, SoundOrigin::None, None, &mut rng, &mut mixer, 0)
            .unwrap();
        assert_eq!(e.outcome, StartOutcome::Started, "volume {volume}");
        assert_eq!(e.args.unwrap()[0], volume, "volume {volume}");
        let u = logic.update(&listener(), &NoPositions, 1).unwrap();
        assert!(u.stops.is_empty(), "volume {volume}: {:?}", u.stops);
        assert!(logic.channels()[0].is_some());
    }
    // At snd_SfxVolume 0 the update frees a playing linked channel (s_sound.c:571-575). No start
    // can make that channel (s_sound.c:287-290 refuses it), so the table is restored.
    let config = LogicConfig {
        sfx_volume: 0,
        ..LogicConfig::fresh(1)
    };
    let entry = ChannelEntry {
        sfx: sfx(CHGUN),
        origin: SoundOrigin::None,
        handle: 100,
    };
    let mut logic =
        SoundLogic::from_parts(config, vec![Some(entry), None, None], [false; NUMSFX]).unwrap();
    let u = logic.update(&listener(), &NoPositions, 1).unwrap();
    assert_eq!(
        u.stops
            .iter()
            .map(|s| (s.cnum, s.reason, s.playing))
            .collect::<Vec<_>>(),
        [(0, StopReason::LinkVolume, true)]
    );
    assert!(logic.channels()[0].is_none());
}

/// `lumpnum` of a sound is `-1` until its first start that obtained a channel (s_sound.c:190-191,
/// 363-365); `lump_looked_up` reports it per sound.
/// Kills logic.rs `SoundLogic::lump_looked_up` -> `false` and -> `true`.
#[test]
fn the_lump_flag_is_set_by_the_first_start_of_that_sound_only() {
    let set = sound_set("m5c_sfx.wad");
    let mut logic = SoundLogic::new(LogicConfig::fresh(1)).unwrap();
    let mut mixer = Mixer::new(set);
    let mut rng = DoomRng::default();
    assert!(!logic.lump_looked_up(sfx(PISTOL)));
    assert!(!logic.lump_looked_up(sfx(2)));
    logic
        .start(PISTOL, SoundOrigin::None, None, &mut rng, &mut mixer, 0)
        .unwrap();
    assert!(logic.lump_looked_up(sfx(PISTOL)));
    assert!(!logic.lump_looked_up(sfx(2)));
    assert_eq!(
        logic.looked_up().iter().filter(|b| **b).count(),
        1,
        "exactly the pistol"
    );
}

/// The checks of `SoundLogic::from_parts` (M5c note §4.8.2): exactly `config.channels` entries,
/// no flag for sound number 0, and a button origin below slot 15 (p_spec.c:1151 reads the next
/// slot; MAXBUTTONS is 16, p_spec.h:244): slot 14 is the last accepted, 15 and above are refused.
/// Kills logic.rs `channels.len() != empty.channels.len() || looked_up[0]` with `||` -> `&&`, and
/// `slot >= MAXBUTTONS - 1` with `-` -> `+` and `-` -> `/`.
#[test]
fn from_parts_checks_the_length_the_zero_flag_and_the_button_slot() {
    let config = LogicConfig::fresh(1);
    let none = [false; NUMSFX];
    // The wrong number of entries alone.
    for len in [2, 4] {
        assert_eq!(
            SoundLogic::from_parts(config, vec![None; len], none),
            Err(SoundRefusal::Config),
            "{len} entries"
        );
    }
    // A flag for sound number 0 alone.
    let mut zero = none;
    zero[0] = true;
    assert_eq!(
        SoundLogic::from_parts(config, vec![None; 3], zero),
        Err(SoundRefusal::Config)
    );
    assert!(SoundLogic::from_parts(config, vec![None; 3], none).is_ok());
    // Button slots.
    let button = |slot| {
        let e = ChannelEntry {
            sfx: sfx(24),
            origin: SoundOrigin::Button(slot),
            handle: 100,
        };
        SoundLogic::from_parts(config, vec![Some(e), None, None], none).map(|_| ())
    };
    assert_eq!(button(14), Ok(()));
    for slot in [15, 16, 255] {
        assert_eq!(button(slot), Err(SoundRefusal::ButtonPastArray { slot }));
    }
}

// ---------------------------------------------------------------------------------------------
// doom-sound detector.rs
// ---------------------------------------------------------------------------------------------

/// s_sound.c:796-817: at exactly `S_CLIPPING_DIST` (not clipped, s_sound.c:773 is `>`) the volume
/// is `(8 * 0) / 1040` = 0, `S_AdjustSoundParams` returns 0 and the start is inaudible, whatever
/// its separation. The detector compares audibility, volume and separation of audible readings
/// only, so a declared reading and a candidate that are both inaudible (distance exactly
/// 1200 units, on different bearings: the separations differ) never fire.
/// Kills detector.rs `start_effect`'s guard `vol > 0` -> `true` and `>` -> `>=`.
#[test]
fn two_inaudible_readings_never_fire_even_with_different_separations() {
    let timer = 1200 << 16;
    let at = SoundPlacement {
        origin: [timer, 0],
        listener: [0, 0],
        listener_angle: 0,
    };
    // The declared reading and the candidate (texture 0, the timer as y) are both at distance
    // 1200 units exactly; their volumes are 0.
    for origin in [[timer, 0], [0, timer]] {
        let a = doom_sound::logic::adjust_sound_params([0, 0], 0, origin, 1, 8).unwrap();
        assert!(!a.audible(), "{origin:?}");
    }
    let mut d = LayoutDetector::new(1).unwrap();
    assert_eq!(d.check_start(5, &at, 1, 8), Ok(false));
    assert_eq!((d.layout_dependent_from(), d.firings()), (None, 0));
}

/// The detector's bounds: a count of exactly `MAX_TEXTURES` is accepted and one more refused
/// (the documented bound "at most `MAX_TEXTURES`"), and the count is reported back.
/// Kills detector.rs `textures > MAX_TEXTURES` -> `>=` in `LayoutDetector::from_parts`, and
/// `LayoutDetector::textures` -> `0` and -> `1`.
#[test]
fn the_detector_keeps_its_texture_count_up_to_the_bound() {
    for n in [7, 80, MAX_TEXTURES] {
        assert_eq!(LayoutDetector::new(n).unwrap().textures(), n);
        assert_eq!(
            LayoutDetector::from_parts(n, Some(3), 2)
                .unwrap()
                .textures(),
            n
        );
    }
    assert_eq!(
        LayoutDetector::new(MAX_TEXTURES + 1),
        Err(DetectorError::Textures)
    );
}

// ---------------------------------------------------------------------------------------------
// doom-sound lumps.rs
// ---------------------------------------------------------------------------------------------

/// `SoundCaps` documents the largest accepted lump and the largest accepted total of padded
/// bytes: a cap equal to the actual figure loads, one byte less is refused with `TooLarge`.
/// The figures are the loaded set's own (`lump_size` of each loaded entry, whose values
/// `tables_and_lumps` checks against the WAD, and the sum of the padded lengths).
/// Kills lumps.rs `size > caps.max_lump` -> `>=`, and `total > caps.max_total` -> `==` and `>=`
/// in `SoundSet::load`.
#[test]
fn the_caps_are_inclusive_bounds() {
    let dir = common::directory(&["m5c_sfx.wad"]);
    let set = SoundSet::load(&dir, SoundCaps::default()).unwrap();
    let (mut biggest, mut total) = (0usize, 0usize);
    for n in 1..NUMSFX as u16 {
        let s = set.sfx(sfx(n));
        biggest = biggest.max(s.lump_size);
        total += s.len as usize;
    }
    assert!(biggest > 0 && total > 0);
    let load = |max_lump, max_total| {
        SoundSet::load(
            &dir,
            SoundCaps {
                max_lump,
                max_total,
            },
        )
    };
    assert!(load(biggest, 1 << 24).is_ok());
    assert!(matches!(
        load(biggest - 1, 1 << 24),
        Err(SoundLoadError::TooLarge { .. })
    ));
    assert!(load(1 << 20, total).is_ok());
    assert!(matches!(
        load(1 << 20, total - 1),
        Err(SoundLoadError::TooLarge { .. })
    ));
}

// ---------------------------------------------------------------------------------------------
// doom-sound mixer.rs
// ---------------------------------------------------------------------------------------------

/// Two mixers are equal when they read the same loaded set, hold the same busy channels and the
/// same handle counter (`PartialEq for Mixer`, documented there). Each of the three conditions
/// alone makes them unequal.
/// Kills mixer.rs `Mixer::eq` -> `true`, and `&&` -> `||` at both operators.
#[test]
fn two_mixers_are_equal_only_if_set_channels_and_counter_all_agree() {
    let set = sound_set("m5c_sfx.wad");
    let mut a = Mixer::new(set.clone());
    assert_eq!(a, Mixer::new(set.clone()));
    a.start(sfx(PISTOL), 8, 128, 128, 0).unwrap();
    assert_eq!(a, a.clone());
    // Same set, same counter, different channel (the read position moved).
    let mut b = a.clone();
    let mut out: Buffer = [[0; 2]; 512];
    b.mix_buffer(&mut out);
    assert_eq!(a.handle_counter(), b.handle_counter());
    assert_ne!(a.channel_states(), b.channel_states());
    assert_ne!(a, b);
    // Same set, same (empty) channels, different counter: a start that has finished.
    let mut c = Mixer::new(set.clone());
    c.start(sfx(PISTOL), 8, 128, 128, 0).unwrap();
    for _ in 0..1000 {
        if c.mask() == 0 {
            break;
        }
        c.mix_buffer(&mut out);
    }
    assert_eq!(c.mask(), 0, "the pistol ends");
    let fresh = Mixer::new(set);
    assert_eq!(c.channel_states(), fresh.channel_states());
    assert_ne!(c.handle_counter(), fresh.handle_counter());
    assert_ne!(c, fresh);
    // A separately loaded set with the same bytes is another set (`Arc::ptr_eq`).
    let other = Mixer::new(sound_set("m5c_sfx.wad"));
    assert_eq!(other.channel_states(), fresh.channel_states());
    assert_eq!(other.handle_counter(), fresh.handle_counter());
    assert_ne!(other, fresh);
}

/// A busy channel's checks: a volume row up to 127 is accepted on either side and 128 refused
/// (i_sound.c:364-368 refuse a start above 127), a remainder up to 65535 is accepted and 65536
/// refused (i_sound.c:603 keeps 16 bits).
/// Kills mixer.rs `MixChannelState::check`'s `right > 127` -> `>=`.
#[test]
fn a_channel_state_accepts_volume_rows_up_to_127_on_both_sides() {
    let ok = MixChannelState {
        sfx: sfx(PISTOL),
        pos: 3,
        end: 512,
        step: STEPTABLE[128],
        frac: 0xffff,
        start: 0,
        handle: 100,
        left: 127,
        right: 127,
    };
    assert_eq!(ok.check(2), Ok(()));
    assert_eq!(
        MixChannelState { left: 128, ..ok }.check(2),
        Err(MixStateError::Volume { slot: 2 })
    );
    assert_eq!(
        MixChannelState { right: 128, ..ok }.check(2),
        Err(MixStateError::Volume { slot: 2 })
    );
    assert_eq!(
        MixChannelState {
            frac: 0x10000,
            ..ok
        }
        .check(2),
        Err(MixStateError::Remainder { slot: 2 })
    );
}

/// `addsfx` hands out `handlenums`, 0 becoming 100 first (i_sound.c:335-340): the handle a start
/// returns is the one `next_handle` announced, which is then one higher; a restored counter is
/// announced as it is.
/// Kills mixer.rs `Mixer::next_handle` -> `-1`, `0`, `1`, and `==` -> `!=` in it.
#[test]
fn next_handle_announces_the_handle_the_next_start_returns() {
    let set = sound_set("m5c_sfx.wad");
    let mut m = Mixer::new(set.clone());
    assert_eq!(m.next_handle(), 100);
    for want in 100..104 {
        assert_eq!(m.next_handle(), want);
        let s = m.start(sfx(2), 8, 128, 128, 0).unwrap();
        assert_eq!(s.handle, want);
    }
    assert_eq!(m.next_handle(), 104);
    let restored = Mixer::from_state(set, 500, &[None; 8]).unwrap();
    assert_eq!(restored.next_handle(), 500);
}

// ---------------------------------------------------------------------------------------------
// doom-sound schedule.rs
// ---------------------------------------------------------------------------------------------

/// `Schedule::at` accepts a schedule that has run up to `MAX_TIC + 1` tics (the state after the
/// last tic `advanced` can run, whose `gametics` then refuses) and refuses one more.
/// Kills schedule.rs `tic > MAX_TIC + 1` with `>` -> `>=` and `+` -> `*`.
#[test]
fn a_schedule_can_be_restored_after_the_last_tic_but_not_beyond() {
    let end = MAX_TIC + 1;
    let s = Schedule::at(end, Schedule::buffers_before(end)).unwrap();
    assert_eq!(s.tic(), end);
    assert_eq!(s.gametics(), Err(ScheduleError::TicLimit));
    assert_eq!(
        Schedule::at(end + 1, Schedule::buffers_before(end + 1)),
        Err(ScheduleError::TicLimit)
    );
}

// ---------------------------------------------------------------------------------------------
// doom-sound session.rs
// ---------------------------------------------------------------------------------------------

struct Slots {
    button: Option<[i32; 2]>,
}
impl OriginPositions for Slots {
    fn actor_xy(&self, _: ActorId) -> Option<[i32; 2]> {
        None
    }
    fn sector_xy(&self, _: u16) -> Option<[i32; 2]> {
        None
    }
    fn button_xy(&self, _: u8) -> Option<[i32; 2]> {
        self.button
    }
}

/// Decision T12: at every `S_UpdateSounds` read of a release origin (s_sound.c:584-595), also
/// the read that finds the channel inaudible and frees it (s_sound.c:592-595), the detector asks
/// whether the 32-bit reading could differ. Here the declared reading (timer, 0) = (1000 units,
/// 0) from a listener at (0, 1000 units) is 1500 units away (beyond S_CLIPPING_DIST, so the
/// channel is freed as inaudible), while texture 0's reading (0, timer) is at distance 0 (audible):
/// the read must fire at that tic. The start has timer 0 (a free next slot) and never fires.
/// Kills session.rs `s.reason == StopReason::Inaudible` -> `!=` in `Sound::tic`.
#[test]
fn an_inaudible_release_read_in_the_update_reaches_the_detector() {
    let timer = 1000 << 16;
    let l = Listener {
        actor: ActorId::from_parts(0, 0),
        xy: [0, timer],
        angle: 0,
    };
    let mut s = Sound::new(sound_set("m5c_sfx.wad"), SoundConfig::fresh(1, 1)).unwrap();
    let start = SoundRecord::Start {
        sfx: 23,
        origin: SoundOrigin::Button(0),
        at: Some(SoundPlacement {
            origin: [0, 0],
            listener: l.xy,
            listener_angle: 0,
        }),
    };
    // Tic 0: the release starts with its next slot free (both builds read 0, 0), is audible
    // (1000 units) and stays; the update reads (0, 0) again.
    let t = s
        .tic(
            &[start],
            0,
            &l,
            &Slots {
                button: Some([0, 0]),
            },
        )
        .unwrap();
    assert_eq!(t.draws, 1);
    assert!(s.logic().channels()[0].is_some());
    assert_eq!(s.detector().firings(), 0);
    // Tic 1: the next slot is busy now; the read is inaudible, the channel freed, the detector
    // fires with this tic as the label.
    let t = s
        .tic(
            &[],
            0,
            &l,
            &Slots {
                button: Some([timer, 0]),
            },
        )
        .unwrap();
    assert_eq!(
        t.update
            .stops
            .iter()
            .map(|x| (x.cnum, x.reason))
            .collect::<Vec<_>>(),
        [(0, StopReason::Inaudible)]
    );
    assert_eq!(s.detector().layout_dependent_from(), Some(1));
    assert_eq!(s.detector().firings(), 1);
}

/// `SimulationPositions::button_xy` reads the declared 64-bit position of a release origin from
/// the simulation (M5c decision T12; p_spec.c:1151): in SNDBTN2 slot 0 releases while slot 1 is
/// busy and the genuine stream's S row at tic 128 gives its position (34, 0)
/// (tests/reference/m5c_sound/SNDBTN2.sound, S,128,6,23,button,0,...,34,0; M5C.md records the
/// 64-bit runner reading 34, 0). The tic (128) and the position (34, 0) below are transcribed
/// from that row, not read from the file at run time; the test checks the position through
/// `SimulationPositions` at the state before that tic, and that the provider agrees with the
/// simulation's own reading at every tic (Rust against Rust: it shows the provider passes the
/// reading on unchanged, not that the reading is the C's).
/// Kills session.rs `<impl OriginPositions for SimulationPositions>::button_xy` -> `Some([0; 2])`.
/// (A gate run cannot see this mutant: the update's use of the position is audibility only, and
/// 34 raw units against 0 never change it, so the provider's value is checked directly.)
#[test]
fn the_simulation_provider_reads_the_busy_next_slot_as_the_position() {
    use common::gate::{Carry, load_item, run};
    use doom_sound::session::SimulationPositions;
    let index = common::wire25::ITEMS
        .iter()
        .position(|i| i.0 == "SNDBTN2")
        .expect("SNDBTN2 is an item");
    let item = load_item(index);
    let sound = Sound::new(sound_set("m5c_sfx.wad"), item.config).unwrap();
    let mut seen = Vec::new();
    let mut disagreements = 0;
    run(
        &item.commands,
        item.sim,
        sound,
        Carry::default(),
        130,
        None,
        |tic, sim, _, _| {
            let provider = SimulationPositions(sim);
            for slot in 0..15u8 {
                let want = sim.button_release_xy(slot).ok();
                if provider.button_xy(slot) != want {
                    disagreements += 1;
                }
            }
            if provider.button_xy(0) == Some([34, 0]) {
                seen.push(tic);
            }
        },
    );
    assert_eq!(disagreements, 0);
    assert_eq!(seen, [128], "the genuine release reads (34, 0) at tic 128");
}

// ---------------------------------------------------------------------------------------------
// doom-sound state.rs
// ---------------------------------------------------------------------------------------------

/// The largest valid `DSND` state: 64 occupied actor channels, 8 busy mixer channels, the
/// largest accepted texture count, `snd_SfxVolume` 127 and 64 channels.
fn largest_state() -> SoundState {
    let tic = 40;
    let channels = (0..64u32)
        .map(|i| {
            Some(ChannelEntry {
                sfx: sfx(PISTOL),
                origin: SoundOrigin::Actor(ActorId::from_parts(i, 1)),
                handle: 100 + i as i32,
            })
        })
        .collect();
    let mixer = std::array::from_fn(|i| {
        Some(MixChannelState {
            sfx: sfx(2),
            pos: 1,
            end: 512,
            step: STEPTABLE[128],
            frac: 7,
            start: i as i32,
            handle: 200 + i as i32,
            left: 3,
            right: 4,
        })
    });
    let mut looked_up = [false; NUMSFX];
    looked_up[PISTOL as usize] = true;
    SoundState {
        config: SoundConfig {
            logic: LogicConfig {
                gamemap: 1,
                sfx_volume: 127,
                channels: 64,
            },
            textures: MAX_TEXTURES,
        },
        schedule: Schedule::at(tic, Schedule::buffers_before(tic)).unwrap(),
        detector: LayoutDetector::new(MAX_TEXTURES).unwrap(),
        looked_up,
        channels,
        handlenums: 300,
        mixer,
    }
}

/// `MAX_BYTES` is "the largest valid encoding" (its doc comment: the 63-byte header, 64 actor
/// channels of 19 bytes, the 2-byte counter and 8 mixer channels of 28 bytes): the largest state
/// encodes to exactly that many bytes and decodes, one byte more is `TooLong`; the limits of the
/// header (volume 127, 64 channels, `MAX_TEXTURES` textures) are accepted.
/// Kills state.rs `MAX_BYTES`'s arithmetic (all nine `+`/`*` mutants), `bytes.len() > MAX_BYTES`
/// -> `>=`, `sfx_volume > 127 || channels > MAX_CHANNELS` with each `>` -> `>=`, and
/// `textures > MAX_TEXTURES` -> `>=` and `==` in `SoundState::decode`.
#[test]
fn the_largest_valid_state_is_exactly_max_bytes_and_the_header_limits_are_inclusive() {
    let state = largest_state();
    let bytes = state.encode();
    assert_eq!(bytes.len(), 63 + 64 * 19 + 2 + 8 * 28);
    assert_eq!(bytes.len(), MAX_BYTES);
    assert_eq!(SoundState::decode(&bytes), Ok(state));
    let mut longer = bytes.clone();
    longer.push(0);
    assert_eq!(SoundState::decode(&longer), Err(StateError::TooLong));
}

/// An unlabelled detector writes 0 in the first-tic field (`encode`), and `decode` refuses any
/// other value there (the documented canonical form: the first tic is present exactly when the
/// label is), even when the firings count is 0 so that the detector's own label check is
/// satisfied. Bytes 33..41 are the first-tic field (4 magic, 2 version, 8 tic, 8 buffers, 1
/// volume, 1 channels, 4 map, 4 textures, 1 label).
/// Kills state.rs `(!labelled && first != 0)`'s `||` -> `&&` (the `||` before it) in
/// `SoundState::decode`.
#[test]
fn an_unlabelled_state_with_a_first_tic_is_refused() {
    let bytes = largest_state().encode();
    assert_eq!(bytes[32], 0, "unlabelled");
    assert_eq!(&bytes[33..41], &[0; 8]);
    assert_eq!(&bytes[41..49], &[0; 8], "no firings");
    let mut forged = bytes.clone();
    forged[33] = 1;
    assert_eq!(SoundState::decode(&forged), Err(StateError::Detector));
}

// ---------------------------------------------------------------------------------------------
// Tests for the mutants the final M5c mutation run (a8abf0c) missed: the two Display texts.
// ---------------------------------------------------------------------------------------------

/// The message of every `SoundRefusal` is plain text that names the facts of the refusal (a
/// Rust-only pin of the wording, which the C does not have, except that the mixer's bounds
/// messages repeat the C's own `I_Error` texts, i_sound.c:364-368). The expected strings are the
/// wording the source has now, one per variant; a message with no text, or one that drops the
/// number, fails.
/// Kills logic.rs `<impl Display for SoundRefusal>::fmt` -> `Ok(Default::default())`.
#[test]
fn every_sound_refusal_has_a_message_that_names_its_facts() {
    use doom_sound::mixer::MixRefusal;
    let cases = [
        (
            SoundRefusal::SfxId { sfx: 109 },
            "sound number 109 is not in the sound table",
        ),
        (
            SoundRefusal::LumpMissing { sfx: 7 },
            "sound 7 has no lump in the WADs",
        ),
        (
            SoundRefusal::Mix(MixRefusal::RightVolume { vol: 1, sep: 2 }),
            "mixer: rightvol out of bounds",
        ),
        (
            SoundRefusal::ButtonPastArray { slot: 15 },
            "a switch release in button slot 15 would read past the button list",
        ),
        (
            SoundRefusal::Placement,
            "a sound start's positions do not fit its origin",
        ),
        (
            SoundRefusal::OriginGone { cnum: 2 },
            "the origin of the sound on channel 2 has no position to read",
        ),
        (
            SoundRefusal::Config,
            "the sound volume or channel count is out of range",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), text, "{refusal:?}");
    }
}

/// The message of every `SessionRefusal` names the tic, the call or the two tics it concerns (a
/// Rust-only pin of the wording, as the expected strings are the wording the source has now); a refusal of the logic carries the logic's message after the
/// place (`tic N, sound call I:` for a record, `tic N, sound update:` for the update).
/// Kills session.rs `<impl Display for SessionRefusal>::fmt` -> `Ok(Default::default())`.
#[test]
fn every_session_refusal_has_a_message_that_names_its_facts() {
    use doom_sound::session::{RefusedAt, SessionRefusal};
    let cases = [
        (
            SessionRefusal::Sound {
                tic: 5,
                at: RefusedAt::Record(3),
                refusal: SoundRefusal::Placement,
            },
            "tic 5, sound call 3: a sound start's positions do not fit its origin",
        ),
        (
            SessionRefusal::Sound {
                tic: 6,
                at: RefusedAt::Update,
                refusal: SoundRefusal::OriginGone { cnum: 1 },
            },
            "tic 6, sound update: the origin of the sound on channel 1 has no position to read",
        ),
        (
            SessionRefusal::TicLimit,
            "the sound session reached its tic limit",
        ),
        (
            SessionRefusal::DrawCount {
                tic: 8,
                record: 65_536,
            },
            "tic 8, sound call 65536: more than 65535 random draws in one tic",
        ),
        (
            SessionRefusal::NoListener,
            "the player has no body to hear sounds",
        ),
        (
            SessionRefusal::TicMismatch {
                session: 4,
                simulation: 9,
            },
            "the sound session is at tic 4 but the simulation at tic 9",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), text, "{refusal:?}");
    }
}
