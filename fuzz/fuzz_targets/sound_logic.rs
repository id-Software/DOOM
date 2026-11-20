#![no_main]
//! An arbitrary sequence of sound records, updates, stops and level starts into
//! `doom_sound::logic::SoundLogic` with a `Mixer` and the `M_Random` stream (M5c), over a
//! committed synthetic sound WAD (no id data).
//!
//! THIN ON PURPOSE. The session, saved-state and layout-dependence detector modules of doom-sound
//! are not written yet. Another step will extend the logic API, and a second fuzz pass will follow
//! with those entry points; this target covers only what `SoundLogic` exposes today (`new`,
//! `start`, `apply`, `stop`, `update`, `level_start`).
//!
//! Input: byte 0 the set (`% 2`, as in `sound_mixer`), byte 1 the map (`% 3`: 1, 8 or 3), byte 2
//! the channel count (`% 8`, with 7 meaning 65, which `SoundLogic::new` refuses), byte 3
//! `snd_SfxVolume` (`% 130`, so 128 and 129 are refused), then eight-byte ops `op a b c d e f g`,
//! at most 128 ops (`-max_len=1060`). `op % 6`: 0 start of sound `a` (any `u8`: 0 and 109 and above
//! are refused) with origin `b % 5` (none, actor `c % 6`, sector `c`, button `c % 16`, the
//! listener's body) and, when `op & 0x10`, a placement whose coordinates are `i8 << 16` from
//! `d`, `e`, `f` and `g`; 1 `S_StopSound` of actor `a % 6`; 2 `S_UpdateSounds` with the listener
//! at `(d << 16, e << 16)` and angle `f << 24`, where an origin is "gone" when bit
//! `(cnum, kind)` of `a` says so; 3 `level_start`; 4 advance `gametic` by `a`; 5 mix one buffer.
//!
//! Invariants: every result is `Ok` or a typed `SoundRefusal`; a sector or button start without a
//! placement is refused (s_sound.c:304 always measures such an origin); on `Err` the logic, the mixer and
//! the random indices are all equal to clones taken before the call; the table keeps exactly the
//! configured number of channels; no non-NULL origin holds two channels after a start (s_sound.c:
//! 349 frees the first, 842-846); the gameplay index never moves; a start draws at most once and
//! exactly when it passed the link-volume and audibility tests and the sound is not itemup or
//! tink (s_sound.c:283-299, 318-346), never when its outcome is `LinkVolume` or `Inaudible`; a
//! started sound occupies the channel it reports with its handle, equal to the mixer's;
//! an update only frees occupied channels (`Finished` exactly when `gametic >= handle`,
//! s_sound.c:600-605) and refuses only with `OriginGone` for an occupied channel; `level_start`
//! empties the table and reports every occupied channel; a stop frees at most one channel.
use std::sync::Arc;

use doom_render::LumpDirectory;
use doom_sim::ActorId;
use doom_sound::logic::{
    Listener, LogicConfig, OriginPositions, RecordEcho, SoundLogic, SoundOrigin, SoundPlacement,
    SoundRecord, SoundRefusal, StartOutcome, StopReason,
};
use doom_sound::lumps::{SoundCaps, SoundSet};
use doom_sound::mixer::{Buffer, Mixer};
use doom_sound::sounds::{SFX_ITEMUP, SFX_TINK};
use doom_types::DoomRng;
use libfuzzer_sys::fuzz_target;

const MAIN: &[u8] = include_bytes!("../../tests/reference/m5c_sfx.wad");
const EDGE: &[u8] = include_bytes!("../../tests/reference/m5c_sfx_edge.wad");
const MAX_OPS: usize = 128;

fn sets() -> &'static [Arc<SoundSet>; 2] {
    static SETS: std::sync::OnceLock<[Arc<SoundSet>; 2]> = std::sync::OnceLock::new();
    SETS.get_or_init(|| {
        let load = |bytes: &[u8]| {
            let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
            let mut dir = LumpDirectory::new();
            dir.add_wad(&wad).unwrap();
            Arc::new(SoundSet::load(&dir, SoundCaps::default()).unwrap())
        };
        [load(MAIN), load(EDGE)]
    })
}

