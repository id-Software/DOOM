#!/usr/bin/env python3
"""Writes docs/media/m5a-renderer-side-by-side.gif, the second animation shown in the README.

    m5a_readme_visual.py [--work DIR] [--out FILE] [--cargo-target DIR] [--check]

The animation shows the genuine linuxdoom-1.10 renderer and the Rust renderer (crates/doom-render)
drawing the same walk through committed synthetic rooms (tests/reference/m5a_wads, the demo scripts
of tests/reference/m5a_scripts and our own generated graphics, tests/reference/m5a_gfx.wad; no
IWAD, no id Software art). Three panels per frame: the genuine frame, the Rust frame and their
per-pixel difference. Every number drawn is computed from the frames:

* the genuine frames come from the M5a runner (tools/reference/m5a_build.py, strict variant, built
  under --work) run with --frames-raw on each room's script;
* the Rust frames come from `cargo run -p doom-render --release --example m5a_view --raw DIR`;
* each shown frame must be byte-identical between the two (the script exits with an error
  otherwise), must equal the committed golden digest (tests/reference/m5a_frames/*.sb11.frames),
  and must not be one of the golden's non-comparable frames or have genuine-flagged columns;
* the difference panel is black where the two agree (a differing pixel would be drawn red) and
  the caption counts the differing pixels, so "0 differing pixels" is a measurement;
* the lettering uses the tiny 5x7 bitmap font defined below (our own, no font files).

The GIF writer is a Python port of the LZW encoder of crates/doom-render/src/gif.rs (variable
width codes, minimum size 8) with frame deltas (a frame stores only the changed rectangle and
leaves the other pixels transparent). The script decodes its own output and requires every
displayed frame to be reproduced exactly. The file has no timestamp or path, so a rerun on the same
inputs writes the same bytes. `--check` regenerates it under --work and compares it with the
committed file instead of writing it (it builds the genuine runner, so it needs a C compiler and
cargo; it is a local check, not part of CI). Keep --work outside the repository.
"""
import argparse, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REF = ROOT / 'tools/reference'
sys.path.insert(0, str(REF))
import m5a_build, m5a_gfx                      # noqa: E402

OUT = ROOT / 'docs/media/m5a-renderer-side-by-side.gif'
GFX = ROOT / 'tests/reference/m5a_gfx.wad'
MANIFEST = ROOT / 'tests/reference/m5a_manifest.json'
FRAMES = ROOT / 'tests/reference/m5a_frames'
SB = 11
W, H = 320, 200
PIXELS = W * H

# (room name in the manifest, first tic, last tic): sky and walls and floors, sprites, fuzz, doors.
SEGMENTS = [('SKY', 140, 259), ('SPRITES', 0, 99), ('FUZZ', 40, 139)]
STEP = 5                      # every 5th tic is shown, in real time (35 tics a second)

