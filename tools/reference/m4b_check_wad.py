#!/usr/bin/env python3
"""Independent verifier for the M4b synthetic mechanism WADs (stdlib only; does not import
the generator m4b_wad.py, the M4a generator or the room builders).

    m4b_check_wad.py WAD [--name NAME]     exits nonzero on the first violated rule
    m4b_check_wad.py --check               every WAD of tests/reference/m4b_manifest.json and
                                           tests/reference/m4b_manifest2.json

The lump parser and the format rules are the frozen M4a checker's (m4a_check_wad.Map and
mobjinfo, imported read-only: directory, resources, textures, switches, record sizes, index
ranges, ML_TWOSIDED, REJECT and BLOCKMAP sizes, the map lumps after the slot). On top of
them this file checks, from the parsed map and the genuine linuxdoom-1.10/info.c table:

- things: exactly one player 1 start, not the last THING; every type in info.c; the spawned
  counts on gameskill 2 and 3 in single player (P_SpawnMapThing, p_mobj.c:740-751) equal the
  manifest's; every thing spawned on either skill fits its sector's height, and no player,
  shootable thing touches a one-sided wall;
- lines: every special in M4a's part-1 set or 97 (WR teleport, p_spec.c:904-907); every
  tagged special's tag on some sector; every 97 line two-sided; the M4a switch-texture rules;
- teleports (EV_Teleport, p_telept.c:74-98): per 97 tag the tagged sectors in index order
  and the landings (doomednum 14) each holds, located through the nodes, with the skill bits;
  the landing the search picks on skills 2 and 3 (the first, in THINGS order, of the
  lowest-indexed tagged sector that holds one); per-WAD expectations in EXPECT;
- the special-line box: the largest number of special two-sided lines that one 32x32
  player box can overlap (PIT_CheckLine's bbox test, p_map.c:191-199, and a straddle for
  these axis-aligned lines, P_BoxOnLineSide), which is what one P_CheckPosition can append to
  spechit (p_map.c:240-244); 8 is the most the array holds; SPECH9 is built to reach 9;
- E1M8: the slot named by the manifest, a tag-666 sector and two Barons (A_BossDeath,
  p_enemy.c:1629-1635, 1729-1734);
- BTUNNEL (M4b step 9): the Baron fireball's tunnel through an inner one-sided wall, re-derived
  from the parsed map and the genuine tables (see `tunnel`);
- TELEZ (M4b step 14): the Demon's first step crosses the 97 line while its box reaches a floor 24
  above its spawn z, and the line's back sector floor is not above that z (see `straddle`).
"""
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m4a_check_wad as C   # noqa: E402  frozen M4a parser and format rules (read-only)
from m4a_check_wad import need, fail  # noqa: E402,F401

MANIFEST = ROOT / 'tests/reference/m4b_manifest.json'
MANIFEST2 = ROOT / 'tests/reference/m4b_manifest2.json'
SPECIALS = C.PART1 | {97}
LANDING, BARON, PLAYER1 = 14, 3003, 1
# Per-WAD expectations: the landing picked per tag on (skill 2, skill 3) as a THINGS index (None: no
# teleport), the largest special-line box count, the slot, the tag-666 sector and Baron count.
EXPECT = {
    'TELE': dict(pick={1: (4, 4), 2: (2, 2)}, box=1),
    'TELEMOVE': dict(pick={1: (1, 1), 2: (3, 3)}, box=2),
    'TFRAG': dict(pick={1: (4, 4), 2: (5, 5), 3: (6, 6)}, box=1),
    'MONTELE': dict(pick={1: (3, 3), 2: (4, 4)}, box=1),
    'SLIDEB': dict(pick={1: (3, 3)}, box=1),
    'SLIDEC': dict(pick={1: (3, 3)}, box=1),
    'BARON': dict(pick={}, box=0),
    'INFIGHT': dict(pick={}, box=0),
    'BVB': dict(pick={}, box=0),
    'BOSS8': dict(pick={1: (3, 3), 2: (4, 4), 3: (5, 5)}, box=1, boss=True),
    'BOSSM1': dict(pick={1: (3, 3), 2: (4, 4), 3: (5, 5)}, box=1, boss=True),
    'SPECH8': dict(pick={9: (None, None)}, box=8),
    'SPECH9': dict(pick={9: (None, None)}, box=9),
    'SKILL': dict(pick={1: (None, 1)}, box=1),
    # The second set (tests/reference/m4b_manifest2.json, M4b steps 5, 7 and 9).
    'TELE2X': dict(pick={1: (1, 1), 2: (2, 2)}, box=1),
    'SAWBITE': dict(pick={}, box=0),
    # M4b step 9: the fireball tunnel; the Baron x positions the cage allows (see `tunnel`).
    'BTUNNEL': dict(pick={}, box=0, tunnel=(24, 32, 40)),
    # M4b step 14 (review finding F1): the first fog's z after a step-up teleport (see `straddle`).
    'TELEZ': dict(pick={1: (2, 2)}, box=1, straddle=24),
}
DEMON = 3002

