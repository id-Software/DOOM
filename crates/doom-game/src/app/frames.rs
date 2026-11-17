//! The frame source: everything that turns simulation state into a 320×200 indexed frame and
//! a palette number, behind one small type (decision S19; M5b investigation §3.4–§3.5, §3.8).
//!
//! It wraps doom-render's composed [`Display`]: the status bar, the 3D view and the HUD message
//! in `D_Display`'s order, and the palette number `ST_doPaletteStuff` chooses. Frames are
//! identical to the genuine engine under the declared sound-free, wipe-free contract (decision
//! S15): the `M_Random` index starts at 0 with every session and only the status bar ticker
//! advances it.

use doom_map::Map;
use doom_render::hu_stuff::Message;
use doom_render::{
    ConfigError, Display, DisplayConfig, DisplayFrame, DisplayObserveError, DisplayRefusal, Frame,
    Level, ResourceError, Resources,
};
use doom_sim::{Simulation, TicCommand};
use std::fmt;
use std::sync::Arc;

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

/// The per-session presentation state: built fresh at every session start (decision S11).
#[derive(Clone, Debug)]
pub(crate) struct FrameSource {
    display: Display,
}

impl FrameSource {
    /// A fresh display at `screenblocks` (10 or 11, decision S14, fixed for the session),
    /// started on the level at tic 0 as a new game.
    pub(crate) fn start(
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
        Ok(Self { display })
    }

    /// Follows one committed tic, which executed `executed`: the status bar and HUD tickers.
    /// Returns the message `HU_Ticker` took in this tic, if any.
    pub(crate) fn observe(
        &mut self,
        sim: &Simulation,
        executed: &TicCommand,
    ) -> Result<Option<Message>, FrameRefusal> {
        self.display
            .observe(sim, executed)
            .map_err(FrameRefusal::Observe)
    }

    /// Draws the current tic into `frame`: its palette number and report.
    pub(crate) fn draw(
        &mut self,
        assets: &FrameAssets,
        sim: &Simulation,
        frame: &mut Frame,
    ) -> Result<DisplayFrame, FrameRefusal> {
        let drawn = self.display.draw(sim, frame).map_err(FrameRefusal::Draw)?;
        assets
            .palette(drawn.palette)
            .map(|_| ())
            .ok_or(FrameRefusal::Palette(drawn.palette))?;
        Ok(drawn)
    }
}
