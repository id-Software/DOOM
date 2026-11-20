#!/usr/bin/env python3
"""Local-only M5c check of sessions played in doom-play with sound, on a real map (never in CI: it reads the IWAD).

The M5c counterpart of tools/local/m5b_session_real.py. Given the IWAD and one or more demos that doom-play recorded
(a restart begins a new demo; pass each), it replays every demo, map and skill taken from its own header as
G_DoPlayDemo reads them (linuxdoom-1.10/g_game.c:1589-1606; version byte 110, doomdef.h:33), through

  Rust     crates/doom-game/examples/m5c_real_rows.rs (release): GameData::load_with_sound and replay, the app's own
           per-tic pipeline (wire 25, the sound session, the display under the contract with sound), at the session's
           screenblocks;
  genuine  linuxdoom-1.10 built with its in-process sound code (one define removed): the M5c runner
           (tools/reference/m5c_build.py) under the confirmed switches (tools/reference/m5c_reference.py
           RECOMMENDED: integrated handles, S_UpdateSounds after every tic, the genuine wipe, sound on) with
           --render every, in the strict, overflow and address variants, whose frame and sound streams and endings
           must be identical (address runs use ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so they are
           not LeakSanitizer coverage); and the genuine mixer harness (tools/reference/m5c_mixer_build.py, the three
           variants identical, schedule ticbuffers) fed the start list tools/reference/m5c_starts.py converts from the
           genuine stream, loading the IWAD's own sound lumps;

and compares, with the comparison code of tools/local/m5c_real_full.py (imported, not copied): every tic's frame row
(the whole-frame digest, or a tracker-flagged frame's masked digest; frames whose switch parity the Rust renderer
reports unknown, or that the genuine tracker flagged whole, are counted and not compared), its counters, M and X rows,
the P row (palette, palette set, face, message flag, M_Random index) and the Q row (face-zeroed digest, cumulative
sound draws, wipe draws); every sound-logic row S, T, K, C, L and R (actor origins matched by one serial bijection);
every mixer row, buffer digests included; the number of tics; and decision T12: the Rust detector's label and firing
count, the release starts against the genuine stream's, and, when the session holds a switch release or the detector
fired, the strict32 build's stream against the 64-bit one (a difference in audibility, volume, separation or draws
with the detector silent is a failure).

The output is one VERDICT line per demo with counts (tics, frames compared whole and masked, sound rows by kind,
starts, buffers), and for a failure the first differing tic and row kind. Counts, verdicts and digests only: no lump
content, no coordinate, no audio, no image. Every stream stays in --work (outside the repository) and is deleted once
compared.

--synthetic needs no IWAD: it runs the same check on committed synthetic demos (SNDBTN2: a switch press, its release
and the strict32 comparison; SNDMONST: monsters' sounds) over tests/reference/m5c_wads and the committed status bar,
sound and music WADs, which must PASS, and then plants faults on SNDMONST, each of which must FAIL at its place: one
command byte (the turn of one tic) changed in the demo fed to the Rust side only, one sound row (a start's volume)
changed and one mixer buffer digest changed in the Rust output.

    python3 tools/local/m5c_session_real.py --iwad PATH --lmp FILE [--lmp FILE ...] [--screenblocks 10|11]
                                            [--work DIR] [--rust PATH] [--jobs N] [--cc CC] [--cargo CARGO]
    python3 tools/local/m5c_session_real.py --synthetic [--work DIR] [--rust PATH]
"""
import argparse, concurrent.futures as cf, json, os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference')); sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m5c_real_full as RF       # noqa: E402  (the comparison code: compare_sound, compare_frames, compare_mixer, ...)
import m5c_build                 # noqa: E402
import m5c_mixer_build           # noqa: E402
import m5c_mixer_reference as MR  # noqa: E402  (read-only: SANITIZER, GOLDEN_SCHEDULE)
import m5c_reference as C        # noqa: E402  (read-only: VARIANTS, RECOMMENDED, ENV, normalise, end_of, rows_of)
import m5c_session as S          # noqa: E402  (read-only: mixer_tic)
import m5c_starts                # noqa: E402

