//! The platform-free app: the session, the 35 Hz loop step, pause, the end and refusal
//! screens and restart (decisions S3, S8–S12, S21; M5b investigation §3.4, §3.6).
//!
//! The platform owns the window and the monotonic clock. It feeds [`App`] input already in
//! DOOM key codes ([`App::key`], [`App::mouse_motion`], [`App::mouse_button`],
//! [`App::focus_lost`]), calls [`App::update`] with the current monotonic time in
//! microseconds, and shows [`App::picture`]. Files and the date come in through [`Host`]. No
//! clock, file or platform type reaches the simulation: a session's tics are a function of
//! its events and of how many tics each update ran, and the demo it writes replays them.
//!
//! One tic (S8): the event layer's events are posted, `G_BuildTiccmd` builds the command, the
//! demo write and re-read gives the executed command ([`RecordedTic`]), the recorder stores it
//! and the simulation advances with the stored copy; the frame source then follows the tic and
//! draws it. Every executed tic is drawn; the picture shown is the last one. With the opt-in
//! [`SessionConfig::unrecorded_full_turning`] (S3 (ii)) the built command itself is executed and
//! nothing is recorded or written; every screen then carries [`UNRECORDED_LABEL`].
//!
//! Sound (M5c, decision T15): game data loaded with [`GameData::load_with_sound`] runs wire 25 and
//! a fresh process's sound session in every session and restart, with the display under
//! doom-render's `RandomContract::WithSound` (the order within a tic is in [`FrameSource`]'s
//! documentation). The mixed buffers leave through [`Host::audio`]'s [`AudioSink`], which also
//! hears the app's pause, resume and session starts; a sound refusal ends the session through the
//! same typed refusal path as every other ([`Witness::Sound`]). [`GameData::load`] keeps M5b's
//! configuration exactly (wire 24, no sound, the sound-free, wipe-free contract) for the frozen
//! M5b tests, examples and tools.

mod audio;
mod frames;
mod gamma;
mod recorder;
mod screens;

pub use audio::{AudioSink, RecordingSink, SinkCall};
pub use frames::{
    FrameAssets, FrameRefusal, FrameSource, PALETTE_BYTES, PresentationRestoreError, Presented,
    SavedPresentation, SoundAssets, StatusAdjustment,
};
pub use gamma::GAMMA;
pub use recorder::{
    DEMO_HEADER_BYTES, DemoCommands, RecordError, Recorder, TIC_CAP, Timestamp, session_name,
    wire24_admits,
};
pub use screens::{
    APP_PALETTE, CLEAR, Screen, glyph, label_band, labelled_pause_overlay, pause_overlay,
};

use crate::input::{Event, EventLayer, Input, InputError, MouseButton, Preset, RecordedTic, keys};
use doom_render::hu_stuff::Message;
use doom_render::{
    DisplayFrame, Frame, GameMode, LumpDirectory, ResourceError, Resources, SCREENHEIGHT,
    SCREENWIDTH,
};
use doom_sim::{
    MapContextOptions, PlayerSlot, SimError, Simulation, Skill, TicCommand, WeaponInventory,
};
use doom_sound::detector::LayoutDetector;
use doom_sound::lumps::{SoundCaps, SoundLoadError, SoundSet};
use doom_sound::session::{SessionRefusal, Sound, SoundConfig, TicSound};
use std::fmt;
use std::sync::Arc;

/// Logical tics per second (`TICRATE`, doomdef.h:122).
pub const TICRATE: u64 = 35;
/// The most tics one [`App::update`] runs; the surplus is dropped (`NetUpdate`'s bound,
/// d_net.c:404; decision S8).
pub const MAX_TICS_PER_UPDATE: u64 = 5;
/// The demo is rewritten whole every this many tics while playing (30 seconds), so a crash loses
/// at most that much (M5b investigation §3.6). A host that writes every tic ([`Host::appends`])
/// still gets this whole rewrite (doom-play's is flushed to the disk), between its appends.
pub const SAVE_INTERVAL: usize = 35 * 30;
/// After a failed write of a host that appends ([`Host::appends`]), the app retries a whole
/// rewrite at the next recorded tic, and while those retries keep failing, at most once every
/// this many recorded tics (one second of play), instead of at every tic (M5d review MIN-1: a
/// synced whole rewrite at every tic could stall the game loop under a persistent failure). An
/// owner-facing choice that can be changed. The whole rewrite every [`SAVE_INTERVAL`] tics, at
/// the session's end and on restart and quit is made as before, and appends resume only after a
/// whole rewrite has succeeded. A host that does not opt in never reaches this path.
pub const FAILED_WRITE_RETRY_TICS: usize = TICRATE as usize;
/// The label of a session with [`SessionConfig::unrecorded_full_turning`], wherever its state is
/// shown (decision S3 (ii), owner ruling 2026-11-17).
pub const UNRECORDED_LABEL: &str = "not recorded: no demo, cannot be replayed";
/// While playing, the pictures of an unrecorded session's first this many tics (five seconds)
/// carry [`UNRECORDED_LABEL`] in a band over the top ([`label_band`]): the picture before the
/// first tic and those of tics 0 to 174. Only the presented picture carries it, never the game
/// frame; a recorded session never shows it.
pub const UNRECORDED_LABEL_TICS: u64 = 5 * TICRATE;
/// The pause overlay's label while the last write of the demo failed (the reason is on the end
/// screen and on doom-play's standard error).
pub const SAVE_FAILED_LABEL: &str = "LAST DEMO SAVE FAILED";
/// The most names [`App`] tries for a session file: the base name, then `-2` to `-9999`.
pub const MAX_NAME_SUFFIX: u32 = 9999;
/// The bytes of [`App::picture`]: 320×200 RGBA.
pub const PICTURE_BYTES: usize = SCREENWIDTH * SCREENHEIGHT * 4;

