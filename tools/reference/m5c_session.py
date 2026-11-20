#!/usr/bin/env python3
"""M5c step 5c: scripted app sessions under the new contract, replayed through the genuine engine with sound.

docs/M5C_REFERENCE_INVESTIGATION.md sections 3.1.3, 3.1.4, 4.3.3 and 4.7.1 and decisions T5, T6, T14 and T15 (with the
owner's rulings of 2026-11-18), in the style of tools/reference/m5b_session.py. Every session script of
tests/reference/m5b_sessions/*.txt (the four M5b scripts, unchanged) and tests/reference/m5c_sessions/*.txt (written by
hand to make sound matter: a fight, switches with their release and a door, a pause, a restart, a death) is a list of
platform-level events with timestamps on a fake clock. For each one:

  app       the headless app (crates/doom-game/examples/m5c_session.rs, a release build) plays the script under the new
            contract: GameData::load_with_sound (wire 25, the sound session, the display with sound) with a sink that
            logs every buffer, pause, resume and session start. A restart (r while paused or on the end screen) is a
            fresh process (decision T15) and begins a new part with its own demo. Per part it writes the demo, the
            command rows, the frame rows in the genuine M5c golden format, the sound-logic rows S, T, K, L, C, R, the
            mixer driver's rows with the drain and every buffer's digest, and requires the replay of its own demo to
            give the same rows (live equals recorded);
  input     each part's posted events become an event_t script of the genuine input driver (tools/reference/
            m5b_input_build.py, strict, through m5b_session.driver_script), whose recorded demo must equal the part's;
  engine    each part's demo is replayed by the genuine M5c runner (tools/reference/m5c_build.py) in the strict, overflow
            and address variants with --render every, the recommended switches (m5c_reference.RECOMMENDED: integrated
            handles, S_UpdateSounds after every tic, the genuine wipe, sound on) and --sound-out. The normalised frame
            streams and the sound streams must be identical across the variants, and on every tic: players[0].cmd (from
            the strict run's game dumps) equals the app's executed command; the frame digest equals the app's (M5b's
            rules: a tracker-flagged frame on its masked digest and flagged columns; switch parity unknown or a
            whole-frame flag is not compared) with its counters, M and X rows; the P row (palette, palette set, face,
            message on, M_Random index) is equal; the Q row (the digest with the face zeroed, cumulative sound draws,
            wipe draws, rectangle) is equal; and every sound row S, T, K, C, L, R is equal (actor origins matched by one
            serial-to-handle bijection per part; the three listener columns of a start with no placement are not
            compared, because the wire-25 record carries no listener there: the doom-sound gate's rule);
  mixer     the genuine stream's start list (tools/reference/m5c_starts.py) is mixed by the genuine mixer harness
            (tools/reference/m5c_mixer_build.py, strict, overflow and address identical, schedule ticbuffers, decision
            T5) on tests/reference/m5c_sfx.wad: its rows (device rows aside) must equal the app's mixer rows, and every
            buffer the app handed to its sink must equal the genuine buffer of the same number by digest, with as many
            genuine buffers due through the part's last tic as the app mixed;
  pause     in the app's log no tic runs and no buffer is mixed between a pause and the next resume, the buffers handed
            to the sink are exactly the parts' buffers in order, and every part begins with session_start. A script
            with a line `#! nopause P R` (frame period 10 ms, P = 10 mod 200, R a multiple of 10) is also played with
            its pause and resume keys removed and every later line moved earlier by R - (P - 10) ms, which puts every
            later event in the same tic: the demo, the commands and state digests, frames, sound rows, mixer rows and
            buffers of both plays must be identical, and the genuine input driver given the play without the pause
            must record the same demo. The event rows differ only by the release the pause posts at the first tic
            after the resume (decision S6: a keyup for each held key and one empty mouse event, given to the genuine
            input driver as well). The genuine side never sees a pause (a demo holds none), so it is the same either way.

The claim is: identical to linuxdoom-1.10 built with its in-process sound code (one define removed), replaying the
session's demo, under the declared schedule (decision T5; T14: a recorded demo holds no menu hot keys). Results
(tests/reference/m5c_session_results.json, schema DOOM-M5C-SESSION-RESULTS-v1) hold counts and digests of the genuine
side only, are written by a plain run and compared by --check (which writes nothing). --selftest plants faults on the
session `fight` and requires each to be caught at its place: a changed command byte in the demo, a changed frame row of
the app, a changed sound row of the app (the volume of one start), a dropped buffer, a dropped tic, a dropped event in
the input driver's script and a tic logged during a pause. Address runs use ASAN_OPTIONS=detect_leaks=0: leak detection
is disabled, so they are not LeakSanitizer coverage.

    m5c_session.py [--check] [--selftest] [--only NAME[,NAME]] [--work DIR] [--jobs N] [--cc CC] [--cargo CARGO]
"""
import argparse, collections, concurrent.futures as cf, hashlib, json, os, re, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5c_build                 # noqa: E402
import m5c_mixer_build as mb     # noqa: E402
import m5c_mixer_reference as MR  # noqa: E402  (read-only: run_driver, expand, DEVICE_ROWS)
import m5c_reference as C        # noqa: E402  (read-only: RECOMMENDED, normalise, end_of, GFX, SFX, MUSIC)
import m5c_starts                # noqa: E402
import m5b_input_build as ib     # noqa: E402
import m5b_session as MS         # noqa: E402  (read-only: header, driver_script, run_driver, rows_of)
import m3h9c_dump as dump        # noqa: E402

