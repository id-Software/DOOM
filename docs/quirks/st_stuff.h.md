# Quirks in st_stuff.h

[Back to the register index](../QUIRKS.md). The quirk register's rows for `st_stuff.h`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-st_stuff_h-32"></a>**Q-st_stuff_h-32** ST_HEIGHT is defined as `32*SCREEN_MUL` without parentheses, a macro that would mis-evaluate inside a larger expression. | st_stuff.h:32 | nothing observable in play (every use passes it whole) | not determined from the documents (M5B §1.4 notes it; no effect) | yes: add parentheses | M5B §1.4 (lines 1316-1317) |

## Notes on the st_stuff.h rows

- **Q-st_stuff_h-32**: latent only.
