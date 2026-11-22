# Quirks in sounds.h

[Back to the register index](../QUIRKS.md). The quirk register's rows for `sounds.h`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-sounds_h-37"></a>**Q-sounds_h-37** sfxinfo_t has a `singularity` field, set true on 23 table entries, but no code in linuxdoom-1.10 reads it, so the promised one-at-a-time rule never applies. | sounds.h:37-38; sounds.c:117-226 (initialisers) (verified: grep finds only the declaration) | nothing observable in play (the intended single-instance rule never applies) | reproduced ("`singularity` as data (no code in linuxdoom-1.10 reads it", crates/doom-sound/src/sounds.rs:5-6, M5c) | yes: enforce it in S_getChannel (changes behaviour) | M5C §1.1.3 (table, lines 148, 170); doom-sound/src/sounds.rs:5-6, 16; doom-sound/tests/tables_and_lumps.rs:53 |

## Notes on the sounds.h rows

- **Q-sounds_h-37**: merges sfx-singularity-field-read-by-nothing (its priority half is the i_sound.c:479 row) and sfx-singularity-field-never-read.