SESSION_DIRS = [ROOT / 'tests/reference/m5b_sessions', ROOT / 'tests/reference/m5c_sessions']
RESULTS = ROOT / 'tests/reference/m5c_session_results.json'
SCHEMA = 'DOOM-M5C-SESSION-RESULTS-v1'
CLAIM = ('identical to linuxdoom-1.10 built with its in-process sound code (one define removed), replaying the '
         "session's demo, under the declared schedule (decision T5; T14: a recorded demo holds no menu hot keys)")
VARIANTS, LUMP, ENV = C.VARIANTS, 'M5CSESS', C.ENV
WADS_AFTER = [C.GFX, C.SFX, C.MUSIC]
MIXER_SC = dict(group='golden', wad='main', lumps=False, probe=False)
SCHEDULE = MR.GOLDEN_SCHEDULE
SELFTEST_SESSION = 'fight'
SOUND_KINDS = 'STKCLR'
FRAME_KINDS = 'FUMPQX'
RANK = {'tics': 0, 'command': 1, 'frame': 2, 'status': 3, 'face': 4, 'sound': 5, 'buffer': 6, 'mixer': 7}
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def sessions(only=None):
    """{name: path} of every session script; names are unique across the two directories."""
    out = {}
    for d in SESSION_DIRS:
        for p in sorted(d.glob('*.txt')):
            if p.stem in out: sys.exit(f'two session scripts named {p.stem}')
            out[p.stem] = p
    if only:
        want = only.split(',')
        bad = [w for w in want if w not in out]
        if bad: sys.exit(f'unknown session in --only: {bad}')
        out = {k: v for k, v in out.items() if k in want}
    return out


# ---------------------------------------------------------------- building
def build_example(cargo):
    """Builds the m5c_session example (release) and returns the executable cargo reports."""
    p = subprocess.run([cargo, 'build', '--release', '--locked', '-p', 'doom-game', '--example', 'm5c_session',
                        '--message-format=json-render-diagnostics'], cwd=ROOT, capture_output=True, text=True)
    if p.returncode: sys.exit(f'cargo build FAILED:\n{p.stderr[-2000:]}')
    exes = [m['executable'] for m in map(json.loads, p.stdout.splitlines())
            if m.get('reason') == 'compiler-artifact' and m['target']['name'] == 'm5c_session' and m.get('executable')]
    if len(exes) != 1: sys.exit(f'cargo reported {len(exes)} m5c_session executables')
    return Path(exes[0])


def build(work, cc, cargo):
    exe = build_example(cargo)
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try:
            bins = {v: m5c_build.build(v, work / 'build', cc) for v in VARIANTS}
            mbins = {v: mb.build(v, work / 'mixerbuild', cc, quiet=True) for v in VARIANTS}
            driver = ib.build('strict', work / 'inputbuild', cc, quiet=True)
        finally: sys.stdout = so
    return dict(exe=exe, bins=bins, mbins=mbins, driver=driver)


