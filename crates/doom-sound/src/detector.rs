//! The layout detector of decision T12 (the owner's ruling of 2026-11-18, M5c note §5 "Owner
//! rulings", with the orchestrator's reading recorded there that later `S_UpdateSounds` reads
//! are covered too).
//!
//! The switch release passes the address of a pointer field, `(mobj_t *)&buttonlist[i].soundorg`
//! (p_spec.c:1151), so `S_AdjustSoundParams` reads its x, y out of the next button slot
//! (s_sound.c:767-768): on the 64-bit build x is slot i+1's `btimer` and y the padding after it
//! (0); on the 32-bit build x is slot i+1's `btexture` and y its `btimer` (`button_t`,
//! p_spec.h:228-235; offsets observed in M5c note §1.2.6.3 and §4.4.5). Both read 0, 0 while slot
//! i+1 is free (a free slot is all zero: p_spec.c:1152, 1357-1358; `btimer` is 0 exactly when the
//! slot is free, p_switch.c:178, p_spec.c:1129). The session follows the declared 64-bit
//! reading always; this detector says whether the 32-bit reading could have changed anything:
//!
//! - at a release start whose next slot is busy, `S_AdjustSoundParams` is evaluated for every
//!   position the 32-bit build could read there (x = any texture number the loaded WAD can hold,
//!   in raw fixed-point units; y = the next slot's timer) against the declared reading (x = the
//!   timer, y = 0); it fires if audibility, volume or separation (after the same-position rule,
//!   s_sound.c:312-316) would differ for any candidate (the `M_Random` draw is made exactly when
//!   the start is audible, s_sound.c:318-346, so it differs exactly when audibility does);
//! - at every later `S_UpdateSounds` read of a release origin (an occupied channel that is still
//!   playing, s_sound.c:560-595), the same with the next slot's then-current values, for
//!   audible-or-not, the only result `S_UpdateSounds` uses (s_sound.c:592-595).
//!
//! If nothing would differ, nothing is recorded. When it fires, the session is labelled
//! layout-dependent from that tic (the first tic is kept) and the firings are counted; the
//! session continues on the 64-bit reading. Both are part of the saved state.
//!
//! The texture count is a configuration value: the caller takes it from the texture directory
//! (`numtextures` of `R_InitTextures`, r_data.c:480, which is `Resources::textures.len()` in
//! doom-render). `btexture` is a texture number from `switchlist` (p_switch.c:182, filled by
//! `R_TextureNumForName` in P_InitSwitchList, p_switch.c:144-145), so it is below that count.

use crate::logic::{Adjusted, Listener, SoundPlacement, SoundRefusal, adjust_sound_params};

/// The largest texture count accepted: a declared bound on the work of one evaluation (one
/// `S_AdjustSoundParams` per candidate).
pub const MAX_TEXTURES: u32 = 1 << 16;

/// The detector's state: the texture count, the first tic it fired and how often.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct LayoutDetector {
    textures: u32,
    first: Option<u64>,
    firings: u64,
}

/// Why a detector state was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DetectorError {
    /// More than [`MAX_TEXTURES`] textures.
    Textures,
    /// A first tic without a firing, or firings without a first tic.
    Label,
}

/// What a start or a later read does with one position: `None` when inaudible, otherwise the
/// volume and the separation `I_StartSound` would get (s_sound.c:306-324).
fn start_effect(adjusted: Adjusted, samexy: bool) -> Option<(i32, i32)> {
    match adjusted {
        Adjusted::Computed { vol, sep } if vol > 0 => {
            Some((vol, if samexy { crate::logic::NORM_SEP } else { sep }))
        }
        _ => None,
    }
}

impl LayoutDetector {
    /// A detector that has never fired, for a WAD with `textures` textures.
    pub fn new(textures: u32) -> Result<Self, DetectorError> {
        Self::from_parts(textures, None, 0)
    }

    /// A detector rebuilt from saved state: the first tic is present exactly when the count is
    /// not 0.
    pub fn from_parts(
        textures: u32,
        first: Option<u64>,
        firings: u64,
    ) -> Result<Self, DetectorError> {
        if textures > MAX_TEXTURES {
            return Err(DetectorError::Textures);
        }
        if first.is_some() != (firings > 0) {
            return Err(DetectorError::Label);
        }
        Ok(Self {
            textures,
            first,
            firings,
        })
    }

    /// The texture count.
    pub fn textures(&self) -> u32 {
        self.textures
    }

    /// The tic from which the session is layout-dependent, if the detector has fired.
    pub fn layout_dependent_from(&self) -> Option<u64> {
        self.first
    }

    /// How many evaluations have fired.
    pub fn firings(&self) -> u64 {
        self.firings
    }

    fn fire(&mut self, tic: u64) {
        self.first.get_or_insert(tic);
        self.firings = self.firings.saturating_add(1);
    }

    /// The 32-bit candidates for a next slot whose timer is `timer`: (texture, timer).
    fn candidates(&self, timer: i32) -> impl Iterator<Item = [i32; 2]> {
        // textures <= 2^16, so every texture number fits in i32.
        (0..self.textures).map(move |t| [t as i32, timer])
    }

    /// A release start (origin `Button(i)`) at session tic `tic` whose positions were read
    /// under the declared reading (`at.origin` = (slot i+1's timer, 0)), on map `gamemap` with
    /// `snd_SfxVolume` `sfx_volume`. Returns whether it fired; a volume outside 0..=127 is
    /// refused as [`adjust_sound_params`] refuses it, with the detector unchanged.
    pub fn check_start(
        &mut self,
        tic: u64,
        at: &SoundPlacement,
        gamemap: i32,
        sfx_volume: i32,
    ) -> Result<bool, SoundRefusal> {
        let effect = |origin: [i32; 2]| {
            adjust_sound_params(at.listener, at.listener_angle, origin, gamemap, sfx_volume)
                .map(|a| start_effect(a, origin == at.listener))
        };
        let declared = effect(at.origin)?;
        let timer = at.origin[0];
        if timer == 0 {
            // The next slot is free: both builds read 0, 0.
            return Ok(false);
        }
        // The volume was accepted above, so no candidate is refused.
        let fired = self
            .candidates(timer)
            .any(|c| effect(c).ok() != Some(declared));
        if fired {
            self.fire(tic);
        }
        Ok(fired)
    }

    /// A later `S_UpdateSounds` read of a release origin, at the end of session tic `tic`, whose
    /// declared position now is `declared` (= (slot i+1's timer, 0)). Returns whether it fired;
    /// a volume outside 0..=127 is refused, with the detector unchanged.
    pub fn check_update(
        &mut self,
        tic: u64,
        listener: &Listener,
        declared: [i32; 2],
        gamemap: i32,
        sfx_volume: i32,
    ) -> Result<bool, SoundRefusal> {
        let audible = |origin: [i32; 2]| {
            adjust_sound_params(listener.xy, listener.angle, origin, gamemap, sfx_volume)
                .map(Adjusted::audible)
        };
        let want = audible(declared)?;
        let timer = declared[0];
        if timer == 0 {
            return Ok(false);
        }
        // The volume was accepted above, so no candidate is refused.
        let fired = self
            .candidates(timer)
            .any(|c| audible(c).ok() != Some(want));
        if fired {
            self.fire(tic);
        }
        Ok(fired)
    }
}
