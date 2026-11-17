#!/usr/bin/env python3
"""Genuine status bar, HUD and palette reference runs of the M5b runner on every synthetic item.

Expected frames, palette numbers, M_Random indices and messages come from the genuine linuxdoom-1.10
code only (decisions S1, S14-S17, S22 and S23 of docs/M5B_REFERENCE_INVESTIGATION.md). Frames are
identical to the genuine engine under the declared sound-free, wipe-free contract (decision S15). The
M5b runner (tools/reference/m5b_build.py) is built in the strict, overflow and address variants under
--work, with the frozen M4b runner for the dump comparison of the new rooms.

Items: the rooms of tests/reference/m5a_manifest.json, the scripts of the two M4b manifests (both read-only)
and the rooms of tests/reference/m5b_manifest.json (tools/reference/m5b_wad.py), each run with
tests/reference/m5b_gfx.wad after its WAD; the m5b room CHEATS is the bespoke scenario of decision S1
(m5b_wad.BESPOKE: --bespoke-keys, key events through the genuine G_Responder). Per item:

  never     the strict build with --render none: its game-dump set (SHA-256 of the concatenated snapshots,
            m3h9c_dump order; the H row's rndindex and the P row's message under the S16 record and mask)
            must equal the committed M4b digest (M4b scripts), the committed M5a never-render digest (M5a
            rooms) or the frozen M4b runner's dump set on the same WAD, graphics and demo (new rooms); the
            bespoke scenario's is recorded only;
  every     at screenblocks 10 and 11 (decision S14) every variant displays every dumped tic (--render every):
            the dump sets must equal the never-render set snapshot for snapshot, and the frame streams
            (normalised as below, with the diagnostic D, W and Z rows) must be byte-identical across the
            variants up to the earliest end; a second strict run must give the same stream (determinism);
  m5a       for the M5a rooms and M4b scripts (the M5a items), every frame with no message showing and no tracker flag must show the M5a
            golden view: at screenblocks 11 the whole-frame digest, at screenblocks 10 the digest with the
            rows below the view zeroed, against tests/reference/m5a_frames (a standing assertion).

Normalisation (as M5a): a frame with tracker-flagged columns has its full digest replaced by "undefined"
(its masked digest stays); a frame with a 2D undefined read, a post outside its lump or a write below the
screen (D-row counts, decision S17) is refused as a whole: both digests become "refused2d" (none occurs).

Goldens (written without --check, compared with --check): tests/reference/m5b_frames/<LUMP>.sb<N>.frames
(schema DOOM-M5B-FRAMES-v1: per frame the F row, the digest of the whole 320x200 frame, and the P row:
palette, palette set during the frame, face, message on, genuine rndindex; the X rows of the messages taken;
M, U, N, O and E rows as M5a), tests/reference/m5b_results.json (schema DOOM-M5B-RESULTS-v1) and, with --pngs,
tests/reference/m5b_png/<LUMP>_sb<N>_<tic>.png for the m5b manifest's png frames, drawn through the frame's own
palette of the synthetic PLAYPAL (--check compares their pixels).

  must_show expressions: as m5a_reference.py ("frame FIELD OP VALUE", "witness NAME", "error TEXT") plus
  "status FIELD OP VALUE [from N]" (a P row column) and "message CODE" (an X row's code).

--selftest plants faults and checks that the comparisons find each at the planted place: a status bar pixel
(--selftest-bar, M5VIEW, screenblocks 10), a palette number (--selftest-palette), an M_Random index
(--selftest-rnd), a message tic (--selftest-message, M5MSG), and the two unmasked dump fields
(--no-mask-rnd, --no-hu-record: the never-render dump set must then differ from the frozen digest).
--update-events writes observed endings and event tics into the m5b manifest (authoring aid; reviewed,
then held fixed by --check). Address runs use ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so
they are not LeakSanitizer coverage.

    m5b_reference.py [--work DIR] [--check] [--pngs] [--selftest] [--update-events] [--only NAME[,NAME]]
                     [--jobs N] [--cc CC]
"""
import argparse, concurrent.futures as cf, hashlib, json, os, re, shutil, struct, subprocess, sys, zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5b_build             # noqa: E402
import m4b_build             # noqa: E402
import m3h9c_dump as dump    # noqa: E402
import m5a_gfx               # noqa: E402
import m5a_reference as A    # noqa: E402  (read-only: stream helpers)
import m5b_wad               # noqa: E402

