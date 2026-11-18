//! The frame source: everything that turns simulation state into a 320×200 indexed frame and
//! a palette number, behind one small type (decision S19; M5b investigation §3.4–§3.5, §3.8).
//!
//! It wraps doom-render's composed [`Display`]: the status bar, the 3D view and the HUD message
//! in `D_Display`'s order, and the palette number `ST_doPaletteStuff` chooses. Frames are
//! identical to the genuine engine under the declared sound-free, wipe-free contract (decision
//! S15): the `M_Random` index starts at 0 with every session and only the status bar ticker
//! advances it (a vanilla session would also draw it for sound and for the level-start wipe).
//! The evidence has two exclusions: 300 frames are compared only on a masked digest and 70 are
//! not compared (counts asserted in crates/doom-render/tests/m5b_display.rs).
//!
//! M5c (decisions T5, T6, T15, T16): game data loaded with sound ([`super::GameData::load_with_sound`])
//! gives a frame source that also runs the session's [`Sound`] and draws under doom-render's
//! `RandomContract::WithSound`. One tic then does, in the C order (M5c note §4.3.3):
//!
//! 1. the simulation has committed the tic (`P_Ticker`, g_game.c:730), its sound records readable;
//! 2. [`Sound::tic_from_simulation`]: every record in call order through the sound logic with
//!    `gametic == t` and the `M_Random` index, then `S_UpdateSounds` (d_main.c:392, no draw) and
//!    the buffer the declared schedule makes due (`I_UpdateSound`, d_main.c:397-400);
//! 3. `Display::observe_after` with the tic's sound draws: the index advances by them, then
//!    `ST_Ticker` draws once (g_game.c:731, st_stuff.c:992) and `HU_Ticker` runs (g_game.c:733);
//! 4. `Display::draw` (`D_Display`, d_main.c:395); the frame of tic 0 is followed by the wipe's
//!    320 draws (d_main.c:327-340, f_wipe.c:159-166).
//!
//! `S_UpdateSounds` and the mix run before the frame is drawn here, where the C draws in between
//! (d_main.c:392-400); nothing in `D_Display` reads sound state and nothing in the mixer reads the
//! display, so the order changes no output (note §4.3.3). The tic is atomic for sound and display:
//! both run on staged copies and are committed together only when the frame is drawn, so a
//! refusal of either leaves both as they were before the tic. The simulation's committed tic is
//! kept, as for M5b's frame refusals (decision S9: the demo replays to the same refusal); it is
//! never cloned per tic.

use doom_map::Map;
use doom_render::hu_stuff::Message;
use doom_render::st_stuff::StatusInputs;
use doom_render::{
    ConfigError, Display, DisplayConfig, DisplayFrame, DisplayObserveError, DisplayRefusal, Frame,
    Level, ResourceError, Resources,
};
use doom_render::{DisplayState, RandomContract, StateDecodeError};
use doom_sim::{Simulation, TicCommand};
use doom_sound::lumps::SoundSet;
use doom_sound::session::{Sound, SoundConfig, TicSound};
use doom_sound::state::{SoundState, StateError};
use std::fmt;
use std::sync::Arc;

use super::Witness;

/// The adjustment of the status bar inputs and message of tic `t` that the bespoke cheat scenario
/// of decision S1 applies (doom-render's `Display::observe_with`); the app never uses one.
pub type StatusAdjustment<'a> = &'a dyn Fn(u64, &mut StatusInputs, &mut Option<Message>);

/// The bytes of one PLAYPAL palette.
pub const PALETTE_BYTES: usize = 768;

/// What the frame source needs from the WADs, loaded once per run and shared by restarts.
#[derive(Clone, Debug)]
pub struct FrameAssets {
    res: Arc<Resources>,
    level: Arc<Level>,
    playpal: Vec<u8>,
}

