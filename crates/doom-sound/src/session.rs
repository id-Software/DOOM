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
//! A tic is transactional: it runs on a copy of the session (about 300 bytes; no simulation is
//! cloned) and a refusal leaves the logic, the mixer, the schedule and the caller's random index
//! exactly as they were (M5c note §4.9).

use std::sync::Arc;

use doom_sim::Simulation;
use doom_types::DoomRng;

use crate::logic::{
    Listener, LogicConfig, OriginPositions, RecordEcho, SoundLogic, SoundRecord, SoundRefusal,
    UpdateEcho,
};
use crate::lumps::SoundSet;
use crate::mixer::{Buffer, Mixer};
use crate::schedule::Schedule;

/// A session's configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SoundConfig {
    /// The sound logic's configuration: the map number, and a fresh process's volume 8 and 3
    /// channels (decision T9, [`LogicConfig::fresh`]).
    pub logic: LogicConfig,
}

impl SoundConfig {
    /// A fresh process on map `gamemap` (decisions T9, T15).
    pub fn fresh(gamemap: i32) -> Self {
        Self {
            logic: LogicConfig::fresh(gamemap),
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

/// One session's sound state.
#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
    pub(crate) config: SoundConfig,
    pub(crate) logic: SoundLogic,
    pub(crate) mixer: Mixer,
    pub(crate) schedule: Schedule,
}

impl Sound {
    /// A fresh process started straight on the map (decision T15): `gametic` 0, the handle
    /// counter 0, no channel occupied, the mixer silent, no buffer mixed.
    pub fn new(set: Arc<SoundSet>, config: SoundConfig) -> Result<Self, SoundRefusal> {
        Ok(Self {
            config,
            logic: SoundLogic::new(config.logic)?,
            mixer: Mixer::new(set),
            schedule: Schedule::new(),
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

    /// Runs the next tic (see the module documentation): the tic's `records` in order with the
    /// `M_Random` index `rndindex` (the miscellaneous stream, m_random.c:63-67), then
    /// `S_UpdateSounds` with `listener` and `positions` read after the tic, then the due buffer.
    /// On a refusal nothing changes.
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
        let mut rng = DoomRng::from_indices(0, rndindex);
        let mut echoes = Vec::with_capacity(records.len());
        let mut draws: u16 = 0;
        for (i, record) in records.iter().enumerate() {
            let echo = logic
                .apply(record, &mut rng, &mut mixer, gametic)
                .map_err(|r| refused(RefusedAt::Record(i), r))?;
            if let RecordEcho::Start(s) = &echo {
                draws = draws.saturating_add(u16::from(s.draws()));
            }
            echoes.push(echo);
        }
        let update = logic
            .update(listener, positions, after)
            .map_err(|r| refused(RefusedAt::Update, r))?;
        let buffer = (self.schedule.due() == 1).then(|| {
            let mut out: Buffer = [[0; 2]; crate::lumps::SAMPLECOUNT];
            mixer.mix_buffer(&mut out);
            out
        });
        // Commit: nothing below can fail.
        self.logic = logic;
        self.mixer = mixer;
        self.schedule = next;
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
