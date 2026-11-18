#!/usr/bin/env python3
"""Deterministic seed inputs for fuzz_targets/render_display_state_v2.rs (M5c, step 11b).

    python3 tools/seed_display_state_v2_fuzz.py DIR     # max_len 70000
    python3 tools/seed_display_state_v2_fuzz.py --check [DIR]
    python3 tools/seed_display_state_v2_fuzz.py --self-test

The input layout is in the target's header comment (byte 0 the part % 3, byte 1 the room or the
saved tic, byte 2 the empty tics or the two modes). The display-state seeds are taken from the
three committed M5b seeds fuzz/corpus/render_display_state/state_* (DDSP version 1 blocks of three
rooms, each after a three-byte header), and DDSP version 2 is WRITTEN FROM THE DOCUMENTED LAYOUT
(docs/STATE_FORMAT.md, "Display state"; crates/doom-render/tests/m5c_contract.rs): version 1's
bytes with the version changed to 2 and one contract byte (1) inserted after it. Nothing is taken
from the output of a run of the Rust encoder in this step: the v1 bytes are the committed ones,
the v2 bytes are produced here by that insertion. Forged variants (contract byte 0, 2, 0xff,
version 0, 3, 0xffffffff, a wrong magic, truncations, trailing bytes, flipped bytes) come from a
fixed-seed generator. Part 2 (FrameSource::restore) seeds use edits records over the genuine
saves the target builds itself (offsets of the contract byte, the version, the sound state's
header and counts), plus a few raw payloads; the genuine saves themselves are made by the target.

--check writes nothing: it builds the seed list twice (the lists must be equal), checks every
length against max_len and, when a directory is given, that it holds exactly the files this tool
would write, byte for byte. --self-test shows that --check fails on a flipped byte, a missing
file and an extra file.
"""
import argparse
import random
import struct
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / 'fuzz' / 'corpus' / 'render_display_state'
MAX_LEN = 70000
STATES = ['state_message_k6', 'state_palsuit_k0', 'state_stdigit_k3']


def v1_blocks():
    """[(room, k, DDSP version 1 bytes)] of the committed M5b seeds."""
    out = []
    for name in STATES:
        data = (CORPUS / name).read_bytes()
        assert data[0] == 0 and data[3:7] == b'DDSP' and data[7:11] == b'\1\0\0\0', name
        out.append((data[1], data[2], data[3:]))
    return out


def to_v2(v1, contract=1):
    """Version 1's bytes with version 2 and one contract byte inserted after the version."""
    return v1[:4] + struct.pack('<I', 2) + bytes([contract]) + v1[8:]


