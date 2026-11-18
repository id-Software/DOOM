#!/usr/bin/env python3
"""Per-tic comparator of a wire-25 run against the genuine engine, DSIM41, with its sound records (M5c).

    level_compare_m5c.py DUMP_DIR RUST --wad WAD --demo LMP --sound STREAM [--values] [--json OUT]

DUMP_DIR holds the genuine whole-engine runner's dumps (`load.txt`, `tic_NNNNN.txt`; the frozen
M4b runner, tools/reference/m4b_build.py), STREAM the genuine sound stream of the same run
(DOOM-M5C-SOUND-v1, the M5c runner: linuxdoom-1.10 built with its in-process sound code, one
define removed), RUST the wire-25 captures of `sound_records_run --captures DIR` (a directory)
or `--capture-stream FILE` (a stream file; a pipe in tools/local/m5c_wire25_real.py). Rust
capture N is the state after N tics and the dump of tic N - 1 (`load.txt` for 0).

Every dump is projected to DSIM41 by the independent encoder level_snapshot_m5c.py (the frozen
DSIM40 projection with version 41, policy 25, context PLAYER-MONSTERS-EPISODE1-3-v1, and the
`events.sounds` group projected from the genuine S and T rows). Two comparisons per tic:

1. the whole engine, as level_compare_m4b.py: both captures without the `map.*` and `events.*`
   groups, field for field (the version word must be 41); the `map.*` group of the load capture
   once (the lump check); the first difference stops the comparison;
2. the sound records (the comparator of M5c note section 4.4.4, check 3): the `events.sounds`
   group of the Rust capture against the encoder's, record by record: order, the tic that made
   it, sfx, origin kind and identity (a handle for an actor, by the encoder's actor store, the
   sector index, the button slot), the placement (origin x, y, listener x, y, angle) of every
   positional start, and every stop. A disagreement is listed (tic, record index, the first
   differing attribute) and the comparison goes on, so that every one is seen.

`match` needs every C snapshot compared, the map groups equal, no divergence, no projection
stop, no sound disagreement and no genuine S or T row after the last dump. `--values` prints
the two differing values (local use only: real-IWAD values are map-derived). `--json OUT`
writes the counts-only result (schema DOOM-M5C-COMPARE-v1).

Both captures must also have the same frame layout, every tic (M5c review NOTE 7): in each, the
records `events.last.<i>` and `events.pending.<i>` follow their count at once, `count` of them in
order; and with those records set aside, the two captures hold the same field paths in the same
order. The encoder writes both event lists with count 0, as DSIM40 does; the Rust capture lists
the tic's events, and its load capture (tic 0) the pending events of the setup. So the position of
the `events.sounds` group in the whole frame is compared: after the last `events.pending` field.

The library entry `compare(..., mutate=f)` lets a self-test plant a fault: `f(tic, records)`
returns the Rust record list (bytes) of that tic to compare in place of the decoded one;
`mutate_frame=g` does the same for the whole Rust field list, `g(tic, fields)`.
"""
import argparse, hashlib, json, os, struct, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
import level_snapshot_m4b as M
import level_snapshot_m5c as S
from level_compare import stripped, first_difference, end_kind, rust_end, map_bytes

SCHEMA = 'DOOM-M5C-COMPARE-v1'
ATTRS = ('tag', 'sfx', 'origin kind', 'origin identity', 'placed', 'ox', 'oy', 'lx', 'ly', 'langle', 'stop handle')


def dir_captures(rust_dir):
    """(tic, bytes) of a --captures directory, in tic order; the end is level_compare.rust_end's."""
    tics = sorted(int(x[4:9]) for x in os.listdir(rust_dir) if x.startswith('tic_') and x.endswith('.bin'))
    for t in tics: yield t, Path(rust_dir, f'tic_{t:05}.bin').read_bytes()


