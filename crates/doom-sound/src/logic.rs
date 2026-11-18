//! The sound logic of `s_sound.c`: `S_StartSoundAtVolume` (s_sound.c:254-395) as `S_StartSound`
//! calls it (s_sound.c:397-407), `S_StopSound` (471-484), `S_UpdateSounds` (519-613),
//! `S_StopChannel` (708-742), `S_AdjustSoundParams` (752-818), `S_getChannel` (827-875) and
//! `S_Start`'s channel loop (202-211). M5c note §1.1 (1.1.12 is the summary this follows).
//!
//! The configuration is a fresh process's: `snd_SfxVolume` 8 and `numChannels` 3
//! (m_misc.c:237, 282; decision T9), the handle convention of the in-process build
//! (`I_StartSound` returns `addsfx`'s handle, i_sound.c:490-499; decision T2), and "still
//! playing" as `gametic < handle` (`I_SoundIsPlaying`, i_sound.c:517-521).
//!
//! The inputs are the simulation's sound records ([`doom_sim::sound::SoundRecord`]); a start's
//! positions are those read at the call ([`SoundPlacement`]), and an update reads current
//! positions through [`OriginPositions`].
//!
//! `M_Random` (m_random.c:63-67) is the miscellaneous stream of [`DoomRng`]: one draw per start
//! that passes the link-volume and audibility tests, none for `sfx_itemup` and `sfx_tink`
//! (s_sound.c:327-346); a start that then finds no channel has still drawn (s_sound.c:351-355).
//!
//! Every refusal ([`SoundRefusal`]) is decided before any state changes: the channel table, the
//! random index and the mixer are exactly as they were.

use doom_render::r_main::{finesine, fixed_mul, point_to_angle};
use doom_sim::ActorId;
pub use doom_sim::sound::{SoundOrigin, SoundPlacement, SoundRecord};
use doom_types::DoomRng;

use crate::mixer::{MixRefusal, MixStart, Mixer};
use crate::sounds::{NUMSFX, SFX_ITEMUP, SFX_SAWHIT, SFX_SAWUP, SFX_TINK, SfxId};

/// `S_CLIPPING_DIST` (s_sound.c:55): 1200.0 in 16.16 fixed point.
pub const S_CLIPPING_DIST: i32 = 1200 * 0x10000;
/// `S_CLOSE_DIST` (s_sound.c:61): 160.0.
pub const S_CLOSE_DIST: i32 = 160 * 0x10000;
/// `S_ATTENUATOR` (s_sound.c:64): `(S_CLIPPING_DIST - S_CLOSE_DIST) >> FRACBITS` = 1040.
pub const S_ATTENUATOR: i32 = (S_CLIPPING_DIST - S_CLOSE_DIST) >> 16;
/// `NORM_PITCH` (s_sound.c:69).
pub const NORM_PITCH: i32 = 128;
/// `NORM_PRIORITY` (s_sound.c:70).
pub const NORM_PRIORITY: i32 = 64;
/// `NORM_SEP` (s_sound.c:71).
pub const NORM_SEP: i32 = 128;
/// `S_STEREO_SWING` (s_sound.c:74): 96.0.
pub const S_STEREO_SWING: i32 = 96 * 0x10000;
/// `ANGLETOFINESHIFT` (tables.h:55).
const ANGLETOFINESHIFT: u32 = 19;
/// `MAXBUTTONS` (p_spec.h:244): the slots of `buttonlist` (p_switch.c:101).
pub const MAXBUTTONS: u8 = 16;
/// The largest channel count accepted (`numChannels` is unchecked in the C, m_misc.c:282,
/// 367-402; this bound is a declared resource limit, the default being 3).
pub const MAX_CHANNELS: usize = 64;

