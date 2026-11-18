#!/usr/bin/env python3
"""M5b step 7b: full-resolution tic commands injected into the genuine runner, matched in Rust.

Decision S3 (ii) of docs/M5B_REFERENCE_INVESTIGATION.md (owner ruling 2026-11-17): before the app may execute a built
command whose angleturn has a nonzero low byte (which no version-110 demo can carry, g_game.c:1500, 1512), the genuine
engine must be shown to agree with Rust on such commands. tools/reference/m5b_inject.c is the M5b runner with a
command-injection mode (contract tests/reference/m5b_inject_contract.json, schema DOOM-M5B-INJECT-v1): the runner's
driver text unchanged, each tic's command written where G_BuildTiccmd writes it under singletics (d_main.c:379) with
demoplayback clear, so G_Ticker executes it whole (g_game.c:656-669) and P_MovePlayer turns by angleturn<<16
(p_user.c:148-154).

Scripts (tests/reference/m5b_inject/*.txt): `wad`, `map`, `skill`, `screenblocks` lines, then commands in order:
  cmd F S A B [xN]            a literal command, N times
  golden NAME [FIRST COUNT]   the built commands (t rows F S A B, r rows expanded) of the genuine input driver's golden
                              tests/reference/m5b_input/goldens/NAME.golden: events -> genuine G_BuildTiccmd -> injected
  executed NAME [FIRST COUNT] the same golden's executed commands (XF XS XA XB, after the genuine demo round trip)
  control                     every low byte must be 0; the commands are also replayed as a demo through the unchanged
                              demo runner (m5b_build.py), which must give the same frames and dumps
Per script: the injection runner in the strict, overflow and address variants with --render every must give identical
frame streams, dumps and endings; the golden (tests/reference/m5b_inject/goldens/NAME.golden, C only: per tic the C row
of players[0].cmd from the dump, D the SHA-256 of the tic's game dump, then the normalised frame stream with its F, P
and X rows) is written by a plain run and compared by --check; the Rust replay of the same commands
(crates/doom-game/examples/m5b_session.rs --commands) must equal it on every tic through m5b_session.compare: the
executed command, the frame digest, the P row, the X rows and the game dump by the independent DSIM40 encoder
(level_compare_m4b.py). The strict runner also plays the script's demo-resolution twin (every angleturn rounded as
G_WriteDemoTiccmd and G_ReadDemoTiccmd do); the first tic whose dump differs is recorded, to show the low byte matters.
Unrecorded app sessions (tests/reference/m5b_inject/sessions/*.txt, the platform-event scripts of m5b_session.py):
the headless app plays each with --unrecorded (decision S3 (ii): the built command executed, nothing saved); the
genuine input driver, fed the app's posted events with recording off (config record 0), must build and execute the
same commands (events -> genuine G_BuildTiccmd); those commands are injected as above and the app's own live rows
must match the genuine runner on every tic in the same way.
Results: tests/reference/m5b_inject_results.json (DOOM-M5B-INJECT-RESULTS-v1). --selftest plants a changed low byte
in one injected command and a changed frame row of Rust; each must be caught at its tic. Frames are identical to the
genuine engine under the declared sound-free, wipe-free contract (decision S15), only for the committed scripts.
Address runs use ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, not LeakSanitizer coverage.

    m5b_inject.py [--check] [--selftest] [--work DIR] [--jobs N] [--cc CC] [--cargo CARGO]
    m5b_inject.py --contract   (rewrite tests/reference/m5b_inject_contract.json)
"""
import argparse, concurrent.futures as cf, json, os, shutil, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import m3h9c_build as base       # noqa: E402
import m5b_build                 # noqa: E402
import m5b_session as S          # noqa: E402

ROOT, HERE = S.ROOT, S.ROOT / 'tools/reference'
INJECT, DIR = HERE / 'm5b_inject.c', S.ROOT / 'tests/reference/m5b_inject'
GOLDENS, INPUT = DIR / 'goldens', S.ROOT / 'tests/reference/m5b_input/goldens'
CONTRACT, RESULTS = S.ROOT / 'tests/reference/m5b_inject_contract.json', S.ROOT / 'tests/reference/m5b_inject_results.json'
R, sha = S.R, S.sha


