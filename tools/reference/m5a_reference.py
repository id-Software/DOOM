#!/usr/bin/env python3
"""Genuine-renderer reference runs of the M5a runner on the synthetic rooms and the M4b synthetic scripts.

Expected frames come from the genuine linuxdoom-1.10 renderer only (decisions R1-R16 of
docs/M5A_REFERENCE_INVESTIGATION.md). The M5a runner (tools/reference/m5a_build.py) is built in the
strict, overflow and address variants under --work (and strict32 with --strict32, local only).
Items: every room of tests/reference/m5a_manifest.json (tools/reference/m5a_wad.py) and every
script of the two M4b manifests (tests/reference/m4b_manifest.json, m4b_manifest2.json, read-only),
each run with tests/reference/m5a_gfx.wad loaded after its WAD. Per item:

  never     the strict build with --render none: its game-dump set (SHA-256 of the concatenated
            snapshots, m3h9c_dump order) must equal the committed M4b digest (M4b scripts) or the
            frozen M4b runner's dump set on the same WAD, graphics and demo (rooms; m4b_build strict);
  every     for each view configuration (screenblocks 11, 10 and 9, detail 0; decision R2) every
            variant renders every dumped tic (--render every): the dump sets (L.flags without
            ML_MAPPED, decision R5) must equal the never-render set (up to the last dumped tic when
            the run ends early at a renderer limit), and the frame streams (with the full-frame
            digest of tracker-flagged frames replaced by "undefined", decision R4) and the view
            records must be byte-identical across the variants up to the earliest end; each
            variant's ending (marker, exit, reborn, refused witness, I_Error, sanitizer report) is
            recorded.

Goldens (written without --check, compared with --check): the strict frame stream of every item and
configuration, normalised as above, as tests/reference/m5a_frames/<LUMP>.sb<N>.frames (one F row per
frame with its SHA-256; schema DOOM-M5A-FRAMES-v1, rows in m5a_runner_contract.json), the strict view
record per item as tests/reference/m5a_views/<LUMP>.view (identical for every configuration, checked),
and tests/reference/m5a_results.json (schema DOOM-M5A-RESULTS-v1): per item and configuration the
frame count, the ending, the stream digests, the tracker counts and origins, the counter maxima, the
witness, the I_Error message, every variant's ending, and the room's `must_show` event tics. --pngs
also writes tests/reference/m5a_png/<LUMP>_sb<N>_<tic>.png (palette 0 of the synthetic PLAYPAL) for
the manifest's `png` frames of every room; --check compares their pixels.

  must_show expressions: "frame FIELD OP VALUE" (an F row column), "view FIELD OP VALUE" (a V row
  column), "witness NAME" (an O row), "error TEXT" (an E row containing TEXT); OP is one of
  == != >= <= > <; the value is the first tic at which the expression holds (None: never). A frame or
  view expression may end with "from N": the first tic >= N at which it holds.

--selftest plants a one-pixel change after R_RenderPlayerView of tic 20 of M5VIEW (strict,
screenblocks 10; --selftest-pixel) and checks that the frame comparison reports exactly that tic and,
from the raw frames, that pixel. --update-events writes observed endings and event tics into the
manifest (authoring aid; reviewed, then held fixed by --check). Address runs use
ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so they are not LeakSanitizer coverage.
At most two runs execute at once (--jobs).

    m5a_reference.py [--work DIR] [--check] [--pngs] [--selftest] [--strict32] [--update-events]
                     [--only NAME[,NAME]] [--variants strict,overflow,address] [--jobs N] [--cc CC]
"""
import argparse, concurrent.futures as cf, hashlib, json, os, re, shutil, struct, subprocess, sys, zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5a_build             # noqa: E402
import m4b_build             # noqa: E402
import m3h9c_dump as dump    # noqa: E402
import m5a_gfx               # noqa: E402

GFX = ROOT / 'tests/reference/m5a_gfx.wad'
MANIFEST = ROOT / 'tests/reference/m5a_manifest.json'
M4B_SETS = [(ROOT / 'tests/reference/m4b_manifest.json', ROOT / 'tests/reference/m4b_synthetic_results.json'),
            (ROOT / 'tests/reference/m4b_manifest2.json', ROOT / 'tests/reference/m4b_synthetic_results2.json')]