# ---------------------------------------------------------------- the app
def run_app(exe, wads, out, h, script):
    if out.exists(): shutil.rmtree(out)
    cmd = [str(exe), *[a for w in wads for a in ('--wad', str(w))], '--out', str(out), '--script', str(script),
           '--map', str(h['map']), '--skill', str(h['skill']), '--screenblocks', str(h['screenblocks'])]
    p = subprocess.run(cmd, capture_output=True, text=True)
    if p.returncode: return dict(error=f'app failed: {p.stderr.strip()[-500:]}')
    summary = dict(l.split(',', 1) for l in (out / 'summary.txt').read_text().splitlines())
    parts = []
    for n in range(int(summary['parts'])):
        d = out / f'part{n}'
        parts.append(dict(rows=MS.rows_of((d / 'rows.txt').read_text()), lmp=(d / 'session.lmp').read_bytes(),
                          frames=(d / 'frames.txt').read_text(), sound=(d / 'sound.txt').read_text(),
                          mixer=(d / 'mixer.txt').read_text(), buffers=(d / 'buffers.txt').read_text().splitlines(),
                          detector=(summary[f'part{n}.detector_first_tic'], int(summary[f'part{n}.detector_firings']))))
    return dict(parts=parts, ending=summary['ending'], log=(out / 'log.txt').read_text().splitlines(), dir=out)


def pause_check(app):
    """The app's log: no tic or buffer between a pause and the next resume; the sink's buffers are the parts' buffers in
    order; each part begins with session_start. Returns (counts, failures)."""
    fails, paused, pauses, tics, bufs, starts = [], False, 0, [], [], 0
    for i, l in enumerate(app['log']):
        k = l.split(',')
        if k[0] == 'pause': paused, pauses = True, pauses + 1
        elif k[0] == 'resume': paused = False
        elif k[0] == 'session_start':
            starts += 1; paused = False
        elif k[0] in ('tic', 'buffer'):
            if paused: fails.append((-1, 'pause', f'log line {i + 1}: {l} while paused'))
            (tics if k[0] == 'tic' else bufs).append(k)
            if k[0] == 'tic' and k[2] == '0' and int(k[1]) + 1 != starts:
                fails.append((-1, 'pause', f'log line {i + 1}: part {k[1]} began without its own session_start'))
        else: fails.append((-1, 'pause', f'log line {i + 1}: {l}'))
    want_bufs = [b.split(',')[3] for p in app['parts'] for b in p['buffers']]
    if [b[2] for b in bufs] != want_bufs: fails.append((-1, 'pause', 'the sink received other buffers than the parts mixed'))
    want_tics = [(str(n), l.split(',')[1]) for n, p in enumerate(app['parts']) for l in p['rows']['T']]
    if [(t[1], t[2]) for t in tics] != want_tics: fails.append((-1, 'pause', 'the logged tics are not the parts\' tics'))
    if starts != len(app['parts']): fails.append((-1, 'pause', f'{starts} session starts for {len(app["parts"])} parts'))
    return dict(pauses=pauses, session_starts=starts, sink_buffers=len(bufs)), fails


def nopause_text(text):
    """The script without its pause (the `#! nopause P R` line), or None when it has none."""
    d = [l.split() for l in text.splitlines() if l.startswith('#! nopause')]
    if not d: return None
    P, R = int(d[0][2]), int(d[0][3])
    frame = [l.split() for l in text.splitlines() if l.split()[:1] == ['frame']]
    if frame != [['frame', '10000']] or P % 200 != 10 or R % 10: sys.exit(f'nopause {P} {R}: needs frame 10000, P = 10 mod 200, R = 0 mod 10')
    D, out, dropped = R - (P - 10), [], 0
    for l in text.splitlines():
        w = l.split('#')[0].split()
        if not w or not w[0].isdigit(): out.append(l); continue
        t = int(w[0])
        if P <= t <= R + 10 and w[1:2] == ['key'] and w[2] in ('esc', 'pause'):
            dropped += 1; continue
        if P <= t <= R and w != [w[0], 'end']: sys.exit(f'nopause: an event between the pause and the resume: {l}')
        out.append(l if t < R else str(t - D) + l[len(w[0]):])
    if dropped != 4: sys.exit(f'nopause: {dropped} pause key lines between {P} and {R + 10}, expected 4 (down and up twice)')
    return '\n'.join(out) + '\n'


