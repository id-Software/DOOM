#!/usr/bin/env python3
"""Local-only M5b real-map display comparison: the genuine status bar, HUD, palette and 3D view
against the Rust `Display` on the committed Episode 1 scripts and DEMO1 (never in CI; it reads the
commercial IWAD). Frames are identical to the genuine engine under the declared sound-free,
wipe-free contract (decision S15 of docs/M5B_REFERENCE_INVESTIGATION.md).

For every run of tests/reference/m4b_real_results.json (the 61 runs of tools/local/m5a_real.py:
the committed E1M1-E1M9 scripts and DEMO1, extracted from the IWAD copy and patched 109 to 110),
with the IWAD only in the lump list (its own status bar, font and palette lumps), it:

  1. runs the genuine M5b runner (tools/reference/m5b_build.py) strict with --render none and
     dumps: the dump set, under the S16 record and mask (H.rndindex without the presentation
     draws, P.message as HU_Ticker found it), must equal the committed M4b strict dumps_sha256;
  2. at screenblocks 10 and 11, every tic (--render every): strict with dumps (each snapshot must
     equal the never-render one), overflow and address without dumps (address with
     ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so this is not LeakSanitizer
     coverage); the three frame streams, normalised as m5b_reference.py does (with the D, W and
     Z rows), must be byte-identical up to the earliest end, and every variant must end alike;
  3. stops on any S17 condition: a 2D undefined read, a skipped patch, a write below the screen,
     a post outside its lump (D rows), a palette pointer outside PLAYPAL (Z row), an M_Random draw
     outside ST_Ticker, a genuine refusal witness, a sanitizer report;
  4. runs `m5b_real` (crates/doom-render/examples/m5b_real.rs: the Rust simulation replays the
     demo, the Rust Display observes and draws every tic) against the strict streams: the
     whole-frame digest (or the masked digest where the genuine tracker flagged columns), the
     renderer counters, the P row (palette, palette set, face, message_on, M_Random index) and,
     on every tic, the message taken; any Rust refusal or observe error (MessageUnderivable,
     AttackerRemoved, MessageNotTaken, a draw refusal) is a failure to report, never worked around.

Stream digest: SHA-256 of, per genuine frame, `tic,digest,palette,palette_set,face,message_on,rndindex`
(digest: the frame's SHA-256, the masked SHA-256 where columns were flagged, `whole` where the genuine
tracker could not attribute the frame), then `X,tic,code,text` per message taken; the Rust side
computes it by the same rule.

Committed (tests/reference/m5b_real_results.json, schema DOOM-M5B-REAL-RESULTS-v1): counts, pass/fail
facts and digests only. Frame streams (which name IWAD lumps and carry message texts) stay in --work.

    python3 tools/local/m5b_real.py [--iwad PATH] [--work DIR] [--maps E1M1] [--only WALK]
                                    [--screenblocks 10,11] [--jobs N] [--write | --check] [--cc CC]
"""
import argparse, collections, concurrent.futures as cf, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference')); sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m5b_build                 # noqa: E402
import m5b_reference as B        # noqa: E402  (read-only: normalise, truncate, frows_of, ending)
import m3h9c_dump as dump        # noqa: E402
from m5a_real import M4B_REAL, runs_selected, demo_path  # noqa: E402

RESULTS = ROOT / 'tests/reference/m5b_real_results.json'
SCHEMA = 'DOOM-M5B-REAL-RESULTS-v1'
VARIANTS = ['strict', 'overflow', 'address']
ENV = B.ENV
ASAN = 'detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)'
sha = lambda b: hashlib.sha256(b).hexdigest()
FC, PC, DC = B.FCOLS, B.PCOLS, B.DCOLS


def run_c(binary, iwad, lmp, out, extra=(), frames=None, nodump=False):
    if out.exists(): shutil.rmtree(out)
    out.mkdir(parents=True)
    cmd = [str(binary), '--wad', str(iwad), '--wad', str(lmp), '--demo', lmp.stem.upper(),
           *(['--no-dump'] if nodump else ['--out', str(out)]), *extra]
    if frames: cmd += ['--frames', str(frames)]
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV)
    names = [] if nodump else dump.snapshot_names(str(out))
    h, per = hashlib.sha256(), []
    for n in names:
        b = (out / n).read_bytes(); h.update(b); per.append(sha(b))
    shutil.rmtree(out)
    return dict(code=p.returncode, summary=B.A.SUMMARY.search(p.stdout), stderr=p.stderr, dumps=h.hexdigest(), per=per,
                killed=p.returncode in (-9, 137))