GFX = ROOT / 'tests/reference/m5b_gfx.wad'
M5A_MANIFEST = ROOT / 'tests/reference/m5a_manifest.json'
M5A_RESULTS = ROOT / 'tests/reference/m5a_results.json'
M5A_FRAMES = ROOT / 'tests/reference/m5a_frames'
MANIFEST = ROOT / 'tests/reference/m5b_manifest.json'
RESULTS = ROOT / 'tests/reference/m5b_results.json'
FRAMES_DIR = ROOT / 'tests/reference/m5b_frames'
PNG_DIR = ROOT / 'tests/reference/m5b_png'
SCHEMA = 'DOOM-M5B-RESULTS-v1'
VARIANTS = ['strict', 'overflow', 'address']
VIEWS = [10, 11]
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
FCOLS = A.FCOLS
PCOLS = ['tic', 'palette', 'palette_set', 'face', 'message_on', 'rndindex']
DCOLS = ['tic', 'st_draws', 'other_draws', 'bar_undef_px', 'skipped_patches', 'beyond_rows', 'lump_overrun', 'view_sha256']
GOLDEN_KINDS = ('F', 'P', 'X', 'M', 'U', 'N', 'O', 'E')
STATUS_FILES = ('st_stuff.c', 'st_lib.c', 'hu_stuff.c', 'hu_lib.c', 'v_video.c')
sha = lambda b: hashlib.sha256(b).hexdigest()


def items(only=None):
    """Every M5a room, M4b script and M5b room: dict(lump, wad, lmp, kind, ...)."""
    out = []
    m5a_res = json.loads(M5A_RESULTS.read_text())['items']
    for r in json.loads(M5A_MANIFEST.read_text())['rooms']:
        s = r['script']
        out.append(dict(lump=s['lump'], name=r['name'], wad=ROOT / r['file'], lmp=ROOT / s['lmp'], kind='m5a',
                        committed=m5a_res[s['lump']]['dumps_never_sha256']))
    for mpath, rpath in A.M4B_SETS:
        res = json.loads(rpath.read_text())['scripts']
        for w in json.loads(mpath.read_text())['wads']:
            for s in w['scripts']:
                out.append(dict(lump=s['lump'], name=s['lump'], wad=ROOT / w['file'], lmp=ROOT / s['lmp'], kind='m4b',
                                committed=res[s['lump']]['dumps_sha256']))
    man = json.loads(MANIFEST.read_text())
    if man.get('schema') != m5b_wad.MANIFEST_SCHEMA: sys.exit('m5b manifest schema')
    for r in man['rooms']:
        s = r['script']
        name = r['name'].lower()
        out.append(dict(lump=s['lump'], name=r['name'], wad=ROOT / r['file'], lmp=ROOT / s['lmp'],
                        kind='bespoke' if name in m5b_wad.BESPOKE else 'm5b', room=r,
                        keys=m5b_wad.BESPOKE.get(name, [])))
    if only:
        want = {x.upper() for x in only.split(',')}
        out = [i for i in out if i['lump'] in want or i['name'] in want]
    return out


def bespoke_args(item):
    return [a for tic, text in item.get('keys', []) for a in ('--bespoke-keys', str(tic), text)]


def run(binary, item, out, extra=(), frames=None, raw=None, nodump=False):
    """One runner process: dict(code, summary, stderr, dumps, snapshots, per_snapshot)."""
    if out.exists(): shutil.rmtree(out)
    out.mkdir(parents=True)
    cmd = [str(binary), '--wad', str(item['wad']), '--wad', str(GFX), '--wad', str(item['lmp']), '--demo', item['lmp'].stem.upper(),
           *(['--no-dump'] if nodump else ['--out', str(out)]), *bespoke_args(item), *extra]
    if frames: cmd += ['--frames', str(frames)]
    if raw: cmd += ['--frames-raw', str(raw)]
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV)
    names = [] if nodump else dump.snapshot_names(str(out))
    h = hashlib.sha256()
    digests = []
    for n in names:
        b = (out / n).read_bytes(); h.update(b); digests.append(sha(b))
    shutil.rmtree(out)
    return dict(code=p.returncode, summary=A.SUMMARY.search(p.stdout), stderr=p.stderr, dumps=h.hexdigest(), snapshots=len(names),
                per_snapshot=digests)


