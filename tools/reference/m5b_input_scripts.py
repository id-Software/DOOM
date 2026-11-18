#!/usr/bin/env python3
"""Write the M5b input driver's event scripts (tests/reference/m5b_input/scripts/*.txt).

The scripts are the inputs of the genuine input driver (tools/reference/m5b_input_driver.c, whose
header specifies the directives); their goldens are made by the C only
(tools/reference/m5b_input_reference.py). Owner rulings of docs/M5B_REFERENCE_INVESTIGATION.md:
S3 (demo resolution: the goldens carry the built command at full 16-bit angleturn and the
executed command after the demo round trip, so the scripts include turns with a nonzero low byte
across the keyboard and mouse ranges), S4 (the app posts one ev_mouse per command build with the
accumulated motion << 2, clamped to 319 pixels: the primary scripts are at the event_t level),
S5 (keys the app never forwards, run through the genuine responders to show why), S6 (repeats
dropped, a release in its press's pump deferred, key-ups and a zero mouse event on pause,
minimise and focus loss), S7 (vanilla bindings and the modern preset expressed only through the
genuine configuration values) and S11 (restart is a new game; the level-load clear).

Name prefixes: ev_ event_t level, vanilla bindings; x_ X level through the genuine i_video.c
(evidence of linuxdoom's own layer); s5_ keys the app withholds; s6_ the app's stream choices;
modern_ the S7 preset; round_ and sens_ the turn and recording rounding tables; dclk_ the
double-click boundaries; restart_ and loadlevel_ S11; beyond_app_ event_t values the app never posts (still
defined in C); queue_ the MAXEVENTS ring; chain_ the demo
replayed by the genuine M5a runner; demo_limit the recording length limit. Witness scripts
(witness/*.txt) go beyond the app's event domain and are run only to show the overflow build
reports them.

    m5b_input_scripts.py            write the scripts
    m5b_input_scripts.py --check    compare the committed scripts with a fresh generation
"""
import argparse, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / 'tests/reference/m5b_input'
SCRIPTS, WITNESS = DIR / 'scripts', DIR / 'witness'

# DOOM key codes (doomdef.h:250-280) and the vanilla bindings (m_misc.c:243-253, 269-271)
UP, DOWN, LEFT, RIGHT = 0xad, 0xaf, 0xac, 0xae
SL, SR, FIRE, USE, STRAFE, SPEED = 0x2c, 0x2e, 0x9d, 0x20, 0xb8, 0xb6
ESC, ENTER, TAB, BACKSPACE, PAUSE, EQUALS, MINUS = 27, 13, 9, 127, 0xff, 0x3d, 0x2d
F = {i: 0x80 + c for i, c in zip(range(1, 13), [0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x57, 0x58])}
VANILLA = dict(up=UP, down=DOWN, left=LEFT, right=RIGHT, strafeleft=SL, straferight=SR, fire=FIRE, use=USE,
               strafe=STRAFE, speed=SPEED)
# S7 modern preset: genuine configuration values only (m_misc.c defaults rows); always-run and the
# vertical-mouse suppression are rewrites of the event stream ahead of the genuine path.
MODERN_CONFIG = [('key_up', ord('w')), ('key_down', ord('s')), ('key_strafeleft', ord('a')), ('key_straferight', ord('d')),
                 ('key_use', ord('e')), ('mouseb_strafe', -1), ('mouseb_forward', -1)]
W, A, S, D, E = (ord(c) for c in 'wasde')

S_ = {}


def add(name, *lines, head=(), witness=False):
    body = []
    for l in lines:
        body.extend(l if isinstance(l, list) else [l])
    text = '\n'.join(list(head) + body) + '\n'
    (S_.setdefault('witness' if witness else 'scripts', {}))[name] = text


def kd(c): return f'ev keydown 0x{c:02x}'
def ku(c): return f'ev keyup 0x{c:02x}'
def ms(b, dx, dy): return f'ev mouse {b} {dx} {dy}'
def tic(n=1): return 'tic' if n == 1 else f'tic {n}'
def hold(c, n): return [kd(c), tic(n), ku(c)]
def app_mouse(b, px, py):
    """The S4 app event: pixels clamped to 319 each way, << 2 as linuxdoom (i_video.c:250-251), y up positive."""
    px, py = max(-319, min(319, px)), max(-319, min(319, py))
    return ms(b, px * 4, -py * 4)