def nopause_same(app, b):
    """Whether the play without the pause gives the same parts: the demo, the commands and state digests, frames,
    sound rows, mixer rows and buffers identical, and the log identical once its pause and resume lines are dropped. The
    event rows differ only by what the pause's release posts at the first tic after the resume (decision S6,
    crates/doom-game/src/input/layer.rs release_all: a keyup for every held key and one mouse event with no buttons
    and no motion), which the genuine input driver is given too. Returns (same, number of such release events)."""
    if 'error' in b or len(b['parts']) != len(app['parts']): return False, None
    if [l for l in app['log'] if l not in ('pause', 'resume')] != b['log']: return False, None
    extra = 0
    for x, y in zip(app['parts'], b['parts']):
        keys = ('lmp', 'frames', 'sound', 'mixer', 'buffers', 'detector')
        if any(x[k] != y[k] for k in keys) or x['rows'].get('T') != y['rows'].get('T'): return False, None
        ex, ey = x['rows'].get('E', []), y['rows'].get('E', [])
        if any(e not in ex for e in ey): return False, None
        more = [e for e in ex if e not in ey]
        if len({e.split(',')[1] for e in more}) > 1: return False, None
        if any(e.split(',')[2:] != ['mouse', '0', '0', '0'] and e.split(',')[2] != 'keyup' for e in more): return False, None
        extra += len(more)
    return True, extra


# ---------------------------------------------------------------- the genuine side
def run_engine(binary, wads, lmp, sb, work, dumps):
    """One genuine replay with sound: dict(golden, full, sound, end, last, cmds)."""
    if work.exists(): shutil.rmtree(work)
    work.mkdir(parents=True)
    demo = work / f'{LUMP}.lmp'; demo.write_bytes(lmp)
    fp, sp = work / 'frames.txt', work / 'sound.txt'
    cmd = [str(binary), *[a for w in wads for a in ('--wad', str(w))], '--wad', str(demo), '--demo', LUMP,
           *(['--out', str(work / 'dumps')] if dumps else ['--no-dump']), '--render', 'every', '--screenblocks', str(sb),
           '--frames', str(fp), '--sound-out', str(sp), *[x for kv in C.RECOMMENDED.items() for x in kv]]
    if dumps: (work / 'dumps').mkdir()
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV)
    r = dict(code=p.returncode, summary=C.B.A.SUMMARY.search(p.stdout), stderr=p.stderr,
             frames=fp.read_text() if fp.exists() else '', sound=sp.read_text() if sp.exists() else '')
    golden, full = C.normalise(r['frames'])
    cmds = {}
    if dumps:
        for name in dump.snapshot_names(str(work / 'dumps')):
            for l in (work / 'dumps' / name).read_text().splitlines():
                f = l.split(',')
                if f[0] == 'P' and f[2] == '0' and name != 'load.txt': cmds[int(f[1])] = f[6:10]
    shutil.rmtree(work)
    return dict(golden=golden, full=full, sound=r['sound'], end=C.end_of(r), last=C.last_tic_of(r), cmds=cmds)


def run_mixer(mbins, sound, work):
    """The genuine mixer on the genuine stream's start list: dict(rows (device rows aside), identical, starts, error)."""
    work.mkdir(parents=True, exist_ok=True)
    try: starts = m5c_starts.convert(sound, 'session')
    except m5c_starts.Refused as e: return dict(error=f'start list refused: {e}')
    lst = work / 'starts.txt'; lst.write_text(starts)
    res = {v: MR.run_driver(b, MIXER_SC, lst, SCHEDULE, work / v) for v, b in mbins.items()}
    s = res['strict']
    bad = [v for v, r in res.items() if r['code'] or MR.SANITIZER.search(r['stderr']) or r['rows'] != s['rows']]
    return dict(rows=MR.strip_device(s['rows']), identical=not bad, bad=bad, starts=starts,
                nstarts=sum(1 for l in starts.splitlines() if l.startswith('s ')))


def by_tic(text, kinds):
    out = collections.defaultdict(list)
    for l in text.splitlines():
        if l[:1] in kinds and l[1:2] == ',': out[int(l.split(',')[1])].append(l)
    return out


class Serials:
    """One bijection between genuine thinker serials and the app's `A<slot>:<generation>` handles."""
    def __init__(self): self.to_g, self.to_a = {}, {}

    def same(self, a, g):
        x, y = self.to_g.get(a), self.to_a.get(g)
        if x is None and y is None:
            self.to_g[a] = g; self.to_a[g] = a; return True
        return x == g and y == a


def compare_sound(app, genuine):
    """Every sound row of every tic, positionally. Returns (counts by kind, unplaced starts, failures)."""
    A, G = by_tic(app, SOUND_KINDS), by_tic(genuine, SOUND_KINDS)
    ser, n, fails, unplaced = Serials(), collections.Counter(), [], 0
    for t in sorted(set(A) | set(G)):
        a, g = A.get(t, []), G.get(t, [])
        for i in range(max(len(a), len(g))):
            ra = a[i].split(',') if i < len(a) else ['<none>']
            rg = g[i].split(',') if i < len(g) else ['<none>']
            if rg[0] != '<none>': n[rg[0]] += 1
            if ra[0] == 'S' == rg[0] and len(ra) == len(rg) == 22 and ra[17:] == ['-'] * 5 and rg[17:19] == ['-', '-']:
                rg = rg[:19] + ['-'] * 3; unplaced += 1
            ok = len(ra) == len(rg) and all(ser.same(x, y) if x.startswith('A') and ':' in x else x == y for x, y in zip(ra, rg))
            if not ok: fails.append((t, 'sound', f'row {i}: genuine {",".join(rg)} app {",".join(ra)}'))
    return dict(sorted(n.items())), unplaced, fails


