#!/usr/bin/env python3
"""Local-only M5c real-map runs of the genuine sound logic and the full frames under the new contract (never in
CI; it reads the commercial IWAD). The genuine side only: no Rust.

For every run of tests/reference/m4b_real_results.json (the 61 runs: the committed E1M1-E1M9 scripts and DEMO1,
extracted from the IWAD copy and patched 109 to 110, tools/local/m5a_real.py), with the IWAD only in the lump
list (its own DS sound and D_ music lumps), the M5c runner (tools/reference/m5c_build.py) under the recommended
switches of tools/reference/m5c_reference.py (integrated handles, S_UpdateSounds after every tic, the genuine
wipe, sound on):

  1. strict, --render none, dumps: the masked dump set must equal the committed M4b strict dumps_sha256;
  2. at screenblocks 10 and 11, every tic: the fork run (--sound off --wipe none) must give the committed M5b
     genuine_stream_sha256 of tests/reference/m5b_real_results.json (tools/local/m5b_real.py's digest rule);
     strict with dumps (each snapshot equal to the never-render one), overflow and address (address with
     ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so this is not LeakSanitizer coverage) and a
     second strict run: frame and sound streams identical through the earliest end, every variant ending
     alike; the face check of section 3.4 against the fork run (outside the face rectangle nothing differs;
     at screenblocks 11 no frame differs); no sanitizer report in s_sound.c, i_sound.c or f_wipe.c;
  3. at screenblocks 10 the alternates (sndserv handles, the buffer schedule, wipe none): digests and counts.

Committed (tests/reference/m5c_real_results.json, schema DOOM-M5C-REAL-RESULTS-v1): counts, pass/fail facts and
digests only. Streams (which carry map coordinates and lump-derived data) stay in --work and are deleted.

    python3 tools/local/m5c_real.py [--iwad PATH] [--work DIR] [--maps E1M1] [--only WALK] [--jobs N]
                                    [--write | --check] [--cc CC]
"""
import argparse, collections, concurrent.futures as cf, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference')); sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m5c_build                 # noqa: E402
import m5c_reference as C        # noqa: E402  (read-only: run helpers, normalise, face check, sound facts)
import m5b_reference as B        # noqa: E402
import m5b_real as MR            # noqa: E402  (read-only: the M5b stream digest rule)
from m5a_real import runs_selected, demo_path  # noqa: E402

RESULTS = ROOT / 'tests/reference/m5c_real_results.json'
M5B_REAL = ROOT / 'tests/reference/m5b_real_results.json'
SCHEMA = 'DOOM-M5C-REAL-RESULTS-v1'
VARIANTS = ['strict', 'overflow', 'address']
sha = lambda b: hashlib.sha256(b).hexdigest()


