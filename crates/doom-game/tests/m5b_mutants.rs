//! Tests that pin what cargo-mutants found unpinned in `doom-game` (M5b step 11), through the
//! public API. Each test names what it pins. Expectations come from C facts already cited next
//! to the code (d_main.c:141-177, doomdef.h:250-277, d_event.h:108, g_game.c:1506-1523) or, for
//! the app's own screens and font (no C reference exists for them), from small pictures written
//! out by hand in the test.
//!
//! What is and is not independent of the code: the C facts and the hand-drawn screen pictures
//! are. The glyph shapes in `glyph_rows_of_a_few_known_shapes` are the font table's own entries
//! copied (the app's font has no other source), so they pin the decoding of the art into bits
//! and catch a changed entry, not the choice of shapes; the decoding itself is also pinned by a
//! binary literal for 'A'. `error_messages_read_as_written` copies the messages' own strings, so
//! it is a change detector.

use doom_game::app::{
    APP_PALETTE, CLEAR, FrameRefusal, RecordError, Recorder, Screen, Witness, glyph,
    labelled_pause_overlay, pause_overlay,
};
use doom_game::input::{
    Controls, ControlsError, Event, EventLayer, EventQueue, Input, InputError, KeyRoute,
    LayerOptions, MAX_EVENTS_PER_BUILD, MAXEVENTS, MouseButton, NUMKEYS, Preset, keys,
};

// ---------------------------------------------------------------------------------------
// The app's font and screens (screens.rs)

/// Pins `glyph`: seven rows of five bits, bit 4 leftmost, `#` set. The bitmaps are the letter
/// shapes drawn out here as pictures.
#[test]
fn glyph_rows_of_a_few_known_shapes() {
    let art = |rows: [&str; 7]| -> [u8; 7] {
        rows.map(|r| r.bytes().fold(0u8, |acc, b| acc * 2 + u8::from(b == b'#')))
    };
    let expect = [
        (
            'A',
            art([
                ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
            ]),
        ),
        (
            'E',
            art([
                "#####", "#....", "#....", "####.", "#....", "#....", "#####",
            ]),
        ),
        (
            '1',
            art([
                "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###.",
            ]),
        ),
        (
            '?',
            art([
                ".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#..",
            ]),
        ),
        (
            '.',
            art([
                ".....", ".....", ".....", ".....", ".....", ".##..", ".##..",
            ]),
        ),
        (
            ':',
            art([
                ".....", ".##..", ".##..", ".....", ".##..", ".##..", ".....",
            ]),
        ),
        (
            '/',
            art([
                "....#", "...#.", "...#.", "..#..", ".#...", ".#...", "#....",
            ]),
        ),
        (' ', [0; 7]),
    ];
    for (c, rows) in expect {
        assert_eq!(glyph(c), rows, "{c:?}");
    }
    // 'A' spelled out in numbers, so the helper above is itself checked
    assert_eq!(
        glyph('A'),
        [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001
        ]
    );
}

/// Pins `glyph`'s fallbacks: lower case is drawn as upper case, any character the font lacks as
/// `?` (the documented behaviour of the font), and the letters and digits are all different.
#[test]
fn glyph_lower_case_unknown_characters_and_distinct_letters() {
    for c in 'a'..='z' {
        assert_eq!(glyph(c), glyph(c.to_ascii_uppercase()), "{c}");
    }
    assert_ne!(glyph('?'), [0; 7]);
    for c in ['~', '@', '\u{e9}', '\u{1f600}', '\t', '`'] {
        assert_eq!(glyph(c), glyph('?'), "{c:?}");
    }
    let shapes: Vec<(char, [u8; 7])> = ('A'..='Z')
        .chain('0'..='9')
        .map(|c| (c, glyph(c)))
        .collect();
    for (i, (a, ra)) in shapes.iter().enumerate() {
        assert!(
            ra.iter().any(|r| *r != 0) && ra.iter().all(|r| *r < 32),
            "{a}"
        );
        for (b, rb) in &shapes[i + 1..] {
            assert_ne!(ra, rb, "{a} and {b}");
        }
    }
}

/// The number of pixels of `screen` that equal `color`.
fn count(screen: &Screen, color: u8) -> usize {
    screen.pixels().iter().filter(|p| **p == color).count()
}

