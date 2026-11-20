#!/usr/bin/env python3
"""Independent DSIM41 projection (profile `episode1v3`) of the genuine engine's dumps and sound stream.

DSIM41 is the canonical schema of wire 25 (`PlayerMonstersEpisode1V3`, M5c decision T3;
docs/M5C_REFERENCE_INVESTIGATION.md section 4.4.4). It is the DSIM40 projection of
level_snapshot_m4b.py, which this module imports and never edits, with exactly these
differences:

- the version word is 41, `profile` is 25 and `context.schema` is
  `PLAYER-MONSTERS-EPISODE1-3-v1` (DELTA_FIELDS);
- the group `events.sounds` after the last `events.pending` field (`events.pending.count`, as
  DSIM40 writes no pending event; the Rust writer's place, after the last `events.pending.<i>`,
  is the same field then): `events.sounds.last.count` (u64) and
  `events.sounds.last.<i>`, the records the tic made in call order, then
  `events.sounds.pending.count` (u64), always 0 in a demo run (records of explicit
  operations between tics; no genuine run makes one).

Like `events.last` and `events.pending`, which DSIM40 encodes with count 0, the sound
records are not in the game dumps. They are projected from the GENUINE sound stream of the
M5c runner (schema DOOM-M5C-SOUND-v1, section 3.1.4: linuxdoom-1.10 built with its in-process
sound code, one define removed, tools/reference/m5c_build.py), never from Rust output:

- one record per `S` row (one per S_StartSound call, s_sound.c:397-466, in call order) and
  per `T` row (one per S_StopSound call, which P_RemoveMobj makes for every removed mobj,
  p_mobj.c:565-566), ordered by the stream's shared `seq` counter;
- the rows of genuine tic t (0-based, the stream's numbering) are the records of DSIM tic
  t + 1, the capture that follows that G_Ticker (the load capture, tic 0, has none);
- start (tag 1): u16 sfx (1..NUMSFX-1, NUMSFX = 109 at sounds.h:288), u8 origin kind and
  identity: `none` (oid -1) kind 0; `player` and `mobj` kind 1 plus the 12-byte actor handle
  (u32 slot, u64 generation); `sector` kind 2 plus u16 index; `button` kind 3 plus u8 slot
  (0..15, buttonlist[MAXBUTTONS], MAXBUTTONS 16 at p_spec.h:244); then u8 placed and, when 1, the origin's
  x, y and the listener's x, y (i32, raw 16.16) and angle (u32), the S row's
  ox, oy, lx, ly, langle. A start is placed exactly when its origin is neither NULL nor the
  listener's body (`origin && origin != players[consoleplayer].mo`, s_sound.c:302), which is
  when the stream writes ox and oy (`player` and `none` write `-`); anything else is refused;
- stop (tag 2): the 12-byte handle of the removed mobj (okind `mobj` or `player`);
- the handle of a mobj is mapped from its thinker serial (the dumps' T rows; the stream's
  oid is the same serial) by the projection's own actor store, the frozen rule DSIM40 uses for
  every actor reference (level_snapshot.py: at load the slots follow the serials, then the
  lowest free slot, generation + 1 on a free); a mobj spawned and freed between two dumps
  has its handle from the K rows like any other. A serial the store never held, a mover's
  serial, or a `player` row whose serial is not the console player's mobj is refused.

Everything else is DSIM40 bytes, so every capture's DSIM41 projection equals its DSIM40
projection outside DELTA_FIELDS and `events.sounds.*` (`compare40`, run on every snapshot
by m5c_wire25_check.py --selftest).

Library: `SoundStream(path)`, `Projector(mapdata, skill, map_number, sounds)`,
`snapshots(directory)` (from level_snapshot_m4b), `project(...)`, `compare40(...)`,
`record_tuple(bytes)` (a record decoded for reports). CLI:

    level_snapshot_m5c.py DUMP_DIR --wad WAD --demo LMP --sound STREAM [--out DIR] [--compare40]

--out writes `tic_NNNNN.bin` (full DSIM41 bytes; scratch only for an IWAD-derived dump).
"""
import argparse, hashlib, os, struct, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
import level_snapshot_m4b as M
from m3h9_geometry import enum_index

