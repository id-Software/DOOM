#!/usr/bin/env python3
"""Bounded Episode 1 part 2 (wire 24) command-stream inputs for fuzz_targets/player_episode1_v2.rs; no Rust-generated expectations."""
import argparse
import json
from pathlib import Path

# Input layout (see the target's module documentation):
#   byte 0  the WAD (% 18), in the order of WADS below;
#   byte 1  bit 0 the skill (0 skill 2, 1 skill 3); bits 1-7 0 for the default context
#           (as the committed scripts run), otherwise the gameplay RNG index of the start;
#   byte 2  the low byte of every command's angleturn;
#   byte 3  the capture stride, 16 << (byte 3 % 4) admitted tics between DSIM40 round trips;
#   then four-byte records: forward, side, angleturn high byte, buttons. Buttons bits
#   0x40/0x80 are the record's repeat count (none 1, 0x40 4, 0x80 16, both 64 tics);
#   the low six bits are sent as they are (1 ATTACK, 2 USE, 4 CHANGE, weapon << 3).
#
# Mapping from the committed scripts (tests/reference/m4b_scripts/*.lmp, version-110
# demos: 13 header bytes, 4-byte ticcmds forward/side/angleturn-high/buttons, the 0x80
# marker): the header and the marker are dropped and each ticcmd is a record as it
# stands; runs of an identical ticcmd are folded into repeat records (64, 16, 4, 1).
# Only the first 600 tics fit the target's budget, so longer scripts are cut there. The
# WAD of each script is the one the two manifests name; its skill is demo header byte 1.
WADS = ['tele', 'telemove', 'tfrag', 'montele', 'slideb', 'slidec', 'baron', 'infight',
        'bvb', 'boss8', 'bossm1', 'spech8', 'spech9', 'skill', 'tele2x', 'sawbite',
        'btunnel', 'telez']
HEADER = 4
TIC_BUDGET = 600
MAX_LEN = HEADER + 4 * TIC_BUDGET
BT_ATTACK, BT_USE, BT_CHANGE = 1, 2, 4
REPEATS = [(64, 0xc0), (16, 0x80), (4, 0x40), (1, 0x00)]
REFERENCE = Path(__file__).resolve().parent.parent / 'tests' / 'reference'
ROOT = REFERENCE / 'm4b_scripts'


def scripts():
    """{script: wad} from both manifests, checked against WADS."""
    out = {}
    for name in ('m4b_manifest.json', 'm4b_manifest2.json'):
        for wad in json.loads((REFERENCE / name).read_text())['wads']:
            for script in wad['scripts']:
                out[script['lump']] = wad['name'].lower()
    assert set(out.values()) <= set(WADS), out
    return out


def demo_commands(name):
    """(skill, [(forward, side, turn high byte, buttons)]) of a committed script."""
    data = (ROOT / f'{name}.lmp').read_bytes()
    assert data[0] == 110 and data[1] in (2, 3) and data[-1] == 0x80, name
    body = data[13:-1]
    assert len(body) % 4 == 0, name
    commands = [tuple(body[i:i + 4]) for i in range(0, len(body), 4)]
    assert all(c[3] & 0xc0 == 0 for c in commands), name
    return data[1], commands


def records(commands):
    """Folds a ticcmd list (at most TIC_BUDGET tics) into repeat records."""
    commands = commands[:TIC_BUDGET]
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
                out += bytes([c[0] & 0xff, c[1] & 0xff, c[2] & 0xff, (c[3] & 0x3f) | bits])
                run -= size
    return bytes(out)


def held(tics, forward=0, side=0, turn=0, buttons=0):
    """`tics` copies of one command (turn is the angleturn high byte)."""
    return [(forward & 0xff, side & 0xff, turn & 0xff, buttons)] * tics


def turn(byte):
    """One tic turning by `byte` × 1.40625 degrees (the scripts' turn byte)."""
    return held(1, turn=byte)


