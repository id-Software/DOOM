#!/usr/bin/env python3
"""Independent DSIM40 projection (profile `episode1v2`) of the genuine-engine runner's dumps.

DSIM40 is the canonical schema of wire 24 (`PlayerMonstersEpisode1V2`, M4b decision I2;
docs/STATE_FORMAT.md "DSIM v40"). It is the DSIM39 layout of the frozen `episode1` profile
of level_snapshot.py, which this module imports and never edits, with exactly these
differences:

- the version word is 40, `profile` is 24 and `context.schema` is
  `PLAYER-MONSTERS-EPISODE1-2-v1` (no skill word: the skill is a field);
- one new field, `level.map` (u8, 1-9), right after `level.secret_exit`: the map number
  of the Episode 1 slot, read from byte 3 of the demo header (G_DoPlayDemo -> G_InitNew
  sets `gamemap` from it, g_game.c), which A_BossDeath (p_enemy.c:1629-1631) and
  PIT_StompThing (p_map.c:102) test;
- `actors.options[0]` (the skill, H.gameskill) may be 2 or 3 (decision I4); it must equal
  byte 1 of the demo header.

Everything else (every field, its path, width, position and source column) is the DSIM39
projection, so a state reachable under wire 23 projects to DSIM39 bytes except for those
four fields; `level_snapshot_m4b.py --compare39` and m4b_snapshot_goldens.py --selftest
check exactly that on every snapshot.

What the projection adds to fail loudly (no silent drops):

- every `#columns` line of a dump must be exactly the M4a driver's (COLUMNS below, the
  frozen m4a_driver.c contract), every row must have a known kind and the width of its
  kind's columns: a new or missing column or row kind is a DumpDeficiency;
- H.gamestate must be GS_LEVEL (0), player 0 in game and players 1-3 not, no cheats;
- every actor's (type, state) must lie in the type's *normal closure*: the states reached
  from its info.c spawnstate, seestate, painstate, meleestate, missilestate, deathstate and
  xdeathstate through `nextstate` (raisestate excluded: no Archvile; the player's
  S_PLAY_ATK2, set by name in p_pspr.c, included), plus `S_GIBS` for a
  corpse (health <= 0; PIT_ChangeSector, p_map.c), plus the removal sentinel -1 for a
  pending record. MT_TELEPORTMAN, whose spawnstate is S_NULL, must be alive in state 0
  (S_NULL as an index, distinct from the removal sentinel) with tics -1 and its info.c
  flags (MF_NOBLOCKMAP|MF_NOSECTOR) and never pending. The closures are computed from the
  C-compiled tables (tests/reference/m3c_states.csv and m3c_types.csv, checked against
  linuxdoom-1.10 by generate_m3c.py --check); test_level_snapshot_m4b.py checks the four
  new types against info.h and info.c text.

Library: `Projector(mapdata, skill, map_number)`, `snapshots(directory)`, `project(...)`,
`demo_header(path)` -> (skill, map number). CLI:

    level_snapshot_m4b.py DUMP_DIR --wad WAD --demo LMP [--out DIR] [--compare39]

The map slot is E1M<map> of the demo header. --out writes `tic_NNNNN.bin` (full DSIM40
bytes; scratch only for an IWAD-derived dump). --compare39 also projects every snapshot
with the DSIM39 profile and requires the field lists to differ only in the four fields
above."""
import argparse, csv, hashlib, os, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
from m3h9_geometry import enum_index

n = L.n
ProjectionError = L.ProjectionError
VERSION = 40
POLICY = 24
CONTEXT = b'PLAYER-MONSTERS-EPISODE1-2-v1'
EPISODE1V2 = L.Profile('episode1v2', VERSION, POLICY, CONTEXT, 1024, L.EPISODE1_MESSAGES, True)
SKILLS = (2, 3)            # sk_medium, sk_hard (doomdef.h skill_t; decision I4)
GS_LEVEL = 0               # gamestate_t (doomdef.h)
# the fields DSIM40 changes or adds relative to DSIM39 (the only allowed differences)
DELTA_FIELDS = ('profile', 'context.schema', 'actors.options', 'level.map')

