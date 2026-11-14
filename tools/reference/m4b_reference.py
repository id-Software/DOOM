#!/usr/bin/env python3
"""Three-variant reference run of the genuine whole-engine runner on the M4b synthetic WADs.

Manifest-driven, as tools/reference/m4a_reference.py (imported as a library for its dump
reader and event expressions): tests/reference/m4b_manifest.json (schema
DOOM-M4B-MANIFEST-v1) lists per WAD (tests/reference/m4b_wads/, written by m4b_wad.py) its
scripts (tests/reference/m4b_scripts/, text and .lmp, written by m4b_scripts.py) with the
expected C ending (`c_end`, `end_tic`), the Rust expectation (`rust`: "tbd" while wire 24
does not exist, or {"refuse": NAME, "tic": N}) and `must_show`, the events the script exists
to produce, each with the tic at which it first appears in the dump ("tic": null means the
event must never happen). Event expressions are m4a_reference.py's (first, gone, edge,
stall; @LABEL resolves through m4b_wad.labels), plus one added here (M4b step 7):

  bite COND...   the first tic at which the player's mobj row satisfies every COND (m4a_reference
                 conditions), its health is below its health on the previous dumped tic, and the
                 player's `attacker` (player->attacker = source, p_inter.c:874) is a Demon or Spectre
                 row that entered S_SARG_ATK3 in that tic (its tics are the state's full 8, info.c:623;
                 the state's action is A_SargAttack). `bite flags&128` is a bite on a body that still
                 carries MF_JUSTATTACKED, which only A_Saw sets (p_pspr.c:542) and the next
                 P_PlayerThink clears (p_user.c:249-255): a bite in the tic of an A_Saw hit.

The runner's strict, overflow and address variants are built with m4b_build.build() (the
M4a driver with the declared M4b telefrag adapter in the two sanitizer builds) under --work;
every script runs in each variant through m3h9c_reference.run(); the three dump sets must be
byte-identical (SHA-256 of the concatenated snapshots) and end the same way; expectations
and the per-script `facts` (observed mechanism facts computed below from the strict dumps:
tics, draws, momenta, landing positions, the shadow-fuzz draw order) are evaluated on the
strict dumps. tests/reference/m4b_synthetic_results.json records per script the tic count,
the ending, prndindex after the load and at the end, the thinker and mobj counts after the
load, the snapshot count, the dump-set digest, the event tics and the facts; its `timing`
block is informational (machine not idle) and is not compared by --check.

A manifest script may carry `overrun_probe` (SPECH9): its `lmp` (a longer demo with the same
first commands) is run on the script's WAD in the address variant only, which must stop with
the sanitizer report naming `report` (the unchecked spechit append, p_map.c:242) after
dumping exactly up to `last_dump`: so the genuine engine appends a ninth spechit entry in
tic last_dump + 1. No dump of that tic or later is used as an expectation: the script
itself ends before it.

Everything here is synthetic. The address variant runs with ASAN_OPTIONS=detect_leaks=0
(leak detection disabled; not LeakSanitizer coverage). Dumps are deleted after the
evaluation unless --keep-dumps is given.

    m4b_reference.py [--work DIR] [--check] [--keep-dumps] [--cc CC] [--only WAD]
                     [--selftest TIC] [--update-events] [--strict32] [--set 1|2]

--set 2 runs the second set instead (tests/reference/m4b_manifest2.json and
m4b_synthetic_results2.json, added in M4b step 5 so that the first set's files stay
byte-identical; it has no selftest script).

--check compares the regenerated results (minus `timing`) with the committed file.
--selftest TIC plants an extra P_Random draw before G_Ticker TIC and a changed command TIC
on the manifest's `selftest` script in the strict build and checks that each is first seen
in the dump of tic TIC. --strict32 (local opt-in, needs -m32) also runs every script in the
strict32 build (genuine text, ILP32, no adapter) and requires its dumps to be identical too;
it writes no result. --update-events writes observed end and event tics into the manifest
(authoring aid; reviewed, then held fixed by --check).
"""
import argparse, hashlib, json, math, os, re, shutil, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m4b_build as build         # noqa: E402
import m3h9c_dump as dump         # noqa: E402
import m3h9c_reference as ref     # noqa: E402
import m4a_reference as R         # noqa: E402  frozen M4a event expressions and dump reader (read-only)
import m4b_wad                    # noqa: E402
from m3h9_geometry import enum_index  # noqa: E402

MANIFEST = ROOT / 'tests/reference/m4b_manifest.json'
RESULTS = ROOT / 'tests/reference/m4b_synthetic_results.json'
# The second set (M4b step 5): new WADs and scripts in their own manifest and results, so that the
# first set's files stay byte-identical. `--set 2` selects it.
SET_FILES = {1: (MANIFEST, RESULTS),
             2: (ROOT / 'tests/reference/m4b_manifest2.json', ROOT / 'tests/reference/m4b_synthetic_results2.json')}
