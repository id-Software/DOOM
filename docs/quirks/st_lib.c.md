# Quirks in st_lib.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `st_lib.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (4)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-st_lib-99"></a>**Q-st_lib-99** STlib_drawNum erases its field with a rectangle sized from digit 0's width and height and ignoring patch offsets, so a digit patch drawn outside that box (another offset or size) leaves pixels behind when the number changes. | st_lib.c:99-100, 120-125 | a row of stale pixels beside a changing number when a digit patch is offset or larger than digit 0 (the synthetic font's digit 7 is one row low on purpose) | reproduced (M5b: the synthetic STTNUM7 leaves "a row ... behind when the number changes, as the genuine code does"; goldens match, M5B §2.4.2) | yes: erase the old digits' rectangles | M5B §1.4.8 step 2, §1.4.12 item 8, §2.4.2; VAL-M5B; tools/reference/m5b_gfx.py:15-21 |
| <a id="Q-st_lib-138"></a>**Q-st_lib-138** STlib_drawNum draws at most `width` digits (`while (num && numdigits--)`), so a value with more digits than the field shows only its low digits, with no sign of truncation. | st_lib.c:96, 138-143 | the high digits of an oversized value vanish (not reachable with Episode 1 values, per the documents' value ranges) | reproduced (the widget logic is ported and compared with the genuine engine, M5b) — the truncation case itself is not stated as exercised | yes: clamp or widen the field | M5B §1.4.8 step 2, §1.4.12 item 8 |
| <a id="Q-st_lib-212"></a>**Q-st_lib-212** STlib_updateMultIcon erases the old icon by copying from the background buffer, which holds STBAR but not STARMS (drawn straight to the foreground), using the previous patch's offsets and size, so after a weapon-slot digit changes the bar can differ from what a full redraw would show. | st_lib.c:212-240 (225-236); st_stuff.c:1308-1324 (arms widgets) | stray pixels or a different background behind weapon digits after pickups (two pixels remain in M5STARM after a screenblocks 11 to 10 switch) | reproduced (VAL-M5B step 3: goldens match the genuine engine; "A port that computes the bar from state alone is wrong after a weapon-digit change") | yes: keep STARMS in the background | M5B §1.4.8 step 6, §1.4.13, §1.4.14; VAL-M5B step 3 |
| <a id="Q-st_lib-223"></a>**Q-st_lib-223** STlib_updateMultIcon does nothing when the value turns to -1 (the old icon stays in the foreground and oldinum keeps the old index), and nothing when the bar is off. | st_lib.c:221-223 | a lost key icon would stay drawn (cannot happen inside a level; keys clear only between levels) | reproduced (the widget logic is ported and compared with the genuine engine, VAL-M5B step 3) | yes: erase on -1 | M5B §1.4.8 step 6, §1.4.11 item 4, §1.4.14 |

## Notes on the st_lib.c rows

- **Q-st_lib-99**: merges the erase half of number-widget-erase-box-uses-digit-zero-and-drops-high-digits with status-number-erase-uses-digit-zero-size.