impl FrameAssets {
    /// The level tables of `map` (episode 1), the status bar and HUD lumps (checked by building
    /// one display) and the 14 palettes, over loaded resources.
    pub(crate) fn load(res: Resources, map: &Map) -> Result<Self, ResourceError> {
        let res = Arc::new(res);
        let level = Arc::new(Level::load(&res, map, 1)?);
        let display = Display::new(
            res.clone(),
            level.clone(),
            DisplayConfig { screenblocks: 10 },
        )?;
        Ok(Self {
            playpal: display.playpal().to_vec(),
            res,
            level,
        })
    }

    /// Palette `number` of PLAYPAL (0..=13), as the display keeps it.
    pub fn palette(&self, number: u8) -> Option<&[u8]> {
        let at = usize::from(number) * PALETTE_BYTES;
        self.playpal.get(at..at + PALETTE_BYTES)
    }
}

/// A refused frame: the tic's frame cannot be drawn as the genuine engine would.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameRefusal {
    /// The view configuration (screenblocks) is not admitted.
    Config(ConfigError),
    /// The display could not be built over the resources.
    Load(ResourceError),
    /// The display could not follow the tic (a witness of decision S17 or of the renderer).
    Observe(DisplayObserveError),
    /// The frame was refused (an M5a render witness or a 2D drawer witness).
    Draw(DisplayRefusal),
    /// The display chose a palette the WADs do not hold.
    Palette(u8),
}

impl fmt::Display for FrameRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(e) => write!(f, "{e}"),
            Self::Load(e) => write!(f, "display: {e:?}"),
            Self::Observe(e) => write!(f, "{e}"),
            Self::Draw(e) => write!(f, "{e}"),
            Self::Palette(n) => write!(f, "palette {n} is not in PLAYPAL"),
        }
    }
}

/// What the sound session needs from the WADs (M5c), loaded once per run and shared by restarts:
/// the padded sound data (`I_InitSound`, i_sound.c:796-813) and a fresh process's configuration
/// for the map (decisions T9, T15), with the texture count the layout detector of decision T12
/// reads (`numtextures`, r_data.c:480).
#[derive(Clone, Debug)]
pub struct SoundAssets {
    /// A fresh process's session, built (and so checked) at load and cloned at every session
    /// start.
    pub(crate) fresh: Sound,
}

impl SoundAssets {
    /// Builds a fresh session over `set` for `config` (refused as `Sound::new` refuses).
    pub(crate) fn new(
        set: Arc<SoundSet>,
        config: SoundConfig,
    ) -> Result<Self, doom_sound::logic::SoundRefusal> {
        Ok(Self {
            fresh: Sound::new(set, config)?,
        })
    }

    /// The loaded sound data.
    pub fn set(&self) -> &Arc<SoundSet> {
        self.fresh.mixer().sound_set()
    }

    /// A fresh session's configuration.
    pub fn config(&self) -> SoundConfig {
        self.fresh.config()
    }
}

/// What one committed tic presented.
#[derive(Clone, Debug)]
pub struct Presented {
    /// The message `HU_Ticker` took in the tic.
    pub message: Option<Message>,
    /// The frame's palette number, palette set, face, message flag and view report.
    pub display: DisplayFrame,
    /// What the sound session did in the tic (M5c), with the buffer mixed after it, if any.
    pub sound: Option<TicSound>,
}

/// A saved frame source (M5c note §4.8): the display state `DDSP` (version 1 under the M5b
/// contract, version 2 with sound) and, with sound, the session state `DSND` version 1. With a
/// clone of the simulation of the same tic it restores and continues ([`FrameSource::restore`]);
/// nothing in the app restores (as in M5b), the encodings exist for tests, fuzzing and a later
/// save and load.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedPresentation {
    /// The `DDSP` bytes.
    pub display: Vec<u8>,
    /// The `DSND` bytes, with sound.
    pub sound: Option<Vec<u8>>,
}

/// Why a saved presentation could not be restored. Nothing is built.
#[derive(Clone, Debug, PartialEq)]
pub enum PresentationRestoreError {
    /// The display bytes do not decode.
    DisplayDecode(StateDecodeError),
    /// The display state does not fit the simulation or the level.
    Display(doom_render::d_display::DisplayRestoreError),
    /// The sound bytes do not decode, or do not fit the sound data or the simulation.
    Sound(StateError),
    /// The saved parts do not fit the game data: sound bytes without sound data or the reverse,
    /// or a display contract that is not the data's.
    Mismatch,
}

