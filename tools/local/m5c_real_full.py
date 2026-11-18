#!/usr/bin/env python3
"""Local-only M5c step 5b: the 61 real Episode 1 runs through the whole Rust sound pipeline against the genuine
engine, every sound-logic row, every frame at screenblocks 10 and 11 and every mixer buffer (never in CI: it reads
the commercial IWAD).

The genuine side is linuxdoom-1.10 built with its in-process sound code (one define removed), under the declared
schedule: the M5c runner (tools/reference/m5c_build.py, strict) under the recommended switches of
tools/reference/m5c_reference.py (RECOMMENDED: integrated handles, S_UpdateSounds after every tic, the genuine wipe,
sound on), with the IWAD only in the lump list; and the genuine mixer driver (tools/reference/m5c_mixer_build.py,
strict) fed the start list that tools/reference/m5c_starts.py converts from the genuine stream (the started S rows),
loading the IWAD's own sound lumps. The Rust side is crates/doom-game/examples/m5c_real_rows.rs: GameData::
load_with_sound and replay, exactly the app's per-tic pipeline (wire 25, the sound session, the display under the
WithSound contract). Expected values come from the genuine C only, never from Rust output.

For every run of tests/reference/m4b_real_results.json (the committed E1M1-E1M9 scripts and DEMO1, extracted from
the IWAD copy and patched 109 to 110 by tools/local/m5a_real.py), one run per worker:

  genuine  the runner at screenblocks 10 and 11, --render every, --frames and --sound-out; the screenblocks-10
           stream must give the committed sound_sha256 of tests/reference/m5c_real_results.json (the runner is the
           one step 2 measured) and the screenblocks-11 stream must equal it; the strict32 build (-m32) with
           --render none and --sound-out (T12's independent check); the mixer driver on the start list;
  Rust     m5c_real_rows at screenblocks 10 and 11;
  sound    every S, T, K, L, C and R row of each tic equal, the actor origins matched to the genuine thinker serials by
           one bijection per run (crates/doom-sound/tests/common/gate.rs's rule); for a start without a placement
           (NULL origin, or the listener's own body) the genuine listener x, y and angle are not compared (the
           wire-25 record does not carry them; the step 4b report's SIM NEEDS) and those rows are counted;
  frames   per screen size, every frame's F row (the whole-frame digest, or where the tracker flagged columns the
           masked digest and the flagged columns, the renderer counters, fuzzpos), M, P (palette, palette set,
           face, message_on, M_Random index), Q (the face-zeroed digest, cumulative sound draws, wipe draws, face
           rectangle) and X rows; a frame whose switch parity the Rust renderer reports unknown, or that the
           genuine tracker could not attribute as a whole, is counted and not digest-compared (as in M5b);
  mixer    every row of the genuine mixer driver's output (start echoes, buffer digests, the drain, the stream
           digest; the device rows d and q aside) equal to the rows the Rust session's echoes and buffers give;
  T12      the Rust detector's firings and first tic, the release starts and the later S_UpdateSounds reads of a
           release origin, with and without a busy next slot; and the strict32 stream against the 64-bit one:
           every differing row is listed by kind and whether it is a release with a busy next slot, and whether
           it differs in audibility, volume, separation or draws.

Every run's streams are deleted once compared (they carry map coordinates and lump-derived data). --write writes
tests/reference/m5c_real_full_results.json (schema DOOM-M5C-REAL-FULL-RESULTS-v1: counts, pass/fail facts and
genuine digests only; never a coordinate, a lump byte, audio or an image); --check compares and writes nothing.
--report PATH (scratch only, never committed) writes the first differences of every run, for diagnosis.

--synthetic runs the same comparison on the 64 committed synthetic items against their committed genuine goldens
(tests/reference/m5c_sound, m5c_frames, m5c_mixer/sessions/goldens; no IWAD, nothing written), the proof that the
comparator and the example agree where the evidence is already settled; M5CHEAT is left out (its two cheats reach
the status bar only through replay_adjusted, decision S1). --selftest plants faults in copies of the genuine rows
(one S row's volume, one frame digest, one face, one mixer buffer digest, one serial swapped) and requires each to
be reported at exactly its place.

    python3 tools/local/m5c_real_full.py --rust PATH [--iwad PATH] [--work DIR] [--maps E1M1,..] [--only WALK,..]
                                         [--jobs N] [--write | --check] [--report PATH] [--selftest] [--cc CC]
    python3 tools/local/m5c_real_full.py --rust PATH --synthetic [--selftest] [--work DIR] [--jobs N]
"""
import argparse, collections, concurrent.futures as cf, hashlib, json, os, re, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference')); sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m5c_build                 # noqa: E402
import m5c_mixer_build           # noqa: E402
import m5c_starts                # noqa: E402
import m5c_reference as C        # noqa: E402  (read-only: switches, normalise, truncate, rows_of)
import m5b_reference as B        # noqa: E402  (read-only: FCOLS, the summary regex)
from m5a_real import runs_selected, demo_path  # noqa: E402

