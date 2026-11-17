#!/usr/bin/env python3
"""Deterministic seed inputs for fuzz_targets/render_frames.rs and fuzz_targets/render_data.rs (M5a); no Rust-generated expectations.

    python3 tools/seed_render_fuzz.py frames DIR     # max_len 516
    python3 tools/seed_render_fuzz.py data DIR       # max_len 2048

frames: byte 0 the WAD (% 41, order of WADS), byte 1 the view and skill (9 + byte % 3
screenblocks, bit 2 skill 3, bits 3-7 the RNG index, 0 the default), byte 2 the low byte of every
angleturn, byte 3 the tic budget (16 << byte % 4), then four-byte records: forward, side,
angleturn high byte, buttons (bits 0x40/0x80 the repeat count: none 1, 0x40 4, 0x80 16, both 64).
The records are the committed scripts' ticcmds (tests/reference/m5a_scripts for the 23 M5a rooms,
tests/reference/m4b_scripts for the 22 M4b scripts; version-110 demos, 13 header bytes, 4-byte
ticcmds, the 0x80 marker), runs of an identical ticcmd folded into repeat records.

data: byte 0 the game mode, byte 1 the number of edit records, six-byte records (lump u16, offset
u16, kind, value) over tests/reference/m5a_gfx.wad, then raw tail bytes; the edits aim at the
parsers' counts, offsets, markers and names (PNAMES, TEXTURE1, COLORMAP, F_START/F_END,
S_START/S_END, patch and sprite lumps), chosen by a fixed-seed generator.
"""
import argparse
import json
import random
import struct
from pathlib import Path

REFERENCE = Path(__file__).resolve().parent.parent / 'tests' / 'reference'
ROOMS = ['amp', 'ampflick', 'clip', 'contrast', 'dist', 'dsegs', 'fuzz', 'hom', 'light', 'mask',
         'openings', 'peg', 'planes', 'short', 'sky', 'solidsegs', 'sprites', 'switch', 'view',
         'viewz', 'vplanes', 'vsprites', 'weapon']
M4B = ['tele', 'telemove', 'tfrag', 'montele', 'slideb', 'slidec', 'baron', 'infight', 'bvb',
       'boss8', 'bossm1', 'spech8', 'spech9', 'skill', 'tele2x', 'sawbite', 'btunnel', 'telez']
WADS = ROOMS + M4B
FRAMES_MAX_LEN = 4 + 4 * 128
DATA_MAX_LEN = 2048
REPEATS = [(64, 0xc0), (16, 0x80), (4, 0x40), (1, 0x00)]
MAX_TICS = 128


def scripts():
    """[(wad index, skill, ticcmds)] of every committed script, in WADS order."""
    out = []
    manifest = json.loads((REFERENCE / 'm5a_manifest.json').read_text())
    for room in manifest['rooms']:
        wad = Path(room['file']).stem
        if wad not in ROOMS:
            continue  # rooms appended after the fuzz target's frozen 23-room list (byte 0 indexes WADS)
        out.append((WADS.index(wad), REFERENCE / 'm5a_scripts' / (room['script']['lump'] + '.lmp')))
    for name in ('m4b_manifest.json', 'm4b_manifest2.json'):
        for wad in json.loads((REFERENCE / name).read_text())['wads']:
            for script in wad['scripts']:
                out.append((WADS.index(Path(wad['file']).stem),
                            REFERENCE / 'm4b_scripts' / (script['lump'] + '.lmp')))
    result = []
    for index, path in sorted(out, key=lambda t: (t[0], t[1].name)):
        data = path.read_bytes()
        assert data[0] == 110 and data[-1] == 0x80, path
        body = data[13:-1]
        assert len(body) % 4 == 0, path
        commands = [tuple(body[i:i + 4]) for i in range(0, len(body), 4)]
        assert all(c[3] & 0xc0 == 0 for c in commands), path
        result.append((index, data[1], commands))
    assert len(result) == 45, len(result)
    return result