impl fmt::Display for PresentationRestoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "presentation restore: {self:?}")
    }
}

impl std::error::Error for PresentationRestoreError {}

/// The per-session presentation state: built fresh at every session start (decision S11): the
/// display and, with sound (M5c), the session's [`Sound`] (a fresh process, decision T15).
#[derive(Clone, Debug)]
pub struct FrameSource {
    display: Display,
    sound: Option<Sound>,
}

impl FrameSource {
    /// A fresh frame source for `data` at `screenblocks` (10 or 11, decision S14, fixed for the
    /// session), started on the level at tic 0 as a new game: with sound data a fresh [`Sound`]
    /// and the display under `RandomContract::WithSound`; without, M5b's display.
    pub fn start(
        data: &super::GameData,
        screenblocks: u8,
        sim: &Simulation,
    ) -> Result<Self, Witness> {
        let mut source =
            Self::start_display(&data.assets, screenblocks, sim).map_err(Witness::Frame)?;
        if let Some(sound) = &data.sound {
            source
                .display
                .set_contract(RandomContract::WithSound)
                .map_err(|e| Witness::Frame(FrameRefusal::Observe(e)))?;
            source.sound = Some(sound.fresh.clone());
        }
        Ok(source)
    }

    /// M5b's display at `screenblocks`, started on the level at tic 0 as a new game.
    fn start_display(
        assets: &FrameAssets,
        screenblocks: u8,
        sim: &Simulation,
    ) -> Result<Self, FrameRefusal> {
        if !(10..=11).contains(&screenblocks) {
            return Err(FrameRefusal::Config(ConfigError::ScreenBlocks(
                screenblocks,
            )));
        }
        let cfg = DisplayConfig { screenblocks };
        let mut display = Display::new(assets.res.clone(), assets.level.clone(), cfg)
            .map_err(FrameRefusal::Load)?;
        display.begin_level(sim).map_err(FrameRefusal::Observe)?;
        Ok(Self {
            display,
            sound: None,
        })
    }

    /// Presents one tic the simulation has just committed, which executed `executed`, into
    /// `frame` (see the module documentation for the order). Without sound this is M5b's
    /// sequence unchanged: the status bar and HUD tickers, then the frame (a frame refused after
    /// the tickers leaves them observed, as in M5b). With sound the tic is atomic for the sound
    /// session and the display: on a refusal both are as they were before the tic.
    pub fn tic(
        &mut self,
        data: &super::GameData,
        sim: &Simulation,
        executed: &TicCommand,
        frame: &mut Frame,
    ) -> Result<Presented, Witness> {
        self.tic_inner(data, sim, executed, frame, None)
    }

    /// [`Self::tic`] with the bespoke cheat scenario's adjustment of the status bar inputs
    /// (decision S1; doom-render's `Display::observe_with` and `observe_after_with`), for its
    /// frame tests only: the app never calls it.
    pub fn tic_adjusted(
        &mut self,
        data: &super::GameData,
        sim: &Simulation,
        executed: &TicCommand,
        frame: &mut Frame,
        adjust: StatusAdjustment<'_>,
    ) -> Result<Presented, Witness> {
        self.tic_inner(data, sim, executed, frame, Some(adjust))
    }

