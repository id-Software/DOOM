//! The frame source: everything that turns simulation state into a 320×200 indexed frame and
//! a palette number, behind one small type (decision S19; M5b investigation §3.4–§3.5).
//!
//! In this step it wraps M5a's [`Renderer`]: the 3D view only, palette number 0, no status bar
//! or HUD (at screenblocks 10 the bottom 32 rows stay 0). When doom-render's composed
//! `Display` lands, this module's bodies change to it and nothing outside this file does:
//! [`FrameAssets::load`] also loads the status-bar and HUD lumps and keeps all 14 PLAYPAL
//! palettes, [`FrameSource::observe`] feeds the executed command to `Display::observe` (the
//! status-bar and HUD tickers), and [`FrameSource::draw`] returns `Display::draw`'s palette.

use doom_map::Map;
use doom_render::{
    ConfigError, Detail, Frame, Level, ObserveError, RenderConfig, RenderRefusal, Renderer,
    ResourceError, Resources,
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
    /// The level tables of `map` (episode 1) and the palettes, over loaded resources.
    pub(crate) fn load(res: Resources, map: &Map) -> Result<Self, ResourceError> {
        let level = Level::load(&res, map, 1)?;
        let playpal = res
            .palette
            .clone()
            .ok_or_else(|| ResourceError::LumpNotFound("PLAYPAL".into()))?;
        Ok(Self {
            res: Arc::new(res),
            level: Arc::new(level),
            playpal,
        })
    }

    /// Palette `number` of PLAYPAL, if the frame source keeps it.
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
    /// The render state could not follow the tic.
    Observe(ObserveError),
    /// The frame was refused (an M5a render witness).
    Render(RenderRefusal),
    /// The frame source chose a palette the WADs do not hold.
    Palette(u8),
}

impl fmt::Display for FrameRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(e) => write!(f, "{e}"),
            Self::Observe(e) => write!(f, "{e}"),
            Self::Render(e) => write!(f, "{e}"),
            Self::Palette(n) => write!(f, "palette {n} is not in PLAYPAL"),
        }
    }
}

/// The per-session presentation state: built fresh at every session start.
#[derive(Clone, Debug)]
pub(crate) struct FrameSource {
    renderer: Renderer,
}

impl FrameSource {
    /// A frame source at `screenblocks` (10 or 11, decision S14), started on the level at
    /// tic 0.
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
        let cfg = RenderConfig {
            screenblocks,
            detail: Detail::High,
        };
        let mut renderer = Renderer::new(assets.res.clone(), assets.level.clone(), cfg)
            .map_err(FrameRefusal::Config)?;
        renderer.begin_level(sim).map_err(FrameRefusal::Observe)?;
        Ok(Self { renderer })
    }

    /// Follows one committed tic, which executed `_executed`.
    pub(crate) fn observe(
        &mut self,
        sim: &Simulation,
        _executed: &TicCommand,
    ) -> Result<(), FrameRefusal> {
        self.renderer.observe(sim).map_err(FrameRefusal::Observe)
    }

    /// Draws the current tic into `frame` and returns its palette number.
    pub(crate) fn draw(
        &mut self,
        assets: &FrameAssets,
        sim: &Simulation,
        frame: &mut Frame,
    ) -> Result<u8, FrameRefusal> {
        self.renderer
            .render(sim, frame)
            .map_err(FrameRefusal::Render)?;
        let palette = 0;
        assets
            .palette(palette)
            .map(|_| palette)
            .ok_or(FrameRefusal::Palette(palette))
    }
}