/// The platform services the app needs outside the simulation.
pub trait Host {
    /// The local date and time, for a new session's file name.
    fn timestamp(&mut self) -> Timestamp;
    /// Whether a session file of this name already exists.
    fn exists(&mut self, name: &str) -> bool;
    /// Creates the session file `name` with `bytes`, never replacing an existing file:
    /// `Ok(false)` when the name is taken. The default checks [`Host::exists`] and then saves,
    /// which is enough for a host no other process writes to; a host on a shared directory
    /// creates the file atomically (doom-play opens it with `create_new`).
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        if self.exists(name) {
            return Ok(false);
        }
        self.save(name, bytes).map(|()| true)
    }
    /// Rewrites the session file `name`, which [`Host::claim`] created, whole.
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String>;
    /// Whether the host keeps the session file current tic by tic (M5d): `true` makes the app
    /// offer every committed, recorded tic to [`Host::append`] (except the tics on which it
    /// rewrites the file whole anyway, every [`SAVE_INTERVAL`] tics). The default, `false`, keeps
    /// the M5b cadence exactly: the file is written when the session begins, every
    /// [`SAVE_INTERVAL`] tics, at the session's end and on restart and quit when behind, and
    /// [`Host::append`] is never called.
    fn appends(&self) -> bool {
        false
    }
    /// Extends the session file `name` by one tic: writes `tail` (the tic's four demo bytes, then
    /// the end marker `DEMOMARKER`, 0x80, g_game.c:1488) at byte `at`, where the file's old marker
    /// is, so the file grows by four bytes. The app calls it only while the file holds exactly the
    /// demo of every earlier tic (the last write succeeded and left `at + 1` bytes) and only when
    /// [`Host::appends`] says so. A host that cannot make the five bytes land all or nothing at
    /// this offset returns `Ok(false)`, and the app then rewrites the file whole with
    /// [`Host::save`]; `Ok(true)` means written; an error is a failed save
    /// ([`SaveStatus::Failed`]), after which the app rewrites the file whole at the next tic
    /// (and, while that keeps failing, at most once every [`FAILED_WRITE_RETRY_TICS`] tics).
    /// The default declines.
    fn append(&mut self, _name: &str, _at: u64, _tail: &[u8; 5]) -> Result<bool, String> {
        Ok(false)
    }
    /// Called after every committed and drawn tic, for headless session verification (M5b
    /// investigation §2.3); a window has no use for it.
    fn tic(&mut self, _report: &TicReport<'_>) {}
    /// The sink of the mixed sound stream (M5c), if the host has one. The default has none: the
    /// sound session still runs (its draws decide the face), and its buffers are dropped.
    fn audio(&mut self) -> Option<&mut dyn AudioSink> {
        None
    }
}

/// What the sound session did in a committed tic (M5c).
#[derive(Clone, Copy, Debug)]
pub struct SoundReport<'a> {
    /// The tic's records, update, draws, index and buffer.
    pub tic: &'a TicSound,
    /// The session after the tic: its channel table (`logic().channels()`), mixer, schedule and
    /// the layout detector of decision T12 (`detector().layout_dependent_from()` is the tic from
    /// which the session is labelled layout-dependent, `detector().firings()` how often it fired;
    /// both persist in every later report).
    pub session: &'a Sound,
}

/// One committed and drawn tic, as [`Host::tic`] receives it.
#[derive(Clone, Copy, Debug)]
pub struct TicReport<'a> {
    /// The tic's number in the genuine runners' numbering: the simulation's tic count before it.
    pub tic: u64,
    /// The events posted before the tic's command was built (none in a [`replay`]).
    pub events: &'a [Event],
    /// The executed command: the demo's re-read copy (decision S3).
    pub executed: TicCommand,
    /// The simulation after the tic.
    pub simulation: &'a Simulation,
    /// The tic's frame.
    pub frame: &'a Frame,
    /// The frame's palette number, palette set, face, message flag and view report.
    pub display: &'a DisplayFrame,
    /// The message `HU_Ticker` took in the tic.
    pub message: Option<Message>,
    /// The `M_Random` index after the tic (with sound, after the wipe's draws on the first
    /// frame).
    pub rndindex: u8,
    /// With sound (M5c): what the sound session did.
    pub sound: Option<SoundReport<'a>>,
}

/// Replays a session's demo through the simulation and the app's frame source as play does,
/// reporting every tic to `host`: the headless check of a recorded session (M5b investigation
/// §2.3). Returns how the replay ended early, or `None` after the last command.
pub fn replay(
    data: &GameData,
    screenblocks: u8,
    commands: &DemoCommands,
    host: &mut dyn Host,
) -> Option<Ending> {
    replay_full_resolution(data, screenblocks, commands.commands(), host)
}

/// [`replay`] of commands as they are, `angle_turn` at full resolution: the commands an
/// unrecorded session executed ([`SessionConfig::unrecorded_full_turning`]) or the commands the
/// injection evidence feeds (`tools/reference/m5b_inject.py`). A command outside the data's wire
/// (wire 24, or wire 25 for data loaded with sound) is refused by the simulation's own preflight.
pub fn replay_full_resolution(
    data: &GameData,
    screenblocks: u8,
    commands: &[TicCommand],
    host: &mut dyn Host,
) -> Option<Ending> {
    replay_inner(data, screenblocks, commands, host, None)
}

/// [`replay_full_resolution`] with the bespoke cheat scenario's adjustment of the status bar
/// inputs (decision S1; [`FrameSource::tic_adjusted`]), for its frame tests only.
pub fn replay_adjusted(
    data: &GameData,
    screenblocks: u8,
    commands: &[TicCommand],
    host: &mut dyn Host,
    adjust: StatusAdjustment<'_>,
) -> Option<Ending> {
    replay_inner(data, screenblocks, commands, host, Some(adjust))
}

fn replay_inner(
    data: &GameData,
    screenblocks: u8,
    commands: &[TicCommand],
    host: &mut dyn Host,
    adjust: Option<StatusAdjustment<'_>>,
) -> Option<Ending> {
    let refuse = |tic, witness| Some(Ending::Refused { tic, witness });
    let mut sim = data.start.clone();
    let mut frames = match FrameSource::start(data, screenblocks, &sim) {
        Ok(f) => f,
        Err(e) => return refuse(0, e),
    };
    if let Some(sink) = host.audio() {
        sink.session_start();
    }
    let (mut frame, none) = (Frame::new(), TicCommand::default());
    for &executed in commands {
        let tic = sim.tic();
        if let Err(e) = sim.advance([executed, none, none, none]) {
            return Some(ending_of(&sim, e, tic));
        }
        let presented = match adjust {
            None => frames.tic(data, &sim, &executed, &mut frame),
            Some(a) => frames.tic_adjusted(data, &sim, &executed, &mut frame, a),
        };
        let presented = match presented {
            Ok(p) => p,
            Err(e) => return refuse(tic, e),
        };
        report(host, &frames, &sim, &frame, tic, &[], executed, &presented);
    }
    None
}

/// Hands a presented tic's buffer to the host's sink and its report to [`Host::tic`].
#[allow(
    clippy::too_many_arguments,
    reason = "one call site per loop, all borrowed"
)]
fn report(
    host: &mut dyn Host,
    frames: &FrameSource,
    sim: &Simulation,
    frame: &Frame,
    tic: u64,
    events: &[Event],
    executed: TicCommand,
    presented: &Presented,
) {
    if let Some(buffer) = presented.sound.as_ref().and_then(|s| s.buffer.as_ref()) {
        if let Some(sink) = host.audio() {
            sink.buffer(buffer);
        }
    }
    let sound = match (&presented.sound, frames.sound()) {
        (Some(tic), Some(session)) => Some(SoundReport { tic, session }),
        _ => None,
    };
    host.tic(&TicReport {
        tic,
        events,
        executed,
        simulation: sim,
        frame,
        display: &presented.display,
        message: presented.message,
        rndindex: frames.rndindex(),
        sound,
    });
}

/// The ending of a refused `advance`: the exit, the reborn or a witness.
fn ending_of(sim: &Simulation, e: SimError, tic: u64) -> Ending {
    match e {
        SimError::LevelExited => Ending::Exit {
            secret: sim
                .snapshot()
                .fields()
                .any(|f| f.path() == "level.secret_exit" && f.value() == [1]),
        },
        SimError::PlayerReborn => Ending::Died,
        e => Ending::Refused {
            tic,
            witness: Witness::Simulation(e),
        },
    }
}

