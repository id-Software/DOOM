//! Platform-free port of linuxdoom-1.10's sound effects: the sound logic of `s_sound.c` and the
//! in-process mixer of `i_sound.c`, with no audio device.
//!
//! The behavioural reference is linuxdoom-1.10 built with its in-process sound code (one define
//! removed): `doomdef.h:84` (`#define SNDSERV 1`) commented out, so that `I_StartSound` calls
//! `addsfx` and returns its handle (i_sound.c:490-499) and `I_UpdateSound` mixes in process
//! (d_main.c:397-400). M5c decision T2; see docs/M5C_REFERENCE_INVESTIGATION.md, sections 1.1
//! (the sound logic), 1.3 (the mixer) and 4.2-4.6 (this crate's design).
//!
//! Every place where the C ends in `I_Error` or reads memory it does not own is a typed refusal
//! made before any state changes (M5c decision T11). Every audible quirk of the source is
//! reproduced as it is (decision T13).
