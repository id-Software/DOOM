#!/usr/bin/env python3
"""Generate the version-110 demo lumps of the M4b synthetic runs from text scripts.

The script format is tools/e1m1_scripts.py's (imported read-only as a library: idle, cmd,
seed ... chaos, `map ExMy`, `# output: NAME.lmp`, the same command-domain checks and the
same lump layout) plus one directive:

  skill N        the demo header's skill byte (header byte 1; G_DoPlayDemo reads it,
                 g_game.c:1596, and G_InitNew uses it, g_game.c:1615); N is the 0-based
                 gameskill and M4b admits 2 (medium) and 3 (hard, decision I4); at most
                 once, before the first tic line; default 2

so a script without `skill` produces exactly the bytes tools/e1m1_scripts.py produces.
tools/e1m1_scripts.py itself is unchanged and keeps checking its own directories; this
tool checks tests/reference/m4b_scripts/ and the M4b real-map script directories
tests/reference/e1m5_scripts/, e1m8_scripts/ and e1m9_scripts/ (M4b step 7; those
present), which tools/e1m1_scripts.py does not read.

    m4b_scripts.py SCRIPT.txt [-o OUT.lmp]
    m4b_scripts.py --check            regenerate every *.txt of those directories in memory
                                      and compare with the committed .lmp next to it
"""
import argparse, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import e1m1_scripts as E   # noqa: E402  frozen script format (read-only)

SCRIPT_DIR = ROOT / 'tests/reference/m4b_scripts'
REAL_DIRS = [ROOT / f'tests/reference/e1m{n}_scripts' for n in (5, 8, 9)]
SKILLS = (2, 3)


def split_skill(text):
    """(skill, the script lines without the skill line)."""
    lines, skill, seen_tic = text.splitlines(), None, False
    rest = []
    for number, raw in enumerate(lines, 1):
        words = raw.split('#', 1)[0].split()
        if words and words[0] == 'skill':
            if len(words) != 2 or not words[1].isdigit() or int(words[1]) not in SKILLS:
                raise E.ScriptError(f'line {number}: expected "skill N" with N in {SKILLS}')
            if skill is not None: raise E.ScriptError(f'line {number}: a second skill line')
            if seen_tic: raise E.ScriptError(f'line {number}: the skill line must precede every tic line')
            skill = int(words[1])
            rest.append('')                      # keep line numbers of later errors
            continue
        if words and words[0] != 'map': seen_tic = True
        rest.append(raw)
    return (2 if skill is None else skill), rest


def lump(text):
    skill, lines = split_skill(text)
    data, tics = E.lump('\n'.join(lines))
    out = bytearray(data)
    out[1] = skill
    return bytes(out), tics


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('script', nargs='?', type=Path)
    ap.add_argument('-o', '--out', type=Path)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    try:
        if a.check:
            sources = sorted(SCRIPT_DIR.glob('*.txt'))
            if not sources: sys.exit(f'no scripts under {SCRIPT_DIR.relative_to(ROOT)}')
            for d in REAL_DIRS:
                if d.is_dir():
                    real = sorted(d.glob('*.txt'))
                    if not real: sys.exit(f'no scripts under {d.relative_to(ROOT)}')
                    sources += real
            bad = []
            for src in sources:
                out = E.default_output(src); E.lump_name(out)
                data, tics = lump(src.read_text())
                if not out.exists() or out.read_bytes() != data: bad.append(out.name)
                else: print(f'{src.parent.name}/{src.name}: {tics} tics, skill {data[1]}, {out.name} matches')
            if bad: sys.exit('differs or missing: ' + ', '.join(bad))
            return
        if not a.script: ap.error('a script or --check is required')
        out = a.out or E.default_output(a.script); E.lump_name(out)
        data, tics = lump(a.script.read_text())
        out.write_bytes(data)
        print(f'{out}: {tics} tics, skill {data[1]}, {len(data)} bytes, lump {E.lump_name(out)}')
    except E.ScriptError as e:
        sys.exit(f'error: {e}')


if __name__ == '__main__':
    main()
