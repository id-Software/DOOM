#!/usr/bin/env python3
"""Local-only M5a real-map frame comparison: the genuine renderer against the Rust renderer on
the committed Episode 1 scripts and DEMO1 (never in CI; it reads the commercial IWAD).

Decisions R2, R4, R7, R9 and R10 of docs/M5A_REFERENCE_INVESTIGATION.md. For every run of
tests/reference/m4b_real_results.json (the committed E1M1-E1M9 scripts and DEMO1) and every view
configuration (screenblocks 11 and 10, high detail) it:

  1. runs the genuine M5a runner (tools/reference/m5a_build.py) in each variant (strict,
     address, overflow; the address variant with ASAN_OPTIONS=detect_leaks=0, so leak detection
     is disabled) with --no-dump --render every --frames, and requires the variants' frame
     streams to be identical up to the earliest end, a tracker-flagged frame compared on its
     masked digest and flagged columns (as m5a_reference.py does on the synthetic items);
  2. runs `m5a_real` (crates/doom-render/examples/m5a_real.rs: the Rust simulation replays the
     same demo and the Rust renderer renders every tic) against the strict stream: frames
     matched, mismatched, non-comparable (switch parity unknown, decision R7; tracker unable to
     attribute, R4), refused by kind, not reached (the simulation stopped first);
  3. records per run and configuration counts and two stream digests: SHA-256 of one line
     `tic,digest` per genuine frame, the digest being the frame's SHA-256, the SHA-256 with the
     tracker-flagged columns zeroed where columns were flagged, or `whole` (tracker could not
     attribute the frame), computed from the genuine strict stream and, by the same rule, from
     the Rust frames (`refused` for a refused frame).

DEMO1 is extracted from the IWAD copy with its version byte patched from 109 to 110 (M4b, decision
I5) and checked against the patched digest recorded in m4b_real_results.json; it exists in --work
only. A demo's lump name is its base name (M5DEMO1).

Committed (tests/reference/m5a_real_results.json, schema DOOM-M5A-REAL-RESULTS-v1): counts,
pass/fail facts and digests only. The genuine frame streams name IWAD textures and lumps (N rows of
the tracker) and stay in --work; the per-texture breakdown of the flagged columns (owner ruling on
R4) is written to --report (default ~/scratch/m5a/reports/real_textures.json) and never committed;
the committed record keeps per run the number of frames with flagged columns, the columns and the
number of distinct origins. Raw frames (--diagnose) and PNG pairs stay in scratch and are deleted
or kept there; per-frame digest lists go to --work/digests.

    python3 tools/local/m5a_real.py [--iwad PATH] [--work DIR] [--maps E1M1,E1M2] [--only WALK,M2DIE]
                                    [--variants strict,address,overflow] [--screenblocks 11,10]
                                    [--jobs 1|2] [--diagnose] [--write | --check] [--cc CC]

--write merges the selected runs' records into the committed file (records of other runs are kept);
--check requires every selected run's record to equal the committed one (counts and digests). A
mismatch, refusal or variant difference is reported, never hidden; --diagnose reruns such a run
with raw frames and lets m5a_real name the feature at the first differing pixel (PNG pair in
~/scratch/m5a/diff).
"""
import argparse, concurrent.futures as cf, hashlib, json, os, re, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5a_build  # noqa: E402

M4B_REAL = ROOT / 'tests/reference/m4b_real_results.json'
RESULTS = ROOT / 'tests/reference/m5a_real_results.json'
SCHEMA = 'DOOM-M5A-REAL-RESULTS-v1'
VARIANTS = ['strict', 'address', 'overflow']
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
SUMMARY = re.compile(r'^summary: tics (\d+) end=(marker|exit|reborn|refused) end_tic (\d+) ', re.M)
FCOLS = ['tic', 'sha256', 'masked_sha256', 'fuzzpos', 'mapped', 'columns', 'visplanes', 'drawsegs', 'vissprites', 'openings',
         'solidsegs_peak', 'drawseg_drops', 'vissprite_drop', 'whole']