# --- the tiny bitmap font: 5x7 capitals, digits and a little punctuation -------------------------
GLYPHS = {
    'A': '01110 10001 10001 11111 10001 10001 10001', 'B': '11110 10001 10001 11110 10001 10001 11110',
    'C': '01110 10001 10000 10000 10000 10001 01110', 'D': '11110 10001 10001 10001 10001 10001 11110',
    'E': '11111 10000 10000 11110 10000 10000 11111', 'F': '11111 10000 10000 11110 10000 10000 10000',
    'G': '01110 10001 10000 10111 10001 10001 01111', 'H': '10001 10001 10001 11111 10001 10001 10001',
    'I': '01110 00100 00100 00100 00100 00100 01110', 'J': '00111 00010 00010 00010 00010 10010 01100',
    'K': '10001 10010 10100 11000 10100 10010 10001', 'L': '10000 10000 10000 10000 10000 10000 11111',
    'M': '10001 11011 10101 10101 10001 10001 10001', 'N': '10001 11001 10101 10011 10001 10001 10001',
    'O': '01110 10001 10001 10001 10001 10001 01110', 'P': '11110 10001 10001 11110 10000 10000 10000',
    'Q': '01110 10001 10001 10001 10101 10010 01101', 'R': '11110 10001 10001 11110 10100 10010 10001',
    'S': '01111 10000 10000 01110 00001 00001 11110', 'T': '11111 00100 00100 00100 00100 00100 00100',
    'U': '10001 10001 10001 10001 10001 10001 01110', 'V': '10001 10001 10001 10001 10001 01010 00100',
    'W': '10001 10001 10001 10101 10101 11011 10001', 'X': '10001 10001 01010 00100 01010 10001 10001',
    'Y': '10001 10001 01010 00100 00100 00100 00100', 'Z': '11111 00001 00010 00100 01000 10000 11111',
    '0': '01110 10001 10011 10101 11001 10001 01110', '1': '00100 01100 00100 00100 00100 00100 01110',
    '2': '01110 10001 00001 00010 00100 01000 11111', '3': '11110 00001 00001 01110 00001 00001 11110',
    '4': '00010 00110 01010 10010 11111 00010 00010', '5': '11111 10000 11110 00001 00001 10001 01110',
    '6': '00110 01000 10000 11110 10001 10001 01110', '7': '11111 00001 00010 00100 01000 01000 01000',
    '8': '01110 10001 10001 01110 10001 10001 01110', '9': '01110 10001 10001 01111 00001 00010 01100',
    ' ': '00000 00000 00000 00000 00000 00000 00000', '.': '00000 00000 00000 00000 00000 01100 01100',
    ':': '00000 01100 01100 00000 01100 01100 00000', ',': '00000 00000 00000 00000 01100 00100 01000',
    '-': '00000 00000 00000 11111 00000 00000 00000', '/': '00001 00001 00010 00100 01000 10000 10000',
    '=': '00000 00000 11111 00000 11111 00000 00000', '(': '00010 00100 01000 01000 01000 00100 00010',
    ')': '01000 00100 00010 00010 00010 00100 01000', '%': '11001 11010 00010 00100 01000 01011 10011',
}
FONT = {c: [[int(b) for b in row] for row in g.split()] for c, g in GLYPHS.items()}

GAP, LABEL_H, CAPTION_H = 4, 22, 76
SCREEN_W = 3 * W + 2 * GAP
SCREEN_H = LABEL_H + H + CAPTION_H


def sha(b):
    return hashlib.sha256(b).hexdigest()


def run(cmd, **kw):
    p = subprocess.run([str(c) for c in cmd], capture_output=True, text=True, **kw)
    return p


def playpal():
    return m5a_gfx.playpal()[:768]


def nearest(pal, rgb):
    """The first palette index closest to rgb."""
    best = min(range(256), key=lambda i: (sum((pal[3 * i + k] - rgb[k]) ** 2 for k in range(3)), i))
    return best


# --- genuine and Rust frames ---------------------------------------------------------------------
def room_info():
    rooms = json.loads(MANIFEST.read_text())['rooms']
    return {r['name']: r for r in rooms}


def golden_rows(lump):
    """tic -> (sha256, whole, n_flagged_columns) of the committed golden stream."""
    rows, cols = {}, {}
    for l in (FRAMES / f'{lump}.sb{SB}.frames').read_text().splitlines():
        c = l.split(',')
        if c[0] == 'F':
            rows[int(c[1])] = [c[2], c[14] == '1']
        elif c[0] == 'U':
            cols[int(c[1])] = cols.get(int(c[1]), 0) + 1
    return {t: (r[0], r[1], cols.get(t, 0)) for t, r in rows.items()}


def build_tools(work, cargo_target):
    binary = m5a_build.build('strict', work / 'build')
    env = dict(os.environ, CARGO_TARGET_DIR=str(cargo_target))
    subprocess.run(['cargo', 'build', '-p', 'doom-render', '--release', '--example', 'm5a_view', '--locked', '--quiet'],
                   cwd=ROOT, env=env, check=True)
    return binary, cargo_target / 'release/examples/m5a_view'


