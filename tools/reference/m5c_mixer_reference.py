#!/usr/bin/env python3
"""Genuine mixer reference runs and goldens (contract DOOM-M5C-MIXER-v1, results DOOM-M5C-MIXER-RESULTS-v1).

The expected sample streams come from the genuine linuxdoom-1.10 in-process mixer only
(docs/M5C_REFERENCE_INVESTIGATION.md sections 1.3 and 3.2): the driver of m5c_mixer_build.py is built
in the strict, overflow and address variants under --work (and strict32 with --strict32, local only,
never required) and runs every scenario of this file. The independent model
(tools/reference/m5c_mixer_model.py) runs every scenario too, as a second check; it is never the
source of a golden.

Scenarios (written by this tool to tests/reference/m5c_mixer/, schema DOOM-M5C-STARTS-v1):
  starts/*.txt   goldened scenarios: every item of section 3.2.5 that is defined behaviour (volume,
                 separation and pitch extremes, the single-instance sounds, more than eight starts and
                 the oldest-channel rule with its ties, clipping, buffer boundaries, the linked
                 chaingun, the header fields, padding, the missing lump and the pistol fallback, the
                 handle wrap, the tables) plus four I_Error probes (x_*) outside s_sound.c's domain;
  witness/*.txt  the three short-lump starts (lumps of 0, 5 and 8 bytes), where the genuine code reads
                 one byte past its 8-byte zone block: never goldened; recorded as the refusal witnesses
                 of decision T11 (the driver flags each with an o row, the model refuses that start);
  probes/*.txt   the pitch-256 probe (out of s_sound.c's domain): the address build must report
                 i_sound.c:495; outside the golden set.

--check requires, and a plain run (which writes the start lists, goldens and results) also requires:
  - the committed start lists equal a fresh generation, and the committed contract the regenerated one;
  - every contract run exits 0 with no sanitizer report, and strict, overflow and address write
    byte-identical rows for every goldened scenario (under both schedules);
  - no m row (a start list handle that differs from the genuine handle) in any golden;
  - the model's rows equal the genuine rows (device rows d and q aside) on every goldened scenario,
    under both schedules;
  - the table digests equal an integer-only reconstruction (steptable as floor(2^(16+(p-128)/64)) by an
    integer root, vol_lookup by truncating division), entry by entry from the genuine dump;
  - each witness is flagged by an o row at the same start in every contract build, and the model
    refuses exactly that start with every earlier row equal;
  - the pitch-256 probe stops the address build with a report at i_sound.c:495;
  - with --check: tests/reference/m5c_mixer/goldens/*.golden (schedule ticbuffers, the recommended value
    of decision T5) and tests/reference/m5c_mixer_results.json equal the strict run. The results file
    records the switch values, counts, digests, the sanitizer findings and, as digests only, what
    --schedule pertic gives on each scenario.

--strict32 also builds the -m32 genuine-text build and requires its goldened rows to equal strict; it
prints whether the witnesses differ there. --selftest plants three faults and requires each to be
caught at its row: one flipped stream bit (only that buffer's row and the stream digest change), one
start moved by a tic (the first difference is that start's echo row), and one changed volume on a start
that is replaced before any buffer is mixed (only that start's echo row changes; the stream digest does
not). --raw writes each strict run's raw stream under --work (scratch only; never committed).

Address runs use ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so they are not LeakSanitizer
coverage. --jobs sets the number of processes run at once (default 4).

    m5c_mixer_reference.py [--work DIR] [--check] [--selftest] [--strict32] [--raw] [--only NAME[,NAME]]
                           [--jobs N] [--cc CC]
    m5c_mixer_reference.py --work DIR --sessions STREAMDIR [--strict32] [--jobs N]

--sessions runs every sound-logic stream (*.sound, DOOM-M5C-SOUND-v1) of STREAMDIR through m5c_starts.py and the
genuine mixer (every variant, both schedules) and the model on m5c_sfx.wad, requires identical rows, no sanitizer
report and every handle equal to the sound-logic run's, and prints the digests; it writes nothing in the tree (the
start lists go under --work). It is the path by which the goldens for the committed sound-logic streams are made.
"""
import argparse, concurrent.futures as cf, hashlib, json, os, re, struct, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5c_mixer_build as mb   # noqa: E402

DIR = ROOT / 'tests/reference/m5c_mixer'
GROUPS = {'golden': DIR / 'starts', 'witness': DIR / 'witness', 'probe': DIR / 'probes'}
GOLDENS = DIR / 'goldens'
RESULTS = ROOT / 'tests/reference/m5c_mixer_results.json'
MODEL = ROOT / 'tools/reference/m5c_mixer_model.py'
SCHEMA = 'DOOM-M5C-MIXER-RESULTS-v1'
WADS = {'main': ['tests/reference/m5c_sfx.wad'], 'edge': ['tests/reference/m5c_sfx_edge.wad']}
VARIANTS = ['strict', 'overflow', 'address']
SCHEDULES = ['ticbuffers', 'pertic']
GOLDEN_SCHEDULE = 'ticbuffers'
DEVICE_ROWS = ('d ', 'q ')
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
SANITIZER = re.compile(r'runtime error|AddressSanitizer|UndefinedBehaviorSanitizer|LeakSanitizer|SUMMARY: ')
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()

# sfx numbers (sounds.h:179-288) and the synthetic lumps (tools/reference/m5c_sfx.py)
PISTOL, SHOTGN, SGCOCK = 1, 2, 3
DBOPN, DBCLS, DBLOAD, PLASMA, BFG, SAWUP, SAWIDL, SAWFUL, SAWHIT = 5, 6, 7, 8, 9, 10, 11, 12, 13
RLAUNC, RXPLOD, FIRSHT, FIRXPL, PSTART, STNMOV, OOF, CHGUN = 14, 15, 16, 17, 18, 22, 34, 86


