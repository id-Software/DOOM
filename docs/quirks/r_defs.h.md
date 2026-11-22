# Quirks in r_defs.h

[Back to the register index](../QUIRKS.md). The quirk register's rows for `r_defs.h`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-r_defs_h-88"></a>**Q-r_defs_h-88** Sector sound origins are `degenmobj_t` structs passed as `void*` and read as `mobj_t*`, which relies on x and y sitting at the same offsets in both structs (formally an aliasing violation). | r_defs.h:88-95; p_mobj.h:207-215; s_sound.c:268 (verified); s_sound.c:529 (cited by the document; that line casts the listener) | nothing observable with gcc's layout | reproduced in effect under wire 25 (the sector origin supplies x and y through a typed sector identity) | yes: pass a common position struct | [M5C#1113-undefin…](../M5C_REFERENCE_INVESTIGATION.md#1113-undefined-implementation-defined-or-layout-dependent-behaviour) (item 4, lines 936-941) and 1.2.2 kind 4 (line 1107); doom-sim/src/sound.rs; doom-sim/src/wire25_tests.rs (`sector_origins_follow_p_group_lines`); doom-sound/src/logic.rs (`Positions::sector_xy`) |