class StreamCaptures:
    """(tic, bytes) of a --capture-stream file object; `.trailer` holds the end text once read."""
    def __init__(self, f):
        self.f, self.trailer = f, None

    def _read(self, k):
        b = self.f.read(k)
        if len(b) != k: raise EOFError('the capture stream ends inside a frame')
        return b

    def __iter__(self):
        while True:
            head = self.f.read(12)
            if not head: self.trailer = self.trailer or 'truncated'; return
            if len(head) != 12: raise EOFError('the capture stream ends inside a frame header')
            tic, length = struct.unpack('<IQ', head)
            data = self._read(length)
            if tic == 0xffffffff: self.trailer = data.decode(); return
            yield tic, data


def stream_end(trailer):
    if trailer and trailer.startswith('refused '):
        return 'refused', int(trailer.split()[1])
    return ('last' if trailer == 'end' else (trailer or 'truncated')), None


def _event_record(p):
    for prefix in ('events.last.', 'events.pending.'):
        if p.startswith(prefix) and p[len(prefix):].isdigit(): return True
    return False


def layout(fields):
    """The field paths of a capture in order, without the `events.last.<i>` and `events.pending.<i>`
    records; raises ValueError unless each list's records follow its count at once, in order."""
    paths = [p for p, _ in fields]
    for prefix in ('events.last', 'events.pending'):
        if f'{prefix}.count' not in paths: raise ValueError(f'no {prefix}.count')
        k = paths.index(f'{prefix}.count')
        count = struct.unpack('<Q', fields[k][1])[0]
        if paths[k + 1:k + 1 + count] != [f'{prefix}.{i}' for i in range(count)] or \
                sum(p.startswith(prefix + '.') for p in paths) != count + 1:
            raise ValueError(f'the {prefix} records do not follow their count')
    return [p for p in paths if not _event_record(p)]


def records_of(fields):
    """The `events.sounds.last` record bytes of a field list, validated: count, indices, then
    `events.sounds.pending.count`, all present and in order."""
    group = [(p, v) for p, v in fields if p.startswith(S.SOUNDS)]
    if not group or group[0][0] != S.SOUNDS + 'last.count': raise ValueError('no events.sounds.last.count')
    count = struct.unpack('<Q', group[0][1])[0]
    if [p for p, _ in group[1:1 + count]] != [f'{S.SOUNDS}last.{i}' for i in range(count)] or \
            [p for p, _ in group[1 + count:]] != [S.SOUNDS + 'pending.count']:
        raise ValueError('the events.sounds group is out of layout')
    if group[-1][1] != struct.pack('<Q', 0): raise ValueError('pending sound records in a demo run')
    return [v for _, v in group[1:1 + count]]


def _flat(t):
    """A decoded record as a tuple aligned with ATTRS."""
    if t[0] == 'stop': return ('stop', None, None, None, None, None, None, None, None, None, t[1])
    at = t[4] or (None,) * 5
    return ('start', t[1], t[2], t[3], t[4] is not None) + tuple(at) + (None,)


def record_diff(tic, c, r, values):
    """Disagreements of one tic's two record lists (bytes): [dict(tic, index, attribute[, c, rust])]."""
    out = []
    if len(c) != len(r): out.append(dict(tic=tic, index=None, attribute=f'record count (C {len(c)}, Rust {len(r)})'))
    for i in range(max(len(c), len(r))):
        if i < len(c) and i < len(r) and c[i] == r[i]: continue
        if i >= len(c) or i >= len(r):
            out.append(dict(tic=tic, index=i, attribute='only in Rust' if i >= len(c) else 'only in C')); continue
        try: rt = _flat(S.record_tuple(r[i]))
        except ValueError as e: out.append(dict(tic=tic, index=i, attribute=f'Rust record bytes ({e})')); continue
        ct = _flat(S.record_tuple(c[i]))
        k = next(j for j in range(len(ATTRS)) if ct[j] != rt[j])
        d = dict(tic=tic, index=i, attribute=ATTRS[k])
        if values: d.update(c=repr(ct[k]), rust=repr(rt[k]))
        out.append(d)
    return out


