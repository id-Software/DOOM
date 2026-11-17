#!/usr/bin/env python3
"""Genuine input-path reference runs of the M5b input driver (contract DOOM-M5B-INPUT-v1).

Expected commands come from the genuine linuxdoom-1.10 input path only (docs/M5B_REFERENCE_INVESTIGATION.md
section 2.2; owner rulings S3-S7, S11, S22). The driver (tools/reference/m5b_input_build.py) is built in
the strict, overflow and address variants under --work (and strict32 with --strict32, local only, never
required). Every script of tests/reference/m5b_input/scripts (tools/reference/m5b_input_scripts.py) runs
in each variant in a private empty working directory.

--check requires, and a plain run (which writes the goldens) also requires:
  - the committed scripts equal a fresh generation and the committed contract equals the regenerated one;
  - every run exits 0 with no sanitizer report, and the three variants write byte-identical goldens;
  - the recorded demo is consistent: every t row's four demo bytes are the G_WriteDemoTiccmd encoding of
    the built command (g_game.c:1510-1513) and the executed command is their G_ReadDemoTiccmd decoding
    (g_game.c:1499-1502); where the run wrote m5binput.lmp (enddemo, q, the length limit), the file equals
    the h row, the t rows' bytes and the 0x80 marker (g_game.c:1678-1679), checked in Python;
  - the witness scripts (beyond the app's event domain) pass in strict and stop the overflow build with a
    signed-integer-overflow report at g_game.c:579 or 580;
  - the chain: the demo chain_walk records is packed into a PWAD and replayed by the genuine M5a runner
    (tools/reference/m5a_build.py, strict) on tests/reference/m5a_wads/peg.wad with tests/reference/m5a_gfx.wad;
    every replayed players[0].cmd must equal the golden's executed command (skip with --no-chain);
  - with --check: tests/reference/m5b_input/goldens/*.golden and tests/reference/m5b_input_results.json equal
    the strict run.

--strict32 also builds the -m32 genuine-text build (no adapter) and requires its goldens to equal strict.
--summary prints the command domain of the goldens and the rows outside what wire 24 admits (forward and side
-127..127, buttons BT_ATTACK, BT_USE, BT_CHANGE and the weapon bits; -128 and bits 0x40 and 0x80 refused).
--selftest plants four faults and requires each to be caught: a flipped bit of one built command
(--selftest-cmd), a dropped event (--selftest-drop-event), a changed default binding and a changed turn table
entry (SELFTEST_MUTANTS of m5b_input_build.py, strict builds of mutated text that are never contract builds).

Address runs use ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so they are not LeakSanitizer
coverage. --jobs sets the number of driver processes run at once (default 4).

    m5b_input_reference.py [--work DIR] [--check] [--selftest] [--strict32] [--summary] [--no-chain]
                           [--jobs N] [--cc CC]
"""
import argparse, concurrent.futures as cf, hashlib, json, os, re, shutil, struct, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5b_input_build as ib       # noqa: E402
import m5b_input_scripts as isc    # noqa: E402

DIR = ROOT / 'tests/reference/m5b_input'
SCRIPTS, WITNESS, GOLDENS = DIR / 'scripts', DIR / 'witness', DIR / 'goldens'
RESULTS = ROOT / 'tests/reference/m5b_input_results.json'
SCHEMA = 'DOOM-M5B-INPUT-RESULTS-v1'
VARIANTS = ['strict', 'overflow', 'address']
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
CHAIN, CHAIN_LUMP = 'chain_walk', 'M5BCHAIN'
PEG, GFX = ROOT / 'tests/reference/m5a_wads/peg.wad', ROOT / 'tests/reference/m5a_gfx.wad'
APP_EXCLUDED = ('x_', 's5_', 'beyond_app_')
SANITIZER = re.compile(r'runtime error|AddressSanitizer|UndefinedBehaviorSanitizer|LeakSanitizer|SUMMARY: ')
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