def genuine_frames(binary, lump, wad, lmp, raw):
    shutil.rmtree(raw, ignore_errors=True); raw.mkdir(parents=True)
    p = run([binary, '--wad', wad, '--wad', GFX, '--wad', lmp, '--demo', lump, '--no-dump', '--render', 'every',
             '--screenblocks', SB, '--frames-raw', raw], env=dict(os.environ, ASAN_OPTIONS='detect_leaks=0'))
    if p.returncode:
        sys.exit(f'{lump}: the genuine runner failed: {p.stderr[-300:]}')


def rust_frames(view, lump, first, last, raw):
    shutil.rmtree(raw, ignore_errors=True); raw.mkdir(parents=True)
    p = run([view, '--room', lump, '--raw', raw, '--screenblocks', SB, '--from', first, '--to', last])
    if p.returncode:
        sys.exit(f'{lump}: m5a_view failed: {p.stderr[-300:]}')


def collect(work, binary, view):
    """The list of frame records [dict(room, what, tic, c, r, diff)] of every shown tic."""
    info, shown = room_info(), []
    for name, first, last in SEGMENTS:
        room = info[name]
        lump = room['script']['lump']
        wad, lmp = ROOT / room['file'], ROOT / room['script']['lmp']
        graw, rraw = work / 'raw' / f'{lump}_genuine', work / 'raw' / f'{lump}_rust'
        genuine_frames(binary, lump, wad, lmp, graw)
        rust_frames(view, lump, first, last, rraw)
        gold = golden_rows(lump)
        for tic in range(first, last + 1, STEP):
            c = (graw / f'frame_{tic:05d}.raw').read_bytes()
            r = (rraw / f'frame_{tic:05d}.raw').read_bytes()
            digest, whole, flagged = gold[tic]
            if len(c) != PIXELS or len(r) != PIXELS:
                sys.exit(f'{lump} tic {tic}: a frame is not {PIXELS} bytes')
            if whole or flagged:
                sys.exit(f'{lump} tic {tic}: the golden marks this frame non-comparable; choose other tics')
            if c != r:
                sys.exit(f'{lump} tic {tic}: the genuine and Rust frames differ in '
                         f'{sum(a != b for a, b in zip(c, r))} pixels')
            if sha(c) != digest or sha(r) != digest:
                sys.exit(f'{lump} tic {tic}: the frame digest is not the committed golden {digest}')
            shown.append(dict(room=name, lump=lump, what=room['script']['what'], tic=tic, ntics=room['script']['tics'],
                              c=c, r=r, differing=sum(a != b for a, b in zip(c, r))))
    return shown


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


def compose(pal, shown):
    black, white = nearest(pal, (0, 0, 0)), nearest(pal, (255, 255, 255))
    green, red = nearest(pal, (80, 255, 80)), nearest(pal, (255, 0, 0))
    grey, bg = nearest(pal, (150, 150, 150)), nearest(pal, (24, 24, 24))
    total, frames = len(shown), []
    for k, s in enumerate(shown):
        cv = bytearray([bg]) * (SCREEN_W * SCREEN_H)
        diff = bytes(red if a != b else black for a, b in zip(s['c'], s['r']))
        for p, img in enumerate((s['c'], s['r'], diff)):
            x0 = p * (W + GAP)
            for y in range(H):
                at = (LABEL_H + y) * SCREEN_W + x0
                cv[at:at + W] = img[y * W:(y + 1) * W]
        text(cv, 0, 4, 'GENUINE C RENDERER', white)
        text(cv, W + GAP, 4, 'RUST RENDERER', white)
        text(cv, 2 * (W + GAP), 4, 'DIFFERENCE', white)
        y = LABEL_H + H + 4
        words, lines = f"{s['room']}: {s['what']}".split(), ['']
        for wd in words:
            if len(lines[-1]) + len(wd) + 1 > 78:
                lines.append('')
            lines[-1] = (lines[-1] + ' ' + wd).strip()
        if len(lines) > 2:
            sys.exit('room caption longer than two lines')
        for i, ln in enumerate(lines):
            text(cv, 0, y + 16 * i, ln, white)
        y += 16 * (len(lines) - 1)
        n = s['differing']
        text(cv, 0, y + 18, f"TIC {s['tic']}/{s['ntics']}  FRAME {k + 1}/{total}  {n} DIFFERING PIXELS OF {PIXELS}",
             green if n == 0 else red)
        text(cv, 0, y + 36, f'EVERY {STEP}TH TIC IS SHOWN, AT REAL TIME (35 TICS A SECOND). SYNTHETIC TEST ROOMS AND ART',
             grey, 1)
        frames.append(bytes(cv))
    return frames