VARIANTS = ref.VARIANTS
SCHEMA = 'DOOM-M4B-SYNTHETIC-RESULTS-v1'
sha = lambda b: hashlib.sha256(b).hexdigest()
U = 65536
MT = lambda name: enum_index('mobjtype_t', name)
ST = lambda name: enum_index('statenum_t', name)
RNDTABLE = [int(v, 0) for v in re.search(r'rndtable\[256\]\s*=\s*\{(.*?)\}', (ROOT / 'linuxdoom-1.10/m_random.c').read_text(), re.S)
            .group(1).replace('\n', ' ').split(',') if v.strip()]
assert len(RNDTABLE) == 256


# ---------------------------------------------------------------- dump helpers ---------------
def mobjs(d, t, kind=None, **cond):
    rows = d.rows(t).get('M', [])
    if kind is not None: rows = [m for m in rows if m['type'] == MT(kind)]
    return [m for m in rows if all(m[k] == v for k, v in cond.items())]


def player(d, t):
    r = d.rows(t)
    return r['M'][r['P'][0]['mo']], r['P'][0]


def draws(d, t):
    """P_Random draws made in tic t (prndindex wraps at 256)."""
    prev = d.rows(t - 1)['H'][0]['prndindex']
    return (d.rows(t)['H'][0]['prndindex'] - prev) % 256


def first(d, pred, start=-1):
    for t in d.tics:
        if t >= start and pred(t): return t
    return None


def units(v): return round(v / U, 4)


def wrapped_thrust(damage, mass):
    """The genuine strict build's thrust (p_inter.c:817 with -fwrapv): the int product wraps mod
    2^32, then the signed division truncates toward zero (computed here independently of Rust)."""
    p = (damage * 8192 * 100) & 0xFFFFFFFF
    p = p - (1 << 32) if p >= 1 << 31 else p
    return int(p / mass)


def entered(m, state):
    """True if m entered `state` in this tic (the state's full duration is still left)."""
    return m['state'] == ST(state)


# ---------------------------------------------------------------- per-script facts ------------
def f_teleport(d, who='MT_PLAYER'):
    """Teleport tics of the player: (tic, x, y, angle in degrees, reactiontime, momentum, draws, fogs)."""
    out, was = [], None
    for t in d.tics:
        if t < 0: continue
        pm, _ = player(d, t)
        fogs = mobjs(d, t, 'MT_TFOG')
        new = [f for f in fogs if f['state'] == ST('S_TFOG') and f['tics'] == 5]   # spawned this tic, thought once
        if new:
            out.append(dict(tic=t, x=units(pm['x']), y=units(pm['y']), angle=round(pm['angle'] * 360 / 2 ** 32, 3),
                            reactiontime=pm['reactiontime'], momx=pm['momx'], momy=pm['momy'], draws=draws(d, t),
                            fogs=[(units(f['x']), units(f['y'])) for f in new]))
    return out


def facts_tele1(d):
    tel = f_teleport(d)
    t = tel[0]['tic']
    back = first(d, lambda u: player(d, u)[0]['x'] < 0)
    still = [player(d, u)[0]['x'] for u in range(t, t + 19)]
    moved = first(d, lambda u: player(d, u)[0]['x'] != player(d, t)[0]['x'] or player(d, u)[0]['y'] != player(d, t)[0]['y'], t)
    cmds = [player(d, u)[1]['cmd_forward'] for u in range(t + 1, t + 19)]
    gone = first(d, lambda u: not mobjs(d, u, 'MT_TFOG'), t)
    return dict(back_side_crossing_tic=back, fogs_before_teleport=sum(len(mobjs(d, u, 'MT_TFOG')) for u in range(0, t)),
                teleports=tel, frozen_tics=sum(x == still[0] for x in still) - 1, forward_commands_during_freeze=sum(1 for c in cmds if c),
                first_move_after_freeze=moved, health_bonus_tic=first(d, lambda u: player(d, u)[1]['itemcount'] == 1),
                fogs_gone_tic=gone)


def facts_tele2(d):
    return dict(teleports=f_teleport(d))


def facts_move(d):
    tel = f_teleport(d)
    t = tel[0]['tic']
    pm1, _ = player(d, t - 1)
    return dict(teleports=tel, momentum_before=(units(pm1['momx']), units(pm1['momy'])),
                split_threshold_exceeded_before=pm1['momx'] > 15 * U or pm1['momy'] > 15 * U,
                health_bonus_tic=first(d, lambda u: player(d, u)[1]['itemcount'] == 1))