/// Checks that `rows` (`#` = `ink`, `.` = `ground`) appear with their top-left corner at
/// (`x`, `y`) and that no other pixel of the screen is `ink`.
fn assert_art(screen: &Screen, x: usize, y: usize, rows: &[&str], ink: u8, ground: u8) {
    let mut inked = 0;
    for (dy, row) in rows.iter().enumerate() {
        for (dx, c) in row.bytes().enumerate() {
            let want = if c == b'#' {
                inked += 1;
                ink
            } else {
                ground
            };
            assert_eq!(
                screen.pixels()[(y + dy) * 320 + x + dx],
                want,
                "({dx},{dy})"
            );
        }
    }
    assert_eq!(
        count(screen, ink),
        inked,
        "pixels of the ink colour outside the art"
    );
}

/// Pins `Screen::text`: the exact pixels of "AB" at (7, 11): the first glyph at the position,
/// the second one 6 columns on (a 5-pixel glyph and one column of spacing), 7 rows tall.
#[test]
fn text_draws_two_glyphs_six_columns_apart_at_the_position() {
    let mut s = Screen::filled(0);
    s.text(7, 11, "AB", 9);
    assert_art(
        &s,
        7,
        11,
        &[
            ".###..####.",
            "#...#.#...#",
            "#...#.#...#",
            "#####.####.",
            "#...#.#...#",
            "#...#.#...#",
            "#...#.####.",
        ],
        9,
        0,
    );
    // a longer string keeps the 6-column advance: "ABAB" is the two-glyph picture twice,
    // with one column of spacing between the pairs
    let mut t = Screen::filled(0);
    t.text(0, 0, "ABAB", 9);
    let ab = [
        ".###..####.",
        "#...#.#...#",
        "#...#.#...#",
        "#####.####.",
        "#...#.#...#",
        "#...#.#...#",
        "#...#.####.",
    ];
    let abab: Vec<String> = ab.iter().map(|r| format!("{r}.{r}")).collect();
    let rows: Vec<&str> = abab.iter().map(String::as_str).collect();
    assert_art(&t, 0, 0, &rows, 9, 0);
}

/// Pins `Screen::text`'s clipping: a pixel at column 320 is not drawn and does not wrap onto the
/// next row's column 0, and rows 200 and below are not drawn (the call neither panics nor
/// writes).
#[test]
fn text_is_clipped_at_the_right_and_bottom_edges_without_wrapping() {
    // 'H' has its left stroke in column 0 of its cell: at x = 320 every pixel is outside
    let mut s = Screen::filled(0);
    s.text(320, 10, "H", 9);
    assert_eq!(count(&s, 9), 0);
    // at x = 317 the cell's columns 0..=2 are inside (317, 318, 319) and 3..=4 are outside;
    // 'A' row 3 is "#####": 3 pixels inside
    let mut s = Screen::filled(0);
    s.text(317, 10, "A", 9);
    assert_eq!(&s.pixels()[13 * 320 + 317..13 * 320 + 320], &[9, 9, 9]);
    assert_eq!(s.pixels()[14 * 320], 0, "no wrap onto the next row");
    assert_eq!(s.pixels()[13 * 320 + 1], 0, "no wrap onto the next row");
    // 'A' has 3, 2, 2, 5, 2, 2, 2 set pixels in its rows; columns 3 and 4 of the cell are at
    // 320 and 321, so of 'A''s row pixels (".###.", "#...#", "#####") only the ones in
    // columns 0..=2 are drawn: row 0 has 2 (columns 1, 2), rows 1, 2, 4, 5, 6 have 1 (column
    // 0) and row 3 has 3
    assert_eq!(count(&s, 9), 2 + 1 + 1 + 3 + 1 + 1 + 1);
    // bottom edge: 'A' at y = 196 draws rows 0..=3 and drops rows 4..=6
    let mut s = Screen::filled(0);
    s.text(0, 196, "A", 9);
    assert_eq!(count(&s, 9), 3 + 2 + 2 + 5);
    assert_eq!(&s.pixels()[199 * 320..199 * 320 + 5], &[9, 9, 9, 9, 9]);
    // a text that starts below the picture draws nothing
    let mut s = Screen::filled(0);
    s.text(0, 200, "A", 9);
    s.text(0, 250, "A", 9);
    assert_eq!(count(&s, 9), 0);
}

