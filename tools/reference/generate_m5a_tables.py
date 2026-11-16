#!/usr/bin/env python3
"""Generate the M5a renderer's trigonometric tables from linuxdoom-1.10/tables.c.

Writes crates/doom-render/src/tables.rs (decision R14 of docs/M5A_REFERENCE_INVESTIGATION.md):

  FINE_TANGENT_SINE  finetangent[4096] (tables.c:67) followed by finesine[10240] (tables.c:584) in
                     one array, the layout the genuine strict and strict32 builds have and the M5a
                     runner checks at start (finesine immediately follows finetangent), so that the
                     genuine negative finesine reads of r_main.c:484-485 and r_segs.c:266 land in
                     finetangent exactly as in the genuine build (decision R12);
  TAN_TO_ANGLE       tantoangle[2049] (tables.c:1870).

The values are copied literally from the preserved source text; nothing is recomputed. The existing
table generators (generate.py, generate_m3a.py) and their outputs are untouched: doom-sim keeps its
private copies, and the renderer does not edit the simulation crate.

  python3 tools/reference/generate_m5a_tables.py           # write
  python3 tools/reference/generate_m5a_tables.py --check   # require the committed bytes
"""
from pathlib import Path
import argparse
import re

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "linuxdoom-1.10" / "tables.c"
TARGET = ROOT / "crates" / "doom-render" / "src" / "tables.rs"


def table(text, declaration):
    match = re.search(re.escape(declaration) + r"\s*=\s*\{(.*?)\}", text, re.S)
    if not match:
        raise SystemExit(f"{declaration} not found in {SOURCE}")
    return [int(n) for n in re.findall(r"-?\d+", match.group(1))]


def rows(values):
    return ["    " + ", ".join(map(str, values[i:i + 12])) + "," for i in range(0, len(values), 12)]


def generate():
    text = SOURCE.read_text()
    tangent = table(text, "int finetangent[4096]")
    sine = table(text, "int finesine[10240]")
    angles = table(text, "angle_t tantoangle[2049]")
    assert len(tangent) == 4096 and len(sine) == 10240 and len(angles) == 2049
    # Spot values the Rust tests repeat (first and last entries of each table).
    assert tangent[0] == -170910304 and tangent[4095] == 170910304
    assert sine[0] == 25 and angles[0] == 0 and angles[2048] == 536870912
    lines = [
        "// Generated from preserved id Software Linux DOOM 1.10; GPL-2.0-only.",
        "// Reproduce with python3 tools/reference/generate_m5a_tables.py.",
        "//",
        "// finetangent[4096] (tables.c:67) immediately followed by finesine[10240] (tables.c:584),",
        "// the contiguous layout of the genuine build (decision R12).",
        "#[rustfmt::skip]",
        "pub(crate) static FINE_TANGENT_SINE: [i32; 14336] = [",
    ]
    lines += rows(tangent + sine)
    lines += [
        "];",
        "",
        "// tantoangle[2049] (tables.c:1870).",
        "#[rustfmt::skip]",
        "pub(crate) static TAN_TO_ANGLE: [u32; 2049] = [",
    ]
    lines += rows(angles)
    lines.append("];")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="require the committed file instead of writing it")
    args = parser.parse_args()
    value = generate()
    if args.check:
        if not TARGET.exists() or TARGET.read_text() != value:
            raise SystemExit(f"table mismatch: {TARGET.relative_to(ROOT)}")
        print(f"Checked {TARGET.relative_to(ROOT)} against linuxdoom-1.10/tables.c.")
    else:
        TARGET.write_text(value)
        print(f"Generated {TARGET.relative_to(ROOT)} from linuxdoom-1.10/tables.c.")


if __name__ == "__main__":
    main()