def facts_tfrag(d):
    out = []
    tel = f_teleport(d)
    for kind, mass in (('MT_POSSESSED', 100), ('MT_SERGEANT', 400), ('MT_BRUISER', 1000)):
        t = first(d, lambda u: any(m['health'] <= 0 for m in mobjs(d, u, kind)))
        v = mobjs(d, t, kind)[0]
        fog1 = next(x['fogs'][0] for x in tel if x['tic'] == t)
        thrust = wrapped_thrust(10000, mass)
        mag = math.hypot(v['momx'], v['momy'])
        towards_old = (v['momx'] * (fog1[0] - v['x'] / U) + v['momy'] * (fog1[1] - v['y'] / U)) > 0
        out.append(dict(victim=kind, mass=mass, tic=t, health=v['health'], state=v['state'], draws=draws(d, t),
                        momx=v['momx'], momy=v['momy'], momentum_units=round(mag / U, 3), wrapped_thrust=thrust,
                        magnitude_matches=abs(mag - abs(thrust)) <= 64, toward_pre_teleport_position=towards_old,
                        momentum_next_tic=(mobjs(d, t + 1, kind)[0]['momx'], mobjs(d, t + 1, kind)[0]['momy'])))
    return dict(telefrags=out, teleports=[x['tic'] for x in tel])


def facts_mtele(d):
    west = first(d, lambda u: any(m['x'] > 1200 * U for m in mobjs(d, u, 'MT_SERGEANT', sp_x=-640)))
    east = first(d, lambda u: any(m['x'] < 256 * U for m in mobjs(d, u, 'MT_SERGEANT', sp_x=640)))
    pairs = [t for t in d.tics if t >= 0 and any(f['state'] == ST('S_TFOG') and f['tics'] == 5 for f in mobjs(d, t, 'MT_TFOG'))]
    return dict(west_demon_teleport_tic=west, east_demon_crossing_tic=east, fog_spawn_tics=pairs,
                east_demon_after_crossing=[(units(m['x']), units(m['y'])) for m in mobjs(d, east, 'MT_SERGEANT', sp_x=640)],
                barrel_on_tag2_landing=[(units(m['x']), units(m['y']), m['health']) for m in mobjs(d, d.tics[-1], 'MT_BARREL')])


def facts_slideb(d):
    b1 = first(d, lambda u: any(m['health'] <= 0 for m in mobjs(d, u, 'MT_BARREL', sp_x=-156)))
    hit = first(d, lambda u: any(m['health'] < 20 for m in mobjs(d, u, 'MT_BARREL', sp_x=-8)))
    m = mobjs(d, hit, 'MT_BARREL', sp_x=-8)[0]
    tel = first(d, lambda u: any(x['x'] > 1000 * U for x in mobjs(d, u, 'MT_BARREL', sp_x=-8)))
    mt = mobjs(d, tel, 'MT_BARREL', sp_x=-8)[0]
    return dict(west_barrel_dies=b1, east_barrel_hit=hit, east_barrel_health=m['health'], east_barrel_momx=m['momx'],
                east_barrel_target=m['target'], east_barrel_teleport_tic=tel, alive_on_landing=mt['health'] > 0,
                draws_at_teleport=draws(d, tel))


def facts_slidec(d):
    k = first(d, lambda u: any(m['health'] <= 0 for m in mobjs(d, u, 'MT_POSSESSED')))
    m = mobjs(d, k, 'MT_POSSESSED')[0]
    tel = first(d, lambda u: any(x['x'] > 1000 * U for x in mobjs(d, u, 'MT_POSSESSED')))
    awake_before = any(x['state'] not in (ST('S_POSS_STND'), ST('S_POSS_STND2')) for u in range(0, k) for x in mobjs(d, u, 'MT_POSSESSED'))
    return dict(killed_tic=k, killed_asleep=not awake_before, momx_at_death=m['momx'], state_at_death=m['state'],
                corpse_teleport_tic=tel, corpse_flags_at_teleport=mobjs(d, tel, 'MT_POSSESSED')[0]['flags'])


def attacks(d, kind='MT_BRUISER'):
    """Each ATK3 entry (A_BruisAttack): ball when a MT_BRUISERSHOT appears that tic, else claw."""
    out = []
    for t in d.tics:
        if t < 0: continue
        for b in mobjs(d, t, kind):
            if b['state'] == ST('S_BOSS_ATK3') and b['tics'] == 8:
                before = {m['idx'] for m in mobjs(d, t - 1, 'MT_BRUISERSHOT')}
                new = [m for m in mobjs(d, t, 'MT_BRUISERSHOT') if m['target'] == b['idx'] and
                       abs(m['x'] - b['x']) < 40 * U and abs(m['y'] - b['y']) < 40 * U]
                out.append(dict(tic=t, baron=b['sp_x'], kind='ball' if new else 'claw', target=b['target']))
    return out


def facts_baron1(d):
    atk = attacks(d)
    hits = [t for t in d.tics if t > 0 and player(d, t)[1]['health'] < player(d, t - 1)[1]['health']]
    wall = first(d, lambda u: any(m['state'] == ST('S_BRBALLX1') and m['x'] < -740 * U for m in mobjs(d, u, 'MT_BRUISERSHOT')))
    pain = [t for t in d.tics if t >= 0 and any(entered(b, 'S_BOSS_PAIN') and b['tics'] == 2 for b in mobjs(d, t, 'MT_BRUISER'))]
    death = first(d, lambda u: any(b['health'] <= 0 for b in mobjs(d, u, 'MT_BRUISER')))
    die7 = first(d, lambda u: any(b['state'] == ST('S_BOSS_DIE7') for b in mobjs(d, u, 'MT_BRUISER')))
    return dict(attacks=atk, player_hit_tics=hits, ball_explodes_on_west_wall=wall, pain_entries=pain, death_tic=death,
                die7_tic=die7, die1_tics=die7 - death - 40 if death is not None and die7 is not None else None)


