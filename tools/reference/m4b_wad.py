#!/usr/bin/env python3
"""Synthetic mechanism WADs for the M4b whole-engine runs (decision I10).

Writes small, fully synthetic IWADs under tests/reference/m4b_wads/, each holding one map
in the slot its mechanism needs (E1M8 for A_BossDeath, E1M1 otherwise) and everything the
headless genuine linuxdoom-1.10 engine needs to load it in `registered` mode:

    tele      line 97 (WR teleport): a back-side crossing (nothing), a front-side crossing
              (two fogs, 18-tic freeze, landing angle, momentum zeroed); a tag with three
              tagged sectors (none, two, one landing) for the search order
    telemove  a teleport inside the first half of a split move (P_XYMovement) and inside a
              wall slide (P_SlideMove), each landing on an item
    tfrag     three chained teleports onto a zombieman (mass 100), a Demon (400) and a
              Baron (1000) standing on the landings: telefrags (P_DamageMobj 10000)
    montele   two Demons walking across 97 lines: one landing free, one blocked by a barrel
    slideb    a barrel pushed across a 97 line by another barrel's explosion
    slidec    a zombieman corpse sliding across a 97 line after a rocket kill
    baron     a Baron of Hell: claws, fireballs hitting the player and the wall, pain, death
    infight   a Baron and a Spectre: a fireball hits the Spectre, which turns on the Baron
    bvb       a Baron on a ledge, a barrel on a pedestal in its line of fire and a sleeping
              Baron beside the barrel: Baron against Baron through barrel splash
    boss8     E1M8 slot: two Barons on teleport landings, a tag-666 step, a special-5 line
              that can make the step busy, a sector-11 exit room
    bossm1    the boss8 map in the E1M1 slot (A_BossDeath does nothing)
    spech8    eight special lines inside one player box (allowed)
    spech9    the same with a ninth (the genuine engine writes past spechit[8])
    skill     things for skill bits 1, 2, 4, all and multiplayer-only, and a hard-only landing

and the second set (tests/reference/m4b_manifest2.json, added in M4b step 5 so that the first set's files
stay byte-identical):

    tele2x    two teleports in one tic: a split move's first half crosses one 97 line, its second half
              continues from the landing across a second 97 line
    sawbite   a chainsaw and two Demons in a short hall: one Demon bites the player in a tic in which
              A_Saw hit the other and set MF_JUSTATTACKED on the player (M4b step 7)
    btunnel   a caged Baron's fireball steps wholly past a 1-unit inner void slab (the tunnel
              window of a 15-unit step and radius 6) and explodes on the far wall (M4b step 9)

The lump framing, game data, grid painter, side textures and builder chain are the frozen
M4a generator's (tools/reference/m4a_wad.py, imported read-only, which itself imports
m3h9c_wad.py and m3h9b_geometry.py); its WADs are untouched. This file adds the M4b maps,
the admitted special set (M4a's part-1 set plus 97), the slot per map and the skill-aware
spawn counts. tools/reference/m4b_check_wad.py re-parses and checks every WAD without
importing this file. tests/reference/m4b_manifest.json (schema DOOM-M4B-MANIFEST-v1) names
each WAD with its slot, SHA-256 and spawned-thing counts on skills 2 and 3 (written here with
--manifest, checked by --check) and its scripts with their genuine-engine expectations
(authored, checked by m4b_reference.py).

    m4b_wad.py [--out-dir DIR] [--only NAME]     write the WADs
    m4b_wad.py --list                            name, file, slot, sectors, lines, things
    m4b_wad.py --manifest                        also refresh the manifest's wad fields
    m4b_wad.py --check                           regenerate in memory, compare with the
                                                 committed WADs and the manifest's wad fields,
                                                 then run m4b_check_wad.py on each WAD
"""
import argparse, hashlib, json, struct, subprocess, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m4a_wad as A              # noqa: E402  frozen M4a generator (painter, sides, framing), read-only
from m4a_wad import W            # noqa: E402  frozen M3h9c game data

OUT_DIR = ROOT / 'tests/reference/m4b_wads'
MANIFEST = ROOT / 'tests/reference/m4b_manifest.json'
CHECKER = ROOT / 'tools/reference/m4b_check_wad.py'
MANIFEST_SCHEMA = 'DOOM-M4B-MANIFEST-v1'

