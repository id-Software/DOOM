#!/usr/bin/env python3
"""Writes docs/media/m5b-status-bar-side-by-side.gif, the third animation shown in the README.

    m5b_readme_visual.py [--work DIR] [--out FILE] [--cargo-target DIR] [--check]

The animation shows the genuine linuxdoom-1.10 engine and the Rust port (the app's composed Display:
status bar, 3D view, HUD message and palette number) playing the same committed scripted session on
a synthetic room: tests/reference/m5b_sessions/palred_damage.txt on tests/reference/m5b_wads/palred.wad
with our own generated graphics (tests/reference/m5b_gfx.wad); no IWAD, no id Software art. The
session is a soulsphere pickup with its message and bonus palette, then a barrel shot point-blank:
damage, the red palettes and the face's reaction. Three panels per frame: the genuine frame, the
Rust frame and their per-pixel difference. Every number drawn is computed from the frames:

* the demo comes from the headless app (crates/doom-game/examples/m5b_session.rs, a release build)
  playing the script on a fake clock; its SHA-256 must equal the committed driver_demo_sha256 of
  tests/reference/m5b_session_results.json, and the script's SHA-256 its script_sha256;
* the genuine frames come from the M5b runner (tools/reference/m5b_build.py, strict variant, built
  under --work) replaying that demo with --render every --frames and --frames-raw; the normalised
  frame stream (frame digests, palette rows, face, message and M_Random index of every tic) must
  hash to the committed frames_sha256, and every raw frame must equal its frame row's digest;
* the Rust frames and palette numbers come from crates/doom-game/examples/m5b_animation_frames.rs
  (the app's replay of the same demo, screenblocks 10);
* all tics are compared on palette indices and palette numbers (not on colours); the script exits
  with the first differing tic and pixel, or palette number, if there is any difference, and no
  animation is written then; the difference panel is black where the panels agree (a differing
  pixel would be drawn red) and the caption counts them;
* each panel is shown through the PLAYPAL of the tic's palette number (the 14 palettes of the
  synthetic PLAYPAL, which the script reads from m5b_gfx.wad and requires to equal m5a_gfx.playpal()),
  so the bonus and red flashes are visible; the lettering uses the 5x7 bitmap font of
  m5a_readme_visual.py. Frames are identical to the genuine engine under the declared sound-free,
  wipe-free contract (decision S15); that contract is not a claim of an identical vanilla session.

Each frame has its own local colour table (the tic's palette, with the unused indices recoloured for
the lettering and the difference panel). The GIF writer reuses the LZW encoder of m5a_readme_visual.py
with frame deltas compared on colours; the script decodes its own output and compares every
displayed frame. The file has no timestamp or path, so a rerun on the same inputs writes the same
bytes. `--check` regenerates the animation and compares it with the committed file instead of writing
it (it builds the genuine runner and runs cargo, so it needs a C compiler and cargo; it is a local
check, not part of CI). Keep --work outside the repository.
"""
import argparse, hashlib, json, os, shutil, struct, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.path.insert(0, str(ROOT / 'tools/media'))
import m5b_build, m5b_reference as R, m5a_gfx      # noqa: E402
import m5a_readme_visual as M                       # noqa: E402

OUT = ROOT / 'docs/media/m5b-status-bar-side-by-side.gif'
SESSION = ROOT / 'tests/reference/m5b_sessions/palred_damage.txt'
RESULTS = ROOT / 'tests/reference/m5b_session_results.json'
NAME = 'palred_damage'
LUMP = 'M5BSESS'
W, H = 320, 200
PIXELS = W * H
SB = 10
STEP = 1                 # every tic is shown
SLOW = 2                 # at half of real speed (35 tics a second)
HOLD_CS = 150            # the last frame stays this long before the loop restarts
GAP, LABEL_H, CAPTION_H = 4, 22, 76
SCREEN_W = 3 * W + 2 * GAP
SCREEN_H = LABEL_H + H + CAPTION_H
FONT = dict(M.FONT)
FONT['_'] = [[0] * 5] * 6 + [[1] * 5]
UI = dict(bg=(24, 24, 24), black=(0, 0, 0), white=(255, 255, 255), green=(80, 255, 80), red=(255, 0, 0),
          grey=(150, 150, 150))
sha = lambda b: hashlib.sha256(b).hexdigest()


def run(cmd, **kw):
    return subprocess.run([str(c) for c in cmd], capture_output=True, text=True, **kw)


