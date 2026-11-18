#!/usr/bin/env python3
"""Genuine sound-logic and full-frame reference runs of the M5c runner on every synthetic item.

Expected sound streams, frames, palette numbers, M_Random indices and messages come from the genuine
linuxdoom-1.10 code only (docs/M5C_REFERENCE_INVESTIGATION.md sections 3.1 and 3.4): s_sound.c and
f_wipe.c compiled whole and unmodified, i_sound.c whole with two declared errno lines, inside the
M5b runner fork tools/reference/m5c_build.py (contract tests/reference/m5c_runner_contract.json).
Never from Rust output and never from the independent Python model below (a second check only).

Items: every item of tools/reference/m5b_reference.py (the M5a rooms, the M4b scripts, the M5b rooms
and the bespoke CHEATS), plus the rooms of tests/reference/m5c_manifest.json (tools/reference/m5c_wad.py).
Every run loads the item WAD, tests/reference/m5b_gfx.wad, tests/reference/m5c_sfx.wad,
tests/reference/m5c_music.wad and the demo lump. The switches of the committed goldens are the
recommended values RECOMMENDED (integrated handles, S_UpdateSounds after every tic, the genuine wipe,
sound on); they are recorded in the results file, and ALTERNATES are run and recorded as digests and
counts only. Per item:

  never     strict, --render none, dumps: the masked game-dump set (the S16 mask now subtracts every
            presentation draw: sound, status bar, melt) must equal the frozen M5b golden
            (tests/reference/m5b_results.json dumps_never_sha256) or, for an M5c room, the frozen M5b
            runner's dump set on the same files;
  fork      strict, --render every --sound off --wipe none, at screenblocks 10 and 11: the golden rows
            (m5b_reference.normalise) must equal the frozen tests/reference/m5b_frames golden, or for an
            M5c room the frozen M5b runner's stream on the same files: the proof the fork is faithful;
  every     the recommended switches at screenblocks 10 and 11 in the strict, overflow and address
            builds and a second strict run: render-every dump sets equal the never-render set snapshot
            for snapshot; frame streams and sound streams identical across the variants through the
            earliest end and run to run;
  face      against the fork run (Q-row check of section 3.4): on every frame without a tracker flag,
            the digest with the face rectangle zeroed is equal; at screenblocks 11 the whole frame is
            equal; every P column but the face and rndindex is equal;
  screens   the sound stream is the same at screenblocks 10 and 11 (through the earliest end);
  alternates sndserv handles, the buffer schedule and wipe none (strict, screenblocks 10 and 11):
            sound-stream and frame-stream digests and counts, recorded only;
  model     every positional start's volume, separation and audibility recomputed independently from
            the S row's positions (a port of S_AdjustSoundParams and R_PointToAngle2 with the tables.c
            tantoangle and finesine tables) must equal the genuine values; mismatch, unknown origins and
            other draws must be 0.

Goldens (written without --check, compared with --check): tests/reference/m5c_sound/<LUMP>.sound
(DOOM-M5C-SOUND-v1, the strict screenblocks-10 stream), tests/reference/m5c_frames/<LUMP>.sb<N>.frames
(DOOM-M5C-FRAMES-v1: the M5b golden rows plus Q and V; tracker-flagged frames lose their full and
face-zeroed digests, "undefined"), tests/reference/m5c_results.json (DOOM-M5C-RESULTS-v1) and the
coverage manifest tests/reference/m5c_coverage.json (DOOM-M5C-COVERAGE-v1: which sfx and which branches
each item reaches, measured from the streams; REQUIRED_SFX and REQUIRED_BRANCHES must all be reached, so
a change that loses coverage fails --check; the unreachable ones are listed with their citations).

--selftest plants faults: a started sound's volume (--selftest-sound: exactly one S row must differ
from the golden), the S16 mask switched off (--no-mask-rnd: the dump set must differ from the frozen
digest), a status bar pixel outside the face (--selftest-bar: the face check must report exactly that
tic) and a coverage loss (one item's streams removed: the required-set check must fail).
--strict32 (local opt-in, needs a -m32 toolchain) runs the strict32 build at screenblocks 10 and compares
its sound and frame streams with the strict ones, reporting every differing row (the button-release
origins of decision T12 in particular). Address runs use ASAN_OPTIONS=detect_leaks=0: leak detection is
disabled, so they are not LeakSanitizer coverage.

    m5c_reference.py [--work DIR] [--check] [--selftest] [--strict32] [--only NAME[,NAME]] [--jobs N] [--cc CC]
"""
import argparse, collections, concurrent.futures as cf, hashlib, json, os, re, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5c_build             # noqa: E402
import m5b_build             # noqa: E402
import m3h9c_dump as dump    # noqa: E402
import m5b_reference as B    # noqa: E402  (read-only: items, stream helpers)
import m5c_sfx               # noqa: E402  (read-only: the sfx table parsed from sounds.c)

