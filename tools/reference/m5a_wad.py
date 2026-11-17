#!/usr/bin/env python3
"""Synthetic renderer rooms for M5a (decision R13; docs/M5A_REFERENCE_INVESTIGATION.md section 3.6).

Writes one small, fully synthetic IWAD per renderer feature under tests/reference/m5a_wads/,
each holding one map in the E1M1 slot, and the demo scripts under tests/reference/m5a_scripts/
(text and .lmp, the tools/reference/m4b_scripts.py format, imported read-only). Every room is run
by the genuine renderer with tests/reference/m5a_gfx.wad loaded after it
(`--wad ROOM.wad --wad m5a_gfx.wad --wad SCRIPT.lmp`). The room WAD itself carries the M5a
texture and flat directory (m5a_gfx.TEXTURES order and sizes, blank patches and flats), so it
also loads alone; the graphics PWAD supplies the pixels, the colour maps and the sprites.

    peg       upper and lower textures with and without ML_DONTPEGTOP / ML_DONTPEGBOTTOM, side x and y
              offsets, a DR door (special 1) whose upper texture moves, a door track
    mask      masked midtextures: a grate filling its opening, a short fence with a row offset pegged
              to the floor, a grate taller than its opening; walked past at grazing angles
    planes    a 5x5 field of small sectors with different heights, flats and lights (visplane
              creation, splitting and merging) and runs of equal planes
    sky       two adjacent F_SKY1 ceilings of different heights (no upper wall between them), a
              plain ceiling inside, a sky floor; a full turn
    light     sixteen alcoves at light 0..240 in steps of 16 off a hall (sector light, fake contrast
              on the axis-aligned walls)
    contrast  an octagon: walls of eight orientations (fake contrast only on the axis-aligned ones)
    dist      a 3,584-unit corridor (distance light and the MAXLIGHTZ clamp), walked slowly
    sprites   eight sleeping monsters in a ring facing eight directions (every rotation, mirrored
              pairs and TROO's separate lumps), items, fullbright frames and corpses; REJECT keeps
              the monsters from ever seeing the player
    clip      things behind a masked grate, on a floor edge, in a pit behind a ledge (drawseg
              silhouettes), and two things on the same spot (equal-scale sort order); strafed
    weapon    the shotgun, chaingun, rocket launcher and chainsaw picked up, fired (flash frames,
              extralight), switched and carried while walking (bob)
    fuzz      two Spectres (one at the bottom view rows, one on a ledge at the top rows) before
              striped walls; a blur sphere makes the player's weapon fuzz
    amp       a dark room and a light amplification visor (fixedcolormap 1)
    hom       a two-sided line whose upper texture is missing (the previous frame shows through)
    short     the tracker's room: a short unpadded single-patch texture, a short composite and a
              composite with unwritten rows (deliberate C-undefined reads, decision R4)
    view      a plain room with things and the pistol for the screenblocks 9, 10 and 11 comparison
    vplanes   seventy deep alcoves, each with its own light level and visible floor and ceiling, swept into view: more than
              MAXVISPLANES (128) visplanes (R_FindPlane I_Error, r_plane.c:245-246)
    dsegs     a far wall of 4-unit linedefs swept into view: more than MAXDRAWSEGS (256) wall ranges
              (silent drop, r_segs.c:386-387)
    vsprites  144 health bonuses swept into view: more than MAXVISSPRITES (128) (silent drop,
              r_things.c:330-331)
    openings  twenty-four full-width masked fences swept into view: more than MAXOPENINGS (20,480)
              openings (written past the array in R_StoreWallRange; R_DrawPlanes I_Error, r_plane.c:384-386)
    solidsegs a hundred thin pillars in a row: an attempt at more than MAXSEGS (32) disjoint solid ranges
              (R_ClipSolidWallSegment writes past solidsegs, r_bsp.c:122-133)

Appended in Phase 2 (after the twenty rooms above, so their WADs, scripts and manifest entries stay
byte-identical):

    ampflick  the light amplification visor picked up and worn until it runs out: the end-of-power
              flicker of P_PlayerThink (fixedcolormap 1 while powers[pw_infrared] > 4*32 or has bit 8,
              p_user.c:371-380)
    viewz     five step-ups (P_ZMovement's viewheight dip, p_mobj.c:251-257, and P_CalcHeight's
              deltaviewheight recovery, p_user.c:113-136), a 568-unit fall into a pit (the hard landing,
              p_mobj.c:298-308, and P_CalcHeight's VIEWHEIGHT/2 clamp, p_user.c:121-126) and the player
              killed there by a shotgun sergeant (P_DeathThink's viewheight descent to 6 units and the turn toward the attacker, p_user.c:182-229)
    switch    an S1 switch (103), an SR switch (70) pressed once and left to expire, an SR switch (70)
              pressed again while its button runs (the re-press parity, decision R7), and a GR switch (46)
              shot twice while its button runs (p_switch.c:201-262, 455-459, 592-595;
              p_spec.c:988-991)

Appended after the independent review of the mutation survivors (after the twenty-three rooms above, so their WADs,
scripts and manifest entries stay byte-identical):

    landing   a step-up, then one tic over a 48-unit gap back to the step's height: the player lands with
              z == floorz and momz != 0, because P_TryMove raises floorz (p_map.c:492) before P_ZMovement runs in
              the same P_MobjThinker (p_mobj.c:416-431), so the smooth-step test `z < floorz` (p_mobj.c:252) is
              false while the step-up recovery is still under way; then walk-offs of 8, 24 and 32 units, which land
              at momz -4, -7 and exactly -8 units per tic, none of them below -GRAVITY*8 (p_mobj.c:299-307)
    segangle  a wall whose SEGS angle (0) disagrees with its vertices (it runs west): with its v1 due south of the
              viewer, rw_normalangle - rw_angle1 == ANG180 exactly (p_setup.c:181, r_bsp.c:267-277), so rw_offset
              keeps its sign (r_segs.c:632-633)
    maskpad   the unpadded M5W72 as a masked midtexture 16 units in front of the viewer, seen head-on at the exact
              scales 10.0 and 9.0: the top row of each post reads texel 127 (`&127` of a slightly negative frac,
              r_draw.c:105-148), past the end of the patch lump in its last column (decision R4)

The grid painter, the builder chain and the lump framing are the frozen generators' (m4b_wad.Plan,
m4a_wad, m3h9b_geometry.room; imported read-only); contrast and dsegs give the builder explicit
rings (non-axis walls; a subdivided wall). Line styles (textures, offsets, ML_DONTPEG* flags) and
REJECT are applied after the build. Only specials and things the wire-24 simulation admits are used
(docs/COMPATIBILITY.md): specials 1, 46, 70 and 103 and the Episode 1 things (no invulnerability, so
INVERSECOLORMAP is not reachable here). tests/reference/m5a_manifest.json (schema
DOOM-M5A-MANIFEST-v1) names each WAD with its SHA-256 and counts (written here with --manifest,
checked by --check) and each script with its view configurations and genuine-renderer expectations
(authored, checked by tools/reference/m5a_reference.py).

    m5a_wad.py [--only NAME]       write the WADs and the scripts' .lmp files
    m5a_wad.py --list              name, file, sectors, lines, things, script tics
    m5a_wad.py --manifest          also refresh the manifest's wad and script fields
    m5a_wad.py --check             regenerate in memory and compare with the committed WADs, .lmp
                                   files and the manifest's generated fields
"""
import argparse, hashlib, json, math, struct, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m4b_wad as B              # noqa: E402  frozen M4b painter (Plan with voids), read-only
import m4a_wad as A              # noqa: E402  frozen M4a builder helpers and lump order
import m3h9b_geometry as G       # noqa: E402  the M3h6..M3h9b builder chain
import m4b_scripts as S          # noqa: E402  frozen script format (read-only)
import m5a_gfx as X              # noqa: E402  the M5a texture and flat directory

