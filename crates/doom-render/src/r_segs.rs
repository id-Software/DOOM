//! r_segs.c: `R_StoreWallRange`, `R_RenderSegLoop`, `R_RenderMaskedSegRange`, drawsegs and
//! the openings they allocate, with `R_ScaleFromGlobalAngle` (r_main.c:453-502).
use crate::r_data::MaskedPosts;
use crate::r_draw::ColumnDraw;
use crate::r_main::{
    ANG90, ANG180, ANGLETOFINESHIFT, LIGHTLEVELS, LIGHTSCALESHIFT, MAXLIGHTSCALE, finesine,
    fixed_div, fixed_mul, point_to_dist,
};
use crate::r_plane::MAXOPENINGS;
use crate::tables::FINE_TANGENT_SINE;
use crate::{Feature, FrameContext, RenderRefusal};

/// `MAXDRAWSEGS` (r_defs.h): further `R_StoreWallRange` calls are dropped silently
/// (r_segs.c:386-387), as the genuine engine does (decision R3).
pub const MAXDRAWSEGS: usize = 256;
/// `HEIGHTBITS` (r_segs.c:203).
const HEIGHTBITS: u32 = 12;
/// `HEIGHTUNIT` (r_segs.c:204).
const HEIGHTUNIT: i32 = 1 << HEIGHTBITS;
/// `LIGHTSEGSHIFT` (r_main.h:70).
pub(crate) const LIGHTSEGSHIFT: u32 = 4;
/// `ML_DONTPEGTOP` (doomdata.h:126).
const ML_DONTPEGTOP: u16 = 8;
/// `ML_DONTPEGBOTTOM` (doomdata.h:129).
const ML_DONTPEGBOTTOM: u16 = 16;
/// `SIL_BOTTOM` (r_defs.h:57).
pub const SIL_BOTTOM: i32 = 1;
/// `SIL_TOP` (r_defs.h:58).
pub const SIL_TOP: i32 = 2;
/// `SIL_BOTH` (r_defs.h:59).
pub const SIL_BOTH: i32 = 3;
/// `MAXSHORT` (doomtype.h:45): the "already drawn" mark of `maskedtexturecol`.
const MAXSHORT: i16 = 0x7fff;

/// A sprite clip array of a drawseg (`sprtopclip`/`sprbottomclip`, r_defs.h:338-340).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Clip {
    /// NULL.
    None,
    /// `screenheightarray` (every entry `viewheight`).
    ScreenHeight,
    /// `negonearray` (every entry -1).
    NegOne,
    /// `openings + base` (indexed by screen x).
    Openings(i32),
    /// `R_DrawSprite`'s local `clipbot` (r_things.c:845), as `mfloorclip`.
    SpriteBottom,
    /// `R_DrawSprite`'s local `cliptop` (r_things.c:846), as `mceilingclip`.
    SpriteTop,
}

/// `drawseg_t` (r_defs.h:322-345).
#[derive(Clone, Copy, Debug)]
pub(crate) struct DrawSeg {
    pub curline: usize,
    pub x1: i32,
    pub x2: i32,
    pub scale1: i32,
    pub scale2: i32,
    /// `scalestep`. For a single-column range the genuine field is not written
    /// (r_segs.c:443-446) and keeps an earlier value; every reader multiplies it by 0 there
    /// (r_segs.c:139, the only use before the loop) or uses it after the last column.
    pub scalestep: i32,
    pub silhouette: i32,
    pub bsilheight: i32,
    pub tsilheight: i32,
    pub sprtopclip: Clip,
    pub sprbottomclip: Clip,
    /// `maskedtexturecol` as `openings + base` (NULL: None).
    pub maskedtexturecol: Option<i32>,
}

/// `walllights`: a row of `scalelight`, or `scalelightfixed` (every entry the fixed map).
#[derive(Clone, Copy, Debug)]
pub(crate) enum WallLights {
    Scale(usize),
    Fixed(u8),
}