def contract():
    return dict(schema='DOOM-M5B-INJECT-v1', base_contract='tests/reference/m5b_runner_contract.json',
                base_contract_sha256=sha(m5b_build.CONTRACT.read_bytes()), inject_sha256=sha(INJECT.read_bytes()),
                driver_sha256=sha(m5b_build.DRIVER.read_bytes()),
                renames={'main': 'm5b_runner_main', 'G_Ticker': 'm5b_inject_ticker (the driver call and the g_game.h prototype)'},
                reason='A version-110 demo carries one byte of angleturn (g_game.c:1500, 1512), so demo playback cannot execute a command with a nonzero low byte; decision S3 (ii) needs genuine evidence for such commands before the app executes them unrecorded.',
                behaviour=[l.strip(' *') for l in INJECT.read_text().split('*/')[0].splitlines()[1:] if l.strip(' *')],
                build='m5b_build.build(variant) unchanged, then m5b_inject.c compiled with the driver flags of the variant in place of m5b_driver.c and linked with the same objects and link flags',
                demo_mode='without --commands the binary is the demo runner of the base contract: m5b_inject_ticker only calls G_Ticker')


def build(work, cc):
    out = {}
    for v in R.VARIANTS:
        b = m5b_build.build(v, work / 'build', cc)
        w = b.parent
        flags = base.VARIANTS[v] + m5b_build.DRIVER_FLAGS.get(v, [])
        shutil.copy2(INJECT, w / 'src' / INJECT.name)
        subprocess.run([cc, *flags, '-I', str(w / 'src'), '-c', str(w / 'src' / INJECT.name), '-o', str(w / 'obj/inject.o')], check=True)
        objs = [str(w / 'obj' / (f[:-2] + '.o')) for f in list(m5b_build.SOURCES) + ['layout_probe.c', m5b_build.STUBS.name]]
        out[v] = w / 'cinject'
        subprocess.run([cc, *flags, *m5b_build.LINK_FLAGS, *objs, str(w / 'obj/inject.o'), '-o', str(out[v])], check=True)
    return out, work / 'build/strict/crunner'


def rounded(a):
    """angleturn after G_WriteDemoTiccmd's (a+128)>>8 and G_ReadDemoTiccmd's byte<<8 into a short (g_game.c:1500, 1512)."""
    v = ((a + 128) >> 8) & 0xff
    return (v << 8) - (65536 if v >= 128 else 0)


def golden_cmds(name, cols, path=None):
    rows, prev = [], None
    for l in (path or INPUT / f'{name}.golden').read_text().splitlines():
        w = l.split()
        if w and w[0] == 't': prev = [int(x) for x in w[cols]]; rows.append(prev)
        elif w and w[0] == 'r': rows += [prev] * int(w[1])
    return rows


def parse(path):
    h, cmds, control = S.header(path.read_text()), [], False
    for l in path.read_text().splitlines():
        w = l.split('#')[0].split()
        if not w or w[0] in ('wad', 'map', 'skill', 'screenblocks'): continue
        if w[0] == 'control': control = True
        elif w[0] == 'cmd': cmds += [[int(x) for x in w[1:5]]] * (int(w[5][1:]) if len(w) > 5 else 1)
        elif w[0] in ('golden', 'executed'):
            g = golden_cmds(w[1], slice(2, 6) if w[0] == 'golden' else slice(6, 10))
            cmds += g[int(w[2]):int(w[2]) + int(w[3])] if len(w) > 3 else g
        else: sys.exit(f'{path.name}: bad line {l!r}')
    if control and any(c[2] & 0xff for c in cmds): sys.exit(f'{path.name}: a control with a nonzero low byte')
    return h, cmds, control


def demo(h, cmds=()):
    body = bytes(b for c in cmds for b in (c[0] & 0xff, c[1] & 0xff, (c[2] >> 8) & 0xff, c[3]))
    return bytes([110, h['skill'], 1, h['map'], 0, 0, 0, 0, 0, 1, 0, 0, 0]) + body + b'\x80'


def engine(binary, h, cmds, work, inject=True):
    work.mkdir(parents=True, exist_ok=True)
    (work.parent / f'{work.name}.cmds').write_text(''.join(f'{f} {s} {a} {b}\n' for f, s, a, b in cmds))
    extra = ('--commands', str(work.parent / f'{work.name}.cmds')) if inject else ()
    return S.run_engine(binary, h['wads'] + [R.GFX], demo(h) if inject else demo(h, cmds), h['screenblocks'], work, extra)