def frows_of(text):
    rows = {k: [] for k in ('F', 'P', 'X', 'D', 'W', 'Z', 'M', 'U', 'N', 'O', 'E')}
    for l in text.splitlines():
        if l.startswith('#'): continue
        rows.setdefault(l.split(',', 1)[0], []).append(l)
    return rows


def normalise(text):
    """(golden stream, full stream for the variant comparison). Tracker-flagged frames lose their full digest
    ("undefined"), frames with a 2D undefined read are refused as a whole ("refused2d"); D rows lose the
    view digest of flagged frames."""
    bad2d = set()
    for l in text.splitlines():
        if l.startswith('D,'):
            f = l.split(',')
            if any(int(f[1 + DCOLS.index(c)]) for c in ('bar_undef_px', 'beyond_rows', 'lump_overrun')): bad2d.add(f[1])
    flagged = set()
    golden, full = [], []
    for l in text.splitlines(keepends=True):
        k = l.split(',', 1)[0]
        if k == 'F':
            f = l.rstrip('\n').split(',')
            if f[1] in bad2d: f[2] = f[3] = 'refused2d'
            elif int(f[1 + FCOLS.index('columns')]) > 0: f[2] = 'undefined'; flagged.add(f[1])
            l = ','.join(f) + '\n'
        if k == 'D':
            f = l.rstrip('\n').split(',')
            if f[1] in flagged or f[1] in bad2d: f[-1] = 'undefined'
            l = ','.join(f) + '\n'
        if l.startswith('#') or k in GOLDEN_KINDS: golden.append(l)
        full.append(l)
    return ''.join(golden), ''.join(full)


def truncate(text, last_tic, keep_end):
    out = []
    for l in text.splitlines(keepends=True):
        k = l.split(',', 1)[0]
        if l.startswith('#') or k == 'N': out.append(l); continue
        if k in ('W', 'Z'):
            if keep_end: out.append(l)
            continue
        if int(l.split(',')[1]) <= last_tic: out.append(l)
    return ''.join(out)


def ending(r, rows):
    if rows['O']: return 'refused:' + rows['O'][0].split(',')[2]
    rep = A.report_of(r['stderr'])
    if rep: return 'sanitizer:' + rep
    if rows['E']: return 'error:' + rows['E'][0].split(',', 2)[2]
    if r['summary']: return r['summary'][2]
    return f'abnormal:{r["code"]}'


def event(expr, rows):
    w = expr.split()
    ops = {'==': lambda a, b: a == b, '!=': lambda a, b: a != b, '>=': lambda a, b: a >= b, '<=': lambda a, b: a <= b,
           '>': lambda a, b: a > b, '<': lambda a, b: a < b}
    if w[0] == 'status':
        start = int(w[5]) if len(w) == 6 and w[4] == 'from' else 0
        if len(w) not in (4, 6): sys.exit(f'bad must_show expression {expr!r}')
        i = PCOLS.index(w[1])
        for l in rows['P']:
            f = l.split(',')
            if int(f[1]) >= start and ops[w[2]](int(f[i + 1]), int(w[3])): return int(f[1])
        return None
    if w[0] == 'message':
        for l in rows['X']:
            f = l.split(',', 3)
            if int(f[2]) == int(w[1]): return int(f[1])
        return None
    if w[0] == 'frame':
        return A.event(expr, {'F': rows['F'], 'O': rows['O'], 'E': rows['E']}, [])
    return A.event(expr, {'F': rows['F'], 'O': rows['O'], 'E': rows['E']}, [])