def fuzz_order(d, actor_kind, shadow_kind):
    """For each A_FaceTarget of `actor_kind` at a MF_SHADOW target (ATK1 or ATK2 entered this tic),
    the observed angle minus the unfuzzed R_PointToAngle2 angle (float, from both things' previous-tic
    positions) against (r1 - r2) << 21 and (r2 - r1) << 21 for each consecutive draw pair r1, r2 of the
    tic (P_Random returns rndtable[++prndindex]): 'left-to-right' if only the first matches somewhere,
    'right-to-left' if only the second does."""
    out = []
    for t in d.tics:
        if t <= 0: continue
        for a in mobjs(d, t, actor_kind):
            if not (a['state'] in (ST('S_BOSS_ATK1'), ST('S_BOSS_ATK2')) and a['tics'] == 8): continue
            tg = [m for m in d.rows(t)['M'] if m['idx'] == a['target']]
            if not tg or tg[0]['type'] != MT(shadow_kind): continue
            prev_a = [m for m in mobjs(d, t - 1, actor_kind) if m['sp_x'] == a['sp_x'] and m['sp_y'] == a['sp_y']][0]
            prev_t = [m for m in mobjs(d, t - 1, shadow_kind) if m['sp_x'] == tg[0]['sp_x'] and m['sp_y'] == tg[0]['sp_y']][0]
            base = math.atan2(prev_t['y'] - prev_a['y'], prev_t['x'] - prev_a['x'])
            obs = ((a['angle'] / 2 ** 32 * 2 * math.pi - base + math.pi) % (2 * math.pi)) - math.pi
            lo, hi = d.rows(t - 1)['H'][0]['prndindex'], d.rows(t)['H'][0]['prndindex']
            n = (hi - lo) % 256
            seq = [RNDTABLE[(lo + 1 + k) & 255] for k in range(n)]
            lr, rl = set(), set()
            for k in range(n - 1):
                r1, r2 = seq[k], seq[k + 1]
                if r1 == r2: continue
                for diff, tag in ((r1 - r2, lr), (r2 - r1, rl)):
                    if abs(obs - diff * 2 ** 21 / 2 ** 32 * 2 * math.pi) < 0.0005: tag.add(k)
            order = 'left-to-right' if lr and not rl else 'right-to-left' if rl and not lr else 'ambiguous' if lr and rl else 'no match'
            out.append(dict(tic=t, state=a['state'], order=order))
    return out


def facts_binf1(d):
    sp = lambda u: mobjs(d, u, 'MT_SHADOWS')
    br = lambda u: mobjs(d, u, 'MT_BRUISER')
    hit = first(d, lambda u: sp(u) and sp(u)[0]['health'] < 150)
    bar_idx = br(hit)[0]['idx']
    hurt = first(d, lambda u: br(u)[0]['health'] < 1000)
    retarget = first(d, lambda u: sp(u) and br(u)[0]['target'] == sp(u)[0]['idx'])
    fz = fuzz_order(d, 'MT_BRUISER', 'MT_SHADOWS')
    return dict(fireball_hits_spectre=hit, spectre_target_is_baron=sp(hit)[0]['target'] == bar_idx,
                baron_hurt_by_spectre=hurt, baron_targets_spectre=retarget, attacks=attacks(d),
                spectre_dies=first(d, lambda u: sp(u) and sp(u)[0]['health'] <= 0), shadow_fuzz=fz,
                shadow_fuzz_orders=sorted({x['order'] for x in fz}))


def facts_bvb1(d):
    A_ = lambda u: mobjs(d, u, 'MT_BRUISER', sp_y=0)[0]
    B_ = lambda u: mobjs(d, u, 'MT_BRUISER', sp_y=112)[0]
    bar = lambda u: mobjs(d, u, 'MT_BARREL')
    wound = first(d, lambda u: bar(u) and 0 < bar(u)[0]['health'] < 20)
    dies = first(d, lambda u: bar(u) and bar(u)[0]['health'] <= 0)
    bhurt = first(d, lambda u: B_(u)['health'] < 1000)
    btarget = first(d, lambda u: B_(u)['target'] == A_(u)['idx'])
    bursts = []
    for t in d.tics:
        if t <= 0: continue
        for m in mobjs(d, t, 'MT_BRUISERSHOT'):
            if m['state'] == ST('S_BRBALLX1') and m['tics'] >= 4:
                prev = [p for p in mobjs(d, t - 1, 'MT_BRUISERSHOT') if p['idx'] == m['idx'] and p['state'] != ST('S_BRBALLX1')]
                if not prev: continue
                owner = 'A' if m['target'] == A_(t)['idx'] else 'B' if m['target'] == B_(t)['idx'] else m['target']
                other = B_(t) if owner == 'A' else A_(t) if owner == 'B' else None
                if other is None: continue
                near = math.hypot(m['x'] - other['x'], m['y'] - other['y']) < (24 + 6 + 15) * U
                if near:
                    bursts.append(dict(tic=t, owner=owner, victim_health_before=(B_ if owner == 'A' else A_)(t - 1)['health'],
                                       victim_health_after=other['health']))
    claws = [t for t in d.tics if t > 0 and A_(t)['health'] < A_(t - 1)['health']]
    return dict(barrel_wounded=wound, barrel_health=bar(wound)[0]['health'], barrel_target_is_A=bar(wound)[0]['target'] == A_(wound)['idx'],
                barrel_dies=dies, B_hurt=bhurt, B_health=B_(bhurt)['health'], B_targets_A=btarget,
                fireballs_bursting_on_the_other_baron=bursts, A_damaged_tics=claws, attacks=attacks(d))