/// The sound logic's configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogicConfig {
    /// `gamemap`: the map number (the map-8 rule of s_sound.c:773-777, 800-808 tests it alone).
    pub gamemap: i32,
    /// `snd_SfxVolume`: 8 in a fresh process (m_misc.c:237; decision T9).
    pub sfx_volume: i32,
    /// `numChannels`: 3 in a fresh process (m_misc.c:282; decision T9).
    pub channels: usize,
}

impl LogicConfig {
    /// A fresh process's configuration on map `gamemap`: volume 8, 3 channels (decision T9).
    pub fn fresh(gamemap: i32) -> Self {
        Self {
            gamemap,
            sfx_volume: 8,
            channels: 3,
        }
    }
}

/// Why the sound logic refused a call. The state is unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoundRefusal {
    /// A sound number below 1 (`I_Error("Bad sfx #: %d")`, s_sound.c:277-278) or of 109 and above
    /// (109 passes the C's check and reads past `S_sfx`, s_sound.c:277-280; decision T11).
    SfxId {
        /// The number given.
        sfx: u16,
    },
    /// The first start of a sound whose `ds` lump is missing, once it has a channel:
    /// `I_GetSfxLumpNum` ends in `I_Error` (s_sound.c:363-365, i_sound.c:451-456,
    /// w_wad.c:399-409; decision T11). Made after the draw and the channel choice, as in the C,
    /// and refused with neither kept.
    LumpMissing {
        /// The sound.
        sfx: u16,
    },
    /// The mixer refused the start (`I_StartSound`, s_sound.c:389-394).
    Mix(MixRefusal),
    /// A switch-release origin in the last button slot: the C reads past `buttonlist`
    /// (p_spec.c:1151 with `MAXBUTTONS` 16, p_spec.h:244, p_switch.c:101; decisions T11, T12).
    ButtonPastArray {
        /// The slot.
        slot: u8,
    },
    /// A start record whose placement does not match its origin (s_sound.c:304 measures exactly
    /// an origin that is neither NULL nor `players[consoleplayer].mo`): a NULL origin with a
    /// placement, or without one a sector or button origin (never the listener) or, in a
    /// session ([`crate::session::Sound::tic`]), an actor that is not the listener. Such a
    /// record is the producer's fault; playing it unmeasured would also skip the layout
    /// detector of decision T12 for a switch release.
    Placement,
    /// `S_UpdateSounds` found an occupied channel whose origin's position cannot be read: a
    /// broken invariant (every removal stops its origin's channel, p_mobj.c:565-566), never a
    /// silent free (M5c note §4.5 item 3).
    OriginGone {
        /// The channel.
        cnum: usize,
    },
    /// A configuration the C refuses or this port bounds: `snd_SfxVolume` outside 0..=127
    /// (`S_SetSfxVolume`'s `I_Error`, s_sound.c:634-635) or more than [`MAX_CHANNELS`] channels.
    Config,
}

/// What `S_AdjustSoundParams` (s_sound.c:752-818) did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Adjusted {
    /// Clipped (s_sound.c:773-777): not map 8 and farther than `S_CLIPPING_DIST`. Returns 0
    /// without writing `*vol` or `*sep`.
    Clipped,
    /// `*vol` and `*sep` were written; the return value is `vol > 0` (s_sound.c:817).
    Computed {
        /// `*vol`.
        vol: i32,
        /// `*sep`.
        sep: i32,
    },
}

impl Adjusted {
    /// The return value of `S_AdjustSoundParams`.
    pub fn audible(self) -> bool {
        matches!(self, Self::Computed { vol, .. } if vol > 0)
    }
}

