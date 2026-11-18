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

mod frames;
mod gamma;
mod recorder;
mod screens;

pub use frames::{FrameAssets, FrameRefusal, PALETTE_BYTES};
pub use gamma::GAMMA;
pub use recorder::{
    DemoCommands, RecordError, Recorder, TIC_CAP, Timestamp, session_name, wire24_admits,
};
pub use screens::{APP_PALETTE, CLEAR, Screen, glyph, labelled_pause_overlay, pause_overlay};

use crate::input::{Event, EventLayer, Input, InputError, MouseButton, Preset, RecordedTic, keys};
use doom_render::hu_stuff::Message;
use doom_render::{
    DisplayFrame, Frame, GameMode, LumpDirectory, ResourceError, Resources, SCREENHEIGHT,
    SCREENWIDTH,
};
use doom_sim::{
    MapContextOptions, PlayerSlot, SimError, Simulation, Skill, TicCommand, WeaponInventory,
};
use frames::FrameSource;
use std::fmt;

/// Logical tics per second (`TICRATE`, doomdef.h:122).
pub const TICRATE: u64 = 35;
/// The most tics one [`App::update`] runs; the surplus is dropped (`NetUpdate`'s bound,
/// d_net.c:404; decision S8).
pub const MAX_TICS_PER_UPDATE: u64 = 5;
/// The demo is rewritten every this many tics while playing (30 seconds), so a crash loses
/// at most that much (M5b investigation §3.6).
pub const SAVE_INTERVAL: usize = 35 * 30;
/// The label of a session with [`SessionConfig::unrecorded_full_turning`], wherever its state is
/// shown (decision S3 (ii), owner ruling 2026-11-17).
pub const UNRECORDED_LABEL: &str = "not recorded: no demo, cannot be replayed";
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
    /// Called after every committed and drawn tic, for headless session verification (M5b
    /// investigation §2.3); a window has no use for it.
    fn tic(&mut self, _report: &TicReport<'_>) {}
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
    /// The `M_Random` index after the tic.
    pub rndindex: u8,
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
/// injection evidence feeds (`tools/reference/m5b_inject.py`). A command outside wire 24 is
/// refused by the simulation's own preflight.
pub fn replay_full_resolution(
    data: &GameData,
    screenblocks: u8,
    commands: &[TicCommand],
    host: &mut dyn Host,
) -> Option<Ending> {
    let refuse = |tic, e| {
        Some(Ending::Refused {
            tic,
            witness: Witness::Frame(e),
        })
    };
    let mut sim = data.start.clone();
    let mut frames = match FrameSource::start(&data.assets, screenblocks, &sim) {
        Ok(f) => f,
        Err(e) => return refuse(0, e),
    };
    let (mut frame, none) = (Frame::new(), TicCommand::default());
    for &executed in commands {
        let tic = sim.tic();
        if let Err(e) = sim.advance([executed, none, none, none]) {
            return Some(ending_of(&sim, e, tic));
        }
        let drawn = frames
            .observe(&sim, &executed)
            .and_then(|m| Ok((m, frames.draw(&data.assets, &sim, &mut frame)?)));
        let (message, display) = match drawn {
            Ok(d) => d,
            Err(e) => return refuse(tic, e),
        };
        host.tic(&TicReport {
            tic,
            events: &[],
            executed,
            simulation: &sim,
            frame: &frame,
            display: &display,
            message,
            rndindex: frames.rndindex(),
        });
    }
    None
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
        }
    }
}

impl std::error::Error for LoadError {}

/// Everything loaded once per run: the frame assets and the simulation at tic 0, which every
/// session (and restart) starts from.
#[derive(Clone, Debug)]
pub struct GameData {
    assets: FrameAssets,
    start: Simulation,
    map: u8,
    skill: u8,
}

