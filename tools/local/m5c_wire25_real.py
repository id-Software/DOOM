#!/usr/bin/env python3
"""Local-only: wire 25 against the genuine engine on the 61 real Episode 1 runs, tic for tic, with the sound
records (M5c step 3b; never in CI: it reads the commercial IWAD). The first genuine comparison of sound on real
maps.

For every run of tests/reference/m4b_real_results.json (the committed E1M1-E1M9 scripts and DEMO1, extracted
from the IWAD copy and patched 109 to 110 by tools/local/m5a_real.py), one run at a time (--jobs runs at once):

  dumps   the frozen whole-engine runner the M4b record names (m4a or m4b driver, strict) on the IWAD and the
          demo; the dump set must equal the record's strict dumps_sha256;
  sound   the M5c runner (strict; linuxdoom-1.10 built with its in-process sound code, one define removed)
          under the recommended switches of tools/reference/m5c_reference.py, --render none, the IWAD only in
          the lump list; the stream must give the committed sound_sha256 of tests/reference/m5c_real_results.json
          (the render-every stream of tools/local/m5c_real.py: a sound call does not depend on rendering);
  Rust    `sound_records_run --capture-stream` (wire 25) through a pipe, so no capture touches the disk;
  compare tools/reference/level_compare_m5c.py: the independent DSIM41 encoder on the dumps and the genuine
          stream against every Rust capture (the whole engine without map.* and events.*, the map group once,
          and every sound record). Every run must match.

Each run's dumps and stream are deleted once it is compared. --write keeps the committed record
tests/reference/m5c_wire25_real_results.json (schema DOOM-M5C-WIRE25-REAL-RESULTS-v1: counts, pass/fail facts
and genuine or encoder digests only; never a coordinate, a lump byte or a Rust digest); --check compares.
--report PATH (scratch, never committed) writes every sound disagreement with both values, for diagnosis.

    python3 tools/local/m5c_wire25_real.py --binary PATH [--iwad PATH] [--work DIR] [--maps E1M1,..]
                                           [--only WALK,..] [--jobs N] [--write | --check] [--report PATH]
"""
import argparse, collections, concurrent.futures as cf, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference')); sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m4a_build                 # noqa: E402
import m4b_build                 # noqa: E402
import m5c_build                 # noqa: E402
import m5c_reference as C        # noqa: E402  (read-only: switches, truncate)
import level_snapshot as L       # noqa: E402
import level_snapshot_m4b as M   # noqa: E402
import level_snapshot_m5c as S   # noqa: E402
import level_compare_m5c as K    # noqa: E402
from m5a_real import runs_selected, demo_path  # noqa: E402

RESULTS = ROOT / 'tests/reference/m5c_wire25_real_results.json'
M5C_REAL = ROOT / 'tests/reference/m5c_real_results.json'
SCHEMA = 'DOOM-M5C-WIRE25-REAL-RESULTS-v1'
sha = lambda b: hashlib.sha256(b).hexdigest()


