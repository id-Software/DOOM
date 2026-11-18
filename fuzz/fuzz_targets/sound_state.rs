#![no_main]
//! The saved state `DSND` (doom_sound::state) and the session (doom_sound::session::Sound) of M5c,
//! over the committed synthetic sound WAD `m5c_sfx.wad` (no id data).
//!
//! Input: byte 0 the mode (`% 2`).
//!
//! - Mode 0, the decoder: the rest is `DSND` bytes. `SoundState::decode` never panics; when it
//!   accepts, the bytes are canonical (`encode` of the result gives them back) and
//!   `decode(encode(s)) == s`; `Sound::from_state` on the set accepts or refuses with a typed
//!   error, and an accepted session's `state()` is `s` again.
//! - Mode 1, the session: byte 1 the map (`% 3`: 1, 8 or 3), byte 2 the texture count of the T12
//!   detector (0..=255), then eight-byte ops `op a b c d e f g`, at most 128 (`-max_len=2048`).
//!   `op % 4`: 0 queue a start of sound `a` (any `u8`: 0 and 109 and above are refused) with
//!   origin `b % 5` (none, actor `c % 6`, sector `c`, button `c % 16`, the listener's body) and,
//!   when `op & 0x10`, a placement `(i8(d) << 16, i8(e) << 16)` from a listener at
//!   `(i8(f) << 16, i8(g) << 16)` with angle `g << 24` (a button origin's placement is instead
//!   the declared reading `(c % 36, 0)`, so that the detector sees busy slots); 1 queue a removal
//!   stop of actor `a % 6`; 2 run the tic with the queued records, `M_Random` index `a`, the
//!   listener at `(i8(d) << 16, i8(e) << 16)` angle `f << 24`, positions unreadable by the bits of
//!   `b` and the button reading `(c % 36, 0)`, then clear the queue; 3 save and restore.
//!
//! Invariants (mode 1): `Sound::new` refuses only more than 65536 textures (never here); a tic
//! is `Ok` or a typed `SessionRefusal`; on a refusal the session equals a clone taken before the
//! tic; on success the schedule advanced by one tic and by the buffers the declared rule gives
//! (a buffer is returned exactly then), the returned index is the given one advanced by the
//! reported draws, and the detector's count never falls and its first tic never changes once set;
//! at every save, `decode(encode(state)) == state`, the encoding is at most `MAX_BYTES`, and
//! `Sound::from_state` rebuilds a session equal to the running one.
use std::sync::Arc;

use doom_render::LumpDirectory;
use doom_sim::ActorId;
use doom_sound::logic::{
    Listener, OriginPositions, RecordEcho, SoundOrigin, SoundPlacement, SoundRecord,
};
use doom_sound::lumps::{SoundCaps, SoundSet};
use doom_sound::mixer::buffers_due;
use doom_sound::session::{Sound, SoundConfig};
use doom_sound::state::{MAX_BYTES, SoundState};
use doom_types::DoomRng;
use libfuzzer_sys::fuzz_target;

const MAIN: &[u8] = include_bytes!("../../tests/reference/m5c_sfx.wad");
const MAX_OPS: usize = 128;
const MAX_RECORDS: usize = 64;

fn set() -> &'static Arc<SoundSet> {
    static SET: std::sync::OnceLock<Arc<SoundSet>> = std::sync::OnceLock::new();
    SET.get_or_init(|| {
        let wad = doom_wad::Wad::parse(MAIN, doom_wad::Limits::default()).unwrap();
        let mut dir = LumpDirectory::new();
        dir.add_wad(&wad).unwrap();
        Arc::new(SoundSet::load(&dir, SoundCaps::default()).unwrap())
    })
}

fn actor(n: u8) -> ActorId {
    ActorId::from_parts(u32::from(n), 1)
}

struct Positions {
    gone: u8,
    timer: i32,
}
impl OriginPositions for Positions {
    fn actor_xy(&self, a: ActorId) -> Option<[i32; 2]> {
        let n = a.slot() as i32;
        (self.gone & 1 == 0).then_some([(n * 300 - 600) << 16, (n * 90) << 16])
    }
    fn sector_xy(&self, s: u16) -> Option<[i32; 2]> {
        (self.gone & 2 == 0).then_some([i32::from(s) << 12, i32::from(s) << 14])
    }
    fn button_xy(&self, _: u8) -> Option<[i32; 2]> {
        (self.gone & 4 == 0).then_some([self.timer, 0])
    }
}

