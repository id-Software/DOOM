#!/usr/bin/env python3
"""The M5a synthetic graphics PWAD (decision R13): our own palette, colour maps, wall patches,
textures, flats and sprites for the genuine linuxdoom-1.10 renderer. No id Software art.

Writes tests/reference/m5a_gfx.wad, a PWAD loaded after a map WAD and before the demo lump
(`--wad MAP.wad --wad m5a_gfx.wad --wad DEMO.lmp`). Name lookups scan the directory backwards
(W_CheckNumForName, w_wad.c:351-392) and R_InitTextures, R_InitFlats and R_InitSpriteLumps use
the last PNAMES, TEXTURE1, F_START/F_END and S_START/S_END (r_data.c:448, 463, 585-586,
608-609), so this PWAD replaces the map WAD's graphics without touching any frozen WAD.

Gameplay identity (the hard rule of docs/M5A_REFERENCE_INVESTIGATION.md section 3.5): TEXTURE1
starts with tools/reference/m3h9c_wad.py's TEXTURES in the same order with the same heights
(texture numbers are dumped in L rows and switch indices derive from them; textureheight is read
by P_FindShortestTextureAround, p_floor.c:386-395); SKY1..SKY3 are 256 wide (the width is read
by the renderer only). The flats start with m3h9c_wad.FLATS in the same order (S.floorpic is
lump - firstflat). New textures and flats are appended. No animdefs start name (p_spec.c:101-132)
is added: the inherited NUKAGE1..3 are the only animation, as in every frozen WAD.

Lumps:
  PLAYPAL   14 x 768: 16 hue ramps x 16 levels, index = hue*16 + level, level 0 black and 15
            the full colour; palettes 1-13 blend toward red, yellow or green by our own rule.
            Not read by the renderer (only by I_SetPalette); used to export PNGs.
  COLORMAP  34 x 256 (R_InitColormaps, r_data.c:629-643): map m < 32 maps hue*16+l to
            hue*16+max(0, l - m//2); map 32 (INVERSECOLORMAP, p_user.c:41) is the inverted grey
            ramp 15 - l for every colour; map 33 is all 0.
  PNAMES, TEXTURE1 and the wall patches (see TEXTURES below): single- and multi-patch,
            composite columns (overlapping patches), a 256-tall texture, masked textures with
            holes, every switch texture P_InitSwitchList needs for episodes 1-2
            (p_switch.c:107-148), SKY1..3 at 256x128, and three deliberately short or gappy
            textures for the read-provenance tracker (M5W72, M5C64, M5GAP).
  F_START .. F_END   64x64 flats, F_SKY1 included (g_game.c:454).
  S_START .. S_END   for every sprite name that a state of info.c uses, every frame from A to
            the highest frame letter any state uses (FF_FULLBRIGHT masked off; r_things.c:255
            requires them all); the player and the monsters of Episode 1 have 8 rotations for
            their walking, attack and pain frames (mirrored pairs such as POSSA2A8, except TROO,
            which has 8 separate lumps per frame), every other frame and sprite rotation 0.

Padding: every patch lump except the tracker's short patches ends with PAD zero bytes, so the
`& 127` row index of R_DrawColumn (r_draw.c:142) from any post's first data byte stays inside
the lump (defined by these bytes). Each patch encodes what it is visibly: hue per texture or
sprite, a white top row and 16-row rulers on walls, rotation bars and a left-pointing notch on
sprites so a wrong rotation or a mirror shows in the image.

    m5a_gfx.py [--out PATH]      write the PWAD (default tests/reference/m5a_gfx.wad)
    m5a_gfx.py --list            lump counts and sizes
    m5a_gfx.py --check           regenerate in memory, compare with the committed file, then run
                                 the independent verifier tools/reference/m5a_check_gfx.py
"""
import argparse, hashlib, re, struct, subprocess, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m3h9c_wad as W   # noqa: E402  frozen M3h9c texture and flat order (read-only)

OUT = ROOT / 'tests/reference/m5a_gfx.wad'
CHECKER = ROOT / 'tools/reference/m5a_check_gfx.py'
PAD = 128


def name8(s):
    return s.upper().encode('ascii').ljust(8, b'\0')


# ---------------------------------------------------------------- palette and colour maps --------
HUES = [(1, 1, 1), (1, 0, 0), (0, 1, 0), (0, 0, 1), (1, 1, 0), (1, 0, 1), (0, 1, 1), (1, .5, 0),
        (.5, 0, 1), (0, 1, .5), (1, 0, .5), (.5, 1, 0), (0, .5, 1), (.6, .4, .2), (.4, .6, .6), (1, .8, .6)]