def one(r, a, bins, work, m5c_real):
    name = f"{r['map']}_{Path(r['script']).stem.upper()}"
    w = work / 'runs' / name
    if w.exists(): shutil.rmtree(w)
    w.mkdir(parents=True)
    fails = []
    try:
        lmp = demo_path(r, w, a.iwad); lump = lmp.stem.upper()
        cdir = w / 'c'; cdir.mkdir()
        p = subprocess.run([str(bins[r['driver']]), '--wad', str(a.iwad), '--wad', str(lmp), '--demo', lump, '--out', str(cdir)],
                           capture_output=True, text=True)
        if p.returncode: return name, None, [f'{name}: genuine runner exit {p.returncode}'], []
        h = hashlib.sha256()
        for x in L.m3h9c_dump.snapshot_names(str(cdir)): h.update((cdir / x).read_bytes())
        rec = dict(map=r['map'], script=r['script'], driver=r['driver'], dumps_equal_m4b=h.hexdigest() == r['dumps_sha256']['strict'])
        if not rec['dumps_equal_m4b']: fails.append(f'{name}: the dumps differ from the M4b strict digest')
        sp = w / 'sound.txt'
        sw = [x for k, v in C.RECOMMENDED.items() for x in (k, v)]
        p = subprocess.run([str(bins['m5c']), '--wad', str(a.iwad), '--wad', str(lmp), '--demo', lump, '--no-dump',
                            '--render', 'none', *sw, '--sound-out', str(sp)], capture_output=True, text=True, env=C.ENV)
        text = sp.read_text() if sp.exists() else ''
        last = max((int(l.split(',')[1]) for l in text.splitlines() if l.startswith('R,')), default=-1)
        want = m5c_real.get(name, {}).get('sound', {}).get('sound_sha256')
        rec['sound_equal_m5c_real'] = sha(C.truncate(text, last, True).encode()) == want
        if p.returncode or not rec['sound_equal_m5c_real']:
            fails.append(f'{name}: the genuine stream (exit {p.returncode}) differs from the committed m5c_real sound_sha256')
        sounds = S.SoundStream(text=text)
        skill, number = M.demo_header(lmp)
        mapdata = L.MapData(str(a.iwad), f'E1M{number}', L.demo_lumps(str(lmp)))
        rfd, wfd = os.pipe()
        with open(w / 'rust.txt', 'w') as out:
            proc = subprocess.Popen([str(a.binary), '--wad', str(a.iwad), '--map', f'E1M{number}', '--demo', str(lmp),
                                     '--capture-stream', f'/dev/fd/{wfd}'], pass_fds=(wfd,), stdout=out, stderr=subprocess.STDOUT)
        os.close(wfd)
        try:
            with os.fdopen(rfd, 'rb', buffering=1 << 20) as f:
                result, lines = K.compare(str(cdir), K.StreamCaptures(f), mapdata, skill, number, sounds, values=bool(a.report), keep=10**6)
        finally:
            if proc.poll() is None:
                try: proc.wait(timeout=600)
                except subprocess.TimeoutExpired: proc.kill()
        code = proc.wait()
        if code: fails.append(f'{name}: sound_records_run exit {code}')
        snd = result['sounds']
        rec.update(skill=skill, tics_compared=result['tics_compared'], snapshots=len(L.m3h9c_dump.snapshot_names(str(cdir))),
                   c_end=result['c_end'], rust_end=result['rust_end'], map_group_equal=result['map_group_equal'],
                   first_divergence=result['first_divergence'], projection_stop=result['projection_stop'],
                   starts=snd['starts'], placed=snd['placed'], stops=snd['stops'], tics_with_records=snd['tics_with_records'],
                   sound_disagreements=snd['disagreements'],
                   disagreement_attributes=dict(sorted(collections.Counter(x['attribute'] for x in snd['first']).items())),
                   rows_after_last_dump=snd['rows_after_last_dump'], c_sequence_sha256=result['c_sequence_sha256'],
                   c_sound_sha256=result['c_sound_sha256'], match=result['match'])
        if not result['match']: fails.append(f'{name}: ' + K.summary_line(result))
        details = [dict(run=name, **x) for x in snd['first']]
        return name, rec, fails, details
    finally:
        shutil.rmtree(w, ignore_errors=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--binary', type=Path, required=True, help='sound_records_run (release)')
    ap.add_argument('--iwad', type=Path, default=Path.home() / 'scratch/m5c/iwad/DOOM.WAD')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5c-wire25-real')
    ap.add_argument('--maps'); ap.add_argument('--only'); ap.add_argument('--jobs', type=int, default=1)
    ap.add_argument('--cc', default='cc'); ap.add_argument('--report', type=Path)
    g = ap.add_mutually_exclusive_group(); g.add_argument('--write', action='store_true'); g.add_argument('--check', action='store_true')
    a = ap.parse_args()
    if not a.iwad.is_file():
        print(f'skip: no IWAD at {a.iwad} (local-only wire-25 real-map comparison)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    if a.report and ROOT in a.report.resolve().parents: sys.exit('--report must lie outside the repository')
    if (a.write or a.check) and (a.maps or a.only): sys.exit('--write and --check cover every run')
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try: bins = dict(m4a=m4a_build.build('strict', work / 'build' / 'm4a', a.cc), m4b=m4b_build.build('strict', work / 'build' / 'm4b', a.cc),
                         m5c=m5c_build.build('strict', work / 'build' / 'm5c', a.cc))
        finally: sys.stdout = so
    m5c_real = json.loads(M5C_REAL.read_text())['runs']
    todo = runs_selected(a.maps.split(',') if a.maps else None, {x.upper() for x in a.only.split(',')} if a.only else None)
    recs, fails, details = {}, [], []
    with cf.ProcessPoolExecutor(max(1, a.jobs)) as pool:
        futs = [pool.submit(one, r, a, bins, work, m5c_real) for r in todo]
        for fut in futs:
            name, rec, f, d = fut.result()
            fails += f; details += d
            if rec is None: print(f'{name}: FAILED {f}', flush=True); continue
            recs[name] = rec
            print(f"{name}: tics {rec['tics_compared']}/{rec['snapshots']} match {rec['match']}; records {rec['starts']} starts "
                  f"({rec['placed']} placed), {rec['stops']} stops; disagreements {rec['sound_disagreements']} "
                  f"{rec['disagreement_attributes'] or ''}; dumps {rec['dumps_equal_m4b']}; stream {rec['sound_equal_m5c_real']}; "
                  f"C end {rec['c_end']['kind']}, Rust end {rec['rust_end']['kind']}", flush=True)
    totals = dict(runs=len(recs), matching=sum(r['match'] for r in recs.values()), tics=sum(r['tics_compared'] for r in recs.values()),
                  starts=sum(r['starts'] for r in recs.values()), placed=sum(r['placed'] for r in recs.values()),
                  stops=sum(r['stops'] for r in recs.values()),
                  sound_disagreements=sum(r['sound_disagreements'] for r in recs.values()),
                  endings=dict(sorted(collections.Counter(r['c_end']['kind'] for r in recs.values()).items())))
    print('totals:', json.dumps(totals))
    if a.report:
        a.report.parent.mkdir(parents=True, exist_ok=True)
        a.report.write_text(json.dumps(dict(totals=totals, disagreements=details, fails=fails), indent=1) + '\n')
        print(f'wrote {a.report} ({len(details)} disagreements)')
    if fails: print('FAILED:\n  ' + '\n  '.join(fails[:200])); return 1
    doc = dict(schema=SCHEMA, note='Wire 25 (sound_records_run) against the genuine engine on the 61 real Episode 1 runs (local only): '
               'the frozen M4a/M4b runner dumps and the M5c runner sound stream (linuxdoom-1.10 built with its in-process sound code, '
               'one define removed) projected to DSIM41 by tools/reference/level_snapshot_m5c.py and compared by level_compare_m5c.py '
               'with every sound record. Counts, pass/fail facts and genuine or encoder digests only.',
               switches=C.RECOMMENDED, totals=totals, runs=dict(sorted(recs.items())))
    text = json.dumps(doc, indent=1) + '\n'
    if a.check:
        if not RESULTS.exists() or RESULTS.read_text() != text: print(f'{RESULTS.relative_to(ROOT)} differs'); return 1
        print(f'{RESULTS.relative_to(ROOT)} matches ({len(recs)} runs)')
    elif a.write:
        RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
