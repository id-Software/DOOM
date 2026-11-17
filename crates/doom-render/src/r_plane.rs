//! r_plane.c: visplanes, openings, `R_ClearPlanes`, `R_FindPlane`, `R_CheckPlane`,
//! `R_MakeSpans`, `R_MapPlane` and `R_DrawPlanes` (with the sky of r_sky.c).
use crate::r_draw::{ColumnDraw, SpanDraw};
use crate::r_main::{
    ANG90, ANGLETOFINESHIFT, LIGHTLEVELS, LIGHTZSHIFT, MAXLIGHTZ, SCREENHEIGHT, SCREENWIDTH,
    finecosine, finesine, fixed_div, fixed_mul,
};
use crate::r_segs::LIGHTSEGSHIFT;
use crate::r_sky::{ANGLETOSKYSHIFT, SKYTEXTUREMID};
use crate::{Feature, FrameContext, RenderRefusal};

/// `MAXVISPLANES` (r_plane.c:52); exceeding it is the `VisplaneOverflow` refusal (decision R3).
pub const MAXVISPLANES: usize = 128;
/// `MAXOPENINGS` (r_plane.c:59); a write past it is the `OpeningOverflow` refusal (decision R3).
pub const MAXOPENINGS: usize = 320 * 64;

/// `visplane_t` (r_defs.h:460-480). `top` and `bottom` hold `SCREENWIDTH + 2` bytes indexed
/// by `x + 1`, so that `top[-1]` and `top[SCREENWIDTH]` are the `pad1`/`pad2` bytes the C
/// writes and reads (r_plane.c:438-447) and `bottom[-1]`/`bottom[SCREENWIDTH]` are `pad3`/`pad4`
/// (never written by the C, so 0).
#[derive(Clone, Debug)]
pub(crate) struct Visplane {
    pub height: i32,
    pub picnum: usize,
    pub lightlevel: i32,
    pub minx: i32,
    pub maxx: i32,
    top: [u8; SCREENWIDTH + 2],
    bottom: [u8; SCREENWIDTH + 2],
}

impl Visplane {
    /// A plane as `R_FindPlane`/`R_CheckPlane` set it up: `top` all 0xff. `bottom` is not
    /// cleared by the C; its stale bytes are read only beside unset columns (`top` 0xff),
    /// where `R_MakeSpans` and the sky loop never use them (they compare `t <= b` with t 255
    /// and b at most 199, or, for pad3/pad4, 0).
    fn new(height: i32, picnum: usize, lightlevel: i32, minx: i32, maxx: i32) -> Self {
        Self {
            height,
            picnum,
            lightlevel,
            minx,
            maxx,
            top: [0xff; SCREENWIDTH + 2],
            bottom: [0; SCREENWIDTH + 2],
        }
    }
    /// `top[x] = top; bottom[x] = bottom` (bytes; r_segs.c:238-239, 256-257).
    pub(crate) fn set(&mut self, x: usize, top: u8, bottom: u8) {
        self.top[x + 1] = top;
        self.bottom[x + 1] = bottom;
    }
    fn top(&self, x: i32) -> Result<u8, RenderRefusal> {
        usize::try_from(x + 1)
            .ok()
            .and_then(|i| self.top.get(i))
            .copied()
            .ok_or(RenderRefusal::RangeCheck("visplane top"))
    }
    fn bottom(&self, x: i32) -> Result<u8, RenderRefusal> {
        usize::try_from(x + 1)
            .ok()
            .and_then(|i| self.bottom.get(i))
            .copied()
            .ok_or(RenderRefusal::RangeCheck("visplane bottom"))
    }
}

/// The r_plane.c globals of one frame (r_plane.c:52-93).
#[derive(Clone, Debug)]
pub(crate) struct PlaneState {
    /// `visplanes[]` up to `lastvisplane`.
    pub visplanes: Vec<Visplane>,
    pub floorplane: Option<usize>,
    pub ceilingplane: Option<usize>,
    /// `openings[MAXOPENINGS]`.
    pub openings: Vec<i16>,
    /// `lastopening - openings` (may pass `MAXOPENINGS` by pointer arithmetic alone; a write
    /// past it is refused).
    pub lastopening: usize,
    pub floorclip: [i16; SCREENWIDTH],
    pub ceilingclip: [i16; SCREENWIDTH],
    spanstart: [i32; SCREENHEIGHT],
    planeheight: i32,
    planezlight: usize,
    cachedheight: [i32; SCREENHEIGHT],
    cacheddistance: [i32; SCREENHEIGHT],
    cachedxstep: [i32; SCREENHEIGHT],
    cachedystep: [i32; SCREENHEIGHT],
    basexscale: i32,
    baseyscale: i32,
    /// The flat of the plane being drawn (`ds_source`'s flat number).
    flat: usize,
    /// The visplane being drawn (first-difference reports only).
    current: usize,
}