RESULTS = ROOT / 'tests/reference/m5a_results.json'
FRAMES_DIR = ROOT / 'tests/reference/m5a_frames'
VIEWS_DIR = ROOT / 'tests/reference/m5a_views'
PNG_DIR = ROOT / 'tests/reference/m5a_png'
SCHEMA = 'DOOM-M5A-RESULTS-v1'
VARIANTS = ['strict', 'overflow', 'address']
VIEWS = [11, 10, 9]
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
SUMMARY = re.compile(r'^summary: tics (\d+) end=(marker|exit|reborn|refused) end_tic (\d+) ', re.M)
FCOLS = ['tic', 'sha256', 'masked_sha256', 'fuzzpos', 'mapped', 'columns', 'visplanes', 'drawsegs', 'vissprites', 'openings',
         'solidsegs_peak', 'drawseg_drops', 'vissprite_drop', 'whole']
VCOLS = ['tic', 'viewx', 'viewy', 'viewz', 'viewangle', 'viewheight', 'deltaviewheight', 'bob', 'onground', 'extralight',
         'fixedcolormap', 'invisibility', 'ps0state', 'ps0tics', 'ps0sx', 'ps0sy', 'ps1state', 'ps1tics', 'ps1sx', 'ps1sy',
         'sector', 'lightlevel']
sha = lambda b: hashlib.sha256(b).hexdigest()


def items(only=None):
    """(lump, wad path, lmp path, kind, entry) for every room and M4b script."""
    out = []
    man = json.loads(MANIFEST.read_text())
    if man.get('schema') != 'DOOM-M5A-MANIFEST-v1': sys.exit('m5a manifest schema')
    for r in man['rooms']:
        s = r['script']
        out.append(dict(lump=s['lump'], wad=ROOT / r['file'], lmp=ROOT / s['lmp'], kind='room', room=r, views=s['views']))
    for mpath, rpath in M4B_SETS:
        m4b = json.loads(mpath.read_text()); res = json.loads(rpath.read_text())['scripts']
        for w in m4b['wads']:
            for s in w['scripts']:
                out.append(dict(lump=s['lump'], wad=ROOT / w['file'], lmp=ROOT / s['lmp'], kind='m4b', views=VIEWS,
                                committed=res[s['lump']]['dumps_sha256'], m4b_end=(res[s['lump']]['end'], res[s['lump']]['end_tic'])))
    if only:
        want = {x.upper() for x in only.split(',')}
        out = [i for i in out if i['lump'] in want or (i['kind'] == 'room' and i['room']['name'] in want)]
    return out


def run(binary, item, out, extra=(), frames=None, view=None):
    """One runner process: (returncode, summary match or None, stderr, dump-set digest, snapshot count)."""
    if out.exists(): shutil.rmtree(out)
    out.mkdir(parents=True)
    cmd = [str(binary), '--wad', str(item['wad']), '--wad', str(GFX), '--wad', str(item['lmp']), '--demo', item['lmp'].stem.upper(),
           '--out', str(out), *extra]
    if frames: cmd += ['--frames', str(frames)]
    if view: cmd += ['--view', str(view)]
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV)
    names = dump.snapshot_names(str(out))
    h = hashlib.sha256()
    for n in names: h.update((out / n).read_bytes())
    digests = [sha((out / n).read_bytes()) for n in names]
    shutil.rmtree(out)
    return dict(code=p.returncode, summary=SUMMARY.search(p.stdout), stderr=p.stderr, dumps=h.hexdigest(), snapshots=len(names),
                per_snapshot=digests)


def report_of(stderr):
    """The sanitizer's report: its SUMMARY line (else the first runtime error line), without directories or addresses."""
    lines = [l for l in stderr.splitlines() if l.startswith('SUMMARY:')] or [l for l in stderr.splitlines() if 'runtime error' in l]
    if not lines: return None
    l = re.sub(r'\S*/([\w.]+\.c:\d+)', r'\1', lines[0].strip())
    l = re.sub(r' ?\([^()]*\)', '', l)               # the binary path and build id of ASan's frame
    return re.sub(r'0x[0-9a-f]+', '0x?', l)[:200]