def compare_frames(app, golden):
    """Every frame's rows. Returns (counts, failures)."""
    A, G = by_tic(app, FRAME_KINDS), by_tic(golden, FRAME_KINDS)
    c, fails = collections.Counter(), []
    kind = lambda rows, k: [x.split(',') for x in rows if x.startswith(k + ',')]
    for t in sorted(set(A) | set(G)):
        a, g = A.get(t, []), G.get(t, [])
        af, gf = kind(a, 'F'), kind(g, 'F')
        if len(af) != 1 or len(gf) != 1:
            fails.append((t, 'tics', f'frame rows: app {len(af)} genuine {len(gf)}')); continue
        af, gf = af[0], gf[0]
        c['frames'] += 1
        parity, whole = af[2] == 'parity', gf[14] == '1'
        if parity or whole: c['non_comparable'] += 1
        elif gf[2] == 'refused2d': fails.append((t, 'frame', 'the genuine frame is refused2d'))
        elif gf[2] == 'undefined' or af[2] == 'undefined':
            c['masked'] += 1
            if af[2:4] != gf[2:4] or [x[2] for x in kind(a, 'U')] != [x[2] for x in kind(g, 'U')]:
                fails.append((t, 'frame', 'masked frame or flagged columns differ'))
        else:
            c['whole'] += 1
            if af[2:4] != gf[2:4]: fails.append((t, 'frame', 'frame digest differs'))
        if af[4:14] != gf[4:14]: fails.append((t, 'frame', f'counters: genuine {gf[4:14]} app {af[4:14]}'))
        for k in 'MX':
            if kind(a, k) != kind(g, k): fails.append((t, 'status', f'{k} rows: genuine {kind(g, k)} app {kind(a, k)}'))
        ap, gp = kind(a, 'P'), kind(g, 'P')
        c['status_rows'] += 1
        if ap != gp: fails.append((t, 'status', f'P: genuine {gp} app {ap}'))
        elif ap: c['face_and_index_equal'] += 1
        aq, gq = kind(a, 'Q'), kind(g, 'Q')
        if len(aq) != 1 or len(gq) != 1: fails.append((t, 'face', 'Q row missing')); continue
        aq, gq = aq[0], gq[0]
        if aq[3:] != gq[3:]: fails.append((t, 'face', f'Q draws or rectangle: genuine {gq[3:]} app {aq[3:]}'))
        if not (parity or whole or 'undefined' in (aq[2], gq[2]) or gq[2] == 'refused2d'):
            c['face_zeroed'] += 1
            if aq[2] != gq[2]: fails.append((t, 'face', 'face-zeroed digest differs'))
    return dict(sorted(c.items())), fails


def mixer_tic(row):
    f = row.split()
    try: return int(f[1]) if f[0] == 's' else int(f[2]) if f[0] == 'b' else int(f[3]) if f[0] == 'r' else -1
    except (IndexError, ValueError): return -1


