#!/usr/bin/env python3
"""Local-only M5b session check on a real map (never in CI; it reads the commercial IWAD).

The mode of tools/reference/m5b_session.py for IWAD maps and for sessions a person played in doom-play (M5b
investigation section 2.3, decision S22). Given the IWAD and either a session script (as
tests/reference/m5b_sessions/*.txt; its `map` and `skill` lines are used, its `wad` lines ignored) or a session demo
written by doom-play, it plays the script through the headless app (or replays the demo through the Rust simulation and
the app's display), replays the demo through the genuine M5b runner in the strict, overflow and address variants, and
compares every tic: the executed command, the frame digest, the palette, palette set, face, message flag and M_Random
index, the messages and the masked game dump (the independent DSIM40 encoder). With a script, the app's demo must also
equal the one the genuine input driver records from the app's events (the driver's header is E1M1 at skill 2; for any
other map and skill only the commands are compared).

Nothing is committed: everything stays in --work. The output is counts and pass/fail facts only (no IWAD content).

    python3 tools/local/m5b_session_real.py --iwad PATH (--lmp SESSION.lmp | --script FILE) [--wad PATH]...
                                            [--screenblocks 10|11] [--variants strict,overflow,address] [--work DIR]
"""
import argparse, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5b_session as S  # noqa: E402


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--iwad', required=True, type=Path)
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument('--lmp', type=Path); g.add_argument('--script', type=Path)
    ap.add_argument('--wad', type=Path, action='append', default=[], help='an extra WAD after the IWAD (repeatable)')
    ap.add_argument('--screenblocks', type=int, default=10)
    ap.add_argument('--variants', default=','.join(S.VARIANTS))
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5b-session-real')
    ap.add_argument('--cc', default='cc'); ap.add_argument('--cargo', default='cargo')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    h = S.header(a.script.read_text()) if a.script else dict(map=1, skill=2)
    rec = S.verify(S.build(work, a.cc, a.cargo), [a.iwad.resolve()] + [w.resolve() for w in a.wad], a.screenblocks, work / 'run',
                   a.script and a.script.resolve(), a.lmp and a.lmp.resolve(), h['map'], h['skill'], a.variants.split(','))
    c = rec.get('counts', {})
    print(f"tics {rec.get('tics')}; commands {c.get('commands')}; frames {c.get('frames')} (digests {c.get('digests')}, "
          f"masked {c.get('masked')}, non-comparable {c.get('non_comparable')}); status fields {c.get('fields')}; "
          f"messages {c.get('messages')}; game dumps {c.get('dump_tics')}; variants {a.variants} identical "
          f"{rec.get('variants_identical')}; genuine end {rec.get('genuine_end')}")
    for t, kind, text in rec['failures'][:10]:
        print(f'  tic {t}: {kind}: {str(text)[:300]}')
    if rec['ok']:
        print(f"VERDICT: PASS: every tic and frame {S.CONTRACT}")
    else:
        t, kind, _ = rec['failures'][0]
        print(f'VERDICT: FAIL: first difference at tic {t} ({kind})')
    return 0 if rec['ok'] else 1


if __name__ == '__main__':
    sys.exit(main())
