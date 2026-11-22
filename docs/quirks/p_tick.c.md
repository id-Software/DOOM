# Quirks in p_tick.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `p_tick.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-p_tick-113"></a>**Q-p_tick-113** P_RunThinkers unlinks and Z_Free's a removed thinker and then reads `currentthinker->next` from the freed block, which works only because the zone allocator's Z_Free leaves the block's bytes in place. | p_tick.c:101-122 (removal test 108, Z_Free 113, read 120) | nothing observable in play | reproduced (M2 onward) in effect: the traversal order is kept "without reproducing use-after-free" (the Rust store saves the next handle before retirement); the C reference adapters retain freed bytes until traversal ends (M3c, M3f8: "Replacing this with libc free is not a valid observer adaptation") | yes: read next before Z_Free | [M2#ambiguities-…](../M2_REFERENCE.md#ambiguities-and-deliberate-safe-policies) (lines 62-64); [M3C#original-sou…](../M3C_REFERENCE.md#original-source-findings) (lines 113, 158); [M3F8#source-findi…](../M3F8_REFERENCE.md#source-findings-and-classification) (lines 33-34); [M3H9C#27-p_ticker-…](../M3H9C_REFERENCE_INVESTIGATION.md#27-p_ticker-and-p_playerthink) (lines 312-313); [M3G10](../M3G10_REFERENCE_INVESTIGATION.md); tools/reference/m3c_driver.c:3; tools/reference/m3d1_shim.h:4, m3d2_shim.h:4, m3d2b_shim.h:4, m3d3a-j shims line 4; tools/reference/generate.py:168; archive pre-m3d3j-compatibility.md#implemented-m2-behavior-and-evidence-limits, pre-m3d3j-ledger.md#m2-reference-comparison-evidence, pre-m3d3j-reference-tools.md, pre-m3d3j-architecture.md |

## Notes on the p_tick.c rows

- **Q-p_tick-113**: The M3H9C note words the safety reason as "`Z_Free` writes the block header alone"; M3F8 says Z_Free "marks/coalesces zone blocks without releasing the backing heap"; both say the freed thinker's next field survives.
