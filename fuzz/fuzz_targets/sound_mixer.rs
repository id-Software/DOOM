#![no_main]
//! An arbitrary sequence of starts, clock changes and buffer mixes into `doom_sound::mixer::Mixer`
//! (M5c), over a committed synthetic sound WAD (no id data).
//!
//! Input: byte 0 the set (`% 2`: `m5c_sfx.wad`, or `m5c_sfx_edge.wad` whose lumps of 0, 5 and 8
//! bytes make the short-lump refusal reachable), byte 1 the starting `gametic` (as `u8`), then
//! eight-byte ops `op a b c d e f g`, at most 256 ops (`-max_len=2050`). `op % 4`: 0 start of sound
//! `a` (a sound number 0 or 109 and above is skipped: the type `SfxId` cannot hold it) with
//! `vol`, `sep`, `pitch` that are in range when `op & 0x10` is clear (`b % 128`, `d`, `f`) and
//! wild when set (`i16(b,c) << (g & 15)`, `i16(d,e) << (g >> 4)`, `3 f - 200`); 1 mix `a % 4 + 1`
//! buffers; 2 advance `gametic` by `a`; 3 set `gametic` to `i32(a,b,c,d)`.
//!
//! Invariants: every start is `Ok` or a typed `MixRefusal`; the refusal is the first of pitch
//! outside 0..=255 (i_sound.c:495), a sound with no sample byte (padded length 0, decision T11),
//! `rightvol` outside 0..=127 (i_sound.c:364-365), `leftvol` outside 0..=127 (i_sound.c:367-368),
//! the volumes recomputed here in 128-bit arithmetic from i_sound.c:352-361; `check_start` agrees
//! with `start`; a refused start leaves the mixer equal to a clone taken before it; an accepted
//! start hands out `next_handle()` (0 becomes 100, 65535 wraps to 0, i_sound.c:335-340), takes a
//! slot below 8 whose bit is then set, and returns the volumes the model computed; a silent mixer
//! mixes zeros; mixing never frees by anything but the end of data (the mask only loses bits);
//! the whole sequence run twice on fresh mixers gives the same digest of results and bytes. Wild
//! volumes and separations are not skipped: since finding step11a-1 was fixed (doom-sound computes
//! i_sound.c:352-361 in 128 bits), every pair of `i32` arguments is a typed result, never a panic.
use std::sync::Arc;

use doom_render::LumpDirectory;
use doom_sound::lumps::{SoundCaps, SoundSet};
use doom_sound::mixer::{Buffer, MixRefusal, Mixer, NUM_CHANNELS, buffer_bytes};
use doom_sound::sounds::SfxId;
use libfuzzer_sys::fuzz_target;

const MAIN: &[u8] = include_bytes!("../../tests/reference/m5c_sfx.wad");
const EDGE: &[u8] = include_bytes!("../../tests/reference/m5c_sfx_edge.wad");
const MAX_OPS: usize = 256;

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

struct Digest(u64);
impl Digest {
    fn put(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
        }
    }
}

/// The volumes of i_sound.c:352-361 in 128-bit arithmetic.
fn model_volumes(vol: i32, sep: i32) -> (i128, i128) {
    let v = i128::from(vol);
    let s = i128::from(sep) + 1;
    let left = v - ((v * s * s) >> 16);
    let s2 = s - 257;
    let right = v - ((v * s2 * s2) >> 16);
    (left, right)
}

fn run(set: &Arc<SoundSet>, start: i32, ops: &[u8]) -> u64 {
    let mut mixer = Mixer::new(set.clone());
    let mut gametic = start;
    let mut digest = Digest(0xcbf2_9ce4_8422_2325);
    let mut buf: Buffer = [[0; 2]; 512];
    for o in ops.chunks_exact(8) {
        let [op, a, b, c, d, e, f, g] = [o[0], o[1], o[2], o[3], o[4], o[5], o[6], o[7]];
        match op % 4 {
            0 => {
                let Some(sfx) = SfxId::new(i64::from(a)) else {
                    continue;
                };
                let (vol, sep, pitch) = if op & 0x10 == 0 {
                    (i32::from(b % 128), i32::from(d), i32::from(f))
                } else {
                    (
                        i32::from(i16::from_le_bytes([b, c])) << (g & 15),
                        i32::from(i16::from_le_bytes([d, e])) << (g >> 4),
                        3 * i32::from(f) - 200,
                    )
                };
                let before = mixer.clone();
                let checked = mixer.check_start(sfx, vol, sep, pitch);
                let (left, right) = model_volumes(vol, sep);
                let expected = if !(0..=255).contains(&pitch) {
                    Err(MixRefusal::Pitch { pitch })
                } else if !set.sfx(sfx).has_first_byte() {
                    Err(MixRefusal::ShortLump { sfx: sfx.get() })
                } else if !(0..=127).contains(&right) {
                    Err(MixRefusal::RightVolume { vol, sep })
                } else if !(0..=127).contains(&left) {
                    Err(MixRefusal::LeftVolume { vol, sep })
                } else {
                    Ok(())
                };
                assert_eq!(checked, expected, "refusal order or condition");
                let next = before.next_handle();
                match mixer.start(sfx, vol, sep, pitch, gametic) {
                    Err(r) => {
                        assert_eq!(Err(r), expected);
                        assert_eq!(mixer, before, "a refused start changed the mixer");
                        digest.put(&[1]);
                    }
                    Ok(s) => {
                        assert_eq!(expected, Ok(()));
                        assert_eq!(s.handle, next);
                        assert!((100..=65535).contains(&s.handle));
                        assert_eq!(
                            mixer.handle_counter(),
                            (s.handle as u16).wrapping_add(1),
                            "handle counter"
                        );
                        assert!(s.slot < NUM_CHANNELS);
                        assert!(mixer.mask() & (1 << s.slot) != 0);
                        assert_eq!(i128::from(s.left), left);
                        assert_eq!(i128::from(s.right), right);
                        assert_eq!(s.before, before.mask());
                        digest.put(&[2, s.slot as u8, s.left, s.right, s.freed, s.before]);
                        digest.put(&s.handle.to_le_bytes());
                        digest.put(&s.step.to_le_bytes());
                    }
                }
            }
            1 => {
                for _ in 0..(a % 4 + 1) {
                    let was = mixer.mask();
                    mixer.mix_buffer(&mut buf);
                    let now = mixer.mask();
                    assert_eq!(now & !was, 0, "mixing started a channel");
                    if was == 0 {
                        assert!(
                            buf.iter().all(|f| *f == [0, 0]),
                            "silent mixer is not silent"
                        );
                    }
                    digest.put(&buffer_bytes(&buf));
                    digest.put(&[now]);
                }
            }
            2 => gametic = gametic.wrapping_add(i32::from(a)),
            _ => gametic = i32::from_le_bytes([a, b, c, d]),
        }
    }
    digest.0
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let set = &sets()[usize::from(data[0] % 2)];
    let start = i32::from(data[1]);
    let ops = &data[2..];
    let ops = &ops[..ops.len().min(MAX_OPS * 8)];
    let first = run(set, start, ops);
    let second = run(set, start, ops);
    assert_eq!(first, second, "the same sequence gave different results");
});
