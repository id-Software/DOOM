//! Rules of the sound logic (s_sound.c) from cited C facts: channel choice, priority eviction,
//! same-origin replacement, the `M_Random` draw rule, `S_UpdateSounds`, and every refusal with
//! the state preserved. The expected values come from the C text and from the genuine runs of the
//! M5c note §1.1.9 (Test 3 and Test 4), never from Rust output.

mod common;

use std::sync::Arc;

use common::sound_set;
use doom_sim::ActorId;
use doom_sound::logic::{
    Listener, LogicConfig, OriginPositions, SoundLogic, SoundOrigin, SoundPlacement, SoundRecord,
    SoundRefusal, StartOutcome, StopReason,
};
use doom_sound::lumps::SoundSet;
use doom_sound::mixer::{MixRefusal, Mixer};
use doom_types::DoomRng;

const POSACT: u16 = 75; // priority 120 (sounds.c:193)
const BGACT: u16 = 76; // 120
const DMACT: u16 = 77; // 120
const STNMOV: u16 = 22; // 119 (sounds.c:140)
const SAWIDL: u16 = 11; // 118 (sounds.c:129)
const SAWUP: u16 = 10;
const TELEPT: u16 = 35; // 32 (sounds.c:153)
const OOF: u16 = 34; // 96
const NOWAY: u16 = 81; // 78
const SLOP: u16 = 31; // 78
const PISTOL: u16 = 1;
const ITEMUP: u16 = 32;
const TINK: u16 = 87;
const CHGUN: u16 = 86;

struct World {
    logic: SoundLogic,
    mixer: Mixer,
    rng: DoomRng,
}

fn world(set: Arc<SoundSet>, config: LogicConfig) -> World {
    World {
        logic: SoundLogic::new(config).unwrap(),
        mixer: Mixer::new(set),
        rng: DoomRng::default(),
    }
}

/// The edge WAD has every sound lump except dspstart, dschgun included; the main WAD has no
/// dschgun (tools/reference/m5c_sfx.py writes the 107 names I_InitSound loads), so a chgun start
/// there is refused as a missing lump (s_sound.c:363-365).
fn edge_world() -> World {
    world(sound_set("m5c_sfx_edge.wad"), LogicConfig::fresh(1))
}

fn main_world() -> World {
    world(sound_set("m5c_sfx.wad"), LogicConfig::fresh(1))
}

fn actor(n: u32) -> SoundOrigin {
    SoundOrigin::Actor(ActorId::from_parts(n, 1))
}

impl World {
    fn start(
        &mut self,
        sfx: u16,
        origin: SoundOrigin,
        at: Option<SoundPlacement>,
        gametic: i32,
    ) -> Result<doom_sound::logic::StartEcho, SoundRefusal> {
        self.logic
            .start(sfx, origin, at, &mut self.rng, &mut self.mixer, gametic)
    }
    fn table(&self) -> Vec<Option<(u16, SoundOrigin)>> {
        self.logic
            .channels()
            .iter()
            .map(|c| c.map(|e| (e.sfx.get(), e.origin)))
            .collect()
    }
    fn snapshot(&self) -> (SoundLogic, Mixer, DoomRng) {
        (self.logic.clone(), self.mixer.clone(), self.rng)
    }
}