/// Why the game data could not be loaded.
#[derive(Debug)]
pub enum LoadError {
    /// The map or skill is outside the wire-24 domain (E1M1–E1M9, skill 2 or 3).
    Domain {
        /// The map number asked for.
        map: u8,
        /// The skill asked for.
        skill: u8,
    },
    /// WAD `index` (0 is the IWAD) is not a valid WAD.
    Wad {
        /// The file's position on the command line.
        index: usize,
        /// The parser's error.
        error: doom_wad::WadError,
    },
    /// The map is missing or malformed.
    Map(doom_map::MapError),
    /// A lump the renderer needs is missing or malformed.
    Resources(ResourceError),
    /// The simulation's registry refused the map.
    Registry(doom_sim::OriginalRegistryError),
    /// The simulation refused the map's set-up.
    Admission(SimError),
    /// [`GameData::load_with_sound`]: the sound data could not be loaded (`I_InitSound`'s
    /// fatal case, or a resource bound).
    SoundData(SoundLoadError),
    /// [`GameData::load_with_sound`]: the sound session refused its configuration (more textures
    /// than the layout detector admits).
    SoundSession(doom_sound::logic::SoundRefusal),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain { map, skill } => write!(
                f,
                "E{}M{map} at skill {skill} is not playable: the app plays E1M1 to E1M9 at skill 2 or 3",
                1
            ),
            Self::Wad { index, error } => {
                write!(f, "file {} is not a valid WAD: {error:?}", index + 1)
            }
            Self::Map(e) => write!(f, "the map could not be loaded: {e:?}"),
            Self::Resources(e) => write!(f, "the graphics could not be loaded: {e:?}"),
            Self::Registry(e) => write!(f, "the simulation does not admit this map: {e:?}"),
            Self::Admission(e) => write!(f, "the simulation does not admit this map: {e:?}"),
            Self::SoundData(e) => write!(f, "the sound data could not be loaded: {e:?}"),
            Self::SoundSession(e) => write!(f, "the sound session refused the data: {e:?}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Everything loaded once per run: the frame assets, with sound the sound data, and the
/// simulation at tic 0, which every session (and restart) starts from.
#[derive(Clone, Debug)]
pub struct GameData {
    assets: FrameAssets,
    sound: Option<SoundAssets>,
    start: Simulation,
    map: u8,
    skill: u8,
}

impl GameData {
    /// Loads `E1M<map>` at `skill` from `wads` (the IWAD first, then extra WADs; later files
    /// take precedence, as `W_AddFile` order does) in game `mode`, in M5b's configuration: wire
    /// 24, no sound, doom-render's sound-free, wipe-free contract (decision S15). Kept exactly for
    /// the frozen M5b tests, examples and tools; the playable build's default is
    /// [`Self::load_with_sound`] (decision T15).
    pub fn load(wads: &[&[u8]], mode: GameMode, map: u8, skill: u8) -> Result<Self, LoadError> {
        Self::load_inner(wads, mode, map, skill, false)
    }

    /// [`Self::load`] with sound (M5c, decision T15, the playable build's default): the
    /// simulation runs wire 25 (`original_player_episode1_v3_map`, canonical sound records), the
    /// sound data is loaded from the same lump directory (`I_InitSound`, i_sound.c:796-813), and
    /// every session runs a fresh process's sound session (volume 8, 3 channels, decisions T9,
    /// T15) with the display under `RandomContract::WithSound`. The `--no-sound` and `--volume`
    /// of decision T9 need nothing from here: the session always runs (so the face stays the
    /// genuine one) and the gain or the silence belongs to the host's [`AudioSink`].
    pub fn load_with_sound(
        wads: &[&[u8]],
        mode: GameMode,
        map: u8,
        skill: u8,
    ) -> Result<Self, LoadError> {
        Self::load_inner(wads, mode, map, skill, true)
    }

    fn load_inner(
        wads: &[&[u8]],
        mode: GameMode,
        map: u8,
        skill: u8,
        with_sound: bool,
    ) -> Result<Self, LoadError> {
        let skill_value = match skill {
            2 => Skill::Medium,
            3 => Skill::Hard,
            _ => return Err(LoadError::Domain { map, skill }),
        };
        if !(1..=9).contains(&map) {
            return Err(LoadError::Domain { map, skill });
        }
        let parsed = wads
            .iter()
            .enumerate()
            .map(|(index, bytes)| {
                doom_wad::Wad::parse(bytes, doom_wad::Limits::default())
                    .map_err(|error| LoadError::Wad { index, error })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut dir = LumpDirectory::new();
        for wad in &parsed {
            dir.add_wad(wad).map_err(LoadError::Resources)?;
        }
        let marker = format!("E1M{map}");
        let map_data = doom_map::Map::load(
            &doom_wad::WadSet::new(&parsed),
            &marker,
            doom_map::MapLimits::default(),
        )
        .map_err(LoadError::Map)?;
        let set = if with_sound {
            Some(SoundSet::load(&dir, SoundCaps::default()).map_err(LoadError::SoundData)?)
        } else {
            None
        };
        let res = Resources::load(dir, mode).map_err(LoadError::Resources)?;
        // numtextures (r_data.c:480), for the layout detector of decision T12.
        let textures = u32::try_from(res.textures.len()).unwrap_or(u32::MAX);
        let assets = FrameAssets::load(res, &map_data).map_err(LoadError::Resources)?;
        let sound = match set {
            Some(set) => Some(
                SoundAssets::new(Arc::new(set), SoundConfig::fresh(i32::from(map), textures))
                    .map_err(LoadError::SoundSession)?,
            ),
            None => None,
        };
        // The M4b/M5a run set-up (player_level_run --policy episode1v2), under wire 25 with sound.
        let mut start = if with_sound {
            Simulation::original_player_episode1_v3_map(
                map_data,
                map,
                skill_value,
                MapContextOptions::default(),
            )
        } else {
            Simulation::original_player_episode1_v2_map(
                map_data,
                map,
                skill_value,
                MapContextOptions::default(),
            )
        }
        .map_err(LoadError::Registry)?;
        start.spawn_map_things().map_err(LoadError::Admission)?;
        start
            .initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
            .map_err(LoadError::Admission)?;
        start.spawn_level_specials().map_err(LoadError::Admission)?;
        Ok(Self {
            assets,
            sound,
            start,
            map,
            skill,
        })
    }

    /// The sound data, when loaded with [`Self::load_with_sound`].
    pub fn sound(&self) -> Option<&SoundAssets> {
        self.sound.as_ref()
    }

    /// The map number.
    pub fn map(&self) -> u8 {
        self.map
    }

    /// The skill byte.
    pub fn skill(&self) -> u8 {
        self.skill
    }

    /// The simulation at tic 0.
    pub fn start(&self) -> &Simulation {
        &self.start
    }

    /// The frame assets.
    pub fn assets(&self) -> &FrameAssets {
        &self.assets
    }
}

/// The session options, fixed for the run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionConfig {
    /// Screenblocks, 10 (the default) or 11 (decision S14).
    pub screenblocks: u8,
    /// The bindings preset (decision S7).
    pub preset: Preset,
    /// The session length bound in tics: [`TIC_CAP`] in the app; smaller only in tests.
    pub tic_cap: usize,
    /// Opt-in, never the default (decision S3 (ii)): the simulation executes each built command
    /// at full resolution, as a live game that does not record does (g_game.c:664-669), and
    /// nothing is recorded: no demo, no file, so the session cannot be replayed. Off: every
    /// session is recorded and executes the demo's re-read command (S3 (i)).
    pub unrecorded_full_turning: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            screenblocks: 10,
            preset: Preset::Vanilla,
            tic_cap: TIC_CAP,
            unrecorded_full_turning: false,
        }
    }
}

/// What refused a tic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Witness {
    /// The input path refused an event.
    Input(InputError),
    /// The recorder refused the command.
    Record(RecordError),
    /// The simulation refused the tic.
    Simulation(SimError),
    /// The frame source refused the tic's frame.
    Frame(FrameRefusal),
    /// The sound session refused the tic (M5c, decision T11: where the C aborts or reads memory
    /// it does not own, at the exact start; and the T12 slot-15 release).
    Sound(SessionRefusal),
}

