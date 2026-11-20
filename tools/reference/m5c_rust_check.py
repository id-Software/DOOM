#!/usr/bin/env python3
"""The wire-24 Rust simulation against the genuine engine on every M5c sound room (tests/reference/m5c_manifest.json).

The M5c rooms must stay inside what the wire-24 simulation admits, because the later Rust sound steps run on
exactly these rooms and scripts. This wrapper runs the existing comparison unchanged (nothing of it is edited):

  genuine   the frozen M4b runner (tools/reference/m4b_build.py, strict, built under --work) on the room WAD and
            the script's demo lump, writing its dumps;
  Rust      the existing example `player_level_run --policy episode1v2 --captures` (crates/doom-sim, built by the
            caller: pass --binary, for instance $CARGO_TARGET_DIR/release/examples/player_level_run);
  compare   the existing comparator tools/reference/level_compare_m4b.py (the independent DSIM40 encoder) with its
            `--json` result.

Every script must `match` (every C snapshot compared, the map group equal, no divergence, no projection stop);
any other result fails, naming the first divergent tic and field. Nothing is written into the repository; the
dumps, captures and comparator results stay under --work.

    m5c_rust_check.py --binary PATH [--work DIR] [--only LUMP[,LUMP]] [--cc CC]
"""
import argparse, json, os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m4b_build             # noqa: E402

MANIFEST = ROOT / 'tests/reference/m5c_manifest.json'
COMPARE = ROOT / 'tools/reference/level_compare_m4b.py'


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--binary', type=Path, required=True)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5c-rust-check')
    ap.add_argument('--only')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    help_text = subprocess.run([str(a.binary), '--help'], capture_output=True, text=True)
    if 'episode1v2' not in help_text.stdout + help_text.stderr: sys.exit(f'{a.binary} does not name the episode1v2 policy')
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try: frozen = m4b_build.build('strict', work / 'm4bbuild', a.cc)
        finally: sys.stdout = so
    want = {x.upper() for x in a.only.split(',')} if a.only else None
    man = json.loads(MANIFEST.read_text())
    results, bad = {}, []
    for room in man['rooms']:
        wad = ROOT / room['file']
        for s in room['scripts']:
            L = s['lump']
            if want and L not in want: continue
            lmp = ROOT / s['lmp']
            cdir, rdir, out = work / 'c' / L, work / 'rust' / L, work / f'{L}.json'
            for d in (cdir, rdir):
                if d.exists(): shutil.rmtree(d)
                d.mkdir(parents=True)
            p = subprocess.run([str(frozen), '--wad', str(wad), '--wad', str(lmp), '--demo', L, '--out', str(cdir)],
                               capture_output=True, text=True)
            if p.returncode: bad.append(f'{L}: genuine run failed: {p.stderr[-300:]}'); continue
            p = subprocess.run([str(a.binary), '--wad', str(wad), '--map', room['slot'], '--demo', str(lmp), '--out', str(rdir),
                                '--captures', '--policy', 'episode1v2'], capture_output=True, text=True)
            if p.returncode: bad.append(f'{L}: Rust run failed: {(p.stdout + p.stderr)[-300:]}'); continue
            p = subprocess.run([sys.executable, str(COMPARE), str(cdir), str(rdir), '--wad', str(wad), '--demo', str(lmp),
                                '--json', str(out)], capture_output=True, text=True)
            r = json.loads(out.read_text())
            results[L] = r
            print(f"{L}: tics compared {r['tics_compared']}; match {r['match']}; C end {r['c_end']['kind']} at {r['c_end']['tic']}; "
                  f"Rust end {r['rust_end']['kind']}; first divergence {r['first_divergence']}; projection stop {r['projection_stop']}",
                  flush=True)
            if not r['match']: bad.append(f'{L}: {p.stdout.strip()}')
    if bad: sys.exit('FAILED:\n  ' + '\n  '.join(bad))
    print(f"m5c_rust_check: {len(results)} scripts match, {sum(r['tics_compared'] for r in results.values())} tics compared")


if __name__ == '__main__':
    main()