def compare_part(part, g, m):
    """Every per-tic comparison of one app part with its genuine run and genuine mixer: (counts, failures)."""
    fails = []
    T = part['rows'].get('T', [])
    n = dict(tics=len(T), commands=0)
    gtics = len(by_tic(g['golden'], 'F'))
    if gtics != len(T): fails.append((min(gtics, len(T)), 'tics', f'genuine frames {gtics}, app tics {len(T)}'))
    for l in T:
        t, c = int(l.split(',')[1]), l.split(',')[2:6]
        if g['cmds'].get(t) != c: fails.append((t, 'command', f'genuine {g["cmds"].get(t)} app {c}'))
        else: n['commands'] += 1
    n['frames'], f = compare_frames(part['frames'], g['golden'])
    fails += f
    n['sound_rows'], n['unplaced_starts'], f = compare_sound(part['sound'], g['sound'])
    fails += f
    if 'error' in m: fails.append((-1, 'mixer', m['error']))
    else:
        gm, am = m['rows'].splitlines(), part['mixer'].splitlines()
        for i in range(max(len(gm), len(am))):
            x, y = (gm[i] if i < len(gm) else '<end>'), (am[i] if i < len(am) else '<end>')
            if x != y:
                fails.append((max(mixer_tic(x), mixer_tic(y)), 'mixer', f'row {i}: genuine {x} app {y}')); break
        gb = [l.split() for _k, l in MR.expand(m['rows'], SCHEDULE) if l.startswith('b ')]
        last = int(T[-1].split(',')[1]) if T else -1
        due = [b for b in gb if int(b[2]) <= last]
        ab = [b.split(',') for b in part['buffers']]
        for k in range(max(len(ab), len(due))):
            a = ab[k] if k < len(ab) else None
            b = due[k] if k < len(due) else None
            if a is None or b is None or (a[2], a[3]) != (b[2], b[3]):
                fails.append((min(int(x[2]) for x in (a, b) if x), 'buffer', f'buffer {k}: genuine {b and b[2:4]} app {a and a[2:4]}')); break
        n['buffers'] = len(ab)
        n['buffers_equal'] = next((k for k in range(min(len(ab), len(due))) if (ab[k][2], ab[k][3]) != (due[k][2], due[k][3])),
                                  min(len(ab), len(due)))
    return n, sorted(fails, key=lambda x: (x[0] < 0, x[0], RANK.get(x[1], 9)))


def verify(tools, name, path, work, jobs_variants=VARIANTS):
    """One session script: the app, the input driver, the genuine runner and mixer per part, the pause check."""
    text = path.read_text()
    h = MS.header(text)
    wads = h['wads'] + WADS_AFTER
    app = run_app(tools['exe'], wads, work / 'app', h, path)
    if 'error' in app: return dict(ok=False, failures=[(-1, 'app', app['error'])])
    rec = dict(source=path.relative_to(ROOT).as_posix(), script_sha256=sha(path.read_bytes()), screenblocks=h['screenblocks'],
               parts=[], failures=[])
    rec['pause'], f = pause_check(app)
    rec['failures'] += f
    np = nopause_text(text)
    if np is not None:
        (work / 'nopause.txt').write_text(np)
        b = run_app(tools['exe'], wads, work / 'nopause', h, work / 'nopause.txt')
        same, extra = nopause_same(app, b)
        if same:   # the genuine input driver, given the play without the pause, records the same demos
            same = all(MS.run_driver(tools['driver'], MS.driver_script(x['rows']), work / 'nopause_driver') == x['lmp']
                       for x in b['parts'])
        rec['pause'].update(nopause_identical=same, release_events=extra)
        if not same: rec['failures'].append((-1, 'pause', 'the play without the pause differs'))
    app_detector = []
    for k, part in enumerate(app['parts']):
        pw = work / f'part{k}'
        p = dict(tics=len(part['rows'].get('T', [])))
        demo = MS.run_driver(tools['driver'], MS.driver_script(part['rows']), pw / 'driver')
        p['driver_demo_sha256'] = demo and sha(demo)
        if demo is None or not (demo == part['lmp'] or ((h['skill'], h['map']) != (2, 1) and demo[13:] == part['lmp'][13:])):
            rec['failures'].append((-1, 'input', f'part {k}: the genuine input driver recorded a different demo'))
        gs = {v: run_engine(tools['bins'][v], wads, part['lmp'], h['screenblocks'], pw / v, v == 'strict') for v in jobs_variants}
        s = gs['strict']
        p['variants_identical'] = all(x['full'] == s['full'] and x['sound'] == s['sound'] and x['end'] == s['end'] for x in gs.values())
        if not p['variants_identical']: rec['failures'].append((-1, 'variants', f'part {k}: {dict((v, x["end"]) for v, x in gs.items())}'))
        m = run_mixer(tools['mbins'], s['sound'], pw / 'mixer')
        p['mixer_variants_identical'] = m.get('identical', False)
        if 'error' not in m and not m['identical']: rec['failures'].append((-1, 'mixer', f'part {k}: mixer variants {m["bad"]} differ'))
        counts, f = compare_part(part, s, m)
        rec['failures'] += [(t, kd, f'part {k}: {x}') for t, kd, x in f]
        P = C.rows_of(s['golden'], 'P')
        Z = (C.rows_of(s['sound'], 'Z') or [['Z']])[0][1:]
        Z = {Z[i]: int(Z[i + 1]) for i in range(0, len(Z) - 1, 2)}
        for key in ('mismatch', 'unknown_origin', 'other_draws'):
            if Z.get(key): rec['failures'].append((-1, 'sound', f'part {k}: genuine Z {key} = {Z[key]}'))
        z = (m.get('rows') or 'z ? ?').splitlines()[-1].split()
        p.update(genuine_end=s['end'], frames_sha256=sha(s['golden']), sound_sha256=sha(s['sound']),
                 starts=m.get('nstarts'), starts_sha256=m.get('starts') and sha(m['starts']),
                 mixer_buffers=int(z[1]) if z[1].isdigit() else None, mixer_stream_sha256=z[2], mixer_rows_sha256=m.get('rows') and sha(m['rows']),
                 sound_rows=counts['sound_rows'], unplaced_starts=counts['unplaced_starts'],
                 outcomes={k2: Z.get(k2, 0) for k2 in ('attempts', 'started', 'inaudible', 'no_channel')},
                 palettes=sorted({int(r[2]) for r in P}), faces=sorted({int(r[4]) for r in P}), counts=counts)
        rec['parts'].append(p)
        app_detector.append(part['detector'])
    rec['app_ending'], rec['app_detector'] = app['ending'], app_detector
    rec['failures'].sort(key=lambda x: (x[0] < 0, x[0], RANK.get(x[1], 9)))
    rec['ok'] = not rec['failures']
    return rec


