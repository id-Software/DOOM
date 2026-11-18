//! The in-process mixer: `I_StartSound` with `addsfx` (i_sound.c:264-381, 470-501) and
//! `I_UpdateSound` (i_sound.c:539-654), M5c note §1.3.4-1.3.5, §1.3.9 and §4.6.
//!
//! The mixer feeds nothing back to the game (`I_StopSound` and `I_UpdateSoundParams` are empty,
//! i_sound.c:505-514, 674-688): its output is a pure function of the starts, the lumps and the
//! schedule (M5c note §3.2.3 item 4). A stop in the sound logic never silences a mixer channel
//! (decision T13 item 2).

use std::sync::Arc;

use crate::lumps::{SAMPLECOUNT, SoundSet};
use crate::sounds::{SFX_PISTOL, SFX_SAWFUL, SFX_SAWHIT, SFX_SAWIDL, SFX_SAWUP, SFX_STNMOV, SfxId};
use crate::tables::{STEPTABLE, VOL_LOOKUP};

/// `NUM_CHANNELS` (i_sound.c:94).
pub const NUM_CHANNELS: usize = 8;

/// One mixed buffer: `SAMPLECOUNT` frames of (left, right), the interleaving of
/// `mixbuffer[2k]`, `mixbuffer[2k+1]` (i_sound.c:565-567, 633-634).
pub type Buffer = [[i16; 2]; SAMPLECOUNT];

/// Why the mixer refused a start. Each is checked before any state changes; the C writes half a
/// channel and exits, or reads memory it does not own (M5c note §4.6.5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MixRefusal {
    /// A pitch outside 0..=255: `steptable[pitch]` reads past the 256-entry table
    /// (i_sound.c:144, 495; observed by UBSan, M5c note §3.2.5 row 12). `s_sound.c` clamps the
    /// pitch, so no game start reaches it.
    Pitch {
        /// The pitch given.
        pitch: i32,
    },
    /// The sound has no sample byte at its start (padded length 0, an 8-byte-or-shorter lump):
    /// its one mixed frame reads one byte past its 8-byte block (i_sound.c:230-247, 591 before
    /// 606), decision T11.
    ShortLump {
        /// The sound.
        sfx: u16,
    },
    /// `rightvol` outside 0..=127: `I_Error("rightvol out of bounds")` (i_sound.c:364-365),
    /// checked first.
    RightVolume {
        /// The volume given.
        vol: i32,
        /// The separation given.
        sep: i32,
    },
    /// `leftvol` outside 0..=127: `I_Error("leftvol out of bounds")` (i_sound.c:367-368).
    LeftVolume {
        /// The volume given.
        vol: i32,
        /// The separation given.
        sep: i32,
    },
}

impl MixRefusal {
    /// The genuine `I_Error` text for the two range checks (i_sound.c:364-368), or a description.
    pub fn message(&self) -> String {
        match self {
            Self::RightVolume { .. } => "rightvol out of bounds".to_string(),
            Self::LeftVolume { .. } => "leftvol out of bounds".to_string(),
            Self::Pitch { pitch } => {
                format!("refused: pitch {pitch} outside 0..255 (i_sound.c:495)")
            }
            Self::ShortLump { sfx } => {
                format!(
                    "refused: sfx {sfx} has no sample byte at its start (i_sound.c:230-247, 591)"
                )
            }
        }
    }
}