# --- GIF writer (port of crates/doom-render/src/gif.rs, plus frame deltas) -----------------------
def lzw(pixels):
    out, acc, bits = bytearray(), 0, 0

    def put(code, width):
        nonlocal acc, bits
        acc |= code << bits; bits += width
        while bits >= 8:
            out.append(acc & 255); acc >>= 8; bits -= 8
    table, nxt, width = {}, 258, 9
    put(256, width)
    it = iter(pixels)
    prefix = next(it)
    for k in it:
        key = (prefix << 8) | k
        code = table.get(key)
        if code is not None:
            prefix = code
            continue
        put(prefix, width)
        if nxt < 4096:
            table[key] = nxt
            if nxt == 1 << width and width < 12:
                width += 1
            nxt += 1
        else:
            put(256, width); table.clear(); nxt = 258; width = 9
        prefix = k
    put(prefix, width)
    if nxt < 4096 and nxt + 1 > 1 << width and width < 12:
        width += 1
    put(257, width)
    if bits:
        out.append(acc & 255)
    return bytes(out)


def unlzw(data, n):
    pos, size, nxt = 0, 9, 258
    table = {i: bytes([i]) for i in range(256)}
    out, prev = bytearray(), None
    acc = bits = 0
    it = iter(data)
    while True:
        while bits < size:
            acc |= next(it) << bits; bits += 8
        code, acc, bits = acc & ((1 << size) - 1), acc >> size, bits - size
        if code == 256:
            table = {i: bytes([i]) for i in range(256)}; nxt, size, prev = 258, 9, None; continue
        if code == 257:
            return bytes(out)
        if prev is None:
            entry = table[code]
        else:
            entry = table[code] if code in table else prev + prev[:1]
            table[nxt] = prev + entry[:1]; nxt += 1
            if nxt == 1 << size and size < 12:
                size += 1
        out += entry; prev = entry


def blocks(data):
    out = bytearray()
    for i in range(0, len(data), 255):
        out.append(len(data[i:i + 255])); out += data[i:i + 255]
    return out + b'\0'