# The dump columns of the M4a driver (tools/reference/m4a_driver.c COLUMNS, frozen contract
# tests/reference/m4a_runner_contract.json; the M4b runner is that driver plus the declared
# telefrag adapter): kind -> columns after the kind, `tic` first.
_MOBJ = ('type x y z momx momy momz health state tics flags height radius reactiontime threshold target lastlook '
         'floorz ceilingz subsector angle sprite frame movedir movecount tracer sp_x sp_y sp_angle sp_type sp_options '
         'player bx by bpos sector spos')
COLUMNS = {k: ['tic'] + v.split() for k, v in {
    'H': 'leveltime gametic prndindex rndindex totalkills totalitems totalsecret gameaction gamestate numthinkers '
         'numlinespecials gameskill secretexit',
    'M': 'idx ' + _MOBJ, 'R': 'idx fn sector', 'RR': 'idx ' + _MOBJ,
    'DR': 'idx type sector topheight speed direction topwait topcountdown',
    'PL': 'idx running sector speed low high wait count status oldstatus crush tag type',
    'FL': 'idx type crush sector direction newspecial texture floordestheight speed',
    'CE': 'idx running type sector bottomheight topheight speed crush direction tag olddirection',
    'LF': 'idx sector count maxlight minlight', 'LL': 'idx sector count maxlight minlight maxtime mintime',
    'LS': 'idx sector count minlight maxlight darktime brighttime', 'LG': 'idx sector minlight maxlight direction',
    'X': 'idx', 'T': 'idx serial linkserial', 'K': 'serial seq adds', 'RM': 'serial ' + _MOBJ,
    'S': 'idx floorheight ceilingheight floorpic ceilingpic lightlevel special tag soundtraversed soundtarget '
         'specialdata linecount',
    'L': 'idx special tag flags s0top s0mid s0bottom s0textureoffset s0rowoffset s1top s1mid s1bottom',
    'B': 'idx line where btexture btimer',
    'P': 'idx ingame mo playerstate cmd_forward cmd_side cmd_turn cmd_buttons viewheight deltaviewheight bob health '
         'armorpoints armortype pw0 pw1 pw2 pw3 pw4 pw5 card0 card1 card2 card3 card4 card5 backpack frag0 frag1 '
         'frag2 frag3 readyweapon pendingweapon wo0 wo1 wo2 wo3 wo4 wo5 wo6 wo7 wo8 ammo0 ammo1 ammo2 ammo3 maxammo0 '
         'maxammo1 maxammo2 maxammo3 attackdown usedown cheats refire killcount itemcount secretcount damagecount '
         'bonuscount attacker extralight fixedcolormap colormap ps0state ps0tics ps0sx ps0sy ps1state ps1tics ps1sx '
         'ps1sy didsecret message',
}.items()}

# ---- C tables ------------------------------------------------------------------------------
_STATES = list(csv.DictReader((ROOT / 'tests/reference/m3c_states.csv').open()))
_TYPES = list(csv.DictReader((ROOT / 'tests/reference/m3c_types.csv').open()))
NEXT = [int(r['next']) for r in _STATES]
TICS = [int(r['tics']) for r in _STATES]
TYPE_NAMES = [r['name'] for r in _TYPES]
INFO_FLAGS = [int(r['flags']) for r in _TYPES]
CLOSURE_FIELDS = ('spawnstate', 'seestate', 'painstate', 'meleestate', 'missilestate', 'deathstate', 'xdeathstate')
S_GIBS = enum_index('statenum_t', 'S_GIBS')
MT_TELEPORTMAN = enum_index('mobjtype_t', 'MT_TELEPORTMAN')
# the four types wire 24 adds to wire 23's (M4b decision I1)
NEW_TYPES = tuple(enum_index('mobjtype_t', t) for t in ('MT_BRUISER', 'MT_BRUISERSHOT', 'MT_TFOG', 'MT_TELEPORTMAN'))
assert len(NEXT) == 967 and len(TYPE_NAMES) == 137 and _STATES[S_GIBS]['name'] == 'S_GIBS'