fn actor(n: u8) -> ActorId {
    ActorId::from_parts(u32::from(n), 1)
}

/// Positions a pure function of the origin; `gone` makes some of them unreadable.
struct Positions {
    gone: u8,
}
impl OriginPositions for Positions {
    fn actor_xy(&self, a: ActorId) -> Option<[i32; 2]> {
        let n = a.slot() as i32;
        (self.gone & 1 == 0).then_some([(n * 300 - 600) << 16, (n * 90) << 16])
    }
    fn sector_xy(&self, s: u16) -> Option<[i32; 2]> {
        (self.gone & 2 == 0).then_some([i32::from(s) << 12, i32::from(s) << 14])
    }
    fn button_xy(&self, b: u8) -> Option<[i32; 2]> {
        (self.gone & 4 == 0).then_some([i32::from(b) << 20, 0])
    }
}

fn occupied(logic: &SoundLogic) -> usize {
    logic.channels().iter().filter(|c| c.is_some()).count()
}

fn check_table(logic: &SoundLogic, channels: usize) {
    assert_eq!(logic.channels().len(), channels, "channel table size");
    for (i, c) in logic.channels().iter().enumerate() {
        let Some(c) = c else { continue };
        assert!((100..=65535).contains(&c.handle));
        if c.origin != SoundOrigin::None {
            let same = logic.channels()[i + 1..]
                .iter()
                .flatten()
                .filter(|o| o.origin == c.origin)
                .count();
            assert_eq!(same, 0, "one origin holds two channels");
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let set = sets()[usize::from(data[0] % 2)].clone();
    let gamemap = [1, 8, 3][usize::from(data[1] % 3)];
    let channels = match data[2] % 8 {
        7 => 65,
        n => usize::from(n),
    };
    let sfx_volume = i32::from(data[3] % 130);
    let config = LogicConfig {
        gamemap,
        sfx_volume,
        channels,
    };
    let mut logic = match SoundLogic::new(config) {
        Ok(l) => l,
        Err(e) => {
            assert_eq!(e, SoundRefusal::Config);
            assert!(!(0..=127).contains(&sfx_volume) || channels > 64);
            return;
        }
    };
    assert!((0..=127).contains(&sfx_volume) && channels <= 64);
    let mut mixer = Mixer::new(set);
    let mut rng = DoomRng::default();
    let mut gametic = 0i32;
    let listener_actor = actor(0);
    let mut buf: Buffer = [[0; 2]; 512];
    let ops = &data[4..];
    for o in ops.chunks_exact(8).take(MAX_OPS) {
        let [op, a, b, c, d, e, f, g] = [o[0], o[1], o[2], o[3], o[4], o[5], o[6], o[7]];
        let (logic0, mixer0, rng0) = (logic.clone(), mixer.clone(), rng);
        let unchanged = |logic: &SoundLogic, mixer: &Mixer, rng: &DoomRng| {
            assert_eq!(logic, &logic0, "refusal changed the logic");
            assert_eq!(mixer, &mixer0, "refusal changed the mixer");
            assert_eq!(rng, &rng0, "refusal changed the random index");
        };
        match op % 6 {
            0 => {
                let origin = match b % 5 {
                    0 => SoundOrigin::None,
                    1 => SoundOrigin::Actor(actor(c % 6)),
                    2 => SoundOrigin::Sector(u16::from(c)),
                    3 => SoundOrigin::Button(c % 16),
                    _ => SoundOrigin::Actor(listener_actor),
                };
                let at = (op & 0x10 != 0).then(|| {
                    let s = |x: u8| i32::from(x as i8) << 16;
                    SoundPlacement {
                        origin: [s(d), s(e)],
                        listener: [s(f), s(g)],
                        listener_angle: u32::from(g) << 24,
                    }
                });
                let record = SoundRecord::Start {
                    sfx: u16::from(a),
                    origin,
                    at,
                };
                let r = logic.apply(&record, &mut rng, &mut mixer, gametic);
                if at.is_none() && matches!(origin, SoundOrigin::Sector(_) | SoundOrigin::Button(_))
                {
                    assert!(
                        matches!(
                            r,
                            Err(SoundRefusal::Placement
                                | SoundRefusal::SfxId { .. }
                                | SoundRefusal::ButtonPastArray { .. })
                        ),
                        "an unplaced sector or button start was accepted: {r:?}"
                    );
                }
                match r {
                    Err(_) => unchanged(&logic, &mixer, &rng),
                    Ok(RecordEcho::Stop(_)) => panic!("a start answered with a stop"),
                    Ok(RecordEcho::Start(s)) => {
                        assert_eq!(rng.indices()[0], rng0.indices()[0], "gameplay index moved");
                        let moved = rng.indices()[1].wrapping_sub(rng0.indices()[1]);
                        assert_eq!(moved, s.draws(), "index movement differs from the draws");
                        assert!(s.draws() <= 1);
                        let exempt = s.sfx.get() == SFX_ITEMUP || s.sfx.get() == SFX_TINK;
                        match s.outcome {
                            StartOutcome::LinkVolume | StartOutcome::Inaudible => {
                                assert_eq!(s.draws(), 0);
                            }
                            StartOutcome::NoChannel | StartOutcome::Started => {
                                assert_eq!(s.draws() == 1, !exempt, "draw rule");
                            }
                        }
                        if s.outcome == StartOutcome::Started {
                            let cnum = s.cnum.expect("a started sound reports its channel");
                            let e = logic.channels()[cnum].expect("its channel is occupied");
                            assert_eq!(e.sfx, s.sfx);
                            assert_eq!(e.origin, origin);
                            assert_eq!(Some(e.handle), s.handle);
                            assert_eq!(Some(e.handle), s.mix.map(|m| m.handle));
                            assert!(mixer.mask() != 0);
                        } else {
                            assert!(s.cnum.is_none() && s.mix.is_none());
                            assert_eq!(mixer, mixer0, "no start, yet the mixer changed");
                        }
                    }
                }
            }
            1 => {
                let r = logic.apply(
                    &SoundRecord::Stop {
                        actor: actor(a % 6),
                    },
                    &mut rng,
                    &mut mixer,
                    gametic,
                );
                let Ok(RecordEcho::Stop(stopped)) = r else {
                    panic!("a stop must succeed: {r:?}");
                };
                assert_eq!(
                    occupied(&logic) + usize::from(stopped.is_some()),
                    occupied(&logic0)
                );
                assert_eq!(rng, rng0);
                assert_eq!(
                    mixer, mixer0,
                    "a stop silenced a mixer channel (T13 item 2)"
                );
            }
            2 => {
                let listener = Listener {
                    actor: listener_actor,
                    xy: [i32::from(d as i8) << 16, i32::from(e as i8) << 16],
                    angle: u32::from(f) << 24,
                };
                let positions = Positions { gone: a };
                match logic.update(&listener, &positions, gametic) {
                    Err(SoundRefusal::OriginGone { cnum }) => {
                        assert!(logic0.channels()[cnum].is_some());
                        unchanged(&logic, &mixer, &rng);
                    }
                    Err(other) => panic!("an update refused with {other:?}"),
                    Ok(echo) => {
                        assert_eq!(rng, rng0);
                        assert_eq!(mixer, mixer0);
                        assert_eq!(
                            occupied(&logic) + echo.stops.len(),
                            occupied(&logic0),
                            "stops against occupancy"
                        );
                        for s in &echo.stops {
                            let before =
                                logic0.channels()[s.cnum].expect("freed an occupied channel");
                            assert_eq!(before, s.entry);
                            assert!(logic.channels()[s.cnum].is_none());
                            assert_eq!(s.playing, gametic < before.handle);
                            assert_eq!(
                                s.reason == StopReason::Finished,
                                gametic >= before.handle,
                                "Finished exactly when gametic >= handle"
                            );
                            assert!(matches!(
                                s.reason,
                                StopReason::Finished
                                    | StopReason::Inaudible
                                    | StopReason::LinkVolume
                            ));
                        }
                    }
                }
            }
            3 => {
                let stops = logic.level_start(gametic);
                assert_eq!(stops.len(), occupied(&logic0));
                assert_eq!(occupied(&logic), 0);
            }
            4 => gametic = gametic.wrapping_add(i32::from(a)),
            _ => mixer.mix_buffer(&mut buf),
        }
        check_table(&logic, channels);
    }
});