# ---------------------------------------------------------------- scenarios
class Lst:
    """Builds one DOOM-M5C-STARTS-v1 list; call orders count up within each tic."""
    def __init__(self, comment, tics=0):
        self.lines = [f'# {c}' for c in comment.split('\n')] + ['schema DOOM-M5C-STARTS-v1', 'config integrated']
        if tics: self.lines.append(f'tics {tics}')
        self.seq = {}

    def _seq(self, tic, n=1):
        s = self.seq.get(tic, 0); self.seq[tic] = s + n
        return s

    def s(self, tic, sfx, vol=15, sep=128, pitch=128, prio=64, handle='-', gametic=None):
        self.lines.append(f's {tic} {self._seq(tic)} {tic if gametic is None else gametic} {sfx} {vol} {sep} {pitch} {prio} {handle}')
        return self

    def n(self, count, tic, sfx, vol=8, sep=128, pitch=128, prio=64):
        self.lines.append(f'n {count} {tic} {self._seq(tic, count)} {tic} {sfx} {vol} {sep} {pitch} {prio}')
        return self

    def g(self, tic, sfx):
        self.lines.append(f'g {tic} {self._seq(tic)} {sfx}')
        return self

    def text(self): return '\n'.join(self.lines) + '\n'


def scenarios():
    """{name: dict(group, wad, lumps, probe, text)} in a fixed order."""
    sc = {}

    def add(name, group, wad, lst, lumps=False, probe=False):
        sc[name] = dict(group=group, wad=wad, lumps=lumps, probe=probe, text=lst.text())

    add('lumps_main', 'golden', 'main', Lst('The tables and every padded lump of m5c_sfx.wad as the genuine I_InitSound and '
        'I_SetChannels build them\n(i_sound.c:185-251, 396-424, 799-813); no start.'), lumps=True)
    add('lumps_edge', 'golden', 'edge', Lst('The edge set m5c_sfx_edge.wad: lumps of 0, 5 and 8 bytes (padded length 0), 1, '
        '511, 512, 513 and 1024 samples,\nthe header fields, pstart missing (getsfx copies dspistol, i_sound.c:213-216) and '
        'a dschgun lump\nthe link ignores (i_sound.c:808-811); no start.'), lumps=True)
    add('basic', 'golden', 'main', Lst('Six starts. The tic-10 pistol is replaced by the tic-11 pistol before any buffer is '
        'mixed (buffer 7 is\nmixed after tic 11; single-instance rule, i_sound.c:285-306), so it contributes no sample. '
        'The start\nlist carries the handles a sound-logic run would have received (checked by the driver).')
        .s(0, PISTOL, 15, 128, 128, 64, 100).s(3, SHOTGN, 8, 64, 120, 64, 101).s(3, 31, 15, 200, 140, 78, 102)
        .s(10, PISTOL, 127, 128, 128, 64, 103).s(11, PISTOL, 127, 32, 128, 64, 104).s(20, OOF, 100, 224, 113, 96, 105))
    l = Lst('Every volume s_sound.c can pass at the centre: 0 (the channel is busy but silent), 1, 8 (the default\n'
            'snd_SfxVolume, m_misc.c:237), 15 (the menu maximum and map 8, m_menu.c:821-826, s_sound.c:800-808) and 127\n'
            '(the S_SetSfxVolume limit, s_sound.c:634-635). Shotgn is 1024 samples.')
    for i, v in enumerate([0, 1, 8, 15, 127]): l.s(8 * i, SHOTGN, v)
    add('volume', 'golden', 'main', l)
    l = Lst('Separation: the s_sound.c extremes 33 and 224 (s_sound.c:793 with finesine -65535..65535) and 128\n'
            '(s_sound.c:315, 323), at volumes 127, 15 and 8.')
    t = 0
    for v in (127, 15, 8):
        for sp in (33, 128, 224): l.s(t, SHOTGN, v, sp); t += 8
    add('separation', 'golden', 'main', l)
    l = Lst('Separation values the mixer accepts beyond s_sound.c\'s domain (section 3.2.5 row 10): 0, 1, 255, 256\n'
            'and -1 at volume 127, and 257 at volume 15 (the range check depends on the volume, i_sound.c:364-368).')
    for i, (v, sp) in enumerate([(127, 0), (127, 1), (127, 255), (127, 256), (127, -1), (15, 257)]): l.s(8 * i, SHOTGN, v, sp)
    add('separation_wide', 'golden', 'main', l)
    l = Lst('Pitch: 0 and 255 (the clamp limits, s_sound.c:331-334, 341-344), 113 and 144 (the random range of most\n'
            'sounds, s_sound.c:339), 121 and 136 (the saw sounds, s_sound.c:329), 128 (itemup and tink, and every\n'
            'sound before the draw) and 135 and 166 (the linked chgun\'s 150, sounds.c:204). Pistol is 512 samples;\n'
            'at pitch 0 it plays 2048 frames.')
    for i, p in enumerate([0, 113, 121, 128, 135, 136, 144, 166, 255]): l.s(8 * i, PISTOL, 15, 128, p)
    add('pitch', 'golden', 'main', l)
    add('single_instance', 'golden', 'main', Lst(
        'The six single-instance sounds (i_sound.c:285-290). Every first copy plays at pitch 0 (2048 frames, four\n'
        'buffers), so it is still playing when it is restarted one or two tics later: the first copy is freed and\n'
        'the new one takes the first free channel. Also: sawup twice in one tic, sawidl while sawup plays (a\n'
        'different id: both play), oof twice (not single-instance: two channels) and chgun while a pistol plays\n'
        '(chgun is not in the list though it uses the pistol\'s data). Tic 31: sawup restarted while channel 0 is free\n'
        'and the first copy plays on channel 1: channel 1 is freed and the new copy takes channel 0.')
        .s(0, SAWUP, 60, 128, 0).s(0, SAWIDL, 60, 128, 0).s(1, SAWUP, 60, 64).s(4, SAWFUL, 60, 128, 0).s(6, SAWFUL, 60, 200)
        .s(8, SAWHIT, 60, 128, 0).s(9, SAWHIT, 60, 40).s(12, STNMOV, 60, 128, 0).s(14, STNMOV, 60, 90)
        .s(16, PISTOL, 60, 128, 0).s(17, PISTOL, 60, 160).s(17, CHGUN, 60, 128, 150).s(20, SAWUP, 70).s(20, SAWUP, 80)
        .s(24, OOF, 50).s(24, OOF, 50, 200).s(28, PLASMA, 50).s(28, SAWUP, 50, 128, 0).s(31, SAWUP, 50, 100))
    l = Lst('Ten starts in one tic: channels 0..7, then 0 and 0 (no channelstart is below the current gametic,\n'
            'so oldestnum stays 0, i_sound.c:309-325).')
    for i in range(10): l.s(0, 20 + i, 30)
    add('nine', 'golden', 'main', l)
    l = Lst('The oldest-channel rule (i_sound.c:309-325) with its ties; every start is shotgn at pitch 0 (4096 frames,\n'
            'so nothing ends). Tic 0: eight starts, channels 0..7, all channelstart 0. Tic 3: all busy, all started\n'
            'at 0: the lowest index, 0. Tic 4: channel 0 started at 3, the rest at 0: channel 1. Tic 5: channel 2.\n'
            'Tic 6: channels 3 and 4 (equal minima: the lowest index first, and the second start sees channel 3 at 6).\n'
            'Tic 7, passed with gametic 0: no channelstart is strictly below 0, so oldestnum stays 0: channel 0.')
    for i in range(8): l.s(0, SHOTGN, 20 + i, 128, 0)
    l.s(3, SHOTGN, 90, 128, 0).s(4, SHOTGN, 90, 128, 0).s(5, SHOTGN, 90, 128, 0).s(6, SHOTGN, 90, 128, 0).s(6, SHOTGN, 91, 128, 0)
    l.s(7, SHOTGN, 92, 128, 0, gametic=0)
    add('oldest', 'golden', 'main', l)
    add('boundary', 'golden', 'edge', Lst(
        'Buffer boundaries. Sawup (512 samples) started at tic 0 ends on the last frame of buffer 0; the tic-1\n'
        'start finds its channel free (a channel frees at the frame its data runs out, i_sound.c:606-607, not at\n'
        'a tic). Sawidl (513) started at tic 3: its samples end one frame into its second buffer and its padding\n'
        '(128) keeps the channel busy to the end of that buffer. A start at tic 9 lands in\n'
        'buffer 6 (mixed after tic 9); the tic-10 start in buffer 7 (after tic 11: no buffer after tic 10).')
        .s(0, SAWUP, 100).s(1, SGCOCK, 100).s(3, SAWIDL, 100).s(3, 4, 100).s(9, SAWUP, 100).s(10, SAWFUL, 100))
    l = Lst('Clipping (i_sound.c:617-630) with the all-255 (rlaunc) and all-0 (rxplod) lumps at volume 127. Tic 0: four of\n'
            'each, centred (the sums nearly cancel). Tic 2: eight all-255, full left: the left sum clips to 32767.\n'
            'Tic 4: eight all-0, full right: the right sum clips to -32768.')
    for i in range(8): l.s(0, RLAUNC if i < 4 else RXPLOD, 127, 128)
    for i in range(8): l.s(2, RLAUNC, 127, 0)
    for i in range(8): l.s(4, RXPLOD, 127, 255)
    add('clip', 'golden', 'edge', l)
    add('chgun', 'golden', 'main', Lst(
        'The linked chaingun (sounds.c:204): lengths[86] = lengths[0] = 0 (i_sound.c:811), so a start mixes exactly\n'
        'one frame, the pistol\'s first sample (i_sound.c:810), at pitch 150 and at pitch 128.')
        .s(0, CHGUN, 127, 128, 150).s(4, CHGUN, 127, 128, 128).s(8, CHGUN, 15, 33, 166))
    add('header', 'golden', 'edge', Lst(
        'Header fields getsfx never reads (i_sound.c:218-250): firsht says 22,050 Hz and plays one byte per frame at\n'
        '11,025; firxpl has a wrong sample count and plays its size - 8 bytes.')
        .s(0, FIRSHT, 127).s(4, FIRXPL, 127))
    add('padding', 'golden', 'edge', Lst(
        'Padding (i_sound.c:230-241): 1, 511, 512, 513 and 1024 samples become 512, 512, 512, 1024 and 1024; the\n'
        'padding is 128, which mixes to 0 but keeps the channel busy.')
        .s(0, PLASMA, 127).s(2, BFG, 127).s(5, SAWUP, 127).s(8, SAWIDL, 127).s(13, SAWFUL, 127))
    add('missing', 'golden', 'edge', Lst(
        'pstart is missing: getsfx gave it a copy of dspistol at load (i_sound.c:213-216), so a start plays the\n'
        'pistol\'s samples; the lookup s_sound.c makes first (I_GetSfxLumpNum, s_sound.c:364-365, i_sound.c:451-456)\n'
        'succeeds for the pistol and ends the run in I_Error for pstart (w_wad.c:399-409).')
        .s(0, PSTART, 127).s(2, PISTOL, 127).g(4, PISTOL).g(4, PSTART))
    add('handle_wrap', 'golden', 'main', Lst(
        'The handle counter (i_sound.c:271, 335-340): 65,436 starts in tic 0 take handles 100..65535; the unsigned\n'
        'short then wraps to 0 and the next start resets it to 100.')
        .n(65436, 0, SGCOCK).s(0, SGCOCK, 8, 128, 128, 64, 100).s(1, SGCOCK, 8, 128, 128, 64, 101))
    l = Lst('Starts at every tic 0..15, alternating pistol (single-instance) and sgcock, to show which starts share\n'
            'a buffer under the schedule (buffer k after tic floor(512k/315)).')
    for t in range(16): l.s(t, PISTOL if t % 2 == 0 else SGCOCK, 10 + t, 33 + 12 * t)
    add('phase', 'golden', 'main', l)
    l = Lst('A deterministic busy session: 300 starts over 400 tics with the sfx, volume (0..15), separation\n'
            '(33..224) and pitch (113..144, or 121..136 for the saws, 128 for itemup and tink, 135..166 for chgun)\n'
            'drawn from a fixed linear congruential sequence.', tics=400)
    x = 12345
    def rnd(m):
        nonlocal x
        x = (x * 1103515245 + 12345) & 0x7fffffff
        return (x >> 8) % m
    starts = sorted((rnd(400), k) for k in range(300))
    for t, _k in starts:
        sfx = 1 + rnd(108)
        base = 150 if sfx == CHGUN else 128
        p = base if sfx in (32, 87) else base + 8 - rnd(16) if SAWUP <= sfx <= SAWHIT else base + 16 - rnd(32)
        l.s(t, sfx, rnd(16), 33 + rnd(192), p, 64)
    add('busy', 'golden', 'main', l)
    for name, v, sp, why in (('x_vol128_left', 128, 0, 'leftvol out of bounds'), ('x_vol128_right', 128, 255, 'rightvol out of bounds'),
                             ('x_sep257', 127, 257, 'leftvol out of bounds'), ('x_sepm2', 127, -2, 'rightvol out of bounds')):
        add(name, 'golden', 'edge', Lst(f'Outside s_sound.c\'s domain (volume <= 127, separation 33..224): volume {v} at separation '
                                        f'{sp}\nends in I_Error "{why}" (i_sound.c:364-368).').s(0, SHOTGN, 60).s(2, SHOTGN, v, sp))
    for name, sfx, size in (('w_short0', DBOPN, 0), ('w_short5', DBCLS, 5), ('w_short8', DBLOAD, 8)):
        add(name, 'witness', 'edge', Lst(f'Refusal witness (decision T11), never a golden: a {size}-byte lump has padded length 0 '
                                         f'(i_sound.c:230), so\nits one mixed frame reads one byte past the 8-byte zone block '
                                         f'(i_sound.c:591 before the test at 606).').s(0, SHOTGN, 60).s(2, sfx, 127))
    add('p_pitch256', 'probe', 'edge', Lst('Declared probe outside the golden set: pitch 256 indexes steptable[256] '
                                           '(i_sound.c:495); s_sound.c\nclamps the pitch to 0..255 (s_sound.c:331-334, '
                                           '341-344), so no game start reaches it.').s(0, SHOTGN, 127, 128, 256),
        probe=True)
    return sc