# ---------------------------------------------------------------- golden parsing
def expand(text):
    """Golden rows with r rows expanded: a list of (tic or None, kind, fields, line)."""
    out, prev = [], None
    for line in text.splitlines():
        if not line or line.startswith('#'): continue
        f = line.split()
        if f[0] == 'r':
            if prev is None: raise ValueError('r row without a preceding t row')
            for _ in range(int(f[1])):
                prev = ['t', str(int(prev[1]) + 1)] + prev[2:]
                out.append((int(prev[1]), 't', prev, ' '.join(prev)))
            continue
        tic = None if f[0] == 'h' else int(f[1])
        out.append((tic, f[0], f, line))
        prev = f if f[0] == 't' else None
    return out


def t_rows(text): return [f for _, k, f, _ in expand(text) if k == 't']


def first_difference(a, b):
    """(tic, line_a, line_b) of the first differing expanded row, or None."""
    ea, eb = expand(a), expand(b)
    for x, y in zip(ea, eb):
        if x[3] != y[3]: return (x[0] if x[0] is not None else y[0], x[3], y[3])
    if len(ea) != len(eb):
        r = (ea + eb)[min(len(ea), len(eb))]
        return (r[0], ea[len(eb)][3] if len(ea) > len(eb) else '<end>', eb[len(ea)][3] if len(eb) > len(ea) else '<end>')
    return None


def s8(v): return v - 256 if v > 127 else v


def demo_checks(name, text, lmp):
    """Python consistency checks of the recorded demo (see the module docstring); returns a list of failures."""
    bad, body = [], b''
    rows = expand(text)
    head = [f for _, k, f, _ in rows if k == 'h']
    for f in t_rows(text):
        fw, sd, at, bt, xf, xs, xa, xb = (int(v) for v in f[2:10])
        if f[10] == '-': continue
        b = bytes.fromhex(f[10])
        enc = bytes([fw & 0xff, sd & 0xff, ((at + 128) >> 8) & 0xff, bt & 0xff])
        if b != enc: bad.append(f'{name} tic {f[1]}: demo bytes {f[10]} are not the encoding {enc.hex()} of the built command')
        dec = (s8(b[0]), s8(b[1]), struct.unpack('<h', struct.pack('<H', b[2] << 8))[0], b[3])
        if dec != (xf, xs, xa, xb): bad.append(f'{name} tic {f[1]}: executed command {xf, xs, xa, xb} is not the decoding {dec}')
        body += b
    if lmp is not None:
        if not head: bad.append(f'{name}: a demo file without an h row')
        elif lmp != bytes.fromhex(head[0][1]) + body + b'\x80':
            bad.append(f'{name}: m5binput.lmp ({len(lmp)} bytes) differs from the h row, the t rows and the marker')
    z = [f for _, k, f, _ in rows if k == 'z']
    if z and head and int(z[0][2]) != 13 + len(body): bad.append(f'{name}: z row demo length {z[0][2]} != {13 + len(body)}')
    return bad


# ---------------------------------------------------------------- the command domain
FIELDS = ['forwardmove', 'sidemove', 'angleturn', 'buttons']