def facts_boss(d):
    die7 = []
    for b in sorted({m['sp_x'] for m in mobjs(d, -1, 'MT_BRUISER')}):
        die7.append(first(d, lambda u: any(m['state'] == ST('S_BOSS_DIE7') for m in mobjs(d, u, 'MT_BRUISER', sp_x=b))))
    kills = [first(d, lambda u: any(m['health'] <= 0 for m in mobjs(d, u, 'MT_BRUISER', sp_x=b)))
             for b in sorted({m['sp_x'] for m in mobjs(d, -1, 'MT_BRUISER')})]
    step = m4b_wad.labels('boss8')['BLOCK']
    fl = lambda typ: first(d, lambda u: any(f['sector'] == step and f['type'] == typ for f in d.rows(u).get('FL', [])))
    h = lambda u: d.rows(u)['S'][step]['floorheight'] // U
    last = d.tics[-1]
    return dict(kill_tics=kills, die7_tics=die7, step_height_at_first_die7=h(die7[0]) if die7[0] is not None else None,
                lower_floor_first_tic=fl(1), raise_floor_first_tic=fl(3), step_at_zero=first(d, lambda u: h(u) == 0),
                step_height_end=h(last), end_gameaction=d.rows(last)['H'][0]['gameaction'], player_health_end=player(d, last)[1]['health'])


def facts_spech(d):
    pm, _ = player(d, d.tics[-1])
    return dict(final_player_x=units(pm['x']), final_player_y=units(pm['y']))


def facts_skill(d):
    h = d.rows(-1)['H'][0]
    types = {}
    for m in d.rows(-1)['M']: types[str(m['type'])] = types.get(str(m['type']), 0) + 1
    return dict(gameskill=h['gameskill'], totalkills=h['totalkills'], totalitems=h['totalitems'], mobj_types_at_load=types,
                teleports=f_teleport(d))


def facts_tele2x(d):
    """Two teleports in one tic: the fogs spawned per tic, the draws, the momentum before the tic and
    the player's position after it."""
    tel = f_teleport(d)
    t = tel[0]['tic']
    pm1, _ = player(d, t - 1)
    pm, _ = player(d, t)
    return dict(teleports=tel, momentum_before=(units(pm1['momx']), units(pm1['momy'])),
                split_threshold_exceeded_before=pm1['momx'] > 15 * U or pm1['momy'] > 15 * U,
                fogs_in_tic=len(tel[0]['fogs']), draws_in_tic=tel[0]['draws'], position_after=(units(pm['x']), units(pm['y'])))


def facts_sawbite(d):
    """A_Saw hits (MF_JUSTATTACKED at the end of the tic) and every bite: its tic, the health lost,
    whether the body carried MF_JUSTATTACKED, the attacker's type and the player's health after it."""
    hits = [t for t in d.tics if t >= 0 and player(d, t)[0]['flags'] & 128]
    return dict(saw_hit_tics=hits,
                bites=[dict(tic=t, health_lost=lost, justattacked=bool(pm['flags'] & 128),
                            attacker='MT_SHADOWS' if a['type'] == MT('MT_SHADOWS') else 'MT_SERGEANT',
                            health_after=pm['health'])
                       for t, pm, a, lost in bites(d)])


SLAB = (368, 369)            # tools/reference/m4b_wad.py btunnel(): the 1-unit void slab's south and north faces
RECT = (-128, -64, 192, 496)   # its vertex rectangle (the hall)
BALL_RADIUS = 6                # MT_BRUISERSHOT radius (info.c:1524-1548)


def fireballs(d):
    """Every MT_BRUISERSHOT by thinker serial (T rows, never reused): its rows by tic."""
    out = {}
    for t in d.tics:
        if t < 0: continue
        r = d.rows(t)
        serial = {x['idx']: x['serial'] for x in r.get('T', [])}
        for m in r.get('M', []):
            if m['type'] == MT('MT_BRUISERSHOT'): out.setdefault(serial[m['idx']], {})[t] = m
    return out