impl fmt::Display for Witness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(e) => write!(f, "input: {e}"),
            Self::Record(e) => write!(f, "recorder: {e}"),
            Self::Simulation(e) => write!(f, "{e}"),
            Self::Frame(e) => write!(f, "{e}"),
            Self::Sound(e) => write!(f, "sound: {e}"),
        }
    }
}

/// How a session ended.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ending {
    /// The level was exited (decision S10); on E1M8 this is the episode's end.
    Exit {
        /// Whether it was the secret exit.
        secret: bool,
    },
    /// The dead player pressed use (the reborn, decision S10).
    Died,
    /// A tic was refused (decision S9). A tic the simulation refused is not in the demo; a tic
    /// whose frame was refused after the simulation committed it is, and the demo replays to
    /// the same refusal.
    Refused {
        /// The refused tic: the simulation's tic count when it was attempted.
        tic: u64,
        /// What refused it.
        witness: Witness,
    },
    /// The session reached [`SessionConfig::tic_cap`] (decision S21).
    TicCap,
}

/// What the app is doing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    /// Tics run.
    Playing,
    /// The app's pause (decision S12): no tics run.
    Paused,
    /// The session ended; its screen is shown.
    Ended(Ending),
}

/// The state of the session's demo file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveStatus {
    /// Not written yet.
    NotSaved,
    /// Written with this many tics.
    Saved(usize),
    /// The last write failed.
    Failed(String),
}

/// What one [`App::update`] did.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Update {
    /// Tics run.
    pub ran: u64,
    /// Due tics dropped beyond [`MAX_TICS_PER_UPDATE`] (or after the session ended).
    pub dropped: u64,
}

/// The tic clock: 35 tics a second from a monotonic microsecond count, computed from an
/// origin (no accumulated rounding), re-based after a pause.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct TicClock {
    origin: Option<(u64, u64)>,
    consumed: u64,
}

impl TicClock {
    fn due(&mut self, now_us: u64) -> u64 {
        let (at, tic) = *self.origin.get_or_insert((now_us, self.consumed));
        let elapsed = u128::from(now_us.saturating_sub(at));
        let now_tic = tic + u64::try_from(elapsed * u128::from(TICRATE) / 1_000_000).unwrap_or(0);
        let due = now_tic.saturating_sub(self.consumed);
        self.consumed = self.consumed.max(now_tic);
        due
    }

    fn next_at(&self) -> Option<u64> {
        let (at, tic) = self.origin?;
        let n = u128::from(self.consumed + 1 - tic);
        let us = (n * 1_000_000).div_ceil(u128::from(TICRATE));
        Some(at.saturating_add(u64::try_from(us).unwrap_or(u64::MAX)))
    }
}

/// The playable app: one session at a time.
#[derive(Debug)]
pub struct App {
    data: GameData,
    cfg: SessionConfig,
    sim: Simulation,
    input: Input,
    layer: EventLayer,
    frames: Option<FrameSource>,
    frame: Frame,
    palette: u8,
    drawn: bool,
    recorder: Recorder,
    name: String,
    started: Timestamp,
    claimed: bool,
    save: SaveStatus,
    /// While the retries of a failed write keep failing (a host that appends): the recorder
    /// length from which the next retry is made ([`FAILED_WRITE_RETRY_TICS`]); `None` retries at
    /// the next tic.
    retry_from: Option<usize>,
    mode: Mode,
    clock: TicClock,
    events: Vec<Event>,
    screen: Option<Screen>,
    quit: bool,
    audio_notes: Vec<AudioNote>,
    /// Earlier sessions of this run (before a restart) that the layout detector of decision T12
    /// labelled layout-dependent.
    layout_earlier: u64,
}

/// The one plain line that says a run held a session labelled layout-dependent by the detector of
/// decision T12 ("counted separately in every report"): `current` is the current session's
/// detector, `earlier` the number of earlier sessions of the run that were labelled. `None` when
/// no session was.
pub fn layout_dependent_line(current: Option<LayoutDetector>, earlier: u64) -> Option<String> {
    let earlier_text = match earlier {
        0 => String::new(),
        1 => "; 1 earlier session of this run was labelled too".to_string(),
        n => format!("; {n} earlier sessions of this run were labelled too"),
    };
    match current.and_then(|d| d.layout_dependent_from().map(|t| (t, d.firings()))) {
        Some((tic, firings)) => Some(format!(
            "layout-dependent session from tic {tic} ({firings} detector {}): a 32-bit build \
             could have played a switch-release sound differently; this one used the 64-bit \
             reading{earlier_text}",
            if firings == 1 { "firing" } else { "firings" }
        )),
        None if earlier > 0 => Some(format!(
            "this session is not layout-dependent{earlier_text}"
        )),
        None => None,
    }
}

/// A notification for the host's [`AudioSink`], kept until the next call that has the host
/// ([`App::focus_lost`] has none).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AudioNote {
    Pause,
    Resume,
    SessionStart,
}

impl App {
    /// Starts the first session.
    pub fn new(data: GameData, cfg: SessionConfig, host: &mut dyn Host) -> Self {
        let controls = cfg.preset.controls();
        let mut app = Self {
            sim: data.start.clone(),
            input: Input::new(controls),
            layer: EventLayer::new(&controls, cfg.preset.layer_options()),
            frames: None,
            frame: Frame::new(),
            palette: 0,
            drawn: false,
            recorder: Recorder::new(data.map, data.skill, cfg.tic_cap),
            name: String::new(),
            started: Timestamp::default(),
            claimed: false,
            save: SaveStatus::NotSaved,
            retry_from: None,
            mode: Mode::Playing,
            clock: TicClock::default(),
            events: Vec::new(),
            screen: None,
            quit: false,
            audio_notes: Vec::new(),
            layout_earlier: 0,
            data,
            cfg,
        };
        app.begin(host);
        app
    }