# ---------------------------------------------------------------- rows
def buf_tic(k, schedule): return k if schedule == 'pertic' else (512 * k) // 315


def expand(text, schedule):
    """Rows with r rows expanded into b rows: a list of (key, line)."""
    out, prev = [], None
    for line in text.splitlines():
        f = line.split()
        if not f: continue
        if f[0] == 'r':
            if prev is None: raise ValueError('r row without a preceding b row')
            k0 = int(prev[1])
            for i in range(1, int(f[1]) + 1):
                k = k0 + i
                out.append((f'b {k}', f'b {k} {buf_tic(k, schedule)} {prev[3]} {prev[4]}'))
            if out[-1][0] != f'b {f[2]}' or out[-1][1].split()[2] != f[3]: raise ValueError(f'r row inconsistent: {line}')
            prev = out[-1][1].split()
            continue
        key = ' '.join(f[:3]) if f[0] in ('s', 'g', 'o', 'm', 'x') else ' '.join(f[:2]) if f[0] in ('b', 'T', 'l', 'P') else f[0]
        out.append((key, line))
        prev = f if f[0] == 'b' else None
    return out


def diff_rows(a, b, schedule):
    """The expanded rows that differ: list of (key_a, line_a, line_b) in order (positional comparison)."""
    ea, eb = expand(a, schedule), expand(b, schedule)
    out = []
    for i in range(max(len(ea), len(eb))):
        x = ea[i] if i < len(ea) else (None, '<end>')
        y = eb[i] if i < len(eb) else (None, '<end>')
        if x[1] != y[1]: out.append((x[0] or y[0], x[1], y[1]))
    return out


