#!/usr/bin/env python3
"""Synthetic sound rooms for M5c (docs/M5C_REFERENCE_INVESTIGATION.md section 3.1.11).

Writes one small, fully synthetic IWAD per room under tests/reference/m5c_wads/, each holding one map in the
E1M1 slot, and the demo scripts under tests/reference/m5c_scripts/ (text and .lmp, the
tools/reference/m4b_scripts.py format, imported read-only). Every room is run by the genuine engine with
tests/reference/m5b_gfx.wad, tests/reference/m5c_sfx.wad and tests/reference/m5c_music.wad loaded after it
(tools/reference/m5c_reference.py). The room WADs carry the M5a texture and flat directory (m5a_wad.py's
framing and room class, imported read-only), so they also load alone, as the wire-24 simulation
loads them. Only things, specials, skills and commands the wire-24 simulation admits are used
(docs/COMPATIBILITY.md): the player start, zombiemen, shotgun guys, imps and a barrel, Episode 1 pickups, the
wire-22 line special 1 and the part-1 specials 2, 5, 26 and 62, skill 2 and the command domain of the M4b scripts; every
script is compared with the Rust simulation tic for tic by tools/reference/m5c_rust_check.py.

Between the existing synthetic items (whose coverage is measured by m5c_reference.py) and these rooms, every
sound the Episode 1 simulation can start is started and every branch of the sound logic level play can reach
is taken (the coverage manifest tests/reference/m5c_coverage.json is the measured record):

    snddist   a stationary listener at (0,0) facing east and 18 zombiemen at fixed offsets, which see it and wake
              within the first 20 tics (A_Look's see sound from the spawn position, p_enemy.c:633-661; the spawn
              state's tics randomised by P_SpawnMapThing, p_mobj.c:787-788): approx_dist (s_sound.c:766-771)
              exactly at S_CLIPPING_DIST, half a unit below and above it (the smallest step two integer positions give: adx + ady - min/2), exactly at
              S_CLOSE_DIST, half a unit below and above it, the volume steps 1 to 7 (8 at close range), sources
              due north and due south (the separation extremes 33 and 224), due east (R_PointToAngle2 equal to
              the listener's angle, s_sound.c:785-788); a start refused for want of a channel (posact, 120)
    sndbusy   a blue locked door used twice without the key at the start: sfx_oof with a NULL origin replacing a
              NULL-origin channel (S_StopSound (NULL), s_sound.c:349, 471-484; p_doors.c:228-253); nine remote
              doors on one tag (W1 2) opened at once: nine positional starts in one tic (more than the mixer's
              eight channels within four tics, for the mixer harness), three
              free channels then six evictions (equal priority 100 >= 100, s_sound.c:853-865); a W1 5 floor
              whose stnmov (priority 119) finds all three channels holding priority 100 and is refused
              (S_getChannel returns -1, s_sound.c:857-860); the pistol (64) evicting a door
    sndmonst  the use key against a plain wall (sfx_noway, p_map.c:1099-1105), a DR door (doropn, dorcls), the fist on
              a barrel (sfx_punch: a hit on any shootable thing, p_pspr.c:483-489), then the rocket launcher against
              zombiemen, shotgun guys and imps beyond the door: the random see and death variants (posit1-3,
              bgsit1-2, podth1-3, bgdth1-2: p_enemy.c:633-661, 1535-1569), popain
              and the active sounds (A_Chase, p_enemy.c:771-775)
    sndbtn    two SR 62 lift switches near the map's origin and one far from it, and a DR 1 door: the switch press
              with button slot 0 free (buttonlist->soundorg is NULL, p_switch.c:228) and busy (the slot-0
              sector's soundorg); the button release (p_spec.c:1151 passes &buttonlist[i].soundorg itself, whose
              x and y S_AdjustSoundParams reads from the bytes after the pointer field: map (0,0) plus a few
              units, section 3.1.11 item 6) with the listener in and out of earshot of (0,0); pstart and pstop
              (p_plats.c:115-126, 230); doropn and dorcls (p_doors.c:80-84). Two scripts: SNDBTN1 with the next button slot
              free at every release; SNDBTN2 (clearly named: the layout-dependent case of decision T12) with
              slot 1 busy when slot 0 is released, so the release reads slot 1's fields (LP64: btimer and
              padding; the 32-bit original: btexture and btimer). Slot 15 is never used.

    m5c_wad.py [--only NAME]       write the WADs and the scripts' .lmp files
    m5c_wad.py --list              name, file, sectors, lines, things, scripts
    m5c_wad.py --manifest          also refresh tests/reference/m5c_manifest.json
    m5c_wad.py --check             regenerate in memory and compare with the committed WADs, scripts and manifest
                                   (writes nothing)
"""
import argparse, hashlib, json, struct, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m4b_wad as B              # noqa: E402  frozen M4b special set and spawn counts (read-only)
import m4a_wad as A              # noqa: E402  frozen M4a lump order (read-only)
import m4b_scripts as S          # noqa: E402  frozen script format (read-only)
import m5a_wad as R              # noqa: E402  the M5a builder, styles and framing (read-only)
import m5a_gfx as X              # noqa: E402  the M5a name helper (read-only)