MODERN = tuple(f'config set {k} {v}' for k, v in MODERN_CONFIG)


def generate():
    S_.clear()
    # ---- vanilla bindings at the event_t level: held, released in the next pump, tapped in one pump
    for n, c in VANILLA.items():
        add(f'ev_bind_{n}_hold', tic(), hold(c, 3), tic(2))
        add(f'ev_bind_{n}_nextpump', tic(), kd(c), tic(), ku(c), tic(2))
        add(f'ev_bind_{n}_samepump', tic(), kd(c), ku(c), tic(2))
    # ---- turning across SLOWTURNTICS (g_game.c:262-275), low bytes 0x40 and 0x80
    add('ev_turn_left_held', tic(), hold(LEFT, 9), tic())
    add('ev_turn_right_held', tic(), hold(RIGHT, 9), tic())
    add('ev_turn_left_repress', tic(), hold(LEFT, 4), tic(), hold(LEFT, 8), tic())
    add('ev_turn_left_then_right', tic(), kd(LEFT), tic(7), kd(RIGHT), tic(2), ku(LEFT), tic(3), ku(RIGHT), tic())
    add('ev_turn_run_left', tic(), kd(SPEED), kd(LEFT), tic(8), ku(LEFT), ku(SPEED), tic())
    add('ev_turn_run_right', tic(), kd(SPEED), kd(RIGHT), tic(8), ku(RIGHT), ku(SPEED), tic())
    add('ev_turn_run_late', tic(), kd(LEFT), tic(7), kd(SPEED), tic(2), ku(SPEED), tic(2), ku(LEFT), tic())
    add('ev_turn_with_mouse', tic(), kd(LEFT), ms(0, 3, 0), tic(), ms(0, -5, 0), tic(5), ms(0, 7, 0), tic(), ku(LEFT),
        kd(RIGHT), ms(0, 9, 0), tic(7), ku(RIGHT), tic())
    # ---- run with each movement key (g_game.c:175-176)
    for n, c in (('up', UP), ('down', DOWN), ('strafeleft', SL), ('straferight', SR)):
        add(f'ev_run_{n}', tic(), kd(SPEED), kd(c), tic(3), ku(c), tic(), ku(SPEED), tic())
    # ---- the strafe modifier (g_game.c:278-292) and the clamps (g_game.c:413-420)
    add('ev_strafe_mod_left', tic(), kd(STRAFE), kd(LEFT), tic(8), ku(LEFT), ku(STRAFE), tic())
    add('ev_strafe_mod_right', tic(), kd(STRAFE), kd(RIGHT), tic(8), ku(RIGHT), ku(STRAFE), tic())
    add('ev_strafe_mod_run_right', tic(), kd(STRAFE), kd(SPEED), kd(RIGHT), tic(3), ku(RIGHT), ku(SPEED), ku(STRAFE), tic())
    add('ev_strafe_mod_release_mid', tic(), kd(LEFT), kd(STRAFE), tic(3), ku(STRAFE), tic(4), ku(LEFT), tic())
    add('ev_strafe_mod_plus_strafekey', tic(), kd(STRAFE), kd(RIGHT), kd(SR), tic(2), kd(SPEED), tic(2), ku(RIGHT), ku(SR),
        ku(STRAFE), ku(SPEED), tic())
    # ---- opposite keys cancel
    add('ev_opp_left_right', tic(), kd(LEFT), kd(RIGHT), tic(8), ku(LEFT), ku(RIGHT), tic())
    add('ev_opp_up_down', tic(), kd(UP), kd(DOWN), tic(2), kd(SPEED), tic(2), ku(UP), ku(DOWN), ku(SPEED), tic())
    add('ev_opp_strafes', tic(), kd(SL), kd(SR), tic(2), ku(SL), tic(2), ku(SR), tic())
    add('ev_opp_strafe_mod_left_right', tic(), kd(STRAFE), kd(LEFT), kd(RIGHT), tic(2), ku(LEFT), tic(), ku(RIGHT), ku(STRAFE), tic())
    add('ev_all_moves_run', tic(), kd(SPEED), kd(UP), kd(SR), kd(STRAFE), kd(RIGHT), tic(2), ku(UP), ku(SR), ku(STRAFE), ku(RIGHT),
        ku(SPEED), tic())
    # ---- weapon keys 1-8 (g_game.c:342-348; i < NUMWEAPONS-1), 9 and 0 do nothing
    for ch in '1234567890':
        add(f'ev_weapon_{ch}', tic(), hold(ord(ch), 2), tic())
    add('ev_weapon_two_keys', tic(), kd(ord('5')), kd(ord('3')), tic(), ku(ord('3')), tic(), ku(ord('5')), tic())
    add('ev_weapon_fire_use', tic(), kd(ord('4')), kd(FIRE), kd(USE), tic(2), ku(ord('4')), ku(FIRE), ku(USE), tic())
    # ---- fire and use
    add('ev_fire_key_and_mouse', tic(), kd(FIRE), ms(1, 0, 0), tic(2), ku(FIRE), tic(), ms(0, 0, 0), tic())
    add('ev_use_key_and_dclick', tic(3), ms(4, 0, 0), tic(2), ms(0, 0, 0), kd(USE), tic(), ku(USE), ms(4, 0, 0), tic(2),
        ms(0, 0, 0), tic())
    # ---- the mouse at the event_t level (sensitivity 5: mousex = data2*10/10, g_game.c:579-580)
    add('ev_mouse_turn_pos', tic(), ms(0, 20, 0), tic(), tic())
    add('ev_mouse_turn_neg', tic(), ms(0, -20, 0), tic(), tic())
    add('ev_mouse_turn_small', tic(), *[[ms(0, v, 0), tic()] for v in (1, -1, 4, -4, 15, 16, -16, -17, 17)])
    add('ev_mouse_forward_clamp', tic(), *[[ms(0, 0, v), tic()] for v in (20, 50, 51, 400, -50, -51, -400)],
        kd(UP), ms(0, 0, 40), tic(), ms(0, 0, -100), tic(), ku(UP), tic())
    add('ev_mouse_strafe_mod', tic(), kd(STRAFE), *[[ms(0, v, 0), tic()] for v in (10, 25, 26, -400, 4, -4)], ku(STRAFE), tic())
    add('ev_mouse_strafe_button', tic(), ms(2, 10, 0), tic(), ms(2, 40, 0), tic(), ms(2, -12, 8), tic(), ms(0, 0, 0), tic())
    add('ev_mouse_two_motions', tic(), ms(0, 40, 0), ms(0, -8, 0), tic(), ms(0, 0, 12), ms(0, 20, 0), tic())
    add('ev_mouse_motion_then_button', tic(), ms(0, 40, 8), ms(1, 0, 0), tic(), ms(0, 0, 0), tic())
    add('ev_mouse_button_then_motion', tic(), ms(1, 0, 0), ms(1, 40, 8), tic(), ms(0, 0, 0), tic())
    add('ev_mouse_forward_button', tic(), ms(4, 0, 0), tic(3), ms(0, 0, 0), tic(), kd(SPEED), ms(4, 0, 0), tic(), ms(0, 0, 0),
        ku(SPEED), tic())
    add('ev_mouse_all_buttons', tic(), ms(7, 8, -8), tic(), ms(7, 0, 0), tic(2), ms(3, -8, 8), tic(), ms(0, 0, 0), tic())
    # ---- S3 rounding tables: built angleturn at full 16 bits and the executed (re-read) value
    add('round_fine_mouse', tic(), *[[ms(0, v, 0), tic()] for v in range(-64, 65)])
    add('round_app_mouse_sens5', tic(), *[[app_mouse(0, p, 0), tic()] for p in range(-319, 320)])
    add('round_keyboard', tic(),
        hold(LEFT, 7), tic(), hold(RIGHT, 7), tic(), kd(SPEED), hold(LEFT, 7), tic(), hold(RIGHT, 7), ku(SPEED), tic(),
        kd(LEFT), ms(0, 1, 0), tic(), ms(0, 2, 0), tic(), ms(0, 3, 0), tic(), ms(0, -1, 0), tic(), ms(0, -2, 0), tic(),
        ms(0, -3, 0), tic(), ms(0, 4, 0), tic(), ku(LEFT), tic())
    add('round_app_mouse_and_keys', tic(), kd(LEFT), *[[app_mouse(0, p, 0), tic()] for p in (-319, -100, -37, -5, -4, -3, 3, 4, 5, 37, 100, 319)],
        ku(LEFT), kd(RIGHT), kd(SPEED), *[[app_mouse(0, p, 0), tic()] for p in (-319, -4, 4, 319)], ku(RIGHT), ku(SPEED), tic())
    pixels = (-319, -200, -128, -64, -33, -32, -31, -17, -16, -15, -9, -8, -7, -6, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8,
              9, 15, 16, 17, 31, 32, 33, 64, 128, 200, 319)
    for s in range(10):
        add(f'sens_{s}_turn', tic(), *[[app_mouse(0, p, 0), tic()] for p in pixels],
            *[[app_mouse(0, 0, p), tic()] for p in (-20, -13, -12, -6, -1, 1, 6, 12, 13, 20)],
            head=(f'config set mouse_sensitivity {s}',))
    add('sens_5_strafe', tic(), kd(STRAFE), *[[app_mouse(0, p, 0), tic()] for p in (-319, -26, -25, -13, -12, -1, 1, 12, 13, 25, 26, 319)],
        ku(STRAFE), tic())
    # beyond the app's clamp (|data2| <= 1276) but inside int: the short narrowing of g_game.c:409 and the round trip
    # of a built turn of 32640 or more, which re-reads as -32768 (g_game.c:1512, 1501)
    add('beyond_app_mouse_short_edge', tic(), *[[ms(0, v, 0), tic()] for v in (4064, 4080, 4095, 4096, -4064, -4080, -4095, -4096,
                                                                             -4097, 8192, 262140, -262140)])
    add('ev_mouse_forward_sweep', tic(), *[[app_mouse(0, 0, p), tic()] for p in range(-16, 17)], kd(UP),
        *[[app_mouse(0, 0, p), tic()] for p in (-16, -7, -6, -1, 1, 6, 7, 16)], ku(UP), tic())
    # ---- double clicks (g_game.c:350-399): fixed cases and the boundary of the press/gap sweep
    add('dclick_fwd_inside', tic(3), ms(4, 0, 0), tic(2), ms(0, 0, 0), tic(2), ms(4, 0, 0), tic(2), ms(0, 0, 0), tic(2))
    add('dclick_fwd_toofast', tic(3), ms(4, 0, 0), tic(), ms(0, 0, 0), tic(), ms(4, 0, 0), tic(), ms(0, 0, 0), tic(3))
    add('dclick_fwd_held_long', tic(3), ms(4, 0, 0), tic(25), ms(0, 0, 0), tic(2), ms(4, 0, 0), tic(2), ms(0, 0, 0), tic())
    add('dclick_strafe_inside', tic(3), ms(2, 0, 0), tic(2), ms(0, 0, 0), tic(2), ms(2, 0, 0), tic(2), ms(0, 0, 0), tic(2))
    add('dclick_at_start', ms(4, 0, 0), ms(0, 0, 0), tic(), ms(4, 0, 0), tic(), ms(0, 0, 0), tic())
    add('dclick_both_buttons', tic(3), ms(6, 0, 0), tic(2), ms(0, 0, 0), tic(2), ms(6, 0, 0), tic(2), ms(0, 0, 0), tic(2))
    add('dclick_triple', tic(3), *[[ms(4, 0, 0), tic(2), ms(0, 0, 0), tic(2)] for _ in range(3)], tic())
    for b, nm in ((4, 'fwd'), (2, 'str')):
        for P in (1, 2, 3):
            for G in sorted({1, 2, 3, 23 - P, 24 - P, 25 - P}):
                add(f'dclk_{nm}_p{P}_g{G:02d}', tic(3), ms(b, 0, 0), tic(P), ms(0, 0, 0), tic(G), ms(b, 0, 0), tic(3), ms(0, 0, 0), tic(3))
    # ---- S5: what the genuine responders do with the keys the app never forwards
    s5 = dict(escape=ESC, enter=ENTER, tab=TAB, pause=PAUSE, minus=MINUS, equals=EQUALS, backspace=BACKSPACE, q=ord('q'),
              **{f'f{i}': c for i, c in F.items()})
    for n, c in s5.items():
        add(f's5_{n}', tic(), kd(c), ku(c), tic(), kd(c), tic(), ku(c), tic(), hold(UP, 2), tic(), hold(LEFT, 1), tic())
    add('s5_q_norecord', tic(), kd(ord('q')), tic(), ku(ord('q')), hold(UP, 1), tic(), head=('config record 0',))
    add('s5_escape_while_held', tic(), kd(UP), tic(), kd(ESC), tic(), ku(ESC), tic(), ku(UP), tic(2), hold(DOWN, 1), tic())
    add('s5_escape_twice', tic(), kd(ESC), tic(), ku(ESC), kd(ESC), tic(), ku(ESC), hold(UP, 1), tic())
    add('s5_pause_twice', tic(), kd(PAUSE), tic(), ku(PAUSE), tic(), kd(PAUSE), tic(), ku(PAUSE), tic(), hold(UP, 1), tic())
    add('s5_menu_mouse', tic(), kd(ESC), tic(), ku(ESC), ms(1, 0, 0), tic(), ms(0, 0, 0), tic(), ms(0, 40, 40), tic(),
        kd(ESC), tic(), ku(ESC), ms(1, 40, 0), tic(), ms(0, 0, 0), tic())
    add('s5_tab_then_keys', tic(), kd(TAB), tic(), ku(TAB), kd(LEFT), kd(MINUS), kd(ord('0')), kd(ord('f')), tic(2), ku(LEFT),
        ku(MINUS), ku(ord('0')), ku(ord('f')), tic(), kd(TAB), tic(), ku(TAB), hold(LEFT, 1), tic())
    for word in ('iddqd', 'idkfa', 'idclev12', 'idbeholds'):
        add(f's5_cheat_{word}', tic(), *[[kd(ord(ch)), tic(), ku(ord(ch))] for ch in word], tic(), hold(UP, 1), tic())
    # iddqd holds 'q' through a tic, which ends a recording (g_game.c:1508-1509): the cheat itself, unrecorded
    add('s5_cheat_iddqd_norecord', tic(), *[[kd(ord(ch)), tic(), ku(ord(ch))] for ch in 'iddqd'], tic(), hold(UP, 1), tic(),
        head=('config record 0',))
    add('s5_quit_f10_y', tic(), kd(F[10]), tic(), ku(F[10]), kd(ord('y')), tic(), ku(ord('y')), tic())
    add('s5_endgame_f7_y', tic(), kd(F[7]), tic(), ku(F[7]), kd(ord('y')), tic(), ku(ord('y')), tic(3))
    # ---- S6: the app's stream choices against the raw streams
    for n, c in (('fire', FIRE), ('use', USE), ('weapon3', ord('3')), ('left', LEFT), ('up', UP)):
        add(f's6_tap_{n}_raw', tic(), kd(c), ku(c), tic(3))                    # linuxdoom: a tap inside one pump is lost
        add(f's6_tap_{n}_deferred', tic(), kd(c), tic(), ku(c), tic(3))        # the app: the release goes to the next pump
    add('s6_repeat_raw', tic(), kd(LEFT), tic(2), kd(LEFT), tic(), kd(LEFT), kd(LEFT), tic(4), ku(LEFT), tic())
    add('s6_repeat_dropped', tic(), kd(LEFT), tic(8), ku(LEFT), tic())
    add('s6_repeat_x_pairs', tic(), kd(LEFT), tic(2), ku(LEFT), kd(LEFT), tic(), ku(LEFT), tic(), kd(LEFT), tic(4), ku(LEFT), tic())
    add('s6_focus_loss', tic(), kd(UP), kd(LEFT), kd(FIRE), ms(5, 8, 0), tic(3), ku(UP), ku(LEFT), ku(FIRE), ms(0, 0, 0), tic(3),
        kd(LEFT), tic(2), ku(LEFT), tic())
    add('s6_pause', tic(), kd(SPEED), kd(RIGHT), kd(SR), ms(2, 0, 0), tic(7), ku(SPEED), ku(RIGHT), ku(SR), ms(0, 0, 0), tic(2),
        kd(RIGHT), tic(2), ku(RIGHT), tic())
    add('s6_minimise', tic(), kd(STRAFE), kd(LEFT), kd(ord('2')), ms(4, 0, 0), tic(2), ku(STRAFE), ku(LEFT), ku(ord('2')),
        ms(0, 0, 0), tic(3))
    add('s6_keyup_order', tic(), kd(UP), kd(SL), kd(USE), tic(2), ku(USE), ku(UP), ku(SL), tic(), kd(UP), kd(SL), kd(USE), tic(2),
        ku(SL), ku(USE), ku(UP), tic())
    add('s6_stuck_without_release', tic(), kd(UP), kd(LEFT), ms(1, 0, 0), tic(10))
    # ---- S7: the modern preset through the genuine configuration values
    ar = (kd(SPEED),)        # always-run: key_speed held from the start
    add('modern_bindings', *ar, tic(), hold(W, 2), tic(), hold(S, 2), tic(), hold(A, 2), tic(), hold(D, 2), tic(), hold(E, 2), tic(),
        hold(UP, 1), hold(USE, 1), tic(), head=MODERN)
    add('modern_bindings_walk', tic(), hold(W, 2), tic(), hold(A, 2), tic(), hold(D, 1), hold(S, 1), tic(), head=MODERN)
    add('modern_alwaysrun_shift_inverts', *ar, tic(), kd(W), tic(2), ku(SPEED), tic(2), kd(SPEED), tic(2), ku(W), tic(), head=MODERN)
    add('modern_turn_slow_phase', *ar, tic(), hold(LEFT, 8), tic(), hold(RIGHT, 8), tic(), head=MODERN)
    add('modern_strafe_mod', *ar, tic(), kd(STRAFE), kd(LEFT), tic(3), ku(LEFT), kd(D), tic(2), ku(D), ku(STRAFE), tic(), head=MODERN)
    add('modern_mouse_horizontal_only', *ar, tic(), *[[app_mouse(0, p, 0), tic()] for p in (-319, -37, -4, 4, 37, 319)],
        app_mouse(1, 10, 0), tic(), app_mouse(0, 0, 0), tic(), head=MODERN)
    add('modern_mouse_vertical_raw', *ar, tic(), *[[app_mouse(0, 0, p), tic()] for p in (-20, 20)], head=MODERN)
    add('modern_mouse_buttons_unbound', *ar, tic(), ms(2, 10, 0), tic(2), ms(4, 0, 0), tic(2), ms(0, 0, 0), tic(2), ms(4, 0, 0), tic(2),
        ms(0, 0, 0), tic(2), ms(2, 0, 0), tic(2), ms(0, 0, 0), tic(2), head=MODERN)
    add('modern_weapons_fire', *ar, tic(), kd(ord('6')), kd(FIRE), tic(2), ku(ord('6')), ku(FIRE), ms(1, 0, 0), tic(), ms(0, 0, 0), tic(),
        head=MODERN)
    add('modern_letters_no_cheat', *ar, tic(), *[[kd(c), tic(), ku(c)] for c in (D, D, E, W, A, S, D)], tic(), head=MODERN)
    add('modern_old_keys_unbound', *ar, tic(), hold(SL, 1), hold(SR, 1), hold(DOWN, 1), tic(), head=MODERN)
    add('modern_focus_loss', *ar, tic(), kd(W), kd(A), ms(1, 8, 0), tic(2), ku(W), ku(A), ms(0, 0, 0), tic(2), head=MODERN)
    # ---- S11: restart is a new game; the level-load clear (g_game.c:489-496) and what it leaves
    add('restart_state_kept_by_reload', tic(3), ms(4, 0, 0), tic(2), ms(0, 0, 0), kd(LEFT), tic(7), 'loadlevel', tic(2), ku(LEFT),
        ms(4, 0, 0), tic(2), ms(0, 0, 0), tic(2))
    add('loadlevel_clears', tic(), kd(UP), kd(LEFT), ms(7, 0, 0), tic(3), 'loadlevel', tic(2), ku(LEFT), kd(LEFT), tic(2), ku(UP),
        ku(LEFT), ms(0, 0, 0), tic())
    add('loadlevel_mouse_strafe', tic(), ms(2, 0, 0), kd(LEFT), tic(2), 'loadlevel', ku(LEFT), kd(LEFT), tic(2), ku(LEFT), ms(0, 0, 0), tic())
    add('loadlevel_mouse_forward', tic(), ms(4, 0, 0), tic(2), 'loadlevel', tic(2), ms(0, 0, 0), tic())
    add('x_loadlevel_mouse_strafe', tic(), 'bpress 2', 'press Left', tic(2), 'loadlevel', 'release Left', 'press Left', tic(2),
        'release Left', 'brelease 2', tic(), head=('config grab 1',))
    # ---- the event queue (MAXEVENTS 64, d_event.h:108; D_PostEvent, d_main.c:150-154)
    add('queue_63', tic(), *([kd(SL), ku(SL)] * 31), kd(UP), tic(), ku(UP), tic())
    add('queue_64', tic(), *([kd(SL), ku(SL)] * 31), kd(SR), kd(UP), tic(), ku(UP), ku(SR), tic())
    add('queue_65', tic(), *([kd(SL), ku(SL)] * 32), kd(UP), tic(), ku(UP), tic())
    # ---- the X level through the genuine i_video.c (xlatekey, I_GetEvent, I_StartTic)
    keysyms = ['Left', 'Right', 'Up', 'Down', 'comma', 'period', 'space', 'Control_L', 'Control_R', 'Shift_L', 'Shift_R', 'Alt_L',
               'Alt_R', 'Meta_L', 'Meta_R', '1', '8', 'a', 'z', 'A', 'Z', 'equal', 'minus', 'KP_Equal', 'KP_Subtract', 'BackSpace',
               'Delete', 'Return', 'Tab', 'Home', 'KP_Enter', 'Caps_Lock', 'Super_L', '0xac', '0xad', '0xb6', '0xb8', '0xe9']
    add('x_keys_translate', 'tic', *[[f'press {k}', 'tic', f'release {k}', 'tic'] for k in keysyms])
    for k in ('Control_R', 'Shift_R', 'Alt_R', 'Meta_L', 'Meta_R'):
        add(f'x_modifier_{k}', 'tic', f'press {k}', 'press Left', 'press Up', 'tic 7', 'release Left', 'release Up', f'release {k}', 'tic')
    for ks, what in (('0xff', 'ydiaeresis_is_KEY_PAUSE'), ('0xbb', 'guillemotright_is_KEY_F1'), ('0xac', 'notsign_is_KEY_LEFTARROW'),
                     ('0xb8', 'cedilla_is_KEY_RALT'), ('0xb6', 'paragraph_is_KEY_RSHIFT'), ('0xad', 'hyphen_is_KEY_UPARROW')):
        add(f'x_latin1_{what}', 'tic', f'press {ks}', 'tic 2', f'release {ks}', 'tic', 'press Left', 'tic', 'release Left', 'tic')
    for k in ('Home', 'KP_Enter', 'Super_L', 'Caps_Lock', 'Delete', 'KP_Subtract', 'KP_Equal'):
        add(f'x_nongame_{k}', 'tic', f'tap {k}', 'tic', f'press {k}', 'tic', f'release {k}', 'press Up', 'tic 2', 'release Up', 'tic')
    add('x_upper_case_keysym', 'tic', 'press I', 'tic', 'release I', 'press D', 'tic', 'release D', 'tic')
    g = ('config grab 1',)
    add('x_fire_mouse', 'tic', 'bpress 1', 'tic 3', 'brelease 1', 'tic 2', 'motion 170 110', 'tic', head=g)
    add('x_buttons', 'tic', 'bpress 1', 'tic', 'bpress 3', 'tic', 'brelease 1', 'tic', 'bpress 2', 'tic', 'brelease 3', 'tic',
        'brelease 2', 'tic', head=g)
    add('x_button_press_release_same_tic', 'tic', 'bpress 1', 'brelease 1', 'tic 2', 'bpress 3', 'brelease 3', 'tic 2', head=g)
    add('x_motion_first_jump', 'tic', 'motion 170 110', 'tic', 'motion 180 120', 'tic', 'motion 175 125', 'tic', head=g)
    add('x_motion_centre_lines', 'tic', 'motion 170 110', 'tic', 'motion 190 100', 'tic', 'motion 160 130', 'tic', 'motion 200 140',
        'tic', head=g)
    add('x_motion_two_in_tic', 'tic', 'motion 170 110', 'tic', 'motion 190 120', 'motion 180 125', 'tic', head=g)
    add('x_motion_and_button', 'tic', 'motion 170 110', 'tic', 'motion 190 120', 'bpress 1', 'tic', 'brelease 1', 'tic', head=g)
    add('x_motion_with_button_held', 'tic', 'motion 170 110', 'tic', 'bpress 3', 'tic', 'motion 175 115', 'tic', 'brelease 3', 'tic',
        head=g)
    add('x_grab_warp_horizontal', 'tic', 'motion 165 100', 'tic', 'motion 175 100', 'tic', 'tic', head=g + ('config warpmodel 1',))
    add('x_grab_warp_diagonal', 'tic', 'motion 170 110', 'tic', 'motion 180 95', 'tic', 'tic', head=g + ('config warpmodel 1',))
    add('x_grab_nowarpmodel', 'tic', 'motion 170 110', 'tic', 'motion 180 95', 'tic', 'tic', head=g)
    add('x_nograb_motion', 'tic', 'motion 170 110', 'tic', 'motion 180 95', 'tic', 'tic')
    add('x_big_motion_short_wrap', 'tic', 'motion 1300 2000', 'tic', 'motion -1300 -2000', 'tic', head=g)
    add('x_window_edges', 'tic', 'motion 0 0', 'tic', 'motion 319 199', 'tic', 'motion 1 1', 'tic', head=g)
    add('x_expose_ignored', 'tic', 'expose', 'press Up', 'tic', 'release Up', 'tic')
    add('x_repeat_samepump', 'tic', 'press Left', 'tic 3', 'release Left', 'press Left', 'tic', 'release Left', 'press Left',
        'release Left', 'press Left', 'tic 3', 'release Left', 'tic')
    add('x_repeat_across_tic', 'tic', 'press Left', 'tic 3', 'release Left', 'tic', 'press Left', 'tic 4', 'release Left', 'tic')
    add('x_repeat_weapon', 'tic', 'press 3', 'tic', 'release 3', 'press 3', 'tic', 'release 3', 'tic')
    add('x_queue_70', 'tic', 'press Up', *(['press comma', 'release comma'] * 34), 'press period', 'tic', 'release Up',
        'release period', 'tic')
    add('x_focus_loss_releases', 'tic', 'press Up', 'press Left', 'bpress 1', 'bpress 3', 'tic 3', 'release Up', 'release Left',
        'brelease 1', 'brelease 3', 'tic 2', head=g)
    add('x_tap_same_pump', 'tic', 'tap Control_L', 'tap space', 'tap 3', 'tic 2')
    # ---- the chain: a recorded demo replayed by the genuine M5a runner (m5b_input_reference.py)
    add('chain_walk', tic(5), kd(UP), tic(20), kd(LEFT), tic(10), ku(LEFT), ku(UP), ms(2, 40, 0), tic(), ms(0, -60, 20), tic(),
        kd(FIRE), tic(3), ku(FIRE), kd(ord('2')), tic(), ku(ord('2')), kd(USE), tic(), ku(USE), kd(STRAFE), kd(RIGHT), tic(3),
        ku(RIGHT), ku(STRAFE), *[[app_mouse(0, p, 0), tic()] for p in (-5, -4, 4, 5, 37)], ms(0, 0, 0), tic(10), 'enddemo')
    # ---- the recording length limit (g_game.c:1515-1519 with maxsize 0x20000, g_game.c:1538)
    add('demo_limit', tic(), kd(UP), tic(32770))
    # ---- witnesses beyond the app's event domain (|data2| <= 1276): int overflow in G_Responder (g_game.c:579-580)
    add('ev_mouse_int_edge', tic(), ms(0, 214748364, 0), tic(), ms(0, 0, -214748364), tic(), witness=True)
    add('ev_mouse_int_overflow', tic(), ms(0, 300000000, 0), tic(), witness=True)
    return S_


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    gen = generate()
    bad = []
    for kind, d in (('scripts', SCRIPTS), ('witness', WITNESS)):
        want = {f'{n}.txt': t for n, t in gen[kind].items()}
        if a.check:
            have = {p.name for p in d.glob('*.txt')} if d.exists() else set()
            bad += [f'{d.name}/{n}' for n in sorted(have ^ set(want))]
            bad += [f'{d.name}/{n}' for n in sorted(set(want) & have) if (d / n).read_text() != want[n]]
        else:
            d.mkdir(parents=True, exist_ok=True)
            bad += [f'{d.name}/{p.name} (stale: not generated; remove it by hand)' for p in d.glob('*.txt') if p.name not in want]
            for n, t in want.items(): (d / n).write_text(t)
    if bad: sys.exit('m5b input scripts differ from a fresh generation: ' + ', '.join(bad))
    print(f"m5b input scripts {'match' if a.check else 'written'}: {len(gen['scripts'])} scripts, {len(gen['witness'])} witnesses")


if __name__ == '__main__':
    main()