def strip_device(text): return ''.join(l + '\n' for l in text.splitlines() if not l.startswith(DEVICE_ROWS))


# ---------------------------------------------------------------- running
def wad_args(wad): return [x for w in WADS[wad] for x in ('--wad', str(ROOT / w))]


def run_driver(binary, sc, starts, schedule, work, extra=()):
    work.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=work) as d:
        o = Path(d) / 'rows.txt'
        cmd = [str(binary), *wad_args(sc['wad']), '--starts', str(starts), '--schedule', schedule, '--out', str(o), *extra]
        if sc['lumps']: cmd.append('--lumps')
        if sc['probe']: cmd += ['--probe', '--max-buffers', '64']
        p = subprocess.run(cmd, capture_output=True, text=True, env=ENV, cwd=d)
        return dict(code=p.returncode, rows=o.read_text() if o.exists() else '', stderr=p.stderr)


def run_model(sc, starts, schedule):
    cmd = [sys.executable, str(MODEL), *wad_args(sc['wad']), '--starts', str(starts), '--schedule', schedule]
    if sc['lumps']: cmd.append('--lumps')
    if sc['probe']: cmd += ['--probe', '--max-buffers', '64']
    p = subprocess.run(cmd, capture_output=True, text=True)
    return dict(code=p.returncode, rows=p.stdout, stderr=p.stderr)


