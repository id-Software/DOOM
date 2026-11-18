#!/usr/bin/env python3
"""Synthetic status bar, HUD and palette rooms for M5b (decision S23; docs/M5B_REFERENCE_INVESTIGATION.md
sections 2.4.3 and 2.4.4).

Writes one small, fully synthetic IWAD per room under tests/reference/m5b_wads/, each holding one map
in the E1M1 slot, and the demo scripts under tests/reference/m5b_scripts/ (text and .lmp, the
tools/reference/m4b_scripts.py format, imported read-only). Every room is run by the genuine engine
with tests/reference/m5b_gfx.wad loaded after it (`--wad ROOM.wad --wad m5b_gfx.wad --wad SCRIPT.lmp`).
The room WADs carry the M5a texture and flat directory (tools/reference/m5a_wad.py's framing, imported
read-only with its builder chain), so they also load alone. Only things and specials the wire-24
simulation admits are used (docs/COMPATIBILITY.md): the Episode 1 pickups, monsters and barrels, the
line specials of the M4b set, and the admitted sector specials 5 (10 damage), 7 (5 damage) and 16 (20
damage) every 32 tics (P_PlayerInSpecialSector, p_spec.c:1030-1075).

The facts each room relies on (pain level 5*(100-min(health,100))/101, st_stuff.c:729-745; the face
priorities, st_stuff.c:752-922; the palettes, st_stuff.c:1000-1050; the messages, p_inter.c:350-660,
p_doors.c:370-410, g_game.c:671-679, and HU_Ticker's 140-tic timeout, hu_stuff.c:505-530):

    stdigit   the ready-ammo number through three, two and one digits to 0 and the fist's blank (1994,
              st_stuff.c:933-936, st_lib.c:121-123); health from 100 down to one digit on two damage floors
              and back up past 100; armor 0, 1, 100 and 200
    starms    the shotgun, chaingun, rocket launcher and chainsaw (the arms icons and the evil grin at pain
              level 0), shells, rockets, the backpack (all four maximum-ammo numbers doubled) and the three
              keycards (the key boxes)
    faceevil  the evil grin (a weapon newly owned while bonuscount is non-zero, st_stuff.c:775-799) at pain
              levels 1, 2, 3 and 4, with a damage floor between the pickups
    faceouch  the ouch face, which the source shows on a health gain above 20 (health - st_oldhealth >
              ST_MUCHPAIN) while damagecount is non-zero (st_stuff.c:845-851), at pain levels 0, 1, 2 and 3
              (pain level 4 needs health <= 19 after a gain above 20: unreachable); the own-damage face at
              every pain level; the rampage (fire held 70 tics, st_stuff.c:862-877); death and the dead face
    facelook  the look-around (st_randomnumber % 3 every 17 tics, st_stuff.c:913-917) standing still at each
              pain level 0 to 4
    facedir   hit by imp fireballs while turning: the turn-right, turn-left and head-on faces by the
              attacker's angle (st_stuff.c:804-842) at every pain level, then death
    palred    a barrel shot point-blank after a soulsphere: damagecount near 100, the red palettes 8 down to 2
              as it decays
    palbonus  one pickup, three stacked health bonuses (bonuscount 18) and a keycard with an armor bonus on
              the same spot, touched in both orders (P_GiveCard sets bonuscount to 6 before the pickup adds 6,
              p_inter.c:274-283, 658, so the card touched last gives 12 and first gives 18): palettes 10-12
    palsuit   the radiation suit worn to its end: palette 13 while powers[pw_ironfeet] > 128 or bit 3 is set
              (st_stuff.c:1040-1043), the blink of its last 128 tics
    message   every reachable message kind: the Episode 1 pickups, the three locked doors (p_doors.c:370-410),
              the turbo message (forwardmove > 50 on a gametic that is a multiple of 128, g_game.c:671-679),
              a repeat and a replacement within 140 tics and a timeout
    cheats    the bespoke scenario of decision S1 (BESPOKE below): the berserk term of the palette and the god
              face, reached only through the cheats idbeholds and iddqd, which wire 24 does not admit

    m5b_wad.py [--only NAME]       write the WADs and the scripts' .lmp files
    m5b_wad.py --list              name, file, sectors, lines, things, script tics
    m5b_wad.py --manifest          also refresh the manifest's generated fields
    m5b_wad.py --check             regenerate in memory and compare with the committed WADs, .lmp files and
                                   the manifest's generated fields
"""
import argparse, hashlib, json, struct, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m4b_wad as B              # noqa: E402  frozen M4b painter and special set (read-only)
import m4a_wad as A              # noqa: E402  frozen M4a lump order (read-only)
import m4b_scripts as S          # noqa: E402  frozen script format (read-only)
import m5a_wad as R              # noqa: E402  the M5a room builder, styles and framing (read-only)
import m5a_gfx as X              # noqa: E402  the M5a name helper (read-only)

