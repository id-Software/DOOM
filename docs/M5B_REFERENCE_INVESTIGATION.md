# M5b: playable (a window, live input, the status bar, HUD and the real-time loop): reference investigation

Phase 1 note for milestone M5b, which makes the game playable in a limited way: one command opens a window and a real Episode 1 map is played with keyboard and mouse at 35 tics a second, on the wire-24 simulation and the M5a renderer, with the status bar, HUD messages and palette flashes. Everything in the 320×200 frame is to stay pixel-identical to linuxdoom-1.10, and live input is to become tic commands exactly as the original makes them. This note records the original C facts (input, recording, the loop, the status bar, the HUD), the genuine-engine harness designs with the results of two scratch prototypes, the Rust-side state inventory and design, a measured comparison of window libraries, the owner decisions S1–S23 with one recommendation each, and the Phase 2 plan, size estimate and cap.

The behavioural authority is the original id Software Linux DOOM 1.10 at `a77dfb96cb91780ca334d0d4cfd86957558007e0`. Milestone status lives in [ROADMAP](ROADMAP.md), support in [COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md) and commands in [TESTING](TESTING.md) (see the [documentation authority map](README.md#documentation-authority)). Observations of this milestone will be in `docs/validation/M5B.md`, which Phase 2 creates. This note records only counts, pass/fail facts and digests from runs against the IWAD: no lump content, no IWAD-derived names, sizes, coordinates or geometry. All dates are America/Los_Angeles.

**Merged since: PR #62, by the owner, on 2026-11-18. The status below is as written before that.** **Status / next step (2026-11-18 00:41 PST).** Phase 1 is complete (`689e09d`); the owner's rulings on S1–S23 are recorded ([§4 Owner rulings](#owner-rulings-2026-11-17)). **Done on the branch** ([§5.1](#51-steps)): everything through step 15. The final gates were run from a clean clone on ext4 at `d5a4c4a` and all passed (workspace, release, Rust 1.85.0, rustdoc, every C and encoder `--check`, the M5a and M5b tools, 55 fuzz builds, seed replays and four bounded campaigns with no finding, all 61 real-map runs and the three saved sessions, and the fourth mutation run: 1,154 caught, 10 missed (the ten argued equivalent), 6 timeouts, 76 unviable); nothing failed in the product, and what was not run is listed in step 15 of the [evidence record](validation/M5B.md#step-15-final-gates-from-clean), which owns the totals; exact CI is in GitHub. This commit records them. **Next:** (1) push; (2) the Windows artifact rebuilt on the final head (label `windows-artifact`); (3) CI green on the exact head; (4) mark pull request #62 ready and leave it unmerged; (5) the owner's review. **Owner decision outstanding:** the 10-minute limit of the required job `Minimum Rust / 1.85.0` (7 min 10 s at `8d167c9` in CI; 7 min 56 s for the same command on the owner's machine at `d5a4c4a`); not changed. Nobody has seen the picture, pressed a real key, moved a real mouse, or run anything on Windows or macOS beyond the CI builds and tests. To resume: `git checkout feat/m5b-playable && git pull --ff-only origin feat/m5b-playable`, `git worktree list`, then `cargo test -p doom-render --test m5b_display --test m5b_restore`, `cargo test -p doom-game -p doom-play`, `python3 tools/reference/m5b_reference.py --work ~/scratch/m5b/step1/ref --check --jobs 6`, `python3 tools/reference/m5b_input_reference.py --check`, `python3 tools/reference/m5b_session.py --check --selftest --work ~/scratch/m5b/step7/work`, `python3 tools/reference/m5b_inject.py --check --selftest --work ~/scratch/m5b/step7b/inject`, `python3 tools/media/m5b_readme_visual.py --check`, then §5.1. Scratch, outside the repository and to be deleted at the end of the milestone, is under `~/scratch/m5b/` (the gates clone is already deleted). `/mnt/c` free: 28 GB (`df -h /mnt/c`).

## Summary

- **Owner rulings.** All decisions S1–S23 were confirmed by the owner on 2026-11-17, with notes on S2, S3, S15, S16, S20 and on C jobs: see [Owner rulings](#owner-rulings-2026-11-17). The bullets below are the Phase 1 record.
- **No simulation change is needed.** Every input of the status bar, the HUD, the palette choice and the command path is in canonical state or exactly derivable by a presentation-side observer ([§1.6](#16-what-the-rust-state-already-carries-and-what-is-missing)), with one exception: the face's idle glance, below.
- **The harness works in prototype.** A fork of the M5a runner with the genuine status bar and HUD compiled whole ran all 48 synthetic items and all 61 real Episode 1 runs at screenblocks 10 and 11: the strict, overflow and address builds were identical, two strict runs were identical, there was no sanitizer finding and no undefined read in the new code, and the 3D view of every message-free frame equalled its M5a golden ([§2.1](#21-full-frames-the-genuine-status-bar-hud-and-palette-in-the-runner)). A dedicated headless driver ran the genuine event-to-command path on 327 scripts, identically in the three builds, and a demo it recorded was replayed by the genuine engine ([§2.2](#22-input-the-genuine-event-path-as-a-headless-driver)).
- **Two dump fields change when the genuine status bar and HUD run** (the `M_Random` index and the message on the tic it is taken), so the proof the brief asked for needs a declared record-and-mask, as `ML_MAPPED` did in M5a (S16).
- **The face's idle glance cannot match a real vanilla session without sound.** `M_Random` is shared with sound starts and the level-start melt; with them modelled, the face differs in 43% of real-map frames. Nothing else on screen and nothing in the game depends on it. The recommendation (confirmed) is a declared sound-free contract until sound exists (S15).
- **Recording decides turning.** Vanilla executes the command it has just re-read from the demo buffer, so a recorded session turns in steps of 256 units and drops smaller mouse turns without carrying them. Recording every session, as recommended and confirmed, makes every session a valid vanilla demo at that cost in mouse feel (S3).
- **linuxdoom's own mouse layer is not playable** (no mouse without `-grabmouse`; only the last motion event of a tic counts; purely horizontal moves are dropped; fire can stick). The recommendation (confirmed) is to post one clean accumulated mouse event per command and keep the genuine path from there on (S4).
- **The window library.** `winit`, the first candidate, carries Apache-2.0-only crates, which the GPL-2.0-only rule may not admit; `minifb` cannot capture the mouse; SDL2 is clean but is a C library that could not be linked or run here; `sdl3` does not compile on Rust 1.85.0. `miniquad` is pure Rust, clean, five crates, and ran under WSLg; it is the recommendation (confirmed), with SDL2 as the fallback (S2).
- **What nobody has observed yet:** a picture on a screen, a real key press or mouse movement, mouse capture under WSLg, input latency, or anything on Windows or macOS. A human has to judge feel.
- **Unreachable under wire 24**, so the targets are 12 of 14 palettes and 40 of 42 faces (41 with the bespoke cheat scenario; see [§2.7](#27-corrections-found-in-phase-2)): palettes 1 and 9 are never chosen by the source, the ouch face at pain level 4 cannot occur, the god face needs invulnerability or a cheat, and berserk is not admitted (S23).

## 1. C facts: input, recording, the loop, the status bar, the HUD and the Rust state

This section has six subsections: the input path (§1.1), recording (§1.2), the loop and the display order (§1.3), the status bar and the palette choice (§1.4), HUD messages (§1.5), and what the Rust state already carries (§1.6). Every C fact cites file:line of linuxdoom-1.10.

### 1.1 Input: events, responders, G_BuildTiccmd, bindings and the X translation

All citations are `file:line` into `linuxdoom-1.10/` at the checked-in original
source. "Inference" marks anything not read directly from that source. One small
C probe was compiled with gcc 15.2.0 (`-std=gnu89 -fwrapv`, at `-O0` and `-O2`,
identical output) under `~/scratch/m5b/probe-input/` (ext4) to confirm the
arithmetic noted in 1.1.9; nothing else was built or run. No IWAD was read.

Cross-references to the other Phase 1 sections: the main loop (TryRunTics,
NetUpdate, `-dup`/ticdup, G_Ticker's command copy), the status bar (ST_Responder's
automap messages, the face's `M_Random`), and the HUD (HU_Responder's message
refresh) are owned there; they are mentioned here only where the input path
touches them.

#### 1.1.1 Events and the event queue

1. **Types** (d_event.h:35-50). `evtype_t` is `ev_keydown, ev_keyup, ev_mouse,
   ev_joystick` (0..3). `event_t` is `{ evtype_t type; int data1; int data2;
   int data3; }`: data1 is the key code or the mouse/joystick button bits, data2
   and data3 the x and y motion.
2. **Queue.** `#define MAXEVENTS 64` (d_event.h:108); `events[MAXEVENTS]`,
   `eventhead`, `eventtail` are globals (d_main.c:142-144), zero at start.
3. **D_PostEvent** (d_main.c:150-154) copies the event to `events[eventhead]`
   and does `eventhead = (++eventhead)&(MAXEVENTS-1);`. There is **no overflow
   check**. If 64 events are posted with none processed, head catches up with
   tail and the whole queue reads as empty: all 64 are silently lost, and later
   posts overwrite slots from the tail. The only poster in this source is
   i_video.c (lines 206, 212, 225, 241, 260).
4. **D_ProcessEvents** (d_main.c:161-178). First the store-demo guard:
   `if ( ( gamemode == commercial ) && (W_CheckNumForName("map01")<0) ) return;`
   (d_main.c:166-168). It returns *without* advancing `eventtail`, so in that
   case events pile up and wrap as in fact 3. Otherwise it walks tail to head
   with the same `(++eventtail)&(MAXEVENTS-1)` idiom (d_main.c:170) and, for each
   event, calls `M_Responder(ev)`; if that returns true the event is consumed
   ("menu ate the event", d_main.c:173-174), else `G_Responder(ev)` is called and
   its return value is ignored (d_main.c:175).
5. **Callers** (cross-reference, main-loop section): D_ProcessEvents runs right
   after `I_StartTic` once per command built, in `NetUpdate` (d_net.c:401-410)
   and in the `-singletics` path of `D_DoomLoop` (d_main.c:376-379). In
   NetUpdate the `break` for a full command buffer (d_net.c:404-405) sits
   *between* D_ProcessEvents and G_BuildTiccmd, so events can be processed (and
   `mousex`/`mousey` assigned) on an iteration that builds no command; the values
   then carry into the next build.

#### 1.1.2 Responders ahead of the game keys (single player, level, no menu, no automap, no chat)

Order: `M_Responder` (d_main.c:173), then inside `G_Responder` (g_game.c:504):
the F12 spy branch (g_game.c:507-518), the demo/"any key" branch
(g_game.c:521-534), then only when `gamestate == GS_LEVEL` `HU_Responder`,
`ST_Responder`, `AM_Responder` (g_game.c:536-550), `F_Responder` only in
`GS_FINALE` (g_game.c:552-556), then the game switch (g_game.c:558-597).

**M_Responder** (m_menu.c:1349-1716), menu inactive:

6. `ch` starts at -1 (m_menu.c:1358). For `ev_keydown`, `ch = ev->data1`
   (m_menu.c:1441-1445). For `ev_mouse` (when `mousewait < I_GetTime()`,
   m_menu.c:1399) it accumulates its own static `mousex`/`mousey` and may set
   `ch` to an arrow, `KEY_ENTER` or `KEY_BACKSPACE` (m_menu.c:1401-1438); with
   the menu inactive none of those values matches a case below, so mouse events
   fall through to G_Responder (the statics and `mousewait` change, and the
   wall-clock `I_GetTime` is read, but nothing that feeds a command). Key-up
   events leave `ch == -1` and return false (m_menu.c:1448-1449).
7. `saveStringEnter` and `messageToPrint` branches (m_menu.c:1453-1508) only
   apply while a save-name or a message box is up (out of scope).
8. `if (devparm && ch == KEY_F1)` takes a screenshot and eats (m_menu.c:1511-1515).
9. With `!menuactive` (m_menu.c:1519-1605), all **eaten** (`return true`):

   | Key (code) | Condition | Effect (state written) | Lines |
   | --- | --- | --- | --- |
   | `KEY_MINUS` 0x2d | not eaten if `automapactive \|\| chat_on` | `M_SizeDisplay(0)`: `screenblocks--`, `screenSize--` if `screenSize > 0`; `R_SetViewSize`; menu sound | m_menu.c:1522-1527, 1152-1172 |
   | `KEY_EQUALS` 0x3d | same | `M_SizeDisplay(1)`: up while `screenSize < 8` | m_menu.c:1529-1534 |
   | `KEY_F1` | - | opens the control panel on the read-this page | m_menu.c:1536-1546 |
   | `KEY_F2` / `KEY_F3` | - | control panel, save / load menu | m_menu.c:1548-1558 |
   | `KEY_F4` | - | control panel, sound volume | m_menu.c:1560-1565 |
   | `KEY_F5` | - | `M_ChangeDetail`: `detailLevel = 1 - detailLevel`, prints "low detail mode n.a.", returns before `R_SetViewSize` | m_menu.c:1567-1570, 1131-1148 |
   | `KEY_F6` | - | `M_QuickSave` | m_menu.c:1572-1575 |
   | `KEY_F7` | - | `M_EndGame(0)` | m_menu.c:1577-1580 |
   | `KEY_F8` | - | `M_ChangeMessages`: `showMessages = 1 - showMessages`, sets the player's message, `message_dontfuckwithme = true` | m_menu.c:1582-1585, 978-990 |
   | `KEY_F9` | - | `M_QuickLoad` | m_menu.c:1587-1590 |
   | `KEY_F10` | - | `M_QuitDOOM(0)` | m_menu.c:1592-1595 |
   | `KEY_F11` | - | `usegamma` cycles 0..4, sets the player's message, reloads the palette | m_menu.c:1597-1603 |
   | `KEY_ESCAPE` 27 | - | `M_StartControlPanel`, sound | m_menu.c:1609-1617 |

   Every other key returns false (m_menu.c:1618). Each sound call reaches
   `S_StartSoundAtVolume`, which draws `M_Random` for pitch (s_sound.c:327-345);
   the status bar face also draws `M_Random` (st_stuff.c:992). That is the menu
   RNG, not `P_Random`, but it is shared with the status bar (cross-reference).

**G_Responder before the game switch:**

10. **F12 spy** (g_game.c:507-518): `gamestate == GS_LEVEL && ev_keydown &&
    data1 == KEY_F12 && (singledemo || !deathmatch)`. It steps `displayplayer`
    until an in-game player or the console player; with one player it comes back
    to the console player (no net change). **Eaten.**
11. **Demo / any key** (g_game.c:521-534): only when `gameaction == ga_nothing
    && !singledemo && (demoplayback || gamestate == GS_DEMOSCREEN)`. Not reached
    in live single-player play.
12. **HU_Responder** (hu_stuff.c:617-759). Any event whose `data1 == KEY_RSHIFT`
    sets the static `shiftdown` and returns false; `KEY_RALT`/`KEY_LALT` likewise
    `altdown` (hu_stuff.c:643-652). Non-keydown returns false (hu_stuff.c:654-655).
    With `!chat_on`: `data1 == HU_MSGREFRESH` (= `KEY_ENTER`, hu_stuff.h:38) sets
    `message_on = true`, `message_counter = HU_MSGTIMEOUT` (4*TICRATE,
    hu_stuff.h:44) and is **eaten** (hu_stuff.c:659-664). The chat toggle `'t'`
    (hu_stuff.c:57, 665) and the destination keys (hu_stuff.c:670-698) require
    `netgame`. So in single player only Enter is eaten, and `chat_on` stays false
    (initialised false at hu_stuff.c:432; set true only under netgame).
13. **ST_Responder** (st_stuff.c:518-724). Key-up events whose
    `(data1 & 0xffff0000) == AM_MSGHEADER` switch the status bar's automap state
    (st_stuff.c:523-537); those messages come from am_map.c calling ST_Responder
    directly (am_map.c:461/495, 562/566), never through the queue. Every keydown is
    fed to the cheat matcher (`cht_CheckCheat`, m_cheat.c:43-73; the god, ammo,
    music, noclip, behold, choppers and mypos cheats under `!netgame`,
    st_stuff.c:540-673; clev at st_stuff.c:675-722). Cheats write player state
    directly, outside the command stream. **ST_Responder never eats**: every path
    ends in `return false` (st_stuff.c:697-724).
14. **AM_Responder** (am_map.c:614-735). With the automap off, keydown
    `AM_STARTKEY` (= `KEY_TAB`, am_map.c:96) calls `AM_Start`, sets
    `viewactive = false` and is **eaten** (am_map.c:625-631). Nothing else is
    eaten while the automap is off.

15. **Summary: keydowns that never reach `gamekeydown[]`** in this situation:
    Esc, `-` and `=` (including keypad minus/equal, see 1.1.7), F1-F11 (menu),
    F12 (spy), Enter (HUD message refresh), Tab (automap), and Pause (turned into
    `sendpause`, 1.1.3). Their **key-up** events are not eaten by M_Responder, HU
    (except shift/alt bookkeeping, which returns false), ST or AM, and reach the
    game switch, which clears `gamekeydown[]` for them (harmless). Every other
    keydown also passes through the cheat matcher on the way.

#### 1.1.3 G_Responder's game part (g_game.c:558-597)

16. **Arrays** (g_game.c:175-199): `#define NUMKEYS 256`;
    `boolean gamekeydown[NUMKEYS]`; `int turnheld`;
    `boolean mousearray[4]; boolean* mousebuttons = &mousearray[1]; // allow [-1]`;
    `int mousex, mousey` ("mouse values are used once");
    `int dclicktime, dclickstate, dclicks, dclicktime2, dclickstate2, dclicks2`;
    `int joyxmove, joyymove` ("joystick values are repeated");
    `boolean joyarray[5]; boolean* joybuttons = &joyarray[1];`.
    `boolean` is `typedef enum {false, true} boolean` (doomtype.h:34), 4 bytes on
    x86-64 gcc (probe).
17. **ev_keydown** (g_game.c:560-568): `if (ev->data1 == KEY_PAUSE) { sendpause
    = true; return true; }`; then `if (ev->data1 <NUMKEYS) gamekeydown[ev->data1]
    = true;` and **eat**. Only the upper bound is checked.
18. **ev_keyup** (g_game.c:570-573): `if (ev->data1 <NUMKEYS) gamekeydown[...] =
    false;` and return false.
19. **ev_mouse** (g_game.c:575-581), exact text:

    ```c
    mousebuttons[0] = ev->data1 & 1;
    mousebuttons[1] = ev->data1 & 2;
    mousebuttons[2] = ev->data1 & 4;
    mousex = ev->data2*(mouseSensitivity+5)/10;
    mousey = ev->data3*(mouseSensitivity+5)/10;
    return true;    // eat events
    ```

    `mousex`/`mousey` are **assigned, not accumulated**: each `ev_mouse`,
    including a button-only event with data2 = data3 = 0, replaces the previous
    value. The arithmetic is `int`: multiply first, then divide by 10 truncating
    toward zero (C89 leaves the direction implementation-defined; gcc truncates
    toward zero). The probe confirms symmetry: data2 = -12 gives -6, -7, -8, -9,
    -10, -12, -13, -14, -15, -16 for sensitivity 0..9 (floor division would give
    -7 at sensitivity 1, for example). At the default sensitivity 5 the factor is
    exactly 1. The stored button values are 1, 2 and 4, not 0/1.
20. **ev_joystick** (g_game.c:583-590): four button bits and
    `joyxmove = data2; joyymove = data3;`. No code in this source posts
    `ev_joystick` (only i_video.c posts, 1.1.7), so the joystick terms are always
    zero/false in linuxdoom.
21. **Index ranges.** `mousebuttons[-1]` is `mousearray[0]`, which nothing ever
    writes, so a binding of -1 reads false; the legal mouse indices are -1..2 and
    joystick -1..3. All shipped defaults are inside these ranges (1.1.6).
    `gamekeydown[key_*]` is in range for the defaults. Nothing validates values
    loaded from a config file (1.1.6), and `gamekeydown[ev->data1]` would go
    negative for a negative data1 (the X layer cannot produce one, 1.1.7).
22. **Level start reset** (G_DoLoadLevel, g_game.c:490-496): clears
    `gamekeydown`, `joyxmove/joyymove`, `mousex/mousey`, `sendpause`, `sendsave`,
    `paused`, then `memset (mousebuttons, 0, sizeof(mousebuttons));` and the same
    for `joybuttons`. `mousebuttons` is a *pointer*, so `sizeof` is 8 on x86-64:
    only `mousebuttons[0]` and `[1]` are cleared and `mousebuttons[2]` (the default
    forward button) survives (probe confirms; on a 32-bit build only `[0]`).
    `turnheld` and the double-click state are not reset.

#### 1.1.4 G_BuildTiccmd, line by line (g_game.c:237-437)

Types: locals `int i; boolean strafe; boolean bstrafe; int speed; int tspeed;
int forward; int side; ticcmd_t* base;` (g_game.c:239-247). `ticcmd_t`
(d_ticcmd.h:36-44) is `{ char forwardmove; char sidemove; short angleturn;
short consistancy; byte chatchar; byte buttons; }`; `char` is signed on x86-64
gcc. Tables (g_game.c:171-177), all `fixed_t` (int):
`forwardmove[2] = {0x19, 0x32}` (25, 50), `sidemove[2] = {0x18, 0x28}` (24, 40),
`angleturn[3] = {640, 1280, 320}` ("+ slow turn"). Constants:
`#define MAXPLMOVE (forwardmove[1])` (g_game.c:171), `TURBOTHRESHOLD 0x32`
(g_game.c:173, used only by G_Ticker), `SLOWTURNTICS 6` (g_game.c:179),
`NUMKEYS 256`, `NUMWEAPONS` = 9 (doomdef.h:182-192). Button codes
(d_event.h:72-100): `BT_ATTACK 1, BT_USE 2, BT_SPECIAL 128, BT_SPECIALMASK 3,
BT_CHANGE 4, BT_WEAPONMASK 56, BT_WEAPONSHIFT 3, BTS_PAUSE 1, BTS_SAVEGAME 2,
BTS_SAVEMASK 28, BTS_SAVESHIFT 2`. `ticdup` is 1 unless `-dup` (i_net.c:254-265;
main-loop section).

1. `base = I_BaseTiccmd();` (g_game.c:249) returns `&emptycmd`, a zero static
   `ticcmd_t` (i_system.c:64-68). `memcpy(cmd, base, sizeof(*cmd));`
   (g_game.c:250): the command starts all zero.
2. `cmd->consistancy = consistancy[consoleplayer][maketic%BACKUPTICS];`
   (g_game.c:252-253). The array (short, g_game.c:140) is written only in
   G_Ticker's netgame branch (g_game.c:681-693), so in single player it is 0.
3. `strafe = gamekeydown[key_strafe] || mousebuttons[mousebstrafe] ||
   joybuttons[joybstrafe];` (g_game.c:256-257).
4. `speed = gamekeydown[key_speed] || joybuttons[joybspeed];` (g_game.c:258).
   Both are 0/1.
5. `forward = side = 0;` (g_game.c:260).
6. Turn hold (g_game.c:264-270): if `joyxmove < 0 || joyxmove > 0 ||
   gamekeydown[key_right] || gamekeydown[key_left]` then `turnheld += ticdup;`
   else `turnheld = 0;`. Note: this counts while strafing too, and while both
   turn keys cancel out.
7. `tspeed = (turnheld < SLOWTURNTICS) ? 2 : speed;` (g_game.c:272-275). With
   ticdup 1, the first five tics of a hold (turnheld 1..5) use `angleturn[2]` =
   320; from the sixth (turnheld >= 6) it is 640 or 1280.
8. If `strafe` (g_game.c:278-294): `key_right` adds `sidemove[speed]`,
   `key_left` subtracts it; `joyxmove > 0` adds, `< 0` subtracts.
9. Else (g_game.c:295-305): `key_right`: `cmd->angleturn -= angleturn[tspeed];`
   `key_left`: `+=`; `joyxmove > 0`: `-=`; `joyxmove < 0`: `+=`. Each is a
   compound assignment on a `short` (computed in int, converted back).
10. `key_up`: `forward += forwardmove[speed]`; `key_down`: `-=`
    (g_game.c:307-316).
11. `joyymove < 0`: `forward +=`; `> 0`: `-=` (g_game.c:317-320).
12. `key_straferight`: `side += sidemove[speed]`; `key_strafeleft`: `-=`
    (g_game.c:321-324). These apply whether or not `strafe` is on.
13. `cmd->chatchar = HU_dequeueChatChar();` (g_game.c:328; hu_stuff.c:600-615
    returns 0 when the queue is empty, which is always in single player, 1.1.2
    fact 12).
14. `if (gamekeydown[key_fire] || mousebuttons[mousebfire] ||
    joybuttons[joybfire]) cmd->buttons |= BT_ATTACK;` (g_game.c:330-332).
15. `if (gamekeydown[key_use] || joybuttons[joybuse]) { cmd->buttons |= BT_USE;
    dclicks = 0; }` (g_game.c:334-339). Only `dclicks` is cleared, not
    `dclickstate` or `dclicks2`.
16. Weapons (g_game.c:341-348): `for (i=0 ; i<NUMWEAPONS-1 ; i++) if
    (gamekeydown['1'+i]) { cmd->buttons |= BT_CHANGE; cmd->buttons |=
    i<<BT_WEAPONSHIFT; break; }`. Keys `'1'`..`'8'` give weapon numbers 0..7;
    the lowest held digit wins; `'9'` is never tested. Key `'8'` yields
    `BT_CHANGE | 7<<3` = 0x3c.
17. `if (mousebuttons[mousebforward]) forward += forwardmove[speed];`
    (g_game.c:351-352).
18. Forward double click (g_game.c:354-376), exactly:
    ```c
    if (mousebuttons[mousebforward] != dclickstate && dclicktime > 1 ) {
        dclickstate = mousebuttons[mousebforward];
        if (dclickstate) dclicks++;
        if (dclicks == 2) { cmd->buttons |= BT_USE; dclicks = 0; }
        else dclicktime = 0;
    } else {
        dclicktime += ticdup;
        if (dclicktime > 20) { dclicks = 0; dclickstate = 0; }
    }
    ```
    `dclickstate` holds the raw stored button value (4 for the default button
    index 2), compared against the same raw value. On a successful double click
    `dclicktime` is *not* reset. After the timeout `dclickstate` is forced to 0
    even if the button is still held, so the next tic counts a fresh press.
19. Strafe double click (g_game.c:378-401): `bstrafe = mousebuttons[mousebstrafe]
    || joybuttons[joybstrafe];` (a 0/1 value), then the same machine with
    `dclickstate2`, `dclicks2`, `dclicktime2`, also giving `BT_USE`.
20. `forward += mousey;` (g_game.c:405).
21. `if (strafe) side += mousex*2; else cmd->angleturn -= mousex*0x8;`
    (g_game.c:406-409).
22. `mousex = mousey = 0;` (g_game.c:411).
23. Clamps (g_game.c:413-420): `forward` and `side` are each clamped to
    `[-MAXPLMOVE, MAXPLMOVE]`. **Both use `forwardmove[1]` (50)**, so side is
    clamped at 50, not 40.
24. `cmd->forwardmove += forward; cmd->sidemove += side;` (g_game.c:422-423):
    `char` = `(char)(0 + forward)`; in range (|value| <= 50) without turbo.
25. Special events (g_game.c:425-436): `if (sendpause) { sendpause = false;
    cmd->buttons = BT_SPECIAL | BTS_PAUSE; }` then `if (sendsave) { sendsave =
    false; cmd->buttons = BT_SPECIAL | BTS_SAVEGAME |
    (savegameslot<<BTS_SAVESHIFT); }`. These **replace** the buttons (0x81, or
    0x82 | slot<<2) and keep the movement fields. `sendsave` is set only by
    `G_SaveGame` (g_game.c:1267), reached from the menu.

**Narrowing in the strict build** (gcc x86-64, `-fwrapv`): int to `short`
(steps 9 and 21) and int to `char` (step 24) wrap modulo 2^16 and 2^8 (C calls
the result implementation-defined; gcc documents wrapping; the probe gives
`(short)(0 - 40000) == 25536` and `(char)200 == -56`). `mousex*0x8`,
`mousex*2`, `forward += mousey` and G_Responder's `data2*(s+5)` are int
operations that overflow only for absurd motion values (wrapping under
`-fwrapv`, undefined otherwise).

#### 1.1.5 The set of commands G_BuildTiccmd can produce (single player, turbo off, no joystick)

Fixed fields: `chatchar = 0`, `consistancy = 0` (steps 2 and 13). Write
`S = speed` and `T = strafe` (0/1 each, one value per tic for all fields).

**Buttons.** Without Pause or a menu save: any combination of `BT_ATTACK`,
`BT_USE`, and at most one `BT_CHANGE | w<<3` with `w` in 0..7. So `buttons &
0xc0 == 0`, and the weapon bits are non-zero only with `BT_CHANGE`; `BT_CHANGE`
with `w == 0` (key `'1'`) is possible and `w == 7` (key `'8'`) is possible.
`BT_SPECIAL` appears only via `sendpause` (0x81) or `sendsave` (0x82 |
slot<<2), each replacing all other buttons.

**A. Keyboard only** (linuxdoom without `-grabmouse` delivers no mouse events at
all, 1.1.7):

| Field | T = 0 | T = 1 |
| --- | --- | --- |
| forward | S=0: {-25, 0, 25}; S=1: {-50, 0, 50} | same |
| side | S=0: {-24, 0, 24}; S=1: {-40, 0, 40} | S=0: {-48, -24, 0, 24, 48}; S=1: {-50, -40, 0, 40, 50} (80 clamps to 50) |
| angleturn | {0, ±320} while turnheld < 6; then S=0: ±640, S=1: ±1280 | 0 |

Impossible single commands include: |forward| = 25 with |side| = 40 or
|angleturn| = 1280; |forward| = 50 with |side| in {24, 48} or |angleturn| = 640;
angleturn != 0 with |side| in {48, 50}. Sequence constraints: ±640/±1280 need
the turn input held for the previous five tics (turnheld counts left/right
keys even while strafing, step 6).

**B. Plus mouse buttons** (no motion): `mousebfire` adds `BT_ATTACK`,
`mousebstrafe` sets T, `mousebforward` adds `forwardmove[S]` (so S=0 can give
forward 50 via up + mouse-forward, and 0 via down + mouse-forward), and either
double click adds `BT_USE`.

**C. Plus mouse motion**, at the level of G_Responder (any int data2/data3):
- forward: any integer in [-50, 50].
- T = 1: angleturn = 0; side any **even** integer in [-50, 50] (24, 40,
  `2*mousex` and the clamp 50 are all even, so odd side values are impossible).
- T = 0: side in {-40, -24, 0, 24, 40}; angleturn any multiple of 8 in
  [-32768, 32760] (320, 640, 1280 and `8*mousex` are multiples of 8, and the
  2^16 wrap keeps that).
- Joint rule: angleturn != 0 implies side in {0, ±24, ±40}.

Through the linuxdoom X layer at the default sensitivity 5, the reachable set is
narrower (inference from facts 19 and 1.1.7 fact 33): data2/data3 are multiples
of 4, so `mousex*8` is a multiple of 32 and angleturn stays a multiple of 32;
side under strafe is a multiple of 8 in [-48, 48] or ±50; forward still covers
every integer in [-50, 50] (25 + 4m and 50 + 4m cover all residues mod 4).

**"Always run".** The speed term is only `gamekeydown[key_speed] ||
joybuttons[joybspeed]` (g_game.c:258). This source has no setting that makes
speed permanently on: there is no "always run" default, and `joybuttons[]` is
never written because nothing posts `ev_joystick`. The only genuine realisation
is holding `key_speed` (Shift) on every tic, which gives S = 1 on every
command. Even then the first five tics of a turn are slow (step 7), because
`tspeed` is 2 regardless of speed. (A `joyb_speed` value outside -1..3 in a
config file would index past `joyarray`; that is undefined behaviour, not a
feature of this source.)

#### 1.1.6 Default bindings and input settings

`defaults[]` (m_misc.c:234-299) is applied by `M_LoadDefaults`
(m_misc.c:340-403): every location is set to its default (m_misc.c:351-354),
then the file named by `-config <file>` (m_misc.c:357-362) or else
`basedefault` is read. On Linux `basedefault` is `$HOME/.doomrc`
(d_main.c:611-614; `-shdev`/`-regdev`/`-comdev` use a dev path,
d_main.c:624-654; `-cdrom` uses `c:/doomdata/default.cfg`, d_main.c:877-882).
Each line is `name value`; values are parsed with `%x` after `0x` or `%i`
(m_misc.c:384-387) and stored with **no range check** (m_misc.c:388-393).
`M_LoadDefaults` runs at d_main.c:1015. No other command-line parameter sets
these values.

The Makefile compiles with `-DNORMALUNIX -DLINUX` (Makefile:9), so the
`#ifdef NORMALUNIX` key block (m_misc.c:242-259) and the `#ifdef LINUX` mouse
device strings (m_misc.c:263-266) are compiled. There is **no DOS `#else`
branch** in this file: the key bindings exist only under NORMALUNIX, and the
`scantranslate`/`untranslated` fields and `scantokey` (m_misc.c:230-231, 338)
are never read by `M_LoadDefaults`.

| Name | Variable | Default | Meaning |
| --- | --- | --- | --- |
| key_right | key_right | `KEY_RIGHTARROW` 0xae | doomdef.h:250 |
| key_left | key_left | `KEY_LEFTARROW` 0xac | doomdef.h:251 |
| key_up | key_up | `KEY_UPARROW` 0xad | doomdef.h:252 |
| key_down | key_down | `KEY_DOWNARROW` 0xaf | doomdef.h:253 |
| key_strafeleft | key_strafeleft | `','` 0x2c | |
| key_straferight | key_straferight | `'.'` 0x2e | |
| key_fire | key_fire | `KEY_RCTRL` 0x9d | doomdef.h:277 |
| key_use | key_use | `' '` 0x20 | |
| key_strafe | key_strafe | `KEY_RALT` 0xb8 | doomdef.h:278 (`KEY_LALT` is the same, doomdef.h:280) |
| key_speed | key_speed | `KEY_RSHIFT` 0xb6 | doomdef.h:276 |
| use_mouse | usemouse | 1 | declared m_misc.c:169; never read anywhere else in this source |
| mouseb_fire | mousebfire | 0 | left button |
| mouseb_strafe | mousebstrafe | 1 | middle button |
| mouseb_forward | mousebforward | 2 | right button |
| mouse_sensitivity | mouseSensitivity | 5 | menu keeps 0..9 (m_menu.c:1118-1123); config unbounded |
| use_joystick | usejoystick | 0 | never read elsewhere |
| joyb_fire / joyb_strafe / joyb_use / joyb_speed | joyb* | 0 / 1 / 3 / 2 | joystick never posted |
| screenblocks | screenblocks | 9 | |
| detaillevel | detailLevel | 0 | |
| show_messages | showMessages | 1 | |
| usegamma | usegamma | 0 | |

(m_misc.c:236, 239, 244-253, 268-281, 286.) Weapon keys `'1'`..`'8'`, Pause,
Esc, Enter, Tab, `-`, `=` and the F-keys are hard-coded, not bindable.

#### 1.1.7 The X translation (i_video.c)

26. **xlatekey** (i_video.c:97-161) reads `XKeycodeToKeysym(X_display,
    keycode, 0)` (i_video.c:102): **index 0 only**, so Shift and other modifiers
    never change the keysym. Mappings: Left/Right/Down/Up to the `KEY_*ARROW`
    codes; Escape to 27; Return to 13; Tab to 9; F1-F12 to `KEY_F1`..`KEY_F12`;
    BackSpace **and Delete** to `KEY_BACKSPACE` 127; Pause to `KEY_PAUSE` 0xff;
    `KP_Equal` and `equal` to `KEY_EQUALS`; `KP_Subtract` and `minus` to
    `KEY_MINUS`; Shift_L and Shift_R to `KEY_RSHIFT`; Control_L and Control_R to
    `KEY_RCTRL`; Alt_L, Meta_L, Alt_R, Meta_R to `KEY_RALT` (i_video.c:104-150).
    Default (i_video.c:152-157): keysyms 0x20..0x7e pass through unchanged
    (`rc - XK_space + ' '` is the identity), then `'A'..'Z'` are lowered to
    `'a'..'z'`. **Every other keysym is returned raw**: keysyms >= 256 (keypad
    digits, Home, Insert, Caps Lock, Super, KP_Enter...) are ignored by the game
    switch (`data1 < NUMKEYS`) but still eaten there and still fed to the cheat
    matcher as `(char)data1`; Latin-1 keysyms 0xa0..0xff would alias `KEY_*`
    codes (for example 0xac..0xaf the arrows, 0xb6 RSHIFT, 0xb8 RALT, 0xbb..0xc4
    F1..F10, 0xff PAUSE). (The keysym values are the X11 protocol's Latin-1
    keysyms, which equal ISO 8859-1 code points; that is external knowledge, not
    from this repository, and whether a layout produces them at index 0 is
    inference.) There is no separate left/right Shift, Ctrl or Alt.
27. **KeyPress / KeyRelease** (i_video.c:203-214): `ev_keydown` / `ev_keyup`
    with `data1 = xlatekey()`; `data2`/`data3` of the local `event` are **left
    uninitialised** (no responder reads them for key events).
28. **ButtonPress** (i_video.c:215-227): `data1` = bits of buttons 1-3 held in
    `xbutton.state` (before the press) OR the pressed button (Button1 1,
    Button2 2, Button3 4); `data2 = data3 = 0`. Any other button (wheel 4/5, an
    inference about modern servers) posts the held-button bits with zero motion.
29. **ButtonRelease** (i_video.c:228-243): the held bits XOR the released
    button; `data2 = data3 = 0`.
30. **MotionNotify** (i_video.c:244-268): `data1` = held bits;
    `data2 = (X_event.xmotion.x - lastmousex) << 2;`
    `data3 = (lastmousey - X_event.xmotion.y) << 2;` (up is positive). Only if
    either is non-zero: `lastmousex/lastmousey` are updated to the event position,
    and the event is posted only if `x != X_width/2 && y != X_height/2`; otherwise
    `mousemoved = true` and nothing is posted. So a real motion whose x **or** y
    lands exactly on the centre line is dropped (its delta is lost because
    `lastmouse` was already updated). `mousemoved` is never read elsewhere.
31. **Expose / ConfigureNotify** do nothing (i_video.c:270-272); the default case
    only records MIT-SHM completion (i_video.c:274-276).
32. **I_StartTic** (i_video.c:309-338): drains `XPending` with `I_GetEvent`; then
    if `grabMouse`, `if (!--doPointerWarp)` warps the pointer to the window
    centre and resets the countdown to `POINTER_WARP_COUNTDOWN` = 1
    (i_video.c:60, 321-333), i.e. it warps on **every** call. The warp's own
    MotionNotify arrives at the centre, so it only resets `lastmouse` and is not
    posted.
33. **Mouse needs `-grabmouse`.** The window's event mask is
    `KeyPressMask | KeyReleaseMask | ExposureMask`, with pointer motion and
    buttons commented out (i_video.c:798-802). Pointer events arrive only through
    `XGrabPointer(..., ButtonPressMask|ButtonReleaseMask|PointerMotionMask, ...,
    confine_to = X_mainWindow, ...)` when `grabMouse = !!M_CheckParm("-grabmouse")`
    (i_video.c:738, 847-851). Without it, linuxdoom produces **no `ev_mouse` at
    all**, whatever `use_mouse` says.
34. **Auto-repeat and focus.** There is no `XAutoRepeatOff`, no detectable
    auto-repeat request, no `FocusChangeMask` and no FocusIn/FocusOut handling
    anywhere in i_video.c. Inference about X behaviour: held keys auto-repeat as
    extra KeyPress (and, without detectable auto-repeat, KeyRelease+KeyPress
    pairs); a release that lands in one tic's batch and its press in the next
    would clear `gamekeydown` for one command and reset `turnheld`, restarting the
    slow turn; repeats of Pause would toggle pause repeatedly. A key released while
    the window lacks focus is never seen, so it stays down in `gamekeydown`.
35. **Streams this layer can produce.** Keys: one event per X key event, data1 in
    0..255 or a raw keysym >= 256, never negative. Mouse (only with -grabmouse):
    one `ev_mouse` per qualifying MotionNotify and per button event, so several
    per tic. Joystick: never. Combined with fact 19 (assignment), **only the last
    `ev_mouse` processed before a G_BuildTiccmd counts**; earlier motion in the
    same batch is discarded, and a button event after motion zeroes it. This is
    confirmed from the code (g_game.c:579-580, 405-411; i_video.c:244-268), not
    measured. The DOS-style alternative, one `ev_mouse` per tic carrying the
    accumulated dx/dy and the current buttons, is an equally legal input to the
    same G_Responder: it yields `mousex = (sum of deltas)*(s+5)/10`. The DOS
    driver itself (i_ibm.c) is not in this source, so its exact stream cannot be
    cited here.

#### 1.1.8 -turbo

36. Parsed at d_main.c:884-898, before `M_LoadDefaults` (d_main.c:1015): scale
    defaults to 200 when `-turbo` is present; if any argument follows,
    `scale = atoi(myargv[p+1])` even if it is another option (so `-turbo -file x`
    gives atoi = 0, clamped to 10); clamped to [10, 400]; then
    `forwardmove[0..1]` and `sidemove[0..1]` are multiplied by scale/100 with int
    truncation. Without `-turbo` the tables are untouched (off by default).
    Because `MAXPLMOVE` is `forwardmove[1]`, turbo also raises the clamp; above
    scale 254 `forwardmove[1]` exceeds 127 and the `char` assignment wraps.
37. Cross-reference: G_Ticker prints "%s is turbo!" when `cmd->forwardmove >
    TURBOTHRESHOLD && !(gametic&31) && ((gametic>>5)&3) == i`
    (g_game.c:671-679). Without turbo forward never exceeds 50, so it never
    fires.

#### 1.1.9 Undefined, implementation-defined and uninitialised behaviour

38. `eventhead = (++eventhead)&(MAXEVENTS-1);` (d_main.c:153) and the same for
    `eventtail` (d_main.c:170, d_net.c:464) modify the variable twice without a
    sequence point: undefined in C89/C99. gcc warns (`-Wsequence-point`) and in
    the probe computes the intended `(x+1) & 63` at -O0 and -O2.
39. `(x - lastmousex) << 2` with a negative left operand (i_video.c:249-250):
    undefined in C99; gcc defines it as multiplication by 4 (probe: `-3<<2 ==
    -12`).
40. Negative division in `data2*(s+5)/10` (g_game.c:579-580): implementation-
    defined in C89, toward zero in gcc (1.1.3 fact 19).
41. Narrowing int to `short`/`char` (g_game.c:298-304, 409, 422-423), int to
    `char` for the cheat matcher (`cht_CheckCheat(..., char key)`,
    m_cheat.c:43-45) and to `unsigned char` in HU_Responder (hu_stuff.c:704,
    defined modulo 256).
42. Out-of-range indices: config-loaded `key_*` outside 0..255, `mouseb_*`
    outside -1..2, `joyb_*` outside -1..3 (no validation, m_misc.c:388-393).
43. `sizeof(mousebuttons)` is the pointer size (g_game.c:495-496): a
    platform-dependent partial clear.
44. Uninitialised `data2`/`data3` in key events (i_video.c:199-214); never read.
45. First call: every variable G_BuildTiccmd reads is either a zero-initialised
    static or assigned before use; no uninitialised read.

#### 1.1.10 Hazards for an exact port of the input path

- Mouse motion is **assigned** per event, and a button event zeroes it; a port
  that accumulates motion, or reorders button and motion events, produces
  different commands than linuxdoom's X stream.
- `side` is clamped by `forwardmove[1]` (50), not `sidemove[1]`.
- The slow turn (320 for turnheld 1..5) applies with speed held; `turnheld`
  counts left/right keys while strafing, and is not reset at level start.
- The double-click machines are subtle (no `dclicktime` reset on success, the
  timeout forces `dclickstate` to 0 while held, the use key clears only
  `dclicks`); port them line by line.
- `mousebuttons[2]` survives the level-start clear on x86-64.
- One translated code for both Shifts, both Ctrls, both Alts: releasing one
  releases the action while the other is still held.
- Several keys are consumed ahead of the game (1.1.2 fact 15); Pause yields
  `BT_SPECIAL`, which the wire-24 simulation refuses.
- Truncation toward zero in the sensitivity scaling; wrap on narrowing.
- The cheat matcher sees every keydown in vanilla; omitting it is a scope choice.

#### 1.1.11 Questions the input facts raise for the owner

1. **Mouse stream.** Options: (a) linuxdoom's per-event stream, last event
   wins (genuine but loses motion); (b) one accumulated event per tic (a legal
   G_Responder input, DOS-like, not citable here); (c) no mouse by default, as
   linuxdoom without `-grabmouse`. Facts: 1.1.3 fact 19, 1.1.7 facts 30-35.
2. **Keys consumed ahead of the game.** For Esc, F1-F12, `-`/`=`, Enter, Tab and
   Pause: drop them, or implement the in-scope genuine effects (screen size,
   message refresh, gamma)? Pause cannot become a command (BT_SPECIAL refused).
   Facts: 1.1.2.
3. **Always run and WASD presets.** This source has no always-run setting; a
   preset is genuine only if it maps onto the same G_BuildTiccmd inputs (for
   example speed = "Shift held every tic", including the slow turn). Accept that
   framing, or treat presets as a declared departure? Facts: 1.1.5.
4. **Auto-repeat and focus loss.** Reproduce X's behaviour (repeats, stuck keys
   after focus loss) or filter repeats and release all keys on focus loss
   (deterministic, but not linuxdoom)? Facts: 1.1.7 fact 34.
5. **Config file.** Read `~/.doomrc`-style defaults with linuxdoom's lack of
   validation, or validate (refuse out-of-range indices)? Facts: 1.1.6, 1.1.9
   fact 42.
6. **Layout-independent keys.** linuxdoom uses keysym column 0, so key meaning
   depends on the keyboard layout; use physical keys instead? Facts: 1.1.7
   fact 26.

Each question is taken up in §4: 1 in S4; 2 in S5; 3 in S7; 4 in S6; 5 in S7 (no configuration file is read in M5b); 6 in S6.

### 1.2 Recording: G_BeginRecording, G_WriteDemoTiccmd and the re-read

All citations are `file:line` in `linuxdoom-1.10/` at the authority commit, except
where a Rust path under `crates/` is named. "Inference" marks anything not read
directly from a line or observed by a run. Two small probes were compiled and run
under `/home/copgunn/scratch/m5b/probe-loop/` (ext4, outside the repository) with
`gcc (Ubuntu 15.2.0-16ubuntu1) 15.2.0`; each is described where it is used.

#### 1.2.1 The `-record` path, from the command line to the first header byte

1. `D_DoomMain` reads the flags `-nomonsters`, `-respawn`, `-fast` and `-devparm` with
   `M_CheckParm` (d_main.c:808-811) and `-altdeath`/`-deathmatch` (812-815). Defaults are
   `startskill = sk_medium`, `startepisode = 1`, `startmap = 1`, `autostart = false`
   (961-964). `-skill N` sets `startskill = N - '1'`, so the argument is one more than the
   `skill_t` value (970). `-episode` (974-980) and `-warp E M` (997-1008) set episode and map.
   Each of these sets `autostart = true`.
2. `-record NAME` calls `G_RecordDemo (myargv[p+1])` and sets `autostart = true`
   (d_main.c:1127-1133).
3. `G_RecordDemo` (g_game.c:1530-1546) does the following:
   - sets `usergame = false` (1535). `G_InitNew` sets it back to `true` later (1443).
   - builds the name with `strcpy (demoname, name); strcat (demoname, ".lmp")` (1536-1537)
     into `char demoname[32]` (127). There is no length check, so a name longer than 27
     characters overflows the buffer.
   - sets `maxsize = 0x20000` (131072 bytes, 1538). `-maxdemo K` replaces it with
     `atoi(K)*1024` (1539-1541).
   - calls `demobuffer = Z_Malloc (maxsize, ...)`, sets `demoend = demobuffer + maxsize`
     (1542-1543) and sets `demorecording = true` (1545).
4. Unless `-playdemo`, `-timedemo` or `-loadgame` intervene, `D_DoomMain` calls
   `G_InitNew (startskill, startepisode, startmap)` directly (d_main.c:1161-1165).
   `G_InitNew` clamps skill, episode and map (g_game.c:1379-1412), calls `M_ClearRandom`
   (1414), sets every `playerstate = PST_REBORN` (1440-1441), sets `gameskill`, `gameepisode`
   and `gamemap` (1448-1450), and ends with a direct call to `G_DoLoadLevel` (1481). The
   level is therefore loaded synchronously before the main loop starts, at `gametic == 0`.
5. `D_DoomMain` then calls `D_DoomLoop` (d_main.c:1170). Its first statement is
   `if (demorecording) G_BeginRecording ();` (d_main.c:356-357). This comes **after**
   `G_InitNew`/`G_DoLoadLevel` and **before** `I_InitGraphics` (367) and before any tic.
   The header therefore holds the clamped values.
6. `G_BeginRecording` (g_game.c:1549-1567) rewinds `demo_p = demobuffer` and writes 13
   bytes:

| Offset | Source line | Expression | Value for a single-player game, skill_t S, E1, map M, no flags |
|---|---|---|---|
| 0 | 1555 | `VERSION` | 110 (doomdef.h:33) |
| 1 | 1556 | `gameskill` | S (0..4); `-skill N` gives S = N-1 |
| 2 | 1557 | `gameepisode` | 1 |
| 3 | 1558 | `gamemap` | M |
| 4 | 1559 | `deathmatch` | 0 |
| 5 | 1560 | `respawnparm` | 0 |
| 6 | 1561 | `fastparm` | 0 |
| 7 | 1562 | `nomonsters` | 0 |
| 8 | 1563 | `consoleplayer` | 0 (i_net.c:289, d_net.c:573) |
| 9-12 | 1565-1566 | `playeringame[0..3]` | 1, 0, 0, 0 (d_net.c:586-587 with `numplayers = 1`, i_net.c:287) |

   Quirk: `boolean` is a C `enum {false, true}` (doomtype.h:34), and `M_CheckParm` returns
   the matching argv index, not 1 (m_argv.c:41-51). When `-nomonsters`, `-respawn` or `-fast`
   is given, its header byte is that argv index (for example 3), not 1. Playback only tests
   these values for truth. The Rust reader requires 0 (§1.2.6).

#### 1.2.2 `G_WriteDemoTiccmd` and the re-read

The text (g_game.c:1506-1523):

```c
if (gamekeydown['q'])  G_CheckDemoStatus ();        // 1508-1509
*demo_p++ = cmd->forwardmove;                        // 1510
*demo_p++ = cmd->sidemove;                           // 1511
*demo_p++ = (cmd->angleturn+128)>>8;                 // 1512
*demo_p++ = cmd->buttons;                            // 1513
demo_p -= 4;                                         // 1514
if (demo_p > demoend - 16) { G_CheckDemoStatus (); return; }  // 1515-1520
G_ReadDemoTiccmd (cmd);   // make SURE it is exactly the same    1522
```

1. **The 'q' key** (1508-1509). If `gamekeydown['q']` is set, `G_CheckDemoStatus` runs
   before this tic's bytes are written. With `demorecording` set, it writes `DEMOMARKER`
   at the current `demo_p` (1679), calls `M_WriteFile` (1680) and ignores its return value
   (m_misc.c:113-133 returns `false` on failure). It then frees the buffer, clears
   `demorecording` (1681-1682) and calls `I_Error ("Demo %s recorded")` (1683), which
   exits the process with status -1 (i_system.c:182). The tic on which 'q' was seen is not
   in the demo. Whether 'q' reaches `gamekeydown` depends on the responder chain
   (g_game.c:544-567). That chain belongs to the input investigation.
2. **The four bytes.** `demo_p` is a `byte*` (g_game.c:132; `byte` is `unsigned char`).
   `ticcmd_t` is `char forwardmove; char sidemove; short angleturn; short consistancy;
   byte chatchar; byte buttons` (d_ticcmd.h:36-44).
   - `forwardmove` and `sidemove` are plain `char`, which is signed on x86-64 gcc (the probe
     printed `char signed: 1`). Storing them into a byte keeps the value modulo 256.
   - `(cmd->angleturn+128)>>8`: `angleturn` is promoted to `int`, so `+128` cannot overflow
     (the largest value is 32767+128 = 32895). The `>>8` of a negative `int` is
     implementation-defined in C. gcc defines it as an arithmetic shift, so the expression
     equals ⌊(a+128)/256⌋. That is round to nearest, with an exact half (a ≡ 128 mod 256)
     rounding **up** toward +∞. The `int` result is stored into a byte modulo 256. `-fwrapv`
     changes nothing here: there is no signed overflow, only an implementation-defined shift
     and conversion.
   - `buttons` is stored unchanged.
3. **The re-read.** `demo_p -= 4` (1514) moves back over the four bytes just written. If
   the buffer still has room, `G_ReadDemoTiccmd (cmd)` (1522) reads those same four bytes
   back into the **same** `cmd` and advances `demo_p` by 4 again (1498-1501). The net
   advance per tic is 4.
4. **What the executed command is.** `G_Ticker` sets `cmd = &players[i].cmd`, copies the
   built command into it from `netcmds[i][buf]` (g_game.c:662-664), and calls
   `G_WriteDemoTiccmd (cmd)` on that pointer (668-669). Later in the same `G_Ticker`,
   `P_Ticker` (730) calls `P_PlayerThink` (p_tick.c:148-150), which reads
   `cmd = &player->cmd` (p_user.c:248). `P_MovePlayer` applies
   `player->mo->angle += (cmd->angleturn<<16)` from the same structure (p_user.c:148-154).
   Nothing writes `players[i].cmd` between 669 and 730, apart from the turbo check, which
   only reads it (672-679). **While recording, the game therefore executes the re-read
   command.** A recorded session turns only by multiples of 256 angleturn units (the high
   byte). The copy in `netcmds` keeps the unrounded value but is never executed.
5. **What `G_ReadDemoTiccmd` writes back** (g_game.c:1491-1503):

| Field | Statement | Effect while recording |
|---|---|---|
| `forwardmove` | `= ((signed char)*demo_p++)` (1498) | identity for -128..127 |
| `sidemove` | `= ((signed char)*demo_p++)` (1499) | identity for -128..127 |
| `angleturn` | `= ((unsigned char)*demo_p++)<<8` (1500) | an `int` 0..65280 stored into a `short`. Values ≥ 32768 wrap to negative (implementation-defined; gcc wraps modulo 2^16). The result is `256 * ⌊(a+128)/256⌋`, wrapped into -32768..32512 |
| `buttons` | `= (unsigned char)*demo_p++` (1501) | identity |
| `consistancy`, `chatchar` | not touched | keep the values `G_BuildTiccmd` built |

6. **The marker test inside the re-read.** `G_ReadDemoTiccmd` first tests
   `*demo_p == DEMOMARKER` (0x80, g_game.c:1488, 1493). The byte it tests is the
   `forwardmove` just written. A recorded `forwardmove` of -128 would therefore end the
   recording through `G_CheckDemoStatus` and `I_Error`. `G_BuildTiccmd` cannot produce
   -128: `forward` is clamped to ±`MAXPLMOVE` = ±0x32 = ±50 (g_game.c:171, 175, 413-416)
   and added to the zeroed `emptycmd` (i_system.c:64-68, g_game.c:249-250, 422).
7. **The buffer limit** (1515-1520). The test runs after the rewind, so `demo_p` points at
   this tic's first byte. The tic at offset 13+4k is refused when 13+4k > maxsize-16. On
   that tic `G_CheckDemoStatus` writes the marker over its `forwardmove` byte, writes the
   file and calls `I_Error`, so the game exits. The number of tics recorded is
   ⌊(maxsize-29)/4⌋+1. With the default 0x20000 that is 32,761 tics (about 15 min 36 s at
   35 Hz), and the file is 13 + 4·32,761 + 1 = 131,058 bytes (arithmetic from the lines
   above).
8. **Other ways a recording ends.** `G_CheckDemoStatus` is also reached from `I_Error`
   (i_system.c:176-177) and from a network exit packet (d_net.c:292-293). `I_Quit`
   (i_system.c:116-124) does **not** call it. Quitting from the menu, or the SIGINT handler
   installed as `I_Quit` (i_video.c:717), therefore loses the demo without writing a file.
   A level exit or a death does not end recording: recording continues through the
   intermission and later levels.
9. **A command changed after the re-read.** `P_PlayerThink` overwrites the executed
   command when `MF_JUSTATTACKED` is set: `angleturn = 0`, `forwardmove = 0xc800/512`
   (100), `sidemove = 0` (p_user.c:249-255). This happens after recording, so the demo holds
   the player's command and the override is replayed by the same rule at playback.

#### 1.2.3 `G_Ticker` for one tic of a single-player level (g_game.c:605-748)

1. Reborns: `PST_REBORN` players get `G_DoReborn` (612-614).
2. `while (gameaction != ga_nothing)` runs the switch (617-652):

   | Action | Call |
   |---|---|
   | `ga_loadlevel` | `G_DoLoadLevel` (621-623) |
   | `ga_newgame` | `G_DoNewGame` (624-626) |
   | `ga_loadgame` | `G_DoLoadGame` (627-629) |
   | `ga_savegame` | `G_DoSaveGame` (630-632) |
   | `ga_playdemo` | `G_DoPlayDemo` (633-635) |
   | `ga_completed` | `G_DoCompleted` (636-638) |
   | `ga_victory` | `F_StartFinale` (639-641) |
   | `ga_worlddone` | `G_DoWorldDone` (642-644) |
   | `ga_screenshot` | `M_ScreenShot`, then `ga_nothing` (645-648) |
   | `ga_nothing` | nothing (649-650) |

3. `buf = (gametic/ticdup)%BACKUPTICS` (656). For each player in the game, the command is
   copied from `netcmds` into `players[i].cmd` (662-664). Then `if (demoplayback)
   G_ReadDemoTiccmd` (666-667), then `if (demorecording) G_WriteDemoTiccmd` (668-669).
4. Turbo check: `forwardmove > TURBOTHRESHOLD` (0x32, 173) on a tic with `!(gametic&31)`
   and `((gametic>>5)&3) == i` sets the "is turbo!" message (672-679). It uses the re-read
   value, which is identical to the built one (§1.2.2 item 5).
5. Consistency: only `if (netgame && !netdemo && !(gametic%ticdup))` (681-693). In single
   player nothing writes `consistancy[][]`. It is a static array (140), so it stays 0, and
   `G_BuildTiccmd` copies that 0 into `cmd->consistancy` (252-253).
6. `BT_SPECIAL` (128, d_event.h:80): `BTS_PAUSE` toggles `paused` (706-712). `BTS_SAVEGAME`
   sets `ga_savegame` (714-720). These are produced only by `sendpause`/`sendsave` in
   `G_BuildTiccmd` (426-436), which **replaces** all other buttons. `sendpause` comes from
   `KEY_PAUSE` in `G_Responder` (561-565).
7. `switch (gamestate)`: for `GS_LEVEL` the order is `P_Ticker ()`, `ST_Ticker ()`,
   `AM_Ticker ()`, `HU_Ticker ()` (727-734). `P_Ticker` returns early if `paused`, or in
   single player while the menu is active once the player has a `viewz` (p_tick.c:135-145).

#### 1.2.4 How a level ends and how death proceeds (cross-reference)

1. Exits call `G_ExitLevel` (`secretexit = false; gameaction = ga_completed`,
   g_game.c:1002-1006) or `G_SecretExitLevel` (1009-1018). Callers include p_spec.c:679,
   762, 1062, 1096, p_switch.c:362, 434 and p_enemy.c:1755, 1898. The call happens inside
   `P_Ticker` of tic t, and the rest of tic t still runs, including `ST_Ticker`, `AM_Ticker`
   and `HU_Ticker`. The frame displayed after tic t is still `GS_LEVEL`.
2. The next `G_Ticker` (tic t+1) runs `G_DoCompleted` at its top (636-638). That
   function clears powers and cards (`G_PlayerFinishLevel`, 779-792), sets `ga_victory` on
   E1M8 (1033-1038), and otherwise sets `gamestate = GS_INTERMISSION; viewactive = false`
   (1133-1134) and calls `WI_Start` (1140). The same `G_Ticker` then still copies and
   records the command for tic t+1 (664-669) and runs `WI_Ticker` (736-737).
   - A vanilla recording therefore continues into the intermission.
   - A demo whose marker follows the exit tic's command is still valid vanilla input. On
     playback, `G_DoCompleted` runs at t+1, then `G_ReadDemoTiccmd` meets the marker
     (1493-1496), and `G_CheckDemoStatus` ends playback (`I_Quit` under `-playdemo`,
     1660-1661). This is inferred from those lines; it was not run.
3. Death: `P_KillMobj` sets `PST_DEAD` (p_inter.c:706). `P_PlayerThink` then calls
   `P_DeathThink` and returns (p_user.c:258-261). A command with `BT_USE` (2,
   d_event.h:77) sets `PST_REBORN` (p_user.c:227-228) during tic t.
4. At the top of the next `G_Ticker` (t+1), `G_DoReborn` sets `gameaction = ga_loadlevel`
   in single player (g_game.c:924-932). The `while` loop right below runs `G_DoLoadLevel`
   **in the same tic** (617-623).
   - `G_DoLoadLevel` reloads the map (`P_SetupLevel`, 484). The player's `PST_REBORN`
     leads to `G_PlayerReborn` in `P_SpawnPlayer` (p_mobj.c:659), so the player restarts
     with a pistol start.
   - It forces a wipe on the next display (`wipegamestate = -1`, g_game.c:472-473) and
     clears `gamekeydown`, the mouse and joystick state (491-496).
   - `M_ClearRandom` is not called, so neither random index is reset.
   - Tic t+1's command was already built, so it is still copied (664) and run by `P_Ticker`
     on the fresh level.
5. As the brief states, the Rust simulation ends a level run and refuses the reborn tic.
   That claim is not re-examined here.

#### 1.2.5 No turning remainder is carried. What small turns become.

1. `G_BuildTiccmd` (g_game.c:237-437) has no turning accumulator.
   - `cmd` starts as a copy of the zeroed `emptycmd` each time (249-250).
   - Keyboard turning adds or subtracts `angleturn[tspeed]` = 640, 1280 or 320
     (177, 298-305).
   - Mouse turning is `cmd->angleturn -= mousex*0x8` (409). Then `mousex = mousey = 0`
     (411).
   - `G_Responder` **assigns** `mousex` from each mouse event rather than adding to it
     (579).
   - The only static turning state is `turnheld` (184, 264-270), which counts held tics to
     choose the slow or fast turn speed. It does not store a remainder.
2. `G_WriteDemoTiccmd` stores only `(a+128)>>8` and keeps nothing (1506-1523). There is no
   other `demo_p` writer or remainder variable in g_game.c (grep for `angleturn` in g_game.c
   finds only 177, 299-305, 409, 1500 and 1512).
3. So the dropped low byte is simply lost each tic.
4. The probe `probe.c` copies the write statement (1510-1513) and the read body (1498-1501)
   into one program, built with `gcc -O2 -fwrapv` on x86-64. Observed results:

| angleturn built (in) | byte stored | angleturn executed and replayed (out) |
|---|---|---|
| -8 | 0x00 | 0 |
| -64 | 0x00 | 0 |
| -120 | 0x00 | 0 |
| -127 | 0x00 | 0 |
| -128 | 0x00 | 0 |
| -129 | 0xff | -256 |
| -136 | 0xff | -256 |
| -200 | 0xff | -256 |
| -256 | 0xff | -256 |
| -320 (slow key turn, right) | 0xff | -256 |
| -384 | 0xff | -256 |
| -640 (key turn, right) | 0xfe | -512 |
| -1280 (fast key turn, right) | 0xfb | -1280 |
| +8, +64, +127 | 0x00 | 0 |
| +128 | 0x01 | +256 |
| +136, +200, +320 (slow, left), +383 | 0x01 | +256 |
| +384 | 0x02 | +512 |
| +640 (key turn, left) | 0x03 | +768 |
| +1280 (fast, left) | 0x05 | +1280 |
| 32639 | 0x7f | 32512 |
| 32640 and 32767 | 0x80 | -32768 (the same angle as +32768: half a turn) |
| -32768 | 0x80 | -32768 |

The same run confirmed that `forwardmove = -50`, `sidemove = -40` and `buttons = 0x3f`
are unchanged by the round trip.

What this means:

- Every turn of magnitude up to 127 units disappears.
- -128 disappears but +128 does not.
- The normal keyboard turn is asymmetric: +768 to the left and -512 to the right.

These follow from the lines cited; the table is an observed probe result.

#### 1.2.6 The Rust demo writer and reader (`crates/doom-sim/src/demo.rs`)

1. Constants:
   - `DEMO_MARKER = 0x80` (demo.rs:19) and `DEMO_VERSION = 110` (21).
   - `DEMO_HEADER = [110, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0]` (25).
   - Map index 3 and admitted maps 1..=9 (27, 30). Skill index 1 and wire-24 skills 2..=3
     (33, 37).
   - `TicCommand` has only `forward: i8`, `side: i8`, `angle_turn: i16` and `buttons: u8`
     (crates/doom-sim/src/actor.rs:5-14). It has no consistancy or chatchar field.
2. Readers:
   - `parse_demo` requires the header to equal `DEMO_HEADER` exactly. The first byte that
     differs is refused as `Header { index, value }`, and fewer than 13 bytes gives
     `Truncated` (132-142).
   - `parse_demo_episode1` admits any map in 1..=9 that equals the caller's map (149-172).
   - `parse_demo_episode1_v2` also admits skill 2 or 3 (179-206).
   - Every other header byte (episode 1, deathmatch, respawn, fast, nomonsters,
     consoleplayer, playeringame 1,0,0,0) must equal `DEMO_HEADER`. A header written by
     vanilla `-nomonsters` (an argv index, §1.2.1) is refused.
3. Commands (`parse_commands`, 210-233):
   - A leading 0x80 byte is the marker, and any byte after it is refused as `TrailingBytes`
     (214-219). Vanilla does not check for trailing bytes.
   - Otherwise four bytes decode as `forward as i8`, `side as i8`,
     `(u16::from(turn) << 8) as i16` and `buttons` (221-227), matching g_game.c:1498-1501.
   - A partial command or a missing marker is refused as `UnterminatedCommands` (230).
   - The reader does **not** refuse buttons 0x40/0x80 or side -128. 0x80 is the marker only
     in the forward position (tested at 366-371).
4. Writers (`write_with`, 280-305):
   - `forward == -128` is refused as `MarkerForward` (288-290).
   - `side == -128` is refused as `MarkerSide` for the Episode 1 and wire-24 writers only
     (291-293, 246-253, 260-278). The original `write_demo` allows it (526).
   - Buttons with bit 0x40 or 0x80 (`BT_SPECIAL`) are refused as `Buttons` (294-296).
   - An `angle_turn` with a **nonzero low byte** is refused as `AngleTurn` (297-299). The
     writer does **not** apply the C rounding `(a+128)>>8`: it stores the high byte of a
     value that must already be a multiple of 256 (300-301). An M5b recorder must apply the
     vanilla rounding itself before both executing and writing (§1.2.2).
   - The writer appends the marker (303).
   - `write_demo_episode1_v2` refuses a map outside 1..=9 as `MapArgument` and a skill
     outside 2..=3 as `Header { index: 1 }` (265-273).

### 1.3 The loop and the display order: TryRunTics, I_GetTime, D_Display

#### 1.3.1 `D_DoomLoop` (d_main.c:354-407)

1. Before the loop: `G_BeginRecording` if recording (356-357), the optional `-debugfile`
   (359-365), then `I_InitGraphics` (367).
2. Each iteration runs in this order:
   1. `I_StartFrame ()` (372), which is empty in the X port (i_video.c:183-187).
   2. If `singletics`, the inline branch (375-386); otherwise `TryRunTics ()` (389).
   3. `S_UpdateSounds (players[consoleplayer].mo)` (392).
   4. `D_Display ()` (395).
   5. `I_UpdateSound ()` unless `SNDSERV` is defined (397-400), then `I_SubmitSound ()`
      unless `SNDINTR` is defined (402-405). The Makefile defines neither (Makefile:9), so
      both run.
3. The `singletics` branch runs `I_StartTic`, `D_ProcessEvents`, then
   `G_BuildTiccmd (&netcmds[consoleplayer][maketic%BACKUPTICS])` directly into `netcmds`,
   then `if (advancedemo) D_DoAdvanceDemo`, `M_Ticker`, `G_Ticker`, `gametic++` and
   `maketic++` (377-385). That is exactly one command and one tic per displayed frame, with
   no pacing.
4. `singletics` is `false` by default (d_main.c:102) and is set only by `G_TimeDemo`
   (g_game.c:1630). **A normal or `-record` session uses `TryRunTics`.**

#### 1.3.2 `TryRunTics` and `NetUpdate` for single player, ticdup 1

Setup facts:

- Without `-net`, `I_InitNetwork` sets `netgame = false`, `numplayers = numnodes = 1` and
  `consoleplayer = 0` (i_net.c:281-289).
- `ticdup = 1` unless `-dup` is given (255-265), and `extratics = 0` unless `-extratic` is
  given (267-270).
- `D_CheckNetGame` copies `ticdup` and marks node 0 as in the game (d_net.c:581-589).
- `BACKUPTICS` is 12 (d_net.h:49).
- The buffers are `localcmds[BACKUPTICS]` (d_net.c:60) and
  `netcmds[MAXPLAYERS][BACKUPTICS]` (62).

**`NetUpdate`** (d_net.c:368-446):

1. `nowtime = I_GetTime ()/ticdup; newtics = nowtime - gametime; gametime = nowtime`
   (377-379). If `newtics <= 0`, it jumps straight to `listen:` (381-382). **In that case
   no events are pumped.**
2. `skiptics` handling (384-393). `skiptics` is only set for a non-key player (706-709),
   so it is always 0 here.
3. The build loop (399-410): `gameticdiv = gametic/ticdup`; then
   `for (i=0; i<newtics; i++) { I_StartTic (); D_ProcessEvents (); if (maketic - gameticdiv
   >= BACKUPTICS/2-1) break; G_BuildTiccmd (&localcmds[maketic%BACKUPTICS]); maketic++; }`.
   - Events are pumped and dispatched immediately before **each** build.
   - The iteration that breaks also pumps events once, without building.
   - At most `maketic - gametic = 5` commands can be outstanding.
   - Because `gametime` was already set to `nowtime`, the tics this loop skips are dropped
     for good.
4. Unless `singletics` (413-414), it "sends" to node 0. `HSendPacket` with node 0 stores a
   rebound packet (d_net.c:149-153) carrying `localcmds[resendto..maketic)`, with
   `resendto[0] = maketic - extratics` (417-441).
5. `listen: GetPackets ()` (444-445) takes the rebound packet (194-199) and copies the new
   commands into `netcmds[0][nettics[0]%BACKUPTICS]`, advancing `nettics[0]` to `maketic`
   (340-355).
6. After each `NetUpdate` that built something, `nettics[0] == maketic`.

**`TryRunTics`** (d_net.c:636-767):

1. `entertic = I_GetTime ()/ticdup; realtics = entertic - oldentertics; oldentertics =
   entertic` (648-650). `oldentertics` is a function `static` that starts at 0.
2. `NetUpdate ()` (653).
3. `lowtic = nettics[0]` over the one node (655-665), then
   `availabletics = lowtic - gametic/ticdup` (666).
4. `counts` is decided (669-677). This is exact, including the odd middle case:

   | Condition | `counts` |
   |---|---|
   | `realtics < availabletics-1` | `realtics+1` |
   | `realtics == availabletics-1` | `realtics` (one less than available) |
   | otherwise | `availabletics` |
   | then, if `counts < 1` | 1 |

5. `frameon++` (679). The adaptation block (686-712) leaves `frameskip` and `gametime`
   alone for the key player: `consoleplayer == i` (693-696) holds in single player.
6. The wait loop (715-733): `while (lowtic < gametic/ticdup + counts)` it calls
   `NetUpdate`, recomputes `lowtic`, and checks `lowtic < gametic` (an `I_Error`). It bails
   out with `M_Ticker ()` and returns **without running a tic** if
   `I_GetTime()/ticdup - entertic >= 20` (728-732).
   - In single player the wait is entered only when `availabletics <= 0` and `counts` is 1.
   - It ends at the next clock tick, when `NetUpdate` builds one command.
   - Inference: the bail-out is unreachable in single player, because the clock that
     triggers it also produces commands, and `skiptics` is 0.
7. The run loop (736-766): for each count, and for `i < ticdup`, it runs
   `if (advancedemo) D_DoAdvanceDemo`, `M_Ticker ()`, `G_Ticker ()`, `gametic++`
   (742-746).
   - The duplicated-tic fix-up (749-763) runs only when `i != ticdup-1`, which never
     happens with ticdup 1.
   - Then `NetUpdate ()` runs after each tic (765). It may build commands for later frames;
     `counts` is already fixed.
8. Pacing: there is **no sleep** in this path. The wait loop spins on `NetUpdate` →
   `I_GetTime`. While the tic count does not advance, `NetUpdate` goes straight to `listen`
   and does not pump X events.

**Slow frames.** `D_Display` itself calls `NetUpdate` after drawing and before the blit
(d_main.c:316). Commands for time spent rendering are therefore built at that point, with
events pumped first. From the lines above:

| Frame took | Commands built (in `D_Display`'s `NetUpdate`) | Tics run in the next `TryRunTics` | Real tics lost |
|---|---|---|---|
| under 1 tic, steady state | 0, then 1 in the wait loop | 1 | 0 |
| about 1 tic every frame | 1 | 1 (that command was built during the previous display, so the input is one tic older) | 0 |
| 3 tics | 3 (3 pumps) | 3, all before the next display | 0 |
| 30 tics | 5, plus 1 pump without a build (6 pumps) | 5 | 25, never recovered |

- At most 5 tics ever run per displayed frame (cap at d_net.c:404). Time beyond that is
  dropped, and the game simply runs slower than real time.
- All mouse motion delivered before a build lands in that one build. `mousex` is assigned
  per event (g_game.c:579) and zeroed per build (411), so a burst of motion during a long
  frame becomes, at most, one large turn on the first command of the batch. The later
  commands in the batch see only events that arrive between builds, usually none.
- Keys are level-triggered (`gamekeydown`), so a held key appears in every command of the
  batch.

A second probe, `loop.c`, transcribes `TryRunTics` and `NetUpdate` for one node with
ticdup 1 and a fake clock. Its declared adapter replaces the rebound packet with
`nettics0 = maketic`, which is what d_net.c:149-153, 194-199 and 340-355 amount to for
node 0. Observed output for render costs 0,0,0,0,3,0,0,0,30,0,…,1,1,1,1, with the clock at
10 on the first frame:

- the first frame ran 5 tics;
- after the 3-tic render, the next frame ran 3;
- after the 30-tic render, `D_Display`'s `NetUpdate` pumped 6 times and built 5, the next
  frame ran 5, and the clock stayed 30 tics ahead of `gametic` for the rest of the run;
- with 1-tic renders, the run settled at 1 tic per frame with one command held in reserve.

This is a model, not the genuine engine.

**The first frame of a session.** `gametime` (d_net.c:366) and `oldentertics` both start
at 0.

- `I_GetTime`'s base is set by its first call. In a `-warp`/`-record` start that call is
  `starttime = I_GetTime ()` at the end of `G_DoLoadLevel` (g_game.c:486). The grep found no
  earlier caller on that path.
- The first `TryRunTics` sees `newtics = I_GetTime()`, which is the sub-second phase of
  that first call plus the time `I_InitGraphics` took. It builds min(that, 5) commands and
  runs that many tics, or waits for 1 if it is 0.
- **So vanilla's first displayed frame shows gametic 1 to 5, depending on the clock**
  (inference from those lines; matches the model's first frame).

#### 1.3.3 `I_GetTime` (i_system.c:88-100)

1. The code is `gettimeofday(&tp, &tzp); if (!basetime) basetime = tp.tv_sec;` followed by
   `newtics = (tp.tv_sec-basetime)*TICRATE + tp.tv_usec*TICRATE/1000000;` with
   `TICRATE 35` (doomdef.h:122). `basetime` is a function static.
2. The division truncates. Tic boundaries are fixed to whole seconds: each second has
   exactly 35 tics, beginning at µs ⌈k·10^6/35⌉, so tics last 28,571 or 28,572 µs.
3. The clock is the wall clock (`gettimeofday`), not a monotonic clock. Inference: a
   system clock step changes the tic count.
4. `I_WaitVBL` (`usleep (count * (1000000/70))`, i_system.c:126-137) is used only by
   `D_QuitNetGame` (d_net.c:620) and the quit-sound delay (m_menu.c:1090), not by the
   loop. **The original busy-loops until the next tic; it never sleeps while playing.**

#### 1.3.4 `D_Display` (d_main.c:193-345) for `GS_LEVEL` with no menu, automap, help or pause

Statics and their initial values:

| Static | Line | Initial value | Notes |
|---|---|---|---|
| `viewactivestate` | 195 | `false` | |
| `menuactivestate` | 196 | `false` | |
| `inhelpscreensstate` | 197 | `false` | |
| `fullscreen` | 198 | `false` | |
| `oldgamestate` | 199 | `-1` | |
| `borderdrawcount` | 200 | 0 | uninitialised static |
| `wipegamestate` | 188 | `GS_DEMOSCREEN` | file scope |
| `setsizeneeded` | | `true` | `R_Init` calls `R_SetViewSize` (r_main.c:783, 662) |

The statements in order:

1. `if (nodrawers) return` (209-210). `nodrawers` is only set by `-timedemo -nodraw`.
2. `redrawsbar = false` (212).
3. `if (setsizeneeded)`: `R_ExecuteSetViewSize ()`, then `oldgamestate = -1;
   borderdrawcount = 3` (215-220).
   - At `setblocks == 11`: `scaledviewwidth = 320`, `viewheight = 200` (r_main.c:682-686).
   - Otherwise `scaledviewwidth = setblocks*32` and `viewheight = (setblocks*168/10)&~7`
     (688-691). At 10 that gives 320 and 168. At 9 (the default, m_misc.c:279) it gives 288
     and 144.
4. **The wipe decision** (223-229): `wipe = (gamestate != wipegamestate)`, followed by
   `wipe_StartScreen`.
   - At a `-warp`/`-record` start, `G_DoLoadLevel` leaves `wipegamestate` alone, because it
     is `GS_DEMOSCREEN`, not `GS_LEVEL` (g_game.c:472-473). It sets
     `gamestate = GS_LEVEL` (475).
   - **The first `D_Display` therefore wipes.**
   - Later wipes happen on a reborn or new level load, when `wipegamestate` was `GS_LEVEL`
     and becomes -1 (472-473), and on a change of gamestate.
   - M5b leaves wipes out (§1.3.6 explains the consequence for `M_Random`).
5. `if (gamestate == GS_LEVEL && gametic) HU_Erase ()` (231-232).
   - At screenblocks 10 and 11 `viewwindowx` is 0 (r_draw.c:705), and every erase is
     guarded by `viewwindowx` (hu_lib.c:154-155). Nothing is drawn.
   - The erase does decrement `needsupdate` (hu_lib.c:172). The HUD investigation covers
     those details.
6. The `GS_LEVEL` case (237-248):
   - `if (!gametic) break`. Inference: this cannot happen in single player, because
     `TryRunTics` always runs at least one tic before the first display.
   - `if (automapactive) AM_Drawer ()`.
   - `redrawsbar = true` if `wipe`, or if `viewheight != 200 && fullscreen`. The second
     condition holds only on the frame after switching from 11 to a smaller size.
   - `redrawsbar = true` if a help screen was just put away (244-245).
   - `ST_Drawer (viewheight == 200, redrawsbar)` (246), then
     `fullscreen = viewheight == 200` (247).
7. `I_UpdateNoBlit ()` (264), which is empty (i_video.c:344-347).
8. `if (gamestate == GS_LEVEL && !automapactive && gametic) R_RenderPlayerView
   (&players[displayplayer])` (267-268).
9. `if (gamestate == GS_LEVEL && gametic) HU_Drawer ()` (270-271).
10. The PLAYPAL reset only runs when `gamestate != oldgamestate && gamestate != GS_LEVEL`
    (274-275). **It never runs for `GS_LEVEL`.**
11. `if (gamestate == GS_LEVEL && oldgamestate != GS_LEVEL)`: `viewactivestate = false;
    R_FillBackScreen ()` (278-282). `R_FillBackScreen` returns at once when
    `scaledviewwidth == 320` (r_draw.c:747-748).
12. The border update is entered only when `scaledviewwidth != 320` (285-295).
    `R_DrawViewBorder` also returns when `scaledviewwidth == SCREENWIDTH`
    (r_draw.c:850-851). **Neither draws anything at screenblocks 10 or 11.**
13. Then `menuactivestate = menuactive; viewactivestate = viewactive; inhelpscreensstate =
    inhelpscreens; oldgamestate = wipegamestate = gamestate` (297-300).
14. `if (paused)` draw `M_PAUSE` (303-311). This is never true in M5b.
15. `M_Drawer ()` (315). With no menu or message it clears `inhelpscreens` and draws
    nothing (m_menu.c:1749 onward).
16. `NetUpdate ()` (316). This builds commands for time spent drawing (§1.3.2).
17. `if (!wipe) { I_FinishUpdate (); return; }` (320-324). Otherwise:
    - `wipe_EndScreen`.
    - A melt loop that spins on `I_GetTime` until at least one tic passes per step, and
      calls `wipe_ScreenWipe`, `I_UpdateNoBlit`, `M_Drawer` and `I_FinishUpdate` each step
      until done (327-344).
    - **No tics run during the melt.** The next `NetUpdate` drops all but 5 of the elapsed
      tics (§1.3.2).
    - Inference from f_wipe.c:171-224: the melt takes about 28-43 tics.

**The resulting draw order.** `ST_Start` runs from `P_SpawnPlayer` during `G_InitNew`
(p_mobj.c:693-699). It sets `st_firsttime = true` (st_stuff.c:1444-1452, 1254), and only
`ST_doRefresh` clears it (1089-1092). `ST_Ticker` does not touch it (988-996).

| Frame | Vanilla `-warp`/`-record` | M5b (one tic per frame, no wipe) |
|---|---|---|
| First displayed | gametic k ∈ 1..5 (§1.3.2) | gametic 1 (the state after tic 0) |
| `redrawsbar` on the first frame | `true` (because of the wipe) | `false`, since `fullscreen` starts `false` |
| `st_firsttime` on the first frame | `true` | `true`, so `ST_doRefresh` runs either way (st_stuff.c:1112, 1118) |

Screenblocks 10 (`viewheight` 168, `scaledviewwidth` 320):

- **First displayed frame:**
  1. `R_ExecuteSetViewSize`
  2. vanilla only: `wipe_StartScreen`
  3. `HU_Erase` (draws nothing)
  4. `ST_Drawer(false, redrawsbar)`: palette, then `ST_doRefresh`
  5. `R_RenderPlayerView` (rows 0-167)
  6. `HU_Drawer`
  7. `R_FillBackScreen` (returns at once)
  8. `M_Drawer` (draws nothing)
  9. `NetUpdate`
  10. `I_FinishUpdate`. Vanilla instead runs `wipe_EndScreen` and the melt.
- **Every later frame:**
  1. `HU_Erase`
  2. `ST_Drawer(false, false)`: palette, then `ST_diffDraw`, unless `st_firsttime` was set
     again (automap entry, st_stuff.c:530)
  3. `R_RenderPlayerView`
  4. `HU_Drawer`
  5. `M_Drawer`
  6. `NetUpdate`
  7. `I_FinishUpdate`

Screenblocks 11 (`viewheight` 200):

- The order is the same.
- `ST_Drawer` gets `fullscreen = true`, so `st_statusbaron = false` (st_stuff.c:1111).
- The status bar is drawn **before** the 3D view, and the view covers rows 168-199.
- `ST_doPaletteStuff` still runs every frame (1115), so damage and bonus palettes still
  apply at 11.

#### 1.3.5 `I_SetPalette` and `I_FinishUpdate` (i_video.c), for presentation only

1. `I_SetPalette` calls `UploadNewPalette` (582-585). That function stores 256 X colours,
   each component mapped through `gammatable[usegamma][...]` and widened `(c<<8)+c`
   (563-574). It does this only on an 8-bit PseudoColor visual, which `I_InitGraphics`
   requires (i_video.c:771-772).
   - `usegamma` defaults to 0 (m_misc.c:286, v_video.c:137). F11 cycles it 0-4 and
     re-uploads PLAYPAL (m_menu.c:1597-1603).
   - **`gammatable[0]` is not the identity**: it maps i → i+1 for i < 128 and i → i for
     i ≥ 128, so 0→1 and 127→128→128 (v_video.c:51-70).
2. Who sets the palette:
   - `D_Display` for non-level states (d_main.c:275).
   - `ST_doPaletteStuff`, only when the computed palette differs from `st_palette`, a
     static that starts at 0 (st_stuff.c:998, 1045-1049).
   - `ST_Stop` (1461), which the first `ST_Start` skips because `st_stopped` starts `true`
     (1441, 1447-1448).
   - The gamma key.
   - `I_InitGraphics` creates the colormap with `AllocAll` (i_video.c:794-795) and stores no
     colours.
   - Inference: in a pure `-warp`/`-record` start, linuxdoom never uploads PLAYPAL 0 until
     the first damage or bonus palette, so the colours are whatever the new X colormap
     holds. This is a port artifact. An M5b presenter has to choose a starting palette.
3. `I_FinishUpdate` (352-521):
   - Under `-devparm` it draws tic dots on the bottom row of `screens[0]` (361-374).
   - With `-2`/`-3`/`-4` it scales the 320×200 `screens[0]` by `multiply` (377 onward,
     719-729).
   - Then it puts the image and syncs (`XShmPutImage` with a completion wait, or
     `XPutImage` and `XSync`, 500-517).
   - With `multiply == 1`, `screens[0]` *is* the X image buffer (910-911). What is shown is
     exactly the 320×200 8-bit `screens[0]`, mapped by the palette.

#### 1.3.6 `M_Random` and `P_Random` (m_random.c)

1. There are two indices, `rndindex` and `prndindex`, both starting at 0 (m_random.c:53-54),
   and one shared `rndtable[256]` (31-51).
   - `P_Random` pre-increments `prndindex` (57-61).
   - `M_Random` pre-increments `rndindex` (63-67).
   - Each masks with `&0xff` and returns `rndtable[index]`.
   - `M_ClearRandom` zeroes both (69-72). Its only caller is `G_InitNew` (g_game.c:1414).
     It is not called by `G_DoLoadLevel`, reborn or `G_DoWorldDone`.
2. Every `M_Random` call site (the grep of `*.c *.h` for `M_Random` outside m_random.c):

| Site | Feeds |
|---|---|
| st_stuff.c:992 (`ST_Ticker`) | `st_randomnumber`, used only by `st_faceindex = ST_calcPainOffset() + (st_randomnumber % 3)` (915): the status-bar face's look direction |
| f_wipe.c:159, 162 (`wipe_initMelt`) | the melt column offsets. The loop runs `i < width` with width = `SCREENWIDTH` 320 (d_main.c:339-340, f_wipe.c:285), so one wipe makes **320 calls** |
| wi_stuff.c:523, 525, 561 | intermission animation timing |
| s_sound.c:330, 340 (`S_StartSoundAtVolume`) | the pitch variation of every started sound except `sfx_itemup` and `sfx_tink`. It runs after the early returns for linked sounds with volume < 1 (283-294) and for sounds that `S_AdjustSoundParams` finds inaudible from the listener (304-320). It runs for game sounds started during `P_Ticker` and for menu sounds |
| g_game.c:692 (`rndindex`, read only) | the network consistency value when a player has no `mo`; only `if (netgame ...)` (681) |

3. **No `M_Random` call can influence game state.** None of its consumers is read by the
   playsim: the face index, the wipe, intermission animation and sound pitch. The one
   gameplay-adjacent use is a netgame consistency check, which can only `I_Error`.
   `P_Random` has its own index.
4. **Vanilla `-warp`/`-record` session**, first `ST_Ticker`:
   - `G_InitNew` zeroed `rndindex`.
   - The first `ST_Ticker` is in tic 0. That tic runs inside the first `TryRunTics`,
     **before** the first `D_Display` and so before the wipe.
   - Its `M_Random` returns index `1 + s₀`, where s₀ is the number of `M_Random`-consuming
     sound starts in tic 0's `P_Ticker`. There is no menu, and `M_Ticker` does not consume.
   - The first frame's remaining k-1 tics continue the sequence, still with no wipe.
   - The wipe after the first frame adds 320 (≡ +64 mod 256).
   - From then on, every audible sound start and every menu sound shifts the stream. The
     shift depends on `snd_SfxVolume` (default 8, m_misc.c:237) and on listener distance.
5. **A harness that calls `G_InitNew` and runs tics with no wipe and no sound**: the
   `ST_Ticker` of tic t sees `rndindex = (t+1) & 0xff`.
6. The two agree only up to the first sound start or the end of the first displayed frame,
   whichever comes first. Inference: on most maps tic 0 starts no sound, so both begin at
   index 1.

#### 1.3.7 Hazards for an exact port of recording and the loop

1. Recording changes the executed command. Executed angleturn = 256·⌊(a+128)/256⌋, with
   halves rounded up.
   - Turns of |a| ≤ 127 vanish, -128 vanishes, +128 becomes +256.
   - The keyboard turn becomes +768 to the left and -512 to the right.
   - 32640..32767 become -32768.
   - A session that executes full resolution while writing rounded bytes does not replay.
2. The Rust writer refuses a nonzero low byte instead of rounding (demo.rs:297-299). The
   rounding must be done once, before both execution and writing, with the C half-up rule.
   Rust's `>>` on `i32` is arithmetic and matches gcc.
3. At most 5 tics run per frame (d_net.c:404). Time beyond that is dropped forever. The
   `counts` rule has the `realtics == availabletics-1` case that runs one fewer tic. The
   first vanilla frame shows gametic 1-5, depending on the clock.
4. Events are pumped only immediately before each build, plus once on a capped iteration,
   and never while `newtics <= 0`. Mouse motion is assigned per event and zeroed per build,
   so a long frame's motion lands in one command.
5. `D_Display` calls `NetUpdate` before the blit (d_main.c:316). With frames of about one
   tic, commands are built during the previous display.
6. The level-start wipe consumes 320 `M_Random` values and freezes ticking during the melt.
   Sound starts consume `M_Random`. Without wipes and sound, the face direction sequence
   differs from vanilla after the first frame or the first sound.
7. linuxdoom never uploads PLAYPAL at a `-warp` start. `gammatable[0]` is not the identity.
8. The vanilla default is `screenblocks` 9, where the border code does draw (r_draw.c:747,
   850; d_main.c:285-295).
9. The vanilla ways a recording ends are 'q', a full buffer (32,761 tics by default) or
   `I_Error`, and each exits the process. Quitting via the menu or SIGINT loses the demo.
   `M_WriteFile` failure is silent. `demoname[32]` can overflow.
10. A level exit takes effect at the top of the *next* tic's `G_Ticker`, before that tic's
    command is copied. Reborn reloads the level in the same `G_Ticker`.
11. `-nomonsters`/`-respawn`/`-fast` header bytes are argv indices, not 1.
12. `MF_JUSTATTACKED` rewrites the executed command after recording (p_user.c:249-255).

#### 1.3.8 Questions the recording and loop facts raise for the owner

1. **Turn resolution.**
   - Options: (a) round every live command to demo resolution before execution, which is
     vanilla `-record` behaviour, so the demo replays the session exactly; (b) execute full
     resolution and record rounded, so the demo does not replay the session, which
     conflicts with "every session writes a demo"; (c) a non-110 or extended format, which
     needs a new schema.
   - Facts: §1.2.2 items 3-5, the §1.2.5 table and demo.rs:297-299.
2. **Pacing.**
   - Options: (a) busy-wait like the original; (b) sleep until the next tic boundary.
   - Facts: `I_GetTime` uses wall-clock `gettimeofday` with second-aligned truncation
     (§1.3.3). A monotonic clock would differ only in presentation timing, not in tic
     content.
3. **Slow-frame policy.**
   - Options: (a) port `TryRunTics` exactly: 5-tic cap, dropped time, the `counts` quirk,
     builds inside `D_Display`; (b) a simpler rule, such as one command and one tic per
     frame, or catch-up without a cap.
   - Facts: §1.3.2. The demo records whatever commands run, so replay is unaffected either
     way. Input latency and how mouse motion is spread across commands differ.
4. **`M_Random` and the face.**
   - Options: (a) the plain stream with no wipe or sound (index t+1 at tic t); (b) emulate
     the wipe's 320 draws, though sound consumption cannot be emulated without sound.
   - Facts: §1.3.6. Only the face is affected.
5. **First displayed frame.**
   - Options: (a) display after the first tic (gametic 1); (b) imitate vanilla's
     clock-dependent 1-5.
   - Facts: §1.3.2, §1.3.4.
6. **Starting palette and gamma.**
   - Options: (a) PLAYPAL 0 at start through `gammatable[0]`, which is the off-by-one ramp;
     (b) PLAYPAL 0 with the identity; (c) linuxdoom's undefined colormap, which is not a
     sensible choice.
   - Facts: §1.3.5.
7. **Screen sizes admitted.**
   - Options: (a) 10 and 11 only, where no border is drawn; (b) also the vanilla default 9
     and below, which needs `R_FillBackScreen`/`R_DrawViewBorder` and HUD border erasing.
   - Facts: §1.3.4 items 3, 5, 11 and 12.
8. **How a recording ends and how big it may be.**
   - Options: end at level exit, at the refused reborn tic, at window close, on 'q'; and
     whether to keep vanilla's 0x20000 cap (32,761 tics).
   - Facts: §1.2.2 items 1, 7 and 8, §1.2.4. A demo whose marker directly follows the exit
     tic is still valid vanilla input.
9. **Header domain.**
   - The existing readers admit only E1M1-9 at skill 2 (and 3 for wire 24), with all flags
     0 (demo.rs:25-37, 179-206).
   - Options: (a) restrict live sessions to that domain; (b) add an explicitly versioned
     reader and writer, leaving the historical ones unchanged.
   - Facts: §1.2.1 table and §1.2.6.

Each question is taken up in §4: 1 in S3; 2 in S8; 3 in S8; 4 in S15; 5 in S8; 6 in S13; 7 in S14; 8 in S21; 9 in S21.

### 1.4 The status bar and the palette choice: st_stuff.c, st_lib.c, v_video.c

All citations are `file:line` in `linuxdoom-1.10/` at the authority commit. Nothing
here was taken from an IWAD. Patch widths, heights and offsets are whatever the lump
header says at run time (`patch_t` fields `width`, `height`, `leftoffset`,
`topoffset`, read through `SHORT()`); only positions and constants fixed by the C code
are given as numbers. Statements marked **(inference)** are reasoning from the cited
code, not something read directly or run. One small arithmetic calculator was compiled
and run under `/home/copgunn/scratch/m5b/probe-st/` (ext4) to check the two tables in
4.6 and 10.3. It copies the expressions from st_stuff.c:735-739 and 1008-1043. It is
not a genuine-source build.

Build configuration that matters throughout: `RANGECHECK` is defined unconditionally in
doomdef.h:75. The Makefile's `CFLAGS=-g -Wall -DNORMALUNIX -DLINUX` (Makefile:9) neither
adds nor removes it. `SCREENWIDTH` is 320 and `SCREENHEIGHT` is 200 (doomdef.h:110,112),
`SCREEN_MUL` is 1 (doomdef.h:104) and `TICRATE` is 35 (doomdef.h:122). So
`ST_HEIGHT` = `32*SCREEN_MUL` = 32, `ST_WIDTH` = 320 and `ST_Y` = 200 - 32 = 168
(st_stuff.h:32-34). Note that `ST_HEIGHT` has no outer parentheses. Every use in these
files passes it as a whole argument, so that is harmless here.

#### 1.4.1 Lifecycle and every lump loaded

1. **ST_Init** (st_stuff.c:1466-1471) sets `veryfirsttime = 0`, calls `ST_loadData()`,
   then allocates `screens[4] = Z_Malloc(ST_WIDTH*ST_HEIGHT, PU_STATIC, 0)`, which is
   320*32 = 10,240 bytes. It is called once, from D_DoomMain (d_main.c:1112-1113),
   after `V_Init()` (d_main.c:1012). `Z_Malloc` does not clear memory: z_zone.c contains
   no `memset` (grep). So `screens[4]` starts indeterminate.
2. **ST_loadData** (st_stuff.c:1201-1205) sets `lu_palette = W_GetNumForName("PLAYPAL")`
   (a lump number only; the data is cached later with `PU_CACHE`, st_stuff.c:1048 and
   1461), then calls `ST_loadGraphics`.
3. **ST_loadGraphics** (st_stuff.c:1124-1199) loads with `W_CacheLumpName(..., PU_STATIC)`,
   in this order:

   | Lump name(s) | Count | Stored in | Line |
   |---|---|---|---|
   | `STTNUM0`..`STTNUM9` | 10 | `tallnum[0..9]` | 1136-1137 |
   | `STYSNUM0`..`STYSNUM9` | 10 | `shortnum[0..9]` (same loop) | 1139-1140 |
   | `STTPRCNT` | 1 | `tallpercent` | 1145 |
   | `STKEYS0`..`STKEYS5` | 6 (`NUMCARDS`) | `keys[0..5]` | 1148-1152 |
   | `STARMS` | 1 | `armsbg` | 1155 |
   | `STGNUM2`..`STGNUM7` | 6 | `arms[i][0]`, i = 0..5 (`"STGNUM%d", i+2`) | 1158-1163 |
   | (no load) | - | `arms[i][1] = shortnum[i+2]` (yellow digits 2..7) | 1166 |
   | `STFB%d` with `consoleplayer` | 1 | `faceback` | 1170-1171 |
   | `STBAR` | 1 | `sbar` | 1174 |
   | face patches (table below) | 42 | `faces[0..41]` | 1177-1197 |

   `STFB<consoleplayer>` is always loaded, so `STFB0` in single player. It is drawn
   only when `netgame` is true (st_stuff.c:506-507), so it is never drawn in single
   player. If any name is missing, `W_GetNumForName` stops with
   `I_Error("W_GetNumForName: %s not found!")` (w_wad.c:406).
4. **STlib_init** (st_lib.c:58-61) loads `STTMINUS` into the global `sttminus`. This is
   **not** done at ST_Init: it runs on every `ST_initData` (st_stuff.c:1276), so on every
   ST_Start. Across both functions, 78 patch lumps are loaded at ST_Init and 1 at each
   ST_Start, and PLAYPAL is looked up by number.
5. **The faces[] order** (st_stuff.c:1177-1197). For pain level `i` = 0..4, eight lumps
   are loaded in sequence: `STFST<i>0`, `STFST<i>1`, `STFST<i>2`, `STFTR<i>0`, `STFTL<i>0`,
   `STFOUCH<i>`, `STFEVL<i>`, `STFKILL<i>`. After those come `STFGOD0` (index 40) and
   `STFDEAD0` (index 41).

   | Index | Lump | Meaning (comment in source) |
   |---|---|---|
   | 8i+0, 8i+1, 8i+2 | `STFST<i>0..2` | straight ahead, three looks |
   | 8i+3 | `STFTR<i>0` | "turn right" (1185) |
   | 8i+4 | `STFTL<i>0` | "turn left" (1187) |
   | 8i+5 | `STFOUCH<i>` | "ouch!" (1189) |
   | 8i+6 | `STFEVL<i>` | "evil grin" (1191) |
   | 8i+7 | `STFKILL<i>` | "pissed off" (rampage, head-on) (1193) |
   | 40 | `STFGOD0` | god face |
   | 41 | `STFDEAD0` | dead face |

   The face constants (st_stuff.c:97-115) are `ST_NUMPAINFACES` 5,
   `ST_NUMSTRAIGHTFACES` 3, `ST_NUMTURNFACES` 2, `ST_NUMSPECIALFACES` 3,
   `ST_FACESTRIDE` = 3+2+3 = 8, `ST_NUMEXTRAFACES` 2, `ST_NUMFACES` = 8*5+2 = 42,
   `ST_TURNOFFSET` = 3, `ST_OUCHOFFSET` = 5, `ST_EVILGRINOFFSET` = 6,
   `ST_RAMPAGEOFFSET` = 7, `ST_GODFACE` = 40 and `ST_DEADFACE` = 41.
6. **PLAYPAL palette constants** (st_stuff.c:71-76) are `STARTREDPALS` 1, `NUMREDPALS` 8,
   `STARTBONUSPALS` 9, `NUMBONUSPALS` 4 and `RADIATIONPAL` 13. The palette chosen is read
   at `W_CacheLumpNum(lu_palette, PU_CACHE) + palette*768` (st_stuff.c:1048). The
   largest palette used is 13, so the lump must be at least 14*768 = 10,752 bytes. The C
   code **does not check** the lump size.
7. **ST_Start** (st_stuff.c:1444-1454) does `if (!st_stopped) ST_Stop();`, then
   `ST_initData(); ST_createWidgets(); st_stopped = false;`. Its only caller is
   P_SpawnPlayer when `mthing->type-1 == consoleplayer` (p_mobj.c:693-697). So it runs
   during P_SetupLevel on every level load, including single-player death reloads
   (G_DoReborn sets `ga_loadlevel`, g_game.c:924-931), inside G_Ticker's
   `gameaction` loop (g_game.c:617-622). That is **before** that same G_Ticker's
   `P_Ticker(); ST_Ticker();` (g_game.c:730-731).
8. **ST_Stop** (st_stuff.c:1456-1464): `if (st_stopped) return;` otherwise
   `I_SetPalette(W_CacheLumpNum(lu_palette, PU_CACHE))`, which loads palette 0, and sets
   `st_stopped = true`. It does **not** write `st_palette`. Its only caller is ST_Start
   (grep). So on every ST_Start except the first in the process, the hardware palette
   is reset to palette 0 during level load.
9. **Other entry points.** `ST_Ticker` is called from G_Ticker in `GS_LEVEL` only
   (g_game.c:731). `ST_Drawer(viewheight == 200, redrawsbar)` is called from D_Display in
   `GS_LEVEL` only when `gametic != 0` (d_main.c:236-247; `if (!gametic) break;` at 238). `ST_Responder` is called from
   G_Responder in `GS_LEVEL` (g_game.c:546). It is also called from the automap's
   AM_initVariables (am_map.c:495) and AM_Stop (am_map.c:566). Every AM_Stop call site
   outside am_map.c is guarded by `automapactive` (g_game.c:1030-1031, p_inter.c:708-714).
   `ST_unloadData`/`ST_unloadGraphics` (st_stuff.c:1207-1247) have no caller outside
   their own file, so the patches stay `PU_STATIC` for the life of the process.

#### 1.4.2 Every piece of state the status bar keeps

Legend for the "Reset" column: **I** = ST_initData (each ST_Start); **W** =
ST_createWidgets (each ST_Start); **T** = rewritten every tic before use; **D** =
rewritten every frame before use; **never** = only the static initialiser, so its
value carries over from earlier levels, earlier games and death reloads for the whole
process. Rows marked **CARRIED** are not reset at a new level.

| # | Variable (line) | Type, initial value | Written at | Reset | Meaning / readers |
|---|---|---|---|---|---|
| 1 | `plyr` (270) | `player_t*`, NULL | 1255 | I | `&players[consoleplayer]` |
| 2 | `st_firsttime` (273) | `boolean`, 0 | 530, 1112, 1092, 1254 | I (true) | next ST_Drawer does a full refresh |
| 3 | `veryfirsttime` (276) | `int`, 1 | 1468 | never | write-only (no reader) |
| 4 | `lu_palette` (279) | `int`, 0 | 1203 | ST_Init only | PLAYPAL lump number |
| 5 | `st_clock` (282) | `unsigned int`, 0 | 991, 1257 | I | write-only (no reader); unsigned wrap is defined |
| 6 | `st_msgcounter` (285) | `int`, 0 | 983 (`--`) | **never, CARRIED** | only read by its own `!--st_msgcounter` test; no other writer. It decrements forever, going negative from the first tic |
| 7 | `st_chatstate` (288) | enum, 0 | 1258 | I | write-only |
| 8 | `st_gamestate` (291) | enum, 0 | 529, 535, 1259 | I | write-only (automap notifications) |
| 9 | Whether AGENTS.md's limit of 2 parallel jobs for the C reference suite still applies under the brief's "no CPU caps or job limits" | Ruled by the owner 2026-11-17: while the machine is idle there is no job limit, C runs included; if anything is OOM-killed, halve and re-run; the 2-job limit applies again whenever the owner says the machine is in use. On 2026-11-17 the owner said the machine is not in use. Factual record: in Phase 1 the reference suite itself was not run, and the full-frame prototype ran its genuine runner with 6 parallel jobs (§2.1.2, §2.1.8) under the brief's wording |
| 10 | `st_chat`, `st_oldchat` (297, 300) | `boolean`, 0 | 984, 1262 | I | `st_chat = st_oldchat` only when `st_msgcounter` reaches 0; neither has another reader |
| 11 | `st_cursoron` (303) | `boolean`, 0 | 1263 | I | write-only |
| 12 | `st_notdeathmatch` (306) | `boolean`, 0 | 965 | T | `w_armsbg` value |
| 13 | `st_armson` (309) | `boolean`, 0 | 968, 1059 | T, D | `w_arms[]` `on`; the frame write at 1059 is the one drawing uses |
| 14 | `st_fragson` (312) | `boolean`, 0 | 971, 1062 | T, D | `w_frags` `on` |
| 15 | patch pointers `sbar`, `tallnum`, `tallpercent`, `shortnum`, `keys`, `faces`, `faceback`, `armsbg`, `arms` (315-339), `sttminus` (st_lib.c:56) | pointers | ST_loadGraphics; STlib_init | ST_Init (and STlib_init per ST_Start) | lump data |
| 16 | widgets `w_ready`, `w_frags`, `w_health`, `w_armsbg`, `w_arms[6]`, `w_faces`, `w_keyboxes[3]`, `w_armor`, `w_ammo[4]`, `w_maxammo[4]` (342-370) | structs | createWidgets; `w_ready.num`/`.data` at 933-945 | W | see 1.4.7; history fields in rows 30-32 |
| 17 | `st_fragscount` (375) | `int`, 0 | 972-980 | T | `w_frags` value |
| 18 | `st_oldhealth` (378) | `int`, -1 | 994 | I (-1) | `plyr->health` as of the end of the previous ST_Ticker |
| 19 | `oldweaponsowned[NUMWEAPONS]` (381) | `boolean[9]`, 0 | 784, 1271 | I (= `plyr->weaponowned`) | evil-grin change detector |
| 20 | `st_facecount` (384) | `int`, 0 | 768, 791, 809, 833, 863, 869, 888, 906, 916, 920 | **never, CARRIED** | tics until the face may change |
| 21 | `st_faceindex` (387) | `int`, 0 | 767, 792, 810, 834-849, 864, 870, 887, 905, 915, 1265 | I (0) | index into `faces[]` |
| 22 | `keyboxes[3]` (390) | `int[3]` | 955-958, 1274 | I (-1), T | key icon per box |
| 23 | `st_randomnumber` (393) | `int`, 0 | 992 | T | one `M_Random()` per tic |
| 24 | cheat sequences `cheat_*` (400-487; non-static globals) and their `cheatseq_t.p` cursor; `firsttime`, `cheat_xlate_table` (m_cheat.c:34-35) | `cheatseq_t` | cht_CheckCheat (m_cheat.c:43-76), cht_GetParam writes into the sequence bytes (m_cheat.c:91-98) | **never, CARRIED** | advanced by every keydown reaching ST_Responder |
| 25 | `st_palette` (998) | `static int`, 0 | 1047, 1266 | I (-1) | last palette number passed to I_SetPalette by this module |
| 26 | `st_stopped` (1441) | `static boolean`, true | 1452, 1463 | never | gate for ST_Stop |
| 27 | `lastcalc`, `oldhealth` in ST_calcPainOffset (732-733) | `static int`, 0 and -1 | 739-740 | never | a cache keyed on clamped health. **Pure in effect** (inference): `player->health` is never below 0 (clamped at p_inter.c:870-871; other writers add to it or set 100). So the cache always returns `8*(((100-min(h,100))*5)/101)` and needs no saved history |
| 28 | `lastattackdown` in ST_updateFaceWidget (757) | `static int`, -1 | 883, 884, 889, 893 | **never, CARRIED** | rampage countdown |
| 29 | `priority` in ST_updateFaceWidget (758) | `static int`, 0 | 766, 790, 805, 862, 868, 886, 903, 917 | **never, CARRIED** | priority of the face currently held |
| 30 | `largeammo` in ST_updateWidgets (926) | `static int`, 1994 | never written | - | the "no ammo" number |
| 31 | `st_number_t.oldnum` (st_lib.h:55) | `int` | st_lib.c:77 (0), 105 | W | **write-only**: STlib_drawNum never reads it, so numbers carry no history |
| 32 | `st_multicon_t.oldinum` (st_lib.h:96) | `int` | st_lib.c:203 (-1), 238 | W | the last icon drawn; used to erase the old patch's box |
| 33 | `st_binicon_t.oldval` (st_lib.h:125) | `int` | st_lib.c:255 (0), 289 | W | the last value drawn |
| 34 | `.data` user fields (st_lib.h:68, 109, 136) | `int` | only `w_ready.data` (945, 1297) | W | write-only |
| 35 | `screens[4]` (BG) contents, 320x32 | bytes, indeterminate | ST_refreshBackground (504, 507) | **never cleared** | the background copied into FG to erase widgets |
| 36 | `screens[0]` (FG) rows 168-199 | bytes | every status bar draw | **never cleared** by ST | the visible bar; diff drawing depends on what is already there |
| 37 | `mypos` `buf` (666) | `static char[52]` | 667 | never | cheat message text |

**Not reset at a new level, or carried over in effect:** `st_facecount`, `priority`,
`lastattackdown`, `st_msgcounter`, the cheat cursors, the BG buffer and the FG bar
rows. The FG rows are overwritten wholesale by the first refresh after ST_Start, and the
BG rows by STBAR (assuming STBAR covers every pixel; see 1.4.9). None of these is in a C
savegame: p_saveg.c archives player and world state only. After a C savegame load they
keep whatever the process last had.

#### 1.4.3 ST_Ticker and ST_updateWidgets

ST_Ticker (st_stuff.c:988-996), in order:

1. `st_clock++;`
2. `st_randomnumber = M_Random();` (m_random.c:63-66: `rndindex = (rndindex+1)&0xff;
   return rndtable[rndindex];`, a value from 0 to 255).
3. `ST_updateWidgets();`
4. `st_oldhealth = plyr->health;`

ST_updateWidgets (st_stuff.c:924-986), in order:

1. Ready ammo (932-935): if `weaponinfo[plyr->readyweapon].ammo == am_noammo`, then
   `w_ready.num = &largeammo` (1994). Otherwise `w_ready.num =
   &plyr->ammo[weaponinfo[plyr->readyweapon].ammo]`. Fist and chainsaw are `am_noammo`
   (d_items.c:51, 114).
2. `w_ready.data = plyr->readyweapon;` (945). This field is write-only.
3. Key boxes (953-959): for i = 0..2, `keyboxes[i] = plyr->cards[i] ? i : -1;` then
   `if (plyr->cards[i+3]) keyboxes[i] = i+3;`. The skull key replaces the card of the
   same colour. Card order (doomdef.h:164-169): blue card, yellow card, red card, blue
   skull, yellow skull, red skull. Box 0 is blue, 1 is yellow and 2 is red.
4. `ST_updateFaceWidget();` (962), described in 1.4.4.
5. `st_notdeathmatch = !deathmatch;`, `st_armson = st_statusbaron && !deathmatch;` and
   `st_fragson = deathmatch && st_statusbaron;` (965-971). `st_statusbaron` here is the
   value left by the last ST_Drawer. The two `on` flags are recomputed before drawing
   anyway (1059, 1062).
6. Frags (972-980): `st_fragscount = 0`, then for each of the 4 players, `+=
   plyr->frags[i]` for others and `-= plyr->frags[i]` for the console player.
7. `if (!--st_msgcounter) st_chat = st_oldchat;` (983-984). Nothing else sets
   `st_msgcounter` (grep), so it reaches 0 only after the 32-bit int wraps, roughly 2^32
   tics in. It overflows a signed int (undefined behaviour) after 2^31 tics, about 1.9
   years at 35 Hz. The only effect is on `st_chat`, which has no reader.

**When this runs.** G_Ticker runs, in order: reborns and gameactions (including level
load and ST_Start), then `P_Ticker(); ST_Ticker(); AM_Ticker(); HU_Ticker();`
(g_game.c:610-734). So the status bar reads the player **after** this tic's
P_PlayerThink (power counters, `damagecount` and `bonuscount` decrements:
p_user.c:337-359) and after P_RunThinkers (this tic's damage and pickups). ST_Ticker is
not gated on `paused` or `menuactive`. Those return early inside P_Ticker only
(p_tick.c:134-143). So `M_Random` advances and the face keeps animating on every
`GS_LEVEL` tic, whatever the screen size.

#### 1.4.4 ST_updateFaceWidget, step by step (st_stuff.c:752-922)

Locals: `int i; angle_t badguyangle, diffang; boolean doevilgrin;`. Statics:
`lastattackdown = -1`, `priority = 0` (757-758). Counters: `ST_EVILGRINCOUNT` = 70,
`ST_STRAIGHTFACECOUNT` = 17 (35/2), `ST_TURNCOUNT` = 35, `ST_RAMPAGEDELAY` = 70 and
`ST_MUCHPAIN` = 20 (120-126). `ST_OUCHCOUNT` (35, line 123) is **defined but unused**:
the ouch face uses `ST_TURNCOUNT`. `P` below stands for a fresh call to
`ST_calcPainOffset()`.

1. **Dead** (761-770). `if (priority < 10)`, which is always true:
   `if (!plyr->health) { priority = 9; st_faceindex = 41; st_facecount = 1; }`.
2. **Evil grin** (772-796). `if (priority < 9)`: `if (plyr->bonuscount)`, then
   `doevilgrin = false;` and for i = 0..8 (`NUMWEAPONS`):
   `if (oldweaponsowned[i] != plyr->weaponowned[i]) { doevilgrin = true;
   oldweaponsowned[i] = plyr->weaponowned[i]; }`. Then
   `if (doevilgrin) { priority = 8; st_facecount = 70; st_faceindex = P + 6; }`.
   `oldweaponsowned` is updated **only** while `bonuscount != 0` and `priority < 9`. A
   weapon gained without a bonus (for example by cheat) leaves the difference pending
   until the next tic with `bonuscount != 0`, and the grin fires then. All 9 weapons are
   compared, including chainsaw and super shotgun.
3. **Pain from an attacker** (798-853). `if (priority < 8)`:
   `if (plyr->damagecount && plyr->attacker && plyr->attacker != plyr->mo)`:
   1. `priority = 7;`
   2. `if (plyr->health - st_oldhealth > ST_MUCHPAIN)` (807, exactly as written). This
      is true only when health has **risen** by more than 20 since the previous tic. Then
      `st_facecount = 35; st_faceindex = P + 5;` (ouch).
   3. Otherwise:
      `badguyangle = R_PointToAngle2(plyr->mo->x, plyr->mo->y, plyr->attacker->x,
      plyr->attacker->y);` (814-817).
      - `if (badguyangle > plyr->mo->angle) { diffang = badguyangle - plyr->mo->angle;
        i = diffang > ANG180; }`
      - `else { diffang = plyr->mo->angle - badguyangle; i = diffang <= ANG180; }`
        (819-830, unsigned `angle_t` arithmetic; `ANG45` = 0x20000000, `ANG180` =
        0x80000000, tables.h:68,70).
      - `st_facecount = 35; st_faceindex = P;`
      - then `if (diffang < ANG45) st_faceindex += 7;` (head-on, 836-840)
      - `else if (i) st_faceindex += 3;` ("turn face right", 841-845)
      - `else st_faceindex += 4;` ("turn face left", 846-850).

      `diffang` is never folded into 0..180 degrees. With the attacker just across the
      0/360 wrap from the view angle, `diffang` is close to `ANG_MAX` and the face turns
      rather than looking head-on. Equal angles take the `else` branch, giving
      `diffang` = 0, which is head-on.
   4. While `damagecount` stays nonzero this block runs every tic, because 7 < 8. It
      re-aims the face and resets `st_facecount` to 35 each time. The 35-tic countdown
      only starts once `damagecount` is 0.
4. **Pain without an attacker, or self-inflicted** (855-875). `if (priority < 7)`, then
   `if (plyr->damagecount)`: if `plyr->health - st_oldhealth > 20`, then
   `{ priority = 7; st_facecount = 35; st_faceindex = P + 5; }`. Otherwise
   `{ priority = 6; st_facecount = 35; st_faceindex = P + 7; }`. This block is reached
   when `attacker` is NULL (environmental damage: `P_DamageMobj(..., NULL, NULL, ...)`,
   for example p_spec.c:1025-1026) or equals `plyr->mo` (own splash damage).
5. **Rampage** (877-895). `if (priority < 6)`: if `plyr->attackdown`, then
   `if (lastattackdown == -1) lastattackdown = 70;`
   `else if (!--lastattackdown) { priority = 5; st_faceindex = P + 7;
   st_facecount = 1; lastattackdown = 1; }`. Otherwise `lastattackdown = -1;`.
   While `priority` is 6 or more, this block is skipped completely, so
   `lastattackdown` is **frozen**: it neither counts nor resets. After the first
   rampage frame, `lastattackdown = 1` makes it fire again every following tic while
   fire is held.
6. **God** (897-910). `if (priority < 5)`: if `(plyr->cheats & CF_GODMODE) ||
   plyr->powers[pw_invulnerability]`, then `{ priority = 4; st_faceindex = 40;
   st_facecount = 1; }` (`CF_GODMODE` = 2, d_player.h:73). Rampage at priority 5 wins
   over god because god's test is `< 5`.
7. **Look around** (912-918). `if (!st_facecount) { st_faceindex = P +
   (st_randomnumber % 3); st_facecount = 17; priority = 0; }`. `st_randomnumber` is
   0..255, so the remainder is 0, 1 or 2.
8. `st_facecount--;` (920). Every setter writes a value of 1 or more immediately before
   this, so `st_facecount` never goes negative (inference from the list of writes in
   1.4.2).

**ST_calcPainOffset** (729-743): `health = plyr->health > 100 ? 100 : plyr->health;`
`if (health != oldhealth) { lastcalc = ST_FACESTRIDE * (((100 - health) *
ST_NUMPAINFACES) / 101); oldhealth = health; }` `return lastcalc;`. The calculator gave:

| `plyr->health` | 80 and above | 60-79 | 40-59 | 20-39 | 0-19 |
|---|---|---|---|---|---|
| offset | 0 | 8 | 16 | 24 | 32 |

The largest index produced is 32 + 7 = 39, then 40 and 41. Every index is in bounds.

**Player fields read**, all of `*plyr` as left by this tic's P_Ticker: `health` (the
`player_t` copy, not `mo->health`), `bonuscount`, `weaponowned[0..8]`, `damagecount`,
`attacker`, `attacker->x`, `attacker->y`, `mo`, `mo->x`, `mo->y`, `mo->angle`,
`attackdown`, `cheats` and `powers[pw_invulnerability]`. The module's own `st_oldhealth`
and `st_randomnumber` are read too.

**`plyr->attacker`.** It is written only at p_inter.c:874 (`player->attacker = source;`
in P_DamageMobj, in the same block that does `damagecount += damage` at p_inter.c:875).
It is cleared to NULL only by G_PlayerReborn's `memset` (g_game.c:815) and by a
savegame load (p_saveg.c:97). P_SpawnPlayer and G_PlayerFinishLevel zero `damagecount`
and `bonuscount` (p_mobj.c:679-680, g_game.c:790-791) but leave `attacker` alone. The
face reads `attacker` only when `damagecount != 0`, and `damagecount` becomes nonzero
only together with an `attacker` write. So whenever it is read, `attacker` names the
source of the most recent damage in the current level (inference). Cases:
- **NULL**: step 3 fails and step 4 applies.
- **The player's own mobj**: excluded by `attacker != plyr->mo`, so step 4 applies.
- **Removed since**: nothing clears the pointer. P_RemoveMobj marks the thinker
  (p_mobj.c:546-569, p_tick.c:83), and the next P_RunThinkers `Z_Free`s it
  (p_tick.c:113). Z_Free rewrites only zone block headers (z_zone.c:122-170), so
  `attacker->x`/`->y` read freed memory. That is the stale coordinates until the block
  is reallocated, then whatever the new owner put there (inference). In Doom 1 a
  damage source that removes itself on death is the lost soul: `S_SKULL_DIE6` leads to
  `S_NULL` (info.c:736), and S_NULL removes the mobj (p_mobj.c:63-66). That holds for
  up to 100 tics after the hit, since `damagecount` is at most 100 (p_inter.c:877-878)
  and falls by 1 per tic. Barrels pass their killer as the source (A_Explode passes
  `thing->target`), not themselves. The read happens only on the turn branch (step 3.3),
  which is the usual branch because the ouch test is reversed.
- `R_PointToAngle2` also writes the renderer globals `viewx = x1; viewy = y1;`
  (r_main.c:384-385). R_SetupFrame overwrites both before any view is drawn
  (r_main.c:835-836). `viewx` is referenced only in r_bsp.c, r_main.c, r_plane.c,
  r_segs.c and r_things.c (grep), so no game logic sees this write.

#### 1.4.5 M_Random, and writes outside the status bar's own statics

1. **Random numbers.** The only random call in st_stuff.c, st_lib.c and v_video.c is
   `M_Random()` at st_stuff.c:992 (grep). It is called once per `GS_LEVEL` tic, in
   ST_Ticker, after P_Ticker. There is **no `P_Random`** call here. `rndindex` is shared
   with the other `M_Random` users: f_wipe.c:159,162, wi_stuff.c:523,525,561 and
   s_sound.c:330,340. It is reset by `M_ClearRandom` (g_game.c:1414). The netgame
   consistency check reads it (g_game.c:692). The face's look-around therefore depends
   on every `M_Random` call made earlier in the process (cross-reference: the D_Display
   and M_Random section).
2. **Writes that game logic or other modules can see.**
   - `ST_Responder` cheats, only on `ev_keydown` (st_stuff.c:541-723) and only when
     `cht_CheckCheat` completes a sequence: `plyr->cheats ^= CF_GODMODE` with
     `plyr->mo->health = 100`, `plyr->health = 100` (551-557); `armorpoints = 200`,
     `armortype = 2`, every `weaponowned[i] = true`, `ammo[i] = maxammo[i]` (566-573,
     580-587); `cards[i] = true` (589-590); `S_ChangeMusic` (611, 620); `cheats ^=
     CF_NOCLIP` (628); `P_GivePower(plyr, i)` or `powers[i] = 1` or `0` (636-645);
     `weaponowned[wp_chainsaw] = true`, `powers[pw_invulnerability] = true` (659-660);
     `plyr->message = ...` (many lines); and `G_DeferedInitNew(gameskill, epsd, map)`
     (721). These writes happen at event time, outside any ticcmd. Even when no cheat
     completes, **every** keydown advances the cheat cursors (m_cheat.c:56-73).
     ST_Responder **always returns false** (697-724), so it never eats an event. With
     no keydown delivered, none of this can run.
   - `ST_Responder` with `ev_keyup` and `(data1 & 0xffff0000) == AM_MSGHEADER`
     (523-538): `AM_MSGENTERED` sets `st_gamestate = AutomapState; st_firsttime =
     true;` and `AM_MSGEXITED` sets `st_gamestate = FirstPersonState;`. These come only
     from the automap (am_map.c:495, 566).
   - `I_SetPalette` at st_stuff.c:1049 (ST_doPaletteStuff) and 1461 (ST_Stop).
   - `screens[4]` (BG) and `screens[0]` (FG) through V_DrawPatch and V_CopyRect;
     V_MarkRect's `dirtybox` (v_video.c:149-150), which no code reads (grep).
   - `viewx`, `viewy` through R_PointToAngle2 (above).
   - Nothing else. Every other write in ST_Ticker and ST_Drawer goes to the module's own
     statics, the widget structs or `rndindex`.

#### 1.4.6 ST_Drawer and the refresh path

1. **ST_Drawer(fullscreen, refresh)** (st_stuff.c:1108-1122):
   `st_statusbaron = (!fullscreen) || automapactive;`
   `st_firsttime = st_firsttime || refresh;`
   `ST_doPaletteStuff();` (every frame, even with the bar off)
   `if (st_firsttime) ST_doRefresh(); else ST_diffDraw();`.
   D_Display passes `fullscreen = (viewheight == 200)`. It passes `refresh =
   redrawsbar`, which is true on a wipe frame, when `viewheight != 200` and the previous
   frame was full screen, or when a help screen has just been put away
   (d_main.c:242-246).
2. **ST_doRefresh** (1089-1100): `st_firsttime = false; ST_refreshBackground();
   ST_drawWidgets(true);`.
3. **ST_diffDraw** (1102-1106): `ST_drawWidgets(false);`.
4. **ST_refreshBackground** (499-512): only `if (st_statusbaron)`:
   `V_DrawPatch(ST_X=0, 0, BG, sbar);` then `if (netgame) V_DrawPatch(ST_FX=143, 0, BG,
   faceback);`, then `V_CopyRect(0, 0, BG, 320, 32, 0, 168, FG);`. This copies all of
   BG to FG rows 168-199. When the bar is off, `st_firsttime` is still consumed
   (step 2) and nothing is drawn.
5. **ST_drawWidgets(refresh)** (1054-1087), in this exact order:
   1. `st_armson = st_statusbaron && !deathmatch;` and `st_fragson = deathmatch &&
      st_statusbaron;`
   2. `STlib_updateNum(&w_ready, refresh);`
   3. for i = 0..3: `STlib_updateNum(&w_ammo[i])`, then `STlib_updateNum(&w_maxammo[i])`
      (interleaved).
   4. `STlib_updatePercent(&w_health)`, `STlib_updatePercent(&w_armor)`
   5. `STlib_updateBinIcon(&w_armsbg)`
   6. for i = 0..5: `STlib_updateMultIcon(&w_arms[i])`
   7. `STlib_updateMultIcon(&w_faces)`
   8. for i = 0..2: `STlib_updateMultIcon(&w_keyboxes[i])`
   9. `STlib_updateNum(&w_frags)`

   Widget values are read **at draw time** through their pointers (health, armour, ammo
   and so on, as they are when D_Display runs). The face index, key boxes, frag sum and
   ready-ammo pointer are the values ST_Ticker computed.

#### 1.4.7 ST_createWidgets: every widget (st_stuff.c:1282-1439)

All `on` pointers are `&st_statusbaron` except where noted. Numbers start with
`oldnum = 0`, multi-icons with `oldinum = -1` and binary icons with `oldval = 0`
(st_lib.c:77, 203, 255).

| Widget | Kind | x, y | Patches | Value pointer | `on` | Digits |
|---|---|---|---|---|---|---|
| `w_ready` (1288-1297) | number | 44, 171 | `tallnum` | `&plyr->ammo[weaponinfo[plyr->readyweapon].ammo]` (re-pointed every tic, 1.4.3); `.data = readyweapon` | statusbaron | 3 |
| `w_health` (1300-1306) | percent | 90, 171 | `tallnum`, `tallpercent` | `&plyr->health` | statusbaron | 3 (hard-coded, st_lib.c:172) |
| `w_armsbg` (1309-1314) | binary icon | 104, 168 | `armsbg` | `&st_notdeathmatch` | statusbaron | - |
| `w_arms[i]`, i = 0..5 (1317-1324) | multi-icon | 111+(i%3)*12, 172+(i/3)*10, that is (111,172) (123,172) (135,172) (111,182) (123,182) (135,182) | `arms[i]` = {`STGNUM<i+2>`, `STYSNUM<i+2>`} | `(int *)&plyr->weaponowned[i+1]` (pistol, shotgun, chaingun, missile, plasma, BFG) | `st_armson` | - |
| `w_frags` (1327-1333) | number | 138, 171 | `tallnum` | `&st_fragscount` | `st_fragson` | 2 |
| `w_faces` (1336-1341) | multi-icon | 143, 168 | `faces` | `&st_faceindex` | statusbaron | - |
| `w_armor` (1344-1349) | percent | 221, 171 | `tallnum`, `tallpercent` | `&plyr->armorpoints` | statusbaron | 3 |
| `w_keyboxes[0..2]` (1352-1371) | multi-icon | (239,171) (239,181) (239,191) | `keys` | `&keyboxes[i]` | statusbaron | - |
| `w_ammo[0..3]` (1374-1404) | number | (288,173) (288,179) (288,191) (288,185) | `shortnum` | `&plyr->ammo[i]` (clip, shell, cell, missile) | statusbaron | 3 |
| `w_maxammo[0..3]` (1407-1437) | number | (314,173) (314,179) (314,191) (314,185) | `shortnum` | `&plyr->maxammo[i]` | statusbaron | 3 |

All position constants defined in st_stuff.c:86-265. Every one is listed. A dash in the
"Used" column means the grep found no use outside the `#define`.

| Constant | Value | Used | Constant | Value | Used |
|---|---|---|---|---|---|
| ST_X | 0 | yes | ST_X2 | 104 | - |
| ST_FX | 143 | faceback x | ST_FY | 169 | - |
| ST_TALLNUMWIDTH | `tallnum[0]->width` | - | ST_FACESX / ST_FACESY | 143 / 168 | yes |
| ST_AMMOWIDTH/X/Y | 3 / 44 / 171 | yes | ST_HEALTHWIDTH | 3 | - |
| ST_HEALTHX/Y | 90 / 171 | yes | ST_ARMSX/Y | 111 / 172 | yes |
| ST_ARMSBGX/Y | 104 / 168 | yes | ST_ARMSXSPACE/YSPACE | 12 / 10 | yes |
| ST_FRAGSX/Y/WIDTH | 138 / 171 / 2 | yes | ST_ARMORWIDTH | 3 | - |
| ST_ARMORX/Y | 221 / 171 | yes | ST_KEY0WIDTH/HEIGHT | 8 / 5 | - (WIDTH only feeds KEY1/2WIDTH) |
| ST_KEY0X/Y, KEY1X/Y, KEY2X/Y | 239 / 171, 181, 191 | yes | ST_AMMO0..3WIDTH | 3 | yes |
| ST_AMMO0HEIGHT | 6 | - | ST_AMMO0..3X / Y | 288 / 173, 179, 191, 185 | yes |
| ST_MAXAMMO0..3WIDTH | 3 | yes | ST_MAXAMMO0HEIGHT | 5 | - |
| ST_MAXAMMO0..3X / Y | 314 / 173, 179, 191, 185 | yes | ST_WEAPON0..5X/Y | 110,122,134 / 172,181 | - (the real weapon slots are ST_ARMS*) |
| ST_WPNSX/Y, ST_DETHX/Y | 109 / 191 | - | ST_MSGTEXTX/Y, ST_MSGWIDTH, ST_MSGHEIGHT | 0 / 0, 52, 1 | only MSGWIDTH (mypos buffer) |
| ST_OUTTEXTX/Y, OUTWIDTH, OUTHEIGHT | 0 / 6, 52, 1 | - | ST_MAPWIDTH/TITLEX/TITLEY/HEIGHT | expressions | - |
| ST_FACEPROBABILITY | 96 | - | ST_TOGGLECHAT | KEY_ENTER | - |

#### 1.4.8 st_lib.c, exactly (st_lib.c:58-292)

Buffers: `FG` = 0 and `BG` = 4 (st_lib.h:33-34). Every erase copies **from BG at
`(x, y - 168)` to FG at `(x, y)`**.

1. **STlib_initNum** (66-82) sets `x, y, oldnum = 0, width, num, on, p`. It does not set
   `data`.
2. **STlib_drawNum(n, refresh)** (91-148). The `refresh` argument is not used.
   1. `numdigits = n->width; num = *n->num; w = SHORT(n->p[0]->width); h =
      SHORT(n->p[0]->height);`. The box size comes from **digit 0's** header, not the
      digit being drawn.
   2. `n->oldnum = *n->num;` (write-only).
   3. `neg = num < 0;` If negative: `if (numdigits == 2 && num < -9) num = -9; else if
      (numdigits == 3 && num < -99) num = -99; num = -num;`.
   4. Erase: `x = n->x - numdigits*w;`. If `n->y - ST_Y < 0`, then
      `I_Error("drawNum: n->y - ST_Y < 0")`; with the constants above it never fires
      (every number y is 171 or more). Then `V_CopyRect(x, n->y - 168, BG, w*numdigits,
      h, x, n->y, FG);`. That is the rectangle `[n->x - numdigits*w, n->x) x [n->y,
      n->y + h)`, ignoring patch offsets.
   5. `if (num == 1994) return;`. The area is erased and nothing is drawn. This blanks
      the ready-ammo field for fist and chainsaw.
   6. `x = n->x; if (!num) V_DrawPatch(x - w, n->y, FG, n->p[0]);`
   7. `while (num && numdigits--) { x -= w; V_DrawPatch(x, n->y, FG, n->p[num % 10]);
      num /= 10; }`. Digits go right to left, each drawn at `x` minus that patch's own
      offsets. A value with more digits than `numdigits` loses its high digits silently.
   8. `if (neg) V_DrawPatch(x - 8, n->y, FG, sttminus);`. The minus goes 8 pixels left
      of the last digit drawn.
   9. Net effect: **every** call erases and redraws, with no comparison against the old
      value.
3. **STlib_updateNum** (153-158): `if (*n->on) STlib_drawNum(n, refresh);`. So every
   frame with the bar on redraws every number.
4. **STlib_initPercent / STlib_updatePercent** (163-188):
   `if (refresh && *per->n.on) V_DrawPatch(per->n.x, per->n.y, FG, per->p);` then
   `STlib_updateNum(&per->n, refresh);`. The percent sign is drawn at `n.x`, outside the
   number's erase box `[n.x - 3w, n.x)`. It is drawn **only on refresh** and otherwise
   persists in FG.
5. **STlib_initMultIcon** (193-207): `oldinum = -1`.
6. **STlib_updateMultIcon** (212-240):
   `if (*mi->on && (mi->oldinum != *mi->inum || refresh) && (*mi->inum != -1)) {`
   1. if `oldinum != -1`: `x = mi->x - SHORT(old->leftoffset); y = mi->y -
      SHORT(old->topoffset); w = SHORT(old->width); h = SHORT(old->height);`, where
      `old = mi->p[mi->oldinum]`. If `y - ST_Y < 0`, then
      `I_Error("updateMultIcon: y - ST_Y < 0")`. Then `V_CopyRect(x, y-168, BG, w, h,
      x, y, FG);`.
   2. `V_DrawPatch(mi->x, mi->y, FG, mi->p[*mi->inum]);`
   3. `mi->oldinum = *mi->inum;`
   - Not redrawn: when `*inum` becomes -1, **nothing happens**. The old icon stays in
     FG and `oldinum` keeps the old index. When the bar is off, nothing happens and
     `oldinum` keeps its old value. The erase restores **BG** pixels, which are STBAR
     (plus STFB in a netgame). It does not restore anything drawn into FG after the
     background, such as STARMS under the weapon digits.
7. **STlib_initBinIcon** (245-259): `oldval = 0`.
8. **STlib_updateBinIcon** (264-292):
   `if (*bi->on && (bi->oldval != *bi->val || refresh)) {` compute the box from the
   current patch's offsets and size. If `y - ST_Y < 0`, then
   `I_Error("updateBinIcon: y - ST_Y < 0")`; this check runs **before** both branches.
   Then `if (*bi->val) V_DrawPatch(bi->x, bi->y, FG, bi->p); else V_CopyRect(x,
   y-168, BG, w, h, x, y, FG);` and `bi->oldval = *bi->val;`. When the value goes
   false, the patch's box is restored from BG. When it is true, the patch is drawn over
   whatever FG holds.

#### 1.4.9 The v_video.c functions used

1. **The buffers.** `byte* screens[5]` (v_video.c:44). V_Init (482-493) allocates
   `I_AllocLow(320*200*4)` (zero-filled, i_system.c:147-154) and points `screens[0..3]`
   into it. I_InitGraphics later **replaces** `screens[0]` with the X image data or a
   separate `malloc(320*200)` (i_video.c:910-913). `screens[1]` is the back screen
   (border pattern, r_draw.c:756, 827). `screens[2]` and `screens[3]` are the wipe
   buffers (f_wipe.c:243, 255). `screens[4]` is the status bar BG, **10,240 bytes**
   (ST_Init). It is indexed with the full `SCREENWIDTH` stride (v_video.c:187-188,
   239), so its 32 rows are 320 bytes each and row r starts at r*320.
2. **V_DrawPatch(x, y, scrn, patch)** (203-263):
   1. `y -= SHORT(patch->topoffset); x -= SHORT(patch->leftoffset);`
   2. RANGECHECK (on in this build): `if (x<0 || x+width > 320 || y<0 || y+height > 200
      || (unsigned)scrn > 4)`, then two `fprintf(stderr, ...)` lines and **`return`**.
      It is **not** an I_Error: the patch is silently skipped (comment: "what is up with
      TNT.WAD?"). The check uses the 320x200 screen size even when `scrn` is 4, so a
      patch reaching past row 31 of BG is **not** caught.
   3. `if (!scrn) V_MarkRect(...)`, which has no effect on pixels.
   4. For `col` = 0..width-1: `column = patch + LONG(columnofs[col])`. Posts follow until
      `topdelta == 0xff`. For each post: `source = column + 3`, `dest = desttop +
      topdelta*320`, copy `length` bytes down the column, next post at `column + length
      + 4`. There is **no clipping** per post. A post with `topdelta + length` beyond
      the header `height` writes below the checked area, and past the buffer's end if it
      passes the last row. Unchecked: `columnofs[]` and the post bytes against the lump
      size.
   5. Transparent pixels are those not covered by any post. They leave the destination
      unchanged.
3. **V_CopyRect** (157-196): RANGECHECK `if (srcx<0 || srcx+width > 320 || srcy<0 ||
   srcy+height > 200 || destx<0 || destx+width > 320 || desty<0 || desty+height > 200
   || (unsigned)srcscrn > 4 || (unsigned)destscrn > 4)`, then **`I_Error("Bad
   V_CopyRect")`**. Then V_MarkRect and a `memcpy` per row with stride 320. Here too
   the source check allows `srcy + height` up to 200, while BG holds only 32 rows. An
   erase box that runs past BG row 31 reads beyond the 10,240-byte zone block and is not
   caught.
4. **V_DrawPatchDirect** (336-396) only calls `V_DrawPatch(x, y, scrn, patch)` (343);
   the rest is commented out. The status bar does not call it.
5. Without RANGECHECK, which this build does not use, every one of the checks above
   would be absent.

#### 1.4.10 ST_doPaletteStuff (st_stuff.c:1000-1052)

1. `cnt = plyr->damagecount;`
2. `if (plyr->powers[pw_strength]) { bzc = 12 - (plyr->powers[pw_strength]>>6); if (bzc
   > cnt) cnt = bzc; }`
3. `if (cnt) { palette = (cnt+7)>>3; if (palette >= 8) palette = 7; palette += 1; }`
4. `else if (plyr->bonuscount) { palette = (bonuscount+7)>>3; if (palette >= 4) palette
   = 3; palette += 9; }`
5. `else if (plyr->powers[pw_ironfeet] > 4*32 || plyr->powers[pw_ironfeet]&8) palette =
   13;`
6. `else palette = 0;`
7. `if (palette != st_palette) { st_palette = palette; pal = W_CacheLumpNum(lu_palette,
   PU_CACHE) + palette*768; I_SetPalette(pal); }`

**Table** (from the calculator; the first matching row wins):

| Condition | Palette |
|---|---|
| `cnt` = max(`damagecount`, `bzc` if berserk) is 1-8 | 2 |
| `cnt` 9-16 / 17-24 / 25-32 / 33-40 / 41-48 | 3 / 4 / 5 / 6 / 7 |
| `cnt` 49 or more (`damagecount` is at most 100) | 8 |
| `cnt` = 0 and `bonuscount` 1-8 / 9-16 / 17 or more | 10 / 11 / 12 |
| both 0 and `ironfeet` > 128, or `ironfeet` in 1..128 with bit 3 set (8-15, 24-31, ..., 120-127) | 13 |
| otherwise | 0 |

Berserk alone: `powers[pw_strength]` is set to 1 on pickup (p_inter.c:319-323) and
counts up every tic (p_user.c:338-340) with no limit. From 1 to 255 it gives `bzc` 12-9,
which is palette 3. From 256 to 767 it gives 8-1, which is palette 2. From 768 on, `bzc`
is 0 or less and has no effect. **Palettes 1 and 9 can never be chosen**, because
`(cnt+7)>>3` is at least 1. Without berserk the reachable set is {0, 2-8, 10-12, 13};
berserk adds no new numbers. Invulnerability does not enter this function. It works
through `fixedcolormap` in p_user.c:362-367, and it blocks damage under 1000 at
p_inter.c:846-851.

8. **First frame.** ST_initData sets `st_palette = -1` (1266), so the first ST_Drawer
   after every ST_Start always calls I_SetPalette. Before that, ST_Stop (every ST_Start
   but the first) has already set palette 0 (1461). The palette is decided per **frame**
   from player state at draw time, including when the bar is off.
9. **What I_SetPalette does on this platform.** It calls `UploadNewPalette` (i_video.c:582-585),
   which applies `gammatable[usegamma][...]` to each byte (i_video.c:563-568). It changes
   the X colormap **only** for an 8-bit PseudoColor visual (i_video.c:546-548); on other
   visuals the call has no visible effect. The 320x200 frame of palette **indices** does
   not depend on the palette at all.

#### 1.4.11 Screenblocks 10 against 11

1. At **11**, `viewheight == 200` (r_main.c:682-685), so `fullscreen` is true and
   `st_statusbaron` is false while the automap is inactive. ST_doPaletteStuff still runs
   and chooses the palette. If `st_firsttime` is set, ST_doRefresh clears it but
   ST_refreshBackground draws nothing. In ST_drawWidgets every `on` flag is false
   (`st_statusbaron`, `st_armson`, `st_fragson`), so **nothing is drawn and `oldinum` and
   `oldval` do not change**. ST_Ticker runs as usual: `M_Random`, the face state machine,
   key boxes, frags and `st_oldhealth` all advance. FG rows 168-199 are covered by the 3D
   view every frame.
2. **When the bar comes back** (11 to 10), D_Display sets `redrawsbar` because
   `viewheight != 200 && fullscreen` (d_main.c:242-243). ST_doRefresh redraws BG and FG
   and every widget with `refresh = true`. Multi-icons first erase their stale `oldinum`
   box from the fresh BG, which changes nothing. After **any** refresh frame the bar's
   pixels depend only on STBAR, the patches and the current values (inference from
   1.4.6 and 1.4.8).
3. **One screenblocks value for the whole session.**
   - **At 11:** no status bar pixels ever, and one I_SetPalette per change of palette
     number.
   - **At 10**, the first displayed frame after each ST_Start is a refresh. `st_firsttime`
     is set, and the level-start wipe also sets `refresh` (g_game.c:472-473,
     d_main.c:242). The frame runs:
     1. the palette;
     2. STBAR into BG rows 0-31;
     3. BG copied into FG rows 168-199;
     4. `w_ready`: its erase box, then digits or blank;
     5. for each ammo type i: `w_ammo[i]`, then `w_maxammo[i]`, each an erase box and
        digits;
     6. health: the percent sign at (90,171), then its erase box and digits;
     7. armour: the percent sign at (221,171), then its erase box and digits;
     8. STARMS at (104,168), since `st_notdeathmatch` is true;
     9. six weapon digits (no erase, because `oldinum` is -1);
     10. the face (no erase);
     11. each key box whose value is not -1;
     12. frags (skipped: off outside deathmatch).
   - Every later frame at 10 until the next ST_Start is a diff frame:
     1. all nine numbers are erased and redrawn;
     2. percent signs and STARMS are not touched;
     3. a weapon digit, the face or a key icon changes only when its index changes: erase
        the old patch's box from BG, then draw the new patch;
     4. a key going to -1 would leave its icon. Inside one level cards are only gained,
        and G_PlayerFinishLevel clears them only between levels (g_game.c:786).
   - Without menus, the automap or a change of screen size, there is no other refresh
     within a level (d_main.c:242-246).

#### 1.4.12 Undefined, implementation-defined and layout-dependent behaviour; every I_Error

1. **Dangling `plyr->attacker`** (st_stuff.c:816-817) after the attacker's mobj is freed
   (1.4.4). This is a read of freed memory, which is undefined behaviour. In Doom 1 it
   is reached by a lost soul that hurt the player within the last `damagecount` tics and
   has since finished dying.
2. **Out-of-bounds pointer** `&plyr->ammo[am_noammo]`, which is `ammo[5]` of an array of
   4 (st_stuff.c:1292, doomdef.h:203-208), when a level starts with fist or chainsaw
   ready. Computing it is undefined behaviour. It is not dereferenced before ST_Ticker
   replaces it, because level load and ST_Ticker run in the same G_Ticker before any
   D_Display (g_game.c:617-622, 730-731; d_main.c:238) (inference).
3. **`(int *)&plyr->weaponowned[i+1]`** (1322) relies on `sizeof(boolean) ==
   sizeof(int)`. `boolean` is a C enum (doomtype.h:34), and its size is
   implementation-defined (GCC: 4). The values are only 0 or 1, so the index into
   `arms[i][...]` stays in bounds.
4. **Indeterminate BG**: `screens[4]` comes from Z_Malloc without clearing. Any BG pixel
   that STBAR's posts do not cover keeps indeterminate contents, which are copied into
   FG and used by every erase. Whether STBAR covers all 320x32 pixels depends on the
   lump.
5. **BG overrun**: V_DrawPatch into BG checks against 200 rows, not 32 (1.4.9). A STBAR
   or STFB taller than 32 pixels, or a post running past row 31, writes past the
   10,240-byte block into zone headers. Erase boxes reading past BG row 31 (srcy + h >
   32) read beyond the block. Examples: a digit-0 patch taller than 9 pixels for the
   y = 191 numbers, or a key or face patch whose `y - topoffset - 168 + height` exceeds
   32. These depend on the lumps and are undetected in C.
6. **Unchecked lump sizes**: PLAYPAL needs at least 10,752 bytes; patch `columnofs` and
   posts are not checked against the lump length.
7. **Signed overflow**: `st_msgcounter` after 2^31 tics (1.4.3). Sim side:
   `powers[pw_strength]++` after 2^31 tics, after which `>>6` on a negative int is
   implementation-defined, would make `bzc` large and the palette red. Both are remote.
8. **Number wider than its field**: high digits are dropped silently (1.4.8 step 2.7).
   Negative values are clamped only for widths 2 and 3. Player health, armour and ammo
   never go below 0 (p_inter.c:870-871 for health); only frags can, and frags are shown
   only in deathmatch.
9. **Face, arms and key indices**: all in bounds (faces 0-41, arms 0/1, keys -1 to 5,
   with -1 never used as an index because of the `!= -1` guard).
10. **I_Error calls that can fire**:
    - `"drawNum: n->y - ST_Y < 0"` (st_lib.c:123): cannot fire with these constants.
    - `"updateMultIcon: y - ST_Y < 0"` (st_lib.c:233): fires if the **old** patch's
      `topoffset` exceeds `mi->y - 168`. For the face at y = 168, any positive
      `topoffset` on the previous face.
    - `"updateBinIcon: y - ST_Y < 0"` (st_lib.c:282): fires on the first refresh if
      STARMS has `topoffset > 0`.
    - `"Bad V_CopyRect"` (v_video.c:182): an erase box outside 320x200, for example a
      large negative `leftoffset`.
    - `"W_GetNumForName: %s not found!"` (w_wad.c:406) for any missing lump.
    - `"Z_Free: freed a pointer without ZONEID"` is not reachable from this code.
    - V_DrawPatch's range failure is a **silent skip** (v_video.c:228-231), not an
      error. The caller still updates `oldinum` and `oldval`.

#### 1.4.13 History the Rust status bar render state must carry

- FG rows 168-199 (10,240 bytes) and the BG buffer (10,240 bytes), or a representation
  proven equal to them. Diff frames draw onto what is already there. The weapon-digit
  erase restores STBAR, not STARMS, so after a weapon pickup the bar can differ from what
  a refresh would draw.
- `oldinum` for `w_arms[6]`, `w_faces` and `w_keyboxes[3]`; `oldval` for `w_armsbg`.
- `st_firsttime`, `st_palette`, `st_stopped`.
- The face machine: `st_facecount`, `st_faceindex`, `priority`, `lastattackdown` and
  `oldweaponsowned[9]`, plus `st_oldhealth`. `st_facecount`, `priority` and
  `lastattackdown` survive level changes, new games and death reloads. For example, a
  level that ends with `st_facecount = k > 0` at `priority` 0 shows `faces[0]` (STFST00,
  whatever the health) for the first k tics of the next level (inference from 1.4.4 and
  `st_faceindex = 0` at 1265).
- `M_Random`'s `rndindex`, shared with wipe, intermission and sound.
- The cheat cursors, only if keydowns are ever delivered to an ST_Responder equivalent.
- Not needed: `oldnum`, the ST_calcPainOffset cache, `st_clock`, `st_msgcounter`, the
  chat statics, `st_randomnumber` (rewritten before use each tic), `keyboxes` and
  `st_fragscount` (recomputed each tic).

#### 1.4.14 Hazards for a pixel-exact port of the status bar

- A port that computes the bar from state alone is wrong after a weapon-digit change.
  Replay the same V_DrawPatch and V_CopyRect calls, in the same order, on persistent FG
  and BG.
- Percent signs and STARMS are drawn only on refresh; numbers on every frame; icons only
  on change. A key going to -1 is never erased.
- Number boxes use digit 0's width and height and ignore offsets; icon boxes use the
  **previous** patch's offsets and size.
- V_DrawPatch silently skips out-of-range patches, while V_CopyRect stops with I_Error.
  Neither range check protects the 32-row BG.
- Widget values are read at draw time, but the face index is computed at tic time. With
  more than one tic per frame, intermediate faces are never drawn.
- The palette is chosen per frame, even at screenblocks 11, from draw-time state.
  Palettes 1 and 9 cannot occur.
- The face uses the reversed ouch test and an unfolded `diffang`, and with a removed
  attacker it reads freed memory.
- Status bar drawing does nothing before the first tic (`gametic` must be nonzero) and
  only in `GS_LEVEL`. Under `-devparm`, I_FinishUpdate writes dots into FG row 199
  (i_video.c:361-372), which the diff drawing never repairs.
- `attackdown` is written during the intermission (wi_stuff.c:1482-1487) and is not reset
  at the next spawn, so it feeds `lastattackdown` at the start of the next level.

#### 1.4.15 Questions the status bar facts raise for the owner

1. **How to represent the bar's history**: keep the raw FG and BG regions (20,480 bytes)
   in the saveable render state, or a compact model (oldinum/oldval plus which weapon
   slots have been erased to STBAR since the last refresh) with a proof that it is
   equal. Facts: 1.4.8 steps 6 and 8, and the STARMS-under-digits case.
2. **A dangling attacker**: options are a refuse-the-tic witness whenever
   `damagecount != 0`, the attacker has been removed, and the turn branch would run; or
   modelling freed-memory contents (not practical); or using the last known position
   (a silent divergence, so not allowed). Facts: 1.4.4; lost souls are removed on death
   (info.c:736). Whether E1 maps contain lost souls was not checked (no IWAD access).
3. **Matching `M_Random`**: the look-around face needs every `M_Random` consumer in the
   process in order (wipe, intermission, sound pitch). Options: model them all, or
   refuse when the reference's `rndindex` differs. Cross-reference: the M_Random
   section.
4. **Key events and cheats**: in C every in-level keydown passes through
   `ST_Responder`. Options: deliver no events to a cheat matcher and make the C harness
   deliver none either (ticcmds only), or carry the matcher and refuse when a sequence
   completes. Facts: 1.4.5.
5. **Lump geometry that C does not check**: STBAR coverage, BG overruns, and offsets
   that trigger I_Error or a silent skip. Options: validate at load time and refuse the
   WAD, or emulate with a witness. Facts: 1.4.9 and 1.4.12.
6. **What to compare for the palette**: the palette number per frame (`st_palette`), or
   RGB after `gammatable[usegamma]`. Facts: 1.4.10 steps 8-9; linuxdoom applies it only
   on 8-bit PseudoColor visuals.
7. **Carried statics across new games and savegame loads**: C resets `st_facecount`,
   `priority` and `lastattackdown` only at process start, and its savegames do not
   contain them. Options: carry them in the Rust render save and accept that a C
   reference must replay from process start, or define a reset point and document it as
   a declared adapter.
8. **Intermission scope**: whether the intermission's write to `attackdown`
   (wi_stuff.c:1482-1487) is part of M5b, since it affects the face timing on the next
   level.

Each question is taken up in §4: 1 in S18; 2 in S17; 3 in S15; 4 in S5; 5 in S17; 6 in S22 (the palette number is what is compared) and S13 (colours are presentation); 7 in S11; 8 is out of scope: M5b has no intermission (S1, S11).

### 1.5 HUD messages: hu_stuff.c, hu_lib.c and every player->message

This section records what the original linuxdoom-1.10 source does for the heads-up
message line, the map-title and chat widgets around it, and every place the game assigns
`player->message`. All citations are into `linuxdoom-1.10/` at the authority commit. No
IWAD was read: glyph widths, heights and offsets are whatever the `STCFN` lumps say at run
time, and nothing below depends on their values except where it says so. No C probe was
compiled; every fact is from reading the source. Statements marked *inference* are
derived from the cited code rather than read directly.

Constants used throughout:

| Name | Value | Where |
| --- | --- | --- |
| `HU_FONTSTART` | `'!'` (33) | hu_stuff.h:30 |
| `HU_FONTEND` | `'_'` (95) | hu_stuff.h:31 |
| `HU_FONTSIZE` | `HU_FONTEND - HU_FONTSTART + 1` = 63 | hu_stuff.h:34 |
| `HU_BROADCAST` | 5 | hu_stuff.h:36 |
| `HU_MSGREFRESH` | `KEY_ENTER` = 13 | hu_stuff.h:38, doomdef.h:255 |
| `HU_MSGX`, `HU_MSGY` | 0, 0 | hu_stuff.h:39-40 |
| `HU_MSGWIDTH` | 64 (unused by any code) | hu_stuff.h:41 |
| `HU_MSGHEIGHT` | 1 line | hu_stuff.h:42 |
| `HU_MSGTIMEOUT` | `4*TICRATE` = 140 tics | hu_stuff.h:44, doomdef.h:122 |
| `HU_MAXLINES` | 4 | hu_lib.h:35 |
| `HU_MAXLINELENGTH` | 80 | hu_lib.h:36 |
| `FG`, `BG` | 0, 1 (screen numbers) | hu_lib.h:30-31 |
| `HU_TITLEX`, `HU_TITLEY` | 0, `167 - SHORT(hu_font[0]->height)` | hu_stuff.c:54-55 |
| `HU_INPUTTOGGLE` | `'t'` | hu_stuff.c:57 |
| `HU_INPUTX`, `HU_INPUTY` | `HU_MSGX`, `HU_MSGY + HU_MSGHEIGHT*(SHORT(hu_font[0]->height)+1)` | hu_stuff.c:58-59 |
| `QUEUESIZE` | 128 (chat queue) | hu_stuff.c:580 |
| `TURBOTHRESHOLD` | 0x32 | g_game.c:173 |

#### 1.5.1 Lifecycle: HU_Init, HU_Start, HU_Stop and their callers

1. **HU_Init** (hu_stuff.c:392-412) is called once, from `D_DoomMain` (d_main.c:1109-1110).
   It sets `shiftxform` (see fact 1.5.10.4 for why the French table is always chosen),
   then loads the font: `j = HU_FONTSTART` and for `i = 0 .. HU_FONTSIZE-1` it does
   `sprintf(buffer, "STCFN%.3d", j++)` and `hu_font[i] = W_CacheLumpName(buffer, PU_STATIC)`
   (hu_stuff.c:405-410). So it loads 63 lumps named `STCFN033` through `STCFN095`, one per
   character code 33 (`!`) to 95 (`_`), and `hu_font[c - '!']` is the glyph for character
   `c`. `buffer` is `char[9]` (hu_stuff.c:397), exactly 8 characters plus the terminator.
   A missing lump is fatal: `W_CacheLumpName` goes through `W_GetNumForName`, which calls
   `I_Error("W_GetNumForName: %s not found!")` (w_wad.c:406).
2. **HU_Stop** (hu_stuff.c:414-417) only sets `headsupactive = false`. Its only caller is
   HU_Start (hu_stuff.c:425-426).
3. **HU_Start** (hu_stuff.c:419-484), in order:
   1. `if (headsupactive) HU_Stop();` (425-426).
   2. `plr = &players[consoleplayer];` (428).
   3. `message_on = false; message_dontfuckwithme = false; message_nottobefuckedwith = false;
      chat_on = false;` (429-432).
   4. `HUlib_initSText(&w_message, HU_MSGX, HU_MSGY, HU_MSGHEIGHT, hu_font, HU_FONTSTART,
      &message_on)` (435-438): one line at (0, 0), font start `'!'`, visibility tied to
      `message_on`. See fact 1.5.8.6 for what it sets.
   5. `HUlib_initTextLine(&w_title, HU_TITLEX, HU_TITLEY, hu_font, HU_FONTSTART)` (441-444),
      then the title string is chosen by `gamemode` (446-467): `shareware`, `registered`,
      `retail` use `HU_TITLE` = `mapnames[(gameepisode-1)*9+gamemap-1]` (hu_stuff.c:49);
      `commercial` and `default` use `HU_TITLE2` = `mapnames2[gamemap-1]` (hu_stuff.c:50).
      The Plutonia and TNT cases (`mapnamesp`, `mapnamest`, hu_stuff.c:208-283) are commented
      out under `FIXME` (454-461), so those tables are never used. `mapnames`
      (hu_stuff.c:115-167) holds `HUSTR_E1M1`..`HUSTR_E4M9` and then nine `"NEWLEVEL"`;
      for Episode 1 the strings are `HUSTR_E1M1`..`HUSTR_E1M9` (d_englsh.h:142-150, e.g.
      `"E1M1: Hangar"`). Every character is appended with `HUlib_addCharToTextLine`
      (469-470). The title is only drawn when `automapactive` (hu_stuff.c:491-492), so it
      is out of scope for M5b; it is still built at every HU_Start.
   6. `HUlib_initIText(&w_chat, HU_INPUTX, HU_INPUTY, hu_font, HU_FONTSTART, &chat_on)`
      (473-476): the chat input line one font-height-plus-one below the message line.
   7. For each of the `MAXPLAYERS` entries, `HUlib_initIText(&w_inputbuffer[i], 0, 0, 0, 0,
      &always_off)` (479-480): font pointer NULL, never drawn or erased (HU_Drawer and
      HU_Erase do not touch them), used only as per-player chat accumulators.
   8. `headsupactive = true;` (482).
   HU_Start reads `hu_font[0]->height` three times (via `HUlib_initSText` hu_lib.c:195,
   `HU_TITLEY` and `HU_INPUTY`) regardless of whether anything is drawn.
4. **Who calls HU_Start.** Only `P_SpawnPlayer` (p_mobj.c:693-698), and only when the
   spawned player is the console player (`mthing->type-1 == consoleplayer`, p_mobj.c:693).
   `P_SpawnPlayer` is called from `P_SpawnMapThing` for a player start of type 1..4 when
   `!deathmatch` (p_mobj.c:729-735) and from `G_DoReborn`/`G_DeathMatchSpawnPlayer` in
   net games (g_game.c:912-965). In single player a death sets `gameaction = ga_loadlevel`
   (g_game.c:928-932), so every level load, including the reload after death, runs
   `G_DoLoadLevel` -> `P_SetupLevel` -> `P_SpawnMapThing` -> `P_SpawnPlayer` -> `HU_Start`.
   If a map has several type-1 starts, HU_Start runs once per start; each run fully
   re-initializes (*inference*: harmless). If a map had no type-1 start, HU_Start would
   not run and `plr` would keep its previous value (NULL before the first level, which
   HU_Ticker would dereference at hu_stuff.c:522; *inference*).
5. **What a new level does not reset.** HU_Start does not touch `message_counter`,
   `always_off`, the chat queue (`chatchars`, `head`, `tail`), `chat_dest`, or the
   HU_Responder statics (`lastmessage`, `shiftdown`, `altdown`, `num_nobrainers`), nor
   `lastautomapactive` in hu_lib.c. See the table in 1.5.2.

#### 1.5.2 Every piece of HUD state

File-scope state in hu_stuff.c (types and initial values as declared):

| Name | Declaration | Initial | Written | Reset at HU_Start | Meaning |
| --- | --- | --- | --- | --- | --- |
| `chat_char` | hu_stuff.c:88 `char` | 0 | nowhere | no | unused ("remove later") |
| `plr` | hu_stuff.c:89 `static player_t*` | NULL | 428 | set | the console player whose `message` the HUD reads and clears |
| `hu_font[63]` | hu_stuff.c:90 `patch_t*[]` | NULL | 409 (HU_Init) | no | glyphs `'!'`..`'_'` |
| `w_title` | hu_stuff.c:91 `static hu_textline_t` | zero | 441-470, 501 | yes | map title (automap only) |
| `chat_on` | hu_stuff.c:92 `boolean` (global, also read by m_menu.c:71, 1523, 1530) | false | 432, 667, 679, 723, 745, 753 | yes (false) | chat input open |
| `w_chat` | hu_stuff.c:93 `static hu_itext_t` | zero | 473-476, 668, 680, 734 | yes | chat input line |
| `always_off` | hu_stuff.c:94 `static boolean` | false | nowhere | n/a | the permanently false visibility flag of `w_inputbuffer` |
| `chat_dest[4]` | hu_stuff.c:95 `static char[]` | 0 | 546 (netgame) | **no** | per-player chat destination |
| `w_inputbuffer[4]` | hu_stuff.c:96 `static hu_itext_t[]` | zero | 480, 551, 570 | yes | per-player incoming chat |
| `message_on` | hu_stuff.c:98 `static boolean` | false (zero) | 429, 514, 527, 563, 661 | yes (false) | the message line is drawn |
| `message_dontfuckwithme` | hu_stuff.c:99 `boolean` (global; written by m_menu.c:989) | false (zero) | 430, 530, m_menu.c:989 | yes (false) | the pending message comes from the messages toggle and must be shown even when messages are off |
| `message_nottobefuckedwith` | hu_stuff.c:100 `static boolean` | false (zero) | 431, 515, 529, 562 | yes (false) | the message on screen is protected: ordinary messages may not replace it until it times out |
| `w_message` | hu_stuff.c:102 `static hu_stext_t` | zero | 435, 525, 558, 499 | yes | the message widget |
| `message_counter` | hu_stuff.c:103 `static int` | 0 | 512, 528, 564, 662 | **no** | tics left before `message_on` is cleared |
| `headsupactive` | hu_stuff.c:108 `static boolean` | false | 416, 482 | set true | read only by HU_Start |
| `shiftxform` | hu_stuff.c:286 `const char*` | NULL | 400/402 (HU_Init) | no | chat shift table |
| `chatchars[128]` | hu_stuff.c:582 `static char[]` | 0 | 595 | **no** | outgoing chat queue |
| `head`, `tail` | hu_stuff.c:583-584 `static int` | 0, 0 | 596, 607 | **no** | queue indices |

Function-local statics in `HU_Responder` (hu_stuff.c:617-759):

| Name | Declaration | Initial | Written | Meaning |
| --- | --- | --- | --- | --- |
| `lastmessage[81]` | hu_stuff.c:620 | zero | 724, 748 (chat only) | copy of the last chat line sent, so `plr->message` can point at it |
| `shiftdown` | hu_stuff.c:623 | false | 645 | right shift held |
| `altdown` | hu_stuff.c:624 | false | 650 | alt held |
| `destination_keys[4]` | hu_stuff.c:629-635 | `'g','i','b','r'` (d_englsh.h:317-320) | never | chat destination keys |
| `num_nobrainers` | hu_stuff.c:637 | 0 | 686 (netgame, more than 2 players) | count of attempts to chat to oneself |

Function-local static in hu_lib.c: `lastautomapactive` (hu_lib.c:148, `static boolean`,
initial `true`), written at hu_lib.c:171 and **never read** (dead state).

`showMessages` is owned by m_menu.c: `int showMessages;` (m_menu.c:79, commented "Show
messages has default, 0 = off, 1 = on", m_menu.c:78). `M_LoadDefaults` first stores the
default value 1 from the defaults table entry `{"show_messages",&showMessages, 1}`
(m_misc.c:239, 352-354), then a config file may override it with any integer parsed by
`sscanf("%i")` (m_misc.c:384-392). hu_stuff.c reads it as `extern int` (hu_stuff.c:105) and
tests it only for nonzero (hu_stuff.c:518). `M_ChangeMessages` sets
`showMessages = 1 - showMessages` (m_menu.c:982), so a config value of 2 toggles to -1,
which is still "on" (*inference* from the arithmetic).

Widget structs (hu_lib.h):

- `hu_textline_t` (hu_lib.h:44-58): `int x, y` (left edge and top of the line, before patch
  offsets), `patch_t** f` (font), `int sc` (start character, `'!'`), `char l[81]` (text,
  NUL-terminated), `int len` (characters in `l`), `int needsupdate` (frames left for which
  the erase routine should repaint the border behind this line; see 1.5.6).
- `hu_stext_t` (hu_lib.h:64-74): `hu_textline_t l[4]`, `int h` (lines in use, 1 for
  `w_message`), `int cl` (index of the current, most recent line), `boolean* on` (points
  at `message_on`), `boolean laston` (value of `*on` at the previous erase).
- `hu_itext_t` (hu_lib.h:80-91): `hu_textline_t l`, `int lm` (left margin that deletion
  may not pass), `boolean* on`, `boolean laston`.

#### 1.5.3 HU_Ticker in full

`HU_Ticker` (hu_stuff.c:505-578) is called only from `G_Ticker`, in the `GS_LEVEL` case,
after `P_Ticker`, `ST_Ticker` and `AM_Ticker` (g_game.c:729-734). It runs even when the
game is paused or the menu is open, because those only make `P_Ticker` return early
(p_tick.c:135-145). It does not run during intermission or finale.

1. **Countdown** (hu_stuff.c:512-516):
   ```c
   if (message_counter && !--message_counter)
   {
       message_on = false;
       message_nottobefuckedwith = false;
   }
   ```
   A zero counter is left alone. The decrement happens before any new message is taken,
   so a message arriving on the tic the counter reaches zero is shown on that same tic.
2. **Taking a message** (hu_stuff.c:518-533):
   ```c
   if (showMessages || message_dontfuckwithme)
   {
       if ((plr->message && !message_nottobefuckedwith)
           || (plr->message && message_dontfuckwithme))
       {
           HUlib_addMessageToSText(&w_message, 0, plr->message);
           plr->message = 0;
           message_on = true;
           message_counter = HU_MSGTIMEOUT;
           message_nottobefuckedwith = message_dontfuckwithme;
           message_dontfuckwithme = 0;
       }
   } // else message_on = false;
   ```
   The text is copied character by character into the widget (no prefix), so after this
   the widget no longer depends on the pointer. The counter becomes 140.
3. **Net chat** (hu_stuff.c:535-576) is guarded by `if (netgame)` (536) and is skipped in
   single player. (It reads `players[i].cmd.chatchar` for other players and appends
   finished lines with a player-name prefix, setting `message_nottobefuckedwith = true`,
   `message_on = true`, `message_counter = HU_MSGTIMEOUT` and starting a sound.)
4. **How long a message is visible** (*inference* from 1 and 2). A message taken in tic T
   leaves `message_counter == 140` and `message_on == true`. Tics T+1 .. T+139 decrement it
   to 1; tic T+140 decrements it to 0 and clears `message_on`. So the message is drawn in
   the frames following tics T through T+139 (140 tics) and not after tic T+140, unless
   something restarts it.
5. **The `plr->message = 0` write.** HU_Ticker writes a field of the player struct.
   (a) **Every reader of `player->message`**: a search of all `.c`/`.h` files for
   `message` finds exactly one reader, HU_Ticker (hu_stuff.c:522-526). No `p_*.c`, `g_game.c`
   or `st_stuff.c` code reads it. Other accesses are writes: the assignments in 1.5.5, the
   clears in `P_SpawnPlayer` (`p->message = NULL`, p_mobj.c:678) and
   `P_UnArchivePlayers` (`players[i].message = NULL`, p_saveg.c:96), and the whole-struct
   `memset` in `G_PlayerReborn` (g_game.c:815). The savegame writer does copy it:
   `P_ArchivePlayers` `memcpy`s the whole `player_t` (p_saveg.c:59-60), so the raw pointer
   value is written into the save file and discarded on load.
   (b) **What this means for a per-tic state dump.** Within one `G_Ticker` call the
   message is assigned (by `P_Ticker`, or earlier in `G_Ticker` by the turbo check or
   `G_DoSaveGame`) and then, when the condition in step 2 holds, cleared by HU_Ticker in
   the same call. A dump taken after `G_Ticker` therefore sees `NULL` whenever HU_Ticker
   took the message. It sees a non-NULL pointer that survives across tics in exactly two
   situations: `showMessages == 0` and `message_dontfuckwithme == 0` (step 2 skipped
   entirely), or `message_nottobefuckedwith` true and `message_dontfuckwithme` false (a
   protected message is showing). A message left pending this way is taken later: in the
   second case on the tic the protection times out, up to 139 tics after it was assigned,
   with whatever pointer is in the field by then (later assignments overwrite earlier
   ones). In the first case it is never taken until something sets
   `message_dontfuckwithme` (the messages toggle) or the player is respawned (p_mobj.c:678).
   The existing C reference stub (tools/reference/m5a_stubs.c:112-120) records the pointer
   and clears it unconditionally; that equals vanilla only while `showMessages != 0` and no
   protected message is showing, which holds in the event-free runners (no menu toggle,
   no chat) with the default configuration.

#### 1.5.4 Repeated and overlapping messages

1. **The same message on consecutive tics, or the same pickup twice.** HU_Ticker never
   compares the new pointer or text with the old one. Each time the condition holds it
   calls `HUlib_addMessageToSText`, which calls `HUlib_addLineToSText` (hu_lib.c:200-214):
   `if (++s->cl == s->h) s->cl = 0;` (with `h == 1`, `cl` stays 0), clears `l[0]`
   (`len = 0`, `l[0] = 0`, `needsupdate = true`, hu_lib.c:46-51), then sets every line's
   `needsupdate = 4` (hu_lib.c:211-212), and the text is re-appended (hu_lib.c:227-228).
   The counter restarts at 140 and `message_on` stays true. The drawn text is identical,
   so the only visible effect is that the timeout restarts (*inference*: the pixels are
   the same, since the line is redrawn every frame from the same characters).
2. **A different message while one is showing** (not protected): the line is cleared and
   replaced by the new text on that tic, and the counter restarts. There is no queue and
   no scrolling with `HU_MSGHEIGHT == 1`.
3. **Two assignments in one tic**: only the last pointer stored before HU_Ticker runs is
   shown; earlier ones are lost (*inference*: the field is a single pointer, overwritten).
   Within `P_Ticker`, `P_PlayerThink` (use lines, so locked-door messages) runs before
   `P_RunThinkers` (movement, so pickups) (p_tick.c:148-152), so a pickup in the same tic
   overwrites a door message (*inference* from that order).
4. **A protected message showing** (`message_nottobefuckedwith` true, only after the
   messages toggle or net chat): ordinary messages are not taken and stay in
   `plr->message` (1.5.3.5); a new toggle message (`message_dontfuckwithme` true) is taken
   immediately through the second clause of the condition.
5. **Toggle and pickup in the same tic** (*inference* from the order of events and
   tickers): `M_ChangeMessages` runs during event processing before the tic, setting
   `plr->message = MSGON/MSGOFF` and `message_dontfuckwithme = true`; a pickup in that
   tic's `P_Ticker` then overwrites the pointer, and HU_Ticker shows the pickup text with
   the toggle's privileges (shown even if messages are now off, and protected for 140 tics).

#### 1.5.5 Every assignment to player->message

All strings are from d_englsh.h (included by dstrings.h:33-37 unless `FRENCH` is defined;
the linuxdoom build does not define it). "SP-E1" = can occur in a single-player
Episode 1 level with no menu, cheats, save, automap, chat or command-line options;
whether a given map contains the thing or line special is a WAD fact this section does
not establish.

Pickups, `P_TouchSpecialThing` (p_inter.c:338-650), assigned to the touching player.
Every case that can refuse (`P_GiveArmor`, `P_GiveBody`, `P_GivePower`, `P_GiveAmmo`,
`P_GiveWeapon` returning false) returns before the assignment, so a refused pickup
assigns nothing.

| Line | Macro | Text (d_englsh.h line) | Trigger | SP-E1 |
| --- | --- | --- | --- | --- |
| p_inter.c:373 | GOTARMOR | "Picked up the armor." (80) | green armor accepted | yes |
| p_inter.c:379 | GOTMEGA | "Picked up the MegaArmor!" (81) | blue armor accepted | yes |
| p_inter.c:388 | GOTHTHBONUS | "Picked up a health bonus." (82) | health bonus (always) | yes |
| p_inter.c:397 | GOTARMBONUS | "Picked up an armor bonus." (83) | armor bonus (always) | yes |
| p_inter.c:405 | GOTSUPER | "Supercharge!" (87) | soul sphere | yes |
| p_inter.c:415 | GOTMSPHERE | "MegaSphere!" (102) | megasphere; returns first unless `gamemode == commercial` (409-410) | no |
| p_inter.c:423 | GOTBLUECARD | "Picked up a blue keycard." (89) | only if the card was not already held (422) | yes |
| p_inter.c:431 | GOTYELWCARD | "Picked up a yellow keycard." (90) | same rule (430) | yes |
| p_inter.c:439 | GOTREDCARD | "Picked up a red keycard." (91) | same rule (438) | yes |
| p_inter.c:447 | GOTBLUESKUL | "Picked up a blue skull key." (92) | same rule (446) | yes |
| p_inter.c:455 | GOTYELWSKUL | "Picked up a yellow skull key." (93) | same rule (454) | yes |
| p_inter.c:463 | GOTREDSKULL | "Picked up a red skull key." (94) | same rule (462) | yes |
| p_inter.c:473 | GOTSTIM | "Picked up a stimpack." (84) | stimpack accepted | yes |
| p_inter.c:481 | GOTMEDINEED | "Picked up a medikit that you REALLY need!" (85) | medikit with `player->health < 25` tested after `P_GiveBody` added 25 (476-481) | see note |
| p_inter.c:483 | GOTMEDIKIT | "Picked up a medikit." (86) | medikit accepted otherwise | yes |
| p_inter.c:491 | GOTINVUL | "Invulnerability!" (96) | invulnerability | yes |
| p_inter.c:498 | GOTBERSERK | "Berserk!" (97) | berserk | yes |
| p_inter.c:507 | GOTINVIS | "Partial Invisibility" (98) | invisibility | yes |
| p_inter.c:514 | GOTSUIT | "Radiation Shielding Suit" (99) | radiation suit | yes |
| p_inter.c:521 | GOTMAP | "Computer Area Map" (100) | computer map | yes |
| p_inter.c:528 | GOTVISOR | "Light Amplification Visor" (101) | light amp visor | yes |
| p_inter.c:544 | GOTCLIP | "Picked up a clip." (104) | clip accepted (dropped or placed) | yes |
| p_inter.c:550 | GOTCLIPBOX | "Picked up a box of bullets." (105) | | yes |
| p_inter.c:556 | GOTROCKET | "Picked up a rocket." (106) | | yes |
| p_inter.c:562 | GOTROCKBOX | "Picked up a box of rockets." (107) | | yes |
| p_inter.c:568 | GOTCELL | "Picked up an energy cell." (108) | | yes |
| p_inter.c:574 | GOTCELLBOX | "Picked up an energy cell pack." (109) | | yes |
| p_inter.c:580 | GOTSHELLS | "Picked up 4 shotgun shells." (110) | | yes |
| p_inter.c:586 | GOTSHELLBOX | "Picked up a box of shotgun shells." (111) | | yes |
| p_inter.c:598 | GOTBACKPACK | "Picked up a backpack full of ammo!" (112) | backpack (always) | yes |
| p_inter.c:605 | GOTBFG9000 | "You got the BFG9000!  Oh, yes." (114) | BFG accepted | yes |
| p_inter.c:612 | GOTCHAINGUN | "You got the chaingun!" (115) | | yes |
| p_inter.c:619 | GOTCHAINSAW | "A chainsaw!  Find some meat!" (116) | | yes |
| p_inter.c:626 | GOTLAUNCHER | "You got the rocket launcher!" (117) | | yes |
| p_inter.c:633 | GOTPLASMA | "You got the plasma gun!" (118) | | yes |
| p_inter.c:640 | GOTSHOTGUN | "You got the shotgun!" (119) | | yes |
| p_inter.c:647 | GOTSHOTGUN2 | "You got the super shotgun!" (120) | | yes (code has no gamemode check; the thing is a WAD fact) |

Note on GOTMEDINEED (*inference*): `P_GiveBody` refuses at `health >= 100` and otherwise
adds 25 (p_inter.c:233-239), and a living toucher has `health >= 1` because
`P_DamageMobj` subtracts the same damage from player and mobj and clamps only the player
at 0 (p_inter.c:870-887; `P_TouchSpecialThing` returns for `toucher->health <= 0`,
p_inter.c:362-363). So the test `health < 25` at p_inter.c:480 never holds and GOTMEDINEED
is unreachable in vanilla.

Locked doors and switches, assigned to the activating player (monsters have no player
and return first):

| Line | Macro | Text (d_englsh.h) | Trigger | SP-E1 |
| --- | --- | --- | --- | --- |
| p_doors.c:378 | PD_BLUEK | "You need a blue key to open this door" (128) | `EV_VerticalDoor`, specials 26 and 32, no blue card or skull (p_doors.c:371-382) | yes |
| p_doors.c:392 | PD_YELLOWK | "You need a yellow key to open this door" (130) | specials 27 and 34 (384-396) | yes |
| p_doors.c:405 | PD_REDK | "You need a red key to open this door" (129) | specials 28 and 33 (398-409) | yes |
| p_doors.c:227 | PD_BLUEO | "You need a blue key to activate this object" (125) | `EV_DoLockedDoor`, specials 99 and 133 (p_doors.c:221-231), called from `P_UseSpecialLine` (p_switch.c:497-505, 628-636) | code path yes; line special is a WAD fact |
| p_doors.c:239 | PD_REDO | "You need a red key to activate this object" (126) | specials 134 and 135 (233-243) | as above |
| p_doors.c:252 | PD_YELLOWO | "You need a yellow key to activate this object" (127) | specials 136 and 137 (245-256) | as above |

`EV_VerticalDoor` is reached from `P_UseSpecialLine` (p_switch.c:325-338). p_spec.c
contains no `message` assignment.

Engine, menu and other:

| Line | Value | Trigger | SP-E1 |
| --- | --- | --- | --- |
| g_game.c:675-678 | `static char turbomessage[80]`; `sprintf(turbomessage, "%s is turbo!", player_names[i])` -> for player 0 `"Green:  is turbo!"` (17 characters, two spaces, from `HUSTR_PLRGREEN` = `"Green: "`, d_englsh.h:312, hu_stuff.c:79-85) | in `G_Ticker`, for any player with `cmd->forwardmove > 0x32` on a tic with `!(gametic&31) && ((gametic>>5)&3) == i`; for player 0 that is `gametic % 128 == 0`. Assigned to the console player. | Only with `-turbo` above 100 % or a demo/ticcmd carrying forwardmove > 50: `G_BuildTiccmd` clamps to `forwardmove[1]` = 0x32 (g_game.c:175, 413-416), which `-turbo` scales (d_main.c:885-899). Not with default controls. |
| g_game.c:1316 | GGSAVED "game saved." (d_englsh.h:135) | end of `G_DoSaveGame` | no (save) |
| m_menu.c:985 / 987 | MSGOFF "Messages OFF" / MSGON "Messages ON" (d_englsh.h:61-62), plus `message_dontfuckwithme = true` (989) | `M_ChangeMessages`: options menu item or the **F8** key, handled by `M_Responder` when the menu is not active (m_menu.c:1582-1585) | F8 works in a level with no menu open; whether F8 is in M5b scope is an owner question |
| m_menu.c:1601 | `gammamsg[usegamma]` (m_menu.c:109-116, `char[5][26]`, GAMMALVL0..4, d_englsh.h:70-74) | **F11** key, menu not active (m_menu.c:1597-1603) | same as F8 |
| m_menu.c:1144 / 1146 | DETAILHI / DETAILLO | inside a comment block after `return` in `M_ChangeDetail` | never |
| m_misc.c:531 | `"screen shot"` (literal, lower case) | `M_ScreenShot`, reached via `ga_screenshot` (g_game.c:645-647) from `G_ScreenShot` (g_game.c:972), called only with `devparm` and F1 (m_menu.c:1511-1515) | no |
| st_stuff.c:558, 561 | STSTR_DQDON / STSTR_DQDOFF | iddqd cheat, `!netgame` | no (cheats) |
| st_stuff.c:575, 592 | STSTR_FAADDED / STSTR_KFAADDED | idfa / idkfa | no |
| st_stuff.c:601, 609, 618 | STSTR_MUS / STSTR_NOMUS | idmus | no |
| st_stuff.c:631, 633 | STSTR_NCON / STSTR_NCOFF | idspispopd / idclip | no |
| st_stuff.c:647, 654 | STSTR_BEHOLDX / STSTR_BEHOLD | idbehold | no |
| st_stuff.c:661 | STSTR_CHOPPERS "... doesn't suck - GM" | idchoppers | no |
| st_stuff.c:671 | `static char buf[ST_MSGWIDTH]` (52, st_stuff.c:247), `"ang=0x%x;x,y=(0x%x,0x%x)"` | idmypos | no |
| st_stuff.c:720 | STSTR_CLEV | idclev | no |
| am_map.c:682, 686, 690, 695 | AMSTR_FOLLOWON/OFF, GRIDON/OFF, a marked-spot buffer, AMSTR_MARKSCLEARED | automap keys | no (automap) |
| hu_stuff.c:591 | HUSTR_MSGU "[Message unsent]" | chat queue full | no (netgame chat) |
| hu_stuff.c:688-696 | HUSTR_TALKTOSELF1..5 | chat to oneself, netgame with more than 2 players | no |
| hu_stuff.c:725, 749 | `lastmessage` | chat macro or chat line sent | no |
| d_net.c:291 | `exitmsg` (`char[80]`, d_net.c:259), "Player N left the game" | net exit packet | no |

Character check (*computed* from the d_englsh.h strings above with a short script, no
IWAD): after `toupper`, every character of every string in the tables lies in `'!'..'_'`
or is a space; no reachable string contains a character outside the font range. All
contain lower-case letters (shown upper-case, 1.5.6.3) except STSTR_NOMUS. The longest
reachable SP-E1 strings are PD_YELLOWO (45 characters) and GOTMEDINEED (41, unreachable);
the longest pickup that can occur is GOTSHELLBOX and GOTBACKPACK (34). None approaches
the 80-character line limit.

#### 1.5.6 HU_Drawer, HU_Erase and where the pixels go

1. **HU_Drawer** (hu_stuff.c:486-494): `HUlib_drawSText(&w_message);`
   `HUlib_drawIText(&w_chat);` and, only if `automapactive`,
   `HUlib_drawTextLine(&w_title, false)`. It is called from `D_Display` when
   `gamestate == GS_LEVEL && gametic` (d_main.c:270-271), after `ST_Drawer` (d_main.c:246)
   and after `R_RenderPlayerView` (d_main.c:267-268), and before `M_Drawer`
   (d_main.c:315). It draws into `screens[0]` (`FG`).
2. **HUlib_drawSText** (hu_lib.c:231-252): `if (!*s->on) return;` (236-237), then for
   `i = 0 .. h-1`, `idx = cl - i`, wrapped by `+= h` if negative, and
   `HUlib_drawTextLine(&s->l[idx], false)` (240-250). With `h == 1` it draws `l[0]` only.
   Each line's own `y` was fixed at init: `y - i*(SHORT(font[0]->height)+1)`
   (hu_lib.c:195), so line 0 is at y = 0. (With more lines, older lines would be drawn
   higher, at negative y; not reachable with `HU_MSGHEIGHT == 1`.)
   `HUlib_drawIText` (hu_lib.c:336-345) returns at once unless `*it->on` (`chat_on`).
3. **HUlib_drawTextLine** (hu_lib.c:99-139), in full:
   ```c
   x = l->x;
   for (i=0;i<l->len;i++)
   {
       c = toupper(l->l[i]);                 /* c is unsigned char */
       if (c != ' '
           && c >= l->sc
           && c <= '_')
       {
           w = SHORT(l->f[c - l->sc]->width);
           if (x+w > SCREENWIDTH)
               break;
           V_DrawPatchDirect(x, l->y, FG, l->f[c - l->sc]);
           x += w;
       }
       else
       {
           x += 4;
           if (x >= SCREENWIDTH)
               break;
       }
   }
   if (drawcursor
       && x + SHORT(l->f['_' - l->sc]->width) <= SCREENWIDTH)
       V_DrawPatchDirect(x, l->y, FG, l->f['_' - l->sc]);
   ```
   Points that matter for a port: the upper bound is the literal `'_'`, not `HU_FONTEND`;
   lower-case letters are drawn with the upper-case glyphs; every character outside
   `'!'..'_'` after `toupper` (space, control characters, `` ` ``, `{|}~`, DEL, bytes 128 and
   above) advances 4 pixels and draws nothing; the advance for a glyph is the patch
   `width` field, not adjusted for `leftoffset`; the first glyph that would end past
   column 320 (by `x + width`, offsets ignored) ends the whole line, even if later
   characters are narrower or spaces; a space that brings `x` to 320 or more also ends it.
   The message line never draws the cursor (`false`, hu_lib.c:249).
4. **HU_Erase** (hu_stuff.c:496-503): `HUlib_eraseSText(&w_message)`,
   `HUlib_eraseIText(&w_chat)`, `HUlib_eraseTextLine(&w_title)`. It is called from
   `D_Display` before any drawing, when `gamestate == GS_LEVEL && gametic`
   (d_main.c:231-232).
5. **HUlib_eraseSText** (hu_lib.c:254-267): for each line, `if (s->laston && !*s->on)
   s->l[i].needsupdate = 4;` then `HUlib_eraseTextLine(&s->l[i])`; finally
   `s->laston = *s->on`. `HUlib_eraseIText` (hu_lib.c:347-353) does the same for one line.
6. **HUlib_eraseTextLine** (hu_lib.c:143-174): erases only when
   `!automapactive && viewwindowx && l->needsupdate` (hu_lib.c:154-155); then for each of
   the `SHORT(l->f[0]->height) + 1` rows starting at `l->y` it copies from the background
   screen `screens[1]` with `R_VideoErase` (r_draw.c:818-828, a `memcpy` from `screens[1]`
   to `screens[0]`): the whole row if the row is outside the view window, otherwise only
   the left and right borders (hu_lib.c:158-168). Then, unconditionally,
   `lastautomapactive = automapactive;` and `if (l->needsupdate) l->needsupdate--;`
   (hu_lib.c:171-172). (`#define noterased viewwindowx`, hu_lib.c:38, is unused.)
7. **Screenblocks 10 and 11.** `R_ExecuteSetViewSize` sets `scaledviewwidth = 320` for
   both (`setblocks*32` = 320 at 10; `SCREENWIDTH` at 11), `viewheight` = 168 at 10
   (`(10*168/10)&~7`) and 200 at 11 (r_main.c:682-691). `R_InitBuffer` sets
   `viewwindowx = (SCREENWIDTH-width)>>1` = 0 and, because the width is 320,
   `viewwindowy = 0` (r_draw.c:705-715). So `viewwindowx == 0` and the erase condition is
   false: **HU_Erase changes no pixels at screenblocks 10 and 11**. It still decrements
   `needsupdate` and updates `laston`, which have no visible effect at these sizes.
8. **Where the message is drawn.** Line 0 of `w_message` starts at x = 0, y = 0. Each
   glyph goes to `(x - leftoffset, 0 - topoffset)` with its own width and height
   (v_video.c:218-219). The view occupies rows 0-167 at screenblocks 10 and rows 0-199 at
   11, full width, so a glyph whose rows stay within 0-167 lies entirely over the 3D view
   at both settings (*inference*; the glyph heights are a WAD fact). The code itself does
   not keep glyphs out of the status bar area at screenblocks 10: a glyph reaching row 168
   or below would be drawn over the status bar after `ST_Drawer` (d_main.c:246 before 271).
9. **When a message times out** at screenblocks 10 and 11, nothing erases it (7). The
   next frame's `R_RenderPlayerView` (d_main.c:267-268) overwrites the view pixels it
   writes, and HU_Drawer no longer draws the line, so the message disappears wherever the
   view is drawn. D_Display has no clear of `screens[0]` at these sizes: the border
   refresh is skipped when `scaledviewwidth == 320` (d_main.c:285). So any view pixel that
   the renderer does not write in a frame (a hall-of-mirrors gap) keeps its previous
   contents, which may be message glyph pixels from an earlier frame (*inference* from the
   absence of any other writer; it relies on the renderer leaving HOM pixels untouched,
   which is the M5a renderer's subject). The same holds while a message is showing: glyph
   pixels are drawn over whatever the view left.

#### 1.5.7 HU_Responder in a single-player level

`HU_Responder` (hu_stuff.c:617-759) is called from `G_Responder` only when
`gamestate == GS_LEVEL`, before `ST_Responder` and `AM_Responder` (g_game.c:536-547).
`D_ProcessEvents` gives each event to `M_Responder` first (d_main.c:170-176). Before
reaching HU_Responder, `G_Responder` takes F12 (spy mode, g_game.c:507-518) and, during
demo playback or the demo screen, turns any key into the menu (g_game.c:521-533). With
the menu closed, `M_Responder` returns false for Enter (m_menu.c:1607-1616), so Enter
reaches HU_Responder.

1. **Shift and alt tracking** (hu_stuff.c:643-652): an event with `data1 == KEY_RSHIFT`
   (0x80+0x36, doomdef.h:276) sets `shiftdown = (ev->type == ev_keydown)`; `KEY_RALT` or
   `KEY_LALT` (both 0x80+0x38, doomdef.h:278-280) set `altdown` the same way; both return
   false (not eaten). These run for every event type and are used only by the chat
   branch. `numplayers` is counted (639-641) but used only under `netgame`.
2. Non-keydown events then return false (654-655).
3. **With `chat_on` false** (always so in single player, because `chat_on` becomes true
   only at hu_stuff.c:667 and 679, both under `netgame`):
   - `data1 == HU_MSGREFRESH` (Enter, 13): `message_on = true;
     message_counter = HU_MSGTIMEOUT; eatkey = true;` (659-664). It does not consult
     `showMessages` and does not change the text: it redisplays whatever is in `l[0]`,
     the last message taken since HU_Start (empty after HU_Start, in which case nothing
     visible is drawn). The key is eaten, so `G_Responder` returns true (g_game.c:544-545)
     and Enter never reaches `gamekeydown`; key-up events for Enter are not eaten.
     (*Inference*: because the refresh sets the counter between tics and the next
     HU_Ticker decrements first, a refresh shows the line for 139 tic-frames, one fewer
     than a newly taken message.)
   - `netgame && data1 == 't'` and `netgame && numplayers > 2` destination keys: never
     in single player.
4. **With `chat_on` true** (hu_stuff.c:702-755): unreachable in single player.
5. **What the app must do to offer the refresh key exactly**: deliver key events between
   tics in vanilla order (menu responder first, then the F12 and demo checks, then the
   HUD before the status bar and automap responders); on an Enter key-down in a level
   with the menu closed and no demo playing, set the render state's `message_on = true`
   and `message_counter = 140` before the next tic's HUD tick, and consume the key so it
   cannot act as a game binding. This is HUD state changed by input, not by the
   simulation, so it lives in the saveable render state and is not visible in the
   simulation's state.

Related: `G_BuildTiccmd` calls `HU_dequeueChatChar()` for every ticcmd
(g_game.c:328); in single player the queue is always empty, so `cmd->chatchar = 0`
(hu_stuff.c:600-615).

#### 1.5.8 hu_lib.c functions used

1. `HUlib_init` (hu_lib.c:42-44): empty, and never called.
2. `HUlib_clearTextLine` (hu_lib.c:46-51): `len = 0; l[0] = 0; needsupdate = true` (1).
3. `HUlib_initTextLine` (hu_lib.c:53-66): stores `x, y, f, sc`, then clears.
4. `HUlib_addCharToTextLine` (hu_lib.c:68-84): if `len == HU_MAXLINELENGTH` (80) returns
   false and drops the character; otherwise stores it, re-terminates, sets
   `needsupdate = 4`, returns true. Callers ignore the result, so longer strings are
   silently truncated to 80 characters. The buffer is `l[81]` (hu_lib.h:52), so the
   terminator always fits.
5. `HUlib_delCharFromTextLine` (hu_lib.c:86-97): if `len` is 0 returns false, else removes
   the last character and sets `needsupdate = 4`. Used only by chat.
6. `HUlib_initSText` (hu_lib.c:176-198): `h = h; on = on; laston = true; cl = 0;` and
   initializes lines `0 .. h-1` at `y - i*(height+1)`. Lines `h .. 3` are not touched.
7. `HUlib_addLineToSText` (hu_lib.c:200-214): advance `cl` modulo `h`, clear that line,
   set `needsupdate = 4` on all `h` lines.
8. `HUlib_addMessageToSText` (hu_lib.c:216-229): add a line, then append the `prefix`
   characters if `prefix` is non-NULL (HU_Ticker passes 0; net chat passes a player
   name), then the message characters, all subject to the 80-character limit.
9. Buffer sizes: text line 81 bytes; `HU_MAXLINES` 4 (only 1 used by `w_message`);
   `lastmessage` 81; chat queue 128; turbo buffer 80 (17 used); `gammamsg` rows 26.
   No reachable message exceeds the line limit (1.5.5).

#### 1.5.9 The patch-drawing path

1. `V_DrawPatchDirect` (v_video.c:337-341 and the commented body to 393) only calls
   `V_DrawPatch(x, y, scrn, patch)`. The `I_Error("Bad V_DrawPatchDirect")` at
   v_video.c:364 is inside the comment and never compiled.
2. `V_DrawPatch` (v_video.c:204-268): `y -= SHORT(patch->topoffset);
   x -= SHORT(patch->leftoffset);` then, under `RANGECHECK` (defined unconditionally,
   doomdef.h:75), if `x < 0 || x + width > SCREENWIDTH || y < 0 || y + height >
   SCREENHEIGHT || (unsigned)scrn > 4`, it prints two lines to stderr and **returns
   without drawing** (v_video.c:221-232; the comment says "No I_Error abort"). Otherwise it
   marks the dirty rectangle for screen 0 (`V_MarkRect`, no pixel effect) and copies each
   column's posts: for each post, `length` bytes from the post data to
   `desttop + topdelta*SCREENWIDTH` downward (v_video.c:237-267). Posts are not checked
   against the patch height.
3. **Bound enforced** (*inference* from 1.5.6.3 and 2): HUlib_drawTextLine keeps
   `x + width <= 320` with offsets ignored; V_DrawPatch then rejects, as a whole glyph,
   any glyph whose offset-adjusted rectangle leaves the 320x200 screen. No reachable
   message can make the HUD path call `I_Error`. With glyphs of arbitrary size the
   possible effects are: a glyph silently skipped (while `x` still advances by its
   width), the line ending early, glyph pixels over the status bar area at screenblocks
   10, or out-of-bounds writes from posts longer than the declared height (a malformed
   lump; the code does not prevent it).

#### 1.5.10 Undefined or layout-dependent behaviour and fatal errors

1. `toupper(l->l[i])` (hu_lib.c:114) passes a plain `char`; on a signed-char platform a
   byte 128-255 is a negative argument, which is undefined for `toupper`. No reachable
   message contains such a byte (1.5.5). The result is stored in an `unsigned char`.
2. The turbo message points at a function-local `static char[80]` (g_game.c:675) that is
   rewritten by every later trigger; HU_Ticker copies the text when it takes it, so only a
   pending (untaken) message could change under the pointer, and in single player the
   text is always the same (*inference*).
3. `P_ArchivePlayers` writes the raw `message` pointer into the save (p_saveg.c:59-60), so
   save bytes depend on whether a message is pending and on the address layout. Out of
   scope (save), but it is a reason not to model the pointer value.
4. `if (french)` (hu_stuff.c:399, 730) tests the enumerator `french` of `Language_t`
   (doomdef.h:63-66), which is the constant 1, not the `language` variable. So
   `shiftxform` is always `french_shiftxform` and chat input always goes through
   `ForeignTranslation`. Chat only; no effect in single player.
5. `plr` is NULL until the first HU_Start; HU_Ticker dereferences it (hu_stuff.c:522).
   Only reachable if a level starts with no console-player start (1.5.1.4).
6. `HUlib_eraseTextLine` would read `l->f[0]` of a line whose font is NULL only if called
   on `w_inputbuffer`, which never happens.
7. Fatal errors on the HUD path: `I_Error("W_GetNumForName: %s not found!")` from HU_Init
   for a missing `STCFN033`..`STCFN095` (w_wad.c:406). None in HU_Start, HU_Ticker,
   HU_Drawer, HU_Erase or HU_Responder. `P_TouchSpecialThing`'s
   `I_Error("P_SpecialThing: Unknown gettable thing")` (p_inter.c:650) is simulation, not HUD.

#### 1.5.11 History the Rust HUD render state must carry

- `message_on`, `message_counter` (not reset at a new level), `message_nottobefuckedwith`
  and `message_dontfuckwithme`.
- The message line's text (up to 80 bytes) and length; `cl` (always 0 with one line).
- `showMessages` (from configuration, any integer, tested for nonzero).
- Whether a message is pending in `plr->message` and which text, for the two cases where
  HU_Ticker leaves it uncleared (1.5.3.5), if those cases are supported.
- For exactness at other screen sizes later (no pixel effect at 10 and 11): `needsupdate`
  for each line, `laston` for `w_message` and `w_chat`. `lastautomapactive` is dead.
- Not needed in single player without chat: `chat_on` (always false), `w_chat`,
  `w_inputbuffer`, `chat_dest`, the chat queue, `lastmessage`, `num_nobrainers`; `shiftdown`
  and `altdown` change but are read only by chat. The title text is needed only with
  the automap.

#### 1.5.12 Hazards for a pixel-exact port of the HUD

- The decrement-then-take order: 140 frames for a new message, 139 for a refresh, and a
  pending message taken on the very tic the previous one expires.
- Same-text messages restart the timeout; the C never deduplicates.
- Only the last assignment before HU_Ticker in a tic is shown.
- `toupper`, the literal `'_'` bound, 4-pixel spaces, width advance ignoring
  `leftoffset`, line end at the first glyph that would pass column 320, and silent
  whole-glyph skips by V_DrawPatch's range check.
- HUD drawn after the status bar and after the view; HU_Erase is a no-op at
  screenblocks 10 and 11, so old glyph pixels survive in view pixels the renderer does
  not write (HOM).
- A per-tic simulation dump never shows a taken message; the HUD needs the per-tic
  "message assigned" value from the simulation before it is cleared, or the simulation
  must not clear it when the HUD would not (showMessages 0, protected message).
- The Enter key is eaten by the HUD in a level; turbo text has two spaces after "Green:".

#### 1.5.13 Questions the HUD facts raise for the owner

1. **Who clears `player->message`.** The simulation is not to change, but vanilla's clear
   is conditional on HUD state (`showMessages`, protection). Options: (a) support only
   `showMessages != 0` with no protected messages, where an unconditional clear is exact
   (the existing C stub's behaviour, tools/reference/m5a_stubs.c:115-120); (b) let the
   render state decide and write back the clear, which makes HUD state influence a
   simulation field; (c) keep the pending message in the render state instead of the
   player struct. Facts: no game logic reads the field (1.5.3.5); the save writes it.
2. **F8 and F11 hotkeys.** They work in a level with the menu closed (m_menu.c:1582-1603)
   and produce messages (F8 also sets `message_dontfuckwithme` and toggles `showMessages`).
   Options: include them as HUD/app input in M5b, or treat them as menu scope and leave
   them out. F11 also changes the palette.
3. **The Enter refresh key.** Options: implement it as render-state input (1.5.7.5), or
   leave it out and refuse Enter-dependent comparisons. Facts: it changes only HUD state
   and eats the key.
4. **`showMessages` source.** Options: fixed at 1 (the default, m_misc.c:239), or read from
   a configuration with vanilla's any-integer semantics.
5. **Turbo message.** Reachable only with `-turbo` above 100 % or crafted ticcmds/demos.
   Options: support (it is computed in `G_Ticker`, simulation side) or declare out of scope.
6. **Glyph-shape hazards** (glyphs taller than 168 rows over the status bar, offsets that
   trigger V_DrawPatch skips, malformed posts). Options: reproduce exactly, or refuse a
   font that would trigger them, as a refuse-the-tic witness.

Each question is taken up in §4: 1 in S16 and S17; 2 in S5; 3 in S5; 4 in S5; 5 in S17; 6 in S17.

### 1.6 What the Rust state already carries, and what is missing

This section lists every input that the original status bar (st_stuff.c, st_lib.c), the heads-up
text (hu_stuff.c, hu_lib.c), the palette choice (`ST_doPaletteStuff`) and the input and recording
path read, and says where the wire-24 Rust simulation (`PlayerMonstersEpisode1V2`) holds it. The
classes are the ones the brief defines:

- **(a)** readable now through the public `&self` API of `Simulation` or through
  `Simulation::snapshot()`;
- **(b)** not stored, but exactly derivable by a presentation-side observer that sees every
  committed tic and its events from level start;
- **(c)** missing and not derivable: a stop-and-ask candidate.

A compile-only probe (`~/scratch/m5b/probe-rust/src/main.rs`) names every class (a) read below. It
builds cleanly against 9a598b8, so every one of them is public today. When the C reads happen
matters. `G_Ticker` runs `P_Ticker`, then `ST_Ticker`, `AM_Ticker` and `HU_Ticker`
(linuxdoom-1.10/g_game.c:730-733), so both tickers read the player **after** the tic. The
drawers run once per displayed frame in `D_Display` (linuxdoom-1.10/d_main.c:233-264). A Rust
observer that reads the simulation right after a committed `Simulation::advance` therefore sees
exactly what the C tickers see.

#### Summary table

| # | Input | C reader | Rust location (wire 24) | Class |
| --- | --- | --- | --- | --- |
| 1 | `health` | face (st_stuff.c:735, 764, 807, 994), `w_health` (st_stuff.c:1304) | `OriginalPlayer::health` via `Simulation::original_player` (crates/doom-sim/src/original_player.rs:76-90, 256-260) | a |
| 1 | `armorpoints` | `w_armor` (st_stuff.c:1348) | `PlayerArmorState::points` via `original_player_armor` (crates/doom-sim/src/player_armor.rs:48-53, 200-212) | a |
| 1 | `armortype` | not read by ST or HU | `PlayerArmorState::kind` | a |
| 1 | `ammo[4]` | `w_ready`, `w_ammo` (st_stuff.c:935, 1292, 1378-1402) | `PlayerWeaponState::{bullets, shells, cells, rockets}` via `original_player_weapons` (crates/doom-sim/src/player_weapons.rs:10-39, 321-331) | a |
| 1 | `maxammo[4]` | `w_maxammo` (st_stuff.c:1411-1435) | snapshot `original_player.0.weapons.max_ammo.0..3`; also exactly base `<< backpack` (STATE_FORMAT DSIM39) | a (snapshot) / b |
| 1 | `weaponowned[]` | face grin (st_stuff.c:781-784), `w_arms` (st_stuff.c:1322), `ST_initData` (st_stuff.c:1271) | `PlayerWeaponState::owned`, bit i = `weapontype_t` i | a |
| 1 | `readyweapon` | `ST_updateWidgets` (st_stuff.c:932-945) | `PlayerWeaponState::ready` | a |
| 1 | `pendingweapon` | not read by ST or HU (only in commented-out code) | `PlayerWeaponState::pending` | a (unused) |
| 1 | `cards[6]` | `keyboxes` (st_stuff.c:955-957) | `PlayerPowers::cards` via `original_player_powers` (crates/doom-sim/src/original_player.rs:109-114; crates/doom-sim/src/episode1.rs:255-262) | a |
| 1 | `backpack` | not read by ST (only through `maxammo`) | `PlayerWeaponState::backpack` | a |
| 2 | `damagecount` | face (st_stuff.c:800, 858), palette (st_stuff.c:1008) | `OriginalPlayer::damage_count` | a |
| 2 | `bonuscount` | face (st_stuff.c:774), palette (st_stuff.c:1029-1031) | `PlayerPickupState::bonus_count` via `original_player_pickups` (crates/doom-sim/src/world_pickups.rs:399-406, 1190-1198) | a |
| 2 | `powers[pw_strength]` | palette (st_stuff.c:1010-1013) | `PlayerPowers::powers[1]`; always 0 under wire 24 (berserk not admitted) | a (constant 0) |
| 2 | `powers[pw_ironfeet]` | palette (st_stuff.c:1039-1040) | `PlayerPowers::powers[3]` | a |
| 2 | `powers[pw_invulnerability]` | god face (st_stuff.c:900-901) | `PlayerPowers::powers[0]`; always 0 (2022 not admitted) | a (constant 0) |
| 2 | `powers[pw_infrared]` | not read by ST or HU (renderer only) | `PlayerPowers::powers[5]` | a |
| 2 | `cheats & CF_GODMODE` | god face (st_stuff.c:900) | not modelled; no cheat input path exists, so it is 0 | b (constant 0) |
| 3 | `attacker` (identity) | face (st_stuff.c:801-802) | `OriginalPlayer::attacker: Option<ActorId>` | a |
| 3 | `attacker->x, ->y` | face (st_stuff.c:814-817) | `Simulation::actor(id)` → `Actor::position()` after the tic | a |
| 3 | `mo->x, ->y, ->angle` | face (st_stuff.c:814-828) | `Simulation::actor(mo)` → `position()`, `angle()` | a |
| 4 | `attackdown` | rampage face (st_stuff.c:880) | `PlayerWeaponState::attack_down` | a |
| 4 | `playerstate` | HU does not read it; the face reads `!health` | `OriginalPlayer::state` (`PlayerBodyState`, crates/doom-sim/src/original_player.rs:68-73) | a |
| 4 | `frags[]` | `st_fragscount` (st_stuff.c:977-979; drawn only in deathmatch) | `OriginalPlayer::frags` (slot 0 only) | a (unused) |
| 5 | `message` (assigned this tic, and which) | `HU_Ticker` (hu_stuff.c:522-526) | stored code `PlayerPickupState::message` (last assigned, never cleared) plus the tic's events and the fed command | b (every kind; see item 5) |
| 6 | level exit and secret exit | (G_Ticker / G_DoCompleted; app only) | `MoverEvent::Exit { secret }` in `events()`; snapshot `level.exit`, `level.secret_exit`; `SimError::LevelExited` on the next tic | a |
| 6 | death, reborn request | (app only) | `OriginalPlayer::state`; `PlayerBodyEvent::Died`; `MoverEvent::Reborn`; `SimError::PlayerReborn` on the next tic | a |
| — | `M_Random` index (`rndindex`) for `st_randomnumber` | `ST_Ticker` (st_stuff.c:992) | not in the simulation: the sim never draws `M_Random`. The C also draws it in `S_StartSoundAtVolume` (s_sound.c:330, 340) and in the level-start wipe (f_wipe.c:159-162) | **c as vanilla; b under a declared sound-free model** (RD1) |
| — | `weaponinfo[].ammo` | `ST_updateWidgets` (st_stuff.c:932) | static source table (fist and chainsaw `am_noammo`) | b (static) |
| — | `deathmatch`, `netgame`, `consoleplayer`, `automapactive` | ST and HU throughout | constants 0, 0, 0 and false in this single-player app | b (static) |
| 7 | the live command domain | `advance` validation | crates/doom-sim/src/player_commands.rs:90-180; crates/doom-sim/src/player_weapons.rs:260-269 | see item 7 |

The detail follows, item by item.

#### Item 1. Health, armour, ammunition, weapons, keys and backpack: class (a)

- `health` is `OriginalPlayer::health` (crates/doom-sim/src/original_player.rs:79). Like the C's
  `player->health` it is clamped at zero, independently of the mobj's signed health
  (doc comment, :78). `ST_calcPainOffset` caps it at 100 (st_stuff.c:735). The dead face tests
  `!plyr->health` (st_stuff.c:764).
- The armour is `PlayerArmorState { kind, points }` (crates/doom-sim/src/player_armor.rs:48-53).
  The status bar draws only `armorpoints` (st_stuff.c:1348).
- Ammunition and weapons come from `PlayerWeaponState` (crates/doom-sim/src/player_weapons.rs:10-39):
  `ready`, `pending`, the `owned` bitmask (bit i is `weapontype_t` i; the DSIM39 reader admits
  bits 0-4 and 7), the four counts, `backpack` and `attack_down`. `Weapon` ordinals are the
  source `weapontype_t` values (crates/doom-sim/src/weapons.rs:18-37), so `owned` maps one to one
  onto `weaponowned[NUMWEAPONS]`. The arms widgets read `weaponowned[1..=6]`
  (st_stuff.c:1322). The grin compares all nine entries with `oldweaponsowned`
  (st_stuff.c:779-786).
- `maxammo` is not a field of `PlayerWeaponState`. It is canonical in the snapshot
  (`original_player.0.weapons.max_ammo.0..3`, STATE_FORMAT DSIM39). The v39 reader also enforces
  that it equals the base capacity (200, 50, 300, 50) shifted left by `backpack`. Deriving it
  from `backpack` is exact and avoids a full snapshot per tic.
- `cards` is `PlayerPowers::cards` (crates/doom-sim/src/original_player.rs:109-114). Skull keys
  are always false under wire 24 (the DSIM39 reader enforces `cards.3..5 == 0`).
- At level start, `ST_initData` copies `weaponowned` into `oldweaponsowned`
  (st_stuff.c:1270-1271). It runs inside `ST_Start`, which `P_SpawnPlayer` calls after
  `P_SetupPsprites` (p_mobj.c:686-698), when the player owns the fist and pistol. In Rust the
  same values can be read after `initialize_player_weapons` with `WeaponInventory::default()`
  (the setup of crates/doom-sim/examples/player_level_run.rs:224-226), so the start value is
  class (a).

All of these are part of the DSIM40 capture that is compared tic for tic with the genuine engine
(the `original_player.0.*` group; COMPATIBILITY wire 23 "Comparison contract"), so their per-tic
values carry genuine-engine evidence.

#### Item 2. `damagecount`, `bonuscount`, powers and cheats

- `damagecount` is `OriginalPlayer::damage_count` (crates/doom-sim/src/original_player.rs:85-87).
  Its decay runs at the source's places, `P_PlayerThink` and `P_DeathThink` (decision G4;
  crates/doom-sim/src/level.rs:1-3, 56-59), and the post-tic value is canonical and compared
  with the genuine dump (`original_player.0.damage_count`). That is exactly the value
  `ST_Ticker` and `ST_doPaletteStuff` read after `P_Ticker`: **class (a)**.
- `bonuscount` is `PlayerPickupState::bonus_count` (crates/doom-sim/src/world_pickups.rs:400-402).
  It decays before the thinkers (`bonus_tick`, crates/doom-sim/src/demon_expanded_map.rs:944) and
  is canonical and compared (`original_player.0.pickups.bonus_count`): **class (a)**.
- The powers are `PlayerPowers::powers` in `powertype_t` order (crates/doom-sim/src/original_player.rs:103-112).
  They count down in `powers_tick` (crates/doom-sim/src/demon_expanded_map.rs:943) and are canonical
  and compared: **class (a)**.
- **What wire 24 admits.** The DSIM39 reader, which DSIM40 inherits, enforces `powers.0 == 0`
  and `powers.1 == 0`: no invulnerability and no berserk in Episode 1 single player
  (STATE_FORMAT DSIM39 domains). COMPATIBILITY's wire-23 "Items and powers" row lists 2013, 2024,
  2025, 2026, 2045 and 8, but neither 2022 (invulnerability) nor 2023 (berserk). Wire 24 adds
  only 14 and 3003. "Every thing outside the lists above" is refused at load as
  `MapSpawnError::UnsupportedType` (COMPATIBILITY wire 23, "Explicitly not admitted"). The
  source metadata rows exist (`MT_INV` 2022 and `MT_MISC13` 2023,
  crates/doom-sim/src/source_actors.rs:1362-1363) but are not admitted. So under wire 24:
  - the berserk palette term (`bzc = 12 - (powers[pw_strength]>>6)`, st_stuff.c:1010-1016)
    always sees 0 and never changes the palette;
  - the god face (st_stuff.c:898-911) cannot appear through `pw_invulnerability`.

  Both should still be implemented as the C reads them. They can only be exercised by
  unit tests and by a bespoke C scenario, never by a wire-24 run.
- `cheats & CF_GODMODE` is not modelled (COMPATIBILITY wire 23: "the view height, colormaps,
  the map display and `cheats` are not modelled"). It can only be set by `ST_Responder`'s cheat
  code (st_stuff.c:551-561). That code also writes `health`, `armorpoints`, `weaponowned`,
  `ammo`, `cards` and `powers` (st_stuff.c:551-661) outside any ticcmd, which wire 24 cannot
  admit without a simulation change. With cheats left out of the input path, it is constant 0:
  **class (b)**. Design consequence: the app must not implement `ST_Responder`'s cheat path
  (risk 5 below).

#### Item 3. The attacker and the face's pain direction: class (a)

- **Identity.** `OriginalPlayer::attacker: Option<ActorId>` (crates/doom-sim/src/original_player.rs:83-84)
  is assigned exactly where `P_DamageMobj` assigns `player->attacker = source`
  (p_inter.c:874; Rust crates/doom-sim/src/combat.rs:1129). A null source stays `None`: floor
  damage and crushing pass a null source. The attacker is canonical
  (`original_player.0.attacker`) and compared.
- **Position after the tic.** `Simulation::actor(id)` returns any actor whose storage has not
  been retired, including corpses and actors marked for removal ("marked storage remains
  readable until retirement", crates/doom-sim/src/lib.rs:969-973). The generational handle never
  resolves to a reused slot (ARCHITECTURE "Actors, lifecycle and ordering"). `Actor::position()`
  and `Actor::angle()` read after `advance` are the post-`P_Ticker` values `ST_Ticker` reads.
- **The cases the brief names:**
  - *The attacker died.* A corpse stays in storage (no nightmare respawn under wire 24), so its
    position is readable.
  - *A fireball's source.* `P_DamageMobj` receives `source = missile->target`, the shooter, not
    the missile (p_inter.c:874). The attacker is the Imp or Baron, which is never removed
    under wire 24.
  - *The player itself* (rocket splash, `source == player->mo`). The C excludes it with
    `plyr->attacker != plyr->mo` (st_stuff.c:802), and Rust can compare the handle with
    `OriginalPlayer::mo`.
  - *Absent* (floor damage, crushing). `None` takes the C's "own damn stupidity" branch
    (st_stuff.c:855-873).
  - *A barrel.* The radius attack's source is the barrel's `target`, the thing that set it off,
    not the barrel. *Inference* from `A_Explode` passing `thingy->target`: I did not re-read
    p_enemy.c for this.
- **Precedent and guard.** The simulation reads the same pointer in `P_DeathThink`
  (p_user.c:200-205). Rust refuses the tic as `PlayerBodyError::Invariant` if that attacker no
  longer resolves to an actor that is not pending removal. Its comment says only a
  nightmare-respawned corpse could be one (crates/doom-sim/src/level.rs:22-41). The status bar
  should do the same: if `actor(attacker)` is `None`, raise a presentation refusal, never a
  guess. The C would read freed zone memory there. By the same argument this is inference: it
  should be unreachable under wire 24.
- **The angle.** `R_PointToAngle2(mo->x, mo->y, attacker->x, attacker->y)` (r_main.c:378-388) is
  `R_PointToAngle(x2, y2)` with `viewx/viewy = x1/y1`. doom-render already exports
  `r_main::point_to_angle(x, y, viewx, viewy)` (crates/doom-render/src/r_main.rs:89-90). The C
  call also overwrites the renderer globals `viewx/viewy`, which `R_SetupFrame` rewrites before
  the next frame. Harmless, and nothing to model.
- **`plyr->health - st_oldhealth > ST_MUCHPAIN`** (st_stuff.c:807, 860). This is the "ouch" face
  when health went **up** by more than 20 since the previous tic: the C writes the test this
  way, so the port must keep it, not fix it. `st_oldhealth` is status-bar state, set at the end
  of every `ST_Ticker` (st_stuff.c:994): class (b), held in the saved status-bar state.

#### Item 4. `attackdown`, `playerstate` and the dead face: class (a)

- `attackdown` is `PlayerWeaponState::attack_down` (crates/doom-sim/src/player_weapons.rs:29-30).
  It is canonical and compared (`attack_down` in the weapon group; the projection reads the
  dump's `attackdown`, tools/reference/level_snapshot.py:54). The rampage face's counter
  `lastattackdown` (st_stuff.c:757, 878-893) is status-bar state.
- The dead face keys on `!plyr->health` (st_stuff.c:764), and `health` is class (a). The player
  state (`OriginalPlayer::state`, `Live`/`Dead`) is also public, but no status-bar or HUD code
  reads `playerstate`.

#### Item 5. `plyr->message`: exactly derivable for every kind wire 24 can produce

**How the C behaves.** The game assigns a string pointer. `HU_Ticker` runs after `P_Ticker` in the
same `G_Ticker` (g_game.c:730-733). With `showMessages` on (the default), when the pointer is
non-null it adds the text to the one-line message widget, writes `plr->message = 0`, sets
`message_on` and restarts `message_counter` at `HU_MSGTIMEOUT` = 140 tics (hu_stuff.c:519-531;
hu_stuff.h:44). So what the HUD needs per tic is: was any message assigned during tic N, and if
so the **last** one (a later assignment in the same tic overwrites the pointer before
`HU_Ticker` reads it). Assigning the same string again restarts the 140-tic counter.

**How Rust stores it.** `PlayerPickupState::message: PickupMessage` is documented as "last assigned
source message, without presentation-side clearing" (crates/doom-sim/src/world_pickups.rs:404-405).
It persists across tics and is overwritten, never cleared. The canonical field is
`original_player.0.pickups.message` (crates/doom-sim/src/snapshot.rs:1231-1232), with codes
0-30 (`PickupMessage`, crates/doom-sim/src/world_pickups.rs:324-397). The independent projection
of the genuine dump collapses the C's per-tic pointer into the same "last assigned" value
(tools/reference/level_snapshot.py:702-708). So the stored code alone **cannot** tell "assigned
again this tic" from "unchanged". The events and the fed command can.

The Rust code assigns the field in exactly three places (a repository grep for `.message =` in
crates/doom-sim/src, tests excluded, finds no others):

| Kind | Codes | Where Rust assigns it | What the observer sees in tic N |
| --- | --- | --- | --- |
| Pickups | 1-6, 8-26: clip, bullet box, shells, shell box, stimpack, medikit, shotgun, chaingun, green armour, blue armour, health bonus, armour bonus, soulsphere, blue/yellow/red card, partial invisibility, radiation suit, computer map, light amplification, rocket, box of rockets, backpack, chainsaw, rocket launcher | crates/doom-sim/src/world_pickups.rs:1133-1135 | `Event::Pickup(PickupEvent::Message(code))`, pushed at every assignment in the same statement. A second identical pickup in a later tic emits it again. A key card already held assigns no message and emits none (:983-994). Code 7 (`GOTMEDINEED`) is never assigned in this positive-health domain (:419 refuses it) |
| Locked doors | 27 `PD_BLUEK`, 28 `PD_YELLOWK`, 29 `PD_REDK` | crates/doom-sim/src/movers.rs:529-546 (`EV_VerticalDoor`'s lock test, p_doors.c:369-410) | **No message event.** It is exactly recognisable as the adjacent pair `Event::ActorState(ActorStateEvent::Sound { actor: None, sound: sfx_oof })` followed immediately by `Event::Mover(MoverEvent::Call { function: VerticalDoor, argument: special, result: -1, created: 0 })` with `special` in {26, 32} → 27, {27, 34} → 28, {28, 33} → 29 (movers.rs:523-528, 535-544, 406-427). A monster at a locked door produces the `Call` without the `sfx_oof` (movers.rs:530-533). The only other `sfx_oof` request (the hard landing) names the player as its actor (crates/doom-sim/src/thinkers.rs:433-437) |
| Turbo | 30 `"%s is turbo!"` | crates/doom-sim/src/demon_expanded_map.rs:782-796, using `turbo_message` (crates/doom-sim/src/player_commands.rs:30-39; g_game.c:671-679) | **No event.** It is a pure function of the tic number before the tic (`Simulation::tic()`) and the command **as fed**: `forward > 50 && (tic & 31) == 0 && ((tic >> 5) & 3) == 0`. It is assigned before every other assignment of the tic, so any later pickup or door message in the same tic replaces it |

**Locked switches and tagged specials.** None exist under wire 24. In this source the key checks
for doors are in `EV_VerticalDoor` (lines 26-28 and 32-34), and wire 24 admits only those.
`EV_DoLockedDoor`'s specials (99, 133-137) do not appear in COMPATIBILITY's special tables.
Nothing else in wire 24 assigns `plyr->message`. In the C, the other writers are cheats
(st_stuff.c:558-720), chat (hu_stuff.c:591, 688-749) and the menu. The app leaves out all
three (item 7 and §3.4).

**The derivation.** For tic N, the last assignment is the last element of the list:

1. the turbo message, if `turbo_message(tic_before, fed_command)` holds;
2. then, in event order, every `PickupEvent::Message(code)` and every locked-door pair.

**The cross-check, which turns the derivation into a witness.**

- If the list is non-empty, its last element must equal the stored code after the tic.
- If the list is empty, the stored code must be unchanged.
- Any mismatch means the derivation missed a writer. The observer then refuses (a presentation
  refusal), never shows a wrong message.

The only undetectable failure would be an unknown writer whose value equals the previous stored
code. The grep above found no such writer.

**The command as fed, not the captured command.** The turbo test must use the command the
app fed. The canonical `player.0.forward` holds the command after the chainsaw-lunge rewrite
(`forward = 100`; STATE_FORMAT DSIM39, crates/doom-sim/src/demon_expanded_map.rs:807-815), so it
would fire a turbo message the C does not show on a lunge tic that falls on a 128th gametic. The
live app never produces `forward > 50` (`G_BuildTiccmd` clamps to `MAXPLMOVE` = 50,
g_game.c:171, 413-416), so only a replayed demo reaches code 30. `gametic` is the level's tic
count because every session starts the level at gametic 0, like a fresh `-warp` start.

**Genuine evidence for the per-tic derivation already exists.** The genuine runners' `HU_Ticker`
stub records the pointer it sees each tic before clearing it, as vanilla does
(tools/reference/m5a_stubs.c:117-127). The dump's `P.message` column reports it per tic
(tools/reference/m4a_driver.c:537). Every existing synthetic and real-map dump therefore holds the
genuine "assigned this tic, and which" sequence, so the Rust derivation can be compared against
it without a new C run.

**Verdict.** "A message was assigned in tic N, and which" is **exactly derivable for every kind
wire 24 can produce**, from the stored code, the tic's events and the fed command: **class (b)**,
with genuine per-tic evidence on hand. The displayed text is the C's string constant for the
code (d_englsh.h, through dstrings). §1.5.5 owns the strings.

#### Item 6. Level end, death and refusals as the app sees them

`Simulation::advance` returns `Result<(), SimError>` (crates/doom-sim/src/lib.rs:1291). A refused
tic keeps the complete canonical state (the whole-tic rollback; COMPATIBILITY wire 23 "Runtime
witnesses"), so the last good frame stays valid to show.

- **Level exit.** The exit tic itself completes. It pushes `Event::Mover(MoverEvent::Exit { secret })`:
  - from a switch or line: crates/doom-sim/src/use_lines.rs:237; RD1 51 sets `secret`;
  - from sector special 11 with health at most 10: crates/doom-sim/src/sector_thinkers.rs:584-590,
    `secret: false`. This is how E1M8 ends: `A_BossDeath` only lowers the tag-666 floor and never
    exits (COMPATIBILITY wire 24).

  The snapshot then reads `level.exit` 1 and, for special 51, `level.secret_exit` 1
  (crates/doom-sim/src/snapshot.rs:861; STATE_FORMAT DSIM39). The next `advance` returns
  `Err(SimError::LevelExited)` before any work (crates/doom-sim/src/lib.rs:1297-1300). The app
  knows the map number (it chose it, and DSIM40's `level.map` holds it). So it can tell a normal
  exit, a secret exit and the end of the episode: on E1M8, `G_DoCompleted` would go to the
  victory finale instead of the intermission (see §1.2.4 and §1.5 for g_game.c).
- **Death.** `OriginalPlayer::state` becomes `Dead` and `health` 0, and
  `Event::PlayerBody(PlayerBodyEvent::Died { .. })` is pushed (crates/doom-sim/src/original_player.rs:217-222).
  Commands keep being admitted while dead: under the level policy every admitted command is
  accepted (crates/doom-sim/src/player_commands.rs:153-160). A command with `BT_USE` makes
  `P_DeathThink` request a reborn. That tic completes with `MoverEvent::Reborn { slot: 0 }`
  (crates/doom-sim/src/demon_expanded_map.rs:927-935), and the next `advance` returns
  `Err(SimError::PlayerReborn)` (crates/doom-sim/src/lib.rs:1301-1303).
- **Every other refusal (a witness)** is a `SimError` variant (crates/doom-sim/src/lib.rs:284-323):
  `PlayerBody`, `Store`, `Actor`, `Combat`, `Weapon`, `Projectile`, `Perception`, `Pursuit`,
  `Monster`, `MapSpawn`, `Geometry` and `MapScale`, each wrapping a `Copy` enum of its own, plus
  `InvalidPlayer`, `UnsupportedButtons` and `TicOverflow`. Examples are
  `MapScale(SpecialCross)`, `Store(Capacity)` and `Monster(Arithmetic)` (COMPATIBILITY wire 24
  witness list). The error names only its kind: no tic, actor or line. `Display` prints
  `simulation: {Debug}` (crates/doom-sim/src/lib.rs:324-328).
- **Renderer refusals are a second source.** They are `RenderRefusal` (crates/doom-render/src/lib.rs:84)
  and `ObserveError` (crates/doom-render/src/lib.rs:146-152). Both leave the render state unchanged.
- **What the app can show.** The map and skill. The tic (`sim.tic() + 1` for the refused tic) and
  the elapsed time (tic/35). The refusal's kind name. For an exit: normal, secret or episode end.
  For a death: that the player died. The path of the session demo just written, so that a refused
  tic can be replayed (§3.6). Optionally kills, items and secrets: `level.single_player_kill_count`,
  `level.total_kills`, `level.total_items`, `level.total_secrets`,
  `original_player.0.pickups.item_count` and `original_player.0.secret_count`, all canonical
  snapshot fields.

#### Item 7. The live command domain of wire 24

**What `advance` admits.** Wire 24 goes through `advance_expanded_demons`, which first calls
`preflight_player_commands` (crates/doom-sim/src/demon_expanded_map.rs:751-770;
crates/doom-sim/src/player_commands.rs:90-180) and then `player_weapons::validate_command`
(crates/doom-sim/src/player_weapons.rs:260-269):

- **Other slots.** Slots 2-4 must be `TicCommand::default()`, else `PlayerBody(Slot)`
  (player_commands.rs:101-107).
- **Buttons.** The admitted set is `BT_ATTACK | BT_USE | BT_CHANGE | BT_WEAPONMASK`, i.e.
  bits 0x01, 0x02, 0x04 and 0x38 (player_commands.rs:109-124; constants at
  crates/doom-sim/src/weapons.rs:8-14). Bit 0x40 and `BT_SPECIAL` 0x80 are `UnsupportedButtons`
  (player_commands.rs:125-131).
- **Weapon bits.** Under the Episode 1 policy, `BT_CHANGE` admits any weapon number 0-7: an
  unowned selection is ignored, and 7 is the chainsaw (COMPATIBILITY wire 23 "Weapons" row).
  Weapon bits without `BT_CHANGE` are `UnsupportedButtons` (player_weapons.rs:261-267).
- **forward and side.** Both are admitted in -127..=127. -128 is `PlayerBody(Unsupported)`
  (player_commands.rs:132-136, with `EPISODE1_MAX_COMMAND_MOVE` = 127 at :10-17).
- **angle_turn.** It is **not validated at all**: every `i16`, nonzero low byte included, is
  applied in full as `angle += (angle_turn as u32) << 16` (`command_thrust`,
  crates/doom-sim/src/lib.rs:486-511, via `apply_player_motion`, :517-544). This matches
  `P_MovePlayer`'s `mo->angle += cmd->angleturn<<16`.
- **While dead**, any admitted command is accepted (player_commands.rs:153-160).

The probe observed this on `m3h9c_synthetic.wad` E1M1, wire 24, skill 2:

| Command fed | Result | Player angle change |
| --- | --- | --- |
| `angle_turn` 0x7fff | `Ok` | 0x7fff0000 |
| `angle_turn` 0x0001 | `Ok` | 0x00010000 |
| `angle_turn` -0x81 | `Ok` | 0xff7f0000 |
| buttons 0x40 | `Err(UnsupportedButtons)` | — |
| buttons 0x80 | `Err(UnsupportedButtons)` | — |
| `BT_CHANGE` with weapon 7 (0x3c) | `Ok` | — |
| `BT_CHANGE` with weapon 6 (0x34) | `Ok` | — |
| forward -128 | `Err(PlayerBody(Unsupported))` | — |
| side -128 | `Err(PlayerBody(Unsupported))` | — |
| a nonzero command in slot 2 | `Err(PlayerBody(Slot))` | — |

So **the simulation admits a nonzero low byte**. Only the demo writer refuses one
(crates/doom-sim/src/demo.rs:297-299).

**Genuine-engine evidence for a nonzero low byte.**

- *None from the whole-engine harness.* It replays version-110 demos, which carry only the high
  byte (`G_ReadDemoTiccmd`, g_game.c:1491-1503).
- *Older bespoke C scenarios did feed full `angleturn` values* through the genuine
  `P_MovePlayer` body, with the declared unsigned-shift adapter. Example:
  tools/reference/m3g4_reference.py:34-35 (`turn=32767`, low byte 0xff, and `turn=-32768`),
  run under the M3g4 command profile, not under wire 24, and only for the movement arithmetic.

**This does not matter for M5b, because of a vanilla rule the app must reproduce.** When
recording, `G_Ticker` calls `G_WriteDemoTiccmd` on the live command before the tic runs
(g_game.c:667-669). That function writes `(angleturn+128)>>8` and then calls `G_ReadDemoTiccmd`
on the same command ("make SURE it is exactly the same", g_game.c:1506-1523). The game therefore
executes the **rounded** command, whose low byte is 0.

Since every M5b session records a demo, the app must apply the same rounding before `advance`:
`angle_turn = ((((angle_turn as i32 + 128) >> 8) as u8 as u16) << 8) as i16`. The live domain
then reduces to the demo domain:

- `|forward|, |side| <= 50` (`G_BuildTiccmd` clamps, g_game.c:413-420);
- buttons from `BT_ATTACK`, `BT_USE` and `BT_CHANGE` with weapons 0-7 (keys '1'-'8',
  g_game.c:330-346);
- `angle_turn` with a zero low byte.

Pause and save (`BT_SPECIAL`) are intercepted by the app and never fed (§3.4). That domain is
exactly the one the whole-engine harness has evidence for, and each session's demo can itself be
replayed through it (§3.6).

#### Item 8. What a level start needs, and what a restart costs

**Today's setup** (crates/doom-sim/examples/player_level_run.rs:208-226 and
crates/doom-render/tests/common/mod.rs:111-157, which m5a_view and m5a_real share):

1. Read the IWAD and parse it: `doom_wad::Wad::parse(&bytes, Limits::default())`.
2. Build `doom_map::Map::from_wad(&wad, "E1M<n>", MapLimits::default())`
   (crates/doom-map/src/parse.rs:105).
3. Construct `Simulation::original_player_episode1_v2_map(map, n, skill, MapContextOptions::default())`
   (crates/doom-sim/src/player_monsters.rs:321-326). `n` must be 1-9. `skill` is `Skill::Medium`
   (2) or `Skill::Hard` (3); others are refused (COMPATIBILITY wire 24).
4. Call `spawn_map_things()`, `initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())`
   and `spawn_level_specials()`, once each, before the first tic.
5. For the renderer: build a `LumpDirectory`, `add_wad` each WAD in load order (and
   `add_single_lump` for a lone lump), then `Resources::load(dir, GameMode)`,
   `Level::load(&res, &map, 1)`, and `Renderer::new(Arc<Resources>, Arc<Level>, RenderConfig { screenblocks, detail: High })`
   (crates/doom-render/src/lib.rs:412-425). Then `begin_level(&sim)` at tic 0, and after every
   committed `advance`, `observe(&sim)` then `render(&sim, &mut frame)`
   (crates/doom-render/src/lib.rs:460-593).
6. Write the demo with `doom_sim::demo::write_demo_episode1_v2(map, skill, &commands)`
   (crates/doom-sim/src/demo.rs:255-279). It takes the whole command list at once: there is no
   streaming writer (§3.6).

`GameMode` (Shareware, Registered or Retail) is the caller's choice
(crates/doom-render/src/r_data.rs:360-369). It changes the switch list. The app must pick it as
the C's version identification does. The M5a runner uses Registered.

**Restart.** `Simulation` implements `Clone` (crates/doom-render/tests/rewind.rs:7-9 relies on
it). The cheapest exact restart is to keep the tic-0 simulation, taken after the four setup calls,
and clone it. Rebuilding from the same `Map`, map number and skill gives the same state, at the
cost of the spawn and specials work. `Resources` and `Level` are reused through their `Arc`s. The
renderer starts again with `begin_level` (zeroed framebuffer, `fuzzpos` 0; decision R8). That is
one clone per restart, not per tic, which is consistent with "no full-simulation clone per tic".

**What must be reset.** The app's restart is a *new game on that map*, the analogue of `-warp`,
**not** vanilla's single-player reborn:

- **Vanilla's reborn.** `G_DoReborn` reloads the level through `G_DoLoadLevel`, which neither
  calls `M_ClearRandom` nor resets `gametic`, `prndindex` or `rndindex`.
- **Wire 24 has no reborn.** It refuses the reborn (`PlayerReborn`) and always starts with
  `prndindex` 0.
- **What a new game resets.** `G_InitNew` calls `M_ClearRandom`, which zeroes both indices
  (g_game.c:1414; m_random.c `M_ClearRandom`). So the status bar's `M_Random` index must restart
  at 0.

The status-bar and HUD history must restart at fresh-process values, which is more than
`ST_Start` resets:

- **`ST_initData`** (st_stuff.c, the `ST_initData` body; called through `ST_Start` from
  `P_SpawnPlayer`, p_mobj.c:693-698) resets `st_firsttime`, `st_clock`, `st_statusbaron`,
  `st_faceindex`, `st_palette = -1`, `st_oldhealth = -1`, `oldweaponsowned` and `keyboxes`.
- **It does not reset** the file statics `st_facecount` (st_stuff.c:384) and `st_msgcounter`
  (:285), or the function statics `priority` and `lastattackdown` (st_stuff.c:757-758) and
  `ST_calcPainOffset`'s cache (:731-732).
- **`HU_Start`** (hu_stuff.c:419-484) resets `message_on` and the two message flags, but not
  `message_counter`.

A fresh process has the static initial values (`priority` 0, `lastattackdown` -1,
`st_facecount` 0, `message_counter` 0). The restart must reproduce them, and the genuine
comparison must start from a fresh process too.

---

## 2. Harness design, evidence first

Both designs were prototyped in scratch (the full-frame runner in `~/scratch/m5b/proto-frames`, the input driver in `~/scratch/m5b/proto-input`); Phase 2 rebuilds them as committed tools.

### 2.1 Full frames: the genuine status bar, HUD and palette in the runner

This section reports a scratch prototype built in M5b Phase 1 to size the full-frame harness. It
lives in `~/scratch/m5b/proto-frames` (ext4; `README.txt` there says how to rebuild and re-run
it) and is **not committed**; no repository file was edited. Source authority is linuxdoom-1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`; every `file:line` is in `linuxdoom-1.10/` unless it
names a `tools/` or `tests/` path. Observed on 2026-11-17 (America/Los_Angeles), gcc 15.2.0,
x86-64. The real-map runs read a scratch copy of the registered IWAD (`~/scratch/m5b/iwad`, the
same SHA-256 as the read-only original); only counts, pass/fail facts and digests are recorded.
Address runs used `ASAN_OPTIONS=detect_leaks=0`: leak detection was disabled, so they are not
LeakSanitizer coverage. No timing was measured.

#### 2.1.1 What the prototype is

A fork of the M5a runner, built from copies (the M5a driver, stubs, build script and contract stay
untouched; the prototype's `build.py` imports `tools/reference/m5a_build.py` read-only for the
sources, adapters, flags and the generated `m5a_types.h`):

- **Sources.** The 41 genuine files of the M5a runner plus `st_stuff.c st_lib.c hu_stuff.c
  hu_lib.c m_cheat.c` compiled whole (46). `v_video.c`, `dstrings.c` (with `d_englsh.h`) and
  `r_draw.c` were already whole. `m_cheat.c` is needed only because `ST_Responder` references
  `cht_CheckCheat`/`cht_GetParam`; the runner has no events, so it is never entered.
- **Stubs** (`m5b_stubs.c`, 53 declared: 37 functions, 16 variables). From M5a's 57 the eight
  status-bar/HUD stubs are removed (`HU_Start`, `HU_Ticker`, `HU_Responder`, `HU_dequeueChatChar`,
  `player_names`, `ST_Start`, `ST_Ticker`, `ST_Responder`); four are added:
  - `I_SetPalette` [io] i_video.c:580-600: passes the pointer to a driver hook that records which
    of the 14 PLAYPAL palettes it is (`(pal - lumpcache[PLAYPAL]) / 768`, refusing anything not on
    a 768-byte boundary inside the lump: 0 unknown in every run);
  - `showMessages` [var] m_menu.c:79, value 1 (the `show_messages` default, m_misc.c:239), read by
    `HU_Ticker` (hu_stuff.c:519);
  - `inhelpscreens` [var] m_menu.c:125, false, read by the D_Display mirror (d_main.c:244, 299);
  - `S_ChangeMusic` [io] s_sound.c, aborting: referenced only by `ST_Responder`'s music cheat.
  No new LP64 or sanitizer adapter was needed (below).
- **Init** (d_main.c order): `V_Init`, `Z_Init`, `W_InitMultipleFiles`, `R_Init`, the P_Init
  pieces and `R_InitSprites` as M5a, the single-player `D_CheckNetGame` values, then `HU_Init`
  (d_main.c:1110) and `ST_Init` (d_main.c:1113). `ST_Start` and `HU_Start` are **not** called by
  the driver: the genuine `P_SpawnPlayer` calls them for the console player (p_mobj.c:693-699)
  inside `P_SetupLevel`, so they run at the vanilla point of the load without help. (The task brief
  placed them in `G_DoLoadLevel`; the source puts them in `P_SpawnPlayer`.)
- **Tic.** `G_Ticker` is unchanged; its `GS_LEVEL` case now reaches the genuine `ST_Ticker` and
  `HU_Ticker` after `P_Ticker` (g_game.c:729-734).
- **Display** (`m5b_status.inc`): a mirror of `D_Display` (d_main.c:191-355) for `GS_LEVEL`
  without menu, automap, pause or wipe, called after the dump of every dumped tic, with its own
  statics (`viewactivestate`, `menuactivestate`, `inhelpscreensstate`, `fullscreen`,
  `oldgamestate = -1`, `borderdrawcount`):
  1. `setsizeneeded` (left true by `R_Init`'s `R_SetViewSize`) makes the first display call
     `R_ExecuteSetViewSize` and set `oldgamestate = -1`, `borderdrawcount = 3` (d_main.c:215-220).
     M5a called `R_ExecuteSetViewSize` right after the load instead; the vanilla timing changed
     no dump and no view pixel (proof (e) below).
  2. `wipe = gamestate != wipegamestate` (d_main.c:223-229): true on the first display only
     (`wipegamestate` starts `GS_DEMOSCREEN`, d_main.c:188, and `G_DoLoadLevel` leaves it,
     g_game.c:472-473). The melt itself is out of scope; its only screen effect ends equal to the
     drawn frame (`wipe_doMelt` copies every end-screen row before reporting done), so only
     `redrawsbar` is mirrored. Its M_Random draws are not (finding FH1).
  3. `HU_Erase ()` when `gametic` (d_main.c:231-232).
  4. `redrawsbar = wipe || (viewheight != 200 && fullscreen)` or help-screen exit, then
     `ST_Drawer (viewheight == 200, redrawsbar)` and `fullscreen = viewheight == 200`
     (d_main.c:237-247), **before** `R_RenderPlayerView` (d_main.c:267-268).
  5. `HU_Drawer ()` (d_main.c:270-271); the `I_SetPalette (PLAYPAL)` of d_main.c:274-275 is
     skipped because it runs only when `gamestate != GS_LEVEL`.
  6. `R_FillBackScreen ()` when `oldgamestate != GS_LEVEL` (d_main.c:278-282) and the border
     block (d_main.c:285-295): both return at once at screenblocks 10 and 11
     (`scaledviewwidth == 320`, r_draw.c:746-747 and 850-851).
  7. The state copies of d_main.c:297-300, `NetUpdate ()` (the declared stub); `paused` and the
     automap abort the run if ever true; `M_Drawer` is not called (no menu).
  The frame (digest, raw dump, tracker rows) is taken after step 7, so it covers the whole
  320x200 `screens[0]`. The prototype keeps M5a's `--selftest-pixel` (after the view) and adds
  `--selftest-bar TIC X Y` (after the display) and `--selftest-palette TIC` (record only).
- **Wrapped symbols** (`-Wl,--wrap`, each records and calls the genuine function): M5a's 16 plus
  14: `HU_Ticker`, `ST_Ticker`, `M_Random`, `M_ClearRandom` (dump remedies, below),
  `STlib_updateMultIcon`, `STlib_updateNum`, `STlib_updatePercent`, `STlib_updateBinIcon` (face
  index and widget values), `HUlib_drawSText` (message showing), `V_DrawPatch`,
  `V_DrawPatchDirect`, `V_CopyRect` (tracker extension), `W_CacheLumpName`, `W_GetNumForName`
  (lump-lookup trace). Every call of these is cross-object (st_stuff.c, st_lib.c, hu_lib.c,
  g_game.c into another file), so the linker redirects all of them; `V_DrawPatchDirect`'s internal
  call to `V_DrawPatch` (v_video.c:343) stays unwrapped, which is why both are wrapped.
- **Frame stream.** The F row is M5a's (now the digest of the whole frame). A new S row per frame:
  `tic, palette, palette_set, face, message_on, rndindex, st_draws, snd_draws, wipe_draws,
  bar_undef_px, skipped_patches, beyond_rows, lump_overrun, view_sha256` (`palette` is the
  current palette, the last index `I_SetPalette` received; `palette_set` the index set during this
  frame or -1, since `ST_doPaletteStuff` calls `I_SetPalette` only on a change, st_stuff.c:1045-1050;
  `view_sha256` is the digest with the rows below the view zeroed, for the M5a comparison). X rows
  record each message `HU_Ticker` took (tic and the M4a message code); W rows (end of run) list per
  widget the calls, first, min and max value and the number of changes.

The prototype builds and runs in all three variants (strict, overflow, address) on all 48
synthetic items and all 61 real runs.

#### 2.1.2 Proof (a): the status bar and HUD leave the game dumps unchanged, except two declared fields

Items: the 26 rooms of `tests/reference/m5a_manifest.json` (the brief's 20 plus AMPFLICK, VIEWZ,
SWITCH, LANDING, SEGANGLE and MASKPAD, added later in M5a) and the 22 scripts of both M4b
manifests: 48 items, each with the prototype graphics PWAD after the map WAD. Command:
`python3 run.py --jobs 6 --out synth_all2.json; python3 summarize.py synth_all2.json`.

**Exactly two dump columns change, and nothing else.** With both remedies switched off
(`--no-mask-rnd`, `--no-hu-record`), cell-by-cell comparison of every snapshot against the
remedied dumps (which equal the frozen ones, below):

| Column | Items | Cells (= snapshots) | Cause |
| --- | ---: | ---: | --- |
| `H.rndindex` | 48 of 48 | 13,502 (every tic snapshot; never the load) | `ST_Ticker` draws `st_randomnumber = M_Random ()` once per tic (st_stuff.c:992). In single player `rndindex` is presentation state: its only gameplay reader is the netgame consistency check (g_game.c:680-692), and `M_Random` is called only by st_stuff.c, s_sound.c, f_wipe.c and wi_stuff.c (grep). |
| `P.message` | 13 of 48 | 26 (the 26 tics a message was taken) | The genuine `HU_Ticker` clears `plr->message` after taking it (hu_stuff.c:519-530), inside the same `G_Ticker`, so a dump after `G_Ticker` reads 0. The M4a stub did the same clear but kept the pointer for the dump (m5a_stubs.c, `stub_hu_message`), so the column differs only on the tic a message is taken, not on every tic after it. |

No other column, row kind or row count differs; `ML_MAPPED` behaves as in M5a (masked in the
render-every dumps, decision R5); `prndindex` is identical (no status-bar or HUD code calls
`P_Random`, grep of st_*.c, hu_*.c).

**Remedy options.**

1. *Record and mask* (prototype default). `__wrap_HU_Ticker` stores
   `players[consoleplayer].message` before calling the genuine function, and the P row prints it
   (exactly the M4a stub's meaning: "the message HU_Ticker found this tic"). `__wrap_M_Random`
   counts every presentation draw (reset by `__wrap_M_ClearRandom`, g_game.c:1414), and the H row
   prints `(rndindex - draws) & 0xff`, which is the gameplay-only index the frozen runners print;
   the genuine `rndindex` and the draw counts go to the frame stream (S row). Proves: the game
   state after every tic is byte-identical to the frozen runners', and the presentation's effect on
   it is exactly these two cells, both carried elsewhere.
2. *Dump between `P_Ticker` and `ST_Ticker`* (a dump taken from `__wrap_ST_Ticker`). Fixes
   `P.message` without a recorded pointer (nothing between `P_Ticker` and `HU_Ticker` writes it),
   but **not** `rndindex`: the earlier tics' `ST_Ticker` draws are already in it. So it still needs
   option 1's mask, and it moves the dump inside `G_Ticker` (the frozen dump point is after it).
3. *Save and restore `rndindex` around `ST_Ticker`.* Rejected: it changes the genuine
   `st_randomnumber` sequence and therefore the face (finding FH1 shows how sensitive it is).

**Recommendation: option 1**, declared in the contract like R5: the P row's message is the
pointer `HU_Ticker` found, the H row's `rndindex` excludes presentation draws, and both genuine
values travel in the frame stream. Observed with option 1:

- never-render dumps: 22 of 22 M4b scripts equal the committed `dumps_sha256`; 26 of 26 rooms
  equal the frozen M4b runner's dump set (same WAD, `m5a_gfx.wad`, demo);
- render-every dumps at screenblocks 10 and 11 equal the never-render set snapshot for snapshot
  (48 of 48 at each configuration), and equal the M5a strict runner's render-every dumps (same
  `ML_MAPPED` masking) snapshot for snapshot (48 of 48 at each).

#### 2.1.3 Proof (b): variant identity, and sanitizer findings

For each of the 48 items at screenblocks 10 and 11 the strict, overflow and address frame streams
(F, S, X, U, M, N rows) are byte-identical up to the earliest end (48 of 48 at each
configuration), with M5a's two normalisations: a tracker-flagged frame is compared on its masked
digest (F digest and S `view_sha256` replaced by `undefined`; only M5SHORT, 134 frames), and rows
from the first limit witness on are not compared (M5OPEN at tic 40; M5SSEG, whose address run
stops at the sanitizer, as in M5a). Frames compared per configuration: 13,391 (rooms 8,156, M4b
scripts 5,235).

**Sanitizer findings in st_*.c, hu_*.c, v_video.c: none.** The only reports in any run are the
two M5a limit rooms' known address-build stops in `R_StoreWallRange` (M5OPEN) and
`R_ClipSolidWallSegment` (M5SSEG), identical to `tests/reference/m5a_results.json`. So no new
adapter is needed for the synthetic set; the real maps (§2.1.8) found none either. Compile-time
only: gcc warns in the sanitizer builds that `sprintf (buffer, "STCFN%.3d", j++)`
(hu_stuff.c:408) could overflow `char buffer[9]` for an arbitrary int; `j` runs from 33 to 95, so
it writes 8 characters and the terminator: no adapter. The address build would catch a
`V_DrawPatch` post running off `screens[4]` (a 320x32 zone block, st_stuff.c:1470) only through
the tracker, not ASan (the zone is one malloc block), so the prototype checks it directly (S row
`beyond_rows`: 0 everywhere).

#### 2.1.4 Proof (c): run-to-run determinism

The strict build run twice per item and configuration (ASLR on: `randomize_va_space` 2): 48 of 48
identical at each configuration, with the normalisations above. Without them, M5SHORT differs in
its tracker-flagged frames (the M5a undefined reads, unchanged) and M5OPEN in the O/E rows of its
witness frame (the opening count printed after the overflow depends on the overwritten memory;
M5a never compares that frame). Nothing in the status bar, the HUD or the first frames depends on
the process: the `--diag-fill 171` diagnostic (fills `screens[0]` after `V_Init`, diagnostic only)
changed **no** frame of any of the 48 items at screenblocks 10 or 11 (`diagfill.py`), so every
pixel of every compared frame is written before it is shown, and `screens[4]` (zone memory the
genuine code never initialises; the ZeroFill wrapper zeroes it here) is fully written by `STBAR`
before any `V_CopyRect` reads it (S row `bar_undef_px` 0 everywhere).

#### 2.1.5 Proof (d): the comparison can fail

On M5VIEW at screenblocks 10: `--selftest-bar 20 200 185` flips one status-bar pixel after the
display of tic 20, and `--selftest-palette 25` records palette 1 instead of 0 at tic 25. Against
the plain run: the first differing F row is tic 20, the raw frames differ in exactly 1 pixel, the
first at (200, 185); the first differing S palette is tic 25 (0 vs 1), 1 row. Pass.

#### 2.1.6 What the synthetic items exercise (sb10 and sb11 identical in all of these)

- Frames per configuration 13,391; **480 with a non-zero palette**: palette 2: 124, 3: 115,
  4: 66, 5: 13, 6: 12, 10: 146, 11: 4 (0: 12,911). Never: 1, 7, 8, 9, 12, 13.
- **1,450 frames show a message**; 26 messages taken in 13 items, 14 kinds (M4a codes 2, 3, 8,
  17, 20, 22, 24, 25, 28, 32, 33, 34, 36 and 41, the turbo message of g_game.c:671-679).
- **Face indices seen: 17 of 42**: 0, 1, 2 (pain 0 straight), 3 (TR0), 4 (TL0), 6 (EVL0),
  7 (KILL0), 11 (TR1), 12 (TL1), 15 (KILL1), 18 (ST22), 19 (TR2), 23 (KILL2), 26 (ST32),
  31 (KILL3), 39 (KILL4), 41 (dead). Never: every ouch face (5, 13, 21, 29, 37), the god face
  (40), the evil grin above pain 0, most straight and turn faces above pain 0.
- **Widgets that never changed** (W rows): the arms background (always on), plasma and BFG arms
  (0), all three key boxes (-1), the cell counter (0), all four max-ammo counters (200, 50, 300,
  50: no backpack), the frags number (0; drawn only in deathmatch). Changed: ready ammo (3 to
  1994, the fist/chainsaw "no number" value), health and armor (0 to 200), clip, shell and rocket
  counters, pistol (always 1), shotgun, chaingun and launcher arms, the face (517 changes).
- No `V_DrawPatch` was skipped by its range check (v_video.c:221-232), no post was outside its
  lump, no post ran below row 199 or row 31 of `screens[4]`.

**Proof (e), M5a consistency.** Frames with no message showing must equal M5a's view: at
screenblocks 11 (nothing of the status bar is drawn, `st_statusbaron` false, st_stuff.c:1110) the
full-frame digest, at screenblocks 10 the `view_sha256` (rows 168-199 zeroed, as M5a never drew
them), against the committed `tests/reference/m5a_frames/*.frames`: 11,791 equal, 0 differ at
each configuration (1,600 skipped: 1,450 message frames, 150 tracker-flagged; the frame after the
M5OPEN witness is not compared).

#### 2.1.7 The first frames

- The first frame is the display after tic 0, with `gametic == 1` (`D_Display` skips the level
  drawers while `gametic == 0`, d_main.c:231, 238-239, 267); the load is never displayed. Same as M5a.
- On it `st_firsttime` is true twice over: `ST_initData` set it (st_stuff.c:1254) and the wipe
  makes `redrawsbar` true (d_main.c:242-243). `ST_Drawer` therefore calls `ST_doRefresh`
  (st_stuff.c:1117-1119): `ST_refreshBackground` draws `STBAR` into `screens[4]` and copies all
  320x32 to rows 168-199 (st_stuff.c:499-511), then every widget is drawn with `refresh` true;
  `st_firsttime` then stays false (st_stuff.c:1092) and later frames use `ST_diffDraw` (numbers
  redrawn every frame, multi-icons and the bin icon only on change; st_lib.c:152-158, 212-241, 268-290).
  `ST_initData` also sets `st_palette = -1` (st_stuff.c:1266), so `I_SetPalette` runs on the
  first frame (observed: S row `palette_set` 0 at tic 0 on every item without early damage).
- At screenblocks 10 nothing outside the view and the bar stays at the initial zero bytes: the
  view covers rows 0-167 and the bar copy rows 168-199 in full; the diagnostic fill changed no
  frame. At screenblocks 11 the status bar draws nothing (`st_statusbaron` false) and the view
  covers all 200 rows.
- **The frames depend on the display cadence.** `STlib_updateMultIcon` erases with the patch it
  last drew (`oldinum`, st_lib.c:224-238) and updates `oldinum` only when it draws, so a face that
  changes twice between two displays erases a different rectangle than one drawn every tic. The
  harness and Rust must therefore display after every tic, as vanilla does under `-timedemo`
  (`singletics`, one `G_Ticker` per `D_Display`, d_main.c:375-395); real-time `-playdemo` pacing
  would give other status-bar pixels with awkward patch sizes.

#### 2.1.8 Real maps (local only)

All 61 runs of `tests/reference/m4b_real_results.json` (every committed E1M1-E1M9 script and
DEMO1, extracted from the IWAD copy and patched 109 to 110 and checked against the M4b digest, as
`tools/local/m5a_real.py` does), IWAD only (its own status bar, font and palette lumps), skills as
recorded. Command: `python3 real.py --jobs 6 --out real_all.json; python3 summarize_real.py
real_all.json`. Per run: one strict never-render run with dumps; at screenblocks 10 and 11 strict
render-every with dumps, overflow and address render-every without dumps, strict again without
dumps, and the FH1 experiment build (strict, no dumps).

- **(a) Dumps:** 61 of 61 never-render dump sets (rndindex and message remedies on) equal the
  committed M4b strict `dumps_sha256`; at both configurations 61 of 61 render-every dump sets equal
  the never-render set snapshot for snapshot.
- **(b) Variants:** 61 of 61 runs at each configuration have identical strict, overflow and
  address frame streams through the end (flagged frames on their masked digest), and every variant
  ended as strict did. **No sanitizer report in any run** (0 in st_*.c, hu_*.c, v_video.c or
  anywhere else), so no new adapter is needed for the real content either.
- **(c) Determinism:** 61 of 61 strict runs identical run to run at each configuration.
- **Counts** (identical at screenblocks 10 and 11 unless noted): 158,966 frames per configuration
  (the M4b tic count, as in M5a); **13,660 frames with a non-zero palette**: palette 2: 3,294,
  3: 1,543, 4: 636, 5: 223, 6: 104, 7: 42, 8: 97, 10: 3,568, 11: 345, 12: 32, 13: 3,776 (0:
  145,306); palettes 1 and 9 never occur (the source says they cannot, §2.4.3). **51,508 frames
  with a message showing**, 698 messages taken. **38 distinct face indices**: every face except
  13, 21, 37 (ouch at pain levels 1, 2, 4) and 40 (god); per map 11 (E1M1) to 35 (E1M9).
- **Tracker:** no status-bar or HUD undefined read (0 frames with an undefined BG byte, a post
  outside its lump, a write below the screen or a skipped patch); the renderer's flagged frames
  are 494 at screenblocks 10 and 552 at 11, the same counts as M5a's real-map record.
- Nothing on the real maps exposed a sanitizer finding or a nondeterminism that the synthetic
  rooms did not.

#### 2.1.9 The read-provenance tracker on the full frame

The M5a tracker taints screen pixels per row from the column drawers and reports whole screen
columns. The status bar and HUD never use the column drawers: they are drawn by `V_DrawPatch`
(`STlib_*`, st_lib.c:135, 141, 147, 185, 237, 285), `V_DrawPatchDirect` (`HUlib_drawTextLine`,
hu_lib.c:122, 137) and `V_CopyRect` (st_stuff.c:509, st_lib.c:125, 235, 287). The prototype extends it
through wrappers of these three:

- `V_DrawPatch`/`V_DrawPatchDirect` to `screens[0]`: each pixel written from a post byte inside its
  lump clears the pixel's taint; a post header or data byte past the lump end (the loop has no
  bound but `0xff`, v_video.c:243-261) taints it with the lump's origin; a write below row 199 is
  counted (`beyond_rows`); a patch the range check skips (v_video.c:221-232, printed and ignored,
  not an `I_Error`) is counted (`skipped_patches`).
- The same to `screens[4]` (BG) maintains a 320x32 "defined" mask; a write below row 31 of BG
  (outside its zone block; the range check tests against 200 rows, not 32) is counted.
- `V_CopyRect` from BG to the screen copies the mask: a pixel copied from a BG byte no patch wrote
  is undefined (zone garbage in vanilla, ZeroFill zeros here), origin `BG:unset`.

Can a status-bar or HUD read be undefined? Only in these ways, all excluded by the synthetic set
by construction (opaque 320x32 `STBAR`, patches within their lumps and within the screen) and
observed 0 times on 48 synthetic items and on the 61 real runs (§2.1.8): an `STBAR` with holes or
smaller than 320x32 (BG bytes never written, then copied by `ST_refreshBackground` or a widget
erase); a widget erase rectangle reaching outside what `STBAR` covers; a patch post past its lump.
The BG buffer is never read before the first refresh: every `V_CopyRect` from BG is inside
`ST_Drawer`, after `ST_doRefresh` on the first frame. At screenblocks 11 BG is never read.
Design: since the 2D reads are per pixel and the M5a U rows are per screen column, Phase 2 should
not mask whole columns for a 2D undefined read; a frame with one should be refused as a whole (a
witness row with the origin and pixel count), which costs nothing as long as the count stays 0.

### 2.2 Input: the genuine event path as a headless driver

This section reports a Phase 1 scratch prototype, observed on 2026-11-17 (America/Los_Angeles) on the
owner's WSL machine with gcc 15.2.0 (Ubuntu 15.2.0-16ubuntu1), plus the design Phase 2 should follow. Nothing
in the repository was changed. The prototype is at `/home/copgunn/scratch/m5b/proto-input/`, and its `README.txt` says how to rebuild
and re-run it. Every line number cites linuxdoom-1.10 at `a77dfb96`.

#### 2.2.1 What the prototype is

The prototype is a small dedicated binary, `m5b_input`. It compiles the genuine input path whole, runs it
headlessly on a text script of X events or `event_t` events, and prints one golden row per tic.

**Genuine files compiled whole (23):** `d_main.c g_game.c m_misc.c m_menu.c hu_stuff.c hu_lib.c st_stuff.c st_lib.c
am_map.c m_cheat.c m_argv.c m_fixed.c m_random.c m_swap.c m_bbox.c tables.c info.c doomstat.c doomdef.c dstrings.c d_items.c
sounds.c i_video.c`. Together they give every body the task names:
- `D_PostEvent` and `D_ProcessEvents` (d_main.c:150-177);
- `G_Responder` (g_game.c:504-597), `G_BuildTiccmd` (g_game.c:237-437) and `G_Ticker`;
- `G_WriteDemoTiccmd` and `G_ReadDemoTiccmd` (g_game.c:1491-1524);
- the defaults table and `M_LoadDefaults` (m_misc.c:234-403);
- `M_Responder` (m_menu.c:1349), `HU_Responder` (hu_stuff.c:617), `ST_Responder` (st_stuff.c:518) and `AM_Responder`
  (am_map.c:614);
- the X translation: `xlatekey`, `I_GetEvent` and `I_StartTic` (i_video.c:97-340).

**The fake Xlib.** The genuine `i_video.c` compiles unchanged against our own headers:
- `fakex/X11/Xlib.h`, `Xutil.h`, `keysym.h` and `extensions/XShm.h`. They declare only what i_video.c uses, with
  X11 protocol values for the event types, masks, buttons and keysyms.
- `fakex/errnos.h`. i_video.c:48 includes `<errnos.h>`, which no current system has.

This host has no X11 headers (`/usr/include/X11` is empty), and none were installed. `fakex.c` is the fake
server:
- a FIFO of scripted `XEvent`s, drained by `XPending` and `XNextEvent`;
- `XKeycodeToKeysym(d, keycode, 0)` returns the keysym the script named;
- the pointer position, and the held-button mask carried in `.state`. A button event carries the state before it,
  which is the X protocol rule that i_video.c:215-243 relies on;
- `XWarpPointer`, counted. With `config warpmodel 1`, a warp that moves the pointer queues a `MotionNotify` at
  the centre ahead of the next tic's events.

Every other Xlib entry point aborts. Those are reachable only from `I_InitGraphics`, `I_FinishUpdate` and
`I_ShutdownGraphics`, which the driver never calls. `i_video.c` compiles with 17 warnings, all in those unreached
functions (pointer-to-int casts in `Expand4` and in the shm setup).

**Stubs (`stubs.c`).** Every stub carries a `STUB symbol [class] file:line: reason` line in the M3h9c style.

- 26 function stubs:
  - `[io]`, `[reads]` and `[writes]`: `I_BaseTiccmd` (the genuine body, i_system.c:64-68), `I_GetTime` (a
    counter advanced once per tic), `I_Error` and `I_Quit` (record the message and longjmp to the driver),
    `I_WaitVBL`, `Z_Malloc` (calloc with a `memblock_t` header carrying ZONEID, so the `Z_ChangeTag` macro check of
    z_zone.h:72-77 passes), `Z_Free`, `Z_ChangeTag2`, `Z_CheckHeap`, the four `W_*` lookups (a zero-filled dummy
    lump behind a ZONEID header), seven `S_*` sound calls (`S_StartSound` is counted), `R_SetViewSize` (records its
    effect), `R_FlatNumForName`, `R_TextureNumForName` and `P_GivePower` (recorded).
  - Two declared adapters:
    - `P_SetupLevel` builds no map. For the console player it does what `P_SpawnPlayer` does that the input path
      reads: `G_PlayerReborn` on `PST_REBORN` (p_mobj.c:651-652), links the mobj, then calls `ST_Start` and
      `HU_Start` (p_mobj.c:694-699).
    - `P_Ticker` runs no simulation. It keeps only the `paused` early return and increments `leveltime`.
- 33 variable stubs: the map arrays, `screens`, `netcmds`, `maketic`, `ticdup = 1`, `maxammo` with its genuine
  values, and others.
- 119 generated stubs that abort if called: the 75 `A_*` action functions that `info.c` references, plus 44
  others such as `R_RenderPlayerView`, `V_DrawPatch`, `TryRunTics` and `P_SpawnMobj`.

**Adapters.**
- Under LP64, `m_misc.c` does not compile. The defaults table stores string pointers in `int defaultvalue`, and GCC
  rejects `(int) "sndserver"` and the 12 similar initialisers with "initializer element is not constant"
  (m_misc.c:257, 264, 265, 288-297).
- The prototype applies 15 one-line LP64 ABI adapters:
  - `int defaultvalue` becomes `intptr_t` (m_misc.c:229);
  - the 13 value casts become `(intptr_t)`;
  - the store at m_misc.c:354 writes string rows through `char**`. It classifies rows the way `M_SaveDefaults`
    does (m_misc.c:320-321).
- `-include stdint.h` is added for that file. strict32 compiles the genuine text with no adapter.

**Linker observers** (`-Wl,--wrap`) log and then call the genuine function: `D_PostEvent`, `M_Responder`,
`G_Responder`, `HU_Responder`, `ST_Responder` and `AM_Responder`. They write the optional trace, not the golden
rows.

**Start-up.** It mirrors `D_DoomMain` for what the input path reads:
1. `myargc = 1`, so there is no `-config` (m_misc.c:357).
2. `gamemode = registered`, `language = english`.
3. `M_LoadDefaults ()` (d_main.c:1015), then `M_Init`, `HU_Init` and `ST_Init` (d_main.c:1092, 1110, 1113).
4. Single-player globals: `playeringame[0]`, `consoleplayer = displayplayer = 0`, `netgame = deathmatch =
   demoplayback = singledemo = 0`.
5. `X_display` points at the fake display; `X_width = 320`, `X_height = 200`.
6. With recording on (the default), `G_RecordDemo` (g_game.c:1530-1545), then `G_InitNew (sk_medium, 1, 1)`. That
   sets `usergame = true` (g_game.c:1443) and, through `G_DoLoadLevel`, `gamestate = GS_LEVEL` and the cleared input
   state.
7. `G_BeginRecording` (d_main.c:356-357).

There is no menu, no demo playback and no netgame. The state dump before tic 0 shows `gamestate=0` (GS_LEVEL),
`menuactive=0` and `usergame=1`.

**Making the defaults deterministic.** No `~/.doomrc` is read. `basedefault` is the static `char basedefault[1024]`
(d_main.c:126). Only `IdentifyVersion` fills it (d_main.c:614-654), and the driver never calls that. So
`defaultfile = ""` (m_misc.c:364) and `fopen ("", "r")` fails (m_misc.c:367). The values are then exactly the table
of m_misc.c:236-297:

| Setting | Default | Source |
| --- | --- | --- |
| Turn right / turn left | `KEY_RIGHTARROW` / `KEY_LEFTARROW` | m_misc.c:243-244 |
| Forward / back | `KEY_UPARROW` / `KEY_DOWNARROW` | m_misc.c:245-246 |
| Strafe left / strafe right | `,` / `.` | m_misc.c:247-248 |
| Fire / use | `KEY_RCTRL` / space | m_misc.c:250-251 |
| Strafe modifier / run | `KEY_RALT` / `KEY_RSHIFT` | m_misc.c:252-253 |
| Mouse sensitivity | 5 | m_misc.c:236 |
| Mouse buttons fire / strafe / forward | 0 / 1 / 2 | m_misc.c:269-271 |
| `use_mouse` / `use_joystick` | 1 / 0 | m_misc.c:268, 273 |

The ordering relied on: the string rows also change `chat_macros`, `mousedev` and `mousetype`, which the input path
never reads.

**Per-tic order.** Each tic is the `singletics` branch of `D_DoomLoop` (d_main.c:376-386), after the script's events
for that tic are queued in the fake server:

```
I_StartTic; D_ProcessEvents; G_BuildTiccmd (&netcmds[consoleplayer][maketic%BACKUPTICS]);
if (advancedemo) D_DoAdvanceDemo; M_Ticker; G_Ticker; gametic++; maketic++;
```

`NetUpdate`'s build loop (d_net.c:400-411) does the same `I_StartTic; D_ProcessEvents; G_BuildTiccmd` once per tic.
`G_Ticker` is genuine:
- the copy into `players[].cmd`;
- the `G_WriteDemoTiccmd` write and re-read (g_game.c:664-669);
- the turbo check, the special buttons (g_game.c:698-720) and the ST, AM and HU tickers.

Only `P_Ticker` is replaced.

#### 2.2.2 Driver shape: recommendation

| | (i) dedicated input driver (prototyped) | (ii) event-script mode in the whole-engine runner |
| --- | --- | --- |
| Files compiled | 23 genuine + fake Xlib + 26/33/119 stubs | the M5a set (41 files) plus d_main.c, m_misc.c, m_menu.c, hu_*.c, st_*.c, am_map.c, m_cheat.c, i_video.c, and changes to its demo-driven loop |
| Needs a WAD | no | yes (a map for every input script) |
| Build / run (observed, prototype) | 1.3 s build per variant; 0.8 s for all 327 scripts (strict) | M5a strict build here: several seconds; one run per script per map |
| Frozen-file impact | none (new files) | the M5a driver, stubs and contract are frozen, so it would be another fork of the runner |
| What it proves | event to command, including the responders and the demo write and re-read | the same, plus simulation feedback into input (reborn and level reload, menu pause) |

**Recommendation: (i) for the input goldens. The session replay uses the whole-engine runner unchanged.**

The two meet through the demo. In the chain experiment (§2.2.8), the input driver recorded a demo from an event
script, and the genuine M5a runner replayed it. All 52 of 52 replayed `players[0].cmd` values equalled the driver's
executed commands.

A separate event mode in the engine runner (ii) would duplicate that chain. It would add only the few places where
simulation state feeds back into input, and those are listed in §2.3. It is not worth a third runner fork for M5b.

The one interaction to cover elsewhere: `G_DoLoadLevel`'s input clearing when the player is reborn or the level
changes (g_game.c:489-496). The input driver exercises it directly with a `loadlevel` directive.

#### 2.2.3 Script and golden formats

**Script:** text, one directive per line, `#` comments. Events queue until `tic`, and the next tic's
`I_StartTic` delivers them.

```
config grab 1            # i_video.c grabMouse (vanilla: -grabmouse, i_video.c:738)
config warpmodel 1       # the fake server answers a moving XWarpPointer with a MotionNotify
config record 0          # recording is on by default (G_RecordDemo/G_BeginRecording)
press Left | release Left | tap Left     # KeyPress / KeyRelease / both in one pump; keysym name, char or 0xNNNN
bpress 1 | brelease 3 | motion 170 110 | expose
ev keydown 0xad | ev keyup 0xad | ev mouse 4 0 0   # event_t level, posted before I_StartTic
tic [N] | loadlevel | enddemo
```

Excerpt, `scripts/turn_left_repress.txt`:

```
tic
press Left
tic 4
release Left
tic
press Left
tic 8
release Left
tic
```

**Golden (CSV, stdout).** Three row kinds:

- `T,tic,forwardmove,sidemove,angleturn,consistancy,chatchar,buttons,hex,exec_forwardmove,exec_sidemove,exec_angleturn,exec_buttons`.
  - "Built" is what `G_BuildTiccmd` wrote.
  - "exec" is `players[consoleplayer].cmd` after `G_Ticker`: after `G_WriteDemoTiccmd` and its re-read when
    recording, which is the command the simulation executes.
  - `hex` holds the eight bytes field by field (`fwd:side:turn:cons:chat:btn`).
- `S,tic,<state vector>`, printed when it changes: `gamestate`, `gameaction`, `menuactive`, `messageToPrint`,
  `automapactive`, `viewactive`, `paused`, `sendpause`, `sendsave`, `setblocks`, `showMessages`, `usegamma`,
  `cheats`, `usergame`, `gamemap`, the weapons and cards masks, armour, and others.
- `E,tic,kind,message` for `I_Error` or `I_Quit`. The run stops.

An optional `--trace` writes three more row kinds:
- `P`: each posted `event_t`, with the queue head and tail. Key events print `data2` and `data3` as `-`, because
  i_video.c:203-212 leaves them indeterminate and no consumer reads them.
- `R`: each responder's verdict.
- `Q`: per tic, the count of drained X events and posted events.

Excerpt, `out/strict/turn_left_repress.csv` (columns cut to tic, fwd, side, turn, btn, exec_turn):

```
T,1,0,0,320,0,256      T,5,0,0,0,0,0          T,11,0,0,640,0,768
T,4,0,0,320,0,256      T,6,0,0,320,0,256      T,13,0,0,640,0,768
```

#### 2.2.4 Observed behaviour (strict build; 327 scripts, 41,633 tics)

In the excerpts, `n:fwd/side/turn/btn>exec_turn` is one tic.

**Bindings.** Each default binding was run alone, tapped, held, and pressed then released in the next pump (43
scripts), plus every combination named in the task.

- Forward and back: ±25 (`forwardmove[0]`). Strafe keys: ±24. With run, ±50 and ±40 (g_game.c:175-176).
- Fire gives button 1 and use gives button 2.
- The strafe modifier or run held alone changes nothing.
- The right-hand variants (`Control_R`, `Shift_R`, `Alt_R`) and `Meta_L` and `Meta_R` translate to the same codes
  (i_video.c:131-147).

**Slow turn.** `turnheld += ticdup` is applied before the test `turnheld < SLOWTURNTICS` (g_game.c:262-275):
- Held Left: tics 1-5 turn by 320 (`angleturn[2]`); from tic 6 on, by 640. Right turns the other way.
- With run pressed after the slow phase the turn is 1280, and 640 again once run is released (`turn_run_late`).
  Run held during the slow phase has no effect, because `tspeed = 2`.
- Released for one tic and pressed again, the turn starts slow again (`turn_left_repress`).
- Released and re-pressed inside one pump, the release is never seen, and the turn goes fast at tic 6 as if the key
  had stayed down (`turn_left_repress_samepump`).

**Strafe modifier.**
- With Alt, Left gives side -24, and with run as well, -40 (g_game.c:278-292).
- Releasing Alt mid-hold switches back to turning. `turnheld` keeps counting, because it depends only on the turn
  keys.
- Alt + Right + `.` gives +48 (+80 with run), clamped to 50 (g_game.c:417-420).

**Opposite keys cancel.** Left + Right, Up + Down (also with run) and `,` + `.` all give 0. Under the strafe
modifier, Left + Right gives side 0.

**Weapon keys** (g_game.c:342-348; the loop runs `i < NUMWEAPONS-1`, so keys `1` to `8`):

| Keys | `buttons` | Meaning |
| --- | --- | --- |
| `1` | 4 | `BT_CHANGE` |
| `7` | 52 | BT_CHANGE + index 6 |
| `8` | 60 | BT_CHANGE + index 7, `wp_chainsaw` (doomdef.h:189) |
| `9`, `0` | 0 | nothing |
| `5` and `3` held together | 20 | the lower index wins (`break`) |
| after releasing `3` | 36 | |
| `4` + fire + use | 31 | |

**Mouse at the `event_t` level** (sensitivity 5, so `mousex = data2*10/10`, g_game.c:579-580):

| Input | Result |
| --- | --- |
| `data2 = 20` | turn -160, exec -256 |
| `data2 = 1, -1, 4, 15` | turns -8, 8, -32, -120: all exec 0 |
| `data2 = 16` | turn -128, exec 0 |
| `data2 = -16` | turn +128, exec +256 |

The demo rounding `(angleturn+128)>>8` (g_game.c:1512) sends every built turn in [-128, 127] to 0.

At the edge of a `short`:

| `data2` | Built turn | Exec |
| --- | --- | --- |
| 4095 | -32760 | -32768 |
| 4096 | -32768 | -32768 |
| -4096 | -32768 (`+32768` narrowed) | -32768 |
| 8192 | 0 | 0 |
| 262140 | 32 | 0 |

Forward clamps: `data3 = 51` and `400` give 50, and Up + `data3 = 40` also gives 50 (g_game.c:413-416). With the
strafe modifier, mouse strafe gives `side += mousex*2`, clamped to ±50. That is beyond the keyboard's ±40, so side
±50 is a reachable value. The strafe button (`data1 & 2`) does the same.

**Two motion events in one tic: the last one wins.**
- `ev mouse 0 40 0` then `ev mouse 0 -8 0` gives turn +64. The first motion is lost.
- `ev mouse 0 0 12` then `ev mouse 0 20 0` gives turn -160 and forward 0. The second event's `data3 = 0` overwrote
  `mousey`.

`G_Responder` assigns `mousex` and `mousey` on every `ev_mouse` (g_game.c:579-580) instead of adding to them. The X
level shows the same thing (`x_motion_two_in_tic`).

**A motion and a button event in one tic.**
- Motion then button: the motion is lost, because the button event carries `data2 = data3 = 0` (i_video.c:224,
  240). Result: fire only.
- Button then motion: both count.

**Double click** (g_game.c:350-399). A swept set of 150 scripts: press P tics, release G tics, press 3 tics, after
30 idle tics.

- A transition registers only on a tic where `dclicktime > 1`. After a registration that does not complete a double
  click, `dclicktime` restarts at 0.
- Observed `BT_USE` on the second press when P + G ≤ 24 (P = 1: G ≤ 23; P = 2: G ≤ 22; P = 3: G ≤ 21). With a
  short gap the `BT_USE` arrives 1 or 2 tics after the second press. P = 1 and G ≤ 2, and P = 2 and G = 1, never
  give `BT_USE`. The strafe button behaves the same.
- Without the idle lead-in, `dclicktime` starts at 0, so a 1-tic press at tic 1 is missed (`dclick_at_start`).
- The use key clears `dclicks` (g_game.c:336-338).

**A key pressed and released inside one tic is not seen.** All 14 `bind_*_tap` scripts give an all-zero command,
including fire, use and the weapon keys. `gamekeydown` is set and cleared again before `G_BuildTiccmd` runs.

**X auto-repeat**, modelled as scripted KeyRelease + KeyPress pairs. No real X server was run.
- Pairs inside one pump: the key is never seen up, and `turnheld` keeps counting (`repeat_samepump`).
- A pair split across a tic boundary: that tic has the key up (`4:0/0/0/0`), and `turnheld` resets, so the turn is
  slow again for 5 tics (`repeat_across_tic`).

**The event queue** (`MAXEVENTS` 64, d_event.h:108; `D_PostEvent` d_main.c:150-154):
- 63 events posted before a pump: all processed.
- Exactly 64: the head wraps onto the tail, the queue looks empty, and all 64 are lost (`queue_64`, forward 0).
- 65: only the 65th is processed.
- The X level, 70 events in one pump (`queue_x_70`): the `press Up` at the front is lost, so forward is 0. Only
  the last 6 events count.

A lost KeyPress cannot make a key stick, but a lost KeyRelease leaves its key down.

**The X translation.** Run with the genuine i_video.c.
- The Button1 bug. `(state & Button1Mask)` puts bit 8 into `data1`, not bit 0 (i_video.c:218, 231, 247; Button1Mask
  is 1<<8), while Button2 and Button3 map through `?:`. Consequences:
  - Releasing the left button gives `data1 = 256 ^ 1 = 257`, so fire stays on after release until the next mouse
    event (`fire_mouse`: fire at tic 4, after `brelease 1`).
  - A left click inside one pump leaves fire on (`x_button_press_release_same_tic`).
  - Pressing the right button while holding the left one gives `data1 = 260`, so fire turns off (`x_buttons`,
    tic 2). Releasing the left button turns it back on (tic 3). Trace rows: `P,2,2,260,0,0` and `P,3,2,261,0,0`.
- `lastmousex` and `lastmousey` start at 0 (i_video.c:189-190). The first motion to (170,110) gives `data2 = 680`
  and `data3 = -440`, so forward -50 and turn -5440 (`x_motion_first_jump`).
- The centre test posts a motion only if `x != X_width/2 && y != X_height/2` (i_video.c:257-264). A motion that ends
  on either centre line is dropped, but still updates `lastmouse*`. Motions to (190,100) and (160,130) are dropped,
  and the next motion's delta is measured from the dropped point (`x_motion_centre_lines`).
- With `-grabmouse` and the warp model, every tic warps to (160,100) (i_video.c:321-332):
  - A purely horizontal move keeps `y == 100` and is dropped. The warp-back motion lands on x = 160 and is also
    dropped. All 3 tics are 0 (`x_grab_warp_horizontal`, 4 X events drained, 0 posted).
  - A diagonal move registers (`x_grab_warp_diagonal`: forward 20, turn -640).
- Expose is ignored. Keysyms 256 and above (Home, KP_Enter, Super_L, Caps_Lock) give `data1` of 256 or more, which
  `G_Responder` ignores (g_game.c:566, 571).
- Latin-1 keysyms pass through unchanged (i_video.c:150-153), and several collide with DOOM key codes:

  | Keysym | Becomes | Observed |
  | --- | --- | --- |
  | 0xff (ÿ) | `KEY_PAUSE` | `BT_SPECIAL` |
  | 0xbb (») | `KEY_F1` | menu opens |
  | 0xac (¬) | turn left | |
  | 0xad (soft hyphen) | forward | |
  | 0xb8 (¸) | `KEY_RALT` | |
  | 0xb6 (¶) | `KEY_RSHIFT` | |

- Without `-grabmouse`, vanilla gets no mouse events at all. The window's event mask selects only KeyPress,
  KeyRelease and Exposure (i_video.c:798-802; the pointer and button masks are commented out). Mouse events arrive
  only through `XGrabPointer`'s mask (i_video.c:847-851), which also confines the pointer to the window
  (`confine_to = X_mainWindow`).
- With a 320×200 window, `|dx| ≤ 319`, so `|data2| ≤ 1276` and `|angleturn| ≤ 10208`: the `short` wrap needs a
  window wider than 1024 (only `-4`, i_video.c:719-726). The `x_*` scripts without `grab` exercise the translation
  code in a configuration vanilla could not reach.

**Keys the responders ahead act on** (one script each: the key tapped, then Up held 2 tics, then Left 1 tic):

| Key (data1) | Responder, effect observed | Later game keys |
| --- | --- | --- |
| Escape (27), F1 (0xbb), F2, F3, F4 | M_Responder: `menuactive=1` (m_menu.c:1609-1616, 1536-1572) | eaten by the menu (Up, Left gave 0) |
| F6 | M_QuickSave with no slot opens the save menu | eaten |
| F7, F10 | message that needs y/n (`messageToPrint=1`) | other keys pass (m_menu.c:1497-1499): Up gave 25 |
| F9 | QuickLoad with no slot: message that needs no input | the next key (Up) is eaten closing it: forward 0 |
| F5 | M_ChangeDetail: sound, "low detail mode n.a." on stderr | pass |
| F8 | `showMessages` 1 to 0 | pass |
| F11 | `usegamma` 0 to 1 (I_SetPalette is genuine and does nothing on the fake visual) | pass |
| F12 (0xd8) | G_Responder spy mode, eaten (g_game.c:507-517) | pass |
| `-` `=` and KP_Subtract, KP_Equal | M_SizeDisplay: `setblocks` 8 and 10 (m_menu.c:1522-1534) | pass |
| Tab (9) | AM_Responder: `automapactive=1`, `viewactive=0` (am_map.c:627-632) | Left passes while following; `-` `=` `0` `f` `g` `m` `c` are eaten |
| Return (13) | HU_Responder eats it (message refresh, hu_stuff.c:659-664) | pass |
| Pause (0xff) | G_Responder `sendpause` (g_game.c:561-565); the next command is `BT_SPECIAL\|BTS_PAUSE` = 129, and `G_Ticker` toggles `paused` | pass |
| `q` held with recording on | `G_WriteDemoTiccmd`: `gamekeydown['q']` makes `G_CheckDemoStatus` write the demo and call `I_Error ("Demo %s recorded")` (g_game.c:1508-1509, 1676-1683): `E,1,I_Error,Demo m5bproto.lmp recorded` | run ends |
| `q` tapped inside one tic | not seen (`gamekeydown['q']` already false at G_Ticker) | pass |
| BackSpace, Delete (127), Home, KP_Enter, Super_L, Caps_Lock | none | pass |
| `iddqd` (taps) | ST_Responder: `cheats=2` | the command is unchanged; player state changes outside the command |
| `idkfa` | weapons 0x1ff, cards 0x3f, armour 200 | |
| `idclev12` | `G_DeferedInitNew`; `G_Ticker` reloads: `gamemap=2` | input cleared by G_DoLoadLevel |
| `idbeholds` | P_GivePower(1) called | |
| F10 then `y` | `I_Quit` | |
| F7 then `y` | `D_StartTitle`: `gamestate=3` (GS_DEMOSCREEN), `usergame=0` | |

Two more cases:
- Opening the menu while Up is held leaves Up held. The key-up still reaches `G_Responder`, because `M_Responder`
  returns false for key-ups (ch = -1, m_menu.c:1448-1449).
- In the menu, mouse motion passes to the game while `mousewait` is pending (m_menu.c:1399): forward 40, turn -320
  observed with the menu open.

**Never forwarding these keys.** `BT_SPECIAL` is written only at g_game.c:429 (`sendpause`) and g_game.c:435
(`sendsave`). Everything else ORs in bits 0-5:
- `BT_ATTACK` = 1, `BT_USE` = 2, `BT_CHANGE` = 4 and `i<<3` with i ≤ 7 (g_game.c:330-347);
- the base command is the zeroed `emptycmd` (i_system.c:64-68).

`sendpause` is set only by a `KEY_PAUSE` keydown (g_game.c:563). `sendsave` is set only by `G_SaveGame`
(g_game.c:1267), which only `M_DoSave` calls (m_menu.c:633), and that is reachable only from the menu.

`chatchar` is non-zero only if `HU_queueChatChar` ran. Its callers (hu_stuff.c:669, 681, 715-720, 738) all sit behind
`netgame` or `chat_on`, and `chat_on` is set only under `netgame` (hu_stuff.c:665-681). So in single player
`chatchar` is 0 for every key stream. Observed: 0 in all 41,633 rows.

Withholding `KEY_PAUSE` therefore removes every `BT_SPECIAL`: observed, only the 6 Pause or ÿ rows had it. The
allow-list form of the proof:

- Suppose only the bound keys (`0xac-0xaf`, `,`, `.`, space, `KEY_RCTRL` 0x9d, `KEY_RALT` 0xb8, `KEY_RSHIFT` 0xb6)
  and `1`-`8` are forwarded, plus all mouse events.
- Then `M_Responder` returns false: none of these is Escape, `-`, `=` or an F-key, the menu never opens, and its
  mouse branch yields only arrows, Enter and Backspace, which do nothing while the menu is closed
  (m_menu.c:1519-1616).
- `HU_Responder` returns false (only Enter is eaten in single player; `KEY_RSHIFT` and `KEY_RALT` set flags and
  return false, hu_stuff.c:643-652).
- `ST_Responder` always returns false. No cheat can complete without letters: every sequence begins `id`, and
  `SCRAMBLE` is a byte bijection compared on the key's low byte (m_cheat.c:62).
- `AM_Responder` acts only on Tab while the automap is closed.
- So every forwarded event reaches `G_Responder`'s switch unchanged, and no responder state ever changes.

The deny-list form, which forwards everything else, must withhold at least Escape, F1-F11, F12 (harmless but
eaten), Tab, `-` and `=` (and their keypad forms), Pause, `q` while recording, and the letters that build cheats.
Withholding `i` alone stops every cheat as long as the app never produces a keysym whose low byte is 0x69.
Return is harmless (eaten by the HUD).

#### 2.2.5 Variants and sanitizer findings

The four builds ran all 327 scripts:
- `overflow` (`-fno-wrapv -fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow
  -fno-sanitize-recover=all`) and `address` (`-fsanitize=address,undefined -fno-sanitize=alignment
  -fno-sanitize-recover=all`, `ASAN_OPTIONS=detect_leaks=0`, so leak detection was disabled and this is not
  LeakSanitizer coverage). These are the M3h9c flags.
- `strict32` (`-m32`, genuine text, no adapter; the toolchain is present).

Command: `for v in strict overflow address strict32; do python3 build.py $v; python3 run_all.py $v; done; python3
run_all.py --compare strict overflow address strict32`.

| Comparison with strict | Identical | Differ |
| --- | --- | --- |
| overflow | 327 | 0 |
| address | 327 | 0 |
| strict32 | 325 | 2 |

All four builds exited 0 on every script.

1. **LP64 ABI difference in g_game.c (observed).** `memset (mousebuttons, 0, sizeof(mousebuttons))` and the same for
   `joybuttons` (g_game.c:495-496) take the size of a *pointer* (`boolean*`, g_game.c:187, 204):
   - ILP32 clears 4 bytes, which is `mousebuttons[0]` (fire) only.
   - LP64 clears 8 bytes, which also clears `mousebuttons[1]` (strafe).
   - Observed in `loadlevel_mouse_strafe_x`. Middle button held, Left pressed, `G_DoLoadLevel`, Left pressed again:
     strict gives turn 320, strict32 gives side -24.
   - In the 32-bit original, a held middle or right button survives a level load or reborn (the strafe and forward
     flags stay), and the held fire button is cleared.
   - Declared adapter: `memset (mousebuttons, 0, sizeof(boolean))` and the same for `joybuttons`. This is an
     ILP32-faithful ABI adapter on every contract build, the same kind as the r_data.c ones. The M3h9c-M5a runners
     compile g_game.c LP64 too, but they never post events, so all three `mousebuttons` stay 0 there and the
     difference cannot show.
2. **Left shift of a negative value (diagnostic build only).** `(x - lastmousex) << 2` and `(lastmousey - y) << 2`
   (i_video.c:250-251). The `-std=gnu89` builds report nothing; GCC checks a negative left operand only under C99
   rules. A diagnostic `ubsan_c99` build (`-std=gnu99 -fpermissive -fsanitize=undefined`, with recovery) reported
   15 times across 9 `x_*` scripts (values -5 to -2600), and its rows matched strict on all 327 scripts. The same
   value comes from `* 4`. A declared adapter is needed only if the variants ever move off gnu89.
3. **`short` narrowing of `cmd->angleturn -= mousex*0x8`** (g_game.c:409) and of `forwardmove`/`sidemove +=`. This
   is implementation-defined conversion, not undefined behaviour. GCC has no sanitizer for it, and there is no clang
   on this host. The narrowed values are in §2.2.4. No adapter is needed: GCC converts modulo 2^16.
4. **Signed overflow in `G_Responder`.** `ev->data2*(mouseSensitivity+5)` (g_game.c:579) overflows for `|data2| >
   214748364` at sensitivity 5. A witness script (`witness/ev_mouse_int_overflow.txt`, data2 = 300000000) aborts in
   the overflow build with `g_game.c:579:20: signed integer overflow: 300000000 * 10`. X cannot produce this: its
   coordinates are 16-bit, so `|data2| ≤ 262140`. The app's translation must bound `data2` and `data3`, and Rust
   must refuse anything beyond, never wrap silently.
5. **`st_stuff.c:1292`, index 5 out of bounds for `int[4]`.** Observed only before the `P_SetupLevel` adapter called
   `G_PlayerReborn`, with `readyweapon = wp_fist`. The address build then aborted all 327 scripts at start-up, and
   that was fixed in the driver. It is a genuine status-bar fact, not input: `ST_createWidgets` takes
   `&plyr->ammo[weaponinfo[readyweapon].ammo]` with `am_noammo` = 5 for the fist and the chainsaw, until
   `ST_updateWidgets` replaces it with `&largeammo` (st_stuff.c:926-935). It is reachable when a level starts with
   the fist or chainsaw ready. Relevant to whoever draws the status bar.
6. **Not sanitizer findings, but source facts on this path:**
   - Key events carry indeterminate `data2` and `data3` (i_video.c:197-212); no consumer reads them.
   - `eventhead = (++eventhead)&(MAXEVENTS-1)` (d_main.c:153, 170) modifies an object twice without a sequence
     point. GCC computes the evident value.
   - `G_Responder` bounds `data1` only from above (`< NUMKEYS`, g_game.c:566, 571), so a negative `data1` would
     index `gamekeydown[-n]`. X keysyms are never negative.
   - `mousebuttons[-1]` and `joybuttons[-1]` are by design (`&mousearray[1]`, "allow [-1]", g_game.c:186-187) for a
     -1 button binding, which the defaults never use.

#### 2.2.6 Observed command domain and the wire-24 check

`python3 run_all.py --summary strict`, over 327 scripts and 41,633 tics:

| Field (built) | Min | Max |
| --- | --- | --- |
| `forwardmove` | -50 | 50 |
| `sidemove` | -50 | 50 |
| `angleturn` | -32768 | 23936 |
| `consistancy` | 0 | 0 |
| `chatchar` | 0 | 0 |
| `buttons` | 0 | 129 |

Distinct `buttons`: 0, 1, 2, 4, 12, 20, 28, 31, 36, 44, 52, 60, 129.

Against the wire-23 Commands bullet that wire 24 inherits (docs/COMPATIBILITY.md, line 2405: `BT_SPECIAL` and bit
0x40 refused, forward and side in -127..127, weapon indices 0-7; the wire-24 section adds nothing for commands):
6 rows are outside the domain, all `buttons = 129` from Pause or keysym 0xff. Every other row is admitted. Index 7
(`8`, the chainsaw) is inside 0-7.

**After recording.** Exec `angleturn` takes 11 distinct values, all multiples of 256. `forwardmove`, `sidemove`
and `buttons` are unchanged by the round trip (signed and unsigned bytes, g_game.c:1499-1502, 1510-1514).
`consistancy` and `chatchar` are not recorded: the re-read leaves the built values in place, which are 0 here.

Sample built-to-exec turns:

| Built | Exec |
| --- | --- |
| 320 | 256 |
| 640 | 768 |
| 1280 | 1280 |
| -160 | -256 |
| -128 | 0 |
| +128 | +256 |
| -32760 | -32768 |

**Recording length.** `demo_limit` (Up held for 32,770 tics) recorded tics 0-32760 (32,761 commands). Tic 32761
ended with `E,32761,I_Error,Demo m5bproto.lmp recorded`: `demo_p > demoend - 16` with the default `maxsize =
0x20000` (g_game.c:1538, 1515-1519). That is 936.0 s, or 15 min 36 s at 35 Hz, unless `-maxdemo` is given
(g_game.c:1539-1541).

#### 2.2.7 Focus loss and stuck keys

linuxdoom has no focus handling:
- The window selects no `FocusChangeMask` (i_video.c:798-802).
- `I_GetEvent` handles only key, button, motion, Expose and ConfigureNotify (i_video.c:201-279).
- `G_DoLoadLevel` is the only place that clears `gamekeydown` (g_game.c:489-496).

A KeyPress whose KeyRelease never arrives stays down indefinitely. Observed in `stuck_key_no_release`: Up, Left and
the left button held for 10 tics with no release give forward 25 and fire on every tic, and the turn goes from slow
to fast at tic 6.

Event streams an app could post on focus loss that the original could also have received:

1. **Nothing** (the genuine behaviour): held keys stay held until their release events, if the OS ever delivers them.
2. **A key-up `event_t` for every key whose last forwarded event was a keydown.** That is what the KeyReleases would
   post. Order does not matter for the command, because each clears one flag (g_game.c:570-573). Plus one
   `ev_mouse` with `data1 = data2 = data3 = 0`.
   - `i_video.c` does post exactly that zero mouse event: the ButtonRelease of button 2 or 3 when no other button is
     held (`2 ^ 2`, i_video.c:229-243; trace `P,6,2,0,0,0` in `x_buttons`).
   - It cannot post it for button 1, whose release sets bit 0 (§2.2.4).
   - Observed (`focus_loss_ev`, `focus_loss_keyups`): the next tic is all zero.
3. Option 2 at the X level: KeyRelease and ButtonRelease events through the genuine translation. If the left button
   was held, this leaves fire on (the Button1 bug). Only the event_t form clears everything.

#### 2.2.8 The chain experiment (removes the doubt that the driver's demo is a real demo)

1. `chain/walk.txt` (forward, a slow then fast turn, mouse strafe, fire, the `2` key, use, then `enddemo`) produced
   `m5bproto.lmp` through the genuine `G_BeginRecording`, `G_WriteDemoTiccmd` and `G_CheckDemoStatus`:
   - 222 bytes: a header of `6e 02 01 01 00 00 00 00 00 01 00 00 00`, 52 tics × 4 bytes, and 0x80;
   - SHA-256 `5d88bc141fd70e27…`.
2. It was packed as lump `M5BWALK` and replayed by the M5a strict runner, built with `tools/reference/m5a_build.py
   --variant strict --out /home/copgunn/scratch/m5b/proto-input/m5a-build`, on `tests/reference/m5a_wads/peg.wad` +
   `m5a_gfx.wad`. The summary was `tics 52 end=marker end_tic 52`, `checkheap every-tic-ok`.
3. For all 52 of 52 tics, the replayed `players[0].cmd` (P rows `cmd_forward, cmd_side, cmd_turn, cmd_buttons`)
   equalled the driver's `exec_*` columns.

#### 2.2.9 Design for Phase 2

**New files only**, in the M5a style. Nothing frozen changes.

| Path | Contents |
| --- | --- |
| `tools/reference/m5b_input_driver.c` | the driver |
| `tools/reference/m5b_input_stubs.c` | declared stubs, with the 119 aborting stubs listed explicitly, not generated, so the contract can hash them; the build's `nm` check requires defined == declared |
| `tools/reference/m5b_fakex/X11/{Xlib.h,Xutil.h,keysym.h,extensions/XShm.h}`, `m5b_fakex/errnos.h`, `tools/reference/m5b_fakex.c` | the fake Xlib, declared in the contract as a fake server with its four behaviours |
| `tools/reference/m5b_input_build.py` | imports `m3h9c_build` for `VARIANTS`, `OPTIONAL_VARIANTS`, `apply_once`, `probe_m32` and `sha` |
| `tests/reference/m5b_input_contract.json` | schema `DOOM-M5B-INPUT-v1`: the 23 genuine files with SHA-256, header hashes, adapted hashes per build class, the 15 m_misc.c adapters plus the g_game.c:495-496 memset adapter if the owner takes it, stubs, wraps, flags, driver behaviour, row kinds |
| `tools/reference/m5b_input_scripts.py` | the script generator; its output is committed |
| `tests/reference/m5b_input_scripts/*.txt` | the scripts |
| `tests/reference/m5b_input_goldens/*.csv` | strict T, S and E rows |
| `tests/reference/m5b_input_goldens/*.events` | per tic, the `event_t` stream after the X translation, from the P rows with key `data2`/`data3` masked |
| `tools/reference/m5b_input_reference.py` | `--check` |
| `tests/reference/m5b_input_results.json` | digests and counts |

**What `--check` does:** it builds strict, overflow and address; runs every script; requires the three variants
byte-identical; and compares with the goldens. `--strict32`, run locally, requires identity once the memset adapter
is applied; without it, only the two declared `loadlevel_*` differences are allowed. `--selftest` passes
`--selftest-cmd TIC` to the driver, which flips bit 0 of one built `forwardmove` after `G_BuildTiccmd`. The checker
must report exactly that script and tic.

**Rust consumption.** The app's input code ports `G_Responder`'s game part and `G_BuildTiccmd`. It does not port
the responders the app withholds keys from. A `cargo test` on Linux, Windows and macOS reads the committed scripts
and goldens, with no C involved:

1. For every script, the `.events` stream goes through the Rust event-to-command code tic by tic. The built
   commands, the exec commands (Rust's demo write and re-read) and the turn and double-click state must equal the
   golden. Scripts that need responder behaviour are skipped by a declared list, each with its reason: the `nongame_*`
   rows with menu or automap effects, the cheats, `q` with recording, Pause.
2. If the owner keeps the X translation: the X-level scripts go through a Rust port of `xlatekey` and `I_GetEvent`
   and must reproduce the `.events` file.
3. A mutation self-test: one perturbed event in one script must make the test fail at that tic.

The genuine driver runs only in Linux CI: `m5b_input_reference.py --check`, about 2 s per variant observed here.

**Estimate.** The prototype set is 327 scripts and 41,633 tics. Its strict CSV is 2,313,597 bytes:
- `demo_limit` is 1,693,071 of those;
- the other 326 scripts are 620,526 bytes and 8,872 T rows;
- the 150-script double-click sweep is 451,050 bytes.

For Phase 2, golden rows identical to the row before could run-length-encode as `=,N`, which takes `demo_limit` to
a few rows. Phase 2 could also keep the sweep at its boundary points (about 20 scripts). That gives about 200
scripts, about 10,000 tics and about 0.4 MB of goldens and events together. Scripts are 28,680 bytes today.

### 2.3 Sessions: replaying a recorded session through both engines

**What a session is.** A version-110 demo:
- a header of version, skill, episode, map, deathmatch, respawn, fast, nomonsters, consoleplayer and playeringame[4]
  (g_game.c:1553-1567);
- 4 bytes per tic;
- the 0x80 marker.

It also needs the map WAD's identity. The header already names episode, map and skill, so the session file adds the
WAD's SHA-256 and the app version. The commands in it are the exec commands: the turn is rounded to a multiple of
256, and `consistancy` and `chatchar` are not carried (g_game.c:1510-1514).

**The tool (design).** Proposed as `tools/reference/m5b_session.py` for CI on synthetic maps and
`tools/local/m5b_session_real.py` for IWAD maps, local only.

1. It packs the session's demo as a lump (as `chain/pack.py` does) and runs the genuine whole-engine runner:
   - today, the M5a runner, with `--wad MAP --wad tests/reference/m5a_gfx.wad --wad SESSION --demo LUMP`;
   - once the full-frame runner exists, its fork.

   It runs in the strict, overflow and address variants and requires variant identity, using `m5a_reference.py`'s
   `run`, `ending`, `normalise` and `truncate_stream` logic.
2. It replays the same demo through the Rust engine, via an example in the `m5a_real.rs` mould that reads the demo
   with `parse_demo_episode1_v2` and runs wire 24. It compares every tic:
   - the game dump, through the independent encoder `level_snapshot.py` and `level_compare.py --profile episode1`,
     as the M4a and M4b comparisons do;
   - the 3D-view frame digest stream, with M5a's tracker masking. Full frame and palette come once the full-frame
     runner exists.

   It reuses from `tools/local/m5a_real.py` `run_c`, `parse`, `normalised`, `stream_digest`, `flagged` and the
   `--write`/`--check` record logic (counts and digests only, never IWAD content).
3. It reports the first differing tic. A Rust refusal is a refusal witness with its kind, never a silent stop.
4. Committed for CI: synthetic sessions (`tests/reference/m5b_sessions/*.lmp` plus the WAD identity) and their
   digests. Real-map sessions stay local, and only counts and digests are recorded.

**Phase 2 step 7: proving the app's own input path end to end.**

1. A committed event script (from §2.2.9) drives the app's headless core on a synthetic map. The core is the same
   code the window uses, minus the OS layer.
2. The app writes its session demo.
3. **Byte check:** the input driver runs the same script with `enddemo` and writes the genuine demo. The two demo
   files must be byte-identical. That proves the app's event-to-command-to-demo path equals the genuine one on that
   script, header included. The experiment in §2.2.8 shows the driver produces a demo the genuine engine replays.
4. **Replay check:** the genuine engine and the Rust engine replay the app's demo, and every tic must match (step 2
   of the tool).
5. **Live-equals-recorded check:** the per-tic state the app's core reached while recording must equal the Rust
   replay of its own demo. A recorded session executes the re-read command, as `G_Ticker` does (g_game.c:664-669),
   so there must be no difference.

**What this proves:**
- that the recorded commands are what the genuine input path would build and record for the same `event_t` (or X)
  stream;
- that the simulation and renderer agree with the genuine engine on those commands;
- that the app executes exactly what it recorded.

**What it does not prove:**
- That the app's OS layer (window events, key codes, focus, auto-repeat, timing) produces the event stream a user
  intended. Real OS events, and which tic they land in, are outside every golden.
- Sub-256 turn precision. It is lost by design; vanilla loses it too when recording.
- Interactions the input driver does not simulate. These are covered only when the session replay reaches them:
  - player death and reborn (`G_DoReborn`, then `ga_loadlevel` and `G_DoLoadLevel`, which clears the input,
    g_game.c:489-496);
  - the menu pause, where `P_Ticker` returns early while the command is still built and recorded (p_tick.c:139-144;
    absent from the app);
  - level exit.

#### 2.3.1 Findings of the input prototype that need an owner decision

1. **Mouse translation.** The genuine X path has three quirks:
   - (a) the centre-line drop (i_video.c:257-264), with grab and warp, loses every purely horizontal or vertical
     move;
   - (b) the Button1 bug (i_video.c:218, 231, 247) leaves fire on after a left release and toggles fire when another
     button changes;
   - (c) the first motion jumps from (0,0) (i_video.c:189-190).

   Without `-grabmouse`, vanilla gets no mouse events at all. Options:
   - reproduce i_video.c exactly (grab mode, warp each tic);
   - post `event_t` `ev_mouse` with `data2 = 4·dx` and `data3 = -4·dy` (the i_video.c scaling) and `data1` from
     the true button state. The genuine `G_Responder` accepts any such stream, and i_video.c can produce each such
     event in some state, but not the same sequence for the same physical input;
   - another mapping. If not exact, the X-level goldens become documentation, and the `.events` level is the
     contract.
2. **Taps shorter than a tic are lost**, both press and release in one pump, and so is the first of two motions in a
   tic. Options: keep that (genuine); or have the app's translation layer defer a release to the next tic's pump,
   which the original could also have received if the release had arrived 28 ms later.
3. **X auto-repeat.** A release and press pair split across tics drops the key for one tic and resets `turnheld`.
   Options: forward OS repeats as the X server did; or drop repeats, which gives the same event_t stream as a
   keyboard with repeat off, also receivable by the original.
4. **Withheld keys.** Choose between the allow-list (bound keys + `1`-`8` + mouse; the proof in §2.2.4 is simplest)
   and a deny-list. Either way: Pause (BT_SPECIAL), Escape, F1-F12, Tab, `-` and `=`, `q` while recording, and the
   cheat letters.
5. **The `q` key while recording** ends the session with `I_Error` in vanilla (g_game.c:1508). Options: withhold
   `q`; treat `q` as "end session"; or ignore it.
6. **Recording length.** Vanilla stops at 32,761 tics (15 min 36 s) with `I_Error` at the default 0x20000 buffer.
   Options: emulate that (end the session); emulate `-maxdemo N`; or unbounded. The format has no limit, and readers
   stop at 0x80.
7. **The g_game.c:495-496 memset LP64 ABI adapter** (`sizeof(boolean)`, the ILP32 behaviour). Should the input
   driver's contract declare it, so strict equals strict32? Should the Rust port keep the 32-bit behaviour, where a
   held middle or right button survives a level load?
8. **Focus loss:** do nothing (genuine), or post key-ups for the held keys plus a zero `ev_mouse` (§2.2.7).
9. **Mouse bounds.** The app must bound `data2` and `data3`: genuine overflow begins at |data2| > 214748364
   (g_game.c:579), and the 320×200 confine gives ≤ 1276. Should a larger window keep the `short` wrap of the turn
   (reachable in vanilla only with `-4`), or clamp?

Each finding is taken up in §4: 1 in S4; 2 in S6; 3 in S6; 4 in S5; 5 in S5; 6 in S21; 7 in S11; 8 in S6; 9 in S4.

#### 2.3.2 What the input prototype did not do or observe

- No real X server, OS event loop or keyboard was used. Auto-repeat, warp-generated `MotionNotify` and focus are
  modelled by the fake server as stated, not observed.
- No clang: no MemorySanitizer run (the indeterminate key `data2` and `data3` are a source fact only) and no
  `-fsanitize=implicit-conversion` on the `short` narrowing.
- No fuzzing and no coverage-guided or sanitizer campaign beyond the 327 scripted runs per variant. Address runs had
  leak detection disabled.
- The joystick path (`ev_joystick`, `joyb*`), sensitivities other than 5, a `.doomrc` or `-config`, netgame, chat
  and devparm were not exercised.
- The input driver does not run the simulation. Reborn and level reload were exercised only through the `loadlevel`
  directive, not from a death. The menu pause of p_tick.c:139-144 was not observed.
- The Rust half of the session-replay tool and the full-frame comparison were not prototyped. Only the genuine half
  of the chain was run (one synthetic map, `peg.wad`, one 52-tic session).
- The prototype's stub citations name the vanilla definition's first line. Phase 2 must re-cite each stub's
  reachable range when it writes the contract.
- The independent line-by-line C-facts write-up of the input path is §1.1. This section relies
  only on the lines cited here.

### 2.4 The synthetic graphics set and the new rooms

#### 2.4.1 Lumps the genuine code requests

From the prototype's lump-lookup trace (`W_CacheLumpName` and `W_GetNumForName` wrappers) on M5VIEW,
the status bar and HUD ask for exactly **142 lumps plus PLAYPAL**, all at startup: `STBAR`,
`STARMS`, `STFB0` (only the console player's face background, `STFB%d` with `consoleplayer` 0,
st_stuff.c:1170; `STFB1`-`STFB3` are never loaded), `STTNUM0`-`9`, `STTPRCNT`, `STTMINUS` (by
`STlib_init` from every `ST_Start` through `ST_initData`, st_stuff.c:1276, st_lib.c:60), `STYSNUM0`-`9`, `STGNUM2`-`7`, `STKEYS0`-`5`, the
42 faces (`STFST00`-`STFST42`, `STFTR00`-`STFTR40`, `STFTL00`-`STFTL40`, `STFOUCH0`-`4`,
`STFEVL0`-`4`, `STFKILL0`-`4`, `STFGOD0`, `STFDEAD0`) and the 63 font glyphs `STCFN033`-`STCFN095`
(hu_stuff.c:405-410), and `PLAYPAL` by number (st_stuff.c:1203). All are loaded unconditionally
(`W_CacheLumpName` calls `I_Error` on a missing name), so all 142 must exist even where Episode 1
can never show them (`STFGOD0`, `STKEYS3`-`5`, `STTMINUS`, `STGNUM6`-`7` for the plasma and BFG arms). The
prototype also generated `STFB1`-`STFB3`; Phase 2 should drop them (142 new lumps).

#### 2.4.2 The prototype set and what it showed

`m5b_gfx.py` (scratch) imports `tools/reference/m5a_gfx.py` read-only and appends the new lumps
after M5a's last lump, so every M5a lump keeps its bytes and its lump number; the regenerated M5a
part equals the committed `m5a_gfx.wad` byte for byte (checked), and the combined PWAD is 1,051
lumps, 1,364,887 bytes. **PLAYPAL and COLORMAP are unchanged**: M5a's PLAYPAL already holds 14
pairwise different palettes (checked: 14 distinct; palettes 1-8 blend toward red by k/12 for k
1..8, 9-12 toward yellow by 1..4, 13 toward green by 4), so palette 0, the colour maps and every
M5a golden keep their meaning. The M5a-consistency proof (e) holds with it.

Our own drawings, sizes chosen from the C constants (none taken from id's lumps):

| Lumps | Size, offsets | Why |
| --- | --- | --- |
| `STBAR` | 320x32, opaque, region bands | `ST_WIDTH x ST_HEIGHT` (st_stuff.h:32-33); opaque so BG is fully defined |
| `STARMS` | 40x32 | drawn at `ST_ARMSBGX,Y` 104,168 |
| `STTNUM0`-`9`, `STTPRCNT` | 14x16; `STTNUM1` 10 wide, leftoffset -2; `STTNUM7` topoffset -1 | the ready-ammo number is right-aligned at x 44 with 3 digits of digit 0's width (st_lib.c:97-125), so the width must be <= 14 or `V_CopyRect` fails its range check (`I_Error`, v_video.c:171-183); digit 7 drawn one row below the erase rectangle leaves a row behind when the number changes (genuine) |
| `STTMINUS` | 6x3, topoffset -6 | drawn at x - 8 (st_lib.c:146-147); unreachable in single player (below) |
| `STYSNUM0`-`9` | 4x6 | ammo and max-ammo counters at x 288 and 314, 3 digits |
| `STGNUM2`-`7` | 5x7, leftoffset 1 | the arms multi-icons erase with the old (grey) patch's offsets and size when the yellow `STYSNUM` replaces it (st_lib.c:224-238) |
| `STKEYS0`-`5` | 7x5 (cards), 7x7 (skulls) | key boxes at y 171, 181, 191 (<= 200) |
| `STFB0` (and the unneeded 1-3) | 24x31 | netgame only |
| 42 faces | 24x29, transparent corners; `STFOUCHn` 24x30 topoffset -1; `STFDEAD0` 26x31 leftoffset 1 | at 143,168; the face multi-icon erases the old face's rectangle, so transparent pixels outside it keep the previous face's pixels; the bottom rows encode the face index in binary, the eyes/mouth the kind, the shade the pain level |
| `STCFN033`-`095` | 8 high; letters 9 wide, digits 7, punctuation 3-5; `,` topoffset -1 | the message line is at y 0 (`HU_MSGY`); 9-wide letters make the longest pickup messages reach the `x + w > SCREENWIDTH` break (hu_lib.c:120) |

The prototype's glyphs are binary-coded blocks (distinct, not legible text); Phase 2 should draw a
small legible 5x7-in-9x8 font of our own. None of the awkward sizes produced a range-check skip,
a post outside its lump or an undefined BG read.

#### 2.4.3 What is reachable in Episode 1 single player (source facts)

- **Negative numbers: unreachable.** Player health is clamped at 0 (p_inter.c:871-872), armor and
  ammo never go below 0, and the frags widget is on only in deathmatch (`st_fragson`, st_stuff.c:971, 1062). `STTMINUS`
  is loaded but never drawn.
- **Palettes.** `ST_doPaletteStuff` (st_stuff.c:1000-1050): damage gives `(cnt+7)>>3` capped at 7,
  plus `STARTREDPALS` 1, so a non-zero damage count gives **2..8, never 1**; the bonus count gives
  `(cnt+7)>>3` capped at 3, plus 9, so **10..12, never 9**; the radiation suit gives 13 while
  `powers[pw_ironfeet] > 128` or its bit 8 is set (the blinking end). Reachable: 0, 2-8, 10-12, 13
  (12 of 14). Red 7 and 8 need a damage count >= 41 and >= 49 (one big hit; the count is capped at
  100, p_inter.c:877-878); 12 needs a bonus count >= 17 (two or three pickups within a few tics;
  a keycard sets the count to 6 and adds 6, p_inter.c:281, 658).
- **Berserk is not admitted by wire 24**: doomednum 2023 is not in `EPISODE1_PICKUPS`
  (crates/doom-sim/src/episode1.rs:27-32), and `validate_powers` rejects a non-zero
  `pw_strength` (episode1.rs:345-361). So the berserk fade (st_stuff.c:1010-1016) is out of scope.
- **Faces** (`ST_updateFaceWidget`, st_stuff.c:752-922): pain level `5*(100-health)/101`
  (st_stuff.c:729-745); dead (9); evil grin on a weapon change of `weaponowned` while a bonus is
  shown (8); being attacked by another mobj: ouch if `health - st_oldhealth > 20`, else turn
  right/left or head-on (`KILL`) by the attacker's angle (7); own damage (no attacker): ouch or
  `KILL` (6/7); rampage after 70 tics of `attackdown` (5); god face (4, invulnerability or the god
  cheat: unreachable, no invulnerability in Episode 1 and no cheats); and the look-around:
  `st_randomnumber % 3` every 17 tics (`TICRATE/2`, st_stuff.c:913-917). The **ouch face** needs a
  health *gain* above 20 while the damage count is non-zero (the comparison is `health -
  st_oldhealth`, so it fires on a medikit or soulsphere taken just after a hit, not on a big
  hit; the real maps reach it at pain levels 0 and 3). 41 of 42 faces are reachable; `STFGOD0`
  is not.
- **Messages** (HU_Ticker, hu_stuff.c:505-530; 140-tic timeout `HU_MSGTIMEOUT`): the Episode 1
  pickups (M4a codes 1-11, 17-20, 22-25, 28-30, 32-34, 36), the three locked-door strings (38-40)
  and the turbo message (41, a demo `forwardmove > 50` on a `gametic` multiple of 128,
  g_game.c:671-679, not netgame-gated). A second message, even the same one, replaces the line and
  restarts the 140 tics.

#### 2.4.4 New rooms and scripts (estimate)

Each room is a `m5a_wad.py`-style `Plan` with one script; all at skill 2 or 3 as wire 24 admits.

| Room | Exercises | Tics (est.) |
| --- | --- | ---: |
| STDIGIT | ready ammo through 1, 2 and 3 digits and back to 0 (fire the pistol dry: the `num == 0` branch, st_lib.c:134-135), 200 bullets, health and armor 1-200 (bonuses, armor, soulsphere), the fist/chainsaw `1994` blank | 900 |
| STARMS | shotgun, chaingun, launcher, chainsaw pickups (arms multi-icons and the evil grin), rockets and shells, the backpack (all four max-ammo numbers), the three keycards | 700 |
| FACEDIR | a monster hitting from the left, right and ahead (`R_PointToAngle2` against the player's angle, ANG45 and ANG180 boundaries) at each pain level 0-4, taken down by nukage between rounds (own-damage `KILL`) | 1,400 |
| FACEOUCH | a medikit and a soulsphere taken while the damage count is non-zero (ouch at two pain levels), the rampage (fire held 70+ tics), death and the dead face to the end | 600 |
| FACELOOK | standing still for 300+ tics at each of three pain levels: the look-around sequence and its M_Random draws (and, if FH1 is decided for vanilla accounting, sounds near and far) | 1,000 |
| PALRED | damage counts reaching red 2-8 (an exploding barrel next to the player for 7-8) | 500 |
| PALBONUS | bonus 10-12 (three pickups in a row; a keycard plus an item) | 300 |
| PALSUIT | the radiation suit to its end, the blink of palette 13 (`IRONTICS` 2,100 tics) | 2,300 |
| MESSAGE | every Episode 1 pickup kind once, the same message twice within 140 tics, a message replaced early, a timeout, the three locked doors, the turbo message | 1,500 |
| SOUND (only if FH1 chooses vanilla accounting) | audible and inaudible sounds, `sfx_itemup`/`sfx_tink` (no draw), the saw's other draw, the linked chaingun sound, map 8 (no clipping distance) | 800 |

About 9-10 rooms and 10,000 tics, so about 10,000 frames per configuration more than today's
13,391. Goldens: an F row (about 160 bytes) and an S-like status row (about 40 bytes) per frame
at screenblocks 10 and 11: (13,391 + 10,000) x 2 x 200 bytes, about 9.4 MB of text, against
4.3 MB of `tests/reference/m5a_frames` today; storing one combined row per frame and dropping
`view_sha256` keeps it nearer 8 MB. CI on Linux: the M5a job already renders every item at three
configurations in three variants; M5b adds two configurations in three variants on about 1.7x the
frames, so roughly the M5a job's runner time again (not measured; the prototype's 48-item proof
pass also ran the frozen, M5a, sound and determinism builds and is not a CI estimate).

#### 2.4.5 Phase 2 files

- `tools/reference/m5b_driver.c` (fork of `m5a_driver.c`, which stays frozen; the
  `m5b_status.inc` block folded in), `tools/reference/m5b_stubs.c` (fork of `m5a_stubs.c`),
  `tools/reference/m5b_build.py` (imports `m5a_build` as a library; contract
  `tests/reference/m5b_runner_contract.json`, schema `DOOM-M5B-RUNNER-v1`, with the 46 whole files,
  the 53 stubs, the 30 wrapped symbols and the two new dump declarations as `driver_behaviour`
  entries next to R5's).
- `tools/reference/m5b_gfx.py` (imports `m5a_gfx`, appends the 142 lumps; writes
  `tests/reference/m5b_gfx.wad`) and an independent `tools/reference/m5b_check_gfx.py` that
  re-parses it without importing the generator and checks: the M5a part byte-identical to
  `m5a_gfx.wad`, the 142 names present, `STBAR` 320x32 with every column one opaque post, each
  widget's patches fit the screen at its C position after offsets (the ammo digit width <= 14,
  faces within 143..319 x 168..199, glyphs at y 0 with `topoffset <= 0`), no post outside its lump.
- `tools/reference/m5b_wad.py` (rooms and scripts in `tests/reference/m5b_wads/`,
  `tests/reference/m5b_scripts/`, manifest `tests/reference/m5b_manifest.json`).
- `tools/reference/m5b_reference.py`: the M5a items plus the new rooms at screenblocks 10 and 11
  in three variants; goldens `tests/reference/m5b_frames/<LUMP>.sb<N>.frames` (schema
  `DOOM-M5B-FRAMES-v1`: M5a's F rows over the whole frame, plus one P row per frame
  `tic,palette,palette_set,face,message_on,rndindex`), `tests/reference/m5b_results.json`, a few
  PNGs per new room drawn through the frame's own palette (`tests/reference/m5b_png/`), and the
  M5a-consistency check of proof (e) as a standing assertion. Frame-stream change: the palette
  row is new; the F row keeps its columns, its digest now covers the status bar and HUD.
- `tools/local/m5b_real.py` for the 61 real runs (as `m5a_real.py`, counts only).

### 2.5 Findings of the full-frame prototype for the owner

**FH1. The face's look-around depends on M_Random draws the runner does not make.**
`st_randomnumber = M_Random ()` (st_stuff.c:992) is read only by the look-around
(`st_randomnumber % 3`, st_stuff.c:915), but in vanilla `rndindex` is also advanced by:
`S_StartSoundAtVolume`, one draw per started sound that passes its link-volume check and, for a
sound with another origin than the player, the audibility test (`S_AdjustSoundParams`, distance
and `snd_SfxVolume`; no distance limit on map 8), except `sfx_itemup` and `sfx_tink`
(s_sound.c:283-343); and `wipe_initMelt`'s 320 draws in the first display (f_wipe.c:159-163; the
level-start wipe of d_main.c:223-229, 324-353). So the vanilla look-around also depends on the
configured sound-effects volume (`sfx_volume`, default 8, m_misc.c:237) and is the same at any
other volume only for the player's own and origin-less sounds. Measured with an experimental build
that compiles `s_sound.c` whole (i_sound.c stubbed, `numChannels` 3 and volume 8 as the m_misc.c
defaults, `S_UpdateSounds` before each display as d_main.c:392) and adds the 320 wipe draws: the
dumps stay identical (48 of 48), and the face differs from the ST-draws-only prototype in
**8,517 of 13,391 synthetic frames** (45 of 48 items; sounds alone 5,882 frames, 262 sound
draws; the wipe alone 7,931 frames) and in **69,139 of 158,966 real-map frames** (all 61 runs; 20,697 sound draws, 4,670 of them on E1M8, where no sound is too far away; DEMO1 alone 542 of 1,710 frames). Options: (a) **vanilla accounting**: count the wipe's 320 draws after the first display and one
draw per qualifying sound start (the harness compiles `s_sound.c` whole over a stubbed i_sound
layer, with nine music lumps `D_E1M1`-`D_E1M9` added to the synthetic PWAD because `S_Start`
changes the music and `S_ChangeMusic` looks the lump up by name, s_sound.c:202-248, 674-675; the Rust side then needs, for every sound start, its sfx id and
origin position and the listener's position at the moment of the call, which the simulation does
not expose today, and must fix `sfx_volume` at 8 as part of the reference configuration);
(b) **status-bar draws only**, a declared deviation like R5 (the look direction then matches no
shipped configuration; every other pixel and the palette are unaffected); (c) **compare the face
only outside the look-around state** (priority 0, st_stuff.c:913-917), declaring its look
direction non-comparable. The prototype's default, and every count in §2.1.6 and §2.1.8 other
than this finding's, is option (b).

**FH2. Display cadence.** The status bar's multi-icon erase uses the last drawn patch (§2.1.7), so
the frames are those of one display per tic (`-timedemo`/`singletics`). Recommendation: state it
as the reference configuration (consistent with M5a's R8), and have Rust run the status-bar
drawer every tic even if a front end skips presenting a frame.

**FH3. Which palettes and faces to require.** Palettes 1 and 9 and the god face are unreachable by
the genuine code in Episode 1 single player (§2.4.3), and berserk is not admitted. Recommendation:
the M5b goal lists 12 palettes and 41 faces, not 14 and 42.

**FH4. Dump remedies.** Option 1 of §2.1.2 (record the message HU_Ticker found; print `rndindex`
without presentation draws; carry both genuine values in the frame stream). The alternatives and
what they prove are listed there.

**FH5. 2D undefined reads.** Refuse a frame with an undefined status-bar or HUD read as a whole
rather than masking whole screen columns (§2.1.9); observed count 0.

Each finding is taken up in §4: FH1 in S15; FH2 in S8; FH3 in S23; FH4 in S16; FH5 in S17.

### 2.6 What the full-frame prototype did not do or observe

- No Rust code, no committed tool, contract, golden or room: Phase 2.
- No new room was built; the synthetic counts are for the existing 48 items only (so ouch faces,
  palettes 7, 8, 12, 13, keys, the backpack and most messages were not exercised synthetically).
- The prototype's font is binary-coded, not legible; `STFB1`-`STFB3` were generated needlessly.
- strict32 (`-m32`) was not built for the prototype.
- No sanitizer campaign beyond the address and overflow runs described; leak detection was
  disabled in every address run.
- Screenblocks 9 and below, low detail, automap, menus, cheats, wipes as pictures and sound output
  were not run (out of scope); `R_FillBackScreen` and the border code were only shown to return
  at screenblocks 10 and 11.
- The vanilla M_Random accounting (FH1) was measured only in the strict build of the experiment,
  with the face index compared, not as a full variant-identity proof; its dumps were compared on
  the synthetic items only (the real-map experiment runs wrote no dumps).
- The FH1 experiment's sound-start stub layer (i_sound.c) and the 4-byte music lumps are scratch
  only and were not reviewed against i_sound.c beyond `I_SoundIsPlaying`; they affect channel
  bookkeeping only, which comes after the draws (s_sound.c:348-355).
- No timing or CI duration was measured.

### 2.7 Corrections found in Phase 2

Each item was checked against the C source and the committed goldens.

- **Faces.** 40 of 42 faces are reachable under wire 24, not 41. The ouch face at pain level 4
  (index 37 = 4 × `ST_FACESTRIDE` + `ST_OUCHOFFSET`, st_stuff.c:97-114) needs
  `plyr->health - st_oldhealth > ST_MUCHPAIN` (20) with a health of 19 or less (st_stuff.c:735-741,
  807, 860). That cannot happen: `st_oldhealth` is never below -1 (st_stuff.c:994, 1268) and a
  player's health never below 0 (p_inter.c:870-872). The god face (index 40, `ST_GODFACE`, st_stuff.c:114) appears
  only in the bespoke cheat scenario, so the C goldens show 41 of 42 faces (`coverage` in
  `tests/reference/m5b_results.json`). S23 said 41 under wire 24.
- **Message code 6 (`GOTMEDINEED`) is unreachable.** A medikit is taken only by a living player
  (p_inter.c:363) whose health is below 100 (p_inter.c:233-234), and it adds 25 (p_inter.c:477), so
  the `player->health < 25` test (p_inter.c:480) is always false.
- **The radiation-suit palette condition** is `powers[pw_ironfeet] > 4*32 || powers[pw_ironfeet] & 8`
  (st_stuff.c:1039-1040): bit 3, not bit 8.
- **A space advances 4 pixels** (hu_lib.c:125-128). The synthetic font's letters are therefore 10
  wide, so that a reachable message (the blue and yellow locked-door strings) reaches the
  `x + w > SCREENWIDTH` cut-off of `HUlib_drawTextLine` (hu_lib.c:120); see `tools/reference/m5b_gfx.py`.
- **The new rooms** total 7,785 tics (the note estimated about 10,000), and the goldens in
  `tests/reference/m5b_frames` are 5.1 MB.
- **Input (step 2).**
  - Typing `iddqd` one key per tic while recording ends the demo, because `q` is still held
    when the next `G_Ticker` writes the command (g_game.c:1508-1509).
  - A built turn of +32640 or +32760, at the positive edge of a short, executes as -32768 after the
    demo re-read: `(angleturn+128)>>8` is written as the byte 0x80 (g_game.c:1512) and read back as
    `0x80<<8` (g_game.c:1501). This is beyond the app's clamp (`beyond_app_mouse_short_edge`).
  - A second press lasting only 2 tics after a 2-tic gap gives no double-click use
    (`dclick_fwd_inside`): the release that ends the first press restarts `dclicktime` at 0, and a
    transition registers only while `dclicktime > 1` (g_game.c:355, 366, 370).
  - `G_DoLoadLevel` leaves the double-click state as it was (g_game.c:489-496 clear only the keys,
    the mouse motion and the buttons).
  - The script set is 251 scripts and 36,453 tics (the note estimated more scripts for the
    double-click sweep).
- **The window library does report focus loss.** Phase 1 (S2, S6) said miniquad has no focus-lost event and that a key held across Alt-Tab stays held. The library reports it through its
  minimise event, and `doom-play` turns that event into a pause that releases every held key. Observed under WSLg on 2026-11-17: a second window taking the focus paused the game on X11 and on Wayland (not checked: Alt-Tab, minimising by hand, Windows, macOS).
- **The two skill numbers.** `--skill 2` and `--skill 3` are the demo-header values (the value stored in the demo and
  in the file name). Vanilla's own `-skill` option counts from 1, so these are vanilla's `-skill 3` (Hurt Me Plenty) and
  `-skill 4` (Ultra-Violence). The wording of S1's out-of-scope skills ("skills 1, 4 and 5") mixed the two numberings: the
  skills meant are baby, easy and nightmare, which are 0, 1 and 4 in demo-header numbering and 1, 2 and 5 in vanilla's
  `-skill`, while the admitted skills are 2 and 3 in the header and 3 and 4 in `-skill`.
- **Input (step 4).** The Rust event layer applies the S6 same-pump deferral to mouse buttons as
  well as keys, posts a mouse event only when there is motion or a button change, and `load_level`
  clears only `mousebuttons[0]`, as the declared 32-bit `sizeof` adapter of the genuine input driver
  does (g_game.c:187, 495).

## 3. Rust design

### 3.1 Summary

| Piece | Home | Platform types? |
| --- | --- | --- |
| Status bar (`st_stuff`, `st_lib`), heads-up text (`hu_stuff`, `hu_lib`), patch drawing (`v_video`), palette choice, frame composition (`d_display`), their saveable state | **doom-render**, new modules | no |
| Input (`G_Responder`, `G_BuildTiccmd`), session recorder, game loop with an injectable clock, app screens and their font, the `Platform` and `Clock` traits | new library crate **doom-game** | no |
| Window, native events to `doom_game::Event`, presenting a frame and palette, the real clock, command-line arguments | new binary crate **doom-play** | yes (the only crate) |

The dependency direction keeps the simulation unreachable from presentation, and the window
unreachable from every library:

```
doom-types <- doom-wad <- doom-map <- doom-sim <- doom-render <- doom-game <- doom-play (bin, windowing dependency)
```

doom-game depends on doom-types, doom-wad, doom-map, doom-sim and doom-render. Only doom-play
depends on the windowing library. Nothing depends on doom-game except doom-play and the fuzz
workspace. `unsafe_code = "forbid"` stays inherited by all three through `[lints] workspace = true`
(Cargo.toml:16-18).

### 3.2 Why the status bar and HUD go into doom-render, not a sibling crate

The owner allowed either. I recommend new modules inside doom-render, for four reasons.

1. **It is the same kind of code.** doom-render is already "one module per C file, named for
   it" (ARCHITECTURE "Software renderer (M5a)"). st_stuff.c, st_lib.c, hu_stuff.c, hu_lib.c and
   the patch half of v_video.c are presentation C files of the same engine.
2. **Composition needs the renderer's private state.** The status bar and HUD draw into the
   same persistent `screens[0]` that the 3D view draws into (item 11 below). doom-render keeps
   that buffer inside `RenderState::screen` (crates/doom-render/src/state.rs:53-54) and copies
   it in and out in `render` (crates/doom-render/src/lib.rs:525, 581). A sibling crate would need
   new public API to read and replace it.
3. **The resources are private.** The status-bar and HUD graphics come from the same global lump
   list, `Resources::dir`, which is `pub(crate)` (crates/doom-render/src/r_data.rs:375). Only
   palette 0 of PLAYPAL is kept (:379), while the palette choice needs all 14.
4. **Tooling already covers it.** The mutation workflow already mutates `-p doom-render`
   (.github/workflows/mutants.yml:84-88), and the dev profile already optimises doom-render
   (Cargo.toml:20-24), which matters for status-bar frame tests in debug builds.

A sibling crate would work too, but it needs the additive public API listed in §3.5 regardless
of where the code lives.

### 3.3 New modules in doom-render

| Module | C | Contents |
| --- | --- | --- |
| `v_video` | v_video.c:158-240 | `V_DrawPatch` (a patch outside the screen is silently skipped, as the RANGECHECK branch does with "bad patch (ignored)", v_video.c:221-231) and `V_CopyRect` (an out-of-range rectangle is `I_Error` in the C, v_video.c:171-182, so it is a typed refusal here). A patch post that leaves its lump is refused, as in M5a's R4 handling. Drawing targets the persistent `screens[0]` and the status-bar background buffer `screens[4]` |
| `st_lib` | st_lib.c | number, percent, multi-icon and binary-icon widgets with their `oldnum`/`oldinum`/`oldval` |
| `st_stuff` | st_stuff.c | `StatusBar`: `ST_loadGraphics`/`STlib_init` lumps, `ST_initData`, `ST_Ticker` (`st_clock`, `M_Random`, `ST_updateWidgets`, `ST_updateFaceWidget`, `st_oldhealth`), `ST_Drawer` (`ST_doPaletteStuff`, refresh or diff draw). No `ST_Responder` cheats (§1.6 item 2) |
| `hu_lib` | hu_lib.c | text line and scrolling text (`HUlib_addMessageToSText`, `HUlib_drawSText`, erase with its `viewwindowx` condition, hu_lib.c `HUlib_eraseTextLine`) |
| `hu_stuff` | hu_stuff.c | `Hud`: font `STCFN033`..`STCFN095` (hu_stuff.c:405-408; hu_stuff.h:30-34), `HU_Start`, `HU_Ticker` fed by the item-5 message derivation, `HU_Drawer`, `HU_Erase`. The title line (automap only) is loaded and not drawn. Chat is not implemented (netgame only) |
| `d_display` | d_main.c:193-320 | `Display`, the analogue of `D_Display`: owns a `Renderer`, a `StatusBar`, a `Hud` and the D_Display statics (`fullscreen`, `oldgamestate`, `viewactivestate`, `borderdrawcount`), and composes the frame in the C's order |
| `m_random` | m_random.c | none needed: `doom_types::DoomRng::m_random` already reproduces `M_Random` on an owned index (crates/doom-types/src/rng.rs:7-35). The display owns its own `DoomRng` and never touches the simulation's |

### 3.4 doom-game: the platform-free app core

All of doom-game is headless and testable, and its input half is fuzzable.

**Events: `Event`, the analogue of `event_t`** (d_event.h):

```rust
pub enum Event {
    KeyDown(u8),                                  // ev_keydown, data1 = DOOM key code (doomdef.h KEY_*, lower-case ASCII)
    KeyUp(u8),                                    // ev_keyup
    Mouse { buttons: u8, dx: i32, dy: i32 },      // ev_mouse: data1 bits 0-2, data2, data3
    Joystick { buttons: u8, x: i32, y: i32 },     // ev_joystick
}
```

Key codes are DOOM's own, not the platform's. The shim translates native keys and scales mouse
motion as the C's i_video.c does: the X11 port shifts motion deltas left by 2. §1.1.7
owns the exact citations.

**`input` module.** This is `G_Responder`'s game part and `G_BuildTiccmd` (g_game.c:200-425,
555-600) over an owned `InputState`:

- `gamekeydown[NUMKEYS]`, `mousebuttons[3]`, `joybuttons[4]`;
- `mousex`/`mousey`. Note that `ev_mouse` *assigns* them (g_game.c:579-580), so only the last
  mouse event before a tic counts, and `G_BuildTiccmd` zeroes them (g_game.c:411);
- `turnheld`, `dclicktime`, `dclickstate`, `dclicks` and the strafe double-click trio;
- `sendpause`.

Its input is a key-binding table with the C defaults (m_misc.c defaults: §1.1.6)
and `mouseSensitivity`. Its outputs:

```rust
pub fn respond(&mut self, ev: &Event) -> bool;                // G_Responder's game half
pub fn build_ticcmd(&mut self) -> TicCommand;                 // G_BuildTiccmd, raw (low byte may be nonzero)
pub fn record_round(cmd: TicCommand) -> TicCommand;           // G_WriteDemoTiccmd + G_ReadDemoTiccmd
```

`build_ticcmd`'s output goes through `record_round` before `advance`, always (§1.6 item 7).
`BT_SPECIAL` is never produced:

- **Pause.** `KEY_PAUSE` becomes an app pause that stops ticking, outside the demo. In vanilla,
  a paused game still runs `ST_Ticker` and `HU_Ticker` every tic, because only `P_Ticker`
  returns early, but a pause ticcmd is not admissible under wire 24. So the app pause freezes
  everything. The replayed demo then reproduces the frames of an unpaused run. This is
  presentation and is stated as such in COMPATIBILITY.
- **Save.** The save key does nothing.
- **Cheats, menu and chat.** Cheat sequences, menu keys and chat keys are not routed to any
  responder that changes game state.

The module is pure: `no_std`-friendly logic, no platform types, no I/O. It is a fuzz target (a
random event and tic stream must always yield a command that `record_round` maps into the wire-24
domain), and mutation-tested.

**`Clock` and `Platform`.**

```rust
pub trait Clock { fn now_tics(&mut self) -> u64; }            // I_GetTime: tics since start at 35 Hz
pub trait Platform {
    fn poll_events(&mut self, out: &mut Vec<Event>);          // I_StartTic
    fn present(&mut self, frame: &[u8], palette: &[u8; 768]); // I_SetPalette + I_FinishUpdate (64,000 indices)
    fn wait(&mut self);                                       // yield until the next tic may be due
    fn quit_requested(&self) -> bool;
}
```

**`session` module: the loop.** `Session::run_once(&mut self, platform, clock)` is one iteration
of the `D_DoomLoop`/`TryRunTics` analogue:

1. Poll events. The app keys go first (Esc opens the app's own "resume / restart / quit" screen,
   which pauses). Then `respond`.
2. Work out the due tics from the clock, bounded per iteration like `NetUpdate`'s maketic
   limit; §1.3.2 gives the exact bound.
3. For each due tic: `build_ticcmd`, then `record_round`, then `advance`. On `Ok`, call
   `Display::observe(sim, &fed_command)` (the renderer's observe plus `ST_Ticker` and
   `HU_Ticker`, §3.7) and append the command to the recorder. On `Err`, change state (below).
4. Draw one frame with `Display::draw` and present it with its palette.

`Session` is a small state machine:

| State | Entered by |
| --- | --- |
| `Playing` | level start, or a restart |
| `Paused` | the pause key, or Esc |
| `Ended { kind: Exit / SecretExit / EpisodeEnd }` | `LevelExited` |
| `Died` | `PlayerReborn` |
| `Refused { tic, what }` | any other `SimError`, `RenderRefusal`, `ObserveError` or display refusal |

Every end state writes the demo and shows the app's screen with "R restart, Q quit". Restart
clones the tic-0 simulation and resets the display and recorder (§1.6 item 8). With a fake
`Clock` and a fake `Platform` that scripts events and captures frames, the whole loop runs in
`cargo test` with no window.

**`screens` module: the app's own screens.** These must be *visibly not vanilla* and must not use
any IWAD graphics. A plain screen in a fixed palette of its own (for example a dark blue
background with white and yellow text, unlike any PLAYPAL palette). Its text uses a small bitmap
font **written for this project** (printable ASCII, 8x8, a few hundred bytes as a `const` array
in doom-game, produced or checked by a committed script). Not STCFN, so it cannot be mistaken for
the game's messages. Not copied from any third-party font file, to keep the licence position
trivial. The screens are composed into the same 320x200 indexed frame and come with their own
768-byte palette, so `Platform::present` needs no special case.

**`recorder` module.** See §3.6.

**What stays out of every library crate.** The windowing library, OS input APIs, file dialogs,
audio, threads and wall-clock reads. The only clock is a `Clock`. Demo files are written through
a small `std::fs` call in doom-play, or through an injectable writer in doom-game. Recommendation:
doom-game returns the demo bytes and a suggested name, and doom-play writes them.

### 3.5 Additive public API in doom-render

Nothing existing changes signature or behaviour. This is an additive list (inference about final
names):

- `pub struct Display` (module `d_display`):
  - `new(Arc<Resources>, Arc<Level>, Arc<HudResources>, DisplayConfig)`;
  - `begin_level(&Simulation)`;
  - `observe(&Simulation, &TicCommand) -> Result<(), DisplayObserveError>`;
  - `draw(&Simulation, &mut Frame) -> Result<DisplayFrame, DisplayRefusal>`;
  - `state() -> &DisplayState` and `restore(DisplayState)`;
  - `set_screenblocks(10 | 11)`, the analogue of `R_SetViewSize`/`setsizeneeded`.

  `DisplayFrame { palette: u8 (0..=13), report: FrameReport, flash: ... }` carries the 3D view's
  `FrameReport` unchanged.
- `pub struct HudResources`, built by `HudResources::load(&Resources)` (item 12).
- A `Resources` accessor for the full PLAYPAL, `playpal(&self) -> Option<&[u8]>` (14 x 768
  bytes, or a refusal when it is shorter). The `palette` field stays as it is.
- `Renderer` is untouched for M5a callers. `Display` uses crate-internal access to draw the view
  over the composed persistent screen and to store the composed screen back (§3.8).

### 3.6 The session recorder

- **What is recorded.** Every command passed to `advance`, after `record_round`, in order. That
  is player 1's ticcmd as `G_WriteDemoTiccmd` writes it.
- **The writer.** `doom_sim::demo::write_demo_episode1_v2(map, skill, &commands)`
  (crates/doom-sim/src/demo.rs:255-279) refuses `forward` or `side` of -128, buttons 0xc0 and a
  nonzero low byte (demo.rs:281-305). None of these can reach it after `record_round` and the
  `MAXPLMOVE` clamp. A refusal there is a bug, and the recorder surfaces it as a refusal.
- **Which commands end a demo.**
  - Committed tics are written.
  - **A refused tic's command is written too, for a witness refusal.** Replaying the demo through
    `player_level_run --policy episode1v2` then reproduces the refusal at the same tic, which
    turns every witness the owner meets into a bug report.
  - For `LevelExited` and `PlayerReborn` the refused tic did no work and its command is omitted.
    The demo then ends at the exit or reborn tic, where vanilla would continue into the
    intermission or the reloaded level.
- **Streaming.** doom-sim has no streaming writer, and editing doom-sim is out of scope. The
  recorder keeps `Vec<TicCommand>` (4 bytes a tic, about 0.5 MB an hour) and rewrites the whole
  file atomically (write to a temporary name, then rename) at every end state, at quit, and
  periodically (for example every 35 x 30 tics), so a crash loses at most that much. Rewriting
  is linear in the session length, which is negligible at these sizes (inference, not measured).
- **Vanilla's demo-buffer limit is not reproduced.** Vanilla stops recording and quits through
  `I_Error("Demo %s recorded")` when its buffer fills: 128 KB by default, about 32,000 tics or
  15 minutes (g_game.c:1513-1518 and `G_RecordDemo`'s `maxsize`). It also does so on the `q`
  key (g_game.c:1508-1509). Both are app behaviour, not frame behaviour. The app records without
  that limit and does not bind `q`, and says so.
- **Every session demo is a whole-engine test case.** It is a valid version-110 lump for the
  same IWAD, map and skill, so the genuine runner can replay it and compare it tic for tic
  (DSIM40) and frame for frame. That is the strongest evidence route for "the frame is
  pixel-identical", and it comes free with the recorder.

### 3.7 The render state: saving and restoring the status bar and HUD history

**What must be saved.** Everything the C carries between tics or frames that is not in canonical
state:

- **Status bar.**
  - The tic counter and `M_Random` index: `st_clock`, `rndindex`, and the last
    `st_randomnumber`, which is read only in the tic that draws it.
  - The face: `st_oldhealth`, `oldweaponsowned[9]`, `st_facecount`, `st_faceindex`, face
    `priority`, `lastattackdown`, and the `ST_calcPainOffset` cache (derivable, but cheap to
    keep).
  - The widget inputs: `keyboxes[3]`, `st_msgcounter`, `st_chat`, `st_oldchat` (inert in single
    player but stateful), `st_palette`, `st_firsttime`, `st_statusbaron`, `st_armson` and the
    `w_ready` source (an ammo index, or `largeammo` 1994).
  - Every widget's old value: `w_ready`, `w_health`, `w_armor`, `w_ammo[4]` and `w_maxammo[4]`
    `oldnum`; `w_arms[6]`, `w_faces` and `w_keyboxes[3]` `oldinum`; `w_armsbg` `oldval`.
  - Whether the background buffer `screens[4]` has been drawn. Once drawn it is a pure function
    of STBAR (plus the face background in netgame only), so a flag suffices. Before it is drawn
    the C buffer is uninitialised `Z_Malloc` memory, but no widget draws while
    `st_statusbaron` is false (st_lib's `*n->on` tests), so nothing reads it (inference from
    st_lib.c's guards).
- **HUD.** `message_on`, `message_counter`, `message_nottobefuckedwith` and
  `message_dontfuckwithme`. The message widget's lines (one line, at most 80 characters, its
  current-line index and `needsupdate`). The previous stored message code, used by the item-5
  cross-check.
- **D_Display statics.** `fullscreen`, `oldgamestate`, `viewactivestate` and
  `borderdrawcount`. The current screenblocks.
- **The composed persistent `screens[0]`** (next paragraph).

**Schema choice.** The rule "keep every historical canonical schema unchanged; new behaviour
requires an explicit schema choice" means:

- DRST version 2's layout stays as it is, along with its decoder (which refuses every other
  version, crates/doom-render/src/state.rs:344-352) and its tests (crates/doom-render/tests/rewind.rs,
  which encode and decode `RenderState` at tic N-k).
- They keep their exact meaning for `Renderer`-only users.

Two clean options:

- **Option A (recommended): a new self-contained encoding with its own magic**, for example
  `DDSP` version 1 ("display state"). It holds the tic, the **unchanged DRST v2 bytes**
  length-prefixed and embedded, then the status-bar, HUD and D_Display block. One file restores
  atomically, the DRST v2 decoder decodes the embedded bytes unmodified, and the DRST v2 tests
  stay byte-identical. The STATE_FORMAT section for DDSP states that inside a DDSP the embedded
  DRST `screen` is the *composed* `screens[0]`, with status bar and HUD pixels. That is the
  C's own meaning of `screens[0]`, but different values from a renderer-only run, which is why
  it needs its own schema and must not be a silent reuse.
- **Option B: DRST version 3** = v2 fields plus the new block, with a new `decode_v3`, and the
  v2 decoder kept exactly (still refusing 3). Equally valid, but it mixes two crates' worth of
  meaning into one magic and invites editing `RenderState`'s public struct.

**The `observe` contract, extended.** `Renderer::observe` must see every committed tic in order
from `begin_level`, and a gap is refused (crates/doom-render/src/state.rs:209-218). `ST_Ticker`
and `HU_Ticker` also run once per tic whether a frame is drawn or not. So `Display::observe(sim, fed_command)`:

1. checks `sim.tic() == state.tic + 1`;
2. computes the next renderer state, the next status-bar state (`ST_Ticker`: one `M_Random`
   draw, the face and the widget inputs) and the next HUD state (`HU_Ticker` with the item-5
   derivation and its cross-check);
3. commits all three only if all succeed, so a refusal leaves everything unchanged.

`Display::draw` runs per displayed frame (`ST_Drawer`, `R_RenderPlayerView`, `HU_Drawer`) and
advances only frame history: the widget old values, `st_palette`, `st_firsttime`, the composed
screen, `fuzzpos`, `ML_MAPPED`, the wall steps and the taint.

**Restore at tic N-k.** Decode the DDSP, check it against the level (the DRST v2 checks plus the
new block's bounds), clone or rebuild the simulation at N-k, then observe and draw N-k+1..N with
the recorded commands. The existing rewind test generalises directly, with a new test file and
the old one untouched.

### 3.8 The frame contract with the status bar and HUD

M5a's `Frame` is 320x200 palette indices (crates/doom-render/src/lib.rs:161-187). The renderer
never draws below the view at screenblocks 10 or outside it at all (COMPATIBILITY "Software
renderer (M5a)", R1 and R2). The C order per displayed frame (d_main.c:233-264) is:

1. `HU_Erase()`. At screenblocks 10 and 11 `viewwindowx` is 0, so it erases nothing
   (hu_lib.c `HUlib_eraseTextLine`'s `viewwindowx` condition). Implement the condition anyway.
2. `ST_Drawer(viewheight == 200, redrawsbar)`:
   - `redrawsbar` is true on a wipe frame, or on a change from full screen back to a reduced
     view (d_main.c:242-245);
   - it always runs `ST_doPaletteStuff`, so the palette is chosen at screenblocks 11 too;
   - at screenblocks 10 it redraws the status bar (refresh) or only its changed widgets (diff)
     into rows 168-199 (`ST_Y`, st_stuff.h:32-34);
   - at 11 `st_statusbaron` is false and nothing is drawn.
3. `R_RenderPlayerView`: the M5a renderer, drawing **over the current composed `screens[0]`**.
4. `HU_Drawer()`: the message line at y 0 (`HU_MSGY`, hu_stuff.h:40) over the view.

**Why the view must start from the composed screen.** `screens[0]` is never cleared, so pixels
the view leaves uncovered keep the previous frame's content (decision R8), and that content
includes the previous frame's HUD text. For pixel identity, the view must therefore start from
the *composed* previous screen, not from a view-only copy. `Display::draw`:

1. copies the composed persistent screen into `out`;
2. draws the status bar;
3. runs the renderer's frame body on `out`, using the crate-internal path that `render` uses
   (crates/doom-render/src/lib.rs:480-593), with the composed screen as its starting buffer;
4. draws the HUD;
5. stores `out` back as the persistent screen, with taint 0 for the status-bar and HUD pixels;
6. returns the palette number.

The output is one 320x200 indexed buffer and one palette number in 0..=13 per drawn tic:
`STARTREDPALS` 1 to 8, `STARTBONUSPALS` 9 to 12, `RADIATIONPAL` 13 (st_stuff.c:71-76,
1004-1054).

The platform receives PLAYPAL bytes `[palette*768 .. +768]`. Gamma correction (`usegamma`) is
`I_SetPalette`'s job in the C's i_video.c; at gamma 0 it leaves the palette unchanged
(inference: §1.3.5 is where the gamma table should be cited).

**Wipe and screenblocks.**

- **Wipe.** The level-start melt (d_main.c:222-226, 326-344) is a wall-clock transition with no
  ticks running. M5b does not draw it. Its only effect on game frames is the forced status-bar
  refresh, which `st_firsttime` already causes after `ST_Start`, plus its `M_Random` draws
  (RD1 below).
- **Screenblocks.** Only 10 (the default here, so the status bar shows) and 11 are offered.
  Screenblocks 9 would need the border (`R_FillBackScreen`, `R_DrawViewBorder`, the background
  in `screens[1]`, and `HU_Erase`'s `R_VideoErase`), which M5a left out. Changing between 10 and
  11 at runtime builds the other `Renderer` configuration and moves the `RenderState` across
  (`Renderer::restore` checks only the level, crates/doom-render/src/lib.rs:454-458). It also
  follows the C's `fullscreen` refresh rule.

### 3.9 Resources

The status bar, HUD and palette need these lumps from the caller's global lump list:

- **PLAYPAL**: all 14 palettes (14 x 768 bytes).
- **Status bar** (`ST_loadGraphics`, st_stuff.c `ST_loadGraphics`; `STlib_init` in st_lib.c):
  STBAR, STARMS, STTNUM0-9, STTPRCNT, STTMINUS, STYSNUM0-9, STGNUM2-7, STKEYS0-5, STFB0 (the
  face background of `consoleplayer` 0, loaded but drawn only in netgame), STFST00-42,
  STFTR00-40, STFTL00-40, STFOUCH0-4, STFEVL0-4, STFKILL0-4, STFGOD0 and STFDEAD0.
- **HUD font**: STCFN033 to STCFN095 (hu_stuff.c:405-408).
- If vanilla's pause picture were drawn: M_PAUSE. Recommendation: no, the app pause is the
  app's screen.

**Lookup.** `W_CacheLumpName` uses `W_GetNumForName`: the last match wins (the backward scan),
and `I_Error` if missing. doom-render already implements exactly that:
`LumpDirectory::check_num_for_name` and `get_num_for_name`, with the C's comparison of the raw
stored name against the upper-cased query (crates/doom-render/src/r_data.rs:185-195).

`HudResources::load(&Resources)` resolves every name above in `ST_loadGraphics`' order, returns a
typed `ResourceError` naming the first missing lump, and validates every patch header and column
offset as M5a validates sprite patches. A bad post is refused when drawn, not at load.
`Resources` itself is unchanged except for the read-only accessors of §3.5 (it needs `dir`, which
is crate-internal). CI uses a new synthetic graphics PWAD for these lumps, made by a new
generator in the style of tools/reference/m5a_gfx.py. The existing m5a_gfx.wad stays unchanged.
No id art is involved.

### 3.10 Toolchain, workspace, fuzz, mutation testing and CI

- **Language and lints.** Edition 2024 and `rust-version = "1.85"` come from `[workspace.package]`
  (Cargo.toml:5-11). The lints are `unsafe_code = "forbid"` and `missing_docs = "warn"`
  (Cargo.toml:16-18). CI runs Clippy with `-D warnings` (.github/workflows/rust.yml:111), which
  turns `missing_docs` into an error, so every public item of doom-game and doom-play needs docs.
- **MSRV.** The `Minimum Rust / 1.85.0` job builds and tests the **whole workspace** on
  ubuntu-latest (rust.yml:413-429), so the windowing library and its whole dependency tree must
  compile on 1.85.0 with the committed `Cargo.lock` (`--locked`). That is a selection
  constraint for the library comparison (§3.12).
- **`unsafe` at the call site.** The library must also offer a fully safe API from the caller's
  side. Some window and surface APIs need `unsafe` at the call site, which `forbid` rejects.
- **Dev profile.** Only doom-render and the hash crates are optimised in dev builds
  (Cargo.toml:20-30). The interactive command should be `cargo run --release -p doom-play -- ...`.
  Whether a debug build of doom-sim sustains 35 tics a second on a real map was **not measured**.
  I recommend not adding a dev-profile override for doom-sim, because the existing timing and
  Callgrind jobs assume its current settings.
- **Every OS builds it.** `Stable / ubuntu, windows, macos` (macOS on `macos-26-intel`) runs
  Clippy, build, test and doc with `--workspace` (rust.yml:76-117, 405-407), so doom-play builds
  and is linted on all three. This is good: it proves portability.
- **Disturbances to expect:**
  - **System packages on Linux.** A Linux windowing backend may need X11 or Wayland development
    packages at build time. Today the Stable job installs none (only the Callgrind job runs
    `apt-get`, rust.yml:482-487). Prefer a library that builds without them, or add a step to
    the Linux leg only, keeping every existing step and required check name.
  - **Headless test runs.** `cargo test --workspace` runs on CI machines without a display.
    doom-play's tests must never open a window. Its `main` should be the only code that does,
    and the loop is tested in doom-game with fakes. A `--help` or `--headless N` smoke run of
    the binary can go in a new CI step.
  - **Lock file and build time.** A windowing library adds dozens of crates; Cargo.lock has 36
    packages today (M5A investigation §4.6). That means more build time on every leg, MSRV and
    mutation builds included, and supply-chain review.
- **Fuzz workspace.** fuzz/Cargo.toml is a separate workspace with path dependencies
  (fuzz/Cargo.toml:11-17). New targets:
  - `input_ticcmd` (an event and tic stream through `respond`, `build_ticcmd` and
    `record_round`, asserting the wire-24 domain);
  - `display_state` (DDSP decode and restore);
  - `hud_message` (the item-5 derivation over arbitrary event lists).

  They depend on doom-game and doom-render only, **never on doom-play**, so the fuzz build never
  compiles the windowing library. That is the reason for the doom-game/doom-play split:
  dependencies are per package, so a library target inside doom-play would drag the windowing
  library into the fuzz workspace. `cargo fmt --manifest-path fuzz/Cargo.toml` already runs in
  CI (rust.yml:108).
- **Mutation testing.** It runs only when the `run-mutants` label is put on the PR (`labeled`
  events, not `synchronize`; remove and re-add the label to re-run), across 8 shards, with
  `cargo mutants -p doom-sim -p doom-render ... --in-diff` against `origin/master`'s
  `crates/` diff (mutants.yml:9-20, 64-96).
  - doom-game must be added as `-p doom-game`, or its changed lines are not mutated.
  - doom-play should stay out: it is platform glue that headless tests cannot kill.
  - A windowing dependency in the workspace makes the baseline build of every shard heavier
    even when doom-play is not mutated (inference: cargo-mutants builds the selected packages
    and their tests, and a workspace dependency is resolved).
- **CI that is unaffected.** The `Simulation instruction counts` job builds `-p doom-sim
  --examples` only (rust.yml:489-491). The M5a genuine-renderer job builds its runner from C.

### 3.11 Size estimate (hand-written lines, ±30%)

| Component | Lines | Reasoning |
| --- | --- | --- |
| `v_video` (patch draw, copy rect, patch validation) | 200 | Two C functions plus bounds checks, reusing M5a's patch parsing |
| `st_lib` | 300 | st_lib.c is 293 lines; Rust widget structs with old values and refresh logic are about 1:1 |
| `st_stuff` (tickers, face, widgets, palette, graphics loading, refresh/diff) | 900 | st_stuff.c is 1,471 lines, of which about 500 are cheats, automap messages and netgame chat that are not ported; plus typed refusals |
| `hu_lib` | 250 | hu_lib.c is 354 lines; the input-text widget is reduced to what `HU_Drawer` calls |
| `hu_stuff` incl. the message derivation and cross-check | 350 | Ticker, drawer, erase and start (about 250 C lines used) plus the item-5 derivation |
| `d_display` (composition, palette output, screenblocks change, refusals) | 300 | The D_Display order, the composed screen hand-off and the state commit |
| State and serialization (DDSP encode, decode and level checks, about 70 fields) | 450 | DRST v2's state.rs is 814 lines for a comparable field count, with half of that shared |
| `input` (events, bindings, `G_Responder` game half, `G_BuildTiccmd`, `record_round`) | 550 | About 330 C lines with double-click and turn acceleration; defaults table |
| `recorder` | 150 | Command buffer, atomic periodic rewrite, naming |
| `session` loop / app core (state machine, clock-driven tic scheduling, restart, quit) | 600 | The `TryRunTics` analogue, end and refusal handling, tic-0 clone restart |
| `screens` + own font | 300 | Font data (about 100 lines as a const table) plus layout and text |
| doom-play platform shim (window, key translation, mouse, scaling, present, real clock, arguments) | 400 | Depends on the chosen library; kept thin by design |
| Tests (ST/HU unit tests from the C facts, input unit tests, message derivation against the genuine `P.message` column, headless loop with a fake clock and platform, DDSP round trip and rewind, frame goldens with the status bar) | 1,800 | M5a's tests, examples and common code came to about 3,000 lines |
| Fuzz targets | 150 | Three small targets |
| **Total in crates** | **≈ 6,700** | |
| Tools outside the crates (Python and C) | ≈ 1,000 | A new genuine runner variant that compiles st_*.c, hu_*.c and v_video.c instead of their stubs, with a frame dump including rows 168-199 and the palette number; a synthetic status-bar and font PWAD generator; a local real-map frame comparison |
| **Grand total** | **≈ 7,700** | |

**Calibration.** M5a's own estimate was about 6,600 hand-written lines (M5A investigation §4.7).
It landed at about 10,600 hand-written lines in `src` (11,968 counted, minus the 1,379-line
generated `tables.rs`) plus 3,029 lines of tests, common code and examples, roughly 2x. Applying
the same factor gives about 13,000 to 15,000. A realistic range is **7,700-15,000**.

### 3.12 Window and input library: a measured comparison

Measured 2026-11-17 (host clock, America/Los_Angeles) on the owner's WSL2 Ubuntu, `cargo 1.100.0` (stable), `rustc 1.85.0`. Trial projects: `/home/copgunn/scratch/m5b/deps/<option>/` (re-run notes in `README.txt` there). Every trial uses `edition = "2024"`, `rust-version = "1.85"`, `resolver = "3"`, `#![forbid(unsafe_code)]`, and a scratch `CARGO_HOME`. "Newest" below means what resolver 3 picked for MSRV 1.85 on this date. Build times are single, non-idle, warm-download figures, so they are rough.

#### Summary

| | winit 0.30.13 + softbuffer 0.4.8 | minifb 0.29.0 | sdl2 0.38.0 (SDL2 C library) | sdl3 0.22.1 (SDL3 C library) | miniquad 0.4.11 |
|---|---|---|---|---|---|
| Pure Rust or C | Pure Rust (system libs are dlopen-ed at run time) | Pure Rust (dlopen at run time) | Rust bindings to C SDL2 | Rust bindings to C SDL3 | Pure Rust (system GL/X11/Wayland libs dlopen-ed at run time); needs OpenGL |
| Licence vs GPL-2.0-only | **winit `Apache-2.0` alone, dpi `Apache-2.0 AND MIT`; 4 more Apache-only crates on Linux** (6 flagged of 111) | None flagged (0 of 37) | None flagged (0 of 7); SDL itself is Zlib | None flagged (0 of 6); SDL itself is Zlib | None flagged (0 of 5) |
| Builds in CI on Linux/Windows/macOS | Compiles with no system dev packages on this Linux box; Windows/macOS trees are pure Rust (not run) | Same | Needs `libsdl2-dev` or `bundled` (cmake + C build) or vcpkg on Windows; here the link failed: `unable to find library -lSDL2` | Same with SDL3: `-lSDL3`; `build-from-source` needs cmake | Compiled with no system dev packages; tree is libc, winapi, objc-rs, malloc_buf (not run on Windows/macOS) |
| Relative mouse | Yes: `DeviceEvent::MouseMotion` raw delta, plus `set_cursor_grab` Locked (Wayland, macOS) / Confined (X11, Windows) | **No** | Yes: `set_relative_mouse_mode(true)` | Yes: `set_relative_mouse_mode(&window, true)` | Yes: `raw_mouse_motion(dx, dy)` handler plus `set_cursor_grab` (observed delivering on X11) |
| MSRV 1.85.0 | Builds (winit declares 1.70, softbuffer 1.71) | Builds (no `rust-version` declared) | sdl2 crate declares none; type-checks on 1.85.0 | **sdl3 0.22.1 does not compile on 1.85.0**; versions 0.16.0 to 0.21.0 also fail (details below) | Builds (declares 1.70) |
| Dependency tree (unique crates, union of 4 targets) | 111 (Linux 85, Windows 17, macOS 36 each) | 37 (Linux 36, Windows 7, macOS 6) | 7 (7 on each) | 6 (6 on each) | 5 (Linux 2, Windows 2, macOS 4 each) |
| Loop style | Callback trait, but `EventLoopExtPumpEvents::pump_app_events` gives a poll style; works here | Pure poll (`update_with_buffer`), but library paces to a set FPS | Pure poll (`EventPump::poll_iter`) | Pure poll | Owns the loop (callback `EventHandler`); no poll API |
| Trial under WSLg | Wayland: ran, exit 0, 71 presents; X11: **panic** (`libxkbcommon-x11.so` not installed); default picks Wayland | X11: ran, exit 0, 70 presents; Wayland: ran, exit 0, 70 presents | Not run: no SDL2 library | Not run: no SDL3 library | X11: ran, exit 0, 407-575 draws; Wayland: ran, exit 0, 95-118 draws; both set: exit 0 |

Facts that look decisive (updated after adding miniquad): (0) miniquad has no Apache-only crate, a 5-crate tree, a raw-mouse handler and grab, and ran under WSLg on both X11 and Wayland, but it is an OpenGL library, not a framebuffer one: presenting the 320x200 frame means our own texture, quad and shader code, there is no focus-lost event, it owns the loop, and it declares no fullscreen-borderless detail beyond `set_fullscreen(bool)`; (1) winit's own licence field is `Apache-2.0` with no alternative, so any winit route needs an owner decision on the FSF position; (2) minifb has no relative mouse, no fullscreen, no scancode or repeat flag, so it cannot meet the brief; (3) the SDL routes have the smallest Rust trees but need either a system SDL package or a cmake C build in all three CI jobs, and sdl3 does not build on the 1.85.0 MSRV at all; (4) winit's X11 path needs a runtime library (`libxkbcommon-x11`) that is absent from this WSL install, while its Wayland path ran.

#### Common method

- Resolve: `cargo generate-lockfile` (resolver 3 reported "Locking N packages to highest Rust 1.85 compatible versions"), then `cargo fetch`.
- Trees: `cargo tree --locked --target T -e normal,build --prefix none --format '{p}'` for `T` in `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`; counted unique name+version, excluding the trial package. "Normal only" uses `-e normal`; the difference is build-time-only crates.
- Licences: `cargo metadata --format-version 1 --filter-platform T --locked` per target, the `license` field of each crate in the union. Analysis script: `/home/copgunn/scratch/m5b/deps/analyze.py`; raw output `/home/copgunn/scratch/m5b/deps/analysis.txt`. The script treats MIT, BSD-2/3-Clause, ISC, Zlib, 0BSD, Unlicense, CC0-1.0, BSL-1.0, Unicode-3.0/DFS-2016 as compatible, `OR` as satisfiable if one branch is compatible, `AND` as needing every part. `MIT/Apache-2.0` is the old spelling of `OR`. No crate in any tree lacked a `license` field; none had only a `license-file`.
- Lock delta: number of `[[package]]` entries in the trial's `Cargo.lock` minus the trial itself (the lock lists crates for every target, including targets outside the four above).
- Builds: `cargo build --release --locked` (stable, timed), `cargo +1.85.0 build --locked`.
- Runs: `timeout 20 env -u WAYLAND_DISPLAY DISPLAY=:0 <bin>` (X11), `timeout 20 env -u DISPLAY WAYLAND_DISPLAY=wayland-0 <bin>` (Wayland), `timeout 20 <bin>` (both set). Output in `/home/copgunn/scratch/m5b/deps/runs/`. After the runs `ps -o pid,cmd --ppid $$` showed only the `ps` and `head` processes.
- For all runs: nobody was at the keyboard and I cannot see the screen. "Ran without error" is all I observed; I did not observe that a window was visible, correct, or scaled correctly.

#### winit 0.30.13 + softbuffer 0.4.8

Source: `/home/copgunn/scratch/m5b/deps/winit-softbuffer/`; trial program `src/main.rs`.

- **Versions (resolver 3):** winit 0.30.13 (`rust-version = "1.70.0"`), softbuffer 0.4.8 (`1.71.0`). Cargo said wasip2 and wayland-protocols had newer releases needing Rust 1.86/1.87, and chose older ones. `cargo +1.85.0 build` finished (35 s debug). Stable `cargo build --release`: **45.2 s** clean (downloads excluded, non-idle machine). Release binary 5.2 MB.
- **Tree:** union 111 crates. Linux 85 (79 normal, 6 build-only), Windows 17 (16 + 1), macOS x86_64 36 (35 + 1), macOS aarch64 36 (35 + 1). Cargo.lock: 202 crates plus the trial (50 KB), because it lists every platform's crates (wasm, Android, Redox and others).
- **Supply chain:** 21 crates with build scripts (ahash, drm-fourcc, drm-sys, getrandom 0.3.4, libc, objc-sys, objc2 0.6.5, proc-macro2, quote, rustix 0.38.44 and 1.1.6, smithay-client-toolkit, thiserror 1.0.69, tiny-xlib, wayland-backend, wayland-client, wayland-sys, windows_x86_64_msvc, winit, x11-dl, zerocopy); 4 proc-macro crates (bytemuck_derive, foreign-types-macros, thiserror-impl, wayland-scanner); one `links` crate (objc-sys). Duplicated versions: bitflags 1.3.2/2.13.2, linux-raw-sys (0.4.15, 0.9.4, 0.12.1), objc2 0.5.3/0.6.5, objc2-foundation 0.2.2/0.3.2, rustix 0.38.44/1.1.6, syn 2.0.119/3.0.10, windows-sys 0.52.0/0.61.2. No crate downloads or builds C at build time (by the build-script list; `drm-sys` and `wayland-sys` and `x11-dl` build scripts only look up or bind libraries, I did not read each one).
- **Licences, distinct expressions (crate counts):** `MIT OR Apache-2.0` 37; `MIT` 36; `MIT/Apache-2.0` 7; `Apache-2.0` 5; `Zlib OR Apache-2.0 OR MIT` 5; `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` 5; `Apache-2.0 OR MIT` 4; `MIT OR Apache-2.0 OR Zlib` 4; `BSD-3-Clause` 2; `BSD-2-Clause` 1; `Apache-2.0 AND MIT` 1; `ISC` 1; `Unlicense OR MIT` 1; `(MIT OR Apache-2.0) AND Unicode-3.0` 1; `BSD-2-Clause OR Apache-2.0 OR MIT` 1.
- **Flagged (need the owner's attention), all normal dependencies:**
  - `winit 0.30.13`: `Apache-2.0` (LICENSE file is the Apache text); all four targets.
  - `dpi 0.1.2`: `Apache-2.0 AND MIT` (so Apache-2.0 applies); all four targets; pulled in by winit.
  - `ab_glyph 0.2.32`: `Apache-2.0`; Linux only, via `sctk-adwaita` (winit feature `wayland-csd-adwaita`).
  - `ab_glyph_rasterizer 0.1.10`: `Apache-2.0`; Linux only, same path.
  - `owned_ttf_parser 0.25.1`: `Apache-2.0`; Linux only, same path.
  - `gethostname 1.1.0`: `Apache-2.0`; Linux only, via `x11rb`, which both winit and softbuffer use.
  - Trimming features helps only partly. Variant `winit-softbuffer-lean` (`default-features = false, features = ["rwh_06","x11","wayland"]`, which checks fine) has union 101 (Linux 75) and removes the three font crates, but still flags winit, dpi and gethostname.
- **Native requirements:** No `-dev` packages needed to build (the system has no `pkg-config` entries for x11, xkbcommon or wayland-client, and it built). `ldd` of the release binary shows only libc, libm, libgcc_s: X11, xkbcommon and Wayland libraries are dlopen-ed at run time. Run time on Linux needs libwayland-client and libxkbcommon for Wayland, and libX11, libxkbcommon and libxkbcommon-x11 for X11. Windows and macOS use system APIs through windows-sys and objc2. CI: per the build result, no extra `apt-get` step seems needed to compile on `ubuntu-latest`; I cannot run the runners. softbuffer uses a Win32 and macOS CPU surface and X11/Wayland shared memory; none needed `unsafe` in our code: `Surface::new(&ctx, Rc<Window>)` takes a safe handle.
- **Input and presentation (from API docs in the crate source, and the trial):** `WindowEvent::KeyboardInput` carries `physical_key` (`PhysicalKey::Code(KeyCode)`), `state`, and `repeat`. `MouseInput`, `Focused`, `Resized`, `CursorEntered/Left` exist. `DeviceEvent::MouseMotion { delta }` is "raw, unfiltered physical motion" and is not tied to the cursor, so it keeps reporting at a screen edge once the cursor is locked or confined. `Window::set_cursor_grab`: `Confined` is "not implemented" on macOS; `Locked` is "not implemented" on X11 (docs in `window.rs`), so the portable pattern is Confined then Locked, with `set_cursor_visible(false)`. In the trial under WSLg-Wayland `Locked` returned `Ok(())` and `Confined` was never tried. Borderless fullscreen is `Window::set_fullscreen(Some(Fullscreen::Borderless(None)))` (API from the crate; the trial did not call it). Scaling: softbuffer gives a window-sized buffer and does no scaling, so we scale (3x loop in the trial); nearest neighbour is therefore ours. Vsync: softbuffer does not promise it (presents are not tied to refresh by the crate); I did not measure.
- **Loop style:** `ApplicationHandler` callbacks, but `winit::platform::pump_events::EventLoopExtPumpEvents::pump_app_events(timeout, &mut app)` lets our own 35 Hz loop (and an injectable clock) own the loop. Its docs warn that on macOS the app is "stopped while outside" pump calls and it is less efficient than polling on other systems; on Windows it uses `PeekMessage`. Window resizes can stall pumping on some platforms (documented by winit; not tested).
- **Trial results (WSLg, `timeout 20`):**
  - Wayland (`-u DISPLAY`): exit 0, 71 presents in 2 s, events: resumed, Resized(960x600), Focused(true), RedrawRequested x2, ModifiersChanged x2, CursorEntered, CursorMoved, CursorLeft. Cursor grab: `Locked=Ok(())`.
  - Both variables set: exit 0, 70 presents, same kinds (it chose Wayland).
  - X11 (`-u WAYLAND_DISPLAY`): exit 101, panic in `xkbcommon-dl-0.4.2/src/x11.rs:59:28`: `Library libxkbcommon-x11.so could not be loaded.` (`ldconfig -p` shows libxkbcommon.so.0 but no libxkbcommon-x11).
- **Commands:** `cargo +1.85.0 build --locked`, `cargo build --release --locked`, as above.

#### minifb 0.29.0

Source: `/home/copgunn/scratch/m5b/deps/minifb/`.

- **Versions:** minifb 0.29.0 (no `rust-version` declared; it built on 1.85.0 in 12.5 s debug). Release clean build **14.9 s**; binary 1.6 MB. Cargo.lock: 80 crates plus the trial (19 KB).
- **Tree:** union 37. Linux 36 (32 normal), Windows 7 (4), macOS x86_64 6 (3), macOS aarch64 6 (3).
- **Supply chain:** 11 build-script crates (getrandom 0.4.3, libc, minifb, proc-macro2, quote, rustix 1.1.6, wayland-backend, wayland-client, wayland-sys, winapi 0.3.9, x11-dl); 1 proc-macro (wayland-scanner); no duplicated versions; no `links` crate.
- **Licences:** `MIT OR Apache-2.0` 17; `MIT` 10; `MIT/Apache-2.0` 3; `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` 2; `Apache-2.0 OR MIT` 1; `ISC` 1; `Unlicense OR MIT` 1; `MIT OR Apache-2.0 OR Zlib` 1; `(MIT OR Apache-2.0) AND Unicode-3.0` 1. **Flagged: none.**
- **Native requirements:** Linux: builds without dev packages; X11 (`x11-dl`) and Wayland are loaded at run time (features `wayland`, `x11`, `dlopen` default). Windows presents through GDI (crate docs), macOS through Cocoa/GPU, X11 scales in software, Wayland optionally via GPU (`WindowOptions::use_gpu`).
- **API (from source and docs):** No event stream; polled state only: `get_keys_pressed(KeyRepeat)`, `is_key_down`, `get_mouse_down`, `get_mouse_pos`, `get_scroll_wheel`, `is_active`. Keys are its own `Key` enum (not a documented scancode), no per-event repeat flag (only `KeyRepeat::Yes/No` filtering plus settable delay and rate). `grep -i "relative\|grab\|raw_motion\|fullscreen"` of `lib.rs` and the README found nothing beyond the position doc comments: **no relative mouse, no cursor grab, no true fullscreen** (only `WindowOptions { borderless, resize, topmost, ... }`, `set_cursor_visibility`). Presenting: `update_with_buffer(&[u32], w, h)` and the library scales to the window with `ScaleMode` (Stretch, AspectRatioStretch, Center, UpperLeft); no documented filter choice, so I did not establish whether it is nearest neighbour. `set_target_fps(35)` paces the loop (it blocks inside `update`, so it competes with an injectable-clock loop; `limit_update_rate(None)` disables it). `get_window_handle()` returns a raw `*mut c_void` (we would not use it).
- **Trial results:** X11: exit 0, 70 presents, `active: true`, size (960, 600), 3 absolute mouse position changes (the WSLg pointer moving). Wayland and both-set: exit 0, 70 presents, stdout line `Failed to create server-side surface decoration: NotPresent` (it ran anyway; it is the Wayland path).

#### sdl2 0.38.0 (and the bundled variant)

Source: `/home/copgunn/scratch/m5b/deps/sdl2/`, `sdl2-bundled/`.

- **Versions:** sdl2 0.38.0 (MIT), sdl2-sys 0.38.0 (`MIT AND Zlib`); no `rust-version` declared; `cargo +1.85.0 check` passes. **Build/link fails here:** `cargo build --release` gives `rust-lld: error: unable to find library -lSDL2` (about 3 s of compile). Found out by `pkg-config --exists sdl2` (not found) and `dpkg -l` (no libsdl2 packages; only libwayland-client0, libwayland-cursor0, libwayland-egl1, libxkbcommon0, libxcursor1, libgl1, libegl1 runtime libs). `cmake` is also absent (`which cmake` empty).
- **Tree:** union 7 on every target (5 normal, 2 build-only). Lock 7 crates plus trial. 3 build-script crates (libc, sdl2, sdl2-sys); `links = "SDL2"`; no proc-macros; no duplicates.
- **Licences:** `MIT OR Apache-2.0` 3, `MIT` 2, `MIT/Apache-2.0` 1, `MIT AND Zlib` 1. **Flagged: none.** SDL2 itself (`sdl2-sys-0.38.0/SDL/LICENSE.txt`, "Copyright (C) 1997-2023 Sam Lantinga") is the Zlib licence.
- **Linking:** default is a dynamic system `libSDL2` (needs `libsdl2-dev` at build time and `libSDL2-2.0.so.0` at run time; on Windows an `SDL2.dll` next to the exe, or vcpkg/`static-link`). Feature `bundled` builds the 90 MB of SDL2 source shipped inside sdl2-sys with cmake (variant `sdl2-bundled`: union 11 crates, adds `cmake` build crate and helpers; licences unchanged, none flagged). `cargo build` of that variant failed here because `cmake` is not installed: the build script ran `cmake .../SDL ...` and it could not execute. Also feature `static-link`, `use-pkgconfig`, `use-vcpkg`.
- **Needs unsafe in our code:** no. The trial uses only safe API and compiles under `#![forbid(unsafe_code)]` (`cargo check`).
- **API (docs and trial source):** `Event::KeyDown { scancode, keycode, repeat, .. }`, `KeyUp`, `MouseButtonDown/Up`, `MouseMotion { xrel, yrel }` (relative when `mouse().set_relative_mouse_mode(true)`, which SDL implements with the OS raw/confined mechanisms), `Window` events for focus and resize, `window.set_fullscreen(FullscreenType::Desktop)` for borderless fullscreen. The library scales: `canvas.set_logical_size(320, 200)` plus hint `SDL_RENDER_SCALE_QUALITY=0` (nearest). Vsync: `into_canvas().present_vsync()`. Poll style: `EventPump::poll_iter()`; no callback ownership. Not run, so none of this is observed.
- **Trial result:** not run (cannot link). I did not try to install anything.

#### sdl3 0.22.1 (and the build-from-source variant)

Source: `/home/copgunn/scratch/m5b/deps/sdl3/`, `sdl3-old/`, `sdl3-build-from-source/`.

- **Versions:** sdl3 0.22.1 (MIT, no `rust-version`), sdl3-sys 0.7.3+SDL-3.4.20 (Zlib, `rust-version = "1.85"`). **`cargo +1.85.0 build` fails in the sdl3 crate itself:** 5 x `error[E0658]: use of unstable library feature 'unsigned_is_multiple_of'` at `sdl3-0.22.1/src/sdl3/render.rs:2565`. The crate does not declare a `rust-version`, so resolver 3 cannot steer away from it. In `sdl3-old` I pinned older releases with `cargo update -p sdl3 --precise X` and ran `cargo +1.85.0 check`: 0.21.0, 0.20.0, 0.19.0, 0.18.0 and 0.17.3 and 0.17.0 fail the same way; 0.16.2 and 0.16.0 additionally fail with `integer_sign_cast`; 0.15.0 is not in the index; 0.14.0 compiles the library on 1.85.0 (the only errors were my trial program using the newer API, `ScaleMode` and `PixelFormat::ARGB8888`). Stable builds 0.22.1; `cargo check` passes with stable.
- **Build/link here:** `unable to find library -lSDL3` (about 5.6 s). `pkg-config --exists sdl3` fails; `cmake` absent. Variant `build-from-source` (pulls `sdl3-src 3.4.20`, 38 MB, plus `cc`, `cmake`, `rpkg-config`, `sdl3-ttf-src`...) failed because cmake could not run.
- **Tree:** union 6 on every target (4 normal, 2 build-only); lock 6 crates plus trial. 3 build-script crates (libc, sdl3, sdl3-sys); `links = "SDL3"`. Build-from-source variant: union 12 crates.
- **Licences:** `MIT OR Apache-2.0` 3, `MIT` 1, `Zlib` 1, `MIT/Apache-2.0` 1. **Flagged: none.** Build-from-source adds `Zlib` x2 (SDL sources, "Copyright (C) 1997-2026 Sam Lantinga") and one `Zlib OR MIT OR Apache-2.0`; none flagged.
- **Native requirements:** as sdl2: dynamic system libSDL3 by default (`use-pkg-config`, `use-vcpkg` default features of sdl3-sys), static or from source with cmake, `link-framework` on macOS. SDL3 is much newer, so Ubuntu LTS runners may not package `libsdl3-dev` (I did not check the runner image).
- **unsafe in our code:** no; trial type-checks with `forbid(unsafe_code)` on stable. (`raw-window-handle` feature is optional.)
- **API:** `sdl3::event::Event::KeyDown { scancode, repeat, .. }`, `MouseMotion` with relative deltas, `mouse().set_relative_mouse_mode(&window, true)`, `Texture::set_scale_mode(ScaleMode::Nearest)`, `set_logical_size(..., LETTERBOX)`, `window.set_fullscreen(true)`; poll style (`poll_iter`). Not run.
- **Trial result:** not run.

#### miniquad 0.4.11

Source: `/home/copgunn/scratch/m5b/deps/miniquad/`. Added later in the census; same method. Trial program `src/main.rs`; raw run output `runs/miniquad-*.out/.err`.

- **Versions:** miniquad 0.4.11, `license = "MIT OR Apache-2.0"`, `rust-version = "1.70"`. `cargo +1.85.0 build --locked` finished (3.1 s). Stable `cargo build --release --locked`: **3.4 s** clean (non-idle; downloads excluded). Release binary 1.0 MB. Cargo.lock: 8 crates plus the trial (libc, malloc_buf, miniquad, ndk-sys, objc-rs, winapi, winapi-i686-pc-windows-gnu, winapi-x86_64-pc-windows-gnu), 2 KB.
- **Tree:** union 5 (libc 0.2.191, malloc_buf 0.0.6, miniquad, objc-rs 0.2.8, winapi 0.3.9), counted including miniquad itself. Linux 2 (libc, miniquad), Windows 2 (miniquad, winapi), macOS x86_64 4 and aarch64 4 (miniquad, libc, objc-rs, malloc_buf); no build-only crates. Build scripts: 3 (libc, miniquad, winapi; I did not read them). No proc-macro crates, no `links`, no duplicated versions.
- **Licences:** `MIT OR Apache-2.0` 2; `MIT` 2; `MIT/Apache-2.0` 1. **Flagged: none** (no missing licence field, no licence-file-only crate).
- **Native requirements:** Pure Rust; it loads libraries at run time with dlopen (strings in the source): Linux X11 path `libX11.so.6`, `libXi.so.6`, `libxkbcommon.so.0`, `libGL.so.1` (GLX, or `libEGL.so.1`); Wayland path `libwayland-client.so.0`, `libwayland-egl.so.1`, `libxkbcommon.so.0`, `libEGL.so.1`. `ldconfig -p` on this WSL install lists all of these (libX11.so.6, libXi.so.6, libxkbcommon.so.0, libGL.so.1, libEGL.so.1, libwayland-client.so.0, libwayland-egl.so.1), and it built with no dev packages (no pkg-config entries needed). Windows: `opengl32.dll` plus winapi (user32/gdi32 etc.). macOS: OpenGL framework (3.2 core profile) by default, or Metal via `conf.platform.apple_gfx_api`. OpenGL versions: README lists Windows GL 3 and 2.2, Linux GL 2.2/3 and GLES 2/3, macOS GL 3 or Metal. Source: GLX context request 2.1 (core flag), EGL client version 2, WGL tries 3.1 then 2.1, macOS 3.2 core. A CI runner needs no extra package to compile (nothing is linked); a real GL context is needed to run. `forbid(unsafe_code)` holds: the trial uses `BufferSource::slice`, `new_texture`, the pipeline and draw calls, all safe functions (compiled with `cargo check`, stable and 1.85.0).
- **Input and presentation (crate source and trial):**
  - `key_down_event(KeyCode, KeyMods, repeat: bool)` and `key_up_event(KeyCode, KeyMods)`: a key code with a repeat flag (a `KeyCode` enum, no raw scancode).
  - `mouse_button_down_event` and `mouse_button_up_event` with button and position.
  - `raw_mouse_motion(dx, dy)`: implemented in the source for X11 (XInput2, `xi_input.rs`), Wayland, macOS, Windows (raw input registration) and wasm. The doc comment says it is delivered regardless of input focus.
  - `window::set_cursor_grab(bool)` and `window::show_mouse(bool)`. The doc says the cursor is bound to the window border on desktop, is not released automatically when focus is lost, and carries "TODO: implement window focus events". There is **no focus gained/lost event**: the only related callbacks are `window_minimized_event` and `window_restored_event`.
  - Resize via `resize_event(w, h)`; `Conf.window_resizable`; `window::set_window_size`.
  - Fullscreen: `Conf.fullscreen` and `window::set_fullscreen(bool)` (not exercised).
  - Presenting: no CPU-surface API. We create an RGBA8 texture with `FilterMode::Nearest`, expand the indexed frame through the palette ourselves, `texture_update` it, and draw a textured quad (shader source GLSL 100 in our code; MSL needed for Metal). Scaling to the window is done by the GPU, nearest if we set it; letterbox or 4:3 means choosing the quad's vertices ourselves.
  - Loop: `miniquad::start(conf, || Box<dyn EventHandler>)` owns the thread; `update()` and `draw()` are called per frame. We would run our fixed 35 Hz logic inside `update`/`draw` using our own clock (an injectable clock is possible; the engine loop cannot be driven from outside, and `conf.platform.blocking_event_loop` with `schedule_update` is the only throttle knob).
  - Vsync: `conf.platform.swap_interval`, `None` means 1 in the source (`unwrap_or(1)`), `Some(0)` should disable; the doc says it is only a hint to the driver and not a reliable FPS limit. Observed here: X11 under WSLg drew 400 to 575 frames in 2 s (no effective vsync), Wayland about 95 to 118 (about 50 to 60 per second).
- **Backend choice:** `conf.platform.linux_backend`: `X11Only` (default), `WaylandOnly`, `X11WithWaylandFallback`, `WaylandWithX11Fallback` (a compile-time-free runtime setting; the trial reads `MQ_BACKEND`).
- **Trial results (WSLg, `timeout 20`, all exit codes 0):**
  - X11-only, `env -u WAYLAND_DISPLAY DISPLAY=:0`: exit 0, 499 draws, events mouse_enter x1, mouse_motion x2, raw_mouse_motion x2, restored x1; stderr empty.
  - X11WithWaylandFallback in the same environment: exit 0, 407 draws.
  - Wayland-only, `env -u DISPLAY WAYLAND_DISPLAY=wayland-0`: exit 0, 95 draws, events: restored x1; stderr: `Wayland compositor does not support cursor shape` and Mesa lines (`libEGL warning: failed to get driver name for fd -1`, `MESA: error: ZINK: failed to choose pdev`, `egl: failed to create dri2 screen`). It still ran.
  - Both variables set: X11Only exit 0 (550 draws), X11WithWaylandFallback exit 0 (575), WaylandOnly exit 0 (117), WaylandWithX11Fallback exit 0 (118). Output files: `runs/miniquad-both-*`, `runs/miniquad-x11-*`, `runs/miniquad-wayland-*`.
  - `renderer info` printed `OpenGl`; window size reported (960.0, 600.0). No `resize` event at start, no focus events (none exist). Again, "ran without error" is all I observed: I saw no window, so nothing is known about visible output, correctness or scaling. The in-program 2 s timeout only fires from `draw`, so a hung window would be stopped only by `timeout 20`.
- **Commands:** `cargo +1.85.0 build --locked`; `cargo build --release --locked`; runs as in the Common method with `MQ_BACKEND=<x11|wayland|x11-fallback|wayland-fallback>`.

#### Other candidates

`pixels` was not evaluated: it renders through wgpu (large tree, GPU requirement) and still sits on winit, so its licence flags would include winit's. Not measured, to avoid padding.

### 3.13 Playing on Windows: WSLg and a native build artifact

**Route (i): the Linux build through WSLg.**

- Observed: the Wayland path of winit+softbuffer and both paths of minifb opened a window and presented 70 frames in 2 s without error (see above). Observed: winit's X11 path panicked for want of `libxkbcommon-x11`. Not observed: whether anything appeared on screen, input, latency.
- Documentation (not observation): Microsoft says WSL supports Linux GUI apps, X11 and Wayland, on Windows 10 build 19044+ or Windows 11, with a vGPU driver for accelerated OpenGL, and that it "does not provide a full desktop experience" (https://learn.microsoft.com/en-us/windows/wsl/tutorials/gui-apps, read 2026-11-17). The open WSLg issue "Mouse locking/hiding not working in OpenGL app" (https://github.com/microsoft/wslg/issues/376, opened July 2021, still open, labelled `bug`, no maintainer reply in the thread) reports that the cursor is not hidden or locked and mouse callbacks do not fire under GLFW's disabled-cursor mode and in SDL2 and GLFW programs; reports cover WSLg 1.0.24 and 1.0.47; the suggested cause (RDP layer) is a user's guess. That thread is the only source I read on relative mouse; it is anecdotal and the versions are old, so whether current WSLg is better is unknown. Latency under WSLg was not measured and I found no documented figure.

**Route (ii): native Windows executable from CI, downloaded by the owner.**

- The existing `Stable / windows-latest` job (matrix in `/home/copgunn/src/DOOM/.github/workflows/rust.yml`, `runs-on: windows-latest`, 45 min limit; the last recorded run took 11 m 57 s per the workflow comment) builds the workspace with `cargo build`, so a new app crate would at least compile there once added as a workspace member. It would not produce an artifact. Linux job on `ubuntu-latest`, macOS on `macos-26-intel` likewise.
- Sketch of a separate workflow, not a required check:

```yaml
name: Windows playable build
on:
  workflow_dispatch:
jobs:
  windows-exe:
    runs-on: windows-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@<pinned sha, as in rust.yml>
        with: { persist-credentials: false }
      - uses: dtolnay/rust-toolchain@stable   # or the repository's own install step
      - run: cargo build --release --locked -p <app-crate>
      - uses: actions/upload-artifact@<pinned sha>
        with:
          name: doom-windows-x86_64
          path: target/release/<app>.exe
          retention-days: 14
```

  (For SDL: also copy `SDL2.dll`/`SDL3.dll` or use a static or bundled build with cmake, which the Windows runner provides, I did not verify.) The workflow does not exist; I wrote none in the repository.
- Cost: public repository, GitHub says Actions is free for standard hosted runners in public repositories (https://docs.github.com/en/actions/concepts/billing-and-usage, read 2026-11-17). The compile-only cost is roughly the Windows build step; I did not measure it, and the winit tree compiled in 45 s on Linux here. A clean app-crate Windows build may take a few minutes (estimate, not measured).
- Artifacts: `actions/upload-artifact` README (https://github.com/actions/upload-artifact, read 2026-11-17): retention defaults to 90 days, `retention-days` 1 to 90, subject to repository settings; shown version v7. The owner downloads it from the run page or `gh run download -R Copgunn/DOOM`.
- Cross-compiling from this WSL install: `rustup target list --installed` prints only `x86_64-unknown-linux-gnu`; `which x86_64-w64-mingw32-gcc` finds nothing (`gcc` and `cc` exist, no mingw, no `cargo-xwin`, no windows target). So not possible without installing something. The MSVC target would additionally need the Windows SDK libraries.

### 3.14 What the library comparison did not do or observe

- miniquad: no key or button event was generated, so key/repeat/button behaviour, focus-loss handling, and whether the grab confines the cursor are from source and docs only (raw and absolute mouse motion were observed from the WSLg pointer on X11). Windows and macOS paths (WGL, Cocoa/OpenGL/Metal) were not built or run; the Windows GL version (3.1 then 2.1) and macOS 3.2 core come from source reading. Fullscreen, `swap_interval = Some(0)` and the visible output were not tested. The earlier statement that `pixels` was not evaluated still applies.
- No screen was viewed; no input event of a real key, button or mouse movement was generated, so key down/up, repeat, button, relative motion and focus-lost behaviour are from documentation or source reading only (focus-gained was observed for winit).
- sdl2 and sdl3 were never linked or run (no libSDL2/libSDL3, no cmake); `bundled` and `build-from-source` builds were attempted and failed at the missing cmake. Their API claims come from the crate sources and are untested.
- Borderless fullscreen, resize while running, 4:3 correction, vsync behaviour, frame pacing accuracy, and latency were not exercised or measured.
- The macOS and Windows trees and licences come from `cargo tree` and `cargo metadata` for those targets; nothing was compiled for them, and I did not read the runner images or docs for system packages (the macOS and Windows runner claims above are inferences from build scripts and feature names).
- Build scripts were listed, not read; I did not audit what they download or execute beyond noting the sdl bundled/source variants require cmake.
- Licence field only (SPDX from the manifest) plus first-lines checks of the winit, dpi, SDL license files; I did not verify each crate's LICENSE text against its field. No legal advice is given.
- Which sdl3 version first needs rust 1.90 was not bisected between 0.17.3 and 0.16.2 beyond the checks listed. `pixels` and other candidates were not trialled. `target/` directories under `/home/copgunn/scratch/m5b/deps/` were left in place (about 2.3 GB; the delete was blocked by the safety check). `df -h /mnt/c` stayed at 47 GB available before and after.

### 3.15 Stop-and-ask candidates of the design

**RD1. The status bar's random "look" direction depends on sound.**

*The fact.* When the face is idle, `ST_updateFaceWidget` picks `ST_calcPainOffset() + st_randomnumber % 3`
(st_stuff.c:913-919), and `st_randomnumber = M_Random()` in every `ST_Ticker` (st_stuff.c:992).
In linuxdoom-1.10 the same `rndindex` is also advanced by:

- `S_StartSoundAtVolume` for every sound except `sfx_itemup` and `sfx_tink` (s_sound.c:327-345).
  The draw happens only after the audibility test, and that test depends on:
  - the distance between the listener and the origin *at the moment of the call, mid-tic*;
  - `snd_SfxVolume`;
  - the map-8 rule (s_sound.c:300-321, 773-817).
- The level-start wipe's `wipe_initMelt`: `width` draws (f_wipe.c:159-162, called with
  `SCREENWIDTH` from d_main.c:339-340), at the first displayed frame, after however many tics
  have run by then.

*Why the simulation cannot supply it.* The simulation never draws `M_Random`
(`rng.miscellaneous` is captured but never advanced; crates/doom-sim/src/snapshot.rs:835-837).
It emits sound events for many requests (`ActorStateEvent::Sound`, crates/doom-sim/src/lifecycle.rs:1252-1258;
`MoverEvent::SectorSound`, crates/doom-sim/src/planes.rs:603-611), but **I did not establish**
that every `S_StartSound` of the source has an event. The events also carry no mid-tic listener
or origin positions.

*The existing evidence.* The genuine runners stub `S_StartSound` and `ST_Ticker` and state that
their `M_Random` draws are not reproduced (tools/reference/m5a_stubs.c:26-31, 101-102, 130-131).

*Options:*

1. **Model vanilla exactly.** A presentation-side sound-start model that reproduces every
   audible `S_StartSound` and the wipe draws. This needs at least a complete sound-event
   inventory, and probably mid-tic positions the simulation does not expose. If those are
   missing it cannot be derived, and it would need a simulation change (forbidden in M5b).
2. **Declared sound-free model (recommended for M5b).** The status bar's `M_Random` index is
   advanced only by `ST_Ticker`, reset at every new game, with no wipe draws. The genuine M5b
   frame runner uses the same declared adapter (its `S_StartSound` stub stays draw-free and the
   melt is not run), so frames compare exactly under that adapter. COMPATIBILITY states that
   the idle face's left, straight or right choice can differ from a vanilla session with sound
   on. When sound arrives in a later milestone, revisit option 1.
3. **Leave the face out of the pixel claim.** Mask the face area in comparisons whenever
   `priority` is 0. This is weaker evidence than option 2, and not recommended.

*Runner note for option 2.* If a C runner compiles the real `ST_Ticker`, the dump's `rndindex`
changes, so the frozen DSIM projection of `rng.miscellaneous` must keep reading a draw-free
counter. Otherwise the DSIM comparison would diverge from Rust's 0.

**RD2 (decision, not a gap). The app's restart is a new game, not a reborn.**

Wire 24 refuses `PlayerReborn`, so vanilla's in-place reload is not reproducible. The reload
keeps `prndindex`, `rndindex` and `gametic` (§1.6 item 8). Recommendation: restart = `G_InitNew`
semantics. That means a tic-0 clone of the simulation, `M_Random` index 0, fresh-process
status-bar and HUD statics, and a new demo file. Document it.

**RD3 (decision). The app pause and the missing vanilla behaviours.**

- *The pause.* It freezes `ST_Ticker` and `HU_Ticker` too; vanilla keeps both running while
  paused. No pause ticcmd can be recorded under wire 24.
- *Vanilla's demo-buffer quit, the `q` key and the melt.* None is reproduced.
- *Screenblocks.* Only 10 and 11 are offered.

Recommendation: accept all of these, as presentation or app behaviour, stated in COMPATIBILITY.

**RD4 (decision). Schema for the saved display state.**

Option A (a new DDSP magic, embedding unchanged DRST v2 bytes whose `screen` is the composed
`screens[0]`) or option B (DRST v3 with a frozen v2 decoder). Recommendation: A.

No class (c) item was found for the status bar's game inputs, the palette, the HUD messages or
the command path. The only underivable input is the vanilla `M_Random` sequence (RD1), and it
becomes class (b) under the declared model of option 2.

Each candidate is taken up in §4: RD1 in S15; RD2 in S11; RD3 in S12; RD4 in S18.

### 3.16 Risks

1. **The face's idle direction (RD1).** If the owner wants vanilla-with-sound fidelity, the work
   grows by a sound model of unknown feasibility.
2. **Composed-screen hand-off.** Getting the order wrong breaks pixel identity in uncovered
   (HOM) pixels under the message line: a view drawn from a view-only buffer instead of the
   composed `screens[0]`. It is rare, but the rooms should include one, for example an M5a
   HOM-style room with a message displayed.
3. **History statics that `ST_Start` does not reset** (`priority`, `lastattackdown`,
   `st_facecount`, `message_counter`). A comparison that does not start each run from a fresh C
   process, or a Rust restart that keeps them, diverges for a tic or two after a level start.
4. **Frame-drop history.** `ST_Drawer`'s diff drawing and the persistent screen depend on which
   tics were drawn. On a slow machine the live display skips frames, which is deterministic but
   not the "every tic" sequence of the goldens. Replays for evidence must draw every tic.
5. **Cheat keys.** Routing keys through a faithful `ST_Responder` would let IDDQD and others
   write player fields outside the simulation (st_stuff.c:551-661). They must be dropped before
   any responder, and a test should assert it.
6. **Turbo derivation and the lunge rewrite.** Using the captured `player.0.forward` instead of
   the fed command would show a false turbo message on a lunge tic at `gametic % 128 == 0`
   (item 5).
7. **MSRV and safe-API constraints on the windowing library** (§3.10). A library that needs
   Rust newer than 1.85, Linux system packages, or `unsafe` at the call site disturbs CI or the
   lint policy.
8. **Mutation coverage.** doom-game must be added to mutants.yml's `-p` list, or its diff goes
   unmutated. Heavier workspace builds lengthen every shard.
9. **Evidence for the HUD derivation.** It rests on the genuine per-tic `P.message` column
   (tools/reference/m4a_driver.c:537), which is in the local real-map dumps and the CI synthetic
   dumps but is not part of the DSIM comparison today. Comparing it is new tooling.
10. **What I did not establish.**
    - The completeness of the simulation's sound events (RD1).
    - The exact `NetUpdate` and `TryRunTics` tic-batching bound, the key defaults and i_video.c's
      mouse scaling (see §1.1.7 and §1.3.2).
    - The gamma table.
    - Whether a debug build sustains 35 tics a second.
    - Whether `A_Explode`'s source is never the barrel (stated as inference, item 3).

## 4. Owner decisions (S1–S23) with recommendations

Each decision is tested against four questions: (a) is it consistent with prior owner decisions (E1–E9, F1–F11, G1–G13, H1–H12, I1–I13, P1–P9, R1–R16) and with the M5b brief? (b) does it admit no silent divergence? (c) does it add no new kind of fidelity gap beyond a declared refusal or a declared presentation contract? (d) are stores and work bounded? "Yes" to all four is written "(a)–(d) yes". Nothing below changes the simulation, a wire, a DSIM byte or a digest.

**Status: all confirmed by the owner on 2026-11-17**, as recommended, with notes on S2, S3, S15, S16, S20 and on C jobs; see [Owner rulings](#owner-rulings-2026-11-17) directly below. The decisions are kept below as written in Phase 1, each with a closing line recording its confirmation. S2 (the dependency) and S3 (turning resolution) were owner decisions whatever the recommendation, as the brief said, and S4, S15 and S16 needed a ruling because the facts contradicted something the brief took for granted; see "Why Phase 1 stopped" at the end of this section.

### Owner rulings (2026-11-17)

On 2026-11-17 the owner ruled on Phase 1 (head `689e09d`): S1–S23 are confirmed as recommended, **confirmed by owner 2026-11-17**, with the notes below. Where a note and the text of a decision differ, the note governs. Each decision's text below is the Phase 1 record, ending with a line saying it was confirmed.

- **S2 (the window library; option (i) is miniquad, (ii) is SDL2).** miniquad. COMPATIBILITY and the README's how-to-run must state the OpenGL requirement and the no-focus-event limit (keys held while the player switches away stay held until pressed again).
- **S3 (turning resolution and recording; (i) is every session recorded at demo resolution, (ii) is a full-resolution mode that is not recorded, (iii) is a remainder carried from tic to tic).** (i) is the default for M5b. Also plan (ii), the full-resolution unrecorded mode, as the last implementation step of this milestone (step 7b in [§5.1](#51-steps)) if the size cap allows, otherwise as the first item of M5c. It needs its own genuine evidence first: a runner mode that injects tic commands directly, and synthetic scripts with nonzero low bytes of `angleturn` across the mouse and keyboard ranges, all three variants, matched in Rust. It is opt-in, labelled "not recorded: no demo, cannot be replayed", and never the default. (iii) is not wanted.
- **S15 (the face's random glance; (i) is the declared sound-free, wipe-free model, (ii) is vanilla accounting now, (iii) is leaving the idle face out of the comparison).** (i). COMPATIBILITY and the README must say in plain words that the idle face's left, ahead or right glance differs from a vanilla session with sound until sound is modelled, and that nothing else on screen is affected. ROADMAP gains an item under M5c to close it. Frames are to be described as identical to the genuine engine under the declared sound-free, wipe-free contract, never as identical to a vanilla session without that qualifier.
- **S16 (the game dump when the genuine status bar and HUD run; (i) is record and mask).** Accepted: the mask is in the new runner only, and the masked fields are compared separately.
- **S20 (how the owner plays on Windows; (i) is WSLg only, (ii) is a native Windows artifact, (iii) is both).** (iii). The Windows artifact job is **not** a fallback item: it stays in M5b. If the size cap is passed, the modern bindings preset (S7) moves out first. The final report gives the exact steps for the owner to download and run the Windows build himself, with the IWAD path as an argument.
- **C reference jobs ([§6](#6-open-questions-and-where-each-is-decided) row 9).** While the machine is idle there is no job limit, C runs included; if anything is OOM-killed, halve the jobs and re-run. The 2-job limit applies again whenever the owner says the machine is in use. On 2026-11-17 the owner said the machine is not in use.
- All other decisions (S1, S4–S14, S17–S19, S21–S23) are confirmed as recommended, without notes.

### Scope

**S1. Scope.** As the brief: input, the real-time loop, the status bar, HUD messages, the palette choice, presentation, session recording, and the app's own end and refusal screens. Out: sound and music, menus, automap, save and load, intermission and level-to-level play, title screen and demo loop, wipes, cheats, vanilla's pause, skills 1, 4 and 5, low detail, screenblocks below 10, multiplayer, joystick. Not reachable under wire 24 and therefore covered by unit tests and one bespoke C scenario only, never by a level run: the berserk term of the palette and the god-mode face (§1.6 item 2). (a)–(d) yes. **Recommendation: as stated.** **Confirmed by owner 2026-11-17.**

### The two decisions the brief reserves for the owner

**S2. The window and input library (the project's first runtime dependency outside its own crates).** Measured in §3.12; every figure there was read from `cargo metadata`, the crates' own manifests and trial builds in scratch, on 2026-11-17.

| | miniquad 0.4.11 | sdl2 0.38.0 (C SDL2) | winit 0.30.13 + softbuffer 0.4.8 | minifb 0.29.0 |
| --- | --- | --- | --- | --- |
| Pure Rust | yes | no: Rust bindings to the C library | yes | yes |
| Licences against GPL-2.0-only | nothing flagged (MIT or `MIT OR Apache-2.0` throughout) | nothing flagged (crate MIT, SDL2 Zlib) | **flagged:** `winit` is `Apache-2.0` alone, `dpi` is `Apache-2.0 AND MIT`, `gethostname` is `Apache-2.0`; none can be trimmed away | nothing flagged |
| Crates added (union of Linux, Windows, macOS) | 5 | 7 | 111 (101 trimmed) | 37 |
| Builds on Rust 1.85.0 | yes | type-checks; could not be linked here | yes | yes |
| Extra CI setup | none expected (system libraries are opened at run time) | Linux needs `libsdl2-dev`; Windows and macOS need a bundled C build with cmake | none expected | none expected |
| Relative mouse with the cursor captured | yes (`raw_mouse_motion`, `set_cursor_grab`) | yes | yes | **no** (also no fullscreen and no key events, only polled state) |
| Trial on this machine under WSLg | ran on X11 and on Wayland | **could not run:** no SDL2 library and no cmake installed, and nothing may be installed for the owner | ran on Wayland; X11 panicked (a system library is missing) | ran on both |
| Other costs | needs OpenGL 2.1 or better; owns the event loop; no CPU-buffer call (we upload a texture and draw one quad); reports minimise and restore but **no focus-lost event** | a C library in the build and at run time | the licence question; 111 crates | cannot do mouse turning |

Options: (i) miniquad; (ii) SDL2; (iii) winit with softbuffer; (iv) minifb, keyboard only. The licence fact needs care and is the owner's to weigh: the Free Software Foundation's position is that Apache-2.0 is not compatible with GPL version 2 (it is with version 3), AGENTS.md keeps this repository GPL-2.0-only, and a winit build of the app would link Apache-2.0-only code into a GPL-2.0-only program. This note records the SPDX strings and that position; it is not legal advice. sdl3 was also measured and does not compile on Rust 1.85.0.

(a) (i), (ii) and (iv) are consistent with the GPL-2.0-only rule; (iii) needs the owner to accept the licence position. (b) and (c) are not affected by the choice: the library sits behind a small interface in the one binary crate (S19), and nothing it does can reach a tic command after it is recorded. (d) yes. `unsafe` stays forbidden in our crates under every option (each trial compiled with `#![forbid(unsafe_code)]`).

**Recommendation: (i) miniquad.** It is the only measured option that is pure Rust, clean against GPL-2.0-only, builds on 1.85.0 with no system packages, captures the mouse, and actually ran here. It is also the smallest tree. Its costs are real and are stated to the player: OpenGL is required, and because it reports no focus loss, keys held while the player switches away with Alt-Tab stay held until pressed again (as in linuxdoom, which has no focus handling either; §1.1). **Fallback: (ii) SDL2**, if the owner prefers a mature C library and installs `libsdl2-dev` himself; the platform interface (S19) makes the switch a change to one file of the binary crate. What nobody has observed: a picture on the screen, a real key or mouse event, mouse capture under WSLg, anything on Windows or macOS. **Confirmed by owner 2026-11-17, with a note: see [Owner rulings](#owner-rulings-2026-11-17).**

**S3. Turning resolution and recording.** Facts (§1.2): while recording, vanilla writes the command to the demo buffer and immediately reads it back into the command the game then executes (g_game.c:1506-1523), so a recorded session turns at demo resolution: the executed turn is `256 * floor((angleturn + 128) / 256)`, and no remainder is carried to the next tic. Forward, side and buttons come back unchanged. The Rust demo writer refuses a nonzero low byte rather than rounding (demo.rs:297-299), so the app must round before it executes and before it writes. The simulation itself does not validate `angle_turn` at all (§1.6 item 7): a scratch probe saw a nonzero low byte applied in full under wire 24.

What demo resolution costs in feel. One step is 256 units, 1.40625 degrees. With the default mouse sensitivity one pixel of motion is 32 units (§1.1), so:

- up to 3 pixels of motion in a tic (about 105 pixels a second) turns nothing, and it is lost, not saved for the next tic;
- 4 pixels in a tic turns one step to the left and nothing to the right (the rounding is `+128` then floor, so it is not symmetric); 5 to 11 pixels turn one step either way;
- keyboard turns are rounded too: the slow turn of 320 becomes 256; the normal 640 becomes 768 to the left and 512 to the right; the running 1,280 stays 1,280.

Slow, precise mouse aiming is therefore coarse and slightly lopsided. That is exactly how vanilla feels under `-record`, and it is the price of every session being a valid vanilla demo.

Options:
- (i) Every session is recorded and executes the re-read command, exactly as vanilla under `-record`. Every session is then a version-110 demo that the genuine engine replays tic for tic (Phase 2 step 7), and every live command lies inside the domain the 61 real-map runs and the demo reader already cover.
- (ii) As (i), plus a full-resolution mode that is not recorded. **It is possible on wire 24 with no simulation change** (the probe above), but: such a session has no demo (version 110 cannot carry the low byte), it cannot be replayed through the genuine engine by the session tool, and no whole-engine evidence exists for a nonzero low byte (only the bespoke M3g4 scenario's `turn=32767`), so it would need its own genuine evidence first (a runner mode that injects commands directly).
- (iii) As (i), plus a remainder carried from tic to tic (what Chocolate Doom adds for low-resolution turning). The commands stay valid demo commands and slow mouse turns stop being lost, but it is not what linuxdoom-1.10 does, so "exactly as the original" would no longer hold for mouse turning.

(a) (i) is the brief's recommendation and matches the project's rule that evidence comes first; (ii) and (iii) add behaviour without genuine evidence or beyond the original. (b)–(d) yes for (i).

**Recommendation: (i) only in M5b.** (ii) and (iii) are recorded here as possible later options, each needing its own decision; (iii) would be opt-in and labelled as not vanilla. **Confirmed by owner 2026-11-17, with a note: see [Owner rulings](#owner-rulings-2026-11-17).**

### Input

**S4. The mouse event stream.** The brief asks for the original's mouse handling including linuxdoom's `i_video.c` scaling. The facts (§1.1) show that "exactly as linuxdoom" has no playable meaning for the mouse: (1) linuxdoom receives no mouse events at all unless started with `-grabmouse` (the pointer events are commented out of the window's event mask, and `use_mouse` is never read); (2) `G_Responder` **assigns** `mousex` and `mousey` from each mouse event rather than adding them up, and a button-only event sets them to zero, so when several X motion events arrive in one tic only the last one counts and the rest of the motion is lost; (3) with the pointer warped to the window centre each tic, the centre test drops every motion that lands on the centre column or row, which is every purely horizontal or purely vertical move (i_video.c:257-264); (4) a mask slip leaves fire on after the left button is released (i_video.c:218, 231, 247). The input prototype observed (2), (3) and (4) in the genuine code (§2.2.4). The DOS mouse code, which delivered one accumulated event per tic, is not in this source.

Options: (i) the app posts **one mouse event per command build**, carrying the motion accumulated since the previous build with linuxdoom's scaling (`<< 2`) and the current button state; everything after that point (`G_Responder`'s sensitivity arithmetic, `G_BuildTiccmd`'s turn, strafe, forward movement, double-click use and clamps) is the genuine path. This is a legal input to the genuine code: it is exactly what the X path produces when one motion event arrives per tic, and the input driver feeds the genuine code the same stream (§2.2). (ii) Reproduce the X stream event for event, last event wins, losing motion. (iii) No mouse by default, as linuxdoom without `-grabmouse`.

(a) (i) fits the brief's intent (keyboard and mouse) and its rule that only commands the original can produce are produced; (b) yes: the choice is which event stream to post, declared in COMPATIBILITY, and the event-to-command step stays exact; (c) the quirks (3) and (4) of the X layer are not reproduced, declared; the genuine `i_video.c` translation still compiles into the input driver against a small fake Xlib, so its behaviour is recorded as evidence even though the app does not imitate it; (d) yes: the motion posted per command is clamped to 319 pixels each way, the most a 320-pixel-wide linuxdoom window can report in one event, which also keeps the turn far from the 16-bit wrap.

**Recommendation: (i).** One raw mouse count is treated as one X pixel; the genuine `mouse_sensitivity` (default 5) is the sensitivity control. Vertical mouse motion moves the player forward and back, as in the original, under the vanilla bindings (the modern preset of S7 can switch it off). **Confirmed by owner 2026-11-17.**

**S5. Keys that never reach the game path.** In a level the genuine responders act on several keys before the game keys (§1.1.2): Esc opens the menu; F1–F11 open menu screens, toggle messages (F8) or change gamma (F11); `-` and `=` change the screen size; Enter re-shows the last message (`HU_Responder`); Tab opens the automap; Pause makes the next command carry `BT_SPECIAL`, which the simulation refuses; every key press also feeds the cheat matcher, which writes player fields outside any tic command; and `q` ends a recording and exits (g_game.c, `G_WriteDemoTiccmd`). Options: (i) the app's translation layer never forwards any of these to the game path: Esc and Pause are the app's own pause (S12), the others do nothing in M5b; the cheat matcher is not ported, and a test proves no key can reach player fields; (ii) implement the in-scope genuine effects now (Enter's message refresh, F8, F11, `-`/`=`).

(a) (i) follows the brief's out-of-scope list (menus, automap, cheats, vanilla pause); Enter's refresh is HUD behaviour, but it is input that is not in the demo, so a replayed session could not reproduce its frames; (b) yes: a withheld key is not an input, and with these keys withheld the genuine path cannot emit `BT_SPECIAL` or a chat character (§2.2 shows it); (c)–(d) yes.

**Recommendation: (i).** `show_messages` stays at its default 1, gamma at 0 (S13), the screen size is fixed for the session (S14). Enter's refresh, F8 and F11 belong with the menus milestone. **Confirmed by owner 2026-11-17.**

**S6. Key repeat, focus loss and keyboard layout.** linuxdoom has no handling for any of them (§1.1.7): X auto-repeat arrives as release-and-press pairs, a key released while the window is unfocused stays down, both Shift keys (and both Ctrl, both Alt) are one key code, and letters follow the keyboard layout. The prototype also showed that a key or button pressed and released between two command builds is never seen by the original, fire, use and weapon keys included (§2.2.4). Options: (i) the translation layer drops repeated key-downs (the stream a keyboard with auto-repeat off produces); a release that arrives in the same pump as its own press is held back to the next pump, so that a tap shorter than a tic still lasts one tic (the stream the original would have received had the release come 28 ms later); on the app's pause (S12) and wherever the library reports minimise or focus loss it posts a key-up for every held key and a zero mouse event; left and right modifiers are one key as in the original; keys are identified as the library reports them; (ii) forward everything as it arrives: auto-repeat pairs, lost taps and stuck keys, as on linuxdoom.

(a)–(d) yes for (i): only the event stream is chosen, every stream is one the original could have received, and the event-to-command path is untouched. **Recommendation: (i).** With miniquad (S2) there is no focus-lost event, so a key held across Alt-Tab stays held until pressed again; pressing Esc first releases everything. That limit is stated in the README. **Confirmed by owner 2026-11-17.**

**S7. Key bindings.** Options: (i) vanilla defaults only (§1.1.6: arrow keys, Ctrl fire, Space use, Shift run, Alt strafe, `,` and `.` strafe, `1`–`8` weapons; mouse: left fire, middle strafe, right forward); (ii) the defaults plus one optional modern preset, selected at launch.

The preset is built only from things the original accepts, which is the proof that it produces only commands `G_BuildTiccmd` can produce:
- **Bindings** are values of the original's own configuration variables (`key_up` = `w`, `key_down` = `s`, `key_strafeleft` = `a`, `key_straferight` = `d`, `key_use` = `e`; `mouseb_strafe` and `mouseb_forward` = −1, an index the source explicitly allows, `mousearray[0]`).
- **Always run** is the translation layer holding `key_speed` down: it posts the key-down once at the start and inverts the physical Shift key. The source has no always-run setting (§1.1.5), and a held Shift is the only genuine form; the slow turn of the first tics of a keyboard turn still applies, as it does in the original with Shift held.
- **No forward movement from the mouse** is the translation layer posting a vertical motion of zero, which is what a mouse moved only sideways produces.

So the preset is a configuration plus a rewrite of the event stream, both ahead of the genuine path; it never touches a command. The evidence is direct: the preset's configuration and its rewritten event scripts run through the genuine input driver and through the Rust input code, and the commands are compared field by field like every other script (§2.2).

(a)–(d) yes for (ii). **Recommendation: (ii)**, vanilla defaults as the default. No configuration file is read in M5b; bindings are one of the two presets. The preset is the first item to move to a follow-up if the size cap is passed (§5.3). **Confirmed by owner 2026-11-17.**

### The loop and the session

**S8. Pacing and slow frames.** Vanilla (§1.3): tics come from the wall clock at 35 a second with integer truncation; the loop never sleeps; after a slow frame at most 5 tics run before the next display and the rest of the lost time is dropped, never recovered; events are pumped just before each command is built; one picture is drawn per loop pass, however many tics ran. The M5a render contract (R8) is different on purpose: the picture is drawn after every tic, because `fuzzpos`, uncovered pixels and now the status bar's incremental drawing depend on which frames were drawn.

Options: (i) 35 Hz from a monotonic clock with vanilla's integer arithmetic; the loop sleeps until the next tic is due; for each due tic: pump events, build the command, round it as recording does (S3), advance, run the status bar and HUD tickers, draw the frame; if more than 5 tics are due, run 5 and drop the rest (vanilla's bound); every executed tic is drawn, and only the last frame of a batch is shown; (ii) port `TryRunTics` exactly, drawing once per batch.

(a) (i) is the brief ("one rendered frame per tic, clean handling of a slow frame") and R8; (b) yes: under (i) the frames of any session equal the frames the harness draws when it replays the session's demo, which is what makes session verification possible; under (ii) they would depend on the machine's speed; (c) the difference from vanilla's draw-once-per-batch is a declared presentation contract (it exists since R8); (d) yes: at most 5 tics of work per pass.

**Recommendation: (i).** The first frame shown is the one after tic 1, as in the harness. **Confirmed by owner 2026-11-17.**

**S9. A refused tic during play (a witness fires).** Options: (i) the game stops on a plain screen of the app's own that names the witness and the tic; the session demo is saved up to the last good tic; nothing is approximated; restart or quit; (ii) skip or retry with a neutral command. (a) (i) is the refuse-the-tic rule (H3) carried to the app; (b)–(d) yes. **Recommendation: (i).** A refused frame (a render witness of M5a or the new ones of S17) is handled the same way. **Confirmed by owner 2026-11-17.**

**S10. Level end and death.** The simulation ends a level run at the exit tic (normal exit, secret exit, E1M8's ending) and refuses the next tic (`LevelExited`); a dead player's use press is the last tic, and the next is refused (`PlayerReborn`). Options: (i) an end screen of the app's own, clearly not vanilla (its own font and layout, the words "level complete" or "you died", the tic count and the path of the saved demo), offering restart or quit; no intermission, no tally, no next level; (ii) imitate the intermission. (a) (i) is the brief; (b)–(d) yes. **Recommendation: (i).** While the player is dead and has not pressed use, play continues as in the original (the view sinks, the face is dead). **Confirmed by owner 2026-11-17.**

**S11. What restart means.** Vanilla's reload after death keeps process state: both random indices, `gametic`, and the status bar, HUD and input variables that no level start resets (`st_facecount`, the face `priority` and `lastattackdown`, `message_counter`, `turnheld`; §1.1, §1.4, §1.5). The simulation refuses the reborn, and every genuine run of the harness starts in a fresh process. Options: (i) restart is a new game: a fresh simulation at tic 0, both random indices 0, every status bar, HUD and input variable as at process start, and a new demo file; (ii) carry the process state across, as vanilla's reborn does. (a) (i) matches how every piece of evidence is produced; (b) yes: each session is exactly one fresh harness run; (c) differs from vanilla's reborn, which is not modelled anyway (declared); (d) yes. **Recommendation: (i).** One consequence: the level-load clear of the input arrays (g_game.c:495-496, whose `sizeof` slip clears a different number of mouse buttons on the 32-bit original and on a 64-bit build; §2.2.5) always runs on state that is already zero, so the difference cannot show; the input driver still declares the one-line adapter so that its 64-bit and 32-bit builds agree. **Confirmed by owner 2026-11-17.**

**S12. Pause.** Options: (i) an app-level pause: Esc or the Pause key (and minimise or focus loss where the library reports it) stops the loop from advancing tics; the simulation, the render state and the demo are untouched; held keys are released (S6); the clock is re-based on resume; the app draws its own "paused" overlay on the presented picture, never into the proven frame; from the paused screen the player can resume, restart or quit; (ii) no pause at all. Vanilla's own pause is a tic command (`BT_SPECIAL`), keeps the status bar and HUD tickers running and is not admitted. (a) the brief allows either; (b)–(d) yes for (i): a paused session's demo and frames are those of the same commands played without the pause. **Recommendation: (i).** **Confirmed by owner 2026-11-17.**

### Presentation

**S13. Presentation.** Options: (i) 35 frames a second; the 320×200 frame scaled by nearest neighbour; 4:3 aspect correction on by default (the picture shown as 320×240) with a square-pixel option; windowed by default and borderless fullscreen as an option; the palette applied at presentation through the original's gamma table 0, which is not the identity (§1.3.5), with the other four genuine tables selectable at launch; (ii) raw palette, integer scaling only. **The proven artefact is the 320×200 indexed frame and the palette number per tic; scaling, aspect, gamma, vsync and whatever the display does are outside that claim.** (a)–(d) yes. **Recommendation: (i).** **Confirmed by owner 2026-11-17.**

**S14. Screen sizes.** Options: (i) screenblocks 10 (with the status bar) by default and 11 (full screen), chosen at launch and fixed for the session; (ii) also 9 and below; (iii) live switching with `-` and `=`. At 10 and 11 no border is drawn and the HUD erase does nothing (§1.3.4, §1.5.6), which is what keeps the frame contract small; 9 and below need the border and the HUD's erase path and were verified only on synthetic rooms in M5a. (a) (i) is the brief; (b)–(d) yes. **Recommendation: (i).** **Confirmed by owner 2026-11-17.**

### Fidelity contracts that need a ruling

**S15. The face's random glance (`M_Random`): the one input that cannot be derived as vanilla has it.** When nothing else claims the face, it looks left, ahead or right by `st_randomnumber % 3`, re-chosen every 17 tics, and `st_randomnumber` is drawn from `M_Random` once per tic (st_stuff.c:992, 913-919). In a real vanilla session the same index is also advanced by every started sound that passes its audibility test (`S_StartSoundAtVolume`, s_sound.c:283-343; the test depends on the positions of the listener and the origin at the moment of the call, in the middle of the tic, and on the configured sound volume) and by the level-start melt (320 draws; §1.3.6). `M_Random` never feeds game logic (§1.3.6 lists every call site), so nothing but the glance is affected. The simulation never draws it, sound and wipes are out of M5b, and the genuine runners have always stubbed both draws (m5a_stubs.c:26-31).

How much it matters was measured (§2.1, finding F1): an experimental build of the runner that compiles `s_sound.c` whole and adds the melt's draws leaves the game dumps identical and changes the face in **8,517 of 13,391 synthetic frames and 69,139 of 158,966 real-map frames** (43%), on all 61 real runs. So under any model without sound, the idle face often looks a different way than a vanilla session would show; pain, ouch, grin, rampage and dead faces, every number, key and message, and the palette are not affected.

Options: (i) a **declared sound-free, wipe-free model**: the index starts at 0 at a new game and only the status bar ticker advances it, in the genuine runner and in Rust alike, so frames compare exactly under that declared contract; COMPATIBILITY says that the idle face's left, ahead or right choice differs from a vanilla session with sound; it is revisited when sound arrives (M5c), under a new saved-state version; (ii) **vanilla accounting now**: the runner compiles `s_sound.c` over a stubbed sound layer (prototyped), and Rust reproduces one draw per qualifying sound start, which needs every sound start's identity and origin position and the listener's position at that moment; the simulation does not expose mid-tic positions and its sound events were not shown to be complete, so this may need a simulation change, which the brief forbids; (iii) leave the idle face out of the pixel comparison.

(a) (i) follows R8's pattern (a declared sequence contract for presentation state); (b) yes: declared, and exact under the contract; (c) it is a new declared presentation gap against real vanilla sessions, which is why it needs a ruling; (d) yes. **Recommendation: (i).** This is the brief's "can't be done without a simulation change: stop and ask" case as far as exact vanilla fidelity of the glance goes, and (i) is how to stay inside the brief. Sound has to model the same sound starts anyway, so (ii) belongs with it. **Confirmed by owner 2026-11-17, with a note: see [Owner rulings](#owner-rulings-2026-11-17).**

**S16. The game dump when the genuine status bar and HUD run.** The brief expected a proof that running them leaves every game dump unchanged. Measured on the 48 synthetic items and the 61 real runs (§2.1.2): **exactly two dump columns change and nothing else.**

- `H.rndindex` changes on every tic of every item, because `ST_Ticker` draws `M_Random` once per tic and the dump header prints that index. In single player it is presentation state: its only gameplay reader is the netgame consistency check (g_game.c:680-692).
- `P.message` changes on the tic a message is taken (26 cells, in 13 of the 48 synthetic items), because the genuine `HU_Ticker` clears the field inside the tic (hu_stuff.c:519-530). The frozen runners' stub cleared it too but kept the pointer for the dump.

Options: (i) **record and mask, in the new runner only**: a wrapper records the message `HU_Ticker` found, and the dump prints that; a wrapper counts the presentation draws of `M_Random`, and the dump prints the index without them (the value the frozen runners print); the genuine index and the draw count travel in the frame stream; both are declared in the new runner's contract beside R5's `ML_MAPPED` mask, and no frozen runner, contract or result file changes; (ii) take the dump inside the tic, before the status bar ticker: this fixes the message but not the index; (iii) save and restore the index around `ST_Ticker`: rejected, because it changes the face.

Observed with (i): the never-render dumps equal the committed or frozen digests on 48 of 48 synthetic items and 61 of 61 real runs; the render-every dumps equal the never-render ones snapshot for snapshot at screenblocks 10 and 11; and the 3D view of every message-free frame equals its M5a golden (11,791 equal, 0 differ).

(a) R5's pattern, applied to two more fields; (b) yes: both values are still compared, in the frame stream; (c) no new gap; (d) yes. **Recommendation: (i).** **Confirmed by owner 2026-11-17, with a note: see [Owner rulings](#owner-rulings-2026-11-17).**

**S17. Derived inputs, and refusals where the original reads undefined memory.** Everything else the status bar, HUD and palette read is in canonical state or exactly derivable (§1.6). Three points need a ruling because they add observers or refusals:

- **The message.** The Rust state keeps the last message code and never clears it, so a repeat of the same message is invisible in state alone; the C shows each assignment anew (restarting its 140-tic timeout) and `HU_Ticker` clears the field (§1.5.3–1.5.4). The assignment is exactly derivable per tic from the tic's events and the command as fed (§1.6 item 5): a pickup event for pickups, the locked-door sound and call pair for the six locked-door lines, and the tic number and command for the turbo message. Proposed: a HUD-side observer derives it, the result is cross-checked against the stored code every tic, and any mismatch refuses the frame (`MessageUnderivable`) rather than guessing. The genuine per-tic message is already in the runners' dumps, so the derivation is compared against the C on every synthetic and real-map tic.
- **The attacker.** `plyr->attacker` is never cleared when its thing is removed, and the face then reads freed memory (§1.4). Under wire 24 no attacker is ever removed (monsters stay as corpses; a fireball's or a barrel's damage is credited to the shooter), so it should be unreachable. Proposed: if the attacker no longer resolves, refuse the frame (`AttackerRemoved`), as the simulation already does for the same pointer in `P_DeathThink`.
- **Status bar and font graphics the original mishandles.** With patches of unusual size or offsets the C silently skips a patch that would leave the screen, copies from a background buffer it never cleared, or stops with `I_Error` (§1.4, §1.5.9). Proposed: reproduce what is deterministic (the silent skip, the stale pixels) and refuse the frame with a named witness where the original stops or reads outside an allocation, as R3 and R4 did for the renderer. The IWAD's own graphics are expected to reach none of these; the real-map runs will say.

(a) the pattern of R3, R4, R6 and R7; (b) yes: a refusal, never a guess; (c) three new witness kinds, each expected to be unreachable on real maps; if any fires on a real map, work stops and the owner is told; (d) yes. **Recommendation: as proposed.** **Confirmed by owner 2026-11-17.**

**S18. The saved display state.** The status bar and HUD history (about 70 values, the two 320×32 status bar buffers, the message line, the `M_Random` index) must be saved and restored beside a snapshot so that a rewind continues to the same frames. Options: (i) a new block with its own magic that embeds the DRST version 2 bytes unchanged, so the version 2 reader, its tests and every M5a golden keep their meaning; (ii) DRST version 3 with the version 2 decoder kept frozen. (a) both keep historical schemas unchanged; (b)–(d) yes. **Recommendation: (i)**, documented in STATE_FORMAT, with the M5a restore-and-continue test extended to the status bar, the HUD and the palette. **Confirmed by owner 2026-11-17.**

### Structure, evidence and delivery

**S19. Crates.** Options: (i) the status bar, HUD, patch drawing, palette choice and frame composition as new modules of `doom-render` (they share its buffer, resources and render state); a new library crate `doom-game` with no platform code for the input path, the recorder, the loop with an injectable clock, the app's own screens and font, and the `Platform` and `Clock` interfaces (events in; a 320×200 indexed frame and a palette out); a new binary crate `doom-play`, the only crate that depends on the windowing library; (ii) one binary crate for everything outside `doom-render`, as the brief first proposed. (i) lets fuzzing, mutation testing and the headless tests reach the input code and the loop without building the window library. `doom-sim` is not edited; `doom-render`'s public API only grows. (a)–(d) yes. **Recommendation: (i)** (§3). **Confirmed by owner 2026-11-17.**

**S20. How the owner plays on Windows.** Facts (§3.13): the Linux build opens a window under WSLg today; whether WSLg captures the mouse properly is unknown, and the only public record found is an old open issue reporting that it did not; latency under WSLg was not measured and cannot be judged by an agent; nothing on this machine can cross-compile to Windows; the existing Windows CI job compiles the workspace but publishes nothing. Options: (i) WSLg only; (ii) a native Windows executable built by a CI job on a Windows runner and uploaded as an artifact the owner downloads himself, triggered by a label on the pull request and by a manual run after merge, not a required check; (iii) both.

(a) the brief's two options; (b)–(d) do not apply to gameplay. **Recommendation: (iii).** WSLg is the first play (Phase 2 step 8 confirms that the window opens); the native build is what to use if the mouse or the latency under WSLg is poor, and it costs one small workflow. The IWAD is always a path the player supplies; nothing from it is bundled, uploaded or committed. The artifact job is not a fallback item and stays in M5b (see [Owner rulings](#owner-rulings-2026-11-17) and §5.3). **Confirmed by owner 2026-11-17, with a note: see [Owner rulings](#owner-rulings-2026-11-17).**

**S21. Where sessions are saved, how long they may be, and their header.** Options for the place: (i) a `--sessions DIR` argument; the documented command for the owner's machine passes `/mnt/d/DoomSessions`, the app creates the directory when he runs it, and without the argument the app writes to `doom-sessions/` in the current directory; (ii) hard-code `/mnt/d/DoomSessions`. A session demo holds only the player's own tic commands and a 13-byte header, nothing from the IWAD, so committing one is allowed; none is committed by default, and agents' own trial runs write to scratch.

- **File.** `E1M<map>-skill<n>-<yyyymmdd-hhmmss>.lmp` (local time, used for the name only), rewritten whole (with its end marker) at intervals and at every end, so that a crash loses at most the last interval.
- **Header.** Version 110, skill 2 or 3, episode 1, map 1–9, single player, no flags: the domain the existing reader and writer already have (§1.2.6), so no new format.
- **Length.** Vanilla's recording buffer ends a session at 131,072 bytes (about 15.6 minutes) unless `-maxdemo` raises it; playback has no such limit. The app does not reproduce that limit; it bounds a session at 1,048,576 tics (about 8.3 hours, 4 MB) and then stops on a plain screen.

(a)–(d) yes. **Recommendation: (i)** with the file, header and length as stated. **Confirmed by owner 2026-11-17.**

**S22. Harness and CI.** Both prototypes ran (§2.1, §2.2); Phase 2 rebuilds them as committed tools. New files only: no m3h9c, m4a, m4b or m5a driver, contract or result file changes.

- **Full frames.** A fork of the M5a runner (`m5b_driver.c`, `m5b_stubs.c`, `m5b_build.py`, contract schema `DOOM-M5B-RUNNER-v1`) that compiles the status bar and HUD whole (46 genuine files in the prototype: M5a's 41 and `st_stuff.c`, `st_lib.c`, `hu_stuff.c`, `hu_lib.c`, `m_cheat.c`), runs their tickers after `P_Ticker` and their drawers in `D_Display`'s order (`ST_Drawer` before the 3D view, `HU_Drawer` after it), and records the palette at `I_SetPalette`. Goldens: the digest of the whole 320×200 frame and one row per frame with the palette number, the face index, the message flag and the `M_Random` index, at screenblocks 10 and 11, from the strict build; the overflow and address builds must be identical. A standing check requires the 3D view of every message-free frame to equal its M5a golden. The prototype found no sanitizer finding and no undefined read in the status bar, HUD or patch-drawing code, on synthetic or real runs.
- **Input.** A small dedicated driver (`m5b_input_driver.c`; 23 genuine files in the prototype, no WAD needed, contract schema `DOOM-M5B-INPUT-v1`) that feeds scripted events through the genuine `D_PostEvent`, `D_ProcessEvents`, the responders, `G_BuildTiccmd` and the demo write and re-read, with linuxdoom's `i_video.c` compiled whole against a small declared fake Xlib so that X-level scripts record the genuine translation too. It needs 15 one-line LP64 adapters in `m_misc.c` (its defaults table stores string pointers in an `int`) and the one at g_game.c:495-496 (S11). Goldens per tic: the event stream after translation, the built command and the executed (re-read) command. About 200 scripts and 10,000 tics after trimming the prototype's 327.
- **Sessions.** One tool for CI on synthetic maps and one for IWAD maps (local only): the app's demo must be byte-identical to the demo the genuine input driver records from the same script; the genuine engine and the Rust engine replay it and every tic's dump, frame and palette must match; and the state the app reached while playing must equal the replay of its own demo (§2.3).
- **Self-tests.** Each comparison must be shown to fail: a flipped status bar pixel, a flipped palette number, a flipped command bit, a perturbed event.
- **CI.** The Rust side (frames, palette numbers, input scripts, the headless app) runs inside the existing `Stable` jobs' `cargo test` on Linux, Windows and macOS, and the app builds there as a workspace member. The genuine side runs on Linux in one new job, proposed name `M5b genuine status bar, HUD and input`; the `M5a genuine renderer` job and every required check name stay as they are, and whether the new job becomes required is the owner's call. Real maps stay local.

(a) R11 and R15's pattern; (b)–(d) yes. **Recommendation: as stated.** **Confirmed by owner 2026-11-17.**

**S23. The synthetic graphics set and the new rooms.** The genuine status bar and HUD ask for 142 lumps besides PLAYPAL (§2.4.1): `STBAR`, 10 large digits, the minus and percent signs, 10 small digits, 6 arms digits, 6 key icons, `STARMS`, one face background, 42 faces and the 63 font glyphs. Proposed: `m5b_gfx.py` appends our own drawings of all 142 to the M5a set (no id art; our own sizes and offsets, chosen from the positions the C fixes and never read from the IWAD; each face state, digit and glyph visibly distinct; a legible font), leaving the M5a part of the WAD, PLAYPAL (which already has 14 palettes) and COLORMAP byte-identical, so that every M5a golden keeps its meaning; an independent `m5b_check_gfx.py` re-parses the result. About nine new rooms and 10,000 tics (§2.4.4) reach what the existing 48 items do not: every digit position and the blank ammo field, the arms and key icons, the backpack, pain from the left, right and ahead at each health level, the ouch face (which the source shows on a health *gain* above 20 while hurt, not on a big hit), the evil grin, the rampage, death, the look-around cycle, each red and bonus palette, the radiation suit to its blinking end, and every message kind with its timeout, a repeat and a replacement.

What the source makes unreachable, so that the targets are stated correctly (§2.4.3): **12 of the 14 palettes** can be chosen (1 and 9 never are; st_stuff.c:1018-1043); **41 of the 42 faces** (the god face needs invulnerability or a cheat); the minus sign is loaded and never drawn in single player; berserk is not admitted by wire 24. The berserk term and the god face are covered by unit tests and one bespoke C scenario (S1).

Committed goldens grow by about 5 to 9 MB of generated text (the M5a frame goldens are 4.3 MB today), reported separately from the hand-written size (§5.2). (a) R13's pattern; (b)–(d) yes. **Recommendation: as stated.** **Confirmed by owner 2026-11-17.**

### Why Phase 1 stopped

This is kept as history. The brief made S2 and S3 owner decisions whatever was recommended, so Phase 1 stopped here, and the owner's rulings arrived on 2026-11-17 (see [Owner rulings](#owner-rulings-2026-11-17)). Four more items would have stopped it under the unattended rule:

- **S2** found a fact the brief did not anticipate: the library named first in the brief (winit) carries Apache-2.0-only crates, which the GPL-2.0-only rule may not admit.
- **S4**: "the original's mouse handling" does not exist in a playable form in this source, so a choice of event stream is needed.
- **S15**: exact vanilla fidelity of the face's idle glance cannot be had without sound and may need a simulation change; the recommendation is a declared contract instead.
- **S16**: the proof the brief asked for (every game dump unchanged) does not hold as worded: two dump fields change, and the recommendation is a declared record-and-mask in the new runner, as R5 did for `ML_MAPPED`.

Nothing in the simulation needed to change under any recommendation.

## 5. Plan, size estimate and cap

### 5.1 Steps

Phase 2 follows the brief's fifteen steps, one commit per step, pushed, with the status line at the top of this note updated after each. Each step runs in a fresh named subagent on its pinned model (AGENTS.md, model routing); one writer on the tree at a time.

| Step | What | Agent |
| --- | --- | --- |
| 1 | Synthetic status bar, font and 14-palette graphics (S23); new rooms and scripts for every widget, every reachable face state, every message kind with its timeout and every reachable palette; the full-frame genuine runner (S22) with frames and palette numbers in the strict, overflow and address variants, identical; goldens from the C only | c-evidence |
| 2 | The genuine input driver, the event scripts (vanilla bindings and the modern preset of S7) and their golden commands, before and after the recording round trip | c-evidence |
| 3 | Rust: patch drawing, the status bar, the HUD with its message observer, the palette choice and frame composition in `doom-render`, each matched against its goldens before the next; the saved display state (S18) and the extended restore-and-continue test | c-evidence |
| 4 | Rust: the input code in `doom-game`, matched command for command against the goldens | c-evidence |
| 5 | Real maps, local only: full frames and palette numbers on all 61 runs at screenblocks 10 and 11, every tic; the derived message compared with the genuine one on every tic; every divergence explained from the C, fixed and reproduced by a synthetic room | c-evidence |
| 6 | The app: `doom-game`'s loop, recorder, pause, end and refusal screens and restart; `doom-play`'s window and event translation; headless tests with scripted events and a fake clock (same events in, same frames, palette numbers and demo bytes out, twice) | c-evidence |
| 7 | Session verification: a scripted session through the app's own input path on a synthetic map, its demo replayed through the genuine engine, every tic and frame matching; the same tool run locally on a real-map session | c-evidence |
| 7b | The full-resolution unrecorded turning mode (S3 (ii)), done only if the size cap allows once steps 1–7 are in, otherwise the first item of M5c. It needs its own genuine evidence first: a runner mode that injects tic commands directly; synthetic scripts with nonzero low bytes of `angleturn` across the mouse and keyboard ranges; all three variants; matched in Rust. Opt-in, labelled "not recorded: no demo, cannot be replayed", never the default | c-evidence |
| 8 | The first play: build, confirm that a window opens under WSLg, write the exact commands into the final report and the README; say what was and was not checked | docs-tools |
| 9 | CI: synthetic frames and input scripts on Linux, Windows and macOS; the genuine status bar, HUD and input drivers on Linux; the app builds on all three; the Windows artifact job (S20). No required check name is added without telling the owner | docs-tools |
| 10 | Docs: COMPATIBILITY, ARCHITECTURE, STATE_FORMAT, TESTING, ROADMAP, `docs/validation/M5B.md`, the README (what is playable, how to run it, under about 220 lines) and the new animation (a synthetic room with the status bar, a pickup message and a damage flash, C beside Rust with the difference panel), generated by a committed script | docs-tools |
| 11 | Mutation testing through the `run-mutants` label, in enough shards that none runs past about 40 minutes; every survivor dispositioned; the reviewer at xhigh effort checks every equivalent and unreachable disposition | tester, reviewer |
| 12 | Fuzz: a target for the input code (random event streams: no panic, only admitted commands, deterministic) and one for the status bar and HUD state; seed replay, coverage-guided and AddressSanitizer campaigns, reported separately | tester |
| 13 | Timings from ext4 on an idle machine: frame time with the status bar and HUD; the loop's tic jitter over a few thousand tics with a real clock; recorded, not gated | docs-tools |
| 14 | Independent review at xhigh effort: the input code against `G_BuildTiccmd` line by line, the status bar and HUD against the C, that the simulation is untouched, that nothing the app does can change a tic command after it is recorded, and overclaiming in the docs | reviewer |
| 15 | Full gates from clean on ext4, the PR title and body, CI green on the exact head, mark the PR ready, stop | docs-tools |

How Phase 2 is run: two tracks at first. Step 1, then step 3, in the main tree; step 2, then step 4, in a detached worktree at `~/scratch/m5b/wt-input`, brought onto the branch by cherry-pick when the main tree is quiet. Never two writers on one tree. From step 5 and 6 on, a single track.

### 5.2 Size

Hand-written lines, projected:

| Part | Lines | Basis |
| --- | ---: | --- |
| Rust in `doom-render` (patch drawing, status bar, HUD, palette, composition, saved state) | 2,750 | §3.11 |
| Rust in `doom-game` and `doom-play` (input, recorder, loop, screens and font, platform shim) | 2,000 | §3.11 |
| Rust tests and fuzz targets | 1,950 | §3.11 |
| Tools: the full-frame runner fork, the input driver with its fake Xlib and stubs, graphics and room generators, reference and session tools, the local real-map tool, the animation script | 4,000–6,000 | the two prototypes (§2); M5a's tools came to 3,000–4,500 |
| Docs: this note (about 5,000 lines, most of it the C facts), the evidence record, COMPATIBILITY, ARCHITECTURE, STATE_FORMAT, TESTING, ROADMAP, README | 6,500–7,500 | M5a's note was 2,726 lines |

The Rust figures are the design's estimate of about 6,700 lines; M5a's Rust landed at about twice its estimate, so the Rust range is taken as 6,700–13,000. **Projection: about 17,000–26,000 lines against a cap of 30,000** on hand-written code, tools and docs. Generated data verified by `--check` tools is reported separately: the frame and palette goldens (about 5–9 MB of text), the input scripts' goldens (about 0.4 MB), the contracts, results files, the graphics PWAD, the room WADs and the committed synthetic sessions.

### 5.3 Fallback

Per the owner's rulings ([Owner rulings](#owner-rulings-2026-11-17)): if the projection passes the cap, the modern bindings preset (S7) moves to a follow-up first. The Windows artifact job (S20) is not a fallback item and stays in M5b. Step 7b (the full-resolution unrecorded turning mode) is attempted only if the cap allows, and otherwise becomes the first item of M5c. The status bar, HUD, palette, input with the vanilla bindings, the loop, recording, session verification, the real-map check and the Windows artifact job stay in M5b.

## 6. Open questions and where each is decided

| # | Question | Decided |
| --- | --- | --- |
| 1 | Which window library, given the licence finding | Confirmed by the owner 2026-11-17 (S2) |
| 2 | Demo-resolution turning for every session | Confirmed by the owner 2026-11-17 (S3) |
| 3 | One accumulated mouse event per command | Confirmed by the owner 2026-11-17 (S4) |
| 4 | The declared sound-free, wipe-free `M_Random` contract for the face | Confirmed by the owner 2026-11-17 (S15) |
| 5 | The game-dump projection when the genuine HUD runs | Confirmed by the owner 2026-11-17 (S16) |
| 6 | Whether mouse capture works under WSLg, and how the game feels | Phase 2 step 8, by the owner at the keyboard; no agent can observe it |
| 7 | Whether any status bar, font or attacker refusal of S17 is reached on a real map | Phase 2 step 5; if one fires, work stops and the owner is told |
| 8 | Whether the simulation's sound events are complete enough for a later sound model | Not needed under S15 (i); a question for the sound milestone |
| 9 | Whether AGENTS.md's limit of 2 parallel jobs for the C reference suite still applies under the brief's "no CPU caps or job limits" | The owner. In Phase 1 the reference suite itself was not run, and the full-frame prototype ran its genuine runner with 6 parallel jobs (§2.1.2, §2.1.8) under the brief's wording. Phase 2 holds every genuine C run to 2 parallel jobs until the owner answers |
| 10 | Whether the new CI job becomes a required check | The owner, after Phase 2 step 9 |
| 11 | Whether the first-frame palette upload and gamma table of linuxdoom are worth imitating beyond S13 | S13; presentation only |