OUT_DIR = ROOT / 'tests/reference/m5a_wads'
SCRIPT_DIR = ROOT / 'tests/reference/m5a_scripts'
MANIFEST = ROOT / 'tests/reference/m5a_manifest.json'
MANIFEST_SCHEMA = 'DOOM-M5A-MANIFEST-v1'
ML_BLOCKING, ML_TWOSIDED, ML_DONTPEGTOP, ML_DONTPEGBOTTOM = 1, 4, 8, 16    # doomdata.h:115-131
EAST, NORTH, WEST, SOUTH = 0, 90, 180, 270
PLAYER1, ZOMBIE, SHOTGUY, CHAINGUY, IMP, DEMON, SPECTRE, BARON = 1, 3004, 9, 65, 3001, 3002, 58, 3003
SHOTGUN, CHAINGUN, LAUNCHER, CHAINSAW = 2001, 2002, 2003, 2005
CLIP, SHELLS, ROCKET, HEALTHBONUS, ARMORBONUS, SOULSPHERE, GREENARMOR = 2007, 2008, 2010, 2014, 2015, 2013, 2018
INVIS, LIGHTAMP, STIMPACK, MEDIKIT = 2024, 2045, 2011, 2012
LAMP, PILLAR, CANDELABRA, GIBS, DEADPLAYER, CANDLE, FIRESTICK = 2028, 48, 35, 24, 15, 34, 46
ALL = 7
DR = 1                            # p_spec.c / p_doors.c EV_VerticalDoor, wire 22
S1_OPEN, SR_TURBO, GR_OPEN = 103, 70, 46   # P_UseSpecialLine (p_switch.c), P_ShootSpecialLine (p_spec.c)
ROOM_SPECIALS = {DR, S1_OPEN, SR_TURBO, GR_OPEN}
VIEWS = [11, 10, 9]               # decision R2: 11 and 10 everywhere, 9 on synthetic rooms


class Room(B.Plan):
    """m4b_wad.Plan plus line styles, REJECT pairs, a default wall texture and room specials."""

    def __init__(self, name, wall='M5WALLA', upper='M5WALLB', lower='M5WALLC'):
        super().__init__(name)
        self.wall, self.upper, self.lower = wall, upper, lower
        self.styles, self.reject, self.rings_override = [], [], None
        self.seg_angles = []

    def sector(self, label, rects, fpic='M5FLAT1', cpic='M5FLAT2', **kw):
        """Plan.sector; the builder chain accepts only its own flat names, so it gets placeholders and the
        SECTORS lump is rewritten with the room's flats after the build (apply_styles)."""
        assert fpic in X.FLATS and cpic in X.FLATS, (fpic, cpic)
        k = super().sector(label, rects, fpic='TESTFLR0', cpic='TESTCEIL', **kw)
        self.sectors[k]['flats'] = (fpic, cpic)
        return k

    def special(self, front, back, at, special, tag=0):
        assert special in ROOM_SPECIALS, special
        self.specials.append((front, back, at, special, tag))

    def style(self, front, back, at=None, flags=0, s0=None, s1=None, axis=None):
        """Style the lines between sector `front` and `back` (None: one-sided walls of front) that
        contain point `at` (or all of them; axis 'x' or 'y' keeps vertical or horizontal lines)."""
        self.styles.append(dict(front=front, back=back, at=at, flags=flags, s0=s0 or {}, s1=s1 or {}, axis=axis))

    def hide(self, a, b):
        self.reject.append((a, b))

    def seg_angle(self, v1, v2, angle):
        """Store `angle` (the SEGS field, BAM >> 16) in the one seg from vertex v1 to v2 after fix_segs, so that the
        seg's stored angle disagrees with its vertices (the renderer reads it, p_setup.c:181; nothing checks it)."""
        self.seg_angles.append((v1, v2, angle))


def build(room):
    """The built map of a room (painter rings, or explicit rings for the non-axis rooms)."""
    if room.rings_override is not None:
        sectors = [(rings, s['floor'], s['ceil'], dict(s['attrs'])) for rings, s in zip(room.rings_override, room.sectors)]
        m = G.room(sectors, portals={}, reject=(), tree=None, segment=4096, lines={})
    else:
        m = B.build_map(room)
    return m


def on_segment(p, a, b):
    return A.on_segment(p, a, b)


def seg_angle(dx, dy):
    """The SEGS angle field (BAM >> 16) of a direction: exact for the axis and diagonal directions, the
    only ones the rooms use."""
    if dy == 0: return 0 if dx > 0 else -32768
    if dx == 0: return 16384 if dy > 0 else -16384
    assert abs(dx) == abs(dy), (dx, dy)
    return {(1, 1): 8192, (-1, 1): 24576, (-1, -1): -24576, (1, -1): -8192}[(dx > 0) - (dx < 0), (dy > 0) - (dy < 0)]


