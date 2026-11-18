//! The heads-up text line of hu_lib.c as the message widget uses it: one line
//! (`HU_MSGHEIGHT` 1, hu_stuff.h:42) of a scrolling text (`hu_stext_t`, hu_lib.h:64-74) at
//! (0, 0), up to `HU_MAXLINELENGTH` characters (hu_lib.h:37).
use crate::SCREENWIDTH;
use crate::v_video::{Canvas, DrawError, Patch, Screen};

/// `HU_MAXLINELENGTH` (hu_lib.h:37).
pub const MAXLINELENGTH: usize = 80;
/// `HU_FONTSTART` (hu_stuff.h:30): the glyph of `'!'` is `hu_font[0]`.
pub const FONTSTART: u8 = b'!';

/// The message widget's line and its erase bookkeeping.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TextLine {
    /// `l[]` up to `len` (hu_lib.h:52-53).
    pub text: Vec<u8>,
    /// `needsupdate` (hu_lib.h:55): frames for which an erase would repaint the border.
    pub needsupdate: i32,
}

impl TextLine {
    /// `HUlib_addMessageToSText(s, 0, msg)` on a one-line widget (hu_lib.c:200-229): the line is
    /// cleared (`HUlib_clearTextLine`, hu_lib.c:46-51), every line gets `needsupdate = 4`
    /// (hu_lib.c:211-212), then each character is appended while `len < HU_MAXLINELENGTH`
    /// (`HUlib_addCharToTextLine`, hu_lib.c:68-84; longer text is silently cut).
    pub fn set_message(&mut self, msg: &[u8]) {
        self.text.clear();
        self.text
            .extend(msg.iter().take(MAXLINELENGTH).take_while(|c| **c != 0));
        self.needsupdate = 4;
    }

    /// `HUlib_drawTextLine(l, false)` (hu_lib.c:99-139) at (`x`, `y`): `toupper`, a glyph for
    /// `'!'`..`'_'` (the literal `'_'` bound) advancing by its `width` (offsets ignored), every
    /// other character advancing 4; the first glyph that would end past column 320, or a space
    /// reaching it, ends the line. `V_DrawPatchDirect` is `V_DrawPatch` (v_video.c:343).
    pub(crate) fn draw(
        &self,
        c: &mut Canvas<'_>,
        [mut x, y]: [i32; 2],
        font: &[Patch],
    ) -> Result<(), DrawError> {
        for &ch in &self.text {
            let ch = ch.to_ascii_uppercase();
            if ch != b' ' && (FONTSTART..=b'_').contains(&ch) {
                let p = &font[usize::from(ch - FONTSTART)];
                let w = i32::from(p.header.width);
                if x + w > SCREENWIDTH as i32 {
                    break;
                }
                c.patch(x, y, Screen::Fg, p)?;
                x += w;
            } else {
                x += 4;
                if x >= SCREENWIDTH as i32 {
                    break;
                }
            }
        }
        Ok(())
    }

    /// `HUlib_eraseTextLine` (hu_lib.c:143-174) with `viewwindowx` 0 (screenblocks 10 and 11,
    /// r_draw.c:705-715): no pixel is erased; `needsupdate` counts down.
    pub fn erase(&mut self) {
        if self.needsupdate > 0 {
            self.needsupdate -= 1;
        }
    }
}