RESULTS = ROOT / 'tests/reference/m5c_real_full_results.json'
M5C_REAL = ROOT / 'tests/reference/m5c_real_results.json'
ITEMS_RS = ROOT / 'crates/doom-sim/tests/wire25/mod.rs'
REF = ROOT / 'tests/reference'
SCHEMA = 'DOOM-M5C-REAL-FULL-RESULTS-v1'
SOUND_KINDS = 'STKCLR'
FRAME_KINDS = 'FUMPQX'
VIEWS = [10, 11]
ENV = C.ENV
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()
KEEP = 12          # differences kept per run for the report


# ------------------------------------------------------------------ comparison (shared by real and synthetic)
def by_tic(text, kinds):
    """{tic: [row, ...]} of the rows of the given kinds, in file order."""
    out = collections.defaultdict(list)
    for l in text.splitlines():
        if l[:1] in kinds and l[1:2] == ',':
            out[int(l.split(',')[1])].append(l)
    return out


class Serials:
    """The bijection between genuine thinker serials and Rust `A<slot>:<generation>` handles (gate.rs Serials)."""
    def __init__(self): self.to_c, self.to_r = {}, {}

    def same(self, r, g):
        a, b = self.to_c.get(r), self.to_r.get(g)
        if a is None and b is None:
            self.to_c[r] = g; self.to_r[g] = r; return True
        return a == g and b == r


def unplaced(r, g):
    """The genuine S row with its listener columns made `-` when neither side places the start (gate.rs rule)."""
    if r[0] != 'S' or g[0] != 'S' or len(r) != 22 or len(g) != 22: return None
    if any(x != '-' for x in r[17:22]) or g[17] != '-' or g[18] != '-': return None
    return g[:19] + ['-', '-', '-']


def compare_sound(rust, genuine):
    """Row-by-row comparison of two DOOM-M5C-SOUND-v1 texts. Returns counts and the first differences."""
    R, G = by_tic(rust, SOUND_KINDS), by_tic(genuine, SOUND_KINDS)
    ser, counts, diffs, unchecked = Serials(), collections.Counter(), [], 0
    tics = sorted(set(R) | set(G))
    for t in tics:
        r, g = R.get(t, []), G.get(t, [])
        for i in range(max(len(r), len(g))):
            rr = r[i].split(',') if i < len(r) else ['<none>']
            gg = g[i].split(',') if i < len(g) else ['<none>']
            p = unplaced(rr, gg)
            if p is not None: gg = p; unchecked += 1
            ok = len(rr) == len(gg)
            if ok:
                for a, b in zip(rr, gg):
                    if a.startswith('A') and ':' in a:
                        if not ser.same(a, b): ok = False; break
                    elif a != b: ok = False; break
            if gg[0] != '<none>': counts[gg[0]] += 1
            if not ok:
                counts['differ'] += 1
                if len(diffs) < KEEP: diffs.append(dict(tic=t, row=i, genuine=','.join(gg), rust=','.join(rr)))
    return dict(rows=dict(sorted((k, v) for k, v in counts.items() if k != 'differ')), differ=counts['differ'],
                listener_unchecked=unchecked, tics=len(tics), first=diffs)