def playpal():
    d = R.GFX.read_bytes()
    n, o = struct.unpack('<ii', d[4:12])
    for i in range(n):
        p, s, name = struct.unpack('<ii8s', d[o + 16 * i:o + 16 * i + 16])
        if name.rstrip(b'\0') == b'PLAYPAL':
            pal = d[p:p + s]
            if len(pal) != 14 * 768 or pal != m5a_gfx.playpal():
                sys.exit('the PLAYPAL of m5b_gfx.wad is not the 14 palettes of m5a_gfx.playpal()')
            return pal
    sys.exit('m5b_gfx.wad has no PLAYPAL')


def build_tools(work, cargo_target):
    env = dict(os.environ, CARGO_TARGET_DIR=str(cargo_target))
    p = run(['cargo', 'build', '-p', 'doom-game', '--release', '--locked', '--example', 'm5b_session',
             '--example', 'm5b_animation_frames'], cwd=ROOT, env=env)
    if p.returncode:
        sys.exit(f'cargo build failed:\n{p.stderr[-2000:]}')
    with open(os.devnull, 'w') as q:                       # the build prints its progress
        so, sys.stdout = sys.stdout, q
        try:
            runner = m5b_build.build('strict', work / 'build')
        finally:
            sys.stdout = so
    ex = cargo_target / 'release/examples'
    return ex / 'm5b_session', ex / 'm5b_animation_frames', runner


def collect(work, tools):
    """Plays the session, replays it twice, checks everything; returns per-tic records."""
    session, frames_ex, runner = tools
    ref = json.loads(RESULTS.read_text())['sessions'][NAME]
    if sha(SESSION.read_bytes()) != ref['script_sha256']:
        sys.exit('the session script is not the one the committed results describe')
    wads = [ROOT / 'tests/reference/m5b_wads/palred.wad', R.GFX]
    app = work / 'app'
    shutil.rmtree(app, ignore_errors=True)
    p = run([session, *[a for w in wads for a in ('--wad', w)], '--out', app, '--screenblocks', SB, '--script', SESSION,
             '--map', 1, '--skill', 2])
    if p.returncode:
        sys.exit(f'the headless app failed: {p.stderr[-500:]}')
    lmp = app / 'session.lmp'
    if sha(lmp.read_bytes()) != ref['driver_demo_sha256']:
        sys.exit('the app demo is not the committed input driver demo')
    # genuine engine
    g = work / 'genuine'
    shutil.rmtree(g, ignore_errors=True); (g / 'raw').mkdir(parents=True)
    shutil.copy(lmp, g / f'{LUMP}.lmp')
    p = run([runner, *[a for w in wads for a in ('--wad', w)], '--wad', g / f'{LUMP}.lmp', '--demo', LUMP, '--no-dump',
             '--render', 'every', '--screenblocks', SB, '--frames', g / 'frames.txt', '--frames-raw', g / 'raw'], env=R.ENV)
    if p.returncode:
        sys.exit(f'the genuine runner failed: {p.stderr[-500:]}')
    golden, _ = R.normalise((g / 'frames.txt').read_text())
    if sha(golden.encode()) != ref['frames_sha256']:
        sys.exit('the genuine frame stream differs from the committed frames_sha256')
    rows = {}
    for l in golden.splitlines():
        if l and not l.startswith('#'):
            rows.setdefault(l.split(',', 1)[0], {})[int(l.split(',')[1])] = l.split(',')
    # Rust
    r = work / 'rust'
    shutil.rmtree(r, ignore_errors=True)
    p = run([frames_ex, *[a for w in wads for a in ('--wad', w)], '--demo', lmp, '--screenblocks', SB, '--out', r])
    if p.returncode:
        sys.exit(f'the Rust frame dump failed: {p.stderr[-500:]}')
    rust_pal = {int(a): int(b) for a, b in (l.split(',') for l in (r / 'palettes.txt').read_text().splitlines())}
    ntics = ref['tics']
    if len(rows['F']) != ntics or len(rust_pal) != ntics:
        sys.exit(f'expected {ntics} tics, genuine {len(rows["F"])}, Rust {len(rust_pal)}')
    recs, differing, pal_bad = [], 0, 0
    for t in range(ntics):
        c = (g / 'raw' / f'frame_{t:05d}.raw').read_bytes()
        x = (r / f'frame_{t:05d}.raw').read_bytes()
        if len(c) != PIXELS or len(x) != PIXELS:
            sys.exit(f'tic {t}: a frame is not {PIXELS} bytes')
        f = rows['F'][t]
        if f[1 + R.FCOLS.index('whole')] == '1' or int(f[1 + R.FCOLS.index('columns')]):
            sys.exit(f'tic {t}: the golden marks this frame non-comparable')
        if sha(c) != f[2]:
            sys.exit(f'tic {t}: the genuine raw frame does not match its committed-stream digest')
        gp = int(rows['P'][t][2])
        if gp != rust_pal[t]:
            sys.exit(f'first palette difference at tic {t}: genuine {gp}, Rust {rust_pal[t]}')
        n = sum(a != b for a, b in zip(c, x))
        if n:
            i = next(k for k in range(PIXELS) if c[k] != x[k])
            sys.exit(f'first difference at tic {t}, pixel x={i % W} y={i // W}: genuine {c[i]}, Rust {x[i]} ({n} pixels)')
        recs.append(dict(tic=t, c=c, r=x, pal=gp, face=int(rows['P'][t][4]), differing=0))
    return recs, ntics