GFX = ROOT / 'tests/reference/m5b_gfx.wad'
SFX = ROOT / 'tests/reference/m5c_sfx.wad'
MUSIC = ROOT / 'tests/reference/m5c_music.wad'
M5B_RESULTS = ROOT / 'tests/reference/m5b_results.json'
M5B_FRAMES = ROOT / 'tests/reference/m5b_frames'
MANIFEST = ROOT / 'tests/reference/m5c_manifest.json'
RESULTS = ROOT / 'tests/reference/m5c_results.json'
COVERAGE = ROOT / 'tests/reference/m5c_coverage.json'
FRAMES_DIR = ROOT / 'tests/reference/m5c_frames'
SOUND_DIR = ROOT / 'tests/reference/m5c_sound'
SCHEMA = 'DOOM-M5C-RESULTS-v1'
COVERAGE_SCHEMA = 'DOOM-M5C-COVERAGE-v1'
MANIFEST_SCHEMA = 'DOOM-M5C-MANIFEST-v1'
VARIANTS = ['strict', 'overflow', 'address']
VIEWS = [10, 11]
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
RECOMMENDED = {'--handle': 'integrated', '--update-schedule': 'tic', '--wipe': 'genuine', '--sound': 'on'}
ALTERNATES = {'sndserv': ['--handle', 'sndserv'], 'buffer': ['--update-schedule', 'buffer'], 'wipe_none': ['--wipe', 'none']}
GOLDEN_KINDS = B.GOLDEN_KINDS + ('Q', 'V')
SOUND_FILES = ('s_sound.c', 'i_sound.c', 'f_wipe.c')
sha = lambda b: hashlib.sha256(b).hexdigest()

# ---- the independent model of S_AdjustSoundParams (s_sound.c:752-818) and R_PointToAngle2 (r_main.c:292-385)
FRACBITS, CLIP, CLOSE = 16, 1200 << 16, 160 << 16
ATT = (CLIP - CLOSE) >> FRACBITS
ANG90, ANG180, ANG270 = 0x40000000, 0x80000000, 0xc0000000


def _table(name, n):
    src = (ROOT / 'linuxdoom-1.10/tables.c').read_text(encoding='latin-1')
    body = src[src.index(f'{name}[{n}] ='):]
    vals = [int(v, 0) for v in re.findall(r'-?(?:0x[0-9a-fA-F]+|\d+)', body[body.index('{'):body.index('};')])]
    if len(vals) != n: sys.exit(f'tables.c: {name} has {len(vals)} values, not {n}')
    return vals


TANTOANGLE, FINESINE = _table('tantoangle', 2049), _table('finesine', 10240)
s32 = lambda v: ((v + 0x80000000) & 0xffffffff) - 0x80000000
u32 = lambda v: v & 0xffffffff


def slope_div(num, den):
    num, den = u32(num), u32(den)
    if den < 512: return 2048
    ans = u32(num << 3) // (den >> 8)
    return ans if ans <= 2048 else 2048


def point_to_angle2(x1, y1, x2, y2):
    x, y = s32(x2 - x1), s32(y2 - y1)
    if not x and not y: return 0
    if x >= 0:
        if y >= 0: return TANTOANGLE[slope_div(y, x)] if x > y else u32(ANG90 - 1 - TANTOANGLE[slope_div(x, y)])
        y = -y
        return u32(-TANTOANGLE[slope_div(y, x)]) if x > y else u32(ANG270 + TANTOANGLE[slope_div(x, y)])
    x = -x
    if y >= 0: return u32(ANG180 - 1 - TANTOANGLE[slope_div(y, x)]) if x > y else u32(ANG90 + TANTOANGLE[slope_div(x, y)])
    y = -y
    return u32(ANG180 + TANTOANGLE[slope_div(y, x)]) if x > y else u32(ANG270 - 1 - TANTOANGLE[slope_div(x, y)])


def cdiv(a, b):
    q = abs(a) // abs(b)
    return q if (a >= 0) == (b > 0) else -q


def adjust(lx, ly, langle, ox, oy, gamemap, sfx_volume=8):
    """(rc, vol, sep, dist, angle_branch) exactly as S_AdjustSoundParams computes them."""
    adx, ady = abs(s32(lx - ox)), abs(s32(ly - oy))
    dist = adx + ady - (min(adx, ady) >> 1)
    if gamemap != 8 and dist > CLIP: return 0, sfx_volume, None, dist, None
    angle = point_to_angle2(lx, ly, ox, oy)
    branch = 'gt' if angle > langle else ('eq' if angle == langle else 'lt')
    angle = u32(angle - langle) if angle > langle else u32(angle + (0xffffffff - langle))
    fs = FINESINE[angle >> 19]
    sep = 128 - (((96 << 16) * fs) >> 16 >> FRACBITS)
    if dist < CLOSE: vol = sfx_volume
    elif gamemap == 8:
        d = min(dist, CLIP)
        vol = 15 + cdiv((sfx_volume - 15) * ((CLIP - d) >> FRACBITS), ATT)
    else: vol = cdiv(sfx_volume * ((CLIP - dist) >> FRACBITS), ATT)
    return int(vol > 0), vol, sep, dist, branch


# ---- coverage: the Episode 1 sound set and the branches level play can reach
SFX_NAMES = {i: n for i, n, _l in m5c_sfx.sfx_table()}
SFX_IDS = {n: i for i, n in SFX_NAMES.items()}
EPISODE1_SFX = ('pistol shotgn sawup sawidl sawful sawhit rlaunc firsht firxpl pstart pstop doropn dorcls stnmov swtchn swtchx '
                'plpain dmpain popain slop itemup wpnup oof telept posit1 posit2 posit3 bgsit1 bgsit2 sgtsit brssit sgtatk claw '
                'pldeth podth1 podth2 podth3 bgdth1 bgdth2 sgtdth brsdth posact bgact dmact noway barexp punch getpow bdopn').split()