PLAYER1, ZOMBIE, SHOTGUY, IMP, DEMON, SPECTRE, BARON = 1, 3004, 9, 3001, 3002, 58, 3003
TELEPORTMAN, BARREL = 14, 2035
LAUNCHER, ROCKET, ROCKETBOX, SHOTGUN = 2003, 2010, 2046, 2001
SHELLS, CLIP, STIMPACK, MEDIKIT, HEALTHBONUS = 2008, 2007, 2011, 2012, 2014
SOULSPHERE, BLUEARMOR, BACKPACK = 2013, 2019, 8
CHAINSAW = 2005
ALL_SKILLS, AMBUSH, NOTSINGLE = 7, 8, 16
EAST, NORTH, WEST, SOUTH = 0, 90, 180, 270
TELEPORT = 97
SPECIALS = A.PART1 | {TELEPORT}


class Plan(A.Plan):
    """m4a_wad.Plan with the M4b special set (part 1 plus the WR teleport 97) and thin void holes."""

    def __init__(self, name):
        super().__init__(name)
        self.voids = []

    def special(self, front, back, at, special, tag=0):
        assert special in SPECIALS, special
        self.specials.append((front, back, at, special, tag))

    def void(self, sector, x0, y0, x1, y1):
        """A void hole (x0, y0)-(x1, y1) of any integer size inside one sector, bounded by four one-sided
        walls; the grid painter's rectangles are multiples of 16, so a wall thinner than that (BTUNNEL's
        1-unit slab) is added here (M4b step 9). Nothing else may touch it: the hole and a 1-unit margin
        lie inside one rectangle of `sector` and meet no rectangle of another sector."""
        assert x0 < x1 and y0 < y1 and not self.specials, (x0, y0, x1, y1)
        inside = lambda r: r[0] < x0 and r[1] < y0 and x1 < r[2] and y1 < r[3]
        meets = lambda r: r[0] <= x1 + 1 and x0 - 1 <= r[2] and r[1] <= y1 + 1 and y0 - 1 <= r[3]
        assert any(inside(r) for r in self.sectors[sector]['rects']), 'void outside its sector'
        assert not any(meets(r) for k, s in enumerate(self.sectors) if k != sector for r in s['rects']), 'void meets another sector'
        self.voids.append((sector, (x0, y0, x1, y1)))


def build_map(plan):
    """m4a_wad.build_map, plus the plan's void holes; a plan without one goes through m4a_wad.build_map
    unchanged, so every earlier WAD stays byte-identical. Each hole is a counter-clockwise ring starting
    at its top-left vertex (the painter's hole convention: the sector on the right of every edge, so
    each edge is a one-sided wall facing into the sector), appended after the painter's rings."""
    if not plan.voids:
        return A.build_map(plan)
    rs = A.rings(plan)
    for s, (x0, y0, x1, y1) in plan.voids:
        rs[s].append([((x0, y1), None), ((x0, y0), None), ((x1, y0), None), ((x1, y1), None)])
    sectors = [([[v for v, _ in r] for r in sr], s['floor'], s['ceil'], dict(s['attrs'])) for s, sr in zip(plan.sectors, rs)]
    m = A.G.room(sectors, portals={}, reject=(), tree=None, segment=4096, lines={})
    m['rings'] = rs
    return m


def tele():
    p = Plan('tele')
    west = p.sector('WEST', [(-512, -128, 0, 128)])
    east = p.sector('EAST', [(0, -128, 512, 128)])
    p.sector('D1', [(1024, -128, 1280, 128)], tag=1)
    north = p.sector('N', [(128, 128, 256, 192)])
    p.sector('T2A', [(1024, 512, 1280, 768)], tag=2)          # tagged, no landing
    p.sector('T2B', [(1536, 512, 1792, 768)], tag=2)          # two landings: the first in THINGS order wins
    p.sector('T2C', [(2048, 512, 2304, 768)], tag=2)          # its landing is first in THINGS, but the sector is later
    p.special(west, east, (0, 0), TELEPORT, 1)                 # front WEST: east-bound crossings teleport
    p.special(east, north, (192, 128), TELEPORT, 2)            # front EAST: north-bound crossings teleport
    p.thing(192, 0, WEST, PLAYER1)
    p.thing(2176, 640, SOUTH, TELEPORTMAN)                     # T2C
    p.thing(1600, 640, EAST, TELEPORTMAN)                      # T2B, first
    p.thing(1728, 640, WEST, TELEPORTMAN)                      # T2B, second
    p.thing(1152, 0, NORTH, TELEPORTMAN)                       # D1
    p.thing(1152, 0, EAST, HEALTHBONUS)                        # on the D1 landing
    return p