def write_gif(pal, frames, delays):
    used = set().union(*[set(f) for f in frames])
    free = [i for i in range(256) if i not in used]
    if not free:
        sys.exit('no unused palette index for transparency')
    t = free[0]
    out = bytearray(b'GIF89a' + SCREEN_W.to_bytes(2, 'little') + SCREEN_H.to_bytes(2, 'little') + bytes([0xf7, 0, 0]) + pal)
    out += bytes([0x21, 0xff, 0x0b]) + b'NETSCAPE2.0' + bytes([3, 1, 0, 0, 0])
    prev = None
    for f, d in zip(frames, delays):
        if prev is None:
            x0, y0, x1, y1 = 0, 0, SCREEN_W, SCREEN_H
        else:
            ys = [y for y in range(SCREEN_H) if f[y * SCREEN_W:(y + 1) * SCREEN_W] != prev[y * SCREEN_W:(y + 1) * SCREEN_W]]
            if not ys:
                ys = [0]
            y0, y1 = ys[0], ys[-1] + 1
            lo, hi = SCREEN_W, 0
            for y in ys:
                a, b = f[y * SCREEN_W:(y + 1) * SCREEN_W], prev[y * SCREEN_W:(y + 1) * SCREEN_W]
                diffs = [x for x in range(SCREEN_W) if a[x] != b[x]]
                if diffs:
                    lo, hi = min(lo, diffs[0]), max(hi, diffs[-1] + 1)
            x0, x1 = (lo, hi) if hi > lo else (0, 1)
        sub = bytearray()
        for y in range(y0, y1):
            row = f[y * SCREEN_W + x0:y * SCREEN_W + x1]
            if prev is None:
                sub += row
            else:
                old = prev[y * SCREEN_W + x0:y * SCREEN_W + x1]
                sub += bytes(t if a == b else a for a, b in zip(row, old))
        out += bytes([0x21, 0xf9, 4, 0x05, d & 255, d >> 8, t, 0])      # keep the previous frame, transparent index t
        out += bytes([0x2c]) + b''.join(v.to_bytes(2, 'little') for v in (x0, y0, x1 - x0, y1 - y0)) + bytes([0, 8])
        data = lzw(bytes(sub))
        if unlzw(data, len(sub)) != bytes(sub):
            sys.exit('internal error: the LZW stream does not round-trip')
        out += blocks(data)
        prev = f
    out.append(0x3b)
    return bytes(out), t


def delays_cs(n, step):
    """Whole-hundredth delays whose running total tracks step/35 s a frame exactly (real time)."""
    ends = [round((k + 1) * step * 100 / 35) for k in range(n)]
    return [ends[0]] + [b - a for a, b in zip(ends, ends[1:])]


def self_check(gif, pal, frames, delays, t):
    """Decodes the file with an independent walk of its blocks and compares each displayed frame."""
    assert gif[:6] == b'GIF89a' and gif[-1] == 0x3b
    at = 13 + 768 + 19
    canvas = bytearray(SCREEN_W * SCREEN_H)
    for f, d in zip(frames, delays):
        assert gif[at:at + 2] == b'\x21\xf9' and int.from_bytes(gif[at + 4:at + 6], 'little') == d
        at += 8
        assert gif[at] == 0x2c
        x0, y0, w, h = (int.from_bytes(gif[at + 1 + 2 * i:at + 3 + 2 * i], 'little') for i in range(4))
        at += 11
        data = bytearray()
        while gif[at]:
            data += gif[at + 1:at + 1 + gif[at]]; at += 1 + gif[at]
        at += 1
        px = unlzw(bytes(data), w * h)
        for y in range(h):
            for x in range(w):
                v = px[y * w + x]
                if v != t:
                    canvas[(y0 + y) * SCREEN_W + x0 + x] = v
        if bytes(canvas) != f:
            sys.exit('internal error: a decoded frame differs from the composed frame')
    assert gif[at] == 0x3b


def generate(work, cargo_target):
    work.mkdir(parents=True, exist_ok=True)
    binary, view = build_tools(work, cargo_target)
    shown = collect(work, binary, view)
    pal = playpal()
    frames = compose(pal, shown)
    delays = delays_cs(len(frames), STEP)
    gif, t = write_gif(pal, frames, delays)
    self_check(gif, pal, frames, delays, t)
    secs = sum(delays) / 100
    print(f'{len(frames)} frames, {SCREEN_W}x{SCREEN_H}, {secs:.2f} s, {len(gif)} bytes, rooms '
          f'{",".join(s[0] for s in SEGMENTS)}; every frame pair byte-identical and equal to its golden')
    if len(gif) >= 2_000_000 or secs >= 15:
        sys.exit('the animation exceeds 2 MB or 15 s')
    return gif


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m5a-readme-visual')
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
