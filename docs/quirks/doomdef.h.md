# Quirks in doomdef.h

[Back to the register index](../QUIRKS.md). The quirk register's rows for `doomdef.h`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-doomdef_h-80"></a>**Q-doomdef_h-80** the comment says the integrated sound is "experimental, and unfinished. Default is synchronous.", yet `#define SNDSERV 1` is set, so the shipped build sends every start to an external sndserver program and the in-process mixer in i_sound.c is compiled out. | doomdef.h:77-85; i_sound.c:482-500; d_main.c:397-405 | the shipped binary plays no sound without a running sndserver (inferred). | not reproduced as shipped (declared): every sound claim is "linuxdoom-1.10 built with its in-process sound code (one define removed)" (decision T2, [COMPATIBILITY#the-referenc…](../COMPATIBILITY.md#the-reference)); the M5c tools model both builds (`--handle integrated`, `--handle sndserv`). | yes: fix the comment or remove the define | [COMPATIBILITY#the-referenc…](../COMPATIBILITY.md#the-reference); M5C note 1.1.0 and Summary, 1.3.0 item 2 (ChangeLog:15-19, README.sound:75-83), 3.2.1 item 1; tools/reference/m5c_build.py:20-21, 66-90; m5c_mixer_build.py:21, 68-73 |