UNREACHABLE_SFX = {
    'swtchx': 'p_switch.c:211 zeroes line->special for a non-repeatable switch before p_switch.c:221 tests it for 11; the exit '
              'switch (special 11) is non-repeatable (P_ChangeSwitchTexture (line, 0), p_switch.c:543-544 via P_UseSpecialLine), '
              'so sfx_swtchx is never started by this source',
    'bdopn': 'not reachable under wire 24: sfx_bdopn is started only for a blazeRaise or blazeOpen door (p_doors.c:320-328, '
             '442-445), which only the line specials 99, 105-118 and 133-137 create (p_spec.c:726-756, 916-931; '
             'p_switch.c:463-503, 600-634), and none of them is admitted by the wire-24 simulation (docs/COMPATIBILITY.md, '
             'admitted line specials); no room may use one, because the simulation refuses it at load',
}
REQUIRED_SFX = [n for n in EPISODE1_SFX if n not in UNREACHABLE_SFX]
REQUIRED_BRANCHES = [
    'origin:none', 'origin:player', 'origin:mobj', 'origin:sector', 'origin:button',
    'outcome:started', 'outcome:inaudible', 'outcome:no_channel', 'inaudible:clip', 'inaudible:vol0', 'samexy',
    'pitch:saw', 'pitch:nodraw', 'pitch:draw',
    'dist:clip_exact', 'dist:clip_minus', 'dist:clip_plus', 'dist:close_below', 'dist:close_exact', 'dist:close_plus',
    'angle:gt', 'angle:lt', 'angle:eq', 'sep:33', 'sep:224',
    'vol:1', 'vol:2', 'vol:3', 'vol:4', 'vol:5', 'vol:6', 'vol:7', 'vol:8', 'map8:far', 'map8:near',
    'getchannel:free', 'getchannel:evict', 'getchannel:refuse',
    'stop:same_origin', 'stop:same_origin_null', 'stopsound:found', 'stopsound:none',
    'stopchannel:playing', 'stopchannel:not_playing', 'update:finished', 'update:inaudible', 'update:params', 'update:local',
    'lump:first', 'button:press_null', 'button:press_slot0', 'button:release_audible', 'button:release_inaudible',
    'button:release_next_busy', 'starts:9_in_4_tics',
]
UNREACHABLE_BRANCHES = {
    'outcome:link_volume': 'sfx_chgun, the only linked sfx (sounds.c:204), is never started by any source file; the chaingun plays '
                           'sfx_pistol (p_pspr.c:737)',
    'getchannel:same_origin': 's_sound.c:837-841: S_StopSound (origin) at s_sound.c:349 has already stopped the one channel with '
                              'this origin (at most one exists, by induction: every start first stops it), and a NULL origin is '
                              'excluded by the "origin &&" test',
    'pitch:clamp': 's_sound.c:331-345: the pitch is 128 plus at most 16 (saw: 8) either way, never outside 0..255',
    'start:bad_sfx': 's_sound.c:277-278: every game call passes an sfx id from sounds.h',
    'update:link': 's_sound.c:567-580: only for a linked sfx, never started',
    'level_start:stop': 's_sound.c:209-211: S_Start runs once per process here, at the level load, before any start',
}


def rows_of(text, kind):
    return [l.split(',') for l in text.splitlines() if l.startswith(kind + ',')]


def level_of(text):
    for l in text.splitlines():
        if l.startswith('#level,'):
            f = l.split(','); return int(f[2]), int(f[4])
    return None, None


