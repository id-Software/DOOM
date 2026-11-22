# Quirks in r_sky.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `r_sky.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-r_sky-55"></a>**Q-r_sky-55** R_InitSkyMap's comment says it is "Called whenever the view size changes", but it is called once from R_Init and its skyflatnum line is commented out; it only sets skytexturemid. | r_sky.c:53-61; r_main.c:788 (only call) | nothing observable in play | reproduced in effect; the renderer uses the fixed sky texture midpoint independent of view size | yes: fix the comment | M5A §1.5; doom-render/src/r_sky.rs (`SKYTEXTUREMID`) |