OUT_DIR = ROOT / 'tests/reference/m5c_wads'
SCRIPT_DIR = ROOT / 'tests/reference/m5c_scripts'
MANIFEST = ROOT / 'tests/reference/m5c_manifest.json'
MANIFEST_SCHEMA = 'DOOM-M5C-MANIFEST-v1'
EAST, NORTH, WEST, SOUTH = 0, 90, 180, 270
PLAYER1, ZOMBIE, SHOTGUY, IMP, BARREL = 1, 3004, 9, 3001, 2035
LAUNCHER, ROCKETBOX, SOULSPHERE = 2003, 2046, 2013
ALL, AMBUSH = 7, 8
DR, W1_OPEN, W1_RAISE, LOCKED_BLUE, SR_LIFT = 1, 2, 5, 26, 62
ML_DONTPEGBOTTOM = 16
VIEWS = [10, 11]
WIRE22 = {1, 11, 36, 38, 48, 63, 88, 109}                    # docs/COMPATIBILITY.md: wire 22's eight, kept by wires 23 and 24


class Room(R.Room):
    """The M5a room with the wire-24 line special set (wire 22's eight, M4a's part-1 set and M4b's 97)."""

    def special(self, front, back, at, special, tag=0):
        assert special in B.SPECIALS | WIRE22, special
        self.specials.append((front, back, at, special, tag))

# SNDDIST: (dx, dy) of each zombieman from the listener at (0,0) and what it is there for. approx_dist =
# adx + ady - (min(adx, ady) >> 1) in fixed point (s_sound.c:766-771); the volume is 8 * ((1200 - d) >> 16) / 1040
# (S_ATTENUATOR = 1200 - 160, s_sound.c:62-66, 808-814).
DIST = [
    ((1200, 0), 'clip exact, due east: the angle equals the listener angle; volume 0'),
    ((1, 1199), 'half a unit below the clipping distance: volume 0'),
    ((-1200, -1), 'half a unit above the clipping distance: clipped'),
    ((-1500, 300), 'far beyond the clipping distance'),
    ((0, 160), 'close distance exactly, due north: volume 8, separation extreme'),
    ((0, -160), 'close distance exactly, due south: volume 8, the other extreme'),
    ((-159, 1), 'half a unit below the close distance: volume 8 by the close branch'),
    ((160, 1), 'half a unit above the close distance: volume 7'),
    ((-250, 0), 'volume 7'), ((0, 380), 'volume 6'), ((0, -510), 'volume 5'), ((640, 0), 'volume 4'),
    ((-770, 0), 'volume 3'), ((0, 900), 'volume 2'), ((0, -1030), 'volume 1'), ((1100, 0), 'volume 0'),
    ((300, 300), 'diagonal: volume 5'), ((-400, 400), 'diagonal: volume 4'),
]