def compare_frames(rust, genuine):
    """Per-frame comparison of the Rust frame rows with the genuine golden rows (m5c_reference.normalise)."""
    R, G = by_tic(rust, FRAME_KINDS), by_tic(genuine, FRAME_KINDS)
    c, diffs = collections.Counter(), []

    def note(t, what):
        c['mismatched_rows'] += 1
        if len(diffs) < KEEP: diffs.append(dict(tic=t, what=what))
    for t in sorted(set(R) | set(G)):
        r, g = R.get(t, []), G.get(t, [])
        kind = lambda rows, k: [x.split(',') for x in rows if x.startswith(k + ',')]
        rf, gf = kind(r, 'F'), kind(g, 'F')
        if len(rf) != 1 or len(gf) != 1:
            note(t, f'frame present: rust {len(rf)} genuine {len(gf)}'); continue
        c['frames'] += 1
        rf, gf = rf[0], gf[0]
        parity, whole = rf[2] == 'parity', gf[14] == '1'
        if parity: c['non_comparable_parity'] += 1
        elif whole: c['non_comparable_whole'] += 1
        elif gf[2] in ('refused2d',): note(t, 'genuine refused2d')
        elif gf[2] == 'undefined' or rf[2] == 'undefined':
            c['compared_masked'] += 1
            if rf[2:4] != gf[2:4]: note(t, 'masked digest or flag')
        else:
            c['compared_whole'] += 1
            if rf[2] != gf[2] or rf[3] != gf[3]:
                rq, gq = kind(r, 'Q'), kind(g, 'Q')
                inside = rq and gq and rq[0][2] == gq[0][2]
                note(t, 'whole digest differs ' + ('inside the face rectangle only' if inside else 'outside the face rectangle'))
        if rf[4:14] != gf[4:14]: note(t, 'counters')
        if [x[2] for x in kind(r, 'U')] != [x[2] for x in kind(g, 'U')]: note(t, 'flagged columns')
        for k in 'MX':
            if [x for x in r if x.startswith(k + ',')] != [x for x in g if x.startswith(k + ',')]: note(t, f'{k} rows')
        rp, gp = kind(r, 'P'), kind(g, 'P')
        if rp != gp: note(t, f'P {rp} genuine {gp}')
        if rp and gp and rp[0][4] == gp[0][4]: c['face_equal'] += 1
        rq, gq = kind(r, 'Q'), kind(g, 'Q')
        if len(rq) != 1 or len(gq) != 1: note(t, 'Q row missing'); continue
        rq, gq = rq[0], gq[0]
        if rq[3:] != gq[3:]: note(t, f'Q draws or rectangle {rq[3:]} genuine {gq[3:]}')
        if not (parity or whole or gq[2] in ('undefined', 'refused2d') or rq[2] == 'undefined'):
            c['face_zeroed_compared'] += 1
            if rq[2] == gq[2]: c['face_zeroed_equal'] += 1
            else: note(t, 'face-zeroed digest')
    return dict(counts=dict(sorted(c.items())), first=diffs)


def compare_mixer(rust, genuine):
    """Line-by-line comparison of the mixer rows (device rows d and q removed from the genuine side)."""
    g = [l for l in genuine.splitlines() if not l.startswith(('d ', 'q '))]
    r = rust.splitlines()
    diffs = [dict(line=i, genuine=g[i] if i < len(g) else '<end>', rust=r[i] if i < len(r) else '<end>')
             for i in range(max(len(g), len(r))) if (g[i] if i < len(g) else None) != (r[i] if i < len(r) else None)]
    z = next((l.split() for l in g if l.startswith('z ')), None)
    return dict(equal=not diffs, differing_lines=len(diffs), buffers=int(z[1]) if z else None,
                starts=sum(1 for l in g if l.startswith('s ')), stream_sha256=z[2] if z else None, first=diffs[:KEEP])


def sound_totals(text):
    """Counts from a genuine stream's S rows: starts, outcomes, draws, release starts (busy next slot: ox or oy set)."""
    S = C.rows_of(text, 'S')
    out = collections.Counter(starts=len(S))
    for s in S:
        out[s[6]] += 1
        out['draws'] += int(s[7])
        if s[4] == 'button':
            out['release_starts'] += 1
            if s[17] not in ('0', '-') or s[18] not in ('0', '-'): out['release_next_busy'] += 1
    out['tics'] = len(C.rows_of(text, 'R'))
    return dict(sorted(out.items()))


