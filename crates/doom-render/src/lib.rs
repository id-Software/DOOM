//! A faithful port of linuxdoom-1.10's software renderer for the 3D view
//! (`R_RenderPlayerView`), milestone M5a. Decisions R1-R16 and the owner rulings are in
//! `docs/M5A_REFERENCE_INVESTIGATION.md` §5; the original id Software Linux DOOM 1.10 source is
//! the behavioural authority.
//!
//! The renderer reads the simulation only through `doom-sim`'s public `&self` API and
//! [`doom_sim::Simulation::snapshot`]; it never writes the simulation. Everything the genuine
//! renderer reads that the simulation does not keep (the view height, the switch textures,
//! the mapped-line set, `fuzzpos`, the persistent framebuffer) lives in a self-contained,
//! saveable [`RenderState`].
//!
//! Use: build [`r_data::Resources`] from the genuine global lump list, a [`Renderer`] per view
//! configuration, call [`Renderer::begin_level`] at tic 0, then after every committed
//! `Simulation::advance` call [`Renderer::observe`] and [`Renderer::render`].
//!
//! Status (M5a step 2a): the foundation (tables, view setup, resources, the render state and
//! the view shadow) is complete; the BSP traversal and every drawer refuse the frame with
//! [`RenderRefusal::NotImplemented`] until steps 2b and 2c.
use std::fmt;
use std::sync::Arc;

pub mod p_spec;
pub mod png;
pub mod r_bsp;
pub mod r_data;
pub mod r_draw;
pub mod r_main;
pub mod r_plane;
pub mod r_segs;
pub mod r_sky;
pub mod r_things;
pub mod state;
mod tables;
pub mod view;

pub use r_data::{GameMode, LumpDirectory, ResourceError, Resources};
pub use r_main::{SCREENHEIGHT, SCREENWIDTH};
pub use state::{RenderState, StateDecodeError};

use doom_sim::{PlayerSlot, Simulation};

/// Detail level (`detailLevel`, m_misc.c:280).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Detail {
    /// High detail (`detailshift` 0).
    High,
    /// Low detail: refused (decision R2; `R_DrawColumnLow` is broken in this source).
    Low,
}

/// A view configuration: `screenblocks` 9, 10 or 11 at high detail (decision R2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderConfig {
    /// `screenblocks`.
    pub screenblocks: u8,
    /// Detail level.
    pub detail: Detail,
}

/// A refused view configuration (decision R2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// Only screenblocks 9, 10 and 11 are compared.
    ScreenBlocks(u8),
    /// Low detail is refused.
    LowDetail,
}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "render configuration refused: {self:?}")
    }
}
impl std::error::Error for ConfigError {}

/// A refused frame. The render state is unchanged by a refused frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderRefusal {
    /// A renderer stage not implemented yet (M5a steps 2b/2c); never a silent no-op.
    NotImplemented(&'static str),
    /// `FixedDiv2`'s `I_Error` (m_fixed.c:83-84).
    FixedDiv,
    /// The renderer has not observed the simulation's current tic.
    NotObserved {
        /// The last tic the renderer observed (None before `begin_level`).
        observed: Option<u64>,
        /// The simulation's tic.
        simulation: u64,
    },
    /// The simulation has no player view (a profile the renderer does not admit).
    NoPlayer,
}
impl fmt::Display for RenderRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "frame refused: {self:?}")
    }
}
impl std::error::Error for RenderRefusal {}

/// Why a tic could not be observed. The render state is unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObserveError {
    /// The view shadow could not follow the tic.
    Shadow(view::ShadowError),
    /// The switch tracker's model of the tic's switch events failed.
    SwitchInconsistency,
    /// A canonical field the renderer reads was malformed.
    Snapshot(String),
}
impl fmt::Display for ObserveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tic not observed: {self:?}")
    }
}
impl std::error::Error for ObserveError {}

/// A 320x200 frame of palette indices, row major (`screens[0]`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pixels: Vec<u8>,
}
impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}
impl Frame {
    /// A zeroed frame.
    pub fn new() -> Self {
        Self {
            pixels: vec![0; SCREENWIDTH * SCREENHEIGHT],
        }
    }
    /// The 64,000 palette indices.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    /// Mutable access (tests and self-tests only).
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }
}

/// What a rendered frame reports beside its pixels.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FrameReport {
    /// Screen columns holding a pixel whose source read was undefined (decision R4).
    pub flagged_columns: Vec<u16>,
    /// The switch-texture parity is unknown: the frame is non-comparable (decision R7).
    pub switch_parity_unknown: bool,
    /// Lines newly marked mapped by this frame (`ML_MAPPED`, decision R5).
    pub newly_mapped: Vec<u16>,
    /// `fuzzpos` after the frame.
    pub fuzzpos: u32,
}

