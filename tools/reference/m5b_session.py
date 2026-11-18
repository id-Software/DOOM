#!/usr/bin/env python3
"""M5b step 7: sessions through the app's own input path, replayed through the genuine engine.

docs/M5B_REFERENCE_INVESTIGATION.md section 2.3 and decisions S3, S12, S15, S16 and S22. Every session of
tests/reference/m5b_sessions/*.txt is a script of platform-level events with timestamps (written by hand, not
played by a person). For each one:

  app       the headless app (crates/doom-game/examples/m5b_session.rs, a release build) plays the script on a fake
            clock through the event layer, G_BuildTiccmd, the demo round trip, the recorder, the simulation and the
            composed display; it writes the demo it saved, its per-tic rows and the canonical state captures, and
            requires the replay of its own demo to give the same rows (the state reached live equals the recording);
  input     the app's posted events, tic by tic, become an event_t script of the genuine input driver
            (tools/reference/m5b_input_build.py, strict), whose recorded demo must be byte-identical to the app's;
  engine    the app's demo is replayed by the genuine M5b runner (tools/reference/m5b_build.py) in the strict,
            overflow and address variants with --render every; the frame streams (normalised as m5b_reference.py)
            and the game dumps must be identical across the variants, and on every tic: players[0].cmd equals the
            app's executed command, the frame digest equals the app's (a tracker-flagged frame on its masked digest
            and flagged columns; switch parity unknown or whole-frame unattributable is non-comparable), the P row
            (palette, palette set, face, message on, M_Random index) and the X rows equal the app's, and the masked
            game dump equals the Rust capture by the independent DSIM40 encoder (level_compare_m4b.py).

Frames are identical to the genuine engine under the declared sound-free, wipe-free contract (decision S15), never
claimed identical to a vanilla session without that qualifier. Results (tests/reference/m5b_session_results.json,
schema DOOM-M5B-SESSION-RESULTS-v1) hold counts and digests of the genuine side only (the input driver's demo, the
strict frame stream) and are written by a plain run and compared by --check. --selftest plants four faults on the
first session and requires each to be caught at its place: a changed command byte in the demo, a changed frame row of
the app, a dropped tic, and a dropped event in the input driver's script. Address runs use
ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so they are not LeakSanitizer coverage.

    m5b_session.py [--check] [--selftest] [--only NAME] [--work DIR] [--jobs N] [--cc CC] [--cargo CARGO]

tools/local/m5b_session_real.py is the local-only mode for real maps and recorded sessions (verify() below).
"""
import argparse, concurrent.futures as cf, hashlib, json, os, shutil, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5b_build                 # noqa: E402
import m5b_input_build as ib     # noqa: E402
import m5b_reference as R        # noqa: E402
import m5a_reference as A        # noqa: E402
import level_snapshot as L       # noqa: E402
import level_compare_m4b as LC   # noqa: E402

SESSIONS = ROOT / 'tests/reference/m5b_sessions'
RESULTS = ROOT / 'tests/reference/m5b_session_results.json'
SCHEMA = 'DOOM-M5B-SESSION-RESULTS-v1'
CONTRACT = 'identical to the genuine engine under the declared sound-free, wipe-free contract (decision S15)'
VARIANTS, LUMP, ENV = R.VARIANTS, 'M5BSESS', R.ENV
sha = lambda b: hashlib.sha256(b).hexdigest()


def header(text):
    """The script's wad, map, skill and screenblocks lines."""
    h = dict(wads=[], map=1, skill=2, screenblocks=10)
    for l in text.splitlines():
        w = l.split('#')[0].split()
        if w and w[0] == 'wad': h['wads'].append(ROOT / w[1])
        elif w and w[0] in ('map', 'skill', 'screenblocks'): h[w[0]] = int(w[1])
    return h


def build_example(cargo):
    """Builds the m5b_session example (release) and returns the executable cargo itself reports, so the run uses
    the binary just built wherever CARGO_TARGET_DIR or a cargo configuration puts it."""
    p = subprocess.run([cargo, 'build', '--release', '--locked', '-p', 'doom-game', '--example', 'm5b_session',
                        '--message-format=json-render-diagnostics'], cwd=ROOT, capture_output=True, text=True)
    if p.returncode: sys.exit(f'cargo build FAILED:\n{p.stderr[-2000:]}')
    exes = [m['executable'] for m in map(json.loads, p.stdout.splitlines())
            if m.get('reason') == 'compiler-artifact' and m['target']['name'] == 'm5b_session' and m.get('executable')]
    if len(exes) != 1: sys.exit(f'cargo reported {len(exes)} m5b_session executables')
    return Path(exes[0])