n = L.n
ProjectionError = L.ProjectionError
VERSION = 41
POLICY = 25
CONTEXT = b'PLAYER-MONSTERS-EPISODE1-3-v1'
EPISODE1V3 = L.Profile('episode1v3', VERSION, POLICY, CONTEXT, 1024, L.EPISODE1_MESSAGES, True)
DELTA_FIELDS = ('profile', 'context.schema')
SOUNDS = 'events.sounds.'
NUMSFX = enum_index('sfxenum_t', 'NUMSFX', 'sounds.h')          # 109 (sounds.h:288)
MAXBUTTONS = 16                                                   # p_spec.h: buttonlist[MAXBUTTONS]
STREAM_SCHEMA = 'DOOM-M5C-SOUND-v1'
S_COLUMNS = ('tic seq sfx okind oid outcome draws rnd vol sep pitch priority cnum handle samexy gametic '
             'ox oy lx ly langle').split()
T_COLUMNS = 'tic seq okind oid stopped'.split()
KIND = {'none': 0, 'player': 1, 'mobj': 1, 'sector': 2, 'button': 3}
assert NUMSFX == 109


class SoundStream:
    """The S and T rows of a genuine sound stream, by tic, in `seq` order (validated)."""
    def __init__(self, path=None, text=None):
        text = Path(path).read_text() if text is None else text
        self.by_tic, self.last_tic, self.error = {}, -1, None
        self.starts = self.stops = 0
        columns, schema, seq = {}, None, 0
        for number, line in enumerate(text.splitlines(), 1):
            f = line.split(',')
            if f[0] == '#schema': schema = f[1]; continue
            if f[0] == '#columns': columns[f[1]] = f[2:]; continue
            if f[0].startswith('#'): continue
            if f[0] == 'R': self.last_tic = max(self.last_tic, int(f[1])); continue
            if f[0] == 'E': self.error = int(f[1]); continue
            if f[0] not in ('S', 'T'): continue
            want = S_COLUMNS if f[0] == 'S' else T_COLUMNS
            if schema != STREAM_SCHEMA or columns.get(f[0]) != want or len(f) != 1 + len(want):
                raise ValueError(f'sound stream line {number}: not a {STREAM_SCHEMA} {f[0]} row')
            row = dict(zip(want, f[1:]))
            tic, s = int(row['tic']), int(row['seq'])
            if s != seq + 1: raise ValueError(f'sound stream line {number}: seq {s} does not follow {seq}')
            if tic < 0 or (self.by_tic and tic < max(self.by_tic)):
                raise ValueError(f'sound stream line {number}: tic {tic} out of order')
            seq = s
            self.by_tic.setdefault(tic, []).append((f[0], row))
            if f[0] == 'S': self.starts += 1
            else: self.stops += 1
        if schema != STREAM_SCHEMA: raise ValueError(f'not a {STREAM_SCHEMA} stream')

    def rows(self, tic):
        return self.by_tic.get(tic, [])


def _i32(text, what, tic):
    v = int(text)
    if not -2**31 <= v < 2**31: raise ProjectionError('DumpDeficiency', tic, f'{what} {v} outside i32')
    return v