def parse_frames(path):
    rows = {'F': [], 'O': [], 'E': [], 'N': [], 'U': [], 'M': []}
    for l in path.read_text().splitlines():
        if l.startswith('#'): continue
        k = l.split(',', 1)[0]
        rows.setdefault(k, []).append(l)
    return rows


def normalise(path):
    """The frame stream with the full digest of tracker-flagged frames replaced by "undefined"."""
    out = []
    for l in path.read_text().splitlines(keepends=True):
        if l.startswith('F,'):
            f = l.rstrip('\n').split(',')
            if int(f[1 + FCOLS.index('columns')]) > 0: f[2] = 'undefined'
            l = ','.join(f) + '\n'
        out.append(l)
    return ''.join(out)


def truncate_stream(text, last_tic):
    """Rows of tic <= last_tic (header kept); N rows are kept in order when their first use is kept."""
    out = []
    for l in text.splitlines(keepends=True):
        if l.startswith('#') or l.startswith('N,'): out.append(l); continue
        if int(l.split(',')[1]) <= last_tic: out.append(l)
    return ''.join(out)


def ending(r, frames_rows):
    """How a run ended. A limit witness (an O row: the genuine code wrote past a renderer array, decision R3) comes
    first: what follows it in that frame (an I_Error message with a count) depends on the overwritten memory."""
    if frames_rows['O']: return 'refused:' + frames_rows['O'][0].split(',')[2]
    rep = report_of(r['stderr'])
    if rep: return 'sanitizer:' + rep
    if frames_rows['E']: return 'error:' + frames_rows['E'][0].split(',', 2)[2]
    if r['summary']: return r['summary'][2]
    return f'abnormal:{r["code"]}'


def event(expr, frows, vrows):
    w = expr.split()
    ops = {'==': lambda a, b: a == b, '!=': lambda a, b: a != b, '>=': lambda a, b: a >= b, '<=': lambda a, b: a <= b,
           '>': lambda a, b: a > b, '<': lambda a, b: a < b}
    if w[0] in ('frame', 'view'):
        cols, rows = (FCOLS, frows['F']) if w[0] == 'frame' else (VCOLS, vrows)
        i = cols.index(w[1]) + 1
        start = 0
        if len(w) == 6 and w[4] == 'from': start = int(w[5])
        elif len(w) != 4: sys.exit(f'bad must_show expression {expr!r}')
        for l in rows:
            f = l.split(',')
            if int(f[1]) >= start and ops[w[2]](int(f[i]), int(w[3])): return int(f[1])
        return None
    if w[0] == 'witness':
        for l in frows['O']:
            if l.split(',')[2] == w[1]: return int(l.split(',')[1])
        return None
    if w[0] == 'error':
        text = ' '.join(w[1:])
        for l in frows['E']:
            if text in l: return int(l.split(',')[1])
        return None
    sys.exit(f'unknown must_show expression {expr!r}')


def facts(frows):
    F = [l.split(',') for l in frows['F']]
    col = lambda c: [int(f[1 + FCOLS.index(c)]) for f in F]
    out = dict(frames=len(F), flagged_frames=sum(1 for c in col('columns') if c), flagged_columns=sum(col('columns')),
               whole_frames=sum(col('whole')), origins=[l.split(',', 2)[2] for l in frows['N']],
               mapped_lines=max(col('mapped'), default=0))
    for c in ('visplanes', 'drawsegs', 'vissprites', 'openings', 'solidsegs_peak'):
        out[f'max_{c}'] = max(col(c), default=0)
    out['drawseg_drop_frames'] = sum(1 for c in col('drawseg_drops') if c)
    out['vissprite_drop_frames'] = sum(1 for c in col('vissprite_drop') if c)
    out['witness'] = frows['O'][0].split(',')[1:3] if frows['O'] else None
    out['error'] = frows['E'][0].split(',', 2)[1:] if frows['E'] else None
    return out