def compare_32(s64, s32):
    """T12's independent check: the strict32 stream against the 64-bit one, row by row through the shorter."""
    a, b = by_tic(s64, SOUND_KINDS), by_tic(s32, SOUND_KINDS)
    last = min(max(a, default=-1), max(b, default=-1))
    rows, behav = [], 0
    for t in range(last + 1):
        x, y = a.get(t, []), b.get(t, [])
        for i in range(max(len(x), len(y))):
            p, q = (x[i] if i < len(x) else '<none>'), (y[i] if i < len(y) else '<none>')
            if p == q: continue
            pf, qf = p.split(','), q.split(',')
            release = pf[0] == 'S' and len(pf) == 22 and pf[4] == 'button' and (pf[17] != '0' or pf[18] != '0')
            update = pf[0] == 'C' and len(pf) > 4 and pf[4] == 'button'
            behaviour = (pf[0] != qf[0] or (pf[0] == 'S' and (pf[6], pf[7], pf[9], pf[10]) != (qf[6], qf[7], qf[9], qf[10]))
                         or (pf[0] == 'R' and pf[2:5] != qf[2:5]))
            behav += behaviour
            rows.append(dict(tic=t, kind=pf[0], release_next_busy=release, release_update=update,
                             audibility_volume_separation_or_draws=bool(behaviour)))
    return dict(identical=not rows and max(a, default=-1) == max(b, default=-1), tics_compared=last + 1,
                end64=max(a, default=-1), end32=max(b, default=-1), differing_rows=len(rows),
                differing_not_release=sum(1 for r in rows if not (r['release_next_busy'] or r['release_update'])),
                differing_in_behaviour=behav, kinds=dict(collections.Counter(r['kind'] for r in rows)))


def summary_of(text):
    return dict(l.split(',', 1) for l in text.splitlines() if ',' in l)


def run_rust(rust, wads, lmp, w):
    paths = dict(sound=w / 'r.sound', mixer=w / 'r.mixer', summary=w / 'r.sum', **{f'sb{s}': w / f'r.sb{s}' for s in VIEWS})
    cmd = [str(rust), *[x for p in wads for x in ('--wad', str(p))], '--demo', str(lmp)]
    for s in VIEWS: cmd += ['--screenblocks', str(s), '--frames', f'{s}={paths[f"sb{s}"]}']
    cmd += ['--sound-out', str(paths['sound']), '--mixer-out', str(paths['mixer']), '--summary', str(paths['summary'])]
    p = subprocess.run(cmd, capture_output=True, text=True)
    if p.returncode: return None, f'm5c_real_rows exit {p.returncode}: {p.stderr[-600:]}'
    out = {k: v.read_text() for k, v in paths.items()}
    return out, None


