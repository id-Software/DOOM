#!/usr/bin/env python3
"""Independent verifier of the M5a synthetic graphics PWAD (stdlib only; it does not import the
generator m5a_gfx.py or m3h9c_wad.py).

    m5a_check_gfx.py [PWAD]       default tests/reference/m5a_gfx.wad; exits nonzero on the first
                                  violated rule

Rules, each from the genuine linuxdoom-1.10 source or the frozen synthetic WAD:
- PWAD header and directory in bounds; no duplicate lump name.
- PLAYPAL is 14 x 768 bytes; COLORMAP is 34 x 256 (R_InitColormaps, r_data.c:629-643) and follows
  the declared rule: map m < 32 sends hue*16+l to hue*16+max(0, l-m//2), map 32 is 15-(i%16), map
  33 is 0.
- TEXTURE1 parses (r_data.c:505-526 layout, 22 + 10*patchcount bytes per texture); every patch index
  is inside PNAMES and every PNAMES name is a lump of this PWAD (r_data.c:544); no TEXTURE2.
- Gameplay identity: the first textures are exactly the names of tests/reference/m3h9c_synthetic.wad's
  TEXTURE1 in order with the same heights; the flats between F_START and F_END start with that WAD's
  flats in order; every flat is 4,096 bytes; F_SKY1 is a flat (g_game.c:454).
- Every switch name of alphSwitchList with episode 1 or 2 (p_switch.c:46-90, read from the source)
  is a texture (P_InitSwitchList, p_switch.c:107-148).
- SKY1 is at least 128 tall and every column has exactly one patch (the sky is drawn with `& 127`).
- No animdefs start name of p_spec.c:101-132 is a lump or texture except NUKAGE1, which the frozen
  WADs already carry.
- Sprites (R_InitSpriteDefs, r_things.c:180-276): for every sprite name a state of info.c uses, the
  lumps between S_START and S_END give every frame from A to the highest frame letter any state
  uses, a frame is either rotation 0 alone or all 8 rotations, no rotation twice, frame letter < 29,
  rotation <= 8 (r_things.c:113-160).
- Patches: every column offset and post lies inside its lump; every post starts below row 255; and
  every patch lump except the tracker's short patches (P_W72, P_C64A, P_C64B, P_GAPA, P_GAPB) holds
  at least 128 bytes from each post's first data byte to its end, so R_DrawColumn's `& 127` row
  index (r_draw.c:142) stays inside the lump.
"""
import re, struct, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / 'linuxdoom-1.10'
SHORT = {'P_W72', 'P_C64A', 'P_C64B', 'P_GAPA', 'P_GAPB'}


def fail(msg):
    sys.exit(f'm5a_check_gfx: FAIL: {msg}')


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


def texture1(raw):
    n = struct.unpack_from('<i', raw, 0)[0]
    offs = struct.unpack_from('<%di' % n, raw, 4)
    out = []
    for o in offs:
        need(0 <= o and o + 22 <= len(raw), 'TEXTURE1 offset')
        name = raw[o:o + 8].rstrip(b'\0').decode('ascii')
        w, h = struct.unpack_from('<hh', raw, o + 12)
        pc = struct.unpack_from('<h', raw, o + 20)[0]
        need(o + 22 + 10 * pc <= len(raw), f'{name}: patches out of bounds')
        ps = [struct.unpack_from('<hhhhh', raw, o + 22 + 10 * k) for k in range(pc)]
        out.append((name, w, h, ps))
    return out


def pnames(raw):
    n = struct.unpack_from('<i', raw, 0)[0]
    return [raw[4 + 8 * k:12 + 8 * k].rstrip(b'\0').decode('ascii').upper() for k in range(n)]


def posts(name, lump):
    """Yield (column, post data offset, topdelta, length) of a patch lump, checking bounds."""
    need(len(lump) >= 8, f'{name}: short patch')
    w, h = struct.unpack_from('<hh', lump, 0)
    need(0 < w <= 1024 and 0 < h, f'{name}: size')
    ofs = struct.unpack_from('<%di' % w, lump, 8)
    for x, o in enumerate(ofs):
        need(8 + 4 * w <= o < len(lump), f'{name}: column {x} offset')
        while True:
            need(o < len(lump), f'{name}: column {x} runs past the lump')
            top = lump[o]
            if top == 0xff: break
            need(o + 2 < len(lump), f'{name}: post header')
            length = lump[o + 1]
            need(o + 4 + length <= len(lump), f'{name}: post data')
            yield x, o + 3, top, length
            o += length + 4