def facts(rows, full_rows):
    F = [l.split(',') for l in rows['F']]
    P = [l.split(',') for l in rows['P']]
    D = [l.split(',') for l in full_rows['D']]
    col = lambda c: [int(f[1 + FCOLS.index(c)]) for f in F]
    pal = {}
    for f in P: pal[f[2]] = pal.get(f[2], 0) + 1
    msgs = [l.split(',', 3) for l in rows['X']]
    widgets = [l.split(',')[1:] for l in full_rows['W']]
    out = dict(frames=len(F), flagged_frames=sum(1 for c in col('columns') if c), flagged_columns=sum(col('columns')),
               whole_frames=sum(col('whole')), origins=[l.split(',', 2)[2] for l in rows['N']],
               palettes={k: pal[k] for k in sorted(pal, key=int)}, palette_sets=sum(1 for f in P if f[3] != '-1'),
               faces=sorted({int(f[4]) for f in P}), message_frames=sum(1 for f in P if f[5] != '0'),
               messages=[[int(m[1]), int(m[2]), m[3]] for m in msgs],
               rndindex_last=int(P[-1][6]) if P else None,
               st_draws=int(D[-1][2]) if D else 0, other_draws=max((int(d[3]) for d in D), default=0),
               undefined_2d=dict(bar_undef_px=sum(int(d[4]) for d in D), skipped_patches=sum(int(d[5]) for d in D),
                                 beyond_rows=sum(int(d[6]) for d in D), lump_overrun=sum(int(d[7]) for d in D)),
               widgets={f'{w[0]}@{w[1]},{w[2]}': dict(calls=int(w[3]), first=int(w[4]), min=int(w[5]), max=int(w[6]), changes=int(w[7]))
                        for w in widgets})
    out['witness'] = rows['O'][0].split(',')[1:3] if rows['O'] else None
    out['error'] = rows['E'][0].split(',', 2)[1:] if rows['E'] else None
    z = full_rows['Z'][0].split(',') if full_rows['Z'] else []
    out['palette_unknown'] = int(z[z.index('palette_unknown') + 1]) if 'palette_unknown' in z else None
    return out


def m5a_consistency(item, sb, rows, full_rows):
    """Proof (e): frames with no message showing and no tracker flag show the M5a golden view."""
    path = M5A_FRAMES / f"{item['lump']}.sb{sb}.frames"
    gold = {l.split(',')[1]: l.split(',')[2] for l in path.read_text().splitlines() if l.startswith('F,')}
    gold_flag = {l.split(',')[1] for l in path.read_text().splitlines() if l.startswith('F,') and int(l.split(',')[1 + FCOLS.index('columns')])}
    msg = {l.split(',')[1] for l in rows['P'] if l.split(',')[5] != '0'}
    view = {l.split(',')[1]: l.split(',')[-1] for l in full_rows['D']}
    eq = differ = skipped = 0
    first = None
    for l in rows['F']:
        f = l.split(',')
        t = f[1]
        if t not in gold or t in msg or t in gold_flag or f[2] in ('undefined', 'refused2d'): skipped += 1; continue
        mine = f[2] if sb == 11 else view.get(t)
        if mine == gold[t]: eq += 1
        else:
            differ += 1
            first = first if first is not None else int(t)
    return dict(equal=eq, differ=differ, skipped=skipped, first_differing_tic=first)


