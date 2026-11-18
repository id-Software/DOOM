#!/usr/bin/env python3
"""Independent verifier of the M5c synthetic sound PWADs (stdlib only; it does not import the
generator m5c_sfx.py: the names come from its own parse of linuxdoom-1.10/sounds.c and the edge
rules are written out here from section 3.3 of docs/M5C_REFERENCE_INVESTIGATION.md).

    m5c_check_sfx.py [DIR]        default tests/reference; checks m5c_sfx.wad, m5c_sfx_edge.wad and
                                  m5c_music.wad and exits nonzero on the first violated rule
    m5c_check_sfx.py --self-test  plants one defect at a time into in-memory copies of the
                                  committed files and requires the verifier to reject each

Rules:
- PWAD: "PWAD" magic, directory and every lump inside the file, no duplicate lump name.
- Main set: exactly "DS" + name for every S_sfx entry 1..108 that has no link (sounds.c:114-232;
  I_InitSound loads exactly these, i_sound.c:799-813), in S_sfx order: 107 lumps, so no start
  can reach the dspistol fallback (i_sound.c:213-216) or I_GetSfxLumpNum's I_Error
  (i_sound.c:451-456, w_wad.c:399-409). Each lump: at least 8 + 2 bytes, header u16 3, u16 11025,
  u32 count == length - 8, samples 0 and 1 equal to the sfx index n and 255 - n; pistol (1) 512
  samples and shotgn (2) 1024 (exact multiples of SAMPLECOUNT 512, i_sound.c:233); all 107 lumps
  pairwise different.
- Edge set: the main set's names without DSPSTART plus DSCHGUN; every lump not named below is
  byte-identical to the main set's; DSDBOPN 0 bytes, DSDBCLS 5 bytes, DSDBLOAD 8 bytes (header,
  zero samples), DSPLASMA 1 sample, DSBFG 511, DSSAWUP 512, DSSAWIDL 513, DSSAWFUL 1024, DSRLAUNC 600
  samples all 255, DSRXPLOD 600 samples all 0, DSFIRSHT header rate 22050, DSFIRXPL header count
  not equal to its sample count, DSCHGUN present; together these give sample counts that are and
  are not multiples of 512 and lumps of 8 bytes and shorter.
- Music set: exactly D_E1M1..D_E1M9 (S_Start's mnum for episode 1, s_sound.c:236-237, looked up
  as "d_" + name by S_ChangeMusic, s_sound.c:672-676; names from sounds.c's S_music), each nonzero
  length, printable ASCII, no "MUS\\x1a" or "MThd" header (a placeholder, not music), pairwise
  different.
"""
import re, struct, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / 'tests' / 'reference'


class Bad(Exception):
    pass


def need(cond, msg):
    if not cond:
        raise Bad(msg)


def sounds_c():
    src = (ROOT / 'linuxdoom-1.10' / 'sounds.c').read_text(encoding='latin-1')
    sfx = src[src.index('sfxinfo_t S_sfx[]'):]
    sfx = sfx[:sfx.index('};')]
    rows = re.findall(r'\{\s*"(\w+)"\s*,\s*\w+\s*,\s*-?\d+\s*,\s*([^,]+?)\s*,', sfx)
    mus = src[src.index('musicinfo_t S_music[]'):]
    mus = re.findall(r'\{\s*"(\w+)"\s*,', mus[:mus.index('};')])
    return rows, mus


def parse(data, what):
    need(len(data) >= 12 and data[:4] == b'PWAD', f'{what}: not a PWAD')
    n, off = struct.unpack_from('<ii', data, 4)
    need(n >= 0 and off >= 12 and off + 16 * n == len(data), f'{what}: directory out of bounds')
    out = {}
    order = []
    for i in range(n):
        p, size, name = struct.unpack_from('<ii8s', data, off + 16 * i)
        name = name.rstrip(b'\0').decode('ascii')
        need(0 <= p and size >= 0 and p + size <= off, f'{what}: lump {name} out of bounds')
        need(name not in out, f'{what}: duplicate lump {name}')
        out[name] = data[p:p + size]
        order.append(name)
    return out, order


