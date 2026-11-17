//! r_main.c: the trigonometric table accessors (tables.c), the fixed-point helpers with the
//! genuine strict build's wrapping (decision R12), `R_PointToAngle`, `R_PointToDist`,
//! `R_ExecuteSetViewSize` with `R_InitBuffer` (r_draw.c:695-720) and `R_InitTextureMapping`,
//! `R_InitLightTables`, `R_SetupFrame` and the `R_RenderPlayerView` driver.
use crate::tables::{FINE_TANGENT_SINE, TAN_TO_ANGLE};
use crate::{ConfigError, Detail, RenderConfig, RenderRefusal};

/// `FINEANGLES` (tables.h:50).
pub const FINEANGLES: i32 = 8192;
/// `ANGLETOFINESHIFT` (tables.h:55).
pub const ANGLETOFINESHIFT: u32 = 19;
/// `ANG90` (tables.h:69).
pub const ANG90: u32 = 0x4000_0000;
/// `FIELDOFVIEW` (r_main.c:48).
pub const FIELDOFVIEW: i32 = 2048;
/// `SCREENWIDTH`.
pub const SCREENWIDTH: usize = 320;
/// `SCREENHEIGHT`.
pub const SCREENHEIGHT: usize = 200;
/// `SBARHEIGHT` (r_draw.c:51).
pub const SBARHEIGHT: i32 = 32;
/// `LIGHTLEVELS` (r_main.h:69).
pub const LIGHTLEVELS: usize = 16;
/// `MAXLIGHTSCALE` (r_main.h:72).
pub const MAXLIGHTSCALE: usize = 48;
/// `LIGHTSCALESHIFT` (r_main.h:73).
pub const LIGHTSCALESHIFT: u32 = 12;
/// `MAXLIGHTZ` (r_main.h:74).
pub const MAXLIGHTZ: usize = 128;
/// `LIGHTZSHIFT` (r_main.h:75).
pub const LIGHTZSHIFT: u32 = 20;
/// `NUMCOLORMAPS` (r_main.h:87).
pub const NUMCOLORMAPS: i32 = 32;
/// `DISTMAP` (r_main.c:629).
pub const DISTMAP: i32 = 2;

/// `finetangent[i]` (tables.c:67); None outside 0..4096.
pub fn finetangent(i: i32) -> Option<i32> {
    usize::try_from(i)
        .ok()
        .filter(|i| *i < 4096)
        .map(|i| FINE_TANGENT_SINE[i])
}
/// `finesine[i]` (tables.c:584) in the genuine contiguous layout: an index in -4096..0 reads
/// `finetangent[4096 + i]` exactly as the genuine build does (decision R12); None outside
/// -4096..10240 (memory before `finetangent` or after `finesine`).
pub fn finesine(i: i32) -> Option<i32> {
    usize::try_from(i64::from(i) + 4096)
        .ok()
        .and_then(|k| FINE_TANGENT_SINE.get(k).copied())
}
/// `finecosine[i]` = `finesine[i + FINEANGLES/4]` (r_main.c:107).
pub fn finecosine(i: i32) -> Option<i32> {
    finesine(i.wrapping_add(FINEANGLES / 4))
}
/// `tantoangle[i]` (tables.c:1870); None outside 0..=2048.
pub fn tantoangle(i: u32) -> Option<u32> {
    TAN_TO_ANGLE.get(i as usize).copied()
}

/// `FixedMul` (m_fixed.c:45-50).
pub const fn fixed_mul(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) >> 16) as i32
}

/// `FixedDiv` (m_fixed.c:57-66) as the genuine strict (`-fwrapv`) build computes it:
/// `abs(INT_MIN)` is `INT_MIN`, then `FixedDiv2`'s binary64 division (m_fixed.c:71-87), whose
/// `I_Error` is a [`RenderRefusal::FixedDiv`].
pub fn fixed_div(a: i32, b: i32) -> Result<i32, RenderRefusal> {
    if (a.wrapping_abs() >> 14) >= b.wrapping_abs() {
        return Ok(if (a ^ b) < 0 { i32::MIN } else { i32::MAX });
    }
    let c = f64::from(a) / f64::from(b) * 65536.0;
    if !(-2147483648.0..2147483648.0).contains(&c) {
        return Err(RenderRefusal::FixedDiv);
    }
    Ok(c as i32)
}