/// The r_segs.c globals (r_segs.c:46-95).
#[derive(Clone, Debug)]
pub(crate) struct SegState {
    pub segtextured: bool,
    pub markfloor: bool,
    pub markceiling: bool,
    pub maskedtexture: bool,
    pub toptexture: usize,
    pub bottomtexture: usize,
    pub midtexture: usize,
    pub rw_normalangle: u32,
    pub rw_angle1: i32,
    pub rw_x: i32,
    pub rw_stopx: i32,
    pub rw_centerangle: u32,
    pub rw_offset: i32,
    pub rw_distance: i32,
    pub rw_scale: i32,
    pub rw_scalestep: i32,
    pub rw_midtexturemid: i32,
    pub rw_toptexturemid: i32,
    pub rw_bottomtexturemid: i32,
    pub worldtop: i32,
    pub worldbottom: i32,
    pub worldhigh: i32,
    pub worldlow: i32,
    pub pixhigh: i32,
    pub pixlow: i32,
    pub pixhighstep: i32,
    pub pixlowstep: i32,
    pub topfrac: i32,
    pub topstep: i32,
    pub bottomfrac: i32,
    pub bottomstep: i32,
    pub walllights: WallLights,
    /// `maskedtexturecol` as `openings + base`.
    pub maskedtexturecol: i32,
    /// `texturecolumn` of `R_RenderSegLoop` (kept across columns, as the C local is only
    /// assigned when `segtextured`).
    texturecolumn: i32,
}

impl SegState {
    /// The globals at the start of a frame: `R_SetupFrame` sets `walllights =
    /// scalelightfixed` when a fixed colour map is active (r_main.c:853-856); otherwise every
    /// textured seg sets it before use (r_segs.c:642-657). `pixhigh`, `pixlow` and their
    /// steps are file-scope globals that are never reset (r_segs.c:81-84): they carry over
    /// from the previous frame (`carry`).
    pub(crate) fn new(fixedcolormap: i32, carry: crate::WallSteps) -> Self {
        Self {
            segtextured: false,
            markfloor: false,
            markceiling: false,
            maskedtexture: false,
            toptexture: 0,
            bottomtexture: 0,
            midtexture: 0,
            rw_normalangle: 0,
            rw_angle1: 0,
            rw_x: 0,
            rw_stopx: 0,
            rw_centerangle: 0,
            rw_offset: 0,
            rw_distance: 0,
            rw_scale: 0,
            rw_scalestep: 0,
            rw_midtexturemid: 0,
            rw_toptexturemid: 0,
            rw_bottomtexturemid: 0,
            worldtop: 0,
            worldbottom: 0,
            worldhigh: 0,
            worldlow: 0,
            pixhigh: carry.pixhigh,
            pixlow: carry.pixlow,
            pixhighstep: carry.pixhighstep,
            pixlowstep: carry.pixlowstep,
            topfrac: 0,
            topstep: 0,
            bottomfrac: 0,
            bottomstep: 0,
            walllights: if fixedcolormap != 0 {
                WallLights::Fixed(fixedcolormap as u8)
            } else {
                WallLights::Scale(0)
            },
            maskedtexturecol: 0,
            texturecolumn: 0,
        }
    }
}