def fix_segs(m):
    """The builder chain writes every seg with angle 0 and offset 0 (m3h6_geometry.room), which the gameplay code
    never reads but the renderer does (P_LoadSegs, p_setup.c:181-182; rw_normalangle r_segs.c:401, rw_offset
    r_segs.c:635). Each seg gets its linedef's direction (reversed for side 1) and its distance from the start
    of that side (line v1 for side 0, v2 for side 1), rounded half up."""
    seg = bytearray(m['lumps']['SEGS'])
    vb = m['lumps']['VERTEXES']
    V = [struct.unpack_from('<hh', vb, 4 * i) for i in range(len(vb) // 4)]
    lb = m['lumps']['LINEDEFS']
    for k in range(len(seg) // 12):
        v1, v2, _, li, side, _ = struct.unpack_from('<6h', seg, 12 * k)
        a, b = struct.unpack_from('<hh', lb, 14 * li)
        (x0, y0), (x1, y1) = (V[a], V[b]) if side == 0 else (V[b], V[a])
        off = math.hypot(V[v1][0] - x0, V[v1][1] - y0)
        struct.pack_into('<h', seg, 12 * k + 4, seg_angle(x1 - x0, y1 - y0))
        struct.pack_into('<h', seg, 12 * k + 10, int(off + 0.5))
    m['lumps']['SEGS'] = bytes(seg)


def apply_styles(room, m):
    """Textures, offsets and flags per line: the defaults (one-sided: the room's wall; two-sided:
    the upper texture where the ceilings differ, the lower where the floors differ, no middle),
    then the room's style rules in order."""
    side = bytearray(m['lumps']['SIDEDEFS'])
    line = bytearray(m['lumps']['LINEDEFS'])
    texnames = {t for t, _, _, _ in X.textures()}
    V = m['vertices']
    for i, l in enumerate(m['lines']):
        two = l['back'] is not None
        a, b = V[l['a']], V[l['b']]
        f, k = room.sectors[l['front']], room.sectors[l['back']] if two else None
        sides = []
        for which in (0, 1):
            if which == 1 and not two: sides.append(None); continue
            st = dict(top='-', bottom='-', mid='-', x=0, y=0)
            if not two: st['mid'] = room.wall
            else:
                if f['ceil'] != k['ceil']: st['top'] = room.upper
                if f['floor'] != k['floor']: st['bottom'] = room.lower
            sides.append(st)
        flags = l['flags']
        for r in room.styles:
            if l['front'] != r['front'] or l['back'] != r['back']: continue
            if r['at'] is not None and not on_segment(r['at'], a, b): continue
            if r['axis'] == 'x' and a[0] != b[0]: continue
            if r['axis'] == 'y' and a[1] != b[1]: continue
            flags |= r['flags']
            sides[0].update(r['s0'])
            if two: sides[1].update(r['s1'])
        struct.pack_into('<h', line, 14 * i + 4, flags)
        for sd, st in zip(l['sides'], sides):
            if sd < 0: continue
            for key in ('top', 'bottom', 'mid'): assert st[key] == '-' or st[key] in texnames, st[key]
            struct.pack_into('<hh', side, 30 * sd, st['x'], st['y'])
            side[30 * sd + 4:30 * sd + 28] = X.name8(st['top']) + X.name8(st['bottom']) + X.name8(st['mid'])
    m['lumps']['SIDEDEFS'], m['lumps']['LINEDEFS'] = bytes(side), bytes(line)
    sec = bytearray(m['lumps']['SECTORS'])
    assert len(sec) == 26 * len(room.sectors)
    for k, s in enumerate(room.sectors):
        sec[26 * k + 4:26 * k + 20] = X.name8(s['flats'][0]) + X.name8(s['flats'][1])
    m['lumps']['SECTORS'] = bytes(sec)
    fix_segs(m)
    if room.seg_angles:
        seg, vb = bytearray(m['lumps']['SEGS']), m['lumps']['VERTEXES']
        V = [struct.unpack_from('<hh', vb, 4 * i) for i in range(len(vb) // 4)]
        for v1, v2, angle in room.seg_angles:
            at = [k for k in range(len(seg) // 12) if (V[struct.unpack_from('<h', seg, 12 * k)[0]], V[struct.unpack_from('<h', seg, 12 * k + 2)[0]]) == (v1, v2)]
            assert len(at) == 1, (v1, v2, at)
            struct.pack_into('<h', seg, 12 * at[0] + 4, angle)
        m['lumps']['SEGS'] = bytes(seg)
    n = len(room.sectors)
    rej = bytearray((n * n + 7) // 8)
    for x, y in room.reject:
        for pair in (x * n + y, y * n + x): rej[pair >> 3] |= 1 << (pair & 7)
    m['lumps']['REJECT'] = bytes(rej)


# ---------------------------------------------------------------- the rooms ----------------------
def peg():
    p = Room('peg')
    hall = p.sector('HALL', [(-256, -256, 256, 256)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    w1 = p.sector('WIN1', [(-192, 256, -64, 384)], floor=32, ceil=96, light=192, fpic='M5FLAT3', cpic='M5FLAT4')
    w2 = p.sector('WIN2', [(64, 256, 192, 384)], floor=48, ceil=80, light=192, fpic='M5FLAT3', cpic='M5FLAT4')
    door = p.sector('DOOR', [(-32, -320, 32, -256)], floor=0, ceil=0, light=192, door=True)
    p.sector('BEYOND', [(-128, -512, 128, -320)], light=160, fpic='M5FLAT5', cpic='M5FLAT6')
    p.special(hall, door, (0, -256), DR)
    p.style(hall, w1, flags=ML_DONTPEGTOP | ML_DONTPEGBOTTOM, s0=dict(top='M5WALLB', bottom='M5WALLC'))
    p.style(hall, w2, s0=dict(top='M5WALLE', bottom='M5WALLF', x=13, y=7))
    p.style(hall, None, at=(-256, 0), s0=dict(mid='M5WALLC', x=0, y=0))
    p.style(hall, None, at=(256, 0), s0=dict(mid='M5WALLC', x=32, y=16))
    p.style(hall, None, at=(-224, 256), s0=dict(mid='M5WIDE', x=5, y=-20))
    p.style(hall, None, at=(224, -256), s0=dict(mid='M5TALL'))
    p.style(hall, door, s0=dict(top='BIGDOOR2'))
    p.style(door, 4, s0=dict(top='BIGDOOR2'))
    p.style(door, None, flags=ML_DONTPEGBOTTOM, s0=dict(mid='DOORTRAK'))
    p.thing(0, -216, SOUTH, PLAYER1)
    return p


def mask():
    p = Room('mask')
    s = [p.sector(f'S{k}', [(-512 + 256 * k, -256, -256 + 256 * k, 256)], ceil=128 if k < 3 else 96, light=176,
                  fpic='M5FLAT7', cpic='M5FLAT8') for k in range(4)]
    p.style(s[0], s[1], s0=dict(mid='M5GRATE'), s1=dict(mid='M5GRATE'))
    p.style(s[1], s[2], flags=ML_DONTPEGBOTTOM, s0=dict(mid='M5FENCE', y=24), s1=dict(mid='M5FENCE', y=24))
    p.style(s[2], s[3], s0=dict(mid='M5GRATE', x=16), s1=dict(mid='M5GRATE'))
    p.thing(-480, -224, 30, PLAYER1)
    return p


def planes():
    p = Room('planes')
    main = p.sector('MAIN', [(-384, -384, 384, 384)], light=160, fpic='M5FLAT1', cpic='M5FLAT2')
    for i in range(5):
        for j in range(5):
            x0, y0 = -160 + 64 * i, -64 + 64 * j
            same = (i + j) % 4 == 0          # equal planes with MAIN: merged by R_FindPlane
            p.sector(f'C{i}{j}', [(x0, y0, x0 + 64, y0 + 64)], layer=1,
                     floor=0 if same else ((i * 7 + j * 13) % 5) * 8, ceil=128 if same else 128 + ((i + j) % 3) * 16,
                     light=160 if same else 112 + ((i * j) % 4) * 32,
                     fpic='M5FLAT1' if same else X.NEW_FLATS[(i + 2 * j) % 8], cpic='M5FLAT2' if same else X.NEW_FLATS[(3 * i + j) % 8])
    p.thing(0, -320, NORTH, PLAYER1)
    return p


def sky():
    p = Room('sky')
    o1 = p.sector('OUT1', [(-256, -256, 0, 256)], ceil=192, light=176, cpic='F_SKY1', fpic='M5FLAT3')
    o2 = p.sector('OUT2', [(0, -256, 256, 256)], ceil=128, light=176, cpic='F_SKY1', fpic='M5FLAT4')
    p.sector('INNER', [(64, -64, 160, 64)], ceil=96, light=176, layer=1, fpic='M5FLAT5', cpic='M5FLAT6')
    p.sector('SKYFLR', [(-224, 128, -96, 224)], floor=-32, ceil=192, light=176, layer=1, fpic='F_SKY1', cpic='F_SKY1')
    p.style(o1, o2, s0=dict(top='M5WALLB'), s1=dict(top='M5WALLB'))
    p.thing(-64, -128, NORTH, PLAYER1)
    return p


def light():
    p = Room('light')
    hall = p.sector('HALL', [(-512, -64, 512, 64)], light=144, fpic='M5FLAT2', cpic='M5FLAT3')
    for k in range(16):
        x0 = -512 + 64 * k
        y = (64, 128) if k % 2 == 0 else (-128, -64)
        p.sector(f'L{k * 16}', [(x0, y[0], x0 + 64, y[1])], light=k * 16, fpic='M5FLAT4', cpic='M5FLAT5')
    p.thing(-448, 0, EAST, PLAYER1)
    return p


def contrast():
    p = Room('contrast')
    p.sector('OCT', [(-256, -256, 256, 256)], light=128, fpic='M5FLAT6', cpic='M5FLAT7')
    r, c = 256, 106
    ring = [(-c, r), (c, r), (r, c), (r, -c), (c, -r), (-c, -r), (-r, -c), (-r, c)]   # clockwise, y up
    p.rings_override = [[ring]]
    p.thing(0, 0, EAST, PLAYER1)
    return p


def dist():
    p = Room('dist', wall='M5WALLE')
    p.sector('CORR', [(0, -64, 3584, 64)], light=160, fpic='M5FLAT1', cpic='M5FLAT8')
    p.thing(32, 0, EAST, PLAYER1)
    return p


def sprites():
    p = Room('sprites', wall='M5WALLF')
    outer = p.sector('OUTER', [(-384, -384, 384, 384)], light=176, fpic='M5FLAT3', cpic='M5FLAT4')
    center = p.sector('CENTER', [(-96, -96, 96, 96)], light=176, fpic='M5FLAT3', cpic='M5FLAT4', layer=1)
    p.hide(outer, center)
    p.thing(0, 0, NORTH, PLAYER1)
    kinds = [ZOMBIE, SHOTGUY, IMP, DEMON, BARON, SHOTGUY, IMP, ZOMBIE]   # no 65: P_LoadThings stops at it outside commercial (p_setup.c:317-337)
    for k, kind in enumerate(kinds):
        a = math.radians(45 * k)
        p.thing(int(round(256 * math.cos(a))), int(round(256 * math.sin(a))), (90 * k) % 360, kind, ALL | 8)
    for k, kind in enumerate([HEALTHBONUS, SOULSPHERE, ARMORBONUS, CLIP, DEADPLAYER, GIBS, LAMP, CANDELABRA]):
        a = math.radians(22 + 45 * k)
        p.thing(int(round(160 * math.cos(a))), int(round(160 * math.sin(a))), EAST, kind)
    return p


def clip():
    p = Room('clip', wall='M5WALLA')
    south = p.sector('SOUTH', [(-256, -256, 256, 0)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    pit = p.sector('PIT', [(-256, 0, 256, 256)], floor=-48, light=176, fpic='M5FLAT5', cpic='M5FLAT2')
    west = p.sector('WEST', [(-512, -256, -256, 0)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.sector('WPIT', [(-512, 0, -256, 256)], floor=-48, light=176, fpic='M5FLAT5', cpic='M5FLAT2')
    p.style(west, south, s0=dict(mid='M5GRATE'), s1=dict(mid='M5GRATE'))
    p.style(south, pit, s1=dict(bottom='M5WALLC'))
    p.thing(0, -192, NORTH, PLAYER1)
    p.thing(-320, -96, EAST, LAMP)                  # behind the grate
    p.thing(-384, -160, EAST, PILLAR)               # behind the grate
    p.thing(64, 0, EAST, CANDELABRA)                # on the floor edge
    p.thing(-64, 96, EAST, PILLAR)                  # in the pit behind the ledge
    p.thing(160, 64, EAST, LAMP)                    # in the pit
    p.thing(128, 160, EAST, CANDLE)                 # two things on one spot: equal scale
    p.thing(128, 160, EAST, FIRESTICK)
    return p


def weapon():
    p = Room('weapon', wall='M5WALLB')
    p.sector('HALL', [(-128, -128, 128, 2048)], light=160, fpic='M5FLAT2', cpic='M5FLAT6')
    p.thing(0, -96, NORTH, PLAYER1)
    for y, kind in ((0, SHOTGUN), (48, SHELLS), (96, CHAINGUN), (144, CLIP), (192, LAUNCHER), (240, ROCKET), (288, CHAINSAW)):
        p.thing(0, y, NORTH, kind)
    return p


def fuzz():
    p = Room('fuzz', wall='M5WALLC')
    pl = p.sector('PL', [(-128, -128, 128, 32)], light=176, fpic='M5FLAT4', cpic='M5FLAT5')
    sp = p.sector('SP', [(-256, 32, 256, 320)], light=176, fpic='M5FLAT6', cpic='M5FLAT5', ceil=192)
    led = p.sector('LEDGE', [(48, 112, 176, 240)], floor=64, ceil=192, light=176, layer=1, fpic='M5FLAT7', cpic='M5FLAT5')
    p.hide(pl, sp); p.hide(pl, led)
    p.style(sp, None, s0=dict(mid='M5COMP'))
    p.thing(0, 0, NORTH, PLAYER1)
    p.thing(0, 64, SOUTH, SPECTRE, ALL | 8)
    p.thing(112, 176, SOUTH, SPECTRE, ALL | 8)
    p.thing(0, -64, NORTH, INVIS)
    return p


def amp():
    p = Room('amp', wall='M5WALLE')
    p.sector('DARK', [(-256, -256, 256, 512)], light=32, fpic='M5FLAT3', cpic='M5FLAT4')
    p.thing(0, -192, NORTH, PLAYER1)
    p.thing(0, -96, NORTH, LIGHTAMP)
    p.thing(0, 256, SOUTH, LAMP)
    p.thing(-96, 320, SOUTH, PILLAR)
    return p


def hom():
    p = Room('hom', wall='M5WALLF')
    a = p.sector('A', [(-256, -256, 256, 256)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    b = p.sector('B', [(-64, 256, 64, 384)], ceil=72, light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.style(a, b, s0=dict(top='-'))                  # the missing upper texture
    p.thing(0, 0, NORTH, PLAYER1)
    return p


def short():
    p = Room('short')
    hall = p.sector('HALL', [(-256, -256, 256, 256)], ceil=128, light=192, fpic='M5FLAT2', cpic='M5FLAT3')
    p.style(hall, None, at=(0, 256), s0=dict(mid='M5W72'))
    p.style(hall, None, at=(256, 0), s0=dict(mid='M5C64'))
    p.style(hall, None, at=(0, -256), s0=dict(mid='M5GAP'))
    p.thing(0, 0, NORTH, PLAYER1)
    return p


def view():
    p = Room('view', wall='M5WALLA')
    p.sector('ROOM', [(-256, -256, 256, 256)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.sector('STEP', [(-64, 128, 64, 256)], floor=16, ceil=112, light=176, fpic='M5FLAT3', cpic='M5FLAT4')
    p.thing(0, -128, NORTH, PLAYER1)
    p.thing(-96, 64, EAST, LAMP)
    p.thing(96, 64, EAST, SOULSPHERE)
    return p


def vplanes():
    p = Room('vplanes', wall='M5WALLA')
    p.sector('MAIN', [(-640, -640, 640, 0)], light=200, fpic='M5FLAT1', cpic='M5FLAT2')
    for k in range(70):              # deep alcoves: floor below and ceiling above the eye, a distinct light each
        x0 = -560 + 16 * k
        p.sector(f'A{k}', [(x0, 0, x0 + 16, 512)], floor=1 + k % 40, ceil=127 - k % 40, light=96 + k, fpic='M5FLAT1', cpic='M5FLAT2')
    p.thing(0, -600, EAST, PLAYER1)
    return p


def dsegs():
    p = Room('dsegs', wall='M5WALLE')
    p.sector('ROOM', [(-1024, -1024, 1024, 512)], light=160, fpic='M5FLAT3', cpic='M5FLAT4')
    north = [(x, 512) for x in range(-1024, 1024, 4)]
    p.rings_override = [[north + [(1024, 512), (1024, -1024), (-1024, -1024)]]]
    p.thing(0, -88, EAST, PLAYER1)
    return p


def vsprites():
    p = Room('vsprites', wall='M5WALLB')
    p.sector('ROOM', [(-512, -256, 512, 1024)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.thing(0, -224, EAST, PLAYER1)
    for i in range(12):
        for j in range(12):
            p.thing(-330 + 60 * i, 140 + 60 * j, EAST, HEALTHBONUS)
    return p


def openings():
    p = Room('openings', wall='M5WALLF')
    strips = [p.sector(f'S{k}', [(-1600, -128 + 32 * k if k else -256, 1600, -96 + 32 * k)], light=176, fpic='M5FLAT2', cpic='M5FLAT3')
              for k in range(26)]
    for k in range(1, 25):
        p.style(strips[k - 1], strips[k], s0=dict(mid='M5FENCE'), s1=dict(mid='M5FENCE'))
    p.thing(0, -192, EAST, PLAYER1)
    return p


def solidsegs():
    p = Room('solidsegs', wall='M5WALLA')
    room = p.sector('ROOM', [(-1024, -512, 1024, 1024)], light=176, fpic='M5FLAT5', cpic='M5FLAT6')
    for k in range(100):
        x0 = -800 + 16 * k
        p.void(room, x0, 192, x0 + 8, 200)
    p.thing(0, -448, EAST, PLAYER1)
    return p


def ampflick():
    p = Room('ampflick', wall='M5WALLE')
    p.sector('DARK', [(-128, -128, 128, 256)], light=48, fpic='M5FLAT3', cpic='M5FLAT4')
    p.thing(0, -96, NORTH, PLAYER1)
    p.thing(0, -32, NORTH, LIGHTAMP)
    p.thing(0, 192, SOUTH, LAMP)
    p.thing(-64, 160, SOUTH, PILLAR)
    return p


def viewz():
    p = Room('viewz', wall='M5WALLA')
    hall = p.sector('HALL', [(-256, -64, 0, 64)], ceil=256, light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    steps = [p.sector(f'STEP{k}', [(32 * k, -64, 32 * k + 32, 64)], floor=24 * (k + 1), ceil=256, light=176,
                      fpic='M5FLAT3', cpic='M5FLAT2') for k in range(4)]
    top = p.sector('TOP', [(128, -64, 192, 64)], floor=120, ceil=256, light=176, fpic='M5FLAT4', cpic='M5FLAT2')
    pit = p.sector('PIT', [(192, -128, 704, 128)], floor=-448, ceil=256, light=176, fpic='M5FLAT5', cpic='M5FLAT6')
    for k in [hall, top] + steps: p.hide(pit, k)     # the sergeant sees the player only once he is over the pit
    p.thing(-224, 0, EAST, PLAYER1)
    p.thing(656, 0, WEST, SHOTGUY)                    # one shooter: no crossfire between monsters
    return p


def switch():
    p = Room('switch', wall='M5WALLB')
    hall = p.sector('HALL', [(-32, -32, 32, 32)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    north = p.sector('NORTH', [(-16, 32, 16, 48)], floor=24, light=192, tag=1, fpic='M5FLAT3', cpic='M5FLAT2')
    east = p.sector('EAST', [(32, -16, 48, 16)], floor=24, light=192, tag=2, fpic='M5FLAT3', cpic='M5FLAT2')
    west = p.sector('WEST', [(-48, -16, -32, 16)], light=192, fpic='M5FLAT3', cpic='M5FLAT2')
    corr = p.sector('CORR', [(-16, -160, 16, -32)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    d1 = p.sector('DOOR1', [(16, -112, 32, -80)], ceil=0, light=192, tag=3, door=True)
    p.sector('ROOM1', [(32, -128, 96, -64)], light=160, fpic='M5FLAT5', cpic='M5FLAT6')
    d2 = p.sector('DOOR2', [(-32, -112, -16, -80)], ceil=0, light=192, tag=4, door=True)
    p.sector('ROOM2', [(-96, -128, -32, -64)], light=160, fpic='M5FLAT5', cpic='M5FLAT6')
    p.special(north, None, (0, 48), SR_TURBO, 1)      # pressed, then pressed again while its button runs
    p.special(east, None, (48, 0), SR_TURBO, 2)       # pressed once, its button left to expire
    p.special(west, None, (-48, 0), S1_OPEN, 3)
    p.special(corr, None, (0, -160), GR_OPEN, 4)      # shot twice while its button runs
    p.style(north, None, at=(0, 48), s0=dict(mid='SW1COMM'))
    p.style(east, None, at=(48, 0), s0=dict(mid='SW1BRN1'))
    p.style(west, None, at=(-48, 0), s0=dict(mid='SW1STARG'))
    p.style(corr, None, at=(0, -160), s0=dict(mid='SW1BRCOM'))
    for d in (d1, d2): p.style(d, None, flags=ML_DONTPEGBOTTOM, s0=dict(mid='DOORTRAK'))
    p.thing(0, 0, NORTH, PLAYER1)
    return p


def landing():
    p = Room('landing', wall='M5WALLA')
    p.sector('HALL', [(-512, -64, 0, 64)], ceil=128, light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.sector('STEP', [(0, -64, 32, 64)], floor=24, ceil=128, light=176, fpic='M5FLAT3', cpic='M5FLAT2')
    p.sector('GAP', [(32, -64, 80, 64)], floor=-64, ceil=128, light=176, fpic='M5FLAT5', cpic='M5FLAT2')
    p.sector('LEDGE', [(80, -64, 336, 64)], floor=24, ceil=128, light=176, fpic='M5FLAT3', cpic='M5FLAT2')
    p.sector('DROP8', [(336, -64, 592, 64)], floor=16, ceil=128, light=176, fpic='M5FLAT4', cpic='M5FLAT2')
    p.sector('DROP24', [(592, -64, 848, 64)], floor=-8, ceil=128, light=176, fpic='M5FLAT6', cpic='M5FLAT2')
    p.sector('DROP32', [(848, -64, 1360, 64)], floor=-40, ceil=128, light=176, fpic='M5FLAT7', cpic='M5FLAT2')
    p.thing(-448, 0, EAST, PLAYER1)
    return p


def segangle():
    p = Room('segangle', wall='M5WALLA')
    p.sector('WEST', [(-128, -48, 0, 128)], ceil=128, light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.sector('EAST', [(0, -48, 128, 128)], ceil=128, light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.style(0, None, at=(-64, -48), s0=dict(mid='M5WALLA', x=13))
    p.seg_angle((0, -48), (-128, -48), 0)            # runs west (fix_segs: -32768); stored as east
    p.thing(0, 4, SOUTH, PLAYER1)                     # v1 (0, -48) due south, 52 units away
    return p


def maskpad():
    p = Room('maskpad', wall='M5WALLE')
    north = p.sector('NORTH', [(-128, 0, 128, 128)], ceil=128, light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    sw = p.sector('SW', [(-128, -128, 0, 0)], ceil=128, light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.sector('SE', [(0, -128, 128, 0)], ceil=128, light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    fence = dict(mid='M5W72', x=56, y=-32)            # pegged to the floor: its top 1 unit below the eye
    p.style(north, sw, flags=ML_DONTPEGBOTTOM, s0=fence)
    p.style(sw, north, flags=ML_DONTPEGBOTTOM, s1=fence)
    p.thing(0, 16, SOUTH, PLAYER1)                    # the fence's v1 (0, 0) due south, 16 units away
    return p


ROOMS = {'peg': peg, 'mask': mask, 'planes': planes, 'sky': sky, 'light': light, 'contrast': contrast, 'dist': dist,
         'sprites': sprites, 'clip': clip, 'weapon': weapon, 'fuzz': fuzz, 'amp': amp, 'hom': hom, 'short': short,
         'view': view, 'vplanes': vplanes, 'dsegs': dsegs, 'vsprites': vsprites, 'openings': openings,
         'solidsegs': solidsegs, 'ampflick': ampflick, 'viewz': viewz, 'switch': switch, 'landing': landing,
         'segangle': segangle, 'maskpad': maskpad}

# Scripts (tools/reference/m4b_scripts.py format; turn is the angleturn high byte, 256 per full turn).
SCRIPTS = {
    'peg': ('M5PEG', 'turn a full circle, then open the door and watch its upper texture rise',
            ['cmd 128 0 0 2 0', 'cmd 1 0 0 0 USE', 'idle 90']),
    'mask': ('M5MASK', 'walk past the masked lines at grazing angles, then turn back across them',
             ['cmd 40 12 0 0 0', 'cmd 32 10 0 1 0', 'cmd 48 0 0 -2 0', 'cmd 40 8 6 0 0']),
    'planes': ('M5PLANE', 'walk into the field, turn a full circle',
               ['cmd 20 10 0 0 0', 'cmd 128 0 0 2 0', 'cmd 20 -10 0 0 0']),
    'sky': ('M5SKY', 'turn a full circle slowly, then look across the sky floor', ['cmd 256 0 0 1 0', 'cmd 32 0 0 4 0']),
    'light': ('M5LIGHT', 'walk the hall past the alcoves, turning left and right',
              ['cmd 32 0 0 4 0', 'cmd 40 16 0 0 0', 'cmd 64 0 0 -4 0', 'cmd 40 16 0 0 0', 'cmd 32 0 0 4 0']),
    'contrast': ('M5CONTR', 'turn a full circle in the octagon', ['cmd 128 0 0 2 0']),
    'dist': ('M5DIST', 'walk down the corridor slowly', ['idle 10', 'cmd 160 12 0 0 0', 'cmd 32 0 0 8 0']),
    'sprites': ('M5SPRIT', 'turn a full circle among the sleeping monsters and items', ['idle 8', 'cmd 128 0 0 2 0']),
    'clip': ('M5CLIP', 'strafe left and right before the grate, the ledge and the pit',
             ['cmd 40 0 -20 0 0', 'cmd 80 0 20 0 0', 'cmd 24 0 0 4 0', 'cmd 40 0 -20 0 0']),
    'weapon': ('M5WEAP', 'walk over the weapons, fire each, switch, walk back',
               ['cmd 72 20 0 0 0', 'idle 20', 'cmd 24 0 0 0 ATTACK', 'cmd 1 0 0 0 CHANGE|W2', 'idle 30', 'cmd 20 0 0 0 ATTACK',
                'cmd 1 0 0 0 CHANGE|W3', 'idle 30', 'cmd 24 0 0 0 ATTACK', 'cmd 1 0 0 0 CHANGE|W4', 'idle 30',
                'cmd 12 0 0 0 ATTACK', 'idle 20', 'cmd 1 0 0 0 CHANGE|W7', 'idle 30', 'cmd 20 0 0 0 ATTACK', 'cmd 60 -20 0 0 0']),
    'fuzz': ('M5FUZZ', 'look at the Spectres, back onto the blur sphere, look again with the fuzzy weapon',
             ['idle 40', 'cmd 8 0 0 2 0', 'cmd 8 0 0 -2 0', 'cmd 24 -12 0 0 0', 'idle 60', 'cmd 16 0 0 2 0']),
    'amp': ('M5AMP', 'walk onto the light amplification visor and look around', ['idle 10', 'cmd 30 15 0 0 0', 'idle 20', 'cmd 64 0 0 4 0']),
    'hom': ('M5HOM', 'turn so the missing upper texture sweeps across the view', ['idle 4', 'cmd 32 0 0 2 0', 'cmd 64 0 0 -2 0', 'cmd 32 0 0 2 0']),
    'short': ('M5SHORT', 'look at the three short and gappy walls', ['idle 8', 'cmd 64 0 0 -4 0', 'cmd 64 0 0 -4 0']),
    'view': ('M5VIEW', 'stand still, then turn a little', ['idle 16', 'cmd 16 0 0 2 0', 'idle 8']),
    'vplanes': ('M5VPLAN', 'turn from east to north toward the alcoves', ['idle 2', 'cmd 80 0 0 1 0']),
    'dsegs': ('M5DSEG', 'turn from east to north toward the subdivided wall', ['idle 2', 'cmd 80 0 0 1 0']),
    'vsprites': ('M5VSPR', 'turn from east to north toward the health bonuses', ['idle 2', 'cmd 80 0 0 1 0']),
    'openings': ('M5OPEN', 'turn from east to north toward the fences', ['idle 2', 'cmd 80 0 0 1 0']),
    'solidsegs': ('M5SSEG', 'turn from east to north toward the pillars', ['idle 2', 'cmd 80 0 0 1 0']),
    'ampflick': ('M5AMPFL', 'walk onto the light amplification visor and wait until it runs out',
                 ['idle 2', 'cmd 8 20 0 0 0', 'idle 4220']),
    'viewz': ('M5VIEWZ', 'climb five steps, walk off into the pit and die there', ['idle 4', 'cmd 64 25 0 0 0', 'idle 500']),
    'switch': ('M5SWTCH', 'press an SR switch twice within its button time, an SR switch once, an S1 switch, then shoot a GR '
               'switch twice', ['idle 2', 'cmd 1 0 0 0 USE', 'idle 9', 'cmd 1 0 0 0 USE', 'idle 45', 'cmd 1 0 0 -64 0',
                                'cmd 1 0 0 0 USE', 'idle 45', 'cmd 2 0 0 64 0', 'cmd 1 0 0 0 USE', 'idle 10', 'cmd 1 0 0 64 0',
                                'cmd 1 0 0 0 ATTACK', 'idle 22', 'cmd 1 0 0 0 ATTACK', 'idle 70']),
    'landing': ('M5LAND', 'run east up a step, over a one-tic gap, then off ledges of 8, 24 and 32 units',
                ['idle 4', 'cmd 100 50 0 0 0', 'idle 40']),
    'segangle': ('M5SEGANG', 'stand facing the wall whose stored angle disagrees with its vertices, turn a little',
                 ['idle 8', 'cmd 24 0 0 1 0', 'idle 8']),
    'maskpad': ('M5MSKPAD', 'stand facing the unpadded masked midtexture head-on', ['idle 16']),
}


# Authored expectations, evaluated on the strict frame stream and view record of each configuration by
# tools/reference/m5a_reference.py (expression syntax there); the tics are observed (--update-events), reviewed,
# then held fixed in the manifest. PNG lists the (screenblocks, tic) frames committed as images.
MUST_SHOW = {
    'peg': [('the door opens: the lines beyond it are mapped', 'frame mapped >= 18')],
    'mask': [('masked midtextures use openings', 'frame openings > 0')],
    'planes': [('many visplanes', 'frame visplanes >= 40')],
    'sky': [('sky planes', 'frame visplanes >= 4')],
    'light': [('alcove planes', 'frame visplanes >= 20')],
    'contrast': [('three walls in view', 'frame drawsegs >= 3')],
    'dist': [('the player walks', 'view viewx > 2097152')],
    'sprites': [('five things in view', 'frame vissprites >= 5')],
    'clip': [('sprites clipped by silhouettes', 'frame vissprites >= 5'), ('masked grate openings', 'frame openings > 0')],
    'weapon': [('a muzzle flash (extralight)', 'view extralight > 0'), ('a second psprite (flash)', 'view ps1state >= 0')],
    'fuzz': [('a Spectre in view', 'frame vissprites >= 1'), ('the player is invisible', 'view invisibility > 0')],
    'amp': [('light amplification', 'view fixedcolormap == 1')],
    'hom': [('the missing upper wall in view', 'frame drawsegs >= 3')],
    'short': [('tracker-flagged columns', 'frame columns > 0')],
    'view': [('the step in view', 'frame visplanes >= 4')],
    'vplanes': [('visplanes near the bound', 'frame visplanes >= 120'), ('R_FindPlane I_Error', 'error no more visplanes')],
    'dsegs': [('drawsegs at the bound', 'frame drawsegs == 256'), ('a drawseg dropped', 'frame drawseg_drops > 0')],
    'vsprites': [('vissprites at the bound', 'frame vissprites == 128'), ('a vissprite dropped', 'frame vissprite_drop > 0')],
    'openings': [('openings near the bound', 'frame openings >= 19000'), ('written past openings', 'witness OpeningOverflow')],
    'solidsegs': [('solid ranges near the bound', 'frame solidsegs_peak >= 30'), ('written past solidsegs', 'witness SolidSegOverflow')],
    'ampflick': [('light amplification', 'view fixedcolormap == 1'),
                 ('the end-of-power flicker (colour map 0 while the power still runs)', 'view fixedcolormap == 0 from 100')],
    'viewz': [('a step-up dip recovering', 'view deltaviewheight > 0'), ('in the air', 'view onground == 0'),
              ('the hard landing', 'view deltaviewheight < 0'), ('the VIEWHEIGHT/2 clamp (first on a 24-unit step-up)', 'view viewheight == 1343488'),
              ('the hard landing reaches the VIEWHEIGHT/2 clamp', 'view viewheight == 1343488 from 98'),
              ('the death descent at 6 units', 'view viewheight == 393216')],
    'switch': [('the pistol fires at the GR switch (extralight)', 'view extralight > 0'), ('the S1 door opens: the lines beyond it mapped', 'frame mapped >= 25')],
    'landing': [('the step-up dip recovering', 'view deltaviewheight > 0'), ('one tic in the air over the gap', 'view onground == 0'),
                ('the gap landing at z == floorz kept deltaviewheight (4.0, then 4.25)', 'view deltaviewheight == 278528'),
                ('in the air after the 8-unit walk-off', 'view onground == 0 from 50'),
                ('in the air after the 24-unit walk-off', 'view onground == 0 from 70'),
                ('in the air after the 32-unit walk-off', 'view onground == 0 from 85'),
                ('no landing squats: momz -2, -4, -7 and -8 are not below -GRAVITY*8 (never)', 'view deltaviewheight < 0')],
    'segangle': [('the wall with the disagreeing SEGS angle in view', 'frame drawsegs >= 2')],
    'maskpad': [('the masked reads past the unpadded patch (tracker-flagged columns)', 'frame columns > 0')],
}
PNG = {'peg': [(10, 40), (10, 200)], 'mask': [(10, 60)], 'planes': [(10, 60)], 'sky': [(11, 100)], 'light': [(10, 60)],
       'contrast': [(10, 50)], 'dist': [(10, 120)], 'sprites': [(11, 0), (11, 100)], 'clip': [(10, 60)], 'weapon': [(10, 150), (10, 330)],
       'fuzz': [(10, 30), (10, 120)], 'amp': [(10, 60)], 'hom': [(10, 60)], 'short': [(10, 8)], 'view': [(9, 20), (10, 20), (11, 20)],
       'vplanes': [(10, 40)], 'dsegs': [(10, 60)], 'vsprites': [(10, 60)], 'openings': [(10, 30)], 'solidsegs': [(10, 12)],
       'ampflick': [(10, 60), (10, 4096)], 'viewz': [(10, 100), (10, 450)], 'switch': [(10, 13), (10, 130)],
       'landing': [(10, 44)], 'segangle': [(10, 4)], 'maskpad': [(10, 4)]}


def framing():
    """The resource lumps every room WAD carries: blank PLAYPAL and COLORMAP (m3h9c_wad), the M5a flat
    names with blank 4,096-byte flats, the M5a PNAMES and TEXTURE1 and blank patches of the same sizes."""
    W = A.W
    out = [('PLAYPAL', W.playpal()), ('COLORMAP', W.colormap()), ('S_START', b''), ('S_END', b''), ('F_START', b'')]
    out += [(f, bytes(4096)) for f in X.FLATS] + [('F_END', b''), ('PNAMES', X.pnames()), ('TEXTURE1', X.texture1())]
    for n, lump in X.PATCHES:
        w, h = struct.unpack_from('<hh', lump, 0)
        out.append((n, W.blank_patch(w, h)))
    return out


def wad_bytes(name):
    room = ROOMS[name]()
    m = build(room)
    apply_styles(room, m)
    lumps = dict(m['lumps'])
    lumps['THINGS'] = b''.join(struct.pack('<hhhhh', *t) for t in room.things)
    assert list(lumps) == ['MAP01'] + A.MAP_LUMPS, list(lumps)
    wad = framing() + [('E1M1', b'')] + [(n, lumps[n]) for n in A.MAP_LUMPS]
    data = bytearray(b'IWAD' + struct.pack('<ii', len(wad), 0))
    directory = b''
    for n, b in wad:
        directory += struct.pack('<ii8s', len(data), len(b), X.name8(n))
        data += b
    struct.pack_into('<i', data, 8, len(data))
    return bytes(data + directory), room, m


def script_text(name):
    lump, what, lines = SCRIPTS[name]
    return f'# M5a room {name}: {what}.\n# Generated by tools/reference/m5a_wad.py.\n' + '\n'.join(lines) + '\n'


def fields(name, data, room, m):
    lump, what, _ = SCRIPTS[name]
    lmp, tics = S.lump(script_text(name))
    return dict(name=name.upper(), file=f'tests/reference/m5a_wads/{name}.wad', slot='E1M1', sha256=hashlib.sha256(data).hexdigest(),
                bytes=len(data), sectors=len(room.sectors), lines=len(m['lines']),
                things={str(k): v for k, v in B.spawned(room, 2).items()}), \
        dict(lump=lump, txt=f'tests/reference/m5a_scripts/{lump}.txt', lmp=f'tests/reference/m5a_scripts/{lump}.lmp',
             what=what, tics=tics, lmp_sha256=hashlib.sha256(lmp).hexdigest(), views=VIEWS), lmp


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--only', choices=list(ROOMS))
    ap.add_argument('--list', action='store_true')
    ap.add_argument('--manifest', action='store_true')
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    names = [n for n in ROOMS if not a.only or n == a.only]
    man = json.loads(MANIFEST.read_text()) if MANIFEST.exists() else dict(schema=MANIFEST_SCHEMA, rooms=[])
    if man.get('schema') != MANIFEST_SCHEMA: sys.exit('manifest schema')
    by = {r['name']: r for r in man['rooms']}
    bad = []
    for n in names:
        data, room, m = wad_bytes(n)
        wf, sf, lmp = fields(n, data, room, m)
        txt = script_text(n)
        if a.list:
            print(f"{wf['name']}: {wf['file']}, {wf['bytes']} bytes, sectors {wf['sectors']}, lines {wf['lines']}, "
                  f"things {sum(wf['things'].values())}, script {sf['lump']} {sf['tics']} tics")
            continue
        if a.check:
            for path, want in ((ROOT / wf['file'], data), (ROOT / sf['txt'], txt.encode()), (ROOT / sf['lmp'], lmp)):
                if not path.exists() or path.read_bytes() != want: bad.append(str(path.relative_to(ROOT)))
            r = by.get(wf['name'])
            if r is None: bad.append(f'manifest: {wf["name"]} missing'); continue
            for k, v in dict(wf, png=[list(x) for x in PNG[n]]).items():
                if r.get(k) != v: bad.append(f'manifest: {wf["name"]} field {k}')
            if [(e['what'], e['expr']) for e in r.get('script', {}).get('must_show', [])] != MUST_SHOW[n]:
                bad.append(f'manifest: {wf["name"]} must_show expressions')
            sc = r.get('script', {})
            for k, v in sf.items():
                if sc.get(k) != v: bad.append(f'manifest: {wf["name"]} script field {k}')
            if not bad: print(f"{wf['file']} matches: {len(data)} bytes; {sf['lmp']} {sf['tics']} tics", flush=True)
            continue
        OUT_DIR.mkdir(parents=True, exist_ok=True); SCRIPT_DIR.mkdir(parents=True, exist_ok=True)
        (ROOT / wf['file']).write_bytes(data)
        (ROOT / sf['txt']).write_text(txt)
        (ROOT / sf['lmp']).write_bytes(lmp)
        print(f"wrote {wf['file']} ({len(data)} bytes), {sf['lmp']} ({sf['tics']} tics)")
        if a.manifest:
            r = by.setdefault(wf['name'], {})
            old = {e['what']: e for e in r.get('script', {}).get('must_show', [])}
            ms = [dict(what=w, expr=e, **({'tic': old[w]['tic']} if w in old and old[w].get('expr') == e and 'tic' in old[w] else {}))
                  for w, e in MUST_SHOW[n]]
            sc = {k: v for k, v in r.get('script', {}).items() if k == 'c_end'}
            r.clear(); r.update(wf); r['png'] = [list(x) for x in PNG[n]]; r['script'] = dict(sf, must_show=ms, **sc)
    if a.check:
        if bad: sys.exit('differs or missing: ' + ', '.join(bad))
        print(f'{MANIFEST.relative_to(ROOT)}: generated fields match ({len(names)} rooms)')
    elif a.manifest and not a.list:
        order = [n.upper() for n in ROOMS]
        man['rooms'] = sorted(by.values(), key=lambda r: order.index(r['name']))
        MANIFEST.write_text(json.dumps(man, indent=1) + '\n')
        print(f'updated {MANIFEST.relative_to(ROOT)}')


if __name__ == '__main__':
    main()