/// `SlopeDiv` (tables.c:50-62).
pub fn slope_div(num: u32, den: u32) -> u32 {
    if den < 512 {
        return 2048;
    }
    let ans = (num << 3) / (den >> 8);
    ans.min(2048)
}

/// `R_PointToAngle` (r_main.c:260-345) for the point `(x, y)` relative to the view origin.
pub fn point_to_angle(x: i32, y: i32, viewx: i32, viewy: i32) -> u32 {
    let mut x = x.wrapping_sub(viewx);
    let mut y = y.wrapping_sub(viewy);
    if x == 0 && y == 0 {
        return 0;
    }
    let t = |n: i32, d: i32| TAN_TO_ANGLE[slope_div(n as u32, d as u32) as usize];
    if x >= 0 {
        if y >= 0 {
            if x > y {
                t(y, x)
            } else {
                (ANG90 - 1).wrapping_sub(t(x, y))
            }
        } else {
            y = y.wrapping_neg();
            if x > y {
                t(y, x).wrapping_neg()
            } else {
                0xc000_0000u32.wrapping_add(t(x, y))
            }
        }
    } else {
        x = x.wrapping_neg();
        if y >= 0 {
            if x > y {
                (0x8000_0000u32 - 1).wrapping_sub(t(y, x))
            } else {
                ANG90.wrapping_add(t(x, y))
            }
        } else {
            y = y.wrapping_neg();
            if x > y {
                0x8000_0000u32.wrapping_add(t(y, x))
            } else {
                (0xc000_0000u32 - 1).wrapping_sub(t(x, y))
            }
        }
    }
}

/// `ANG180` (tables.h:70).
pub const ANG180: u32 = 0x8000_0000;
/// `DBITS` (tables.h:76).
pub const DBITS: u32 = 5;

/// `R_PointOnSide` (r_main.c:161-211) for a node `(x, y, dx, dy)` in 16.16.
pub fn point_on_side(x: i32, y: i32, node: [i32; 4]) -> usize {
    let [nx, ny, ndx, ndy] = node;
    if ndx == 0 {
        // r_main.c:172-178
        return usize::from(if x <= nx { ndy > 0 } else { ndy < 0 });
    }
    if ndy == 0 {
        // r_main.c:179-185
        return usize::from(if y <= ny { ndx < 0 } else { ndx > 0 });
    }
    let dx = x.wrapping_sub(nx);
    let dy = y.wrapping_sub(ny);
    // Sign fast path (r_main.c:191-199).
    if (ndy ^ ndx ^ dx ^ dy) < 0 {
        return usize::from((ndy ^ dx) < 0);
    }
    let left = fixed_mul(ndy >> 16, dx);
    let right = fixed_mul(dy, ndx >> 16);
    // r_main.c:204-210
    usize::from(right >= left)
}

/// `R_PointToDist` (r_main.c:391-418). `abs` of `INT_MIN` is `INT_MIN` as in the strict build;
/// a `tantoangle` index outside the table (the viewer exactly on the point gives
/// `FixedDiv(0,0) = MAXINT`, hazard 2 of investigation §1.11) is a [`RenderRefusal::TableRead`].
pub fn point_to_dist(x: i32, y: i32, viewx: i32, viewy: i32) -> Result<i32, RenderRefusal> {
    let mut dx = x.wrapping_sub(viewx).wrapping_abs();
    let mut dy = y.wrapping_sub(viewy).wrapping_abs();
    if dy > dx {
        std::mem::swap(&mut dx, &mut dy);
    }
    let k = fixed_div(dy, dx)? >> DBITS;
    let t = u32::try_from(k)
        .ok()
        .and_then(tantoangle)
        .ok_or(RenderRefusal::TableRead("tantoangle (R_PointToDist)"))?;
    let angle = (t.wrapping_add(ANG90) >> ANGLETOFINESHIFT) as i32;
    let sine = finesine(angle).ok_or(RenderRefusal::TableRead("finesine (R_PointToDist)"))?;
    fixed_div(dx, sine)
}