REF = ROOT / 'tests/reference'
LUMP = 'M5CSREAL'                # the demo's lump name in the genuine runner's lump list (8 characters)
VERSION = 110                    # doomdef.h:33; G_DoPlayDemo refuses any other first byte (g_game.c:1589-1594)
DEMOMARKER = 0x80                # g_game.c:1488
SYNTHETIC = [('SNDBTN2', 'm5c_wads/sndbtn.wad', 'm5c_scripts/SNDBTN2.lmp'),
             ('SNDMONST', 'm5c_wads/sndmonst.wad', 'm5c_scripts/SNDMONST.lmp')]
RANK = {'tics': 0, 'frame': 1, 'sound': 2, 'mixer': 3, 't12': 4}


def header(data):
    """The demo header as G_DoPlayDemo reads it (g_game.c:1589-1606), or an error text for a demo the tool refuses."""
    if len(data) < 14: return None, f'{len(data)} bytes: shorter than a header and a marker'
    if data[0] != VERSION: return None, f'version byte {data[0]}, not {VERSION} (g_game.c:1589)'
    if any(data[10:13]): return None, 'more than one player in game (g_game.c:1605-1611): not a doom-play session'
    if data[-1] != DEMOMARKER or (len(data) - 14) % 4:   # g_game.c:1493-1497
        return None, 'the commands do not end with DEMOMARKER at a 4-byte boundary'
    return dict(skill=data[1], episode=data[2], map=data[3], commands=(len(data) - 14) // 4), None


# ------------------------------------------------------------------ building
def build_rust(cargo):
    """Builds m5c_real_rows (release) and returns the executable cargo reports."""
    p = subprocess.run([cargo, 'build', '--release', '--locked', '-p', 'doom-game', '--example', 'm5c_real_rows',
                        '--message-format=json-render-diagnostics'], cwd=ROOT, capture_output=True, text=True)
    if p.returncode: sys.exit(f'cargo build FAILED:\n{p.stderr[-2000:]}')
    exes = [m['executable'] for m in map(json.loads, p.stdout.splitlines())
            if m.get('reason') == 'compiler-artifact' and m['target']['name'] == 'm5c_real_rows' and m.get('executable')]
    if len(exes) != 1: sys.exit(f'cargo reported {len(exes)} m5c_real_rows executables')
    return Path(exes[0])


def build_c(work, cc):
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try:
            bins = {v: m5c_build.build(v, work / 'build' / 'm5c', cc) for v in C.VARIANTS + ['strict32']}
            mbins = {v: m5c_mixer_build.build(v, work / 'build' / 'mixer', cc, quiet=True) for v in C.VARIANTS}
        finally: sys.stdout = so
    return dict(bins=bins, mbins=mbins)


# ------------------------------------------------------------------ the genuine side
def genuine(binary, wads, lmp, sb, w, render):
    """One genuine replay with sound: dict(golden, sound, end)."""
    if w.exists(): shutil.rmtree(w)
    w.mkdir(parents=True)
    demo, fp, sp = w / f'{LUMP}.lmp', w / 'frames.txt', w / 'sound.txt'
    demo.write_bytes(lmp)
    cmd = [str(binary), *[x for p in wads for x in ('--wad', str(p))], '--wad', str(demo), '--demo', LUMP, '--no-dump',
           '--render', 'every' if render else 'none', '--screenblocks', str(sb), '--sound-out', str(sp),
           *[x for kv in C.RECOMMENDED.items() for x in kv]]
    if render: cmd += ['--frames', str(fp)]
    p = subprocess.run(cmd, capture_output=True, text=True, env=C.ENV)
    r = dict(code=p.returncode, summary=C.B.A.SUMMARY.search(p.stdout), stderr=p.stderr,
             frames=fp.read_text() if fp.exists() else '', sound=sp.read_text() if sp.exists() else '')
    shutil.rmtree(w)
    return dict(golden=C.normalise(r['frames'])[0] if render else '', sound=r['sound'], end=C.end_of(r))


def mixer(mbins, mixer_wads, sound, w):
    """The genuine mixer harness on the genuine stream's start list, every variant: dict(rows, identical) or error."""
    w.mkdir(parents=True, exist_ok=True)
    try: starts = m5c_starts.convert(sound, 'session')
    except m5c_starts.Refused as e: return dict(error=f'start list refused: {e}')
    lst = w / 'starts.txt'; lst.write_text(starts)
    res = {}
    for v, b in mbins.items():
        o = w / f'{v}.rows'
        p = subprocess.run([str(b), *[x for p in mixer_wads for x in ('--wad', str(p))], '--starts', str(lst),
                            '--schedule', MR.GOLDEN_SCHEDULE, '--out', str(o)], capture_output=True, text=True, env=C.ENV, cwd=w)
        res[v] = (p.returncode, o.read_text() if o.exists() else '', MR.SANITIZER.search(p.stderr))
    shutil.rmtree(w)
    s = res['strict'][1]
    bad = [v for v, (code, rows, san) in res.items() if code or san or rows != s or not rows]
    return dict(rows=s, identical=not bad, bad=bad)


def genuine_side(tools, wads, mixer_wads, lmp, sb, w, jobs):
    """The three runner variants, the mixer and (when asked later) strict32: dict(golden, sound, end, mixer, fails)."""
    with cf.ThreadPoolExecutor(max(1, jobs)) as pool:
        gs = dict(zip(C.VARIANTS, pool.map(lambda v: genuine(tools['bins'][v], wads, lmp, sb, w / v, True), C.VARIANTS)))
    s, fails = gs['strict'], []
    if not all(x['golden'] == s['golden'] and x['sound'] == s['sound'] and x['end'] == s['end'] for x in gs.values()):
        fails.append((-1, 'variants', f'the genuine variants differ: {dict((v, x["end"]) for v, x in gs.items())}'))
    if s['end'].split(':')[0] in ('refused', 'sanitizer', 'error', 'abnormal'):
        fails.append((-1, 'ending', f'the genuine runner ended {s["end"]}'))
    Z = (C.rows_of(s['sound'], 'Z') or [['Z']])[0][1:]
    Z = {Z[i]: int(Z[i + 1]) for i in range(0, len(Z) - 1, 2)}
    for key in ('mismatch', 'unknown_origin', 'other_draws'):
        if Z.get(key): fails.append((-1, 'sound', f'genuine Z row {key} = {Z[key]}'))
    m = mixer(tools['mbins'], mixer_wads, s['sound'], w / 'mixer')
    if 'error' in m: fails.append((-1, 'mixer', m['error']))
    elif not m['identical']: fails.append((-1, 'mixer', f'the genuine mixer variants {m["bad"]} differ or failed'))
    return dict(s, mixer=m.get('rows', ''), variants_identical=not fails, fails=fails)


# ------------------------------------------------------------------ the Rust side and the comparison
def rust_side(rust, wads, lmp, sb, w):
    """m5c_real_rows at one screen size: dict(sound, frames, mixer, summary) or dict(error)."""
    w.mkdir(parents=True, exist_ok=True)
    demo = w / 'rust.lmp'; demo.write_bytes(lmp)
    paths = dict(sound=w / 'r.sound', mixer=w / 'r.mixer', summary=w / 'r.sum', frames=w / 'r.frames')
    cmd = [str(rust), *[x for p in wads for x in ('--wad', str(p))], '--demo', str(demo), '--screenblocks', str(sb),
           '--frames', f'{sb}={paths["frames"]}', '--sound-out', str(paths['sound']), '--mixer-out', str(paths['mixer']),
           '--summary', str(paths['summary'])]
    p = subprocess.run(cmd, capture_output=True, text=True)
    out = dict(error=f'm5c_real_rows exit {p.returncode}: {p.stderr[-600:]}') if p.returncode else \
        {k: v.read_text() for k, v in paths.items()}
    shutil.rmtree(w)
    if 'summary' in out: out['summary'] = RF.summary_of(out['summary'])
    return out


def compare(g, r, g32=None):
    """Every comparison of one demo: (record of counts, failures sorted by tic then kind)."""
    fails = list(g['fails'])
    snd = RF.compare_sound(r['sound'], g['sound'])
    fails += [(d['tic'], 'sound', f"row {d['row']}: genuine {d['genuine']} rust {d['rust']}") for d in snd['first']]
    fr = RF.compare_frames(r['frames'], g['golden'])
    fails += [(d['tic'], 'frame', d['what']) for d in fr['first']]
    mx = RF.compare_mixer(r['mixer'], g['mixer'])
    fails += [(max(S.mixer_tic(d['genuine']), S.mixer_tic(d['rust'])), 'mixer', f"line {d['line']}: genuine {d['genuine']} rust {d['rust']}")
              for d in mx['first'][:1]]
    gt, rt = len(C.rows_of(g['sound'], 'R')), int(r['summary']['tics'])
    if gt != rt: fails.append((min(gt, rt), 'tics', f'genuine {gt} tics, rust {rt}'))
    tot = RF.sound_totals(g['sound'])
    sm = r['summary']
    t12 = dict(detector_firings=int(sm['detector_firings']), detector_first_tic=sm['detector_first_tic'],
               release_starts=int(sm['release_starts']), release_next_busy=int(sm['release_next_busy']),
               release_reads=int(sm['release_reads']), release_reads_next_busy=int(sm['release_reads_next_busy']))
    for k in ('release_starts', 'release_next_busy'):
        if t12[k] != tot.get(k, 0): fails.append((-1, 't12', f'{k}: rust {t12[k]}, genuine stream {tot.get(k, 0)}'))
    s32 = RF.compare_32(g['sound'], g32['sound']) if g32 is not None else None
    if s32 is not None and s32['differing_in_behaviour'] and not t12['detector_firings']:
        fails.append((-1, 't12', f"the strict32 stream differs in behaviour ({s32['differing_in_behaviour']} rows) and the detector is silent"))
    fails.sort(key=lambda x: (x[0] < 0, x[0], RANK.get(x[1], 9)))
    rec = dict(tics=rt, genuine_tics=gt, genuine_end=g['end'], rust_end=sm.get(next((k for k in sm if k.startswith('ending.')), ''), '?'),
               sound=dict(rows=snd['rows'], differ=snd['differ'], listener_unchecked=snd['listener_unchecked']),
               frames=fr['counts'], mixer=dict(equal=mx['equal'], buffers=mx['buffers'], starts=mx['starts'],
                                               differing_lines=mx['differing_lines'], stream_sha256=mx['stream_sha256']),
               genuine=tot, t12=t12, strict32=s32, variants_identical=g['variants_identical'], failures=fails, ok=not fails)
    return rec


def verify(tools, rust, wads, mixer_wads, lmp, sb, work, jobs, rust_lmp=None):
    """One demo end to end. rust_lmp (the planted-fault check only) feeds the Rust side another demo."""
    try:
        g = genuine_side(tools, wads, mixer_wads, lmp, sb, work / 'genuine', jobs)
        r = rust_side(rust, wads, rust_lmp or lmp, sb, work / 'rust')
        if 'error' in r: return dict(ok=False, failures=[(-1, 'rust', r['error'])]), g, None, r
        tot = RF.sound_totals(g['sound'])
        g32 = None
        if tools['bins']['strict32'] is not None and (tot.get('release_starts', 0) or int(r['summary']['detector_firings'])):
            g32 = genuine(tools['bins']['strict32'], wads, lmp, sb, work / 'strict32', False)
        return compare(g, r, g32), g, g32, r
    finally:
        shutil.rmtree(work, ignore_errors=True)


def show(name, h, sb, rec):
    """The per-demo lines and the VERDICT line; returns whether it passed."""
    if 'tics' in rec:
        f, s, m, t = rec['frames'], rec['sound'], rec['mixer'], rec['t12']
        label = f"layout-dependent from tic {t['detector_first_tic']}" if t['detector_firings'] else 'not layout-dependent'
        s32 = rec['strict32']
        print(f"{name}: episode {h['episode']} map {h['map']} skill {h['skill']} screenblocks {sb}; genuine end {rec['genuine_end']}, "
              f"rust ending {rec['rust_end']}; variants strict, overflow, address identical {rec['variants_identical']} "
              f"(address with leak detection disabled)")
        print(f"  T12: {label} (detector firings {t['detector_firings']}); release starts {t['release_starts']} "
              f"(next slot busy {t['release_next_busy']}), later reads {t['release_reads']} (next slot busy {t['release_reads_next_busy']}); "
              + (f"strict32: identical {s32['identical']}, differing rows {s32['differing_rows']}, in behaviour {s32['differing_in_behaviour']}"
                 if s32 else 'strict32: not run (no switch release, detector silent)'))
    for t, kind, text in rec['failures'][:10]:
        print(f'  tic {t}: {kind}: {str(text)[:300]}')
    if 'tics' in rec:
        f, s, m = rec['frames'], rec['sound'], rec['mixer']
        counts = (f"tics {rec['tics']}; frames {f.get('frames', 0)} (whole {f.get('compared_whole', 0)}, masked {f.get('compared_masked', 0)}, "
                  f"not comparable {f.get('non_comparable_parity', 0) + f.get('non_comparable_whole', 0)}, face equal {f.get('face_equal', 0)}); "
                  f"sound rows {' '.join(f'{k} {v}' for k, v in s['rows'].items())} (listener unchecked {s['listener_unchecked']}); "
                  f"starts {m['starts']}; buffers {m['buffers']}")
    else: counts = 'no counts'
    if rec['ok']: print(f'VERDICT: PASS: {name}: {counts}')
    else:
        t, kind, _ = rec['failures'][0]
        print(f'VERDICT: FAIL: {name}: first difference at tic {t} ({kind}); {counts}')
    return rec['ok']


# ------------------------------------------------------------------ the synthetic self-check
def synthetic(tools, rust, work, jobs):
    wads_after = [C.GFX, C.SFX, C.MUSIC]
    ok, cache = True, {}
    for name, wad, lmp in SYNTHETIC:
        data = (REF / lmp).read_bytes()
        h, err = header(data)
        if err: print(f'VERDICT: FAIL: {name}: {err}'); ok = False; continue
        rec, g, g32, r = verify(tools, rust, [REF / wad] + wads_after, [C.SFX], data, 10, work / name, jobs)
        ok &= show(name, h, 10, rec)
        cache[name] = (data, [REF / wad] + wads_after, g, g32, r, rec)
    if not ok: print('SELF-CHECK: FAIL: a committed synthetic demo did not pass'); return 1
    data, wads, g, g32, r, rec = cache['SNDMONST']
    results = []
    # 1. One command byte: the turn (angleturn, the third byte of a tic's command, g_game.c:1501) of the
    #    middle tic, changed in the demo fed to the Rust side only. The view turns on that tic, so its frame differs.
    k = rec['tics'] // 2
    planted = bytearray(data); planted[13 + 4 * k + 2] = (planted[13 + 4 * k + 2] + 16) & 0xff
    r2 = rust_side(rust, wads, bytes(planted), 10, work / 'planted')
    x = compare(g, r2, g32)
    results.append(('command byte', f'the turn of tic {k} changed in the Rust side\'s demo', x, k))
    # 2. One sound row: the volume of the first started sound after tic 10, in the Rust output.
    rows = r['sound'].splitlines()
    i = next(j for j, l in enumerate(rows) if l.startswith('S,') and l.split(',')[6] == 'started' and int(l.split(',')[1]) >= 10)
    f = rows[i].split(','); t = int(f[1]); f[9] = str(int(f[9]) % 8 + 1); rows[i] = ','.join(f)
    x = compare(g, dict(r, sound='\n'.join(rows) + '\n'), g32)
    results.append(('sound row', f'the volume of the start at tic {t} changed in the Rust output', x, t))
    # 3. One mixer buffer digest in the Rust output (the first buffer row after the first start).
    ml = r['mixer'].splitlines()
    first_s = next(j for j, l in enumerate(ml) if l.startswith('s '))
    bi = next(j for j, l in enumerate(ml) if j > first_s and l.startswith('b '))
    f = ml[bi].split(); f[3] = ('0' if f[3][0] != '0' else '1') + f[3][1:]; ml[bi] = ' '.join(f)
    bt = S.mixer_tic(ml[bi])
    x = compare(g, dict(r, mixer='\n'.join(ml) + '\n'), g32)
    results.append(('mixer buffer', f'the digest of the buffer row after tic {bt} changed in the Rust output', x, bt))
    good = True
    for what, how, x, want in results:
        first = x['failures'][0][:2] if x['failures'] else None
        kind = {'command byte': 'frame', 'sound row': 'sound', 'mixer buffer': 'mixer'}[what]
        hit = not x['ok'] and first == (want, kind)
        good &= hit
        print(f"planted {what}: {how}: verdict {'PASS' if x['ok'] else 'FAIL'}, first difference {first}, expected ({want}, '{kind}'): "
              f"{'caught at its place' if hit else 'NOT CAUGHT AT ITS PLACE'}")
    print(f"SELF-CHECK: {'PASS' if good else 'FAIL'}: {len(SYNTHETIC)} synthetic demos passed, {sum(1 for *_, x, w in results if not x['ok'])} "
          f"of {len(results)} planted faults failed{' at their place' if good else ''}")
    return 0 if good else 1


# ------------------------------------------------------------------ main
def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--iwad', type=Path)
    ap.add_argument('--lmp', type=Path, action='append', default=[], help='a demo doom-play recorded (repeatable)')
    ap.add_argument('--synthetic', action='store_true', help='the self-check on committed synthetic demos (no IWAD)')
    ap.add_argument('--screenblocks', type=int, default=10, choices=[10, 11])
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5c-session-real')
    ap.add_argument('--rust', type=Path, help='m5c_real_rows (release); built with cargo when absent')
    ap.add_argument('--jobs', type=int, default=2, help='genuine runs at once (default 2)')
    ap.add_argument('--cc', default='cc'); ap.add_argument('--cargo', default='cargo')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    if not a.synthetic and (not a.iwad or not a.lmp): sys.exit('give --iwad and at least one --lmp, or --synthetic')
    if not a.synthetic and not a.iwad.is_file(): sys.exit(f'no IWAD at {a.iwad}')
    heads = []
    for p in a.lmp:
        h, err = header(p.read_bytes())
        if err: sys.exit(f'{p.name}: refused: {err}')
        heads.append(h)
    rust = a.rust.resolve() if a.rust else build_rust(a.cargo)
    tools = build_c(work, a.cc)
    if tools['bins']['strict32'] is None: print('note: no 32-bit toolchain: the strict32 comparison cannot run')
    if a.synthetic: return synthetic(tools, rust, work / 'synthetic', a.jobs)
    iwad, ok = a.iwad.resolve(), True
    for n, (p, h) in enumerate(zip(a.lmp, heads)):
        rec, *_ = verify(tools, rust, [iwad], [iwad], p.read_bytes(), a.screenblocks, work / f'demo{n}', a.jobs)
        ok &= show(p.name, h, a.screenblocks, rec)
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
