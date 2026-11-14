#!/usr/bin/env python3
"""DSIM40 goldens of the 30 M4a synthetic scripts, from the genuine-engine dumps (M4b step 3).

    m4b_m4a_goldens.py [--dumps RUNS_DIR | --work DIR] [--check] [--compare39]

Wire 24 (`PlayerMonstersEpisode1V2`, M4b decision I2) is a strict superset of wire 23, so
every M4a synthetic script must run under it exactly as under wire 23 (decision I11). This
tool gives that claim C expectations in DSIM40: every script of tests/reference/m4a_manifest.json
(schema DOOM-M4A-MANIFEST-v1, frozen M4a evidence, read only) is projected by the independent
DSIM40 encoder (level_snapshot_m4b.py, profile `episode1v2`) from the strict variant's dumps
(`strict_<SCRIPT>/`) on its WAD, the skill and map number read from its demo header. Nothing
here reads Rust output. Before projecting, the dump set's SHA-256 (the concatenated snapshots,
as m3h9c_reference.run computes it) must equal the script's `dumps_sha256` in the frozen
tests/reference/m4a_synthetic_results.json, so the goldens derive from exactly the committed
M4a C evidence. RUNS_DIR is the `runs` directory of `m4a_reference.py --keep-dumps --work DIR`;
without --dumps the tool runs `m4a_reference.py --keep-dumps --check --work DIR` itself (DIR
outside the repository; the dumps are deleted afterwards).

Output (synthetic, committable): tests/reference/m4b_m4a_goldens.json, per script its WAD,
slot, skill and map, the snapshot count, the end (marker, exit or reborn; cross-checked with
the M4a manifest), the M4a manifest's Rust expectation, the projection stop if the encoder
refuses a tic, the per-tic digest list (the first 16 hex digits of the SHA-256 of each capture
without `map.*` and `events.*`, the comparator's stripped form), the SHA-256 of the
concatenated full per-tic SHA-256 digests and the final stripped SHA-256. No capture files:
the DSIM39 fixtures of M4a stand for these states.

DSIM tic numbering: the load is tic 0 and the dump `tic_NNNNN` is tic N + 1 (as M4a).

`--check` regenerates the JSON in memory and fails unless it equals the committed file.
`--compare39` also projects every snapshot with the frozen DSIM39 profile and requires the two
projections to differ only in the DSIM40 delta (level_snapshot_m4b.compare39).
tools/reference/check_player_episode1v2.py checks the Rust example against these goldens."""
import argparse, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
import level_snapshot_m4b as M
from level_compare import stripped, end_kind

SCHEMA = 'DOOM-M4B-M4A-GOLDENS-v1'
MANIFEST = ROOT / 'tests/reference/m4a_manifest.json'
RESULTS = ROOT / 'tests/reference/m4a_synthetic_results.json'
OUT = ROOT / 'tests/reference/m4b_m4a_goldens.json'


def dump_digest(directory):
    digest = hashlib.sha256()
    for name in L.m3h9c_dump.snapshot_names(directory): digest.update(Path(directory, name).read_bytes())
    return digest.hexdigest()


def end_check(script, s, kind, last, secretexit):
    """The dump's end against the M4a manifest (marker: end_tic is the DSIM tic of the last
    snapshot; exit and reborn: end_tic is the last dump tic, one less)."""
    want = s['end_tic'] if kind == 'marker' else s['end_tic'] + 1
    if kind != s['c_end'] or last != want or secretexit != s['secretexit']:
        sys.exit(f"{script}: the dumps end {kind} at tic {last}; the manifest says {s['c_end']} at {s['end_tic']}")