def plant_and_check(rust_out, gsound, gframes, gmixer):
    """--selftest: plant one fault in a copy of each genuine text and require the comparator to report it there."""
    res, ok = {}, True
    lines = gsound.splitlines()
    si = next((i for i, l in enumerate(lines) if l.startswith('S,') and l.split(',')[6] == 'started'), None)
    if si is not None:
        f = lines[si].split(','); f[9] = str(int(f[9]) + 1); planted = '\n'.join(lines[:si] + [','.join(f)] + lines[si + 1:]) + '\n'
        x = compare_sound(rust_out['sound'], planted)
        res['sound_volume'] = x['differ'] == 1 and x['first'][0]['tic'] == int(f[1])
    oids = [(i, l.split(',')) for i, l in enumerate(lines) if l.startswith('S,') and l.split(',')[4] == 'mobj']
    pair = next(((i, j) for (i, a) in oids for (j, b) in oids if a[5] != b[5] and i < j), None)
    if pair:
        i, j = pair
        f = lines[i].split(','); g2 = lines[j].split(',')
        f[5] = g2[5]
        planted = '\n'.join(lines[:i] + [','.join(f)] + lines[i + 1:]) + '\n'
        x = compare_sound(rust_out['sound'], planted)
        res['serial_swapped'] = x['differ'] >= 1
    fl = gframes.splitlines()
    fi = next((i for i, l in enumerate(fl) if l.startswith('F,') and l.split(',')[3] == '-' and l.split(',')[2] != 'undefined'
               and ',parity,' not in rust_out['sb10']), None)
    if fi is not None:
        f = fl[fi].split(','); f[2] = ('0' if f[2][0] != '0' else '1') + f[2][1:]
        x = compare_frames(rust_out['sb10'], '\n'.join(fl[:fi] + [','.join(f)] + fl[fi + 1:]) + '\n')
        res['frame_digest'] = x['counts'].get('mismatched_rows') == 1 and x['first'][0]['tic'] == int(f[1])
    pi = [i for i, l in enumerate(fl) if l.startswith('P,')]
    if pi:
        i = pi[len(pi) // 2]; f = fl[i].split(','); f[4] = str(int(f[4]) ^ 1)
        x = compare_frames(rust_out['sb10'], '\n'.join(fl[:i] + [','.join(f)] + fl[i + 1:]) + '\n')
        res['face'] = x['counts'].get('mismatched_rows') == 1 and x['counts'].get('face_equal', 0) == \
            compare_frames(rust_out['sb10'], gframes)['counts'].get('face_equal', 0) - 1
    ml = gmixer.splitlines()
    bi = next((i for i, l in enumerate(ml) if l.startswith('b ')), None)
    if bi is not None:
        f = ml[bi].split(); f[3] = ('0' if f[3][0] != '0' else '1') + f[3][1:]
        x = compare_mixer(rust_out['mixer'], '\n'.join(ml[:bi] + [' '.join(f)] + ml[bi + 1:]) + '\n')
        res['mixer_buffer'] = x['differing_lines'] == 1
    for v in res.values(): ok &= v
    return dict(planted=res, **{'pass': ok and len(res) >= 4})


# ------------------------------------------------------------------ the real runs
def genuine_run(binary, iwad, lmp, w, extra, frames=True):
    w.mkdir(parents=True, exist_ok=True)
    fp, sp = w / 'frames.txt', w / 'sound.txt'
    cmd = [str(binary), '--wad', str(iwad), '--wad', str(lmp), '--demo', lmp.stem.upper(), '--no-dump',
           *[x for k, v in C.RECOMMENDED.items() for x in (k, v)], *extra, '--sound-out', str(sp)]
    if frames: cmd += ['--frames', str(fp)]
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV)
    out = dict(code=p.returncode, summary=B.A.SUMMARY.search(p.stdout), stderr=p.stderr,
               frames=fp.read_text() if fp.exists() else '', sound=sp.read_text() if sp.exists() else '')
    for x in (fp, sp):
        if x.exists(): x.unlink()
    return out