def sound_facts(text):
    """Counts, model check and coverage of one sound stream."""
    _ep, gamemap = level_of(text)
    S, T, K, C, L, R = (rows_of(text, k) for k in 'STKCLR')
    z = rows_of(text, 'Z'); z = z[0][1:] if z else []
    Z = {z[i]: int(z[i + 1]) for i in range(0, len(z), 2)}
    br, sfx_started, sfx_attempted = collections.Counter(), collections.Counter(), collections.Counter()
    model_bad, model_checked, vols, map8vols, seps = [], 0, collections.Counter(), collections.Counter(), collections.Counter()
    for s in S:
        sfx, okind, outcome = int(s[3]), s[4], s[6]
        sfx_attempted[SFX_NAMES[sfx]] += 1
        br['origin:' + okind] += 1
        br['outcome:' + outcome] += 1
        if outcome == 'started': sfx_started[SFX_NAMES[sfx]] += 1
        if s[15] == '1': br['samexy'] += 1
        if outcome in ('started', 'no_channel'):
            br['pitch:saw' if 10 <= sfx <= 13 else ('pitch:nodraw' if s[7] == '0' else 'pitch:draw')] += 1
        if okind in ('mobj', 'sector', 'button'):
            lx, ly, la, ox, oy = int(s[19]), int(s[20]), int(s[21]), int(s[17]), int(s[18])
            rc, vol, sep, dist, ab = adjust(lx, ly, la, ox, oy, gamemap)
            model_checked += 1
            if s[15] == '1': sep = 128
            got_rc = 0 if outcome == 'inaudible' else 1
            ok = rc == got_rc and (outcome != 'inaudible' or int(s[9]) == vol) and \
                (outcome != 'started' or (int(s[9]) == vol and int(s[10]) == sep))
            if not ok: model_bad.append(','.join(s))
            if dist == CLIP: br['dist:clip_exact'] += 1
            if dist == CLIP - 32768: br['dist:clip_minus'] += 1
            if dist == CLIP + 32768: br['dist:clip_plus'] += 1
            if dist < CLOSE: br['dist:close_below'] += 1
            if dist == CLOSE: br['dist:close_exact'] += 1
            if dist == CLOSE + 32768: br['dist:close_plus'] += 1
            if ab: br['angle:' + ab] += 1
            if outcome == 'inaudible': br['inaudible:clip' if (gamemap != 8 and dist > CLIP) else 'inaudible:vol0'] += 1
            if gamemap == 8 and outcome == 'started' and dist >= CLOSE: br['map8:far' if dist > CLIP else 'map8:near'] += 1
            if outcome == 'started':
                (map8vols if gamemap == 8 else vols)[int(s[9])] += 1
                seps[int(s[10])] += 1
            if okind == 'button':
                br['button:release_audible' if outcome != 'inaudible' else 'button:release_inaudible'] += 1
                if ox or oy: br['button:release_next_busy'] += 1
        if SFX_NAMES[sfx] == 'swtchn' and okind == 'none': br['button:press_null'] += 1
        if SFX_NAMES[sfx] == 'swtchn' and okind == 'sector': br['button:press_slot0'] += 1
    evicted = {k[2] for k in K if k[8] == 'priority'}
    for s in S:
        if s[6] == 'started': br['getchannel:evict' if s[2] in evicted else 'getchannel:free'] += 1
        if s[6] == 'no_channel': br['getchannel:refuse'] += 1
    for v in vols:
        if 1 <= v <= 8: br[f'vol:{v}'] += vols[v]
    if 33 in seps: br['sep:33'] += seps[33]
    if 224 in seps: br['sep:224'] += seps[224]
    for k in K:
        br['stop:' + k[8]] += 1
        if k[8] == 'same_origin' and k[5] == 'none': br['stop:same_origin_null'] += 1
        br['stopchannel:playing' if k[9] == '1' else 'stopchannel:not_playing'] += 1
        if k[8] in ('finished', 'inaudible'): br['update:' + k[8]] += 1
    for t in T: br['stopsound:found' if t[5] != '0' else 'stopsound:none'] += 1
    for c in C:
        if c[7] != '-': br['update:params'] += 1
        if c[4] in ('none', 'player'): br['update:local'] += 1
    if L: br['lump:first'] += len(L)
    started_tics = sorted(int(s[1]) for s in S if s[6] == 'started')
    if any(started_tics[i + 8] - started_tics[i] < 4 for i in range(len(started_tics) - 8)): br['starts:9_in_4_tics'] += 1
    occ = collections.Counter(int(r[9]) for r in R if r[8] == '1')
    return dict(Z=Z, level=[_ep, gamemap], outcomes=dict(sorted(collections.Counter(s[6] for s in S).items())),
                okinds=dict(sorted(collections.Counter(s[4] for s in S).items())),
                stop_reasons=dict(sorted(collections.Counter(k[8] for k in K).items())),
                sfx_started=dict(sorted(sfx_started.items())), sfx_attempted=dict(sorted(sfx_attempted.items())),
                volumes=dict(sorted(vols.items())), map8_volumes=dict(sorted(map8vols.items())),
                separation_min_max=[min(seps), max(seps)] if seps else None,
                occupied_after_update=dict(sorted(occ.items())),
                model_checked=model_checked, model_mismatch=len(model_bad), model_first_mismatch=model_bad[0] if model_bad else None,
                branches=dict(sorted((k, v) for k, v in br.items() if v)))


# ---- items and runs
def items(only=None):
    out = []
    for i in B.items():
        i = dict(i)
        out.append(i)
    if MANIFEST.exists():
        man = json.loads(MANIFEST.read_text())
        if man.get('schema') != MANIFEST_SCHEMA: sys.exit('m5c manifest schema')
        for r in man['rooms']:
            for s in r['scripts']:
                out.append(dict(lump=s['lump'], name=s['lump'], wad=ROOT / r['file'], lmp=ROOT / s['lmp'], kind='m5c', room=r, script=s))
    if only:
        want = {x.upper() for x in only.split(',')}
        out = [i for i in out if i['lump'] in want or i['name'].upper() in want]
    return out


def run(binary, item, out, extra=(), dumps=False, frames=True, sound=True):
    """One runner process: dict(code, summary, stderr, dumps, per_snapshot, frames, sound)."""
    if out.exists(): shutil.rmtree(out)
    out.mkdir(parents=True)
    fp, sp = out / 'frames.txt', out / 'sound.txt'
    wads = [item['wad'], GFX, SFX, MUSIC, item['lmp']]
    cmd = [str(binary), *[x for w in wads for x in ('--wad', str(w))], '--demo', item['lmp'].stem.upper(),
           *(['--out', str(out)] if dumps else ['--no-dump']), *B.bespoke_args(item), *extra]
    if frames: cmd += ['--frames', str(fp)]
    if sound: cmd += ['--sound-out', str(sp)]
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV)
    names = dump.snapshot_names(str(out)) if dumps else []
    h, per = hashlib.sha256(), []
    for n in names:
        b = (out / n).read_bytes(); h.update(b); per.append(sha(b))
    r = dict(code=p.returncode, summary=B.A.SUMMARY.search(p.stdout), stderr=p.stderr, dumps=h.hexdigest(), snapshots=len(names),
             per_snapshot=per, frames=fp.read_text() if fp.exists() else '', sound=sp.read_text() if sp.exists() else '')
    shutil.rmtree(out)
    return r