    fn tic_inner(
        &mut self,
        data: &super::GameData,
        sim: &Simulation,
        executed: &TicCommand,
        frame: &mut Frame,
        adjust: Option<StatusAdjustment<'_>>,
    ) -> Result<Presented, Witness> {
        let t = sim.tic().wrapping_sub(1);
        let observe = |d: &mut Display, draws: Option<u16>| {
            let r = match (adjust, draws) {
                (None, None) => d.observe(sim, executed),
                (Some(a), None) => d.observe_with(sim, executed, |i, m| a(t, i, m)),
                (None, Some(n)) => d.observe_after(sim, executed, n),
                (Some(a), Some(n)) => d.observe_after_with(sim, executed, n, |i, m| a(t, i, m)),
            };
            r.map_err(|e| Witness::Frame(FrameRefusal::Observe(e)))
        };
        let Some(sound) = &self.sound else {
            let message = observe(&mut self.display, None)?;
            let display = draw(&mut self.display, &data.assets, sim, frame)?;
            return Ok(Presented {
                message,
                display,
                sound: None,
            });
        };
        let mut staged_sound = sound.clone();
        let ticked = staged_sound
            .tic_from_simulation(sim, self.display.status().rndindex)
            .map_err(Witness::Sound)?;
        let mut staged = self.display.clone();
        let message = observe(&mut staged, Some(ticked.draws))?;
        let display = draw(&mut staged, &data.assets, sim, frame)?;
        // Commit: nothing below can fail.
        self.display = staged;
        self.sound = Some(staged_sound);
        Ok(Presented {
            message,
            display,
            sound: Some(ticked),
        })
    }

    /// The `M_Random` index after the last observed tic (and, with sound, after the wipe's draws
    /// once the first frame is drawn).
    pub fn rndindex(&self) -> u8 {
        self.display.status().rndindex
    }

    /// The display.
    pub fn display(&self) -> &Display {
        &self.display
    }

    /// The sound session, with sound.
    pub fn sound(&self) -> Option<&Sound> {
        self.sound.as_ref()
    }

    /// The saved state (`None` before the display has a level).
    pub fn save(&self) -> Option<SavedPresentation> {
        Some(SavedPresentation {
            display: self.display.state()?.encode(),
            sound: self.sound.as_ref().map(|s| s.state().encode()),
        })
    }

    /// A frame source rebuilt from `saved` next to `sim`, a simulation of the same tic (a clone
    /// of the one the state was saved with), over `data` at the saved screen size. Every decoder
    /// and restore check of doom-render (`DisplayState::decode_any`, `Display::restore`) and
    /// doom-sound (`SoundState::decode`, `Sound::restore`) applies; the display's contract must be
    /// the data's (with sound exactly when the data has sound). On an error nothing is built.
    pub fn restore(
        data: &super::GameData,
        sim: &Simulation,
        saved: &SavedPresentation,
    ) -> Result<Self, PresentationRestoreError> {
        let state = DisplayState::decode_any(&saved.display)
            .map_err(PresentationRestoreError::DisplayDecode)?;
        let want = if data.sound.is_some() {
            RandomContract::WithSound
        } else {
            RandomContract::SoundFreeWipeFree
        };
        if state.contract != want || data.sound.is_some() != saved.sound.is_some() {
            return Err(PresentationRestoreError::Mismatch);
        }
        let sound = match (&data.sound, &saved.sound) {
            (Some(assets), Some(bytes)) => {
                let s = SoundState::decode(bytes).map_err(PresentationRestoreError::Sound)?;
                Some(
                    Sound::restore(assets.set().clone(), &s, sim)
                        .map_err(PresentationRestoreError::Sound)?,
                )
            }
            _ => None,
        };
        let cfg = DisplayConfig {
            screenblocks: state.screenblocks,
        };
        let mut display = Display::new(data.assets.res.clone(), data.assets.level.clone(), cfg)
            .map_err(|_| PresentationRestoreError::Mismatch)?;
        display
            .restore(state, sim)
            .map_err(PresentationRestoreError::Display)?;
        Ok(Self { display, sound })
    }
}

/// Draws the current tic into `frame`: its palette number and report, refused when the palette
/// is not in the WADs.
fn draw(
    display: &mut Display,
    assets: &FrameAssets,
    sim: &Simulation,
    frame: &mut Frame,
) -> Result<DisplayFrame, Witness> {
    let drawn = display
        .draw(sim, frame)
        .map_err(|e| Witness::Frame(FrameRefusal::Draw(e)))?;
    assets
        .palette(drawn.palette)
        .map(|_| ())
        .ok_or(Witness::Frame(FrameRefusal::Palette(drawn.palette)))?;
    Ok(drawn)
}
