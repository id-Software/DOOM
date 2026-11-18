//! The padded sound data of the in-process configuration: `I_InitSound`'s load loop
//! (i_sound.c:796-813) with `getsfx` (i_sound.c:185-251), and the lookup `I_GetSfxLumpNum`
//! (i_sound.c:451-456) that `S_StartSoundAtVolume` makes at a sound's first start
//! (s_sound.c:363-365). M5c note §1.3.2, §4.6.3.

use std::sync::Arc;

use doom_render::LumpDirectory;

use crate::sounds::{NUMSFX, S_SFX, SfxId};

/// `SAMPLECOUNT` (i_sound.c:93): frames per mixed buffer, and the padding unit of sound data.
pub const SAMPLECOUNT: usize = 512;

/// Resource bounds of [`SoundSet::load`] (M5c note §4.9): a declared over-refusal at load.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SoundCaps {
    /// The largest sound lump accepted, in bytes.
    pub max_lump: usize,
    /// The largest total of padded bytes over all loaded sounds.
    pub max_total: usize,
}

impl Default for SoundCaps {
    /// 1 MiB per lump and 16 MiB in total (M5c note §4.9).
    fn default() -> Self {
        Self {
            max_lump: 1 << 20,
            max_total: 16 << 20,
        }
    }
}

/// Why [`SoundSet::load`] failed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SoundLoadError {
    /// A sound's `ds` lump is missing and so is `dspistol`, which `getsfx` copies instead
    /// (i_sound.c:213-214): the genuine `W_GetNumForName` ends the program with `I_Error`
    /// (w_wad.c:399-409) during `I_InitSound`.
    PistolMissing {
        /// The sound being loaded.
        sfx: u16,
    },
    /// A lump or the total is larger than [`SoundCaps`] allows.
    TooLarge {
        /// The sound being loaded.
        sfx: u16,
        /// Its lump's size in bytes.
        size: usize,
    },
}

/// Where a loaded sound's bytes came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SfxSource {
    /// Its own `"ds" + name` lump (i_sound.c:213, 216).
    Own,
    /// A copy of `dspistol`, because its own lump is missing (i_sound.c:213-214).
    Fallback,
    /// The data of the entry it is linked to, with length 0 (i_sound.c:808-811).
    Linked,
}

/// One loaded `S_sfx` entry: `S_sfx[i].data` and `lengths[i]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SfxData {
    /// The bytes from `S_sfx[i].data` on: for an unlinked sound the lump's bytes after its
    /// 8-byte header, then 128 up to `len` (i_sound.c:239-241, 250); for the linked sound the
    /// target's bytes (i_sound.c:810).
    pub data: Arc<[u8]>,
    /// `lengths[i]`: the padded length (i_sound.c:230, 247); 0 for the linked sound, which reads
    /// `lengths[0]` (i_sound.c:811, M5c note §1.3.2 item 12).
    pub len: u32,
    /// Where the bytes came from.
    pub source: SfxSource,
    /// The size of the lump read (the own lump or `dspistol`); 0 for the linked sound.
    pub lump_size: usize,
}

impl SfxData {
    /// Whether a start of this sound has a sample byte at its start. A sound whose padded length
    /// is 0 and whose data are empty (an unlinked lump of 8 bytes or fewer) mixes one frame that
    /// reads one byte past its 8-byte block in the C (i_sound.c:230-247, 591 before 606), a
    /// layout-dependent value: such a start is refused (M5c note §3.2.5 row 3, decision T11).
    pub fn has_first_byte(&self) -> bool {
        !self.data.is_empty()
    }
}

/// Every loaded sound of a session, and the result of each sound's `I_GetSfxLumpNum`.
#[derive(Clone, Debug)]
pub struct SoundSet {
    sfx: Vec<Option<SfxData>>,
    lump_num: Vec<Option<usize>>,
}

/// `"ds" + name`, as `sprintf(name, "ds%s", sfxname)` writes it (i_sound.c:201, 455).
fn ds_name(sfx: usize) -> Vec<u8> {
    let mut n = b"ds".to_vec();
    n.extend_from_slice(S_SFX[sfx].name.as_bytes());
    n
}