def header(b):
    return struct.unpack_from('<HHI', b, 0)


def check_main(data):
    rows, _ = sounds_c()
    need(len(rows) == 109 and rows[0][0] == 'none', 'sounds.c: S_sfx not 109 rows')
    want = [(i, 'DS' + n.upper()) for i, (n, link) in enumerate(rows) if i and link == '0']
    need(len(want) == 107, f'sounds.c: {len(want)} unlinked sfx, not 107')
    L, order = parse(data, 'm5c_sfx.wad')
    need(order == [n for _i, n in want], 'm5c_sfx.wad: names or order differ from S_sfx without links')
    for i, n in want:
        b = L[n]
        need(len(b) >= 10, f'{n}: shorter than header + signature')
        fmt, rate, count = header(b)
        need((fmt, rate, count) == (3, 11025, len(b) - 8), f'{n}: header {fmt},{rate},{count}')
        need(b[8] == i and b[9] == 255 - i, f'{n}: signature is not {i},{255 - i}')
    need(len(L['DSPISTOL']) - 8 == 512 and len(L['DSSHOTGN']) - 8 == 1024, 'pistol/shotgn not exact buffers')
    need(len(set(L.values())) == len(L), 'm5c_sfx.wad: two lumps are identical')
    return L


EDGE_RULES = {
    'DSDBOPN': lambda b: len(b) == 0,
    'DSDBCLS': lambda b: len(b) == 5,
    'DSDBLOAD': lambda b: len(b) == 8 and header(b)[2] == 0,
    'DSPLASMA': lambda b: len(b) == 9,
    'DSBFG': lambda b: len(b) == 8 + 511,
    'DSSAWUP': lambda b: len(b) == 8 + 512,
    'DSSAWIDL': lambda b: len(b) == 8 + 513,
    'DSSAWFUL': lambda b: len(b) == 8 + 1024,
    'DSRLAUNC': lambda b: b[8:] == b'\xff' * 600,
    'DSRXPLOD': lambda b: b[8:] == b'\0' * 600,
    'DSFIRSHT': lambda b: len(b) > 8 and header(b)[1] == 22050,
    'DSFIRXPL': lambda b: len(b) > 8 and header(b)[2] != len(b) - 8,
    'DSCHGUN': lambda b: len(b) > 8,
}


def check_edge(data, main):
    E, order = parse(data, 'm5c_sfx_edge.wad')
    want = [n for n in main if n != 'DSPSTART'] + ['DSCHGUN']
    need(order == want, 'm5c_sfx_edge.wad: names differ from the main set without DSPSTART plus DSCHGUN')
    for n in want:
        if n in EDGE_RULES:
            need(EDGE_RULES[n](E[n]), f'edge {n}: rule violated')
            need(n == 'DSCHGUN' or E[n] != main[n], f'edge {n}: not substituted')
        else:
            need(E[n] == main[n], f'edge {n}: differs from the main set')
    counts = [len(b) - 8 for b in E.values() if len(b) >= 8]
    need(any(c % 512 == 0 and c for c in counts) and any(c % 512 for c in counts), 'edge: 512 multiples not covered')
    need(any(len(b) == 8 for b in E.values()) and any(len(b) < 8 for b in E.values()), 'edge: short lumps not covered')


def check_music(data):
    _, mus = sounds_c()
    need(mus[:9] == [f'e1m{m}' for m in range(1, 10)], 'sounds.c: S_music[1..9] not e1m1..e1m9')  # S_music[0] is { 0 }, unmatched
    M, order = parse(data, 'm5c_music.wad')
    need(order == ['D_' + m.upper() for m in mus[:9]], 'm5c_music.wad: names not D_E1M1..D_E1M9')
    for n, b in M.items():
        need(len(b) > 0, f'{n}: zero length')
        need(all(32 <= c < 127 or c == 10 for c in b), f'{n}: not printable ASCII')
        need(not b.startswith(b'MUS\x1a') and not b.startswith(b'MThd'), f'{n}: carries a music header')
    need(len(set(M.values())) == 9, 'm5c_music.wad: two placeholders are identical')