def telemove():
    p = Plan('telemove')
    hall = p.sector('HALL', [(-1024, -128, 0, 128)])
    pada = p.sector('PADA', [(0, -128, 128, 0)])
    padb = p.sector('PADB', [(0, 0, 128, 128)])
    p.sector('DA', [(1024, -384, 1280, -128)], tag=1)
    p.sector('DB', [(1024, 128, 1280, 384)], tag=2)
    p.special(hall, pada, (0, -64), TELEPORT, 1)
    p.special(hall, padb, (0, 64), TELEPORT, 2)
    p.thing(-952, -64, EAST, PLAYER1)
    p.thing(1152, -256, NORTH, TELEPORTMAN)
    p.thing(1152, -256, EAST, HEALTHBONUS)
    p.thing(1152, 256, EAST, TELEPORTMAN)
    p.thing(1152, 256, EAST, HEALTHBONUS)
    return p


def tfrag():
    p = Plan('tfrag')
    hall = p.sector('HALL', [(-512, -128, 0, 128)])
    pad1 = p.sector('PAD1', [(0, -128, 128, 128)])
    r1 = p.sector('R1', [(1024, -256, 1536, 256)], tag=1)
    pad2 = p.sector('PAD2', [(1536, -128, 1664, 128)])
    r2 = p.sector('R2', [(2048, -256, 2560, 256)], tag=2)
    pad3 = p.sector('PAD3', [(2560, -128, 2688, 128)])
    p.sector('R3', [(3072, -256, 3584, 256)], tag=3)
    p.special(hall, pad1, (0, 0), TELEPORT, 1)
    p.special(r1, pad2, (1536, 0), TELEPORT, 2)
    p.special(r2, pad3, (2560, 0), TELEPORT, 3)
    # the victims come before the player in THINGS, so their thinkers run before the player's
    # and the telefrag tic's dump shows the thrust before any movement of theirs
    p.thing(1152, 0, EAST, ZOMBIE, ALL_SKILLS | AMBUSH)
    p.thing(2176, 0, EAST, DEMON, ALL_SKILLS | AMBUSH)
    p.thing(3200, 0, EAST, BARON, ALL_SKILLS | AMBUSH)
    p.thing(-448, 0, EAST, PLAYER1)
    p.thing(1152, 0, EAST, TELEPORTMAN)
    p.thing(2176, 0, EAST, TELEPORTMAN)
    p.thing(3200, 0, EAST, TELEPORTMAN)
    return p


def montele():
    p = Plan('montele')
    aw = p.sector('AW', [(-768, -128, -256, 128)])
    ae = p.sector('AE', [(256, -128, 768, 128)])
    b = p.sector('B', [(-256, -128, 256, 128)])
    p.sector('R1', [(1280, -128, 1536, 128)], tag=1)
    p.sector('R2', [(1280, 256, 1536, 512)], tag=2)
    p.special(aw, b, (-256, 0), TELEPORT, 1)                   # front AW: an east-bound Demon teleports
    p.special(ae, b, (256, 0), TELEPORT, 2)                    # front AE: a west-bound Demon tries to
    p.thing(0, 0, NORTH, PLAYER1)
    p.thing(-640, 0, EAST, DEMON)
    p.thing(640, 0, WEST, DEMON)
    p.thing(1408, 0, EAST, TELEPORTMAN)
    p.thing(1408, 384, EAST, TELEPORTMAN)
    p.thing(1408, 384, EAST, BARREL)                           # on the tag-2 landing: blocks a monster teleport
    return p


