#!/usr/bin/env python3
"""The M5b synthetic graphics PWAD (decision S23): the M5a set plus our own status bar, face and
HUD font lumps. No id Software art: every lump is drawn by this script.

Writes tests/reference/m5b_gfx.wad, loaded like the M5a set (`--wad MAP.wad --wad m5b_gfx.wad
--wad DEMO.lmp`). It imports tools/reference/m5a_gfx.py read-only and appends the 142 lumps the
genuine status bar and HUD load (docs/M5B_REFERENCE_INVESTIGATION.md section 2.4.1) after the last
M5a lump, so every M5a lump keeps its bytes and its lump number, PLAYPAL (14 pairwise different
palettes) and COLORMAP included: every M5a golden keeps its meaning.

The appended lumps, in this order (sizes are ours, chosen from the positions the C fixes):
  STBAR      320x32 opaque (ST_WIDTH x ST_HEIGHT, st_stuff.h:32-33), every column one post, so the
             background buffer screens[4] is fully defined after ST_refreshBackground
             (st_stuff.c:499-511); region bands so a misplaced copy shows.
  STARMS     40x32 opaque, drawn at ST_ARMSBGX,Y 104,168 (st_stuff.c:150-151).
  STTNUM0-9  14x16 with holes (the ready-ammo number is right-aligned at x 44 with 3 digits of
             digit 0's width, st_stuff.c:138-140 and st_lib.c:97-125, so 14 is the widest that keeps
             V_CopyRect inside the screen); STTNUM1 10 wide with leftoffset -2, STTNUM7 topoffset -1
             (one row below the erase rectangle of STlib_drawNum, which uses digit 0's size: a row is
             left behind when the number changes, as the genuine code does).
  STTPRCNT   14x16 with holes.  STTMINUS 6x3, topoffset -6 (loaded, never drawn in single player).
  STYSNUM0-9 4x6 (ammo counters at x 288 and 314, st_stuff.c:178-206).
  STGNUM2-7  5x7, leftoffset 1 (the arms multi-icons erase with the old patch's offsets and size,
             st_lib.c:206-216, when the yellow STYSNUM replaces the grey one).
  STKEYS0-5  7x5 cards, 7x7 skulls (key boxes at y 171, 181, 191).
  STFB0      24x31 (loaded for consoleplayer 0 only, st_stuff.c:1170; STFB1-3 are never loaded).
  42 faces   STFST00-42, STFTR00-40, STFTL00-40, STFOUCH0-4, STFEVL0-4, STFKILL0-4 in ST_loadGraphics
             order (st_stuff.c:1180-1200), then STFGOD0 and STFDEAD0: 24x29 with transparent corners;
             STFOUCHn 24x30 with topoffset -1; STFDEAD0 26x31 with leftoffset 1. The bottom three rows
             encode the face index in binary (6 bits), the eyes and mouth the kind, the shade the pain
             level, so a wrong face shows in the picture.
  STCFN033-095 the HUD font (hu_stuff.c:405-410): a legible 5x7 font of our own with a one-pixel
             shadow, 8 rows high, transparent around the strokes; letters drawn double width, 10
             wide (a space advances 4, hu_lib.c:127-128, so the blue and yellow locked-door
             strings, d_englsh.h:128, 130, reach the `x + w > SCREENWIDTH` break of
             HUlib_drawTextLine, hu_lib.c:120; with 9-wide letters no reachable message does), digits 7 wide,
             punctuation 3 to 6 wide; ',' has topoffset -1.

Status bar and font patches carry no padding (the 2D drawers read each post's own bytes only,
v_video.c:243-261); tools/reference/m5b_check_gfx.py verifies the result independently.

    m5b_gfx.py [--out PATH]      write the PWAD (default tests/reference/m5b_gfx.wad)
    m5b_gfx.py --list            lump counts and sizes
    m5b_gfx.py --check           regenerate in memory, compare with the committed file, then run
                                 the independent verifier tools/reference/m5b_check_gfx.py
"""
import argparse, hashlib, struct, subprocess, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m5a_gfx as G   # noqa: E402  the M5a graphics set (read-only)

OUT = ROOT / 'tests/reference/m5b_gfx.wad'
CHECKER = ROOT / 'tools/reference/m5b_check_gfx.py'


def patch(w, h, pix, left=0, top=0, hole=None):
    return G.patch(w, h, pix, left=left, top=top, hole=hole, pad=0)