# ---------------------------------------------------------------- self-test
def selftest(tools, path, work):
    """Plant faults on one session; each must be caught at its place."""
    text = path.read_text()
    h = MS.header(text)
    wads = h['wads'] + WADS_AFTER
    app = run_app(tools['exe'], wads, work / 'app', h, path)
    part = app['parts'][0]
    T = [l.split(',')[2:6] for l in part['rows']['T']]
    k = next(i for i in range(10, len(T) - 1) if T[i] != T[i + 1] and T[i][0] != '5')
    out = []

    def genuine(lmp):
        g = run_engine(tools['bins']['strict'], wads, lmp, h['screenblocks'], work / 'planted', True)
        return g, run_mixer({'strict': tools['mbins']['strict']}, g['sound'], work / 'planted_mixer')
    g0, m0 = genuine(part['lmp'])
    first = lambda p, g=g0, m=m0: compare_part(p, g, m)[1][:1]
    clean = first(part)
    out.append(('no fault', f'the unchanged session: first failure {clean}', clean == []))
    lmp = bytearray(part['lmp']); lmp[13 + 4 * k] = 5
    f = first(part, *genuine(bytes(lmp)))
    out.append(('changed command byte', f'forward of tic {k} set to 5; first failure {f}', bool(f) and f[0][:2] == (k, 'command')))
    rows = part['frames'].splitlines()
    i = next(j for j, l in enumerate(rows) if l.startswith(f'F,{k},'))
    fr = rows[i].split(','); fr[2] = '0' * 64; rows[i] = ','.join(fr)
    f = first(dict(part, frames='\n'.join(rows) + '\n'))
    out.append(('changed frame row', f'app frame digest of tic {k} replaced; first failure {f}', bool(f) and f[0][:2] == (k, 'frame')))
    rows = part['sound'].splitlines()
    i = next(j for j, l in enumerate(rows) if l.startswith('S,') and l.split(',')[6] == 'started' and int(l.split(',')[1]) >= 10)
    sr = rows[i].split(','); t = int(sr[1]); sr[9] = str(int(sr[9]) % 8 + 1); rows[i] = ','.join(sr)
    f = first(dict(part, sound='\n'.join(rows) + '\n'))
    out.append(('changed sound row', f'the volume of the start at tic {t} (seq {sr[2]}) changed; first failure {f}',
                bool(f) and f[0][:2] == (t, 'sound')))
    bufs = list(part['buffers'])
    j = next(x for x, b in enumerate(bufs) if x >= 5 and b.split(',')[3] != bufs[0].split(',')[3])
    bt = int(bufs[j].split(',')[2]); del bufs[j]
    f = first(dict(part, buffers=bufs))
    out.append(('dropped buffer', f'buffer {j} (after tic {bt}) left out of the sink stream; first failure {f}',
                bool(f) and f[0][:2] == (bt, 'buffer')))
    f = first(part, *genuine(part['lmp'][:13 + 4 * k] + part['lmp'][13 + 4 * (k + 1):]))
    out.append(('dropped tic', f'tic {k} removed from the demo; first failure {f}', bool(f) and f[0][:2] == (k, 'command')))
    drop = next(j for j, l in enumerate(part['rows']['E']) if ',keydown,' in l)
    demo = MS.run_driver(tools['driver'], MS.driver_script(part['rows'], drop), work / 'driver')
    out.append(('dropped event', f'event {drop} left out of the input driver script; demos equal {demo == part["lmp"]}', demo != part['lmp']))
    log = list(app['log']); log.insert(log.index('session_start') + 1, 'pause'); log.insert(log.index('pause') + 2, 'resume')
    _c, f = pause_check(dict(app, log=log))
    out.append(('tic during a pause', f'a pause planted around the first tic of the log; failures {f[:1]}', bool(f) and f[0][1] == 'pause'))
    return out