def facts_btunnel(d):
    """Each Baron fireball (in spawn order): its spawn tic, the Baron's x then, its momentum, the last tic
    its box is wholly south of the slab and the first tic wholly north of it (tunnel_tics; the tunnel is
    one step if the second is the next tic), any tic whose box overlaps the slab's y range (must be
    none), its explosion tic and position (S_BRBALLX1, entered on P_ExplodeMissile, p_mobj.c:90-104),
    and whether any row of it lies outside the vertex rectangle; and the tics the player lost health."""
    out = []
    for serial, rows in sorted(fireballs(d).items()):
        tics = sorted(rows)
        t0 = tics[0]
        baron = [b for b in mobjs(d, t0, 'MT_BRUISER') if b['idx'] == rows[t0]['target']]
        fly = [t for t in tics if rows[t]['flags'] & 0x10000]                      # MF_MISSILE still set
        south = [t for t in fly if rows[t]['y'] + BALL_RADIUS * U <= SLAB[0] * U]
        north = [t for t in fly if rows[t]['y'] - BALL_RADIUS * U >= SLAB[1] * U]
        overlap = [t for t in fly if t not in south and t not in north and
                   rows[t]['y'] - BALL_RADIUS * U < SLAB[1] * U and rows[t]['y'] + BALL_RADIUS * U > SLAB[0] * U]
        boom = [t for t in tics if rows[t]['state'] == ST('S_BRBALLX1')]
        outside = [t for t in tics if not (RECT[0] * U < rows[t]['x'] < RECT[2] * U and RECT[1] * U < rows[t]['y'] < RECT[3] * U)]
        out.append(dict(spawn_tic=t0, baron_x=units(baron[0]['x']) if baron else None,
                        mom=(rows[t0]['momx'], rows[t0]['momy'], rows[t0]['momz']),
                        tunnel_tics=(south[-1], north[0]) if south and north and north[0] > south[-1] else None,
                        slab_overlap_tics=overlap,
                        explodes=dict(tic=boom[0], x=units(rows[boom[0]]['x']), y=units(rows[boom[0]]['y'])) if boom else None,
                        outside_rectangle_tics=outside))
    hits = [t for t in d.tics if t > 0 and player(d, t)[1]['health'] < player(d, t - 1)[1]['health']]
    return dict(fireballs=out, player_hit_tics=hits)


def facts_telez(d):
    """The Demon's teleport tic (first dump with it on the landing); its x, z, floorz and the box's
    highest floor on the dump before (z below floorz is the straddling spawn: floorz is the centre
    sector's, p_mobj.c:517-525, but P_TryMove's tmfloorz is the box's); and each MT_TFOG spawned in the
    teleport tic with its x, y and z: fog 1 at the moved x, y with the pre-move z (EV_Teleport's oldz,
    p_telept.c:100-111, runs inside P_TryMove before P_Move sets z, p_enemy.c:327-328), fog 2 beside the
    landing at the landing floor."""
    tic = first(d, lambda u: any(m['x'] == 1152 * U for m in mobjs(d, u, 'MT_SERGEANT')))
    if tic is None: return dict(teleport_tic=None)
    before = mobjs(d, tic - 1, 'MT_SERGEANT')[0]
    fogs = [f for f in mobjs(d, tic, 'MT_TFOG') if f['state'] == ST('S_TFOG')]
    return dict(teleport_tic=tic, demon_before=dict(x=units(before['x']), z=units(before['z']), floorz=units(before['floorz'])),
                demon_after=[dict(x=units(m['x']), z=units(m['z']), floorz=units(m['floorz'])) for m in mobjs(d, tic, 'MT_SERGEANT')],
                fogs=[dict(x=units(f['x']), y=units(f['y']), z=units(f['z']), tics=f['tics']) for f in fogs])


FACTS = {'TELE1': facts_tele1, 'TELE2': facts_tele2, 'TSPLIT': facts_move, 'TSLIDE': facts_move, 'TFRAG1': facts_tfrag,
         'MTELE1': facts_mtele, 'SLIDEB': facts_slideb, 'SLIDEC': facts_slidec, 'BARON1': facts_baron1, 'BINF1': facts_binf1,
         'BVB1': facts_bvb1, 'BOSS1': facts_boss, 'BOSS2': facts_boss, 'BOSSM1': facts_boss, 'SPECH8': facts_spech,
         'SPECH9': facts_spech, 'SKILL2': facts_skill, 'SKILL3': facts_skill, 'TELE2X': facts_tele2x, 'SAWBITE': facts_sawbite,
         'BTUNNEL': facts_btunnel, 'TELEZ': facts_telez}


# ---------------------------------------------------------------- runs ------------------------
SARG_ATK3_TICS = 8   # info.c:623, S_SARG_ATK3 {SPR_SARG,6,8,{A_SargAttack},S_SARG_RUN1}