def records(commands):
    """Folds up to MAX_TICS ticcmds into repeat records."""
    commands = commands[:MAX_TICS]
    out = bytearray()
    i = 0
    while i < len(commands):
        c = commands[i]
        run = 1
        while i + run < len(commands) and commands[i + run] == c:
            run += 1
        i += run
        for size, bits in REPEATS:
            while run >= size:
                out += bytes([c[0], c[1], c[2], (c[3] & 0x3f) | bits])
                run -= size
    return bytes(out)


def frames_seeds():
    seeds = []
    for index, skill, commands in scripts():
        body = records(commands)
        for sb in (11, 10, 9):
            view = (sb - 9) % 3 | (4 if skill == 3 else 0)
            seeds.append(bytes([index, view, 0, 3]) + body)
        # A second start context and angleturn low byte, a faster cut, a shorter budget.
        seeds.append(bytes([index, (6 << 3) | (11 - 9) % 3, 0x5a, 3]) + body)
        seeds.append(bytes([index, 1, 0x80, 2]) + records(commands[::2]))
    for index in range(len(WADS)):
        seeds.append(bytes([index, 2, 0, 0]))                       # tic 0 only
        seeds.append(bytes([index, 0, 0, 1]) + bytes([50, 0, 0, 0xc0]))   # 64 tics forward
        seeds.append(bytes([index, 1, 0x40, 2]) + bytes([0, 0, 1, 0x80, 25, 25, 3, 0x40 | 2]))
    assert all(len(s) <= FRAMES_MAX_LEN for s in seeds)
    return list(dict.fromkeys(seeds))


def gfx_lumps():
    d = (REFERENCE / 'm5a_gfx.wad').read_bytes()
    n, o = struct.unpack('<II', d[4:12])
    names = [struct.unpack('<ii8s', d[o + 16 * i:o + 16 * i + 16])[2].rstrip(b'\0') for i in range(n)]
    sizes = [struct.unpack('<ii8s', d[o + 16 * i:o + 16 * i + 16])[1] for i in range(n)]
    return names, sizes


def edit(lump, offset, kind, value):
    return struct.pack('<HHBB', lump, offset, kind, value)


def data_seeds():
    names, sizes = gfx_lumps()
    index = {name: i for i, name in enumerate(names)}
    key = [index[n] for n in (b'PNAMES', b'TEXTURE1', b'COLORMAP', b'PLAYPAL', b'F_START', b'F_END',
                              b'S_START', b'S_END') if n in index]
    other = [i for i in range(len(names)) if i not in key]
    rng = random.Random(0x4d35)
    seeds = [bytes([mode, 0]) for mode in range(3)]
    for mode in range(3):
        seeds.append(bytes([mode, 0]) + b'DRST\x01\x00\x00\x00' + bytes(32))
        seeds.append(bytes([mode, 0]) + struct.pack('<hhhh', 64, 128, -3, 100) + struct.pack('<i', 64) * 4)
    for n in range(400):
        count = rng.randint(1, 8)
        records_ = b''
        for _ in range(count):
            lump = rng.choice(key if rng.random() < 0.6 else other)
            limit = max(1, min(sizes[lump], 96)) if rng.random() < 0.8 else 65535
            kind = rng.randrange(8)
            records_ += edit(lump, rng.randrange(limit), kind, rng.choice((0, 1, 2, 3, 0x7f, 0x80, 0xfe, 0xff)))
        tail = bytes(rng.randrange(256) for _ in range(rng.choice((0, 8, 40))))
        seeds.append(bytes([n % 3, count]) + records_ + tail)
    assert all(len(s) <= DATA_MAX_LEN for s in seeds)
    return list(dict.fromkeys(seeds))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('target', choices=('frames', 'data'))
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    seeds = frames_seeds() if args.target == 'frames' else data_seeds()
    for i, data in enumerate(seeds):
        (args.directory / f'render-{args.target}-{i:04d}').write_bytes(data)
    limit = FRAMES_MAX_LEN if args.target == 'frames' else DATA_MAX_LEN
    print(f'{len(seeds)} render {args.target} seeds (max_len {limit})')


if __name__ == '__main__':
    main()
