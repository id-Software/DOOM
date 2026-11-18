//! The saved state of a session, `DSND` version 1 (M5c note §4.8), for restore-and-continue.
//!
//! A session snapshot is the `Simulation` clone, the display state and these bytes. Nothing about
//! the sound data is saved (the sound set is reloaded from the WADs) and nothing about `M_Random`
//! (its index belongs to the status bar's state, M5c note §4.3.2).
//!
//! Layout, little endian, exactly this many bytes (at most [`MAX_BYTES`]):
//!
//! | Offset | Size | Field |
//! | --- | --- | --- |
//! | 0 | 4 | magic `DSND` |
//! | 4 | 2 | version, 1 |
//! | 6 | 8 | tics run (the next tic; `gametic` after the last tic) |
//! | 14 | 8 | buffers mixed (must be `ceil(315 * tics / 512)`, the declared schedule) |
//! | 22 | 1 | `snd_SfxVolume` (8 in a fresh process; 0..=127) |
//! | 23 | 1 | `numChannels` N (3 in a fresh process; 0..=64) |
//! | 24 | 4 | `gamemap` (i32) |
//! | 28 | 4 | texture count of the T12 detector (0..=65536) |
//! | 32 | 1 | detector label: 0 none, 1 layout-dependent |
//! | 33 | 8 | the first tic it fired (0 without a label) |
//! | 41 | 8 | the number of firings (0 exactly without a label) |
//! | 49 | 14 | lump looked up, one bit per sound number 0..=108 (bit `n % 8` of byte `n / 8`; bit 0 and bits 109..=111 are 0) |
//! | 63 | | N sound-logic channels: tag 0 free, or 1 then the sfx (u8, 1..=108), the origin kind (u8: 0 none, 1 actor, 2 sector, 3 button) and its identity (actor: slot u32 and generation u64; sector: u16; button: u8 below 15; none: nothing), and the handle (i32, 100..=65535) |
//! | | 2 | the mixer's `handlenums` (u16, never 1..=99) |
//! | | | 8 mixer channels: tag 0 free, or 1 then sfx (u8), pos, end, step, frac (u32 each), start (i32), handle (i32), left, right (u8 each) |
//!
//! The "lump looked up" flags are not in the note's sketch: they decide only which start writes
//! the genuine L row (s_sound.c:363-365), and are saved so that a restored session writes the
//! same rows. A mixer channel's `channelids` entry is its sfx (i_sound.c:377) and is not stored.
//!
//! Decoding checks every count, index, handle and offset ([`StateError`]); restoring
//! ([`Sound::restore`]) also checks the state against the caller's own configuration (the bytes
//! never choose the map, volume, channel count or texture count of the session they restore
//! into), the loaded sound set and the simulation.
//! These are consistency checks, not a proof that a run reaches the state: they trust that the
//! occupied channels, positions and remainders came from some sequence of starts.

use std::sync::Arc;

use doom_sim::{ActorId, Simulation};

use crate::detector::{DetectorError, LayoutDetector, MAX_TEXTURES};
use crate::logic::{ChannelEntry, LogicConfig, MAX_CHANNELS, MAXBUTTONS, SoundLogic, SoundOrigin};
use crate::lumps::SoundSet;
use crate::mixer::{MixChannelState, MixStateError, Mixer, NUM_CHANNELS};
use crate::schedule::{Schedule, ScheduleError};
use crate::session::{Sound, SoundConfig};
use crate::sounds::{NUMSFX, SfxId};

/// `DSND`.
pub const MAGIC: [u8; 4] = *b"DSND";
/// The only version written and read.
pub const VERSION: u16 = 1;
/// The fixed part's length.
const HEADER: usize = 63;
/// The largest valid encoding: the header, 64 occupied actor channels of 19 bytes, the counter
/// and 8 busy mixer channels of 28 bytes.
pub const MAX_BYTES: usize = HEADER + MAX_CHANNELS * 19 + 2 + NUM_CHANNELS * 28;

/// A session's saved state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoundState {
    /// The configuration.
    pub config: SoundConfig,
    /// The schedule (tics run and buffers mixed).
    pub schedule: Schedule,
    /// The detector's label and count.
    pub detector: LayoutDetector,
    /// The "lump looked up" flags.
    pub looked_up: [bool; NUMSFX],
    /// The sound-logic channel table.
    pub channels: Vec<Option<ChannelEntry>>,
    /// The mixer's handle counter.
    pub handlenums: u16,
    /// The busy mixer channels.
    pub mixer: [Option<MixChannelState>; NUM_CHANNELS],
}

