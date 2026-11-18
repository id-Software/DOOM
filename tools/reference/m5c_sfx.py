#!/usr/bin/env python3
"""The M5c synthetic sound PWADs: our own tones, no id Software audio. Every sample is computed
here from integer arithmetic; the output is deterministic.

What the genuine linuxdoom-1.10 code reads (docs/M5C_REFERENCE_INVESTIGATION.md section 3.3):
  - I_InitSound (integrated configuration) loads every S_sfx entry 1..NUMSFX-1 that has no link
    (i_sound.c:799-813) with getsfx, which looks up "ds" + name and falls back to "dspistol" when
    the name is missing (i_sound.c:201, 213-216). Only chgun is linked (sounds.c:204), so 107
    names are loaded; sfx_chgun is never started by any source file.
  - getsfx reads the whole lump, ignores every header field, treats bytes 8.. as unsigned 8-bit
    samples and pads the sample count up to a multiple of SAMPLECOUNT 512 with the value 128
    (i_sound.c:218-250).
  - S_StartSoundAtVolume looks the name up with I_GetSfxLumpNum -> W_GetNumForName, which calls
    I_Error when it is missing (s_sound.c:364-365, i_sound.c:451-456, w_wad.c:399-409), so every
    sound a session can start must be present even though getsfx would substitute the pistol.
  - S_Start changes the music at every level load (s_sound.c:236-245) and S_ChangeMusic looks up
    "d_" + S_music[mnum].name by W_GetNumForName and caches it (s_sound.c:672-679); I_RegisterSong
    never reads it (i_sound.c:875-881). An episode 1 map therefore needs D_E1M1..D_E1M9.
The conventional sound lump layout (not read by this source, written so other tools accept the
lumps): u16 format 3, u16 sample rate, u32 sample count, then the samples.

Three PWADs, written to tests/reference/:
  m5c_sfx.wad       one lump per sfx that I_InitSound loads (107 names, DS + S_sfx[1..108] without
                    the linked chgun, in S_sfx order), each with its own length and waveform:
                    sfx n has 40 + (389 n mod 700) samples, except pistol 512 and shotgn 1024 (exact
                    buffers); a triangle wave of period 6 + (n mod 23) and amplitude 20 + (37 n mod
                    100) around 128; samples 0 and 1 are the signature n and 255 - n, so a
                    wrong-sound bug changes a mixed stream. Every lump is distinct.
  m5c_sfx_edge.wad  the main set with the edge cases below substituted, one name missing (getsfx
                    falls back to dspistol) and a dschgun lump the link must ignore. The edge names
                    are Doom II or weapon names except pstart (the missing one): the edge WAD is for
                    the mixer harness, never for a level session, because a start of sfx_pstart would
                    call I_Error there. It is a separate full set because a PWAD cannot delete a name.
  m5c_music.wad     the nine D_E1M1..D_E1M9 placeholders S_Start needs for episode 1 maps: our own
                    ASCII bytes (no MUS or MIDI header, not music), one distinct line per lump,
                    nonzero length (a zero-length music lump is untested). Kept apart from the sound
                    set so that m5c_sfx.wad keeps the bytes and digest recorded in section 3.3.

Edge cases (sample counts unless noted; by lump name):
  DSDBOPN  0-byte lump          DSDBCLS  5-byte lump          DSDBLOAD 8 bytes: header, zero samples
  DSPLASMA 1 sample             DSBFG    511                  DSSAWUP  512 (exact buffer)
  DSSAWIDL 513 (one over)       DSSAWFUL 1024 (two buffers)   DSRLAUNC 600 x 255, DSRXPLOD 600 x 0
  DSFIRSHT header rate 22,050   DSFIRXPL header count 9999 (wrong)
  DSPSTART missing              DSCHGUN  present, 700 x 77 (ignored: chgun is linked)

    m5c_sfx.py [--out DIR]     write the three PWADs (default tests/reference)
    m5c_sfx.py --list          lump counts, sample counts, sizes and digests
    m5c_sfx.py --check         regenerate in memory, compare with the committed files (writes
                               nothing), then run the independent verifier m5c_check_sfx.py
                               and its self-test
"""
import argparse, hashlib, re, struct, subprocess, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
SOUNDS_C = ROOT / 'linuxdoom-1.10' / 'sounds.c'
OUT = ROOT / 'tests' / 'reference'
CHECKER = ROOT / 'tools' / 'reference' / 'm5c_check_sfx.py'


def sfx_table():
    """(index, name, linked) for S_sfx, parsed from sounds.c (sounds.c:114-232)."""
    src = SOUNDS_C.read_text(encoding='latin-1')
    body = src[src.index('sfxinfo_t S_sfx[] ='):]
    body = body[:body.index('};')]
    rows = re.findall(r'\{\s*"(\w+)",\s*(\w+),\s*(-?\d+),\s*([^,]+),', body)
    out = [(i, n, link.strip() != '0') for i, (n, _s, _p, link) in enumerate(rows)]
    if len(out) != 109 or out[0][1] != 'none' or out[86] != (86, 'chgun', True) or sum(l for *_x, l in out) != 1:
        sys.exit('sounds.c table not as expected')
    return out