# --- drawing -------------------------------------------------------------------------------------
def text(canvas, x, y, s, color, scale=2):
    if x + 6 * scale * len(s) > SCREEN_W + scale:
        sys.exit(f'caption too wide: {s!r}')
    for ch in s.upper():
        g = FONT.get(ch)
        if g is None:
            sys.exit(f'no glyph for {ch!r}')
        for gy, row in enumerate(g):
            for gx, bit in enumerate(row):
                if bit:
                    for dy in range(scale):
                        base = (y + gy * scale + dy) * SCREEN_W + x + gx * scale
                        canvas[base:base + scale] = bytes([color]) * scale
        x += 6 * scale


def compose(pal, recs, ntics):
    """[(local colour table 768 bytes, indices, transparent index)] per shown tic."""
    out, shown = [], recs[::STEP]
    for k, s in enumerate(shown):
        used = set(s['c']) | set(s['r'])
        free = [i for i in range(255, -1, -1) if i not in used]
        if len(free) < len(UI) + 1:
            sys.exit(f'tic {s["tic"]}: only {len(free)} unused palette indices for lettering')
        table = bytearray(pal[s['pal'] * 768:(s['pal'] + 1) * 768])
        idx = {}
        for name, i in zip(UI, free):
            idx[name] = i
            table[3 * i:3 * i + 3] = bytes(UI[name])
        trans = free[len(UI)]
        cv = bytearray([idx['bg']]) * (SCREEN_W * SCREEN_H)
        diff = bytes(idx['red'] if a != b else idx['black'] for a, b in zip(s['c'], s['r']))
        for p, img in enumerate((s['c'], s['r'], diff)):
            x0 = p * (W + GAP)
            for y in range(H):
                at = (LABEL_H + y) * SCREEN_W + x0
                cv[at:at + W] = img[y * W:(y + 1) * W]
        white = idx['white']
        text(cv, 0, 4, 'GENUINE C ENGINE', white)
        text(cv, W + GAP, 4, 'RUST DISPLAY', white)
        text(cv, 2 * (W + GAP), 4, 'DIFFERENCE', white)
        y = LABEL_H + H + 4
        text(cv, 0, y, 'PALRED_DAMAGE: SOULSPHERE PICKUP WITH ITS MESSAGE AND BONUS PALETTE,', white)
        text(cv, 0, y + 16, 'THEN A BARREL SHOT POINT-BLANK: DAMAGE, RED PALETTES, FACE', white)
        n = s['differing']
        col = idx['green'] if n == 0 else idx['red']
        text(cv, 0, y + 34, f"TIC {s['tic']}/{ntics} PALETTE {s['pal']} FACE {s['face']}: "
             f"{n} OF {PIXELS} PIXELS DIFFER", col)
        text(cv, 0, y + 52, 'ALL TICS COMPARED ON PALETTE INDICES. HALF SPEED. SYNTHETIC TEST ROOM AND ART, NOT THE GAMES',
             idx['grey'], 1)
        out.append((bytes(table), bytes(cv), trans))
    return out