def one_real(r, a, bins, rust, work, m5c_real):
    name = f"{r['map']}_{Path(r['script']).stem.upper()}"
    w = work / 'runs' / name
    if w.exists(): shutil.rmtree(w)
    w.mkdir(parents=True)
    fails, report = [], {}
    try:
        lmp = demo_path(r, w, a.iwad)
        rec = dict(map=r['map'], script=r['script'])
        g = {s: genuine_run(bins['strict'], a.iwad, lmp, w / f'g{s}', ['--render', 'every', '--screenblocks', str(s)]) for s in VIEWS}
        g32 = genuine_run(bins['strict32'], a.iwad, lmp, w / 'g32', ['--render', 'none'], frames=False) if bins.get('strict32') else None
        for s in VIEWS:
            if g[s]['code'] or C.rows_of(g[s]['sound'], 'E') or B.A.report_of(g[s]['stderr']):
                fails.append(f'{name} sb{s}: genuine runner exit {g[s]["code"]} or I_Error or sanitizer report')
        s10 = g[10]['sound']
        last = max((int(x[1]) for x in C.rows_of(s10, 'R')), default=-1)
        rec['genuine_sound_equal_m5c_real'] = sha(C.truncate(s10, last, True)) == m5c_real.get(name, {}).get('sound', {}).get('sound_sha256')
        if not rec['genuine_sound_equal_m5c_real']: fails.append(f'{name}: the genuine stream differs from the committed m5c_real sound_sha256')
        rec['genuine_sound_screens_identical'] = g[11]['sound'] == s10
        if not rec['genuine_sound_screens_identical']: fails.append(f'{name}: the genuine sound streams differ between screenblocks 10 and 11')
        rec['genuine_end'] = g[10]['summary'].group(2) if g[10]['summary'] else f'abnormal:{g[10]["code"]}'
        rec['genuine'] = sound_totals(s10)
        lst = w / 'starts.txt'
        lst.write_text(m5c_starts.convert(s10, name))
        mo = w / 'mixer.txt'
        p = subprocess.run([str(bins['mixer']), '--wad', str(a.iwad), '--starts', str(lst), '--schedule', 'ticbuffers', '--out', str(mo)],
                           capture_output=True, text=True, env=ENV, cwd=w)
        gmixer = mo.read_text() if mo.exists() else ''
        if p.returncode or re.search(r'^m ', gmixer, re.M): fails.append(f'{name}: mixer driver exit {p.returncode} or a handle differs (m row)')
        ro, err = run_rust(rust, [a.iwad], lmp, w)
        if ro is None: return name, rec, fails + [f'{name}: {err}'], report
        sm = summary_of(ro['summary'])
        rec['rust_end'] = {s: sm.get(f'ending.{s}') for s in VIEWS}
        rec['rust_tics'] = int(sm['tics'])
        rec['t12'] = dict(detector_firings=int(sm['detector_firings']), detector_first_tic=sm['detector_first_tic'],
                          detector_textures=int(sm['detector_textures']), release_starts=int(sm['release_starts']),
                          release_next_busy=int(sm['release_next_busy']), release_reads=int(sm['release_reads']),
                          release_reads_next_busy=int(sm['release_reads_next_busy']))
        rec['rust_sound_screens_identical'] = sm['sound_screens_identical'] == '1'
        snd = compare_sound(ro['sound'], s10)
        rec['sound'] = {k: v for k, v in snd.items() if k != 'first'}
        report['sound'] = snd['first']
        if snd['differ']: fails.append(f'{name}: {snd["differ"]} sound rows differ, first {snd["first"][0]}')
        rec['frames'] = {}
        for s in VIEWS:
            gold = C.normalise(g[s]['frames'])[0]
            x = compare_frames(ro[f'sb{s}'], gold)
            rec['frames'][str(s)] = x['counts']
            report[f'frames{s}'] = x['first']
            if x['counts'].get('mismatched_rows'): fails.append(f'{name} sb{s}: {x["counts"]["mismatched_rows"]} frame rows differ, first {x["first"][0]}')
        mx = compare_mixer(ro['mixer'], gmixer)
        rec['mixer'] = {k: v for k, v in mx.items() if k != 'first'}
        report['mixer'] = mx['first']
        if not mx['equal']: fails.append(f'{name}: {mx["differing_lines"]} mixer lines differ, first {mx["first"][0]}')
        if g32 is not None:
            rec['strict32'] = compare_32(s10, g32['sound'])
            if g32['code']: fails.append(f'{name}: strict32 exit {g32["code"]}')
        if not rec['rust_sound_screens_identical']: fails.append(f'{name}: the Rust sound differs between screen sizes')
        if a.selftest:
            rec_st = plant_and_check(ro, s10, C.normalise(g[10]['frames'])[0], gmixer)
            report['selftest'] = rec_st
            if not rec_st['pass']: fails.append(f'{name}: selftest {rec_st}')
        return name, rec, fails, report
    finally:
        shutil.rmtree(w, ignore_errors=True)


# ------------------------------------------------------------------ the synthetic items
def items():
    text = ITEMS_RS.read_text()
    body = text[text.index('pub const ITEMS'):]
    return re.findall(r'\(\s*"(\w+)",\s*"([^"]+)",\s*"([^"]+)",?\s*\)', body[:body.index('];')])


