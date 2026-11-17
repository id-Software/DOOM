//! r_draw.c: the column and span drawers at high detail (`R_DrawColumn`, `R_DrawSpan`), with
//! the read-provenance taint of decision R4 kept per framebuffer pixel as the genuine runner's
//! tracker keeps it (`tools/reference/m5a_driver.c`, `track_column` and the span wrapper).
//! The fuzz drawer `R_DrawFuzzColumn` advances the persistent `fuzzpos` (decision R8). Low
//! detail is refused (decision R2); `R_DrawTranslatedColumn` is refused where it would be
//! called (decision R1, `r_things::ColFunc::Translated`).
use crate::r_data::ColumnSource;
use crate::r_main::{SCREENHEIGHT, SCREENWIDTH};
use crate::{Feature, FrameContext, RenderRefusal};

/// `FUZZTABLE` (r_draw.c:259).
pub const FUZZTABLE: usize = 50;
/// `FUZZOFF` (r_draw.c:260): one row of the framebuffer.
const FUZZOFF: i32 = SCREENWIDTH as i32;
/// `fuzzoffset[FUZZTABLE]` (r_draw.c:263-272).
const FUZZOFFSET: [i32; FUZZTABLE] = [
    FUZZOFF, -FUZZOFF, FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF, //
    FUZZOFF, FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF, //
    FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF, -FUZZOFF, -FUZZOFF, -FUZZOFF, //
    FUZZOFF, -FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF, //
    FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF, -FUZZOFF, FUZZOFF, //
    FUZZOFF, -FUZZOFF, -FUZZOFF, -FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, //
    FUZZOFF, FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF, FUZZOFF,
];
/// The colour map of every fuzz pixel (`colormaps[6*256 + ...]`, r_draw.c:358).
const FUZZCOLORMAP: u8 = 6;

/// The `dc_*` globals of one `R_DrawColumn` call (r_draw.c:79-96).
pub(crate) struct ColumnDraw<'a> {
    /// `dc_x`.
    pub x: i32,
    /// `dc_yl`.
    pub yl: i32,
    /// `dc_yh`.
    pub yh: i32,
    /// `dc_iscale`.
    pub iscale: i32,
    /// `dc_texturemid`.
    pub texturemid: i32,
    /// `dc_colormap` as a colour-map number.
    pub colormap: u8,
    /// `dc_source` with its definedness.
    pub source: ColumnSource<'a>,
    /// The taint origin of an undefined read from this source (nonzero).
    pub origin: u32,
    /// The stage, for first-difference reports.
    pub feature: Feature,
}

/// The `ds_*` globals of one `R_DrawSpan` call (r_draw.c:500-512).
pub(crate) struct SpanDraw {
    pub y: i32,
    pub x1: i32,
    pub x2: i32,
    pub xfrac: i32,
    pub yfrac: i32,
    pub xstep: i32,
    pub ystep: i32,
    pub colormap: u8,
    /// The flat number (`ds_source` is its lump).
    pub flat: usize,
    /// The stage, for first-difference reports.
    pub feature: Feature,
}