/// What one start did: the genuine harness's `s` row (tests/reference/m5c_mixer/README.md).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MixStart {
    /// The handle `addsfx` returned (i_sound.c:335-340, 380).
    pub handle: i32,
    /// The mixer channel taken (i_sound.c:309-325).
    pub slot: usize,
    /// `steptable[pitch]` (i_sound.c:495, 344).
    pub step: u32,
    /// The left volume row (i_sound.c:357-358, 372).
    pub left: u8,
    /// The right volume row (i_sound.c:360-361, 373).
    pub right: u8,
    /// The occupancy mask (bit c: channel c busy) before the call.
    pub before: u8,
    /// The sfx that occupied `slot` before the call, if it was busy.
    pub prev: Option<u16>,
    /// Other channels the single-instance rule freed (i_sound.c:285-306), as a mask.
    pub freed: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Channel {
    /// `channels[c] != NULL` (i_sound.c:122): the sfx whose data it reads.
    sfx: Option<SfxId>,
    /// The read position, as an offset from `S_sfx[sfx].data`.
    pos: u32,
    /// `channelsend[c]` as an offset: `lengths[sfx]` (i_sound.c:332).
    end: u32,
    /// `channelstep[c]` (i_sound.c:116, 344).
    step: u32,
    /// `channelstepremainder[c]`, 0..=65535 (i_sound.c:118, 346, 599-603).
    frac: u32,
    /// `channelstart[c]` (i_sound.c:131, 348).
    start: i32,
    /// `channelhandles[c]` (i_sound.c:137, 340): written, never read by the C.
    handle: i32,
    /// `channelids[c]` (i_sound.c:141, 377).
    id: u16,
    /// The row of `channelleftvol_lookup[c]` (i_sound.c:150, 372).
    left: u8,
    /// The row of `channelrightvol_lookup[c]` (i_sound.c:151, 373).
    right: u8,
}

/// The eight mixer channels and the handle counter of `addsfx`.
#[derive(Clone, Debug)]
pub struct Mixer {
    set: Arc<SoundSet>,
    channels: [Channel; NUM_CHANNELS],
    /// `static unsigned short handlenums = 0;` (i_sound.c:271).
    handlenums: u16,
}

impl PartialEq for Mixer {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.set, &other.set)
            && self.channels == other.channels
            && self.handlenums == other.handlenums
    }
}

/// The single-instance sounds of `addsfx` (i_sound.c:285-290).
fn single_instance(id: u16) -> bool {
    matches!(
        id,
        SFX_SAWUP | SFX_SAWIDL | SFX_SAWFUL | SFX_SAWHIT | SFX_STNMOV | SFX_PISTOL
    )
}

/// `addsfx`'s separation arithmetic (i_sound.c:352-361): `(left, right)`.
///
/// Computed in 128 bits, where no pair of `i32` arguments can overflow: `|vol| <= 2^31` and
/// `|sep + 1 - 257| < 2^32`, so `|vol * s * s| < 2^95` and each difference stays below `2^96`.
/// For the game's range (volume 0..=127, separation 0..=255) every value equals the C's `int`
/// arithmetic, whose products cannot overflow there (M5c note §1.3.8 item 4); outside it the
/// range checks of i_sound.c:364-368 then refuse the start. (An earlier 64-bit form overflowed
/// for arguments such as `vol = sep = 0x7fff << 15`, found by the sound_mixer fuzz target.)
fn volumes(vol: i32, sep: i32) -> (i128, i128) {
    let v = i128::from(vol);
    let mut s = i128::from(sep) + 1;
    let left = v - ((v * s * s) >> 16);
    s -= 257;
    let right = v - ((v * s * s) >> 16);
    (left, right)
}

impl Mixer {
    /// A silent mixer over a loaded sound set: all channels free and the handle counter 0, as in
    /// a fresh process (static storage, i_sound.c:122, 271; decision T15).
    pub fn new(set: Arc<SoundSet>) -> Self {
        Self {
            set,
            channels: [Channel::default(); NUM_CHANNELS],
            handlenums: 0,
        }
    }

    /// The sound set the mixer reads.
    pub fn sound_set(&self) -> &Arc<SoundSet> {
        &self.set
    }

    /// The occupancy mask: bit c set while mixer channel c is busy (`channels[c] != NULL`).
    pub fn mask(&self) -> u8 {
        self.channels
            .iter()
            .enumerate()
            .filter(|(_, c)| c.sfx.is_some())
            .fold(0, |m, (i, _)| m | (1 << i))
    }

    /// The handle counter (`handlenums`, i_sound.c:271).
    pub fn handle_counter(&self) -> u16 {
        self.handlenums
    }

    /// The handle `addsfx` will hand out next (i_sound.c:335-340), without changing anything.
    pub fn next_handle(&self) -> i32 {
        if self.handlenums == 0 {
            100
        } else {
            i32::from(self.handlenums)
        }
    }

