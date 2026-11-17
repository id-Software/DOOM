//! The app's own screens (decisions S9, S10 and S12; M5b investigation §3.4): plain 320×200
//! indexed pictures in a palette of the app's own, with text in a small font written for this
//! project (not STCFN, not from any IWAD or third-party font file), so they cannot be mistaken
//! for the game's. The pause overlay is drawn only on the presented picture, never into the
//! game frame.

use doom_render::{SCREENHEIGHT, SCREENWIDTH};

/// The app's palette: dark blue, white, yellow, grey and red, unlike any PLAYPAL palette.
pub const APP_PALETTE: [[u8; 3]; 5] = [
    [16, 24, 64],
    [240, 240, 240],
    [250, 210, 60],
    [150, 160, 190],
    [235, 95, 95],
];
/// Background colour index.
pub const BACKGROUND: u8 = 0;
/// Body text colour index.
pub const WHITE: u8 = 1;
/// Heading colour index.
pub const YELLOW: u8 = 2;
/// Border and hint colour index.
pub const GREY: u8 = 3;
/// Refusal heading colour index.
pub const RED: u8 = 4;
/// An overlay pixel that shows the game picture through.
pub const CLEAR: u8 = u8::MAX;

/// Glyph cell: 5×7 glyph, one column and three rows of spacing.
const ADVANCE: usize = 6;
const LINE: usize = 10;
/// Characters per line.
pub const COLUMNS: usize = SCREENWIDTH / ADVANCE - 2;

/// The font: each glyph is seven rows of five pixels, `#` set. Lower case is drawn as upper
/// case; any other character is drawn as `?`.
const FONT: [(char, &str); 59] = [
    (' ', "..... ..... ..... ..... ..... ..... ....."),
    ('A', ".###. #...# #...# ##### #...# #...# #...#"),
    ('B', "####. #...# #...# ####. #...# #...# ####."),
    ('C', ".###. #...# #.... #.... #.... #...# .###."),
    ('D', "####. #...# #...# #...# #...# #...# ####."),
    ('E', "##### #.... #.... ####. #.... #.... #####"),
    ('F', "##### #.... #.... ####. #.... #.... #...."),
    ('G', ".###. #...# #.... #.### #...# #...# .####"),
    ('H', "#...# #...# #...# ##### #...# #...# #...#"),
    ('I', ".###. ..#.. ..#.. ..#.. ..#.. ..#.. .###."),
    ('J', "..### ...#. ...#. ...#. ...#. #..#. .##.."),
    ('K', "#...# #..#. #.#.. ##... #.#.. #..#. #...#"),
    ('L', "#.... #.... #.... #.... #.... #.... #####"),
    ('M', "#...# ##.## #.#.# #.#.# #...# #...# #...#"),
    ('N', "#...# #...# ##..# #.#.# #..## #...# #...#"),
    ('O', ".###. #...# #...# #...# #...# #...# .###."),
    ('P', "####. #...# #...# ####. #.... #.... #...."),
    ('Q', ".###. #...# #...# #...# #.#.# #..#. .##.#"),
    ('R', "####. #...# #...# ####. #.#.. #..#. #...#"),
    ('S', ".#### #.... #.... .###. ....# ....# ####."),
    ('T', "##### ..#.. ..#.. ..#.. ..#.. ..#.. ..#.."),
    ('U', "#...# #...# #...# #...# #...# #...# .###."),
    ('V', "#...# #...# #...# #...# #...# .#.#. ..#.."),
    ('W', "#...# #...# #...# #.#.# #.#.# #.#.# .#.#."),
    ('X', "#...# #...# .#.#. ..#.. .#.#. #...# #...#"),
    ('Y', "#...# #...# .#.#. ..#.. ..#.. ..#.. ..#.."),
    ('Z', "##### ....# ...#. ..#.. .#... #.... #####"),
    ('0', ".###. #...# #..## #.#.# ##..# #...# .###."),
    ('1', "..#.. .##.. ..#.. ..#.. ..#.. ..#.. .###."),
    ('2', ".###. #...# ....# ...#. ..#.. .#... #####"),
    ('3', "####. ....# ....# .###. ....# ....# ####."),
    ('4', "...#. ..##. .#.#. #..#. ##### ...#. ...#."),
    ('5', "##### #.... ####. ....# ....# #...# .###."),
    ('6', "..##. .#... #.... ####. #...# #...# .###."),
    ('7', "##### ....# ...#. ..#.. .#... .#... .#..."),
    ('8', ".###. #...# #...# .###. #...# #...# .###."),
    ('9', ".###. #...# #...# .#### ....# ...#. .##.."),
    ('.', "..... ..... ..... ..... ..... .##.. .##.."),
    (',', "..... ..... ..... ..... .##.. ..#.. .#..."),
    (':', "..... .##.. .##.. ..... .##.. .##.. ....."),
    ('-', "..... ..... ..... ##### ..... ..... ....."),
    ('/', "....# ...#. ...#. ..#.. .#... .#... #...."),
    ('(', "...#. ..#.. .#... .#... .#... ..#.. ...#."),
    (')', ".#... ..#.. ...#. ...#. ...#. ..#.. .#..."),
    ('_', "..... ..... ..... ..... ..... ..... #####"),
    ('\'', "..#.. ..#.. .#... ..... ..... ..... ....."),
    ('!', "..#.. ..#.. ..#.. ..#.. ..#.. ..... ..#.."),
    ('=', "..... ..... ##### ..... ##### ..... ....."),
    ('+', "..... ..#.. ..#.. ##### ..#.. ..#.. ....."),
    ('?', ".###. #...# ....# ...#. ..#.. ..... ..#.."),
    ('{', "...## ..#.. ..#.. .#... ..#.. ..#.. ...##"),
    ('}', "##... ..#.. ..#.. ...#. ..#.. ..#.. ##..."),
    ('[', ".###. .#... .#... .#... .#... .#... .###."),
    (']', ".###. ...#. ...#. ...#. ...#. ...#. .###."),
    ('<', "...#. ..#.. .#... #.... .#... ..#.. ...#."),
    ('>', ".#... ..#.. ...#. ....# ...#. ..#.. .#..."),
    ('"', ".#.#. .#.#. ..... ..... ..... ..... ....."),
    ('#', ".#.#. ##### .#.#. .#.#. .#.#. ##### .#.#."),
    ('%', "##..# ##.#. ...#. ..#.. .#... .#.## #..##"),
];

