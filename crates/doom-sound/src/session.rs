//! One sound session: the sound logic, the mixer and the declared schedule of one fresh process
//! started straight on a map (M5c decisions T5 and T15; note §4.2.3 and §4.3.3).
//!
//! Per tic, [`Sound::tic`] does what the C does between two `gametic++`, in the C order:
//!
//! 1. every sound record of the tic, in call order, through the sound logic with `gametic == t`
//!    and the `M_Random` stream (`S_StartSoundAtVolume` and `S_StopSound` inside `P_Ticker`,
//!    g_game.c:727-733): the caller passes the index in and gets it back with the number of
//!    draws; the session does not own it (the status bar's state does, M5c note §4.3.2 (i));
//! 2. `S_UpdateSounds` with `gametic == t + 1`, the listener after the tic and the positions of
//!    the occupied origins (d_main.c:392), after every tic (decision T5);
//! 3. the buffer due under the declared rule ([`crate::schedule`]), mixed by `I_UpdateSound`
//!    (d_main.c:397-400).
//!
//! The layout detector of decision T12 ([`crate::detector`]) watches every release start and
//! every later update read of a release origin; it never changes what the session does.
//!
//! A tic is transactional: it runs on a copy of the session (about 300 bytes; no simulation is
//! cloned) and a refusal leaves the logic, the mixer, the schedule and the caller's random index
//! exactly as they were (M5c note §4.9).

use std::sync::Arc;

use doom_sim::Simulation;
use doom_types::DoomRng;

use crate::detector::LayoutDetector;
use crate::logic::{
    ChannelEntry, Listener, LogicConfig, OriginPositions, RecordEcho, SoundLogic, SoundOrigin,
    SoundPlacement, SoundRecord, SoundRefusal, StopReason, UpdateEcho,
};
use crate::lumps::SoundSet;
use crate::mixer::{Buffer, Mixer};
use crate::schedule::Schedule;
use crate::sounds::SfxId;

/// A session's configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SoundConfig {
    /// The sound logic's configuration: the map number, and a fresh process's volume 8 and 3
    /// channels (decision T9, [`LogicConfig::fresh`]).
    pub logic: LogicConfig,
    /// The texture count of the loaded WADs (`numtextures`, r_data.c:480;
    /// `Resources::textures.len()`), which the layout detector of decision T12 needs
    /// ([`crate::detector`]). At most [`crate::detector::MAX_TEXTURES`].
    pub textures: u32,
}

impl SoundConfig {
    /// A fresh process on map `gamemap` (decisions T9, T15) with `textures` textures loaded.
    pub fn fresh(gamemap: i32, textures: u32) -> Self {
        Self {
            logic: LogicConfig::fresh(gamemap),
            textures,
        }
    }
}

/// Where in a tic a refusal was made.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefusedAt {
    /// The record at this index of the tic's records.
    Record(usize),
    /// The `S_UpdateSounds` after the tic.
    Update,
}

/// Why a session refused a tic. The session is unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionRefusal {
    /// The sound logic refused a record or the update.
    Sound {
        /// The session tic.
        tic: u64,
        /// Where.
        at: RefusedAt,
        /// Why.
        refusal: SoundRefusal,
    },
    /// The session has run [`crate::schedule::MAX_TIC`] tics: `gametic` would leave `int`.
    TicLimit,
    /// The tic's starts would make more than 65,535 `M_Random` draws, which [`TicSound::draws`]
    /// cannot count (a declared bound of this port: each start draws at most once,
    /// s_sound.c:327-346, so this needs more than 65,535 start records in one tic; the C has no
    /// such count). Refused at the record that would exceed it, with nothing changed.
    DrawCount {
        /// The session tic.
        tic: u64,
        /// The record.
        record: usize,
    },
    /// [`Sound::tic_from_simulation`]: the simulation has no listener to read (no bound console
    /// player body), which `S_UpdateSounds` dereferences (s_sound.c:529, 584-590).
    NoListener,
    /// [`Sound::tic_from_simulation`]: the simulation has not completed exactly the session's
    /// next tic (`Simulation::tic` must be the session tic plus one).
    TicMismatch {
        /// The session's next tic.
        session: u64,
        /// `Simulation::tic()`.
        simulation: u64,
    },
}

impl std::fmt::Display for SessionRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sound {
                tic,
                at: RefusedAt::Record(i),
                refusal,
            } => write!(f, "tic {tic}, sound call {i}: {refusal}"),
            Self::Sound {
                tic,
                at: RefusedAt::Update,
                refusal,
            } => write!(f, "tic {tic}, sound update: {refusal}"),
            Self::TicLimit => write!(f, "the sound session reached its tic limit"),
            Self::DrawCount { tic, record } => write!(
                f,
                "tic {tic}, sound call {record}: more than 65535 random draws in one tic"
            ),
            Self::NoListener => write!(f, "the player has no body to hear sounds"),
            Self::TicMismatch {
                session,
                simulation,
            } => write!(
                f,
                "the sound session is at tic {session} but the simulation at tic {simulation}"
            ),
        }
    }
}