impl Default for PlaneState {
    fn default() -> Self {
        Self {
            visplanes: Vec::with_capacity(MAXVISPLANES),
            floorplane: None,
            ceilingplane: None,
            openings: vec![0; MAXOPENINGS],
            lastopening: 0,
            floorclip: [0; SCREENWIDTH],
            ceilingclip: [0; SCREENWIDTH],
            spanstart: [0; SCREENHEIGHT],
            planeheight: 0,
            planezlight: 0,
            cachedheight: [0; SCREENHEIGHT],
            cacheddistance: [0; SCREENHEIGHT],
            cachedxstep: [0; SCREENHEIGHT],
            cachedystep: [0; SCREENHEIGHT],
            basexscale: 0,
            baseyscale: 0,
            flat: 0,
            current: 0,
        }
    }
}

impl FrameContext<'_> {
    /// `R_ClearPlanes` (r_plane.c:185-209).
    pub(crate) fn clear_planes(&mut self) -> Result<(), RenderRefusal> {
        let viewwidth = self.view.viewwidth as usize;
        for i in 0..viewwidth {
            self.planes.floorclip[i] = self.view.viewheight as i16;
            self.planes.ceilingclip[i] = -1;
        }
        self.planes.visplanes.clear();
        self.planes.lastopening = 0;
        self.planes.cachedheight = [0; SCREENHEIGHT];
        // Left to right mapping (r_plane.c:204-208).
        let angle = (self.frame.viewangle.wrapping_sub(ANG90) >> ANGLETOFINESHIFT) as i32;
        let cos = finecosine(angle).ok_or(RenderRefusal::TableRead("finecosine"))?;
        let sin = finesine(angle).ok_or(RenderRefusal::TableRead("finesine"))?;
        // Scale will be unit scale at SCREENWIDTH/2 distance.
        self.planes.basexscale = fixed_div(cos, self.view.centerxfrac)?;
        self.planes.baseyscale = fixed_div(sin, self.view.centerxfrac)?.wrapping_neg();
        Ok(())
    }

    /// `R_FindPlane` (r_plane.c:217-259).
    pub(crate) fn find_plane(
        &mut self,
        mut height: i32,
        picnum: usize,
        mut lightlevel: i32,
    ) -> Result<usize, RenderRefusal> {
        if self.res.skyflatnum == Some(picnum) {
            // All skies map together.
            height = 0;
            lightlevel = 0;
        }
        if let Some(i) =
            self.planes.visplanes.iter().position(|p| {
                p.height == height && p.picnum == picnum && p.lightlevel == lightlevel
            })
        {
            return Ok(i);
        }
        // `I_Error ("R_FindPlane: no more visplanes")` (r_plane.c:245-246).
        if self.planes.visplanes.len() == MAXVISPLANES {
            return Err(RenderRefusal::VisplaneOverflow);
        }
        self.planes.visplanes.push(Visplane::new(
            height,
            picnum,
            lightlevel,
            SCREENWIDTH as i32,
            -1,
        ));
        Ok(self.planes.visplanes.len() - 1)
    }

    /// `R_CheckPlane` (r_plane.c:265-324); a new plane past `MAXVISPLANES` is the C's
    /// unchecked write past `visplanes[]`, refused (decision R3).
    pub(crate) fn check_plane(
        &mut self,
        index: usize,
        start: i32,
        stop: i32,
    ) -> Result<usize, RenderRefusal> {
        let pl = &self.planes.visplanes[index];
        let (intrl, unionl) = if start < pl.minx {
            (pl.minx, start)
        } else {
            (start, pl.minx)
        };
        let (intrh, unionh) = if stop > pl.maxx {
            (pl.maxx, stop)
        } else {
            (stop, pl.maxx)
        };
        let mut x = intrl;
        while x <= intrh {
            if pl.top(x)? != 0xff {
                break;
            }
            x += 1;
        }
        if x > intrh {
            // Use the same one (r_plane.c:303-310).
            let pl = &mut self.planes.visplanes[index];
            pl.minx = unionl;
            pl.maxx = unionh;
            return Ok(index);
        }
        // Make a new visplane (r_plane.c:313-323).
        if self.planes.visplanes.len() == MAXVISPLANES {
            return Err(RenderRefusal::VisplaneOverflow);
        }
        let (h, p, l) = (pl.height, pl.picnum, pl.lightlevel);
        self.planes
            .visplanes
            .push(Visplane::new(h, p, l, start, stop));
        Ok(self.planes.visplanes.len() - 1)
    }

    /// `R_MapPlane` (r_plane.c:120-178).
    fn map_plane(&mut self, y: i32, x1: i32, x2: i32) -> Result<(), RenderRefusal> {
        // RANGECHECK (r_plane.c:131-139; `> viewheight` is the genuine, permissive test).
        if x2 < x1
            || x1 < 0
            || x2 >= self.view.viewwidth
            || (y as u32) > self.view.viewheight as u32
        {
            return Err(RenderRefusal::RangeCheck("R_MapPlane"));
        }
        let yi = usize::try_from(y)
            .ok()
            .filter(|y| *y < SCREENHEIGHT && (*y as i32) < self.view.viewheight)
            .ok_or(RenderRefusal::RangeCheck("R_MapPlane (yslope)"))?;
        let p = &mut self.planes;
        let (distance, xstep, ystep) = if p.planeheight != p.cachedheight[yi] {
            p.cachedheight[yi] = p.planeheight;
            let distance = fixed_mul(p.planeheight, self.view.yslope[yi]);
            p.cacheddistance[yi] = distance;
            p.cachedxstep[yi] = fixed_mul(distance, p.basexscale);
            p.cachedystep[yi] = fixed_mul(distance, p.baseyscale);
            (distance, p.cachedxstep[yi], p.cachedystep[yi])
        } else {
            (p.cacheddistance[yi], p.cachedxstep[yi], p.cachedystep[yi])
        };
        let f = self.frame;
        let length = fixed_mul(distance, self.view.distscale[x1 as usize]);
        let angle = (f
            .viewangle
            .wrapping_add(self.view.xtoviewangle[x1 as usize])
            >> ANGLETOFINESHIFT) as i32;
        let cos = finecosine(angle).ok_or(RenderRefusal::TableRead("finecosine"))?;
        let sin = finesine(angle).ok_or(RenderRefusal::TableRead("finesine"))?;
        let xfrac = f.viewx.wrapping_add(fixed_mul(cos, length));
        let yfrac = f.viewy.wrapping_neg().wrapping_sub(fixed_mul(sin, length));
        // Light (r_plane.c:160-170).
        let colormap = if f.fixedcolormap != 0 {
            f.fixedcolormap as u8
        } else {
            let index = ((distance as u32) >> LIGHTZSHIFT).min(MAXLIGHTZ as u32 - 1) as usize;
            self.view.zlight[self.planes.planezlight][index]
        };
        let flat = self.planes.flat;
        self.draw_span(&SpanDraw {
            y,
            x1,
            x2,
            xfrac,
            yfrac,
            xstep,
            ystep,
            colormap,
            flat,
            feature: Feature::Plane {
                visplane: self.planes.current as u16,
            },
        })
    }

    /// `spanstart[i]`.
    fn spanstart(&self, i: i32) -> Result<i32, RenderRefusal> {
        usize::try_from(i)
            .ok()
            .and_then(|i| self.planes.spanstart.get(i))
            .copied()
            .ok_or(RenderRefusal::RangeCheck("spanstart"))
    }
    fn set_spanstart(&mut self, i: i32, x: i32) -> Result<(), RenderRefusal> {
        let slot = usize::try_from(i)
            .ok()
            .and_then(|i| self.planes.spanstart.get_mut(i))
            .ok_or(RenderRefusal::RangeCheck("spanstart"))?;
        *slot = x;
        Ok(())
    }

    /// `R_MakeSpans` (r_plane.c:330-359).
    fn make_spans(
        &mut self,
        x: i32,
        mut t1: i32,
        mut b1: i32,
        mut t2: i32,
        mut b2: i32,
    ) -> Result<(), RenderRefusal> {
        while t1 < t2 && t1 <= b1 {
            let s = self.spanstart(t1)?;
            self.map_plane(t1, s, x - 1)?;
            t1 += 1;
        }
        while b1 > b2 && b1 >= t1 {
            let s = self.spanstart(b1)?;
            self.map_plane(b1, s, x - 1)?;
            b1 -= 1;
        }
        while t2 < t1 && t2 <= b2 {
            self.set_spanstart(t2, x)?;
            t2 += 1;
        }
        while b2 > b1 && b2 >= t2 {
            self.set_spanstart(b2, x)?;
            b2 -= 1;
        }
        Ok(())
    }

    /// `R_DrawPlanes` (r_plane.c:367-453). Its RANGECHECK counts (r_plane.c:375-387) cannot
    /// fire: every overflow they test was refused at its first write.
    pub(crate) fn draw_planes(&mut self) -> Result<(), RenderRefusal> {
        let res = self.res;
        for i in 0..self.planes.visplanes.len() {
            let (minx, maxx, picnum, height, lightlevel) = {
                let pl = &self.planes.visplanes[i];
                (pl.minx, pl.maxx, pl.picnum, pl.height, pl.lightlevel)
            };
            if minx > maxx {
                continue;
            }
            if res.skyflatnum == Some(picnum) {
                // Sky flat (r_plane.c:396-420): colour map 0, unaffected by light and by the
                // fixed colour maps.
                let iscale = self.view.pspriteiscale >> self.view.detailshift;
                for x in minx..=maxx {
                    let pl = &self.planes.visplanes[i];
                    let yl = i32::from(pl.top(x)?);
                    let yh = i32::from(pl.bottom(x)?);
                    if yl <= yh {
                        let angle = (self
                            .frame
                            .viewangle
                            .wrapping_add(self.view.xtoviewangle[x as usize])
                            >> ANGLETOSKYSHIFT) as i32;
                        self.draw_column(&ColumnDraw {
                            x,
                            yl,
                            yh,
                            iscale,
                            texturemid: SKYTEXTUREMID,
                            colormap: 0,
                            source: res.get_column(res.skytexture, angle),
                            origin: res.skytexture as u32 + 1,
                            feature: Feature::Sky { visplane: i as u16 },
                        })?;
                    }
                }
                continue;
            }
            // Regular flat (r_plane.c:423-451).
            self.planes.flat = self.flattranslation[picnum];
            self.planes.current = i;
            self.planes.planeheight = height.wrapping_sub(self.frame.viewz).wrapping_abs();
            let light = ((lightlevel >> LIGHTSEGSHIFT) + self.frame.extralight)
                .clamp(0, LIGHTLEVELS as i32 - 1);
            self.planes.planezlight = light as usize;
            {
                let pl = &mut self.planes.visplanes[i];
                pl.top[(maxx + 2) as usize] = 0xff;
                pl.top[minx as usize] = 0xff;
            }
            let stop = maxx + 1;
            for x in minx..=stop {
                let pl = &self.planes.visplanes[i];
                let (t1, b1, t2, b2) = (
                    i32::from(pl.top(x - 1)?),
                    i32::from(pl.bottom(x - 1)?),
                    i32::from(pl.top(x)?),
                    i32::from(pl.bottom(x)?),
                );
                self.make_spans(x, t1, b1, t2, b2)?;
            }
        }
        Ok(())
    }
}