FRACUNIT, MAXMOVE = 65536, 30 * 65536      # m_fixed.h; p_local.h:54
SRC = ROOT / 'linuxdoom-1.10'


def table(name):
    """An initialised array of linuxdoom-1.10/tables.c (finesine, tantoangle)."""
    text = (SRC / 'tables.c').read_text(encoding='latin-1')
    body = text[text.index(name + ' ='):]
    body = body[body.index('{') + 1:body.index('}')]
    return [int(v) for v in re.findall(r'-?\d+', re.sub(r'//[^\n]*', '', body))]


def info_row(member):
    """speed and radius of one mobjinfo row of linuxdoom-1.10/info.c, in fixed point."""
    text = (SRC / 'info.c').read_text(encoding='latin-1')
    block = text[text.index('// ' + member + '\n'):]
    block = block[:block.index('\n    }')]
    f = {k: v.strip() for v, k in re.findall(r'^\t(.*?),?\t+// (\w+)\s*$', block, re.M)}
    fixed = lambda v: int(v.replace('*FRACUNIT', '')) * (FRACUNIT if 'FRACUNIT' in v else 1)
    return fixed(f['speed']), fixed(f['radius'])


def point_to_angle2(x1, y1, x2, y2, tan):
    """R_PointToAngle2 (r_main.c:292-388) with SlopeDiv (tables.c:50-62), on fixed-point values."""
    def slope(num, den):
        return 2048 if den < 512 else min(((num << 3) & 0xFFFFFFFF) // (den >> 8), 2048)
    x, y, m = x2 - x1, y2 - y1, 0xFFFFFFFF
    if not x and not y: return 0
    if x >= 0:
        if y >= 0: return tan[slope(y, x)] if x > y else 0x40000000 - 1 - tan[slope(x, y)]
        return -tan[slope(-y, x)] & m if x > -y else 0xC0000000 + tan[slope(x, -y)]
    if y >= 0: return 0x80000000 - 1 - tan[slope(y, -x)] if -x > y else 0x40000000 + tan[slope(-x, y)]
    return 0x80000000 + tan[slope(-y, -x)] if -x > -y else 0xC0000000 - 1 - tan[slope(-x, -y)]


def tunnel(m, th, expect):
    """The BTUNNEL design, from the parsed map only. The Baron's sector must be an axis-aligned
    rectangle exactly as tall as its box, ringed by two-sided lines whose other side is more than 24
    lower (P_TryMove's dropoff test, p_map.c:481-483): it can then only step east or west by its speed
    (P_Move, p_enemy.c:289-292), to the expected x positions. For each, the fireball aimed at the player
    start (P_SpawnMissile, p_mobj.c:900-923; P_CheckMissileSpawn's half step, 876-878; one unsplit step
    a tic, 150-164) is stepped through PIT_CheckLine's tests on every line (bbox, p_map.c:191-195;
    P_BoxOnLineSide for axis-aligned lines, p_maputl.c:110-153; one-sided blocks, p_map.c:209-210)
    with a fixed z clear of every floor and ceiling it passes. Required: the Baron-to-player segment
    crosses no one-sided line (P_CheckSight can succeed), exactly one step's centre segment crosses a
    one-sided line without being blocked, that line lies strictly inside the vertex rectangle, the
    step lands inside a sector's area (even-odd over that sector's lines) and inside the rectangle,
    and the first blocked step straddles a one-sided line on the rectangle's edge. Returns report lines
    with the smallest margins (units)."""
    fs, tan = table('int finesine[10240]'), table('angle_t tantoangle[2049]')
    need(len(fs) == 10240 and len(tan) == 2049, 'tables.c arrays')
    speed, radius = info_row('MT_BRUISERSHOT')
    bspeed, bradius = info_row('MT_BRUISER')
    barons = [t for t in th if t[3] == BARON]
    start = next(t for t in th if t[3] == PLAYER1)
    need(len(barons) == 1, 'one Baron')
    bx, by = barons[0][0], barons[0][1]
    V = m.verts
    seg = lambda L: (V[L['a']], V[L['b']])
    xs = [v[0] for v in V]; ys = [v[1] for v in V]
    rect = (min(xs), min(ys), max(xs), max(ys))
    cage = locate(m, bx, by)
    cl = [m.lines[i] for i in m.sector_lines(cage)]
    cx = sorted({p[0] for L in cl for p in seg(L)}); cy = sorted({p[1] for L in cl for p in seg(L)})
    need(len(cl) == 4 and len(cx) == 2 and len(cy) == 2, 'the Baron sector is one rectangle')
    r = bradius // FRACUNIT
    need(by - r == cy[0] and by + r == cy[1], 'the Baron box spans the cage height')
    for L in cl:
        need(L['s1'] != -1, 'cage line one-sided')
        other = L['back'] if L['front'] == cage else L['front']
        need(m.secs[cage]['floor'] - m.secs[other]['floor'] > 24, 'cage neighbour within a step')
    step = bspeed
    pos = sorted({bx + k * step for k in range(-8, 9) if cx[0] <= bx + k * step - r and bx + k * step + r <= cx[1]})
    need(tuple(pos) == expect, 'cage x positions %s, expected %s' % (pos, expect))
    one = [L for L in m.lines if L['s1'] == -1]
    def crosses(p, q, L):
        (ax, ay), (bx_, by_) = seg(L)
        d = lambda o, a, b: (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
        A, B = (ax * FRACUNIT, ay * FRACUNIT), (bx_ * FRACUNIT, by_ * FRACUNIT)
        return d(p, q, A) * d(p, q, B) < 0 and d(A, B, p) * d(A, B, q) < 0
    sx, sy = start[0] * FRACUNIT, start[1] * FRACUNIT
    need(not any(crosses((bx * FRACUNIT, by * FRACUNIT), (sx, sy), L) for L in one), 'a one-sided line blocks the sight segment')
    floor = m.secs[cage]['floor']
    z = (floor + 32) * FRACUNIT                                   # P_SpawnMissile's source->z + 32 (p_mobj.c:897-899)
    need(m.secs[locate(m, start[0], start[1])]['floor'] == floor, 'momz is not 0')
    def blocked(x, y):
        """The one-sided line straddled by the box at (x, y), if any (axis-aligned lines only)."""
        l, rr, b, t = x - radius, x + radius, y - radius, y + radius
        for L in one:
            (x0, y0), (x1, y1) = seg(L)
            need(x0 == x1 or y0 == y1, 'one-sided line not axis-aligned')
            lo_x, hi_x, lo_y, hi_y = min(x0, x1) * FRACUNIT, max(x0, x1) * FRACUNIT, min(y0, y1) * FRACUNIT, max(y0, y1) * FRACUNIT
            if rr <= lo_x or l >= hi_x or t <= lo_y or b >= hi_y: continue
            if (y0 == y1 and b <= lo_y < t) or (x0 == x1 and l <= lo_x < rr): return L
        return None
    def inside(x, y, s):
        n = 0
        for i in m.sector_lines(s):
            (x0, y0), (x1, y1) = seg(m.lines[i])
            if x0 == x1 and (y0 * FRACUNIT <= y) != (y1 * FRACUNIT <= y) and x < x0 * FRACUNIT: n += 1
        return n % 2 == 1
    on_edge = lambda L: all(p[0] in (rect[0], rect[2]) for p in seg(L)) or all(p[1] in (rect[1], rect[3]) for p in seg(L))
    strictly_in = lambda L: all(rect[0] < p[0] < rect[2] and rect[1] < p[1] < rect[3] for p in seg(L))
    report, margins = [], []
    for x0 in pos:
        an = point_to_angle2(x0 * FRACUNIT, by * FRACUNIT, sx, sy, tan) >> 19
        mx, my = (speed * fs[an + 2048]) >> 16, (speed * fs[an]) >> 16
        need(abs(mx) <= MAXMOVE // 2 and abs(my) <= MAXMOVE // 2, 'the step splits')
        x, y = x0 * FRACUNIT + (mx >> 1), by * FRACUNIT + (my >> 1)
        need(blocked(x, y) is None, 'the spawn half step is blocked')
        tunnels, n = [], 0
        while True:
            n += 1
            need(n < 1000, 'the fireball never stops')
            nx, ny = x + mx, y + my
            L = blocked(nx, ny)
            if L is not None:
                need(on_edge(L), 'blocked by an inner line')
                (lx0, ly0), (lx1, ly1) = seg(L)
                wall = (ly0 * FRACUNIT, ny) if ly0 == ly1 else (lx0 * FRACUNIT, nx)
                margins.append(min(wall[1] + radius - wall[0], wall[0] - (wall[1] - radius)))
                break
            s = locate(m, nx / FRACUNIT, ny / FRACUNIT)
            need(m.secs[s]['floor'] * FRACUNIT <= z and z + 8 * FRACUNIT <= m.secs[s]['ceil'] * FRACUNIT, 'z leaves the sector')
            hit = [L for L in one if crosses((x, y), (nx, ny), L)]
            if hit:
                need(all(strictly_in(L) for L in hit), 'tunnel through an outer wall')
                need(inside(nx, ny, s) and rect[0] * FRACUNIT < nx < rect[2] * FRACUNIT
                     and rect[1] * FRACUNIT < ny < rect[3] * FRACUNIT, 'the tunnel lands outside a sector')
                for L in hit:
                    (lx0, ly0), (lx1, ly1) = seg(L)
                    need(ly0 == ly1, 'the tunnelled wall is not horizontal')
                    tunnels.append((n, ly0))
            x, y = nx, ny
        need(len(tunnels) == 2, 'expected the two faces of one slab in one step, got %s' % tunnels)
        (n1, w1), (n2, w2) = tunnels
        need(n1 == n2, 'the slab faces are crossed in different steps')
        lo, hi = min(w1, w2) * FRACUNIT, max(w1, w2) * FRACUNIT
        prev = y - (n - n1) * my                                  # the centre before the tunnelling step
        margins.append(min(lo - (prev + radius), (prev + my - radius) - hi))
        report.append('Baron x %d: angle %d, mom (%d, %d), tunnel at step %d past the slab y %d..%d, explodes at step %d'
                      % (x0, an, mx, my, n1, lo // FRACUNIT, hi // FRACUNIT, n))
    need(min(margins) > 0, 'no margin')
    report.append('smallest margin %.4f units' % (min(margins) / FRACUNIT))
    return report


def straddle(m, th, rise):
    """The TELEZ design, from the parsed map only. One Demon, its centre in sector F; one 97 line, vertical,
    front F (side 0: EV_Teleport acts, p_telept.c:68-71), back sector M, east of the Demon's centre and
    inside its box (radius from info.c), its y span covering the centre; the player start east of the
    Demon by more than 10 units with |dy| <= 10, so the Demon's first P_Move is DI_EAST (P_NewChaseDir,
    p_enemy.c); the Demon's speed (info.c) takes its centre into M in that step. For the box after that
    step, P_TryMove's tmfloorz (the highest floor over the centre sector and both sides of every
    two-sided line the box straddles, PIT_CheckLine, p_map.c:191-262; axis-aligned lines) is exactly
    `rise` (1..24, the step-up test p_map.c:477-479) above F's floor, the Demon's spawn z (P_SpawnMobj,
    p_mobj.c:517-525), and the lowest floor is within 24 of it (p_map.c:481-483); the box meets no
    one-sided line; M's floor is at or below F's, so fog 1 (spawned at the pre-move z in M) is not lifted
    by its own P_ZMovement (p_mobj.c:284-311): the fog's z tells the pre-move z from tmfloorz."""
    dspeed, dradius = info_row('MT_SERGEANT')
    demons = [t for t in th if t[3] == DEMON]
    start = next(t for t in th if t[3] == PLAYER1)
    need(len(demons) == 1, 'one Demon')
    dx, dy = demons[0][0], demons[0][1]
    r, step = dradius // FRACUNIT, dspeed          # a monster's speed is in units: P_Move multiplies it by xspeed (FRACUNIT), p_enemy.c:290-291
    tele = [L for L in m.lines if L['special'] == 97]
    need(len(tele) == 1, 'one 97 line')
    L = tele[0]
    (x0, y0), (x1, y1) = m.verts[L['a']], m.verts[L['b']]
    need(x0 == x1, 'the 97 line is vertical')
    f = locate(m, dx, dy)
    mid = locate(m, dx + step, dy)
    need(L['front'] == f and L['back'] == mid, 'the step does not go from the 97 line front sector to its back sector')
    need(dx < x0 < dx + r and dx + step > x0 and min(y0, y1) < dy < max(y0, y1), 'the Demon box does not straddle the line, or one step does not cross it')
    need(start[0] - dx > 10 and abs(start[1] - dy) <= 10, 'the player is not due east')
    nx = dx + step
    one = [(m.verts[K['a']], m.verts[K['b']]) for K in m.lines if K['s1'] == -1]
    need(not any(C.box_meets(p, q, nx, dy, r) for p, q in one), 'the step meets a one-sided wall')
    floors = [m.secs[mid]['floor']]
    for K in m.lines:
        if K['s1'] == -1: continue
        (a0, b0), (a1, b1) = m.verts[K['a']], m.verts[K['b']]
        need(a0 == a1 or b0 == b1, 'two-sided line not axis-aligned')
        lo_x, hi_x, lo_y, hi_y = min(a0, a1), max(a0, a1), min(b0, b1), max(b0, b1)
        if nx + r <= lo_x or nx - r >= hi_x or dy + r <= lo_y or dy - r >= hi_y: continue      # bbox, p_map.c:191-195
        if (a0 == a1 and nx - r < a0 < nx + r) or (b0 == b1 and dy - r < b0 < dy + r):         # P_BoxOnLineSide == -1
            floors += [m.secs[K['front']]['floor'], m.secs[K['back']]['floor']]
    z = m.secs[f]['floor']
    need(1 <= rise <= 24 and max(floors) - z == rise, 'tmfloorz - z is %d, expected %d' % (max(floors) - z, rise))
    need(max(floors) - min(floors) <= 24, 'a dropoff')
    need(m.secs[mid]['floor'] <= z, 'the fog sector floor is above the pre-move z')
    need(min(m.secs[s]['ceil'] for s in (f, mid)) - max(floors) >= 56, 'no headroom')
    return ['Demon at x %d (box %d..%d) z %d, 97 line x %d (sector %d to %d), one step to x %d: tmfloorz %d, fog sector floor %d'
            % (dx, dx - r, dx + r, z, x0, f, mid, nx, max(floors), m.secs[mid]['floor'])]


def spawned(things, skill):
    bit = 1 if skill <= 1 else 4 if skill >= 3 else 1 << (skill - 1)
    return [t for t in things if t[3] == PLAYER1 or (not t[4] & 16 and t[4] & bit)]


def counts(things):
    out = {}
    for t in things: out[str(t[3])] = out.get(str(t[3]), 0) + 1
    return dict(sorted(out.items(), key=lambda kv: int(kv[0])))


def box_max(m):
    """Largest count of special two-sided lines one 32x32 box overlaps (axis-aligned lines only)."""
    lines = [L for L in m.lines if L['special'] and L['s1'] != -1]
    if not lines: return 0
    segs = []
    for L in lines:
        (x0, y0), (x1, y1) = m.verts[L['a']], m.verts[L['b']]
        need(x0 == x1 or y0 == y1, 'special two-sided line not axis-aligned')
        segs.append((min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1)))
    def centres(vals):
        edges = sorted({v + d for v in vals for d in (-16, 16)})
        return [(a + b) / 2 for a, b in zip(edges, edges[1:])]
    xs = centres({c for s in segs for c in (s[0], s[2])})
    ys = centres({c for s in segs for c in (s[1], s[3])})
    best = 0
    for cx in xs:
        for cy in ys:
            l, r, b, t = cx - 16, cx + 16, cy - 16, cy + 16
            n = 0
            for x0, y0, x1, y1 in segs:
                if x0 == x1: n += l < x0 < r and not (t <= y0 or b >= y1)      # vertical: straddled, y overlap
                else: n += b < y0 < t and not (r <= x0 or l >= x1)              # horizontal
            best = max(best, n)
    return best


def locate(m, x, y):
    """m4a_check_wad.Map.locate, plus the one-subsector map with no node (R_PointInSubsector returns
    subsectors[0] when numnodes is 0, r_main.c:807-808)."""
    if not m.nodes:
        a, b, ang, ln, sd, off = m.segs[m.ss[0][1]]
        L = m.lines[ln]
        return m.sides[L['s1'] if sd else L['s0']]['sector']
    return m.locate(x, y)


def check(path, name, slot, expect_things):
    info = C.mobjinfo()
    m = C.Map(path, slot)
    th = m.things
    starts = [i for i, t in enumerate(th) if t[3] == PLAYER1]
    need(len(starts) == 1 and starts[0] != len(th) - 1, 'one player 1 start, not the last THING')
    need(all(t[3] in info or t[3] == PLAYER1 for t in th), 'unknown thing types')
    for sk in (2, 3):
        got = counts(spawned(th, sk))
        if expect_things is not None:
            need(got == expect_things[str(sk)], 'spawned counts on skill %d differ from the manifest' % sk)
    one_sided = [(m.verts[L['a']], m.verts[L['b']]) for L in m.lines if L['s1'] == -1]
    for t in {t for sk in (2, 3) for t in spawned(th, sk)}:
        s = m.secs[locate(m, t[0], t[1])]
        h = 56 if t[3] == PLAYER1 else info[t[3]]['height']
        need(s['ceil'] - s['floor'] >= h, 'thing %d at (%d,%d) taller than its sector' % (t[3], t[0], t[1]))
        if t[3] == PLAYER1 or info[t[3]]['flags'] & C.MF_SHOOTABLE:
            r = 16 if t[3] == PLAYER1 else info[t[3]]['radius']
            need(not any(C.box_meets(p, q, t[0], t[1], r) for p, q in one_sided), 'thing %d at (%d,%d) touches a wall' % (t[3], t[0], t[1]))
    tags = {s['tag'] for s in m.secs}
    spec = {}
    for L in m.lines:
        sp = L['special']
        if not sp: continue
        spec[sp] = spec.get(sp, 0) + 1
        need(sp in SPECIALS, 'special %d outside the M4b set' % sp)
        if sp not in C.UNTAGGED: need(L['tag'] and L['tag'] in tags, 'special %d tag %d on no sector' % (sp, L['tag']))
        if sp == 97: need(L['s1'] != -1, 'teleport line one-sided')
        side = m.sides[L['s0']]
        has_switch = any(side[k] in C.SWITCH_NAMES for k in ('top', 'mid', 'bot'))
        if sp in C.SWITCHED and L['s1'] == -1: need(has_switch, 'switch line %d without a switch texture' % sp)
    for L in m.lines:
        if L['special'] in C.SWITCHED and L['s1'] == -1: continue
        for sd in (L['s0'], L['s1']):
            if sd != -1: need(not any(m.sides[sd][k] in C.SWITCH_NAMES for k in ('top', 'mid', 'bot')), 'switch texture on a non-switch line')
    exp = EXPECT.get(name)
    need(exp is not None, 'no expectations for ' + name)
    report = []
    landing_sector = {i: locate(m, t[0], t[1]) for i, t in enumerate(th) if t[3] == LANDING}
    tele_tags = sorted({L['tag'] for L in m.lines if L['special'] == 97})
    need(set(tele_tags) == set(exp['pick']), 'teleport tags %s, expected %s' % (tele_tags, sorted(exp['pick'])))
    for tag in tele_tags:
        tagged = [i for i, s in enumerate(m.secs) if s['tag'] == tag]
        picks = []
        for sk in (2, 3):
            live = {i for i, t in enumerate(th) if t[3] == LANDING and t in spawned(th, sk)}
            pick = next((i for s in tagged for i in sorted(live) if landing_sector[i] == s), None)
            picks.append(pick)
        report.append('tag %d: tagged sectors %s, landings %s, picked on skills 2/3: %s' % (
            tag, tagged, {i: (landing_sector[i], th[i][4]) for i in landing_sector if landing_sector[i] in tagged}, picks))
        need(tuple(picks) == exp['pick'][tag], 'tag %d picks %s, expected %s' % (tag, picks, exp['pick'][tag]))
    bm = box_max(m)
    need(bm == exp['box'], 'special-line box maximum %d, expected %d' % (bm, exp['box']))
    if exp.get('tunnel'):
        report += tunnel(m, th, exp['tunnel'])
    if exp.get('straddle'):
        report += straddle(m, th, exp['straddle'])
    if exp.get('boss'):
        need(any(s['tag'] == 666 for s in m.secs), 'no tag-666 sector')
        need(sum(t[3] == BARON for t in spawned(th, 2)) == 2, 'two Barons')
    print('OK %s (%s, %s): %d bytes; sectors %d, lines %d, things %d (spawned %d on skill 2, %d on skill 3), specials %s, '
          'special-line box maximum %d' % (path, name, slot, len(m.d), len(m.secs), len(m.lines), len(th), len(spawned(th, 2)),
                                           len(spawned(th, 3)), ' '.join('%dx%d' % kv for kv in sorted(spec.items())), bm))
    for r in report: print('  ' + r)


def main():
    args = sys.argv[1:]
    wads = [w for m in (MANIFEST, MANIFEST2) for w in json.loads(m.read_text())['wads']]
    if '--check' in args:
        for w in wads: check(ROOT / w['file'], w['name'], w['slot'], w['things'])
        return
    if not args or args[0].startswith('--'): sys.exit(__doc__)
    path = Path(args[0])
    rel = path.resolve().relative_to(ROOT).as_posix() if path.resolve().is_relative_to(ROOT) else str(path)
    w = {x['file']: x for x in wads}.get(rel)
    name = args[args.index('--name') + 1] if '--name' in args else (w['name'] if w else path.stem.upper())
    check(path, name, w['slot'] if w else 'E1M1', w['things'] if w else None)


if __name__ == '__main__':
    main()