# States the source sets by name outside a type's info.c fields, for the types an Episode 1
# map admits: the player's S_PLAY_ATK2 (A_GunFlash p_pspr.c:453, A_FirePistol :653,
# A_FireShotgun :676, A_FireShotgun2 :706, A_FireCGun :742). Every other literal P_SetMobjState of
# p_*.c names a state already in a closure (S_PLAY, S_PLAY_RUN1, S_PLAY_ATK1, S_PUFF3,
# S_BLOOD2/3), S_GIBS (handled below) or a type outside Episode 1 (Archvile, boss brain).
DIRECT = {enum_index('mobjtype_t', 'MT_PLAYER'): (enum_index('statenum_t', 'S_PLAY_ATK2'),)}


def normal_closure(t):
    """The states of type `t` reachable from its info.c state fields other than raisestate,
    following nextstate (S_NULL, 0, ends a chain and is not a member)."""
    row = _TYPES[t]; out = set(); todo = [int(row[f]) for f in CLOSURE_FIELDS] + list(DIRECT.get(t, ()))
    while todo:
        s = todo.pop()
        if s == 0 or s in out: continue
        out.add(s); todo.append(NEXT[s])
    return frozenset(out)


CLOSURES = [normal_closure(t) for t in range(len(TYPE_NAMES))]


def demo_header(path):
    """(skill, map) from a version-110 demo header (g_game.c G_DoPlayDemo, G_BeginRecording):
    version, skill, episode, map, deathmatch, respawnparm, fastparm, nomonsters,
    consoleplayer, playeringame[4]. Wire 24 admits skill 2 or 3, episode 1, map 1-9 and a
    single-player, plain header."""
    data = Path(path).read_bytes()
    if len(data) < 13: raise ValueError(f'{path}: a demo header is 13 bytes')
    version, skill, episode, map_number, dm, respawn, fast, nomonsters, console = data[:9]
    if version != 110: raise ValueError(f'{path}: demo version {version}, not 110')
    if skill not in SKILLS: raise ValueError(f'{path}: skill {skill} outside 2-3 (decision I4)')
    if episode != 1 or not 1 <= map_number <= 9: raise ValueError(f'{path}: E{episode}M{map_number} is not an Episode 1 map')
    if (dm, respawn, fast, nomonsters, console) != (0, 0, 0, 0, 0) or data[9:13] != b'\1\0\0\0':
        raise ValueError(f'{path}: not a plain single-player header')
    return skill, map_number


class Dump(L.Dump):
    """A dump file, refused unless every row kind and column list is the driver's."""
    def __init__(self, path):
        columns, rows = L.m3h9c_dump.parse(path)
        label = Path(path).stem
        for kind, cols in columns.items():
            if COLUMNS.get(kind) != cols:
                raise ProjectionError('DumpDeficiency', label, f'row kind {kind} has columns outside the driver contract')
        if set(columns) != set(COLUMNS):
            raise ProjectionError('DumpDeficiency', label, f'row kinds {sorted(set(COLUMNS) - set(columns))} missing')
        for r in rows:
            if r[0] not in COLUMNS or len(r) - 1 != len(COLUMNS[r[0]]):
                raise ProjectionError('DumpDeficiency', label, f'a row of unknown kind or width ({r[0]}, {len(r) - 1} values)')
        super().__init__(path)


class Projector(L.Projector):
    """The DSIM40 projector: the frozen DSIM39 (`episode1`) projection with the DSIM40
    header, `level.map` and the checks above."""
    def __init__(self, mapdata, skill, map_number):
        if skill not in SKILLS or not 1 <= map_number <= 9: raise ValueError('skill 2-3 and map 1-9')
        super().__init__(mapdata, skill, False, EPISODE1V2)
        self.header_skill, self.map_number = skill, map_number

    def _fields(self, d):
        self._v2_checks(d)
        f = super()._fields(d)
        k = [p for p, _ in f].index('level.secret_exit')
        f.insert(k + 1, ('level.map', n('B', self.map_number)))
        return f

    def _v2_checks(self, d):
        tic, h = d.tic, d.h
        if h['gameskill'] != self.header_skill:
            raise ProjectionError('DumpDeficiency', tic, f'H.gameskill {h["gameskill"]} differs from the demo header')
        if h['gamestate'] != GS_LEVEL: raise ProjectionError('Unadmitted', tic, f'gamestate {h["gamestate"]}')
        if [p['ingame'] for p in d.players] != [1, 0, 0, 0] or d.players[0]['cheats']:
            raise ProjectionError('Unadmitted', tic, 'not a single-player game without cheats')
        for e in self.entries:
            if e.cls != 'actor': continue
            r = e.row; t, s = r['type'], r['state']
            if not 0 <= t < len(TYPE_NAMES): raise ProjectionError('Unadmitted', tic, f'mobj type {t}')
            if t == MT_TELEPORTMAN:
                if e.pending or s != 0 or r['tics'] != -1 or r['flags'] != INFO_FLAGS[t]:
                    raise ProjectionError('Unadmitted', tic, 'an MT_TELEPORTMAN outside S_NULL/tics -1/info flags')
                continue
            if s in CLOSURES[t] or (s == S_GIBS and r['health'] <= 0) or (e.pending and s == -1): continue
            raise ProjectionError('Unadmitted', tic, f'{TYPE_NAMES[t]} in state {s}, outside its normal closure')