    /// A fresh session (decision S11): the tic-0 simulation, fresh input, presentation and
    /// recorder, a new file name, the clock re-based.
    fn begin(&mut self, host: &mut dyn Host) {
        if self.mode == Mode::Paused {
            self.audio_notes.push(AudioNote::Resume);
        }
        self.audio_notes.push(AudioNote::SessionStart);
        let labelled = self
            .sound()
            .is_some_and(|s| s.detector().layout_dependent_from().is_some());
        self.layout_earlier += u64::from(labelled);
        let controls = self.cfg.preset.controls();
        self.sim = self.data.start.clone();
        self.input = Input::new(controls);
        self.layer = EventLayer::new(&controls, self.cfg.preset.layer_options());
        self.frame = Frame::new();
        self.palette = 0;
        self.drawn = false;
        self.recorder = Recorder::new(self.data.map, self.data.skill, self.cfg.tic_cap);
        self.save = SaveStatus::NotSaved;
        self.retry_from = None;
        self.clock = TicClock::default();
        self.screen = None;
        self.mode = Mode::Playing;
        self.name = String::new();
        self.claimed = false;
        self.started = host.timestamp();
        // the file is created at once (an empty demo), so an unwritable directory shows at
        // the first pause, not at the session's end
        self.save(host);
        match FrameSource::start(&self.data, self.cfg.screenblocks, &self.sim) {
            Ok(f) => self.frames = Some(f),
            Err(witness) => {
                self.frames = None;
                self.end(Ending::Refused { tic: 0, witness }, host);
            }
        }
        self.flush_audio(host);
    }

    /// Hands the pending pause, resume and session-start notifications to the host's sink.
    fn flush_audio(&mut self, host: &mut dyn Host) {
        if self.audio_notes.is_empty() {
            return;
        }
        let notes = std::mem::take(&mut self.audio_notes);
        if let Some(sink) = host.audio() {
            for n in &notes {
                match n {
                    AudioNote::Pause => sink.pause(),
                    AudioNote::Resume => sink.resume(),
                    AudioNote::SessionStart => sink.session_start(),
                }
            }
        }
    }

    /// The sound session, when the data was loaded with sound (M5c): its channel table, mixer,
    /// schedule and the layout detector of decision T12.
    pub fn sound(&self) -> Option<&Sound> {
        self.frames.as_ref().and_then(FrameSource::sound)
    }

    /// The run's layout-dependence line ([`layout_dependent_line`]) for the current session and
    /// the earlier sessions of this run, or `None` when no session was labelled. The window prints
    /// it on standard error at exit; nothing is drawn.
    pub fn layout_dependent_line(&self) -> Option<String> {
        layout_dependent_line(self.sound().map(Sound::detector), self.layout_earlier)
    }

    /// Creates the session's file under the first free name made from the session's start
    /// time: the base name, then `-2`, `-3` and so on up to [`MAX_NAME_SUFFIX`]. No existing
    /// file is ever replaced; when every name is taken, nothing is written and the save fails
    /// visibly.
    fn claim(&mut self, bytes: &[u8], host: &mut dyn Host) -> Result<(), String> {
        let base = session_name(self.data.map, self.data.skill, self.started);
        let stem = base.trim_end_matches(".lmp");
        for n in 1..=MAX_NAME_SUFFIX {
            let name = match n {
                1 => base.clone(),
                n => format!("{stem}-{n}.lmp"),
            };
            if host.claim(&name, bytes)? {
                self.name = name;
                self.claimed = true;
                return Ok(());
            }
        }
        Err(format!(
            "every name from {base} to {stem}-{MAX_NAME_SUFFIX}.lmp exists"
        ))
    }

    /// The current mode.
    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    /// The simulation's tic count (committed tics this session).
    pub fn tic(&self) -> u64 {
        self.sim.tic()
    }

    /// The simulation.
    pub fn simulation(&self) -> &Simulation {
        &self.sim
    }

    /// The recorder.
    pub fn recorder(&self) -> &Recorder {
        &self.recorder
    }

    /// Whether sessions are recorded (the default); `false` with
    /// [`SessionConfig::unrecorded_full_turning`].
    pub fn recorded(&self) -> bool {
        !self.cfg.unrecorded_full_turning
    }

    /// The tics this session has run: the recorded tics, or the simulation's tic count when
    /// nothing is recorded.
    fn played(&self) -> u64 {
        if self.recorded() {
            self.recorder.len() as u64
        } else {
            self.sim.tic()
        }
    }

    /// The session's file name: empty when nothing is recorded, or while no write has succeeded
    /// (the file is created when the session begins).
    pub fn session_name(&self) -> &str {
        &self.name
    }

    /// The state of the session's file.
    pub fn save_status(&self) -> &SaveStatus {
        &self.save
    }

    /// The host's later word on the session file (M5d): a host that answers [`Host::save`] and
    /// [`Host::append`] before it has made the write (doom-play hands every write to a writer
    /// thread) tells the app here, at a moment no tic runs, what became of it. `Ok(())`: the
    /// file now holds every tic recorded so far ([`SaveStatus::Saved`]). `Err(e)`: its last write
    /// failed ([`SaveStatus::Failed`]), so the pause overlay and the end screen say so, and
    /// [`App::restart`] and [`App::quit`] rewrite the file whole, as after any failed save (so
    /// does the next tic if play resumes, and then at most once every
    /// [`FAILED_WRITE_RETRY_TICS`] tics while the retries keep failing). An end screen being
    /// shown is drawn again from the new status. Nothing changes for a session whose file was
    /// never created, which includes every unrecorded session.
    pub fn settle_save(&mut self, outcome: Result<(), String>) {
        if !self.claimed {
            return;
        }
        self.retry_from = None;
        self.save = match outcome {
            Ok(()) => SaveStatus::Saved(self.recorder.len()),
            Err(e) => SaveStatus::Failed(e),
        };
        let screen = match &self.mode {
            Mode::Ended(ending) => Some(self.end_screen(ending)),
            _ => None,
        };
        if screen.is_some() {
            self.screen = screen;
        }
    }

    /// Whether the player asked to quit (the save was attempted; see [`App::save_status`]). No
    /// tic runs after it.
    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    /// Whether the platform should capture the mouse: only while playing.
    pub fn wants_mouse_capture(&self) -> bool {
        self.mode == Mode::Playing
    }

    /// The last drawn game frame (64,000 indices) and its palette number; `None` before the
    /// first tic.
    pub fn frame(&self) -> Option<(&[u8], u8)> {
        self.drawn.then(|| (self.frame.pixels(), self.palette))
    }

    /// The monotonic time at which the next tic falls due, once the clock is running.
    pub fn next_tic_at(&self) -> Option<u64> {
        (self.mode == Mode::Playing)
            .then(|| self.clock.next_at())
            .flatten()
    }