/// `S_AdjustSoundParams(listener, source, &vol, &sep, &pitch)` (s_sound.c:752-818), a pure
/// function of the listener's x, y and angle, the source's x, y, `gamemap` and `snd_SfxVolume`.
/// `*pitch` is never written by the C. A `snd_SfxVolume` outside 0..=127, which the C refuses
/// when it is set (`S_SetSfxVolume`, s_sound.c:634-635) and no [`SoundLogic`] holds, is refused
/// as [`SoundRefusal::Config`] (the volume arithmetic of s_sound.c:808-815 could overflow).
///
/// The arithmetic is the reference build's (`-fwrapv`, M5c note §3.2.2): the differences, `abs`
/// and the distance sum wrap (s_sound.c:767-771; undefined in ISO C, §1.1.13 item 2); the right
/// shift of a negative product is arithmetic and the division truncates toward zero (§1.1.13
/// item 3).
pub fn adjust_sound_params(
    listener: [i32; 2],
    listener_angle: u32,
    source: [i32; 2],
    gamemap: i32,
    sfx_volume: i32,
) -> Result<Adjusted, SoundRefusal> {
    if !(0..=127).contains(&sfx_volume) {
        return Err(SoundRefusal::Config);
    }
    // s_sound.c:767-768.
    let adx = listener[0].wrapping_sub(source[0]).wrapping_abs();
    let ady = listener[1].wrapping_sub(source[1]).wrapping_abs();
    // s_sound.c:771: adx + ady - (min >> 1).
    let mut approx_dist = adx.wrapping_add(ady).wrapping_sub(adx.min(ady) >> 1);
    // s_sound.c:773-777.
    if gamemap != 8 && approx_dist > S_CLIPPING_DIST {
        return Ok(Adjusted::Clipped);
    }
    // s_sound.c:780-783: R_PointToAngle2(listener, source) (r_main.c:378-388).
    let mut angle = point_to_angle(source[0], source[1], listener[0], listener[1]);
    // s_sound.c:785-788: the else branch is angle - listener_angle - 1 mod 2^32.
    if angle > listener_angle {
        angle = angle.wrapping_sub(listener_angle);
    } else {
        angle = angle.wrapping_add(0xffff_ffff - listener_angle);
    }
    angle >>= ANGLETOFINESHIFT; // s_sound.c:790: 0..8191
    let sine = finesine(angle as i32).expect("finesine index 0..8191 is in the table");
    // s_sound.c:793.
    let sep = 128 - (fixed_mul(S_STEREO_SWING, sine) >> 16);
    // s_sound.c:796-815.
    let vol = if approx_dist < S_CLOSE_DIST {
        sfx_volume
    } else if gamemap == 8 {
        if approx_dist > S_CLIPPING_DIST {
            approx_dist = S_CLIPPING_DIST;
        }
        15 + ((sfx_volume - 15) * ((S_CLIPPING_DIST - approx_dist) >> 16)) / S_ATTENUATOR
    } else {
        (sfx_volume * ((S_CLIPPING_DIST - approx_dist) >> 16)) / S_ATTENUATOR
    };
    Ok(Adjusted::Computed { vol, sep })
}

/// One occupied channel of `channels[]` (s_sound.c:93-108).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelEntry {
    /// `sfxinfo`: the sound (a free channel is `None` in the table).
    pub sfx: SfxId,
    /// `origin`: compared by identity only.
    pub origin: SoundOrigin,
    /// `handle`: what `I_StartSound` returned (s_sound.c:389).
    pub handle: i32,
}

/// Why a channel was freed (the K rows of the genuine sound stream, M5c note §3.1.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopReason {
    /// `S_StopSound(origin)` inside a start (s_sound.c:349).
    SameOrigin,
    /// `S_getChannel`'s same-origin branch (s_sound.c:842-846), unreachable from s_sound.c:352.
    SameOriginGetChannel,
    /// The priority eviction of `S_getChannel` (s_sound.c:853-865).
    Priority,
    /// A game `S_StopSound`: only `P_RemoveMobj` (p_mobj.c:565-566).
    Remove,
    /// `S_Start` (s_sound.c:209-211).
    LevelStart,
    /// `S_UpdateSounds`: no longer playing (s_sound.c:600-605).
    Finished,
    /// `S_UpdateSounds`: playing but inaudible (s_sound.c:592-595).
    Inaudible,
    /// `S_UpdateSounds`: a linked sound's volume below 1 (s_sound.c:571-575).
    LinkVolume,
}