/// Pins `Screen::centred`: the text's width is 6 columns a character, it starts at
/// (320 - width) / 2, and a text wider than the picture starts at column 0.
#[test]
fn centred_text_starts_at_half_the_unused_width() {
    let ab: [&str; 7] = [
        ".###..####.",
        "#...#.#...#",
        "#...#.#...#",
        "#####.####.",
        "#...#.#...#",
        "#...#.#...#",
        "#...#.####.",
    ];
    // two characters: width 12, so column 154
    let mut s = Screen::filled(0);
    s.centred(40, "AB", 4);
    assert_art(&s, 154, 40, &ab, 4, 0);
    let mut s = Screen::filled(0);
    s.centred(0, "I", 4);
    // one character: width 6, column 157; 'I' row 0 is ".###." so its pixels are 158..=160
    assert_eq!(&s.pixels()[157..163], &[0, 4, 4, 4, 0, 0]);
    // 54 characters are 324 columns wide: the text starts at column 0
    let long = "I".repeat(54);
    let mut s = Screen::filled(0);
    s.centred(0, &long, 4);
    assert_eq!(&s.pixels()[0..5], &[0, 4, 4, 4, 0]);
    assert_eq!(&s.pixels()[6..11], &[0, 4, 4, 4, 0]);
}

/// The pause overlay as a flat vector of `(x, y, index)` for the box and the text, built by
/// hand from the documented geometry: a grey frame 2 pixels wide around a dark box over clear
/// pixels, then the heading and the key line centred in yellow and white.
fn expected_overlay(frame: (usize, usize, usize, usize), lines: &[(usize, &str, u8)]) -> Vec<u8> {
    let (x0, y0, x1, y1) = frame;
    let mut px = vec![CLEAR; 320 * 200];
    for y in y0..y1 {
        for x in x0..x1 {
            let border = x < x0 + 2 || x >= x1 - 2 || y < y0 + 2 || y >= y1 - 2;
            px[y * 320 + x] = if border { 3 } else { 0 };
        }
    }
    for (y, text, color) in lines {
        let mut layer = Screen::filled(CLEAR);
        let x = (320 - text.chars().count() * 6) / 2;
        layer.text(x, *y, text, *color);
        for (i, p) in layer.pixels().iter().enumerate() {
            if *p != CLEAR {
                px[i] = *p;
            }
        }
    }
    px
}

/// Pins the pause box: frame 40..280 by 76..124 in grey (index 3), 2 pixels wide, dark inside
/// (index 0), "PAUSED" in yellow (2) centred on row 86 and the key line in white (1) on row
/// 104; everything else is [`CLEAR`].
#[test]
fn the_pause_box_has_its_frame_and_colours() {
    let o = pause_overlay();
    let keys_line = "ESC: RESUME   R: RESTART   Q: QUIT";
    let want = expected_overlay(
        (40, 76, 280, 124),
        &[(86, "PAUSED", 2), (104, keys_line, 1)],
    );
    assert_eq!(o.pixels(), want.as_slice());
    let at = |x: usize, y: usize| o.pixels()[y * 320 + x];
    // the frame, explicitly
    assert_eq!(
        (at(39, 100), at(40, 100), at(41, 100), at(42, 100)),
        (CLEAR, 3, 3, 0)
    );
    assert_eq!(
        (at(277, 100), at(278, 100), at(279, 100), at(280, 100)),
        (0, 3, 3, CLEAR)
    );
    assert_eq!(
        (at(100, 75), at(100, 76), at(100, 77), at(100, 78)),
        (CLEAR, 3, 3, 0)
    );
    assert_eq!(
        (at(100, 121), at(100, 122), at(100, 123), at(100, 124)),
        (0, 3, 3, CLEAR)
    );
    assert!(count(&o, 2) > 0 && count(&o, 1) > 0);
    assert_eq!(count(&o, 4), 0);
}