def normalise(text):
    """(golden, full) frame streams: as m5b_reference.normalise, plus Q and V rows, whose digests become
    "undefined" ("refused2d") on a tracker-flagged (2D-refused) tic."""
    bad2d, flagged = set(), set()
    for l in text.splitlines():
        f = l.split(',')
        if f[0] == 'D' and any(int(f[1 + B.DCOLS.index(c)]) for c in ('bar_undef_px', 'beyond_rows', 'lump_overrun')): bad2d.add(f[1])
        if f[0] == 'F' and int(f[1 + B.FCOLS.index('columns')]) > 0: flagged.add(f[1])
    golden, full = [], []
    for l in text.splitlines(keepends=True):
        k = l.split(',', 1)[0]
        f = l.rstrip('\n').split(',')
        if k == 'F':
            if f[1] in bad2d: f[2] = f[3] = 'refused2d'
            elif f[1] in flagged: f[2] = 'undefined'
            l = ','.join(f) + '\n'
        elif k in ('Q', 'V', 'D') and (f[1] in bad2d or f[1] in flagged):
            f[2 if k == 'Q' else -1] = 'refused2d' if f[1] in bad2d else 'undefined'
            l = ','.join(f) + '\n'
        if l.startswith('#') or k in GOLDEN_KINDS: golden.append(l)
        full.append(l)
    return ''.join(golden), ''.join(full)


def truncate(text, last_tic, keep_end):
    """Rows of tics <= last_tic; the end rows (frame W and Z, sound Z) only when the runs end alike."""
    out = []
    for l in text.splitlines(keepends=True):
        k = l.split(',', 1)[0]
        if l.startswith('#') or k == 'N': out.append(l); continue
        if k == 'Z' or (k == 'W' and not l.split(',')[1].lstrip('-').isdigit()):
            if keep_end: out.append(l)
            continue
        if int(l.split(',')[1]) <= last_tic: out.append(l)
    return ''.join(out)


def last_tic_of(r):
    R = rows_of(r['sound'], 'R')
    if R: return int(R[-1][1])
    F = rows_of(r['frames'], 'F')
    return int(F[-1][1]) if F else -1


def end_of(r):
    rows = B.frows_of(r['frames'])
    if rows['O']: return 'refused:' + rows['O'][0].split(',')[2]
    rep = B.A.report_of(r['stderr'])
    if rep: return 'sanitizer:' + rep
    if rows['E']: return 'error:' + rows['E'][0].split(',', 2)[2]
    if r['summary']: return r['summary'][2]
    return f'abnormal:{r["code"]}'


def build_all(work, cc, strict32=False):
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try:
            bins = {v: m5c_build.build(v, work / 'build', cc) for v in VARIANTS + (['strict32'] if strict32 else [])}
            frozen = m5b_build.build('strict', work / 'm5bbuild', cc)
        finally: sys.stdout = so
    return bins, frozen


def face_check(fork, full, sb):
    """Section 3.4's Q-row check: nothing but the face rectangle (and the face and rndindex fields) differs."""
    fl = {l.split(',')[1] for l in fork.splitlines() + full.splitlines()
          if l.startswith('F,') and l.split(',')[2] in ('undefined', 'refused2d')}
    qa = {f[1]: f[2] for f in rows_of(full, 'Q')}
    fa, fb = {f[1]: f[2] for f in rows_of(full, 'F')}, {f[1]: f[2] for f in rows_of(fork, 'F')}
    pa, pb = {f[1]: f for f in rows_of(full, 'P')}, {f[1]: f for f in rows_of(fork, 'P')}
    qb = {f[1]: f[2] for f in rows_of(fork, 'Q')}
    tics = [t for t in fb if t in fa and t not in fl]
    out = dict(frames=len(tics), flagged_skipped=len(fl),
               face_differs=sum(1 for t in tics if pa[t][4] != pb[t][4]),
               rndindex_differs=sum(1 for t in tics if pa[t][6] != pb[t][6]),
               full_digest_differs=sum(1 for t in tics if fa[t] != fb[t]),
               noface_digest_differs=[int(t) for t in tics if qa.get(t) != qb.get(t)],
               other_status_differs=[int(t) for t in tics if pa[t][:4] + pa[t][5:6] != pb[t][:4] + pb[t][5:6]])
    out['ok'] = not out['noface_digest_differs'] and not out['other_status_differs'] and (sb == 10 or not out['full_digest_differs'])
    return out