/// One freed channel: `S_StopChannel(cnum)` (s_sound.c:708-742) on an occupied channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelStop {
    /// The channel.
    pub cnum: usize,
    /// What it held.
    pub entry: ChannelEntry,
    /// Why.
    pub reason: StopReason,
    /// Whether `I_SoundIsPlaying(handle)` was true (`gametic < handle`), in which case the C
    /// calls the empty `I_StopSound` (s_sound.c:717-724, i_sound.c:505-514).
    pub playing: bool,
}

/// How a start ended (M5c note §3.1.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartOutcome {
    /// A linked sound with volume below 1 (s_sound.c:283-290).
    LinkVolume,
    /// `S_AdjustSoundParams` returned 0 (s_sound.c:318-319).
    Inaudible,
    /// `S_getChannel` returned -1 (s_sound.c:352-355).
    NoChannel,
    /// `I_StartSound` was called (s_sound.c:389-394).
    Started,
}

/// What one start did: everything an S row of the genuine sound stream records
/// (DOOM-M5C-SOUND-v1, M5c note §3.1.4), plus the channels it freed and the mixer's echo.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StartEcho {
    /// The sound.
    pub sfx: SfxId,
    /// The origin.
    pub origin: SoundOrigin,
    /// The outcome.
    pub outcome: StartOutcome,
    /// The `M_Random` value drawn (`rndtable[rndindex]` after the increment), if any.
    pub rnd: Option<u8>,
    /// For a positional start, what `S_AdjustSoundParams` did (s_sound.c:306-310).
    pub adjusted: Option<Adjusted>,
    /// Whether the origin stands on the listener's x, y (s_sound.c:312-316).
    pub samexy: bool,
    /// The arguments of `I_StartSound` (vol, sep, pitch, priority), for a started sound.
    pub args: Option<[i32; 4]>,
    /// The channel taken, for a started sound.
    pub cnum: Option<usize>,
    /// The handle `I_StartSound` returned, for a started sound.
    pub handle: Option<i32>,
    /// Whether this start made the sound's `I_GetSfxLumpNum` call (the first start of that sound
    /// that obtained a channel, s_sound.c:363-365): an L row of the genuine stream.
    pub lump_lookup: bool,
    /// The mixer's echo, for a started sound.
    pub mix: Option<MixStart>,
    /// Channels freed by the start, in order (at most two: s_sound.c:349 and 853-865).
    pub stops: [Option<ChannelStop>; 2],
}

impl StartEcho {
    /// The number of `M_Random` draws (0 or 1).
    pub fn draws(&self) -> u8 {
        u8::from(self.rnd.is_some())
    }
}

/// What one record did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordEcho {
    /// A start.
    Start(StartEcho),
    /// `S_StopSound` from `P_RemoveMobj`: the channel it freed, if any.
    Stop(Option<ChannelStop>),
}

/// The listener at an `S_UpdateSounds` (s_sound.c:519-529; d_main.c:392 passes
/// `players[consoleplayer].mo`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Listener {
    /// The console player's body (the origin identity `listener_p != c->origin` compares,
    /// s_sound.c:584).
    pub actor: ActorId,
    /// Its x, y (16.16).
    pub xy: [i32; 2],
    /// Its angle.
    pub angle: u32,
}

/// The current x, y of each kind of origin, read by `S_UpdateSounds` (s_sound.c:586-590).
pub trait OriginPositions {
    /// A live (or marked, not yet retired) actor's x, y.
    fn actor_xy(&self, actor: ActorId) -> Option<[i32; 2]>;
    /// `sectors[sector].soundorg`'s x, y.
    fn sector_xy(&self, sector: u16) -> Option<[i32; 2]>;
    /// The switch-release origin of button slot `slot`, under the declared 64-bit reading of
    /// p_spec.c:1151 (the next slot's timer and 0; decision T12).
    fn button_xy(&self, slot: u8) -> Option<[i32; 2]>;
}