/// Pins the pause box of an unrecorded session: the taller box 20..300 by 70..130, "PAUSED" on
/// row 80, the key line on row 96 and the label in yellow on row 112.
#[test]
fn the_labelled_pause_box_has_its_frame_label_and_colours() {
    let label = "not recorded: no demo, cannot be replayed";
    let o = labelled_pause_overlay(label);
    let want = expected_overlay(
        (20, 70, 300, 130),
        &[
            (80, "PAUSED", 2),
            (96, "ESC: RESUME   R: RESTART   Q: QUIT", 1),
            (112, label, 2),
        ],
    );
    assert_eq!(o.pixels(), want.as_slice());
    assert_eq!(o.pixels()[100 * 320 + 19], CLEAR);
    assert_eq!(o.pixels()[100 * 320 + 20], 3);
    assert_eq!(o.pixels()[69 * 320 + 100], CLEAR);
    assert_eq!(o.pixels()[129 * 320 + 100], 3);
    assert_eq!(o.pixels()[130 * 320 + 100], CLEAR);
}

/// Pins the app palette: five colours, and the indices the screens use are inside it.
#[test]
fn the_app_palette_has_five_colours_and_the_clear_index_is_outside() {
    assert_eq!(APP_PALETTE.len(), 5);
    assert_eq!(CLEAR, 255);
    assert_eq!(APP_PALETTE[3], [150, 160, 190]);
    assert_eq!(APP_PALETTE[0], [16, 24, 64]);
}

// ---------------------------------------------------------------------------------------
// The event ring (queue.rs; d_main.c:141-177, d_event.h:108)

/// Collects what `process` delivers, giving up with an error after `limit` deliveries so that
/// a ring that never reaches its head fails the test instead of looping.
fn deliver(q: &mut EventQueue, limit: usize) -> (Vec<Event>, Result<(), InputError>) {
    let mut got = Vec::new();
    let r = q.process(|e| {
        got.push(e);
        if got.len() > limit {
            Err(InputError::MouseOverflow)
        } else {
            Ok(())
        }
    });
    (got, r)
}

/// Pins `EventQueue::post`, `pending` and `process`: the ring has `MAXEVENTS` = 64 slots (an
/// index is taken `& 63`), the head and the tail advance by one slot and wrap at 64, and
/// `pending` counts the slots between them.
#[test]
fn the_ring_counts_and_delivers_in_order_across_the_wrap() {
    assert_eq!(MAXEVENTS, 64);
    let mut q = EventQueue::new();
    assert_eq!(q.pending(), 0);
    q.post(Event::KeyDown(b'a'));
    assert_eq!(q.pending(), 1);
    q.post(Event::KeyDown(b'b'));
    q.post(Event::KeyUp(b'a'));
    assert_eq!(q.pending(), 3);
    let (got, r) = deliver(&mut q, 100);
    assert_eq!(r, Ok(()));
    assert_eq!(
        got,
        [
            Event::KeyDown(b'a'),
            Event::KeyDown(b'b'),
            Event::KeyUp(b'a')
        ]
    );
    assert_eq!(q.pending(), 0);
    // head 3, tail 3: 61 more posts move the head to 64 mod 64 = 0 and then beyond
    for i in 0..61u8 {
        q.post(Event::KeyDown(i));
    }
    assert_eq!(q.pending(), 61);
    let (got, r) = deliver(&mut q, 100);
    assert_eq!(r, Ok(()));
    assert_eq!(got.len(), 61);
    assert_eq!(got[0], Event::KeyDown(0));
    assert_eq!(got[60], Event::KeyDown(60));
    // head and tail are both at 0 now (3 + 61 = 64); 5 posts and a delivery cross no wrap,
    // then the head wraps ahead of the tail: tail 5, head 4 after 63 further posts
    for i in 0..5u8 {
        q.post(Event::KeyUp(i));
    }
    let (_, r) = deliver(&mut q, 100);
    assert_eq!(r, Ok(()));
    for i in 0..63u8 {
        q.post(Event::KeyDown(i));
    }
    // head = (5 + 63) mod 64 = 4, tail = 5: pending = (4 - 5) mod 64 = 63
    assert_eq!(q.pending(), 63);
    let (got, r) = deliver(&mut q, 100);
    assert_eq!(r, Ok(()));
    assert_eq!(got.len(), 63);
    assert_eq!(got[0], Event::KeyDown(0));
    assert_eq!(got[62], Event::KeyDown(62));
}