    /// Checks a start without changing anything: [`Self::start`] succeeds exactly when this does.
    pub fn check_start(
        &self,
        sfx: SfxId,
        vol: i32,
        sep: i32,
        pitch: i32,
    ) -> Result<(), MixRefusal> {
        if !(0..=255).contains(&pitch) {
            return Err(MixRefusal::Pitch { pitch });
        }
        if !self.set.sfx(sfx).has_first_byte() {
            return Err(MixRefusal::ShortLump { sfx: sfx.get() });
        }
        let (left, right) = volumes(vol, sep);
        if !(0..=127).contains(&right) {
            return Err(MixRefusal::RightVolume { vol, sep });
        }
        if !(0..=127).contains(&left) {
            return Err(MixRefusal::LeftVolume { vol, sep });
        }
        Ok(())
    }

    /// `I_StartSound(id, vol, sep, pitch, priority)` with `addsfx` (i_sound.c:470-501, 264-381),
    /// with `gametic` as the value of the global at the call. The priority is discarded by the C
    /// (`priority = 0;`, i_sound.c:480) and is not taken. Every refusal is decided before any
    /// change ([`Self::check_start`]).
    pub fn start(
        &mut self,
        sfx: SfxId,
        vol: i32,
        sep: i32,
        pitch: i32,
        gametic: i32,
    ) -> Result<MixStart, MixRefusal> {
        self.check_start(sfx, vol, sep, pitch)?;
        let id = sfx.get();
        let before = self.mask();
        // i_sound.c:495: steptable[pitch] (pitch is 0..=255, checked above).
        let step = STEPTABLE[pitch as usize];
        // i_sound.c:285-306: the first busy channel playing the same id is freed.
        if single_instance(id) {
            if let Some(c) = self
                .channels
                .iter_mut()
                .find(|c| c.sfx.is_some() && c.id == id)
            {
                c.sfx = None;
            }
        }
        // i_sound.c:274-277, 309-325: the first free channel, else the lowest index with the
        // smallest channelstart strictly below gametic, else 0.
        let mut oldest = gametic;
        let mut oldestnum = 0;
        let mut i = 0;
        while i < NUM_CHANNELS && self.channels[i].sfx.is_some() {
            if self.channels[i].start < oldest {
                oldestnum = i;
                oldest = self.channels[i].start;
            }
            i += 1;
        }
        let slot = if i == NUM_CHANNELS { oldestnum } else { i };
        let prev = (before & (1 << slot) != 0).then_some(self.channels[slot].id);
        // i_sound.c:335-340: 0 becomes 100; the unsigned short wraps from 65535 to 0.
        if self.handlenums == 0 {
            self.handlenums = 100;
        }
        let handle = i32::from(self.handlenums);
        self.handlenums = self.handlenums.wrapping_add(1);
        let (left, right) = volumes(vol, sep);
        let c = &mut self.channels[slot];
        *c = Channel {
            sfx: Some(sfx),             // i_sound.c:330
            pos: 0,                     // i_sound.c:330
            end: self.set.sfx(sfx).len, // i_sound.c:332
            step,                       // i_sound.c:344
            frac: 0,                    // i_sound.c:346
            start: gametic,             // i_sound.c:348
            handle,                     // i_sound.c:340
            id,                         // i_sound.c:377
            left: left as u8,           // i_sound.c:372 (0..=127, checked)
            right: right as u8,         // i_sound.c:373
        };
        let after = self.mask();
        let freed = (before & !after) & !(1u8 << slot);
        Ok(MixStart {
            handle,
            slot,
            step,
            left: left as u8,
            right: right as u8,
            before,
            prev,
            freed,
        })
    }