/// What one `S_UpdateSounds` did.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UpdateEcho {
    /// The channels freed, in channel order.
    pub stops: Vec<ChannelStop>,
    /// For each channel that called `I_UpdateSoundParams` (s_sound.c:597): (cnum, vol, sep,
    /// pitch). The call has no effect (i_sound.c:674-688); kept for the C rows.
    pub params: Vec<(usize, i32, i32, i32)>,
}

/// The channel table and the per-sound "lump looked up" flags.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoundLogic {
    config: LogicConfig,
    channels: Vec<Option<ChannelEntry>>,
    looked_up: [bool; NUMSFX],
}

impl SoundLogic {
    /// `S_Init`'s channel table (s_sound.c:179-184): every channel free; no lump looked up yet
    /// (`lumpnum = -1`, s_sound.c:190-191).
    pub fn new(config: LogicConfig) -> Result<Self, SoundRefusal> {
        if !(0..=127).contains(&config.sfx_volume) || config.channels > MAX_CHANNELS {
            return Err(SoundRefusal::Config);
        }
        Ok(Self {
            config,
            channels: vec![None; config.channels],
            looked_up: [false; NUMSFX],
        })
    }

    /// The configuration.
    pub fn config(&self) -> LogicConfig {
        self.config
    }

    /// The channel table: `None` where `sfxinfo` is NULL.
    pub fn channels(&self) -> &[Option<ChannelEntry>] {
        &self.channels
    }

    /// Whether `I_GetSfxLumpNum` has run for sound `sfx` (`lumpnum >= 0`, s_sound.c:363-365).
    pub fn lump_looked_up(&self, sfx: SfxId) -> bool {
        self.looked_up[sfx.index()]
    }

    /// The "lump looked up" flag of every sound number (index 0 is always false).
    pub fn looked_up(&self) -> &[bool; NUMSFX] {
        &self.looked_up
    }

    /// A table rebuilt from saved state, after checking it (M5c note §4.8.2): the configuration
    /// as [`Self::new`] checks it, exactly `config.channels` entries, at most one occupied
    /// channel per origin (NULL included: s_sound.c:349 frees the first channel of the origin
    /// before every start, M5c note §1.1.10 item 3), every handle in 100..=65535 (what
    /// `addsfx` hands out, i_sound.c:335-340), a button origin below slot 15 (p_spec.c:1151),
    /// and no flag for sound number 0. Whether actor and sector origins exist is the caller's
    /// check, against the simulation.
    pub fn from_parts(
        config: LogicConfig,
        channels: Vec<Option<ChannelEntry>>,
        looked_up: [bool; NUMSFX],
    ) -> Result<Self, SoundRefusal> {
        let empty = Self::new(config)?;
        if channels.len() != empty.channels.len() || looked_up[0] {
            return Err(SoundRefusal::Config);
        }
        for (i, c) in channels.iter().enumerate() {
            let Some(e) = c else { continue };
            if !(100..=65535).contains(&e.handle) {
                return Err(SoundRefusal::Config);
            }
            if let SoundOrigin::Button(slot) = e.origin {
                if slot >= MAXBUTTONS - 1 {
                    return Err(SoundRefusal::ButtonPastArray { slot });
                }
            }
            if channels[..i]
                .iter()
                .flatten()
                .any(|other| other.origin == e.origin)
            {
                return Err(SoundRefusal::Config);
            }
        }
        Ok(Self {
            config,
            channels,
            looked_up,
        })
    }

    /// `S_StopChannel(cnum)` (s_sound.c:708-742) on a channel known to be occupied.
    fn stop_channel(
        channels: &mut [Option<ChannelEntry>],
        cnum: usize,
        reason: StopReason,
        gametic: i32,
    ) -> Option<ChannelStop> {
        let entry = channels[cnum].take()?;
        Some(ChannelStop {
            cnum,
            entry,
            reason,
            playing: gametic < entry.handle, // i_sound.c:520
        })
    }

