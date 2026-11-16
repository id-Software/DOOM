//! r_draw.c: the column and span drawers at high detail (`R_DrawColumn`, `R_DrawSpan`), with
//! the read-provenance taint of decision R4 kept per framebuffer pixel as the genuine runner's
//! tracker keeps it (`tools/reference/m5a_driver.c`, `track_column` and the span wrapper).
//! Low detail is refused (decision R2); the fuzz drawer is step 2c.
use crate::r_data::ColumnSource;
use crate::r_main::{SCREENHEIGHT, SCREENWIDTH};
use crate::{Feature, FrameContext, RenderRefusal};

/// `FUZZTABLE` (r_draw.c:259).
pub const FUZZTABLE: usize = 50;

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
            self.put(off, value, taint, Feature::Plane);
            xfrac = xfrac.wrapping_add(s.xstep);
            yfrac = yfrac.wrapping_add(s.ystep);
        }
        Ok(())
    }
}