def compare(dump_dir, rust, mapdata, skill, map_number, sounds, values=False, mutate=None, keep=50, mutate_frame=None):
    """`rust` is a directory or a StreamCaptures. Returns (result dict, text lines)."""
    result = dict(schema=SCHEMA, tics_compared=0, match=False, first_divergence=None, projection_stop=None,
                  map_group_equal=None, skill=skill, map=map_number)
    kind, last_tic = end_kind(dump_dir)
    if isinstance(rust, (str, Path)):
        captures = dir_captures(rust)
        rkind, refused_at, _ = rust_end(rust)
    else:
        captures, rkind, refused_at = iter(rust), None, None
    result['c_end'] = dict(kind=kind, tic=last_tic)
    snd = dict(records_c=0, records_rust=0, starts=0, placed=0, stops=0, tics_with_records=0,
               disagreements=0, first=[], rows_after_last_dump=0)
    seq_c, seq_r, sq_c, sq_r = [], [], hashlib.sha256(), hashlib.sha256()
    last_c = last_r = None; text = []; ended = None
    proj = S.Projector(mapdata, skill, map_number, sounds)
    try:
        for dump, following in S.snapshots(dump_dir):
            tic = dump.tic
            fields, _ = proj.step(dump, following)
            got = next(captures, None)
            if got is None: text.append(f'tic {tic}: no Rust capture'); break
            rtic, data = got
            if rtic != tic: text.append(f'tic {tic}: the Rust capture is numbered {rtic}'); break
            vr, fr = L.decode(data)
            if mutate_frame: fr = mutate_frame(tic, list(fr))
            bc, kc = stripped(S.VERSION, fields); br, kr = stripped(vr, fr)
            if result['map_group_equal'] is None:
                mc = [x for x in fields if x[0].startswith('map.')]; mr = [x for x in fr if x[0].startswith('map.')]
                result['map_group_equal'] = mc == mr
                result['map_group_sha256'] = dict(c=hashlib.sha256(map_bytes(mc)).hexdigest(),
                                                  rust=hashlib.sha256(map_bytes(mr)).hexdigest())
            seq_c.append(hashlib.sha256(bc).digest()); seq_r.append(hashlib.sha256(br).digest())
            last_c, last_r = seq_c[-1].hex(), seq_r[-1].hex()
            result['tics_compared'] += 1
            if vr != S.VERSION:
                result['first_divergence'] = dict(tic=tic, field='version'); text.append(f'tic {tic}: version {S.VERSION} versus {vr}'); break
            lc = layout(fields)
            try: lr = layout(fr)
            except ValueError as e:
                result['first_divergence'] = dict(tic=tic, field=f'frame layout ({e})'); text.append(f'tic {tic}: Rust frame layout: {e}'); break
            if lc != lr:
                at = next((i for i, (x, y) in enumerate(zip(lc, lr)) if x != y), min(len(lc), len(lr)))
                where = f'{lc[at] if at < len(lc) else "-"} (C) / {lr[at] if at < len(lr) else "-"} (Rust)'
                result['first_divergence'] = dict(tic=tic, field=f'frame layout at field {at}: {where}')
                text.append(f'tic {tic}: frame layout differs at field {at}: {where}'); break
            # the sound records (before the whole-engine verdict of this tic, so a tic whose state
            # also diverges still has its records compared)
            c_rec = records_of(fields)
            try: r_rec = records_of(fr)
            except ValueError as e:
                result['first_divergence'] = dict(tic=tic, field=f'events.sounds ({e})'); text.append(f'tic {tic}: Rust {e}'); break
            if mutate: r_rec = mutate(tic, list(r_rec))
            for b in c_rec:
                t = S.record_tuple(b)
                if t[0] == 'stop': snd['stops'] += 1
                else: snd['starts'] += 1; snd['placed'] += t[4] is not None
            snd['records_c'] += len(c_rec); snd['records_rust'] += len(r_rec); snd['tics_with_records'] += bool(c_rec)
            sq_c.update(struct.pack('<QQ', tic, len(c_rec)) + b''.join(struct.pack('<Q', len(b)) + b for b in c_rec))
            sq_r.update(struct.pack('<QQ', tic, len(r_rec)) + b''.join(struct.pack('<Q', len(b)) + b for b in r_rec))
            if c_rec != r_rec:
                d = record_diff(tic, c_rec, r_rec, values)
                snd['disagreements'] += len(d)
                for x in d:
                    if len(snd['first']) < keep:
                        snd['first'].append(x)
                        text.append(f"tic {tic}: sound record {x['index']}: {x['attribute']}"
                                    + (f" (C {x['c']} versus Rust {x['rust']})" if 'c' in x else ''))
            diff = first_difference(kc, kr)
            if diff:
                result['first_divergence'] = dict(tic=tic, field=diff[0])
                line = f'tic {tic}: first differing field {diff[0]}'
                if values: line += f' (C {diff[1].hex() if diff[1] is not None else "-"} versus Rust {diff[2].hex() if diff[2] is not None else "-"})'
                text.append(line); break
            ended = tic
    except L.ProjectionError as err:
        result['projection_stop'] = dict(kind=err.kind, tic=err.tic, reason=err.text)
        text.append(f'projection stopped: {err.kind} at tic {err.tic}: {err.text}')
    if rkind is None:
        for _ in captures: pass                      # drain the stream to its trailer
        rkind, refused_at = stream_end(rust.trailer)
    result['rust_end'] = dict(kind=rkind, refused_tic=refused_at)
    if ended is not None:
        snd['rows_after_last_dump'] = sum(len(v) for t, v in sounds.by_tic.items() if t > ended - 1)
    complete = result['tics_compared'] == len(L.m3h9c_dump.snapshot_names(dump_dir))
    result['complete'] = complete
    result['sounds'] = snd
    result['sound_match'] = snd['disagreements'] == 0 and snd['rows_after_last_dump'] == 0
    result['match'] = complete and result['map_group_equal'] is True and result['first_divergence'] is None \
        and result['projection_stop'] is None and result['sound_match']
    if snd['rows_after_last_dump']: text.append(f"{snd['rows_after_last_dump']} genuine sound rows after the last dump")
    result['c_sequence_sha256'] = hashlib.sha256(b''.join(seq_c)).hexdigest()
    result['rust_sequence_sha256'] = hashlib.sha256(b''.join(seq_r)).hexdigest()
    result['c_sound_sha256'], result['rust_sound_sha256'] = sq_c.hexdigest(), sq_r.hexdigest()
    result['final'] = dict(c_stripped_sha256=last_c, rust_stripped_sha256=last_r)
    return result, text


