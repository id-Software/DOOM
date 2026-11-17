//! The 2D drawers of v_video.c that the status bar and the heads-up text use: `V_DrawPatch`
//! (v_video.c:204-263, which `V_DrawPatchDirect` only calls, v_video.c:336-343) and `V_CopyRect`
//! (v_video.c:158-196), on the persistent `screens[0]` (FG) and the status bar background
//! `screens[4]` (BG, 320x32, st_stuff.c:1466-1471).
//!
//! What the genuine code does deterministically is reproduced: a patch whose offset-adjusted
//! rectangle leaves the 320x200 screen is silently skipped (the `RANGECHECK` branch,
//! v_video.c:221-232, "bad patch (ignored)"; `RANGECHECK` is defined unconditionally,
//! doomdef.h:75). Where the genuine code stops (`I_Error("Bad V_CopyRect")`, v_video.c:182) or
//! reads or writes outside an allocation (a post outside its lump, a write below the buffer,
//! a copy from a background byte no patch wrote, a negative `memcpy` size), the frame is refused
//! with a named [`DrawError`] (decision S17), never drawn differently.
use crate::r_data::{PatchHeader, patch_column_offset, patch_header};
use crate::{Resources, SCREENHEIGHT, SCREENWIDTH};

/// Rows of the status bar background `screens[4]` (`ST_HEIGHT`, st_stuff.h:32; allocated as
/// `ST_WIDTH*ST_HEIGHT` bytes, st_stuff.c:1470).
pub const BG_ROWS: usize = 32;

/// A refused 2D draw (decision S17): the genuine code would stop or touch memory outside an
/// allocation. The display state is unchanged by a refused frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DrawError {
    /// A column offset or post of patch lump `lump` lies outside the lump (the genuine walk reads
    /// past it until a byte reads as 0xff, v_video.c:244-262).
    PostOutsideLump {
        /// The patch's lump number.
        lump: usize,
    },
    /// A post of patch lump `lump` writes below the last row of its buffer: row 200 of
    /// `screens[0]` or row 32 of `screens[4]` (the range check tests 200 rows for every screen,
    /// v_video.c:221-224, and posts are not clipped to the patch height, v_video.c:248-257).
    BeyondBuffer {
        /// The patch's lump number.
        lump: usize,
    },
    /// `I_Error("Bad V_CopyRect")` (v_video.c:171-182).
    BadCopyRect,
    /// A copy from `screens[4]` reaches past its 32 rows (the range check allows 200,
    /// v_video.c:174) or has a negative width with a positive height (a negative `memcpy` size,
    /// v_video.c:192).
    CopyOutsideBackground,
    /// A copy from a `screens[4]` byte no patch wrote: `Z_Malloc` does not clear the buffer
    /// (st_stuff.c:1470; z_zone.c has no `memset`), so the genuine value is indeterminate.
    UndefinedBackground,
    /// One of st_lib.c's `I_Error`s: `"updateMultIcon: y - ST_Y < 0"` (st_lib.c:233) or
    /// `"updateBinIcon: y - ST_Y < 0"` (st_lib.c:282).
    StatusBarError(&'static str),
}

/// A patch lump of the global lump list with its validated header (`patch_t`, r_defs.h:356-364):
/// the header and, for a positive width, every `columnofs` entry lie inside the lump.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Patch {
    /// The lump number.
    pub lump: usize,
    /// The header.
    pub header: PatchHeader,
}

impl Patch {
    /// Validates lump `lump` as a patch: the 8-byte header and the column directory must lie in
    /// the lump (posts are checked when drawn).
    pub(crate) fn load(res: &Resources, lump: usize) -> Option<Self> {
        let bytes = res.lump(lump);
        let header = patch_header(bytes)?;
        for col in 0..i32::from(header.width.max(0)) {
            patch_column_offset(bytes, col)?;
        }
        Some(Self { lump, header })
    }
}

/// A 2D drawing target: FG (`screens[0]`, 320x200, with the M5a read-provenance taint, which a
/// pixel written from defined bytes clears) and BG (`screens[4]`, 320x32, with a mask of the
/// bytes a patch has written).
pub(crate) struct Canvas<'a> {
    pub res: &'a Resources,
    pub fg: &'a mut [u8],
    pub taint: &'a mut [u32],
    pub bg: &'a mut [u8],
    pub bg_defined: &'a mut [bool],
}

/// Which buffer a patch goes to (`scrn` 0 or 4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Screen {
    Fg,
    Bg,
}