/// Pins the loss of d_main.c:150-154: 64 posts with none processed leave head equal to tail, the
/// ring reads as empty, and nothing is delivered.
#[test]
fn sixty_four_posts_wrap_the_head_onto_the_tail_and_read_as_empty() {
    let mut q = EventQueue::new();
    for i in 0..64u8 {
        q.post(Event::KeyDown(i));
        assert_eq!(q.pending(), usize::from(i + 1) % 64);
    }
    assert_eq!(q.pending(), 0);
    let (got, r) = deliver(&mut q, 100);
    assert_eq!((got.len(), r), (0, Ok(())));
}

/// Pins `EventQueue::process` on an error: the refused event is consumed and the events after it
/// stay queued, in order, for the next call.
#[test]
fn a_refused_event_is_consumed_and_the_rest_stay_queued() {
    let mut q = EventQueue::new();
    for k in *b"abc" {
        q.post(Event::KeyDown(k));
    }
    let mut seen = Vec::new();
    let r = q.process(|e| {
        seen.push(e);
        if e == Event::KeyDown(b'b') {
            Err(InputError::WithheldKey(b'b'))
        } else {
            Ok(())
        }
    });
    assert_eq!(r, Err(InputError::WithheldKey(b'b')));
    assert_eq!(seen, [Event::KeyDown(b'a'), Event::KeyDown(b'b')]);
    assert_eq!(q.pending(), 1);
    let (got, r) = deliver(&mut q, 10);
    assert_eq!((got, r), (vec![Event::KeyDown(b'c')], Ok(())));
}

// ---------------------------------------------------------------------------------------
// Key codes (doomdef.h:250-277) and accessors

/// Pins the function-key and other key codes of doomdef.h:250-277 that the goldens never press:
/// `KEY_F1`..`KEY_F10` are `0x80+0x3b`..`0x80+0x44` and `KEY_F11`, `KEY_F12` are `0x80+0x57`,
/// `0x80+0x58`.
#[test]
fn the_function_key_codes_are_the_c_values() {
    let want: [u8; 12] = [
        0xbb, 0xbc, 0xbd, 0xbe, 0xbf, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xd7, 0xd8,
    ];
    assert_eq!(keys::KEY_F, want);
    assert_eq!(keys::KEY_RIGHTARROW, 0xae);
    assert_eq!(keys::KEY_LEFTARROW, 0xac);
    assert_eq!(keys::KEY_UPARROW, 0xad);
    assert_eq!(keys::KEY_DOWNARROW, 0xaf);
    assert_eq!(keys::KEY_ESCAPE, 27);
    assert_eq!(keys::KEY_ENTER, 13);
    assert_eq!(keys::KEY_TAB, 9);
    assert_eq!(keys::KEY_BACKSPACE, 127);
    assert_eq!(keys::KEY_PAUSE, 0xff);
    assert_eq!(keys::KEY_EQUALS, 0x3d);
    assert_eq!(keys::KEY_MINUS, 0x2d);
}

/// Pins `Input::controls`: it returns the controls the input was made with.
#[test]
fn input_returns_its_own_controls() {
    let modern = Preset::Modern.controls();
    assert_ne!(
        modern,
        Controls::vanilla(),
        "the preset must differ for this test"
    );
    assert_eq!(*Input::new(modern).controls(), modern);
    assert_eq!(
        *Input::new(Controls::vanilla()).controls(),
        Controls::vanilla()
    );
}

/// Pins `EventLayer::options`: it returns the options the layer was made with.
#[test]
fn the_layer_returns_its_own_options() {
    let p = Preset::Modern;
    let layer = EventLayer::new(&p.controls(), p.layer_options());
    assert_eq!(layer.options(), p.layer_options());
    assert_ne!(p.layer_options(), LayerOptions::default());
}

