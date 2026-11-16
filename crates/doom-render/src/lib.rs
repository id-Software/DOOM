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
//! Status (M5a step 2c): the whole of `R_RenderPlayerView` at high detail is drawn (BSP,
//! solid and two-sided walls, visplanes, sky, lighting and fixed colour maps, masked
//! midtextures, sprites with drawseg clipping, weapon sprites and fuzz); low detail and
//! player colour translation are refused (decisions R1, R2).
use std::fmt;
use std::sync::Arc;

pub mod gif;
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
    /// `R_DrawTranslatedColumn` reached: player colour translation (`MF_TRANSLATION`,
    /// r_things.c:417-422) belongs to players 2-4 only and is refused (decision R1).
    PlayerTranslation,
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
    /// More than `MAXVISPLANES` visplanes: `R_FindPlane`'s `I_Error` (r_plane.c:245-246) or
    /// `R_CheckPlane`'s unchecked write past `visplanes[]` (r_plane.c:313-317) (decision R3).
    VisplaneOverflow,
    /// The first write past `openings[MAXOPENINGS]` (r_segs.c:356, 611-612, 721-731; decision R3).
    OpeningOverflow,
    /// The first access, read or write, past `solidsegs[MAXSEGS]` (r_bsp.c:122-133, 178-182;
    /// decision R3 as applied by the orchestrator and accepted by the owner on 2026-11-15).
    SolidSegOverflow,
    /// A masked midtexture column that is a multi-patch composite: the genuine post walk
    /// (r_segs.c:181-184, r_things.c:359-385) finds no 0xff terminator inside the composite and
    /// runs into zone memory (orchestrator application of R3, 2026-11-16).
    MaskedCompositeColumn,
    /// A masked midtexture column whose post walk would leave its patch lump, or whose lump and
    /// offset `R_GenerateLookup` left uninitialised (r_data.c:353-358): layout-dependent.
    MaskedColumnUndefined,
    /// One of the genuine `RANGECHECK` `I_Error`s (named by function), or a draw outside the
    /// view rectangle that the genuine check lets through (decision R3).
    RangeCheck(&'static str),
    /// A table read outside the genuine contiguous tables (decision R12), such as
    /// `tantoangle[FixedDiv(0,0)>>DBITS]` for a viewer exactly on a seg's start vertex
    /// (r_main.c:412).
    TableRead(&'static str),
    /// The genuine frame would read a renderer global left over from an earlier wall or frame
    /// (`pixhigh`/`pixlow`, r_segs.c:694-704; investigation §1.11 hazard 8).
    StaleGlobal(&'static str),
    /// A canonical field the renderer reads is missing or malformed.
    Snapshot(String),
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
    /// Lines newly marked mapped by this frame (`ML_MAPPED`, decision R5), ascending.
    pub newly_mapped: Vec<u16>,
    /// `fuzzpos` after the frame (r_draw.c:274, 361-362).
    pub fuzzpos: u32,
    /// The frame's renderer counters, as the genuine runner reports them per frame.
    pub counters: FrameCounters,
}

/// The per-frame counters of the genuine M5a runner's F rows (`tools/reference/m5a_driver.c`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameCounters {
    /// Lines with `ML_MAPPED` after the frame.
    pub mapped: u32,
    /// `lastvisplane - visplanes`.
    pub visplanes: u32,
    /// `ds_p - drawsegs`.
    pub drawsegs: u32,
    /// `vissprite_p - vissprites`.
    pub vissprites: u32,
    /// `lastopening - openings`.
    pub openings: u32,
    /// The largest `newend - solidsegs` seen at any `R_StoreWallRange` call or the frame end.
    pub solidsegs_peak: u32,
    /// `R_StoreWallRange` calls dropped at `MAXDRAWSEGS` (r_segs.c:385-387).
    pub drawseg_drops: u32,
    /// Whether `R_NewVisSprite` returned `overflowsprite` and it was written (r_things.c:330-331).
    pub vissprite_drop: bool,
}

/// The renderer stage, and the object of that stage, that wrote a framebuffer pixel in the
/// last frame (first-difference reporting only; never part of the comparison or the render
/// state).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Feature {
    /// Not written by the last frame (the persistent framebuffer's earlier contents).
    #[default]
    Untouched,
    /// A solid or two-sided wall column (r_segs.c `R_RenderSegLoop`) of seg `seg`.
    Wall {
        /// The seg (`segs[]` index).
        seg: u32,
    },
    /// A floor or ceiling span (r_plane.c `R_MapPlane`) of visplane `visplane`.
    Plane {
        /// The visplane (`visplanes[]` index in the frame).
        visplane: u16,
    },
    /// A sky column (r_plane.c `R_DrawPlanes`) of visplane `visplane`.
    Sky {
        /// The visplane.
        visplane: u16,
    },
    /// A masked midtexture column (r_segs.c `R_RenderMaskedSegRange`) of seg `seg`.
    MaskedMid {
        /// The seg.
        seg: u32,
    },
    /// A sprite column (r_things.c `R_DrawVisSprite` from `R_DrawSprite`).
    Sprite {
        /// The vissprite (`vissprites[]` index: projection order).
        vissprite: u16,
        /// Its sprite number (`sprnames` order).
        sprite: u16,
        /// Its frame (`frame & FF_FRAMEMASK`).
        frame: u16,
    },
    /// A weapon sprite column (r_things.c `R_DrawPSprite`): 0 weapon, 1 flash.
    PSprite {
        /// `psprites[]` index.
        psprite: u8,
        /// Its sprite number.
        sprite: u16,
        /// Its frame.
        frame: u16,
    },
    /// A fuzz column (r_draw.c `R_DrawFuzzColumn`) of a shadow sprite (`psprite` false:
    /// `vissprite` is the `vissprites[]` index) or of the weapon sprites (`psprite` true:
    /// `vissprite` is the `psprites[]` index).
    Fuzz {
        /// Whether it is a weapon sprite.
        psprite: bool,
        /// The vissprite or psprite index.
        vissprite: u16,
    },
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
    /// The BSP geometry as `P_LoadVertexes`, `P_LoadSegs`, `P_LoadSubsectors` and
    /// `P_LoadNodes` build it (16.16 coordinates, resolved references).
    pub(crate) geometry: r_bsp::Geometry,
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
            geometry: r_bsp::Geometry::load(map)?,
            map: map.clone(),
            side_textures,
            sector_flats,
            switch_lines,
        })
    }
}