/// M5c note §1.1.9 Test 3 (genuine, gametic held at 0), and the priority rule of
/// s_sound.c:853-854 (`>=`, first in index order).
#[test]
fn allocation_and_priority_follow_the_genuine_test_3() {
    let mut w = main_world();
    let (a, b, c) = (actor(1), actor(2), actor(3));
    for (sfx, o) in [(POSACT, a), (STNMOV, b), (SAWIDL, c)] {
        assert_eq!(
            w.start(sfx, o, None, 0).unwrap().outcome,
            StartOutcome::Started
        );
    }
    assert_eq!(
        w.table(),
        [Some((POSACT, a)), Some((STNMOV, b)), Some((SAWIDL, c))]
    );
    // bgact (120) evicts channel 0 (first with priority >= 120); dmact (120) again (equal evicts).
    let d = actor(4);
    let e = w.start(BGACT, d, None, 0).unwrap();
    assert_eq!(
        (e.cnum, e.stops[1].map(|s| (s.cnum, s.reason))),
        (Some(0), Some((0, StopReason::Priority)))
    );
    let e = w.start(DMACT, actor(5), None, 0).unwrap();
    assert_eq!(e.cnum, Some(0));
    // telept (32) evicts channel 0, the first in index order with >= 32.
    let t = actor(6);
    assert_eq!(w.start(TELEPT, t, None, 0).unwrap().cnum, Some(0));
    // A NULL-origin oof (96) evicts channel 1 (stnmov 119).
    let e = w.start(OOF, SoundOrigin::None, None, 0).unwrap();
    assert_eq!(
        (e.cnum, e.stops[1].map(|s| s.entry.sfx.get())),
        (Some(1), Some(STNMOV))
    );
    // A NULL-origin noway takes channel 1 again: S_StopSound(NULL) frees the oof (s_sound.c:349).
    let e = w.start(NOWAY, SoundOrigin::None, None, 0).unwrap();
    assert_eq!(
        e.stops[0].map(|s| (s.cnum, s.reason)),
        Some((1, StopReason::SameOrigin))
    );
    assert_eq!(e.cnum, Some(1));
    // slop from the origin of channel 0 reuses channel 0.
    let e = w.start(SLOP, t, None, 0).unwrap();
    assert_eq!((e.cnum, e.stops[0].map(|s| s.cnum)), (Some(0), Some(0)));
    assert_eq!(
        w.table(),
        [
            Some((SLOP, t)),
            Some((NOWAY, SoundOrigin::None)),
            Some((SAWIDL, c))
        ]
    );
    // Handles 100, 101, ... in the order of all I_StartSound calls (i_sound.c:335-340).
    assert_eq!(w.logic.channels()[2].unwrap().handle, 102);
}

/// M5c note §1.1.9 Test 4 and s_sound.c:283-346: one draw per start past the link-volume and
/// audibility tests, none for itemup and tink; a start without a channel has drawn.
#[test]
fn the_draw_rule_follows_the_genuine_test_4() {
    let draws = |w: &mut World, sfx, o, at, g| {
        let before = w.rng.indices()[1];
        let e = w.start(sfx, o, at, g).unwrap();
        assert_eq!(w.rng.indices()[1].wrapping_sub(before), e.draws());
        (e.outcome, e.draws())
    };
    let mut w = edge_world();
    assert_eq!(
        draws(&mut w, ITEMUP, SoundOrigin::None, None, 0),
        (StartOutcome::Started, 0)
    );
    assert_eq!(
        draws(&mut w, TINK, SoundOrigin::None, None, 0),
        (StartOutcome::Started, 0)
    );
    assert_eq!(
        draws(&mut w, SAWUP, SoundOrigin::None, None, 0),
        (StartOutcome::Started, 1)
    );
    assert_eq!(
        draws(&mut w, PISTOL, SoundOrigin::None, None, 0),
        (StartOutcome::Started, 1)
    );
    let far = SoundPlacement {
        origin: [5000 << 16, 0],
        listener: [0, 0],
        listener_angle: 0,
    };
    assert_eq!(
        draws(&mut w, PISTOL, actor(9), Some(far), 0),
        (StartOutcome::Inaudible, 0)
    );
    let mut w8 = world(sound_set("m5c_sfx.wad"), LogicConfig::fresh(8));
    assert_eq!(
        draws(&mut w8, PISTOL, actor(9), Some(far), 0),
        (StartOutcome::Started, 1)
    );
    // The linked chgun draws (its link volume 0 keeps it at 8) ...
    assert_eq!(
        draws(&mut w, CHGUN, SoundOrigin::None, None, 0),
        (StartOutcome::Started, 1)
    );
    // ... and at snd_SfxVolume 0 returns before the draw (s_sound.c:287-290).
    let mut w0 = world(
        sound_set("m5c_sfx.wad"),
        LogicConfig {
            sfx_volume: 0,
            ..LogicConfig::fresh(1)
        },
    );
    assert_eq!(
        draws(&mut w0, CHGUN, SoundOrigin::None, None, 0),
        (StartOutcome::LinkVolume, 0)
    );
    // A NULL-origin pistol at volume 0 still draws (no volume test for non-positional starts).
    assert_eq!(
        draws(&mut w0, PISTOL, SoundOrigin::None, None, 0),
        (StartOutcome::Started, 1)
    );
    // posact (120) against three telept (32): no channel, one draw, the table unchanged.
    let mut w = main_world();
    for n in 1..=3 {
        w.start(TELEPT, actor(n), None, 0).unwrap();
    }
    let table = w.table();
    assert_eq!(
        draws(&mut w, POSACT, actor(7), None, 0),
        (StartOutcome::NoChannel, 1)
    );
    assert_eq!(w.table(), table);
}

