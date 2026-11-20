#!/usr/bin/env python3
"""Deterministic seed inputs for fuzz_targets/sound_lumps.rs, sound_mixer.rs, sound_logic.rs and
sound_state.rs (M5c).

    python3 tools/seed_sound_fuzz.py lumps DIR     # max_len 4096
    python3 tools/seed_sound_fuzz.py mixer DIR     # max_len 2050
    python3 tools/seed_sound_fuzz.py logic DIR     # max_len 1060
    python3 tools/seed_sound_fuzz.py state DIR     # max_len 2048
    python3 tools/seed_sound_fuzz.py --check [DIR_LUMPS DIR_MIXER DIR_LOGIC DIR_STATE]
    python3 tools/seed_sound_fuzz.py --self-test

Seeds come only from committed synthetic data (tests/reference/m5c_sfx.wad, m5c_sfx_edge.wad, the
start lists of m5c_mixer/starts and m5c_mixer/sessions/starts, the genuine sound streams of
tests/reference/m5c_sound) and fixed-seed edge cases; nothing is taken from Rust output, no id data
is read. The layouts of the inputs are documented in each target's header comment. The mapping of
the genuine streams onto the logic target is coarse (the target's coordinates are 8-bit
multiples of 65536 and its origins are small numbers): a seed starts the fuzzer near real call
shapes, it does not replay the stream. The state target's decoder seeds are DSND version 1 bytes
written here from the layout documented in crates/doom-sound/src/state.rs (an independent writer:
the mixer ends are the padded lengths of getsfx, i_sound.c:230, computed from m5c_sfx.wad, and the
steps are read from the committed generated steptable), valid ones and corrupted copies.

--check writes nothing: it builds every seed list twice (the lists must be equal), checks every
length against the target's max_len and, when directories are given, that each holds exactly the
files this tool would write, byte for byte. --self-test shows that --check fails on a flipped byte,
a missing file and an extra file.
"""
import argparse
import random
import struct
import sys
import tempfile
from pathlib import Path

REFERENCE = Path(__file__).resolve().parent.parent / 'tests' / 'reference'
LUMPS_MAX, MIXER_MAX, LOGIC_MAX, STATE_MAX = 4096, 2050, 1060, 2048
WADS = ['m5c_sfx.wad', 'm5c_sfx_edge.wad']


def wad_lumps(name):
    d = (REFERENCE / name).read_bytes()
    n, o = struct.unpack('<II', d[4:12])
    out = []
    for i in range(n):
        pos, size, raw = struct.unpack('<ii8s', d[o + 16 * i:o + 16 * i + 16])
        out.append((raw.rstrip(b'\0').decode(), d[pos:pos + size]))
    return out


def sfx_names():
    """The 108 sound names in table order, from the committed generated table."""
    text = (Path(__file__).resolve().parent.parent / 'crates' / 'doom-sound' / 'src' / 'sounds_table.rs').read_text()
    import re
    names = re.findall(r'name:\s*"([^"]*)"', text)
    assert len(names) == 109, len(names)
    return names


def lumps_seeds():
    names = sfx_names()
    seeds = []
    for wad in WADS:
        have = {n.upper(): data for n, data in wad_lumps(wad)}
        recs, tail = [], b''
        for i in range(1, 109):
            data = have.get('DS' + names[i].upper())
            if data is None:
                recs.append(struct.pack('<BBH', i, 1, 0))
                continue
            chunk = data[:30]
            recs.append(struct.pack('<BBH', i, 0, len(chunk)))
            tail += chunk
        for caps in (0, 1, 2, 3):
            seeds.append(bytes([caps, len(recs)]) + b''.join(recs) + tail)
    # Boundary sizes of getsfx's padding (i_sound.c:230): 0..9, 519, 520, 521, 1031, 1032 bytes.
    for size in (0, 1, 7, 8, 9, 519, 520, 521, 1031, 1032):
        seeds.append(bytes([0, 2]) + struct.pack('<BBH', 1, 0, size) + struct.pack('<BBH', 2, 0, size)
                     + bytes(range(256)) * 4)
    seeds.append(bytes([0, 0]))                                    # no lump at all: PistolMissing
    seeds.append(bytes([0, 1]) + struct.pack('<BBH', 0, 0, 40) + bytes(40))   # only dspistol
    seeds.append(bytes([0, 2]) + struct.pack('<BBH', 5, 3, 600) * 2 + bytes(range(256)) * 5)  # duplicate names
    seeds.append(bytes([1, 1]) + struct.pack('<BBH', 1, 0, 701) + bytes(701))  # over a 700-byte cap
    assert all(len(s) <= LUMPS_MAX for s in seeds)
    return list(dict.fromkeys(seeds))