/// A sector as the renderer reads it in one frame (`sector_t` heights, pics and light).
#[derive(Clone, Copy, Debug)]
pub(crate) struct SectorView {
    pub floorheight: i32,
    pub ceilingheight: i32,
    pub floorpic: usize,
    pub ceilingpic: usize,
    pub lightlevel: i32,
}

/// Per-frame context handed to the renderer stages: the frame's inputs and the renderer
/// globals of r_bsp.c, r_segs.c, r_plane.c and r_things.c (one field group per C file).
pub(crate) struct FrameContext<'a> {
    pub res: &'a Resources,
    pub level: &'a Level,
    pub view: &'a r_main::ViewSize,
    pub frame: r_main::FrameView,
    pub sim: &'a Simulation,
    pub screen: &'a mut [u8],
    pub taint: &'a mut [u32],
    pub trace: Option<&'a mut [Feature]>,
    /// The mapped-line set (`ML_MAPPED`, decision R5), updated in place.
    pub mapped: &'a mut [bool],
    pub newly_mapped: Vec<u16>,
    /// `fuzzpos` (r_draw.c:274), carried from frame to frame (decision R8).
    pub fuzzpos: usize,
    pub sectors: Vec<SectorView>,
    /// Per sidedef `[top, mid, bottom]` after the tracked switch changes.
    pub sides: Vec<[usize; 3]>,
    /// Per sidedef `textureoffset` (16.16) after the scrolling walls.
    pub textureoffset: Vec<i32>,
    pub flattranslation: Vec<usize>,
    pub texturetranslation: Vec<usize>,
    pub bsp: r_bsp::BspState,
    pub segs: r_segs::SegState,
    pub planes: r_plane::PlaneState,
    pub things: r_things::ThingState,
}

