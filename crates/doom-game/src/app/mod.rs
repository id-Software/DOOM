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
//! draws it. Every executed tic is drawn; the picture shown is the last one.

mod frames;
mod gamma;
mod recorder;
mod screens;

pub use frames::{FrameAssets, FrameRefusal, PALETTE_BYTES};
pub use gamma::GAMMA;
pub use recorder::{RecordError, Recorder, TIC_CAP, Timestamp, session_name, wire24_admits};
pub use screens::{APP_PALETTE, CLEAR, Screen, glyph, pause_overlay};

use crate::input::{Event, EventLayer, Input, InputError, MouseButton, Preset, RecordedTic, keys};
use doom_render::{
    Frame, GameMode, LumpDirectory, ResourceError, Resources, SCREENHEIGHT, SCREENWIDTH,
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
/// The bytes of [`App::picture`]: 320×200 RGBA.
pub const PICTURE_BYTES: usize = SCREENWIDTH * SCREENHEIGHT * 4;

/// The platform services the app needs outside the simulation.
pub trait Host {
    /// The local date and time, for a new session's file name.
    fn timestamp(&mut self) -> Timestamp;
    /// Whether a session file of this name already exists.
    fn exists(&mut self, name: &str) -> bool;
    /// Writes (or rewrites) the session file `name` whole.
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String>;
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
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            screenblocks: 10,
            preset: Preset::Vanilla,
            tic_cap: TIC_CAP,
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
    /// A tic was refused (decision S9).
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
        self.name = Self::free_name(&self.data, host);
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

    fn free_name(data: &GameData, host: &mut dyn Host) -> String {
        let base = session_name(data.map, data.skill, host.timestamp());
        let mut name = base.clone();
        for n in 2..1000 {
            if !host.exists(&name) {
                break;
            }
            name = format!("{}-{n}.lmp", base.trim_end_matches(".lmp"));
        }
        name
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

    /// The session's file name.
    pub fn session_name(&self) -> &str {
        &self.name
    }

    /// The state of the session's file.
    pub fn save_status(&self) -> &SaveStatus {
        &self.save
    }

    /// Whether the player asked to quit (the demo is already saved).
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

    /// Relative mouse motion in window pixels (`dy` positive downward); ignored unless playing.
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

    /// The window was minimised or lost focus: pauses a playing session.
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

    /// Restarts: saves the current session if it has not ended, then begins a new one.
    pub fn restart(&mut self, host: &mut dyn Host) {
        if !matches!(self.mode, Mode::Ended(_)) {
            self.save(host);
        }
        self.begin(host);
    }

    /// Quits: saves the session if it has not ended.
    pub fn quit(&mut self, host: &mut dyn Host) {
        if !matches!(self.mode, Mode::Ended(_)) {
            self.save(host);
        }
        self.quit = true;
    }

    /// Runs the tics due at monotonic time `now_us`: at most [`MAX_TICS_PER_UPDATE`], the rest
    /// dropped. The first call only starts the clock; the first tic falls due 1/35 s later.
    pub fn update(&mut self, now_us: u64, host: &mut dyn Host) -> Update {
        if self.mode != Mode::Playing {
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
        if self.recorder.full() {
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
        let executed = match self.recorder.record(recorded) {
            Ok(c) => c,
            Err(e) => return self.end(refuse(Witness::Record(e)), host),
        };
        let none = TicCommand::default();
        if let Err(e) = self.sim.advance([executed, none, none, none]) {
            self.recorder.drop_last();
            let ending = match e {
                SimError::LevelExited => Ending::Exit {
                    secret: self.secret_exit(),
                },
                SimError::PlayerReborn => Ending::Died,
                e => refuse(Witness::Simulation(e)),
            };
            return self.end(ending, host);
        }
        let Some(frames) = self.frames.as_mut() else {
            return;
        };
        let drawn = frames
            .observe(&self.sim, &executed)
            .and_then(|_| frames.draw(&self.data.assets, &self.sim, &mut self.frame));
        match drawn {
            Ok(drawn) => {
                self.palette = drawn.palette;
                self.drawn = true;
            }
            Err(e) => return self.end(refuse(Witness::Frame(e)), host),
        }
        if self.recorder.len() % SAVE_INTERVAL == 0 {
            self.save(host);
        }
    }

    fn secret_exit(&self) -> bool {
        self.sim
            .snapshot()
            .fields()
            .any(|f| f.path() == "level.secret_exit" && f.value() == [1])
    }

    fn save(&mut self, host: &mut dyn Host) {
        self.save = match self.recorder.bytes() {
            Ok(bytes) => match host.save(&self.name, &bytes) {
                Ok(()) => SaveStatus::Saved(self.recorder.len()),
                Err(e) => SaveStatus::Failed(e),
            },
            Err(e) => SaveStatus::Failed(format!("the demo writer refused: {e:?}")),
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
                self.recorder.len()
            ),
            WHITE,
        ));
        let saved = match &self.save {
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

    /// The picture to present, as 320×200 RGBA in `out` ([`PICTURE_BYTES`] bytes): the game
    /// frame through its PLAYPAL palette and gamma table `gamma` (0–4, decision S13; larger
    /// values use 4), with the pause overlay composed on top while paused, or the end screen
    /// in the app's palette. The game frame itself is never changed.
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
        let overlay = (self.mode == Mode::Paused).then(pause_overlay);
        for (i, px) in out.chunks_exact_mut(4).enumerate() {
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
