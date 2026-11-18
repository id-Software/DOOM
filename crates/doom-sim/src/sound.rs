//! Canonical sound records of the Episode 1 part 3 policy (wire 25, M5c decision T3).
//!
//! Under `OriginalExecutionProfile::PlayerMonstersEpisode1V3` the simulation
//! records, in the C call order within a tic, every `S_StartSound` a level tic makes
//! and every `S_StopSound` made by `P_RemoveMobj` (p_mobj.c:565-566). The records
//! are the canonical DSIM41 group `events.sounds`; they leave `events.last` and
//! `events.pending` exactly as wire 24 emits them (apart from the corrected
//! hard-death scream). No sound logic, no `M_Random` and no mixer run here: the
//! records are what `S_StartSoundAtVolume` (s_sound.c:254-391) is given, with the
//! positions it reads at the moment of the call.
//!
//! Older policies never produce a record.

use crate::ActorId;

/// What `S_StartSound`'s first argument points at (M5c note §1.2.2, the five kinds
/// of origin): the identity `S_StopSound` compares (s_sound.c:349, 471-482).
///
/// The listener's own body is an [`SoundOrigin::Actor`] like any other actor; the
/// sound logic tells it apart by comparing the handle with the listener, as the C
/// compares `origin != players[consoleplayer].mo` (s_sound.c:302).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SoundOrigin {
    /// A NULL origin: full volume, centre separation (s_sound.c:304, 321-324).
    None,
    /// A `mobj_t`, by its generational handle (pointer identity: `P_RemoveMobj`
    /// stops its channel before the memory can be reused, p_mobj.c:565-569).
    Actor(ActorId),
    /// `&sectors[index].soundorg` (r_defs.h:121; computed once at load,
    /// p_setup.c:536-557).
    Sector(u16),
    /// `(mobj_t *)&buttonlist[slot].soundorg`, the switch-release origin of
    /// `P_UpdateSpecials` (p_spec.c:1151): the address of the pointer field, not
    /// its value (M5c note §1.2.6.3, §4.4.5).
    Button(u8),
}

/// The positions `S_AdjustSoundParams` reads (s_sound.c:761-793), taken at the
/// moment of the call: raw 16.16 `fixed_t` bits and an `angle_t`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SoundPlacement {
    /// `origin->x`, `origin->y` (for a button origin, the declared 64-bit reading
    /// of p_spec.c:1151: the next slot's `btimer` and 0).
    pub origin: [i32; 2],
    /// `players[consoleplayer].mo->x`, `->y`.
    pub listener: [i32; 2],
    /// `players[consoleplayer].mo->angle`.
    pub listener_angle: u32,
}

/// One sound call of a tic, in the C call order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SoundRecord {
    /// `S_StartSound(origin, sfx)` (s_sound.c:425-431).
    Start {
        /// The `sfxenum_t` value (sounds.h:179-288), 1..=108 at every call site.
        sfx: u16,
        /// The origin's identity.
        origin: SoundOrigin,
        /// Present exactly for a positional start: an origin that is not NULL and
        /// not the listener's own body (s_sound.c:302).
        at: Option<SoundPlacement>,
    },
    /// `S_StopSound(mobj)` from `P_RemoveMobj` (p_mobj.c:565-566).
    Stop {
        /// The removed actor.
        actor: ActorId,
    },
}