def rust(exe, h, cmds, work):
    if work.exists(): shutil.rmtree(work)
    work.mkdir(parents=True)
    (work / 'c.txt').write_text(''.join(f'{f} {s} {a} {b}\n' for f, s, a, b in cmds))
    p = subprocess.run([str(exe), *[a for w in h['wads'] + [R.GFX] for a in ('--wad', str(w))], '--out', str(work / 'out'), '--commands',
                        str(work / 'c.txt'), '--map', str(h['map']), '--skill', str(h['skill']), '--screenblocks', str(h['screenblocks']), '--captures'],
                       capture_output=True, text=True)
    if p.returncode: sys.exit(f'Rust replay failed: {p.stderr[-500:]}')
    return dict(rows=S.rows_of((work / 'out/rows.txt').read_text()), captures=work / 'out/captures')


def driver_cmds(driver, rows, work):
    """The commands the genuine input driver builds and executes for the app's posted events, recording off."""
    work.mkdir(parents=True, exist_ok=True)
    (work / 's.txt').write_text('config record 0\n' + S.driver_script(rows).replace('enddemo\n', ''))
    p = subprocess.run([str(driver), 's.txt', '--out', 'golden.txt'], capture_output=True, text=True, cwd=work, env=S.ENV)
    return p.returncode == 0 and [b + x for b, x in zip(golden_cmds('', slice(2, 6), work / 'golden.txt'), golden_cmds('', slice(6, 10), work / 'golden.txt'))]


def golden_text(g):
    cmd = []
    for n in S.L.m3h9c_dump.snapshot_names(str(g['dumps'])):
        cmd += [f'C,{r[1]},{",".join(r[6:10])}' for r in (l.split(',') for l in (g['dumps'] / n).read_text().splitlines())
                if r[0] == 'P' and r[2] == '0' and n != 'load.txt']
    return '\n'.join(cmd + [f'D,{i - 1},{d}' for i, d in enumerate(g['snapshots'])]) + '\n' + g['golden']


def one(path, tools, work):
    (exe, inj, demo_runner, driver), app, fails = tools, None, []
    if path.parent.name == 'sessions':
        h, control = S.header(path.read_text()), False
        app = S.run_app(exe, h['wads'] + [R.GFX], work / 'app', h['screenblocks'], path, None, h['map'], h['skill'], ('--unrecorded',))
        if 'error' in app: sys.exit(f"{path.name}: {app['error']}")
        cmds = [[int(x) for x in l.split(',')[2:6]] for l in app['rows']['T']]
        if app['lmp'] or driver_cmds(driver, app['rows'], work / 'driver') != [c + c for c in cmds]:
            fails.append((-1, 'input', 'the app saved a file, or the genuine input driver built other commands'))
    else:
        h, cmds, control = parse(path)
    gs = {v: engine(inj[v], h, cmds, work / v) for v in R.VARIANTS}
    s = gs['strict']
    same = all(x['full'] == s['full'] and x['snapshots'] == s['snapshots'] and x['end'] == s['end'] for x in gs.values())
    if not same: fails.append((-1, 'variants', {v: x['end'] for v, x in gs.items()}))
    app = app or rust(exe, h, cmds, work / 'rust')
    counts, f = S.compare(app, s, h['wads'][0], h['skill'], h['map'])
    fails += f
    rec = dict(tics=len(cmds), low_byte_commands=sum(1 for c in cmds if c[2] & 0xff), counts=counts, genuine_end=s['end'],
               variants_identical=same, golden_sha256=sha(golden_text(s).encode()))
    twin = engine(inj['strict'], h, [[a, b, rounded(c), d] for a, b, c, d in cmds], work / 'twin')
    diff = [i - 1 for i, (x, y) in enumerate(zip(s['snapshots'], twin['snapshots'])) if x != y]
    rec['rounded_twin'] = dict(first_differing_dump_tic=diff[0] if diff else None, end=twin['end'],
                               messages=len(twin['rows'].get('X', [])), messages_full_resolution=len(s['rows'].get('X', [])))
    if control:
        d = engine(demo_runner, h, cmds, work / 'demo', inject=False)
        rec['control_equals_demo_replay'] = d['full'] == s['full'] and d['snapshots'] == s['snapshots'] and d['end'] == s['end']
        if not rec['control_equals_demo_replay']: fails.append((-1, 'control', 'the injected control differs from its demo replay'))
    return rec, golden_text(s), fails, (h, cmds, app, s)