/// Unit tests of `R_DrawPlanes` below the public API: the light index and the taint origin of a
/// sky column, on the `amp.wad` fixture (`test_support`). The public frames reach both only
/// with the committed rooms' own values (every golden light index lies below the clamp, and no
/// committed sky texture has an undefined column).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::r_data::ColumnEntry;
    use crate::test_support::{with_ctx, with_ctx_res};

    #[test]
    fn plane_light_is_the_sector_light_plus_extralight_clamped_to_the_sixteen_levels() {
        // R_DrawPlanes (r_plane.c:437-444): `light = (pl->lightlevel >> LIGHTSEGSHIFT) +
        // extralight; if (light >= LIGHTLEVELS) light = LIGHTLEVELS-1; if (light < 0) light =
        // 0; planezlight = zlight[light]`; LIGHTSEGSHIFT 4, LIGHTLEVELS 16.
        for (lightlevel, extralight, want) in [
            (255, 0, 15),
            (255, 5, 15),
            (240, 0, 15),
            (240, 1, 15),
            (224, 1, 15),
            (224, 0, 14),
            (128, 0, 8),
            (128, 1, 9),
            (16, -1, 0),
            (0, -3, 0),
            (0, 0, 0),
        ] {
            with_ctx(11, |ctx| {
                ctx.frame.extralight = extralight;
                // A flat plane (picnum 0 is not the sky flat) with no column set: nothing is
                // drawn, but the light index is computed before the column loop.
                ctx.planes
                    .visplanes
                    .push(Visplane::new(0, 0, lightlevel, 0, 0));
                ctx.draw_planes().expect("planes");
                assert_eq!(
                    ctx.planes.planezlight, want,
                    "light {lightlevel} extralight {extralight}"
                );
            });
        }
    }

    #[test]
    fn a_sky_column_taints_with_the_sky_texture_number_plus_one() {
        // A sky pixel read from an undefined texel carries the origin `skytexture + 1` (nonzero
        // and distinct per texture: STATE_FORMAT "taint ... origin", decision R4; the
        // evidence reports name it `T:<texture>`). The committed sky textures are fully
        // defined, so the fixture's SKY1 (texture number 6) is made undefined here.
        with_ctx_res(
            11,
            |res| {
                let sky = res.skytexture;
                res.textures[sky].columns.fill(ColumnEntry::Undefined);
            },
            |ctx| {
                let sky = ctx.res.skytexture;
                assert!(
                    sky >= 1,
                    "the label must differ from the bare texture number"
                );
                let flat = ctx.res.skyflatnum.expect("sky flat");
                let mut plane = Visplane::new(0, flat, 255, 10, 10);
                plane.set(10, 5, 7);
                ctx.planes.visplanes.push(plane);
                ctx.draw_planes().expect("planes");
                let row = |y: usize| ctx.taint[y * SCREENWIDTH + 10];
                assert_eq!(row(5), sky as u32 + 1);
                assert_eq!(row(7), sky as u32 + 1);
                assert_eq!(row(4), 0);
                assert_eq!(row(8), 0);
            },
        );
    }

    #[test]
    fn a_new_visplane_starts_empty_and_equal_planes_are_shared() {
        // R_FindPlane (r_plane.c:217-259): a new plane has `minx = SCREENWIDTH`, `maxx = -1`
        // (r_plane.c:253-254); a plane with the same height, picture and light is reused, and
        // every sky plane maps to height 0 and light 0 (r_plane.c:224-228).
        with_ctx(11, |ctx| {
            let a = ctx.find_plane(64 << 16, 3, 100).unwrap();
            assert_eq!(a, 0);
            assert_eq!(
                (ctx.planes.visplanes[a].minx, ctx.planes.visplanes[a].maxx),
                (320, -1)
            );
            assert_eq!(ctx.find_plane(64 << 16, 3, 100).unwrap(), a);
            assert_eq!(ctx.find_plane(65 << 16, 3, 100).unwrap(), 1);
            assert_eq!(ctx.find_plane(64 << 16, 2, 100).unwrap(), 2);
            assert_eq!(ctx.find_plane(64 << 16, 3, 101).unwrap(), 3);
            let sky = ctx.res.skyflatnum.expect("sky flat");
            let s = ctx.find_plane(99 << 16, sky, 77).unwrap();
            assert_eq!(ctx.planes.visplanes[s].height, 0);
            assert_eq!(ctx.planes.visplanes[s].lightlevel, 0);
            assert_eq!(ctx.find_plane(-5 << 16, sky, 5).unwrap(), s);
        });
    }

    #[test]
    fn map_plane_refuses_each_bad_span_with_its_own_check() {
        // R_MapPlane's RANGECHECK (r_plane.c:131-139): `x2 < x1 || x1 < 0 || x2 >= viewwidth ||
        // y > viewheight` is an I_Error, each alone; a row equal to `viewheight` passes that
        // permissive test and then reads `yslope[viewheight]`, outside the view (the Rust
        // renderer's own refusal, decision R3). Screenblocks 9: viewwidth 288, viewheight 144.
        with_ctx(9, |ctx| {
            let (w, h) = (ctx.view.viewwidth, ctx.view.viewheight);
            assert_eq!((w, h), (288, 144));
            let range = Err(RenderRefusal::RangeCheck("R_MapPlane"));
            let slope = Err(RenderRefusal::RangeCheck("R_MapPlane (yslope)"));
            assert_eq!(ctx.map_plane(10, 20, 10), range, "x2 < x1");
            assert_eq!(ctx.map_plane(10, -1, 10), range, "x1 < 0");
            assert_eq!(ctx.map_plane(10, 20, w), range, "x2 >= viewwidth");
            assert_eq!(ctx.map_plane(h + 1, 20, 30), range, "y > viewheight");
            assert_eq!(ctx.map_plane(-1, 20, 30), range, "y negative");
            assert_eq!(ctx.map_plane(h, 20, 30), slope, "y == viewheight");
            assert_eq!(
                ctx.map_plane(200, 20, 30),
                range,
                "y == SCREENHEIGHT is above viewheight"
            );
        });
    }
}
