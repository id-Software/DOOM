# Quirks in r_bsp.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `r_bsp.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-r_bsp-88"></a>**Q-r_bsp-88** R_ClipSolidWallSegment inserts and shifts clip ranges with no limit, so a view with more than about 30 separated solid wall ranges writes past `solidsegs[MAXSEGS]` (32). | r_bsp.c:88, 92, 120-133 (124-125); r_bsp.c:88-92, 122-133 (insert), 178-184 (crunch) | memory corruption or a crash on a very complex view (not stated beyond that) | refused (`SolidSegOverflow`, M5a decision R3; "refuses at the first out-of-bounds access, read or write", so the refusal can come one frame before the genuine write, as the owner accepted; room SOLIDSEGS) | yes: bound check or larger array | [COMPAT#software-ren…](../COMPATIBILITY.md#software-renderer-m5a) ("Vanilla limits (R3)"); M5A §1.8, R3, §7 item 29; VAL-M5A ("Limit rooms" SOLIDSEGS); VAL-M5C#runner-processes-that-end-on-a-signal (M5SSEG); M5C §3.1.6; [ROADMAP#limits-overf…](../ROADMAP.md#limits-overflows-and-quirks); commit 700ee5c; [M5A#18-vanilla-l…](../M5A_REFERENCE_INVESTIGATION.md#18-vanilla-limits-and-overflow-behaviour-linuxdoom-110-exactly), R3 (line 2403), finding 29 (lines 2524-2532); [COMPATIBILITY#software-ren…](../COMPATIBILITY.md#software-renderer-m5a) (lines 2755-2756); [VAL-M5A](../validation/M5A.md):114, 123, 239, 337; doom-render/src/lib.rs:115-116; doom-render/src/r_bsp.rs:10, 173-177; tools/reference/m5a_driver.c header item 9 and line 304 |