sha = lambda b: hashlib.sha256(b).hexdigest()


def runs_selected(maps, only):
    rec = json.loads(M4B_REAL.read_text())
    out = []
    for r in rec['runs']:
        if maps and r['map'] not in maps: continue
        stem = Path(r['script']).stem
        if only and stem.upper() not in only: continue
        out.append(r)
    return out


def demo_path(r, work, iwad):
    """The committed demo of a run, or DEMO1 extracted from the IWAD and patched (checked against the record)."""
    if r['script'] == 'M5DEMO1.lmp':
        data = Path(iwad).read_bytes()
        n, off = int.from_bytes(data[4:8], 'little'), int.from_bytes(data[8:12], 'little')
        for i in range(n):
            e = data[off + 16 * i: off + 16 * i + 16]
            if e[8:16].rstrip(b'\0') == b'DEMO1':
                lump = bytearray(data[int.from_bytes(e[0:4], 'little'):][:int.from_bytes(e[4:8], 'little')])
                if lump[0] != 109: sys.exit(f'DEMO1 version byte is {lump[0]}, expected 109')
                lump[0] = 110
                if sha(bytes(lump)) != r['lmp_sha256']: sys.exit('patched DEMO1 digest differs from the M4b record')
                p = work / 'M5DEMO1.lmp'; p.write_bytes(bytes(lump)); return p
        sys.exit('no DEMO1 lump in the IWAD')
    p = ROOT / f"tests/reference/{r['map'].lower()}_scripts/{r['script']}"
    if sha(p.read_bytes()) != r['lmp_sha256']: sys.exit(f'{p}: digest differs from the M4b record')
    return p


def parse(path):
    rows = {'F': [], 'O': [], 'E': [], 'N': [], 'U': [], 'M': []}
    for l in path.read_text().splitlines():
        if l.startswith('#'): continue
        rows.setdefault(l.split(',', 1)[0], []).append(l)
    return rows


def end_tic_of(rows):
    """The genuine limit ending (first O or E row), whose frame is not compared (decision R3)."""
    t = [int(rows[k][0].split(',')[1]) for k in ('O', 'E') if rows[k]]
    return min(t) if t else None


def normalised(rows, last):
    """F and U rows through tic `last`, a flagged frame's full digest replaced (the variant-identity form)."""
    out = []
    for k in ('F', 'U', 'M'):
        for l in rows[k]:
            f = l.split(',')
            if int(f[1]) > last: continue
            if k == 'F' and int(f[1 + FCOLS.index('columns')]) > 0: f[2] = 'undefined'
            out.append(','.join(f))
    return sorted(out, key=lambda l: (int(l.split(',')[1]), l))


def stream_digest(rows):
    end = end_tic_of(rows)
    lines = []
    for l in rows['F']:
        f = l.split(',')
        tic = int(f[1])
        if end is not None and tic >= end: continue
        d = 'whole' if f[1 + FCOLS.index('whole')] == '1' else (f[2] if f[3] == '-' else f[3])
        lines.append(f'{tic},{d}\n')
    return sha(''.join(lines).encode()), len(lines), ''.join(lines)


def flagged(rows):
    """Flagged frames and columns, and per origin name (local report only) frames and columns."""
    names = {l.split(',')[1]: l.split(',', 2)[2] for l in rows['N']}
    per, frames = {}, set()
    for l in rows['U']:
        _, tic, x, origins = l.split(',', 3)
        frames.add(tic)
        for o in origins.split('+'):
            d = per.setdefault(names.get(o, f'?{o}'), [set(), 0]); d[0].add(tic); d[1] += 1
    return len(frames), len(rows['U']), {n: dict(frames=len(v[0]), columns=v[1]) for n, v in sorted(per.items())}