def one_synthetic(item, a, rust, work):
    lump, wad, lmp = item
    w = work / 'synthetic' / lump
    if w.exists(): shutil.rmtree(w)
    w.mkdir(parents=True)
    try:
        wads = [REF / wad, REF / 'm5b_gfx.wad', REF / 'm5c_sfx.wad', REF / 'm5c_music.wad']
        ro, err = run_rust(rust, wads, REF / lmp, w)
        if ro is None: return lump, None, [f'{lump}: {err}']
        gs = (REF / f'm5c_sound/{lump}.sound').read_text()
        gm = (REF / f'm5c_mixer/sessions/goldens/{lump}.golden').read_text()
        rec = dict(sound=compare_sound(ro['sound'], gs), mixer=compare_mixer(ro['mixer'], gm),
                   frames={s: compare_frames(ro[f'sb{s}'], (REF / f'm5c_frames/{lump}.sb{s}.frames').read_text()) for s in VIEWS},
                   summary=summary_of(ro['summary']))
        fails = []
        if rec['sound']['differ']: fails.append(f"{lump}: sound {rec['sound']['first'][:2]}")
        if not rec['mixer']['equal']: fails.append(f"{lump}: mixer {rec['mixer']['first'][:2]}")
        for s in VIEWS:
            if rec['frames'][s]['counts'].get('mismatched_rows'): fails.append(f"{lump} sb{s}: frames {rec['frames'][s]['first'][:2]}")
        if a.selftest and lump in ('SNDBUSY', 'SNDMONST', 'BARON1'):
            rec['selftest'] = plant_and_check(ro, gs, (REF / f'm5c_frames/{lump}.sb10.frames').read_text(), gm)
            if not rec['selftest']['pass']: fails.append(f"{lump}: selftest {rec['selftest']}")
        return lump, rec, fails
    finally:
        shutil.rmtree(w, ignore_errors=True)


def synthetic(a, rust, work):
    todo = [i for i in items() if i[0] != 'M5CHEAT']
    tot, fails = collections.Counter(), []
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        for lump, rec, f in pool.map(lambda i: one_synthetic(i, a, rust, work), todo):
            fails += f
            if rec is None: continue
            tot['items'] += 1
            for k, v in rec['sound']['rows'].items(): tot[f'sound_{k}'] += v
            tot['sound_listener_unchecked'] += rec['sound']['listener_unchecked']
            tot['mixer_buffers'] += rec['mixer']['buffers'] or 0
            tot['mixer_starts'] += rec['mixer']['starts']
            tot['mixer_equal'] += rec['mixer']['equal']
            for s in VIEWS:
                for k, v in rec['frames'][s]['counts'].items(): tot[f'sb{s}_{k}'] += v
            tot['detector_firings'] += int(rec['summary']['detector_firings'])
            if 'selftest' in rec: print(f"{lump} selftest: {rec['selftest']}", flush=True)
    print('synthetic totals:', json.dumps(dict(sorted(tot.items()))))
    if fails: print('FAILED:\n  ' + '\n  '.join(fails[:60])); return 1
    print(f"synthetic: {tot['items']} items, every compared row equal")
    return 0