def generate(runs, compare39=False):
    man = json.loads(MANIFEST.read_text())
    if man.get('schema') != 'DOOM-M4A-MANIFEST-v1': sys.exit('M4a manifest schema')
    res = json.loads(RESULTS.read_text())
    goldens = dict(schema=SCHEMA, encoder='tools/reference/level_snapshot_m4b.py (profile episode1v2)',
                   dsim=dict(version=M.VERSION, policy=M.POLICY, context=M.CONTEXT.decode()),
                   manifest='tests/reference/m4a_manifest.json',
                   manifest_sha256=hashlib.sha256(MANIFEST.read_bytes()).hexdigest(),
                   results_sha256=hashlib.sha256(RESULTS.read_bytes()).hexdigest(), scripts={})
    compared = 0
    for w in man['wads']:
        wad = ROOT / w['file']
        if hashlib.sha256(wad.read_bytes()).hexdigest() != w['sha256']: sys.exit(f"{w['name']}: WAD differs from the manifest")
        for s in w['scripts']:
            script = s['lump']; lmp = ROOT / s['lmp']; directory = os.path.join(runs, f'strict_{script}')
            if dump_digest(directory) != res['scripts'][script]['dumps_sha256']:
                sys.exit(f'{script}: the strict dumps differ from m4a_synthetic_results.json')
            skill, map_number = M.demo_header(lmp)
            if w['slot'] != f'E1M{map_number}': sys.exit(f'{script}: the demo header names another slot')
            mapdata = L.MapData(str(wad), w['slot'], L.demo_lumps(lmp))
            names = L.m3h9c_dump.snapshot_names(directory)
            kind, last = end_kind(directory)
            end_check(script, s, kind, last, bool(L.Dump(os.path.join(directory, names[-1])).h['secretexit']))
            entry = dict(wad=w['name'], wad_sha256=w['sha256'], slot=w['slot'], skill=skill, map=map_number,
                         lmp_sha256=hashlib.sha256(lmp.read_bytes()).hexdigest(), snapshots=len(names),
                         end=dict(kind=kind, tic=last), manifest_end=dict(c_end=s['c_end'], end_tic=s['end_tic']),
                         rust=s['rust'], projection_stop=None)
            digests = []
            try:
                for _, fields in M.project(directory, mapdata, skill, map_number):
                    digests.append(hashlib.sha256(stripped(M.VERSION, fields)[0]).digest())
            except L.ProjectionError as err:
                entry['projection_stop'] = dict(kind=err.kind, tic=err.tic, reason=err.text)
            if compare39 and entry['projection_stop'] is None:
                compared += M.compare39(directory, mapdata, skill, map_number)
            entry.update(tics_projected=len(digests), sequence_sha256=hashlib.sha256(b''.join(digests)).hexdigest(),
                         final_sha256=digests[-1].hex() if digests else None, tic_digests=[d.hex()[:16] for d in digests])
            goldens['scripts'][script] = entry
    return (json.dumps(goldens, indent=1) + '\n').encode(), goldens, compared


def reference_runs(work):
    work = work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    subprocess.run([sys.executable, str(HERE / 'm4a_reference.py'), '--work', str(work), '--keep-dumps', '--check'],
                   check=True, stdout=subprocess.DEVNULL)
    return work / 'runs'


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--dumps', help="m4a_reference.py's runs directory (kept dumps)")
    ap.add_argument('--work', type=Path, default=Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-m4b-m4a-goldens',
                    help='scratch directory for the reference run when --dumps is not given')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--compare39', action='store_true', help='also check DSIM40 = DSIM39 outside the delta')
    a = ap.parse_args()
    runs = a.dumps or str(reference_runs(a.work))
    try:
        data, goldens, compared = generate(runs, a.compare39)
    finally:
        if not a.dumps: shutil.rmtree(Path(runs).parent, ignore_errors=True)
    total = 0
    for script, e in goldens['scripts'].items():
        stop = e['projection_stop']; total += e['tics_projected']
        print(f"{script}: skill {e['skill']} map {e['map']} snapshots {e['snapshots']} end {e['end']['kind']} at tic "
              f"{e['end']['tic']}; projected {e['tics_projected']}"
              + (f"; projection stopped: {stop['kind']} at tic {stop['tic']}: {stop['reason']}" if stop else '')
              + f"; sequence {e['sequence_sha256'][:16]}; final {(e['final_sha256'] or '-')[:16]}")
    print(f"{len(goldens['scripts'])} scripts, {total} snapshots projected")
    if a.compare39: print(f'DSIM40 equals DSIM39 outside {", ".join(M.DELTA_FIELDS)} on {compared} snapshots')
    if a.check:
        if not OUT.exists() or OUT.read_bytes() != data:
            print('check FAILED: tests/reference/m4b_m4a_goldens.json differs'); return 1
        print('check passed: 1 file'); return 0
    OUT.write_bytes(data)
    print('wrote tests/reference/m4b_m4a_goldens.json'); return 0


if __name__ == '__main__':
    sys.exit(main())