def slideb():
    p = Plan('slideb')
    hall = p.sector('HALL', [(-512, -256, 0, 256)])
    pad = p.sector('PAD', [(0, -256, 128, 256)])
    p.sector('R', [(1024, -128, 1280, 128)], tag=1)
    p.special(hall, pad, (0, 0), TELEPORT, 1)
    p.thing(-400, 100, EAST, PLAYER1)
    p.thing(-156, 100, EAST, BARREL)
    p.thing(-8, 100, EAST, BARREL)                             # the first, pushed east by the bullets, explodes about 125 units away: 14 splash damage
    p.thing(1152, 0, EAST, TELEPORTMAN)
    return p


def slidec():
    p = Plan('slidec')
    hall = p.sector('HALL', [(-512, -256, 0, 256)])
    pad = p.sector('PAD', [(0, -256, 128, 256)])
    p.sector('R', [(1024, -128, 1280, 128)], tag=1)
    p.special(hall, pad, (0, 0), TELEPORT, 1)
    p.thing(-224, -100, EAST, PLAYER1)
    p.thing(-192, -100, EAST, SHOTGUN)
    p.thing(-20, -100, EAST, ZOMBIE, ALL_SKILLS | AMBUSH)
    p.thing(1152, 0, EAST, TELEPORTMAN)
    return p


def baron():
    p = Plan('baron')
    p.sector('HALL', [(-768, -192, 640, 192)])
    p.thing(-704, 0, EAST, PLAYER1)
    for x, kind in ((-656, LAUNCHER), (-608, ROCKETBOX), (-560, ROCKETBOX), (-512, BLUEARMOR), (-464, SOULSPHERE), (-416, ROCKETBOX)):
        p.thing(x, 0, EAST, kind)
    p.thing(384, 0, WEST, BARON)
    return p


def infight():
    p = Plan('infight')
    p.sector('HALL', [(-768, -256, 768, 256)])
    p.thing(-704, 0, EAST, PLAYER1)
    p.thing(-640, 0, EAST, BLUEARMOR)
    p.thing(-576, 0, EAST, SOULSPHERE)
    p.thing(512, 0, WEST, BARON)
    p.thing(160, 0, WEST, SPECTRE)
    return p


def bvb():
    p = Plan('bvb')
    p.sector('HALL', [(-768, -256, 768, 256)])
    p.sector('CAGE', [(448, -32, 512, 32)], floor=32)       # a 32-unit ledge: the Baron on it cannot step down (24)
    p.sector('PED', [(-16, -16, 16, 16)], floor=32)         # the barrel's pedestal, at the fireballs' height
    p.thing(-704, 0, EAST, PLAYER1)
    p.thing(-640, 0, EAST, BLUEARMOR)
    p.thing(-576, 0, EAST, SOULSPHERE)
    p.thing(480, 0, WEST, BARON)                              # awake, firing at the player along y = 0
    p.thing(0, 0, EAST, BARREL)                               # in its line of fire
    p.thing(0, 112, EAST, BARON, ALL_SKILLS | AMBUSH)       # asleep, facing away, inside the barrel's blast radius
    return p


def boss8():
    p = Plan('boss8')
    hall = p.sector('HALL', [(-512, -128, 0, 128)])
    pad1 = p.sector('PAD1', [(0, -128, 128, 128)])
    r1 = p.sector('R1', [(1024, -256, 1536, 256)], tag=1)
    pad2 = p.sector('PAD2', [(1536, -128, 1664, 128)])
    r2 = p.sector('R2', [(2048, -256, 2560, 256)], tag=2)
    pad3 = p.sector('PAD3', [(2560, -128, 2688, 128)])
    r2s = p.sector('R2S', [(2112, -128, 2240, -64)])
    p.sector('H2', [(0, 512, 512, 768)], tag=3)
    p.sector('BLOCK', [(512, 576, 576, 704)], floor=64, tag=666)
    p.sector('END', [(576, 512, 832, 768)], special=11)
    p.special(hall, pad1, (0, 0), TELEPORT, 1)
    p.special(r1, pad2, (1536, 0), TELEPORT, 2)
    p.special(r2, pad3, (2560, 0), TELEPORT, 3)
    p.special(r2, r2s, (2176, -64), 5, 666)                    # W1 raiseFloor on the boss step: makes it busy
    p.thing(1152, 0, EAST, BARON, ALL_SKILLS | AMBUSH)
    p.thing(2176, 0, EAST, BARON, ALL_SKILLS | AMBUSH)
    p.thing(-448, 0, EAST, PLAYER1)
    p.thing(1152, 0, EAST, TELEPORTMAN)
    p.thing(2176, 0, EAST, TELEPORTMAN)
    p.thing(128, 640, EAST, TELEPORTMAN)
    return p