def build(work, cc, cargo):
    exe = build_example(cargo)
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try:
            bins = {v: m5b_build.build(v, work / 'build', cc) for v in VARIANTS}
            driver = ib.build('strict', work / 'inputbuild', cc, quiet=True)
        finally: sys.stdout = so
    return exe, bins, driver


def rows_of(text):
    out = {}
    for l in text.splitlines():
        if l and not l.startswith('#'): out.setdefault(l.split(',', 1)[0], []).append(l)
    return out


def run_app(exe, wads, out, sb, script=None, lmp=None, mapnum=1, skill=2, extra=()):
    if out.exists(): shutil.rmtree(out)
    cmd = [str(exe), *[a for w in wads for a in ('--wad', str(w))], '--out', str(out), '--screenblocks', str(sb), '--captures']
    cmd += ['--script', str(script), '--map', str(mapnum), '--skill', str(skill)] if script else ['--demo', str(lmp)]
    cmd += list(extra)
    p = subprocess.run(cmd, capture_output=True, text=True)
    if p.returncode: return dict(error=f'app failed: {p.stderr.strip()[-500:]}')
    return dict(rows=rows_of((out / 'rows.txt').read_text()), lmp=(out / 'session.lmp').read_bytes(),
                ending=p.stdout.strip().split('ending ', 1)[-1], captures=out / 'captures')


def driver_script(rows, drop_event=None):
    """The app's posted events as an event_t script of the genuine input driver, one `tic` per executed tic."""
    events = {}
    for i, l in enumerate(rows.get('E', [])):
        if i == drop_event: continue
        _, t, kind, d1, d2, d3 = l.split(',')
        events.setdefault(int(t), []).append(f'ev {kind} {d1} {d2} {d3}' if kind == 'mouse' else f'ev {kind} {int(d1):#x}')
    out = []
    for l in rows['T']:
        out += events.get(int(l.split(',')[1]), []) + ['tic']
    return '\n'.join(out + ['enddemo']) + '\n'


def run_driver(binary, text, work):
    work.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=work) as cwd:
        (Path(cwd) / 's.txt').write_text(text)
        p = subprocess.run([str(binary), 's.txt', '--out', 'golden.txt'], capture_output=True, text=True, cwd=cwd, env=ENV)
        lmp = Path(cwd) / 'm5binput.lmp'
        return lmp.read_bytes() if p.returncode == 0 and lmp.exists() else None


def run_engine(binary, wads, lmp, sb, work, extra=()):
    """One genuine replay with --render every (and `extra` arguments): dict(golden, full, rows, end, dumps dir, per-snapshot digests)."""
    if work.exists(): shutil.rmtree(work)
    (work / 'dumps').mkdir(parents=True)
    demo = work / f'{LUMP}.lmp'; demo.write_bytes(lmp)
    frames = work / 'frames.txt'
    cmd = [str(binary), *[a for w in wads for a in ('--wad', str(w))], '--wad', str(demo), '--demo', LUMP, '--out', str(work / 'dumps'),
           '--render', 'every', '--screenblocks', str(sb), '--frames', str(frames), *extra]
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV)
    text = frames.read_text() if frames.exists() else ''
    golden, full = R.normalise(text)
    names = L.m3h9c_dump.snapshot_names(str(work / 'dumps'))
    r = dict(summary=A.SUMMARY.search(p.stdout), stderr=p.stderr, code=p.returncode)
    return dict(golden=golden, full=full, rows=rows_of(golden), end=R.ending(r, R.frows_of(text)), dumps=work / 'dumps',
                demo=demo, snapshots=[sha((work / 'dumps' / n).read_bytes()) for n in names])