/// The renderer stage that wrote a framebuffer pixel in the last frame (first-difference
/// reporting only; never part of the comparison or the render state).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Feature {
    /// Not written by the last frame (the persistent framebuffer's earlier contents).
    #[default]
    Untouched,
    /// A solid or two-sided wall column (r_segs.c).
    Wall,
    /// A floor or ceiling span (r_plane.c).
    Plane,
    /// A sky column (r_plane.c).
    Sky,
    /// A masked midtexture column (r_segs.c `R_RenderMaskedSegRange`).
    MaskedMid,
    /// A sprite column (r_things.c).
    Sprite,
    /// A weapon sprite column (r_things.c `R_DrawPSprite`).
    PSprite,
    /// A fuzz column (r_draw.c `R_DrawFuzzColumn`).
    Fuzz,
}

/// The map-derived tables the renderer reads (`P_LoadSideDefs`, `P_LoadSectors`).
#[derive(Clone, Debug)]
pub struct Level {
    /// The map.
    pub map: doom_map::Map,
    /// Per sidedef `[top, mid, bottom]` texture numbers (`R_TextureNumForName`).
    pub side_textures: Vec<[usize; 3]>,
    /// Per sector `[floorpic, ceilingpic]` flat numbers (`R_FlatNumForName`).
    pub sector_flats: Vec<[usize; 2]>,
    /// Non-reusable switch lines (`p_spec::NONREUSABLE_SWITCH_SPECIALS`) whose front side has
    /// a switch texture, ascending.
    pub switch_lines: Vec<u16>,
}

impl Level {
    /// Resolves the map's names as `P_SetupLevel` does, refusing what the genuine load would
    /// `I_Error` on or read out of bounds.
    pub fn load(res: &Resources, map: &doom_map::Map) -> Result<Self, ResourceError> {
        let mut side_textures = Vec::with_capacity(map.sidedefs().len());
        for s in map.sidedefs() {
            // doom-map keeps the lump order upper, lower, middle (p_setup.c:438-440).
            let [upper, lower, middle] = s.textures;
            side_textures.push([
                res.texture_num_for_name(upper.as_bytes())?,
                res.texture_num_for_name(middle.as_bytes())?,
                res.texture_num_for_name(lower.as_bytes())?,
            ]);
        }
        let mut sector_flats = Vec::with_capacity(map.sectors().len());
        for s in map.sectors() {
            sector_flats.push([
                res.flat_num_for_name(s.flats[0].as_bytes())?,
                res.flat_num_for_name(s.flats[1].as_bytes())?,
            ]);
        }
        let mut scrollers = 0;
        let mut switch_lines = Vec::new();
        for (i, l) in map.linedefs().iter().enumerate() {
            if l.flags & 4 != 0 && l.sides[1].is_none() {
                return Err(ResourceError::Map(format!(
                    "line {i} is two-sided without a back side (sides[-1], p_setup.c:189-192)"
                )));
            }
            if l.special == 48 {
                scrollers += 1;
            }
            if p_spec::NONREUSABLE_SWITCH_SPECIALS.contains(&l.special) {
                let front = l.sides[0].map(usize::from);
                if front.is_some_and(|f| {
                    p_spec::switch_flip(&res.switchlist, side_textures[f]).is_some()
                }) {
                    switch_lines.push(i as u16);
                }
            }
        }
        if scrollers > 64 {
            return Err(ResourceError::Map(
                "more than MAXLINEANIMS (64) scrolling lines (p_spec.c:1336-1345)".into(),
            ));
        }
        Ok(Self {
            map: map.clone(),
            side_textures,
            sector_flats,
            switch_lines,
        })
    }
}

/// Per-frame context handed to the renderer stages.
pub(crate) struct FrameContext<'a> {
    #[expect(dead_code, reason = "read by the BSP and drawers of steps 2b/2c")]
    pub res: &'a Resources,
    #[expect(dead_code, reason = "read by the BSP and drawers of steps 2b/2c")]
    pub level: &'a Level,
    #[expect(dead_code, reason = "read by the BSP and drawers of steps 2b/2c")]
    pub view: &'a r_main::ViewSize,
    #[expect(dead_code, reason = "read by the BSP and drawers of steps 2b/2c")]
    pub frame: r_main::FrameView,
    #[expect(dead_code, reason = "written by the drawers of steps 2b/2c")]
    pub screen: &'a mut [u8],
    #[expect(dead_code, reason = "written by the drawers of steps 2b/2c")]
    pub taint: &'a mut [u32],
    #[expect(dead_code, reason = "written by the drawers of steps 2b/2c")]
    pub trace: Option<&'a mut [Feature]>,
}