def op(kind, a=0, b=0, c=0, d=0, e=0, f=0, g=0):
    return bytes([kind, a, b, c, d, e, f, g])


def start_rows(path):
    rows = []
    for line in path.read_text().splitlines():
        p = line.split()
        if p and p[0] == 's':
            rows.append(('s', int(p[3]), int(p[4]), int(p[5]), int(p[6]), int(p[7])))
        elif p and p[0] == 'n':
            for _ in range(min(int(p[1]), 4)):
                rows.append(('s', int(p[5]), int(p[6]), int(p[7]), int(p[8]), int(p[9])))
    return rows


def mixer_seeds():
    seeds = []
    files = sorted((REFERENCE / 'm5c_mixer' / 'starts').glob('*.txt'))
    files += sorted((REFERENCE / 'm5c_mixer' / 'sessions' / 'starts').glob('*.txt'))
    for path in files:
        gametic = None
        body = b''
        text = path.read_text().splitlines()
        for line in text:
            p = line.split()
            if not p or p[0] not in ('s', 'n'):
                continue
            if p[0] == 's':
                gt, sfx, vol, sep, pitch = (int(p[3]), int(p[4]), int(p[5]), int(p[6]), int(p[7]))
            else:
                gt, sfx, vol, sep, pitch = (int(p[4]), int(p[5]), int(p[6]), int(p[7]), int(p[8]))
            if len(body) >= 8 * 250:
                break
            if gt != gametic:
                body += op(3, *struct.pack('<i', gt))
                body += op(1, 0)
                gametic = gt
            if 1 <= sfx <= 108 and 0 <= vol < 128 and 0 <= sep < 256 and 0 <= pitch < 256:
                body += op(0, sfx, vol, 0, sep, 0, pitch, 0)
        for wad in (0, 1):
            seeds.append(bytes([wad, 0]) + body[:8 * MAX_OPS_MIXER])
    # Wild values and refusals: out-of-range pitch, volumes and separations, the short lumps.
    rng = random.Random(0x4d35c)
    for n in range(60):
        body = b''
        for _ in range(rng.randint(4, 40)):
            kind = rng.choice((0, 0, 0x10, 0x10, 1, 2, 3))
            body += op(kind, rng.choice((1, 2, 5, 8, 10, 22, 86, 87, 108, 0, 109, rng.randrange(256))),
                       *[rng.randrange(256) for _ in range(6)])
        seeds.append(bytes([n % 2, rng.randrange(256)]) + body)
    assert all(len(s) <= MIXER_MAX for s in seeds)
    return list(dict.fromkeys(seeds))


MAX_OPS_MIXER = 250
KINDS = {'none': 0, 'mobj': 1, 'sector': 2, 'button': 3, 'player': 4}


def logic_seeds():
    seeds = []
    for path in sorted((REFERENCE / 'm5c_sound').glob('*.sound')):
        body = b''
        last_tic = None
        for line in path.read_text().splitlines():
            p = line.split(',')
            if p[0] != 'S' or len(p) < 22:
                continue
            if len(body) >= 8 * 120:
                break
            tic = int(p[1])
            if tic != last_tic:
                body += op(4, 1) + op(2, 0, 0, 0, 0, 0, 0) + op(5)
                last_tic = tic
            kind = KINDS[p[4]]
            oid = int(p[5]) if p[5] != '-1' else 0
            flags = 0x10 if p[17] != '-' else 0
            xs = [(int(v) >> 16) & 0xff if v != '-' else 0 for v in (p[17], p[18], p[19], p[20])]
            body += op(flags, int(p[3]) & 0xff, kind, oid & 0xff, *xs)
        for gamemap, vol in ((0, 8), (1, 8)):
            seeds.append(bytes([0, gamemap, 3, vol]) + body)
    rng = random.Random(0x4d35d)
    for n in range(60):
        body = b''
        for _ in range(rng.randint(4, 60)):
            body += op(rng.choice((0, 0x10, 0, 0x10, 1, 2, 3, 4, 5, 6)),
                       *[rng.choice((rng.randrange(256), rng.choice((0, 1, 22, 32, 86, 87, 108, 109)))) for _ in range(7)])
        seeds.append(bytes([n % 2, rng.randrange(3), rng.choice((0, 1, 2, 3, 3, 3, 5, 7)),
                            rng.choice((8, 8, 0, 127, 128))]) + body)
    assert all(len(s) <= LOGIC_MAX for s in seeds)
    return list(dict.fromkeys(seeds))


