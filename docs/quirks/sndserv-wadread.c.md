# Quirks in sndserv/wadread.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `sndserv/wadread.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

The C for this file is in the top-level `sndserv/` folder of the repository, not in `linuxdoom-1.10/`.

## Rows (2)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-wadread-206"></a>**Q-wadread-206** The shipped sound server finds lumps by a forward scan (first match wins, where w_wad.c scans backward so PWADs win), opens its own IWAD by fixed names under DOOMWADDIR and ignores the game's -file PWADs. | sndserv/wadread.c:206-210; sndserv/soundsrv.c:320-366 (verified); soundsrv.c:430, 478, 629 (channelstart, as cited, not verified) | replacement sounds in PWADs are not heard through the shipped server (inferred) | not yet ported — out of scope: the declared reference is the in-process build (decision T2); M5C §3.2.1 item 5 ("[inf]"; not built or run) | yes: scan backward and take the game's WAD list | M5C §3.2.1 item 5, §1.3.11 Q1; [COMPAT#the-referenc…](../COMPATIBILITY.md#the-reference) |
| <a id="Q-wadread-245"></a>**Q-wadread-245** In the sound server, a missing `ds` lump makes loadlump return 0 without setting `*size`, so getsfx computes the padding from an uninitialised int and reallocs a NULL pointer to that garbage size (undefined); there is no dspistol fallback as in the in-process getsfx. | sndserv/wadread.c:212-217 (not found), 239, 245-251 | a crash or garbage for a missing sound lump with the shipped server (for example Doom II names in a Doom 1 IWAD) | not yet ported — out of scope (the server is not the reference, decision T2; M5C §3.2.1 item 5, read only, "nothing built or run") | yes: initialise size and fall back | M5C §3.2.1 |