# ---------------------------------------------------------------- status bar background ----------
def stbar():
    def pix(x, y):
        if y == 0 or y == 31 or x == 0 or x == 319: return 15
        if x in (104, 143, 168, 236, 280): return 4
        region = 1 if x < 104 else 3 if x < 143 else 13 if x < 168 else 2 if x < 236 else 4 if x < 280 else 6
        return region * 16 + 5 + ((x // 4 + y // 4) & 1)
    return patch(320, 32, pix)


def starms():
    return patch(40, 32, lambda x, y: 15 if (x in (0, 39) or y in (0, 31)) else 8 * 16 + 4 + (y // 8))


# ---------------------------------------------------------------- numbers -------------------------
SEG = {0: 'abcdef', 1: 'bc', 2: 'abged', 3: 'abgcd', 4: 'fgbc', 5: 'afgcd', 6: 'afgedc', 7: 'abc', 8: 'abcdefg', 9: 'abcdfg'}


def segments(d, w, h):
    """on(x, y): the seven-segment strokes of digit d in a w x h box."""
    s, t, mid = SEG[d], max(1, h // 8), h // 2
    def on(x, y):
        return (('a' in s and y < t) or ('d' in s and y >= h - t) or ('g' in s and mid - t // 2 <= y < mid - t // 2 + t)
                or ('f' in s and x < t + 1 and y < mid) or ('b' in s and x >= w - t - 1 and y < mid)
                or ('e' in s and x < t + 1 and y >= mid) or ('c' in s and x >= w - t - 1 and y >= mid))
    return on


def tallnum(d):
    w, left, top = (10, -2, 0) if d == 1 else (14, 0, -1) if d == 7 else (14, 0, 0)
    on, hue = segments(d, w, 16), 1 + d % 15
    return patch(w, 16, lambda x, y: hue * 16 + 15 if on(x, y) else hue * 16 + 2, left=left, top=top,
                 hole=lambda x, y: not on(x, y) and (x + y) % 3 == 0)


def shortnum(d):
    on = segments(d, 4, 6)
    return patch(4, 6, lambda x, y: 4 * 16 + 15 if on(x, y) else 4 * 16 + 1)


def graynum(d):
    on = segments(d, 5, 7)
    return patch(5, 7, lambda x, y: 15 if on(x, y) else 5, left=1)


def percent():
    return patch(14, 16, lambda x, y: 7 * 16 + 15, hole=lambda x, y: abs((x - 7) - (7 - y) * 14 // 16) > 1
                 and not ((x - 3) ** 2 + (y - 3) ** 2 < 5 or (x - 10) ** 2 + (y - 12) ** 2 < 5))


def minus():
    return patch(6, 3, lambda x, y: 1 * 16 + 15, top=-6)


def key(i):
    w, h = (7, 5) if i < 3 else (7, 7)
    hue = (3, 4, 1)[i % 3]
    return patch(w, h, lambda x, y: hue * 16 + (15 if i < 3 else 9) - (x + y) % 3)


def faceback():
    return patch(24, 31, lambda x, y: 2 * 16 + 6)


# ---------------------------------------------------------------- faces ---------------------------
FACE_NAMES = []
for _i in range(5):        # ST_loadGraphics order (st_stuff.c:1180-1200): 3 straight, TR, TL, ouch, evil, kill
    FACE_NAMES += [f'STFST{_i}{_j}' for _j in range(3)] + [f'STFTR{_i}0', f'STFTL{_i}0', f'STFOUCH{_i}', f'STFEVL{_i}', f'STFKILL{_i}']
FACE_NAMES += ['STFGOD0', 'STFDEAD0']
assert len(FACE_NAMES) == 42


def face(idx):
    name = FACE_NAMES[idx]
    w, h, left, top = 24, 29, 0, 0
    if name.startswith('STFOUCH'): h, top = 30, -1
    if name == 'STFDEAD0': w, h, left = 26, 31, 1
    pain = idx // 8 if idx < 40 else 0
    kind = idx % 8 if idx < 40 else 8 + (idx - 40)    # 0-2 straight, 3 right, 4 left, 5 ouch, 6 evil, 7 kill, 8 god, 9 dead
    skin = (13 if kind != 8 else 4) * 16 + 12 - 2 * pain
    eye = 1

    def pix(x, y):
        if y >= h - 3:                                 # the face index in binary, 6 bits, most significant left
            b = x // 4
            return 15 if b < 6 and (idx >> (5 - b)) & 1 else 3
        cx = w // 2 + (-4 if kind == 3 else 4 if kind == 4 else 0)
        px = (-2, 0, 2)[kind] if kind < 3 else 0
        if kind == 9 and 7 <= y <= 11 and (abs(x - (cx - 5)) == abs(y - 9) or abs(x - (cx + 5)) == abs(y - 9)): return eye
        if kind != 9 and 8 <= y <= 10 and (abs(x - (cx - 5 + px)) <= 1 or abs(x - (cx + 5 + px)) <= 1): return eye
        if kind == 7 and y == 6 and abs(x - cx) in range(2, 8): return 1 * 16 + 4
        if kind == 5 and 16 <= y <= 22 and abs(x - cx) <= 3: return 1 * 16 + 2
        if kind == 6 and y in (18, 19) and abs(x - cx) <= 6: return 15
        if kind == 8 and y in (2, 3) and 4 <= x < w - 4: return 4 * 16 + 15
        if kind not in (5, 6) and y == 19 and abs(x - cx) <= 4: return 1 * 16 + 6
        return skin
    return patch(w, h, pix, left=left, top=top, hole=lambda x, y: (x < 3 and y < 3) or (x >= w - 3 and y < 3))


# ---------------------------------------------------------------- the HUD font --------------------
# Our own 5x7 font: seven rows per character, bit 4 the leftmost of five columns.
FONT = {
    '!': (4, 4, 4, 4, 4, 0, 4), '"': (10, 10, 10, 0, 0, 0, 0), '#': (10, 10, 31, 10, 31, 10, 10),
    '$': (4, 15, 20, 14, 5, 30, 4), '%': (24, 25, 2, 4, 8, 19, 3), '&': (12, 18, 20, 8, 21, 18, 13),
    "'": (12, 4, 8, 0, 0, 0, 0), '(': (2, 4, 8, 8, 8, 4, 2), ')': (8, 4, 2, 2, 2, 4, 8),
    '*': (0, 4, 21, 14, 21, 4, 0), '+': (0, 4, 4, 31, 4, 4, 0), ',': (0, 0, 0, 0, 12, 4, 8),
    '-': (0, 0, 0, 31, 0, 0, 0), '.': (0, 0, 0, 0, 0, 12, 12), '/': (0, 1, 2, 4, 8, 16, 0),
    '0': (14, 17, 19, 21, 25, 17, 14), '1': (4, 12, 4, 4, 4, 4, 14), '2': (14, 17, 1, 2, 4, 8, 31),
    '3': (31, 2, 4, 2, 1, 17, 14), '4': (2, 6, 10, 18, 31, 2, 2), '5': (31, 16, 30, 1, 1, 17, 14),
    '6': (6, 8, 16, 30, 17, 17, 14), '7': (31, 1, 2, 4, 8, 8, 8), '8': (14, 17, 17, 14, 17, 17, 14),
    '9': (14, 17, 17, 15, 1, 2, 12), ':': (0, 12, 12, 0, 12, 12, 0), ';': (0, 12, 12, 0, 12, 4, 8),
    '<': (2, 4, 8, 16, 8, 4, 2), '=': (0, 0, 31, 0, 31, 0, 0), '>': (8, 4, 2, 1, 2, 4, 8),
    '?': (14, 17, 1, 2, 4, 0, 4), '@': (14, 17, 1, 13, 21, 21, 14),
    'A': (14, 17, 17, 17, 31, 17, 17), 'B': (30, 17, 17, 30, 17, 17, 30), 'C': (14, 17, 16, 16, 16, 17, 14),
    'D': (28, 18, 17, 17, 17, 18, 28), 'E': (31, 16, 16, 30, 16, 16, 31), 'F': (31, 16, 16, 30, 16, 16, 16),
    'G': (14, 17, 16, 23, 17, 17, 15), 'H': (17, 17, 17, 31, 17, 17, 17), 'I': (14, 4, 4, 4, 4, 4, 14),
    'J': (7, 2, 2, 2, 2, 18, 12), 'K': (17, 18, 20, 24, 20, 18, 17), 'L': (16, 16, 16, 16, 16, 16, 31),
    'M': (17, 27, 21, 21, 17, 17, 17), 'N': (17, 17, 25, 21, 19, 17, 17), 'O': (14, 17, 17, 17, 17, 17, 14),
    'P': (30, 17, 17, 30, 16, 16, 16), 'Q': (14, 17, 17, 17, 21, 18, 13), 'R': (30, 17, 17, 30, 20, 18, 17),
    'S': (15, 16, 16, 14, 1, 1, 30), 'T': (31, 4, 4, 4, 4, 4, 4), 'U': (17, 17, 17, 17, 17, 17, 14),
    'V': (17, 17, 17, 17, 17, 10, 4), 'W': (17, 17, 17, 21, 21, 21, 10), 'X': (17, 17, 10, 4, 10, 17, 17),
    'Y': (17, 17, 17, 10, 4, 4, 4), 'Z': (31, 1, 2, 4, 8, 16, 31), '[': (14, 8, 8, 8, 8, 8, 14),
    '\\': (0, 16, 8, 4, 2, 1, 0), ']': (14, 2, 2, 2, 2, 2, 14), '^': (4, 10, 17, 0, 0, 0, 0),
    '_': (0, 0, 0, 0, 0, 0, 31),
}
assert sorted(map(ord, FONT)) == list(range(33, 96))
INK, SHADOW = 15, 0 * 16 + 1          # white strokes, a near-black shadow one pixel right and below


def glyph(c):
    ch = chr(c)
    rows = FONT[ch]
    used = [x for x in range(5) if any(r >> (4 - x) & 1 for r in rows)]
    x0 = used[0]
    if ch.isalpha():                   # letters: the 5x7 glyph doubled in width, 10 wide
        w = 10
        ink = lambda x, y: 0 <= y < 7 and 0 <= x < 10 and rows[y] >> (4 - x // 2) & 1
    else:
        w = 7 if ch.isdigit() else max(3, used[-1] - x0 + 3)
        ink = lambda x, y: 0 <= y < 7 and 0 <= x + x0 < 5 and rows[y] >> (4 - (x + x0)) & 1

    def pix(x, y):
        return INK if ink(x, y) else SHADOW
    return patch(w, 8, pix, top=-1 if ch == ',' else 0, hole=lambda x, y: not ink(x, y) and not ink(x - 1, y - 1))


def status_lumps():
    out = [('STBAR', stbar()), ('STARMS', starms())]
    out += [(f'STTNUM{d}', tallnum(d)) for d in range(10)]
    out += [('STTPRCNT', percent()), ('STTMINUS', minus())]
    out += [(f'STYSNUM{d}', shortnum(d)) for d in range(10)]
    out += [(f'STGNUM{d}', graynum(d)) for d in range(2, 8)]
    out += [(f'STKEYS{i}', key(i)) for i in range(6)]
    out += [('STFB0', faceback())]
    out += [(n, face(i)) for i, n in enumerate(FACE_NAMES)]
    out += [(f'STCFN{c:03d}', glyph(c)) for c in range(33, 96)]
    assert len(out) == 142
    return out


def lumps():
    return G.lumps() + status_lumps()


def wad_bytes():
    ls = lumps()
    names = [n for n, _ in ls]
    assert len(set(names)) == len(names), 'duplicate lump name'
    data, directory = bytearray(b'PWAD' + struct.pack('<ii', len(ls), 0)), bytearray()
    for n, b in ls:
        directory += struct.pack('<ii8s', len(data), len(b), G.name8(n))
        data += b
    struct.pack_into('<i', data, 8, len(data))
    return bytes(data + directory), ls


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--out', type=Path, default=OUT)
    ap.add_argument('--list', action='store_true')
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    data, ls = wad_bytes()
    new = status_lumps()
    summary = (f'{len(ls)} lumps ({len(ls) - len(new)} M5a + {len(new)} status bar, face and font), {len(data)} bytes, '
               f'sha256 {hashlib.sha256(data).hexdigest()}')
    if a.list:
        print(summary); return
    if a.check:
        if not OUT.exists() or OUT.read_bytes() != data: sys.exit(f'{OUT.relative_to(ROOT)} differs from the regenerated PWAD')
        print(f'{OUT.relative_to(ROOT)} matches: {summary}', flush=True)
        sys.exit(subprocess.run([sys.executable, str(CHECKER), str(OUT)], cwd=ROOT).returncode)
    a.out.write_bytes(data)
    print(f'wrote {a.out}: {summary}')


if __name__ == '__main__':
    main()