/// `R_RenderPlayerView` (r_main.c:870-898) after `R_SetupFrame`: `R_ClearClipSegs`,
/// `R_ClearDrawSegs`, `R_ClearPlanes`, `R_ClearSprites`, `R_RenderBSPNode (numnodes-1)`,
/// `R_DrawPlanes` and `R_DrawMasked` (`NetUpdate` builds input commands only, §1.1).
pub(crate) fn render_player_view(
    ctx: &mut crate::FrameContext<'_>,
    weapons: &doom_sim::PlayerWeaponState,
) -> Result<(), RenderRefusal> {
    ctx.clear_clip_segs();
    ctx.clear_planes()?;
    ctx.render_bsp()?;
    ctx.draw_planes()?;
    ctx.draw_masked(weapons)
}

/// The view-size tables of `R_ExecuteSetViewSize` (r_main.c:671-753), `R_InitBuffer` and
/// `R_InitTextureMapping` (r_main.c:544-612), and `R_InitLightTables` (r_main.c:630-657).
/// Light tables hold colour-map numbers (the genuine tables hold `colormaps + level*256`).
#[derive(Clone, Debug)]
pub struct ViewSize {
    /// `scaledviewwidth`.
    pub scaledviewwidth: i32,
    /// `viewwidth`.
    pub viewwidth: i32,
    /// `viewheight`.
    pub viewheight: i32,
    /// `detailshift` (always 0: low detail is refused, decision R2).
    pub detailshift: u32,
    /// `centerx`.
    pub centerx: i32,
    /// `centery`.
    pub centery: i32,
    /// `centerxfrac`.
    pub centerxfrac: i32,
    /// `centeryfrac`.
    pub centeryfrac: i32,
    /// `projection`.
    pub projection: i32,
    /// `viewwindowx`.
    pub viewwindowx: i32,
    /// `viewwindowy`.
    pub viewwindowy: i32,
    /// `viewangletox[FINEANGLES/2]`.
    pub viewangletox: Vec<i32>,
    /// `xtoviewangle[SCREENWIDTH+1]`.
    pub xtoviewangle: Vec<u32>,
    /// `clipangle`.
    pub clipangle: u32,
    /// `pspritescale`.
    pub pspritescale: i32,
    /// `pspriteiscale`.
    pub pspriteiscale: i32,
    /// `screenheightarray[0..viewwidth]`.
    pub screenheightarray: Vec<i16>,
    /// `yslope[0..viewheight]`.
    pub yslope: Vec<i32>,
    /// `distscale[0..viewwidth]`.
    pub distscale: Vec<i32>,
    /// `scalelight[LIGHTLEVELS][MAXLIGHTSCALE]` (colour-map numbers).
    pub scalelight: Vec<[u8; MAXLIGHTSCALE]>,
    /// `zlight[LIGHTLEVELS][MAXLIGHTZ]` (colour-map numbers).
    pub zlight: Vec<[u8; MAXLIGHTZ]>,
}