def spech(n):
    def plan():
        p = Plan(f'spech{n}')
        p.sector('HALL', [(-512, -128, 256, 128)])
        c = {(i, j): p.sector(f'C{i}{j}', [(64 + 16 * i, -16 + 16 * j, 80 + 16 * i, 16 * j)], tag=9 if (i, j) == (0, 0) else 0,
                              layer=1) for i in range(3) for j in range(3)}
        lines = [(c[0, j], c[1, j], (80, -8 + 16 * j)) for j in range(3)] + [(c[1, j], c[2, j], (96, -8 + 16 * j)) for j in range(3)]
        lines += [(c[i, 0], c[i, 1], (72 + 16 * i, 0)) for i in range(3)] + [(c[i, 1], c[i, 2], (72 + 16 * i, 16)) for i in range(3)]
        for front, back, at in lines[:n]:                       # 6 vertical, then the y = 0 row from the west
            p.special(front, back, at, TELEPORT, 9)             # no landing anywhere: EV_Teleport returns 0
        p.thing(-448, 8, EAST, PLAYER1)
        p.thing(-448, 96, EAST, HEALTHBONUS)                    # off the path (the player start is not the last THING)
        return p
    return plan


def skill():
    p = Plan('skill')
    hall = p.sector('HALL', [(-512, -128, 0, 128)])
    pad = p.sector('PAD', [(0, -128, 128, 128)])
    p.sector('R', [(1024, -128, 1280, 128)], tag=1)
    p.sector('PEN', [(-512, 512, -128, 768)])
    p.special(hall, pad, (0, 0), TELEPORT, 1)
    p.thing(-448, 0, EAST, PLAYER1)
    p.thing(1152, 0, EAST, TELEPORTMAN, 4)                     # hard only: no teleport on medium
    for x, kind, opt in ((-384, SHELLS, 1), (-320, CLIP, 2), (-256, STIMPACK, 4), (-192, MEDIKIT, 7), (-128, HEALTHBONUS, 7 | NOTSINGLE)):
        p.thing(x, 96, EAST, kind, opt)
    for x, kind, opt in ((-448, SHOTGUY, 1), (-384, ZOMBIE, 2), (-320, IMP, 4), (-256, BARON, 4), (-192, DEMON, 7),
                         (-192, SPECTRE, 7 | NOTSINGLE)):
        p.thing(x, 640 if kind != SPECTRE else 704, WEST, kind, opt | AMBUSH)
    return p


def tele2x():
    """Two teleports in one tic (owner request, 2026-11-10): TSPLIT's hall and run, so the first half of
    P_XYMovement's split move crosses the first 97 line; its landing stands 8 units west of a second 97
    line (tag 2), which the second half (continued from the landing, p_mobj.c:147-191) crosses."""
    p = Plan('tele2x')
    hall = p.sector('HALL', [(-1024, -128, 0, 128)])
    pad = p.sector('PAD', [(0, -128, 128, 128)])
    da = p.sector('DA', [(1024, -128, 1200, 128)], tag=1)
    db = p.sector('DB', [(1200, -128, 1456, 128)])
    p.sector('D2', [(1024, 512, 1280, 768)], tag=2)
    p.special(hall, pad, (0, 0), TELEPORT, 1)
    p.special(da, db, (1200, 0), TELEPORT, 2)                  # front DA: east-bound crossings teleport
    p.thing(-952, 0, EAST, PLAYER1)
    p.thing(1192, 0, EAST, TELEPORTMAN)                        # DA, 8 units west of the second line
    p.thing(1152, 640, NORTH, TELEPORTMAN)                     # D2
    p.thing(1152, 704, EAST, HEALTHBONUS)                      # off the path (the player start is not the last THING)
    return p


