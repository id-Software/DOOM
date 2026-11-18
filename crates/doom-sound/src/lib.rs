//! Platform-free port of linuxdoom-1.10's sound effects: the sound logic of `s_sound.c` and the
//! in-process mixer of `i_sound.c`, with no audio device.
//!
//! The behavioural reference is linuxdoom-1.10 built with its in-process sound code (one define
//! removed): `doomdef.h:84` (`#define SNDSERV 1`) commented out, so that `I_StartSound` calls
//! `addsfx` and returns its handle (i_sound.c:490-499) and `I_UpdateSound` mixes in process
//! (d_main.c:397-400). M5c decision T2; see docs/M5C_REFERENCE_INVESTIGATION.md, sections 1.1
//! (the sound logic), 1.3 (the mixer) and 4.2-4.6 (this crate's design).
//!
//! Modules, in the order of the C they port:
//!
//! - [`sounds`]: the 109-entry `S_sfx` table (sounds.h, sounds.c).
//! - [`tables`]: `steptable` and `vol_lookup` (`I_SetChannels`, i_sound.c:396-424), with no
//!   floating point at run time.
//! - [`lumps`]: the padded sound data (`getsfx` and `I_InitSound`'s load loop, i_sound.c:185-251,
//!   799-813) and the lump lookup of `I_GetSfxLumpNum` (i_sound.c:451-456).
//! - [`mixer`]: `I_StartSound` with `addsfx` and `I_UpdateSound` (i_sound.c:264-381, 470-521,
//!   539-654).
//!
//! Every place where the C ends in `I_Error` or reads memory it does not own is a typed refusal
//! made before any state changes (M5c decision T11). Every audible quirk of the source is
//! reproduced as it is (decision T13).

pub mod lumps;
pub mod mixer;
pub mod sounds;
pub mod tables;