def tables_check(binary, work):
    """Compare the genuine tables entry by entry with an integer-only reconstruction (section 3.2.6)."""
    work.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=work) as d:
        tf = Path(d) / 'tables.bin'
        p = subprocess.run([str(binary), *wad_args('main'), '--tables', str(tf), '--out', str(Path(d) / 'rows.txt')],
                           capture_output=True, text=True, env=ENV, cwd=d)
        if p.returncode: return None, [f'tables run failed: {p.stderr[-200:]}']
        data = tf.read_bytes()
    step = list(struct.unpack('<256i', data[:1024]))
    vol = list(struct.unpack('<32768i', data[1024:]))

    def root64(x):                         # floor(x ** (1/64)) for a non-negative integer, by bisection
        lo, hi = 0, 1
        while hi ** 64 <= x: hi *= 2
        while hi - lo > 1:
            mid = (lo + hi) // 2
            if mid ** 64 <= x: lo = mid
            else: hi = mid
        return lo

    def trunc_div(n, d):                   # C99 division: the quotient truncated toward zero
        q = abs(n) // abs(d)
        return -q if (n < 0) != (d < 0) else q

    want_step = [root64(1 << (1024 + p - 128)) for p in range(256)]       # floor(2^(16 + (p-128)/64))
    want_vol = [trunc_div(i * (j - 128) * 256, 127) for i in range(128) for j in range(256)]
    bad = []
    if step != want_step: bad.append(f'steptable: {sum(a != b for a, b in zip(step, want_step))} entries differ from the integer reconstruction')
    if vol != want_vol: bad.append(f'vol_lookup: {sum(a != b for a, b in zip(vol, want_vol))} entries differ from the truncating division')
    # distance of each non-power-of-two entry's exact value to the nearest integer, in billionths (integer arithmetic,
    # within one billionth)
    dist = []
    for p in range(256):
        if (p - 128) % 64 == 0: continue
        nano = root64((1 << (1024 + p - 128)) * 10 ** (9 * 64))            # floor(2^(16+(p-128)/64) * 10^9)
        frac = nano % 10 ** 9
        dist.append((min(frac, 10 ** 9 - frac), p))
    md = min(dist)
    rec = dict(steptable_sha256=sha(data[:1024]), vol_lookup_sha256=sha(data[1024:]),
               steptable_integer_reconstruction='equal' if step == want_step else 'DIFFERS',
               vol_lookup_truncating_division='equal' if vol == want_vol else 'DIFFERS',
               steptable_min=min(step), steptable_max=max(step), steptable_exact_powers_of_two=4,
               steptable_min_distance_to_integer_billionths=md[0], steptable_min_distance_at_pitch=md[1],
               vol_lookup_min=min(vol), vol_lookup_max=max(vol))
    return rec, bad


def original_site(site):
    """FILE:LINE:COL of a compiled (adapted) file mapped back to the genuine file's line: the errno adapter adds one
    line before i_sound.c:27, so a report at adapted line 496 is genuine line 495."""
    f, line, col = site.rsplit(':', 2)
    line, off = int(line), 0
    for n, k in sorted((x['line'], x['adapted'].count('\n') - x['original'].count('\n'))
                       for x in mb.adapters_for('strict') if x['file'] == f):
        at = n + off
        if line < at: break
        if line <= at + k: return f'{f}:{n}:{col}'
        off += k
    return f'{f}:{line - off}:{col}'


def first_o(rows):
    for line in rows.splitlines():
        if line.startswith('o '): return line.split()[1:3]
    return None