def domain(goldens):
    """Min and max of each built and executed field, distinct buttons, and the rows outside wire 24."""
    mins, maxs, buttons, outside, tics, low_byte = {}, {}, {}, [], 0, 0
    for name, text in sorted(goldens.items()):
        rows = expand(text)
        for f in (f for _, k, f, _ in rows if k == 't'):
            tics += 1
            v = [int(x) for x in f[2:10]]
            if v[2] & 0xff: low_byte += 1
            for i, n in enumerate(['built_' + x for x in FIELDS] + ['exec_' + x for x in FIELDS]):
                mins[n] = min(mins.get(n, v[i]), v[i]); maxs[n] = max(maxs.get(n, v[i]), v[i])
            buttons.setdefault(v[3], set()).add(name)
            for side, (fw, sd, _, bt) in (('built', v[0:4]), ('exec', v[4:8])):
                why = []
                if not -127 <= fw <= 127: why.append(f'forwardmove {fw}')
                if not -127 <= sd <= 127: why.append(f'sidemove {sd}')
                if bt & 0x80: why.append('BT_SPECIAL (0x80)')
                if bt & 0x40: why.append('bit 0x40')
                if bt & 0x38 and not bt & 0x04 and not bt & 0x80: why.append('weapon bits without BT_CHANGE')
                if why: outside.append(dict(script=name, tic=int(f[1]), side=side, buttons=bt, why=why))
        for _, k, f, _ in rows:
            if k == 'c': outside.append(dict(script=name, tic=int(f[1]), side='both', why=['consistancy or chatchar nonzero']))
    return dict(tics=tics, built_turns_with_low_byte=low_byte, min=mins, max=maxs,
                distinct_buttons={str(b): len(s) for b, s in sorted(buttons.items())}, outside_wire24=outside)


# ---------------------------------------------------------------- running
def run_one(binary, script, work, extra=()):
    """Run the driver on SCRIPT in a private empty directory: dict(code, golden, stderr, lmp)."""
    work.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=work) as cwd:
        out = Path(cwd) / 'golden.txt'
        p = subprocess.run([str(binary), str(script), '--out', str(out), *extra], capture_output=True, text=True, cwd=cwd, env=ENV)
        lmp = Path(cwd) / 'm5binput.lmp'
        return dict(code=p.returncode, golden=out.read_text() if out.exists() else '', stderr=p.stderr,
                    lmp=lmp.read_bytes() if lmp.exists() else None)


def run_all(binary, scripts, work, jobs):
    with cf.ThreadPoolExecutor(jobs) as pool:
        futs = {s.stem: pool.submit(run_one, binary, s, work) for s in scripts}
        return {n: f.result() for n, f in futs.items()}


def build_quiet(variant, out, cc, mutant=None):
    return ib.build(variant, out, cc, mutant, quiet=True)


def chain(work, cc, golden, lmp):
    """Replay the chain demo through the genuine M5a runner; returns (summary text, failures)."""
    import m5a_build
    so, sys.stdout = sys.stdout, open(os.devnull, 'w')
    try: crunner = m5a_build.build('strict', work / 'm5a', cc)
    finally: sys.stdout.close(); sys.stdout = so
    wad = work / 'chain.wad'
    name = CHAIN_LUMP.encode().ljust(8, b'\0')
    wad.write_bytes(struct.pack('<4sii', b'PWAD', 1, 12 + len(lmp)) + lmp + struct.pack('<ii8s', 12, len(lmp), name))
    dumps = work / 'chain_dumps'
    if dumps.exists(): shutil.rmtree(dumps)
    dumps.mkdir()
    p = subprocess.run([str(crunner), '--wad', str(PEG), '--wad', str(GFX), '--wad', str(wad), '--demo', CHAIN_LUMP, '--out', str(dumps)],
                       capture_output=True, text=True, env=ENV)
    m = re.search(r'^summary: tics (\d+) end=(\w+) end_tic (\d+) .*checkheap (\S+)', p.stdout, re.M)
    replayed = []
    for f in sorted(dumps.glob('tic_*.txt')):
        for line in f.read_text().splitlines():
            r = line.split(',')
            if r[0] == 'P' and r[2] == '0': replayed.append((int(r[1]), tuple(int(x) for x in r[6:10])))
    shutil.rmtree(dumps)
    want = [(int(f[1]), tuple(int(x) for x in f[6:10])) for f in t_rows(golden)]
    bad = []
    if p.returncode or not m: bad.append(f'M5a runner failed (exit {p.returncode}): {p.stderr[-300:]}')
    if len(replayed) != len(want): bad.append(f'replayed {len(replayed)} tics, the golden has {len(want)}')
    diff = [(a, b) for a, b in zip(want, replayed) if a[1] != b[1]]
    if diff: bad.append(f'{len(diff)} tics differ, first: golden {diff[0][0]} replayed {diff[0][1]}')
    matched = sum(1 for a, b in zip(want, replayed) if a[1] == b[1])
    summary = (f'chain: {CHAIN} demo ({len(lmp)} bytes, sha256 {sha(lmp)[:16]}) replayed by the genuine M5a strict runner on '
               f'peg.wad: {m.group(0) if m else "no summary"}; {matched} of {len(want)} replayed players[0].cmd equal the executed commands')
    return dict(summary=summary, demo_bytes=len(lmp), demo_sha256=sha(lmp), tics=len(want), matched=matched,
                runner_end=m.group(2) if m else None, checkheap=m.group(4) if m else None), bad