    /// A DOOM key code went down or up. While playing, Esc and Pause pause and every other
    /// key goes to the event layer (which withholds the keys of decision S5); while paused,
    /// Esc or Pause resumes, `r` restarts and `q` quits; on an end screen `r` restarts and
    /// `q` or Esc quits.
    pub fn key(&mut self, code: u8, down: bool, host: &mut dyn Host) {
        let pause_key = code == keys::KEY_ESCAPE || code == keys::KEY_PAUSE;
        match self.mode {
            Mode::Playing if down && pause_key => self.pause(),
            Mode::Playing => {
                self.layer.key(code, down);
            }
            Mode::Paused if down && pause_key => {
                self.mode = Mode::Playing;
                self.audio_notes.push(AudioNote::Resume);
            }
            Mode::Paused | Mode::Ended(_) if down && code == b'r' => self.restart(host),
            Mode::Paused if down && code == b'q' => self.quit(host),
            Mode::Ended(_) if down && (code == b'q' || code == keys::KEY_ESCAPE) => self.quit(host),
            _ => {}
        }
        self.flush_audio(host);
    }

    /// Relative mouse motion in the platform's raw device units (doom-play passes miniquad's raw
    /// mouse motion, which is not in pixels), `dy` positive downward; ignored unless playing.
    pub fn mouse_motion(&mut self, dx: i32, dy: i32) {
        if self.mode == Mode::Playing {
            self.layer.mouse_motion(dx, dy);
        }
    }

    /// A mouse button went down or up; ignored unless playing.
    pub fn mouse_button(&mut self, button: MouseButton, down: bool) {
        if self.mode == Mode::Playing {
            self.layer.mouse_button(button, down);
        }
    }

    /// The platform reports that the window was minimised or lost the input focus: pauses a
    /// playing session, which releases every held key. doom-play calls it on miniquad's
    /// `window_minimized_event`, which that library's source sends on focus loss as well as on
    /// minimising (X11 FocusOut, Wayland keyboard leave, Windows `WM_ACTIVATE` inactive, macOS
    /// resign-key and window move).
    pub fn focus_lost(&mut self) {
        if self.mode == Mode::Playing {
            self.pause();
        }
    }

    /// The app's pause: no tics run, held keys are released through the event layer (posted
    /// before the first tic after the pause) and the clock is re-based on resume. The host's
    /// sink hears [`AudioSink::pause`] at the next call that has the host.
    fn pause(&mut self) {
        self.audio_notes.push(AudioNote::Pause);
        self.mode = Mode::Paused;
        self.layer.release_all();
        self.clock.origin = None;
    }

    /// Restarts: saves the current session unless its file already holds every tic (a failed
    /// save at the session's end is retried here; with a host that appends, the file is
    /// normally current already), then begins a new one.
    pub fn restart(&mut self, host: &mut dyn Host) {
        self.save_if_behind(host);
        self.begin(host);
    }

    /// Quits: saves the session unless its file already holds every tic (a failed save at the
    /// session's end is retried here). No tic runs afterwards.
    pub fn quit(&mut self, host: &mut dyn Host) {
        self.save_if_behind(host);
        self.quit = true;
    }

    fn save_if_behind(&mut self, host: &mut dyn Host) {
        if self.save != SaveStatus::Saved(self.recorder.len()) {
            self.save(host);
        }
    }

    /// Runs the tics due at monotonic time `now_us`: at most [`MAX_TICS_PER_UPDATE`], the rest
    /// dropped. The first call only starts the clock; the first tic falls due 1/35 s later.
    /// After [`App::quit`] no tic runs.
    pub fn update(&mut self, now_us: u64, host: &mut dyn Host) -> Update {
        self.flush_audio(host);
        if self.mode != Mode::Playing || self.quit {
            return Update::default();
        }
        let due = self.clock.due(now_us);
        let mut ran = 0;
        while ran < due.min(MAX_TICS_PER_UPDATE) && self.mode == Mode::Playing {
            self.run_tic(host);
            ran += 1;
        }
        Update {
            ran,
            dropped: due - ran,
        }
    }

    fn run_tic(&mut self, host: &mut dyn Host) {
        if self.played() >= self.cfg.tic_cap as u64 {
            return self.end(Ending::TicCap, host);
        }
        let tic = self.sim.tic();
        let refuse = |witness| Ending::Refused { tic, witness };
        let mut events = std::mem::take(&mut self.events);
        events.clear();
        self.layer.drain(&mut events);
        for e in &events {
            self.input.post(*e);
        }
        self.events = events;
        let recorded: RecordedTic = match self.input.tic() {
            Ok(r) => r,
            Err(e) => return self.end(refuse(Witness::Input(e)), host),
        };
        let executed = if self.recorded() {
            match self.recorder.record(recorded) {
                Ok(c) => c,
                Err(e) => return self.end(refuse(Witness::Record(e)), host),
            }
        } else if wire24_admits(&recorded.built()) {
            recorded.built()
        } else {
            let witness = Witness::Record(RecordError::NotAdmitted);
            return self.end(refuse(witness), host);
        };
        let none = TicCommand::default();
        if let Err(e) = self.sim.advance([executed, none, none, none]) {
            self.recorder.drop_last();
            return self.end(ending_of(&self.sim, e, tic), host);
        }
        let Some(frames) = self.frames.as_mut() else {
            return;
        };
        match frames.tic(&self.data, &self.sim, &executed, &mut self.frame) {
            Ok(presented) => {
                self.palette = presented.display.palette;
                self.drawn = true;
                report(
                    host,
                    frames,
                    &self.sim,
                    &self.frame,
                    tic,
                    &self.events,
                    executed,
                    &presented,
                );
            }
            // The simulation committed the tic, so its command stays in the demo (only a
            // simulation refusal drops it): the demo replays to this same frame or sound
            // refusal. With sound, the sound session and the display are as before the tic.
            Err(witness) => return self.end(refuse(witness), host),
        }
        if self.recorder.len() % SAVE_INTERVAL == 0 {
            self.save(host);
        } else if self.recorded() && host.appends() {
            self.append(host);
        }
    }

    /// Keeps the file current after a committed, recorded tic (a host that [`Host::appends`]):
    /// when the file holds exactly the earlier tics, the tic's four bytes and a new marker go at
    /// the old marker's offset in one write ([`Host::append`]); otherwise (an earlier write
    /// failed, or the host declines this offset) the file is rewritten whole, which repairs it.
    /// The forward byte of a recorded tic is never 0x80 (wire 24 refuses forward −128,
    /// [`wire24_admits`]), so `G_ReadDemoTiccmd`, which tests only a tic's first byte for the
    /// marker (g_game.c:1493), stops at the new marker and nowhere before it. After a failed
    /// write, the whole rewrite is retried at the next tic and then at most once every
    /// [`FAILED_WRITE_RETRY_TICS`] tics while it keeps failing ([`App::retry`]).
    fn append(&mut self, host: &mut dyn Host) {
        let n = self.recorder.len();
        if matches!(self.save, SaveStatus::Failed(_)) {
            return self.retry(n, host);
        }
        let current = self.claimed && n > 0 && self.save == SaveStatus::Saved(n - 1);
        let (at, bytes) = match self.recorder.last_tic_tail() {
            Ok(Some(tail)) if current => tail,
            // the whole rewrite, which also reports a refusal of the demo writer
            _ => return self.save(host),
        };
        self.save = match host.append(&self.name, at, &bytes) {
            Ok(true) => SaveStatus::Saved(n),
            Ok(false) => return self.save(host),
            Err(e) => SaveStatus::Failed(e),
        };
    }