def compare(app, g, wad, skill, mapnum):
    """Every per-tic comparison of the app's rows with one genuine replay: (counts, failures [(tic, kind, text)])."""
    fails, n = [], dict(tics=len(app['rows']['T']), commands=0, frames=0, digests=0, masked=0, non_comparable=0, fields=0,
                        messages=0, dump_tics=0)
    cmds = {}
    for name in L.m3h9c_dump.snapshot_names(str(g['dumps'])):
        for l in (g['dumps'] / name).read_text().splitlines():
            r = l.split(',')
            if r[0] == 'P' and r[2] == '0' and name != 'load.txt': cmds[int(r[1])] = r[6:10]
    gf = {int(l.split(',')[1]): l.split(',') for l in g['rows'].get('F', [])}
    gu, gp = {}, {int(l.split(',')[1]): l for l in g['rows'].get('P', [])}
    for l in g['rows'].get('U', []): gu.setdefault(int(l.split(',')[1]), []).append(l.split(',')[2])
    ap = {int(l.split(',')[1]): l for l in app['rows'].get('P', [])}
    af = {int(l.split(',')[1]): l.split(',') for l in app['rows'].get('F', [])}
    if len(gf) != n['tics']: fails.append((min(len(gf), n['tics']), 'tics', f'genuine frames {len(gf)}, app tics {n["tics"]}'))
    for l in app['rows']['T']:
        t, c = int(l.split(',')[1]), l.split(',')[2:6]
        if cmds.get(t) != c: fails.append((t, 'command', f'genuine {cmds.get(t)} app {c}'))
        else: n['commands'] += 1
        if t not in gf: continue
        f, a = gf[t], af[t]
        n['frames'] += 1
        if f[1 + R.FCOLS.index('whole')] == '1' or a[5] == '1': n['non_comparable'] += 1
        elif int(f[1 + R.FCOLS.index('columns')]) == 0:
            n['digests'] += 1
            if (a[2], a[4]) != (f[2], ''): fails.append((t, 'frame', 'frame digest differs'))
        else:
            n['masked'] += 1
            if (a[3], a[4].split()) != (f[3], gu.get(t, [])): fails.append((t, 'frame', 'masked frame or flagged columns differ'))
        n['fields'] += 5
        if gp.get(t) != ap.get(t): fails.append((t, 'status', f'genuine {gp.get(t)} app {ap.get(t)}'))
    gx, ax = g['rows'].get('X', []), app['rows'].get('X', [])
    n['messages'] = len(gx)
    if gx != ax: fails.append((-1, 'messages', f'genuine {gx} app {ax}'))
    res, _ = LC.compare(str(g['dumps']), str(app['captures']), L.MapData(str(wad), f'E1M{mapnum}', L.demo_lumps(g['demo'])),
                        skill, mapnum)
    n['dump_tics'] = res['tics_compared']
    if not res['match']: fails.append(((res['first_divergence'] or {}).get('tic', -1), 'state', f'game dump: {res}'))
    return n, sorted(fails, key=lambda x: (x[0] < 0, x[0]))


def verify(tools, wads, sb, work, script=None, lmp=None, mapnum=1, skill=2, variants=VARIANTS):
    """One session through the app (or a recorded demo through the Rust replay) and the genuine engine."""
    exe, bins, driver = tools
    app = run_app(exe, wads, work / 'app', sb, script, lmp, mapnum, skill)
    if 'error' in app: return dict(ok=False, failures=[(-1, 'app', app['error'])])
    rec = dict(tics=len(app['rows']['T']), app_ending=app['ending'], failures=[])
    if script:
        demo = run_driver(driver, driver_script(app['rows']), work / 'driver')
        rec['driver_demo_sha256'] = demo and sha(demo)
        same = demo is not None and (demo == app['lmp'] or ((skill, mapnum) != (2, 1) and demo[13:] == app['lmp'][13:]))
        if not same: rec['failures'].append((-1, 'input', 'the genuine input driver recorded a different demo'))
    gs = {v: run_engine(bins[v], wads, app['lmp'], sb, work / v) for v in variants}
    s = gs[variants[0]]
    rec['variants_identical'] = all(x['full'] == s['full'] and x['snapshots'] == s['snapshots'] and x['end'] == s['end'] for x in gs.values())
    if not rec['variants_identical']: rec['failures'].append((-1, 'variants', {v: x['end'] for v, x in gs.items()}))
    rec['counts'], fails = compare(app, s, wads[0], app['lmp'][1], app['lmp'][3])
    rec['failures'] += fails
    P = [l.split(',') for l in s['rows'].get('P', [])]
    rec.update(genuine_end=s['end'], frames_sha256=sha(s['golden'].encode()), palettes=sorted({int(p[2]) for p in P}),
               faces=sorted({int(p[4]) for p in P}), ok=not rec['failures'])
    for x in gs.values(): shutil.rmtree(x['dumps'])
    return rec