def palette_png(px, palette):
    pal = m5a_gfx.playpal()[palette * 768:(palette + 1) * 768]
    rows = b''.join(b'\0' + b''.join(pal[c * 3:c * 3 + 3] for c in px[y * 320:(y + 1) * 320]) for y in range(200))
    ch = lambda k, d: struct.pack('>I', len(d)) + k + d + struct.pack('>I', zlib.crc32(k + d) & 0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + ch(b'IHDR', struct.pack('>IIBBBBB', 320, 200, 8, 2, 0, 0, 0)) + ch(b'IDAT', zlib.compress(rows, 9)) + ch(b'IEND', b'')


def build_all(work, cc):
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try:
            bins = {v: m5b_build.build(v, work / 'build', cc) for v in VARIANTS}
            frozen = m4b_build.build('strict', work / 'm4bbuild', cc)
        finally: sys.stdout = so
    return bins, frozen


def run_item(item, bins, frozen, runs):
    """Every run and comparison of one item: (record, goldens {sb: text}, failures, coverage)."""
    L = item['lump']; out = {}; fails = []; goldens = {}
    never = run(bins['strict'], item, runs / f'{L}_never')
    if not never['summary']: fails.append(f'{L}: never-render run failed: {never["stderr"][-300:]}')
    out['dumps_never_sha256'] = never['dumps']
    if item['kind'] in ('m4b', 'm5a'):
        out['never_equals_committed'] = never['dumps'] == item['committed']
        if not out['never_equals_committed']: fails.append(f'{L}: masked never-render dumps differ from the committed digest')
    elif item['kind'] == 'm5b':
        fr = run(frozen, item, runs / f'{L}_frozen')
        out['never_equals_frozen_m4b_runner'] = fr['dumps'] == never['dumps']
        if not out['never_equals_frozen_m4b_runner']: fails.append(f'{L}: masked never-render dumps differ from the frozen M4b runner')
    out['views'] = {}
    for sb in VIEWS:
        per = {}
        for v in VARIANTS + ['again']:
            fp = runs / f'{L}_{sb}_{v}.frames'
            r = run(bins['strict' if v == 'again' else v], item, runs / f'{L}_{sb}_{v}', ['--render', 'every', '--screenblocks', str(sb)],
                    fp, nodump=v == 'again')
            text = fp.read_text(); fp.unlink()
            g, full = normalise(text)
            per[v] = dict(r=r, rows=frows_of(g), full_rows=frows_of(full), golden=g, full=full, end=ending(r, frows_of(text)))
        s = per['strict']
        tics = {v: max([int(l.split(',')[1]) for l in p['rows']['F']], default=-1) for v, p in per.items()}
        last_tic = min(tics.values())
        same_end = len({p['end'] for p in per.values()}) == 1 and len(set(tics.values())) == 1
        ident = all(truncate(p['full'], last_tic, same_end) == truncate(s['full'], last_tic, same_end) for v, p in per.items() if v != 'again')
        determ = truncate(per['again']['full'], last_tic, same_end) == truncate(s['full'], last_tic, same_end)
        nd = min(p['r']['snapshots'] for v, p in per.items() if v != 'again')
        dident = all(p['r']['per_snapshot'][:nd] == never['per_snapshot'][:nd] for v, p in per.items() if v != 'again')
        if not (ident and dident and determ):
            fails.append(f'{L} sb{sb}: variants identical {ident}, dumps equal the never-render set {dident}, two strict runs identical {determ}')
        if s['rows']['O']: end_tic = int(s['rows']['O'][0].split(',')[1])
        elif s['rows']['E']: end_tic = int(s['rows']['E'][0].split(',')[1])
        elif s['r']['summary']: end_tic = int(s['r']['summary'][3])
        else: end_tic = s['r']['snapshots'] - 2
        golden = truncate(s['golden'], last_tic, True)
        grows = frows_of(golden)
        rec = dict(facts(grows, s['full_rows']), end=s['end'], end_tic=end_tic, frames_sha256=sha(golden.encode()),
                   variant_endings={v: p['end'] for v, p in per.items() if v != 'again'}, compared_through_tic=last_tic,
                   variants_identical=ident, strict_runs_identical=determ, dumps_equal_never_through_snapshot=nd)
        rec['sanitizer_reports'] = {v: A.report_of(p['r']['stderr']) for v, p in per.items() if v in ('overflow', 'address') and A.report_of(p['r']['stderr'])}
        rec['status_sanitizer_reports'] = sum(1 for x in rec['sanitizer_reports'].values() if any(f in x for f in STATUS_FILES))
        if item['kind'] in ('m5a', 'm4b'): rec['m5a_view_consistency'] = m5a_consistency(item, sb, grows, s['full_rows'])
        if item.get('room'):
            rec['must_show'] = {e['what']: event(e['expr'], grows) for e in item['room']['script'].get('must_show', [])}
        out['views'][str(sb)] = rec
        goldens[sb] = golden
    return out, goldens, fails


def check_events(item, out, update, failures):
    s = item['room']['script']
    for sb, rec in out['views'].items():
        for e in s.get('must_show', []):
            want = e.get('tic', {}).get(sb, 'missing') if isinstance(e.get('tic'), dict) else 'missing'
            if rec['must_show'][e['what']] != want and not update:
                failures.append(f"{item['lump']} sb{sb}: event '{e['what']}' at {rec['must_show'][e['what']]}, expected {want}")
            if rec['must_show'][e['what']] is None and update:
                failures.append(f"{item['lump']} sb{sb}: event '{e['what']}' never happens")
    if update:
        s['c_end'] = {sb: [rec['end'], rec['end_tic']] for sb, rec in out['views'].items()}
        for e in s.get('must_show', []):
            e['tic'] = {sb: rec['must_show'][e['what']] for sb, rec in out['views'].items()}
    else:
        for sb, rec in out['views'].items():
            if s.get('c_end', {}).get(sb) != [rec['end'], rec['end_tic']]:
                failures.append(f"{item['lump']} sb{sb}: ending {rec['end']} at {rec['end_tic']}, expected {s.get('c_end', {}).get(sb)}")


def coverage(results):
    """Palettes, faces, message kinds and widget values reached over the items reachable under wire 24 (not the
    bespoke scenario), and separately with it."""
    def gather(kinds):
        pal, faces, msgs, wid = set(), set(), set(), {}
        for r in results.values():
            if r['kind'] not in kinds: continue
            for sb, v in r['views'].items():
                pal |= {int(k) for k in v['palettes']}; faces |= set(v['faces']); msgs |= {m[1] for m in v['messages']}
                if sb == '10':
                    for k, w in v['widgets'].items():
                        o = wid.setdefault(k, dict(min=w['min'], max=w['max'], changes=0))
                        o['min'] = min(o['min'], w['min']); o['max'] = max(o['max'], w['max']); o['changes'] += w['changes']
        return dict(palettes=sorted(pal), faces=sorted(faces), message_codes=sorted(msgs),
                    widgets={k: wid[k] for k in sorted(wid)})
    wire = gather(('m4b', 'm5a', 'm5b'))
    allk = gather(('m4b', 'm5a', 'm5b', 'bespoke'))
    return dict(wire24=dict(wire, palettes_count=len(wire['palettes']), faces_count=len(wire['faces']),
                            faces_missing=[f for f in range(42) if f not in wire['faces']],
                            palettes_missing=[p for p in range(14) if p not in wire['palettes']]),
                with_bespoke=dict(palettes=allk['palettes'], faces=allk['faces'], palettes_count=len(allk['palettes']),
                                  faces_count=len(allk['faces']),
                                  faces_missing=[f for f in range(42) if f not in allk['faces']],
                                  palettes_missing=[p for p in range(14) if p not in allk['palettes']]))


def selftest(bins, frozen, work):
    """Plant faults; each comparison must report the planted place. Returns the list of outcomes."""
    runs = work / 'self'
    res = []

    def stream(item, extra, raw=None):
        fp = work / 'self.frames'
        run(bins['strict'], item, runs, ['--render', 'every', '--screenblocks', '10'] + extra, fp, raw=raw, nodump=True)
        t = fp.read_text(); fp.unlink(); return t

    def first_diff(a, b, kind):
        ra = [l for l in a.splitlines() if l.startswith(kind + ',')]
        rb = [l for l in b.splitlines() if l.startswith(kind + ',')]
        return next((int(p.split(',')[1]) for p, q in zip(ra, rb) if p != q), None if len(ra) == len(rb) else -1)

    view = next(iter(items('M5VIEW'))); msgi = next(iter(items('M5MSG')))
    gold_view = (FRAMES_DIR / 'M5VIEW.sb10.frames').read_text()
    gold_msg = (FRAMES_DIR / 'M5MSG.sb10.frames').read_text()
    # 1. a status bar pixel
    t, x, y = 20, 200, 185
    raws = {}
    for name, extra in (('plain', []), ('planted', ['--selftest-bar', str(t), str(x), str(y)])):
        raw = work / f'self_raw_{name}'; shutil.rmtree(raw, ignore_errors=True); raw.mkdir(parents=True)
        txt = stream(view, extra, raw)
        raws[name] = (normalise(txt)[0], raw)
    first = first_diff(gold_view, raws['planted'][0], 'F')
    pa = (raws['plain'][1] / f'frame_{t:05d}.raw').read_bytes(); pb = (raws['planted'][1] / f'frame_{t:05d}.raw').read_bytes()
    pix = [(k % 320, k // 320) for k in range(len(pa)) if pa[k] != pb[k]]
    ok = first == t and pix == [(x, y)] and raws['plain'][0] == gold_view
    res.append(('status bar pixel', f'planted ({x}, {y}) at tic {t}; first differing F row against the golden {first}; differing pixels {pix}', ok))
    for name in raws: shutil.rmtree(raws[name][1])
    # 2. a palette number, 3. an M_Random index
    for what, flag, tic in (('palette number', '--selftest-palette', 25), ('M_Random index', '--selftest-rnd', 30)):
        g = normalise(stream(view, [flag, str(tic)]))[0]
        first = first_diff(gold_view, g, 'P')
        n = sum(1 for p, q in zip(gold_view.splitlines(), g.splitlines()) if p != q)
        res.append((what, f'planted at tic {tic}; first differing P row {first}; differing rows {n}', first == tic and n == 1))
    # 4. a message tic
    taken = [int(l.split(',')[1]) for l in gold_msg.splitlines() if l.startswith('X,')]
    tic = taken[3]
    g = normalise(stream(msgi, ['--selftest-message', str(tic)]))[0]
    xa = [l for l in gold_msg.splitlines() if l.startswith('X,')]; xb = [l for l in g.splitlines() if l.startswith('X,')]
    bad = [(p, q) for p, q in zip(xa, xb) if p != q]
    res.append(('message tic', f'message taken at tic {tic} recorded one tic late; differing X rows {bad}',
                len(bad) == 1 and bad[0][0].split(',')[1] == str(tic) and bad[0][1].split(',')[1] == str(tic + 1)))
    # 5. the unmasked dump fields
    for what, flag, item in (('unmasked rndindex', '--no-mask-rnd', view), ('unmasked message', '--no-hu-record', msgi)):
        ref = item['committed'] if 'committed' in item else run(frozen, item, runs)['dumps']
        plain = run(bins['strict'], item, runs, [])
        planted = run(bins['strict'], item, runs, [flag])
        names = ['load'] + [str(i) for i in range(planted['snapshots'] - 1)]
        first = next((names[i] for i, (p, q) in enumerate(zip(plain['per_snapshot'], planted['per_snapshot'])) if p != q), None)
        ok = plain['dumps'] == ref and planted['dumps'] != ref
        if what == 'unmasked message': ok = ok and first == str(next(int(l.split(',')[1]) for l in gold_msg.splitlines() if l.startswith('X,')))
        else: ok = ok and first == '0'
        res.append((what, f'{flag}: dump set equals the frozen digest {planted["dumps"] == ref} '
                          f'(masked: {plain["dumps"] == ref}); first differing snapshot {first}', ok))
    return res


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5b-reference')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--pngs', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--update-events', action='store_true')
    ap.add_argument('--only')
    ap.add_argument('--jobs', type=int, default=2)
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    bins, frozen = build_all(work, a.cc)
    failures, results, goldens, total = [], {}, {}, {sb: 0 for sb in VIEWS}
    todo = items(a.only) if (a.check or a.update_events or a.pngs or not a.selftest or a.only) else []
    man = json.loads(MANIFEST.read_text())
    rooms_by = {r['script']['lump']: r for r in man['rooms']}
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        futs = [pool.submit(run_item, i, bins, frozen, work / 'runs' / i['lump']) for i in todo]
        for item, fut in zip(todo, futs):
            out, gold, fails = fut.result()
            failures += fails
            L = item['lump']
            results[L] = dict(kind=item['kind'], wad=item['wad'].relative_to(ROOT).as_posix(), lmp_sha256=sha(item['lmp'].read_bytes()), **out)
            if item.get('keys'): results[L]['bespoke_keys'] = [[t, k] for t, k in item['keys']]
            for sb, g in gold.items(): goldens[(L, sb)] = g; total[sb] += out['views'][str(sb)]['frames']
            if item.get('room'):
                item['room'] = rooms_by[L]
                check_events(item, out, a.update_events, failures)
            v = out['views']
            print(f"{L}: dumps {out.get('never_equals_committed', out.get('never_equals_frozen_m4b_runner', 'recorded'))}; "
                  + '; '.join(f"sb{sb} frames {r['frames']} end {r['end']} identical {r['variants_identical']} "
                              f"determ {r['strict_runs_identical']} 2d {sum(r['undefined_2d'].values())}"
                              + (f" m5a {r['m5a_view_consistency']['equal']}/{r['m5a_view_consistency']['differ']}" if 'm5a_view_consistency' in r else '')
                              for sb, r in v.items()), flush=True)
    if a.update_events:
        MANIFEST.write_text(json.dumps(man, indent=1) + '\n'); print(f'updated {MANIFEST.relative_to(ROOT)}')
    if todo: print('frames per configuration: ' + ', '.join(f'sb{sb} {n}' for sb, n in total.items()))
    for L, r in results.items():
        for sb, v in r['views'].items():
            c = v.get('m5a_view_consistency')
            if c and c['differ']: failures.append(f'{L} sb{sb}: {c["differ"]} message-free frames differ from the M5a golden view (first {c["first_differing_tic"]})')
            if v['other_draws']: failures.append(f'{L} sb{sb}: M_Random drawn outside ST_Ticker')
            if v['palette_unknown']: failures.append(f'{L} sb{sb}: a palette pointer outside PLAYPAL')
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    full = not a.only and todo
    if full:
        doc = dict(schema=SCHEMA, runner_contract_sha256=sha(m5b_build.CONTRACT.read_bytes()), gfx_sha256=sha(GFX.read_bytes()),
                   manifest_sha256=sha(MANIFEST.read_bytes()), variants=VARIANTS, views=VIEWS,
                   contract='frames identical to the genuine engine under the declared sound-free, wipe-free contract (decision S15)',
                   asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
                   frames_per_configuration={str(k): v for k, v in total.items()}, coverage=coverage(results), items=results)
        files = {RESULTS: json.dumps(doc, indent=1) + '\n'}
        for (L, sb), t in goldens.items(): files[FRAMES_DIR / f'{L}.sb{sb}.frames'] = t
        if a.check:
            bad = [str(p.relative_to(ROOT)) for p, t in files.items() if not p.exists() or p.read_text() != t]
            extra = [str(p.relative_to(ROOT)) for p in FRAMES_DIR.iterdir() if p not in files] if FRAMES_DIR.exists() else []
            if bad or extra: sys.exit(f'differs: {bad}; not produced by this run: {extra}')
            print(f'{RESULTS.relative_to(ROOT)} and {len(files) - 1} golden files match ({len(results)} items)')
        else:
            FRAMES_DIR.mkdir(exist_ok=True)
            for p, t in files.items(): p.write_text(t)
            print(f'wrote {RESULTS.relative_to(ROOT)} and {len(files) - 1} golden files')
    if a.pngs:
        n = 0
        for item in todo:
            if not item.get('room'): continue
            for sb, tic in item['room'].get('png', []):
                raw = work / 'raw'; shutil.rmtree(raw, ignore_errors=True); raw.mkdir(parents=True)
                fp = work / 'png.frames'
                run(bins['strict'], item, work / 'runs' / 'png', ['--render', 'every', '--screenblocks', str(sb)], fp, raw=raw, nodump=True)
                rows = frows_of(fp.read_text())
                pal = int(next(l.split(',')[2] for l in rows['P'] if l.split(',')[1] == str(tic)))
                px = bytearray((raw / f'frame_{tic:05d}.raw').read_bytes())
                for l in rows['U']:
                    if l.split(',')[1] == str(tic):
                        for yy in range(200): px[yy * 320 + int(l.split(',')[2])] = 0     # tracker-flagged columns (R4)
                data = palette_png(bytes(px), pal)
                path = PNG_DIR / f"{item['lump']}_sb{sb}_{tic}.png"
                if a.check:
                    if not path.exists() or A.png_pixels(path.read_bytes()) != A.png_pixels(data): sys.exit(f'{path.relative_to(ROOT)} differs')
                else:
                    PNG_DIR.mkdir(exist_ok=True); path.write_bytes(data)
                n += 1
                shutil.rmtree(raw); fp.unlink()
        print(f"{'checked' if a.check else 'wrote'} {n} PNGs")
    if a.selftest:
        outcomes = selftest(bins, frozen, work)
        for what, detail, ok in outcomes: print(f"selftest {what}: {detail}: {'pass' if ok else 'FAIL'}")
        if not all(ok for _, _, ok in outcomes): sys.exit(1)


if __name__ == '__main__':
    main()