    /// `S_StopSound(origin)` (s_sound.c:471-484) over a table: the first occupied channel with
    /// this origin (NULL included) is freed.
    fn stop_first(
        channels: &mut [Option<ChannelEntry>],
        origin: SoundOrigin,
        reason: StopReason,
        gametic: i32,
    ) -> Option<ChannelStop> {
        let cnum = channels
            .iter()
            .position(|c| c.is_some_and(|c| c.origin == origin))?;
        Self::stop_channel(channels, cnum, reason, gametic)
    }

    /// `S_StopSound(origin)` as the game calls it (only `P_RemoveMobj`, p_mobj.c:565-566).
    pub fn stop(&mut self, origin: SoundOrigin, gametic: i32) -> Option<ChannelStop> {
        Self::stop_first(&mut self.channels, origin, StopReason::Remove, gametic)
    }

    /// `S_Start`'s channel loop (s_sound.c:209-211): every occupied channel is freed.
    pub fn level_start(&mut self, gametic: i32) -> Vec<ChannelStop> {
        (0..self.channels.len())
            .filter_map(|c| {
                Self::stop_channel(&mut self.channels, c, StopReason::LevelStart, gametic)
            })
            .collect()
    }

    /// Runs one record of a tic: a start ([`Self::start`]) or a removal stop ([`Self::stop`]).
    pub fn apply(
        &mut self,
        record: &SoundRecord,
        rng: &mut DoomRng,
        mixer: &mut Mixer,
        gametic: i32,
    ) -> Result<RecordEcho, SoundRefusal> {
        match *record {
            SoundRecord::Start { sfx, origin, at } => self
                .start(sfx, origin, at, rng, mixer, gametic)
                .map(RecordEcho::Start),
            SoundRecord::Stop { actor } => Ok(RecordEcho::Stop(
                self.stop(SoundOrigin::Actor(actor), gametic),
            )),
        }
    }