def record_bytes(kind, row, handle_of, tic):
    """The DSIM41 bytes of one S or T row. `handle_of(serial, okind)` gives (slot, generation)."""
    okind, oid = row['okind'], int(row['oid'])
    if kind == 'T':
        if okind not in ('mobj', 'player'): raise ProjectionError('DumpDeficiency', tic, f'a stop of a {okind} origin')
        return b'\2' + struct.pack('<IQ', *handle_of(oid, okind))
    sfx = int(row['sfx'])
    if not 1 <= sfx < NUMSFX: raise ProjectionError('Unadmitted', tic, f'sfx {sfx} outside 1..{NUMSFX - 1}')
    if okind not in KIND: raise ProjectionError('DumpDeficiency', tic, f'origin kind {okind}')
    b = b'\1' + n('H', sfx) + n('B', KIND[okind])
    if okind == 'none':
        if oid != -1: raise ProjectionError('DumpDeficiency', tic, 'a NULL origin with an identity')
    elif okind in ('player', 'mobj'):
        b += struct.pack('<IQ', *handle_of(oid, okind))
    elif okind == 'sector':
        if not 0 <= oid < 65536: raise ProjectionError('DumpDeficiency', tic, f'sector {oid}')
        b += n('H', oid)
    else:
        if not 0 <= oid < MAXBUTTONS: raise ProjectionError('DumpDeficiency', tic, f'button slot {oid}')
        b += n('B', oid)
    placed = okind in ('mobj', 'sector', 'button')
    given = [row[k] != '-' for k in ('ox', 'oy')]
    if given != [placed, placed] or any(row[k] == '-' for k in ('lx', 'ly', 'langle')):
        raise ProjectionError('DumpDeficiency', tic, f'a {okind} start with positions {row["ox"]},{row["oy"]}')
    if not placed: return b + b'\0'
    angle = int(row['langle'])
    if not 0 <= angle < 2**32: raise ProjectionError('DumpDeficiency', tic, f'listener angle {angle}')
    return b + b'\1' + b''.join(n('i', _i32(row[k], k, tic)) for k in ('ox', 'oy', 'lx', 'ly')) + n('I', angle)


def record_tuple(b):
    """A record's bytes decoded for reports: ('start', sfx, kind, identity, placement or None) or
    ('stop', handle); raises ValueError on bytes outside the layout."""
    if b[:1] == b'\2' and len(b) == 13: return ('stop', struct.unpack_from('<IQ', b, 1))
    if b[:1] != b'\1' or len(b) < 5: raise ValueError('record tag')
    sfx, kind = struct.unpack_from('<HB', b, 1); i = 4
    if kind == 0: ident = None
    elif kind == 1: ident = struct.unpack_from('<IQ', b, i); i += 12
    elif kind == 2: ident = struct.unpack_from('<H', b, i)[0]; i += 2
    elif kind == 3: ident = b[i]; i += 1
    else: raise ValueError('origin kind')
    placed = b[i]; i += 1
    at = None
    if placed == 1: at = struct.unpack_from('<iiiiI', b, i); i += 20
    elif placed != 0: raise ValueError('placed flag')
    if i != len(b): raise ValueError('record length')
    return ('start', sfx, ('none', 'actor', 'sector', 'button')[kind], ident, at)


class Projector(M.Projector):
    """The DSIM41 projector: the DSIM40 projection with the DSIM41 header and the
    `events.sounds` group from the genuine stream."""
    def __init__(self, mapdata, skill, map_number, sounds):
        super().__init__(mapdata, skill, map_number)
        self.profile = EPISODE1V3
        self.sounds = sounds

    def _handle_of(self, tic, dump):
        player = next((e for e in self.entries if e.cls == 'actor' and e.row.get('player') == 0 and not e.pending), None)
        def handle_of(serial, okind):
            if serial not in self.actors.handles:
                raise ProjectionError('DumpDeficiency', tic, f'a sound names serial {serial}, which is no actor of the store')
            h = self.actors.handles[serial]
            if okind == 'player' and (player is None or player.serial != serial):
                raise ProjectionError('DumpDeficiency', tic, f'a player origin with serial {serial}, not the player mobj')
            return h
        return handle_of

    def sound_fields(self, dump):
        """The `events.sounds` fields of the capture of `dump` (the rows of genuine tic dump.tic - 1)."""
        tic = dump.tic
        if tic > 0 and self.sounds.last_tic < tic - 1:
            raise ProjectionError('DumpDeficiency', tic, f'the sound stream ends at tic {self.sounds.last_tic}')
        rows = self.sounds.rows(tic - 1) if tic > 0 else []
        handle_of = self._handle_of(tic, dump)
        f = [(SOUNDS + 'last.count', n('Q', len(rows)))]
        f += [(f'{SOUNDS}last.{i}', record_bytes(kind, row, handle_of, tic)) for i, (kind, row) in enumerate(rows)]
        return f + [(SOUNDS + 'pending.count', n('Q', 0))]

    def step(self, dump, following=None):
        fields, report = super().step(dump, following)
        k = _after_pending(fields)
        return fields[:k] + self.sound_fields(dump) + fields[k:], report