/// Why a saved state was refused. The target session is never changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateError {
    /// More than [`MAX_BYTES`].
    TooLong,
    /// The bytes end inside a field.
    Truncated,
    /// Bytes after the last field.
    Trailing,
    /// Not `DSND`.
    Magic,
    /// A version other than 1.
    Version(u16),
    /// `snd_SfxVolume` above 127 or more than 64 channels.
    Config,
    /// The schedule: too many tics, or a buffer count the rule does not give.
    Schedule(ScheduleError),
    /// The detector: too many textures, or a label without firings or the reverse, or a first
    /// tic not before the tics run, or a label byte other than 0 or 1.
    Detector,
    /// A "looked up" bit for sound number 0 or 109..=111, or (on restore) for a sound whose lump
    /// is missing (its lookup would have ended in `I_Error`).
    LookedUp {
        /// The bit.
        sfx: u16,
    },
    /// A tag other than 0 or 1.
    Tag,
    /// A sound number outside 1..=108.
    Sfx {
        /// The number.
        sfx: u8,
    },
    /// An origin kind above 3.
    OriginKind {
        /// The kind.
        kind: u8,
    },
    /// A button origin in slot 15 or above (p_spec.c:1151).
    Button {
        /// The slot.
        slot: u8,
    },
    /// Two channels with one origin (s_sound.c:349; M5c note §1.1.10 item 3).
    DuplicateOrigin {
        /// The second channel.
        cnum: usize,
    },
    /// A sound-logic handle outside 100..=65535.
    Handle {
        /// The channel.
        cnum: usize,
    },
    /// A mixer channel refused ([`MixStateError`]).
    Mixer(MixStateError),
    /// A mixer channel started at or after the tics run (`channelstart` is `gametic` at the
    /// start, i_sound.c:348), or below 0.
    StartTic {
        /// The channel.
        slot: usize,
    },
    /// A busy channel before any tic has run.
    BusyAtStart,
    /// Restore: the simulation has not completed exactly the state's tics.
    TicMismatch {
        /// The state's tics.
        state: u64,
        /// `Simulation::tic()`.
        simulation: u64,
    },
    /// Restore: an actor origin does not resolve in the simulation.
    ActorGone {
        /// The channel.
        cnum: usize,
    },
    /// Restore: a sector origin past the simulation's sectors.
    Sector {
        /// The channel.
        cnum: usize,
    },
    /// Restore: the saved configuration (map number, `snd_SfxVolume`, `numChannels`, texture
    /// count) is not the one the caller's session runs with. The bytes are never trusted to
    /// choose it: a state of another map would change the map-8 rule (s_sound.c:773-777,
    /// 800-808), another volume or channel count the draws and the channel choice, another
    /// texture count the layout detector of decision T12.
    ConfigMismatch {
        /// The configuration in the state.
        saved: SoundConfig,
        /// The configuration the caller expects.
        expected: SoundConfig,
    },
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], StateError> {
        let end = self.at.checked_add(N).ok_or(StateError::Truncated)?;
        let s = self.bytes.get(self.at..end).ok_or(StateError::Truncated)?;
        self.at = end;
        let mut out = [0; N];
        out.copy_from_slice(s);
        Ok(out)
    }
    fn u8(&mut self) -> Result<u8, StateError> {
        Ok(self.take::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16, StateError> {
        Ok(u16::from_le_bytes(self.take()?))
    }
    fn u32(&mut self) -> Result<u32, StateError> {
        Ok(u32::from_le_bytes(self.take()?))
    }
    fn i32(&mut self) -> Result<i32, StateError> {
        Ok(i32::from_le_bytes(self.take()?))
    }
    fn u64(&mut self) -> Result<u64, StateError> {
        Ok(u64::from_le_bytes(self.take()?))
    }
    fn tag(&mut self) -> Result<bool, StateError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(StateError::Tag),
        }
    }
    fn sfx(&mut self) -> Result<SfxId, StateError> {
        let n = self.u8()?;
        SfxId::new(i64::from(n)).ok_or(StateError::Sfx { sfx: n })
    }
}