impl FrameContext<'_> {
    /// Refuses a column or span the genuine RANGECHECK lets through but that lies outside the
    /// view rectangle (`ylookup`/`columnofs` beyond the view: layout-dependent writes).
    fn in_view(&self, x: i32, y: i32) -> bool {
        (0..self.view.viewwidth).contains(&x) && (0..self.view.viewheight).contains(&y)
    }

    fn put(&mut self, off: usize, value: u8, taint: u32, feature: Feature) {
        self.screen[off] = value;
        self.taint[off] = taint;
        if let Some(t) = self.trace.as_deref_mut() {
            t[off] = feature;
        }
    }

    /// `R_DrawColumn` (r_draw.c:105-148): rows `dc_yl..=dc_yh`, texel
    /// `dc_source[(frac>>FRACBITS)&127]` through `dc_colormap`. A texel read outside the source
    /// block or on an unwritten composite byte taints its pixel (decision R4).
    pub(crate) fn draw_column(&mut self, c: &ColumnDraw<'_>) -> Result<(), RenderRefusal> {
        // Every caller sets `dc_source = R_GetColumn(..)` just before `colfunc ()`
        // (r_segs.c:286, 308, 337; r_plane.c:415), so a column whose tables or
        // composite no allocation holds is read whatever the count.
        if c.source.unallocated() {
            return Err(RenderRefusal::UnallocatedColumn);
        }
        let count = c.yh.wrapping_sub(c.yl);
        if count < 0 {
            return Ok(());
        }
        // RANGECHECK (r_draw.c:118-123).
        if (c.x as u32) >= SCREENWIDTH as u32 || c.yl < 0 || c.yh >= SCREENHEIGHT as i32 {
            return Err(RenderRefusal::RangeCheck("R_DrawColumn"));
        }
        if !self.in_view(c.x, c.yl) || !self.in_view(c.x, c.yh) {
            return Err(RenderRefusal::RangeCheck("R_DrawColumn outside the view"));
        }
        let mut frac = c
            .texturemid
            .wrapping_add(c.yl.wrapping_sub(self.view.centery).wrapping_mul(c.iscale));
        for y in c.yl..=c.yh {
            let (texel, defined) = c.source.texel(((frac >> 16) & 127) as usize);
            let off = self.view.offset(c.x, y);
            let value = self.colormap(c.colormap, texel);
            self.put(off, value, if defined { 0 } else { c.origin }, c.feature);
            frac = frac.wrapping_add(c.iscale);
        }
        Ok(())
    }

    /// `R_DrawFuzzColumn` (r_draw.c:286-368): rows `dc_yl..=dc_yh` after the border clamps,
    /// top to bottom in place, each pixel `colormaps[6*256 + dest[fuzzoffset[fuzzpos]]]`
    /// (the row above, already fuzzed, or the row below, not yet), advancing `fuzzpos`. The
    /// pixel's taint is copied from the pixel read, as the genuine tracker does (R4).
    pub(crate) fn draw_fuzz_column(
        &mut self,
        x: i32,
        mut yl: i32,
        mut yh: i32,
        feature: Feature,
    ) -> Result<(), RenderRefusal> {
        // Adjust borders (r_draw.c:293-298; exact-equality tests).
        if yl == 0 {
            yl = 1;
        }
        if yh == self.view.viewheight - 1 {
            yh = self.view.viewheight - 2;
        }
        let count = yh.wrapping_sub(yl);
        if count < 0 {
            return Ok(());
        }
        // RANGECHECK (r_draw.c:307-314).
        if (x as u32) >= SCREENWIDTH as u32 || yl < 0 || yh >= SCREENHEIGHT as i32 {
            return Err(RenderRefusal::RangeCheck("R_DrawFuzzColumn"));
        }
        if !self.in_view(x, yl) || !self.in_view(x, yh) {
            return Err(RenderRefusal::RangeCheck(
                "R_DrawFuzzColumn outside the view",
            ));
        }
        for y in yl..=yh {
            let delta = FUZZOFFSET[self.fuzzpos];
            // The pixel read is one row up or down; outside the view rectangle it would be
            // a layout-dependent read (the clamps above keep it inside on every frame).
            if !self.in_view(x, y + delta / FUZZOFF) {
                return Err(RenderRefusal::RangeCheck(
                    "R_DrawFuzzColumn read outside the view",
                ));
            }
            let off = self.view.offset(x, y);
            let src = (off as isize + delta as isize) as usize;
            let value = self.colormap(FUZZCOLORMAP, self.screen[src]);
            let taint = self.taint[src];
            self.put(off, value, taint, feature);
            self.fuzzpos += 1;
            if self.fuzzpos == FUZZTABLE {
                self.fuzzpos = 0;
            }
        }
        Ok(())
    }

    /// `R_DrawSpan` (r_draw.c:520-563). A flat lump shorter than 4,096 bytes taints the whole
    /// span, as the genuine tracker does; otherwise the span clears the taint.
    pub(crate) fn draw_span(&mut self, s: &SpanDraw) -> Result<(), RenderRefusal> {
        // RANGECHECK (r_draw.c:528-538; `> SCREENHEIGHT` is the genuine test).
        if s.x2 < s.x1
            || s.x1 < 0
            || s.x2 >= SCREENWIDTH as i32
            || (s.y as u32) > SCREENHEIGHT as u32
        {
            return Err(RenderRefusal::RangeCheck("R_DrawSpan"));
        }
        if !self.in_view(s.x1, s.y) || !self.in_view(s.x2, s.y) {
            return Err(RenderRefusal::RangeCheck("R_DrawSpan outside the view"));
        }
        let res = self.res;
        let source = res.flat(s.flat);
        let taint = if source.len() >= 4096 {
            0
        } else {
            (res.firstflat + s.flat) as u32 | 0x8000_0000
        };
        let mut xfrac = s.xfrac;
        let mut yfrac = s.yfrac;
        for x in s.x1..=s.x2 {
            let spot = (((yfrac >> (16 - 6)) & (63 * 64)) + ((xfrac >> 16) & 63)) as usize;
            let texel = source.get(spot).copied().unwrap_or(0);
            let off = self.view.offset(x, s.y);
            let value = self.colormap(s.colormap, texel);
            self.put(off, value, taint, s.feature);
            xfrac = xfrac.wrapping_add(s.xstep);
            yfrac = yfrac.wrapping_add(s.ystep);
        }
        Ok(())
    }
}