/// Pins the raw-input bound `MAX_PENDING` = 4 x `NUMKEYS` = 1024 (the code's own documented
/// bound): 1024 inputs between two drains are taken, the 1025th overflows.
#[test]
fn the_layer_takes_exactly_1024_raw_inputs_between_two_drains() {
    assert_eq!(NUMKEYS, 256);
    let mut l = EventLayer::new(&Controls::vanilla(), LayerOptions::default());
    for i in 0..1024 {
        assert_eq!(l.key(b'b', i % 2 == 0), KeyRoute::Game, "input {i}");
    }
    assert_eq!(l.key(b'b', true), KeyRoute::Overflowed);
    let mut m = EventLayer::new(&Controls::vanilla(), LayerOptions::default());
    for i in 0..1024 {
        assert!(m.mouse_button(MouseButton::Left, i % 2 == 0), "input {i}");
    }
    assert!(!m.mouse_button(MouseButton::Left, true));
}

/// Pins the per-build bound for keyups waiting in the raw queue: 100 held keys released at once
/// give at most 63 events a build, in order, and none is lost.
#[test]
fn many_physical_releases_are_spread_over_builds_of_at_most_63_events() {
    let mut l = EventLayer::new(&Controls::vanilla(), LayerOptions::default());
    let forwarded: Vec<u8> = (0..=u8::MAX)
        .filter(|k| !keys::is_withheld(*k))
        .take(100)
        .collect();
    for k in &forwarded {
        l.key(*k, true);
    }
    let mut downs = 0;
    for _ in 0..50 {
        let mut out = Vec::new();
        l.drain(&mut out);
        if out.is_empty() {
            break;
        }
        assert!(out.len() <= MAX_EVENTS_PER_BUILD);
        downs += out.len();
    }
    assert_eq!(downs, 100);
    for k in &forwarded {
        l.key(*k, false);
    }
    let mut ups = Vec::new();
    for _ in 0..50 {
        let mut out = Vec::new();
        l.drain(&mut out);
        if out.is_empty() {
            break;
        }
        assert!(out.len() <= MAX_EVENTS_PER_BUILD, "{} events", out.len());
        ups.extend(out);
    }
    let want: Vec<Event> = forwarded.iter().map(|k| Event::KeyUp(*k)).collect();
    assert_eq!(ups, want);
}

// ---------------------------------------------------------------------------------------
// Messages

/// Pins the text of the refusal messages the end screen shows.
#[test]
fn error_messages_read_as_written() {
    assert_eq!(
        InputError::WithheldKey(0x1b).to_string(),
        "key 0x1b is withheld from the game path"
    );
    assert_eq!(
        InputError::MouseOverflow.to_string(),
        "mouse motion overflows the genuine int scaling"
    );
    assert_eq!(
        RecordError::Cap.to_string(),
        "the session reached its tic cap"
    );
    assert_eq!(
        RecordError::NotReread.to_string(),
        "the executed command is not the demo's re-read command"
    );
    assert_eq!(
        RecordError::NotAdmitted.to_string(),
        "the command is outside the wire-24 domain"
    );
    assert_eq!(
        Witness::Input(InputError::WithheldKey(0x1b)).to_string(),
        "input: key 0x1b is withheld from the game path"
    );
    assert_eq!(
        Witness::Record(RecordError::Cap).to_string(),
        "recorder: the session reached its tic cap"
    );
    assert_eq!(
        Witness::Frame(FrameRefusal::Palette(14)).to_string(),
        "palette 14 is not in PLAYPAL"
    );
    assert_eq!(
        FrameRefusal::Palette(3).to_string(),
        "palette 3 is not in PLAYPAL"
    );
    let mut c = Controls::vanilla();
    let e = c.set("key_nonesuch", 1).expect_err("unknown name");
    assert!(matches!(e, ControlsError::UnknownName(_)));
    assert_eq!(e.to_string(), "unknown input setting \"key_nonesuch\"");
    assert_eq!(
        ControlsError::SensitivityOutOfRange(12).to_string(),
        "mouse_sensitivity 12 is not 0..=9"
    );
    assert_eq!(
        ControlsError::MouseButtonOutOfRange {
            name: "mouseb_fire",
            value: 5
        }
        .to_string(),
        "mouseb_fire 5 is not -1..=2"
    );
    assert_eq!(
        ControlsError::KeyOutOfRange {
            name: "key_up",
            value: 300
        }
        .to_string(),
        "key_up 300 is not 0..=255"
    );
    assert_eq!(
        ControlsError::WithheldKey {
            name: "key_up",
            value: 0x1b
        }
        .to_string(),
        "key_up 0x1b is a key the app withholds"
    );
}

// ---------------------------------------------------------------------------------------
// The recorder