def run_item(item, bins, frozen, runs, frozen_gold):
    """Every run and comparison of one item: (record, frame goldens {sb: text}, sound golden, sound streams, failures)."""
    L = item['lump']; out = {}; fails = []; goldens = {}
    never = run(bins['strict'], item, runs / f'{L}_never', ['--render', 'none'], dumps=True, frames=False)
    out['dumps_never_sha256'] = never['dumps']
    if item['kind'] == 'm5c':
        fr = run(frozen, item, runs / f'{L}_frozen', ['--render', 'none'], dumps=True, frames=False, sound=False)
        out['never_equals_frozen_m5b_runner'] = fr['dumps'] == never['dumps']
        if not out['never_equals_frozen_m5b_runner']: fails.append(f'{L}: masked never-render dumps differ from the frozen M5b runner')
    else:
        out['never_equals_frozen_m5b_golden'] = never['dumps'] == frozen_gold[L]['dumps_never_sha256']
        if not out['never_equals_frozen_m5b_golden']: fails.append(f'{L}: masked never-render dumps differ from the frozen M5b golden')
    out['views'], sounds = {}, {}
    for sb in VIEWS:
        base = ['--render', 'every', '--screenblocks', str(sb)]
        fork = run(bins['strict'], item, runs / f'{L}_{sb}_fork', base + ['--sound', 'off', '--wipe', 'none'], sound=False)
        fg = B.normalise(fork['frames'])[0]
        if item['kind'] == 'm5c':
            fz = run(frozen, item, runs / f'{L}_{sb}_frozenf', base, sound=False)
            ref = B.normalise(fz['frames'])[0]
            fork_ok = fg == ref
        else:
            ref = (M5B_FRAMES / f'{L}.sb{sb}.frames').read_text()
            last = max([int(l.split(',')[1]) for l in ref.splitlines() if l.startswith('F,')], default=-1)
            fork_ok = B.truncate(fg, last, True) == ref
        if not fork_ok: fails.append(f'{L} sb{sb}: --sound off --wipe none differs from the frozen M5b frames')
        per = {}
        for v in VARIANTS + ['again']:
            r = run(bins['strict' if v == 'again' else v], item, runs / f'{L}_{sb}_{v}', base, dumps=v != 'again')
            g, full = normalise(r['frames'])
            per[v] = dict(r=r, golden=g, full=full, end=end_of(r), last=last_tic_of(r))
        s = per['strict']
        last = min(p['last'] for p in per.values())
        same_end = len({p['end'] for p in per.values()}) == 1 and len({p['last'] for p in per.values()}) == 1
        tr = lambda t: truncate(t, last, same_end)
        fident = all(tr(p['full']) == tr(s['full']) for v, p in per.items() if v != 'again')
        sident = all(tr(p['r']['sound']) == tr(s['r']['sound']) for v, p in per.items() if v != 'again')
        determ = tr(per['again']['full']) == tr(s['full']) and tr(per['again']['r']['sound']) == tr(s['r']['sound'])
        nd = min(p['r']['snapshots'] for v, p in per.items() if v != 'again')
        dident = all(p['r']['per_snapshot'][:nd] == never['per_snapshot'][:nd] for v, p in per.items() if v != 'again')
        if not (fident and sident and determ and dident):
            fails.append(f'{L} sb{sb}: frames identical {fident}, sound identical {sident}, two strict runs identical {determ}, '
                         f'dumps equal the never-render set {dident}')
        golden = truncate(s['golden'], last, True)
        fc = face_check(normalise(fork['frames'])[0], golden, sb)
        if not fc['ok']: fails.append(f"{L} sb{sb}: outside the face something differs from the sound-free, wipe-free frames: "
                                      f"face-zeroed digests at {fc['noface_digest_differs'][:5]} ({len(fc['noface_digest_differs'])} tics), "
                                      f"status fields at {fc['other_status_differs'][:5]}, whole frames {fc['full_digest_differs']}")
        grows, full_rows = B.frows_of(golden), B.frows_of(s['full'])
        rec = dict(B.facts(grows, full_rows), end=s['end'], end_tic=last, frames_sha256=sha(golden.encode()),
                   variant_endings={v: p['end'] for v, p in per.items() if v != 'again'}, compared_through_tic=last,
                   variants_identical=fident, sound_variants_identical=sident, strict_runs_identical=determ,
                   dumps_equal_never_through_snapshot=nd, fork_equals_frozen=fork_ok, face_check=fc,
                   melt_steps=len(rows_of(golden, 'V')))
        rec['sanitizer_reports'] = {v: B.A.report_of(p['r']['stderr']) for v, p in per.items()
                                    if v in ('overflow', 'address') and B.A.report_of(p['r']['stderr'])}
        rec['sound_sanitizer_reports'] = sum(1 for x in rec['sanitizer_reports'].values() if any(f in x for f in SOUND_FILES))
        if rec['sound_sanitizer_reports']: fails.append(f'{L} sb{sb}: a sanitizer report in a sound file: {rec["sanitizer_reports"]}')
        alts = {}
        for name, ex in ALTERNATES.items():
            a = run(bins['strict'], item, runs / f'{L}_{sb}_{name}', base + ex)
            ag = truncate(normalise(a['frames'])[0], last_tic_of(a), True)
            sf = sound_facts(a['sound'])
            alts[name] = dict(frames_sha256=sha(ag.encode()), sound_sha256=sha(a['sound'].encode()), end=end_of(a),
                              outcomes=sf['outcomes'], stop_reasons=sf['stop_reasons'], updates=sf['Z'].get('updates'),
                              start_rows_sha256=sha('\n'.join(','.join(x) for x in rows_of(a['sound'], 'S')).encode()),
                              rnd_rows_equal_recommended=[r[:5] for r in rows_of(a['sound'], 'R')] == [r[:5] for r in rows_of(s['r']['sound'], 'R')],
                              face_frames_equal_recommended=[f[:7] for f in rows_of(ag, 'P')] == [f[:7] for f in rows_of(golden, 'P')])
        rec['alternates'] = alts
        out['views'][str(sb)] = rec
        goldens[sb] = golden
        sounds[sb] = (tr(s['r']['sound']), last, s['r']['sound'])
    l10, l11 = sounds[10][1], sounds[11][1]
    lt = min(l10, l11)
    out['sound_screens_identical'] = truncate(sounds[10][2], lt, l10 == l11) == truncate(sounds[11][2], lt, l10 == l11)
    out['sound_never_equals_render'] = never['sound'] == sounds[10][2]
    sgold = truncate(sounds[10][2], l10, True)
    sf = sound_facts(sgold)
    out['sound'] = dict(sound_sha256=sha(sgold.encode()), **{k: v for k, v in sf.items() if k != 'branches'})
    for k in ('mismatch', 'unknown_origin', 'other_draws'):
        if sf['Z'].get(k): fails.append(f'{L}: Z {k} = {sf["Z"][k]}')
    if sf['model_mismatch']: fails.append(f'{L}: the independent model disagrees with the genuine start: {sf["model_first_mismatch"]}')
    return out, goldens, sgold, sf, fails


