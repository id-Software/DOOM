# Quirks in i_system.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `i_system.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (2)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-i_system-88"></a>**Q-i_system-88** I_GetTime uses gettimeofday with a function-static base second and integer truncation, so tics are aligned to whole seconds (28,571 or 28,572 microseconds long) and a system clock step changes the tic count. | i_system.c:88-100 | a clock change can jump or stall the game (inferred). | not reproduced (declared): M5B note S8 (i) "35 Hz from a monotonic clock with vanilla's integer arithmetic" | yes: a monotonic clock | M5B note 1.3.3 items 2-3 |
| <a id="Q-i_system-116"></a>**Q-i_system-116** I_Quit does not call G_CheckDemoStatus, so quitting from the menu or through the SIGINT handler writes no demo, and when the demo is written the return value of M_WriteFile is ignored. | i_system.c:116-124; g_game.c:1677-1683 | the recording silently disappears when the game is quit normally, or the file is not written. | not reproduced (declared session recording; the app attempts to save on quit and exposes the outcome) | yes: finish the demo in I_Quit | M5B note 1.2.2 item 8, 1.3.7 item 9; [VAL-M5D](../validation/M5D.md) "The C citations for the marker" and A10 Question 1 ("I_Quit does not call it"); [COMPATIBILITY](../COMPATIBILITY.md#input); doom-game/src/app/mod.rs (`quit`, `save_status`); doom-game/tests/app.rs (`keys_on_the_end_screen_and_quit`) |

## Notes on the i_system.c rows

- **Q-i_system-116**: the SIGINT handler path was not opened for this row (taken from the M5B note).