OUT_DIR = ROOT / 'tests/reference/m5b_wads'
SCRIPT_DIR = ROOT / 'tests/reference/m5b_scripts'
MANIFEST = ROOT / 'tests/reference/m5b_manifest.json'
MANIFEST_SCHEMA = 'DOOM-M5B-MANIFEST-v1'
EAST, NORTH, WEST, SOUTH = 0, 90, 180, 270
PLAYER1, IMP, BARREL = 1, 3001, 2035
SHOTGUN, CHAINGUN, LAUNCHER, CHAINSAW = 2001, 2002, 2003, 2005
CLIP, CLIPBOX, SHELLS, SHELLBOX, ROCKET, ROCKETBOX, BACKPACK = 2007, 2048, 2008, 2049, 2010, 2046, 8
STIMPACK, MEDIKIT, HEALTHBONUS, ARMORBONUS, GREENARMOR, BLUEARMOR, SOULSPHERE = 2011, 2012, 2014, 2015, 2018, 2019, 2013
INVIS, SUIT, ALLMAP, LIGHTAMP, LAMP = 2024, 2025, 2026, 2045, 2028
BLUECARD, YELLOWCARD, REDCARD = 5, 6, 13
LOCKED_BLUE, LOCKED_YELLOW, LOCKED_RED = 26, 27, 28          # DR locked doors (p_doors.c:370-410)
HURT10, HURT5, HURT20 = 5, 7, 16                             # sector specials (p_spec.c:1030-1075)
VIEWS = [10, 11]                                             # decision S14


class Room(R.Room):
    """The M5a room (m4b_wad.Plan with line styles) with the M4b line special set."""

    def special(self, front, back, at, special, tag=0):
        assert special in B.SPECIALS, special
        self.specials.append((front, back, at, special, tag))

    def hurt(self, label, rect, special, light=160):
        return self.sector(label, [rect], light=light, special=special, fpic='NUKAGE1', cpic='M5FLAT2')