def layout_label(detector):
    """The label of decision T12 for one part, from the app's own report of the detector (first tic, firings), worded
    as tools/local/m5c_session_real.py prints it."""
    first, firings = detector
    return f'layout-dependent from tic {first}' if firings else 'not layout-dependent'


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5c-session')
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--only'); ap.add_argument('--jobs', type=int, default=3)
    ap.add_argument('--cc', default='cc'); ap.add_argument('--cargo', default='cargo')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    tools = build(work, a.cc, a.cargo)
    paths = sessions(a.only)
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        recs = dict(zip(paths, pool.map(lambda kv: verify(tools, kv[0], kv[1], work / 'runs' / kv[0]), paths.items())))
    failures = []
    for name, r in recs.items():
        det = r.get('app_detector') or []
        parts = '; '.join(f"part {k}: {p['tics']} tics, frames {p['counts']['frames']}, sound rows {p['sound_rows']}, "
                          f"buffers {p['counts'].get('buffers')}, genuine end {p['genuine_end']}, variants identical "
                          f"{p['variants_identical']}/{p['mixer_variants_identical']}, "
                          f"{layout_label(det[k]) if k < len(det) else 'layout label unknown'}"
                          for k, p in enumerate(r.get('parts', [])))
        print(f"{name}: {parts}; app {r.get('app_ending')}; T12 detector (first tic, firings) {r.get('app_detector')}; "
              f"pause {r.get('pause')}; {'pass' if r['ok'] else 'FAIL'}", flush=True)
        failures += [f'{name}: tic {t} {k}: {x}' for t, k, x in r['failures'][:5]]
    if failures:
        for line in failures: print(f'FINDING-CANDIDATE: {line}')
        sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    if not a.only:
        doc = dict(schema=SCHEMA, claim=CLAIM, switches=C.RECOMMENDED, mixer_schedule=SCHEDULE, variants=VARIANTS,
                   wads_after_the_room=[w.relative_to(ROOT).as_posix() for w in WADS_AFTER],
                   wads_sha256={w.relative_to(ROOT).as_posix(): sha(w.read_bytes()) for w in WADS_AFTER},
                   runner_contract_sha256=sha(m5c_build.CONTRACT.read_bytes()), mixer_contract_sha256=sha(mb.CONTRACT.read_bytes()),
                   input_contract_sha256=sha(ib.CONTRACT.read_bytes()),
                   asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
                   totals=dict(sessions=len(recs), parts=sum(len(r['parts']) for r in recs.values()),
                               tics=sum(p['tics'] for r in recs.values() for p in r['parts']),
                               starts=sum(p['starts'] or 0 for r in recs.values() for p in r['parts']),
                               buffers=sum(int(p['counts']['buffers']) for r in recs.values() for p in r['parts'])),
                   sessions={n: {k: v for k, v in r.items() if k not in ('ok', 'failures', 'app_ending', 'app_detector')}
                             for n, r in recs.items()})
        text = json.dumps(doc, indent=1) + '\n'
        if a.check:
            if not RESULTS.exists() or RESULTS.read_text() != text:
                (work / 'regenerated.json').write_text(text)
                sys.exit(f'{RESULTS.relative_to(ROOT)} differs (regenerated text at {work / "regenerated.json"})')
            print(f'{RESULTS.relative_to(ROOT)} matches ({len(recs)} sessions)')
        else:
            RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)}')
    print(f'every tic, frame, sound row and buffer of {len(recs)} sessions {CLAIM}')
    if a.selftest:
        outcomes = selftest(tools, sessions()[SELFTEST_SESSION], work / 'self')
        for what, detail, ok in outcomes: print(f"selftest {what}: {detail}: {'pass' if ok else 'FAIL'}")
        if not all(ok for *_, ok in outcomes): sys.exit(1)


if __name__ == '__main__':
    main()
