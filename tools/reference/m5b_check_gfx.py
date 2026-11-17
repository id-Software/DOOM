#!/usr/bin/env python3
"""Independent verifier of the M5b synthetic graphics PWAD (stdlib only; it does not import the
generators m5b_gfx.py or m5a_gfx.py).

    m5b_check_gfx.py [PWAD]       default tests/reference/m5b_gfx.wad; exits nonzero on the first
                                  violated rule

Rules, each from the genuine linuxdoom-1.10 source or the committed M5a set:
- PWAD header and directory in bounds; no duplicate lump name.
- The M5a part: the first lumps are exactly the lumps of tests/reference/m5a_gfx.wad, same names,
  same order, same bytes (so PLAYPAL, COLORMAP and every M5a lump number are unchanged), and the
  M5a verifier tools/reference/m5a_check_gfx.py's rules therefore still hold for them.
- Exactly the 142 lumps the genuine status bar and HUD load follow, in this order: STBAR, STARMS,
  STTNUM0-9, STTPRCNT, STTMINUS, STYSNUM0-9, STGNUM2-7, STKEYS0-5, STFB0, the 42 faces in
  ST_loadGraphics order (st_stuff.c:1180-1200: per pain level 0-4 STFST<p>0-2, STFTR<p>0, STFTL<p>0,
  STFOUCH<p>, STFEVL<p>, STFKILL<p>; then STFGOD0, STFDEAD0) and STCFN033-STCFN095 (hu_stuff.c:405-410,
  HU_FONTSTART '!' to HU_FONTEND '_', hu_stuff.h:34-37). Names are rebuilt here from those loops.
- Every patch (patch_t, r_defs.h): header and column offsets inside the lump, every post inside the
  lump, ends with 0xff inside the lump, topdelta + length <= height (no post below the patch).
- STBAR is 320x32 (ST_WIDTH x ST_HEIGHT, st_stuff.h:32-33) with zero offsets and every column one post
  of 32 rows from row 0 (fully opaque, so screens[4] is fully defined after ST_refreshBackground,
  st_stuff.c:499-511).
- Widgets fit the screen at their C positions (V_DrawPatch skips a patch that would leave the screen,
  v_video.c:221-232; V_CopyRect calls I_Error, v_video.c:171-183): the tall numbers at x 44 (ammo,
  3 digits), 90 (health) and 221 (armor), y 171 (st_stuff.c:138-163): the erase rectangle of
  STlib_drawNum (x - 3*w0, w0 and h0 of digit 0, st_lib.c:97-118) inside the bar, and every digit
  and the percent sign drawn inside the screen; the small numbers at x 288 and 314 (st_stuff.c:178-206);
  the arms digits at 111 + 12*(i%3), 172 + 10*(i/3) (st_stuff.c:148-149, 1320-1329), both grey and
  yellow; the keys at 239, 171/181/191; STARMS at 104,168; the faces at 143,168 inside 0..319 x 168..199.
- Font: every glyph has topoffset <= 0 and fits rows 0..199 when drawn at y 0 (HU_MSGY, hu_stuff.h:40);
  letters are 10 wide (with the 4-pixel space of hu_lib.c:127-128 the blue and yellow locked-door strings then reach
  the line break, hu_lib.c:120).
"""
import struct, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
M5A = ROOT / 'tests/reference/m5a_gfx.wad'


def fail(msg):
    sys.exit(f'm5b_check_gfx: FAIL: {msg}')


def need(cond, msg):
    if not cond: fail(msg)


def directory(data):
    need(len(data) >= 12, 'short file')
    ident, n, off = struct.unpack_from('<4sii', data, 0)
    need(ident in (b'PWAD', b'IWAD') and 0 <= off and off + 16 * n <= len(data), 'header')
    out = []
    for k in range(n):
        pos, size, name = struct.unpack_from('<ii8s', data, off + 16 * k)
        need(0 <= pos and pos + size <= len(data), f'lump {k} out of bounds')
        out.append((name.rstrip(b'\0').decode('ascii'), data[pos:pos + size]))
    return out


def names_from_source():
    out = ['STBAR', 'STARMS'] + [f'STTNUM{i}' for i in range(10)] + ['STTPRCNT', 'STTMINUS']
    out += [f'STYSNUM{i}' for i in range(10)] + [f'STGNUM{i + 2}' for i in range(6)] + [f'STKEYS{i}' for i in range(6)]
    out += ['STFB0']                                   # sprintf ("STFB%d", consoleplayer), consoleplayer 0
    for i in range(5):
        out += [f'STFST{i}{j}' for j in range(3)] + [f'STFTR{i}0', f'STFTL{i}0', f'STFOUCH{i}', f'STFEVL{i}', f'STFKILL{i}']
    out += ['STFGOD0', 'STFDEAD0']
    out += [f'STCFN{j:03d}' for j in range(ord('!'), ord('_') + 1)]
    return out