def write_gif(frames, delays):
    out = bytearray(b'GIF89a' + SCREEN_W.to_bytes(2, 'little') + SCREEN_H.to_bytes(2, 'little') + bytes([0xf7, 0, 0])
                    + bytes(768))
    out += bytes([0x21, 0xff, 0x0b]) + b'NETSCAPE2.0' + bytes([3, 1, 0, 0, 0])
    prev = None                                            # the previous frame as RGB bytes
    shown = []
    for (table, cv, trans), d in zip(frames, delays):
        rgb = b''.join(table[3 * i:3 * i + 3] for i in cv)
        shown.append(rgb)
        if prev is None:
            x0, y0, x1, y1 = 0, 0, SCREEN_W, SCREEN_H
        else:
            rows = [y for y in range(SCREEN_H) if rgb[3 * y * SCREEN_W:3 * (y + 1) * SCREEN_W]
                    != prev[3 * y * SCREEN_W:3 * (y + 1) * SCREEN_W]]
            if not rows:
                rows = [0]
            y0, y1 = rows[0], rows[-1] + 1
            lo, hi = SCREEN_W, 0
            for y in rows:
                a, b = rgb[3 * y * SCREEN_W:3 * (y + 1) * SCREEN_W], prev[3 * y * SCREEN_W:3 * (y + 1) * SCREEN_W]
                xs = [x for x in range(SCREEN_W) if a[3 * x:3 * x + 3] != b[3 * x:3 * x + 3]]
                if xs:
                    lo, hi = min(lo, xs[0]), max(hi, xs[-1] + 1)
            x0, x1 = (lo, hi) if hi > lo else (0, 1)
        sub = bytearray()
        for y in range(y0, y1):
            for x in range(x0, x1):
                at = y * SCREEN_W + x
                same = prev is not None and rgb[3 * at:3 * at + 3] == prev[3 * at:3 * at + 3]
                sub.append(trans if same else cv[at])
        out += bytes([0x21, 0xf9, 4, 0x05, d & 255, d >> 8, trans, 0])
        out += bytes([0x2c]) + b''.join(v.to_bytes(2, 'little') for v in (x0, y0, x1 - x0, y1 - y0)) + bytes([0x87])
        out += table
        out += bytes([8])
        data = M.lzw(bytes(sub))
        if M.unlzw(data, len(sub)) != bytes(sub):
            sys.exit('internal error: the LZW stream does not round-trip')
        out += M.blocks(data)
        prev = rgb
    out.append(0x3b)
    return bytes(out), shown


def self_check(gif, shown, delays):
    """Decodes the file with an independent walk of its blocks and compares each displayed frame's colours."""
    assert gif[:6] == b'GIF89a' and gif[-1] == 0x3b
    at = 13 + 768 + 19
    canvas = bytearray(3 * SCREEN_W * SCREEN_H)
    for want, d in zip(shown, delays):
        assert gif[at:at + 2] == b'\x21\xf9' and int.from_bytes(gif[at + 4:at + 6], 'little') == d
        trans = gif[at + 6]
        at += 8
        assert gif[at] == 0x2c and gif[at + 9] == 0x87
        x0, y0, w, h = (int.from_bytes(gif[at + 1 + 2 * i:at + 3 + 2 * i], 'little') for i in range(4))
        at += 10
        table = gif[at:at + 768]
        at += 769
        data = bytearray()
        while gif[at]:
            data += gif[at + 1:at + 1 + gif[at]]; at += 1 + gif[at]
        at += 1
        px = M.unlzw(bytes(data), w * h)
        for y in range(h):
            for x in range(w):
                v = px[y * w + x]
                if v != trans:
                    o = 3 * ((y0 + y) * SCREEN_W + x0 + x)
                    canvas[o:o + 3] = table[3 * v:3 * v + 3]
        if bytes(canvas) != want:
            sys.exit('internal error: a decoded frame differs from the composed frame')
    assert gif[at] == 0x3b


def delays_cs(n):
    """Whole-hundredth delays whose running total tracks STEP*SLOW/35 s a frame; the last one holds."""
    ends = [round((k + 1) * STEP * SLOW * 100 / 35) for k in range(n)]
    d = [ends[0]] + [b - a for a, b in zip(ends, ends[1:])]
    d[-1] += HOLD_CS
    return d


def generate(work, cargo_target):
    work.mkdir(parents=True, exist_ok=True)
    tools = build_tools(work, cargo_target)
    recs, ntics = collect(work, tools)
    frames = compose(playpal(), recs, ntics)
    delays = delays_cs(len(frames))
    gif, shown = write_gif(frames, delays)
    self_check(gif, shown, delays)
    secs = sum(delays) / 100
    pal = sorted({r['pal'] for r in recs})
    print(f'{len(frames)} frames, {SCREEN_W}x{SCREEN_H}, {secs:.2f} s, {len(gif)} bytes; {ntics} tics compared, '
          f'0 differing pixels, 0 palette mismatches; palettes {pal}, faces {sorted({r["face"] for r in recs})}; '
          f'every frame equal to the committed golden stream')
    if len(gif) >= 2_000_000 or secs >= 15:
        sys.exit('the animation exceeds 2 MB or 15 s')
    return gif


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m5b-readme-visual')
    ap.add_argument('--out', type=Path, default=OUT)
    ap.add_argument('--cargo-target', type=Path, default=None)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents:
        sys.exit('--work must lie outside the repository')
    gif = generate(work, (a.cargo_target or work / 'target').resolve())
    if a.check:
        if not OUT.exists() or OUT.read_bytes() != gif:
            sys.exit(f'{OUT.relative_to(ROOT)} differs from the regenerated animation')
        print(f'{OUT.relative_to(ROOT)}: identical to the regenerated animation')
    else:
        a.out.parent.mkdir(parents=True, exist_ok=True)
        a.out.write_bytes(gif)
        print(f'wrote {a.out}')


if __name__ == '__main__':
    main()