def bites(d, conds=(), labels=None):
    """Tics of Demon/Spectre bites on the player (see `bite` in the module docstring): (tic, the
    player mobj row, the attacker row, the health lost)."""
    ok = R.matcher(list(conds), labels or {})
    out, prev = [], None
    for t in d.tics:
        r = d.rows(t)
        if not r.get('P'):
            continue
        p = r['P'][0]
        pm = r['M'][p['mo']]
        if prev is not None and pm['health'] < prev and ok(pm):
            att = [m for m in r['M'] if m['idx'] == p['attacker']]
            if (att and att[0]['type'] in (MT('MT_SERGEANT'), MT('MT_SHADOWS'))
                    and att[0]['state'] == ST('S_SARG_ATK3') and att[0]['tics'] == SARG_ATK3_TICS):
                out.append((t, pm, att[0], prev - pm['health']))
        prev = pm['health']
    return out


def event_tic(d, expr, labels):
    words = expr.split()
    if words[0] == 'bite':
        b = bites(d, words[1:], labels)
        return b[0][0] if b else None
    return R.event_tic(d, expr, labels)


def quiet_build(variant, out, cc):
    with open(os.devnull, 'w') as q:
        stdout, sys.stdout = sys.stdout, q
        try: return build.build(variant, out, cc)
        finally: sys.stdout = stdout


