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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::with_ctx;

    const W: i32 = 4;
    /// Digit `k` is filled with `10 + k`, the minus sign with `MINUS`.
    const MINUS: u8 = 99;

    /// A patch lump `w` columns by 2 rows, offsets `(lx, ty)`, every pixel `v`.
    fn lump(w: i16, lx: i16, ty: i16, v: u8) -> Vec<u8> {
        let mut b = Vec::new();
        for x in [w, 2, lx, ty] {
            b.extend(x.to_le_bytes());
        }
        let first = 8 + 4 * i32::from(w);
        for k in 0..i32::from(w) {
            b.extend((first + 7 * k).to_le_bytes());
        }
        for _ in 0..w {
            b.extend([0, 2, 0, v, v, 0, 0xff]);
        }
        b
    }

    /// Runs `f` on a canvas with FG zero and every BG byte defined and at least 100
    /// (`bg[s] = 100 + s % 150`, so rows differ), the ten digit patches, the minus patch and an
    /// icon patch (2 columns, header height 3, offsets (3, 2), pixels 7) loaded.
    fn with_widgets(f: impl FnOnce(&mut Canvas<'_>, &[Patch; 10], &Patch, &Patch)) {
        with_ctx(11, |ctx| {
            let mut res = ctx.res.clone();
            for k in 0..10u8 {
                res.dir
                    .add_single_lump(&format!("TSTNUM{k}"), &lump(4, 0, 0, 10 + k))
                    .expect("digit");
            }
            res.dir
                .add_single_lump("TSTMINUS", &lump(3, 0, 0, MINUS))
                .expect("minus");
            let mut icon = lump(2, 3, 2, 7);
            icon[2..4].copy_from_slice(&3i16.to_le_bytes());
            res.dir.add_single_lump("TSTICON", &icon).expect("icon");
            let n = res.dir.len();
            let digits: [Patch; 10] =
                std::array::from_fn(|k| Patch::load(&res, n - 12 + k).expect("digit patch"));
            let minus = Patch::load(&res, n - 2).expect("minus patch");
            let icon = Patch::load(&res, n - 1).expect("icon patch");
            let (mut fg, mut taint) = (vec![0u8; 64000], vec![0u32; 64000]);
            let mut bg: Vec<u8> = (0..10240).map(|s| (100 + s % 150) as u8).collect();
            let mut def = vec![true; 10240];
            let mut c = Canvas {
                res: &res,
                fg: &mut fg,
                taint: &mut taint,
                bg: &mut bg,
                bg_defined: &mut def,
            };
            f(&mut c, &digits, &minus, &icon);
        });
    }

    /// FG row `y` as the glyph pixels it holds, `(x, value)` for every pixel below 100 (an
    /// erased box shows BG bytes, all 100 or more).
    fn glyph_pixels(c: &Canvas<'_>, y: i32) -> Vec<(i32, u8)> {
        (0..320)
            .map(|x| (x, c.fg[y as usize * 320 + x as usize]))
            .filter(|&(_, v)| (1..100).contains(&v))
            .collect()
    }

    /// The pixels a row of glyphs `(left x, value)` leaves: digits are 4 wide, the minus 3.
    fn expect(glyphs: &[(i32, u8)]) -> Vec<(i32, u8)> {
        let mut out = Vec::new();
        for &(left, v) in glyphs {
            let w = if v == MINUS { 3 } else { W };
            out.extend((left..left + w).map(|x| (x, v)));
        }
        out.sort();
        out
    }

    fn drawn(width: i32, value: i32) -> Vec<(i32, u8)> {
        let mut r = Vec::new();
        with_widgets(|c, d, m, _| {
            draw_num(c, [90, 171], width, value, d, m).expect("draw");
            r = glyph_pixels(c, 171);
        });
        r
    }

    fn digit(k: u8) -> u8 {
        10 + k
    }

    /// st_lib.c:105-110 for `numdigits == 3`: a negative number below -99 is -99; a number of
    /// -98..-1 is drawn as it is (so the width-2 clamp to -9 does not apply to 3 digits); the
    /// digits go right to left from x (st_lib.c:131-137) and the minus sign is 8 pixels left of
    /// the leftmost digit (st_lib.c:143-144). The public API cannot reach negative values:
    /// the health a player holds is never negative (p_inter.c:870-872), and
    /// `Display::observe_with` refuses them.
    #[test]
    fn negative_numbers_clamp_at_minus_99_for_three_digits() {
        let m99 = vec![(74, MINUS), (82, digit(9)), (86, digit(9))];
        assert_eq!(drawn(3, -250), expect(&m99));
        assert_eq!(drawn(3, -100), expect(&m99));
        assert_eq!(drawn(3, -99), expect(&m99));
        assert_eq!(
            drawn(3, -98),
            expect(&[(74, MINUS), (82, digit(9)), (86, digit(8))])
        );
        assert_eq!(
            drawn(3, -50),
            expect(&[(74, MINUS), (82, digit(5)), (86, digit(0))])
        );
        assert_eq!(drawn(3, -5), expect(&[(78, MINUS), (86, digit(5))]));
        assert_eq!(drawn(3, 50), expect(&[(82, digit(5)), (86, digit(0))]));
    }

    /// st_lib.c:105-106 for `numdigits == 2` (the frags widget, drawn only in deathmatch,
    /// st_stuff.c:971, so no single-player frame reaches it): below -9 is -9, -9..-1 is drawn
    /// as it is.
    #[test]
    fn negative_numbers_clamp_at_minus_9_for_two_digits() {
        let m9 = vec![(78, MINUS), (86, digit(9))];
        assert_eq!(drawn(2, -50), expect(&m9));
        assert_eq!(drawn(2, -10), expect(&m9));
        assert_eq!(drawn(2, -9), expect(&m9));
        assert_eq!(drawn(2, -5), expect(&[(78, MINUS), (86, digit(5))]));
        assert_eq!(drawn(2, -1), expect(&[(78, MINUS), (86, digit(1))]));
    }

    /// st_lib.c:131-137, `while (num && numdigits--)`: only `width` least significant digits.
    #[test]
    fn a_number_shows_only_its_width_in_digits() {
        let d234 = vec![(78, digit(2)), (82, digit(3)), (86, digit(4))];
        assert_eq!(drawn(3, 1234), expect(&d234));
        assert_eq!(drawn(3, 234), expect(&d234));
        assert_eq!(drawn(3, 34), expect(&[(82, digit(3)), (86, digit(4))]));
        assert_eq!(drawn(3, 0), expect(&[(86, digit(0))]));
    }

    /// st_lib.c:121-124: every call erases the `width` digit boxes, `width * w` wide and as high
    /// as digit 0, from BG at `(x, y - ST_Y)` (here the three-digit box at x 78, rows 171-172
    /// from BG rows 3-4) before drawing; only the digits' own pixels differ from BG.
    #[test]
    fn a_number_erases_its_box_from_bg() {
        with_widgets(|c, d, m, _| {
            draw_num(c, [90, 171], 3, 7, d, m).expect("draw");
            for j in 0..2usize {
                for i in 0..8usize {
                    assert_eq!(
                        c.fg[(171 + j) * 320 + 78 + i],
                        c.bg[(3 + j) * 320 + 78 + i],
                        "row {j} column {i}"
                    );
                }
            }
        });
    }

    /// st_lib.c:224-240 with an old icon: the old patch's box (its own offsets, `(x - leftoffset,
    /// y - topoffset)`, and size) is restored from BG at `y - topoffset - ST_Y`, then the new
    /// patch is drawn at (x, y). With the icon (2 wide, header height 3, offsets (3, 2),
    /// 2 pixel rows drawn per column): box x 7..9, rows 170..173; the new patch overwrites columns 7 and 8
    /// in rows 170 and 171, so row 172 of the box still shows BG row 4.
    #[test]
    fn a_multiple_icon_erases_the_old_box_from_bg() {
        with_widgets(|c, _, _, p| {
            let patches = [*p, *p];
            let mut old = 0;
            update_multicon(c, false, [10, 172], &mut old, 1, &patches).expect("draw");
            assert_eq!(old, 1);
            for (row, col) in [(172, 7), (172, 8)] {
                assert_eq!(
                    c.fg[row * 320 + col],
                    c.bg[(row - 168) * 320 + col],
                    "({col}, {row})"
                );
            }
            assert_eq!(c.fg[170 * 320 + 7], 7, "the new patch");
            assert_eq!(c.fg[171 * 320 + 8], 7, "the new patch");
        });
    }

    /// st_lib.c:272-290 with `val` false (V_CopyRect of the patch's box from BG, st_lib.c:285-286):
    /// the box is at `(x - leftoffset, y - topoffset)` and is read from BG at `y - topoffset -
    /// ST_Y`. The only caller passes `val = true` under wire 24 (st_stuff.rs, deathmatch 0), so
    /// the erase branch is reachable only by a direct call.
    #[test]
    fn a_false_binary_icon_restores_its_box_from_bg() {
        with_widgets(|c, _, _, p| {
            let mut old = true;
            update_binicon(c, false, [10, 172], &mut old, false, p).expect("erase");
            assert!(!old);
            // Box: x 7..9, FG rows 170..172 (height 3) from BG rows 2..4.
            for j in 0..3usize {
                for i in 0..2usize {
                    assert_eq!(c.fg[(170 + j) * 320 + 7 + i], c.bg[(2 + j) * 320 + 7 + i]);
                }
            }
            let changed = c.fg.iter().filter(|v| **v != 0).count();
            assert_eq!(changed, 6, "only the box is written");
        });
    }
}