impl SoundState {
    /// The `DSND` version 1 bytes. A state no session holds (a volume outside 0..=127 or more
    /// than 64 channels) encodes to bytes that [`Self::decode`] refuses, never to another state.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(MAX_BYTES);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&self.schedule.tic().to_le_bytes());
        out.extend_from_slice(&self.schedule.buffers().to_le_bytes());
        // Never truncated: a volume outside 0..=127 or more than 64 channels (which no session
        // holds: SoundLogic::new refuses them, but the fields are public) is written as 255,
        // which `decode` refuses (`Config`), so a forged value cannot wrap into an accepted one.
        let volume = u8::try_from(self.config.logic.sfx_volume)
            .ok()
            .filter(|v| *v <= 127);
        let channels = u8::try_from(self.channels.len())
            .ok()
            .filter(|n| usize::from(*n) <= MAX_CHANNELS);
        out.push(volume.unwrap_or(u8::MAX));
        out.push(channels.unwrap_or(u8::MAX));
        out.extend_from_slice(&self.config.logic.gamemap.to_le_bytes());
        out.extend_from_slice(&self.config.textures.to_le_bytes());
        let first = self.detector.layout_dependent_from();
        out.push(u8::from(first.is_some()));
        out.extend_from_slice(&first.unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&self.detector.firings().to_le_bytes());
        let mut bits = [0u8; 14];
        for (n, _) in self.looked_up.iter().enumerate().filter(|(_, b)| **b) {
            bits[n / 8] |= 1 << (n % 8);
        }
        out.extend_from_slice(&bits);
        for c in &self.channels {
            let Some(e) = c else {
                out.push(0);
                continue;
            };
            out.push(1);
            out.push(e.sfx.get() as u8);
            match e.origin {
                SoundOrigin::None => out.push(0),
                SoundOrigin::Actor(a) => {
                    out.push(1);
                    out.extend_from_slice(&a.slot().to_le_bytes());
                    out.extend_from_slice(&a.generation().to_le_bytes());
                }
                SoundOrigin::Sector(s) => {
                    out.push(2);
                    out.extend_from_slice(&s.to_le_bytes());
                }
                SoundOrigin::Button(b) => {
                    out.push(3);
                    out.push(b);
                }
            }
            out.extend_from_slice(&e.handle.to_le_bytes());
        }
        out.extend_from_slice(&self.handlenums.to_le_bytes());
        for c in &self.mixer {
            let Some(m) = c else {
                out.push(0);
                continue;
            };
            out.push(1);
            out.push(m.sfx.get() as u8);
            for v in [m.pos, m.end, m.step, m.frac] {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&m.start.to_le_bytes());
            out.extend_from_slice(&m.handle.to_le_bytes());
            out.push(m.left);
            out.push(m.right);
        }
        out
    }

    /// Reads `DSND` bytes, checking everything that needs neither the sound set nor the
    /// simulation ([`StateError`]). Never panics.
    pub fn decode(bytes: &[u8]) -> Result<Self, StateError> {
        if bytes.len() > MAX_BYTES {
            return Err(StateError::TooLong);
        }
        let mut r = Reader { bytes, at: 0 };
        if r.take::<4>()? != MAGIC {
            return Err(StateError::Magic);
        }
        let version = r.u16()?;
        if version != VERSION {
            return Err(StateError::Version(version));
        }
        let tic = r.u64()?;
        let buffers = r.u64()?;
        let schedule = Schedule::at(tic, buffers).map_err(StateError::Schedule)?;
        let sfx_volume = i32::from(r.u8()?);
        let channels = usize::from(r.u8()?);
        let gamemap = r.i32()?;
        let textures = r.u32()?;
        let logic = LogicConfig {
            gamemap,
            sfx_volume,
            channels,
        };
        if sfx_volume > 127 || channels > MAX_CHANNELS {
            return Err(StateError::Config);
        }
        let labelled = r.tag().map_err(|_| StateError::Detector)?;
        let first = r.u64()?;
        let firings = r.u64()?;
        if textures > MAX_TEXTURES || (!labelled && first != 0) || (labelled && first >= tic) {
            return Err(StateError::Detector);
        }
        let detector = LayoutDetector::from_parts(textures, labelled.then_some(first), firings)
            .map_err(|e| match e {
                DetectorError::Textures | DetectorError::Label => StateError::Detector,
            })?;
        let bits = r.take::<14>()?;
        let mut looked_up = [false; NUMSFX];
        for n in 0..112 {
            let set = bits[n / 8] & (1 << (n % 8)) != 0;
            if set && (n == 0 || n >= NUMSFX) {
                return Err(StateError::LookedUp { sfx: n as u16 });
            }
            if set {
                looked_up[n] = true;
            }
        }
        let mut table = Vec::with_capacity(channels);
        for cnum in 0..channels {
            if !r.tag()? {
                table.push(None);
                continue;
            }
            let sfx = r.sfx()?;
            let origin = match r.u8()? {
                0 => SoundOrigin::None,
                1 => {
                    let slot = r.u32()?;
                    SoundOrigin::Actor(ActorId::from_parts(slot, r.u64()?))
                }
                2 => SoundOrigin::Sector(r.u16()?),
                3 => {
                    let slot = r.u8()?;
                    if slot >= MAXBUTTONS - 1 {
                        return Err(StateError::Button { slot });
                    }
                    SoundOrigin::Button(slot)
                }
                kind => return Err(StateError::OriginKind { kind }),
            };
            let handle = r.i32()?;
            if !(100..=65535).contains(&handle) {
                return Err(StateError::Handle { cnum });
            }
            if table
                .iter()
                .flatten()
                .any(|e: &ChannelEntry| e.origin == origin)
            {
                return Err(StateError::DuplicateOrigin { cnum });
            }
            table.push(Some(ChannelEntry {
                sfx,
                origin,
                handle,
            }));
        }
        let handlenums = r.u16()?;
        if (1..100).contains(&handlenums) {
            return Err(StateError::Mixer(MixStateError::Counter));
        }
        let mut mixer = [None; NUM_CHANNELS];
        for (slot, m) in mixer.iter_mut().enumerate() {
            if !r.tag()? {
                continue;
            }
            let sfx = r.sfx()?;
            let c = MixChannelState {
                sfx,
                pos: r.u32()?,
                end: r.u32()?,
                step: r.u32()?,
                frac: r.u32()?,
                start: r.i32()?,
                handle: r.i32()?,
                left: r.u8()?,
                right: r.u8()?,
            };
            c.check(slot).map_err(StateError::Mixer)?;
            if u64::try_from(c.start).map_or(true, |start| start >= tic) {
                return Err(StateError::StartTic { slot });
            }
            *m = Some(c);
        }
        if r.at != bytes.len() {
            return Err(StateError::Trailing);
        }
        if tic == 0 && (table.iter().any(Option::is_some) || mixer.iter().any(Option::is_some)) {
            return Err(StateError::BusyAtStart);
        }
        Ok(Self {
            config: SoundConfig { logic, textures },
            schedule,
            detector,
            looked_up,
            channels: table,
            handlenums,
            mixer,
        })
    }
}

