#!/usr/bin/env python3
"""Unit checks of the DSIM40 projection's tables and header rules (no dumps needed).

The type and state numbers come from the enum order of linuxdoom-1.10/info.h and the
mobjinfo/states text of info.c, parsed here, never hand-typed; they are compared with the
C-compiled tables level_snapshot_m4b.py reads (tests/reference/m3c_*.csv). The dump
column contract is compared with the COLUMNS string of tools/reference/m4a_driver.c."""
import re
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import level_snapshot as L          # noqa: E402
import level_snapshot_m4b as M      # noqa: E402
from m3h9_geometry import enum_members  # noqa: E402

SOURCE = HERE.parents[1] / 'linuxdoom-1.10'
TYPES = enum_members('mobjtype_t')
STATES = enum_members('statenum_t')
assert (TYPES.pop(), STATES.pop()) == ('NUMMOBJTYPES', 'NUMSTATES')


def info_c_states():
    """name -> next-state name from info.c's states[] lines `{SPR,frame,tics,{action},NEXT,..},  // S_NAME`."""
    out = {}
    for m in re.finditer(r'\{SPR_\w+,\s*\d+,\s*-?\d+,\s*\{\w+\},\s*(S_\w+),[^}]*\},?\s*//\s*(S_\w+)', (SOURCE / 'info.c').read_text()):
        out[m.group(2)] = m.group(1)
    return out


def info_c_mobj(name):
    """The 23 fields of one mobjinfo record of info.c as source text."""
    text = (SOURCE / 'info.c').read_text()
    body = text[text.index('// ' + name + '\n'):]
    body = body[:body.index('}')]
    return [line.split('//')[0].strip().rstrip(',') for line in body.split('\n')[1:] if '//' in line]


class Tables(unittest.TestCase):
    def test_enum_order_matches_the_compiled_tables(self):
        self.assertEqual(TYPES, M.TYPE_NAMES)
        self.assertEqual(len(STATES), len(M.NEXT))
        self.assertEqual(STATES[:3], ['S_NULL', 'S_LIGHTDONE', 'S_PUNCH'])

    def test_next_states_match_info_c(self):
        nxt = info_c_states()
        self.assertEqual(len(nxt), len(STATES))
        self.assertEqual([STATES.index(nxt[s]) for s in STATES], M.NEXT)

    def test_new_types_and_their_info_records(self):
        self.assertEqual(M.NEW_TYPES, tuple(TYPES.index(t) for t in ('MT_BRUISER', 'MT_BRUISERSHOT', 'MT_TFOG', 'MT_TELEPORTMAN')))
        flags = {'MF_SOLID': 2, 'MF_SHOOTABLE': 4, 'MF_NOSECTOR': 8, 'MF_NOBLOCKMAP': 16, 'MF_NOGRAVITY': 0x200,
                 'MF_DROPOFF': 0x400, 'MF_MISSILE': 0x10000, 'MF_COUNTKILL': 0x400000}
        for name in ('MT_BRUISER', 'MT_BRUISERSHOT', 'MT_TFOG', 'MT_TELEPORTMAN'):
            rec = info_c_mobj(name)
            self.assertEqual(len(rec), 23, name)
            t = TYPES.index(name)
            self.assertEqual(sum(flags[f] for f in rec[21].split('|')), M.INFO_FLAGS[t], name)
            fields = [rec[i] for i in (1, 3, 7, 10, 11, 12, 13)]      # the closure fields of info.c's order
            self.assertEqual([STATES.index(s) for s in fields], [int(M._TYPES[t][f]) for f in M.CLOSURE_FIELDS], name)

    def test_normal_closures_of_the_new_types(self):
        span = lambda a, b: frozenset(range(STATES.index(a), STATES.index(b) + 1))
        self.assertEqual(M.CLOSURES[TYPES.index('MT_BRUISER')], span('S_BOSS_STND', 'S_BOSS_DIE7'))
        self.assertEqual(M.CLOSURES[TYPES.index('MT_BRUISERSHOT')], span('S_BRBALL1', 'S_BRBALLX3'))
        self.assertEqual(M.CLOSURES[TYPES.index('MT_TFOG')], span('S_TFOG', 'S_TFOG10'))
        self.assertEqual(M.CLOSURES[TYPES.index('MT_TELEPORTMAN')], frozenset())
        self.assertFalse(span('S_BOSS_RAISE1', 'S_BOSS_RAISE7') & M.CLOSURES[TYPES.index('MT_BRUISER')])
        self.assertIn(STATES.index('S_PLAY_ATK2'), M.CLOSURES[TYPES.index('MT_PLAYER')])

    def test_numbers_quoted_in_state_format(self):
        quoted = {'MT_BRUISER': 15, 'MT_BRUISERSHOT': 16, 'MT_TFOG': 39, 'MT_TELEPORTMAN': 41}
        self.assertEqual({k: TYPES.index(k) for k in quoted}, quoted)
        states = {'S_TFOG': 130, 'S_TFOG10': 141, 'S_BRBALL1': 522, 'S_BRBALLX3': 526, 'S_BOSS_STND': 527,
                  'S_BOSS_ATK3': 539, 'S_BOSS_PAIN': 540, 'S_BOSS_DIE1': 542, 'S_BOSS_DIE7': 548, 'S_BOSS_RAISE1': 549}
        self.assertEqual({k: STATES.index(k) for k in states}, states)


class Contract(unittest.TestCase):
    def test_columns_equal_the_m4a_driver(self):
        text = (HERE / 'm4a_driver.c').read_text()
        body = text[text.index('static const char* COLUMNS ='):]
        body = ''.join(re.findall(r'"((?:[^"\\]|\\.)*)"', body[:body.index(';')])).replace('\\n', '\n')
        driver = {f[1]: f[2:] for f in (line.split(',') for line in body.strip().split('\n'))}
        self.assertEqual(driver, M.COLUMNS)

    def test_profile(self):
        self.assertEqual((M.EPISODE1V2.version, M.EPISODE1V2.policy, M.EPISODE1V2.context),
                         (40, 24, b'PLAYER-MONSTERS-EPISODE1-2-v1'))
        self.assertEqual(M.EPISODE1V2.max_slots, L.EPISODE1.max_slots)
        self.assertEqual(M.EPISODE1V2.messages, L.EPISODE1.messages)
        self.assertNotIn('episode1v2', L.PROFILES)        # the frozen module is untouched


class Header(unittest.TestCase):
    def header(self, data):
        with tempfile.NamedTemporaryFile(suffix='.lmp', delete=False) as f:
            f.write(bytes(data) + b'\x80')
        try:
            return M.demo_header(f.name)
        finally:
            Path(f.name).unlink()

    def test_admitted(self):
        self.assertEqual(self.header([110, 2, 1, 8, 0, 0, 0, 0, 0, 1, 0, 0, 0]), (2, 8))
        self.assertEqual(self.header([110, 3, 1, 5, 0, 0, 0, 0, 0, 1, 0, 0, 0]), (3, 5))

    def test_refused(self):
        good = [110, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0]
        for index, value in [(0, 109), (1, 1), (1, 4), (2, 2), (3, 0), (3, 10), (4, 1), (5, 1), (6, 1), (7, 1),
                             (8, 1), (10, 1)]:
            bad = list(good); bad[index] = value
            with self.subTest(index=index, value=value), self.assertRaises(ValueError):
                self.header(bad)


if __name__ == '__main__':
    unittest.main()