/// Guard tests below the public API. The public API (`Renderer::render`) reaches these drawers
/// only with arguments the BSP, plane and sprite clipping have already kept inside the view
/// (every committed golden frame passes the guards), so the refusals are exercised by calling
/// the crate-private drawers directly on the `amp.wad` fixture (`test_support::with_ctx`).
/// Each expectation is the genuine RANGECHECK of r_draw.c (investigation §2.2.8) or the
/// view-rectangle refusal of decision R3 (`RenderRefusal::RangeCheck` documentation).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::r_data::Resources;
    use crate::test_support::{with_ctx, with_ctx_res};

    fn column<'a>(x: i32, yl: i32, yh: i32, texels: &'a [u8]) -> ColumnDraw<'a> {
        ColumnDraw {
            x,
            yl,
            yh,
            iscale: 1 << 16,
            texturemid: 0,
            colormap: 0,
            source: Resources::lump_source(texels, 0),
            origin: 1,
            feature: Feature::Untouched,
        }
    }

    fn span(y: i32, x1: i32, x2: i32) -> SpanDraw {
        SpanDraw {
            y,
            x1,
            x2,
            xfrac: 0,
            yfrac: 0,
            xstep: 1 << 16,
            ystep: 0,
            colormap: 0,
            flat: 0,
            feature: Feature::Untouched,
        }
    }

    #[test]
    fn draw_column_range_check_outside_the_screen() {
        // Guard: r_draw.c:118-123 (`dc_x >= SCREENWIDTH`, `dc_yl < 0`, `dc_yh >= SCREENHEIGHT`).
        let texels = [7u8; 128];
        with_ctx(11, |ctx| {
            for (x, yl, yh) in [(320, 10, 20), (-1, 10, 20), (5, -1, 20), (5, 10, 200)] {
                assert_eq!(
                    ctx.draw_column(&column(x, yl, yh, &texels)),
                    Err(RenderRefusal::RangeCheck("R_DrawColumn")),
                    "{x} {yl} {yh}"
                );
            }
        });
    }

    #[test]
    fn draw_column_refuses_rows_and_columns_outside_the_view_window() {
        // Guard: decision R3's view-rectangle refusal. The genuine RANGECHECK passes a column
        // inside the 320x200 screen but outside the 288x144 window of screenblocks 9
        // (`viewwidth` 288, `viewheight` 144, investigation §2.2.1); its write lands in the
        // border and is layout-dependent.
        let texels = [7u8; 128];
        with_ctx(9, |ctx| {
            for (x, yl, yh) in [(288, 10, 20), (10, 10, 144), (10, 150, 160)] {
                assert_eq!(
                    ctx.draw_column(&column(x, yl, yh, &texels)),
                    Err(RenderRefusal::RangeCheck("R_DrawColumn outside the view")),
                    "{x} {yl} {yh}"
                );
            }
            // The last view row and column are inside.
            assert_eq!(ctx.draw_column(&column(287, 143, 143, &texels)), Ok(()));
        });
    }

    #[test]
    fn draw_column_with_a_negative_count_draws_nothing() {
        // r_draw.c:112-114: `count = dc_yh - dc_yl; if (count < 0) return;` runs before the
        // RANGECHECK, so even an out-of-screen coordinate draws and refuses nothing.
        let texels = [7u8; 128];
        with_ctx(11, |ctx| {
            let before = ctx.screen.to_vec();
            assert_eq!(ctx.draw_column(&column(999, 20, 10, &texels)), Ok(()));
            assert_eq!(ctx.screen, &before[..]);
        });
    }

    #[test]
    fn draw_fuzz_column_range_check_outside_the_screen() {
        // Guard: r_draw.c:307-314 after the border adjustment of r_draw.c:293-298 (only
        // `dc_yl == 0` and `dc_yh == viewheight - 1` are adjusted).
        with_ctx(11, |ctx| {
            for (x, yl, yh) in [(320, 10, 20), (-1, 10, 20), (5, -1, 20), (5, 10, 200)] {
                assert_eq!(
                    ctx.draw_fuzz_column(x, yl, yh, Feature::Untouched),
                    Err(RenderRefusal::RangeCheck("R_DrawFuzzColumn")),
                    "{x} {yl} {yh}"
                );
            }
        });
        with_ctx(9, |ctx| {
            assert_eq!(
                ctx.draw_fuzz_column(288, 10, 20, Feature::Untouched),
                Err(RenderRefusal::RangeCheck(
                    "R_DrawFuzzColumn outside the view"
                ))
            );
        });
    }

    #[test]
    fn draw_fuzz_column_advances_fuzzpos_per_row_and_wraps_at_fuzztable() {
        // r_draw.c:361-362: `fuzzpos` advances once per drawn row and wraps at FUZZTABLE (50);
        // the border adjustment of r_draw.c:293-298 turns `dc_yl == 0` into 1, so rows 0..=2
        // draw two rows.
        with_ctx(11, |ctx| {
            ctx.fuzzpos = 48;
            assert_eq!(ctx.draw_fuzz_column(10, 0, 2, Feature::Untouched), Ok(()));
            assert_eq!(ctx.fuzzpos, 0);
            assert_eq!(ctx.draw_fuzz_column(10, 5, 7, Feature::Untouched), Ok(()));
            assert_eq!(ctx.fuzzpos, 3);
            // An empty range (after the adjustment) neither draws nor advances.
            assert_eq!(ctx.draw_fuzz_column(10, 9, 8, Feature::Untouched), Ok(()));
            assert_eq!(ctx.fuzzpos, 3);
        });
    }

    #[test]
    fn draw_span_range_check() {
        // Guard: r_draw.c:528-538 (`ds_x2 < ds_x1 || ds_x1 < 0 || ds_x2 >= SCREENWIDTH ||
        // (unsigned)ds_y > SCREENHEIGHT`: the genuine test is `>`, so row 200 passes it).
        with_ctx(11, |ctx| {
            for (y, x1, x2) in [
                (10, 20, 19),
                (10, -1, 5),
                (10, 0, 320),
                (201, 0, 5),
                (-1, 0, 5),
            ] {
                assert_eq!(
                    ctx.draw_span(&span(y, x1, x2)),
                    Err(RenderRefusal::RangeCheck("R_DrawSpan")),
                    "{y} {x1} {x2}"
                );
            }
            // Row 200 passes the genuine test; it is outside the 200-row view, so the
            // renderer refuses it as a layout-dependent write (decision R3).
            assert_eq!(
                ctx.draw_span(&span(200, 0, 5)),
                Err(RenderRefusal::RangeCheck("R_DrawSpan outside the view"))
            );
        });
        with_ctx(9, |ctx| {
            assert_eq!(
                ctx.draw_span(&span(10, 0, 287)),
                Ok(()),
                "the whole 288-column window is inside"
            );
            assert_eq!(
                ctx.draw_span(&span(10, 0, 288)),
                Err(RenderRefusal::RangeCheck("R_DrawSpan outside the view"))
            );
        });
    }

    #[test]
    fn draw_fuzz_column_refuses_either_end_outside_the_view_window() {
        // Decision R3 (as for `draw_column`): inside the 320x200 screen but outside the 288x144
        // window of screenblocks 9, the bottom row alone (rows 10 to 144) is refused as well as
        // a column or top row alone, and the last window row and column are drawn. The fuzz
        // read of the row above or below is inside the window for every clamped column
        // (r_draw.c:293-298 keeps `yl >= 1` and `yh <= viewheight - 2`).
        let outside = Err(RenderRefusal::RangeCheck(
            "R_DrawFuzzColumn outside the view",
        ));
        with_ctx(9, |ctx| {
            assert_eq!(
                ctx.draw_fuzz_column(10, 10, 144, Feature::Untouched),
                outside
            );
            assert_eq!(
                ctx.draw_fuzz_column(10, 150, 160, Feature::Untouched),
                outside
            );
            assert_eq!(
                ctx.draw_fuzz_column(288, 10, 20, Feature::Untouched),
                outside
            );
            assert_eq!(
                ctx.draw_fuzz_column(10, 10, 143, Feature::Untouched),
                Ok(())
            );
        });
    }

    #[test]
    fn a_short_flat_taints_its_span_with_the_flat_lump_number() {
        // A flat lump shorter than 4,096 bytes is read past its end by the genuine `R_DrawSpan`
        // (r_draw.c:545-558, a 64x64 flat): the pixels carry the origin `0x8000_0000 | lump
        // number` (decision R4); a full flat leaves them defined (0).
        with_ctx_res(
            11,
            |res| {
                let mut dir = crate::r_data::LumpDirectory::new();
                for i in 0..res.directory().len() {
                    let name = *res.directory().name(i).unwrap();
                    let mut data = res.directory().data(i).unwrap().to_vec();
                    if i == res.firstflat + 2 {
                        data.truncate(100);
                    }
                    dir.add_single_lump(&crate::r_data::name_text(&name), &data)
                        .unwrap();
                }
                res.dir = dir;
            },
            |ctx| {
                let lump = (ctx.res.firstflat + 2) as u32;
                assert_eq!(ctx.res.flat(2).len(), 100);
                for (flat, want) in [(2, lump | 0x8000_0000), (1, 0)] {
                    ctx.taint.fill(9);
                    let mut sp = span(30, 40, 43);
                    sp.flat = flat;
                    assert_eq!(ctx.draw_span(&sp), Ok(()));
                    for x in 40..=43 {
                        assert_eq!(ctx.taint[30 * 320 + x], want, "flat {flat} x {x}");
                    }
                    assert_eq!(ctx.taint[30 * 320 + 44], 9, "only the span is written");
                }
            },
        );
    }
}