impl std::error::Error for SessionRefusal {}

/// What one tic did.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TicSound {
    /// The session tic (`gametic` while it ran).
    pub tic: u64,
    /// What each record did, in order.
    pub records: Vec<RecordEcho>,
    /// What the `S_UpdateSounds` after the tic did.
    pub update: UpdateEcho,
    /// The `M_Random` draws the tic's starts made.
    pub draws: u16,
    /// The `M_Random` index after them (the value to hand on to `ST_Ticker`).
    pub rndindex: u8,
    /// The buffer mixed after the tic, if one was due (at most one per tic).
    pub buffer: Option<Buffer>,
}

/// Positions read from a simulation: actor origins by handle (marked actors stay readable until
/// they retire), sector origins computed at load, and button-release origins under the declared
/// 64-bit reading (decision T12): `Simulation::sound_origin_xy`, `sector_sound_origin` and
/// `button_release_xy`.
pub struct SimulationPositions<'a>(pub &'a Simulation);

impl OriginPositions for SimulationPositions<'_> {
    fn actor_xy(&self, actor: doom_sim::ActorId) -> Option<[i32; 2]> {
        self.0.sound_origin_xy(actor)
    }
    fn sector_xy(&self, sector: u16) -> Option<[i32; 2]> {
        self.0.sector_sound_origin(sector)
    }
    fn button_xy(&self, slot: u8) -> Option<[i32; 2]> {
        self.0.button_release_xy(slot).ok()
    }
}

/// The listener a simulation gives after a tic (`players[consoleplayer].mo`, d_main.c:392), if
/// its console player has a body.
pub fn simulation_listener(sim: &Simulation) -> Option<Listener> {
    sim.sound_listener().map(|l| Listener {
        actor: l.actor,
        xy: l.xy,
        angle: l.angle,
    })
}

/// The placement of a start record.
fn at_of(record: &SoundRecord) -> Option<SoundPlacement> {
    match *record {
        SoundRecord::Start { at, .. } => at,
        SoundRecord::Stop { .. } => None,
    }
}

/// One session's sound state.
#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
    pub(crate) config: SoundConfig,
    pub(crate) logic: SoundLogic,
    pub(crate) mixer: Mixer,
    pub(crate) schedule: Schedule,
    pub(crate) detector: LayoutDetector,
}

impl Sound {
    /// A fresh process started straight on the map (decision T15): `gametic` 0, the handle
    /// counter 0, no channel occupied, the mixer silent, no buffer mixed.
    /// More than [`crate::detector::MAX_TEXTURES`] textures is refused as
    /// [`SoundRefusal::Config`].
    pub fn new(set: Arc<SoundSet>, config: SoundConfig) -> Result<Self, SoundRefusal> {
        Ok(Self {
            config,
            logic: SoundLogic::new(config.logic)?,
            mixer: Mixer::new(set),
            schedule: Schedule::new(),
            detector: LayoutDetector::new(config.textures).map_err(|_| SoundRefusal::Config)?,
        })
    }

    /// The configuration.
    pub fn config(&self) -> SoundConfig {
        self.config
    }

    /// The sound logic (the channel table).
    pub fn logic(&self) -> &SoundLogic {
        &self.logic
    }

    /// The mixer.
    pub fn mixer(&self) -> &Mixer {
        &self.mixer
    }

    /// The schedule: the next tic and the buffers mixed.
    pub fn schedule(&self) -> Schedule {
        self.schedule
    }

    /// The layout detector of decision T12: the tic from which the session is layout-dependent
    /// and the number of firings ([`crate::detector`]).
    pub fn detector(&self) -> LayoutDetector {
        self.detector
    }