def parse_patch(name, b):
    """(w, h, left, top, columns as lists of (topdelta, length)); every byte read is checked in bounds."""
    need(len(b) >= 8, f'{name}: short header')
    w, h, left, top = struct.unpack_from('<hhhh', b, 0)
    need(w > 0 and h > 0 and len(b) >= 8 + 4 * w, f'{name}: header or column table outside the lump')
    cols = []
    for x in range(w):
        pos = struct.unpack_from('<i', b, 8 + 4 * x)[0]
        posts = []
        while True:
            need(0 <= pos < len(b), f'{name}: column {x} runs outside the lump')
            if b[pos] == 0xff: break
            need(pos + 1 < len(b), f'{name}: column {x} post header outside the lump')
            td, ln = b[pos], b[pos + 1]
            need(pos + 4 + ln <= len(b), f'{name}: column {x} post data outside the lump')
            need(td + ln <= h, f'{name}: column {x} post below the patch')
            posts.append((td, ln))
            pos += ln + 4
        cols.append(posts)
    return w, h, left, top, cols


def inside(name, p, x, y, x0=0, y0=0, x1=320, y1=200):
    w, h, left, top, _ = p
    x -= left; y -= top
    need(x0 <= x and x + w <= x1 and y0 <= y and y + h <= y1, f'{name} at {x + left},{y + top} leaves {x0}..{x1 - 1} x {y0}..{y1 - 1}')


def main():
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / 'tests/reference/m5b_gfx.wad'
    lumps = directory(path.read_bytes())
    names = [n for n, _ in lumps]
    need(len(set(names)) == len(names), 'duplicate lump name')
    base = directory(M5A.read_bytes())
    need(lumps[:len(base)] == base, 'the M5a part differs from tests/reference/m5a_gfx.wad (names, order or bytes)')
    want = names_from_source()
    need(len(want) == 142 and names[len(base):] == want, f'the appended lumps are not the {len(want)} status bar and HUD lumps in order')
    P = {n: parse_patch(n, b) for n, b in lumps[len(base):]}
    sb = P['STBAR']
    need(sb[:4] == (320, 32, 0, 0) and all(c == [(0, 32)] for c in sb[4]), 'STBAR is not 320x32 opaque with one post per column')
    inside('STARMS', P['STARMS'], 104, 168, y0=168)
    tall = [P[f'STTNUM{i}'] for i in range(10)]
    w0, h0 = tall[0][0], tall[0][1]
    for x in (44, 90, 221):                            # ammo, health, armor (st_stuff.c:139, 144, 162)
        need(x - 3 * w0 >= 0 and 171 - 168 >= 0 and 171 + h0 <= 200, f'drawNum erase rectangle at x {x} leaves the bar')
        for k in range(1, 4):
            for d, p in enumerate(tall): inside(f'STTNUM{d}', p, x - k * w0, 171, y0=168)
    for x in (90, 221): inside('STTPRCNT', P['STTPRCNT'], x, 171, y0=168)
    for k in (1, 2):                                   # a negative number keeps at most 2 digits (st_lib.c:105-113), minus at x - 8
        inside('STTMINUS', P['STTMINUS'], 44 - k * w0 - 8, 171, y0=168)
    small = [P[f'STYSNUM{i}'] for i in range(10)]
    sw = small[0][0]
    for x in (288, 314):
        for y in (173, 179, 185, 191):
            need(x - 3 * sw >= 0 and y + small[0][1] <= 200, f'small number erase rectangle at {x},{y}')
            for k in range(1, 4):
                for d, p in enumerate(small): inside(f'STYSNUM{d}', p, x - k * sw, y, y0=168)
    for i in range(6):
        x, y = 111 + (i % 3) * 12, 172 + (i // 3) * 10
        inside(f'STGNUM{i + 2}', P[f'STGNUM{i + 2}'], x, y, y0=168)
        inside(f'STYSNUM{i + 2}', P[f'STYSNUM{i + 2}'], x, y, y0=168)
    for i in range(6):
        for y in (171, 181, 191): inside(f'STKEYS{i}', P[f'STKEYS{i}'], 239, y, y0=168)
    inside('STFB0', P['STFB0'], 143, 0, y1=32)        # drawn into BG at ST_FX,0 in a netgame only (st_stuff.c:506-507)
    faces = want[want.index('STFST00'):want.index('STFDEAD0') + 1]
    need(len(faces) == 42, 'face count')
    for n in faces: inside(n, P[n], 143, 168, y0=168)
    for c in range(ord('!'), ord('_') + 1):
        n = f'STCFN{c:03d}'
        need(P[n][3] <= 0, f'{n}: topoffset > 0')
        inside(n, P[n], 0, 0)
        if chr(c).isalpha(): need(P[n][0] == 10, f'{n}: a letter is not 10 wide')
    print(f'm5b_check_gfx: OK: {path.name}: {len(base)} M5a lumps byte-identical, {len(want)} status bar and HUD lumps, '
          f'{len(faces)} faces, {ord("_") - ord("!") + 1} glyphs')


if __name__ == '__main__':
    main()