def snddist():
    p = Room('snddist', wall='M5WALLA')
    p.sector('HALL', [(-1536, -1280, 1280, 1280)], ceil=256, light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.thing(0, 0, EAST, PLAYER1)
    for (dx, dy), _what in DIST: p.thing(dx, dy, WEST if dx > 0 else EAST, ZOMBIE)
    return p


def door_row(p, hall, x0, n, tag, y=128):
    """n remote doors (ceil 0, tagged) on the north wall of `hall` from x0, each with a closet behind it."""
    doors = []
    for k in range(n):
        x = x0 + 96 * k
        d = p.sector(f'DOOR{tag}_{k}', [(x, y, x + 64, y + 16)], ceil=0, light=176, tag=tag, door=True)
        p.sector(f'CLOSET{tag}_{k}', [(x, y + 16, x + 64, y + 96)], light=144, fpic='M5FLAT3', cpic='M5FLAT4')
        p.style(d, None, flags=ML_DONTPEGBOTTOM, s0=dict(mid='DOORTRAK'))
        doors.append(d)
    return doors


def sndbusy():
    p = Room('sndbusy', wall='M5WALLB')
    hall = p.sector('HALL', [(-128, -128, 1024, 128)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    door_row(p, hall, 0, 9, 1)
    strip = p.sector('STRIP1', [(64, -128, 80, 0)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.special(hall, strip, (80, -64), W1_OPEN, 1)
    p.sector('PAD', [(448, -224, 576, -128)], light=176, tag=2, fpic='M5FLAT5', cpic='M5FLAT2')
    strip2 = p.sector('STRIP2', [(192, -128, 208, 0)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.special(hall, strip2, (208, -64), W1_RAISE, 2)
    lock = p.sector('LOCK', [(-96, -144, -32, -128)], ceil=0, light=176, door=True)
    p.sector('VAULT', [(-96, -224, -32, -144)], light=144, fpic='M5FLAT3', cpic='M5FLAT4')
    p.special(hall, lock, (-64, -128), LOCKED_BLUE)
    p.style(lock, None, flags=ML_DONTPEGBOTTOM, s0=dict(mid='DOORTRAK'))
    p.thing(-64, -100, SOUTH, PLAYER1)
    return p


def sndmonst():
    p = Room('sndmonst', wall='M5WALLC')
    west = p.sector('WEST', [(-128, -128, 256, 128)], ceil=128, light=176, fpic='M5FLAT4', cpic='M5FLAT2')
    door = p.sector('DOOR', [(256, -64, 272, 64)], ceil=0, light=176, door=True)
    p.sector('EAST', [(272, -384, 1152, 384)], ceil=128, light=176, fpic='M5FLAT5', cpic='M5FLAT2')
    p.special(west, door, (256, 0), DR)
    p.style(door, None, flags=ML_DONTPEGBOTTOM, s0=dict(mid='DOORTRAK'))
    p.thing(-96, 0, WEST, PLAYER1)                     # 32 units from the west wall: the use key finds no line special
    p.thing(0, 0, EAST, SOULSPHERE)
    p.thing(64, 0, EAST, LAUNCHER)
    p.thing(128, 0, EAST, ROCKETBOX)
    p.thing(232, 60, EAST, BARREL)                     # the fist's target, north of the place where the door is used
    for k, (x, y) in enumerate(((560, -240), (560, 0), (560, 240), (720, -120), (720, 120), (880, -240), (880, 240),
                                (880, 0), (1040, -200), (1040, 200), (1040, 0), (720, 300))):
        p.thing(x, y, WEST, ZOMBIE if k % 3 else SHOTGUY)
    for x, y in ((800, -320), (800, 320), (960, -100), (960, 100), (1100, -300), (1100, 300)):
        p.thing(x, y, WEST, IMP)
    return p


def sndbtn():
    p = Room('sndbtn', wall='M5WALLE')
    hall = p.sector('HALL', [(-128, -128, 2048, 128)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    switches = {}
    for name, x, tag in (('A', 384, 1), ('C', 480, 2), ('B', 1792, 3)):
        al = p.sector(f'ALCOVE{name}', [(x, 128, x + 64, 144)], light=176, fpic='M5FLAT3', cpic='M5FLAT2')
        p.special(al, None, (x + 32, 144), SR_LIFT, tag)
        p.style(al, None, at=(x + 32, 144), s0=dict(mid='SW1COMM'))
        p.sector(f'LIFT{name}', [(x, -192, x + 64, -128)], floor=48, light=176, tag=tag, fpic='M5FLAT5', cpic='M5FLAT2')
        switches[name] = al
    door = p.sector('DOOR', [(1920, -144, 1984, -128)], ceil=0, light=176, door=True)
    p.sector('ROOM', [(1920, -256, 1984, -144)], light=144, fpic='M5FLAT3', cpic='M5FLAT4')
    p.special(hall, door, (1952, -128), DR)
    p.style(door, None, flags=ML_DONTPEGBOTTOM, s0=dict(mid='DOORTRAK'))
    p.thing(-96, 0, EAST, PLAYER1)
    return p


ROOMS = {'snddist': snddist, 'sndbusy': sndbusy, 'sndmonst': sndmonst, 'sndbtn': sndbtn}

# Scripts (tools/reference/m4b_scripts.py format; turn is the angleturn high byte, 256 per full turn). The tics were
# chosen by running the genuine engine.
SCRIPTS = {
    'snddist': [('SNDDIST', 'a stationary listener and 18 zombiemen at fixed distances that wake and call out',
                 ['idle 48'])],
    'sndbusy': [('SNDBUSY', 'a locked door used twice, nine doors at once, refused floor sounds and a pistol eviction',
                 ['idle 2', 'cmd 1 0 0 0 USE', 'idle 6', 'cmd 1 0 0 0 USE', 'idle 3', 'cmd 1 0 0 64 0', 'cmd 40 25 0 0 0', 'idle 10', 'cmd 1 0 0 0 ATTACK', 'idle 60'])],
    'sndmonst': [('SNDMONST', 'the use key against a wall, a door, the fist on a barrel, then rockets against zombiemen, shotgun guys and imps',
                  ['idle 2', 'cmd 1 0 0 0 USE', 'idle 3', 'cmd 1 0 0 128 0', 'cmd 40 25 0 0 0', 'idle 10', 'cmd 1 0 0 0 USE',
                   'cmd 1 0 0 64 0', 'cmd 1 0 0 0 CHANGE|W0', 'idle 30', 'cmd 40 0 0 0 ATTACK', 'cmd 1 0 0 -64 0', 'cmd 1 0 0 0 CHANGE|W4',
                   'idle 60', 'cmd 200 0 0 0 ATTACK', 'idle 60'])],
    'sndbtn': [('SNDBTN1', 'a lift switch near the origin and one far from it, each released with the next button slot free, and a door',
                ['cmd 61 25 0 0 0', 'idle 10', 'cmd 1 0 0 64 0', 'cmd 16 25 0 0 0', 'idle 6', 'cmd 1 0 0 0 USE', 'cmd 8 -25 0 0 0', 'idle 40',
                 'cmd 1 0 0 -64 0', 'cmd 170 25 0 0 0', 'idle 10', 'cmd 1 0 0 64 0', 'cmd 16 25 0 0 0', 'idle 6', 'cmd 1 0 0 0 USE',
                 'cmd 8 -25 0 0 0', 'idle 40', 'cmd 1 0 0 -64 0', 'cmd 12 25 0 0 0', 'idle 10', 'cmd 1 0 0 -64 0', 'cmd 34 25 0 0 0', 'idle 6',
                 'cmd 1 0 0 0 USE', 'idle 260']),
               ('SNDBTN2', 'two lift switches pressed within one button time: slot 0 released while slot 1 is busy (decision T12)',
                ['cmd 61 25 0 0 0', 'idle 10', 'cmd 1 0 0 64 0', 'cmd 16 25 0 0 0', 'idle 6', 'cmd 1 0 0 0 USE', 'cmd 6 -25 0 0 0', 'idle 2',
                 'cmd 12 0 25 0 0', 'idle 4', 'cmd 6 25 0 0 0', 'idle 2', 'cmd 1 0 0 0 USE', 'idle 120'])],
}


def wad_bytes(name, slot='E1M1'):
    room = ROOMS[name]()
    m = R.build(room)
    R.apply_styles(room, m)
    lumps = dict(m['lumps'])
    lumps['THINGS'] = b''.join(struct.pack('<hhhhh', *t) for t in room.things)
    assert list(lumps) == ['MAP01'] + A.MAP_LUMPS, list(lumps)
    wad = R.framing() + [(slot, b'')] + [(n, lumps[n]) for n in A.MAP_LUMPS]
    data = bytearray(b'IWAD' + struct.pack('<ii', len(wad), 0))
    directory = b''
    for n, b in wad:
        directory += struct.pack('<ii8s', len(data), len(b), X.name8(n))
        data += b
    struct.pack_into('<i', data, 8, len(data))
    return bytes(data + directory), room, m


def script_text(lump, what, lines):
    return f'# M5c script {lump}: {what}.\n# Generated by tools/reference/m5c_wad.py.\n' + '\n'.join(lines) + '\n'


def fields(name, data, room, m):
    scripts = []
    for lump, what, lines in SCRIPTS[name]:
        lmp, tics = S.lump(script_text(lump, what, lines))
        scripts.append((dict(lump=lump, txt=f'tests/reference/m5c_scripts/{lump}.txt', lmp=f'tests/reference/m5c_scripts/{lump}.lmp',
                             what=what, tics=tics, lmp_sha256=hashlib.sha256(lmp).hexdigest(), views=VIEWS),
                        script_text(lump, what, lines), lmp))
    room_fields = dict(name=name.upper(), file=f'tests/reference/m5c_wads/{name}.wad', slot='E1M1',
                       sha256=hashlib.sha256(data).hexdigest(), bytes=len(data), sectors=len(room.sectors), lines=len(m['lines']),
                       things={str(k): v for k, v in B.spawned(room, 2).items()})
    return room_fields, scripts


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--only', choices=list(ROOMS))
    ap.add_argument('--list', action='store_true')
    ap.add_argument('--manifest', action='store_true')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--out', type=Path, help='write the WADs and scripts under this directory instead (authoring)')
    a = ap.parse_args()
    names = [n for n in ROOMS if not a.only or n == a.only]
    man = json.loads(MANIFEST.read_text()) if MANIFEST.exists() else dict(schema=MANIFEST_SCHEMA, rooms=[])
    if man.get('schema') != MANIFEST_SCHEMA: sys.exit('manifest schema')
    by = {r['name']: r for r in man['rooms']}
    bad, new_rooms = [], []
    base = a.out.resolve() if a.out else ROOT
    for n in names:
        data, room, m = wad_bytes(n)
        rf, scripts = fields(n, data, room, m)
        entry = dict(rf, scripts=[sf for sf, _t, _l in scripts])
        new_rooms.append(entry)
        if a.list:
            print(f"{rf['name']}: {rf['file']}, {rf['bytes']} bytes, sectors {rf['sectors']}, lines {rf['lines']}, "
                  f"things {sum(rf['things'].values())}, scripts " + ', '.join(f"{sf['lump']} {sf['tics']} tics" for sf, _t, _l in scripts))
            continue
        if a.check:
            files = [(ROOT / rf['file'], data)] + [(ROOT / sf[k], b) for sf, t, l in scripts for k, b in (('txt', t.encode()), ('lmp', l))]
            for path, want in files:
                if not path.exists() or path.read_bytes() != want: bad.append(str(path.relative_to(ROOT)))
            if by.get(rf['name']) != entry: bad.append(f"manifest: {rf['name']}")
            continue
        for d in ('tests/reference/m5c_wads', 'tests/reference/m5c_scripts'): (base / d).mkdir(parents=True, exist_ok=True)
        (base / rf['file']).write_bytes(data)
        for sf, t, l in scripts:
            (base / sf['txt']).write_text(t)
            (base / sf['lmp']).write_bytes(l)
        print(f"wrote {base / rf['file']} ({len(data)} bytes), " + ', '.join(f"{sf['lmp']} ({sf['tics']} tics)" for sf, _t, _l in scripts))
        by[rf['name']] = entry
    if a.check:
        if not a.only and [r['name'] for r in man['rooms']] != [n.upper() for n in ROOMS]: bad.append('manifest: room list')
        if bad: sys.exit('differs or missing: ' + ', '.join(bad))
        print(f'{MANIFEST.relative_to(ROOT)} and {len(names)} rooms match')
    elif a.manifest and not a.list:
        man['rooms'] = [by[n.upper()] for n in ROOMS if n.upper() in by]
        MANIFEST.write_text(json.dumps(man, indent=1) + '\n')
        print(f'updated {MANIFEST.relative_to(ROOT)}')


if __name__ == '__main__':
    main()