impl ViewSize {
    /// The tables for `cfg` (screenblocks 9, 10 or 11, high detail; decision R2).
    pub fn new(cfg: RenderConfig) -> Result<Self, ConfigError> {
        if cfg.detail != Detail::High {
            return Err(ConfigError::LowDetail);
        }
        if !(9..=11).contains(&cfg.screenblocks) {
            return Err(ConfigError::ScreenBlocks(cfg.screenblocks));
        }
        let setblocks = i32::from(cfg.screenblocks);
        let (scaledviewwidth, viewheight) = if setblocks == 11 {
            (SCREENWIDTH as i32, SCREENHEIGHT as i32)
        } else {
            (setblocks * 32, (setblocks * 168 / 10) & !7)
        };
        let detailshift = 0;
        let viewwidth = scaledviewwidth >> detailshift;
        let centery = viewheight / 2;
        let centerx = viewwidth / 2;
        let centerxfrac = centerx << 16;
        let centeryfrac = centery << 16;
        // R_InitBuffer (r_draw.c:695-720).
        let viewwindowx = (SCREENWIDTH as i32 - scaledviewwidth) >> 1;
        let viewwindowy = if scaledviewwidth == SCREENWIDTH as i32 {
            0
        } else {
            (SCREENHEIGHT as i32 - SBARHEIGHT - viewheight) >> 1
        };
        // R_InitTextureMapping.
        let tan = |i: i32| finetangent(i).expect("index in 0..4096");
        let focallength = fixed_div(centerxfrac, tan(FINEANGLES / 4 + FIELDOFVIEW / 2))
            .expect("constant focal length");
        let mut viewangletox = vec![0i32; (FINEANGLES / 2) as usize];
        for (i, v) in viewangletox.iter_mut().enumerate() {
            let ft = tan(i as i32);
            *v = if ft > 2 << 16 {
                -1
            } else if ft < -(2 << 16) {
                viewwidth + 1
            } else {
                let t = fixed_mul(ft, focallength);
                let t = centerxfrac.wrapping_sub(t).wrapping_add((1 << 16) - 1) >> 16;
                t.clamp(-1, viewwidth + 1)
            };
        }
        let mut xtoviewangle = vec![0u32; SCREENWIDTH + 1];
        for x in 0..=viewwidth {
            let mut i = 0usize;
            while viewangletox[i] > x {
                i += 1;
            }
            xtoviewangle[x as usize] = ((i as u32) << ANGLETOFINESHIFT).wrapping_sub(ANG90);
        }
        for v in viewangletox.iter_mut() {
            if *v == -1 {
                *v = 0;
            } else if *v == viewwidth + 1 {
                *v = viewwidth;
            }
        }
        let clipangle = xtoviewangle[0];
        let pspritescale = (1 << 16) * viewwidth / SCREENWIDTH as i32;
        let pspriteiscale = (1 << 16) * SCREENWIDTH as i32 / viewwidth;
        let screenheightarray = vec![viewheight as i16; viewwidth as usize];
        let mut yslope = Vec::with_capacity(viewheight as usize);
        for i in 0..viewheight {
            let dy = (((i - viewheight / 2) << 16) + (1 << 15)).wrapping_abs();
            yslope.push(
                fixed_div(((viewwidth << detailshift) / 2) << 16, dy)
                    .expect("bounded view geometry"),
            );
        }
        let mut distscale = Vec::with_capacity(viewwidth as usize);
        for angle in xtoviewangle.iter().take(viewwidth as usize) {
            let cosadj = finecosine((angle >> ANGLETOFINESHIFT) as i32)
                .expect("index in range")
                .wrapping_abs();
            distscale.push(fixed_div(1 << 16, cosadj).expect("bounded view geometry"));
        }
        let mut scalelight = vec![[0u8; MAXLIGHTSCALE]; LIGHTLEVELS];
        for (i, row) in scalelight.iter_mut().enumerate() {
            let startmap =
                ((LIGHTLEVELS as i32 - 1 - i as i32) * 2) * NUMCOLORMAPS / LIGHTLEVELS as i32;
            for (j, v) in row.iter_mut().enumerate() {
                let level =
                    startmap - j as i32 * SCREENWIDTH as i32 / (viewwidth << detailshift) / DISTMAP;
                *v = level.clamp(0, NUMCOLORMAPS - 1) as u8;
            }
        }
        Ok(Self {
            scaledviewwidth,
            viewwidth,
            viewheight,
            detailshift,
            centerx,
            centery,
            centerxfrac,
            centeryfrac,
            projection: centerxfrac,
            viewwindowx,
            viewwindowy,
            viewangletox,
            xtoviewangle,
            clipangle,
            pspritescale,
            pspriteiscale,
            screenheightarray,
            yslope,
            distscale,
            scalelight,
            zlight: init_light_tables(),
        })
    }
    /// The framebuffer offset of view pixel `(x, y)` (`ylookup[y] + columnofs[x]`).
    pub fn offset(&self, x: i32, y: i32) -> usize {
        ((y + self.viewwindowy) as usize) * SCREENWIDTH + (x + self.viewwindowx) as usize
    }
}

/// `R_InitLightTables` (r_main.c:630-657).
pub fn init_light_tables() -> Vec<[u8; MAXLIGHTZ]> {
    let mut zlight = vec![[0u8; MAXLIGHTZ]; LIGHTLEVELS];
    for (i, row) in zlight.iter_mut().enumerate() {
        let startmap =
            ((LIGHTLEVELS as i32 - 1 - i as i32) * 2) * NUMCOLORMAPS / LIGHTLEVELS as i32;
        for (j, v) in row.iter_mut().enumerate() {
            let scale = fixed_div(
                (SCREENWIDTH as i32 / 2) << 16,
                ((j as i32) + 1) << LIGHTZSHIFT,
            )
            .expect("bounded light table")
                >> LIGHTSCALESHIFT;
            let level = startmap - scale / DISTMAP;
            *v = level.clamp(0, NUMCOLORMAPS - 1) as u8;
        }
    }
    zlight
}

