# Quirks in p_ceilng.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `p_ceilng.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-p_ceilng-251"></a>**Q-p_ceilng-251** when all 30 activeceilings slots are in use, P_AddActiveCeiling returns without registering the new ceiling and without any error, unlike lifts and buttons, which stop the game with I_Error. | p_ceilng.c:251-263, p_spec.h:518, p_plats.c:298, p_switch.c:189; p_ceilng.c:251-263 (loop 255-262) | nothing in Episode 1 ("no ceilings in Episode 1") | not yet ported (ceilings are "not needed" for Episode 1, M4a; ROADMAP checklist: "ceilings not started") | yes: I_Error like the plat table | [ROADMAP#vanilla-comp…](../ROADMAP.md#vanilla-completeness-checklist) (row "Tables that stop the original when full: 30 lifts, 30 ceilings, 16 buttons"); [M4A#313-unchecke…](../M4A_REFERENCE_INVESTIGATION.md#313-unchecked-bounds-undefined-reads-and-hazards-a-rust-witness-must-guard) (line 1219), 3.1.9 (lines 640-641), line 1823 |

## Notes on the p_ceilng.c rows

- **Q-p_ceilng-251**: the ROADMAP row groups ceilings with lifts and buttons as tables "that stop the original when full"; the C stops (I_Error) for lifts (p_plats.c:298) and buttons (p_switch.c:189) but not for ceilings.
