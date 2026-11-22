# Quirks in w_wad.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `w_wad.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (2)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-w_wad-370"></a><a id="Q-w_wad-370a"></a>**Q-w_wad-370a** W_AddFile stores directory names as given, but W_CheckNumForName upper-cases only the query before comparing raw bytes, so a lump whose directory name has a lower-case letter can never be found by name. | w_wad.c:221; w_wad.c:351-390 (364-370, 381-382) | not stated (a PWAD lump with a lower-case name is ignored) | refused at renderer load (`ResourceError::DirectoryNameCase`, M5a; VAL-M5A unit test); within the admitted directory the renderer lookup compares raw names to an upper-cased query, as the C does (M5B) | yes: upper-case names at load | M5A §4.3 ("Name-matching difference"); VAL-M5A (unit tests, `DirectoryNameCase`); M5B line 4677; M5C §1.3.2 (line 1824, lookup "case-insensitive"); doom-render/src/r_data.rs:20-23, 156, 183; docs/validation/archive pre-m3d3j-compatibility.md#implemented-m1-policies-and-deviations |
| <a id="Q-w_wad-370b"></a>**Q-w_wad-370b** W_AddFile stores directory names as given, but W_CheckNumForName upper-cases only the query before comparing raw bytes, so a lump whose directory name has a lower-case letter can never be found by name. | w_wad.c:221; w_wad.c:351-390 (364-370, 381-382) | not stated (a PWAD lump with a lower-case name is ignored) | not reproduced (declared general doom-wad lookup): the general archive lookup folds case on both sides (M5A §4.3); the archived M1 policy says "strict lowercase-directory bug emulation is not claimed" | yes: upper-case names at load | M5A §4.3 ("Name-matching difference"); VAL-M5A (unit tests, `DirectoryNameCase`); M5B line 4677; M5C §1.3.2 (line 1824, lookup "case-insensitive"); doom-render/src/r_data.rs:20-23, 156, 183; docs/validation/archive pre-m3d3j-compatibility.md#implemented-m1-policies-and-deviations |

## Notes on the w_wad.c rows

- **Q-w_wad-370**: The Rust comment attributes the refusal to "decision R14", which in M5A is the crate-layout decision, not a case rule.