/// The seven 5-bit rows of `c` (bit 4 is the leftmost pixel).
pub fn glyph(c: char) -> [u8; 7] {
    let c = c.to_ascii_uppercase();
    let art = FONT
        .iter()
        .find(|(k, _)| *k == c)
        .or_else(|| FONT.iter().find(|(k, _)| *k == '?'))
        .map_or("", |(_, art)| art);
    let mut rows = [0u8; 7];
    for (row, bits) in rows.iter_mut().zip(art.split(' ')) {
        *row = bits
            .bytes()
            .fold(0, |acc, b| (acc << 1) | u8::from(b == b'#'));
    }
    rows
}

/// A 320×200 picture of app colour indices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Screen {
    pixels: Vec<u8>,
}

impl Screen {
    /// A picture filled with `fill`.
    pub fn filled(fill: u8) -> Self {
        Self {
            pixels: vec![fill; SCREENWIDTH * SCREENHEIGHT],
        }
    }

    /// The 64,000 colour indices, row by row.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    fn fill_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: u8) {
        for y in y0..y1.min(SCREENHEIGHT) {
            for x in x0..x1.min(SCREENWIDTH) {
                self.pixels[y * SCREENWIDTH + x] = color;
            }
        }
    }

    /// Draws `text` with its top-left corner at (`x`, `y`), clipped to the picture.
    pub fn text(&mut self, x: usize, y: usize, text: &str, color: u8) {
        for (i, c) in text.chars().enumerate() {
            let left = x + i * ADVANCE;
            for (dy, bits) in glyph(c).iter().enumerate() {
                for dx in 0..5 {
                    let (px, py) = (left + dx, y + dy);
                    if bits & (0x10 >> dx) != 0 && px < SCREENWIDTH && py < SCREENHEIGHT {
                        self.pixels[py * SCREENWIDTH + px] = color;
                    }
                }
            }
        }
    }

    /// Draws `text` centred on row `y`.
    pub fn centred(&mut self, y: usize, text: &str, color: u8) {
        let width = text.chars().count() * ADVANCE;
        self.text(SCREENWIDTH.saturating_sub(width) / 2, y, text, color);
    }
}

/// Splits `text` into lines of at most [`COLUMNS`] characters, breaking after spaces and
/// commas where it can.
pub fn wrap(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut rest: Vec<char> = text.chars().collect();
    while rest.len() > COLUMNS {
        let cut = rest[..COLUMNS]
            .iter()
            .rposition(|c| *c == ' ' || *c == ',')
            .map_or(COLUMNS, |p| p + 1);
        lines.push(
            rest[..cut]
                .iter()
                .collect::<String>()
                .trim_end()
                .to_string(),
        );
        rest.drain(..cut);
    }
    lines.push(rest.into_iter().collect());
    lines
}

/// A full-screen app screen: `lines` (text and colour) centred vertically, each centred.
pub fn page(lines: &[(String, u8)]) -> Screen {
    let mut s = Screen::filled(BACKGROUND);
    s.fill_rect(0, 0, SCREENWIDTH, 2, GREY);
    s.fill_rect(0, SCREENHEIGHT - 2, SCREENWIDTH, SCREENHEIGHT, GREY);
    let top = SCREENHEIGHT.saturating_sub(lines.len() * LINE) / 2;
    for (i, (text, color)) in lines.iter().enumerate() {
        s.centred(top + i * LINE, text, *color);
    }
    s
}

/// The pause overlay: a framed box of text over [`CLEAR`] pixels.
pub fn pause_overlay() -> Screen {
    let mut s = Screen::filled(CLEAR);
    s.fill_rect(40, 76, 280, 124, GREY);
    s.fill_rect(42, 78, 278, 122, BACKGROUND);
    s.centred(86, "PAUSED", YELLOW);
    s.centred(104, "ESC: RESUME   R: RESTART   Q: QUIT", WHITE);
    s
}
