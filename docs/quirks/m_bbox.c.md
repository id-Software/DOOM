# Quirks in m_bbox.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `m_bbox.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-m_bbox-51"></a>**Q-m_bbox-51** M_AddToBox tests `x<left` and only otherwise `x>right` (the same for y), so the first point sets only left and bottom; in a closed sector later vertices fix the box, but a sector with an unclosed outline keeps BOXRIGHT (or BOXTOP) at MININT and P_GroupLines puts its sound origin at a wrong point (as low as (MININT+left)/2). | m_bbox.c:51-58, p_setup.c:539-557 | a sector sound (door, lift, floor) of a malformed sector comes from the wrong place; not stated further (only matters for malformed sectors). | reproduced (wire 25, M5c): "computed once at load as P_GroupLines does (p_setup.c:536-557)"; tests `sector_origin_box_arithmetic`, `add_to_box_second_point_with_the_same_x_sets_the_right_edge` | yes: two independent ifs | M5C §1.2.6.2; [COMPAT#original-epi…](../COMPATIBILITY.md#original-episode-1-part-3-canonical-sound-records-wire-25) ("Sector sound origins"); doom-sim/src/sound.rs:305, 326-337 (comments); doom-sim/src/wire25_tests.rs:122-125, 829-833 (tests); commits 980af26 "M5c step 3a: unit tests of wire 25's sector origins ...", 5c0495e "M5c step 3a: wire 25 ..." |

## Notes on the m_bbox.c rows

- **Q-m_bbox-51**: M5C §1.2.6.2 labels the consequence an inference and adds that the origin uses a signed /2 (truncation toward zero, implementation-defined in C89) and that soundorg.z and soundorg.thinker are never set (unused by the call sites).