/// The renderer for one view configuration.
#[derive(Clone, Debug)]
pub struct Renderer {
    res: Arc<Resources>,
    level: Arc<Level>,
    view: r_main::ViewSize,
    cfg: RenderConfig,
    state: Option<RenderState>,
    trace: Option<Vec<Feature>>,
}

impl Renderer {
    /// A renderer for `cfg` over loaded resources and a level.
    pub fn new(
        res: Arc<Resources>,
        level: Arc<Level>,
        cfg: RenderConfig,
    ) -> Result<Self, ConfigError> {
        Ok(Self {
            view: r_main::ViewSize::new(cfg)?,
            res,
            level,
            cfg,
            state: None,
            trace: None,
        })
    }
    /// The configuration.
    pub fn config(&self) -> RenderConfig {
        self.cfg
    }
    /// The view-size tables.
    pub fn view_size(&self) -> &r_main::ViewSize {
        &self.view
    }
    /// The resources.
    pub fn resources(&self) -> &Resources {
        &self.res
    }
    /// Records, per pixel, which stage wrote it in the last frame (see [`Feature`]).
    pub fn set_trace(&mut self, on: bool) {
        self.trace = on.then(|| vec![Feature::Untouched; SCREENWIDTH * SCREENHEIGHT]);
    }
    /// The last frame's per-pixel stage record, when enabled.
    pub fn trace(&self) -> Option<&[Feature]> {
        self.trace.as_deref()
    }
    /// The render state (None before [`Self::begin_level`]).
    pub fn state(&self) -> Option<&RenderState> {
        self.state.as_ref()
    }
    /// Replaces the render state (a rewind: restore it next to the simulation snapshot of
    /// the same tic).
    pub fn restore(&mut self, state: RenderState) {
        self.state = Some(state);
    }
    /// Starts the level at tic 0 (decision R8: zeroed framebuffer, `fuzzpos` 0).
    pub fn begin_level(&mut self, sim: &Simulation) -> Result<(), ObserveError> {
        self.state = Some(RenderState::level_start(sim, &self.level)?);
        Ok(())
    }
    /// Observes the simulation's next committed tic (exactly once per tic, in order).
    pub fn observe(&mut self, sim: &Simulation) -> Result<(), ObserveError> {
        let state = self
            .state
            .as_ref()
            .ok_or(ObserveError::Shadow(view::ShadowError::Gap {
                expected: 0,
                got: sim.tic(),
            }))?;
        let next = state.next(sim, &self.level)?;
        self.state = Some(next);
        Ok(())
    }
    /// Renders the simulation's current (observed) tic into `out`. On success the render
    /// state's framebuffer, `fuzzpos` and mapped set advance; on refusal the render state is
    /// unchanged and `out` holds the partially drawn frame (unspecified).
    pub fn render(
        &mut self,
        sim: &Simulation,
        out: &mut Frame,
    ) -> Result<FrameReport, RenderRefusal> {
        let state = self.state.as_ref().ok_or(RenderRefusal::NotObserved {
            observed: None,
            simulation: sim.tic(),
        })?;
        if state.tic != sim.tic() {
            return Err(RenderRefusal::NotObserved {
                observed: Some(state.tic),
                simulation: sim.tic(),
            });
        }
        let player = sim
            .original_player(PlayerSlot::One)
            .map_err(|_| RenderRefusal::NoPlayer)?;
        let mo = player
            .mo
            .and_then(|id| sim.actor(id))
            .ok_or(RenderRefusal::NoPlayer)?;
        let weapons = sim
            .original_player_weapons(PlayerSlot::One)
            .ok()
            .flatten()
            .ok_or(RenderRefusal::NoPlayer)?;
        let [x, y] = mo.position().map(|f| f.bits());
        let frame = r_main::FrameView::setup(
            x,
            y,
            mo.angle(),
            weapons.extra_light,
            state.view.viewz,
            state.view.fixedcolormap,
        );
        out.pixels.copy_from_slice(&state.screen);
        let mut taint = state.taint.clone();
        if let Some(t) = self.trace.as_mut() {
            t.fill(Feature::Untouched);
        }
        let mut ctx = FrameContext {
            res: &self.res,
            level: &self.level,
            view: &self.view,
            frame,
            screen: &mut out.pixels,
            taint: &mut taint,
            trace: self.trace.as_deref_mut(),
        };
        r_bsp::render_bsp_node(&mut ctx)?;
        // Steps 2b/2c: R_DrawPlanes, R_DrawMasked, then commit the frame state.
        Err(RenderRefusal::NotImplemented("R_DrawPlanes"))
    }
}