def sawbite():
    """A Demon bite on a body A_Saw flagged in the same tic (M4b step 7, the E1M5 M5WALK3 refusal): a
    hub between a west and an east corridor, a Demon in each. The player, facing west, picks up the
    chainsaw and blue armor and saws the west Demon; each A_Saw hit sets MF_JUSTATTACKED (p_pspr.c:542)
    in P_PlayerThink, which P_Ticker runs before the thinkers (p_tick.c:150-152), so the east Demon's
    A_SargAttack (p_enemy.c:935-948) bites the flagged body later in that tic. A spawned mobj's movedir
    is DI_EAST (0), so the east Demon starts near its corridor's east wall and turns back at once."""
    p = Plan('sawbite')
    p.sector('HALL', [(-48, -48, 48, 48), (-448, -32, -48, 32), (48, -32, 320, 32)])
    p.thing(0, 0, WEST, PLAYER1)
    p.thing(-32, 0, EAST, CHAINSAW)
    p.thing(-32, -32, EAST, BLUEARMOR)
    p.thing(-160, 0, EAST, DEMON)
    p.thing(288, 0, WEST, DEMON)
    return p


def btunnel():
    """A Baron fireball stepping wholly past an inner one-sided wall (M4b step 9, owner ruling 2026-11-13).
    MT_BRUISERSHOT moves 15 units a tic unsplit (speed 15, radius 6, info.c:1524-1548; p_mobj.c:150-164)
    and P_CheckPosition tests only the destination box (p_map.c:191-195, 374-442), so a wall thinner
    than 15 - 2*6 = 3 units can be stepped over. The Baron stands in a 64x48 cage ringed by a 32-unit
    moat, which it cannot enter (tmfloorz - tmdropoffz > 24, p_map.c:481-483); with speed 8
    (info.c:1514) it can only step east or west, to x = 24, 32 or 40, so its y stays 24. It must be able
    to step at all: P_TryWalk sets movecount only after a successful P_Move (p_enemy.c:349-357), so a
    Baron that can never move keeps movecount nonzero and never fires (p_enemy.c:738-742, 762-766).
    Cage and hall floors are both 0, so momz is 0 (p_mobj.c:917-923). The player stands due north; the
    fireball (y = 24 + (momy >> 1) + n*momy, p_mobj.c:876-878, 152-164) reaches the 1-unit void slab at
    y = 368 with half a unit to spare on each side of the tunnel window for all three Baron positions,
    and its box straddles the north wall (y = 496) on the first step that reaches it, so it explodes
    there (p_mobj.c:166-186). tools/reference/m4b_check_wad.py re-derives all of this from the WAD. The
    script stands still until the first fireball is spawned, then strafes out of its path."""
    p = Plan('btunnel')
    hall = p.sector('HALL', [(-128, -64, 192, 496)])
    p.sector('MOAT', [(-16, -16, 80, 64)], floor=-32)
    p.sector('CAGE', [(0, 0, 64, 48)])
    p.void(hall, -32, 368, 96, 369)
    p.thing(32, 312, SOUTH, PLAYER1)
    p.thing(32, 24, NORTH, BARON)
    return p