def run_c(binary, iwad, lmp, sb, frames, raw=None, view=None):
    cmd = [str(binary), '--wad', str(iwad), '--wad', str(lmp), '--demo', lmp.stem.upper(), '--no-dump', '--render', 'every',
           '--screenblocks', str(sb), '--frames', str(frames)]
    if raw: cmd += ['--frames-raw', str(raw)]
    if view: cmd += ['--view', str(view)]
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV)
    m = SUMMARY.search(p.stdout)
    rep = [l for l in p.stderr.splitlines() if l.startswith('SUMMARY:') or 'runtime error' in l]
    rep = re.sub(r'0x[0-9a-f]+', '0x?', re.sub(r'\S*/([\w.]+\.c:\d+)', r'\1', rep[0]))[:200] if rep else None
    return dict(code=p.returncode, end=m[2] if m else None, end_tic=int(m[3]) if m else None, tics=int(m[1]) if m else None,
                sanitizer=rep)


def one(r, a, bins, rust, work):
    name = f"{r['map']}_{Path(r['script']).stem.upper()}"
    lmp = demo_path(r, work, a.iwad)
    rec = dict(map=r['map'], script=r['script'], skill=r['skill'], lmp_sha256=r['lmp_sha256'], views={})
    local, fails = {}, []
    sdir = work / 'streams'; sdir.mkdir(exist_ok=True)
    strict_streams = {}
    for sb in a.screenblocks:
        per = {}
        for v in a.variants:
            fp = sdir / f'{name}.sb{sb}.{v}.frames'
            per[v] = (run_c(bins[v], a.iwad, lmp, sb, fp), parse(fp))
            if v != 'strict': fp.unlink()
        s, srows = per['strict']
        strict_streams[sb] = sdir / f'{name}.sb{sb}.strict.frames'
        lasts = []
        for res, rows in per.values():
            t = [int(l.split(',')[1]) for l in rows['F']]
            e = end_tic_of(rows)
            lasts.append(min(max(t, default=-1), e - 1 if e is not None else 1 << 30))
        last = min(lasts)
        ident = all(normalised(rows, last) == normalised(srows, last) for _, rows in per.values())
        if not ident: fails.append(f'{name} sb{sb}: variants differ')
        digest, nframes, lines = stream_digest(srows)
        (work / 'digests').mkdir(exist_ok=True)
        (work / 'digests' / f'{name}.sb{sb}.c.txt').write_text(lines)
        ff, fc, per_origin = flagged(srows)
        local[str(sb)] = per_origin
        endings = {v: (f"sanitizer:{res['sanitizer']}" if res['sanitizer'] else
                       (f"refused:{rows['O'][0].split(',')[2]}" if rows['O'] else
                        (f"error:{rows['E'][0].split(',', 2)[2]}" if rows['E'] else (res['end'] or f"abnormal:{res['code']}"))))
                   for v, (res, rows) in per.items()}
        rec['views'][str(sb)] = dict(frames=nframes, end=endings['strict'], end_tic=s['end_tic'], tics=s['tics'],
                                     genuine_stream_sha256=digest, flagged_frames=ff, flagged_columns=fc,
                                     flagged_origins=len(per_origin),
                                     whole_frames=sum(1 for l in srows['F'] if l.split(',')[1 + FCOLS.index('whole')] == '1'),
                                     variants=list(per), variant_endings=endings, compared_through_tic=last,
                                     variants_identical=ident)
    out = work / 'rust' / f'{name}.json'; out.parent.mkdir(exist_ok=True)
    cmd = [str(rust), '--iwad', str(a.iwad), '--lmp', str(lmp), '--json', str(out)]
    for sb in a.screenblocks:
        cmd += ['--frames', f'{sb}={strict_streams[sb]}', '--digests', f"{sb}={work / 'digests' / f'{name}.sb{sb}.rust.txt'}"]
    p = subprocess.run(cmd, capture_output=True, text=True)
    if p.returncode: fails.append(f'{name}: m5a_real failed: {p.stderr[-600:]}'); return name, rec, local, fails, None
    rj = json.loads(out.read_text())
    diag = None
    for sb in a.screenblocks:
        v, x = rec['views'][str(sb)], rj[str(sb)]
        for k in ('frames', 'matched', 'mismatched', 'non_comparable', 'parity_unknown', 'whole', 'refused', 'refusals',
                  'not_reached', 'counter_mismatches', 'flag_disagreements', 'rust_stream_sha256'):
            v[k if k != 'frames' else 'rust_frames'] = x[k]
        v['observe_error'] = x['observe_error'] is not None
        v['rust_end'] = x['end']
        v['streams_equal'] = v['rust_stream_sha256'] == v['genuine_stream_sha256']
        if x['frames'] != v['frames']: fails.append(f"{name} sb{sb}: Rust saw {x['frames']} genuine frames, the stream has {v['frames']}")
        if x['first']: (diag := diag or {})[str(sb)] = x['first']
        if x['mismatched'] or x['refused'] or x['counter_mismatches'] or x['flag_disagreements'] or x['observe_error']:
            fails.append(f"{name} sb{sb}: mismatched {x['mismatched']} refused {x['refused']} {x['refusals']} counters "
                         f"{x['counter_mismatches']} flags {x['flag_disagreements']} observe {x['observe_error']}; first {x['first']}")
    if diag and a.diagnose:
        for sb in a.screenblocks:
            if str(sb) not in diag: continue
            raw = work / f'raw_{name}_sb{sb}'; shutil.rmtree(raw, ignore_errors=True); raw.mkdir()
            fp = sdir / f'{name}.sb{sb}.diag.frames'
            run_c(bins['strict'], a.iwad, lmp, sb, fp, raw)
            dj = work / 'rust' / f'{name}.sb{sb}.diag.json'
            subprocess.run([str(rust), '--iwad', str(a.iwad), '--lmp', str(lmp), '--json', str(dj), '--frames', f'{sb}={fp}',
                            '--raw', f'{sb}={raw}'], capture_output=True, text=True)
            diag[str(sb)] = json.loads(dj.read_text())[str(sb)]
            shutil.rmtree(raw); fp.unlink()
    return name, rec, local, fails, diag


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--iwad', type=Path, default=Path.home() / 'scratch/m5a/iwad/DOOM.WAD')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m5a/real')
    ap.add_argument('--report', type=Path, default=Path.home() / 'scratch/m5a/reports/real_textures.json')
    ap.add_argument('--maps'); ap.add_argument('--only')
    ap.add_argument('--variants', default=','.join(VARIANTS))
    ap.add_argument('--screenblocks', default='11,10')
    ap.add_argument('--jobs', type=int, default=2)
    ap.add_argument('--diagnose', action='store_true')
    g = ap.add_mutually_exclusive_group(); g.add_argument('--write', action='store_true'); g.add_argument('--check', action='store_true')
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--rust', type=Path, help='a built m5a_real example (default: cargo build --release)')
    a = ap.parse_args()
    if not a.iwad.is_file():
        print(f'skip: no IWAD at {a.iwad} (local-only M5a real-map comparison)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    work.mkdir(parents=True, exist_ok=True)
    a.variants = a.variants.split(',')
    assert a.variants[0] == 'strict'
    a.screenblocks = [int(x) for x in a.screenblocks.split(',')]
    maps = set(a.maps.upper().split(',')) if a.maps else None
    only = set(a.only.upper().split(',')) if a.only else None
    todo = runs_selected(maps, only)
    if not todo: sys.exit('no run selected')
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try: bins = {v: m5a_build.build(v, work / 'build', a.cc) for v in a.variants}
        finally: sys.stdout = so
    rust = a.rust
    if rust is None:
        subprocess.run(['cargo', 'build', '-q', '--release', '-p', 'doom-render', '--example', 'm5a_real'], cwd=ROOT, check=True)
        rust = ROOT / 'target/release/examples/m5a_real'
    records, report, failures = {}, {}, []
    with cf.ThreadPoolExecutor(max(1, min(a.jobs, 2))) as pool:
        for name, rec, local, fails, diag in pool.map(lambda r: one(r, a, bins, rust, work), todo):
            records[(rec['map'], rec['script'])] = rec
            report[name] = local
            failures += fails
            print(f'{name}: ' + '; '.join(
                f"sb{sb} frames {v['frames']} matched {v.get('matched')} non-comparable {v.get('non_comparable')} "
                f"(parity {v.get('parity_unknown')}, whole {v.get('whole')}) refused {v.get('refused')} not-reached "
                f"{v.get('not_reached')} flagged {v['flagged_frames']}/{v['flagged_columns']}/{v['flagged_origins']} "
                f"variants {'identical' if v['variants_identical'] else 'DIFFER'} end {v['end']}"
                for sb, v in rec['views'].items()), flush=True)
            if diag:
                for sb, d in diag.items(): print(f'  {name} sb{sb} first difference: {json.dumps(d)}', flush=True)
    a.report.parent.mkdir(parents=True, exist_ok=True)
    old = json.loads(a.report.read_text()) if a.report.exists() else {}
    old.update(report); a.report.write_text(json.dumps(old, indent=1, sort_keys=True) + '\n')
    print(f'per-origin report (local only, never committed): {a.report}')
    totals = {}
    for rec in records.values():
        for sb, v in rec['views'].items():
            t = totals.setdefault(sb, {})
            for k in ('frames', 'matched', 'mismatched', 'non_comparable', 'parity_unknown', 'whole', 'refused', 'not_reached',
                      'flagged_frames', 'flagged_columns'):
                t[k] = t.get(k, 0) + (v.get(k) or 0)
    for sb, t in totals.items(): print(f'total sb{sb}: ' + ' '.join(f'{k} {n}' for k, n in t.items()))
    if a.write or a.check:
        doc = json.loads(RESULTS.read_text()) if RESULTS.exists() else None
        if a.check:
            if doc is None: sys.exit(f'{RESULTS.relative_to(ROOT)} does not exist')
            have = {(r['map'], r['script']): r for r in doc['runs']}
            bad = [f'{k[0]} {k[1]}' for k, rec in records.items() if have.get(k) != rec]
            if bad: failures.append('records differ from the committed file: ' + ', '.join(bad))
            else: print(f'{len(records)} records equal the committed {RESULTS.relative_to(ROOT)}')
        else:
            if doc is None:
                doc = dict(schema=SCHEMA,
                           note=('M5a step 4 real-map frame comparison (local only): the genuine linuxdoom-1.10 renderer '
                                 '(M5a runner, strict/address/overflow) against the Rust renderer fed by the Rust simulation, '
                                 'every tic, screenblocks 11 and 10, high detail. Counts, pass/fail facts and stream digests only; '
                                 'see tools/local/m5a_real.py for the digest rule.'),
                           asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)', runs=[])
            doc['runner_contract_sha256'] = sha(m5a_build.CONTRACT.read_bytes())
            have = {(r['map'], r['script']): r for r in doc['runs']}
            have.update(records)
            order = {(r['map'], r['script']): i for i, r in enumerate(json.loads(M4B_REAL.read_text())['runs'])}
            doc['runs'] = [have[k] for k in sorted(have, key=lambda k: order.get(k, 1 << 30))]
            RESULTS.write_text(json.dumps(doc, indent=1) + '\n')
            print(f'wrote {len(records)} records into {RESULTS.relative_to(ROOT)} ({len(doc["runs"])} in all)')
    if failures:
        print('FAILURES:\n  ' + '\n  '.join(failures)); return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