/// s_sound.c:327-346: the drawn value is `rndtable[++rndindex]` and the pitch 128 + 8 - (v & 15)
/// for the saws, 128 + 16 - (v & 31) otherwise; the chgun's base is its link pitch 150.
#[test]
fn pitches_come_from_the_drawn_values() {
    let mut w = edge_world();
    let mut r = DoomRng::default();
    for (sfx, base, mask, add) in [
        (SAWUP, 128, 15, 8),
        (PISTOL, 128, 31, 16),
        (CHGUN, 150, 31, 16),
    ] {
        let v = r.m_random();
        let e = w.start(sfx, SoundOrigin::None, None, 0).unwrap();
        assert_eq!(e.rnd, Some(v));
        assert_eq!(e.args.unwrap()[2], base + add - i32::from(v & mask));
    }
    // Non-positional starts: volume snd_SfxVolume, separation NORM_SEP, priority 64 (or the
    // link's table priority, s_sound.c:286).
    let e = w.start(ITEMUP, SoundOrigin::None, None, 0).unwrap();
    assert_eq!(e.args, Some([8, 128, 128, 64]));
}

/// s_sound.c:312-316: an origin on the listener's x, y is centred; the volume is the close one.
#[test]
fn an_origin_on_the_listener_is_centred() {
    let mut w = main_world();
    let p = SoundPlacement {
        origin: [64 << 16, 32 << 16],
        listener: [64 << 16, 32 << 16],
        listener_angle: 0x1234_5678,
    };
    let e = w.start(PISTOL, actor(3), Some(p), 0).unwrap();
    assert!(e.samexy);
    assert_eq!(e.args.unwrap()[..2], [8, 128]);
}