def patterns():
    """Synthetic command lists aimed at every WAD's mechanisms from its start."""
    out = [
        held(600),                                                        # idle: monsters and timers
        held(600, buttons=BT_ATTACK),                                     # fire in place
        held(40, 50) + held(560, buttons=BT_USE),                         # walk, then USE every tic
        held(120, 50, 0, 0, BT_USE),                                      # walk using
        sum((turn(64) + held(30, 50, 0, 0, BT_USE) + held(8, buttons=BT_USE)
             for _ in range(12)), []),                                    # square walks with uses
        sum((turn(t) + held(25, 50, 25, 0, BT_ATTACK | BT_USE) for t in (32, 96, 160, 224)), []) * 4,
        held(200, 25, 0, 4, BT_USE),                                      # a slow circle using
        held(300, -50, 50, 2, BT_ATTACK),                                 # backward strafe circle
        # every weapon change 0-7 (4-7 outside this step's command domain), each fired
        sum((held(1, buttons=BT_CHANGE | (w << 3)) + held(30, buttons=BT_ATTACK)
             for w in range(8)), []),
        sum((held(1, 50, 0, 0, BT_ATTACK | BT_CHANGE | (w << 3)) + held(15, 40, 0, 0, BT_USE)
             for w in (7, 3, 6, 2, 5, 1, 4, 0)), []),
        # movement outside ±50 (refused at this step), then inside
        held(20, 100) + held(20, 50) + held(20, -127, 127) + held(20, -50, 50) + held(20, 127, -100, 8),
        # weapon bits without BT_CHANGE (refused), interleaved with admitted tics
        [(0, 0, 0, 0x38), (25, 0, 0, 0), (0, 0, 0, 0x18 | BT_USE), (25, 0, 0, BT_USE)] * 50,
        # a run into a wall and around; turns of every high byte
        [(50, 0, t, BT_USE) for t in range(256)] + held(100, 50, 0, 0, BT_USE),
        # sprint and use toward each quarter
        sum((turn(q) + held(60, 50, 0, 0, BT_USE) + held(10) for q in (0, 64, 128, 192)), []) * 2,
    ]
    return out


def header(wad, skill=2, rng=0, low=0, stride=3):
    return bytes([wad, (rng << 1) | (skill == 3), low, stride])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    seeds = []
    # Each committed script on its WAD at its own skill: the faithful replay, a second RNG,
    # a nonzero angleturn low byte and the shortest stride; the other skill; then a faster
    # cut (every 2nd tic dropped) and the script after a 30-tic wait.
    for name, wad_name in sorted(scripts().items()):
        wad = WADS.index(wad_name)
        skill, commands = demo_commands(name)
        other = 5 - skill
        body = records(commands)
        seeds.append(header(wad, skill) + body)
        seeds.append(header(wad, skill, rng=6) + body)
        seeds.append(header(wad, skill, low=0x80, stride=0) + body)
        seeds.append(header(wad, skill, rng=100, low=0x5a, stride=1) + body)
        seeds.append(header(wad, other) + body)
        seeds.append(header(wad, skill) + records(commands[::2]))
        seeds.append(header(wad, skill, stride=2) + records(held(30) + commands))
        if len(commands) > TIC_BUDGET:
            seeds.append(header(wad, skill) + records(commands[:60] + commands[TIC_BUDGET:]))
    # Every synthetic pattern on every WAD at both skills.
    for wad in range(len(WADS)):
        for i, commands in enumerate(patterns()):
            for skill, rng in ((2, 0), (3, 6)):
                seeds.append(header(wad, skill, rng=rng, low=(i * 37) & 0xff, stride=i) + records(commands))
    # Header-only and odd-length inputs.
    for wad in range(len(WADS)):
        seeds.append(header(wad))
        seeds.append(header(wad, 3) + bytes([50, 0, 0]))
    assert all(len(s) <= MAX_LEN for s in seeds)
    seeds = list(dict.fromkeys(seeds))  # a cut or shift can repeat another seed
    for i, data in enumerate(seeds):
        (args.directory / f'episode1v2-{i:04d}').write_bytes(data)
    print(f'{len(seeds)} Episode 1 part 2 command-stream seeds (max_len {MAX_LEN})')


if __name__ == '__main__':
    main()