def witness_check(name, runs, model):
    """A witness passes when every contract build flags the same start with an o row and the model refuses it."""
    bad, flagged = [], {}
    for v, r in runs.items():
        o = first_o(r['rows'])
        flagged[v] = o
        if r['code'] or SANITIZER.search(r['stderr']): bad.append(f'{name} {v}: exit {r["code"]} or a sanitizer report')
    sites = {tuple(o) if o else None for o in flagged.values()}
    if len(sites) != 1 or None in sites: bad.append(f'{name}: not flagged at one start in every build: {flagged}')
    site = next(iter(sites)) if len(sites) == 1 else None
    mx = [l for l in model['rows'].splitlines() if l.startswith('x ')]
    refused = bool(site) and len(mx) == 1 and mx[0].split()[1:3] == list(site) and 'refused' in mx[0]
    if not refused: bad.append(f'{name}: the model does not refuse the flagged start ({mx})')
    # every row before the flagged start's echo is the same in the model and the genuine strict build
    gs = strip_device(runs['strict']['rows']).splitlines()
    ms = model['rows'].splitlines()
    cut = next((i for i, l in enumerate(gs) if site and l.startswith(f's {site[0]} {site[1]} ')), None)
    if cut is None or gs[:cut] != ms[:cut]: bad.append(f'{name}: the model and the genuine rows differ before the flagged start')
    digests = {v: r['rows'].splitlines()[-1] for v, r in runs.items()}
    return dict(flagged_start=[int(x) for x in site] if site else None, o_row_in=sorted(v for v, o in flagged.items() if o),
                model_refusal=mx[0] if mx else None,
                stream_digest={v: d.split()[2] if d.startswith('z ') else None for v, d in digests.items()}), bad


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5c-mixer-reference')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--strict32', action='store_true')
    ap.add_argument('--raw', action='store_true')
    ap.add_argument('--only')
    ap.add_argument('--sessions', type=Path, metavar='DIR')
    ap.add_argument('--jobs', type=int, default=4)
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    jobs = max(1, a.jobs)
    sc = scenarios()
    # the start lists and the contract
    want = {GROUPS[s['group']] / f'{n}.txt': s['text'] for n, s in sc.items()}
    if a.check or a.only:
        have = {p: p.read_text() for g in GROUPS.values() for p in g.glob('*.txt')}
        if have != want:
            bad = sorted(str(p.relative_to(ROOT)) for p in set(have) | set(want) if have.get(p) != want.get(p))
            sys.exit(f'the committed start lists differ from this tool\'s scenarios: {bad[:10]}')
    else:
        for g in GROUPS.values(): g.mkdir(parents=True, exist_ok=True)
        for p, t in want.items(): p.write_text(t)
    if not mb.CONTRACT.exists() or mb.CONTRACT.read_text() != mb.render_json(mb.contract()):
        sys.exit('tests/reference/m5c_mixer_contract.json differs from the regenerated contract (m5c_mixer_build.py --check)')
    names = [n for n in sc if not a.only or n in a.only.split(',')]
    if a.only and len(names) != len(a.only.split(',')): sys.exit(f'unknown scenario in --only: {a.only}')
    variants = VARIANTS + (['strict32'] if a.strict32 else [])
    so, sys.stdout = sys.stdout, open(os.devnull, 'w')
    try: bins = {v: mb.build(v, work / 'build', a.cc, quiet=True) for v in variants}
    finally: sys.stdout.close(); sys.stdout = so
    if a.strict32 and bins['strict32'] is None: sys.exit('strict32 requested but -m32 is not available')
    if a.sessions:
        fails = sessions(a.sessions.resolve(), bins, variants, work, jobs)
        if fails: sys.exit('sessions FAILED:\n  ' + '\n  '.join(fails))
        return
    path = {n: GROUPS[sc[n]['group']] / f'{n}.txt' for n in names}
    jobs_list = []
    for n in names:
        g = sc[n]['group']
        if g == 'probe':
            jobs_list.append((n, 'address', GOLDEN_SCHEDULE))
            continue
        for v in variants:
            for s in (SCHEDULES if g == 'golden' else [GOLDEN_SCHEDULE]): jobs_list.append((n, v, s))
        for s in (SCHEDULES if g == 'golden' else [GOLDEN_SCHEDULE]): jobs_list.append((n, 'model', s))
    raw_dir = work / 'raw'
    if a.raw: raw_dir.mkdir(parents=True, exist_ok=True)

    def job(j):
        n, v, s = j
        if v == 'model': return j, run_model(sc[n], path[n], s)
        extra = ['--raw', str(raw_dir / f'{n}.{s}.raw')] if a.raw and v == 'strict' else []
        return j, run_driver(bins[v], sc[n], path[n], s, work / 'runs', extra)
    with cf.ThreadPoolExecutor(jobs) as pool:
        res = dict(pool.map(job, jobs_list))
    failures, findings = [], []
    per, wit, probes, pertic = {}, {}, {}, {}
    goldens = {}
    for n in names:
        g = sc[n]['group']
        if g == 'probe':
            r = res[(n, 'address', GOLDEN_SCHEDULE)]
            rep = re.search(r'([\w.]+:\d+:\d+): runtime error: ([^\n]*)', r['stderr'])
            site = original_site(rep.group(1)) if rep else None
            ok = r['code'] != 0 and rep is not None and site.startswith('i_sound.c:495:')
            probes[n] = dict(variant='address', exit=r['code'], report=f'{site}: {rep.group(2)}' if rep else None,
                             compiled_site=rep.group(1) if rep else None, as_expected=ok)
            if rep: findings.append(dict(scenario=n, variant='address', site=site, compiled_site=rep.group(1),
                                         message=rep.group(2), in_s_sound_domain=False))
            print(f'probe {n}: address exit {r["code"]}, {probes[n]["report"]}')
            if not ok: failures.append(f'probe {n}: no report at i_sound.c:495')
            continue
        for v in variants:
            for s in (SCHEDULES if g == 'golden' else [GOLDEN_SCHEDULE]):
                r = res[(n, v, s)]
                for m in re.finditer(r'([\w.]+:\d+:\d+): runtime error: ([^\n]*)', r['stderr']):
                    findings.append(dict(scenario=n, variant=v, site=original_site(m.group(1)), compiled_site=m.group(1),
                                         message=m.group(2), in_s_sound_domain=True))
                if r['code'] or SANITIZER.search(r['stderr']):
                    failures.append(f'{n} {v} {s}: exit {r["code"]}: {(r["stderr"].strip().splitlines() or [""])[-1][:200]}')
        if g == 'witness':
            rec, bad = witness_check(n, {v: res[(n, v, GOLDEN_SCHEDULE)] for v in VARIANTS}, res[(n, 'model', GOLDEN_SCHEDULE)])
            wit[n] = rec
            failures += bad
            print(f"witness {n}: flagged at {rec['flagged_start']} in {', '.join(rec['o_row_in'])}; model: {rec['model_refusal']}; "
                  f"{len(set(rec['stream_digest'].values()))} distinct stream digest(s) over the {len(VARIANTS)} contract builds")
            continue
        for s in SCHEDULES:
            ref = res[(n, 'strict', s)]['rows']
            for v in variants[1:]:
                if res[(n, v, s)]['rows'] != ref:
                    d = diff_rows(ref, res[(n, v, s)]['rows'], s)
                    failures.append(f'{n} {s}: {v} differs from strict, first at {d[0] if d else "?"}')
            mrows = res[(n, 'model', s)]
            if mrows['code'] or strip_device(ref) != mrows['rows']:
                d = diff_rows(strip_device(ref), mrows['rows'], s)
                failures.append(f'{n} {s}: the model differs from the genuine code, first at {d[0] if d else mrows["stderr"][-200:]}')
            if re.search(r'^m ', ref, re.M): failures.append(f'{n} {s}: a start list handle differs from the genuine handle')
        rows = res[(n, 'strict', GOLDEN_SCHEDULE)]['rows']
        goldens[n] = rows
        z = rows.splitlines()[-1].split()
        pz = res[(n, 'strict', 'pertic')]['rows'].splitlines()[-1].split()
        lines = rows.splitlines()
        per[n] = dict(wad=WADS[sc[n]['wad']][0].split('/')[-1], starts=sum(int(l.split()[1]) if l.startswith('n ') else 1
                                                                      for l in sc[n]['text'].splitlines() if l[:2] in ('s ', 'n ')),
                      buffers=int(z[1]), stream_sha256=z[2], golden_sha256=sha(rows), golden_bytes=len(rows.encode()),
                      end=next((' '.join(l.split()[:3]) for l in lines if l.startswith('x ')), 'drained'),
                      model='identical')
        pertic[n] = dict(buffers=int(pz[1]), stream_sha256=pz[2], rows_sha256=sha(res[(n, 'strict', 'pertic')]['rows']),
                         model='identical')
    tab, bad = tables_check(bins['strict'], work / 'runs')
    failures += bad
    if tab:
        print(f"tables: steptable {tab['steptable_integer_reconstruction']} to the integer reconstruction, vol_lookup "
              f"{tab['vol_lookup_truncating_division']} to the truncating division; minimum distance to an integer "
              f"{tab['steptable_min_distance_to_integer_billionths']} billionths at pitch {tab['steptable_min_distance_at_pitch']}")
        for n, rows in goldens.items():
            t = [l.split()[2] for l in rows.splitlines() if l.startswith('T ')]
            if t and t != [tab['steptable_sha256'], tab['vol_lookup_sha256']]: failures.append(f'{n}: T rows differ from the table dump')
    print(f"goldened scenarios: {len(goldens)}; strict, overflow, address identical under both schedules; model identical: "
          f"{sum(1 for n in goldens if not any(f.startswith(n + ' ') for f in failures))}")
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    if a.only:
        print('--only: a subset was run; nothing compared with the committed goldens and nothing written')
    else:
        doc = dict(schema=SCHEMA, contract_sha256=sha(mb.CONTRACT.read_bytes()),
                   switches=dict(configuration='integrated', schedule=GOLDEN_SCHEDULE, frames_per_buffer=512,
                                 frames_per_tic=315, rule=mb.SCHEDULES[GOLDEN_SCHEDULE],
                                 also_recorded='pertic (digests only, never goldens)'),
                   variants=VARIANTS, asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
                   scenarios=len(goldens), starts=sum(p['starts'] for p in per.values()),
                   buffers=sum(p['buffers'] for p in per.values()), golden_bytes=sum(p['golden_bytes'] for p in per.values()),
                   goldens_sha256=sha(''.join(f'{n}\n{g}' for n, g in sorted(goldens.items()))),
                   tables=tab, per_scenario=per, pertic=pertic,
                   refusal_witnesses_T11=dict(note='never goldened: the genuine code reads one byte past an 8-byte zone '
                                                   'block (i_sound.c:230-247, 591, 606); AddressSanitizer cannot see it '
                                                   'because the read stays inside the zone\'s single malloc. The three contract builds (LP64) agree; the opt-in '
                                                   'strict32 build reads a different byte (m5c_mixer_reference.py --strict32, '
                                                   'local only), as Phase 1 observed (section 3.2.5 row 3)', **wit),
                   sanitizer_findings=findings, probes=probes)
        text = json.dumps(doc, indent=1) + '\n'
        files = {RESULTS: text, **{GOLDENS / f'{n}.golden': g for n, g in goldens.items()}}
        if a.check:
            bad = [str(p.relative_to(ROOT)) for p, t in files.items() if not p.exists() or p.read_text() != t]
            extra = [str(p.relative_to(ROOT)) for p in GOLDENS.glob('*') if p not in files]
            if bad or extra:
                for p in bad[:5]:
                    n = Path(p).stem
                    if n in goldens and (ROOT / p).exists():
                        d = diff_rows((ROOT / p).read_text(), goldens[n], GOLDEN_SCHEDULE)
                        print(f'  {n}: first difference {d[0] if d else None}')
                sys.exit(f'differs: {bad[:20]} ({len(bad)}); not produced by this run: {extra}')
            print(f'{RESULTS.relative_to(ROOT)} and {len(goldens)} goldens match ({doc["buffers"]} buffers, {doc["starts"]} starts)')
        else:
            GOLDENS.mkdir(parents=True, exist_ok=True)
            for p, t in files.items(): p.write_text(t)
            print(f'wrote {RESULTS.relative_to(ROOT)} and {len(goldens)} goldens ({doc["buffers"]} buffers, {doc["starts"]} starts)')
    if a.strict32:
        same = [n for n in goldens if res[(n, 'strict32', GOLDEN_SCHEDULE)]['rows'] == goldens[n]
                and res[(n, 'strict32', 'pertic')]['rows'] == res[(n, 'strict', 'pertic')]['rows']]
        print(f'strict32 vs strict: {len(same)} of {len(goldens)} goldened scenarios identical under both schedules')
        for n, w in wit.items():
            r32 = res[(n, 'strict32', GOLDEN_SCHEDULE)]
            z32 = r32['rows'].splitlines()[-1].split()[2]
            o32 = first_o(r32['rows'])
            print(f"strict32 witness {n}: o row at {o32}; stream digest {'differs from' if z32 != w['stream_digest']['strict'] else 'equals'} "
                  f"the LP64 builds' (the byte read past the block depends on the memory layout)")
            if [int(x) for x in o32 or []] != w['flagged_start']: sys.exit(f'strict32 witness {n}: not flagged at the same start')
    if a.selftest:
        fails = selftest(bins['strict'], sc, work)
        if fails: sys.exit('selftest FAILED: ' + ', '.join(fails))