def check_all(files):
    main = check_main(files['m5c_sfx.wad'])
    check_edge(files['m5c_sfx_edge.wad'], main)
    check_music(files['m5c_music.wad'])
    return len(main)


def load(d):
    return {n: (d / n).read_bytes() for n in ('m5c_sfx.wad', 'm5c_sfx_edge.wad', 'm5c_music.wad')}


def replace_lump(data, name, new):
    """Rebuild a PWAD with one lump's bytes replaced (or removed when new is None)."""
    L, order = parse(data, 'self-test')
    ls = [(n, new if n == name else L[n]) for n in order if not (n == name and new is None)]
    body, dirx = bytearray(b'PWAD' + struct.pack('<ii', len(ls), 0)), bytearray()
    for n, b in ls:
        dirx += struct.pack('<ii8s', len(body), len(b), n.encode('ascii'))
        body += b
    struct.pack_into('<i', body, 8, len(body))
    return bytes(body + dirx)


def self_test(d):
    files = load(d)
    check_all(files)
    L, _ = parse(files['m5c_sfx.wad'], 'x')
    E, _ = parse(files['m5c_sfx_edge.wad'], 'x')
    sig = bytearray(L['DSOOF']); sig[8] ^= 1
    cnt = bytearray(L['DSPUNCH']); cnt[4] ^= 1
    plants = [
        ('main: signature changed', 'm5c_sfx.wad', replace_lump(files['m5c_sfx.wad'], 'DSOOF', bytes(sig))),
        ('main: header count wrong', 'm5c_sfx.wad', replace_lump(files['m5c_sfx.wad'], 'DSPUNCH', bytes(cnt))),
        ('main: a name missing', 'm5c_sfx.wad', replace_lump(files['m5c_sfx.wad'], 'DSPSTOP', None)),
        ('main: pistol not an exact buffer', 'm5c_sfx.wad', replace_lump(files['m5c_sfx.wad'], 'DSPISTOL', L['DSPISTOL'][:-1])),
        ('main: directory truncated', 'm5c_sfx.wad', files['m5c_sfx.wad'][:-1]),
        ('edge: 8-byte lump lengthened', 'm5c_sfx_edge.wad', replace_lump(files['m5c_sfx_edge.wad'], 'DSDBLOAD', E['DSDBLOAD'] + b'\x80')),
        ('edge: an untouched lump changed', 'm5c_sfx_edge.wad', replace_lump(files['m5c_sfx_edge.wad'], 'DSOOF', bytes(sig))),
        ('music: a placeholder emptied', 'm5c_music.wad', replace_lump(files['m5c_music.wad'], 'D_E1M3', b'')),
        ('music: a MIDI header', 'm5c_music.wad', replace_lump(files['m5c_music.wad'], 'D_E1M4', b'MThd placeholder\n')),
    ]
    caught = 0
    for what, fname, data in plants:
        try:
            check_all({**files, fname: data})
            print(f'self-test: NOT CAUGHT: {what}')
        except Bad as e:
            caught += 1
            print(f'self-test: caught: {what} ({e})')
    if caught != len(plants):
        sys.exit(f'm5c_check_sfx self-test FAILED: {caught} of {len(plants)} planted defects caught')
    print(f'm5c_check_sfx self-test: OK: {caught} of {len(plants)} planted defects caught')


def main():
    args = sys.argv[1:]
    if args[:1] == ['--self-test']:
        self_test(Path(args[1]) if len(args) > 1 else DIR)
        return
    d = Path(args[0]) if args else DIR
    try:
        n = check_all(load(d))
    except Bad as e:
        sys.exit(f'm5c_check_sfx: FAILED: {e}')
    print(f'm5c_check_sfx: OK: m5c_sfx.wad {n} sound lumps, m5c_sfx_edge.wad {len(EDGE_RULES)} edge rules, '
          f'm5c_music.wad 9 placeholders')


if __name__ == '__main__':
    main()