def lump(samples, rate=11025, count=None):
    s = bytes(samples)
    return struct.pack('<HHI', 3, rate, len(s) if count is None else count) + s


def length(n):
    return {1: 512, 2: 1024}.get(n, 40 + (n * 389) % 700)


def wave(n, count):
    """Triangle of period P and amplitude A around 128, after a 2-sample signature (n, 255-n)."""
    period = 6 + (n % 23)
    amp = 20 + (n * 37) % 100
    out = []
    for k in range(count):
        if k == 0:
            out.append(n); continue
        if k == 1:
            out.append(255 - n); continue
        ph = k % period
        if ph < period // 2:
            tri = (4 * ph * amp) // period - amp
        else:
            tri = amp - (4 * (ph - period // 2) * amp) // period
        out.append(max(0, min(255, 128 + tri)))
    return out


def main_lumps():
    return [('DS' + name.upper(), lump(wave(i, length(i)))) for i, name, linked in sfx_table()[1:] if not linked]


EDGE = {
    'DSDBOPN':  ('raw', b''),
    'DSDBCLS':  ('raw', bytes([3, 0, 0x11, 0x2b, 1])),
    'DSDBLOAD': ('raw', struct.pack('<HHI', 3, 11025, 0)),
    'DSPLASMA': ('samples', [200], {}),
    'DSBFG':    ('samples', 'n511', {}),
    'DSSAWUP':  ('samples', 'n512', {}),
    'DSSAWIDL': ('samples', 'n513', {}),
    'DSSAWFUL': ('samples', 'n1024', {}),
    'DSRLAUNC': ('samples', [255] * 600, {}),
    'DSRXPLOD': ('samples', [0] * 600, {}),
    'DSFIRSHT': ('samples', 'n300', dict(rate=22050)),
    'DSFIRXPL': ('samples', 'n300', dict(count=9999)),
    'DSCHGUN':  ('samples', [77] * 700, {}),
}
EDGE_MISSING = ['DSPSTART']


def edge_lumps():
    base = dict(main_lumps())
    idx = {('DS' + n.upper()): i for i, n, _l in sfx_table()}
    out = []
    for name in list(base) + ['DSCHGUN']:
        if name in EDGE_MISSING:
            continue
        if name in EDGE:
            kind, v, *kw = EDGE[name]
            if kind == 'raw':
                data = v
            else:
                if isinstance(v, str):
                    v = wave(idx[name], int(v[1:]))
                data = lump(v, **kw[0])
            out.append((name, data))
        else:
            out.append((name, base[name]))
    return out


def music_lumps():
    return [(f'D_E1M{m}', f'M5C placeholder D_E1M{m}: not music; S_ChangeMusic caches it, I_RegisterSong ignores it.\n'
             .encode('ascii')) for m in range(1, 10)]


def wad(lumps):
    names = [n for n, _ in lumps]
    assert len(set(names)) == len(names), 'duplicate lump name'
    data, directory = bytearray(b'PWAD' + struct.pack('<ii', len(lumps), 0)), bytearray()
    for n, b in lumps:
        assert len(n) <= 8
        directory += struct.pack('<ii8s', len(data), len(b), n.encode('ascii'))
        data += b
    struct.pack_into('<i', data, 8, len(data))
    return bytes(data + directory)


SETS = {'m5c_sfx.wad': main_lumps, 'm5c_sfx_edge.wad': edge_lumps, 'm5c_music.wad': music_lumps}


def build():
    """{file name: (lumps, bytes, summary)} for the three sets."""
    out = {}
    for fname, fn in SETS.items():
        ls = fn()
        data = wad(ls)
        samples = sum(max(0, len(b) - 8) for _n, b in ls) if fname != 'm5c_music.wad' else 0
        what = f'{samples} samples, ' if fname != 'm5c_music.wad' else ''
        out[fname] = (ls, data, f'{fname}: {len(ls)} lumps, {what}{len(data)} bytes, '
                                f'sha256 {hashlib.sha256(data).hexdigest()}')
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--out', type=Path, default=OUT)
    ap.add_argument('--list', action='store_true')
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    sets = build()
    if a.list:
        for _ls, _d, s in sets.values():
            print(s)
        return
    if a.check:
        bad = 0
        for fname, (_ls, data, s) in sets.items():
            p = OUT / fname
            ok = p.exists() and p.read_bytes() == data
            print(('matches: ' if ok else 'DIFFERS: ') + s, flush=True)
            bad += not ok
        if bad:
            sys.exit(f'{bad} committed sound PWAD(s) differ from the regenerated bytes')
        rc = subprocess.run([sys.executable, str(CHECKER)], cwd=ROOT).returncode
        sys.exit(rc or subprocess.run([sys.executable, str(CHECKER), '--self-test'], cwd=ROOT).returncode)
    a.out.mkdir(parents=True, exist_ok=True)
    for fname, (_ls, data, s) in sets.items():
        (a.out / fname).write_bytes(data)
        print(f'wrote {a.out / fname}: {s}')


if __name__ == '__main__':
    main()