    /// `S_StartSound(origin, sfx)` = `S_StartSoundAtVolume(origin, sfx, snd_SfxVolume)`
    /// (s_sound.c:397-407, 254-395), with `gametic` the global's value at the call.
    ///
    /// `at` is present exactly for a positional start, an origin that is neither NULL nor the
    /// listener's body (s_sound.c:304), and holds the positions read at the call. A NULL origin
    /// with a placement and a sector or button origin without one are refused
    /// ([`SoundRefusal::Placement`]); an actor origin without one is taken as the listener's
    /// body, which only the session can check.
    pub fn start(
        &mut self,
        sfx: u16,
        origin: SoundOrigin,
        at: Option<SoundPlacement>,
        rng: &mut DoomRng,
        mixer: &mut Mixer,
        gametic: i32,
    ) -> Result<StartEcho, SoundRefusal> {
        // s_sound.c:277-280 (decision T11 for 109).
        let id = SfxId::new(i64::from(sfx)).ok_or(SoundRefusal::SfxId { sfx })?;
        if let SoundOrigin::Button(slot) = origin {
            if slot >= MAXBUTTONS - 1 {
                return Err(SoundRefusal::ButtonPastArray { slot });
            }
        }
        // s_sound.c:304: a NULL origin is never measured; a sector's `soundorg` or a button's
        // (p_spec.c:1151) is never `players[consoleplayer].mo`, so it is always measured and
        // its start must carry the positions read at the call. (An actor that is not the
        // listener is the session's check, `Sound::tic`: the logic does not know the listener.)
        let measured = matches!(origin, SoundOrigin::Sector(_) | SoundOrigin::Button(_));
        if (origin == SoundOrigin::None && at.is_some()) || (measured && at.is_none()) {
            return Err(SoundRefusal::Placement);
        }
        let info = id.info();
        let snd = self.config.sfx_volume;
        let mut echo = StartEcho {
            sfx: id,
            origin,
            outcome: StartOutcome::Started,
            rnd: None,
            adjusted: None,
            samexy: false,
            args: None,
            cnum: None,
            handle: None,
            lump_lookup: false,
            mix: None,
            stops: [None; 2],
        };
        // s_sound.c:283-299.
        let mut volume = snd;
        let (mut pitch, priority) = if info.link.is_some() {
            volume += info.link_volume;
            if volume < 1 {
                echo.outcome = StartOutcome::LinkVolume;
                return Ok(echo);
            }
            if volume > snd {
                volume = snd;
            }
            (info.link_pitch, info.priority)
        } else {
            (NORM_PITCH, NORM_PRIORITY)
        };
        // s_sound.c:304-324.
        let sep = match at {
            Some(p) => {
                let adjusted = adjust_sound_params(
                    p.listener,
                    p.listener_angle,
                    p.origin,
                    self.config.gamemap,
                    snd,
                )?;
                echo.adjusted = Some(adjusted);
                echo.samexy = p.origin == p.listener;
                let Adjusted::Computed { vol, sep } = adjusted else {
                    echo.outcome = StartOutcome::Inaudible;
                    return Ok(echo);
                };
                if vol <= 0 {
                    echo.outcome = StartOutcome::Inaudible;
                    return Ok(echo);
                }
                volume = vol;
                if echo.samexy { NORM_SEP } else { sep }
            }
            None => NORM_SEP,
        };
        // s_sound.c:327-346, on a copy of the index (committed only if the start is kept).
        let mut r = *rng;
        let n = sfx;
        if (SFX_SAWUP..=SFX_SAWHIT).contains(&n) {
            let v = r.m_random();
            echo.rnd = Some(v);
            pitch = (pitch + 8 - i32::from(v & 15)).clamp(0, 255);
        } else if n != SFX_ITEMUP && n != SFX_TINK {
            let v = r.m_random();
            echo.rnd = Some(v);
            pitch = (pitch + 16 - i32::from(v & 31)).clamp(0, 255);
        }
        // s_sound.c:349 then 352-355, on a copy of the table.
        let mut table = self.channels.clone();
        echo.stops[0] = Self::stop_first(&mut table, origin, StopReason::SameOrigin, gametic);
        let (cnum, stop) = Self::get_channel(&mut table, origin, id, gametic);
        echo.stops[1] = stop;
        let Some(cnum) = cnum else {
            echo.outcome = StartOutcome::NoChannel;
            *rng = r;
            self.channels = table;
            return Ok(echo);
        };
        // s_sound.c:363-365: the first lookup of a missing lump ends in I_Error.
        let set = mixer.sound_set();
        if set.lump_num(id).is_none() {
            return Err(SoundRefusal::LumpMissing { sfx });
        }
        echo.lump_lookup = !self.looked_up[id.index()];
        // s_sound.c:389-394: I_StartSound(sfx_id, volume, sep, pitch, priority).
        let mix = mixer
            .start(id, volume, sep, pitch, gametic)
            .map_err(SoundRefusal::Mix)?;
        table[cnum] = Some(ChannelEntry {
            sfx: id,
            origin,
            handle: mix.handle,
        });
        echo.args = Some([volume, sep, pitch, priority]);
        echo.cnum = Some(cnum);
        echo.handle = Some(mix.handle);
        echo.mix = Some(mix);
        *rng = r;
        self.channels = table;
        self.looked_up[id.index()] = true;
        Ok(echo)
    }