    /// Runs the next tic (see the module documentation): the tic's `records` in order with the
    /// `M_Random` index `rndindex` (the miscellaneous stream, m_random.c:63-67), then
    /// `S_UpdateSounds` with `listener` and `positions` read after the tic, then the due buffer.
    /// A start of an actor other than `listener.actor` without a placement is refused
    /// ([`SoundRefusal::Placement`]): the C measures it (s_sound.c:304). On a refusal nothing
    /// changes.
    pub fn tic(
        &mut self,
        records: &[SoundRecord],
        rndindex: u8,
        listener: &Listener,
        positions: &dyn OriginPositions,
    ) -> Result<TicSound, SessionRefusal> {
        let tic = self.schedule.tic();
        let (gametic, after) = self
            .schedule
            .gametics()
            .map_err(|_| SessionRefusal::TicLimit)?;
        let next = self
            .schedule
            .advanced()
            .map_err(|_| SessionRefusal::TicLimit)?;
        let refused = |at, refusal| SessionRefusal::Sound { tic, at, refusal };
        let mut logic = self.logic.clone();
        let mut mixer = self.mixer.clone();
        let mut detector = self.detector;
        let LogicConfig {
            gamemap,
            sfx_volume,
            ..
        } = self.config.logic;
        let mut rng = DoomRng::from_indices(0, rndindex);
        let mut echoes = Vec::with_capacity(records.len());
        let mut draws: u16 = 0;
        for (i, record) in records.iter().enumerate() {
            // s_sound.c:304: an actor other than `players[consoleplayer].mo` is always measured,
            // so its start must carry a placement (the sound number is checked first, as the
            // logic checks it before the placement). The body after the tic is the body at the
            // call: `players[0].mo` is set only by P_SpawnPlayer (p_mobj.c:675), which a
            // single-player session reaches again only through a reborn's level reload
            // (g_game.c:928-931), never inside a tic.
            if let SoundRecord::Start {
                sfx,
                origin: SoundOrigin::Actor(a),
                at: None,
            } = *record
            {
                if a != listener.actor {
                    let why = match SfxId::new(i64::from(sfx)) {
                        Some(_) => SoundRefusal::Placement,
                        None => SoundRefusal::SfxId { sfx },
                    };
                    return Err(refused(RefusedAt::Record(i), why));
                }
            }
            let echo = logic
                .apply(record, &mut rng, &mut mixer, gametic)
                .map_err(|r| refused(RefusedAt::Record(i), r))?;
            if let RecordEcho::Start(s) = &echo {
                draws = draws
                    .checked_add(u16::from(s.draws()))
                    .ok_or(SessionRefusal::DrawCount { tic, record: i })?;
                // Decision T12: a release start that reached S_AdjustSoundParams.
                if let (SoundOrigin::Button(_), Some(at), Some(_)) =
                    (s.origin, at_of(record), s.adjusted)
                {
                    detector
                        .check_start(tic, &at, gamemap, sfx_volume)
                        .map_err(|r| refused(RefusedAt::Record(i), r))?;
                }
            }
            echoes.push(echo);
        }
        let table = logic.channels().to_vec();
        let update = logic
            .update(listener, positions, after)
            .map_err(|r| refused(RefusedAt::Update, r))?;
        // Decision T12: every channel the update read a position for (s_sound.c:584-595) whose
        // origin is a release origin.
        let read = update.params.iter().map(|p| p.0).chain(
            update
                .stops
                .iter()
                .filter(|s| s.reason == StopReason::Inaudible)
                .map(|s| s.cnum),
        );
        for cnum in read {
            if let Some(Some(ChannelEntry {
                origin: SoundOrigin::Button(slot),
                ..
            })) = table.get(cnum)
            {
                if let Some(xy) = positions.button_xy(*slot) {
                    detector
                        .check_update(tic, listener, xy, gamemap, sfx_volume)
                        .map_err(|r| refused(RefusedAt::Update, r))?;
                }
            }
        }
        let buffer = (self.schedule.due() == 1).then(|| {
            let mut out: Buffer = [[0; 2]; crate::lumps::SAMPLECOUNT];
            mixer.mix_buffer(&mut out);
            out
        });
        // Commit: nothing below can fail.
        self.logic = logic;
        self.mixer = mixer;
        self.schedule = next;
        self.detector = detector;
        Ok(TicSound {
            tic,
            records: echoes,
            update,
            draws,
            rndindex: rng.indices()[1],
            buffer,
        })
    }

    /// [`Self::tic`] with the records, the listener and the positions read from a wire-25
    /// simulation that has just completed the session's next tic (`Simulation::sound_records`,
    /// [`simulation_listener`], [`SimulationPositions`]).
    pub fn tic_from_simulation(
        &mut self,
        sim: &Simulation,
        rndindex: u8,
    ) -> Result<TicSound, SessionRefusal> {
        let session = self.schedule.tic();
        if session.checked_add(1) != Some(sim.tic()) {
            return Err(SessionRefusal::TicMismatch {
                session,
                simulation: sim.tic(),
            });
        }
        let listener = simulation_listener(sim).ok_or(SessionRefusal::NoListener)?;
        self.tic(
            sim.sound_records(),
            rndindex,
            &listener,
            &SimulationPositions(sim),
        )
    }
}