impl FrameContext<'_> {
    /// The `scalelight` row for a seg's light (r_segs.c:644-656, 122-134): the sector light
    /// plus `extralight`, one darker for a horizontal seg and one brighter for a vertical one
    /// (fake contrast), clamped to the table.
    fn seg_light_row(&self, sector: usize, seg: usize) -> usize {
        let s = &self.level.geometry.segs[seg];
        let mut lightnum =
            (self.sectors[sector].lightlevel >> LIGHTSEGSHIFT) + self.frame.extralight;
        if s.v1[1] == s.v2[1] {
            lightnum -= 1;
        } else if s.v1[0] == s.v2[0] {
            lightnum += 1;
        }
        lightnum.clamp(0, LIGHTLEVELS as i32 - 1) as usize
    }

    /// The colour map `walllights[index]`.
    fn wall_light(&self, lights: WallLights, index: usize) -> u8 {
        match lights {
            WallLights::Fixed(m) => m,
            WallLights::Scale(row) => self.view.scalelight[row][index],
        }
    }

    /// `R_ScaleFromGlobalAngle` (r_main.c:453-502). `anglea`/`angleb` are `int` and shifted
    /// arithmetically, so an angle sum at or past `ANG180` indexes `finesine` negatively and
    /// reads `finetangent` (the genuine contiguous layout; decision R12).
    fn scale_from_global_angle(&self, visangle: u32) -> Result<i32, RenderRefusal> {
        let f = self.frame;
        let anglea = ANG90.wrapping_add(visangle.wrapping_sub(f.viewangle)) as i32;
        let angleb = ANG90.wrapping_add(visangle.wrapping_sub(self.segs.rw_normalangle)) as i32;
        let sinea = finesine(anglea >> ANGLETOFINESHIFT).ok_or(RenderRefusal::TableRead(
            "finesine (R_ScaleFromGlobalAngle)",
        ))?;
        let sineb = finesine(angleb >> ANGLETOFINESHIFT).ok_or(RenderRefusal::TableRead(
            "finesine (R_ScaleFromGlobalAngle)",
        ))?;
        let num = fixed_mul(self.view.projection, sineb).wrapping_shl(self.view.detailshift);
        let den = fixed_mul(self.segs.rw_distance, sinea);
        Ok(if den > num >> 16 {
            // r_main.c:493-496 (the upper test first; the bounds do not overlap).
            fixed_div(num, den)?.clamp(256, 64 << 16)
        } else {
            64 << 16
        })
    }

    /// `xtoviewangle[x]`.
    fn xtoviewangle(&self, x: i32) -> Result<u32, RenderRefusal> {
        usize::try_from(x)
            .ok()
            .and_then(|x| self.view.xtoviewangle.get(x))
            .copied()
            .ok_or(RenderRefusal::RangeCheck("xtoviewangle"))
    }

    /// Writes `openings[index]`; past `MAXOPENINGS` is the `OpeningOverflow` refusal (R3).
    fn write_opening(&mut self, index: i32, value: i16) -> Result<(), RenderRefusal> {
        let slot = usize::try_from(index)
            .ok()
            .filter(|i| *i < MAXOPENINGS)
            .ok_or(RenderRefusal::OpeningOverflow)?;
        self.planes.openings[slot] = value;
        Ok(())
    }

    /// Reads `openings[index]`.
    pub(crate) fn read_opening(&self, index: i32) -> Result<i16, RenderRefusal> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.planes.openings.get(i))
            .copied()
            .ok_or(RenderRefusal::OpeningOverflow)
    }

    /// `R_StoreWallRange` (r_segs.c:374-745).
    pub(crate) fn store_wall_range(&mut self, start: i32, stop: i32) -> Result<(), RenderRefusal> {
        self.note_solid_peak();
        // Don't overflow and crash (r_segs.c:385-387): a silent drop.
        if self.bsp.drawsegs.len() == MAXDRAWSEGS {
            self.bsp.drawseg_drops += 1;
            return Ok(());
        }
        // RANGECHECK (r_segs.c:389-392).
        if start >= self.view.viewwidth || start > stop {
            return Err(RenderRefusal::RangeCheck("R_StoreWallRange"));
        }
        let level = self.level;
        let curline = self.bsp.curline;
        let seg = level.geometry.segs[curline];
        let sidedef = seg.sidedef;
        let linedef = seg.linedef;
        let line_flags = level.geometry.line_flags[linedef];
        let side_tex = self.sides[sidedef];
        let frontsector = self.sectors[self.bsp.frontsector];
        let backsector = self.bsp.backsector.map(|b| self.sectors[b]);
        let f = self.frame;
        let viewz = f.viewz;
        // Mark the segment as visible for the automap (r_segs.c:398; decision R5).
        if !self.mapped[linedef] {
            self.mapped[linedef] = true;
            self.newly_mapped.push(linedef as u16);
        }
        // rw_distance for scale calculation (r_segs.c:401-410).
        self.segs.rw_normalangle = seg.angle.wrapping_add(ANG90);
        let mut offsetangle = (self
            .segs
            .rw_normalangle
            .wrapping_sub(self.segs.rw_angle1 as u32) as i32)
            .wrapping_abs() as u32;
        if offsetangle > ANG90 {
            offsetangle = ANG90;
        }
        let distangle = ANG90 - offsetangle;
        let hyp = point_to_dist(seg.v1[0], seg.v1[1], f.viewx, f.viewy)?;
        let sineval = finesine((distangle >> ANGLETOFINESHIFT) as i32)
            .ok_or(RenderRefusal::TableRead("finesine (R_StoreWallRange)"))?;
        self.segs.rw_distance = fixed_mul(hyp, sineval);
        // r_segs.c:413-416
        self.segs.rw_x = start;
        self.segs.rw_stopx = stop.wrapping_add(1);
        // Scale at both ends and step (r_segs.c:419-446).
        let scale1 =
            self.scale_from_global_angle(f.viewangle.wrapping_add(self.xtoviewangle(start)?))?;
        self.segs.rw_scale = scale1;
        let scale2 = if stop > start {
            let scale2 =
                self.scale_from_global_angle(f.viewangle.wrapping_add(self.xtoviewangle(stop)?))?;
            self.segs.rw_scalestep = scale2.wrapping_sub(scale1) / (stop - start);
            scale2
        } else {
            scale1
        };
        let mut ds = DrawSeg {
            curline,
            x1: start,
            x2: stop,
            scale1,
            scale2,
            scalestep: self.segs.rw_scalestep,
            silhouette: 0,
            bsilheight: 0,
            tsilheight: 0,
            sprtopclip: Clip::None,
            sprbottomclip: Clip::None,
            maskedtexturecol: None,
        };
        // Texture boundaries and plane marks (r_segs.c:450-454).
        self.segs.worldtop = frontsector.ceilingheight.wrapping_sub(viewz);
        self.segs.worldbottom = frontsector.floorheight.wrapping_sub(viewz);
        self.segs.midtexture = 0;
        self.segs.toptexture = 0;
        self.segs.bottomtexture = 0;
        self.segs.maskedtexture = false;
        let rowoffset = level.geometry.rowoffset[sidedef];
        let textureheight = |t: usize| self.res.textures[t].textureheight;
        match backsector {
            None => {
                // Single sided line (r_segs.c:456-481).
                self.segs.midtexture = self.texturetranslation[side_tex[1]];
                self.segs.markfloor = true;
                self.segs.markceiling = true;
                self.segs.rw_midtexturemid = if line_flags & ML_DONTPEGBOTTOM != 0 {
                    // Bottom of texture at bottom; the untranslated texture's height.
                    frontsector
                        .floorheight
                        .wrapping_add(textureheight(side_tex[1]))
                        .wrapping_sub(viewz)
                } else {
                    // Top of texture at top.
                    self.segs.worldtop
                };
                self.segs.rw_midtexturemid = self.segs.rw_midtexturemid.wrapping_add(rowoffset);
                ds.silhouette = SIL_BOTH;
                ds.sprtopclip = Clip::ScreenHeight;
                ds.sprbottomclip = Clip::NegOne;
                ds.bsilheight = i32::MAX;
                ds.tsilheight = i32::MIN;
            }
            Some(back) => {
                // Two sided line (r_segs.c:482-614).
                ds.sprtopclip = Clip::None;
                ds.sprbottomclip = Clip::None;
                ds.silhouette = 0;
                if frontsector.floorheight > back.floorheight {
                    ds.silhouette = SIL_BOTTOM;
                    ds.bsilheight = frontsector.floorheight;
                } else if back.floorheight > viewz {
                    ds.silhouette = SIL_BOTTOM;
                    ds.bsilheight = i32::MAX;
                }
                if frontsector.ceilingheight < back.ceilingheight {
                    ds.silhouette |= SIL_TOP;
                    ds.tsilheight = frontsector.ceilingheight;
                } else if back.ceilingheight < viewz {
                    ds.silhouette |= SIL_TOP;
                    ds.tsilheight = i32::MIN;
                }
                if back.ceilingheight <= frontsector.floorheight {
                    ds.sprbottomclip = Clip::NegOne;
                    ds.bsilheight = i32::MAX;
                    ds.silhouette |= SIL_BOTTOM;
                }
                if back.floorheight >= frontsector.ceilingheight {
                    ds.sprtopclip = Clip::ScreenHeight;
                    ds.tsilheight = i32::MIN;
                    ds.silhouette |= SIL_TOP;
                }
                self.segs.worldhigh = back.ceilingheight.wrapping_sub(viewz);
                self.segs.worldlow = back.floorheight.wrapping_sub(viewz);
                // Hack to allow height changes in outdoor areas (r_segs.c:530-534).
                let sky = self.res.skyflatnum;
                if sky == Some(frontsector.ceilingpic) && sky == Some(back.ceilingpic) {
                    self.segs.worldtop = self.segs.worldhigh;
                }
                // r_segs.c:537-560
                self.segs.markfloor = self.segs.worldlow != self.segs.worldbottom
                    || back.floorpic != frontsector.floorpic
                    || back.lightlevel != frontsector.lightlevel;
                self.segs.markceiling = self.segs.worldhigh != self.segs.worldtop
                    || back.ceilingpic != frontsector.ceilingpic
                    || back.lightlevel != frontsector.lightlevel;
                if back.ceilingheight <= frontsector.floorheight
                    || back.floorheight >= frontsector.ceilingheight
                {
                    // Closed door (r_segs.c:562-567).
                    self.segs.markceiling = true;
                    self.segs.markfloor = true;
                }
                if self.segs.worldhigh < self.segs.worldtop {
                    // Top texture (r_segs.c:570-588).
                    self.segs.toptexture = self.texturetranslation[side_tex[0]];
                    self.segs.rw_toptexturemid = if line_flags & ML_DONTPEGTOP != 0 {
                        self.segs.worldtop
                    } else {
                        back.ceilingheight
                            .wrapping_add(textureheight(side_tex[0]))
                            .wrapping_sub(viewz)
                    };
                }
                if self.segs.worldlow > self.segs.worldbottom {
                    // Bottom texture (r_segs.c:589-602).
                    self.segs.bottomtexture = self.texturetranslation[side_tex[2]];
                    self.segs.rw_bottomtexturemid = if line_flags & ML_DONTPEGBOTTOM != 0 {
                        self.segs.worldtop
                    } else {
                        self.segs.worldlow
                    };
                }
                // r_segs.c:603-604 (unconditional, so a stale mid also moves).
                self.segs.rw_toptexturemid = self.segs.rw_toptexturemid.wrapping_add(rowoffset);
                self.segs.rw_bottomtexturemid =
                    self.segs.rw_bottomtexturemid.wrapping_add(rowoffset);
                // Masked midtexture: allocate its column table (r_segs.c:607-613).
                if side_tex[1] != 0 {
                    self.segs.maskedtexture = true;
                    let base = self.planes.lastopening as i32 - self.segs.rw_x;
                    self.segs.maskedtexturecol = base;
                    ds.maskedtexturecol = Some(base);
                    self.planes.lastopening += (self.segs.rw_stopx - self.segs.rw_x) as usize;
                }
            }
        }
        // rw_offset (r_segs.c:617-658).
        self.segs.segtextured = self.segs.midtexture != 0
            || self.segs.toptexture != 0
            || self.segs.bottomtexture != 0
            || self.segs.maskedtexture;
        if self.segs.segtextured {
            let diff = self
                .segs
                .rw_normalangle
                .wrapping_sub(self.segs.rw_angle1 as u32);
            let mut offsetangle = diff;
            if offsetangle > ANG180 {
                offsetangle = offsetangle.wrapping_neg();
            }
            if offsetangle > ANG90 {
                offsetangle = ANG90;
            }
            let sineval = finesine((offsetangle >> ANGLETOFINESHIFT) as i32)
                .ok_or(RenderRefusal::TableRead("finesine (rw_offset)"))?;
            self.segs.rw_offset = fixed_mul(hyp, sineval);
            if diff < ANG180 {
                self.segs.rw_offset = self.segs.rw_offset.wrapping_neg();
            }
            self.segs.rw_offset = self
                .segs
                .rw_offset
                .wrapping_add(self.textureoffset[sidedef].wrapping_add(seg.offset));
            self.segs.rw_centerangle = ANG90
                .wrapping_add(f.viewangle)
                .wrapping_sub(self.segs.rw_normalangle);
            // Light table: fake contrast (r_segs.c:642-657).
            if f.fixedcolormap == 0 {
                self.segs.walllights =
                    WallLights::Scale(self.seg_light_row(self.bsp.frontsector, curline));
            }
        }
        // Planes on the wrong side of the view plane are not marked (r_segs.c:665-676).
        if frontsector.floorheight >= viewz {
            self.segs.markfloor = false;
        }
        if frontsector.ceilingheight <= viewz && self.res.skyflatnum != Some(frontsector.ceilingpic)
        {
            self.segs.markceiling = false;
        }
        // Incremental stepping values for texture edges (r_segs.c:680-704).
        let centeryfrac4 = self.view.centeryfrac >> 4;
        let s = &mut self.segs;
        s.worldtop >>= 4;
        s.worldbottom >>= 4;
        s.topstep = fixed_mul(s.rw_scalestep, s.worldtop).wrapping_neg();
        s.topfrac = centeryfrac4.wrapping_sub(fixed_mul(s.worldtop, s.rw_scale));
        s.bottomstep = fixed_mul(s.rw_scalestep, s.worldbottom).wrapping_neg();
        s.bottomfrac = centeryfrac4.wrapping_sub(fixed_mul(s.worldbottom, s.rw_scale));
        if backsector.is_some() {
            s.worldhigh >>= 4;
            s.worldlow >>= 4;
            let high = s.worldhigh < s.worldtop;
            let low = s.worldlow > s.worldbottom;
            if high {
                s.pixhigh = centeryfrac4.wrapping_sub(fixed_mul(s.worldhigh, s.rw_scale));
                s.pixhighstep = fixed_mul(s.rw_scalestep, s.worldhigh).wrapping_neg();
            }
            if low {
                s.pixlow = centeryfrac4.wrapping_sub(fixed_mul(s.worldlow, s.rw_scale));
                s.pixlowstep = fixed_mul(s.rw_scalestep, s.worldlow).wrapping_neg();
            }
            // The textures were chosen on the unshifted heights (r_segs.c:570, 589): a top or
            // bottom texture whose shifted test fails draws with the pixhigh/pixlow an earlier
            // seg (of this frame or an earlier one) left, as the genuine globals do (hazard 8
            // of investigation §1.11; carried in the render state).
        }
        // Render it (r_segs.c:708-714).
        let (rw_x, rw_stopx) = (self.segs.rw_x, self.segs.rw_stopx);
        if self.segs.markceiling {
            let pl = self.planes.ceilingplane.ok_or(RenderRefusal::RangeCheck(
                "R_CheckPlane (NULL ceilingplane)",
            ))?;
            self.planes.ceilingplane = Some(self.check_plane(pl, rw_x, rw_stopx - 1)?);
        }
        if self.segs.markfloor {
            let pl = self
                .planes
                .floorplane
                .ok_or(RenderRefusal::RangeCheck("R_CheckPlane (NULL floorplane)"))?;
            self.planes.floorplane = Some(self.check_plane(pl, rw_x, rw_stopx - 1)?);
        }
        self.render_seg_loop()?;
        // Save sprite clipping info (r_segs.c:718-732).
        let maskedtexture = self.segs.maskedtexture;
        let n = rw_stopx - start;
        if (ds.silhouette & SIL_TOP != 0 || maskedtexture) && ds.sprtopclip == Clip::None {
            let base = self.copy_clip(start, n, true)?;
            ds.sprtopclip = Clip::Openings(base);
        }
        if (ds.silhouette & SIL_BOTTOM != 0 || maskedtexture) && ds.sprbottomclip == Clip::None {
            let base = self.copy_clip(start, n, false)?;
            ds.sprbottomclip = Clip::Openings(base);
        }
        // r_segs.c:734-743
        if maskedtexture && ds.silhouette & SIL_TOP == 0 {
            ds.silhouette |= SIL_TOP;
            ds.tsilheight = i32::MIN;
        }
        if maskedtexture && ds.silhouette & SIL_BOTTOM == 0 {
            ds.silhouette |= SIL_BOTTOM;
            ds.bsilheight = i32::MAX;
        }
        self.bsp.drawsegs.push(ds);
        Ok(())
    }

    /// `memcpy (lastopening, ceilingclip+start, 2*n)` (or `floorclip`), returning
    /// `lastopening - start` and advancing `lastopening` (r_segs.c:721-723, 729-731).
    fn copy_clip(&mut self, start: i32, n: i32, ceiling: bool) -> Result<i32, RenderRefusal> {
        let at = self.planes.lastopening;
        for k in 0..n {
            let x = (start + k) as usize;
            let v = if ceiling {
                self.planes.ceilingclip[x]
            } else {
                self.planes.floorclip[x]
            };
            self.write_opening(at as i32 + k, v)?;
        }
        self.planes.lastopening += n as usize;
        Ok(at as i32 - start)
    }

    /// `R_RenderSegLoop` (r_segs.c:206-364).
    fn render_seg_loop(&mut self) -> Result<(), RenderRefusal> {
        let res = self.res;
        let viewheight = self.view.viewheight;
        while self.segs.rw_x < self.segs.rw_stopx {
            let rw_x = self.segs.rw_x;
            let x = rw_x as usize;
            // Mark floor / ceiling areas (r_segs.c:222-259).
            let mut yl = self.segs.topfrac.wrapping_add(HEIGHTUNIT - 1) >> HEIGHTBITS;
            let cc = i32::from(self.planes.ceilingclip[x]);
            if yl < cc + 1 {
                yl = cc + 1;
            }
            if self.segs.markceiling {
                let top = cc + 1;
                let mut bottom = yl - 1;
                let fc = i32::from(self.planes.floorclip[x]);
                if bottom >= fc {
                    bottom = fc - 1;
                }
                if top <= bottom {
                    let pl = self.planes.ceilingplane.expect("marked plane");
                    self.planes.visplanes[pl].set(x, top as u8, bottom as u8);
                }
            }
            let mut yh = self.segs.bottomfrac >> HEIGHTBITS;
            let fc = i32::from(self.planes.floorclip[x]);
            if yh >= fc {
                yh = fc - 1;
            }
            if self.segs.markfloor {
                let mut top = yh + 1;
                let bottom = fc - 1;
                let cc = i32::from(self.planes.ceilingclip[x]);
                if top <= cc {
                    top = cc + 1;
                }
                if top <= bottom {
                    let pl = self.planes.floorplane.expect("marked plane");
                    self.planes.visplanes[pl].set(x, top as u8, bottom as u8);
                }
            }
            // Texture column and lighting (r_segs.c:262-277).
            let mut colormap = 0u8;
            let mut iscale = 0i32;
            if self.segs.segtextured {
                let angle = (self
                    .segs
                    .rw_centerangle
                    .wrapping_add(self.xtoviewangle(rw_x)?)
                    >> ANGLETOFINESHIFT) as usize;
                // finetangent[angle] with angle up to 8191 reads on into finesine (the
                // genuine contiguous layout, decision R12).
                let tangent = FINE_TANGENT_SINE[angle];
                self.segs.texturecolumn = self
                    .segs
                    .rw_offset
                    .wrapping_sub(fixed_mul(tangent, self.segs.rw_distance))
                    >> 16;
                let index = ((self.segs.rw_scale as u32) >> LIGHTSCALESHIFT)
                    .min(MAXLIGHTSCALE as u32 - 1) as usize;
                colormap = self.wall_light(self.segs.walllights, index);
                if self.segs.rw_scale == 0 {
                    return Err(RenderRefusal::RangeCheck("dc_iscale division by zero"));
                }
                iscale = (0xffff_ffffu32 / self.segs.rw_scale as u32) as i32;
            }
            let texturecolumn = self.segs.texturecolumn;
            let seg = self.bsp.curline as u32;
            let draw = |yl: i32, yh: i32, tex: usize, mid: i32| ColumnDraw {
                x: rw_x,
                yl,
                yh,
                iscale,
                texturemid: mid,
                colormap,
                source: res.get_column(tex, texturecolumn),
                origin: tex as u32 + 1,
                feature: Feature::Wall { seg },
            };
            // Draw the wall tiers (r_segs.c:280-358).
            if self.segs.midtexture != 0 {
                // Single sided line.
                let c = draw(yl, yh, self.segs.midtexture, self.segs.rw_midtexturemid);
                self.draw_column(&c)?;
                self.planes.ceilingclip[x] = viewheight as i16;
                self.planes.floorclip[x] = -1;
            } else {
                if self.segs.toptexture != 0 {
                    // Top wall.
                    let mut mid = self.segs.pixhigh >> HEIGHTBITS;
                    self.segs.pixhigh = self.segs.pixhigh.wrapping_add(self.segs.pixhighstep);
                    let fc = i32::from(self.planes.floorclip[x]);
                    if mid >= fc {
                        mid = fc - 1;
                    }
                    if mid >= yl {
                        let c = draw(yl, mid, self.segs.toptexture, self.segs.rw_toptexturemid);
                        self.draw_column(&c)?;
                        self.planes.ceilingclip[x] = mid as i16;
                    } else {
                        self.planes.ceilingclip[x] = (yl - 1) as i16;
                    }
                } else if self.segs.markceiling {
                    // No top wall.
                    self.planes.ceilingclip[x] = (yl - 1) as i16;
                }
                if self.segs.bottomtexture != 0 {
                    // Bottom wall.
                    let mut mid = self.segs.pixlow.wrapping_add(HEIGHTUNIT - 1) >> HEIGHTBITS;
                    self.segs.pixlow = self.segs.pixlow.wrapping_add(self.segs.pixlowstep);
                    let cc = i32::from(self.planes.ceilingclip[x]);
                    if mid <= cc {
                        mid = cc + 1;
                    }
                    if mid <= yh {
                        let c = draw(
                            mid,
                            yh,
                            self.segs.bottomtexture,
                            self.segs.rw_bottomtexturemid,
                        );
                        self.draw_column(&c)?;
                        self.planes.floorclip[x] = mid as i16;
                    } else {
                        self.planes.floorclip[x] = (yh + 1) as i16;
                    }
                } else if self.segs.markfloor {
                    // No bottom wall.
                    self.planes.floorclip[x] = (yh + 1) as i16;
                }
                if self.segs.maskedtexture {
                    // Save texturecol for backdrawing of masked mid texture (r_segs.c:352-357).
                    let at = self.segs.maskedtexturecol.wrapping_add(rw_x);
                    self.write_opening(at, texturecolumn as i16)?;
                }
            }
            let s = &mut self.segs;
            s.rw_scale = s.rw_scale.wrapping_add(s.rw_scalestep);
            s.topfrac = s.topfrac.wrapping_add(s.topstep);
            s.bottomfrac = s.bottomfrac.wrapping_add(s.bottomstep);
            s.rw_x += 1;
        }
        Ok(())
    }

    /// `R_RenderMaskedSegRange (ds, x1, x2)` (r_segs.c:102-190) for the drawseg at `index`.
    pub(crate) fn render_masked_seg_range(
        &mut self,
        index: usize,
        x1: i32,
        x2: i32,
    ) -> Result<(), RenderRefusal> {
        let ds = self.bsp.drawsegs[index];
        let level = self.level;
        let curline = &level.geometry.segs[ds.curline];
        let front = self.sectors[curline.frontsector];
        let back = self.sectors[curline.backsector.ok_or(RenderRefusal::RangeCheck(
            "R_RenderMaskedSegRange (NULL backsector)",
        ))?];
        let texnum = self.texturetranslation[self.sides[curline.sidedef][1]];
        // Light table, computed even with a fixed colour map (r_segs.c:122-134).
        let walllights = WallLights::Scale(self.seg_light_row(curline.frontsector, ds.curline));
        let maskedtexturecol = ds.maskedtexturecol.expect("masked drawseg");
        let rw_scalestep = ds.scalestep;
        let mut spryscale = ds
            .scale1
            .wrapping_add(x1.wrapping_sub(ds.x1).wrapping_mul(rw_scalestep));
        let mfloorclip = ds.sprbottomclip;
        let mceilingclip = ds.sprtopclip;
        let viewz = self.frame.viewz;
        // Find positioning (r_segs.c:144-156): the translated texture's height here.
        let mut texturemid = if level.geometry.line_flags[curline.linedef] & ML_DONTPEGBOTTOM != 0 {
            front
                .floorheight
                .max(back.floorheight)
                .wrapping_add(self.res.textures[texnum].textureheight)
                .wrapping_sub(viewz)
        } else {
            front
                .ceilingheight
                .min(back.ceilingheight)
                .wrapping_sub(viewz)
        };
        texturemid = texturemid.wrapping_add(level.geometry.rowoffset[curline.sidedef]);
        let fixed = self.frame.fixedcolormap;
        let mut colormap = fixed as u8;
        // Draw the columns (r_segs.c:162-188).
        for dc_x in x1..=x2 {
            let col = self.read_opening(maskedtexturecol.wrapping_add(dc_x))?;
            if col != MAXSHORT {
                if fixed == 0 {
                    let index = ((spryscale as u32) >> LIGHTSCALESHIFT)
                        .min(MAXLIGHTSCALE as u32 - 1) as usize;
                    colormap = self.wall_light(walllights, index);
                }
                let sprtopscreen = self
                    .view
                    .centeryfrac
                    .wrapping_sub(fixed_mul(texturemid, spryscale));
                if spryscale == 0 {
                    return Err(RenderRefusal::RangeCheck("dc_iscale division by zero"));
                }
                let iscale = (0xffff_ffffu32 / spryscale as u32) as i32;
                // `R_GetColumn (texnum, col) - 3` (r_segs.c:181-182).
                let (bytes, at) = match self.res.masked_posts(texnum, i32::from(col)) {
                    MaskedPosts::Lump { bytes, at } => (bytes, at),
                    MaskedPosts::Composite => return Err(RenderRefusal::MaskedCompositeColumn),
                    MaskedPosts::Undefined => return Err(RenderRefusal::MaskedColumnUndefined),
                    MaskedPosts::Unallocated => return Err(RenderRefusal::UnallocatedColumn),
                };
                self.draw_masked_column(&crate::r_things::MaskedColumn {
                    bytes,
                    at,
                    x: dc_x,
                    texturemid,
                    sprtopscreen,
                    spryscale,
                    iscale,
                    colormap,
                    colfunc: crate::r_things::ColFunc::Base,
                    mfloorclip,
                    mceilingclip,
                    origin: texnum as u32 + 1,
                    feature: Feature::MaskedMid {
                        seg: ds.curline as u32,
                    },
                })?;
                self.write_opening(maskedtexturecol.wrapping_add(dc_x), MAXSHORT)?;
            }
            spryscale = spryscale.wrapping_add(rw_scalestep);
        }
        Ok(())
    }
}