def png_bytes(px):
    pal = m5a_gfx.playpal()[:768]
    rows = b''.join(b'\0' + b''.join(pal[c * 3:c * 3 + 3] for c in px[y * 320:(y + 1) * 320]) for y in range(200))
    ch = lambda k, d: struct.pack('>I', len(d)) + k + d + struct.pack('>I', zlib.crc32(k + d) & 0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + ch(b'IHDR', struct.pack('>IIBBBBB', 320, 200, 8, 2, 0, 0, 0)) + ch(b'IDAT', zlib.compress(rows, 9)) + ch(b'IEND', b'')


def png_pixels(data):
    """Decompressed scanlines of a PNG written by png_bytes (compared by --check, not the zlib bytes)."""
    pos, idat = 8, b''
    while pos < len(data):
        n = struct.unpack_from('>I', data, pos)[0]; kind = data[pos + 4:pos + 8]
        if kind == b'IDAT': idat += data[pos + 8:pos + 8 + n]
        pos += 12 + n
    return zlib.decompress(idat)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5a-reference')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--pngs', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--strict32', action='store_true')
    ap.add_argument('--update-events', action='store_true')
    ap.add_argument('--only')
    ap.add_argument('--variants', default=','.join(VARIANTS))
    ap.add_argument('--jobs', type=int, default=2)
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    jobs = max(1, min(a.jobs, 2))
    variants = a.variants.split(',') + (['strict32'] if a.strict32 else [])
    assert variants[0] == 'strict'
    full = not a.only and variants[:3] == VARIANTS and not a.strict32
    bins = {}
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try:
            for v in variants: bins[v] = m5a_build.build(v, work / 'build', a.cc)
            frozen = m4b_build.build('strict', work / 'm4bbuild', a.cc)
        finally: sys.stdout = so
    if a.strict32 and bins['strict32'] is None: sys.exit('strict32 requested but -m32 is not available')
    todo = items(a.only)
    man = json.loads(MANIFEST.read_text())
    rooms_by = {r['script']['lump']: r for r in man['rooms']}
    results, failures, goldens, total_frames = {}, [], {}, {sb: 0 for sb in VIEWS}
    runs = work / 'runs'
    pool = cf.ThreadPoolExecutor(jobs)

    def one(item):
        L = item['lump']; out = {}; fails = []
        never = run(bins['strict'], item, runs / f'{L}_never')
        if not never['summary']: fails.append(f'{L}: never-render run failed: {never["stderr"][-300:]}')
        out['dumps_never_sha256'] = never['dumps']
        if item['kind'] == 'm4b':
            out['never_equals_committed_m4b'] = never['dumps'] == item['committed']
            if not out['never_equals_committed_m4b']: fails.append(f'{L}: never-render dumps differ from the committed M4b digest')
        else:
            fr = run(frozen, item, runs / f'{L}_frozen')
            out['never_equals_frozen_m4b_runner'] = fr['dumps'] == never['dumps']
            if not out['never_equals_frozen_m4b_runner']: fails.append(f'{L}: never-render dumps differ from the frozen M4b runner')
        out['views'] = {}
        view_text = None
        for sb in item['views']:
            per = {}
            for v in variants:
                fp, vp = runs / f'{L}_{sb}_{v}.frames', runs / f'{L}_{sb}_{v}.view'
                r = run(bins[v], item, runs / f'{L}_{sb}_{v}', ['--render', 'every', '--screenblocks', str(sb)], fp, vp)
                frows = parse_frames(fp)
                per[v] = dict(r=r, frows=frows, norm=normalise(fp), view=vp.read_text(), end=ending(r, frows))
                fp.unlink(); vp.unlink()
            s = per['strict']
            last = [int(l.split(',')[1]) for l in s['frows']['F']]
            last_tic = min((max([int(l.split(',')[1]) for l in p['frows']['F']], default=-1)) for p in per.values())
            ident = all(truncate_stream(p['norm'], last_tic) == truncate_stream(s['norm'], last_tic) for p in per.values())
            vtrunc = lambda t: ''.join(l for l in t.splitlines(keepends=True) if l.startswith('#') or int(l.split(',')[1]) <= last_tic)
            vident = all(vtrunc(p['view']) == vtrunc(s['view']) for p in per.values())
            nd = min(p['r']['snapshots'] for p in per.values())
            dident = all(p['r']['per_snapshot'][:nd] == never['per_snapshot'][:nd] for p in per.values())
            if not (ident and vident and dident):
                fails.append(f'{L} sb{sb}: variants differ (frames {ident}, view {vident}, dumps vs never-render {dident})')
            ends = {v: p['end'] for v, p in per.items()}
            if s['frows']['O']: end_tic = int(s['frows']['O'][0].split(',')[1])
            elif s['frows']['E']: end_tic = int(s['frows']['E'][0].split(',')[1])
            elif s['r']['summary']: end_tic = int(s['r']['summary'][3])
            else: end_tic = s['r']['snapshots'] - 2
            golden = truncate_stream(s['norm'], last_tic)
            gtmp = runs / f'{L}_{sb}.golden'; gtmp.write_text(golden); grows = parse_frames(gtmp); gtmp.unlink()
            rec = dict(facts(grows), end=s['end'], end_tic=end_tic,
                       frames_sha256=sha(golden.encode()), view_sha256=sha(s['view'].encode()), variant_endings=ends,
                       compared_through_tic=last_tic, variants_identical=ident and vident and dident,
                       dumps_equal_never_through_snapshot=nd)
            if item['kind'] == 'room':
                rec['must_show'] = {e['what']: event(e['expr'], s['frows'], [l for l in s['view'].splitlines() if l.startswith('V,')])
                                    for e in item['room']['script'].get('must_show', [])}
            out['views'][str(sb)] = rec
            goldens[(L, sb)] = golden
            short_, long_ = sorted([view_text or '', s['view']], key=len)
            if not long_.startswith(short_): fails.append(f'{L}: view record differs between configurations')
            view_text = long_
        goldens[(L, 'view')] = view_text
        return L, item, out, fails

    for L, item, out, fails in pool.map(one, todo):
        failures += fails
        results[L] = dict(kind=item['kind'], wad=item['wad'].relative_to(ROOT).as_posix(), lmp_sha256=sha(item['lmp'].read_bytes()), **out)
        for sb, rec in out['views'].items(): total_frames[int(sb)] += rec['frames']
        if item['kind'] == 'room':
            s = item['room']['script']
            for sb, rec in out['views'].items():
                for e in s.get('must_show', []):
                    want = e.get('tic', {}).get(sb, 'missing') if isinstance(e.get('tic'), dict) else 'missing'
                    if rec['must_show'][e['what']] != want and not a.update_events:
                        failures.append(f"{L} sb{sb}: event '{e['what']}' at {rec['must_show'][e['what']]}, expected {want}")
            if a.update_events:
                r = rooms_by[L]
                r['script']['c_end'] = {sb: [rec['end'], rec['end_tic']] for sb, rec in out['views'].items()}
                for e in r['script'].get('must_show', []):
                    e['tic'] = {sb: rec['must_show'][e['what']] for sb, rec in out['views'].items()}
            else:
                want = s.get('c_end', {})
                for sb, rec in out['views'].items():
                    if want.get(sb) != [rec['end'], rec['end_tic']]:
                        failures.append(f"{L} sb{sb}: ending {rec['end']} at {rec['end_tic']}, expected {want.get(sb)}")
        v10 = out['views'].get('10', next(iter(out['views'].values())))
        print(f"{L}: never-render dumps {'= committed M4b' if item['kind'] == 'm4b' else '= frozen M4b runner'}: "
              f"{out.get('never_equals_committed_m4b', out.get('never_equals_frozen_m4b_runner'))}; "
              + '; '.join(f"sb{sb} frames {rec['frames']} end {rec['end']} flagged {rec['flagged_frames']}/{rec['flagged_columns']} "
                          f"identical {rec['variants_identical']}" for sb, rec in out['views'].items()), flush=True)
    pool.shutdown()
    if a.update_events:
        MANIFEST.write_text(json.dumps(man, indent=1) + '\n'); print(f'updated {MANIFEST.relative_to(ROOT)}')
    print('frames per configuration: ' + ', '.join(f'sb{sb} {n}' for sb, n in total_frames.items()))
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    if full:
        doc = dict(schema=SCHEMA, runner_contract_sha256=sha(m5a_build.CONTRACT.read_bytes()), gfx_sha256=sha(GFX.read_bytes()),
                   manifest_sha256=sha(MANIFEST.read_bytes()), variants=VARIANTS, views=VIEWS,
                   asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
                   frames_per_configuration={str(k): v for k, v in total_frames.items()}, items=results)
        text = json.dumps(doc, indent=1) + '\n'
        files = {RESULTS: text}
        for (L, sb), t in goldens.items():
            files[(VIEWS_DIR / f'{L}.view') if sb == 'view' else (FRAMES_DIR / f'{L}.sb{sb}.frames')] = t
        if a.check:
            bad = [str(p.relative_to(ROOT)) for p, t in files.items() if not p.exists() or p.read_text() != t]
            extra = [str(p.relative_to(ROOT)) for d in (FRAMES_DIR, VIEWS_DIR) if d.exists() for p in d.iterdir() if p not in files]
            if bad or extra: sys.exit(f'differs: {bad}; not produced by this run: {extra}')
            print(f'{RESULTS.relative_to(ROOT)} and {len(files) - 1} golden files match ({len(results)} items)')
        else:
            FRAMES_DIR.mkdir(exist_ok=True); VIEWS_DIR.mkdir(exist_ok=True)
            for p, t in files.items(): p.write_text(t)
            print(f'wrote {RESULTS.relative_to(ROOT)} and {len(files) - 1} golden files')
    if a.pngs:
        pngs = 0
        for item in todo:
            if item['kind'] != 'room': continue
            for spec in item['room'].get('png', []):
                sb, tic = spec
                raw = work / 'raw'; shutil.rmtree(raw, ignore_errors=True); raw.mkdir(parents=True)
                fp = work / 'png.frames'
                run(bins['strict'], item, runs / 'png', ['--render', 'every', '--screenblocks', str(sb), '--frames-raw', str(raw)], fp)
                px = bytearray((raw / f'frame_{tic:05d}.raw').read_bytes())
                cols = [int(l.split(',')[2]) for l in fp.read_text().splitlines() if l.startswith(f'U,{tic},')]
                for x in cols:
                    for y in range(200): px[y * 320 + x] = 0          # tracker-flagged columns are not defined (R4)
                data = png_bytes(bytes(px))
                path = PNG_DIR / f"{item['lump']}_sb{sb}_{tic}.png"
                if a.check:
                    if not path.exists() or png_pixels(path.read_bytes()) != png_pixels(data): sys.exit(f'{path.relative_to(ROOT)} differs')
                else:
                    PNG_DIR.mkdir(exist_ok=True); path.write_bytes(data)
                pngs += 1
                shutil.rmtree(raw); fp.unlink()
        print(f"{'checked' if a.check else 'wrote'} {pngs} PNGs")
    if a.selftest:
        item = next(i for i in items('M5VIEW'))
        t, x, y = 20, 123, 77
        res = {}
        for name, extra in (('plain', []), ('planted', ['--selftest-pixel', str(t), str(x), str(y)])):
            raw = work / f'self_{name}'; shutil.rmtree(raw, ignore_errors=True); raw.mkdir(parents=True)
            fp = work / f'self_{name}.frames'
            run(bins['strict'], item, runs / 'self', ['--render', 'every', '--screenblocks', '10', '--frames-raw', str(raw)] + extra, fp)
            res[name] = (fp.read_text(), raw)
        fa = [l for l in res['plain'][0].splitlines() if l.startswith('F,')]
        fb = [l for l in res['planted'][0].splitlines() if l.startswith('F,')]
        first = next((int(p.split(',')[1]) for p, q in zip(fa, fb) if p != q), None)
        pixel = None
        if first is not None:
            pa = (res['plain'][1] / f'frame_{first:05d}.raw').read_bytes(); pb = (res['planted'][1] / f'frame_{first:05d}.raw').read_bytes()
            k = next(i for i in range(len(pa)) if pa[i] != pb[i]); pixel = (k % 320, k // 320)
        ok = first == t and pixel == (x, y)
        print(f'selftest: planted pixel ({x}, {y}) at tic {t}; first differing frame {first}, first differing pixel {pixel}: '
              + ('pass' if ok else 'FAIL'))
        for name in res: shutil.rmtree(res[name][1]); (work / f'self_{name}.frames').unlink()
        if not ok: sys.exit(1)


if __name__ == '__main__':
    main()