def telez():
    """A monster teleport whose first fog keeps the monster's pre-move z (M4b step 14, review finding F1).
    P_Move calls P_TryMove (p_enemy.c:293), whose crossing loop runs EV_Teleport (p_map.c:500-514) after
    x, y and floorz are set (p_map.c:490-495) but before P_Move sets z = floorz (p_enemy.c:327-328);
    EV_Teleport spawns fog 1 at oldz = thing->z (p_telept.c:100-102, 111). The fog is appended to the
    thinker list (P_AddThinker, p_tick.c:65-71) and runs P_ZMovement in the same tic, which lifts it to
    its own centre sector's floor if it is below (p_mobj.c:428-436, 284-311), so oldz shows only when
    that floor is at or below oldz and the box's highest floor (tmfloorz) is above it. The Demon spawns
    at its centre sector's floor (P_SpawnMobj, p_mobj.c:517-525) in LOW (floor 0), 8 units west of a 97
    line to MID (floor 0); HIGH (floor 24) lies north of MID and LOW from y = 16, inside the Demon's box
    (radius 30, info.c) before and after the step. z stays 0 until the Demon first moves (P_MobjThinker
    runs P_ZMovement only if z != floorz, p_mobj.c:428-436, and floorz is the centre sector's). The step
    up of exactly 24 passes P_TryMove (p_map.c:473-483). The player stands due east in MID, so the
    Demon's first P_Move (DI_EAST either way in P_NewChaseDir) crosses the 97 line from its front side
    and teleports; fog 1 stands at (2, 0) in MID at z 0 (the C order) and not at the box's floor 24."""
    p = Plan('telez')
    low = p.sector('LOW', [(-256, -128, 0, 16)])
    mid = p.sector('MID', [(0, -128, 512, 16)])
    p.sector('HIGH', [(-256, 16, 512, 128)], floor=24)
    p.sector('DEST', [(1024, -128, 1280, 128)], tag=1)
    p.special(low, mid, (0, 0), TELEPORT, 1)                   # front LOW: the east-bound Demon teleports
    p.thing(448, -8, WEST, PLAYER1)
    p.thing(-8, 0, EAST, DEMON)
    p.thing(1152, 0, EAST, TELEPORTMAN)
    return p


MAPS = {'tele': (tele, 'E1M1'), 'telemove': (telemove, 'E1M1'), 'tfrag': (tfrag, 'E1M1'), 'montele': (montele, 'E1M1'),
        'slideb': (slideb, 'E1M1'), 'slidec': (slidec, 'E1M1'), 'baron': (baron, 'E1M1'), 'infight': (infight, 'E1M1'), 'bvb': (bvb, 'E1M1'),
        'boss8': (boss8, 'E1M8'), 'bossm1': (boss8, 'E1M1'), 'spech8': (spech(8), 'E1M1'), 'spech9': (spech(9), 'E1M1'),
        'skill': (skill, 'E1M1')}
# The second M4b set (owner request, 2026-11-10): new WADs listed in tests/reference/m4b_manifest2.json, so
# that tests/reference/m4b_manifest.json and every file of the first set stay byte-identical.
MAPS2 = {'tele2x': (tele2x, 'E1M1'), 'sawbite': (sawbite, 'E1M1'),
         'btunnel': (btunnel, 'E1M1'), 'telez': (telez, 'E1M1')}
MANIFEST2 = ROOT / 'tests/reference/m4b_manifest2.json'
SETS = [(MAPS, MANIFEST), (MAPS2, MANIFEST2)]
ALL_MAPS = {**MAPS, **MAPS2}


def labels(name):
    """Sector label -> index for a map (used by the event expressions of m4b_reference.py)."""
    return {s['label']: k for k, s in enumerate(ALL_MAPS[name][0]().sectors)}


def wad_bytes(name):
    """The WAD of one map in its slot, with the m4a_wad/m3h9c_wad lump framing."""
    plan_fn, slot = ALL_MAPS[name]
    plan = plan_fn()
    m = build_map(plan)
    A.fix_sides(plan, m)
    lumps = dict(m['lumps'])
    lumps['THINGS'] = b''.join(struct.pack('<hhhhh', *t) for t in plan.things)
    assert list(lumps) == ['MAP01'] + A.MAP_LUMPS, list(lumps)
    flat = bytes(4096)
    wad = ([('PLAYPAL', W.playpal()), ('COLORMAP', W.colormap()), ('S_START', b''), ('S_END', b''), ('F_START', b'')] +
           [(f, flat) for f in W.FLATS] + [('F_END', b''), ('PNAMES', W.pnames()), ('TEXTURE1', W.texture1()),
           (W.PATCH, W.blank_patch(W.TEX_W, W.TEX_H)), (slot, b'')] + [(n, lumps[n]) for n in A.MAP_LUMPS])
    data = bytearray(b'IWAD' + struct.pack('<ii', len(wad), 0))
    directory = b''
    for n, b in wad:
        directory += struct.pack('<ii8s', len(data), len(b), W.name8(n))
        data += b
    struct.pack_into('<i', data, 8, len(data))
    return bytes(data + directory), plan, m, slot