def selftest(tools, path, work):
    """Plant four faults on one session; each must be caught at its place."""
    exe, bins, driver = tools
    h = header(path.read_text())
    app = run_app(exe, h['wads'] + [R.GFX], work / 'app', h['screenblocks'], path, None, h['map'], h['skill'])
    T = [l.split(',')[2:6] for l in app['rows']['T']]
    k = next(i for i in range(10, len(T) - 1) if T[i] != T[i + 1] and T[i][0] != '5')
    out = []

    def engine(lmp, rows=None):
        g = run_engine(bins['strict'], h['wads'] + [R.GFX], lmp, h['screenblocks'], work / 'planted')
        first = compare(dict(app, rows=rows or app['rows']), g, h['wads'][0], h['skill'], h['map'])[1][:1]
        shutil.rmtree(g['dumps']); return first
    lmp = bytearray(app['lmp']); lmp[13 + 4 * k] = 5
    f = engine(bytes(lmp))
    out.append(('changed command byte', f'forward of tic {k} set to 5; first failure {f}', f[:1] and f[0][:2] == (k, 'command')))
    rows = {kk: list(v) for kk, v in app['rows'].items()}
    i = next(j for j, l in enumerate(rows['F']) if l.split(',')[1] == str(k))
    rows['F'][i] = rows['F'][i].replace(rows['F'][i].split(',')[2], '0' * 64)
    f = engine(app['lmp'], rows)
    out.append(('changed frame row', f'app frame digest of tic {k} replaced; first failure {f}', f[:1] and f[0][:2] == (k, 'frame')))
    f = engine(app['lmp'][:13 + 4 * k] + app['lmp'][13 + 4 * (k + 1):])
    out.append(('dropped tic', f'tic {k} removed from the demo; first failure {f}', f[:1] and f[0][:2] == (k, 'command')))
    drop = next(j for j, l in enumerate(app['rows']['E']) if ',keydown,' in l)
    demo = run_driver(driver, driver_script(app['rows'], drop), work / 'driver')
    out.append(('dropped event', f'event {drop} left out of the input driver script; demos equal {demo == app["lmp"]}', demo != app['lmp']))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5b-session')
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--only'); ap.add_argument('--jobs', type=int, default=3)
    ap.add_argument('--cc', default='cc'); ap.add_argument('--cargo', default='cargo')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    tools = build(work, a.cc, a.cargo)
    paths = sorted(p for p in SESSIONS.glob('*.txt') if not a.only or p.stem == a.only)
    def one(p):
        h = header(p.read_text())
        return verify(tools, h['wads'] + [R.GFX], h['screenblocks'], work / 'runs' / p.stem, p, None, h['map'], h['skill'])
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        recs = dict(zip((p.stem for p in paths), pool.map(one, paths)))
    failures = []
    for name, r in recs.items():
        print(f"{name}: {r.get('tics')} tics; {r.get('counts')}; genuine end {r.get('genuine_end')}; app {r.get('app_ending')}; "
              f"variants identical {r.get('variants_identical')}; input demo identical {not any(f[1] == 'input' for f in r['failures'])}; "
              f"{'pass' if r['ok'] else 'FAIL'}")
        failures += [f'{name}: tic {t} {k}: {x}' for t, k, x in r['failures'][:5]]
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    if not a.only:
        doc = dict(schema=SCHEMA, contract=f'frames {CONTRACT}', variants=VARIANTS, gfx_sha256=sha(R.GFX.read_bytes()),
                   runner_contract_sha256=sha(m5b_build.CONTRACT.read_bytes()), asan_options='detect_leaks=0 (leak detection disabled)',
                   sessions={n: dict({k: v for k, v in r.items() if k not in ('ok', 'failures', 'app_ending')}, script_sha256=sha((SESSIONS / f'{n}.txt').read_bytes()))
                             for n, r in recs.items()})
        text = json.dumps(doc, indent=1) + '\n'
        if a.check:
            if not RESULTS.exists() or RESULTS.read_text() != text: sys.exit(f'{RESULTS.relative_to(ROOT)} differs')
            print(f'{RESULTS.relative_to(ROOT)} matches ({len(recs)} sessions)')
        else:
            RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)}')
    print(f'every tic and frame of {len(recs)} sessions {CONTRACT}')
    if a.selftest:
        outcomes = selftest(tools, paths[0], work / 'self')
        for what, detail, ok in outcomes: print(f"selftest {what}: {detail}: {'pass' if ok else 'FAIL'}")
        if not all(ok for *_, ok in outcomes): sys.exit(1)


if __name__ == '__main__':
    main()