def one(r, a, bins, work, m5b):
    name = f"{r['map']}_{Path(r['script']).stem.upper()}"
    W = work / 'runs' / name; W.mkdir(parents=True, exist_ok=True)
    lmp = demo_path(r, W, a.iwad)
    rec, fails = dict(map=r['map'], script=r['script'], views={}), []
    never = run1(bins['strict'], a.iwad, lmp, W / 'never', ['--render', 'none'], dumps=True, frames=False)
    rec['never_dumps_equal_m4b'] = never['dumps'] == r['dumps_sha256']['strict']
    if not rec['never_dumps_equal_m4b']: fails.append(f'{name}: masked never-render dumps differ from the M4b strict digest')
    sounds = {}
    for sb in a.screenblocks:
        base = ['--render', 'every', '--screenblocks', str(sb)]
        fork = run1(bins['strict'], a.iwad, lmp, W / f'{sb}_fork', base + ['--sound', 'off', '--wipe', 'none'], sound=False)
        fg = B.normalise(fork['frames'])[0]
        fork_ok = sha(MR.stream_lines(B.frows_of(fg)).encode()) == m5b[(r['map'], r['script'])]['views'][str(sb)]['genuine_stream_sha256']
        if not fork_ok: fails.append(f'{name} sb{sb}: --sound off --wipe none differs from the M5b genuine stream digest')
        per = {}
        for v in VARIANTS + ['again']:
            x = run1(bins['strict' if v == 'again' else v], a.iwad, lmp, W / f'{sb}_{v}', base, dumps=(v == 'strict'))
            g, full = C.normalise(x['frames'])
            per[v] = dict(r=x, golden=g, full=full, end=C.end_of(x), last=C.last_tic_of(x))
        s = per['strict']
        last = min(p['last'] for p in per.values())
        same_end = len({p['end'] for p in per.values()}) == 1 and len({p['last'] for p in per.values()}) == 1
        tr = lambda t: C.truncate(t, last, same_end)
        fident = all(tr(p['full']) == tr(s['full']) for v, p in per.items() if v != 'again')
        sident = all(tr(p['r']['sound']) == tr(s['r']['sound']) for v, p in per.items() if v != 'again')
        determ = tr(per['again']['full']) == tr(s['full']) and tr(per['again']['r']['sound']) == tr(s['r']['sound'])
        nd = min(len(s['r']['per_snapshot']), len(never['per_snapshot']))
        dident = s['r']['per_snapshot'][:nd] == never['per_snapshot'][:nd] and nd > 0
        if not (fident and sident and determ and dident):
            fails.append(f'{name} sb{sb}: frames identical {fident}, sound identical {sident}, determinism {determ}, dumps {dident}')
        golden = C.truncate(s['golden'], last, True)
        fc = C.face_check(C.normalise(fork['frames'])[0], golden, sb)
        if not fc['ok']: fails.append(f"{name} sb{sb}: outside the face something differs ({len(fc['noface_digest_differs'])} face-zeroed, "
                                      f"{len(fc['other_status_differs'])} status, {fc['full_digest_differs']} whole)")
        reps = {v: B.A.report_of(p['r']['stderr']) for v, p in per.items() if v in ('overflow', 'address') and B.A.report_of(p['r']['stderr'])}
        sound_reps = sum(1 for x in reps.values() if any(f in x for f in C.SOUND_FILES))
        if sound_reps: fails.append(f'{name} sb{sb}: sanitizer report in a sound file: {reps}')
        view = dict(end=s['end'], end_tic=last, frames=fc['frames'], flagged_skipped=fc['flagged_skipped'],
                    face_differs=fc['face_differs'], full_digest_differs=fc['full_digest_differs'],
                    noface_digest_differs=len(fc['noface_digest_differs']), other_status_differs=len(fc['other_status_differs']),
                    fork_equals_m5b=fork_ok, variants_identical=fident, sound_variants_identical=sident, strict_runs_identical=determ,
                    dumps_equal_never=dident, sanitizer_reports=len(reps), sound_sanitizer_reports=sound_reps,
                    variant_endings={v: p['end'] for v, p in per.items() if v != 'again'},
                    frames_sha256=sha(golden.encode()), melt_steps=len(C.rows_of(golden, 'V')))
        if sb == 10:
            alts = {}
            for an, ex in C.ALTERNATES.items():
                x = run1(bins['strict'], a.iwad, lmp, W / f'{sb}_{an}', base + ex)
                ag = C.truncate(C.normalise(x['frames'])[0], C.last_tic_of(x), True)
                sf = C.sound_facts(x['sound'])
                alts[an] = dict(frames_sha256=sha(ag.encode()), sound_sha256=sha(''.join(l for l in x['sound'].splitlines(keepends=True) if not l.startswith('E,')).encode()), outcomes=sf['outcomes'],
                                stop_reasons=sf['stop_reasons'], updates=sf['Z'].get('updates'),
                                rnd_rows_equal_recommended=[q[:5] for q in C.rows_of(x['sound'], 'R')] == [q[:5] for q in C.rows_of(s['r']['sound'], 'R')],
                                face_frames_equal_recommended=all(x[:7] == y[:7] for x, y in zip(C.rows_of(ag, 'P'), C.rows_of(golden, 'P'))))   # through the shorter stream
            view['alternates'] = alts
        rec['views'][str(sb)] = view
        sounds[sb] = (s['r']['sound'], last)
    if len(sounds) == 2:
        lt = min(sounds[10][1], sounds[11][1])
        rec['sound_screens_identical'] = C.truncate(sounds[10][0], lt, False) == C.truncate(sounds[11][0], lt, False)
    st = C.truncate(sounds[a.screenblocks[0]][0], sounds[a.screenblocks[0]][1], True)
    sf = C.sound_facts(st)
    rec['sound'] = dict(sound_sha256=sha(st.encode()), Z=sf['Z'], outcomes=sf['outcomes'], okinds=sf['okinds'], stop_reasons=sf['stop_reasons'],
                        sfx_started_count=len(sf['sfx_started']), volumes=sf['volumes'], map8_volumes=sf['map8_volumes'],
                        separation_min_max=sf['separation_min_max'], occupied_after_update=sf['occupied_after_update'],
                        model_checked=sf['model_checked'], model_mismatch=sf['model_mismatch'],
                        branches=sorted(sf['branches']))
    if sf['model_mismatch']: fails.append(f'{name}: the independent model disagrees with {sf["model_mismatch"]} genuine starts')
    for k in ('mismatch', 'unknown_origin', 'other_draws'):
        if sf['Z'].get(k): fails.append(f'{name}: Z {k} = {sf["Z"][k]}')
    shutil.rmtree(W, ignore_errors=True)
    return name, rec, fails