def sessions(src, bins, variants, work, jobs):
    """Every DOOM-M5C-SOUND-v1 stream (*.sound) in SRC through tools/reference/m5c_starts.py and then the genuine mixer
    (every variant, both schedules) and the model, on m5c_sfx.wad. Writes nothing in the tree; prints one line per stream."""
    import m5c_starts
    lists = work / 'session_lists'
    lists.mkdir(parents=True, exist_ok=True)
    sc, fails = {}, []
    for f in sorted(src.glob('*.sound')):
        try: text = m5c_starts.convert(f.read_text(), f.name)
        except m5c_starts.Refused as e: fails.append(f'{f.name}: refused: {e}'); continue
        (lists / f'{f.stem}.txt').write_text(text)
        sc[f.stem] = dict(group='golden', wad='main', lumps=False, probe=False, text=text)
    todo = [(n, v, s) for n in sc for s in SCHEDULES for v in variants + ['model']]

    def job(j):
        n, v, s = j
        if v == 'model': return j, run_model(sc[n], lists / f'{n}.txt', s)
        return j, run_driver(bins[v], sc[n], lists / f'{n}.txt', s, work / 'runs')
    with cf.ThreadPoolExecutor(jobs) as pool:
        res = dict(pool.map(job, todo))
    for n in sc:
        line = []
        for s in SCHEDULES:
            ref = res[(n, 'strict', s)]
            bad = [v for v in variants if res[(n, v, s)]['code'] or SANITIZER.search(res[(n, v, s)]['stderr'])
                   or res[(n, v, s)]['rows'] != ref['rows']]
            model_ok = res[(n, 'model', s)]['code'] == 0 and res[(n, 'model', s)]['rows'] == strip_device(ref['rows'])
            m = len(re.findall(r'^m ', ref['rows'], re.M))
            z = ref['rows'].splitlines()[-1].split()
            starts = sum(1 for l in sc[n]['text'].splitlines() if l.startswith('s '))
            line.append(f'{s}: {z[1]} buffers, stream {z[2][:16]}, rows {sha(ref["rows"])[:16]}')
            if bad: fails.append(f'{n} {s}: {bad} differ from strict or failed')
            if not model_ok: fails.append(f'{n} {s}: the model differs')
            if m: fails.append(f'{n} {s}: {m} handles differ from the sound-logic run')
        print(f'session {n}: {starts} starts; ' + '; '.join(line) + f'; variants and model identical: {"yes" if not any(x.startswith(n + " ") for x in fails) else "NO"}')
    return fails