def selftest(tools, kept, work):
    exe, inj = tools[:2]
    h, cmds, app, s = kept
    k = next(i for i in range(5, len(cmds)) if cmds[i][2] & 0xff)
    bad = [list(c) for c in cmds]; bad[k][2] ^= 8
    g = engine(inj['strict'], h, bad, work / 'planted')
    f1 = S.compare(app, g, h['wads'][0], h['skill'], h['map'])[1][:1]
    rows = {kk: list(v) for kk, v in app['rows'].items()}
    i = next(j for j, l in enumerate(rows['F']) if l.split(',')[1] == str(k))
    rows['F'][i] = rows['F'][i].replace(rows['F'][i].split(',')[2], '0' * 64)
    f2 = S.compare(dict(app, rows=rows), s, h['wads'][0], h['skill'], h['map'])[1][:1]
    return [('changed low byte', f'angleturn of tic {k} xor 8 in the injected command; first failure {f1}', bool(f1) and f1[0][:2] == (k, 'command')),
            ('changed frame row', f'Rust frame digest of tic {k} replaced; first failure {f2}', bool(f2) and f2[0][:2] == (k, 'frame'))]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5b-inject')
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--contract', action='store_true'); ap.add_argument('--jobs', type=int, default=3)
    ap.add_argument('--cc', default='cc'); ap.add_argument('--cargo', default='cargo')
    a = ap.parse_args()
    text = json.dumps(contract(), indent=1) + '\n'
    if a.contract: CONTRACT.write_text(text); print(f'wrote {CONTRACT.relative_to(ROOT)}'); return
    if CONTRACT.read_text() != text: sys.exit(f'{CONTRACT.relative_to(ROOT)} differs from the regenerated contract')
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    exe = S.build_example(a.cargo)
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try: inj, demo_runner = build(work, a.cc); driver = S.ib.build('strict', work / 'inputbuild', a.cc, quiet=True)
        finally: sys.stdout = so
    tools = (exe, inj, demo_runner, driver)
    paths = sorted(DIR.glob('*.txt')) + sorted(DIR.glob('sessions/*.txt'))
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        out = dict(zip((p.stem for p in paths), pool.map(lambda p: one(p, tools, work / 'runs' / p.stem), paths)))
    failures, files = [], {}
    for name, (rec, gold, fails, _) in out.items():
        print(f"{name}: {rec['tics']} tics, {rec['low_byte_commands']} with a nonzero low byte; {rec['counts']}; end {rec['genuine_end']}; "
              f"variants identical {rec['variants_identical']}; rounded twin {rec['rounded_twin']}"
              + (f"; control equals demo replay {rec['control_equals_demo_replay']}" if 'control_equals_demo_replay' in rec else '')
              + f"; {'pass' if not fails else 'FAIL'}")
        failures += [f'{name}: tic {t} {k}: {x}' for t, k, x in fails[:5]]
        files[GOLDENS / f'{name}.golden'] = gold
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    doc = dict(schema='DOOM-M5B-INJECT-RESULTS-v1', contract=f'frames {S.CONTRACT}, for these scripts only', variants=R.VARIANTS,
               inject_contract_sha256=sha(CONTRACT.read_bytes()), gfx_sha256=sha(R.GFX.read_bytes()), asan_options='detect_leaks=0 (leak detection disabled)',
               scripts={n: dict(r[0], script_sha256=sha(p.read_bytes())) for p, (n, r) in zip(paths, out.items())})
    files[RESULTS] = json.dumps(doc, indent=1) + '\n'
    if a.check:
        bad = [str(p.relative_to(ROOT)) for p, t in files.items() if not p.exists() or p.read_text() != t]
        extra = [str(p.relative_to(ROOT)) for p in GOLDENS.glob('*') if p not in files]
        if bad or extra: sys.exit(f'differs: {bad}; not produced by this run: {extra}')
        print(f'{RESULTS.relative_to(ROOT)} and {len(files) - 1} goldens match')
    else:
        GOLDENS.mkdir(parents=True, exist_ok=True)
        for p, t in files.items(): p.write_text(t)
        print(f'wrote {RESULTS.relative_to(ROOT)} and {len(files) - 1} goldens')
    print(f'every tic and frame of {len(out)} scripts {S.CONTRACT}')
    if a.selftest:
        name = next(n for n, r in out.items() if r[0]['low_byte_commands'])
        res = selftest(tools, out[name][3], work / 'self')
        for what, detail, ok in res: print(f"selftest {what}: {detail}: {'pass' if ok else 'FAIL'}")
        if not all(ok for *_, ok in res): sys.exit(1)


if __name__ == '__main__':
    main()