def coverage_doc(per_item):
    by_item = {L: dict(sfx=sorted(f['sfx_started']), branches=sorted(f['branches'])) for L, f in per_item.items()}
    sfx_all = sorted({s for f in by_item.values() for s in f['sfx']})
    br_all = sorted({b for f in by_item.values() for b in f['branches']})
    first = lambda key, x: sorted(L for L, f in by_item.items() if x in f[key])
    return dict(schema=COVERAGE_SCHEMA,
                source='measured from the genuine sound streams of tools/reference/m5c_reference.py under the recommended switches',
                required_sfx=REQUIRED_SFX, required_branches=REQUIRED_BRANCHES,
                unreachable_sfx=UNREACHABLE_SFX, unreachable_branches=UNREACHABLE_BRANCHES,
                missing_sfx=[s for s in REQUIRED_SFX if s not in sfx_all],
                missing_branches=[b for b in REQUIRED_BRANCHES if b not in br_all],
                sfx_reached_by={s: first('sfx', s) for s in sfx_all},
                branches_reached_by={b: first('branches', b) for b in br_all},
                by_item=by_item)


def selftest(bins, frozen, work, frozen_gold):
    res = []
    runs = work / 'self'
    item = next(i for i in items('BARON1'))
    base = ['--render', 'every', '--screenblocks', '10']
    gold = (SOUND_DIR / 'BARON1.sound').read_text()
    planted = run(bins['strict'], item, runs, base + ['--selftest-sound', '10'])
    pl = truncate(planted['sound'], last_tic_of(planted), True)
    diff = [(a, b) for a, b in zip(gold.splitlines(), pl.splitlines()) if a != b]
    res.append(('started volume', f'--selftest-sound 10 on BARON1: differing rows {diff}',
                len(diff) == 1 and diff[0][0].startswith('S,') and int(diff[0][0].split(',')[1]) >= 10))
    nm = run(bins['strict'], item, runs, ['--render', 'none', '--no-mask-rnd'], dumps=True, frames=False)
    res.append(('unmasked rndindex', f'--no-mask-rnd: dump set equals the frozen digest {nm["dumps"] == frozen_gold["BARON1"]["dumps_never_sha256"]}',
                nm['dumps'] != frozen_gold['BARON1']['dumps_never_sha256']))
    fork = run(bins['strict'], item, runs, base + ['--sound', 'off', '--wipe', 'none'], sound=False)
    bar = run(bins['strict'], item, runs, base + ['--selftest-bar', '20', '200', '185'])
    fc = face_check(normalise(fork['frames'])[0], normalise(bar['frames'])[0], 10)
    res.append(('pixel outside the face', f'--selftest-bar 20 200 185: face check ok {fc["ok"]}, differing face-zeroed tics {fc["noface_digest_differs"]}',
                not fc['ok'] and fc['noface_digest_differs'] == [20]))
    cov = json.loads(COVERAGE.read_text())
    lost = {L: v for L, v in cov['by_item'].items() if L != max(cov['by_item'], key=lambda k: len(cov['by_item'][k]['sfx']))}
    sfx_all = {s for f in lost.values() for s in f['sfx']}; br_all = {b for f in lost.values() for b in f['branches']}
    missing = [s for s in REQUIRED_SFX if s not in sfx_all] + [b for b in REQUIRED_BRANCHES if b not in br_all]
    res.append(('coverage loss', f'the item with the most sfx removed: {len(missing)} required sfx or branches no longer reached', bool(missing)))
    return res