def seeds():
    out = []
    rng = random.Random(0x5d2b)
    for room, k, v1 in v1_blocks():
        v2 = to_v2(v1)
        for part in (0, 1):
            out.append(bytes([part, room, k]) + v1)
            out.append(bytes([part, room, k]) + v2)
        # part 1 also against another room (restore refusals) and another tic
        out.append(bytes([1, (room + 3) % 11, (k + 5) % 8]) + v2)
        out.append(bytes([1, room, 7]) + v2)
        forged = [to_v2(v1, c) for c in (0, 2, 0x80, 0xff)]
        forged += [v2[:4] + struct.pack('<I', v) + v2[8:] for v in (0, 3, 0x20000, 0xffffffff)]
        forged += [b'DDSQ' + v2[4:], b'ddsp' + v2[4:], v2 + b'\0', v1 + b'\0', v2[:-1], v2[:8], v2[:9], v2[:10], v2[:4],
                   v2[:3], b'', b'DDSP', b'DDSP\2\0\0\0']
        forged += [v2[:n] for n in (12, 100, len(v2) // 2, len(v2) - 20)]
        for f in forged:
            out.append(bytes([0, room, k]) + f)
        out.append(bytes([1, room, k]) + forged[0])
        for _ in range(40):
            b = bytearray(v2)
            for _ in range(rng.randint(1, 3)):
                # most flips in the fields after the embedded render state, some anywhere
                at = rng.randrange(len(b)) if rng.random() < 0.3 else len(b) - rng.randint(1, 60)
                b[at] ^= 1 << rng.randrange(8)
            out.append(bytes([rng.randrange(2), room, k]) + bytes(b))
    # part 2: byte 1 the saved tic, byte 2 the modes (display | sound << 2), a u16 display length,
    # the display payload, the sound payload. Edits records are (offset low, offset high, value).
    def edits(*records):
        return b''.join(struct.pack('<HB', at, v) for at, v in records)
    for tic in range(5):
        out.append(bytes([2, tic, 0, 0, 0]))                                   # both genuine
        for contract in (0, 2, 0xff):
            e = edits((8, contract))
            out.append(bytes([2, tic, 1, len(e), 0]) + e)                      # forged contract
        e = edits((4, 1))
        out.append(bytes([2, tic, 1, len(e), 0]) + e)                          # version 1 with sound
        e = edits((4, 3))
        out.append(bytes([2, tic, 1, len(e), 0]) + e)
        for at in (0, 4, 5, 6, 8, 12, 20):
            e = edits((at, 0xff))
            out.append(bytes([2, tic, 4, 0, 0]) + e)                           # sound edit
            out.append(bytes([2, tic, 5, len(e), 0]) + e + e)                  # both edited
        for _ in range(12):
            e = b''.join(struct.pack('<HB', rng.randrange(64), rng.randrange(256)) for _ in range(rng.randint(1, 3)))
            out.append(bytes([2, tic, 4, 0, 0]) + e)
        out.append(bytes([2, tic, 8, 0, 0]) + b'DSND\1\0')                    # raw, truncated sound
        out.append(bytes([2, tic, 10, 0, 0]) + b'DDSP')                       # raw, raw
    out = [s for s in dict.fromkeys(out)]
    assert all(len(s) <= MAX_LEN for s in out)
    return out


def names(items):
    return {f'render-display-v2-{i:04d}': s for i, s in enumerate(items)}


def check(directory):
    a, b = seeds(), seeds()
    if a != b:
        return ['the seed list is not deterministic']
    problems = [f'seed {i} is {len(s)} bytes' for i, s in enumerate(a) if len(s) > MAX_LEN]
    if directory is not None:
        want = names(a)
        have = {p.name: p for p in directory.iterdir()}
        for n in sorted(set(want) - set(have)):
            problems.append(f'{n} is missing')
        for n in sorted(set(have) - set(want)):
            problems.append(f'{n} is not a seed of this tool')
        for n in sorted(set(want) & set(have)):
            if have[n].read_bytes() != want[n]:
                problems.append(f'{n} differs')
    return problems


def self_test():
    with tempfile.TemporaryDirectory() as t:
        d = Path(t)
        write(d)
        assert not check(d), check(d)
        files = sorted(d.iterdir())
        results = []
        data = bytearray(files[0].read_bytes())
        data[-1] ^= 1
        files[0].write_bytes(bytes(data))
        results.append(bool(check(d)))
        write(d)
        files[1].unlink()
        results.append(bool(check(d)))
        write(d)
        (d / 'extra').write_bytes(b'x')
        results.append(bool(check(d)))
    print(f'self-test: {sum(results)} of 3 faults detected')
    return all(results)


def write(directory):
    directory.mkdir(parents=True, exist_ok=True)
    for n, s in names(seeds()).items():
        (directory / n).write_bytes(s)


def main():
    p = argparse.ArgumentParser()
    p.add_argument('directory', nargs='?', type=Path)
    p.add_argument('--check', action='store_true')
    p.add_argument('--self-test', action='store_true')
    a = p.parse_args()
    if a.self_test:
        return 0 if self_test() else 1
    if a.check:
        problems = check(a.directory)
        for q in problems:
            print('FAIL', q)
        print(f'{len(seeds())} seeds, {len(problems)} problems')
        return 1 if problems else 0
    if a.directory is None:
        p.error('a directory is required')
    write(a.directory)
    print(f'{len(seeds())} display state v2 seeds (max_len {MAX_LEN})')
    return 0


if __name__ == '__main__':
    sys.exit(main())