def selftest(work, cc, jobs, committed, scripts):
    fails, lines = [], []
    strict = build_quiet('strict', work / 'build', cc)
    # 1. a flipped command bit
    name, t = 'ev_turn_left_held', 3
    r = run_one(strict, SCRIPTS / f'{name}.txt', work / 'runs', ['--selftest-cmd', str(t)])
    d = first_difference(committed[name], r['golden'])
    ok = d is not None and d[0] == t and d[1].startswith(f't {t} ')
    lines.append(f'selftest flipped command bit: {name} tic {t}; first difference at tic {d[0] if d else None}: {"pass" if ok else "FAIL"}')
    if not ok: fails.append('flipped command bit')
    # 2. a dropped event
    name, n = 'ev_bind_up_hold', 1
    r = run_one(strict, SCRIPTS / f'{name}.txt', work / 'runs', ['--selftest-drop-event', str(n)])
    m = re.search(r'selftest: event \d+ not posted at tic (\d+)', r['stderr'])
    d = first_difference(committed[name], r['golden'])
    ok = m is not None and d is not None and d[0] == int(m.group(1))
    lines.append(f'selftest dropped event: {name} event {n} (posted at tic {m.group(1) if m else None}); first difference at '
                 f'tic {d[0] if d else None}: {"pass" if ok else "FAIL"}')
    if not ok: fails.append('dropped event')
    # 3, 4. mutated genuine text (never a contract build)
    for mutant, must in (('binding', ('ev_bind_up_hold', 1)), ('turn_table', ('ev_turn_left_held', 1))):
        b = build_quiet('strict', work / 'build', cc, mutant)
        res = run_all(b, scripts, work / 'runs', jobs)
        differ = {n: first_difference(committed[n], r['golden']) for n, r in res.items() if r['golden'] != committed[n]}
        d = differ.get(must[0])
        ok = d is not None and d[0] == must[1]
        lines.append(f'selftest mutant {mutant} ({ib.SELFTEST_MUTANTS[mutant]["file"]}:{ib.SELFTEST_MUTANTS[mutant]["line"]}): '
                     f'{len(differ)} of {len(res)} scripts differ; {must[0]} first differs at tic {d[0] if d else None} '
                     f'(expected {must[1]}): {"pass" if ok else "FAIL"}')
        if not ok: fails.append(f'mutant {mutant}')
    return lines, fails


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5b-input-reference')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--strict32', action='store_true')
    ap.add_argument('--summary', action='store_true')
    ap.add_argument('--no-chain', action='store_true')
    ap.add_argument('--jobs', type=int, default=4)
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    jobs = max(1, a.jobs)
    failures = []
    # the scripts and the contract
    gen = isc.generate()
    have = {p.stem: p.read_text() for p in SCRIPTS.glob('*.txt')}
    if have != gen['scripts'] or {p.stem: p.read_text() for p in WITNESS.glob('*.txt')} != gen['witness']:
        sys.exit('the committed scripts differ from tools/reference/m5b_input_scripts.py (run it, or its --check)')
    if not ib.CONTRACT.exists() or ib.CONTRACT.read_text() != ib.render_json(ib.contract()):
        sys.exit('tests/reference/m5b_input_contract.json differs from the regenerated contract (m5b_input_build.py --check)')
    scripts = sorted(SCRIPTS.glob('*.txt'))
    witnesses = sorted(WITNESS.glob('*.txt'))
    variants = VARIANTS + (['strict32'] if a.strict32 else [])
    bins = {v: build_quiet(v, work / 'build', a.cc) for v in variants}
    if a.strict32 and bins['strict32'] is None: sys.exit('strict32 requested but -m32 is not available')
    runs = {v: run_all(bins[v], scripts, work / 'runs', jobs) for v in variants}
    strict = runs['strict']
    for v in variants:
        for n, r in runs[v].items():
            if r['code'] or SANITIZER.search(r['stderr']):
                failures.append(f'{v} {n}: exit {r["code"]}: {r["stderr"].strip().splitlines()[-1:] if r["stderr"] else ""}')
    ident = {}
    for v in variants[1:]:
        diff = sorted(n for n in strict if runs[v][n]['golden'] != strict[n]['golden'] or runs[v][n]['lmp'] != strict[n]['lmp'])
        ident[v] = dict(identical=len(strict) - len(diff), differ=diff)
        print(f'{v} vs strict: {len(strict) - len(diff)} identical, {len(diff)} differ {diff[:10]}')
        if diff: failures.append(f'{v}: {len(diff)} scripts differ from strict')
    goldens = {n: r['golden'] for n, r in strict.items()}
    demo_bad = []
    for n, r in strict.items(): demo_bad += demo_checks(n, r['golden'], r['lmp'])
    failures += demo_bad
    lmps = sum(1 for r in strict.values() if r['lmp'] is not None)
    print(f'demo consistency: {sum(1 for g in goldens.values() if g.startswith("h "))} recording scripts, {lmps} demo files; '
          f'{len(demo_bad)} failures')
    # the witnesses
    wit = {}
    for s in witnesses:
        rs, ro = run_one(bins['strict'], s, work / 'runs'), run_one(bins['overflow'], s, work / 'runs')
        rep = re.search(r'g_game\.c:(\d+):\d+: runtime error: (signed integer overflow[^\n]*)', ro['stderr'])
        ok = rs['code'] == 0 and ro['code'] != 0 and rep is not None and rep.group(1) in ('579', '580')
        wit[s.stem] = dict(strict_exit=rs['code'], overflow_exit=ro['code'],
                           overflow_report=f'g_game.c:{rep.group(1)}: {rep.group(2)}' if rep else None, as_expected=ok)
        print(f'witness {s.stem}: strict exit {rs["code"]}; overflow exit {ro["code"]}, {wit[s.stem]["overflow_report"]}')
        if not ok and s.stem != 'ev_mouse_int_edge': failures.append(f'witness {s.stem} not as expected')
        if s.stem == 'ev_mouse_int_edge' and (rs['code'] or ro['code']): failures.append('witness ev_mouse_int_edge must pass in both')
    # the chain
    chain_rec = None
    if not a.no_chain:
        if strict[CHAIN]['lmp'] is None: failures.append(f'{CHAIN} wrote no demo')
        else:
            chain_rec, bad = chain(work, a.cc, goldens[CHAIN], strict[CHAIN]['lmp'])
            print(chain_rec['summary'])
            failures += [f'chain: {b}' for b in bad]
    dom = domain(goldens)
    # the scripts whose events the app can post: not the X level, not the withheld keys, not beyond the clamp
    app = {n: g for n, g in goldens.items() if not n.startswith(APP_EXCLUDED)}
    dom_app = domain(app)
    if dom_app['outside_wire24']: failures.append(f"{len(dom_app['outside_wire24'])} app-domain rows outside wire 24")
    if a.summary:
        print(f"domain over {len(goldens)} scripts, {dom['tics']} tics ({dom['built_turns_with_low_byte']} built turns with a nonzero low byte):")
        for k in dom['min']: print(f"  {k}: {dom['min'][k]} .. {dom['max'][k]}")
        print('  distinct buttons (scripts): ' + ', '.join(f'{b} ({n})' for b, n in dom['distinct_buttons'].items()))
        print(f"  rows outside wire 24: {len(dom['outside_wire24'])}")
        for o in dom['outside_wire24']: print(f"    {o['script']} tic {o['tic']} {o['side']}: {', '.join(o['why'])}")
        print(f"app domain ({len(app)} scripts without the prefixes {', '.join(APP_EXCLUDED)}; {dom_app['tics']} tics): "
              + '; '.join(f"{k} {dom_app['min'][k]}..{dom_app['max'][k]}" for k in dom_app['min'])
              + f"; rows outside wire 24: {len(dom_app['outside_wire24'])}")
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    total_tics = dom['tics']
    doc = dict(schema=SCHEMA, contract_sha256=sha(ib.CONTRACT.read_bytes()), variants=VARIANTS,
               asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
               scripts=len(goldens), tics=total_tics, golden_bytes=sum(len(g.encode()) for g in goldens.values()),
               goldens_sha256=sha(''.join(f'{n}\n{g}' for n, g in sorted(goldens.items()))),
               per_script={n: dict(tics=len(t_rows(g)), golden_sha256=sha(g),
                                   end=next((f[2] for _, k, f, _ in expand(g) if k == 'x'), 'z'),
                                   demo_file_sha256=sha(strict[n]['lmp']) if strict[n]['lmp'] is not None else None)
                           for n, g in sorted(goldens.items())},
               witnesses=wit, chain=chain_rec, domain=dom,
               app_domain=dict(excluded_prefixes=list(APP_EXCLUDED), scripts=len(app), **dom_app))
    text = json.dumps(doc, indent=1) + '\n'
    files = {RESULTS: text, **{GOLDENS / f'{n}.golden': g for n, g in goldens.items()}}
    if a.check:
        bad = [str(p.relative_to(ROOT)) for p, t in files.items() if not p.exists() or p.read_text() != t]
        extra = [str(p.relative_to(ROOT)) for p in GOLDENS.glob('*') if p not in files and p.name != '.gitattributes']
        if a.no_chain:
            committed = json.loads(RESULTS.read_text()) if RESULTS.exists() else {}
            doc['chain'] = committed.get('chain')
            files[RESULTS] = json.dumps(doc, indent=1) + '\n'
            bad = [str(p.relative_to(ROOT)) for p, t in files.items() if not p.exists() or p.read_text() != t]
        if bad or extra:
            for p in bad[:5]:
                n = Path(p).stem
                if n in goldens and (ROOT / p).exists():
                    print(f'  {n}: first difference {first_difference((ROOT / p).read_text(), goldens[n])}')
            sys.exit(f'differs: {bad[:20]} ({len(bad)}); not produced by this run: {extra}')
        print(f'{RESULTS.relative_to(ROOT)} and {len(goldens)} goldens match ({total_tics} tics)')
    elif a.no_chain:
        print('--no-chain without --check writes nothing')
    else:
        GOLDENS.mkdir(parents=True, exist_ok=True)
        for p, t in files.items(): p.write_text(t)
        print(f'wrote {RESULTS.relative_to(ROOT)} and {len(goldens)} goldens ({total_tics} tics)')
    if a.strict32:
        print(f"strict32 vs strict: {ident['strict32']['identical']} identical, {len(ident['strict32']['differ'])} differ")
    if a.selftest:
        committed = {p.stem: p.read_text() for p in GOLDENS.glob('*.golden')}
        lines, fails = selftest(work, a.cc, jobs, committed, scripts)
        for l in lines: print(l)
        if fails: sys.exit('selftest FAILED: ' + ', '.join(fails))


if __name__ == '__main__':
    main()