def _after_pending(fields):
    """The index just after the last `events.pending` field (count and records) of a field list."""
    return 1 + max(i for i, (p, _) in enumerate(fields) if p.startswith('events.pending.'))


def snapshots(directory):
    return M.snapshots(directory)


def project(directory, mapdata, skill, map_number, sounds):
    """Yield (dsim tic, field list) for every snapshot of a dump directory."""
    proj = Projector(mapdata, skill, map_number, sounds)
    for dump, following in snapshots(directory):
        fields, _ = proj.step(dump, following)
        yield dump.tic, fields


def compare40(directory, mapdata, skill, map_number, sounds):
    """Project every snapshot with DSIM41 and with the frozen DSIM40 projection; raise
    AssertionError unless they differ exactly in DELTA_FIELDS and the `events.sounds` group
    (which sits right after the last `events.pending` field). Returns the snapshot count."""
    count = 0
    for (tic, f41), (tic40, f40) in zip(project(directory, mapdata, skill, map_number, sounds),
                                        M.project(directory, mapdata, skill, map_number)):
        assert tic == tic40
        rest41 = [(p, v) for p, v in f41 if p not in DELTA_FIELDS and not p.startswith(SOUNDS)]
        rest40 = [(p, v) for p, v in f40 if p not in DELTA_FIELDS]
        assert rest41 == rest40, f'tic {tic}: DSIM41 differs from DSIM40 outside the delta'
        v41, v40 = dict(f41), dict(f40)
        assert (v40['profile'], v41['profile']) == (n('I', 24), n('I', 25))
        assert (v40['context.schema'], v41['context.schema']) == (M.CONTEXT, CONTEXT)
        paths = [p for p, _ in f41]; k = _after_pending([(p, v) for p, v in f41 if not p.startswith(SOUNDS)])
        group = [p for p in paths if p.startswith(SOUNDS)]
        assert paths[k:k + len(group)] == group and not any(p.startswith(SOUNDS) for p, _ in f40)
        count += 1
    return count


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('dump'); ap.add_argument('--wad', required=True); ap.add_argument('--demo', required=True)
    ap.add_argument('--sound', required=True, help='the genuine DOOM-M5C-SOUND-v1 stream of the same run')
    ap.add_argument('--out'); ap.add_argument('--compare40', action='store_true')
    a = ap.parse_args()
    skill, map_number = M.demo_header(a.demo)
    mapdata = L.MapData(a.wad, f'E1M{map_number}', L.demo_lumps(a.demo))
    sounds = SoundStream(a.sound)
    if a.out: os.makedirs(a.out, exist_ok=True)
    count = 0; seq = hashlib.sha256(); last = None
    try:
        for tic, fields in project(a.dump, mapdata, skill, map_number, sounds):
            data = L.encode(fields, VERSION); count += 1; seq.update(hashlib.sha256(data).digest()); last = data
            if a.out: Path(a.out, f'tic_{tic:05}.bin').write_bytes(data)
        if a.compare40: print(f'DSIM40 comparison passed: {compare40(a.dump, mapdata, skill, map_number, sounds)} snapshots')
    except ProjectionError as err:
        print(f'projection stopped: {err.kind} at tic {err.tic}: {err.text}')
        print(f'projected {count} snapshots'); return 1
    print(f'projected {count} snapshots; sequence sha256 {seq.hexdigest()}; final fnv1a64 {L.fnv(last):016x}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
