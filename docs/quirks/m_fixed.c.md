# Quirks in m_fixed.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `m_fixed.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (2)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-m_fixed-62"></a>**Q-m_fixed-62** FixedDiv returns MININT/MAXINT whenever `(abs(a)>>14) >= abs(b)`, a deliberately conservative guard that saturates quotients from about 2^30 upward that would still fit in 32 bits, instead of dividing. | m_fixed.c:62-63 | not stated. | reproduced (M2): "The guard is deliberately conservative; do not replace it with a generic saturation check"; the saturation "is tested, not replaced" (archive) | no: it is the engine's arithmetic; results would change | [M2#confirmed-ob…](../M2_REFERENCE.md#confirmed-observations); [M3F2#source-and-r…](../M3F2_REFERENCE.md#source-and-rust-inventory) ("FixedDiv has its original magnitude guard"); docs/validation/archive/pre-m3d3j-compatibility.md#findings-from-the-preserved-source and #implemented-m2-behavior-and-evidence-limits |
| <a id="Q-m_fixed-69"></a>**Q-m_fixed-69** the active FixedDiv2 divides in IEEE double (`(double)a / (double)b * FRACUNIT`; the integer version is under `#if 0`), so its results depend on the floating-point unit (SSE binary64 against an i386 x87 build), and its out-of-range error says "FixedDiv: divide by zero" (unreachable through FixedDiv, which saturates first; FixedDiv2 has no other caller). | m_fixed.c:69-87 (verified; grep finds FixedDiv2 called only from m_fixed.c:64) | not stated. | reproduced with Rust's binary64 division; "FixedDiv precision (binary64 SSE versus an i386 x87 original build): open" (M5a §7 item 11); the reference build is SSE x86-64 | yes: the `#if 0` integer version | [M5A](../M5A_REFERENCE_INVESTIGATION.md) (summary, "the double division in FixedDiv2 (m_fixed.c:69-87)"; §4.4 tables; §7 item 11); [M3F2#source-and-r…](../M3F2_REFERENCE.md#source-and-rust-inventory) ("active binary64 FixedDiv2"); docs/validation/archive/pre-m3d3j-compatibility.md#findings-from-the-preserved-source |

## Notes on the m_fixed.c rows

- **Q-m_fixed-69**: crates/doom-types/src/fixed.rs comments cite the I_Error at m_fixed.c:83-84; it is at 84-85.
