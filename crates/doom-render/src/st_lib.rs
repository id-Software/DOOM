//! The status bar widgets of st_lib.c (st_lib.c:58-292): numbers, percentages, multiple icons
//! and binary icons, drawn into FG with erasures copied from BG at `(x, y - ST_Y)`.
//!
//! The widgets' pointers (`num`, `inum`, `val`, `on`) become arguments: the caller passes the
//! value the genuine pointer would read at draw time. Only the history the genuine structs carry
//! between frames is kept by the caller: `oldinum` of a multiple icon and `oldval` of a binary
//! icon. A number's `oldnum` is written and never read (st_lib.c:105), so it is not kept.
use crate::v_video::{Canvas, DrawError, Patch, Screen};

/// `ST_Y` (st_stuff.h:34): 200 - 32.
pub const ST_Y: i32 = 168;
/// The "no ammunition" value of `largeammo` (st_stuff.c:926): erased and not drawn
/// (st_lib.c:126-128).
pub const LARGEAMMO: i32 = 1994;

/// `STlib_drawNum` (st_lib.c:91-148), called through `STlib_updateNum` (st_lib.c:153-158) when
/// the widget is on. `digits` is `n->p` (ten patches), `width` is `n->width`. Every call erases
/// the `width` digit boxes, sized by digit 0's header and ignoring offsets (st_lib.c:99-100,
/// 121-124), and redraws: there is no comparison with the old value.
pub(crate) fn draw_num(
    c: &mut Canvas<'_>,
    [x0, y]: [i32; 2],
    width: i32,
    value: i32,
    digits: &[Patch; 10],
    minus: &Patch,
) -> Result<(), DrawError> {
    let w = i32::from(digits[0].header.width);
    let h = i32::from(digits[0].header.height);
    let neg = value < 0;
    let mut num = value;
    if neg {
        if width == 2 && num < -9 {
            num = -9;
        } else if width == 3 && num < -99 {
            num = -99;
        }
        num = -num;
    }
    // `y - ST_Y < 0` cannot hold: every number of st_stuff.c is at y 171 or below
    // (st_stuff.c:86-265), so the `I_Error` of st_lib.c:122-123 is not reachable.
    let x = x0 - width * w;
    c.copy_bg([x, y - ST_Y], [w * width, h], [x, y])?;
    if num == LARGEAMMO {
        return Ok(());
    }
    let mut x = x0;
    if num == 0 {
        c.patch(x - w, y, Screen::Fg, &digits[0])?;
    }
    let mut left = width;
    while num != 0 && left > 0 {
        left -= 1;
        x -= w;
        c.patch(x, y, Screen::Fg, &digits[(num % 10) as usize])?;
        num /= 10;
    }
    if neg {
        c.patch(x - 8, y, Screen::Fg, minus)?;
    }
    Ok(())
}

/// `STlib_updatePercent` (st_lib.c:180-188): the percent sign at `(x, y)`, outside the number's
/// erase box, only on a refresh; then the 3-digit number (st_lib.c:172).
pub(crate) fn update_percent(
    c: &mut Canvas<'_>,
    refresh: bool,
    xy: [i32; 2],
    value: i32,
    digits: &[Patch; 10],
    minus: &Patch,
    percent: &Patch,
) -> Result<(), DrawError> {
    if refresh {
        c.patch(xy[0], xy[1], Screen::Fg, percent)?;
    }
    draw_num(c, xy, 3, value, digits, minus)
}

/// `STlib_updateMultIcon` (st_lib.c:212-240) with the widget on: when the index changed (or on
/// a refresh) and is not -1, erase the old patch's box (its own offsets and size) from BG, draw
/// the new patch and remember it. An index going to -1 leaves the old icon and `oldinum`.
pub(crate) fn update_multicon(
    c: &mut Canvas<'_>,
    refresh: bool,
    [x, y]: [i32; 2],
    oldinum: &mut i32,
    inum: i32,
    patches: &[Patch],
) -> Result<(), DrawError> {
    if (*oldinum == inum && !refresh) || inum == -1 {
        return Ok(());
    }
    if *oldinum != -1 {
        let h = patches[*oldinum as usize].header;
        let bx = x - i32::from(h.leftoffset);
        let by = y - i32::from(h.topoffset);
        if by - ST_Y < 0 {
            return Err(DrawError::StatusBarError("updateMultIcon: y - ST_Y < 0"));
        }
        c.copy_bg(
            [bx, by - ST_Y],
            [i32::from(h.width), i32::from(h.height)],
            [bx, by],
        )?;
    }
    c.patch(x, y, Screen::Fg, &patches[inum as usize])?;
    *oldinum = inum;
    Ok(())
}

/// `STlib_updateBinIcon` (st_lib.c:264-292) with the widget on: when the value changed (or on a
/// refresh), draw the patch over FG if true, else restore its box from BG.
pub(crate) fn update_binicon(
    c: &mut Canvas<'_>,
    refresh: bool,
    [x, y]: [i32; 2],
    oldval: &mut bool,
    val: bool,
    p: &Patch,
) -> Result<(), DrawError> {
    if *oldval == val && !refresh {
        return Ok(());
    }
    let h = p.header;
    let bx = x - i32::from(h.leftoffset);
    let by = y - i32::from(h.topoffset);
    if by - ST_Y < 0 {
        return Err(DrawError::StatusBarError("updateBinIcon: y - ST_Y < 0"));
    }
    if val {
        c.patch(x, y, Screen::Fg, p)?;
    } else {
        c.copy_bg(
            [bx, by - ST_Y],
            [i32::from(h.width), i32::from(h.height)],
            [bx, by],
        )?;
    }
    *oldval = val;
    Ok(())
}