def summary_line(result):
    s = result['sounds']
    return (f"tics compared {result['tics_compared']}; match {'yes' if result['match'] else 'no'}; map group equal "
            f"{result['map_group_equal']}; C end {result['c_end']['kind']} at tic {result['c_end']['tic']}; Rust end "
            f"{result['rust_end']['kind']}; sound records {s['records_c']} C, {s['records_rust']} Rust ({s['starts']} starts, "
            f"{s['placed']} placed, {s['stops']} stops); sound disagreements {s['disagreements']}")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('dump'); ap.add_argument('rust', help='a --captures directory or a --capture-stream file')
    ap.add_argument('--wad', required=True); ap.add_argument('--demo', required=True); ap.add_argument('--sound', required=True)
    ap.add_argument('--values', action='store_true', help='print the two differing values (local use only)')
    ap.add_argument('--json')
    a = ap.parse_args()
    skill, map_number = M.demo_header(a.demo)
    mapdata = L.MapData(a.wad, f'E1M{map_number}', L.demo_lumps(a.demo))
    sounds = S.SoundStream(a.sound)
    if os.path.isdir(a.rust): result, text = compare(a.dump, a.rust, mapdata, skill, map_number, sounds, a.values)
    else:
        with open(a.rust, 'rb') as f: result, text = compare(a.dump, StreamCaptures(f), mapdata, skill, map_number, sounds, a.values)
    for line in text: print(line)
    print(summary_line(result))
    if a.json: Path(a.json).write_text(json.dumps(result, indent=1) + '\n')
    return 0 if result['match'] else 1


if __name__ == '__main__':
    sys.exit(main())
