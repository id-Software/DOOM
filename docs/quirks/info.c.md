# Quirks in info.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `info.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-info-40"></a>**Q-info-40** `char *sprnames[NUMSPRITES]` has exactly NUMSPRITES entries and no NULL terminator, while R_InitSpriteDefs counts names until a NULL, so it reads at least one word past the array (into whatever object follows). | info.c:40-55, r_things.c:190-192 | nothing observable in the genuine build, where the following word happens to be zero (observed by symbol addresses: `states[]` follows, with padding zero). | not determined from the documents for the port; the C harness passes a NULL-terminated copy, "a declared deviation from calling P_Init", equivalent to vanilla when the word after sprnames is NULL (M5a decision R11) | yes: add a NULL terminator | [M5A](../M5A_REFERENCE_INVESTIGATION.md) §3.2 ("sprnames is not NULL-terminated"), R11, and the list of declared deviations ("sprnames terminator"); tools/reference/m5a_driver.c header item 1 (about lines 24-30, line from the sweep); tools/reference/m5b_driver.c (about lines 62-68, line from the sweep) |