impl GameData {
    /// Loads `E1M<map>` at `skill` from `wads` (the IWAD first, then extra WADs; later files
    /// take precedence, as `W_AddFile` order does) in game `mode`.
    pub fn load(wads: &[&[u8]], mode: GameMode, map: u8, skill: u8) -> Result<Self, LoadError> {
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
        let res = Resources::load(dir, mode).map_err(LoadError::Resources)?;
        let assets = FrameAssets::load(res, &map_data).map_err(LoadError::Resources)?;
        // The M4b/M5a run set-up (player_level_run --policy episode1v2).
        let mut start = Simulation::original_player_episode1_v2_map(
            map_data,
            map,
            skill_value,
            MapContextOptions::default(),
        )
        .map_err(LoadError::Registry)?;
        start.spawn_map_things().map_err(LoadError::Admission)?;
        start
            .initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
            .map_err(LoadError::Admission)?;
        start.spawn_level_specials().map_err(LoadError::Admission)?;
        Ok(Self {
            assets,
            start,
            map,
            skill,
        })
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
}

impl fmt::Display for Witness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(e) => write!(f, "input: {e}"),
            Self::Record(e) => write!(f, "recorder: {e}"),
            Self::Simulation(e) => write!(f, "{e}"),
            Self::Frame(e) => write!(f, "{e}"),
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
    mode: Mode,
    clock: TicClock,
    events: Vec<Event>,
    screen: Option<Screen>,
    quit: bool,
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
            mode: Mode::Playing,
            clock: TicClock::default(),
            events: Vec::new(),
            screen: None,
            quit: false,
            data,
            cfg,
        };
        app.begin(host);
        app
    }

    /// A fresh session (decision S11): the tic-0 simulation, fresh input, presentation and
    /// recorder, a new file name, the clock re-based.
    fn begin(&mut self, host: &mut dyn Host) {
        let controls = self.cfg.preset.controls();
        self.sim = self.data.start.clone();
        self.input = Input::new(controls);
        self.layer = EventLayer::new(&controls, self.cfg.preset.layer_options());
        self.frame = Frame::new();
        self.palette = 0;
        self.drawn = false;
        self.recorder = Recorder::new(self.data.map, self.data.skill, self.cfg.tic_cap);
        self.save = SaveStatus::NotSaved;
        self.clock = TicClock::default();
        self.screen = None;
        self.mode = Mode::Playing;
        self.name = String::new();
        self.claimed = false;
        self.started = host.timestamp();
        // the file is created at once (an empty demo), so an unwritable directory shows at
        // the first pause, not at the session's end
        self.save(host);
        match FrameSource::start(&self.data.assets, self.cfg.screenblocks, &self.sim) {
            Ok(f) => self.frames = Some(f),
            Err(e) => {
                self.frames = None;
                self.end(
                    Ending::Refused {
                        tic: 0,
                        witness: Witness::Frame(e),
                    },
                    host,
                );
            }
        }
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
            Mode::Paused if down && pause_key => self.mode = Mode::Playing,
            Mode::Paused | Mode::Ended(_) if down && code == b'r' => self.restart(host),
            Mode::Paused if down && code == b'q' => self.quit(host),
            Mode::Ended(_) if down && (code == b'q' || code == keys::KEY_ESCAPE) => self.quit(host),
            _ => {}
        }
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
    /// before the first tic after the pause) and the clock is re-based on resume.
    fn pause(&mut self) {
        self.mode = Mode::Paused;
        self.layer.release_all();
        self.clock.origin = None;
    }

    /// Restarts: saves the current session unless its file already holds every tic (a failed
    /// save at the session's end is retried here), then begins a new one.
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
        let drawn = frames.observe(&self.sim, &executed).and_then(|m| {
            let d = frames.draw(&self.data.assets, &self.sim, &mut self.frame)?;
            Ok((m, d))
        });
        match drawn {
            Ok((message, drawn)) => {
                self.palette = drawn.palette;
                self.drawn = true;
                host.tic(&TicReport {
                    tic,
                    events: &self.events,
                    executed,
                    simulation: &self.sim,
                    frame: &self.frame,
                    display: &drawn,
                    message,
                    rndindex: frames.rndindex(),
                });
            }
            // The simulation committed the tic, so its command stays in the demo (only a
            // simulation refusal drops it): the demo replays to this same frame refusal.
            Err(e) => return self.end(refuse(Witness::Frame(e)), host),
        }
        if self.recorder.len() % SAVE_INTERVAL == 0 {
            self.save(host);
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
            Ok(()) => SaveStatus::Saved(self.recorder.len()),
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
    /// values use 4), with the pause overlay composed on top while paused (naming a failed save
    /// of the demo), or the end screen in the app's palette. The game frame is never changed.
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
        let overlay = (self.mode == Mode::Paused).then(|| match (self.recorded(), &self.save) {
            (false, _) => labelled_pause_overlay(UNRECORDED_LABEL),
            (true, SaveStatus::Failed(_)) => labelled_pause_overlay(SAVE_FAILED_LABEL),
            (true, _) => pause_overlay(),
        });
        let pixels = out.chunks_exact_mut(4).take(SCREENWIDTH * SCREENHEIGHT);
        for (i, px) in pixels.enumerate() {
            let rgb = match (&overlay, palette) {
                (Some(o), _) if o.pixels()[i] != CLEAR => app(o.pixels()[i]),
                (_, Some(pal)) => {
                    let at = usize::from(self.frame.pixels()[i]) * 3;
                    let c = [0, 1, 2].map(|k| ramp[usize::from(pal[at + k])]);
                    if overlay.is_some() {
                        c.map(|v| v / 2)
                    } else {
                        c
                    }
                }
                (_, None) => [0, 0, 0],
            };
            px.copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
}