def stream_lines(rows):
    """The genuine stream digest's text (module docstring), frames before a limit ending only."""
    end = min([int(rows[k][0].split(',')[1]) for k in ('O', 'E') if rows[k]], default=None)
    P = {l.split(',')[1]: l.split(',')[2:] for l in rows['P']}
    out = []
    for l in rows['F']:
        f = l.split(',')
        if end is not None and int(f[1]) >= end: continue
        d = 'whole' if f[1 + FC.index('whole')] == '1' else (f[2] if int(f[1 + FC.index('columns')]) == 0 else f[3])
        out.append(f"{f[1]},{d},{','.join(P[f[1]])}\n")
    out += [l + '\n' for l in rows['X'] if end is None or int(l.split(',')[1]) < end]
    return ''.join(out)


def selftest(name, path, sb, rust, a, lmp, W):
    """The comparison can fail: a copy of the strict stream with one frame digest, one face and one
    message's tic changed must give exactly one mismatched frame, one face mismatch and two message
    tics (the message missing at its tic and present one tic late) at the planted places."""
    lines = path.read_text().splitlines()
    Fi = [i for i, l in enumerate(lines) if l.startswith('F,') and l.split(',')[1 + FC.index('columns')] == '0']
    Pi = [i for i, l in enumerate(lines) if l.startswith('P,')]
    Xi = [i for i, l in enumerate(lines) if l.startswith('X,')]
    fi, pi = Fi[len(Fi) // 2], Pi[len(Pi) // 3]
    f = lines[fi].split(','); f[2] = f[2][:-1] + ('0' if f[2][-1] != '0' else '1'); lines[fi] = ','.join(f)
    p = lines[pi].split(','); p[4] = str(int(p[4]) ^ 1); lines[pi] = ','.join(p)
    planted = dict(frame=int(f[1]), face=int(p[1]))
    xt = {int(lines[i].split(',')[1]) for i in Xi}
    Xi = [i for i in Xi if int(lines[i].split(',')[1]) + 1 not in xt and int(lines[i].split(',')[1]) + 1 in {int(lines[j].split(',')[1]) for j in Pi}]
    if Xi:
        x = lines[Xi[0]].split(','); planted['message'] = int(x[1]); x[1] = str(int(x[1]) + 1); lines[Xi[0]] = ','.join(x)
    fp = W / 'planted.frames'; fp.write_text('\n'.join(lines) + '\n')
    out = W / 'planted.json'
    subprocess.run([str(rust), '--iwad', str(a.iwad), '--lmp', str(lmp), '--json', str(out), '--frames', f'{sb}={fp}'],
                   capture_output=True, text=True)
    x = json.loads(out.read_text())[str(sb)]
    first = min(planted.values())
    ok = (x['mismatched'] == 1 and x['field_mismatches'] == {'face': 1} and x['message_mismatches'] == (2 if Xi else 0)
          and x['first']['tic'] == first)
    return dict(planted=planted, mismatched=x['mismatched'], field_mismatches=x['field_mismatches'],
                message_mismatches=x['message_mismatches'], first_tic=x['first']['tic'], **{'pass': ok})


def one(r, a, bins, rust, work):
    name = f"{r['map']}_{Path(r['script']).stem.upper()}"
    W = work / 'runs' / name; W.mkdir(parents=True, exist_ok=True)
    lmp = demo_path(r, W, a.iwad)
    rec = dict(map=r['map'], script=r['script'], skill=r['skill'], lmp_sha256=r['lmp_sha256'])
    fails, stops, killed = [], [], []
    never = run_c(bins['strict'], a.iwad, lmp, W / 'never')
    if never['killed']: killed.append('never')
    rec['never_end'] = never['summary'][2] if never['summary'] else f"abnormal:{never['code']}"
    rec['never_snapshots'] = len(never['per'])
    rec['never_dumps_equal_m4b'] = never['dumps'] == r['dumps_sha256']['strict']
    if not rec['never_dumps_equal_m4b']: fails.append(f'{name}: masked never-render dumps differ from the committed M4b strict digest')
    rec['views'] = {}
    sdir = work / 'streams'; sdir.mkdir(exist_ok=True)
    strict_paths = {}
    for sb in a.screenblocks:
        per = {}
        for v in VARIANTS:
            fp = sdir / f'{name}.sb{sb}.{v}.frames'
            res = run_c(bins[v], a.iwad, lmp, W / f'sb{sb}_{v}', ['--render', 'every', '--screenblocks', str(sb)], fp, nodump=v != 'strict')
            if res['killed']: killed.append(f'sb{sb} {v}')
            text = fp.read_text()
            g, full = B.normalise(text)
            per[v] = dict(res=res, golden=g, full=full, rows=B.frows_of(g), full_rows=B.frows_of(full),
                          end=B.ending(res, B.frows_of(text)))
            if v == 'strict': strict_paths[sb] = fp
            else: fp.unlink()
        s = per['strict']
        tics = {v: max([int(l.split(',')[1]) for l in p['rows']['F']], default=-1) for v, p in per.items()}
        last = min(tics.values())
        same_end = len({p['end'] for p in per.values()}) == 1 and len(set(tics.values())) == 1
        ident = same_end and all(B.truncate(p['full'], last, True) == B.truncate(s['full'], last, True) for p in per.values())
        dumps_eq = s['res']['per'] == never['per']
        rows, fr = s['rows'], s['full_rows']
        F = [l.split(',') for l in rows['F']]
        P = [l.split(',') for l in rows['P']]
        D = [l.split(',') for l in fr['D']]
        z = fr['Z'][0].split(',') if fr['Z'] else []
        zget = lambda k: int(z[z.index(k) + 1]) if k in z else None
        twod = {c: sum(int(d[1 + DC.index(c)]) for d in D) for c in ('bar_undef_px', 'skipped_patches', 'beyond_rows', 'lump_overrun')}
        pal = collections.Counter(int(p[2]) for p in P)
        lines = stream_lines(rows)
        (work / 'digests').mkdir(exist_ok=True)
        (work / 'digests' / f'{name}.sb{sb}.c.txt').write_text(lines)
        flagged = [f for f in F if int(f[1 + FC.index('columns')]) > 0]
        v = dict(tics=int(s['res']['summary'][1]) if s['res']['summary'] else None, end=s['end'],
                 end_tic=int(s['res']['summary'][3]) if s['res']['summary'] else None, frames=len(F),
                 genuine_whole_digest_frames=sum(1 for f in F if f[1 + FC.index('whole')] != '1' and int(f[1 + FC.index('columns')]) == 0),
                 genuine_masked_frames=sum(1 for f in flagged if f[1 + FC.index('whole')] != '1'),
                 genuine_whole_frames=sum(1 for f in F if f[1 + FC.index('whole')] == '1'),
                 flagged_columns=sum(int(f[1 + FC.index('columns')]) for f in flagged),
                 nonzero_palette_frames=sum(n for k, n in pal.items() if k), palettes={str(k): pal[k] for k in sorted(pal)},
                 palette_sets=sum(1 for p in P if p[3] != '-1'), message_frames=sum(1 for p in P if p[5] != '0'),
                 faces=sorted({int(p[4]) for p in P}), messages_taken=len(rows['X']),
                 message_codes=sorted({int(x.split(',')[2]) for x in rows['X']}),
                 rndindex_last=int(P[-1][6]) if P else None, undefined_2d=twod,
                 palette_unknown=zget('palette_unknown'), other_draws=zget('other_draws'),
                 variant_endings={k: p['end'] for k, p in per.items()}, compared_through_tic=last, variants_identical=ident,
                 sanitizer_reports=sum(1 for k, p in per.items() if B.A.report_of(p['res']['stderr'])),
                 dumps_equal_never=dumps_eq, genuine_stream_sha256=sha(lines.encode()))
        if not ident: stops.append(f'{name} sb{sb}: genuine variants differ (endings {v["variant_endings"]}, through tic {last})')
        if not dumps_eq: fails.append(f'{name} sb{sb}: render-every dumps differ from the never-render set')
        if any(twod.values()): stops.append(f'{name} sb{sb}: 2D tracker counts {twod}')
        if v['palette_unknown'] or v['other_draws']: stops.append(f'{name} sb{sb}: palette_unknown {v["palette_unknown"]} other_draws {v["other_draws"]}')
        if v['sanitizer_reports']: stops.append(f'{name} sb{sb}: sanitizer reports {[B.A.report_of(p["res"]["stderr"]) for p in per.values()]}')
        if rows['O'] or rows['E'] or not str(s['end']) in ('marker', 'exit', 'reborn'):
            stops.append(f'{name} sb{sb}: genuine ending {s["end"]}')
        rec['views'][str(sb)] = v
    out = W / 'rust.json'
    cmd = [str(rust), '--iwad', str(a.iwad), '--lmp', str(lmp), '--json', str(out)]
    for sb in a.screenblocks:
        cmd += ['--frames', f'{sb}={strict_paths[sb]}', '--digests', f"{sb}={work / 'digests' / f'{name}.sb{sb}.rust.txt'}"]
    p = subprocess.run(cmd, capture_output=True, text=True)
    if p.returncode:
        fails.append(f'{name}: m5b_real failed ({p.returncode}): {p.stderr[-800:]}')
        return name, rec, fails, stops, killed
    rj = json.loads(out.read_text())
    for sb in a.screenblocks:
        v, x = rec['views'][str(sb)], rj[str(sb)]
        for k in ('frames', 'compared_whole', 'compared_masked', 'non_comparable_whole', 'non_comparable_parity', 'mismatched',
                  'flag_disagreements', 'counter_mismatches', 'fields_compared', 'field_mismatches', 'message_tics_compared',
                  'message_mismatches', 'messages_rust', 'refused', 'not_reached', 'rust_stream_sha256'):
            v['rust_frames' if k == 'frames' else k] = x[k]
        v['observe_error'] = x['observe_error'] is not None
        v['streams_equal'] = v['rust_stream_sha256'] == v['genuine_stream_sha256']
        bad = (x['mismatched'] or x['flag_disagreements'] or x['counter_mismatches'] or x['field_mismatches'] or x['message_mismatches']
               or x['not_reached'] or x['frames'] != v['frames'] or not v['streams_equal'] or x['non_comparable_parity'])
        if x['refused'] or x['observe_error']:
            stops.append(f"{name} sb{sb}: Rust refusal {x['refused']} observe {x['observe_error']}; first {x['first']}")
        elif bad:
            fails.append(f"{name} sb{sb}: divergence: mismatched {x['mismatched']} fields {x['field_mismatches']} messages "
                         f"{x['message_mismatches']} counters {x['counter_mismatches']} flags {x['flag_disagreements']} parity "
                         f"{x['non_comparable_parity']} not-reached {x['not_reached']}; first {x['first']}")
    if a.selftest:
        rec['selftest'] = selftest(name, strict_paths[a.screenblocks[0]], a.screenblocks[0], rust, a, lmp, W)
        if not rec['selftest']['pass']: fails.append(f"{name}: selftest {rec['selftest']}")
    shutil.rmtree(W)
    return name, rec, fails, stops, killed


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--iwad', type=Path, default=Path.home() / 'scratch/m5b/iwad/DOOM.WAD')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m5b/real')
    ap.add_argument('--maps'); ap.add_argument('--only')
    ap.add_argument('--screenblocks', default='10,11')
    ap.add_argument('--jobs', type=int, default=4)
    g = ap.add_mutually_exclusive_group(); g.add_argument('--write', action='store_true'); g.add_argument('--check', action='store_true')
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--rust', type=Path, help='a built m5b_real example (default: cargo build --release)')
    ap.add_argument('--selftest', action='store_true', help='also plant faults in a copy of each strict stream (not recorded)')
    ap.add_argument('--log', type=Path, help='append each run record as one JSON line (resumable bookkeeping)')
    a = ap.parse_args()
    if not a.iwad.is_file():
        print(f'skip: no IWAD at {a.iwad} (local-only M5b real-map comparison)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    work.mkdir(parents=True, exist_ok=True)
    a.screenblocks = [int(x) for x in a.screenblocks.split(',')]
    todo = runs_selected(set(a.maps.upper().split(',')) if a.maps else None, set(a.only.upper().split(',')) if a.only else None)
    if not todo: sys.exit('no run selected')
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try: bins = {v: m5b_build.build(v, work / 'build', a.cc) for v in VARIANTS}
        finally: sys.stdout = so
    rust = a.rust
    if rust is None:
        subprocess.run(['cargo', 'build', '-q', '--release', '-p', 'doom-render', '--example', 'm5b_real'], cwd=ROOT, check=True)
        rust = ROOT / 'target/release/examples/m5b_real'
    records, failures, stops, killed = {}, [], [], []
    order = sorted(todo, key=lambda r: -r['tics'])
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        for name, rec, fails, st, k in pool.map(lambda r: one(r, a, bins, rust, work), order):
            st_rec = rec.pop('selftest', None)
            if st_rec: print(f'{name} selftest: {st_rec}', flush=True)
            records[(rec['map'], rec['script'])] = rec
            failures += fails; stops += st; killed += [f'{name} {x}' for x in k]
            if a.log:
                with open(a.log, 'a') as f: f.write(json.dumps(dict(name=name, rec=rec, fails=fails, stops=st, killed=k)) + '\n')
            print(f"{name}: never {rec['never_dumps_equal_m4b']}; " + '; '.join(
                f"sb{sb} frames {v['frames']} whole {v.get('compared_whole')} masked {v.get('compared_masked')} "
                f"nc {v.get('non_comparable_whole')}+{v.get('non_comparable_parity')} msgs {v['messages_taken']}/{v.get('messages_rust')} "
                f"variants {'identical' if v['variants_identical'] else 'DIFFER'} dumps {v['dumps_equal_never']} "
                f"streams {'equal' if v.get('streams_equal') else 'DIFFER'} end {v['end']}"
                for sb, v in rec['views'].items()), flush=True)
    totals = {}
    for rec in records.values():
        for sb, v in rec['views'].items():
            t = totals.setdefault(sb, collections.Counter())
            for k in ('tics', 'frames', 'compared_whole', 'compared_masked', 'non_comparable_whole', 'non_comparable_parity',
                      'mismatched', 'fields_compared', 'message_tics_compared', 'messages_taken', 'messages_rust',
                      'nonzero_palette_frames', 'message_frames'):
                t[k] += v.get(k) or 0
    for sb, t in totals.items(): print(f'total sb{sb}: ' + ' '.join(f'{k} {n}' for k, n in t.items()))
    if a.write or a.check:
        doc = json.loads(RESULTS.read_text()) if RESULTS.exists() else None
        if a.check:
            if doc is None: sys.exit(f'{RESULTS.relative_to(ROOT)} does not exist')
            have = {(r['map'], r['script']): r for r in doc['runs']}
            bad = [f'{k[0]} {k[1]}' for k, rec in records.items() if have.get(k) != rec]
            if bad: failures.append('records differ from the committed file: ' + ', '.join(bad))
            else: print(f'{len(records)} records equal the committed {RESULTS.relative_to(ROOT)}')
        elif not (failures or stops or killed):
            doc = doc or dict(schema=SCHEMA, note=(
                'M5b step 5 real-map display comparison (local only): the genuine linuxdoom-1.10 status bar, HUD, palette '
                'and renderer (M5b runner, strict/overflow/address) against the Rust Display fed by the Rust simulation, every '
                'tic, screenblocks 10 and 11, high detail, the IWAD only in the lump list. Frames are identical to the genuine '
                'engine under the declared sound-free, wipe-free contract (decision S15). Counts, pass/fail facts and stream '
                'digests only; see tools/local/m5b_real.py for the digest rule.'), asan_options=ASAN, runs=[])
            doc['runner_contract_sha256'] = sha(m5b_build.CONTRACT.read_bytes())
            have = {(r['map'], r['script']): r for r in doc['runs']}
            have.update(records)
            pos = {(r['map'], r['script']): i for i, r in enumerate(json.loads(M4B_REAL.read_text())['runs'])}
            doc['runs'] = [have[k] for k in sorted(have, key=lambda k: pos.get(k, 1 << 30))]
            RESULTS.write_text(json.dumps(doc, indent=1) + '\n')
            print(f'wrote {len(records)} records into {RESULTS.relative_to(ROOT)} ({len(doc["runs"])} in all)')
    if killed: print('KILLED (possible OOM; re-run these with half the jobs):\n  ' + '\n  '.join(killed))
    if stops: print('S17 STOP CONDITIONS:\n  ' + '\n  '.join(stops))
    if failures: print('FAILURES:\n  ' + '\n  '.join(failures))
    return 1 if (failures or stops or killed) else 0


if __name__ == '__main__':
    sys.exit(main())