# ---------------------------------------------------------------- the rooms ----------------------
def stdigit():
    p = Room('stdigit', wall='M5WALLA')
    p.sector('START', [(-128, -64, 64, 64)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.hurt('HURT20', (64, -64, 192, 64), HURT20)
    p.hurt('HURT5', (192, -64, 320, 64), HURT5)
    p.sector('ITEMS', [(320, -64, 1280, 64)], light=176, fpic='M5FLAT3', cpic='M5FLAT2')
    p.thing(-96, 0, EAST, PLAYER1)
    for x, kind in ((384, HEALTHBONUS), (448, STIMPACK), (512, MEDIKIT), (576, SOULSPHERE), (640, HEALTHBONUS),
                    (704, ARMORBONUS), (768, GREENARMOR), (832, BLUEARMOR), (896, CLIPBOX), (960, CHAINGUN)):
        p.thing(x, 0, EAST, kind)
    return p


def starms():
    p = Room('starms', wall='M5WALLB')
    p.sector('HALL', [(-128, -64, 1152, 64)], light=176, fpic='M5FLAT4', cpic='M5FLAT2')
    p.thing(-96, 0, EAST, PLAYER1)
    for x, kind in ((0, SHOTGUN), (64, SHELLS), (128, CHAINGUN), (192, LAUNCHER), (256, ROCKET), (320, CHAINSAW),
                    (448, BACKPACK), (512, SHELLBOX), (576, ROCKETBOX), (640, BLUECARD), (768, YELLOWCARD), (896, REDCARD)):
        p.thing(x, 0, EAST, kind)
    return p


def faceevil():
    p = Room('faceevil', wall='M5WALLC')
    p.sector('START', [(-128, -64, 0, 64)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    x = 0
    for k, (special, weapon) in enumerate(((HURT20, SHOTGUN), (HURT20, CHAINGUN), (HURT20, LAUNCHER), (HURT10, CHAINSAW))):
        p.hurt(f'HURT{k}', (x, -64, x + 128, 64), special)
        p.sector(f'SAFE{k}', [(x + 128, -64, x + 256, 64)], light=176, fpic='M5FLAT3', cpic='M5FLAT2')
        p.thing(x + 192, 0, EAST, weapon)
        x += 256
    p.thing(-96, 0, EAST, PLAYER1)
    return p


def faceouch():
    p = Room('faceouch', wall='M5WALLE')
    p.sector('START', [(-128, -64, 0, 64)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    x = 0
    for k, special in enumerate((HURT20, HURT20, HURT20, HURT10)):
        p.hurt(f'HURT{k}', (x, -64, x + 128, 64), special)
        p.sector(f'SAFE{k}', [(x + 128, -64, x + 256, 64)], light=176, fpic='M5FLAT3', cpic='M5FLAT2')
        p.thing(x + 160, 0, EAST, MEDIKIT)
        x += 256
    p.hurt('LAST', (x, -64, x + 256, 64), HURT20)
    p.thing(-96, 0, EAST, PLAYER1)
    return p


def facelook():
    p = Room('facelook', wall='M5WALLF')
    p.sector('START', [(-128, -64, 0, 64)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    x = 0
    for k, special in enumerate((HURT20, HURT20, HURT20, HURT10)):
        p.hurt(f'HURT{k}', (x, -64, x + 128, 64), special)
        p.sector(f'SAFE{k}', [(x + 128, -64, x + 256, 64)], light=176, fpic='M5FLAT3', cpic='M5FLAT2')
        x += 256
    p.thing(-96, 0, EAST, PLAYER1)
    return p


def facedir():
    p = Room('facedir', wall='M5WALLA')
    p.sector('ARENA', [(-384, -256, 384, 512)], light=176, fpic='M5FLAT5', cpic='M5FLAT6')
    p.thing(0, 0, NORTH, PLAYER1)
    p.thing(0, -96, NORTH, GREENARMOR)
    for x, y, a in ((-256, 384, 300), (0, 448, 270), (256, 384, 240)):
        p.thing(x, y, a, IMP)                        # imp fireballs never hurt another imp (PIT_CheckThing, p_map.c:298-312)
    return p


def palred():
    p = Room('palred', wall='M5WALLB')
    p.sector('ROOM', [(-128, -128, 256, 128)], light=176, fpic='M5FLAT7', cpic='M5FLAT8')
    p.thing(112, 0, EAST, PLAYER1)
    p.thing(176, 0, EAST, SOULSPHERE)
    p.thing(245, 0, EAST, BARREL)                    # against the east wall: the shots cannot push it away
    return p


def palbonus():
    p = Room('palbonus', wall='M5WALLC')
    p.sector('HALL', [(-128, -64, 768, 64)], light=176, fpic='M5FLAT4', cpic='M5FLAT5')
    p.thing(-96, 0, EAST, PLAYER1)
    p.thing(64, 0, EAST, CLIP)
    for _ in range(3): p.thing(320, 0, EAST, HEALTHBONUS)
    p.thing(576, 0, EAST, BLUECARD)                  # the later thing is linked first (P_SetThingPosition) and touched first:
    p.thing(576, 0, EAST, ARMORBONUS)                #   armor bonus 6, then P_GiveCard resets to 6 and adds 6: 12
    p.thing(704, 0, EAST, ARMORBONUS)
    p.thing(704, 0, EAST, YELLOWCARD)                # keycard first (6 + 6), then the armor bonus: 18
    return p


def palsuit():
    p = Room('palsuit', wall='M5WALLE')
    p.sector('ROOM', [(-128, -128, 256, 128)], light=176, fpic='M5FLAT6', cpic='M5FLAT7')
    p.thing(-96, 0, EAST, PLAYER1)
    p.thing(0, 0, EAST, SUIT)
    return p


def message():
    p = Room('message', wall='M5WALLF')
    hall = p.sector('HALLW', [(-128, -64, 448, 48)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.hurt('HURT', (448, -64, 576, 48), HURT20)
    p.sector('HALLE', [(576, -64, 2688, 48)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    for k, (x, sp) in enumerate(((128, LOCKED_BLUE), (256, LOCKED_YELLOW), (384, LOCKED_RED))):
        d = p.sector(f'DOOR{k}', [(x, 48, x + 64, 64)], ceil=0, light=176, door=True)
        p.sector(f'CLOSET{k}', [(x, 64, x + 64, 128)], light=144, fpic='M5FLAT3', cpic='M5FLAT4')
        p.special(hall, d, (x + 32, 48), sp)
    p.thing(-96, 0, EAST, PLAYER1)
    items = [(640, CLIP), (704, CLIP), (832, SHELLS), (1152, STIMPACK), (1216, MEDIKIT), (1280, HEALTHBONUS),
             (1344, ARMORBONUS), (1408, GREENARMOR), (1472, BLUEARMOR), (1536, SOULSPHERE), (1600, CLIPBOX),
             (1664, SHELLBOX), (1728, ROCKET), (1792, ROCKETBOX), (1856, BACKPACK), (1920, SHOTGUN),
             (1984, CHAINGUN), (2048, LAUNCHER), (2112, CHAINSAW), (2176, BLUECARD), (2240, YELLOWCARD),
             (2304, REDCARD), (2368, INVIS), (2432, SUIT), (2496, ALLMAP), (2560, LIGHTAMP)]
    for x, kind in items: p.thing(x, 0, EAST, kind)
    return p


def cheats():
    p = Room('cheats', wall='M5WALLA')
    p.sector('ROOM', [(-128, -128, 256, 128)], light=176, fpic='M5FLAT1', cpic='M5FLAT2')
    p.thing(-96, 0, EAST, PLAYER1)
    p.thing(192, 64, WEST, LAMP)
    return p


ROOMS = {'stdigit': stdigit, 'starms': starms, 'faceevil': faceevil, 'faceouch': faceouch, 'facelook': facelook,
         'facedir': facedir, 'palred': palred, 'palbonus': palbonus, 'palsuit': palsuit, 'message': message, 'cheats': cheats}

# Scripts (tools/reference/m4b_scripts.py format; turn is the angleturn high byte, 256 per full turn). The tics were
# chosen by running the genuine engine (damage floors hurt on tics that are multiples of 32, p_spec.c:1037-1072).
SCRIPTS = {
    'stdigit': ('M5STDIG', 'two damage floors down to one-digit health, the items back up, then the chaingun fired dry',
        ['cmd 24 25 0 0 0', 'idle 104', 'cmd 8 50 0 0 0', 'idle 89', 'cmd 92 25 0 0 0', 'idle 10', 'cmd 520 0 0 0 ATTACK', 'idle 30']),
    'starms': ('M5STARM', 'walk over every weapon, the ammunition, the backpack and the three keycards, then select three weapons',
        ['cmd 130 25 0 0 0', 'idle 40', 'cmd 1 0 0 0 CHANGE|W1', 'idle 30', 'cmd 1 0 0 0 CHANGE|W2', 'idle 30', 'cmd 1 0 0 0 CHANGE|W3', 'idle 40']),
    'faceevil': ('M5EVIL', 'a damage floor before each new weapon: the evil grin at pain levels 1 to 4',
        ['cmd 20 25 0 0 0', 'idle 45', 'cmd 14 25 0 0 0', 'idle 30', 'cmd 14 25 0 0 0', 'idle 6', 'cmd 14 25 0 0 0', 'idle 52', 'cmd 18 25 0 0 0', 'idle 4', 'cmd 14 25 0 0 0', 'idle 60', 'cmd 14 25 0 0 0', 'idle 6', 'cmd 22 25 0 0 0', 'idle 80']),
    'faceouch': ('M5OUCH', 'a medikit right after damage at pain levels 0 to 3, the rampage, then death on the last floor',
        ['cmd 20 25 0 0 0', 'idle 45', 'cmd 34 25 0 0 0', 'idle 62', 'cmd 34 25 0 0 0', 'idle 48', 'cmd 27 25 0 0 0', 'idle 147', 'cmd 6 50 0 0 0', 'idle 20', 'cmd 80 0 0 0 ATTACK', 'cmd 16 25 0 0 0', 'idle 120']),
    'facelook': ('M5LOOK', 'stand still at each pain level 0 to 4',
        ['idle 150', 'cmd 20 25 0 0 0', 'idle 60', 'cmd 14 25 0 0 0', 'idle 150', 'cmd 14 25 0 0 0', 'idle 10', 'cmd 14 25 0 0 0', 'idle 150', 'cmd 14 25 0 0 0', 'idle 10', 'cmd 14 25 0 0 0', 'idle 150', 'cmd 14 25 0 0 0', 'idle 10', 'cmd 22 25 0 0 0', 'idle 150']),
    'facedir': ('M5FACED', 'step back onto the armor, then turn while three imps throw fireballs, until dead',
        ['cmd 8 -25 0 0 0', 'cmd 480 0 0 24 0', 'idle 60']),
    'palred': ('M5PALRD', 'a soulsphere, then a barrel against the wall shot point-blank',
        ['cmd 5 25 0 0 0', 'idle 10', 'cmd 40 0 0 0 ATTACK', 'idle 120']),
    'palbonus': ('M5PALBN', 'one pickup, three stacked health bonuses, a keycard with an armor bonus in both orders',
        ['cmd 100 25 0 0 0', 'idle 30']),
    'palsuit': ('M5PALST', 'the radiation suit worn to its end',
        ['cmd 8 25 0 0 0', 'idle 2140']),
    'message': ('M5MSG', 'the three locked doors, a timeout, the turbo message, a damage floor, every pickup message and a repeat',
        ['cmd 33 25 0 0 0', 'idle 25', 'cmd 1 0 0 64 0', 'cmd 1 0 0 0 USE', 'idle 20', 'cmd 1 0 0 -64 0', 'cmd 15 25 0 0 0', 'idle 25', 'cmd 1 0 0 64 0', 'cmd 1 0 0 0 USE', 'idle 20', 'cmd 1 0 0 -64 0', 'cmd 15 25 0 0 0', 'idle 25', 'cmd 1 0 0 64 0', 'cmd 1 0 0 0 USE', 'idle 20', 'cmd 1 0 0 -64 0', 'idle 177', 'cmd 1 60 0 0 0', 'cmd 10 25 0 0 0', 'idle 21', 'cmd 240 25 0 0 0', 'idle 40']),
    'cheats': ('M5CHEAT', 'the bespoke scenario (decision S1): stand still while the cheats idbeholds and iddqd are typed',
        ['idle 900']),
}

# The bespoke scenario (decision S1): not reachable through the wire-24 domain. tools/reference/m5b_reference.py runs it
# with --bespoke-keys TIC TEXT (key events through the genuine G_Responder, m5b_driver.c); its dumps are compared with
# no frozen digest. idbeholds gives pw_strength 1, which counts up (p_user.c:339-340), so the berserk term of the
# palette, 12 - (pw_strength >> 6) (st_stuff.c:1008-1016), fades through palettes 3 and 2; iddqd sets CF_GODMODE, the
# god face (st_stuff.c:870-882).
BESPOKE = {'cheats': [(10, 'idbeholds'), (40, 'iddqd')]}

# Authored expectations, evaluated on the strict frame stream of each configuration by
# tools/reference/m5b_reference.py (expression syntax there); the tics are observed (--update-events), reviewed,
# then held fixed in the manifest. PNG lists the (screenblocks, tic) frames committed as images.
MUST_SHOW = {
    'stdigit': [('the own-damage face at pain level 4 on the second floor', 'status face == 39'),
                ('the box of bullets: three-digit ammunition', 'message 23'), ('the chaingun picked up', 'message 32')],
    'starms': [('the evil grin at pain level 0', 'status face == 6'), ('the backpack', 'message 30'), ('the red keycard', 'message 11')],
    'faceevil': [('the evil grin at pain level 1', 'status face == 14'), ('the evil grin at pain level 2', 'status face == 22'),
                 ('the evil grin at pain level 3', 'status face == 30'), ('the evil grin at pain level 4', 'status face == 38')],
    'faceouch': [('ouch at pain level 0', 'status face == 5'), ('ouch at pain level 1', 'status face == 13'),
                 ('ouch at pain level 2', 'status face == 21'), ('ouch at pain level 3', 'status face == 29'),
                 ('the dead face', 'status face == 41')],
    'facelook': [('looking left at pain level 4', 'status face == 32'), ('ahead at pain level 4', 'status face == 33'),
                 ('right at pain level 4', 'status face == 34')],
    'facedir': [('turned right at pain level 4', 'status face == 35'), ('turned left at pain level 4', 'status face == 36'),
                ('the dead face', 'status face == 41')],
    'palred': [('red palette 8', 'status palette == 8'), ('red palette 7', 'status palette == 7'), ('red palette 2', 'status palette == 2')],
    'palbonus': [('bonus palette 10', 'status palette == 10'), ('bonus palette 11', 'status palette == 11'),
                 ('bonus palette 12', 'status palette == 12')],
    'palsuit': [('the radiation palette', 'status palette == 13'), ('the blink: palette 0 while the suit runs', 'status palette == 0 from 1900')],
    'message': [('the blue locked door', 'message 38'), ('the yellow locked door', 'message 39'), ('the red locked door', 'message 40'),
                ('the timeout: no message showing', 'status message_on == 0 from 200'), ('the turbo message', 'message 41'),
                ('the light amplification visor', 'message 20')],
    'cheats': [('the berserk palette', 'status palette == 3'), ('the god face', 'status face == 40')],
}
PNG = {'stdigit': [(10, 248), (10, 860)], 'starms': [(10, 140)], 'faceevil': [(10, 250)], 'faceouch': [(10, 80), (10, 240)],
       'facelook': [(10, 900)], 'facedir': [(10, 300)], 'palred': [(10, 46)], 'palbonus': [(10, 56)], 'palsuit': [(10, 100)],
       'message': [(10, 125), (10, 390), (11, 125)], 'cheats': [(10, 60)]}


def wad_bytes(name):
    room = ROOMS[name]()
    m = R.build(room)
    R.apply_styles(room, m)
    lumps = dict(m['lumps'])
    lumps['THINGS'] = b''.join(struct.pack('<hhhhh', *t) for t in room.things)
    assert list(lumps) == ['MAP01'] + A.MAP_LUMPS, list(lumps)
    wad = R.framing() + [('E1M1', b'')] + [(n, lumps[n]) for n in A.MAP_LUMPS]
    data = bytearray(b'IWAD' + struct.pack('<ii', len(wad), 0))
    directory = b''
    for n, b in wad:
        directory += struct.pack('<ii8s', len(data), len(b), X.name8(n))
        data += b
    struct.pack_into('<i', data, 8, len(data))
    return bytes(data + directory), room, m


def script_text(name):
    lump, what, lines = SCRIPTS[name]
    return f'# M5b room {name}: {what}.\n# Generated by tools/reference/m5b_wad.py.\n' + '\n'.join(lines) + '\n'


def fields(name, data, room, m):
    lump, what, _ = SCRIPTS[name]
    lmp, tics = S.lump(script_text(name))
    return dict(name=name.upper(), file=f'tests/reference/m5b_wads/{name}.wad', slot='E1M1', sha256=hashlib.sha256(data).hexdigest(),
                bytes=len(data), sectors=len(room.sectors), lines=len(m['lines']),
                things={str(k): v for k, v in B.spawned(room, 2).items()}), \
        dict(lump=lump, txt=f'tests/reference/m5b_scripts/{lump}.txt', lmp=f'tests/reference/m5b_scripts/{lump}.lmp',
             what=what, tics=tics, lmp_sha256=hashlib.sha256(lmp).hexdigest(), views=VIEWS), lmp


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
    bad = []
    base = a.out.resolve() if a.out else ROOT
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
        for d in ('tests/reference/m5b_wads', 'tests/reference/m5b_scripts'): (base / d).mkdir(parents=True, exist_ok=True)
        (base / wf['file']).write_bytes(data)
        (base / sf['txt']).write_text(txt)
        (base / sf['lmp']).write_bytes(lmp)
        print(f"wrote {base / wf['file']} ({len(data)} bytes), {sf['lmp']} ({sf['tics']} tics)")
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