    /// `S_getChannel(origin, sfxinfo)` (s_sound.c:827-875) over a table: the channel chosen (its
    /// entry is installed by the caller once the start is kept) and the channel it freed.
    fn get_channel(
        table: &mut [Option<ChannelEntry>],
        origin: SoundOrigin,
        sfx: SfxId,
        gametic: i32,
    ) -> (Option<usize>, Option<ChannelStop>) {
        let n = table.len();
        let mut stop = None;
        // s_sound.c:838-847: the first free channel, or one with the same non-NULL origin.
        let mut cnum = 0;
        while cnum < n {
            match table[cnum] {
                None => break,
                Some(e) if origin != SoundOrigin::None && e.origin == origin => {
                    stop =
                        Self::stop_channel(table, cnum, StopReason::SameOriginGetChannel, gametic);
                    break;
                }
                Some(_) => cnum += 1,
            }
        }
        if cnum == n {
            // s_sound.c:853-865: every channel is occupied here; the first whose table priority
            // is >= the new sound's is evicted, else none (-1).
            let prio = sfx.info().priority;
            cnum = match table
                .iter()
                .position(|c| c.is_some_and(|c| c.sfx.info().priority >= prio))
            {
                Some(c) => c,
                None => return (None, None),
            };
            stop = Self::stop_channel(table, cnum, StopReason::Priority, gametic);
        }
        (Some(cnum), stop)
    }

    /// `S_UpdateSounds(listener)` (s_sound.c:519-613) with `gametic` the global's value: for each
    /// occupied channel in order, free it if `gametic >= handle` (s_sound.c:560, 600-605);
    /// otherwise, for a linked sound with volume below 1, free it (571-575); otherwise, for an
    /// origin that is not NULL and not the listener, recompute audibility from the current
    /// positions and free it if inaudible (584-595). No same-position rule here (contrast
    /// s_sound.c:312-316), no draw.
    ///
    /// Every position is read before any channel is freed, so a missing one refuses
    /// ([`SoundRefusal::OriginGone`]) with the table unchanged; freeing does not change what a
    /// later channel of the same call reads.
    pub fn update(
        &mut self,
        listener: &Listener,
        positions: &dyn OriginPositions,
        gametic: i32,
    ) -> Result<UpdateEcho, SoundRefusal> {
        let snd = self.config.sfx_volume;
        // (cnum, Ok(params of I_UpdateSoundParams) or Err(why the channel is freed)).
        let mut plan: Vec<(usize, Result<[i32; 3], StopReason>)> = Vec::new();
        for (cnum, c) in self.channels.iter().enumerate() {
            let Some(e) = c else { continue };
            if gametic >= e.handle {
                plan.push((cnum, Err(StopReason::Finished)));
                continue;
            }
            // s_sound.c:563-580: the volume set here is overwritten by S_AdjustSoundParams
            // before any use, so only the link test and the pitch matter.
            let info = e.sfx.info();
            let mut pitch = NORM_PITCH;
            if info.link.is_some() {
                pitch = info.link_pitch;
                if snd + info.link_volume < 1 {
                    plan.push((cnum, Err(StopReason::LinkVolume)));
                    continue;
                }
            }
            let xy = match e.origin {
                SoundOrigin::None => continue,
                SoundOrigin::Actor(a) if a == listener.actor => continue,
                SoundOrigin::Actor(a) => positions.actor_xy(a),
                SoundOrigin::Sector(s) => positions.sector_xy(s),
                SoundOrigin::Button(b) => positions.button_xy(b),
            };
            let xy = xy.ok_or(SoundRefusal::OriginGone { cnum })?;
            match adjust_sound_params(listener.xy, listener.angle, xy, self.config.gamemap, snd)? {
                Adjusted::Computed { vol, sep } if vol > 0 => {
                    plan.push((cnum, Ok([vol, sep, pitch])));
                }
                // s_sound.c:592-595.
                _ => plan.push((cnum, Err(StopReason::Inaudible))),
            }
        }
        let mut echo = UpdateEcho::default();
        for (cnum, step) in plan {
            match step {
                Ok([v, sep, p]) => echo.params.push((cnum, v, sep, p)),
                Err(reason) => {
                    if let Some(stop) =
                        Self::stop_channel(&mut self.channels, cnum, reason, gametic)
                    {
                        echo.stops.push(stop);
                    }
                }
            }
        }
        Ok(echo)
    }
}