def run1(binary, iwad, lmp, out, extra, dumps=False, frames=True, sound=True):
    """C.run with the IWAD as the only WAD before the demo lump (its own sound, music and graphics lumps)."""
    if out.exists(): shutil.rmtree(out)
    out.mkdir(parents=True)
    fp, sp = out / 'frames.txt', out / 'sound.txt'
    cmd = [str(binary), '--wad', str(iwad), '--wad', str(lmp), '--demo', lmp.stem.upper(),
           *(['--out', str(out)] if dumps else ['--no-dump']), *extra]
    if frames: cmd += ['--frames', str(fp)]
    if sound: cmd += ['--sound-out', str(sp)]
    p = subprocess.run(cmd, capture_output=True, text=True, env=C.ENV)
    names = C.dump.snapshot_names(str(out)) if dumps else []
    h, per = hashlib.sha256(), []
    for n in names:
        b = (out / n).read_bytes(); h.update(b); per.append(sha(b))
    r = dict(code=p.returncode, summary=B.A.SUMMARY.search(p.stdout), stderr=p.stderr, dumps=h.hexdigest(), snapshots=len(names),
             per_snapshot=per, frames=fp.read_text() if fp.exists() else '', sound=sp.read_text() if sp.exists() else '')
    shutil.rmtree(out)
    return r


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--iwad', type=Path, default=Path.home() / 'scratch/m5c/iwad/DOOM.WAD')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5c-real')
    ap.add_argument('--maps'); ap.add_argument('--only'); ap.add_argument('--jobs', type=int, default=2)
    ap.add_argument('--screenblocks', default='10,11')
    ap.add_argument('--cc', default='cc')
    g = ap.add_mutually_exclusive_group(); g.add_argument('--write', action='store_true'); g.add_argument('--check', action='store_true')
    a = ap.parse_args()
    a.screenblocks = [int(x) for x in a.screenblocks.split(',')]
    if not a.iwad.is_file():
        print(f'skip: no IWAD at {a.iwad} (local-only M5c real-map runs)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try: bins = {v: m5c_build.build(v, work / 'build', a.cc) for v in VARIANTS}
        finally: sys.stdout = so
    m5b = {(r['map'], r['script']): r for r in json.loads(M5B_REAL.read_text())['runs']}
    todo = runs_selected(a.maps.split(',') if a.maps else None, {x.upper() for x in a.only.split(',')} if a.only else None)
    recs, fails = {}, []
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        for name, rec, f in pool.map(lambda r: one(r, a, bins, work, m5b), todo):
            recs[name] = rec; fails += f
            v = rec['views']['10']; z = rec['sound']['Z']
            print(f"{name}: dumps {rec['never_dumps_equal_m4b']}; fork {v['fork_equals_m5b']}; identical {v['variants_identical']} "
                  f"{v['sound_variants_identical']} determ {v['strict_runs_identical']}; face {v['face_differs']}/{v['frames']}; "
                  f"starts {z.get('attempts')} started {z.get('started')} refused {z.get('no_channel')}; end {v['end']}", flush=True)
    totals = dict(runs=len(recs), tics=sum(r['views']['10']['end_tic'] + 1 for r in recs.values()),
                  starts=sum(r['sound']['Z'].get('attempts', 0) for r in recs.values()),
                  started=sum(r['sound']['Z'].get('started', 0) for r in recs.values()),
                  inaudible=sum(r['sound']['Z'].get('inaudible', 0) for r in recs.values()),
                  refused=sum(r['sound']['Z'].get('no_channel', 0) for r in recs.values()),
                  sound_draws=sum(r['sound']['Z'].get('snd_draws', 0) for r in recs.values()),
                  frames_compared={str(sb): sum(r['views'][str(sb)]['frames'] for r in recs.values()) for sb in a.screenblocks},
                  face_differs={str(sb): sum(r['views'][str(sb)]['face_differs'] for r in recs.values()) for sb in a.screenblocks},
                  full_frames_differ={str(sb): sum(r['views'][str(sb)]['full_digest_differs'] for r in recs.values()) for sb in a.screenblocks},
                  model_checked=sum(r['sound']['model_checked'] for r in recs.values()),
                  endings=dict(collections.Counter(r['views']['10']['end'] for r in recs.values())))
    print('totals:', json.dumps(totals))
    if fails: print('FAILED:\n  ' + '\n  '.join(fails)); return 1
    doc = dict(schema=SCHEMA, note='M5c real-map runs (local only): the genuine linuxdoom-1.10 sound logic (s_sound.c), level-start melt '
               '(f_wipe.c), status bar, HUD and renderer in the M5c runner (strict/overflow/address) under the recommended switches, '
               'the IWAD only in the lump list. Counts, pass/fail facts and digests only.',
               switches=C.RECOMMENDED, asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
               runner_contract_sha256=sha(m5c_build.CONTRACT.read_bytes()), totals=totals, runs=recs)
    text = json.dumps(doc, indent=1) + '\n'
    if a.check:
        if not RESULTS.exists() or RESULTS.read_text() != text: print(f'{RESULTS.relative_to(ROOT)} differs'); return 1
        print(f'{RESULTS.relative_to(ROOT)} matches ({len(recs)} runs)')
    elif a.write:
        RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
