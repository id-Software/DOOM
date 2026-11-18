//! The sound table `S_sfx` (sounds.h:28-62, 177-289; sounds.c:114-227).
//!
//! The table data live in `sounds_table.rs`, generated from linuxdoom-1.10/sounds.c and sounds.h
//! by `tools/reference/m5c_sounds_table.py` (run it with `--check` to compare). Only the fields a
//! behaviour depends on are kept, plus `singularity` as data (no code in linuxdoom-1.10 reads it,
//! M5c note §1.1.3); `data`, `usefulness` and `lumpnum` are run-time state, not table data.

/// `NUMSFX` (sounds.h:288): the number of `S_sfx` entries, ids 0 to 108.
pub const NUMSFX: usize = 109;

/// One `sfxinfo_t` initialiser (sounds.h:32-62) without the run-time fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SfxInfo {
    /// `name`: up to 6 characters; the lump is `"ds" + name` (i_sound.c:201, 453-455).
    pub name: &'static str,
    /// `singularity`: read by nothing (M5c note §1.1.3).
    pub singularity: bool,
    /// `priority`: the table priority `S_getChannel` compares (s_sound.c:853-854); a lower
    /// number is more important.
    pub priority: i32,
    /// `link`: the index of the entry this one is linked to (`&S_sfx[...]`), if any. Only
    /// `sfx_chgun` is linked, to `sfx_pistol` (sounds.c:204).
    pub link: Option<u8>,
    /// `pitch`: the pitch used if linked (s_sound.c:285, 569); -1 otherwise.
    pub link_pitch: i32,
    /// `volume`: the volume added if linked (s_sound.c:287, 570); -1 otherwise.
    pub link_volume: i32,
}

include!("sounds_table.rs");

/// `sfx_pistol` (sounds.h:180).
pub const SFX_PISTOL: u16 = 1;
/// `sfx_sawup` (sounds.h:189), the first of the four saw sounds with the narrower pitch
/// perturbation (s_sound.c:327-330).
pub const SFX_SAWUP: u16 = 10;
/// `sfx_sawidl` (sounds.h:190).
pub const SFX_SAWIDL: u16 = 11;
/// `sfx_sawful` (sounds.h:191).
pub const SFX_SAWFUL: u16 = 12;
/// `sfx_sawhit` (sounds.h:192), the last saw sound.
pub const SFX_SAWHIT: u16 = 13;
/// `sfx_stnmov` (sounds.h:201).
pub const SFX_STNMOV: u16 = 22;
/// `sfx_itemup` (sounds.h:211): starts without a pitch draw (s_sound.c:337-338).
pub const SFX_ITEMUP: u16 = 32;
/// `sfx_chgun` (sounds.h:265): the only linked entry (sounds.c:204); never started by any
/// source file (M5c note §1.1.3).
pub const SFX_CHGUN: u16 = 86;
/// `sfx_tink` (sounds.h:266): starts without a pitch draw (s_sound.c:337-338).
pub const SFX_TINK: u16 = 87;

/// A sound number the C can index safely: 1..=108.
///
/// `S_StartSoundAtVolume` refuses `sfx_id < 1` with `I_Error` but lets `NUMSFX` (109) through
/// and reads past the table (s_sound.c:277-280, M5c note §1.1.13 item 1); both are refused here
/// (decision T11), so every `SfxId` indexes `S_sfx` in bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SfxId(u8);

impl SfxId {
    /// The id for sound number `n`, or `None` for 0 and for 109 and above.
    pub fn new(n: i64) -> Option<Self> {
        (1..NUMSFX as i64).contains(&n).then_some(Self(n as u8))
    }
    /// The sound number (1..=108).
    pub fn get(self) -> u16 {
        u16::from(self.0)
    }
    /// The index into [`S_SFX`] and the per-sfx arrays.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
    /// The table entry (`&S_sfx[sfx_id]`, s_sound.c:280).
    pub fn info(self) -> &'static SfxInfo {
        &S_SFX[self.index()]
    }
}