impl Sound {
    /// The session's saved state.
    pub fn state(&self) -> SoundState {
        SoundState {
            config: self.config,
            schedule: self.schedule,
            detector: self.detector,
            looked_up: *self.logic.looked_up(),
            channels: self.logic.channels().to_vec(),
            handlenums: self.mixer.handle_counter(),
            mixer: self.mixer.channel_states(),
        }
    }

    /// A session rebuilt from saved state over a loaded sound set, checking the state against
    /// the caller's configuration and the set: the saved configuration must equal `expected`
    /// (the configuration the caller's own session was built with, for example
    /// [`Sound::config`] of its fresh session; [`StateError::ConfigMismatch`]) and the
    /// detector's texture count the configuration's; each busy mixer channel's end is its
    /// sound's padded length and the sound has a sample byte, and no "looked up" flag is set for
    /// a sound whose lump is missing.
    pub fn from_state(
        set: Arc<SoundSet>,
        state: &SoundState,
        expected: SoundConfig,
    ) -> Result<Self, StateError> {
        if state.config != expected {
            return Err(StateError::ConfigMismatch {
                saved: state.config,
                expected,
            });
        }
        if state.detector.textures() != state.config.textures {
            return Err(StateError::Detector);
        }
        for (n, _) in state.looked_up.iter().enumerate().filter(|(_, b)| **b) {
            let id = SfxId::new(n as i64).ok_or(StateError::LookedUp { sfx: n as u16 })?;
            if set.lump_num(id).is_none() {
                return Err(StateError::LookedUp { sfx: n as u16 });
            }
        }
        let mixer =
            Mixer::from_state(set, state.handlenums, &state.mixer).map_err(StateError::Mixer)?;
        let logic =
            SoundLogic::from_parts(state.config.logic, state.channels.clone(), state.looked_up)
                .map_err(|_| StateError::Config)?;
        Ok(Self {
            config: state.config,
            logic,
            mixer,
            schedule: state.schedule,
            detector: state.detector,
        })
    }

    /// [`Self::from_state`] against `expected`, also checked against the simulation the session
    /// continues with: it has completed exactly the state's tics (`Simulation::tic`), every
    /// actor origin resolves (live or marked, `Simulation::sound_origin_xy`) and every sector
    /// origin is a sector of the map (`Simulation::sector_sound_origin`). The map number is
    /// checked only against `expected` (the simulation does not expose it under wire 25).
    pub fn restore(
        set: Arc<SoundSet>,
        state: &SoundState,
        sim: &Simulation,
        expected: SoundConfig,
    ) -> Result<Self, StateError> {
        if sim.tic() != state.schedule.tic() {
            return Err(StateError::TicMismatch {
                state: state.schedule.tic(),
                simulation: sim.tic(),
            });
        }
        for (cnum, e) in state.channels.iter().enumerate() {
            match e.map(|e| e.origin) {
                Some(SoundOrigin::Actor(a)) if sim.sound_origin_xy(a).is_none() => {
                    return Err(StateError::ActorGone { cnum });
                }
                Some(SoundOrigin::Sector(s)) if sim.sector_sound_origin(s).is_none() => {
                    return Err(StateError::Sector { cnum });
                }
                _ => {}
            }
        }
        Self::from_state(set, state, expected)
    }
}