/// Decision T11 and §4.6.5: every refusal leaves the channel table, the random index and the
/// mixer unchanged.
#[test]
fn refusals_preserve_the_state() {
    let edge = sound_set("m5c_sfx_edge.wad");
    let mut w = world(edge, LogicConfig::fresh(1));
    w.start(PISTOL, actor(1), None, 0).unwrap();
    let snap = w.snapshot();
    let check = |w: &World| {
        assert_eq!(w.logic, snap.0);
        assert_eq!(w.mixer, snap.1);
        assert_eq!(w.rng, snap.2);
    };
    // s_sound.c:277-278 (0) and the off-by-one 109 (s_sound.c:277-280).
    assert_eq!(
        w.start(0, SoundOrigin::None, None, 1),
        Err(SoundRefusal::SfxId { sfx: 0 })
    );
    assert_eq!(
        w.start(109, SoundOrigin::None, None, 1),
        Err(SoundRefusal::SfxId { sfx: 109 })
    );
    check(&w);
    // pstart (18) has no lump in the edge WAD: refused at the start that obtains a channel
    // (s_sound.c:363-365), after its draw and channel choice, with neither kept.
    assert_eq!(
        w.start(18, actor(2), None, 1),
        Err(SoundRefusal::LumpMissing { sfx: 18 })
    );
    check(&w);
    // dbopn (5) is a 0-byte lump: the mixer start is refused (padded length 0).
    assert_eq!(
        w.start(5, actor(2), None, 1),
        Err(SoundRefusal::Mix(MixRefusal::ShortLump { sfx: 5 }))
    );
    check(&w);
    // The switch release of slot 15 reads past buttonlist (p_spec.c:1151, MAXBUTTONS 16).
    assert_eq!(
        w.start(24, SoundOrigin::Button(15), None, 1),
        Err(SoundRefusal::ButtonPastArray { slot: 15 })
    );
    check(&w);
    let p = SoundPlacement {
        origin: [0, 0],
        listener: [0, 0],
        listener_angle: 0,
    };
    assert_eq!(
        w.start(PISTOL, SoundOrigin::None, Some(p), 1),
        Err(SoundRefusal::Placement)
    );
    check(&w);
    // Review finding M1 (review-sound): a sector or button origin is never the listener, so
    // s_sound.c:304-320 always measures it; such a start without a placement is refused, not
    // played as the listener's own sound (and a switch release never skips the T12 detector).
    for origin in [SoundOrigin::Sector(3), SoundOrigin::Button(0)] {
        assert_eq!(
            w.start(20, origin, None, 1),
            Err(SoundRefusal::Placement),
            "{origin:?}"
        );
        check(&w);
        let mut placed = edge_world();
        assert_eq!(
            placed.start(20, origin, Some(p), 1).map(|e| e.outcome),
            Ok(StartOutcome::Started),
            "{origin:?} with a placement"
        );
    }
    // With no channel free the lump is never looked up (s_sound.c:352-355 before 363-365): the
    // missing-lump start is then a plain no-channel start.
    let mut w = world(sound_set("m5c_sfx_edge.wad"), LogicConfig::fresh(1));
    for n in 1..=3 {
        w.start(TELEPT, actor(n), None, 0).unwrap();
    }
    assert_eq!(
        w.start(18, actor(9), None, 0).unwrap().outcome,
        StartOutcome::NoChannel
    );
    assert!(
        SoundLogic::new(LogicConfig {
            sfx_volume: 128,
            ..LogicConfig::fresh(1)
        })
        .is_err()
    );
}

/// `I_GetSfxLumpNum` runs once per sound, at its first start that obtains a channel.
#[test]
fn the_lump_is_looked_up_once_per_sound() {
    let mut w = main_world();
    assert!(
        w.start(PISTOL, SoundOrigin::None, None, 0)
            .unwrap()
            .lump_lookup
    );
    assert!(
        !w.start(PISTOL, SoundOrigin::None, None, 0)
            .unwrap()
            .lump_lookup
    );
}

struct Positions(Vec<(ActorId, [i32; 2])>);
impl OriginPositions for Positions {
    fn actor_xy(&self, a: ActorId) -> Option<[i32; 2]> {
        self.0.iter().find(|(b, _)| *b == a).map(|(_, xy)| *xy)
    }
    fn sector_xy(&self, _: u16) -> Option<[i32; 2]> {
        None
    }
    fn button_xy(&self, _: u8) -> Option<[i32; 2]> {
        None
    }
}