def playpal():
    base = [tuple(int(round(255 * c * l / 15)) for c in h) for h in HUES for l in range(16)]
    out = bytearray()
    for p in range(14):
        tint, k = ((0, 0, 0), 0) if p == 0 else ((255, 0, 0), p) if p < 9 else ((255, 255, 0), p - 8) if p < 13 else ((0, 255, 0), 4)
        for rgb in base:
            out += bytes((c * (12 - k) + t * k) // 12 for c, t in zip(rgb, tint))
    return bytes(out)


def colormap():
    out = bytearray()
    for m in range(32):
        out += bytes(h * 16 + max(0, l - m // 2) for h in range(16) for l in range(16))
    out += bytes(15 - (i % 16) for i in range(256))
    out += bytes(256)
    return bytes(out)


# ---------------------------------------------------------------- patches -----------------------
def patch(w, h, pix, left=0, top=0, hole=None, pad=PAD):
    """patch_t (r_defs.h): w, h, leftoffset, topoffset, columnofs[w], then per column its posts
    (topdelta, length, pad byte, data, pad byte) and 0xFF. pix(x, y) -> index; hole(x, y) -> True
    for a transparent texel. Posts are at most 128 rows (topdelta stays below 255)."""
    cols = []
    for x in range(w):
        posts, y = bytearray(), 0
        while y < h:
            if hole and hole(x, y): y += 1; continue
            y0 = y
            while y < h and not (hole and hole(x, y)) and y - y0 < 128: y += 1
            data = bytes(pix(x, yy) for yy in range(y0, y))
            assert y0 < 255
            posts += bytes([y0, len(data), data[0]]) + data + bytes([data[-1]])
        cols.append(bytes(posts) + b'\xff')
    off, ofs = 8 + 4 * w, []
    for c in cols: ofs.append(off); off += len(c)
    return struct.pack('<hhhh', w, h, left, top) + struct.pack('<%di' % w, *ofs) + b''.join(cols) + bytes(pad)


def wallpix(hue, w, h):
    def pix(x, y):
        if y == 0: return 15                          # white top row
        if y == h - 1: return 0 * 16 + 6              # grey bottom row
        if x == 0: return 0 * 16 + 11                 # grey left column
        if y % 16 == 0: return hue * 16 + 15          # 16-row ruler
        if x % 16 == 0: return hue * 16 + 13
        lvl = 8 + ((x // 8 + y // 8) & 1) * 2 + (y * 3 // h)
        return hue * 16 + lvl
    return pix


# name -> (width, height, hue, kind); kind 'wall' solid, 'mask' with holes, 'short' unpadded
PATCH_SPECS = [
    ('P_TESTW', 64, 128, 13, 'wall'), ('P_STARTN', 64, 128, 14, 'wall'), ('P_BIGDR', 64, 128, 7, 'wall'),
    ('P_DTRAK', 64, 128, 13, 'wall'), ('P_PLAT1', 64, 128, 4, 'wall'), ('P_SUPP2', 64, 128, 15, 'wall'),
    ('P_SW1', 64, 128, 9, 'wall'), ('P_SW2', 64, 128, 10, 'wall'),
    ('P_WALLA', 64, 128, 1, 'wall'), ('P_WALLB', 64, 128, 2, 'wall'), ('P_WALLC', 64, 128, 3, 'wall'),
    ('P_WALLD', 64, 128, 5, 'wall'), ('P_WALLE', 64, 128, 11, 'wall'), ('P_WALLF', 64, 128, 12, 'wall'),
    ('P_DECAL', 16, 16, 4, 'wall'), ('P_TALL', 64, 256, 6, 'wall'),
    ('P_GRATE', 64, 128, 8, 'mask'), ('P_FENCE', 64, 64, 10, 'mask'),
    ('P_W72', 64, 72, 2, 'short'), ('P_C64A', 64, 64, 5, 'short'), ('P_C64B', 32, 32, 11, 'short'),
    ('P_GAPA', 64, 48, 6, 'short'), ('P_GAPB', 32, 40, 7, 'short'),
]


def sky_patch():
    def pix(x, y):
        if y % 32 == 0: return 0 * 16 + 14
        return 12 * 16 + 4 + (y * 10 // 128) + (1 if (x // 32) & 1 else 0)
    return patch(256, 128, pix)


def make_patch(name, w, h, hue, kind):
    if kind == 'mask':
        r = min(w, h) // 5
        hole = lambda x, y: ((x % (w // 2)) - w // 4) ** 2 + ((y % (h // 2)) - h // 4) ** 2 < r * r or (x % 16 == 7 and y > h // 2)
        return patch(w, h, wallpix(hue, w, h), hole=hole)
    return patch(w, h, wallpix(hue, w, h), pad=0 if kind == 'short' else PAD)


PATCHES = [(n, make_patch(n, w, h, hue, k)) for n, w, h, hue, k in PATCH_SPECS] + [('P_SKY', sky_patch())]
PNAMES = [n for n, _ in PATCHES]

# Textures: (name, width, height, [(originx, originy, patch name)]).
INHERITED = {'TESTWALL': 'P_TESTW', 'STARTAN2': 'P_STARTN', 'BIGDOOR2': 'P_BIGDR', 'DOORTRAK': 'P_DTRAK',
             'PLAT1': 'P_PLAT1', 'SUPPORT2': 'P_SUPP2'}


def textures():
    out = []
    for t in W.TEXTURES:
        if t.startswith('SKY'): out.append((t, 256, 128, [(0, 0, 'P_SKY')]))
        elif t.startswith('SW1'): out.append((t, W.TEX_W, W.TEX_H, [(0, 0, 'P_SW1')]))
        elif t.startswith('SW2'): out.append((t, W.TEX_W, W.TEX_H, [(0, 0, 'P_SW2')]))
        else: out.append((t, W.TEX_W, W.TEX_H, [(0, 0, INHERITED[t])]))
    out += [
        ('M5WALLA', 64, 128, [(0, 0, 'P_WALLA')]),
        ('M5WALLB', 64, 128, [(0, 0, 'P_WALLB')]),
        ('M5WALLC', 128, 128, [(0, 0, 'P_WALLC'), (64, 0, 'P_WALLD')]),      # two patches side by side
        ('M5WALLE', 64, 128, [(0, 0, 'P_WALLE')]),
        ('M5WALLF', 64, 128, [(0, 0, 'P_WALLF')]),
        ('M5COMP', 64, 128, [(0, 0, 'P_WALLB'), (24, 40, 'P_DECAL'), (40, 100, 'P_DECAL')]),   # composite columns
        ('M5TALL', 64, 256, [(0, 0, 'P_TALL')]),
        ('M5WIDE', 96, 128, [(0, 0, 'P_WALLE'), (64, 0, 'P_WALLF')]),        # non-power-of-two width (96)
        ('M5GRATE', 64, 128, [(0, 0, 'P_GRATE')]),
        ('M5FENCE', 64, 64, [(0, 0, 'P_FENCE')]),
        ('M5W72', 64, 72, [(0, 0, 'P_W72')]),                                # short, unpadded single patch
        ('M5C64', 64, 64, [(0, 0, 'P_C64A'), (16, 16, 'P_C64B')]),            # short composite
        ('M5GAP', 64, 128, [(0, 0, 'P_GAPA'), (16, 80, 'P_GAPB')]),           # composite with unwritten rows
    ]
    return out


def texture1():
    defs = textures()
    body, offs, base = b'', [], 4 + 4 * len(defs)
    for t, w, h, ps in defs:
        offs.append(base + len(body))
        body += name8(t) + struct.pack('<ihhih', 0, w, h, 0, len(ps))
        body += b''.join(struct.pack('<hhhhh', x, y, PNAMES.index(p), 1, 0) for x, y, p in ps)
    return struct.pack('<i', len(defs)) + struct.pack('<%di' % len(defs), *offs) + body


def pnames():
    return struct.pack('<i', len(PNAMES)) + b''.join(name8(n) for n in PNAMES)


# ---------------------------------------------------------------- flats -------------------------
NEW_FLATS = ['M5FLAT1', 'M5FLAT2', 'M5FLAT3', 'M5FLAT4', 'M5FLAT5', 'M5FLAT6', 'M5FLAT7', 'M5FLAT8']


def flat(k, name):
    hue = 1 + k % 15
    if name == 'F_SKY1':
        return bytes(12 * 16 + 8 for _ in range(4096))
    def pix(x, y):
        if x == 0 or y == 0: return 15
        if x % 16 == 0 or y % 16 == 0: return hue * 16 + 14
        return hue * 16 + 6 + ((x // 8 + y // 8) & 1) * 3 + (x * 3 // 64)
    return bytes(pix(x, y) for y in range(64) for x in range(64))


FLATS = list(W.FLATS) + NEW_FLATS


# ---------------------------------------------------------------- sprites -----------------------
def sprite_frames():
    """(sprnames in order, {name: highest frame index used by any state})."""
    src = (ROOT / 'linuxdoom-1.10/info.c').read_text(encoding='latin-1')
    a = src.index('char *sprnames[NUMSPRITES]')
    names = re.findall(r'"(\w{4})"', src[a:src.index('};', a)])
    st = src[src.index('state_t\tstates[NUMSTATES]'):]
    st = st[:st.index('};')]
    mx = {}
    for spr, fr in re.findall(r'\{SPR_(\w{4}),(\d+),', st):
        f = int(fr) & 0x7fff
        mx[spr] = max(mx.get(spr, -1), f)
    return names, mx


# rotated frames: the frames up to and including the pain frame of each walking actor
ROTATED = {'PLAY': 6, 'POSS': 6, 'SPOS': 6, 'CPOS': 6, 'TROO': 7, 'SARG': 7, 'BOSS': 7}
SEPARATE = {'TROO'}             # 8 lumps per rotated frame (no mirrored pair)
WEAPONS = {'PUNG', 'PISG', 'SHTG', 'SHT2', 'CHGG', 'MISG', 'SAWG', 'PLSG', 'BFGG'}
FLASHES = {'PISF', 'SHTF', 'CHGF', 'MISF', 'PLSF', 'BFGF'}
SMALL = {'BLUD', 'PUFF', 'BAL1', 'BAL2', 'PLSS', 'PLSE', 'MISL', 'BAL7', 'FATB', 'MANF', 'APLS', 'BFS1'}


def sprite_patch(si, frame, rot, kind):
    hue = 1 + si % 15
    if kind == 'weapon': w, h = 64, 40
    elif kind == 'flash': w, h = 32, 24
    elif kind == 'actor': w, h = 28, 44
    elif kind == 'small': w, h = 12, 12
    else: w, h = 16, 22
    def pix(x, y):
        if (rot or 0) and y < 6 and (rot - 1) * w // 8 <= x < rot * w // 8: return 15          # rotation bar
        if y >= h - 1 - frame % 8 * 2 and x % 4 == 0: return 0 * 16 + 13                     # frame ticks
        if x < w // 4 and abs(y - h // 3) < x // 2 + 1: return hue * 16 + 15                 # left-pointing notch
        return hue * 16 + 7 + ((x + y) // 3) % 3 + (frame // 8) * 3
    hole = (lambda x, y: (x - w // 2) ** 2 + (y - h // 2) ** 2 < (w // 6) ** 2) if kind in ('actor', 'weapon', 'other') else None
    if kind == 'weapon': left, top = w // 2 - 159, h - 136
    elif kind == 'flash': left, top = w // 2 - 159, 32 - (168 - 40 - h)
    else: left, top = w // 2, h - 4
    return patch(w, h, pix, left=left, top=top, hole=hole)


def sprites():
    names, mx = sprite_frames()
    out = []
    for si, n in enumerate(names):
        if n not in mx: continue
        kind = ('weapon' if n in WEAPONS else 'flash' if n in FLASHES else 'actor' if n in ROTATED
                else 'small' if n in SMALL else 'other')
        for f in range(mx[n] + 1):
            L = chr(65 + f)
            if f <= ROTATED.get(n, -1):
                if n in SEPARATE:
                    out += [(f'{n}{L}{r}', sprite_patch(si, f, r, kind)) for r in range(1, 9)]
                else:
                    out += [(f'{n}{L}1', sprite_patch(si, f, 1, kind)), (f'{n}{L}2{L}8', sprite_patch(si, f, 2, kind)),
                            (f'{n}{L}3{L}7', sprite_patch(si, f, 3, kind)), (f'{n}{L}4{L}6', sprite_patch(si, f, 4, kind)),
                            (f'{n}{L}5', sprite_patch(si, f, 5, kind))]
            else:
                out.append((f'{n}{L}0', sprite_patch(si, f, 0, kind)))
    return out


# ---------------------------------------------------------------- the PWAD ----------------------
def lumps():
    out = [('PLAYPAL', playpal()), ('COLORMAP', colormap()), ('S_START', b'')] + sprites() + [('S_END', b''), ('F_START', b'')]
    out += [(f, flat(k, f)) for k, f in enumerate(FLATS)] + [('F_END', b''), ('PNAMES', pnames()), ('TEXTURE1', texture1())]
    out += PATCHES
    return out


def wad_bytes():
    ls = lumps()
    names = [n for n, _ in ls]
    assert len(set(names)) == len(names), 'duplicate lump name'
    data, directory = bytearray(b'PWAD' + struct.pack('<ii', len(ls), 0)), bytearray()
    for n, b in ls:
        directory += struct.pack('<ii8s', len(data), len(b), name8(n))
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
    s0 = [n for n, _ in ls].index('S_START'); s1 = [n for n, _ in ls].index('S_END')
    summary = (f'{len(ls)} lumps, {len(data)} bytes, sha256 {hashlib.sha256(data).hexdigest()}: {s1 - s0 - 1} sprite lumps, '
               f'{len(FLATS)} flats, {len(textures())} textures, {len(PATCHES)} wall patches')
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
