# Quirks in wi_stuff.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `wi_stuff.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (2)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-wi_stuff-588"></a>**Q-wi_stuff-588** WI_drawAnimatedBack tests `if (commercial)`, the enum constant (non-zero), instead of `gamemode == commercial`, so it always returns and the episode maps' animations never appear, while WI_updateAnimatedBack, which tests gamemode correctly, still runs and draws M_Random. | wi_stuff.c:583-589 (588); wi_stuff.c:537; M_Random at wi_stuff.c:523, 525, 561 (as cited by M5C §1.4.1) (verified for 537 and 588) | the intermission map's animations (flames, lights) never appear | not yet ported (intermission "not started"; ROADMAP: "whose drawing never runs, `wi_stuff.c:588`, while their random draws do") | yes: compare gamemode | [ROADMAP#intermission…](../ROADMAP.md#intermission-and-finale) |
| <a id="Q-wi_stuff-1482"></a>**Q-wi_stuff-1482** WI_checkForAccelerate sets player->attackdown and usedown from the command during the intermission, and nothing resets them at the next level's spawn, so a button held through the intermission carries into the next level (feeding the rampage-face counter and the refire rules). | wi_stuff.c:1470-1498 (1482-1487, 1490-1495) (verified for the writes; the absence of a reset was not re-checked) | not stated by M4A; M5B: the rampage face may fire early on a new level if fire was held through the intermission | not yet ported (no intermission in M5b, S1/S11; level transitions are a later direction, M4A §8; ROADMAP intermission "not started") | yes: reset them at level start | [M4A#3111-normal-…](../M4A_REFERENCE_INVESTIGATION.md) (around line 1175); M5B §1.4.14 last bullet, §1.4.15 question 8 |
