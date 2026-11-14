#!/usr/bin/env python3
"""Per-tic comparator of a wire-24 run against the genuine engine, DSIM40 (M4b).

    level_compare_m4b.py DUMP_DIR RUST_DIR --wad WAD --demo LMP [--values] [--json OUT]

The method and contract of level_compare.py (frozen; it has no DSIM40 profile), with the
dumps projected by the independent DSIM40 encoder level_snapshot_m4b.py (profile
`episode1v2`: version 40, policy 24, context PLAYER-MONSTERS-EPISODE1-2-v1, `level.map`),
whose skill and map number come from the demo header (bytes 1 and 3) and whose map slot is
E1M<map>. DUMP_DIR holds the genuine runner's dumps (`load.txt`, `tic_NNNNN.txt`), RUST_DIR
the captures of `player_level_run --policy episode1v2 --captures`; Rust capture N is the
dump of tic N - 1 (`load.txt` for 0). Both are compared without the `map.*` and `events.*`
groups; the `map.*` group of the load capture is compared once (the lump check), and `match`
needs it equal. The first differing tic and field path are reported; `--values` prints both
values (local use only: real-IWAD values are map-derived). The comparison stops at the first
divergence, the first tic the encoder cannot project, or where either side ends. A run is
complete when every C snapshot was compared; on an exit or reborn end Rust refuses the next
tic, which is not compared.

`--json OUT` writes level_compare.py's counts-only result (schema DOOM-M4B-COMPARE-v1): tics
compared, match, first divergence (tic and field path), the projection stop, the `map.*`
group equality and digests, the per-tic stripped-sequence SHA-256 of each side and each
side's final stripped digest."""
import argparse, hashlib, json, os, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
import level_snapshot_m4b as M
from level_compare import stripped, first_difference, end_kind, rust_end, map_bytes

SCHEMA = 'DOOM-M4B-COMPARE-v1'


def compare(dump_dir, rust_dir, mapdata, skill, map_number, values=False):
    result = dict(schema=SCHEMA, tics_compared=0, match=False, first_divergence=None, projection_stop=None,
                  map_group_equal=None, skill=skill, map=map_number)
    kind, last_tic = end_kind(dump_dir)
    rkind, refused_at, rust_last = rust_end(rust_dir)
    result.update(c_end=dict(kind=kind, tic=last_tic), rust_end=dict(kind=rkind, refused_tic=refused_at, last_capture=rust_last))
    seq_c, seq_r = [], []; last_c = last_r = None; text = []
    proj = M.Projector(mapdata, skill, map_number)
    try:
        for dump, following in M.snapshots(dump_dir):
            tic = dump.tic
            fields, _ = proj.step(dump, following)
            path = os.path.join(rust_dir, f'tic_{tic:05}.bin')
            if not os.path.exists(path):
                text.append(f'tic {tic}: no Rust capture'); break
            vr, fr = L.decode(Path(path).read_bytes())
            bc, kc = stripped(M.VERSION, fields); br, kr = stripped(vr, fr)
            if result['map_group_equal'] is None:
                mc = [x for x in fields if x[0].startswith('map.')]; mr = [x for x in fr if x[0].startswith('map.')]
                result['map_group_equal'] = mc == mr
                result['map_group_sha256'] = dict(c=hashlib.sha256(map_bytes(mc)).hexdigest(),
                                                  rust=hashlib.sha256(map_bytes(mr)).hexdigest())
            seq_c.append(hashlib.sha256(bc).digest()); seq_r.append(hashlib.sha256(br).digest())
            last_c, last_r = seq_c[-1].hex(), seq_r[-1].hex()
            result['tics_compared'] += 1
            if vr != M.VERSION:
                result['first_divergence'] = dict(tic=tic, field='version'); text.append(f'tic {tic}: version {M.VERSION} versus {vr}'); break
            diff = first_difference(kc, kr)
            if diff:
                result['first_divergence'] = dict(tic=tic, field=diff[0])
                line = f'tic {tic}: first differing field {diff[0]}'
                if values: line += f' (C {diff[1].hex() if diff[1] is not None else "-"} versus Rust {diff[2].hex() if diff[2] is not None else "-"})'
                text.append(line); break
    except L.ProjectionError as err:
        result['projection_stop'] = dict(kind=err.kind, tic=err.tic, reason=err.text)
        text.append(f'projection stopped: {err.kind} at tic {err.tic}: {err.text}')
    complete = result['tics_compared'] == len(L.m3h9c_dump.snapshot_names(dump_dir))
    result['complete'] = complete
    result['match'] = complete and result['map_group_equal'] is True and result['first_divergence'] is None \
        and result['projection_stop'] is None
    result['c_sequence_sha256'] = hashlib.sha256(b''.join(seq_c)).hexdigest()
    result['rust_sequence_sha256'] = hashlib.sha256(b''.join(seq_r)).hexdigest()
    result['final'] = dict(c_stripped_sha256=last_c, rust_stripped_sha256=last_r)
    return result, text


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('dump'); ap.add_argument('rust')
    ap.add_argument('--wad', required=True); ap.add_argument('--demo', required=True)
    ap.add_argument('--values', action='store_true', help='print the two differing values (local use only)')
    ap.add_argument('--json')
    a = ap.parse_args()
    skill, map_number = M.demo_header(a.demo)
    mapdata = L.MapData(a.wad, f'E1M{map_number}', L.demo_lumps(a.demo))
    result, text = compare(a.dump, a.rust, mapdata, skill, map_number, a.values)
    for line in text: print(line)
    print(f"tics compared {result['tics_compared']}; match {'yes' if result['match'] else 'no'}; "
          f"map group equal {result['map_group_equal']}; C end {result['c_end']['kind']} at tic "
          f"{result['c_end']['tic']}; Rust end {result['rust_end']['kind']}")
    if a.json: Path(a.json).write_text(json.dumps(result, indent=1) + '\n')
    return 0 if result['match'] else 1


if __name__ == '__main__':
    sys.exit(main())