impl SoundSet {
    /// `I_InitSound`'s load loop for the in-process configuration (i_sound.c:799-813) over a lump
    /// directory, and every sound's `I_GetSfxLumpNum` result (i_sound.c:451-456).
    ///
    /// For i in 1..109: a linked entry takes its target's data and `lengths[0]` = 0
    /// (i_sound.c:808-811); otherwise `getsfx`: the lump `"ds" + name` if present, else a copy of
    /// `dspistol` (i_sound.c:201, 213-216); `paddedsize = ((size-8 + 511) / 512) * 512` in C `int`
    /// arithmetic, never negative (i_sound.c:230); the lump's bytes after the header, then 128 up
    /// to `paddedsize` (i_sound.c:239-241, 250). A lump of 8 bytes or fewer loads with length 0,
    /// as in the C; its start is refused later ([`SfxData::has_first_byte`]).
    pub fn load(dir: &LumpDirectory, caps: SoundCaps) -> Result<Self, SoundLoadError> {
        let mut sfx: Vec<Option<SfxData>> = vec![None; NUMSFX];
        let mut lump_num = vec![None; NUMSFX];
        let mut total = 0usize;
        for i in 1..NUMSFX {
            lump_num[i] = dir.check_num_for_name(&ds_name(i));
            if let Some(target) = S_SFX[i].link {
                // i_sound.c:810-811; the target has a lower index (pistol 1 < chgun 86), so it is
                // already loaded, as in the C.
                let t = sfx[usize::from(target)].as_ref().map(|d| d.data.clone());
                sfx[i] = Some(SfxData {
                    data: t.unwrap_or_else(|| Arc::from(Vec::new())),
                    len: 0,
                    source: SfxSource::Linked,
                    lump_size: 0,
                });
                continue;
            }
            let (lump, source) = match lump_num[i] {
                Some(l) => (l, SfxSource::Own),
                None => match dir.check_num_for_name(b"dspistol") {
                    Some(l) => (l, SfxSource::Fallback),
                    None => return Err(SoundLoadError::PistolMissing { sfx: i as u16 }),
                },
            };
            let bytes = dir.data(lump).unwrap_or(&[]);
            let size = bytes.len();
            if size > caps.max_lump {
                return Err(SoundLoadError::TooLarge {
                    sfx: i as u16,
                    size,
                });
            }
            // i_sound.c:230: ((size-8 + (SAMPLECOUNT-1)) / SAMPLECOUNT) * SAMPLECOUNT; for size
            // 0..8 the dividend is 503..511 and the result 0 (M5c note §1.3.2 item 11).
            let padded = (size + SAMPLECOUNT - 1 - 8) / SAMPLECOUNT * SAMPLECOUNT;
            total = total.saturating_add(padded);
            if total > caps.max_total {
                return Err(SoundLoadError::TooLarge {
                    sfx: i as u16,
                    size,
                });
            }
            let mut data = Vec::with_capacity(padded);
            data.extend_from_slice(bytes.get(8..).unwrap_or(&[]));
            data.resize(padded, 128); // i_sound.c:240-241
            sfx[i] = Some(SfxData {
                data: Arc::from(data),
                len: padded as u32,
                source,
                lump_size: size,
            });
        }
        Ok(Self { sfx, lump_num })
    }

    /// The loaded entry of `sfx` (always present for 1..=108 after a successful load).
    pub fn sfx(&self, sfx: SfxId) -> &SfxData {
        // Every index 1..NUMSFX is filled by `load`; SfxId is 1..=108.
        self.sfx[sfx.index()]
            .as_ref()
            .expect("SoundSet::load fills every sound 1..=108")
    }

    /// `I_GetSfxLumpNum(&S_sfx[sfx])` (i_sound.c:451-456): `W_GetNumForName("ds" + name)`, the
    /// lump number, or `None` where the genuine call ends in `I_Error`
    /// ("W_GetNumForName: ds... not found!", w_wad.c:399-409).
    pub fn lump_num(&self, sfx: SfxId) -> Option<usize> {
        self.lump_num[sfx.index()]
    }
}