def steptable():
    text = (Path(__file__).resolve().parent.parent / 'crates' / 'doom-sound' / 'src' / 'steptable.rs').read_text()
    import re
    body = text[text.index('= ['):]
    values = [int(v) for v in re.findall(r'\d+', body)]
    assert len(values) == 256, len(values)
    return values


def padded_lengths():
    """getsfx's padded length per sound number of m5c_sfx.wad (i_sound.c:213-241): own lump or the
    dspistol copy, size - 8 rounded up to 512, 0 for a lump of 8 bytes or fewer."""
    names = sfx_names()
    have = {n.upper(): data for n, data in wad_lumps('m5c_sfx.wad')}
    out = {}
    for i in range(1, 109):
        data = have.get('DS' + names[i].upper(), have.get('DSPISTOL'))
        size = len(data)
        out[i] = ((size - 8 + 511) // 512) * 512 if size > 8 else 0
    return out, {i for i in range(1, 109) if 'DS' + names[i].upper() in have}


def dsnd(tic, channels, mixer, looked=(), label=None, firings=0, textures=80, gamemap=1,
         volume=8, handlenums=0):
    """DSND version 1 (crates/doom-sound/src/state.rs). channels: list of None or
    (sfx, kind, ident, handle); mixer: 8 entries of None or (sfx, pos, end, step, frac, start,
    handle, left, right)."""
    buffers = -(-315 * tic // 512)
    out = b'DSND' + struct.pack('<HQQBBiI', 1, tic, buffers, volume, len(channels), gamemap, textures)
    out += struct.pack('<BQQ', 1 if label is not None else 0, label or 0, firings)
    bits = bytearray(14)
    for n in looked:
        bits[n // 8] |= 1 << (n % 8)
    out += bytes(bits)
    for c in channels:
        if c is None:
            out += b'\0'
            continue
        sfx, kind, ident, handle = c
        out += bytes([1, sfx, kind])
        if kind == 1:
            out += struct.pack('<IQ', *ident)
        elif kind == 2:
            out += struct.pack('<H', ident)
        elif kind == 3:
            out += bytes([ident])
        out += struct.pack('<i', handle)
    out += struct.pack('<H', handlenums)
    for m in mixer:
        if m is None:
            out += b'\0'
            continue
        out += bytes([1, m[0]]) + struct.pack('<IIIIiiBB', *m[1:])
    return out


def state_seeds():
    seeds = []
    steps = steptable()
    ends, own = padded_lengths()
    free8 = [None] * 8
    valid = [dsnd(0, [None] * 3, free8)]
    valid.append(dsnd(1, [(1, 0, None, 100), None, None], [(1, 0, ends[1], steps[128], 0, 0, 100, 8, 8)] + [None] * 7,
                      looked=[1], handlenums=101))
    valid.append(dsnd(40, [(1, 0, None, 100), (20, 1, (3, 7), 101), (20, 2, 2, 102)],
                      [(1, 10, ends[1], steps[120], 65535, 39, 100, 127, 0), None,
                       (20, 0, ends[20], steps[0], 0, 0, 101, 3, 4)] + [None] * 5,
                      looked=[1, 20], label=12, firings=3, handlenums=103))
    valid.append(dsnd(545, [(23, 3, 14, 65535), None, None, (82, 1, (9, 1), 300)], free8,
                      looked=sorted(own)[:10], gamemap=8, handlenums=0, textures=65536))
    valid.append(dsnd(2147483647, [None] * 64, free8, volume=127))
    seeds += [bytes([0]) + v for v in valid]
    rng = random.Random(0x4d35e)
    for v in valid:
        for _ in range(12):
            b = bytearray(v)
            for _ in range(rng.randint(1, 3)):
                b[rng.randrange(len(b))] ^= 1 << rng.randrange(8)
            seeds.append(bytes([0]) + bytes(b))
        seeds.append(bytes([0]) + v[:rng.randrange(len(v))])
    # The session: the coarse call shapes of the genuine streams, as for the logic target, with a
    # tic op after each stream tic and a save every tenth tic.
    for path in sorted((REFERENCE / 'm5c_sound').glob('*.sound')):
        body = b''
        last_tic = None
        ntic = 0
        for line in path.read_text().splitlines():
            p = line.split(',')
            if p[0] != 'S' or len(p) < 22:
                continue
            if len(body) >= 8 * 110:
                break
            tic = int(p[1])
            if tic != last_tic:
                if last_tic is not None:
                    body += op(2, last_tic & 0xff, 0, 0, 0, 0, 0)
                    ntic += 1
                    if ntic % 10 == 0:
                        body += op(3)
                last_tic = tic
            kind = KINDS[p[4]]
            oid = int(p[5]) if p[5] != '-1' else 0
            flags = 0x10 if p[17] != '-' else 0
            xs = [(int(v) >> 16) & 0xff if v != '-' else 0 for v in (p[17], p[18], p[19], p[20])]
            body += op(flags, int(p[3]) & 0xff, kind, oid & 0xff, *xs)
        body += op(2) + op(3)
        seeds.append(bytes([1, 0, 80]) + body)
    for n in range(40):
        body = b''
        for _ in range(rng.randint(4, 120)):
            body += op(rng.choice((0, 0x10, 0x10, 1, 2, 2, 2, 3)),
                       *[rng.choice((rng.randrange(256), rng.choice((0, 1, 20, 23, 32, 86, 87, 108, 109))))
                         for _ in range(7)])
        seeds.append(bytes([1, rng.randrange(3), rng.choice((0, 1, 19, 80, 255))]) + body)
    assert all(len(s) <= STATE_MAX for s in seeds)
    return list(dict.fromkeys(seeds))


TARGETS = {'lumps': (lumps_seeds, LUMPS_MAX), 'mixer': (mixer_seeds, MIXER_MAX), 'logic': (logic_seeds, LOGIC_MAX),
           'state': (state_seeds, STATE_MAX)}


def named(target):
    make, _ = TARGETS[target]
    return {f'sound-{target}-{i:04d}': data for i, data in enumerate(make())}


def check_dir(target, directory):
    want = named(target)
    have = {p.name: p.read_bytes() for p in directory.iterdir()} if directory.is_dir() else {}
    problems = []
    for name in sorted(set(want) | set(have)):
        if name not in have:
            problems.append(f'{directory}/{name}: missing')
        elif name not in want:
            problems.append(f'{directory}/{name}: not a seed of this tool')
        elif have[name] != want[name]:
            problems.append(f'{directory}/{name}: differs')
    return problems


def check(dirs):
    problems = []
    for target, (make, limit) in TARGETS.items():
        first, second = make(), make()
        if first != second:
            problems.append(f'{target}: generation is not deterministic')
        if any(len(s) > limit for s in first):
            problems.append(f'{target}: a seed exceeds max_len {limit}')
        print(f'{target}: {len(first)} seeds, {sum(map(len, first))} bytes, max_len {limit}')
    for target, directory in zip(TARGETS, dirs):
        problems += check_dir(target, directory)
    for p in problems:
        print('FAIL', p)
    return not problems


def self_test():
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        for name, data in named('logic').items():
            (directory / name).write_bytes(data)
        assert not check_dir('logic', directory)
        first = directory / 'sound-logic-0000'
        original = first.read_bytes()
        first.write_bytes(bytes([original[0] ^ 1]) + original[1:])
        assert check_dir('logic', directory), 'flipped byte not detected'
        first.write_bytes(original)
        first.unlink()
        assert check_dir('logic', directory), 'missing file not detected'
        first.write_bytes(original)
        (directory / 'extra').write_bytes(b'x')
        assert check_dir('logic', directory), 'extra file not detected'
    print('self-test: 3 of 3 faults detected')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('target', nargs='?', choices=sorted(TARGETS))
    parser.add_argument('directory', nargs='?', type=Path)
    parser.add_argument('--check', nargs='*', type=Path, metavar='DIR')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if args.check is not None:
        dirs = args.check
        if dirs and len(dirs) != len(TARGETS):
            parser.error('--check takes no directory or four (lumps mixer logic state)')
        sys.exit(0 if check(dirs) else 1)
    if not args.target or not args.directory:
        parser.error('target and directory are required')
    args.directory.mkdir(parents=True, exist_ok=True)
    seeds = named(args.target)
    for name, data in seeds.items():
        (args.directory / name).write_bytes(data)
    print(f'{len(seeds)} sound {args.target} seeds (max_len {TARGETS[args.target][1]})')


if __name__ == '__main__':
    main()