def probe(binary, wad, spec, work):
    """The overrun probe: run spec['lmp'] in `binary`; it must stop with the sanitizer report after dumping up to last_dump."""
    lmp = ROOT / spec['lmp']
    out = work / 'runs' / 'probe'
    if out.exists(): shutil.rmtree(out)
    out.mkdir(parents=True)
    p = subprocess.run([str(binary), '--wad', str(wad), '--wad', str(lmp), '--demo', lmp.stem.upper(), '--out', str(out)],
                       capture_output=True, text=True, env=dict(os.environ, ASAN_OPTIONS='detect_leaks=0'))
    names = dump.snapshot_names(str(out))
    last = names[-1]
    report = [l for l in p.stderr.splitlines() if 'runtime error' in l or 'ERROR: AddressSanitizer' in l]
    rel = [re.sub(r'^.*/(p_\w+\.c:\d+)', r'\1', l) for l in report]
    ok = p.returncode != 0 and last == f"tic_{spec['last_dump']:05d}.txt" and any(spec['report'] in l for l in rel)
    shutil.rmtree(out)
    return dict(exit_nonzero=p.returncode != 0, last_dump=last, report=rel[:1]), ok


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-m4b-reference')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--keep-dumps', action='store_true')
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--only', help='run the scripts of one WAD (no results file is written or checked)')
    ap.add_argument('--selftest', type=int, metavar='TIC')
    ap.add_argument('--update-events', action='store_true')
    ap.add_argument('--strict32', action='store_true', help='also run the opt-in strict32 build and require identical dumps')
    ap.add_argument('--set', type=int, choices=sorted(SET_FILES), default=1,
                    help='1: tests/reference/m4b_manifest.json (default); 2: the second set, m4b_manifest2.json')
    a = ap.parse_args()
    global MANIFEST, RESULTS
    MANIFEST, RESULTS = SET_FILES[a.set]
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    man = json.loads(MANIFEST.read_text())
    if man.get('schema') != 'DOOM-M4B-MANIFEST-v1': sys.exit('manifest schema')
    start = time.monotonic()
    variants = list(VARIANTS) + (['strict32'] if a.strict32 else [])
    binaries = {v: quiet_build(v, work / 'build', a.cc) for v in variants}
    if a.strict32 and binaries['strict32'] is None: sys.exit('strict32 requested but -m32 is not available')
    built = time.monotonic()
    results, timing, failures = {}, {}, []
    for w in man['wads']:
        if a.only and w['name'] != a.only.upper(): continue
        wad = ROOT / w['file']
        if sha(wad.read_bytes()) != w['sha256']: sys.exit(f"{w['name']}: WAD differs from the manifest (run m4b_wad.py --check)")
        labels = m4b_wad.labels(w['name'].lower())
        ref.WAD = wad
        for s in w['scripts']:
            lmp = ROOT / s['lmp']
            per, us = {}, {}
            for v in variants:
                out = work / 'runs' / f"{v}_{s['lump']}"
                per[v], us[v] = ref.run(binaries[v], lmp, out)
                if v == 'strict':
                    d = R.Dumps(str(out))
                    obs = [event_tic(d, e['event'], labels) for e in s.get('must_show', [])]
                    facts = FACTS[s['lump']](d)
                    rust = s.get('rust')
                    rust_tic = event_tic(d, rust['event'], labels) if isinstance(rust, dict) and 'event' in rust else None
                if not a.keep_dumps: shutil.rmtree(out)
            if len({json.dumps(r, sort_keys=True) for r in per.values()}) != 1:
                sys.exit(f"FAILED: {s['lump']}: the variants differ ({', '.join(variants)})")
            r = per['strict']
            entry = dict(wad=w['name'], lmp_sha256=sha(lmp.read_bytes()), **r,
                         must_show={e['what']: t for e, t in zip(s.get('must_show', []), obs)}, facts=facts)
            if 'overrun_probe' in s:
                pr, ok = probe(binaries[s['overrun_probe']['variant']], wad, s['overrun_probe'], work)
                entry['overrun_probe'] = pr
                if not ok: failures.append(f"{s['lump']}: overrun probe {pr}")
            if rust_tic is not None: entry['rust_refusal_tic'] = rust_tic
            results[s['lump']] = entry
            timing[s['lump']] = us
            if a.update_events:
                s['c_end'], s['end_tic'] = r['end'], r['end_tic']
                for e, t in zip(s.get('must_show', []), obs): e['tic'] = t
            bad = []
            if r['end'] != s.get('c_end') or r['end_tic'] != s.get('end_tic'): bad.append(f"end {r['end']} at {r['end_tic']}")
            for e, t in zip(s.get('must_show', []), obs):
                if t != e.get('tic', 'missing'): bad.append(f"event '{e['what']}' at {t}, expected {e.get('tic')}")
            if r['unknown_messages']: bad.append(f"{r['unknown_messages']} unknown messages")
            if bad: failures.append(f"{s['lump']}: " + '; '.join(bad))
            print(f"{s['lump']}: tics {r['tics']} end={r['end']} at tic {r['end_tic']} prndindex {r['prndindex_after_load']}->"
                  f"{r['prndindex_end']} events {len(obs)} variants identical ({', '.join(variants)}): yes"
                  + (f"; probe {entry['overrun_probe']}" if 'overrun_probe' in entry else '')
                  + ('' if not bad else ' EXPECTATION MISMATCH'), flush=True)
    runs = time.monotonic() - built
    print(f'wall time: build {built - start:.1f} s, runs {runs:.1f} s, total {time.monotonic() - start:.1f} s')
    if a.update_events:
        MANIFEST.write_text(json.dumps(man, indent=1) + '\n'); print(f'updated {MANIFEST.relative_to(ROOT)}')
        failures = [f for f in failures if 'overrun probe' in f]
    if failures:
        sys.exit('FAILED expectations:\n  ' + '\n  '.join(failures))
    if not a.only and not a.strict32:
        doc = dict(schema=SCHEMA, manifest_sha256=sha(MANIFEST.read_bytes()),
                   runner_contract_sha256=sha(build.M4B.contract.read_bytes()),
                   variants=VARIANTS, variants_identical=True, asan_options='detect_leaks=0 (leak detection disabled)',
                   scripts=results)
        body = json.dumps(doc, indent=1)
        if a.check:
            old = json.loads(RESULTS.read_text()) if RESULTS.exists() else {}
            old.pop('timing', None)
            if json.dumps(old, indent=1) != body: sys.exit(f'{RESULTS.relative_to(ROOT)} differs from this run')
            print(f'{RESULTS.relative_to(ROOT)} matches ({len(results)} scripts)')
        else:
            doc['timing'] = dict(note='us/tic of G_Ticker per variant from the runner summary and the wall time of this '
                                      'run; measured on a machine that was not idle (informational, not compared by --check)',
                                 wall_seconds=dict(build=round(built - start, 1), runs=round(runs, 1)), us_per_tic=timing)
            RESULTS.write_text(json.dumps(doc, indent=1) + '\n'); print(f'wrote {RESULTS.relative_to(ROOT)}')
    if a.selftest is not None:
        st = man['selftest']
        w = next(x for x in man['wads'] if x['name'] == st['wad'])
        s = next(x for x in w['scripts'] if x['lump'] == st['script'])
        ref.WAD = ROOT / w['file']
        lmp, t = ROOT / s['lmp'], a.selftest
        plain = work / 'runs' / 'selftest_plain'; ref.run(binaries['strict'], lmp, plain)
        drawn = work / 'runs' / 'selftest_draw'; ref.run(binaries['strict'], lmp, drawn, ['--selftest-extra-draw', str(t)])
        data = bytearray(lmp.read_bytes()); at = 13 + 4 * t
        if at >= len(data) - 1: sys.exit('selftest TIC beyond the demo')
        data[at] = (data[at] + 25) & 0xff if data[at] not in range(103, 128) else (data[at] - 25) & 0xff
        changed = work / (s['lump'][:7] + 'C.lmp'); changed.write_bytes(data)
        moved = work / 'runs' / 'selftest_cmd'; ref.run(binaries['strict'], changed, moved)
        draw = dump.first_difference(str(plain), str(drawn)); cmd = dump.first_difference(str(plain), str(moved))
        ok = draw == f'tic {t}: row kind H #0 column prndindex' and cmd is not None and cmd.startswith(f'tic {t}:')
        print(f"selftest on {s['lump']}: extra draw {draw!r}; ticcmd {cmd!r}: " + ('pass' if ok else 'FAIL'))
        for x in (plain, drawn, moved): shutil.rmtree(x, ignore_errors=True)
        changed.unlink(missing_ok=True)
        if not ok: sys.exit(1)


if __name__ == '__main__':
    main()