/// s_sound.c:553-607: a channel is freed when `gametic >= handle`, or when a positional origin
/// is inaudible now; the listener's own and NULL origins are left alone; a missing position
/// refuses with the table unchanged.
#[test]
fn update_frees_finished_and_inaudible_channels() {
    let mut w = main_world();
    let me = ActorId::from_parts(1, 1);
    let near = SoundPlacement {
        origin: [100 << 16, 0],
        listener: [0, 0],
        listener_angle: 0,
    };
    w.start(PISTOL, SoundOrigin::Actor(me), None, 0).unwrap(); // handle 100, the listener's own
    w.start(OOF, actor(2), Some(near), 0).unwrap(); // 101
    w.start(ITEMUP, SoundOrigin::None, None, 0).unwrap(); // 102
    let listener = Listener {
        actor: me,
        xy: [0, 0],
        angle: 0,
    };
    // Actor 2 still near: nothing freed, its parameters go to I_UpdateSoundParams.
    let pos = Positions(vec![(ActorId::from_parts(2, 1), [100 << 16, 0])]);
    let e = w.logic.update(&listener, &pos, 1).unwrap();
    assert!(e.stops.is_empty());
    // Bearing 0 equals the facing 0, so the off-by-one of s_sound.c:785-788 gives 0xffffffff,
    // finesine[8191] is negative and the separation 129 (the genuine BARON1 stream's first row
    // has the same 129); distance 100 < 160 gives snd_SfxVolume (s_sound.c:796-799).
    assert_eq!(e.params, [(1, 8, 129, 128)]);
    // A missing position refuses with the table unchanged.
    let before = w.logic.clone();
    assert_eq!(
        w.logic.update(&listener, &Positions(vec![]), 1),
        Err(SoundRefusal::OriginGone { cnum: 1 })
    );
    assert_eq!(w.logic, before);
    // Actor 2 far away: freed as inaudible; at gametic 102 the handles 100 and 101 are finished.
    let far = Positions(vec![(ActorId::from_parts(2, 1), [3000 << 16, 0])]);
    let e = w.logic.update(&listener, &far, 1).unwrap();
    assert_eq!(
        e.stops
            .iter()
            .map(|s| (s.cnum, s.reason, s.playing))
            .collect::<Vec<_>>(),
        [(1, StopReason::Inaudible, true)]
    );
    let e = w.logic.update(&listener, &far, 102).unwrap();
    assert_eq!(
        e.stops
            .iter()
            .map(|s| (s.cnum, s.reason, s.playing))
            .collect::<Vec<_>>(),
        [
            (0, StopReason::Finished, false),
            (2, StopReason::Finished, false)
        ]
    );
    assert!(w.logic.channels().iter().all(Option::is_none));
}

/// S_Start frees every channel (s_sound.c:209-211); a removal stop frees the actor's channel
/// (p_mobj.c:565-566).
#[test]
fn level_start_and_removal_free_channels() {
    let mut w = main_world();
    w.start(PISTOL, actor(1), None, 0).unwrap();
    w.start(OOF, actor(2), None, 0).unwrap();
    let rec = SoundRecord::Stop {
        actor: ActorId::from_parts(2, 1),
    };
    let e = w.logic.apply(&rec, &mut w.rng, &mut w.mixer, 0).unwrap();
    assert!(
        matches!(e, doom_sound::logic::RecordEcho::Stop(Some(s)) if s.cnum == 1 && s.reason == StopReason::Remove)
    );
    let stops = w.logic.level_start(0);
    assert_eq!(stops.iter().map(|s| s.cnum).collect::<Vec<_>>(), [0]);
    assert!(w.logic.channels().iter().all(Option::is_none));
}

/// Review finding N3 (review-sound): the mixer's refusals come in the C's order. For a 0-byte lump
/// (dbopn, 5, in the edge WAD) started with an out-of-range volume, the volume check of addsfx
/// (i_sound.c:364-365) comes first; the short lump's byte would be read only at the next mix
/// (i_sound.c:591). A pitch outside the table is read before both (i_sound.c:495).
#[test]
fn the_mixer_refuses_in_the_c_order() {
    let w = edge_world();
    let dbopn = doom_sound::sounds::SfxId::new(5).unwrap();
    assert_eq!(
        w.mixer.check_start(dbopn, 1000, 128, 128),
        Err(MixRefusal::RightVolume {
            vol: 1000,
            sep: 128
        })
    );
    assert_eq!(
        w.mixer.check_start(dbopn, 1000, 128, 256),
        Err(MixRefusal::Pitch { pitch: 256 })
    );
    assert_eq!(
        w.mixer.check_start(dbopn, 8, 128, 128),
        Err(MixRefusal::ShortLump { sfx: 5 })
    );
}