# ------------------------------------------------------------------ main
def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--rust', type=Path, required=True, help='m5c_real_rows (release)')
    ap.add_argument('--iwad', type=Path, default=Path.home() / 'scratch/m5c/iwad/DOOM.WAD')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5c-real-full')
    ap.add_argument('--maps'); ap.add_argument('--only'); ap.add_argument('--jobs', type=int, default=2)
    ap.add_argument('--cc', default='cc'); ap.add_argument('--report', type=Path)
    ap.add_argument('--synthetic', action='store_true'); ap.add_argument('--selftest', action='store_true')
    g = ap.add_mutually_exclusive_group(); g.add_argument('--write', action='store_true'); g.add_argument('--check', action='store_true')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    if a.report and ROOT in a.report.resolve().parents: sys.exit('--report must lie outside the repository')
    if a.synthetic: return synthetic(a, a.rust, work)
    if not a.iwad.is_file():
        print(f'skip: no IWAD at {a.iwad} (local-only M5c whole-pipeline real-map comparison)'); return 0
    if (a.write or a.check) and (a.maps or a.only): sys.exit('--write and --check cover every run')
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try:
            bins = dict(strict=m5c_build.build('strict', work / 'build' / 'm5c', a.cc),
                        strict32=m5c_build.build('strict32', work / 'build' / 'm5c', a.cc),
                        mixer=m5c_mixer_build.build('strict', work / 'build' / 'mixer', a.cc, quiet=True))
        finally: sys.stdout = so
    m5c_real = json.loads(M5C_REAL.read_text())['runs']
    todo = runs_selected(a.maps.split(',') if a.maps else None, {x.upper() for x in a.only.split(',')} if a.only else None)
    todo = sorted(todo, key=lambda r: -r['tics'])
    recs, fails, reports = {}, [], {}
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        for name, rec, f, rep in pool.map(lambda r: one_real(r, a, bins, a.rust, work, m5c_real), todo):
            recs[name] = rec; fails += f; reports[name] = rep
            fr = rec.get('frames', {})
            print(f"{name}: tics {rec.get('rust_tics')} sound differ {rec.get('sound', {}).get('differ')} "
                  + ' '.join(f"sb{s} frames {fr.get(str(s), {}).get('frames')} mism {fr.get(str(s), {}).get('mismatched_rows', 0)}" for s in VIEWS)
                  + f" mixer {rec.get('mixer', {}).get('equal')} ({rec.get('mixer', {}).get('buffers')} buffers) "
                  f"T12 {rec.get('t12')} s32 {(rec.get('strict32') or {}).get('differing_rows')} end {rec.get('genuine_end')}/{rec.get('rust_end')}"
                  + (f" FAIL {len(f)}" if f else ''), flush=True)
    recs = dict(sorted(recs.items()))
    S = lambda f: sum(f(r) for r in recs.values())
    totals = dict(
        runs=len(recs),
        genuine={k: S(lambda r, k=k: r.get('genuine', {}).get(k, 0)) for k in ('tics', 'starts', 'started', 'inaudible', 'no_channel',
                                                                       'link_volume', 'draws', 'release_starts', 'release_next_busy')},
        sound_rows_compared={k: S(lambda r, k=k: r.get('sound', {}).get('rows', {}).get(k, 0)) for k in SOUND_KINDS},
        sound_rows_differ=S(lambda r: r.get('sound', {}).get('differ', 0)),
        sound_listener_unchecked=S(lambda r: r.get('sound', {}).get('listener_unchecked', 0)),
        frames={str(s): {k: S(lambda r, k=k, s=s: r.get('frames', {}).get(str(s), {}).get(k, 0))
                         for k in ('frames', 'compared_whole', 'compared_masked', 'non_comparable_parity', 'non_comparable_whole',
                                   'face_equal', 'face_zeroed_compared', 'face_zeroed_equal', 'mismatched_rows')} for s in VIEWS},
        mixer=dict(runs_equal=S(lambda r: r.get('mixer', {}).get('equal', False)), buffers=S(lambda r: r.get('mixer', {}).get('buffers') or 0),
                   starts=S(lambda r: r.get('mixer', {}).get('starts', 0))),
        t12={k: S(lambda r, k=k: r.get('t12', {}).get(k, 0)) for k in ('detector_firings', 'release_starts', 'release_next_busy',
                                                                      'release_reads', 'release_reads_next_busy')},
        t12_layout_dependent_runs=S(lambda r: r.get('t12', {}).get('detector_firings', 0) > 0),
        strict32=dict(runs_identical=S(lambda r: (r.get('strict32') or {}).get('identical', False)),
                      differing_rows=S(lambda r: (r.get('strict32') or {}).get('differing_rows', 0)),
                      differing_not_release=S(lambda r: (r.get('strict32') or {}).get('differing_not_release', 0)),
                      differing_in_behaviour=S(lambda r: (r.get('strict32') or {}).get('differing_in_behaviour', 0))),
        endings=dict(sorted(collections.Counter(r.get('genuine_end') for r in recs.values()).items())))
    print('totals:', json.dumps(totals))
    if a.report:
        a.report.parent.mkdir(parents=True, exist_ok=True)
        a.report.write_text(json.dumps(dict(totals=totals, runs=reports, fails=fails), indent=1) + '\n')
        print(f'wrote {a.report}')
    if fails: print('FAILED:\n  ' + '\n  '.join(fails[:200])); return 1
    doc = dict(schema=SCHEMA, note='M5c step 5b (local only): the 61 real Episode 1 runs through the whole Rust sound pipeline '
               '(crates/doom-game/examples/m5c_real_rows.rs: GameData::load_with_sound and replay) against linuxdoom-1.10 built with '
               'its in-process sound code (one define removed), under the declared schedule: every sound-logic row, every frame at '
               'screenblocks 10 and 11 and every mixer buffer, the T12 detector and the strict32 comparison. Counts, pass/fail facts '
               'and genuine digests only.', switches=C.RECOMMENDED,
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