/// What `R_SetupFrame` (r_main.c:846-875) reads from the player and computes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameView {
    /// `viewx`.
    pub viewx: i32,
    /// `viewy`.
    pub viewy: i32,
    /// `viewz`.
    pub viewz: i32,
    /// `viewangle` (`viewangleoffset` is 0 in a single-screen game).
    pub viewangle: u32,
    /// `extralight`.
    pub extralight: i32,
    /// `viewsin`.
    pub viewsin: i32,
    /// `viewcos`.
    pub viewcos: i32,
    /// `player->fixedcolormap` (0: none; else the colour map number).
    pub fixedcolormap: i32,
}

impl FrameView {
    /// `R_SetupFrame` from the player's mobj position and angle, `extralight`, the shadow's
    /// `viewz` and `fixedcolormap`.
    pub fn setup(
        viewx: i32,
        viewy: i32,
        viewangle: u32,
        extralight: i32,
        viewz: i32,
        fixedcolormap: i32,
    ) -> Self {
        let fine = (viewangle >> ANGLETOFINESHIFT) as i32;
        Self {
            viewx,
            viewy,
            viewz,
            viewangle,
            extralight,
            viewsin: finesine(fine).expect("index in range"),
            viewcos: finecosine(fine).expect("index in range"),
            fixedcolormap,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_layout_and_spot_values() {
        assert_eq!(FINE_TANGENT_SINE.len(), 4096 + 10240);
        assert_eq!(TAN_TO_ANGLE.len(), 2049);
        assert_eq!(finetangent(0), Some(-170910304));
        assert_eq!(finetangent(4095), Some(170910304));
        assert_eq!(finesine(0), Some(25));
        assert_eq!(finesine(-1), finetangent(4095));
        assert_eq!(finesine(-4097), None);
        assert_eq!(finesine(10240), None);
        assert_eq!(tantoangle(2048), Some(536870912));
        assert_eq!(finecosine(0), finesine(2048));
    }

    #[test]
    fn fixed_div_wraps_like_the_strict_build() {
        // abs(INT_MIN) is INT_MIN: the guard does not fire, and FixedDiv2 overflows.
        assert_eq!(fixed_div(i32::MIN, 1), Err(RenderRefusal::FixedDiv));
        assert_eq!(fixed_div(i32::MIN, 1 << 16), Ok(i32::MIN));
        assert_eq!(fixed_div(1, i32::MIN), Ok(i32::MIN));
        assert_eq!(fixed_div(1 << 16, 2 << 16), Ok(1 << 15));
        assert_eq!(fixed_div(5, 0), Ok(i32::MAX));
    }

    #[test]
    fn view_sizes() {
        for (sb, w, h, wx, wy) in [
            (11u8, 320, 200, 0, 0),
            (10, 320, 168, 0, 0),
            (9, 288, 144, 16, 12),
        ] {
            let v = ViewSize::new(RenderConfig {
                screenblocks: sb,
                detail: Detail::High,
            })
            .unwrap();
            assert_eq!(
                (v.viewwidth, v.viewheight, v.viewwindowx, v.viewwindowy),
                (w, h, wx, wy)
            );
        }
        assert_eq!(
            ViewSize::new(RenderConfig {
                screenblocks: 10,
                detail: Detail::Low
            })
            .err(),
            Some(ConfigError::LowDetail)
        );
        assert_eq!(
            ViewSize::new(RenderConfig {
                screenblocks: 8,
                detail: Detail::High
            })
            .err(),
            Some(ConfigError::ScreenBlocks(8))
        );
    }

    #[test]
    fn point_to_angle_octants() {
        assert_eq!(point_to_angle(1 << 16, 0, 0, 0), 0);
        assert_eq!(point_to_angle(0, 1 << 16, 0, 0), ANG90 - 1);
        assert_eq!(point_to_angle(-(1 << 16), 0, 0, 0), 0x8000_0000 - 1);
        assert_eq!(point_to_angle(0, 0, 0, 0), 0);
    }

    #[test]
    fn point_to_angle_octant_7_diagonal_where_slopediv_wraps() {
        // R_PointToAngle (r_main.c:320-335): `y < 0` and `x == -y` take the `else` branch
        // (octant 7), `ANG270 + tantoangle[SlopeDiv(x, y)]`, not octant 8's
        // `-tantoangle[SlopeDiv(y, x)]`. SlopeDiv (tables.c:50-62) computes `num<<3` in
        // `unsigned`, which wraps for `num >= 2^29`, so `SlopeDiv(2^29, 2^29)` is 0 and the two
        // branches differ there: ANG270 against 0. A vertex 8,192 units east and 8,192 units south
        // of the viewer is that point (independent review of the mutation survivors, 2026-11-17).
        assert_eq!(point_to_angle(1 << 29, -(1 << 29), 0, 0), 0xc000_0000);
        assert_eq!(slope_div(1 << 29, 1 << 29), 0, "num<<3 wraps to 0");
        // Below the wrap the seam is continuous (`tantoangle[2048] == ANG45`).
        assert_eq!(point_to_angle(1 << 20, -(1 << 20), 0, 0), 0xe000_0000);
    }

    #[test]
    fn point_to_dist_on_the_point_is_a_table_read_refusal() {
        // Investigation §1.11 hazard 2: with the viewpoint exactly on the point,
        // `FixedDiv(0,0) = MAXINT` (m_fixed.c:62-63) and `tantoangle[MAXINT>>5]` reads outside
        // the table (r_main.c:412); the renderer refuses it (decision R12). The golden frames
        // never put the viewer on a seg's start vertex, so the public API does not reach it.
        assert_eq!(
            point_to_dist(5 << 16, -7 << 16, 5 << 16, -7 << 16),
            Err(RenderRefusal::TableRead("tantoangle (R_PointToDist)"))
        );
    }

    #[test]
    fn fixed_div_refuses_the_i_error_overflow_guard() {
        // `FixedDiv2`'s `I_Error` (m_fixed.c:83-84) on a quotient outside the int range: the
        // strict build's `abs(INT_MIN)` is INT_MIN, so the overflow guard of m_fixed.c:62-63
        // (`(abs(a) >> 14) >= abs(b)`, which returns MAXINT/MININT) does not fire for
        // `a = INT_MIN`.
        assert_eq!(fixed_div(i32::MIN, 1), Err(RenderRefusal::FixedDiv));
        assert_eq!(fixed_div(i32::MIN, 2), Err(RenderRefusal::FixedDiv));
        assert_eq!(fixed_div(i32::MAX, 1 << 16), Ok(i32::MAX));
        // The guard path: |a| >> 14 >= |b| (m_fixed.c:62-63).
        assert_eq!(fixed_div(1 << 30, 1 << 15), Ok(i32::MAX));
        assert_eq!(fixed_div(-(1 << 30), 1 << 15), Ok(i32::MIN));
    }

    #[test]
    fn finesine_follows_the_sine_of_the_bin_centre() {
        // tables.c:584 `finesine[10240]`: entry i is 65536 * sin((i + 1/2) * 2 pi / 8192)
        // (first entries 25, 75, 125) to within 1.5 (rounding or truncation of the real sine; observed worst case 1.0001), and indices 8192.. repeat the first
        // quarter turn as the cosine overhang. Pins every entry's sign and size (a table
        // entry whose sign flipped differs by twice its magnitude), which no frame reads in
        // full.
        for i in 0..10240i32 {
            let want = 65536.0 * ((f64::from(i) + 0.5) * std::f64::consts::TAU / 8192.0).sin();
            let got = f64::from(finesine(i).unwrap());
            assert!(
                (got - want).abs() <= 1.5,
                "finesine[{i}] = {got}, sine says {want}"
            );
        }
        for i in 0..4096 {
            let (a, b) = (finesine(i).unwrap(), finesine(i + 4096).unwrap());
            assert!((a + b).abs() <= 1, "sin(x + pi) = -sin(x) at {i}: {a} {b}");
        }
    }

    #[test]
    fn finetangent_ends_at_4095() {
        // `finetangent[4096]` (tables.c:67): an index of 4096 would read `finesine[0]`.
        assert_eq!(finetangent(4095), Some(170910304));
        assert_eq!(finetangent(4096), None);
        assert_eq!(finetangent(-1), None);
    }

    #[test]
    fn fixed_div_overflow_follows_the_signs_of_the_operands() {
        // FixedDiv (m_fixed.c:57-66): `if ((abs(a)>>14) >= abs(b)) return (a^b) < 0 ? MININT :
        // MAXINT;` (in the strict build abs(INT_MIN) stays INT_MIN, so `a` = INT_MIN overflows
        // like a large value).
        let max = Ok(i32::MAX);
        let min = Ok(i32::MIN);
        assert_eq!(fixed_div(1 << 30, 1), max);
        assert_eq!(
            fixed_div(-(1 << 30), -1),
            max,
            "both negative: xor is positive"
        );
        assert_eq!(fixed_div(1 << 30, -1), min);
        assert_eq!(fixed_div(-(1 << 30), 1), min);
        // a == b == 0: 0 >= 0 overflows, and the xor of equal values is 0, not negative.
        assert_eq!(fixed_div(0, 0), max);
        assert_eq!(fixed_div(5, 0), max);
        assert_eq!(fixed_div(-5, 0), min);
        // No overflow: 1/2 = 0.5.
        assert_eq!(fixed_div(1 << 16, 2 << 16), Ok(1 << 15));
        assert_eq!(fixed_div(-(1 << 16), 2 << 16), Ok(-(1 << 15)));
    }

    #[test]
    fn slope_div_follows_tables_c() {
        // SlopeDiv (tables.c:50-62): `if (den < 512) return SLOPERANGE; ans = (num<<3)/(den>>8);
        // if (ans <= SLOPERANGE) return ans; return SLOPERANGE;` with SLOPERANGE 2048.
        assert_eq!(slope_div(1, 0), 2048);
        assert_eq!(slope_div(7, 100), 2048);
        assert_eq!(slope_div(0, 511), 2048);
        assert_eq!(slope_div(1, 512), 4, "den == 512 is divided: 8 / 2");
        assert_eq!(slope_div(100, 1 << 16), 3);
        assert_eq!(slope_div(256, 512), 1024);
        assert_eq!(slope_div(257, 512), 1028);
        assert_eq!(slope_div(1000, 512), 2048, "clamped to SLOPERANGE");
        assert_eq!(slope_div(u32::MAX >> 3, 512), 2048);
    }

    /// `R_PointOnSide` transliterated from r_main.c:160-210 (the source's `int` arithmetic,
    /// wrapping as the strict build does).
    fn point_on_side_c(x: i32, y: i32, node: [i32; 4]) -> usize {
        let [nx, ny, ndx, ndy] = node;
        if ndx == 0 {
            if x <= nx {
                return usize::from(ndy > 0);
            }
            return usize::from(ndy < 0);
        }
        if ndy == 0 {
            if y <= ny {
                return usize::from(ndx < 0);
            }
            return usize::from(ndx > 0);
        }
        let dx = x.wrapping_sub(nx);
        let dy = y.wrapping_sub(ny);
        if (ndy ^ ndx ^ dx ^ dy) as u32 & 0x8000_0000 != 0 {
            if (ndy ^ dx) as u32 & 0x8000_0000 != 0 {
                return 1;
            }
            return 0;
        }
        let left = fixed_mul(ndy >> 16, dx);
        let right = fixed_mul(dy, ndx >> 16);
        if right < left {
            return 0;
        }
        1
    }

    #[test]
    fn point_on_side_matches_the_c_on_every_sign_and_tie_combination() {
        // The C function is `R_PointOnSide`: the axis-aligned shortcuts (including the zero
        // partition), the sign-bit shortcut, then the cross-product comparison. The grid holds
        // every extreme, sign, zero and tie of the 16.16 operands (11 x 11 x 9 x 9 x 11 x 11 combinations).
        let v = [
            i32::MIN,
            -(3 << 16),
            -(1 << 16) - 1,
            -(1 << 16),
            -1,
            0,
            1,
            1 << 16,
            (1 << 16) + 1,
            3 << 16,
            i32::MAX,
        ];
        let w = [
            i32::MIN,
            -(2 << 16),
            -(1 << 16),
            -1,
            0,
            1,
            1 << 16,
            2 << 16,
            i32::MAX,
        ];
        let mut seen = [0usize; 2];
        for &x in &v {
            for &y in &v {
                for &nx in &w {
                    for &ny in &w {
                        for &ndx in &v {
                            for &ndy in &v {
                                let node = [nx, ny, ndx, ndy];
                                let want = point_on_side_c(x, y, node);
                                seen[want] += 1;
                                assert_eq!(point_on_side(x, y, node), want, "{x} {y} {node:?}");
                            }
                        }
                    }
                }
            }
        }
        assert!(
            seen[0] > 1000 && seen[1] > 1000,
            "both sides occur: {seen:?}"
        );
    }
}