def strict32_compare(bins, todo, work):
    out = {}
    for item in todo:
        L = item['lump']
        base = ['--render', 'every', '--screenblocks', '10']
        a = run(bins['strict'], item, work / 's32' / f'{L}_64', base)
        b = run(bins['strict32'], item, work / 's32' / f'{L}_32', base)
        last = min(last_tic_of(a), last_tic_of(b))
        sa, sb_ = truncate(a['sound'], last, False).splitlines(), truncate(b['sound'], last, False).splitlines()
        fa, fb = truncate(normalise(a['frames'])[1], last, False), truncate(normalise(b['frames'])[1], last, False)
        diff = [(x, y) for x, y in zip(sa, sb_) if x != y]
        btn = [(x, y) for x, y in zip(sa, sb_) if x.split(',')[4:5] == ['button'] or y.split(',')[4:5] == ['button']]
        out[L] = dict(sound_identical=sa == sb_, differing_sound_rows=len(diff) + abs(len(sa) - len(sb_)), frames_identical=fa == fb,
                      first_differing=diff[0] if diff else None, button_rows=btn, end64=end_of(a), end32=end_of(b))
        print(f"strict32 {L}: sound identical {sa == sb_} ({out[L]['differing_sound_rows']} rows differ), frames identical {fa == fb}, "
              f"button rows {len(btn)}", flush=True)
        for x, y in btn: print(f'   64: {x}\n   32: {y}')
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5c-reference')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--strict32', action='store_true')
    ap.add_argument('--only')
    ap.add_argument('--jobs', type=int, default=2)
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    bins, frozen = build_all(work, a.cc, a.strict32)
    if a.strict32:
        if not bins.get('strict32'): sys.exit('strict32: no -m32 toolchain (skipped)')
        res = strict32_compare(bins, items(a.only), work)
        n = sum(1 for r in res.values() if r['sound_identical'] and r['frames_identical'])
        print(f'strict32: {n} of {len(res)} items with identical sound and frame streams')
        return
    frozen_gold = json.loads(M5B_RESULTS.read_text())['items']
    failures, results, goldens, sgoldens, facts = [], {}, {}, {}, {}
    todo = items(a.only) if (a.check or a.only or not a.selftest) else []
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        futs = [pool.submit(run_item, i, bins, frozen, work / 'runs' / i['lump'], frozen_gold) for i in todo]
        for item, fut in zip(todo, futs):
            out, gold, sgold, sf, fails = fut.result()
            failures += fails
            L = item['lump']
            results[L] = dict(kind=item['kind'], wad=item['wad'].relative_to(ROOT).as_posix(), lmp_sha256=sha(item['lmp'].read_bytes()), **out)
            for sb, g in gold.items(): goldens[(L, sb)] = g
            sgoldens[L] = sgold; facts[L] = sf
            v = out['views']['10']; z = out['sound']['Z']
            print(f"{L}: dumps {out.get('never_equals_frozen_m5b_golden', out.get('never_equals_frozen_m5b_runner'))}; "
                  f"fork {v['fork_equals_frozen']}/{out['views']['11']['fork_equals_frozen']}; "
                  f"identical {v['variants_identical']} {v['sound_variants_identical']} determ {v['strict_runs_identical']}; "
                  f"face {v['face_check']['face_differs']}/{v['face_check']['frames']} ok {v['face_check']['ok']}/{out['views']['11']['face_check']['ok']}; "
                  f"starts {z.get('attempts')} started {z.get('started')} model {out['sound']['model_checked']}/{out['sound']['model_mismatch']}; end {v['end']}",
                  flush=True)
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    full = not a.only and todo
    if full:
        cov = coverage_doc(facts)
        if cov['missing_sfx'] or cov['missing_branches']:
            failures.append(f"coverage: required sfx not reached {cov['missing_sfx']}, required branches not reached {cov['missing_branches']}")
        tot = lambda k: sum(r['views'][str(sb)][k] for r in results.values() for sb in VIEWS)
        doc = dict(schema=SCHEMA, runner_contract_sha256=sha(m5c_build.CONTRACT.read_bytes()),
                   tools_sha256={f'tools/reference/{n}': sha(m5c_build.text(ROOT / 'tools/reference' / n).encode('latin-1'))
                                 for n in ('m5c_build.py', 'm5c_driver.c', 'm5c_stubs.c', 'm5c_sfx.py', 'm5c_wad.py')},
                   gfx_sha256=sha(GFX.read_bytes()), sfx_sha256=sha(SFX.read_bytes()), music_sha256=sha(MUSIC.read_bytes()),
                   manifest_sha256=sha(MANIFEST.read_bytes()) if MANIFEST.exists() else None,
                   switches=RECOMMENDED, alternates={k: ' '.join(v) for k, v in ALTERNATES.items()}, variants=VARIANTS, views=VIEWS,
                   contract='sound streams and frames identical to the genuine engine built with its integrated sound configuration '
                            '(doomdef.h:84 commented out), S_UpdateSounds after every tic and the genuine level-start melt',
                   asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
                   totals=dict(items=len(results), frames=tot('frames'),
                               starts=sum(r['sound']['Z'].get('attempts', 0) for r in results.values()),
                               started=sum(r['sound']['Z'].get('started', 0) for r in results.values()),
                               model_checked=sum(r['sound']['model_checked'] for r in results.values())),
                   coverage_sha256=sha((json.dumps(cov, indent=1) + '\n').encode()), items=results)
        files = {RESULTS: json.dumps(doc, indent=1) + '\n', COVERAGE: json.dumps(cov, indent=1) + '\n'}
        for (L, sb), t in goldens.items(): files[FRAMES_DIR / f'{L}.sb{sb}.frames'] = t
        for L, t in sgoldens.items(): files[SOUND_DIR / f'{L}.sound'] = t
        if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
        if a.check:
            bad = [str(p.relative_to(ROOT)) for p, t in files.items() if not p.exists() or p.read_text() != t]
            extra = [str(p.relative_to(ROOT)) for d in (FRAMES_DIR, SOUND_DIR) if d.exists() for p in d.iterdir() if p not in files]
            if bad or extra: sys.exit(f'differs: {bad}; not produced by this run: {extra}')
            print(f'{RESULTS.relative_to(ROOT)}, {COVERAGE.relative_to(ROOT)} and {len(files) - 2} golden files match ({len(results)} items)')
        else:
            FRAMES_DIR.mkdir(exist_ok=True); SOUND_DIR.mkdir(exist_ok=True)
            for p, t in files.items(): p.write_text(t)
            print(f'wrote {RESULTS.relative_to(ROOT)}, {COVERAGE.relative_to(ROOT)} and {len(files) - 2} golden files')
    if a.selftest:
        outcomes = selftest(bins, frozen, work, frozen_gold)
        for what, detail, ok in outcomes: print(f"selftest {what}: {detail}: {'pass' if ok else 'FAIL'}")
        if not all(ok for _, _, ok in outcomes): sys.exit(1)


if __name__ == '__main__':
    main()