def spawned(plan, skill):
    """THING counts spawned in single player on a 0-based gameskill (P_SpawnMapThing,
    p_mobj.c:740-751: options & 16 skips; bit 1 for skills 0-1, 4 for skills 3-4, else
    1 << (skill - 1)), keyed by DoomEd number; the player start is counted too."""
    bit = 1 if skill <= 1 else 4 if skill >= 3 else 1 << (skill - 1)
    out = {}
    for t in plan.things:
        if t[3] != PLAYER1 and (t[4] & NOTSINGLE or not t[4] & bit): continue
        out[str(t[3])] = out.get(str(t[3]), 0) + 1
    return dict(sorted(out.items(), key=lambda kv: int(kv[0])))


def wad_fields(name, data, plan, m, slot):
    return dict(name=name.upper(), file=f'tests/reference/m4b_wads/{name}.wad', slot=slot, episode=int(slot[1]), map=int(slot[3]),
                sha256=hashlib.sha256(data).hexdigest(), bytes=len(data), sectors=len(m['heights']), lines=len(m['lines']),
                things={str(s): spawned(plan, s) for s in (2, 3)})


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--out-dir', type=Path, default=OUT_DIR)
    ap.add_argument('--only', choices=list(ALL_MAPS))
    ap.add_argument('--list', action='store_true')
    ap.add_argument('--manifest', action='store_true')
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    status = 0
    for maps, manifest in SETS:
        names = [n for n in maps if not a.only or n == a.only]
        if names: status |= run_set(a, names, manifest)
    sys.exit(status)


def run_set(a, names, manifest):
    """One set: its maps and its manifest (write, list, refresh the manifest, or check)."""
    fields, bad = {}, []
    for n in names:
        data, plan, m, slot = wad_bytes(n)
        f = fields[n] = wad_fields(n, data, plan, m, slot)
        if a.list:
            print(f"{f['name']}: {f['file']} slot {slot}, {f['bytes']} bytes, sectors {f['sectors']}, lines {f['lines']}, "
                  f"things skill 2 {sum(f['things']['2'].values())}, skill 3 {sum(f['things']['3'].values())}")
            continue
        if a.check:
            path = ROOT / f['file']
            if not path.exists() or path.read_bytes() != data: bad.append(f['file'])
            else: print(f"{f['file']} matches: {len(data)} bytes, sha256 {f['sha256']}", flush=True)
        else:
            a.out_dir.mkdir(parents=True, exist_ok=True)
            (a.out_dir / f'{n}.wad').write_bytes(data)
            print(f"wrote {a.out_dir / (n + '.wad')}: {len(data)} bytes")
    if a.list: return 0
    if a.check:
        if bad: sys.exit('differs or missing: ' + ', '.join(bad))
        man = json.loads(manifest.read_text())
        if man.get('schema') != MANIFEST_SCHEMA: sys.exit('manifest schema')
        by = {w['name']: w for w in man['wads']}
        for n in names:
            w = by.get(n.upper())
            if w is None: sys.exit(f'{n}: not in the manifest')
            for k, v in fields[n].items():
                if w.get(k) != v: sys.exit(f'{n}: manifest field {k} differs from the regenerated WAD')
        print(f'{manifest.relative_to(ROOT)}: wad fields match ({len(names)} WADs)', flush=True)
        status = 0
        for n in names:
            status |= subprocess.run([sys.executable, str(CHECKER), fields[n]['file']], cwd=ROOT).returncode
        return status
    if a.manifest:
        man = json.loads(manifest.read_text()) if manifest.exists() else dict(schema=MANIFEST_SCHEMA, wads=[])
        by = {w['name']: w for w in man['wads']}
        for n in names:
            w = by.setdefault(n.upper(), {})
            scripts = w.pop('scripts', [])
            w.clear(); w.update(fields[n]); w['scripts'] = scripts
        order = [n.upper() for n in ALL_MAPS]
        man['wads'] = sorted(by.values(), key=lambda w: order.index(w['name']))
        manifest.write_text(json.dumps(man, indent=1) + '\n')
        print(f'updated {manifest.relative_to(ROOT)}')
    return 0


if __name__ == '__main__':
    main()