    /// `I_UpdateSound` (i_sound.c:539-635): mixes one buffer of 512 frames. Per frame, channels
    /// 0..7 in order: read the byte at the position (i_sound.c:591), add both volume rows
    /// (596-597), add the step to the 16.16 remainder (599), advance by `remainder >> 16` (601),
    /// keep `remainder & 65535` (603), free the channel when the position reaches the padded end
    /// (606-607); then clamp each sum to -32768..32767 (617-630).
    pub fn mix_buffer(&mut self, out: &mut Buffer) {
        for frame in out.iter_mut() {
            let mut dl: i32 = 0;
            let mut dr: i32 = 0;
            for c in self.channels.iter_mut() {
                let Some(sfx) = c.sfx else { continue };
                // A start is refused unless its data have a byte at position 0 (ShortLump), and
                // afterwards pos < end <= data.len(), so the read is in bounds.
                let sample = usize::from(self.set.sfx(sfx).data[c.pos as usize]);
                dl += VOL_LOOKUP[usize::from(c.left)][sample];
                dr += VOL_LOOKUP[usize::from(c.right)][sample];
                c.frac += c.step;
                c.pos += c.frac >> 16;
                c.frac &= 65536 - 1;
                if c.pos >= c.end {
                    c.sfx = None;
                }
            }
            *frame = [clamp(dl), clamp(dr)];
        }
    }
}

/// i_sound.c:617-630.
fn clamp(v: i32) -> i16 {
    v.clamp(-0x8000, 0x7fff) as i16
}

/// One buffer as the bytes `I_SubmitSound` writes (i_sound.c:665-670): 2,048 bytes of
/// little-endian signed 16-bit samples, left first (the device format `AFMT_S16_LE`,
/// i_sound.c:788-789).
pub fn buffer_bytes(buf: &Buffer) -> [u8; SAMPLECOUNT * 4] {
    let mut out = [0u8; SAMPLECOUNT * 4];
    for (k, [l, r]) in buf.iter().enumerate() {
        out[4 * k..4 * k + 2].copy_from_slice(&l.to_le_bytes());
        out[4 * k + 2..4 * k + 4].copy_from_slice(&r.to_le_bytes());
    }
    out
}

/// The declared schedule (decision T5 part 2): the number of buffers due once tic `tic`
/// (counted from the session's first tic, 0) has run, given that `mixed` buffers were mixed
/// before: every buffer k not yet mixed with `512*k < 315*(tic+1)`, so buffer k is mixed after
/// tic `floor(512k/315)`, none or one per tic.
pub fn buffers_due(tic: u64, mixed: u64) -> u64 {
    let frames = 315u128 * (u128::from(tic) + 1);
    let total = frames.div_ceil(SAMPLECOUNT as u128);
    total
        .saturating_sub(u128::from(mixed))
        .min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lumps::SoundCaps;
    use crate::sounds::SFX_PISTOL;

    fn mixer() -> Mixer {
        let bytes = include_bytes!("../../../tests/reference/m5c_sfx.wad");
        let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).expect("WAD");
        let mut dir = doom_render::LumpDirectory::new();
        dir.add_wad(&wad).expect("directory");
        Mixer::new(Arc::new(
            SoundSet::load(&dir, SoundCaps::default()).expect("sound set"),
        ))
    }

    /// Fuzz finding step11a-1: `vol = sep = 0x7fff << 15` overflowed the 64-bit products of
    /// i_sound.c:352-361 and panicked. Now the start is refused by the first range check
    /// (`rightvol`, i_sound.c:364-365) with the mixer unchanged, and no pair of extreme
    /// arguments panics.
    #[test]
    fn volumes_far_outside_the_game_range_are_refused_without_overflow() {
        let pistol = SfxId::new(i64::from(SFX_PISTOL)).expect("pistol");
        let mut m = mixer();
        let before = m.clone();
        let wild = 0x7fff << 15;
        let want = MixRefusal::RightVolume {
            vol: wild,
            sep: wild,
        };
        assert_eq!(m.check_start(pistol, wild, wild, 128), Err(want));
        assert_eq!(m.start(pistol, wild, wild, 128, 0), Err(want));
        assert_eq!(m, before);
        let edges = [
            i32::MIN,
            i32::MIN + 1,
            -258,
            -1,
            0,
            1,
            127,
            255,
            256,
            i32::MAX,
        ];
        for vol in edges {
            for sep in edges {
                let (left, right) = volumes(vol, sep);
                let got = m.check_start(pistol, vol, sep, 128);
                let ok = (0..=127).contains(&left) && (0..=127).contains(&right);
                assert_eq!(got.is_ok(), ok, "vol {vol} sep {sep}");
                if got.is_err() {
                    assert_eq!(m.start(pistol, vol, sep, 128, 0).map(|_| ()), got);
                    assert_eq!(m, before);
                }
            }
        }
    }
}