    /// The per-tic retry after a failed write (a host that appends; `n` the recorded tics): a
    /// whole rewrite, unless an earlier retry of this failure failed less than
    /// [`FAILED_WRITE_RETRY_TICS`] tics ago (then nothing is written this tic, and the status
    /// stays failed).
    fn retry(&mut self, n: usize, host: &mut dyn Host) {
        if self.retry_from.is_some_and(|from| n < from) {
            return;
        }
        self.save(host);
        if matches!(self.save, SaveStatus::Failed(_)) {
            self.retry_from = Some(n + FAILED_WRITE_RETRY_TICS);
        }
    }

    /// Writes the demo: the first successful write creates the file ([`App::claim`]), later
    /// ones rewrite it.
    fn save(&mut self, host: &mut dyn Host) {
        if !self.recorded() {
            return;
        }
        let written = match self.recorder.bytes() {
            Ok(bytes) if self.claimed => host.save(&self.name, &bytes),
            Ok(bytes) => self.claim(&bytes, host),
            Err(e) => Err(format!("the demo writer refused: {e:?}")),
        };
        self.save = match written {
            Ok(()) => {
                self.retry_from = None;
                SaveStatus::Saved(self.recorder.len())
            }
            Err(e) => SaveStatus::Failed(e),
        };
    }

    fn end(&mut self, ending: Ending, host: &mut dyn Host) {
        self.save(host);
        self.screen = Some(self.end_screen(&ending));
        self.mode = Mode::Ended(ending);
    }

    fn end_screen(&self, ending: &Ending) -> Screen {
        use screens::{GREY, RED, WHITE, YELLOW};
        let mut lines: Vec<(String, u8)> = Vec::new();
        match ending {
            Ending::Exit { secret } => {
                let head = match (self.data.map, secret) {
                    (8, _) => "EPISODE 1 COMPLETE",
                    (_, true) => "LEVEL COMPLETE: SECRET EXIT",
                    _ => "LEVEL COMPLETE",
                };
                lines.push((head.into(), YELLOW));
            }
            Ending::Died => lines.push(("YOU DIED".into(), YELLOW)),
            Ending::TicCap => lines.push(("SESSION LENGTH CAP REACHED".into(), YELLOW)),
            Ending::Refused { tic, witness } => {
                lines.push(("TIC REFUSED".into(), RED));
                lines.push((format!("TIC {tic}, WITNESS:"), WHITE));
                for l in screens::wrap(&witness.to_string()).into_iter().take(6) {
                    lines.push((l, WHITE));
                }
            }
        }
        lines.push((String::new(), WHITE));
        lines.push((
            format!(
                "E1M{} SKILL {}  {} TICS",
                self.data.map,
                self.data.skill,
                self.played()
            ),
            WHITE,
        ));
        let saved = match &self.save {
            _ if !self.recorded() => UNRECORDED_LABEL.into(),
            SaveStatus::Saved(_) => format!("DEMO SAVED: {}", self.name),
            SaveStatus::Failed(e) => format!("DEMO NOT SAVED: {e}"),
            SaveStatus::NotSaved => "DEMO NOT SAVED".into(),
        };
        for l in screens::wrap(&saved).into_iter().take(3) {
            lines.push((l, WHITE));
        }
        lines.push((String::new(), WHITE));
        lines.push(("R: RESTART    Q: QUIT".into(), GREY));
        screens::page(&lines)
    }