/// Pins `Recorder::is_empty` and `len`: a new recorder is empty, one recorded tic is not.
#[test]
fn a_recorder_is_empty_until_a_tic_is_recorded() {
    let mut r = Recorder::new(1, 2, 10);
    assert!(r.is_empty());
    assert_eq!(r.len(), 0);
    let mut input = Input::new(Controls::vanilla());
    let tic = input.tic().expect("an idle tic");
    r.record(tic).expect("recorded");
    assert!(!r.is_empty());
    assert_eq!(r.len(), 1);
}

// ---------------------------------------------------------------------------------------
// The event layer's per-build rules (layer.rs, S6)

fn mouse(buttons: u8, dx: i32, dy: i32) -> Event {
    Event::Mouse { buttons, dx, dy }
}

fn drain(l: &mut EventLayer) -> Vec<Event> {
    let mut out = Vec::new();
    l.drain(&mut out);
    out
}

/// Pins the release of the only held mouse button: a release is applied whatever other buttons
/// are held, including when the released button is the only one down (so the buttons field
/// of the posted event goes back to 0).
#[test]
fn releasing_the_only_held_button_posts_a_release() {
    let mut l = EventLayer::new(&Controls::vanilla(), LayerOptions::default());
    l.mouse_button(MouseButton::Left, true);
    assert_eq!(drain(&mut l), [mouse(1, 0, 0)]);
    l.mouse_button(MouseButton::Left, false);
    assert_eq!(drain(&mut l), [mouse(0, 0, 0)]);
    l.mouse_button(MouseButton::Right, true);
    l.mouse_button(MouseButton::Middle, true);
    assert_eq!(drain(&mut l), [mouse(6, 0, 0)]);
    l.mouse_button(MouseButton::Middle, false);
    assert_eq!(drain(&mut l), [mouse(4, 0, 0)]);
    // a release of a button that is not down posts nothing
    l.mouse_button(MouseButton::Left, false);
    assert_eq!(drain(&mut l), []);
}

/// Pins the deferral of a button's inputs: after a press and a release in one build (the release
/// is held back), a further press in the same build is held back too, so the buttons the game
/// path ends with are the physical ones: the button is down and no release is ever posted.
#[test]
fn a_press_after_a_deferred_release_waits_for_it() {
    let mut l = EventLayer::new(&Controls::vanilla(), LayerOptions::default());
    l.mouse_button(MouseButton::Left, true);
    l.mouse_button(MouseButton::Left, false);
    l.mouse_button(MouseButton::Left, true);
    assert_eq!(drain(&mut l), [mouse(1, 0, 0)]);
    // the held-back release and press now run in order: down, up, down again, the net state is
    // unchanged and nothing is posted
    assert_eq!(drain(&mut l), []);
    assert_eq!(drain(&mut l), []);
    // the button is down: its release is posted
    l.mouse_button(MouseButton::Left, false);
    assert_eq!(drain(&mut l), [mouse(0, 0, 0)]);
}

/// Pins which key's keydown is held back after a keyup in one build with always-run: only the
/// inverted `key_speed` tap (its keyup then keydown is a tap, S7); any other key's release and
/// press in one build are both posted in that build, in order.
#[test]
fn only_the_speed_key_tap_is_deferred_under_always_run() {
    let p = Preset::Modern;
    let mut l = EventLayer::new(&p.controls(), p.layer_options());
    assert_eq!(drain(&mut l), [Event::KeyDown(keys::KEY_RSHIFT)]);
    l.key(b'a', true);
    assert_eq!(drain(&mut l), [Event::KeyDown(b'a')]);
    l.key(b'a', false);
    l.key(b'a', true);
    assert_eq!(drain(&mut l), [Event::KeyUp(b'a'), Event::KeyDown(b'a')]);
    // the speed key: a physical press is a keyup of the held key_speed; the matching release is
    // the keydown, held back to the next build
    l.key(keys::KEY_RSHIFT, true);
    l.key(keys::KEY_RSHIFT, false);
    assert_eq!(drain(&mut l), [Event::KeyUp(keys::KEY_RSHIFT)]);
    assert_eq!(drain(&mut l), [Event::KeyDown(keys::KEY_RSHIFT)]);
}
