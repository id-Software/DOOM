# Quirks in hu_lib.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `hu_lib.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (3)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-hu_lib-114"></a>**Q-hu_lib-114** HUlib_drawTextLine passes a plain char to toupper, so on a signed-char platform a byte 128-255 is a negative argument, which is undefined. | hu_lib.c:108, 114 | nothing observable in play (no reachable message contains such a byte). | not reproduced (declared HUD message domain; fixed messages use ASCII uppercasing on unsigned bytes) | yes: cast to unsigned char | M5B note 1.5.10 item 1; [COMPATIBILITY](../COMPATIBILITY.md#status-bar-hud-and-palette-choice); doom-render/src/hu_lib.rs (`to_ascii_uppercase`), hu_stuff.rs (`message_text`) |
| <a id="Q-hu_lib-120"></a>**Q-hu_lib-120** HUlib_drawTextLine bounds glyphs by the literal '_' rather than HU_FONTEND, advances by the glyph width ignoring its left offset, ends the whole line at the first glyph whose right edge passes column 320 (even if later characters are narrower), and ends it when a 4-pixel space reaches column 320. | hu_lib.c:99-139 (verified; break at 120-121, space advance at 125-130) | a message wider than the screen is cut off at the glyph that passes the right edge (from the C; which of the real messages are that wide is not stated). | reproduced (M5B record steps 1 and 3: goldens, the HUD space-advance mutation caught; M5B note 2.7 "A space advances 4 pixels") | yes: skip, not stop, at a wide glyph | M5B note 1.5.6 item 3, 1.5.12, 2.7; M5B record steps 1 and 3 |
| <a id="Q-hu_lib-154"></a>**Q-hu_lib-154** at screenblocks 10 and 11 viewwindowx is 0, so HUlib_eraseTextLine changes no pixels and the border redraw is skipped, and any view pixel the renderer does not write keeps old message-glyph pixels. | hu_lib.c:154-169; d_main.c:285 | message text can stay visible in hall-of-mirrors gaps after it should have gone. | reproduced (M5B note 3.8: the view is drawn over "the *composed* previous screen"; M5B record step 3 goldens match); a HOM room with a message is listed as a risk to test (M5B note 3.16 risk 2). | yes: clear the view area | M5B note 1.5.6 items 7 and 9, 3.8, 3.16 risk 2; M5B record step 3 |

## Notes on the hu_lib.c rows

- **Q-hu_lib-154**: the note labels it an inference relying on the renderer leaving HOM pixels untouched.