impl Canvas<'_> {
    /// `V_DrawPatch(x, y, scrn, patch)` (v_video.c:204-263). Returns whether it drew (false:
    /// skipped by the range check).
    pub fn patch(&mut self, x: i32, y: i32, scrn: Screen, p: &Patch) -> Result<bool, DrawError> {
        let h = p.header;
        let y = y - i32::from(h.topoffset);
        let x = x - i32::from(h.leftoffset);
        let (sw, sh) = (SCREENWIDTH as i32, SCREENHEIGHT as i32);
        if x < 0 || x + i32::from(h.width) > sw || y < 0 || y + i32::from(h.height) > sh {
            return Ok(false);
        }
        let bytes = self.res.lump(p.lump);
        let rows = match scrn {
            Screen::Fg => SCREENHEIGHT,
            Screen::Bg => BG_ROWS,
        };
        let outside = DrawError::PostOutsideLump { lump: p.lump };
        for col in 0..i32::from(h.width.max(0)) {
            let mut at = usize::try_from(patch_column_offset(bytes, col).ok_or(outside)?)
                .map_err(|_| outside)?;
            loop {
                let top = *bytes.get(at).ok_or(outside)?;
                if top == 0xff {
                    break;
                }
                let len = usize::from(*bytes.get(at + 1).ok_or(outside)?);
                let src = bytes.get(at + 3..at + 3 + len).ok_or(outside)?;
                for (k, &v) in src.iter().enumerate() {
                    let row = y as usize + usize::from(top) + k;
                    if row >= rows {
                        return Err(DrawError::BeyondBuffer { lump: p.lump });
                    }
                    let i = row * SCREENWIDTH + (x + col) as usize;
                    match scrn {
                        Screen::Fg => {
                            self.fg[i] = v;
                            self.taint[i] = 0;
                        }
                        Screen::Bg => {
                            self.bg[i] = v;
                            self.bg_defined[i] = true;
                        }
                    }
                }
                at += len + 4;
            }
        }
        Ok(true)
    }

    /// `V_CopyRect(srcx, srcy, BG, width, height, destx, desty, FG)` (v_video.c:158-196), the
    /// only form the status bar uses (st_lib.c:123, 234, 286; st_stuff.c:508).
    pub fn copy_bg(
        &mut self,
        [srcx, srcy]: [i32; 2],
        [width, height]: [i32; 2],
        [destx, desty]: [i32; 2],
    ) -> Result<(), DrawError> {
        let (sw, sh) = (SCREENWIDTH as i32, SCREENHEIGHT as i32);
        if srcx < 0
            || srcx + width > sw
            || srcy < 0
            || srcy + height > sh
            || destx < 0
            || destx + width > sw
            || desty < 0
            || desty + height > sh
        {
            return Err(DrawError::BadCopyRect);
        }
        if height <= 0 {
            return Ok(());
        }
        if width < 0 || srcy + height > BG_ROWS as i32 {
            return Err(DrawError::CopyOutsideBackground);
        }
        for j in 0..height as usize {
            for i in 0..width as usize {
                let s = (srcy as usize + j) * SCREENWIDTH + srcx as usize + i;
                if !self.bg_defined[s] {
                    return Err(DrawError::UndefinedBackground);
                }
                let d = (desty as usize + j) * SCREENWIDTH + destx as usize + i;
                self.fg[d] = self.bg[s];
                self.taint[d] = 0;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::with_ctx;

    /// A patch lump: width 2, height 3, offsets (lx, ty); column 0 one post of 2 pixels at
    /// topdelta 1, column 1 empty.
    fn lump(lx: i16, ty: i16, height: i16, top: u8) -> Vec<u8> {
        let mut b = Vec::new();
        for v in [2i16, height, lx, ty] {
            b.extend(v.to_le_bytes());
        }
        b.extend(16i32.to_le_bytes());
        b.extend(22i32.to_le_bytes());
        b.extend([top, 2, 0, 7, 9, 0, 0xff, 0xff]);
        b
    }

    fn with_canvas(bytes: Vec<u8>, f: impl FnOnce(&mut Canvas<'_>, &Patch)) {
        with_ctx(11, |ctx| {
            let mut res = ctx.res.clone();
            res.dir.add_single_lump("TESTPTCH", &bytes).expect("lump");
            let lump = res.dir.len() - 1;
            let p = Patch::load(&res, lump).expect("patch");
            let (mut fg, mut taint) = (vec![0u8; 64000], vec![5u32; 64000]);
            let (mut bg, mut def) = (vec![0u8; 10240], vec![false; 10240]);
            let mut c = Canvas {
                res: &res,
                fg: &mut fg,
                taint: &mut taint,
                bg: &mut bg,
                bg_defined: &mut def,
            };
            f(&mut c, &p);
        });
    }

    /// v_video.c:216-217 and 248-257: the offsets move the patch, a post starts at its
    /// topdelta, and a written pixel clears the taint; transparent pixels are untouched.
    #[test]
    fn a_patch_is_offset_and_posted() {
        with_canvas(lump(1, 2, 3, 1), |c, p| {
            assert_eq!(c.patch(10, 20, Screen::Fg, p), Ok(true));
            assert_eq!(c.fg[19 * 320 + 9], 7);
            assert_eq!(c.fg[20 * 320 + 9], 9);
            assert_eq!(c.taint[19 * 320 + 9], 0);
            assert_eq!(c.fg[18 * 320 + 9], 0);
            assert_eq!(c.taint[18 * 320 + 9], 5);
            assert_eq!(c.fg[19 * 320 + 10], 0);
        });
    }

    /// v_video.c:221-232: out of the 320x200 screen by one pixel on any side is skipped, at
    /// the edge it is drawn; the BG uses the same 200-row test, so a post past row 31 of BG is
    /// a refused write, not a skip.
    #[test]
    fn the_range_check_skips_and_the_background_overrun_is_refused() {
        with_canvas(lump(0, 0, 3, 0), |c, p| {
            assert_eq!(c.patch(-1, 0, Screen::Fg, p), Ok(false));
            assert_eq!(c.patch(319, 0, Screen::Fg, p), Ok(false));
            assert_eq!(c.patch(318, 0, Screen::Fg, p), Ok(true));
            assert_eq!(c.patch(0, -1, Screen::Fg, p), Ok(false));
            assert_eq!(c.patch(0, 198, Screen::Fg, p), Ok(false));
            assert_eq!(c.patch(0, 197, Screen::Fg, p), Ok(true));
            assert_eq!(c.patch(0, 30, Screen::Bg, p), Ok(true));
            assert_eq!(
                c.patch(0, 31, Screen::Bg, p),
                Err(DrawError::BeyondBuffer { lump: p.lump })
            );
        });
        // A post taller than the declared height is not clipped (v_video.c:248-257).
        with_canvas(lump(0, 0, 1, 199), |c, p| {
            assert_eq!(
                c.patch(0, 0, Screen::Fg, p),
                Err(DrawError::BeyondBuffer { lump: p.lump })
            );
        });
    }

    /// A post whose data runs past the lump is refused.
    #[test]
    fn a_post_outside_its_lump_is_refused() {
        let mut b = lump(0, 0, 3, 0);
        b.truncate(b.len() - 4);
        with_canvas(b, |c, p| {
            assert_eq!(
                c.patch(0, 0, Screen::Fg, p),
                Err(DrawError::PostOutsideLump { lump: p.lump })
            );
        });
    }

    /// v_video.c:171-182 and 186-195: the 320x200 test, then the copy; BG has 32 rows and
    /// undefined bytes.
    #[test]
    fn copy_rect_checks_and_copies() {
        with_canvas(lump(0, 0, 3, 0), |c, _| {
            c.bg[320 + 4] = 66;
            c.bg_defined[320 + 4] = true;
            assert_eq!(c.copy_bg([4, 1], [1, 1], [4, 169]), Ok(()));
            assert_eq!(c.fg[169 * 320 + 4], 66);
            assert_eq!(c.taint[169 * 320 + 4], 0);
            assert_eq!(
                c.copy_bg([5, 1], [1, 1], [5, 169]),
                Err(DrawError::UndefinedBackground)
            );
            assert_eq!(
                c.copy_bg([-1, 0], [1, 1], [0, 0]),
                Err(DrawError::BadCopyRect)
            );
            assert_eq!(
                c.copy_bg([0, 0], [1, 1], [320, 0]),
                Err(DrawError::BadCopyRect)
            );
            assert_eq!(
                c.copy_bg([0, 0], [1, 1], [0, 200]),
                Err(DrawError::BadCopyRect)
            );
            assert_eq!(
                c.copy_bg([0, 199], [1, 2], [0, 0]),
                Err(DrawError::BadCopyRect)
            );
            assert_eq!(
                c.copy_bg([0, 31], [1, 2], [0, 0]),
                Err(DrawError::CopyOutsideBackground)
            );
            assert_eq!(
                c.copy_bg([4, 1], [-1, 1], [4, 1]),
                Err(DrawError::CopyOutsideBackground)
            );
            assert_eq!(c.copy_bg([4, 1], [-1, 0], [4, 1]), Ok(()));
        });
    }
}