def snapshots(directory):
    """(dump, following dump or None) pairs of a run, in order (strict dumps)."""
    names = L.m3h9c_dump.snapshot_names(directory)
    following = Dump(os.path.join(directory, names[0])) if names else None
    for k in range(len(names)):
        dump = following
        following = Dump(os.path.join(directory, names[k + 1])) if k + 1 < len(names) else None
        yield dump, following


def project(directory, mapdata, skill, map_number):
    """Yield (dsim tic, field list) for every snapshot of a dump directory."""
    proj = Projector(mapdata, skill, map_number)
    for dump, following in snapshots(directory):
        fields, _ = proj.step(dump, following)
        yield dump.tic, fields


def compare39(directory, mapdata, skill, map_number):
    """Project every snapshot with DSIM40 and with the frozen DSIM39 profile; raise
    AssertionError unless the two field lists differ exactly in DELTA_FIELDS (and the
    DSIM39 list lacks `level.map`). Returns the snapshot count."""
    p39 = L.Projector(mapdata, skill, False, L.EPISODE1); count = 0
    for (tic, f40), (dump, following) in zip(project(directory, mapdata, skill, map_number), L.snapshots(directory)):
        f39, _ = p39.step(dump, following)
        assert dump.tic == tic
        rest40 = [(p, v) for p, v in f40 if p not in DELTA_FIELDS]
        rest39 = [(p, v) for p, v in f39 if p not in DELTA_FIELDS]
        assert rest40 == rest39, f'tic {tic}: DSIM40 differs from DSIM39 outside the delta'
        v40, v39 = dict(f40), dict(f39)
        assert 'level.map' not in v39 and v40['level.map'] == n('B', map_number)
        assert [p for p, _ in f40].index('level.map') == [p for p, _ in f40].index('level.secret_exit') + 1
        assert (v39['profile'], v40['profile']) == (n('I', 23), n('I', 24))
        assert (v39['context.schema'], v40['context.schema']) == (L.EPISODE1.context, CONTEXT)
        assert v39['actors.options'] == v40['actors.options'] == bytes([skill, 0, 0])
        count += 1
    return count


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('dump'); ap.add_argument('--wad', required=True); ap.add_argument('--demo', required=True)
    ap.add_argument('--out'); ap.add_argument('--compare39', action='store_true')
    a = ap.parse_args()
    skill, map_number = demo_header(a.demo)
    mapdata = L.MapData(a.wad, f'E1M{map_number}', L.demo_lumps(a.demo))
    if a.out: os.makedirs(a.out, exist_ok=True)
    count = 0; seq = hashlib.sha256(); last = None
    try:
        for tic, fields in project(a.dump, mapdata, skill, map_number):
            data = L.encode(fields, VERSION); count += 1; seq.update(hashlib.sha256(data).digest()); last = data
            if a.out: Path(a.out, f'tic_{tic:05}.bin').write_bytes(data)
        if a.compare39: print(f'DSIM39 comparison passed: {compare39(a.dump, mapdata, skill, map_number)} snapshots')
    except ProjectionError as err:
        print(f'projection stopped: {err.kind} at tic {err.tic}: {err.text}')
        print(f'projected {count} snapshots'); return 1
    print(f'projected {count} snapshots; sequence sha256 {seq.hexdigest()}; final fnv1a64 {L.fnv(last):016x}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