def selftest(binary, sc, work):
    fails = []
    committed = {p.stem: p.read_text() for p in GOLDENS.glob('*.golden')}

    def run_text(name, text, extra=()):
        work.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile('w', dir=work, suffix='.txt', delete=False) as f: f.write(text)
        try: return run_driver(binary, sc[name], Path(f.name), GOLDEN_SCHEDULE, work / 'runs', extra)['rows']
        finally: os.unlink(f.name)
    # 1. one flipped stream bit: byte 5000 lies in buffer 2
    name, byte = 'basic', 5000
    d = diff_rows(committed[name], run_text(name, sc[name]['text'], ['--perturb-byte', str(byte)]), GOLDEN_SCHEDULE)
    keys = [k for k, _x, _y in d]
    ok = keys == [f'b {byte // 2048}', 'z']
    print(f'selftest flipped stream bit: {name} byte {byte}; differing rows {keys}: {"pass" if ok else "FAIL"}')
    if not ok: fails.append('flipped stream bit')
    # 2. one start moved by a tic (the tic-3 shotgn to tic 4)
    old = 's 3 0 3 2 8 64 120 64 101\ns 3 1 3 31 15 200 140 78 102\n'
    moved = sc[name]['text'].replace(old, 's 3 0 3 31 15 200 140 78 -\ns 4 0 4 2 8 64 120 64 -\n')
    d = diff_rows(committed[name], run_text(name, moved), GOLDEN_SCHEDULE)
    ok = bool(d) and d[0][0] == 's 3 0' and any(k.startswith('b ') for k, _x, _y in d) and moved != sc[name]['text']
    print(f'selftest moved start: {name} shotgn from tic 3 to tic 4; first difference {d[0][0] if d else None}, '
          f'{sum(1 for k, _x, _y in d if k.startswith("b "))} buffer rows differ: {"pass" if ok else "FAIL"}')
    if not ok: fails.append('moved start')
    # 3. a changed volume on a start replaced before any buffer is mixed (the tic-10 pistol)
    changed = sc[name]['text'].replace('s 10 0 10 1 127 128', 's 10 0 10 1 126 128')
    d = diff_rows(committed[name], run_text(name, changed), GOLDEN_SCHEDULE)
    keys = [k for k, _x, _y in d]
    ok = keys == ['s 10 0'] and changed != sc[name]['text']
    print(f'selftest unheard volume: {name} tic-10 pistol volume 127 -> 126; differing rows {keys}: {"pass" if ok else "FAIL"}')
    if not ok: fails.append('unheard volume')
    return fails


if __name__ == '__main__':
    main()