fn decoder(bytes: &[u8]) {
    let Ok(state) = SoundState::decode(bytes) else {
        return;
    };
    let again = state.encode();
    assert_eq!(again, bytes, "an accepted encoding is not canonical");
    assert_eq!(SoundState::decode(&again).as_ref(), Ok(&state));
    if let Ok(sound) = Sound::from_state(set().clone(), &state) {
        assert_eq!(sound.state(), state, "a restored session's state differs");
    }
}

fn session(map: u8, textures: u8, ops: &[u8]) {
    let gamemap = [1, 8, 3][usize::from(map % 3)];
    let mut sound = Sound::new(
        set().clone(),
        SoundConfig::fresh(gamemap, u32::from(textures)),
    )
    .expect("a fresh session");
    let listener_actor = actor(0);
    let mut queue: Vec<SoundRecord> = Vec::new();
    for o in ops.chunks_exact(8).take(MAX_OPS) {
        let [op, a, b, c, d, e, f, g] = [o[0], o[1], o[2], o[3], o[4], o[5], o[6], o[7]];
        let s = |x: u8| i32::from(x as i8) << 16;
        match op % 4 {
            0 if queue.len() < MAX_RECORDS => {
                let origin = match b % 5 {
                    0 => SoundOrigin::None,
                    1 => SoundOrigin::Actor(actor(c % 6)),
                    2 => SoundOrigin::Sector(u16::from(c)),
                    3 => SoundOrigin::Button(c % 16),
                    _ => SoundOrigin::Actor(listener_actor),
                };
                let at = (op & 0x10 != 0).then(|| SoundPlacement {
                    origin: if matches!(origin, SoundOrigin::Button(_)) {
                        [i32::from(c % 36), 0]
                    } else {
                        [s(d), s(e)]
                    },
                    listener: [s(f), s(g)],
                    listener_angle: u32::from(g) << 24,
                });
                queue.push(SoundRecord::Start {
                    sfx: u16::from(a),
                    origin,
                    at,
                });
            }
            1 if queue.len() < MAX_RECORDS => queue.push(SoundRecord::Stop {
                actor: actor(a % 6),
            }),
            2 => {
                let before = sound.clone();
                let listener = Listener {
                    actor: listener_actor,
                    xy: [s(d), s(e)],
                    angle: u32::from(f) << 24,
                };
                let positions = Positions {
                    gone: b,
                    timer: i32::from(c % 36),
                };
                match sound.tic(&queue, a, &listener, &positions) {
                    Err(_) => assert_eq!(sound, before, "a refused tic changed the session"),
                    Ok(t) => {
                        let (s0, s1) = (before.schedule(), sound.schedule());
                        assert_eq!(t.tic, s0.tic());
                        assert_eq!(s1.tic(), s0.tic() + 1);
                        let due = buffers_due(s0.tic(), s0.buffers());
                        assert_eq!(s1.buffers(), s0.buffers() + due);
                        assert_eq!(t.buffer.is_some(), due == 1);
                        let draws: u16 = t
                            .records
                            .iter()
                            .map(|r| match r {
                                RecordEcho::Start(e) => u16::from(e.draws()),
                                RecordEcho::Stop(_) => 0,
                            })
                            .sum();
                        assert_eq!(draws, t.draws);
                        let mut rng = DoomRng::from_indices(0, a);
                        for _ in 0..draws {
                            rng.m_random();
                        }
                        assert_eq!(t.rndindex, rng.indices()[1], "index against the draws");
                        let (d0, d1) = (before.detector(), sound.detector());
                        assert!(d1.firings() >= d0.firings());
                        if d0.layout_dependent_from().is_some() {
                            assert_eq!(d1.layout_dependent_from(), d0.layout_dependent_from());
                        }
                        assert_eq!(d1.layout_dependent_from().is_some(), d1.firings() > 0);
                    }
                }
                queue.clear();
            }
            3 => {
                let state = sound.state();
                let bytes = state.encode();
                assert!(bytes.len() <= MAX_BYTES);
                assert_eq!(
                    SoundState::decode(&bytes).as_ref(),
                    Ok(&state),
                    "round trip"
                );
                let restored =
                    Sound::from_state(set().clone(), &state).expect("a reached state restores");
                assert_eq!(restored, sound, "a restored session differs");
            }
            _ => {}
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }
    if data[0] % 2 == 0 {
        decoder(&data[1..]);
    } else {
        session(data[1], data[2], &data[3..]);
    }
});