impl FrameContext<'_> {
    /// The colour-map byte `colormaps[map*256 + index]`.
    pub(crate) fn colormap(&self, map: u8, index: u8) -> u8 {
        self.res.colormaps[usize::from(map) * 256 + usize::from(index)]
    }
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
        let sectors = sector_views(sim, &self.res, &self.level)?;
        let (flattranslation, texturetranslation) = p_spec::translations(&self.res, sim.tic());
        let sides = p_spec::side_textures(
            &self.res,
            &self.level.side_textures,
            self.level.map.linedefs(),
            &state.switches,
        );
        let textureoffset = p_spec::texture_offsets(&self.level.map, sim.tic());
        out.pixels.copy_from_slice(&state.screen);
        let mut taint = state.taint.clone();
        let mut mapped = state.mapped.clone();
        let switch_parity_unknown = state.switches.parity_unknown();
        if let Some(t) = self.trace.as_mut() {
            t.fill(Feature::Untouched);
        }
        let nsectors = sectors.len();
        let mut ctx = FrameContext {
            res: &self.res,
            level: &self.level,
            view: &self.view,
            frame,
            sim,
            screen: &mut out.pixels,
            taint: &mut taint,
            trace: self.trace.as_deref_mut(),
            mapped: &mut mapped,
            newly_mapped: Vec::new(),
            fuzzpos: state.fuzzpos as usize,
            sectors,
            sides,
            textureoffset,
            flattranslation,
            texturetranslation,
            bsp: r_bsp::BspState::default(),
            segs: r_segs::SegState::new(frame.fixedcolormap),
            planes: r_plane::PlaneState::default(),
            things: r_things::ThingState::new(nsectors),
        };
        r_main::render_player_view(&mut ctx, &weapons)?;
        let counters = FrameCounters {
            mapped: ctx.mapped.iter().filter(|m| **m).count() as u32,
            visplanes: ctx.planes.visplanes.len() as u32,
            drawsegs: ctx.bsp.drawsegs.len() as u32,
            vissprites: ctx.things.vissprites.len() as u32,
            openings: ctx.planes.lastopening as u32,
            solidsegs_peak: ctx.bsp.solid_peak as u32,
            drawseg_drops: ctx.bsp.drawseg_drops,
            vissprite_drop: ctx.things.vissprite_drop,
        };
        let mut newly_mapped = std::mem::take(&mut ctx.newly_mapped);
        newly_mapped.sort_unstable();
        let fuzzpos = ctx.fuzzpos as u32;
        drop(ctx);
        let flagged_columns: Vec<u16> = (0..SCREENWIDTH)
            .filter(|x| (0..SCREENHEIGHT).any(|y| taint[y * SCREENWIDTH + x] != 0))
            .map(|x| x as u16)
            .collect();
        let state = self.state.as_mut().expect("checked above");
        state.screen.copy_from_slice(&out.pixels);
        state.taint = taint;
        state.mapped = mapped;
        state.fuzzpos = fuzzpos;
        Ok(FrameReport {
            flagged_columns,
            switch_parity_unknown,
            newly_mapped,
            fuzzpos,
            counters,
        })
    }
}

/// Every sector as `R_Subsector` and `R_DrawPlanes` read it after the simulation's tic: the
/// runtime heights, the runtime `floorpic` (`sector.{i}.floorpic` of the snapshot, decision
/// D2's normalized name resolved with `R_FlatNumForName`), the map `ceilingpic` (no admitted
/// special writes it, investigation §4.1.5) and the runtime light level.
fn sector_views(
    sim: &Simulation,
    res: &Resources,
    level: &Level,
) -> Result<Vec<SectorView>, RenderRefusal> {
    let map_sectors = level.map.sectors();
    let runtime = sim.sectors();
    if runtime.len() != map_sectors.len() {
        return Err(RenderRefusal::Snapshot("sector count".into()));
    }
    let mut floorpics: Vec<Option<[u8; 8]>> = vec![None; map_sectors.len()];
    for f in sim.snapshot().fields() {
        let Some(rest) = f.path().strip_prefix("sector.") else {
            continue;
        };
        let Some(i) = rest.strip_suffix(".floorpic") else {
            continue;
        };
        let i: usize = i
            .parse()
            .map_err(|_| RenderRefusal::Snapshot(f.path().into()))?;
        let name: [u8; 8] = f
            .value()
            .try_into()
            .map_err(|_| RenderRefusal::Snapshot(f.path().into()))?;
        *floorpics
            .get_mut(i)
            .ok_or_else(|| RenderRefusal::Snapshot(f.path().into()))? = Some(name);
    }
    let lights: Vec<i32> = match sim.sector_lights_and_specials() {
        Some(it) => it.map(|(l, _)| i32::from(l)).collect(),
        None => map_sectors.iter().map(|s| i32::from(s.light)).collect(),
    };
    if lights.len() != map_sectors.len() {
        return Err(RenderRefusal::Snapshot("sector lights".into()));
    }
    let mut out = Vec::with_capacity(map_sectors.len());
    for (i, s) in map_sectors.iter().enumerate() {
        let floorpic = match floorpics[i] {
            Some(name) if r_data::c_name(&name) != r_data::c_name(s.flats[0].as_bytes()) => res
                .flat_num_for_name(&name)
                .map_err(|e| RenderRefusal::Snapshot(format!("sector.{i}.floorpic: {e:?}")))?,
            Some(_) => level.sector_flats[i][0],
            None => return Err(RenderRefusal::Snapshot(format!("sector.{i}.floorpic"))),
        };
        out.push(SectorView {
            floorheight: runtime[i].floor.bits(),
            ceilingheight: runtime[i].ceiling.bits(),
            floorpic,
            ceilingpic: level.sector_flats[i][1],
            lightlevel: lights[i],
        });
    }
    Ok(out)
}