    /// The picture to present, as 320×200 RGBA in `out` ([`PICTURE_BYTES`] bytes; a shorter
    /// buffer receives the leading whole pixels and a longer one keeps its tail): the game
    /// frame through its PLAYPAL palette and gamma table `gamma` (0–4, decision S13; larger
    /// values use 4), with the pause overlay composed on a half-bright picture while paused
    /// (naming a failed save of the demo), or the end screen in the app's palette. While an
    /// unrecorded session plays, the pictures of its first [`UNRECORDED_LABEL_TICS`] tics carry
    /// [`label_band`] (the game picture at full brightness around it). The game frame is never
    /// changed.
    pub fn picture(&self, gamma: usize, out: &mut [u8]) {
        let ramp = &GAMMA[gamma.min(GAMMA.len() - 1)];
        let app = |c: u8| APP_PALETTE[usize::from(c)];
        if let (Mode::Ended(_), Some(screen)) = (&self.mode, &self.screen) {
            for (px, &c) in out.chunks_exact_mut(4).zip(screen.pixels()) {
                let [r, g, b] = app(c);
                px.copy_from_slice(&[r, g, b, 255]);
            }
            return;
        }
        let palette = self.frame().and_then(|(_, n)| self.data.assets.palette(n));
        let paused = self.mode == Mode::Paused;
        let overlay = match (&self.mode, self.recorded(), &self.save) {
            (Mode::Paused, false, _) => Some(labelled_pause_overlay(UNRECORDED_LABEL)),
            (Mode::Paused, true, SaveStatus::Failed(_)) => {
                Some(labelled_pause_overlay(SAVE_FAILED_LABEL))
            }
            (Mode::Paused, true, _) => Some(pause_overlay()),
            (Mode::Playing, false, _) if self.sim.tic() <= UNRECORDED_LABEL_TICS => {
                Some(label_band(UNRECORDED_LABEL))
            }
            _ => None,
        };
        let pixels = out.chunks_exact_mut(4).take(SCREENWIDTH * SCREENHEIGHT);
        for (i, px) in pixels.enumerate() {
            let rgb = match (&overlay, palette) {
                (Some(o), _) if o.pixels()[i] != CLEAR => app(o.pixels()[i]),
                (_, Some(pal)) => {
                    let at = usize::from(self.frame.pixels()[i]) * 3;
                    let c = [0, 1, 2].map(|k| ramp[usize::from(pal[at + k])]);
                    if paused { c.map(|v| v / 2) } else { c }
                }
                (_, None) => [0, 0, 0],
            };
            px.copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use screens::YELLOW;
    use std::path::Path;

    struct NullHost;

    impl Host for NullHost {
        fn timestamp(&mut self) -> Timestamp {
            Timestamp::default()
        }
        fn exists(&mut self, _name: &str) -> bool {
            false
        }
        fn save(&mut self, _name: &str, _bytes: &[u8]) -> Result<(), String> {
            Ok(())
        }
    }

    fn app_on(map_wad: &str, map: u8) -> App {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
        let wad = std::fs::read(root.join(map_wad)).expect("map WAD");
        let gfx = std::fs::read(root.join("m5b_gfx.wad")).expect("gfx WAD");
        let data = GameData::load(&[&wad, &gfx], GameMode::Registered, map, 2).expect("data");
        App::new(data, SessionConfig::default(), &mut NullHost)
    }

    /// The number of heading-coloured pixels of `text` drawn alone.
    fn yellow_of(text: &str) -> usize {
        let mut s = Screen::filled(0);
        s.text(0, 0, text, YELLOW);
        s.pixels().iter().filter(|p| **p == YELLOW).count()
    }

    /// Pins `App::end_screen`'s heading for a secret exit and for E1M8 (a private function: the
    /// public API reaches only the exits the synthetic maps contain, and none of them is a secret
    /// exit): the heading is the only yellow text of an exit screen, so the screen's yellow
    /// pixel count is the heading's. E1M8 says "EPISODE 1 COMPLETE" for either exit; any other
    /// map says "LEVEL COMPLETE", with ": SECRET EXIT" after a secret exit.
    #[test]
    fn the_exit_headings_follow_the_map_and_the_secret_flag() {
        let level = yellow_of("LEVEL COMPLETE");
        let secret = yellow_of("LEVEL COMPLETE: SECRET EXIT");
        let episode = yellow_of("EPISODE 1 COMPLETE");
        assert!(level != secret && level != episode && secret != episode);
        let yellow = |app: &App, secret: bool| {
            let s = app.end_screen(&Ending::Exit { secret });
            s.pixels().iter().filter(|p| **p == YELLOW).count()
        };
        let one = app_on("m5a_wads/amp.wad", 1);
        assert_eq!(yellow(&one, false), level);
        assert_eq!(yellow(&one, true), secret);
        let eight = app_on("m4b_wads/boss8.wad", 8);
        assert_eq!(yellow(&eight, false), episode);
        assert_eq!(yellow(&eight, true), episode);
    }

    /// A session whose layout detector (decision T12) has fired: a fresh process over the app's
    /// own sound data (`SoundConfig::fresh(1, 1)`), one empty tic, then at tic 1 the switch
    /// release start of p_spec.c:1151 (`S_StartSound(&buttonlist[i].soundorg, sfx_swtchn)`,
    /// sfx 23, origin button 0) with the declared 64-bit reading (34, 0) placed 34 raw units
    /// inside the clipping distance from the listener, where the 32-bit reading (0, 34) would be
    /// out of range: the same construction as tests/m5c_app.rs
    /// `a_layout_dependent_session_gives_one_plain_line`, which pins the detector's result
    /// (labelled from tic 1, one firing). This helper asserts that result again before use.
    fn fired_sound(app: &App) -> Sound {
        use doom_sound::logic::{Listener, OriginPositions, S_CLIPPING_DIST, SoundOrigin};
        use doom_sound::logic::{SoundPlacement, SoundRecord};
        struct Quiet;
        impl OriginPositions for Quiet {
            fn actor_xy(&self, _: doom_sim::ActorId) -> Option<[i32; 2]> {
                None
            }
            fn sector_xy(&self, _: u16) -> Option<[i32; 2]> {
                None
            }
            fn button_xy(&self, _: u8) -> Option<[i32; 2]> {
                Some([0, 0])
            }
        }
        let set = app.data.sound().expect("sound data").set().clone();
        let edge = S_CLIPPING_DIST - (130 << 16);
        let listener = Listener {
            actor: doom_sim::ActorId::from_parts(0, 0),
            xy: [edge + 34, 0],
            angle: 0,
        };
        let release = SoundRecord::Start {
            sfx: 23,
            origin: SoundOrigin::Button(0),
            at: Some(SoundPlacement {
                origin: [34, 0],
                listener: listener.xy,
                listener_angle: 0,
            }),
        };
        let mut s = Sound::new(set, SoundConfig::fresh(1, 1)).expect("session");
        s.tic(&[], 0, &listener, &Quiet).expect("tic 0");
        s.tic(&[release], 0, &listener, &Quiet).expect("tic 1");
        let d = s.detector();
        assert_eq!((d.layout_dependent_from(), d.firings()), (Some(1), 1));
        s
    }

    /// `App::layout_dependent_line` and the run's count of earlier labelled sessions, through a
    /// real `App` (decision T12: a layout-dependent session is "counted separately in every
    /// report"). The app is built on the M5a AMP room with `m5b_gfx.wad` and the synthetic sound
    /// lumps `m5c_sfx.wad`. The committed scenarios never fire the detector, so the test makes
    /// the next session labelled by replacing the game data's fresh sound session
    /// (`SoundAssets::fresh`, which `App::begin` clones through `FrameSource::start`) with the
    /// fired session of [`fired_sound`], and unlabelled again by putting the original back, each
    /// followed by `App::restart`. No tic runs. The expected lines are the documented texts of
    /// the pure [`layout_dependent_line`] (pinned by tests/m5c_app.rs); the expected counts
    /// follow from the documentation of `App::layout_earlier` and `App::begin`: one more for each
    /// labelled session a restart ends, no change otherwise.
    #[test]
    fn the_app_counts_layout_dependent_sessions_across_restarts() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
        let files: Vec<Vec<u8>> = ["m5a_wads/amp.wad", "m5b_gfx.wad", "m5c_sfx.wad"]
            .iter()
            .map(|w| std::fs::read(root.join(w)).expect("WAD"))
            .collect();
        let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
        let data = GameData::load_with_sound(&refs, GameMode::Registered, 1, 2).expect("data");
        let mut host = NullHost;
        let mut app = App::new(data, SessionConfig::default(), &mut host);
        // an unlabelled run prints no line, before and after a restart
        assert_eq!(app.layout_dependent_line(), None);
        app.restart(&mut host);
        assert_eq!(app.layout_dependent_line(), None);
        let fresh = app.data.sound.as_ref().expect("sound").fresh.clone();
        let fired = fired_sound(&app);
        let mut install = |app: &mut App, sound: &Sound| {
            app.data.sound.as_mut().expect("sound").fresh = sound.clone();
            app.restart(&mut host);
        };
        let labelled = "layout-dependent session from tic 1 (1 detector firing): a 32-bit build \
                        could have played a switch-release sound differently; this one used the \
                        64-bit reading";
        // a labelled current session, no earlier one
        install(&mut app, &fired);
        assert_eq!(app.layout_dependent_line().as_deref(), Some(labelled));
        // the restart ends a labelled session: one earlier, the new session not labelled
        install(&mut app, &fresh);
        assert_eq!(
            app.layout_dependent_line().as_deref(),
            Some(
                "this session is not layout-dependent; 1 earlier session of this run was \
                 labelled too"
            )
        );
        // the restart ends an unlabelled session: the count stays one
        install(&mut app, &fired);
        let one = format!("{labelled}; 1 earlier session of this run was labelled too");
        assert_eq!(app.layout_dependent_line(), Some(one));
        // the restart ends the second labelled session: two
        install(&mut app, &fresh);
        assert_eq!(
            app.layout_dependent_line().as_deref(),
            Some(
                "this session is not layout-dependent; 2 earlier sessions of this run were \
                 labelled too"
            )
        );
    }
}