def main():
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / 'tests/reference/m5a_gfx.wad'
    lumps = directory(path.read_bytes())
    names = [n for n, _ in lumps]
    need(len(set(names)) == len(names), 'duplicate lump name')
    by = dict(lumps)
    need(len(by['PLAYPAL']) == 14 * 768, 'PLAYPAL size')
    cm = by['COLORMAP']
    need(len(cm) == 34 * 256, 'COLORMAP size')
    for m in range(32):
        for i in range(256):
            need(cm[m * 256 + i] == (i // 16) * 16 + max(0, i % 16 - m // 2), f'COLORMAP map {m} entry {i}')
    need(cm[32 * 256:33 * 256] == bytes(15 - (i % 16) for i in range(256)), 'COLORMAP map 32')
    need(cm[33 * 256:] == bytes(256), 'COLORMAP map 33')
    need('TEXTURE2' not in by, 'TEXTURE2 present')
    tex = texture1(by['TEXTURE1'])
    pn = pnames(by['PNAMES'])
    for p in pn: need(p in by, f'PNAMES {p} is not a lump')
    for t, w, h, ps in tex:
        need(ps, f'{t}: no patch')
        for x, y, p, _, _ in ps: need(0 <= p < len(pn), f'{t}: patch index {p}')
    # gameplay identity against the frozen synthetic WAD
    frozen = dict(directory((ROOT / 'tests/reference/m3h9c_synthetic.wad').read_bytes()))
    ftex = texture1(frozen['TEXTURE1'])
    need([(t, h) for t, _, h, _ in tex[:len(ftex)]] == [(t, h) for t, _, h, _ in ftex], 'inherited textures differ in name, order or height')
    def flats_of(ls):
        n = [x for x, _ in ls]
        a, b = len(n) - 1 - n[::-1].index('F_START'), len(n) - 1 - n[::-1].index('F_END')
        return ls[a + 1:b]
    fl = flats_of(lumps)
    ffl = flats_of(directory((ROOT / 'tests/reference/m3h9c_synthetic.wad').read_bytes()))
    need([n for n, _ in fl[:len(ffl)]] == [n for n, _ in ffl], 'inherited flats differ')
    for n, b in fl: need(len(b) == 4096, f'flat {n} size')
    need('F_SKY1' in [n for n, _ in fl], 'F_SKY1 missing')
    # switches
    sw = (SRC / 'p_switch.c').read_text(encoding='latin-1')
    tnames = {t for t, _, _, _ in tex}
    for a, b, ep in re.findall(r'\{"(\w+)",\s*"(\w+)",\s*(\d)\}', sw):
        if int(ep) <= 2:
            need(a in tnames and b in tnames, f'switch {a}/{b} missing')
    # sky
    sky = next(x for x in tex if x[0] == 'SKY1')
    need(sky[2] >= 128, 'SKY1 shorter than 128')
    cover = [0] * sky[1]
    for x, y, p, _, _ in sky[3]:
        pw = struct.unpack_from('<h', by[pn[p]], 0)[0]
        for c in range(max(0, x), min(sky[1], x + pw)): cover[c] += 1
    need(all(c == 1 for c in cover), 'SKY1 columns not single-patch')
    # animdefs
    sp = (SRC / 'p_spec.c').read_text(encoding='latin-1')
    a = sp.index('animdef_t\t\tanimdefs[]')
    for _, last, first in re.findall(r'\{(true|false),\s*"(\w+)",\s*"(\w+)"', sp[a:sp.index('};', a)]):
        if first == 'NUKAGE1': continue
        need(first not in by and first not in tnames, f'animdefs start name {first} present')
    # sprites
    info = (SRC / 'info.c').read_text(encoding='latin-1')
    a = info.index('char *sprnames[NUMSPRITES]')
    sprn = re.findall(r'"(\w{4})"', info[a:info.index('};', a)])
    st = info[info.index('state_t\tstates[NUMSTATES]'):]
    st = st[:st.index('};')]
    used = {}
    for s, f in re.findall(r'\{SPR_(\w{4}),(\d+),', st):
        used[s] = max(used.get(s, -1), int(f) & 0x7fff)
    s0 = len(names) - 1 - names[::-1].index('S_START'); s1 = len(names) - 1 - names[::-1].index('S_END')
    slumps = names[s0 + 1:s1]
    nframes = 0
    for s in sprn:
        rot = {}
        def install(frame, r, nm):
            need(frame < 29 and r <= 8, f'{nm}: frame or rotation out of range')
            cur = rot.setdefault(frame, {})
            if r == 0:
                need(not cur, f'{nm}: rotation 0 mixed with rotations'); cur[0] = nm
            else:
                need(0 not in cur and r not in cur, f'{nm}: rotation repeated or mixed'); cur[r] = nm
        for nm in slumps:
            if nm[:4] != s: continue
            install(ord(nm[4]) - 65, ord(nm[5]) - 48, nm)
            if len(nm) > 6: install(ord(nm[6]) - 65, ord(nm[7]) - 48, nm)
        if s in used:
            for f in range(used[s] + 1):
                need(f in rot, f'{s} frame {chr(65 + f)} missing')
        for f, r in rot.items():
            need(set(r) == {0} or set(r) == set(range(1, 9)), f'{s} frame {chr(65 + f)} rotations incomplete')
        if rot: need(max(rot) + 1 == len(rot), f'{s}: frames not contiguous')
        nframes += len(rot)
    # patches: bounds and padding
    patch_lumps = set(pn) | set(slumps)
    checked = 0
    for nm in sorted(patch_lumps):
        lump = by[nm]
        for x, d, top, length in posts(nm, lump):
            need(top < 255, f'{nm}: topdelta')
            if nm not in SHORT: need(d + 128 <= len(lump), f'{nm}: column {x} post at {d} has fewer than 128 bytes to the lump end')
        checked += 1
    print(f'{path.name}: ok: {len(lumps)} lumps, {len(tex)} textures ({len(ftex)} inherited), {len(fl)} flats '
          f'({len(ffl)} inherited), {len(slumps)} sprite lumps, {nframes} sprite frames, {checked} patches checked')


if __name__ == '__main__':
    main()
