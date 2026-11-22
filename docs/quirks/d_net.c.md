# Quirks in d_net.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `d_net.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (5)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-d_net-377"></a>**Q-d_net-377** the first TryRunTics sees as new tics the sub-second phase of the first I_GetTime call plus the time I_InitGraphics took, so vanilla's first displayed frame shows gametic 1 to 5 depending on the clock. | d_net.c:377-410; g_game.c:486; d_main.c:367 | the very first frame can be at a different game time between runs. | not reproduced (declared): M5B note S8 "The first frame shown is the one after tic 1, as in the harness" | yes: start the clock after initialisation | M5B note 1.3.2 "The first frame of a session", 1.3.8 question 5, 4 S8 |
| <a id="Q-d_net-381"></a>**Q-d_net-381** NetUpdate jumps to `listen:` without pumping events when no new tic is due, and the pass that hits the build cap pumps events without building, so mouse motion arriving during a long frame lands in one command. | d_net.c:381-382, 399-410 | not stated (a burst of mouse motion in a slow frame becomes one large turn on the first command of the batch). | not reproduced (declared): M5B note S4/S8, one accumulated mouse event per command build | no: inherent in polled input | M5B note 1.3.2 NetUpdate items 1 and 3, "Slow frames"; 1.3.7 item 4 |
| <a id="Q-d_net-404"></a>**Q-d_net-404** NetUpdate stops building once maketic-gameticdiv reaches BACKUPTICS/2-1, so at most 5 tics run per displayed frame and the clock time beyond that is never recovered. | d_net.c:399-410 (verified; the test is at 404-405) | after a long frame the game runs behind real time for good (a 30-tic frame loses 25 tics). | reproduced as a bound: M5B note S8 (i) "if more than 5 tics are due, run 5 and drop the rest (vanilla's bound)"; M5B record step 6 "5 due run 5; 6 due run 5 and drop 1; 30 late run 5 and drop 25" | yes: carry the excess to later frames | M5B note 1.3.2 "Slow frames", 1.3.7 item 3, 4 S8; M5B record step 6 |
| <a id="Q-d_net-669"></a>**Q-d_net-669** TryRunTics runs realtics+1 tics when realtics < availabletics-1, but only realtics when realtics == availabletics-1, an odd middle case between the two neighbours. | d_net.c:668-677 | not stated (one tic fewer than available in that case). | not reproduced (declared): M5B note S8 (i) uses its own monotonic due-tic rule; option (ii) "port TryRunTics exactly" was not taken | no: a pacing heuristic of the original | M5B note 1.3.2 item 4, 1.3.7 item 3, 4 S8 |
| <a id="Q-d_net-715"></a>**Q-d_net-715** the TryRunTics wait loop spins on NetUpdate and I_GetTime with no sleep, so the original burns a core while waiting; I_WaitVBL, which sleeps, is called only by D_QuitNetGame and the quit sound. | d_net.c:714-733; i_system.c:126-137; d_net.c:620; m_menu.c:1090 | not stated (full CPU use while waiting). | not reproduced (declared): M5B note S8 (i) "the loop sleeps until the next tic is due" | yes: sleep until the next tic | M5B note 1.3.2 item 8, 1.3.3 item 4, 1.3.8 question 2 |

## Notes on the d_net.c rows

- **Q-d_net-377**: the note labels it an inference from those lines.
- **Q-d_net-669**: the note's probe `loop.c` is "a model, not the genuine engine".
