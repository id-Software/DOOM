# M5c: sound effects matched to the genuine engine, and the face-glance gap closed: reference investigation

Phase 1 note for milestone M5c, which gives the playable build Doom's sound effects. Which sounds start, on which channel, at what volume, stereo position and pitch is to be identical to linuxdoom-1.10 tic for tic, and the mixed samples are to match the original mixer under a declared mixing schedule. With sound starts modelled, the status bar face's idle glance is to match a vanilla session with sound, which closes decision S15 of [M5b](M5B_REFERENCE_INVESTIGATION.md#4-owner-decisions-s1s23-with-recommendations). Music, menus and their sounds, the automap, intermission and level-to-level play, save and load, sound options screens and the external `sndserver` process are out of scope. This note records the original C facts (the sound logic, every call that starts or stops a sound, the mixer, `M_Random` and the wipe), what the Rust simulation emits today, the genuine-engine harness designs with the results of two scratch prototypes, the Rust design, a measured comparison of audio libraries, the owner decisions T1–T16 with one recommendation each, and the Phase 2 plan, size estimate and cap.

The behavioural authority is the original id Software Linux DOOM 1.10 at `a77dfb96cb91780ca334d0d4cfd86957558007e0`. Milestone status lives in [ROADMAP](ROADMAP.md), support in [COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md) and commands in [TESTING](TESTING.md) (see the [documentation authority map](README.md#documentation-authority)). Observations of this milestone are in the [M5c evidence record](validation/M5C.md). This note records only counts, pass/fail facts and digests from runs against the IWAD: no lump content, no IWAD-derived names, sizes, coordinates or geometry, and never audio. All dates are America/Los_Angeles.

**Merged since: PR #63, by the owner, on 2026-11-19. The status below is as written before that.** **Status / next step (2026-11-19 14:53 PST).** **M5c is complete on the branch and waits for the owner's review.** Pull request [#63](https://github.com/Copgunn/DOOM/pull/63) is marked ready once CI and the Windows artifact have run on this head; GitHub is the record of that, and nobody but the owner merges it. The owner ruled four times: on T1–T16 ([Owner rulings](#owner-rulings-2026-11-18)), on the open points ([second set](#owner-rulings-on-the-open-points-2026-11-18-second-set)), after his playtest ([third set](#the-owners-playtest-and-rulings-2026-11-18-third-set)) and after his Windows retest ([fourth set](#the-owners-windows-retest-and-rulings-2026-11-19-fourth-set)). **Done** ([§6.1](#61-steps); observations in the [evidence record](validation/M5C.md)): steps 1 to 14. **The final gates** ran from a clean clone at `a15e0dc` on 2026-11-19 and every gate passed ([Step 14](validation/M5C.md#step-14-final-gates-from-clean)): build, format, Clippy, rustdoc and 1,876 workspace tests; the release tests; Rust 1.85.0; the instruction counts; the 64 Linux steps of the Stable job; the M5a, M5b and M5c genuine-engine jobs with their self-tests; all 60 fuzz targets built, 7 seed replays and 10 bounded campaigns on the five M5c targets with no finding (leak detection on in the AddressSanitizer campaigns; disabled in every address variant of the genuine C runs, which are therefore not LeakSanitizer coverage); the 61 real Episode 1 runs through every local tool (158,966 tics, 0 sound rows differing, 0 mismatched frames, 98,578 mixer buffers equal); and the owner's 13 hand-played session demos (27,146 tics, 13 of 13 passing). The run kept the in-use limit of 2 parallel C reference jobs. **Mutation testing** is closed (confirming run at `937cf9d`: 879 caught, 14 missed and all 14 judged equivalent or unreachable, 3 timeouts, 103 unviable). **Frozen-wire sound faults found: 1** (the hard-death scream of wires 22 to 24, fixed in wire 25 only). **Layout-dependent sessions: 0.** **Known limits found by the owner's playtest and recorded, not fixed, by his rulings:** fire can stick on Windows when a left-button release is lost at the window's edge (his input log: 18 downs, 15 ups; [record](validation/M5C.md#the-owners-windows-retest-with-the-input-log-2026-11-19)); the sound latency, by arithmetic and not measured, is outside the original's range; a session is safe only when `doom-play` ends it; WSLg is keyboard-only. **Carried to the next milestone as its first item** ([ROADMAP](ROADMAP.md#later)): the stuck-fire fix, measuring the real sound latency on Windows and the cheapest way to shorten it, a session demo that Ctrl+C or a closed console cannot lose, moving the mouse forwarding out of `input_log.rs`, and the review's three open minor points (M1 to M3). The last head that changed code is `b141767`. Scratch under `~/scratch/m5c` is deleted after the owner has the final report.

## Summary

- **Owner decisions.** T1–T16 are written in [§5](#5-owner-decisions-t1t16-with-recommendations), each with options, the four test questions and one recommendation. **All were confirmed by the owner on 2026-11-18**, with notes on T2, T3, T4, T5 and T6, T7, T12 and T13: see [Owner rulings](#owner-rulings-2026-11-18). Before the rulings only genuine-engine tools and evidence were built (Phase 2 steps 1 and 2). The bullets below are the Phase 1 record.
- **The build the source ships with does not run the mixer this milestone ports.** `doomdef.h:84` defines `SNDSERV`, which sends sound to the external `sndserver` program; `i_sound.c`'s in-process mixer exists only with that one define removed. The two builds make identical `M_Random` draws and identical volume, separation and pitch for every start, so the face's glance does not depend on the choice; they differ in which starts find a free channel (T2).
- **What "playing" means in this source.** A channel of the sound logic is busy while `gametic < handle` (`i_sound.c:517-521`), which has nothing to do with the sound's length. `I_StopSound` and `I_UpdateSoundParams` are empty, so a stop never silences the mixer and volume and stereo position never follow a moving origin. The sound logic tracks 3 channels by default; the mixer has 8.
- **`M_Random` in a level run has exactly three sources**: the status bar once per tic, each sound start that passes its audibility test (one draw, except two sounds that draw none), and the level-start wipe (exactly 320 draws). A sound that finds no channel still draws; an inaudible one does not.
- **The simulation's sound events are not enough as they are.** Every reachable start has an event in the right place, but none carries a position, and audibility depends on where the origin and the listener are at the moment of the call, in the middle of the tic. One event is wrong: the hard-death scream is `sfx_pdiehi` in every game mode, where the original uses it only in commercial mode. No sound event of wires 22–24 has ever been compared with the genuine engine. The recommendation is a wire 25 that adds only what sound needs (T3).
- **Timing outside the tic has to be declared.** The original frees channels and mixes one 512-frame buffer per pass of its main loop, paced by the sound card; a tic is worth 315 frames. The proposed schedule ties all of it to tics (T5).
- **Two prototypes ran.** The genuine `s_sound.c`, `i_sound.c` and `f_wipe.c`, compiled whole into a fork of the M5b runner, leave every game dump unchanged (59 of 59 synthetic items, 61 of 61 real runs) and change only the face: 68,995 of 158,472 real-map frames. The genuine mixer, compiled whole with no audio device, gave byte-identical sample streams in four builds on 18 of 21 scenarios and matched an independent model; the other 3 are lumps too short for the original to read safely (T11).
- **Layout-dependent behaviour found.** The switch release sound reads its position out of the neighbouring button slot, and 32-bit and 64-bit builds read different fields (T12). Lumps of 8 bytes or fewer are read one byte past their block.
- **Nothing on this machine can make sound today**, and nobody has heard anything from any candidate library: every trial wrote digital silence. The recommended library is maudio (the miniaudio C library behind safe Rust bindings), with a pure-Rust per-system route as the fallback (T7).
- **Size.** About 19,000–28,000 hand-written lines are projected against a proposed cap of 30,000 ([§6.2](#62-size)).

## 1. C facts: the sound logic, its call sites, the mixer, `M_Random` and the wipe

Every fact in this section cites the file and line of linuxdoom-1.10 it was read from. Each subsection separates what was read in the source, what was observed by running something, and inference, and ends with the undefined or layout-dependent behaviour it found, the questions it raises for the owner (taken up in [§5](#5-owner-decisions-t1t16-with-recommendations)) and what was not done.

### 1.1 The sound logic: s_sound.c in full

This section states, with file:line citations into `linuxdoom-1.10/`, everything the
machine-independent sound layer (`s_sound.c`) does, plus the parts of `sounds.c`, `m_misc.c`,
`d_main.c` and `i_sound.c` it depends on. Terms used throughout:

- **sound start**: one call of `S_StartSound` or `S_StartSoundAtVolume`.
- **channel**: one slot of `s_sound.c`'s own table `channels[]` (s_sound.c:108). This is *not*
  the same thing as the eight mixing channels inside `i_sound.c` (i_sound.c:94, 122); the two
  tables are independent and this section calls the latter "mixer channels".
- **origin**: the `void*` a caller passes as the sound's source: a `mobj_t*`, a sector's
  `degenmobj_t soundorg` (r_defs.h:82-95, 121), or `NULL` (no position).
- **listener**: the `mobj_t*` positional sounds are measured against; always
  `players[consoleplayer].mo` (s_sound.c:304-306; d_main.c:392).
- **draw**: one call of `M_Random` (m_random.c:63-67), which advances `rndindex`.

Facts read in the source are numbered. Observations from running the genuine code are in
1.1.9 and are marked as such. Inference is labelled "Inference".

#### 1.1.0 Which branches are compiled: the shipped build uses the external sound server

1. The Makefile's flags are `CFLAGS=-g -Wall -DNORMALUNIX -DLINUX # -DUSEASM` (Makefile:9). It
   defines neither `SNDSERV` nor `SNDINTR` nor `SNDSRV`.
2. **But `doomdef.h` defines `SNDSERV` unconditionally**: `#define SNDSERV  1` (doomdef.h:84);
   `SNDINTR` is commented out (doomdef.h:85). Every file that tests these macros includes
   `doomdef.h` first: i_sound.c:59 (directly) and i_sound.c:54 via i_sound.h:26; d_main.c:44;
   m_misc.c:39; s_sound.c via i_sound.h (s_sound.c:33, i_sound.h:26).
3. Therefore, in linuxdoom-1.10 as shipped:

| `#if` | Where | Branch compiled |
|---|---|---|
| `#ifndef SNDSRV` (note the spelling) | s_sound.c:367-381 | **compiled**: `SNDSRV` is defined nowhere (grep of the whole tree finds only this line), so the "not pre-cached" `fprintf` block is in the build |
| `#ifdef SAWDEBUG` | s_sound.c:402-405, 411-464, 719-722 | **not compiled**: `SAWDEBUG` is defined nowhere |
| `#ifdef SNDSERV` / `#elif SNDINTR` / `#else` | i_sound.c:62-78 | the `SNDSERV` branch: `FILE* sndserver=0; char* sndserver_filename = "./sndserver ";` (64-65) |
| `#ifdef SNDSERV ... #else` in `I_StartSound` | i_sound.c:482-500 | the `SNDSERV` branch (483-489): write a `p` command to the server pipe if it is open, **return `id`** |
| `#ifdef SNDINTR` | i_sound.c:541-544, 637-653, 720-722, 762-765 | not compiled |
| `#ifdef SNDSERV ... #else` in `I_ShutdownSound` | i_sound.c:695-726 | `SNDSERV` (696-701) |
| `#ifdef SNDSERV ... #else` in `I_InitSound` | i_sound.c:740-824 | `SNDSERV` (741-757): `popen` of `./sndserver -quiet` (or `$DOOMWADDIR/./sndserver`) if executable. **No sound data is loaded**, no `/dev/dsp` is opened, `S_sfx[i].data` stays 0 |
| `#ifndef SNDSERV` around `I_UpdateSound()` | d_main.c:397-400 | **not compiled**: the in-process mixer is never run |
| `#ifndef SNDINTR` around `I_SubmitSound()` | d_main.c:402-405 | compiled: `I_SubmitSound` (i_sound.c:665-670) writes the never-mixed, zero `mixbuffer` to `audio_fd`, a zero-initialised global (i_sound.c:106) that `I_InitSound` never opens in this branch, i.e. file descriptor 0 |
| `#ifdef SNDSERV` | m_misc.c:211-214, 256-259 | compiled: `sndserver` and `mb_used` config defaults |

4. Consequence (inference, from items 2-3): **the shipped linuxdoom-1.10 does not mix sound in
   process at all**; it sends one text line per sound start to an external `sndserver`
   (`fprintf(sndserver, "p%2.2x%2.2x%2.2x%2.2x\n", id, pitch, vol, sep)`, i_sound.c:485).
   The in-process mixer (`addsfx`, `I_UpdateSound`, i_sound.c:264-381, 539-654) is compiled
   but unreachable. A build with the in-process mixer requires removing doomdef.h:84, which is a
   configuration choice, not the shipped default. Where the two builds differ for `s_sound.c`
   this section gives both and names them **build S** (shipped, `SNDSERV`) and **build M**
   (doomdef.h:84 removed, `SNDINTR` still undefined: in-process synchronous mixer).
5. What differs between the builds for the logic of this section: only the **handle** that
   `I_StartSound` returns (build S: the sfx id, i_sound.c:489; build M: `addsfx`'s counter,
   i_sound.c:495, 334-340), and whether `S_sfx[].data` is loaded (build S: never; build M:
   i_sound.c:799-813). Everything before `S_getChannel` (all early returns, both `M_Random`
   draws, all volume, separation and pitch arithmetic) is identical in both builds.

#### 1.1.1 Constants and macros (s_sound.c:47-80)

| Name | Definition | Value | Unit / meaning | Used at |
|---|---|---|---|---|
| `snd_prefixen[]` | `{ 'P','P','A','S','S','S','M','M','M','S','S','S' }` (48-49) | 12 chars | "Purpose?" (47); unused | nowhere (grep) |
| `S_MAX_VOLUME` | `127` (51) | 127 | volume | nowhere in s_sound.c (the 127 limits at 618, 634 are literals) |
| `S_CLIPPING_DIST` | `(1200*0x10000)` (55) | 78,643,200 | fixed-point map units (1200.0) | 774, 802-803, 806, 813 |
| `S_CLOSE_DIST` | `(160*0x10000)` (61) | 10,485,760 | fixed (160.0); comment says originally 200 (60) | 796 |
| `S_ATTENUATOR` | `((S_CLIPPING_DIST-S_CLOSE_DIST)>>FRACBITS)` (64) | 1040 | integer map units | 807, 814 |
| `NORM_VOLUME` | `snd_MaxVolume` (67) | - | never expanded; `snd_MaxVolume` is not declared anywhere (grep), so any use would not compile | nowhere |
| `NORM_PITCH` | `128` (69) | 128 | pitch, 0-255 scale | 297, 564 |
| `NORM_PRIORITY` | `64` (70) | 64 | priority | 298 |
| `NORM_SEP` | `128` (71) | 128 | separation, centre | 315, 323, 565 |
| `S_PITCH_PERTURB` | `1` (73) | 1 | unused (the perturbation at 327-346 is unconditional) | nowhere |
| `S_STEREO_SWING` | `(96*0x10000)` (74) | 6,291,456 | fixed (96.0) | 793 |
| `S_IFRACVOL` | `30` (77) | 30 | "percent attenuation from front to back"; unused | nowhere |
| `NA` | `0` (79) | 0 | unused | nowhere |
| `S_NUMCHANNELS` | `2` (80) | 2 | unused; the channel count is `numChannels` | nowhere |

`FRACBITS` is 16 and `fixed_t` is `int` (m_fixed.h:35, 38). `FixedMul(a,b)` is
`((long long) a * (long long) b) >> FRACBITS` (m_fixed.c:44-49). `ANGLETOFINESHIFT` is
19 (tables.h:55), `FINEANGLES` 8192 (tables.h:50), `finesine` has `5*FINEANGLES/4` = 10240
entries (tables.h:58; tables.c:584), `angle_t` is `unsigned` (tables.h:78).

`s_sound.c` also declares, unused by it, `extern int snd_MusicDevice, snd_SfxDevice,
snd_DesiredMusicDevice, snd_DesiredSfxDevice;` (85-89).

#### 1.1.2 `channel_t`, the globals, and the values in a fresh process

`channel_t` (s_sound.c:93-104), three fields:

| Field | Type | Meaning |
|---|---|---|
| `sfxinfo` | `sfxinfo_t*` | the `S_sfx` entry being played; **`NULL` means the channel is free** (95-96) |
| `origin` | `void*` | the origin pointer as passed (98-99); compared only by pointer equality, and dereferenced (as `mobj_t*`, reading `x`, `y`) in `S_AdjustSoundParams` |
| `handle` | `int` | the value `I_StartSound` returned (101-102); read only by `I_SoundIsPlaying`, `I_StopSound`, `I_UpdateSoundParams` |

Statics and globals of s_sound.c:

| Name | Declaration | Initial value | Who sets it |
|---|---|---|---|
| `channels` | `static channel_t* channels;` (108) | NULL until `S_Init` | `S_Init`: `Z_Malloc(numChannels*sizeof(channel_t), PU_STATIC, 0)` (179-180) |
| `snd_SfxVolume` | `int snd_SfxVolume = 15;` (113) | 15 at load, **8 after `M_LoadDefaults`** | m_misc.c:237 default 8, written by `M_LoadDefaults` (m_misc.c:352-354) at d_main.c:1015, before `S_Init` (d_main.c:1107); then `S_SetSfxVolume` (637); menu changes it 0..15 (m_menu.c:821-830); `I_SetSfxVolume` also writes it (i_sound.c:434) but nothing calls `I_SetSfxVolume` (grep) |
| `snd_MusicVolume` | `int snd_MusicVolume = 15;` (116) | 15, then 8 (m_misc.c:238) | `S_SetMusicVolume` (626), `I_SetMusicVolume` (i_sound.c:441) |
| `mus_paused` | `static boolean mus_paused;` (121) | 0 | `S_Init` (187), `S_Start` (214), pause/resume (502, 511) |
| `mus_playing` | `static musicinfo_t* mus_playing=0;` (124) | 0 | `S_ChangeMusic` (685), `S_StopMusic` (701) |
| `numChannels` | `int numChannels;` (129), comment "set by the defaults code in M_misc" (126-128) | **3** | m_misc.c:282 `{"snd_channels",&numChannels, 3}`; a config file can set any int (m_misc.c:367-402, no range check) |
| `nextcleanup` | `static int nextcleanup;` (131) | 0 | `S_Start` sets 15 (247); read only inside the commented-out block (536-551): **dead** |

Fresh process (no `~/.doomrc`, no `-config`): `M_LoadDefaults` sets every default first
(m_misc.c:352-354), then reads `defaultfile` if it exists (m_misc.c:357-402; `basedefault` is
`$HOME/.doomrc`, d_main.c:614). So a fresh process has **`numChannels = 3`,
`snd_SfxVolume = 8`, `snd_MusicVolume = 8`** when `S_Init(snd_SfxVolume, snd_MusicVolume)` runs
(d_main.c:1107). The initialisers 15 at s_sound.c:113 and 116 are never observed by the game.

Startup order: `M_LoadDefaults` (d_main.c:1015), `I_Init` → `I_InitSound` (d_main.c:1101,
i_system.c:107-110), `S_Init` (d_main.c:1107). In build M this means `S_sfx[].data` is loaded
before `S_Init`; in build S it is never loaded.

#### 1.1.3 `sounds.h` / `sounds.c`: the `S_sfx` table

`sfxinfo_t` (sounds.h:30-62), in declaration order:

| Field | Type | Comment in source | Who reads it (whole tree, by grep) |
|---|---|---|---|
| `name` | `char*` | "up to 6-character name" (34-35) | `I_GetSfxLumpNum`: `sprintf(namebuf, "ds%s", sfx->name)` into `char namebuf[9]` (i_sound.c:453-455); build M only: `getsfx` via `I_InitSound` (i_sound.c:805, 201). In s_sound.c only in commented-out or `SAWDEBUG` code (272-274). All 108 names are at most 6 characters (checked over the table below), so `namebuf` does not overflow |
| `singularity` | `int` | "Sfx singularity (only one at a time)" (37-38) | **nobody**. The only occurrences are the declaration (sounds.h:38), the table initialisers (sounds.c:117-226) and the sound server's own copy of the struct (sndserv/soundst.h:100). No code in linuxdoom-1.10 reads it |
| `priority` | `int` | "Sfx priority" (40-41) | s_sound.c:854 (`S_getChannel`'s kick-out test); s_sound.c:286 (linked sounds only, passed to `I_StartSound`, which overwrites its copy with `priority = 0;` at i_sound.c:480 and never uses it) |
| `link` | `sfxinfo_t*` | "referenced sound if a link" (43-44) | s_sound.c:283, 567 (tested for non-NULL only; the target is never dereferenced by s_sound.c); build M: i_sound.c:802, 810-811 |
| `pitch` | `int` | "pitch if a link" (46-47) | s_sound.c:285, 569, only when `link` is non-NULL |
| `volume` | `int` | "volume if a link" (49-50) | s_sound.c:287, 570, only when `link` is non-NULL |
| `data` | `void*` | "sound data" (52-53) | s_sound.c:369 (tested; prints a warning when 0); build M: i_sound.c:330, 805, 810 |
| `usefulness` | `int` | "checked every second ... if -1, then throw out" (55-58) | written at s_sound.c:191, 384-385, 738; read only at 384 (its own increment) and in the commented-out cleanup (540-546). **No behaviour depends on it** |
| `lumpnum` | `int` | "lump number of sfx" (60-61) | s_sound.c:191 (set -1), 364-365 (if < 0, look up and store). **The stored value is never used for anything**; the lookup's only observable effect is that `W_GetNumForName` calls `I_Error` when the lump is missing (w_wad.c:399-409) |

The table `sfxinfo_t S_sfx[]` (sounds.c:114-227) has 109 entries, ids 0 to 108, in the same
order as `sfxenum_t` (sounds.h:177-289, `sfx_None` = 0 at line 179, `NUMSFX` = 109 at line 288;
the id of the enumerator on sounds.h line L is L - 179; the table row of id N ≥ 1 is
sounds.c line 118 + N, id 0 is line 117). The enumerator names and table names were checked
to match pairwise for all 109 (script, 1.1.9). Each initialiser has seven values: `name,
singularity, priority, link, pitch, volume, data`; `usefulness` and `lumpnum` are not given and
so start at 0 (static storage), and `S_Init` sets both to -1 for ids 1..108 (s_sound.c:190-191;
id 0 keeps 0, 0). `S_sfx[0]` "needs to be a dummy for odd reasons" (sounds.c:116).

Summary: every non-linked entry has `link 0, pitch -1, volume -1, data 0`; exactly one entry
is linked: `sfx_chgun` (86) → `&S_sfx[sfx_pistol]`, pitch 150, volume 0 (sounds.c:204).
**`sfx_chgun` is never started anywhere in the source** (grep for `sfx_chgun` finds only
sounds.h:265 and sounds.c:204), so the link branches of `S_StartSoundAtVolume` (283-294) and
`S_UpdateSounds` (567-580) are unreachable from game code. 23 entries have `singularity` true
(ids 32, 33, 36-50, 75-80); it has no effect. Priority values in use: 32 (13 entries), 60 (4),
64 (15), 70 (33), 78 (6), 90 (6), 92 (1), 94 (1), 96 (7), 98 (7), 100 (10), 118 (1),
119 (1), 120 (3), and 0 for id 0.

Full table (id, enumerator, name, singularity, priority, link, link pitch, link volume,
sounds.c line):

| id | enum | name | sing. | prio | link | pitch | volume | line |
|---|---|---|---|---|---|---|---|---|
| 0 | sfx_None | none | false | 0 | 0 (none) | -1 | -1 | 117 |
| 1 | sfx_pistol | pistol | false | 64 | 0 (none) | -1 | -1 | 119 |
| 2 | sfx_shotgn | shotgn | false | 64 | 0 (none) | -1 | -1 | 120 |
| 3 | sfx_sgcock | sgcock | false | 64 | 0 (none) | -1 | -1 | 121 |
| 4 | sfx_dshtgn | dshtgn | false | 64 | 0 (none) | -1 | -1 | 122 |
| 5 | sfx_dbopn | dbopn | false | 64 | 0 (none) | -1 | -1 | 123 |
| 6 | sfx_dbcls | dbcls | false | 64 | 0 (none) | -1 | -1 | 124 |
| 7 | sfx_dbload | dbload | false | 64 | 0 (none) | -1 | -1 | 125 |
| 8 | sfx_plasma | plasma | false | 64 | 0 (none) | -1 | -1 | 126 |
| 9 | sfx_bfg | bfg | false | 64 | 0 (none) | -1 | -1 | 127 |
| 10 | sfx_sawup | sawup | false | 64 | 0 (none) | -1 | -1 | 128 |
| 11 | sfx_sawidl | sawidl | false | 118 | 0 (none) | -1 | -1 | 129 |
| 12 | sfx_sawful | sawful | false | 64 | 0 (none) | -1 | -1 | 130 |
| 13 | sfx_sawhit | sawhit | false | 64 | 0 (none) | -1 | -1 | 131 |
| 14 | sfx_rlaunc | rlaunc | false | 64 | 0 (none) | -1 | -1 | 132 |
| 15 | sfx_rxplod | rxplod | false | 70 | 0 (none) | -1 | -1 | 133 |
| 16 | sfx_firsht | firsht | false | 70 | 0 (none) | -1 | -1 | 134 |
| 17 | sfx_firxpl | firxpl | false | 70 | 0 (none) | -1 | -1 | 135 |
| 18 | sfx_pstart | pstart | false | 100 | 0 (none) | -1 | -1 | 136 |
| 19 | sfx_pstop | pstop | false | 100 | 0 (none) | -1 | -1 | 137 |
| 20 | sfx_doropn | doropn | false | 100 | 0 (none) | -1 | -1 | 138 |
| 21 | sfx_dorcls | dorcls | false | 100 | 0 (none) | -1 | -1 | 139 |
| 22 | sfx_stnmov | stnmov | false | 119 | 0 (none) | -1 | -1 | 140 |
| 23 | sfx_swtchn | swtchn | false | 78 | 0 (none) | -1 | -1 | 141 |
| 24 | sfx_swtchx | swtchx | false | 78 | 0 (none) | -1 | -1 | 142 |
| 25 | sfx_plpain | plpain | false | 96 | 0 (none) | -1 | -1 | 143 |
| 26 | sfx_dmpain | dmpain | false | 96 | 0 (none) | -1 | -1 | 144 |
| 27 | sfx_popain | popain | false | 96 | 0 (none) | -1 | -1 | 145 |
| 28 | sfx_vipain | vipain | false | 96 | 0 (none) | -1 | -1 | 146 |
| 29 | sfx_mnpain | mnpain | false | 96 | 0 (none) | -1 | -1 | 147 |
| 30 | sfx_pepain | pepain | false | 96 | 0 (none) | -1 | -1 | 148 |
| 31 | sfx_slop | slop | false | 78 | 0 (none) | -1 | -1 | 149 |
| 32 | sfx_itemup | itemup | true | 78 | 0 (none) | -1 | -1 | 150 |
| 33 | sfx_wpnup | wpnup | true | 78 | 0 (none) | -1 | -1 | 151 |
| 34 | sfx_oof | oof | false | 96 | 0 (none) | -1 | -1 | 152 |
| 35 | sfx_telept | telept | false | 32 | 0 (none) | -1 | -1 | 153 |
| 36 | sfx_posit1 | posit1 | true | 98 | 0 (none) | -1 | -1 | 154 |
| 37 | sfx_posit2 | posit2 | true | 98 | 0 (none) | -1 | -1 | 155 |
| 38 | sfx_posit3 | posit3 | true | 98 | 0 (none) | -1 | -1 | 156 |
| 39 | sfx_bgsit1 | bgsit1 | true | 98 | 0 (none) | -1 | -1 | 157 |
| 40 | sfx_bgsit2 | bgsit2 | true | 98 | 0 (none) | -1 | -1 | 158 |
| 41 | sfx_sgtsit | sgtsit | true | 98 | 0 (none) | -1 | -1 | 159 |
| 42 | sfx_cacsit | cacsit | true | 98 | 0 (none) | -1 | -1 | 160 |
| 43 | sfx_brssit | brssit | true | 94 | 0 (none) | -1 | -1 | 161 |
| 44 | sfx_cybsit | cybsit | true | 92 | 0 (none) | -1 | -1 | 162 |
| 45 | sfx_spisit | spisit | true | 90 | 0 (none) | -1 | -1 | 163 |
| 46 | sfx_bspsit | bspsit | true | 90 | 0 (none) | -1 | -1 | 164 |
| 47 | sfx_kntsit | kntsit | true | 90 | 0 (none) | -1 | -1 | 165 |
| 48 | sfx_vilsit | vilsit | true | 90 | 0 (none) | -1 | -1 | 166 |
| 49 | sfx_mansit | mansit | true | 90 | 0 (none) | -1 | -1 | 167 |
| 50 | sfx_pesit | pesit | true | 90 | 0 (none) | -1 | -1 | 168 |
| 51 | sfx_sklatk | sklatk | false | 70 | 0 (none) | -1 | -1 | 169 |
| 52 | sfx_sgtatk | sgtatk | false | 70 | 0 (none) | -1 | -1 | 170 |
| 53 | sfx_skepch | skepch | false | 70 | 0 (none) | -1 | -1 | 171 |
| 54 | sfx_vilatk | vilatk | false | 70 | 0 (none) | -1 | -1 | 172 |
| 55 | sfx_claw | claw | false | 70 | 0 (none) | -1 | -1 | 173 |
| 56 | sfx_skeswg | skeswg | false | 70 | 0 (none) | -1 | -1 | 174 |
| 57 | sfx_pldeth | pldeth | false | 32 | 0 (none) | -1 | -1 | 175 |
| 58 | sfx_pdiehi | pdiehi | false | 32 | 0 (none) | -1 | -1 | 176 |
| 59 | sfx_podth1 | podth1 | false | 70 | 0 (none) | -1 | -1 | 177 |
| 60 | sfx_podth2 | podth2 | false | 70 | 0 (none) | -1 | -1 | 178 |
| 61 | sfx_podth3 | podth3 | false | 70 | 0 (none) | -1 | -1 | 179 |
| 62 | sfx_bgdth1 | bgdth1 | false | 70 | 0 (none) | -1 | -1 | 180 |
| 63 | sfx_bgdth2 | bgdth2 | false | 70 | 0 (none) | -1 | -1 | 181 |
| 64 | sfx_sgtdth | sgtdth | false | 70 | 0 (none) | -1 | -1 | 182 |
| 65 | sfx_cacdth | cacdth | false | 70 | 0 (none) | -1 | -1 | 183 |
| 66 | sfx_skldth | skldth | false | 70 | 0 (none) | -1 | -1 | 184 |
| 67 | sfx_brsdth | brsdth | false | 32 | 0 (none) | -1 | -1 | 185 |
| 68 | sfx_cybdth | cybdth | false | 32 | 0 (none) | -1 | -1 | 186 |
| 69 | sfx_spidth | spidth | false | 32 | 0 (none) | -1 | -1 | 187 |
| 70 | sfx_bspdth | bspdth | false | 32 | 0 (none) | -1 | -1 | 188 |
| 71 | sfx_vildth | vildth | false | 32 | 0 (none) | -1 | -1 | 189 |
| 72 | sfx_kntdth | kntdth | false | 32 | 0 (none) | -1 | -1 | 190 |
| 73 | sfx_pedth | pedth | false | 32 | 0 (none) | -1 | -1 | 191 |
| 74 | sfx_skedth | skedth | false | 32 | 0 (none) | -1 | -1 | 192 |
| 75 | sfx_posact | posact | true | 120 | 0 (none) | -1 | -1 | 193 |
| 76 | sfx_bgact | bgact | true | 120 | 0 (none) | -1 | -1 | 194 |
| 77 | sfx_dmact | dmact | true | 120 | 0 (none) | -1 | -1 | 195 |
| 78 | sfx_bspact | bspact | true | 100 | 0 (none) | -1 | -1 | 196 |
| 79 | sfx_bspwlk | bspwlk | true | 100 | 0 (none) | -1 | -1 | 197 |
| 80 | sfx_vilact | vilact | true | 100 | 0 (none) | -1 | -1 | 198 |
| 81 | sfx_noway | noway | false | 78 | 0 (none) | -1 | -1 | 199 |
| 82 | sfx_barexp | barexp | false | 60 | 0 (none) | -1 | -1 | 200 |
| 83 | sfx_punch | punch | false | 64 | 0 (none) | -1 | -1 | 201 |
| 84 | sfx_hoof | hoof | false | 70 | 0 (none) | -1 | -1 | 202 |
| 85 | sfx_metal | metal | false | 70 | 0 (none) | -1 | -1 | 203 |
| 86 | sfx_chgun | chgun | false | 64 | &S_sfx[sfx_pistol] | 150 | 0 | 204 |
| 87 | sfx_tink | tink | false | 60 | 0 (none) | -1 | -1 | 205 |
| 88 | sfx_bdopn | bdopn | false | 100 | 0 (none) | -1 | -1 | 206 |
| 89 | sfx_bdcls | bdcls | false | 100 | 0 (none) | -1 | -1 | 207 |
| 90 | sfx_itmbk | itmbk | false | 100 | 0 (none) | -1 | -1 | 208 |
| 91 | sfx_flame | flame | false | 32 | 0 (none) | -1 | -1 | 209 |
| 92 | sfx_flamst | flamst | false | 32 | 0 (none) | -1 | -1 | 210 |
| 93 | sfx_getpow | getpow | false | 60 | 0 (none) | -1 | -1 | 211 |
| 94 | sfx_bospit | bospit | false | 70 | 0 (none) | -1 | -1 | 212 |
| 95 | sfx_boscub | boscub | false | 70 | 0 (none) | -1 | -1 | 213 |
| 96 | sfx_bossit | bossit | false | 70 | 0 (none) | -1 | -1 | 214 |
| 97 | sfx_bospn | bospn | false | 70 | 0 (none) | -1 | -1 | 215 |
| 98 | sfx_bosdth | bosdth | false | 70 | 0 (none) | -1 | -1 | 216 |
| 99 | sfx_manatk | manatk | false | 70 | 0 (none) | -1 | -1 | 217 |
| 100 | sfx_mandth | mandth | false | 70 | 0 (none) | -1 | -1 | 218 |
| 101 | sfx_sssit | sssit | false | 70 | 0 (none) | -1 | -1 | 219 |
| 102 | sfx_ssdth | ssdth | false | 70 | 0 (none) | -1 | -1 | 220 |
| 103 | sfx_keenpn | keenpn | false | 70 | 0 (none) | -1 | -1 | 221 |
| 104 | sfx_keendt | keendt | false | 70 | 0 (none) | -1 | -1 | 222 |
| 105 | sfx_skeact | skeact | false | 70 | 0 (none) | -1 | -1 | 223 |
| 106 | sfx_skesit | skesit | false | 70 | 0 (none) | -1 | -1 | 224 |
| 107 | sfx_skeatk | skeatk | false | 70 | 0 (none) | -1 | -1 | 225 |
| 108 | sfx_radio | radio | false | 60 | 0 (none) | -1 | -1 | 226 |

#### 1.1.4 The priority rule, exactly

`S_getChannel` evicts with (s_sound.c:853-854):

```c
for (cnum=0 ; cnum<numChannels ; cnum++)
    if (channels[cnum].sfxinfo->priority >= sfxinfo->priority) break;
```

1. It compares the **table** priority of the occupying sound with the **table** priority of the
   new sound (`sfxinfo` is `&S_sfx[sfx_id]`, s_sound.c:280, 352). The local `priority`
   variable of `S_StartSoundAtVolume` (64 for every non-linked sound, 286, 298) is not used for
   channel choice; it goes only to `I_StartSound`, which discards it (i_sound.c:480).
2. The new sound takes the **first channel in index order whose priority value is greater than
   or equal to its own**. So a **numerically lower** priority value is **more important**: a
   sound of priority 32 (`telept`, `pldeth`, ...) can evict anything; a sound of priority 120
   (`posact`, `bgact`, `dmact`) can evict only other 120s. **Equal values evict** (`>=`), so a
   new sound replaces an equally important old one rather than being refused. The comment
   "Look for lower priority" (852) means "lower importance", i.e. a higher or equal number.
3. If no channel qualifies, the sound is refused: `return -1` (856-860), and
   `S_StartSoundAtVolume` returns (354-355) without calling `I_StartSound`.
4. This loop runs only when every channel is occupied (850), so `channels[cnum].sfxinfo` is
   never NULL when dereferenced at 854.

#### 1.1.5 Every function, line by line

##### `S_Init(sfxVolume, musicVolume)` (s_sound.c:161-192)

Called once, from d_main.c:1107 with `snd_SfxVolume`, `snd_MusicVolume` (8, 8 by default).

1. `fprintf(stderr, "S_Init: default sfx volume %d\n", sfxVolume)` (167).
2. `I_SetChannels()` (170): fills `steptable` (via `steptablemid = steptable + 128`,
   `steptablemid[i] = (int)(pow(2.0, (i/64.0))*65536.0)` for i in -128..127) and
   `vol_lookup[i*256+j] = (i*(j-128)*256)/127` for i 0..127, j 0..255 (i_sound.c:396-424).
   Pure table initialisation, used only by build M's mixer.
3. `S_SetSfxVolume(sfxVolume)` (172): `I_Error` unless 0 ≤ volume ≤ 127, then
   `snd_SfxVolume = volume` (631-639).
4. `S_SetMusicVolume(musicVolume)` (174): same range check with `I_Error` (618-622), then
   `I_SetMusicVolume(127); I_SetMusicVolume(volume);` (624-625), which only store into
   `snd_MusicVolume` (i_sound.c:438-444), then `snd_MusicVolume = volume` (626).
5. `channels = Z_Malloc(numChannels*sizeof(channel_t), PU_STATIC, 0)` (179-180).
6. `channels[i].sfxinfo = 0` for i in 0..numChannels-1 (183-184). `origin` and `handle` are
   left uninitialised (Z_Malloc does not clear); see 1.1.13 for why that is never observed.
7. `mus_paused = 0` (187).
8. `S_sfx[i].lumpnum = S_sfx[i].usefulness = -1` for i in 1..NUMSFX-1 (190-191).

No `M_Random`, no channel use.

##### `S_Start()` (s_sound.c:202-248): level start

Called only from `P_SetupLevel` (p_setup.c:607), inside `G_DoLoadLevel` (g_game.c:484), which
runs from `G_InitNew` (g_game.c:1481), from a `ga_loadlevel` action in `G_Ticker`
(g_game.c:622), and from `G_DoWorldDone` (g_game.c:1172-1176). It runs **before** `Z_FreeTags(PU_LEVEL, ...)` frees the
previous level's mobjs (p_setup.c:606-618: "Make sure all sounds are stopped before
Z_FreeTags").

1. For every channel 0..numChannels-1 with non-NULL `sfxinfo`: `S_StopChannel(cnum)`
   (209-211). Afterwards **every channel is free**. This does not depend on whether the sound
   is "playing"; `S_StopChannel` frees unconditionally (1.1.5, `S_StopChannel`).
2. `mus_paused = 0` (214).
3. Music number: `commercial`: `mus_runnin + gamemap - 1` (216-217); otherwise episode < 4:
   `mus_e1m1 + (gameepisode-1)*9 + gamemap-1` (235-236); episode ≥ 4: `spmus[gamemap-1]` from a
   9-entry local array (220-233, 238).
4. `S_ChangeMusic(mnum, true)` (245), s_sound.c:649-686: `I_Error("Bad music number %d")` if
   `musicnum <= mus_None || musicnum >= NUMMUSIC` (657-661); return if it is already
   `mus_playing` (665-666); `S_StopMusic` (669; 689-703: resume if paused, `I_StopSong`,
   `I_UnRegisterSong`, `Z_ChangeTag(data, PU_CACHE)`); look up `d_<name>` with
   `W_GetNumForName` (I_Error if missing) the first time (672-676, `char namebuf[9]`);
   `W_CacheLumpNum(lumpnum, PU_MUSIC)` (679); `I_RegisterSong` returns 1 (i_sound.c:875-881);
   `I_PlaySong` only sets `musicdies = gametic + TICRATE*30` (i_sound.c:841-846), which only
   `I_QrySongPlaying` reads (i_sound.c:884-889), and nothing calls that (the only call is
   commented out, s_sound.c:609-612). `mus_playing = music` (685).
   **The music part touches no channel, draws no `M_Random`, and reads nothing a level run
   observes.** Its observable effects are: a possible `I_Error` (bad number or missing
   `d_` lump), and zone memory traffic (a `PU_MUSIC` allocation and a `PU_CACHE` retag).
5. `nextcleanup = 15` (247): dead (only the commented-out block reads it).

##### `S_StartSoundAtVolume(origin_p, sfx_id, volume)` (s_sound.c:254-395)

Locals: `int rc, sep, pitch, priority; sfxinfo_t* sfx; int cnum;` and
`mobj_t* origin = (mobj_t *) origin_p;` (261-268). In order:

1. Debug print, **commented out** (271-274).
2. **Range check** (277-278): `if (sfx_id < 1 || sfx_id > NUMSFX) I_Error("Bad sfx #: %d", sfx_id);`
   Note `> NUMSFX`, not `>= NUMSFX`: **`sfx_id == NUMSFX` (109) passes** and indexes
   `S_sfx[109]`, one past the 109-entry array (out of bounds; 1.1.13). `sfx_id == 0` is fatal.
3. `sfx = &S_sfx[sfx_id]` (280).
4. **Link handling** (283-299):
   - if `sfx->link`: `pitch = sfx->pitch; priority = sfx->priority; volume += sfx->volume;`
     (285-287); **early return 1**: `if (volume < 1) return;` (289-290); then
     `if (volume > snd_SfxVolume) volume = snd_SfxVolume;` (292-293).
   - else: `pitch = NORM_PITCH` (128); `priority = NORM_PRIORITY` (64) (297-298).
   - The incoming `volume` of a non-linked sound is not checked at all: a volume of 0 does
     not return here.
5. **Audibility** (304-324): if `origin && origin != players[consoleplayer].mo`:
   - `rc = S_AdjustSoundParams(players[consoleplayer].mo, origin, &volume, &sep, &pitch)`
     (306-310). This **overwrites `volume`** (and `sep`) when audible; the caller's volume and
     the link adjustment are discarded for positional sounds. `pitch` is passed but never
     written by `S_AdjustSoundParams`.
   - **Same-position rule** (312-316): `if (origin->x == players[consoleplayer].mo->x &&
     origin->y == players[consoleplayer].mo->y) sep = NORM_SEP;` (z is not compared). This
     runs before the `rc` test, so it also runs for inaudible sounds (harmless: they return
     next). When the positions are equal, `R_PointToAngle` returns 0 (r_main.c:299-300), and
     the separation computed from that angle would depend on the listener's facing; this rule
     replaces it with 128.
   - **early return 2**: `if (!rc) return;` (318-319).
   - else (origin NULL or the listener's own mobj): `sep = NORM_SEP` (323). No distance test;
     `volume` stays as passed (or link-adjusted).
6. **Pitch perturbation** (326-346), with `M_Random` returning `rndtable[rndindex]`, 0..255,
   after pre-incrementing `rndindex` mod 256 (m_random.c:63-67):
   - if `sfx_id >= sfx_sawup && sfx_id <= sfx_sawhit` (ids **10..13**: `sawup`, `sawidl`,
     `sawful`, `sawhit`): `pitch += 8 - (M_Random()&15);` (330): adds -7..+8; then clamp
     `if (pitch<0) pitch = 0; else if (pitch>255) pitch = 255;` (332-335).
   - else if `sfx_id != sfx_itemup && sfx_id != sfx_tink` (ids 32 and 87 skip): `pitch += 16 -
     (M_Random()&31);` (340): adds -15..+16; same clamp (342-345).
   - `sfx_itemup` (32) and `sfx_tink` (87): **no draw**, pitch stays 128.
   - So **every sound start that gets past early returns 1 and 2 draws exactly one `M_Random`,
     except ids 32 and 87, which draw none.** The draw happens **before** the channel search:
     it happens even when no channel is available (step 8) and even when `snd_SfxVolume` is 0
     for a NULL-origin or listener-origin sound (its volume is then 0 but it still draws).
   - Resulting pitches for non-linked sounds: saw ids 121..136, others 113..144, ids 32/87:
     128. The clamps can never fire for the shipped table (the only linked entry, chgun, would
     give 135..166).
7. **Kill old sound** (348-349): `S_StopSound(origin)`: frees the first occupied channel whose
   `origin` equals this origin, **including `origin == NULL`** (see `S_StopSound`).
8. **Channel** (351-355): `cnum = S_getChannel(origin, sfx); if (cnum<0) return;` (**early
   return 3**: no channel; the draw of step 6 has already happened and step 7 has already freed
   a channel if one matched).
9. **Lump** (363-365): `if (sfx->lumpnum < 0) sfx->lumpnum = I_GetSfxLumpNum(sfx);`:
   `W_GetNumForName("ds<name>")`, which `I_Error`s if absent (i_sound.c:451-456,
   w_wad.c:399-409). Only reached when a channel was obtained; done once per sfx per process.
   The channel's `sfxinfo` and `origin` are already set when this can `I_Error`.
10. **Compiled `#ifndef SNDSRV` block** (367-381): `if (!sfx->data) fprintf(stderr,
    "S_StartSoundAtVolume: 16bit and not pre-cached - wtf?\n");` (369-372). In build S `data`
    is always 0, so **this line is printed to stderr for every sound start that obtains a
    channel**; in build M `data` is set for ids 1..108 and nothing is printed. The DOS
    caching lines inside are commented out (374-378).
11. **Usefulness** (384-385): `if (sfx->usefulness++ < 0) sfx->usefulness = 1;` (no behavioural
    effect, 1.1.3).
12. **Start** (389-394): `channels[cnum].handle = I_StartSound(sfx_id, volume, sep, pitch,
    priority);` (the `sfx->data` argument is commented out, 390).
    - Build S (i_sound.c:480-489): `priority = 0;` then, only if the pipe `sndserver` is open,
      `fprintf(sndserver, "p%2.2x%2.2x%2.2x%2.2x\n", id, pitch, vol, sep); fflush(sndserver);`
      then **`return id;`**, the sfx id 1..108 (the comment "control reaches end of
      non-void function" at 488 is stale; the return is there).
    - Build M (i_sound.c:490-499): `id = addsfx(id, vol, steptable[pitch], sep); return id;`.
      `addsfx` returns `rc = handlenums++` where `static unsigned short handlenums = 0` is
      reset to 100 whenever it is 0 (i_sound.c:271, 334-340): the handles are 100, 101, ...,
      65535, 100, ... in order of all `I_StartSound` calls since the process started.

Inputs `I_StartSound` receives, summarised: `vol` = `snd_SfxVolume` (or the passed volume) for
NULL/listener origins; otherwise `S_AdjustSoundParams`'s volume (0 < vol ≤ snd_SfxVolume, or
up to 15 on map 8). `sep` = 128 for NULL/listener origins and same-x,y origins; otherwise
33..224 (1.1.5, `S_AdjustSoundParams`). `pitch` as in step 6.

##### `S_StartSound(origin, sfx_id)` (s_sound.c:397-466)

1. `#ifdef SAWDEBUG` (402-405): not compiled (and its body is commented out anyway).
2. `S_StartSoundAtVolume(origin, sfx_id, snd_SfxVolume)` (407). This is the whole compiled
   body.
3. `#ifdef SAWDEBUG` block (411-464, "UNUSED. We had problems, had we not?"): **not compiled**.
   It would only print diagnostics about saw sounds.

`S_StartSoundAtVolume` has no other caller in the tree (grep), so in practice the starting
volume is always `snd_SfxVolume`.

##### `S_StopSound(origin)` (s_sound.c:471-484)

For cnum 0..numChannels-1: if `channels[cnum].sfxinfo && channels[cnum].origin == origin`,
`S_StopChannel(cnum)` and **break** (476-483). Only the **first** matching channel is freed.
`origin` may be NULL, in which case the first occupied NULL-origin channel is freed. Callers:
s_sound.c:349 and `P_RemoveMobj` (p_mobj.c:565-566, "stop any playing sound").

##### `S_getChannel(origin, sfxinfo)` (s_sound.c:827-875)

1. First pass (838-847), cnum 0 upwards: stop at the first channel that is free
   (`!channels[cnum].sfxinfo`, 840-841) **or** that is occupied with the same non-NULL origin
   (`origin && channels[cnum].origin == origin`, 842), in which case `S_StopChannel(cnum)`
   first (844). Whichever comes first in index order wins.
2. If the loop ran off the end (`cnum == numChannels`, 850): the kick-out search of 1.1.4
   (853-865): evict the first channel with priority value ≥ the new one's via
   `S_StopChannel(cnum)` (864), or return -1 if none (856-860).
3. `c->sfxinfo = sfxinfo; c->origin = origin;` (868-872), return `cnum` (874). The handle is set
   by the caller.
4. Inference, proved from the code: **the same-origin branch (842-846) never fires when called
   from s_sound.c:352**, because `S_StopSound(origin)` at 349 has just freed the only channel
   that could match (see the invariant in 1.1.10). It is dead in practice, not in text.

##### `S_StopChannel(cnum)` (s_sound.c:708-742)

1. `c = &channels[cnum]` (712). Nothing happens if `c->sfxinfo` is NULL (714).
2. `if (I_SoundIsPlaying(c->handle)) I_StopSound(c->handle);` (717-724; the `SAWDEBUG` print at
   719-722 is not compiled). `I_StopSound` is `handle = 0;` and nothing else
   (i_sound.c:505-514): **it stops nothing**, in either build. In build S no stop command
   exists in the server protocol used here; in build M the mixer channel keeps playing to the
   end of the sample.
3. Loop over the other channels looking for one with the same `sfxinfo` (728-735); on a match
   it only `break`s. **The loop has no effect** (its result is not used).
4. `c->sfxinfo->usefulness--` (738): no behavioural effect.
5. `c->sfxinfo = 0` (740): the channel is free. `origin` and `handle` keep stale values, which
   are never read while `sfxinfo` is NULL.

##### `S_AdjustSoundParams(listener, source, vol, sep, pitch)` (s_sound.c:752-818)

Locals `fixed_t approx_dist, adx, ady; angle_t angle;` (760-763). All `int` arithmetic except
`angle` (unsigned 32-bit) and the `long long` inside `FixedMul`.

1. `adx = abs(listener->x - source->x); ady = abs(listener->y - source->y);` (767-768).
2. `approx_dist = adx + ady - ((adx < ady ? adx : ady)>>1);` (771): `max + min/2`,
   i.e. `adx + ady - floor(min/2)` (the shift is of a non-negative value).
3. **Clip** (773-777): `if (gamemap != 8 && approx_dist > S_CLIPPING_DIST) return 0;`
   `*vol` and `*sep` are not written on this path. `gamemap` is the map number only:
   **E1M8, E2M8, E3M8, E4M8 and MAP08 all have no clipping**. Exactly 1200.0 is not clipped
   here (strict `>`), but gets volume 0 below.
4. **Angle** (780-783): `angle = R_PointToAngle2(listener->x, listener->y, source->x,
   source->y)`, the angle from the listener to the source. `R_PointToAngle2` **writes the
   renderer globals `viewx = x1; viewy = y1;`** and calls `R_PointToAngle(x2, y2)`
   (r_main.c:378-388), which returns 0 when the two points coincide (r_main.c:299-300) and
   otherwise uses `SlopeDiv` and `tantoangle` per octant (r_main.c:292-375, tables.c:50-62).
   No playsim code reads `viewx`/`viewy` (grep of p_*.c, g_game.c, st_stuff.c, hu_stuff.c,
   wi_stuff.c, am_map.c), and `R_SetupFrame` sets them before rendering reads them, so this
   write is invisible to the game (inference, from the grep).
5. **Relative angle** (785-788):
   `if (angle > listener->angle) angle = angle - listener->angle; else angle = angle +
   (0xffffffff - listener->angle);` All unsigned, mod 2^32. The else branch equals
   `angle - listener->angle - 1` mod 2^32: **an off-by-one when the source's bearing is less
   than or equal to the listener's facing**. Example: bearing equal to facing gives
   0xffffffff, not 0.
6. `angle >>= ANGLETOFINESHIFT;` (790): 0..8191, always a valid index into `finesine[10240]`.
7. **Separation** (793): `*sep = 128 - (FixedMul(S_STEREO_SWING, finesine[angle]) >> FRACBITS);`
   - `FixedMul(96<<16, s)` = `(6291456 * s) >> 16` = `96*s` exactly (the product is a
     multiple of 65536).
   - `(96*s) >> 16` is an arithmetic right shift of a possibly negative `int` in gcc
     (implementation-defined in C): it rounds toward minus infinity.
   - `finesine[0..8191]` ranges over -65535..65535 (counted from tables.c, 1.1.9), so the
     shifted term is -96..95 and **`*sep` is 33..224**. A source to the listener's left
     (relative angle in (0, 180°), sine positive) gives sep < 128.
8. **Volume** (796-815):
   - `approx_dist < S_CLOSE_DIST` (under 160.0): `*vol = snd_SfxVolume;` (796-799).
   - else if `gamemap == 8` (800-808): clamp `approx_dist` to `S_CLIPPING_DIST` (802-803), then
     `*vol = 15 + ((snd_SfxVolume-15) * ((S_CLIPPING_DIST - approx_dist)>>FRACBITS)) / S_ATTENUATOR;`
     With k = (1200.0 - d) >> 16 in 0..1040: `vol = 15 + ((snd-15)*k)/1040`, the division
     truncating toward zero (gcc; implementation-defined in C89 for a negative operand).
     **At the default `snd_SfxVolume` 8 this gives 8 at 160 units, rising to 15 at 1200 units
     and beyond: on map 8, distant sounds are louder than near ones** and never inaudible
     (vol ≥ min(snd, 15) > 0 whenever snd > 0).
   - else (805-815): `*vol = (snd_SfxVolume * ((S_CLIPPING_DIST - approx_dist)>>FRACBITS)) / S_ATTENUATOR;`
     k in 0..1040, non-negative, so `vol = floor(snd*k/1040)`, 0..snd. With snd 8 the sound
     is audible (vol ≥ 1) only when k ≥ 130, i.e. approx_dist ≤ 1070.0 (70,123,520)
     (exactly: `(78643200 - d) >> 16 >= 130`).
9. `return (*vol > 0);` (817). `*pitch` is never touched.

##### `S_UpdateSounds(listener_p)` (s_sound.c:519-613)

1. `listener = (mobj_t*)listener_p` (529).
2. The **cleanup block is commented out** (533-551: `/*if (gametic > nextcleanup) ... */`).
   Not compiled.
3. For cnum 0..numChannels-1 (553-607), `c = &channels[cnum]`, `sfx = c->sfxinfo` (555-556);
   only occupied channels are examined (558):
   - **if `I_SoundIsPlaying(c->handle)`** (560), i.e. `gametic < c->handle` (1.1.6):
     - `volume = snd_SfxVolume; pitch = NORM_PITCH; sep = NORM_SEP;` (563-565);
     - link branch (567-580): `pitch = sfx->pitch; volume += sfx->volume;` and if `volume < 1`
       **stop the channel** and `continue` (571-575), else clamp to `snd_SfxVolume`
       (576-579). Unreachable from game code (chgun is never started).
     - if `c->origin && listener_p != c->origin` (584): `audible = S_AdjustSoundParams(listener,
       c->origin, &volume, &sep, &pitch)` (586-590), from the **current** positions of the
       listener and the origin and the listener's current facing; if `!audible`
       **`S_StopChannel(cnum)`** (592-595), else `I_UpdateSoundParams(c->handle, volume, sep,
       pitch)` (597). There is **no same-position rule** here (contrast 312-316).
       `I_UpdateSoundParams` is `handle = vol = sep = pitch = 0;` and nothing else
       (i_sound.c:674-688): **the recomputed parameters go nowhere** in either build.
     - NULL-origin and listener-origin channels are left alone.
   - **else** (600-605): **`S_StopChannel(cnum)`**: "if channel is allocated but sound has
     stopped, free it".
4. Music end check commented out (608-612).
5. Net effect: a call frees (a) every occupied channel whose `gametic >= handle`, and (b)
   every still-"playing" positional channel that is now inaudible from the listener. Nothing
   else changes; no `M_Random`; nothing is sent to the sound layer.

##### `S_SetSfxVolume(volume)` (s_sound.c:631-639)

`I_Error("Attempt to set sfx volume at %d")` unless 0 ≤ volume ≤ 127 (634-635), then
`snd_SfxVolume = volume` (637). Callers: `S_Init` (172), the menu (m_menu.c:830). It does not
touch channels; the new value applies to the next start and the next update.

##### `S_PauseSound()` / `S_ResumeSound()` (s_sound.c:497-513)

Music only: if `mus_playing && !mus_paused`, `I_PauseSong(handle)` (a no-op,
i_sound.c:848-852) and `mus_paused = true` (499-503); resume symmetric with `I_ResumeSong`
(no-op, i_sound.c:854-858) (508-512). **Sound-effect channels are not touched.** Callers: the
pause toggle in `G_Ticker` (g_game.c:707-711) and `G_InitNew` (g_game.c:1372-1375). While
paused, `G_Ticker` still runs and `gametic` still advances (d_net.c:744-746; `P_Ticker`
returns early when paused, p_tick.c:135), and `S_UpdateSounds` still runs every loop
iteration, so channels keep expiring during a pause.

##### Other functions in s_sound.c

`S_StartMusic(m_id)` = `S_ChangeMusic(m_id, false)` (644-647); `S_ChangeMusic`, `S_StopMusic`
(649-703) as in `S_Start` step 4; `S_SetMusicVolume` (616-627) as in `S_Init` step 4. None of
them touches channels or `M_Random`.

#### 1.1.6 What "playing" means, and exactly when a channel is freed, in both builds

1. `int I_SoundIsPlaying(int handle) { return gametic < handle; }` (i_sound.c:517-521), with
   the comment "Ouch." (519). It is the same in both builds. It does not look at any mixer or
   server state.
2. `gametic` is a global `int` (g_game.c:123), zero at process start, **never reset** (grep:
   the only writes are `gametic++` at d_main.c:384 and d_net.c:746). It counts every tic run
   since the process started, title-screen demos, intermissions and pauses included.
3. The handle is what `I_StartSound` returned (s_sound.c:389): build S the sfx id
   (i_sound.c:489); build M the next value of `addsfx`'s `static unsigned short handlenums`,
   100, 101, ..., 65535, then 100 again (i_sound.c:271, 334-340), counting every
   `I_StartSound` since the process started (menu sounds included).
4. Channels are freed in exactly five places: `S_UpdateSounds` when `gametic >= handle`
   (600-605); `S_UpdateSounds` when a still-"playing" positional channel is inaudible
   (592-595) (or a linked sound's volume < 1, 571-575, unreachable); `S_StopSound(origin)`
   (s_sound.c:349 before every start that passes the early returns, and `P_RemoveMobj`);
   `S_getChannel`'s same-origin and kick-out paths (844, 864); `S_Start` (209-211).
   **None of these depends on the sound's length.** Freeing a channel never silences anything:
   `I_StopSound` is a no-op (i_sound.c:505-514).
5. **`gametic` at the first tic of a `-warp` session**: with `-warp` (or `-record`) `autostart`
   is set and `D_DoomMain` calls `G_InitNew` before entering the loop (d_main.c:1163-1164),
   which ends in `G_DoLoadLevel` → `P_SetupLevel` → `S_Start` (g_game.c:1481, 484;
   p_setup.c:607). No tic has run, so `gametic` is 0 at level start, the first tic's
   `G_Ticker` runs with `gametic == 0`, it becomes 1 right after (d_net.c:745-746), and the
   first `S_UpdateSounds` (d_main.c:392) sees `gametic` = the number of tics the first
   `TryRunTics` ran. A game started from the title screen instead starts with `gametic` equal
   to however many tics the attract loop has run.
6. A sound started during the tic that runs with `gametic == g` is first examined by the
   `S_UpdateSounds` that follows that loop iteration's tics, when `gametic ≥ g + 1`.

**Build S (shipped): handle = sfx id N (1..108).** The channel counts as playing while
`gametic < N`. It is freed (if nothing else frees it first) by the first `S_UpdateSounds` with
`gametic ≥ N`.

- While `gametic < 108` (the first 108 tics of the process, about 3.1 s): a channel holding
  sfx N stays occupied until `gametic` reaches N, whenever it started, so high-numbered sounds
  hold channels longer, a pure artefact of the enum order.
- From `gametic ≥ 108` on, **every channel is freed by the first `S_UpdateSounds` after its
  start**, i.e. at the end of the loop iteration in which it started. The channel table then
  only matters *within* one loop iteration (one or more tics).
- Worked example (observed, 1.1.9 run A): `-warp` conditions, `gametic` 0, `numChannels` 3.
  In tic 0 the player fires (`sfx_pistol`, id 1, origin = listener) and a door 64 units away
  opens (`sfx_doropn`, id 20, sector origin). Channels: 0 = pistol, handle 1; 1 = doropn,
  handle 20. After tic 0 `gametic` = 1: the update frees channel 0 (1 < 1 is false) and keeps
  channel 1 (1 < 20). Channel 1 stays occupied through the updates at `gametic` 1..19 and is
  freed at the update with `gametic` = 20 (unless the listener walks out of earshot first, or
  a later start with priority ≤ 100 evicts it, or the sector starts another sound). During
  tics 1..19 only two channels are free for everything else.

**Build M (integrated mixer): handle = 100 + (k mod 65436), k = number of earlier
`I_StartSound` calls in the process.** The channel counts as playing while
`gametic < handle`.

- Worked example (observed, 1.1.9 run B): same start as above. Channel 0 = pistol, handle
  100; channel 1 = doropn, handle 101. The updates at `gametic` 1..22 free neither; with no
  other event they would be freed at the updates with `gametic` = 100 and 101 (about 2.9 s
  after a `-warp` start).
- In general, a start made when K sounds have been started before it gets handle 100 + K
  and holds its channel until `gametic` ≥ 100 + K. **While the process has started more than
  (gametic - 100) sounds, no channel ever times out**: channels are freed only by
  `S_StopSound(origin)`, the same-origin/kick-out paths, inaudibility in `S_UpdateSounds`, and
  `S_Start`. In a busy level the three channels stay full and a new sound gets one only by
  stopping a channel with its own origin or evicting one of priority value ≥ its own; others
  are refused (no `I_StartSound`, nothing mixed).
- Once `gametic` exceeds 65535 (65,536 tics, 31 min 12 s of tics since process start), every
  handle (at most 65535) is below `gametic` and build M frees every channel at the first update
  after its start, like build S after tic 108.
- The lifetime therefore depends on the process's whole history of sound starts (including
  menu and attract-mode sounds) and on `gametic`, not on the sound's length.

**What can and cannot differ between build S and build M** (from 1.1.0 item 5 and the
order of `S_StartSoundAtVolume`):

| Result | Can differ? | Why |
|---|---|---|
| Early returns 1 (link volume) and 2 (inaudible) | no | computed from `S_sfx`, `snd_SfxVolume`, positions, `gamemap` only (283-324) |
| `M_Random` draws: count, order, values | **no** | the draw (330, 340) precedes `S_StopSound` and `S_getChannel` and depends only on the above; channel state never influences it. So the status bar face's `rndindex` stream is the same in both builds |
| volume, separation, pitch of a start that reaches `I_StartSound` | no | fixed before the channel search (283-346) |
| **which starts reach `I_StartSound`** | **yes** | `S_getChannel` refuses (-1) depending on channel occupancy, which depends on handles |
| channel table contents (which slot, which origin, when freed) | **yes** | handles differ |
| which channel `S_StopSound(origin)` frees, and whether a start finds its origin's old channel | yes | occupancy |
| when the `ds<name>` lookup runs (and so a missing-lump `I_Error`) | yes | only after a channel is obtained (363-365) |
| the "not pre-cached - wtf?" stderr line | yes | build S prints it per channel obtained; build M never |
| what is audible | yes, entirely | build S: external `sndserver`; build M: in-process mixer, which also ignores s_sound's channel freeing (no-op `I_StopSound`) |

#### 1.1.7 Where `S_UpdateSounds` is called

1. Once per iteration of `D_DoomLoop`'s `while (1)` (d_main.c:369-406), at d_main.c:392:
   `S_UpdateSounds (players[consoleplayer].mo);// move positional sounds`. It is the only
   caller (grep).
2. Order in one iteration: `I_StartFrame` (372); then either one tic (singletics:
   `I_StartTic`, `D_ProcessEvents`, `G_BuildTiccmd`, `D_DoAdvanceDemo` if needed, `M_Ticker`,
   `G_Ticker`, `gametic++`, 375-386) or `TryRunTics` (389), which runs `counts` tics
   (d_net.c:736-766, each `M_Ticker`, `G_Ticker`, `gametic++`) and can also return having run
   **none** after waiting 20 tics for network input (d_net.c:727-732); then **`S_UpdateSounds`**
   (392); then `D_Display` (395); then (build M only) `I_UpdateSound` (397-400); then
   `I_SubmitSound` (402-405).
3. So `S_UpdateSounds` runs **once per loop iteration, not once per tic**. All sound starts of
   all tics in one iteration share the channel table between two updates; how many tics an
   iteration runs depends on wall-clock time (`I_GetTime`, d_net.c:648-677), so in vanilla the
   set of refused starts can depend on timing. The loop and the mixing schedule are covered
   in depth elsewhere in the note.

#### 1.1.8 The listener

1. At a start: `players[consoleplayer].mo` (s_sound.c:304-313). At an update: the argument,
   which is `players[consoleplayer].mo` (d_main.c:392). Always the console player, **not**
   `displayplayer` (the F12 spy view does not move the listener).
2. Fields read: `x`, `y`, `angle` of the listener (767-768, 780-786, 312-313); `x`, `y` of the
   origin (767-768, 780-783, 312-313). Never `z`.
3. `players[i].mo` is set by `P_SpawnPlayer` (p_mobj.c:675), set to NULL for deathmatch
   respawn (p_setup.c:672) and by save loading (p_saveg.c:95, 307). Before the first level it
   is NULL (static storage).
4. **At level start**: `S_Start` runs in `P_SetupLevel` before `Z_FreeTags` and before the
   things are loaded (p_setup.c:607, 618, 664), so during the rest of `P_SetupLevel`
   `players[consoleplayer].mo` still points at the previous level's (now freed) player mobj,
   or is NULL on the first level, until `P_SpawnPlayer` replaces it. Inference: a positional
   start in that window would read freed memory or dereference NULL; none was found in it
   (`G_CheckSpot` suppresses its telefog sound on the first frame with
   `players[consoleplayer].viewz != 1`, g_game.c:885-886, viewz being set to 1 at
   p_setup.c:604), but this section did not audit every call site.
5. **During death**: the player's mobj is not removed; `player->mo` keeps pointing at the
   corpse, which stays the listener. `P_DeathThink` turns `player->mo->angle` toward the
   attacker in ANG5 steps (p_user.c:200-221), so the listener's facing (and every separation)
   keeps changing while dead. In single player the level is reloaded on respawn, which runs
   `S_Start` again.
6. After a level ends (intermission, finale, title), `players[consoleplayer].mo` still points
   at the old level's player mobj, whose zone memory stays allocated until the next
   `P_SetupLevel` runs `Z_FreeTags` (p_setup.c:618), which is after `S_Start` has freed every
   channel (p_setup.c:607). `S_UpdateSounds` dereferences the listener only for occupied
   channels with a non-NULL origin other than the listener; such channels can survive the end
   of a level until they expire, and are then measured against old, still-allocated mobjs.
   Intermission and finale sounds use NULL origins (every `S_StartSound` in wi_stuff.c and
   f_finale.c passes `0` or `NULL`, by grep).

#### 1.1.9 Observations (running the genuine code)

Work directory `~/scratch/m5c/c-ssound/` (outside the repository). Compiler gcc 15.2.0,
`-std=gnu89 -g -O0 -w -DNORMALUNIX -DLINUX`, x86-64.

- **Run A, build S** (`h/`): `harness.c` `#include`s the genuine `s_sound.c` (so its statics
  are visible) and links the genuine `i_sound.c`, `sounds.c`, `m_random.c`, `tables.c`,
  `m_fixed.c`, plus `pta.c` = lines 291-388 of `r_main.c` (`R_PointToAngle`,
  `R_PointToAngle2`) verbatim with `viewx`/`viewy` defined. Declared adapters: `i_sound.c`
  compiled with `-Derrno=adapter_errno` (its `extern int errno;` at i_sound.c:166 does not
  link against modern glibc's TLS errno; affects only `myioctl`, never called); stub
  `I_Error` (longjmp), `W_*` (every lump found), `Z_*` (calloc). Output `h/out.txt`,
  sha256 prefix `d5466b53c5809b31`.
- **Run B, build M** (`nosrv/`): the same, compiled from copies whose `doomdef.h` line 84 is
  commented out (the only edit; declared). `I_InitSound` not called (so `data` is 0 and the
  stderr line appears there too: an artefact of the harness, not of build M). Output
  `nosrv/out.txt`, sha256 prefix `c869257c8357e086`.

Results:

1. **Test 1, `S_AdjustSoundParams`**: 576 cases (maps 1 and 8; `snd_SfxVolume` 8, 15, 1; 16
   offsets including 159/160/161, 1199/1200/1201 units and ±20000 on both axes; 6 listener
   angles including 0, 0x40000000, 0x80000000, 0xC0000000, 0x12345678, 0xFFFFFFFF). An
   independent Python restatement of 1.1.5's prose (`enc.py`, which parses `finesine` and
   `tantoangle` from tables.c and re-implements `SlopeDiv` and `R_PointToAngle`; written from
   this section, not from Rust) reproduces rc, vol and sep in **576 of 576** cases, with
   truncating division on map 8 (a case like d = 1000 on map 8 at volume 8 gives 14; floor
   would give 13). `*pitch` was never written (576 of 576 rows kept the sentinel). Test 1's output is
   byte-identical between runs A and B. `finesine[0..8191]` min/max counted: -65535/65535.
2. **Test 2, lifetime**: build S: pistol (handle 1) freed at the first update (`gametic` 1),
   doropn (handle 20) freed exactly at the update with `gametic` 20. Build M: handles 100 and
   101, neither freed through `gametic` 22.
3. **Test 3, allocation and priority** (`gametic` held at 0 so nothing expires): channels filled
   by posact(120), stnmov(119), sawidl(118); bgact(120) evicted channel 0 (first with
   priority ≥ 120); dmact(120) evicted channel 0 again (equal priority evicts); telept(32)
   evicted channel 0 (first in index order with ≥ 32); a NULL-origin oof(96) evicted channel 1
   (stnmov 119); a NULL-origin noway took channel 1 again (it freed the NULL-origin oof via
   `S_StopSound(NULL)` and reused the freed slot); slop from the origin of channel 0 reused
   channel 0. Same channel choices in both builds; handles differ as in 1.1.6.
4. **Test 4, `M_Random` draws**: itemup 0, tink 0, sawup 1, pistol 1, a pistol 5000 units away on
   map 1: 0 (inaudible), the same on map 8: 1, chgun 1, chgun at `snd_SfxVolume` 0: 0 (link
   volume < 1 return), a NULL-origin pistol at volume 0: 1, a sound refused for lack of a
   channel (posact 120 against three telept 32): 1 draw, and the channel table unchanged.
5. **Test 5**: `sfx_id` 0 gives `I_Error: Bad sfx #: 0`; `sfx_id` = `NUMSFX` (109) passed the check
   (no `I_Error`), confirming the off-by-one (what it then did depends on the memory after
   `S_sfx`; it is undefined).
6. stderr: "16bit and not pre-cached - wtf?" printed 21 times in run A, once per start that
   obtained a channel (counted per section with unbuffered output).
7. **IWAD lump presence** (local only; `dscheck.py` reads the WAD directories of
   `/mnt/d/DoomIWADs/DOOM.WAD` (sha256 prefix 6fdf361847b46228) and `DOOM2.WAD`
   (10d67824b11025dd) read-only; no contents read): for the 108 sfx ids 1..108, the lump
   `ds<name>` that `I_GetSfxLumpNum` looks up is missing for **41 ids in DOOM.WAD** (4-7,
   28-30, 46-50, 53, 54, 56, 70-74, 78-80, 86, 91, 92, 94-108) and **1 id in DOOM2.WAD (86,
   `sfx_chgun`)**. Starting any of those, once it obtains a channel, is a fatal `I_Error` in
   both builds. Whether Doom 1 game code can start any of the 41 was not audited here (call
   sites are §1.2's subject); `sfx_chgun` is never started.

   Note (2026-11-18): the call-site audit was done afterwards (§1.2). A local check that day found a lump for each of the 43 sounds named at the Episode 1 call sites, so none of the 41 can be started there, and the lump for `sfx_pdiehi` is present in both IWADs (decision T11 in §5).

Files that matter for Phase 2: `~/scratch/m5c/c-ssound/h/harness.c`, `h/pta.c`,
`h/errno_adapter.c`, `nosrv/` (the build-M copy, `doomdef.h` edited at line 84 only),
`enc.py` (independent restatement of `S_AdjustSoundParams`), `table.md` (the 109-row table),
`dscheck.py`.

#### 1.1.10 Order sensitivity within a tic

Everything below depends on the order of sound starts and on positions at the moment of each
call (mid-tic), so the Rust simulation must deliver sound starts in C order with the
positions current at each call:

1. **`M_Random`**: one draw per qualifying start, in call order (330, 340). The draw's value
   sets that start's pitch, and the shared `rndindex` feeds `ST_Ticker`'s
   `st_randomnumber` (st_stuff.c:992). Any reordering of starts changes which pitch each gets;
   adding or removing a qualifying start shifts every later draw, including the face.
2. **Audibility** is decided from positions at the time of the call (767-777), so whether a
   start draws depends on whether it happens before or after a move in the same tic.
3. **Origin uniqueness** (proved from the code): at most one occupied channel per origin
   value, NULL included. Proof: initially none (184, 209-211); a start calls
   `S_StopSound(origin)` (349), which frees the first, hence (by induction) the only, channel
   with that origin; `S_getChannel` then installs one channel with that origin (871-872);
   every other path only frees. Consequences: a start replaces its origin's previous channel,
   so **all NULL-origin sounds share one channel at a time**; `P_RemoveMobj`'s
   `S_StopSound` (p_mobj.c:566) frees the removed mobj's only channel; `S_getChannel`'s
   same-origin branch (842-846) is unreachable from 352.
4. **Channel allocation**: first free slot in index order (838-841); kick-out picks the
   first slot in index order with priority value ≥ the new one (853-854). Which slot a sound
   takes, which one it evicts, and whether it is refused all depend on the order of starts
   since the last `S_UpdateSounds` (and, in build S before tic 108 or build M generally,
   since much earlier).
5. **Refusal is audible**: a refused start never reaches `I_StartSound` (354-355). Its draw
   has already been made, and `S_StopSound` has already freed its origin's old channel.
6. **`S_UpdateSounds` order**: frees in index order with the listener's position at that time
   (after all of the iteration's tics). Freeing order does not affect anything else in the
   same call.
7. **Missing-lump `I_Error`** happens at the first start of that sfx that obtains a channel
   (363-365), so whether a session dies can depend on order and occupancy.
8. **`S_Start` mid-tic**: a level load from `G_Ticker` (g_game.c:622) clears every channel at
   that point in the tic.

#### 1.1.11 Re-verification of docs/M5B_REFERENCE_INVESTIGATION.md

Lines re-read: s_sound.c:277-395, against M5B §1.3.6 item 2 (the table row for
s_sound.c:330, 340), §1.3.6 item 4, §2.5 FH1 (the "finding F1" of S15), §2.6, the S15 text and
line 110.

1. "the pitch variation of every started sound except `sfx_itemup` and `sfx_tink`":
   **agrees**, with the precision that the saw sounds (ids 10-13) draw too (a different mask,
   `&15`), and that the draw is made also for starts that then get **no channel**
   (354-355) and for NULL/listener-origin starts at volume 0. "Started" should be read as
   "passed the early returns", not "got a channel".
2. "It runs after the early returns for linked sounds with volume < 1 (283-294) and for sounds
   that `S_AdjustSoundParams` finds inaudible from the listener (304-320)": **agrees**. The
   only other exit before it is the fatal range check (277-278).
3. "It runs for game sounds started during `P_Ticker` and for menu sounds": **incomplete, not
   wrong**: any caller counts (e.g. `HU_Responder`'s chat sounds, hu_stuff.c:566-568; level
   transitions; intermission), not only `P_Ticker`.
4. FH1: "one draw per started sound that passes its link-volume check and, for a sound with
   another origin than the player, the audibility test ... no distance limit on map 8 ...
   (s_sound.c:283-343)": **agrees**. Precisions: "the player" is exactly
   `players[consoleplayer].mo` (304); the range should end at 346 (the second clamp ends at
   345-346), a cosmetic difference; "no distance limit on map 8" holds for every
   `gamemap == 8` in every episode, and with `snd_SfxVolume` > 0 such sounds are never
   inaudible (1.1.5, `S_AdjustSoundParams` item 8).
5. FH1: "is the same at any other volume only for the player's own and origin-less sounds":
   **agrees** (those skip the audibility test; on map 8 positional starts are also
   volume-independent for any volume ≥ 1).
6. §2.6: the stub layer "affect[s] channel bookkeeping only, which comes after the draws
   (s_sound.c:348-355)": **agrees**; 1.1.6's table shows the draws cannot depend on the
   channel model in either build. So FH1's measured draw counts do not depend on the
   unreviewed stub.
7. No disagreement found with the M5B note's statements about s_sound.c:283-343. The M5B
   note does not mention that linuxdoom-1.10 is built with `SNDSERV` (doomdef.h:84); that
   does not affect its M_Random claims (1.1.6), but it matters for M5c.

#### 1.1.12 Implementation-ready summary

**State a port must keep.**

- Global: `snd_SfxVolume` (default 8); `numChannels` (default 3); the channel array; the
  `M_Random` index (shared with the status bar); `gametic`; the per-sfx "lump checked" flag
  (only to reproduce the missing-lump `I_Error`, or replaced by a pre-check); build M only: the
  handle counter (`unsigned short`, starts 0, 0 becomes 100 before use).
- Per channel: occupied sfx id (or free), origin identity (mobj handle, sector, or none),
  handle (`int`). Origin identity must compare equal exactly when the C pointers would; the
  origin must give current x, y while the channel is occupied.
- Not needed: `usefulness`, `lumpnum` values, `singularity`, `nextcleanup`, `snd_MusicVolume`,
  music state (no effect on sound effects).

**Pseudocode, equivalent to the C under the default configuration** (all integers 32-bit,
`>>` arithmetic, `/` truncating, angles mod 2^32):

```
adjust(L, S):                         # returns None (inaudible) or (vol, sep)
  adx = |L.x - S.x|; ady = |L.y - S.y|
  d = adx + ady - (min(adx, ady) >> 1)
  if gamemap != 8 and d > 1200<<16: return None
  a = point_to_angle(S.x - L.x, S.y - L.y)          # R_PointToAngle2(L.x, L.y, S.x, S.y)
  a = (a > L.angle) ? a - L.angle : a + (0xFFFFFFFF - L.angle)
  sep = 128 - ((96 * finesine[a >> 19]) >> 16)
  if d < 160<<16:      vol = SFXVOL
  elif gamemap == 8:   vol = 15 + ((SFXVOL - 15) * ((1200<<16) - min(d, 1200<<16) >> 16)) / 1040
  else:                vol = (SFXVOL * (((1200<<16) - d) >> 16)) / 1040
  return (vol > 0) ? (vol, sep) : None

start(origin, id):                    # S_StartSound
  if id < 1 or id > 109: fatal         # (109 is out of bounds in C: refuse it)
  vol = SFXVOL; pitch = 128            # (chgun link branch: unreachable)
  if origin != none and origin != listener():
     r = adjust(listener(), origin)
     if r is None: return
     (vol, sep) = r
     if origin.xy == listener().xy: sep = 128
  else: sep = 128
  if 10 <= id <= 13: pitch = clamp(pitch + 8 - (M_Random() & 15), 0, 255)
  elif id not in (32, 87): pitch = clamp(pitch + 16 - (M_Random() & 31), 0, 255)
  stop_first(origin)                   # first occupied channel with this origin (none included)
  c = first free channel
  if c is None: c = first channel with prio[chan.id] >= prio[id]; if None: return; free(c)
  chan[c] = (id, origin)
  if lump "ds"+name[id] missing: fatal
  chan[c].handle = (build S) id | (build M) next_handle()
  emit sound start (id, vol, sep, pitch, chan c)

update(L):                            # S_UpdateSounds, once per loop iteration
  for c in 0..numChannels-1 if occupied:
     if not (gametic < chan[c].handle): free(c)
     elif chan[c].origin != none and chan[c].origin != L and adjust(L, chan[c].origin) is None: free(c)

level_start(): free all channels      # S_Start
remove_mobj(m): stop_first(m)         # P_RemoveMobj
```

Configuration flags: `numChannels` changes how many starts are refused and which slot each
gets (0 refuses every start, still drawing); `snd_SfxVolume` changes audibility and hence the
draws (except map 8, NULL and listener origins) and every volume; the build (S or M) changes
handles and so channel lifetime and refusals, never draws.

#### 1.1.13 Undefined, implementation-defined or layout-dependent behaviour

1. **Off-by-one range check**: `sfx_id > NUMSFX` (277) lets `sfx_id == 109` index
   `S_sfx[109]`, past the end of the 109-entry array (sounds.c:114-227): undefined. No call
   site with 109 was found (not audited exhaustively).
2. **Signed overflow** in `listener->x - source->x` (767-768) if the two x (or y) differ by
   2^31 or more (32768 map units), `abs(INT_MIN)`, and `adx + ady` (771) when the sum exceeds
   2^31-1 (both offsets around 16384 units or more). Undefined in C; gcc at -O0 wraps (run A's
   ±20000 case wrapped twice and ended positive, clipped). Real maps would need to span more
   than 16384 units.
3. **Right shift of a negative `int`** (`>> FRACBITS` of a negative `FixedMul` result, 793)
   and **division of a negative `int`** (map 8, 805-807): implementation-defined (C89);
   gcc shifts arithmetically and truncates division toward zero (observed, 1.1.9 item 1).
4. **The degenmobj cast**: sector sound origins are `degenmobj_t` (`thinker_t thinker;
   fixed_t x, y, z;`, r_defs.h:88-95) passed as `void*` and read through `mobj_t*`
   (s_sound.c:268, 529); only `x` and `y` are read, which sit at the same offsets as in
   `mobj_t` (`thinker_t thinker; fixed_t x, y, z;`, p_mobj.h:207-215). Layout-dependent and
   formally an aliasing violation; works with gcc's layout. Listener `angle` is only read from
   a real `mobj_t`.
5. **Freed mobjs**: by the origin invariant (1.1.10 item 3), `P_RemoveMobj`'s `S_StopSound`
   frees the removed mobj's only channel before the thinker is freed. A start with a removed
   (not yet freed) mobj as origin later in the same tic would leave a channel pointing at
   memory freed by `P_RunThinkers`; this section did not audit call sites for this. Level change is safe
   (`S_Start` before `Z_FreeTags`, p_setup.c:606-618). The listener pointer is stale between
   `S_Start` and `P_SpawnPlayer` (1.1.8 item 4).
6. **NULL listener**: a non-NULL-origin start or update while `players[consoleplayer].mo` is
   NULL dereferences NULL (306, 312, 586). Not reached in single player as far as was found.
7. **Uninitialised channel fields**: `origin` and `handle` are not initialised by `S_Init`
   (179-184) but are only read when `sfxinfo` is non-NULL, and every path that sets `sfxinfo`
   sets `origin` (871-872) and then `handle` (389), except when the lump lookup `I_Error`s in
   between (fatal anyway). `sep` in `S_StartSoundAtVolume` is only read after being set.
8. **`numChannels` from a config file** is unchecked (m_misc.c:282, 367-402): negative makes
   `Z_Malloc` a negative size (undefined/zone error); 0 refuses every start.
9. **`snd_SfxVolume` from a config file** outside 0..127 is fatal at `S_Init` (634-635); values
   16..127 are accepted (the menu only offers 0..15).
10. **`R_PointToAngle2` writes `viewx`, `viewy`** (r_main.c:384-385) from inside the playsim;
    harmless as far as found (1.1.5).
11. Build M only, outside s_sound.c: `lengths[(S_sfx[i].link - S_sfx)/sizeof(sfxinfo_t)]`
    (i_sound.c:811) divides an element count by `sizeof`, so chgun gets `lengths[0]` = 0;
    unreachable since chgun is never started.
12. Build S: `I_SubmitSound` writes 2048 bytes of zeros to file descriptor 0 every loop
    iteration (d_main.c:402-405, i_sound.c:665-670, 106), never opened for writing by this
    build.

#### 1.1.14 Questions the sound logic's facts raise for the owner

Each question is taken up in [§5](#5-owner-decisions-t1t16-with-recommendations): 1 in T2; 2 in T5; 3 in T5 (the process history and the counters at level start) and T15 (restart); 4 in T11; 5 in T11.

1. **Reference build: shipped build S (doomdef.h:84 `SNDSERV`, sound via the external
   `sndserver`, handle = sfx id) or build M (doomdef.h:84 removed, in-process mixer, handle =
   start counter)?** They agree on every `M_Random` draw and on the parameters of every start,
   but differ in which starts reach `I_StartSound`. Recommendation: build M, declared as an
   explicit configuration of the reference (one-line `doomdef.h` change), because M5c's goal
   is to match the `i_sound.c` mixer, which only build M runs; matching s_sound's channel
   logic from build S while mixing as build M would be a hybrid no binary ever ran. The
   alternative, build S, keeps the shipped binary but puts the audible output in the
   out-of-scope `sndserv/`.
2. **Channel-update schedule: one `S_UpdateSounds` per tic, or the loop's real
   tics-per-iteration?** Refusals depend on how many tics share an update (1.1.7 item 3),
   which in vanilla depends on wall-clock time. Recommendation: declare one update after each
   tic (the `-singletics` shape, d_main.c:375-392) as the reference schedule, and record it in
   COMPATIBILITY; draws are unaffected either way.
3. **Process history in build M**: handles depend on all `I_StartSound` calls since process
   start and channels on `gametic` since process start. Either (a) define the reference as a
   `-warp` process with no sound before `G_InitNew` (handle counter 0, `gametic` 0 at level
   start) and carry both counters in saved state, or (b) refuse sessions that do not start
   that way. Recommendation: (a).
4. **Missing `ds` lumps** (41 ids for DOOM.WAD): reproduce vanilla's fatal `I_Error` on the
   first start that obtains a channel, or refuse such a session up front as a fidelity gap?
   Recommendation: a refuse-the-tic witness at the exact start that would `I_Error`, with
   canonical state intact, per AGENTS.md.
5. **`sfx_id == 109`**: refuse explicitly (recommended) rather than model the out-of-bounds read.

#### 1.1.15 What this section did not do or observe

- This section did not run the full engine, any real map, or the C reference suite; runs A and B are
  unit-level harnesses over the genuine s_sound.c, i_sound.c, sounds.c, m_random.c, tables.c,
  m_fixed.c and the extracted `R_PointToAngle(2)`, with the stubs listed in 1.1.9.
- This section did not run `I_InitSound` in either build, open `/dev/dsp`, or start `sndserver`, and did not
  read `sndserv/` beyond the `singularity` grep.
- This section did not audit the call sites of `S_StartSound` (which sounds Doom 1 can start, starts
  during `P_SetupLevel`, starts on removed mobjs); 1.1.8 item 4 and 1.1.13 items 1 and 5 depend
  on that audit.
- This section did not test the 65535 handle wrap or `gametic` > 65535 by running them; they follow from
  i_sound.c:271, 334-336 and i_sound.c:520.
- No Rust was built or run. No sanitizer was run. No file inside the repository was written.

### 1.2 Every call that starts or stops a sound

Written 2026-11-18 (America/Los_Angeles). Source: linuxdoom-1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`, read in `/home/copgunn/src/DOOM/linuxdoom-1.10`. Every
file:line below was read for this section. This section lists what each call **passes** (sound
number, origin, guard, trigger, moment in the tic). What `s_sound.c` does with those arguments
(attenuation, channels, priorities, pitch) belongs to the `s_sound.c` section and is only touched
here where a call's meaning depends on it.

**Terms.**
- *Call site*: one source line that calls `S_StartSound`, `S_StartSoundAtVolume` or `S_StopSound`.
- *Origin*: the first argument of `S_StartSound(origin, sfx_id)`, a `void*` (s_sound.c:398-401) that
  `S_StartSoundAtVolume` casts to `mobj_t*` (s_sound.c:268). It is either `NULL`, a map object
  (`mobj_t`, a "mobj"), or the address of something else cast to `mobj_t*`.
- *Listener*: `players[consoleplayer].mo`, the console player's mobj; `S_StartSoundAtVolume`
  measures against it (s_sound.c:304-306).
- *(L)*: reachable while a single-player level runs with no menu, automap, chat, cheat or pause.
  *(X)*: not reachable then (the reason is given).
- *Phase*: the part of the tic in which a call runs (defined in 1.2.4).
- *E1*: reachable on the nine Episode 1 maps (shareware or registered) in single player.

Separation used below: **(S)** read in the source; **(O)** observed by running something (command,
counts, digests given); **(I)** inference.

#### 1.2.1 The inventory: all 167 lines accounted for

**(S)** `grep -n "S_StartSound\|S_StopSound" *.c` in linuxdoom-1.10 prints **167 lines** in 18
files (command run in the source directory; **(O)** the count is 167, and the per-file counts below
sum to 167). `S_StartSoundAtVolume` is called only inside `s_sound.c` (s_sound.c:407). The other
interface functions asked about:

| Function | Call lines in *.c | Class |
| --- | --- | --- |
| `S_Start` | p_setup.c:607 (in `P_SetupLevel`) | level load, see 1.2.7 |
| `S_UpdateSounds` | d_main.c:392 (in `D_DoomLoop`, once per frame after the tics) | outside the tic (X for this section) |
| `S_PauseSound` / `S_ResumeSound` | g_game.c:709, 711 (pause button in `G_Ticker`), g_game.c:1375 (`G_InitNew`) | pause, X; both touch only music (s_sound.c:497-513) |
| `S_SetSfxVolume` | m_menu.c:830 (menu), s_sound.c:172 (`S_Init`) | X |

Per-file accounting of the 167 lines (L = level call sites, X = outside a level run, D = not a call
site: a definition, a comment or `s_sound.c` internals):

| File | Lines | Class | Lines and reason |
| --- | --- | --- | --- |
| p_mobj.c | 8 | 7 L, 1 X | L: 104, 307, 379, 386, 566 (`S_StopSound`), 903, 976. X: 610 (`P_RespawnSpecials` returns unless `deathmatch == 2`, p_mobj.c:592-593) |
| p_plats.c | 10 | 10 L | 68, 77, 86, 115, 126, 194, 204, 217, 230, 248 |
| p_ceilng.c | 4 | 4 L | 75, 91, 118, 128 |
| p_doors.c | 20 | 20 L | 77, 83, 89, 108, 132, 161, 228, 240, 253, 301, 309, 316, 327, 337, 379, 393, 406, 444, 449, 453 (none lies in the `#if 0` blocks at p_doors.c:42-53 and 557-764) |
| p_floor.c | 2 | 2 L | 219, 250 |
| p_enemy.c | 35 | 35 L | 657, 660, 729, 774, 815, 832, 855, 923, 988, 1098, 1113, 1210, 1235, 1247, 1253, 1323, 1354, 1431, 1565, 1568, 1574, 1580, 1761, 1767, 1773, 1782, 1790, 1803, 1839, 1845, 1870, 1922, 1932, 1951, 2007 |
| p_pspr.c | 10 | 10 L | 146, 299, 489, 520, 523, 651, 675, 705, 737, 822 |
| p_switch.c | 3 | 3 L | 228, 240, 252 |
| p_spec.c | 1 | 1 L | 1151 |
| p_map.c | 1 | 1 L | 1104 |
| p_telept.c | 2 | 2 L | 112, 118 |
| p_inter.c | 2 | 1 L, 1 X | L: 660. X: 194 (in `P_GiveWeapon`'s branch `if (netgame && (deathmatch!=2) && !dropped)`, p_inter.c:176-178) |
| g_game.c | 1 | X | 886 (`G_CheckSpot`, called only from `G_DeathMatchSpawnPlayer` g_game.c:909 and the netgame branch of `G_DoReborn` g_game.c:947, 956; `G_DeathMatchSpawnPlayer` is called from g_game.c:943 and p_setup.c:673, both deathmatch) |
| hu_stuff.c | 2 | X | 566, 568 (`HU_Ticker` incoming chat, inside `if (netgame)`, hu_stuff.c:535-537) |
| m_menu.c | 30 | X | 686, 694, 723, 1011, 1087, 1089, 1507, 1526, 1533, 1545, 1550, 1556, 1564, 1569, 1573, 1578, 1584, 1588, 1593, 1614, 1630, 1640, 1648, 1657, 1670, 1675, 1683, 1692, 1701, 1708: menus and `M_Responder` keys (see the note below) |
| wi_stuff.c | 24 | X | 905, 913, 952, 961, 1132, 1139, 1158, 1165, 1182, 1189, 1208, 1215, 1234, 1242, 1343, 1352, 1357, 1366, 1371, 1380, 1385, 1393, 1408, 1417: intermission (`GS_INTERMISSION`) |
| f_finale.c | 4 | X | 411, 457 (`F_CastTicker`), 517 (`F_CastResponder`), 688 (`F_BunnyScroll`): finale |
| s_sound.c | 8 | D | 255 and 398 (definitions of `S_StartSoundAtVolume` and `S_StartSound`), 471 (definition of `S_StopSound`), 273 and 377 (inside comments), 372 (inside an `fprintf` string), 349 (`S_StartSoundAtVolume`'s own `S_StopSound(origin)`), 407 (`S_StartSound`'s call of `S_StartSoundAtVolume`) |

Totals: **96 L**, **63 X** (30 + 24 + 4 + 2 + 1 + 1 + 1), **8 D** = 167. No file outside this list
mentions either name; st_stuff.c (status bar and cheats), am_map.c (automap) and d_main.c have no
start or stop call. p_inter.c:731 is a comment naming `I_StartSound`, not counted by the grep.

**Note on m_menu.c (S, I).** Not all menu sounds need the menu open. `M_Responder`'s "F-Keys" block
runs `if (!menuactive)` (m_menu.c:1519-1520): the screen-size keys `-` and `=` play
`S_StartSound(NULL,sfx_stnmov)` (m_menu.c:1526, 1533; refused only while the automap or chat is on,
m_menu.c:1523-1524, 1530-1531), F5 (detail) and F8 (messages) play `sfx_swtchn` without opening a
menu (m_menu.c:1569, 1584), and F1-F4, F6, F7, F9, F10 and Escape open a menu with `sfx_swtchn`. These
run from the event responder between tics, not from `G_Ticker`. They are menu sounds and out of M5c's
scope, but each is a NULL-origin start and so reaches `S_StartSoundAtVolume`'s pitch draw on
`M_Random` (s_sound.c:304, 326-343). See the questions at the end.

#### 1.2.2 What a call passes: the five kinds of origin

**(S)** `S_StartSoundAtVolume` treats the origin in three ways (s_sound.c:304-324): if
`origin && origin != players[consoleplayer].mo` it calls `S_AdjustSoundParams` with the listener and
the origin, forces centre separation when the two have equal `x` and `y` (s_sound.c:312-316), and
drops the sound if the result is 0 (s_sound.c:318-319); otherwise (NULL origin, or the origin *is*
the listener) it keeps full volume and centre separation (s_sound.c:321-324). Every start then calls
`S_StopSound(origin)` (s_sound.c:349), which stops the first playing channel whose origin pointer
equals this one (s_sound.c:471-482). So the **identity** of the pointer matters as well as its
position. The call sites pass five kinds:

1. `NULL`: no position; full volume, centre. Used for item pickups (p_inter.c:660), locked-door
   "oof" (p_doors.c:228, 240, 253, 379, 393, 406), boss sight and death (p_enemy.c:657, 1565), the
   boss-brain sounds (p_enemy.c:1839, 1845, 1870, 1922), and the switch sound whenever button slot 0
   is free (1.2.6.3). All NULL-origin starts share one identity, so each stops the previous one that
   is still playing (s_sound.c:349, 471-482).
2. The listener's own mobj, written `player->mo` (weapons p_pspr.c, p_enemy.c:1782-1803), `mo` where
   `mo->player` is set (p_mobj.c:307, `A_Pain`/`A_XScream`/`A_PlayerScream` on the player) or
   `usething` (p_map.c:1104, set to `player->mo` at p_map.c:1138). In single player this pointer
   equals `players[consoleplayer].mo`, so it gets full volume and centre (s_sound.c:304, 321-324),
   wherever the player is. All of these share one identity.
3. Another mobj: a monster, a missile, a barrel or a fog just spawned. Positional.
4. A sector's sound origin `(mobj_t *)&sec->soundorg`, a `degenmobj_t` (r_defs.h:88-95: a
   `thinker_t` then `x`, `y`, `z`), embedded in `sector_t` (r_defs.h:121) and cast to `mobj_t*`. Its
   `x`,`y` are computed once at load (1.2.6.2) and never change. One identity per sector, shared by
   that sector's doors, plats, floors and ceilings and by switches whose button slot 0 holds a line
   facing it.
5. The address of a `button_t` field, `(mobj_t *)&buttonlist[i].soundorg` (p_spec.c:1151), the
   switch-release origin. It is not a mobj or a `degenmobj_t`; see 1.2.6.3.

**(S)** A sound number below 1 stops the program: `if (sfx_id < 1 || sfx_id > NUMSFX)
I_Error("Bad sfx #: %d", sfx_id);` (s_sound.c:277-278). `sfx_None` is 0 (sounds.h:179). Every call
site that passes an `info` sound field guards it against 0 (listed in §1.2.8, item 4).

#### 1.2.3 The (L) call sites

Columns: **Site**; **Function**; **Sound** (constant, or the expression and every value it can take,
with the gameplay random draws, `P_Random`, that pick it); **Origin** exactly as written; **Guard**
(conditions on the path to the call); **Reached from**; **Phase** (codes defined in 1.2.4: PT player
think, PM the player's own mobj thinker, MT a monster's or barrel's own thinker, MS a missile's or
fog's thinker, SE a sector thinker, US `P_UpdateSpecials`, LD level load; "dmg" means "inside
`P_DamageMobj`/`P_KillMobj`, in whatever phase the damage was dealt"); **E1** (yes, or why not).

##### Weapons and the player (p_pspr.c, p_mobj.c, p_map.c, p_enemy.c, p_inter.c)

| Site | Function | Sound | Origin | Guard | Reached from | Phase | E1 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| p_pspr.c:146 | `P_BringUpWeapon` | `sfx_sawup` | `player->mo` | `player->pendingweapon == wp_chainsaw` (145) | `A_Lower` when the old weapon is down (p_pspr.c:415), and `P_SetupPsprites` (p_pspr.c:841) from `P_SpawnPlayer` (p_mobj.c:686) at load | PT; LD | yes (chainsaw pickup on E1M2, E1M3, E1M5, E1M7, E1M9, census in 1.2.8) |
| p_pspr.c:299 | `A_WeaponReady` | `sfx_sawidl` | `player->mo` | `player->readyweapon == wp_chainsaw && psp->state == &states[S_SAW]` (297-298) | entering `S_SAW` (info.c:203, 4 tics) from `S_SAWB` (info.c:204, 4 tics) or the raise/refire: once per 8 tics while the saw idles | PT (`P_MovePsprites` p_user.c:334, or p_user.c:187 when dead) | yes |
| p_pspr.c:489 | `A_Punch` | `sfx_punch` | `player->mo` | `if (linetarget)` after the attack (487) | `S_PUNCH2` (info.c:142) | PT | yes |
| p_pspr.c:520 | `A_Saw` | `sfx_sawful` | `player->mo` | `!linetarget` (518) | `S_SAW1`, `S_SAW2` (info.c:207-208) | PT | yes |
| p_pspr.c:523 | `A_Saw` | `sfx_sawhit` | `player->mo` | `linetarget` set | same | PT | yes |
| p_pspr.c:651 | `A_FirePistol` | `sfx_pistol` | `player->mo` | none (first statement) | `S_PISTOL2` (info.c:150) | PT | yes |
| p_pspr.c:675 | `A_FireShotgun` | `sfx_shotgn` | `player->mo` | none | `S_SGUN2` (info.c:158) | PT | yes |
| p_pspr.c:705 | `A_FireShotgun2` | `sfx_dshtgn` | `player->mo` | none | `S_DSGUN2` (info.c:172); the super shotgun is selectable only when `gamemode == commercial` (p_user.c:298-303, p_pspr.c:187-192) | PT | no (Doom II) |
| p_pspr.c:737 | `A_FireCGun` | `sfx_pistol` | `player->mo` | none; **the sound precedes the ammo test** `if (!player->ammo[...]) return;` (739-740), so the second frame plays it with no ammo | `S_CHAIN1`, `S_CHAIN2` (info.c:188-189) | PT | yes |
| p_pspr.c:822 | `A_BFGsound` | `sfx_bfg` | `player->mo` | none | `S_BFG1` (info.c:220) | PT | no (no BFG on E1 maps; refused in shareware, p_user.c:307-317) |
| p_enemy.c:1782, 1790, 1803 | `A_OpenShotgun2`, `A_LoadShotgun2`, `A_CloseShotgun2` | `sfx_dbopn`, `sfx_dbload`, `sfx_dbcls` | `player->mo` | none | `S_DSGUN5`, `S_DSGUN7`, `S_DSGUN9` (info.c:175, 177, 179) | PT | no (Doom II) |
| p_mobj.c:976 | `P_SpawnPlayerMissile` | `th->info->seesound`: `sfx_rlaunc` for `MT_ROCKET` (info.c:1971), `sfx_plasma` for `MT_PLASMA` (info.c:1997); `MT_BFG` has 0 (info.c:2023) | `th`, the missile just spawned at the player's `x`,`y` and `z+32` (p_mobj.c:969-973) | `if (th->info->seesound)` (975) | `A_FireMissile` (p_pspr.c:556), `A_FirePlasma` (p_pspr.c:588), `A_FireBFG` (p_pspr.c:569) | PT | rocket yes; plasma, BFG no |
| p_mobj.c:307 | `P_ZMovement` | `sfx_oof` | `mo` (the player's mobj) | `mo->player && mo->momz < -GRAVITY*8` when hitting the floor (299-302) | the player's mobj thinker (p_mobj.c:428-431) | PM | yes |
| p_map.c:1104 | `PTR_UseTraverse` | `sfx_noway` | `usething` (= `player->mo`, p_map.c:1138) | the first line on the use path has no special and `openrange <= 0` (1099-1102) | `P_UseLines` (p_map.c:1148) from `P_PlayerThink` when use is newly pressed (p_user.c:322-327) | PT | yes |
| p_inter.c:660 | `P_TouchSpecialThing` | `sound`: `sfx_itemup` by default (358); `sfx_getpow` for soul sphere, mega sphere, invulnerability, berserk, invisibility, suit, map, visor (406, 416, 492, 501, 508, 515, 522, 529); `sfx_wpnup` for weapons (606-648) | `NULL` | `player == &players[consoleplayer]` (659); reached only if the item was taken (each case `return`s when the give fails); keys `break` to the sound only `if (!netgame)` (424-470) | `PIT_CheckThing` with `tmflags & MF_PICKUP` (p_map.c:331-337) inside `P_CheckPosition`; that runs from `P_TryMove` (p_map.c:463) in the player's `P_XYMovement`, and from `P_ThingHeightClip` (p_map.c:536) when `P_ChangeSector` (p_map.c:1261) moves a floor or ceiling around the player | PM; SE | yes (mega sphere is commercial-only, 409-411) |
| p_enemy.c:2007 | `A_PlayerScream` | `sfx_pldeth`; `sfx_pdiehi` only if `gamemode == commercial && mo->health < -50` (1997-2004) | `mo` (the player's mobj) | none | `S_PLAY_DIE2` (info.c:295), entered from `S_PLAY_DIE1` (10 tics minus `P_Random()&3`, p_inter.c:726-729) | PM | yes |

`A_Pain` and `A_XScream` on the player are listed with the monster rows (p_enemy.c:1580, 1574).

##### Monsters (p_enemy.c)

| Site | Function | Sound | Origin | Guard | Reached from | Phase | E1 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| p_enemy.c:660 | `A_Look` | `sound`: if `seesound` is `sfx_posit1/2/3`, `sfx_posit1 + P_Random()%3` (637-640); if `sfx_bgsit1/2`, `sfx_bgsit1 + P_Random()%2` (643-645); otherwise `seesound` (648-649). E1 values: zombieman and shotgun guy posit1-3 (info.c:1139, 1165), imp bgsit1-2 (1399), demon and spectre `sfx_sgtsit` (1425, 1451), baron `sfx_brssit` (1503) | `actor` | `if (actor->info->seesound)` (631); not `MT_SPIDER`/`MT_CYBORG` (653-654) | the target was found (`goto seeyou` 617-621, or `P_LookForPlayers` 626) in the spawn states' `A_Look` (e.g. info.c:310-311) | MT | yes |
| p_enemy.c:657 | `A_Look` | as above (`sfx_spisit`, `sfx_cybsit`, info.c:1607, 1659) | `NULL` ("full volume") | `actor->type==MT_SPIDER \|\| actor->type == MT_CYBORG` (653-654) | same | MT | no (no Spider Mastermind or Cyberdemon on E1 maps) |
| p_enemy.c:729 | `A_Chase` | `actor->info->attacksound`. E1: demon and spectre `sfx_sgtatk` (info.c:1427, 1453); the imp and baron have 0 (1401, 1505); zombieman's `sfx_pistol` (1141) is never used here because it has no `meleestate` (1145) | `actor` | `actor->info->meleestate && P_CheckMeleeRange(actor)` (725-726) and `if (actor->info->attacksound)` (728); not reached on a tic that returned earlier (target lost 703-711, `MF_JUSTATTACKED` 714-720) | run states' `A_Chase` (e.g. info.c:613-620), also called at once by `P_SetMobjState` (p_mobj.c:76-77) when `A_Look` (663) or `P_DamageMobj` (p_inter.c:912-914) sets the see state | MT; dmg | yes |
| p_enemy.c:774 | `A_Chase` | `actor->info->activesound`. E1: `sfx_posact` (zombieman, shotgun guy, info.c:1155, 1181), `sfx_bgact` (imp, 1415), `sfx_dmact` (demon, spectre, baron, 1441, 1467, 1519) | `actor` (after its `P_Move` this tic, 764-768) | `actor->info->activesound && P_Random () < 3` (771-772): the draw is made only when `activesound` is non-zero (all E1 monsters), on every `A_Chase` that reaches the end | as above | MT; dmg | yes |
| p_enemy.c:815 | `A_PosAttack` | `sfx_pistol` | `actor` | `actor->target` (807-808) | `S_POSS_ATK2` (info.c:321) | MT | yes |
| p_enemy.c:832 | `A_SPosAttack` | `sfx_shotgn` | `actor` | `actor->target` (829-830) | `S_SPOS_ATK2` (info.c:354); also `S_SPID_ATK2/3` (752-753) | MT | yes (shotgun guy) |
| p_enemy.c:855 | `A_CPosAttack` | `sfx_shotgn` | `actor` | `actor->target` | `S_CPOS_ATK2/3` (info.c:553-554) | MT | no (Doom II; `P_LoadThings` skips type 65 unless commercial, p_setup.c:318-337) |
| p_enemy.c:923 | `A_TroopAttack` | `sfx_claw` | `actor` | `actor->target` and `P_CheckMeleeRange` (921) | `S_TROO_ATK3` (info.c:590) | MT | yes |
| p_enemy.c:988 | `A_BruisAttack` | `sfx_claw` | `actor` | `actor->target` and `P_CheckMeleeRange` (986) | `S_BOSS_ATK3` (info.c:675), `S_BOS2_ATK3` (704) | MT | yes (baron, E1M8) |
| p_enemy.c:1098, 1113 | `A_SkelWhoosh`, `A_SkelFist` | `sfx_skeswg`; `sfx_skepch` (melee range) | `actor` | target; melee | `S_SKEL_FIST2/4` (info.c:472, 474) | MT | no (Doom II) |
| p_enemy.c:1210, 1235, 1247, 1253, 1323 | `A_VileChase`, `A_VileStart`, `A_StartFire`, `A_FireCrackle`, `A_VileAttack` | `sfx_slop` (origin `corpsehit`), `sfx_vilatk`, `sfx_flamst`, `sfx_flame`, `sfx_barexp` | `corpsehit` / `actor` | various | Arch-vile and its fire | MT | no (Doom II) |
| p_enemy.c:1354 | `A_FatRaise` | `sfx_manatk` | `actor` | none | `S_FATT_ATK1` (info.c:512) | MT | no (Doom II) |
| p_enemy.c:1431 | `A_SkullAttack` | `actor->info->attacksound` (`sfx_sklatk`, info.c:1583) | `actor`, which then gets momentum (1433-1436) | `actor->target` (1424-1425); **no zero guard**, safe only because `MT_SKULL` is the only user (info.c:726) | `S_SKULL_ATK2` | MT | no (no Lost Soul on E1 maps at any skill, 1.2.8) |
| p_enemy.c:1568 | `A_Scream` | `sound`: `deathsound` 0 returns (1541-1542); `sfx_podth1/2/3` → `sfx_podth1 + P_Random ()%3` (1544-1547); `sfx_bgdth1/2` → `sfx_bgdth1 + P_Random ()%2` (1550-1552); otherwise `deathsound`. E1 values: zombieman, shotgun guy podth1-3 (info.c:1149, 1175), imp bgdth1-2 (1409), demon, spectre `sfx_sgtdth` (1435, 1461), baron `sfx_brsdth` (1513), barrel `sfx_barexp` (1903) | `actor` | not spider/cyborg (1561-1562) | death states: `S_POSS_DIE2` (info.c:326), `S_SPOS_DIE2` (359), `S_TROO_DIE2` (594), `S_SARG_DIE2` (627), `S_BOSS_DIE2` (679), `S_BEXP2` (945) | MT (the dying thing's own thinker: every E1 first death state has no action, so the scream comes `deathstate.tics - (P_Random()&3)` tics after `P_KillMobj`, minimum 1, p_inter.c:722-729) | yes |
| p_enemy.c:1565 | `A_Scream` | `sfx_spidth`, `sfx_cybdth` (info.c:1617, 1669) | `NULL` ("full volume") | `MT_SPIDER` or `MT_CYBORG` | `S_SPID_DIE1` (info.c:757, the *first* death state, so it runs inside `P_KillMobj`), `S_CYBER_DIE2` (828) | dmg (spider), MT (cyborg) | no |
| p_enemy.c:1574 | `A_XScream` | `sfx_slop` | `actor` | none | gib states `S_PLAY_XDIE2` (info.c:302), `S_POSS_XDIE2` (331), `S_SPOS_XDIE2` (364), `S_TROO_XDIE2` (599); chosen when `target->health < -target->info->spawnhealth && target->info->xdeathstate` (p_inter.c:719-723) | MT; PM (player) | yes |
| p_enemy.c:1580 | `A_Pain` | `actor->info->painsound`. E1: player `sfx_plpain` (info.c:1118), zombieman, shotgun guy, imp `sfx_popain` (1144, 1170, 1404), demon, spectre, baron `sfx_dmpain` (1430, 1456, 1508) | `actor` | `if (actor->info->painsound)` (1579) | second pain state (`S_PLAY_PAIN2` info.c:293, `S_POSS_PAIN2` 324, `S_SPOS_PAIN2` 357, `S_TROO_PAIN2` 592, `S_SARG_PAIN2` 625, `S_BOSS_PAIN2` 677); the first pain state, set by `P_DamageMobj` when `P_Random () < painchance` (p_inter.c:894-899), has no action | MT; PM (player) | yes |
| p_enemy.c:1761, 1767 | `A_Hoof`, `A_Metal` | `sfx_hoof`, `sfx_metal` | `mo` | none | Cyberdemon (info.c:812, 818) and Spider (`A_Metal`, info.c:739, 743, 747) walk states | MT | no |
| p_enemy.c:1773 | `A_BabyMetal` | `sfx_bspwlk` | `mo` | none | Arachnotron | MT | no (Doom II) |
| p_enemy.c:1839, 1845, 1870, 1922 | `A_BrainAwake`, `A_BrainPain`, `A_BrainScream`, `A_BrainSpit` | `sfx_bossit`, `sfx_bospn`, `sfx_bosdth`, `sfx_bospit` | `NULL` | none (`A_BrainSpit` may return early on easy, 1909-1910) | Boss brain | MT | no (Doom II) |
| p_enemy.c:1932, 1951 | `A_SpawnSound`, `A_SpawnFly` | `sfx_boscub`; `sfx_telept` | `mo` (the cube); `fog` (an `MT_SPAWNFIRE` just spawned, 1950) | `A_SpawnFly` only when `--mo->reactiontime` reaches 0 (1945-1946) | Boss cube | MS | no (Doom II) |

##### Missiles, fogs and removal (p_mobj.c, p_telept.c)

| Site | Function | Sound | Origin | Guard | Reached from | Phase | E1 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| p_mobj.c:903 | `P_SpawnMissile` | `th->info->seesound`. E1: `MT_TROOPSHOT` and `MT_BRUISERSHOT` `sfx_firsht` (info.c:1919, 1529) | `th`, just spawned at the shooter's `x`,`y`, `z+32` (p_mobj.c:898-900) | `if (th->info->seesound)` (902) | `A_TroopAttack` (p_enemy.c:930), `A_BruisAttack` (997); not E1: `A_HeadAttack`, `A_CyberAttack`, `A_BspiAttack`, `A_FatAttack*`, `A_BrainSpit` | MT | yes |
| p_mobj.c:104 | `P_ExplodeMissile` | `mo->info->deathsound`. E1: `sfx_firxpl` (imp and baron balls, info.c:1929, 1539), `sfx_barexp` (rocket, 1981) | `mo` (momentum zeroed first, 92) | `if (mo->info->deathsound)` (103); after `P_SetMobjState(deathstate)` (94, whose action can be `A_Explode`, info.c:263) and the draw `mo->tics -= P_Random()&3` (96) | blocked move in the missile's `P_XYMovement` (p_mobj.c:186), floor or ceiling in its `P_ZMovement` (313-317, 342-346), or at once in `P_CheckMissileSpawn` (881) | MS; or the shooter's phase (PT, MT) when it explodes on spawn | yes |
| p_mobj.c:566 | `P_RemoveMobj` | `S_StopSound (mobj)` | `mobj` | none: every removal | death states ending in `S_NULL` (p_mobj.c:63-67): rocket `S_EXPLODE1-3` (8+6+4 tics, the first reduced by `P_Random()&3`), imp ball `S_TBALLX1-3` (6+6+6), baron ball `S_BRBALLX1-3` (6+6+6), barrel `S_BEXP`..`S_BEXP5` (5+5+5+10+10), teleport fog `S_TFOG`..`S_TFOG10` (twelve states of 6 tics) (info.c:235-242, 263-277, 658-662, 944-948); a missile hitting a sky ceiling (p_mobj.c:176-184); a picked-up item (p_inter.c:657); the old body in `P_NightmareRespawn` (p_mobj.c:408) | any | yes |
| p_mobj.c:379 | `P_NightmareRespawn` | `sfx_telept` | `mo`, an `MT_TFOG` spawned at the corpse (374-377) | `P_CheckPosition` at the spawn point (370-371) | `P_MobjThinker` when `respawnmonsters`, `movecount >= 12*35`, `!(leveltime&31)` and `P_Random () <= 4` (p_mobj.c:452-470) | MT | E1 only with nightmare skill or `-respawn` |
| p_mobj.c:386 | `P_NightmareRespawn` | `sfx_telept` | `mo`, an `MT_TFOG` at the spawn point (384) | same | same | MT | same |
| p_telept.c:112 | `EV_Teleport` | `sfx_telept` | `fog`, an `MT_TFOG` at the thing's **old** `x`,`y`,`z` (111) | not a missile (65-66), front side (70-71), a teleport destination in a tagged sector, `P_TeleportMove` succeeded (103-104) | `P_CrossSpecialLine` (p_spec.c:904-906, line 97; 39 not on E1) from `P_TryMove` (p_map.c:511); monsters may trigger 39/97 (p_spec.c:520-528) | PM; MT | yes (line 97 on E1M5, E1M8, E1M9) |
| p_telept.c:118 | `EV_Teleport` | `sfx_telept` | `fog`, an `MT_TFOG` 20 units in front of the destination along the destination's angle (114-115) | same | same | PM; MT | yes |

##### Sectors: doors, plats, floors, ceilings (all origins are `(mobj_t *)&<sector>->soundorg`)

In the table `door`, `plat`, `floor` and `ceiling` name the thinker; the origin column gives the sector
expression exactly.

| Site | Function | Sound | Origin | Guard | Reached from | Phase | E1 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| p_doors.c:77 | `T_VerticalDoor` (waiting, 0) | `sfx_bdcls` | `&door->sector->soundorg` | `!--door->topcountdown`, type `blazeRaise` (70-75) | door thinker | SE | no (117 and blazing raise lines absent) |
| p_doors.c:83 | same | `sfx_dorcls` | same | countdown done, type `normal` (80-82) | door thinker (doors from lines 1, 26-28, 63, 90) | SE | yes |
| p_doors.c:89 | same | `sfx_doropn` | same | countdown done, type `close30ThenOpen` (86-88) | door thinker (lines 16, 76) | SE | yes |
| p_doors.c:108 | `T_VerticalDoor` (initial wait, 2) | `sfx_doropn` | same | countdown done, type `raiseIn5Mins` (99-106) | sector special 14 (`P_SpawnDoorRaiseIn5Mins`, p_doors.c:528-549) | SE | no (sector 14 absent) |
| p_doors.c:132 | `T_VerticalDoor` (down, -1) | `sfx_bdcls` | same | `res == pastdest`, `blazeRaise`/`blazeClose`, after `P_RemoveThinker` (127-131) | door thinker | SE | no |
| p_doors.c:161 | same | `sfx_doropn` | same | `res == crushed` (blocked; `T_MovePlane` restores and returns `crushed`, p_floor.c:153-161) and type not `blazeClose`/`close` (152-160) | door thinker | SE | yes |
| p_doors.c:228, 240, 253 | `EV_DoLockedDoor` | `sfx_oof` | `NULL` | `thing->player` (216-217) and no matching card or skull | lines 99, 133-137 | PT | no (absent) |
| p_doors.c:301 | `EV_DoDoor` | `sfx_bdcls` | `&door->sector->soundorg` | type `blazeClose` | lines 105/107/... (blazing close) | PT, PM | no |
| p_doors.c:309 | `EV_DoDoor` | `sfx_dorcls` | same | type `close` | lines 3, 42, 50, 75 | PT, PM | no (absent) |
| p_doors.c:316 | `EV_DoDoor` | `sfx_dorcls` | same | type `close30ThenOpen` | lines 16, 76 (p_spec.c:597-601, 807-810) | PM | yes |
| p_doors.c:327 | `EV_DoDoor` | `sfx_bdopn` | same | types `blazeRaise`/`blazeOpen` and `door->topheight != sec->ceilingheight` (326) | line 109 (p_spec.c:730-732) | PM | yes (109) |
| p_doors.c:337 | `EV_DoDoor` | `sfx_doropn` | same | types `normal`/`open` and `door->topheight != sec->ceilingheight` (336) | lines 2, 86, 90 (walk), 63, 103 (switch), 46 (shot, p_spec.c:988-992) | PM; PT; MT (a monster's hitscan on line 46, p_spec.c:966-977, p_map.c:919-920) | yes |
| p_doors.c:379, 393, 406 | `EV_VerticalDoor` | `sfx_oof` | `NULL` | lines 26/32, 27/34, 28/33, `player` non-null (373-374, 386-387, 400-401) and the key missing | `P_UseSpecialLine` (p_switch.c:337) | PT | yes |
| p_doors.c:444 | `EV_VerticalDoor` | `sfx_bdopn` | `(mobj_t *)&sec->soundorg` where `sec = sides[ line->sidenum[side^1]] .sector` (413) | specials 117, 118; not taken by the reuse branch (416-436) | `P_UseSpecialLine` | PT | no |
| p_doors.c:449 | `EV_VerticalDoor` | `sfx_doropn` | same | specials 1, 31; not taken by the reuse branch | player use; a monster's `P_Move` on a blocked special-1 line (p_enemy.c:321; monsters are limited to 1, 32-34 by p_switch.c:301-318, and 32-34 return before any sound at p_doors.c:373-374, 386-387, 400-401) | PT; MT | yes |
| p_doors.c:453 | `EV_VerticalDoor` | `sfx_doropn` | same | the `default` case: 26-28, 32-34 with the key | player use | PT | yes |
| p_plats.c:68 | `T_PlatRaise` (up) | `sfx_stnmov` | `&plat->sector->soundorg` | type `raiseAndChange`/`raiseToNearestAndChange` and `!(leveltime&7)` (64-67), every such tic including the arrival tic | plat thinker | SE | yes (20, 22) |
| p_plats.c:77 | same | `sfx_pstart` | same | `res == crushed && (!plat->crush)` (73), plat turns down | plat thinker | SE | yes |
| p_plats.c:86 | same | `sfx_pstop` | same | `res == pastdest` (82), before `P_RemoveActivePlat` for the four removable types (90-101) | plat thinker | SE | yes |
| p_plats.c:115 | `T_PlatRaise` (down) | `sfx_pstop` | same | `res == pastdest` (111) | plat thinker | SE | yes |
| p_plats.c:126 | `T_PlatRaise` (waiting) | `sfx_pstart` | same | `!--plat->count` (121) | plat thinker | SE | yes |
| p_plats.c:194 | `EV_DoPlat` | `sfx_stnmov` | `(mobj_t *)&sec->soundorg` | type `raiseToNearestAndChange`, sector without `specialdata` (168-169) | lines 20, 22 (p_switch.c:383-387, p_spec.c:615-619) | PT; PM | yes |
| p_plats.c:204 | `EV_DoPlat` | `sfx_stnmov` | same | `raiseAndChange` | lines 14, 15, 66, 67 | PT; PM | no |
| p_plats.c:217 | `EV_DoPlat` | `sfx_pstart` | same | `downWaitUpStay` | lines 62 (switch), 88 (walk; monsters too, p_spec.c:525-526) | PT; PM; MT | yes |
| p_plats.c:230 | `EV_DoPlat` | `sfx_pstart` | same | `blazeDWUS` | lines 120-123 | PT; PM | no |
| p_plats.c:248 | `EV_DoPlat` | `sfx_pstart` | same | `perpetualRaise`, after `plat->status = P_Random()&1` (246) | lines 53, 87 | PM | no |
| p_floor.c:219 | `T_MoveFloor` | `sfx_stnmov` | `&floor->sector->soundorg` | `!(leveltime&7)` (218) after `T_MovePlane`, whatever its result, including the arrival tic | floor thinkers from `EV_DoFloor`, `EV_BuildStairs`, `EV_DoDonut`, and E1M8's `A_BossDeath` (p_enemy.c:1731-1734) | SE | yes |
| p_floor.c:250 | `T_MoveFloor` | `sfx_pstop` | same | `res == pastdest` (221), after `P_RemoveThinker` (248) | floor thinker | SE | yes |
| p_ceilng.c:75, 91 | `T_MoveCeiling` (up) | `sfx_stnmov` every `!(leveltime&7)` (68) unless `silentCrushAndRaise`; `sfx_pstop` at the top for `silentCrushAndRaise` (89-91) | `&ceiling->sector->soundorg` | as stated | ceiling thinker | SE | no (no ceiling special on E1) |
| p_ceilng.c:118, 128 | `T_MoveCeiling` (down) | `sfx_stnmov` every `!(leveltime&7)` (112) unless silent; `sfx_pstop` at the bottom for silent (126-128) | same | as stated | ceiling thinker | SE | no |

##### Switches (p_switch.c, p_spec.c)

| Site | Function | Sound | Origin | Guard | Reached from | Phase | E1 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| p_switch.c:228, 240, 252 | `P_ChangeSwitchTexture` | `sound`: `sfx_swtchn` (218); `sfx_swtchx` only `if (line->special == 11)` (221-222), **never true** (1.2.6.4) | `buttonlist->soundorg` (= `buttonlist[0].soundorg`: `NULL`, or a sector's `degenmobj_t`; see 1.2.6.3) | side 0's top (226), middle (238) or bottom (250) texture is in `switchlist` (225); otherwise no sound | switch specials in `P_UseSpecialLine` (p_switch.c:347-647), mostly only when the `EV_` call succeeded; the gun-triggered lines (p_spec.c:983-997) | PT; MT (monster hitscan on 46) | yes |
| p_spec.c:1151 | `P_UpdateSpecials` | `sfx_swtchn` | `(mobj_t *)&buttonlist[i].soundorg` (the field's address) | a button's `btimer` reaches 0 (1129-1132), 35 tics (`BUTTONTIME`, p_spec.h:247) after `P_StartButton` | reusable switches (`useAgain` 1: 62, 63, 70 and gun line 46 on E1) | US | yes |

#### 1.2.4 The moment within the tic

**(S) Order of one game tic in single player.** `D_DoomLoop` runs one or more tics (`TryRunTics`,
d_net.c:740-746, or the single-tic path d_main.c:374-384), each `M_Ticker ()` then `G_Ticker ()`,
then once per frame `S_UpdateSounds (players[consoleplayer].mo)` (d_main.c:392) and the display.
`G_Ticker` (g_game.c:605):

1. reborns: `G_DoReborn` for any player in `PST_REBORN` (612-614);
2. the `gameaction` loop (617-651): `ga_loadlevel` → `G_DoLoadLevel`, `ga_completed` →
   `G_DoCompleted`, `ga_victory` → `F_StartFinale`, ...;
3. copy each player's ticcmd (658-668);
4. special buttons, pause: `paused ^= 1` with `S_PauseSound`/`S_ResumeSound` (698-724);
5. `GS_LEVEL`: `P_Ticker ()`, `ST_Ticker ()`, `AM_Ticker ()`, `HU_Ticker ()` (727-733).

`P_Ticker` (p_tick.c:130-158) returns at once when `paused` (135-136) or when, in single player, a
menu is open after the first tic (139-145); otherwise:

1. `P_PlayerThink (&players[i])` for each player in game (148-150): **phase PT**;
2. `P_RunThinkers ()` (152): every thinker in list order (p_tick.c:96-111): **phases PM, MT, MS, SE**;
3. `P_UpdateSpecials ()` (153): level timer, animations, buttons: **phase US**;
4. `P_RespawnSpecials ()` (154): deathmatch 2 only;
5. `leveltime++` (157).

So during a tic's sounds `leveltime` holds the number of the tic being run (0 on the first tic:
`leveltime = 0` at p_setup.c:646), and the `!(leveltime&7)` mover sounds fire on tics 0, 8, 16, ...
of the level.

`P_PlayerThink` (p_user.c:236) in order: chainsaw lunge sets the ticcmd (249-255); if `PST_DEAD`,
`P_DeathThink` and return (258-262); otherwise `reactiontime--` or `P_MovePlayer` (267-270);
`P_CalcHeight` (272); `P_PlayerInSpecialSector` (274-275); weapon change (283-317); use →
`P_UseLines` (322-331); `P_MovePsprites` (334), which runs the weapon state actions (p_pspr.c:851-875,
action call at p_pspr.c:87-92). `P_DeathThink` runs `P_MovePsprites` first (p_user.c:187), then
turns toward the attacker (200-221).

**(S) Thinker order.** `P_AddThinker` appends at the tail (p_tick.c:59-65). `P_RunThinkers` walks from
`thinkercap.next` to the cap following `next` (p_tick.c:96-111), so a thinker appended during the
walk (a missile, a fog, a new door) is **run later in the same walk**, and a removed thinker is
marked (`function.acv = -1`, p_tick.c:73-77) and unlinked and freed when the walk reaches it.
`P_SetupLevel` resets the list (`P_InitThinkers`, p_setup.c:622), spawns the map's THINGS in lump
order (`P_LoadThings` p_setup.c:664 → loop p_setup.c:313-347 → `P_SpawnMapThing` → `P_SpawnMobj` →
`P_AddThinker` p_mobj.c:531), with player 1's mobj spawned at the position of its start record
(`P_SpawnPlayer`, p_mobj.c:729-734, 665), and only then `P_SpawnSpecials` (p_setup.c:682) adds the
sector thinkers of the sector specials (lights; doors of specials 10, 14). Therefore the list is:

1. mobjs from THINGS records **before** player 1's start record;
2. **the listener's mobj**;
3. mobjs from the records after it;
4. load-time sector thinkers;
5. everything created during play, in creation order (missiles, puffs, blood, fogs, drops, and all
   door, plat, floor and ceiling thinkers from `EV_*`), all after the listener.

In single player the listener's mobj is never replaced during a level: a reborn reloads the level
(`G_DoReborn`, g_game.c:927-931).

**(S) When the listener's `x`,`y` change.** Only inside the listener's own `P_MobjThinker`:
`P_XYMovement` (p_mobj.c:418-427) through `P_TryMove`, which commits `thing->x = x; thing->y = y`
(p_map.c:488-497) and then fires crossed specials (p_map.c:502-512), and `P_SlideMove`; and inside
`EV_Teleport`'s `P_TeleportMove` (p_map.c:167-174), which is itself reached from that `P_TryMove`.
`P_MovePlayer` (phase PT) changes only momentum (`P_Thrust`, p_user.c:59-68, 160-164). `P_DamageMobj`'s
thrust also changes only momentum. `P_ZMovement` and `P_ThingHeightClip` change only `z`. **(I)**
No other writer of the player mobj's `x`,`y` exists in the level code (searched: every `->x =` in
p_*.c is a spawn, a `P_TryMove`/`P_TeleportMove` commit, `P_CheckMissileSpawn` on a missile, or
`A_VileAttack` on its fire).

**(S) When the listener's angle changes.** `P_MovePlayer`: `player->mo->angle += (cmd->angleturn<<16)`
(p_user.c:154), phase PT, skipped while `reactiontime` counts down after a teleport (267-268);
`P_DeathThink` (p_user.c:213, 219, 221), PT; `A_Punch` (p_pspr.c:490-493, after its sound at 489) and
`A_Saw` (p_pspr.c:526-540, after its sound at 520 or 523), PT; `EV_Teleport`: `thing->angle =
m->angle` (p_telept.c:124), **after** both fog sounds; spawn (p_mobj.c:671). The search of
`angle =`, `angle +=`, `angle -=` in p_*.c finds no other writer for the player's mobj (the
`actor->angle` writers in p_enemy.c act on monsters).

**What the listener looks like at each call (S for the order, I for the conclusion):**

| Call class (phase) | Listener `x`,`y` at the call | Listener angle at the call | Origin position at the call vs end of tic |
| --- | --- | --- | --- |
| Weapon actions, use (switches, doors, oof, noway), `P_BringUpWeapon` from `A_Lower`, player missile spawn, player hitscan effects (line 46, the see-state `A_Chase` of a monster it hurts) (PT) | start of tic (= end of the previous tic) | this tic's turn already applied (p_user.c:154 runs before 326 and 334); `A_Punch`/`A_Saw` sound before their own turn; dead player: psprites before the death turn | weapon sounds: origin is the listener (position irrelevant, s_sound.c:304). Player missile: at the player's `x`,`y` at the call (equal `x`,`y` → centre separation, s_sound.c:312-316); `P_CheckMissileSpawn` then moves it by half its momentum (p_mobj.c:876-878) and its own thinker, appended at the tail, moves it again later in the same `P_RunThinkers` walk. Sector origins fixed |
| Pickups (PM, inside the listener's `P_TryMove`) | the position *before* the step being tried: `P_CheckPosition` (p_map.c:463) runs before the commit (494) | end-of-tic angle (no turn happens in PM except teleport) | NULL origin |
| Walk-over specials, teleport (PM, after the commit) | after the step that crossed the line; if the move was split in halves (`xmove > MAXMOVE/2`, p_mobj.c:162-174, `MAXMOVE` 30 units, p_local.h:54) a later half-step can move it further in the same tic. For a teleport: already at the **destination** (`P_TeleportMove` p_telept.c:103 before the fog sounds 111-118), and a remaining half-step continues from there with the old local `xmove` | for a teleport: the **old** angle (set at p_telept.c:124 after the sounds) | sector origins fixed; fogs stationary (`MT_TFOG` never moves) |
| Landing oof, player pain, death, gib (PM, after `P_XYMovement`) | end of tic | end of tic | origin is the listener |
| A thinker before the listener in the list (MT, MS: only mobjs from earlier THINGS records) | **start of tic** | this tic's turn applied | see "moving origins" below |
| A thinker after the listener (MT, MS, all monsters from later records and everything spawned in play) | **end of tic** | end of tic (including a teleport's new angle) | same |
| Sector thinkers (SE): always after the listener | end of tic | end of tic | fixed |
| `P_UpdateSpecials` button release (US) | end of tic | end of tic | the bogus field address, 1.2.6.3 |
| Level load (LD, `sawup`) | spawn position | spawn angle (`ANG45 * (mthing->angle/45)`, p_mobj.c:671) | origin is the listener |

**Moving origins (S, I).**
- A monster's own thinker moves it first (`P_XYMovement`/`P_ZMovement` before the state tick,
  p_mobj.c:418-449), so at its actions it is already at its post-momentum position. Within the
  actions: `A_Chase`'s melee `attacksound` (p_enemy.c:729) returns before `P_Move` (no step that
  tic); its `activesound` (774) comes after `P_Move` (764-768), at the post-step position; the
  attacks (`A_PosAttack`, `A_SPosAttack`, `A_TroopAttack`, `A_BruisAttack`) do not move the actor.
  `A_Look`'s sound (660) comes **before** `P_SetMobjState (actor, actor->info->seestate)` (663), whose
  `A_Chase` runs at once (p_mobj.c:76-77) and may step the monster in the same tic, so the origin can
  be one step further at the end of the tic than at the call.
- A see-state `A_Chase` triggered by damage (p_inter.c:912-914) runs in the damager's phase (PT for a
  player's shot, MT for a monster, MS for a missile, SE for a crusher), not the victim's: its melee or
  active sound then uses the listener position of that phase.
- A missile's spawn sound (p_mobj.c:903, 976) is at the shooter's `x`,`y`; by the end of the tic the
  missile has moved 1.5 times its momentum if it was spawned in PT or by a thinker earlier than its
  own (it is appended at the tail and runs later in the same walk), or 0.5 times if `P_CheckMissileSpawn`
  exploded it (then its death sound replaces the launch sound on the same origin, s_sound.c:349).
- An origin spawned and removed in one tic: a missile whose first `P_XYMovement` hits a sky ceiling
  is removed with no explosion (p_mobj.c:176-184); `P_RemoveMobj` stops its launch sound
  (p_mobj.c:566) in the same tic that started it. No E1 path removes a fog or a monster in the tic it
  makes a sound.
- Teleport fogs, barrels, corpses and sector origins do not move.

#### 1.2.5 Level start and end

**(S)**
1. `S_Start` is called only by `P_SetupLevel` (p_setup.c:607), from `G_DoLoadLevel` (g_game.c:483),
   itself from the `ga_loadlevel` case of `G_Ticker` (g_game.c:621-622), from `G_DoWorldDone`
   (g_game.c:1176) and from `G_InitNew`. `P_SetupLevel` first sets `players[consoleplayer].viewz = 1`
   (p_setup.c:604), then calls `S_Start` "before Z_FreeTags" (p_setup.c:606-607), which stops every
   channel that holds a sound (s_sound.c:209-211) and changes the music (s_sound.c:213-245).
2. Sounds started **at load**, after `S_Start`, before any tic: only `P_BringUpWeapon`'s `sfx_sawup`
   (p_pspr.c:145-146), via `P_SpawnPlayer` → `P_SetupPsprites` (p_mobj.c:686, p_pspr.c:840-841), when
   the carried-over ready weapon is the chainsaw (`P_SetupPsprites` sets `pendingweapon =
   readyweapon`, p_pspr.c:840). `p->mo = mobj` (p_mobj.c:675) is set before (686), so the origin is
   the listener. After a death the reborn player has the pistol (`G_PlayerReborn`, called from
   `P_SpawnPlayer` p_mobj.c:658-659), so no saw sound then. **(I)** So entering a level with the saw
   raised plays `sawup` at load. `P_SpawnSpecials`'s door spawns (p_doors.c:505-549) start no sound.
   `G_CheckSpot`'s fog sound is suppressed while `viewz == 1` ("don't start sound on first frame",
   g_game.c:885-886) and is netgame-only anyway.
3. **Tic 0** of the level runs in the same `G_Ticker` call as the load: after the `gameaction` loop
   (g_game.c:617-651) the same call reaches `P_Ticker` (730). Any (L) site can fire in tic 0 (a
   monster's `A_Look`, whose spawn-state tics were randomised by `1 + (P_Random () % mobj->tics)`,
   p_mobj.c:787-788; the mover sounds' `!(leveltime&7)` holds at tic 0).
4. **Exit.** `G_ExitLevel` only sets `secretexit = false; gameaction = ga_completed;`
   (g_game.c:1002-1006). The exit switch calls `P_ChangeSwitchTexture(line,0)` and then
   `G_ExitLevel ()` (p_switch.c:359-363), in phase PT: the rest of that tic (thinkers,
   `P_UpdateSpecials`, `leveltime++`) still runs and can start sounds. The next `G_Ticker` runs
   `G_DoCompleted` (g_game.c:636-637), which sets `GS_INTERMISSION` and calls `WI_Start`
   (g_game.c:1132-1139) and contains no sound call (the only `S_` calls in g_game.c are 709, 711, 886,
   1375); in that same `G_Ticker` the `GS_INTERMISSION` case calls `WI_Ticker`, not `P_Ticker`
   (g_game.c:735-737). **(I)** So the switch sound and any sound of the exit tic keep playing into the
   intermission until `S_Start` of the next level; `S_UpdateSounds` keeps being called with the old
   level's listener mobj (d_main.c:392), still allocated (`PU_LEVEL`) until that level's
   `Z_FreeTags` (p_setup.c:618), which follows `S_Start`. E1M8 goes to `ga_victory` instead
   (g_game.c:1031-1036). The level timer's `G_ExitLevel` (p_spec.c:1091-1096) starts no sound.
5. **Death.** `P_KillMobj` sets `PST_DEAD` and lowers the weapon (p_inter.c:700-707) but starts no
   sound; the death sound is `A_PlayerScream` 7 to 10 tics later (p_inter.c:726-729, info.c:294-295)
   or `A_XScream` for a gib. `P_DeathThink` sets `PST_REBORN` when use is pressed (p_user.c:227-228);
   the next `G_Ticker`'s reborn loop calls `G_DoReborn`, which in single player sets `gameaction =
   ga_loadlevel` (g_game.c:928-932); the `gameaction` loop of the **same** `G_Ticker` reloads the
   level (`S_Start` stops all channels) and then runs tic 0 of the reloaded level.
6. **Pause.** `S_PauseSound`/`S_ResumeSound` act on music only (s_sound.c:497-513); while paused
   `P_Ticker` returns at once (p_tick.c:135-136) so no gameplay sound starts, but the frame loop's
   `S_UpdateSounds` still runs.

#### 1.2.6 Skipped, redirected and special sounds

##### 1.2.6.1 NULL origins and the listener as origin

**(S)** A NULL origin means "no position": full volume, centre (s_sound.c:304, 321-324), always
past the audibility test. The source uses it deliberately for pickups (p_inter.c:659-660 adds
`player == &players[consoleplayer]`, so only the console player's pickups sound), locked doors, the
Spider Mastermind and Cyberdemon sight and death ("full volume", p_enemy.c:653-657, 1561-1565), and
the boss brain. The listener as origin (weapons, oof, noway, pain, death) gets the same treatment by
the `origin != players[consoleplayer].mo` test (s_sound.c:304). All boss *other* sounds (pain,
`A_Hoof`, `A_Metal`, attacks) are positional.

##### 1.2.6.2 The sector sound origin and its arithmetic

**(S)** Computed once in `P_GroupLines` (p_setup.c:499), called by `P_SetupLevel` (p_setup.c:660)
before the THINGS. For each sector: `M_ClearBox (bbox)` sets `bbox[BOXTOP] = bbox[BOXRIGHT] = MININT`
and `bbox[BOXBOTTOM] = bbox[BOXLEFT] = MAXINT` (m_bbox.c:39-43; `MININT` `0x80000000`, `MAXINT`
`0x7fffffff`, doomtype.h:48, 54); for every line, in `lines` order, whose front or back sector is
this sector, `M_AddToBox` its `v1` then its `v2` (p_setup.c:542-551). Then:

```c
sector->soundorg.x = (bbox[BOXRIGHT]+bbox[BOXLEFT])/2;   /* p_setup.c:556 */
sector->soundorg.y = (bbox[BOXTOP]+bbox[BOXBOTTOM])/2;   /* p_setup.c:557 */
```

in `fixed_t` (vertex coordinates are fixed-point). `M_AddToBox` uses **`else if`**: `if
(x<box[BOXLEFT]) box[BOXLEFT] = x; else if (x>box[BOXRIGHT]) box[BOXRIGHT] = x;` and the same for
`y` (m_bbox.c:51-58). **(I)** The first vertex therefore sets only `BOXLEFT` and `BOXBOTTOM`;
`BOXRIGHT` is set by a later vertex that is not a new minimum. For a closed sector every vertex is
visited at least twice, so the box is the true box; a sector with an unclosed outline (for instance
a single line whose `v2` has a smaller `x` than `v1`) keeps `BOXRIGHT == MININT`. The division is a
signed `/2`, which truncates toward zero on this compiler (C89 leaves the rounding of a negative
quotient implementation-defined), not a shift. `z` is never set (stays 0 from the zone allocation;
**(I)** unused by the call sites). `soundorg.thinker` is never initialised or linked ("not used for
anything", r_defs.h:89).

##### 1.2.6.3 The switch sound's origin, and the button release origin

**(S)** `buttonlist` is an array, `button_t buttonlist[MAXBUTTONS]` (p_switch.c:101, `MAXBUTTONS` 16,
p_spec.h:244), with `button_t` = `{ line_t* line; bwhere_e where; int btexture; int btimer; mobj_t*
soundorg; }` (p_spec.h:228-235). So `buttonlist->soundorg` is exactly `buttonlist[0].soundorg`: the
**value** of slot 0's pointer field.

1. **The press sound** (p_switch.c:228, 240, 252) passes `buttonlist->soundorg`, i.e. slot 0's
   pointer, whatever line slot 0 belongs to. It is called **before** `P_StartButton` (231-232,
   243-244, 255-256), so the line being pressed never contributes its own origin. Slot 0 is cleared
   at every level start (`memset` of all 16 slots in `P_SpawnSpecials`, p_spec.c:1357-1358) and when
   its button expires (p_spec.c:1152); `P_StartButton` fills the first free slot (p_switch.c:176-186)
   with `buttonlist[i].soundorg = (mobj_t *)&line->frontsector->soundorg` (184). **(I)** Hence: while
   slot 0 is free (no reusable switch pressed within the last 35 tics, or only switches that landed
   in later slots), the press sound has a **NULL origin: full volume, centre, at any distance**.
   While slot 0 holds a timing button, the press sound comes from **that** button's line's front
   sector, which may be a different sector from the switch pressed.
2. **The release sound** (p_spec.c:1151) passes `(mobj_t *)&buttonlist[i].soundorg`: the **address of
   the field**, not its value. Reading `x`,`y` through it reads whatever lies at `mobj_t`'s `x`,`y`
   offsets past that address, which is not slot `i`'s data. **(O)** A probe compiled against the
   genuine headers (`~/scratch/m5c/c-callsites/layout.c`, `gcc -std=gnu89 [-m32] -DNORMALUNIX
   -DLINUX -I.../linuxdoom-1.10`, gcc 15.2.0) printed:

   | ABI | `sizeof(button_t)` | offset of `soundorg` | `mobj_t` `x`, `y` offsets | `degenmobj_t` `x`, `y` offsets |
   | --- | --- | --- | --- | --- |
   | x86-64 (LP64) | 32 | 24 | 24, 28 | 24, 28 |
   | i386 (`-m32`) | 20 | 16 | 12, 16 | 12, 16 |

   **(I)** So the release origin's `x` is LP64: `buttonlist[i+1].btimer`, `y` the 4 padding bytes
   after it; `-m32`: `x` is `buttonlist[i+1].btexture`, `y` is `buttonlist[i+1].btimer`. Both are tiny
   raw integers (a timer 0..35, a texture number) read as `fixed_t`, so the release click is heard
   **as if from the map point (0,0)** (exactly (0,0) when slot `i+1` is free, padding being zero in
   practice), not from the switch, and the exact value differs between the two ABIs when slot `i+1`
   is busy. For `i = 15` the read leaves the array. On E1 maps the number of reusable switch lines
   per map is at most 10 (M4a census, docs/M4A_REFERENCE_INVESTIGATION.md §3.0.4) and a line holds
   at most one slot (p_switch.c:164-171), so `i+1 <= 10` and the read stays inside `buttonlist`.
   The release origin pointer is distinct from every mobj and sector origin, so it shares an identity
   (s_sound.c:349) only with earlier releases of the same slot.

##### 1.2.6.4 The exit switch sound

**(S)** `P_ChangeSwitchTexture` first does `if (!useAgain) line->special = 0;` (p_switch.c:211-212)
and only then tests `if (line->special == 11) sound = sfx_swtchx;` (221-222). The exit switch calls
it with `useAgain` 0 (p_switch.c:361), so the special is already 0 and the sound is `sfx_swtchn`.
**(I)** `sfx_swtchx` is never played by a level switch in this source (its only other users are
menu lines, m_menu.c:686, 723, 1507, 1683). The exit sound's origin is slot 0 (normally NULL); the
level then ends at the next `G_Ticker` (1.2.5).

##### 1.2.6.5 Random sound choices and their draws

**(S)** Only `P_Random` (gameplay RNG) picks sounds in the call sites; no call site draws
`M_Random`:
- `A_Look`: `P_Random()%3` for posit, `P_Random()%2` for bgsit (p_enemy.c:640, 645), drawn **before**
  `S_StartSound` and whether or not the sound turns out audible (the audibility test is inside
  `S_StartSoundAtVolume`, s_sound.c:318-319, and draws nothing); for every other seesound no draw.
- `A_Scream`: `P_Random ()%3` for podth, `P_Random ()%2` for bgdth (p_enemy.c:1547, 1552), same remark;
  no draw for the demon, spectre, baron or barrel.
- `A_Chase`: `P_Random () < 3` gates the active sound (p_enemy.c:771-772); the draw happens on every
  `A_Chase` that reaches the end when `activesound` is non-zero, regardless of audibility.
- `A_Pain`: no draw (the pain *chance* draw is in `P_DamageMobj`, p_inter.c:894).
- `EV_DoPlat` `perpetualRaise` draws `P_Random()&1` before its sound (p_plats.c:246; not E1).
- `P_ExplodeMissile` draws `P_Random()&3` before the death sound (p_mobj.c:96).
The pitch variation draws `M_Random` inside `S_StartSoundAtVolume` (s_sound.c:326-343), owned by the
`s_sound.c` section.

##### 1.2.6.6 Chainsaw, oof and noway

**(S)** Chainsaw: `sawup` when the saw is brought up (p_pspr.c:145-146), `sawidl` each entry into
`S_SAW` while ready (p_pspr.c:296-300; `S_SAW`/`S_SAWB` alternate every 4 tics, info.c:203-204, so
once per 8 tics), `sawful`/`sawhit` on each attack frame (p_pspr.c:518-523). All have the listener
as origin. "Oof" has two uses: a hard landing, origin the listener (p_mobj.c:299-307), and a locked
door without the key, origin NULL (p_doors.c:379, 393, 406 on E1). "Noway" (p_map.c:1104) needs a
non-special line with `openrange <= 0` first on the use path, once per use press.

##### 1.2.6.7 Doors, plats, floors, ceilings

**(S)** Start sounds are issued when the thinker is created (`EV_DoDoor` p_doors.c:294-345, after
`P_AddThinker` 285; `EV_VerticalDoor` before creating it, p_doors.c:439-458; `EV_DoPlat`
p_plats.c:186-249). `EV_DoDoor`'s open sounds need `door->topheight != sec->ceilingheight`
(p_doors.c:326, 336); `EV_VerticalDoor`'s do not. Re-using a raise door (specials 1, 26-28, 117) whose
thinker is alive only flips its direction and returns with **no sound** (p_doors.c:416-436): a door
going down reverses silently, and a player closing an open door makes it go down immediately with
no `dorcls`. Floors have no start sound (`EV_DoFloor`, `EV_BuildStairs`, `EV_DoDonut` start none);
`T_MoveFloor` plays `sfx_stnmov` whenever `!(leveltime&7)` (p_floor.c:218-220), whatever the result
of the move (also while blocked and on the arrival tic), then `sfx_pstop` on arrival (p_floor.c:250).
Plats play the 8-tic `stnmov` only while going **up** and only for `raiseAndChange` /
`raiseToNearestAndChange` (p_plats.c:64-69); `downWaitUpStay` moves silently between `pstart` and
`pstop`. Ceilings play it both ways except `silentCrushAndRaise` (p_ceilng.c:68-78, 112-120). Doors
have no moving sound. Crushing makes no sound of its own (`PIT_ChangeSector` has none; the victims'
pain and death come through `A_Pain`/`A_Scream`). On the arrival tic the `stnmov` and the `pstop`
are started in that order on the same origin.

##### 1.2.6.8 Teleport fog, explosions, removal

**(S)** Each teleport starts two `sfx_telept` sounds on two fresh `MT_TFOG` mobjs (p_telept.c:111-118);
a fog lives twelve 6-tic states (info.c:266-277) and `P_RemoveMobj` then stops its sound if still
playing (p_mobj.c:566). Explosions: a rocket's or fireball's `deathsound` in `P_ExplodeMissile`
(p_mobj.c:102-104) and the barrel's `A_Scream` (`sfx_barexp`, info.c:945, 1903); the barrel is
removed 30 tics after its scream (info.c:946-948), the rocket 16 to 18 tics after exploding, the
imp and baron balls 15 to 18 tics after (info.c:235-242, 263-265, 658-662, with p_mobj.c:96-99),
each removal stopping the explosion sound if still playing. `P_RemoveMobj` stops at most one
channel (the first match, s_sound.c:476-480).

#### 1.2.7 Episode 1 reachability, and the sound set of every Episode 1 thing

**(O)** Which things exist on Episode 1 at each skill: `python3 -I tools/iwad_counts.py --things E1M1`
… `E1M9` run from a private export of the tools (`git archive 7f7fd99 tools linuxdoom-1.10` into
`~/scratch/m5c/c-callsites/src`) over the read-only IWAD directory; output saved to
`~/scratch/m5c/c-callsites/things_e1.txt`, sha256
`e6b4b92a513754e30d90455d04b39a757cbd45251fc7ca6920c76d6266c05796`. Counting records that
`P_SpawnMapThing` spawns in single player at easy, medium or hard, the monster and barrel types
present are exactly `MT_POSSESSED`, `MT_SHOTGUY`, `MT_TROOP`, `MT_SERGEANT`, `MT_SHADOWS` (none on
E1M1, E1M2, E1M4 at any skill), `MT_BRUISER` (E1M8 only) and `MT_BARREL`. **No** `MT_SKULL`,
`MT_HEAD`, `MT_CYBORG` or `MT_SPIDER` at any skill. Weapons on the maps: shotgun, chaingun, rocket
launcher, chainsaw; no plasma rifle, BFG or cells. Power-ups: soul sphere, invisibility, radiation
suit, computer map, light amplification; no invulnerability, and berserk only as a multiplayer
record. Line specials present: the M4a census list (docs/M4A_REFERENCE_INVESTIGATION.md §3.0.1:
wire 22's 1, 11, 36, 38, 48, 63, 88, 109 and the 30 listed there); no ceiling, crusher, locked
`EV_DoLockedDoor`, blazing-raise, close-door or perpetual-plat special; sector specials 10 and 14
absent (§3.0.2). These are counts and type facts only.

**(I) E1 classification of the 96 (L) lines.** 54 are E1-reachable: p_mobj.c 104, 307, 379, 386
(nightmare or `-respawn` only), 566, 903, 976; p_plats.c 68, 77, 86, 115, 126, 194, 217; p_doors.c 83,
89, 161, 316, 327, 337, 379, 393, 406, 449, 453; p_floor.c 219, 250; p_enemy.c 660, 729, 774, 815,
832, 923, 988, 1568, 1574, 1580, 2007; p_spec.c 1151; p_map.c 1104; p_inter.c 660; p_telept.c 112,
118; p_switch.c 228, 240, 252; p_pspr.c 146, 299, 489, 520, 523, 651, 675, 737. The other 42 need
Doom II things or weapons (p_enemy.c 855, 1098, 1113, 1210, 1235, 1247, 1253, 1323, 1354, 1773, 1782,
1790, 1803, 1839, 1845, 1870, 1922, 1932, 1951; p_pspr.c 705), Doom 1 monsters absent from E1
(p_enemy.c 657, 1431, 1565, 1761, 1767), the BFG (p_pspr.c 822), or specials absent from E1 (p_plats.c
204, 230, 248; p_ceilng.c 75, 91, 118, 128; p_doors.c 77, 108, 132, 228, 240, 253, 301, 309, 444).
Cheats (`idkfa` giving the BFG in registered) are out of the (L) definition.

**Sound set per Episode 1 thing (S, from info.c and the action bodies).** "Origin" is the thing
unless stated.

| Thing | Sound | Via | Notes |
| --- | --- | --- | --- |
| Player (`MT_PLAYER`) | `sfx_plpain` | `A_Pain`, `S_PLAY_PAIN2` (info.c:293, 1118) | 4 tics after the pain state (info.c:292); `painchance` 255 (1117) |
| | `sfx_pldeth` | `A_PlayerScream`, `S_PLAY_DIE2` (info.c:295; p_enemy.c:1997) | `sfx_pdiehi` is commercial-only |
| | `sfx_slop` | `A_XScream`, `S_PLAY_XDIE2` (info.c:302) | gib |
| | `sfx_oof` | `P_ZMovement` hard landing (p_mobj.c:307); locked door (NULL origin) | |
| | `sfx_noway` | `PTR_UseTraverse` (p_map.c:1104) | |
| | `sfx_itemup`, `sfx_getpow`, `sfx_wpnup` | `P_TouchSpecialThing` (p_inter.c:660), NULL origin | console player only |
| Fist | `sfx_punch` | `A_Punch` (p_pspr.c:489) | only on a hit |
| Chainsaw | `sfx_sawup`, `sfx_sawidl`, `sfx_sawful`, `sfx_sawhit` | `P_BringUpWeapon`, `A_WeaponReady`, `A_Saw` | |
| Pistol | `sfx_pistol` | `A_FirePistol` (info.c:150) | |
| Shotgun | `sfx_shotgn` | `A_FireShotgun` (info.c:158) | |
| Chaingun | `sfx_pistol` ×2 per cycle | `A_FireCGun` (info.c:188-189) | second even with no ammo |
| Rocket launcher | `sfx_rlaunc`; `sfx_barexp` | `MT_ROCKET` seesound (p_mobj.c:976) and deathsound (p_mobj.c:104) | origin the rocket |
| Zombieman (`MT_POSSESSED`) | `sfx_posit1/2/3` | `A_Look` (`P_Random()%3`) | |
| | `sfx_posact` | `A_Chase` (`P_Random()<3`) | |
| | `sfx_pistol` | `A_PosAttack` (info.c:321) | |
| | `sfx_popain` | `A_Pain` (info.c:324) | |
| | `sfx_podth1/2/3` | `A_Scream` (`P_Random()%3`, info.c:326) | |
| | `sfx_slop` | `A_XScream` (info.c:331) | |
| Shotgun guy (`MT_SHOTGUY`) | `sfx_posit1/2/3`, `sfx_posact`, `sfx_popain`, `sfx_podth1/2/3`, `sfx_slop` | as the zombieman (info.c:343-364) | |
| | `sfx_shotgn` | `A_SPosAttack` (info.c:354) | |
| Imp (`MT_TROOP`) | `sfx_bgsit1/2` | `A_Look` (`P_Random()%2`) | |
| | `sfx_bgact` | `A_Chase` | |
| | `sfx_claw` | `A_TroopAttack` in melee range (info.c:590) | else a fireball: |
| | `sfx_firsht`; `sfx_firxpl` | `MT_TROOPSHOT` seesound (`P_SpawnMissile`) and deathsound (`P_ExplodeMissile`) | origin the fireball |
| | `sfx_popain` | `A_Pain` (info.c:592) | |
| | `sfx_bgdth1/2` | `A_Scream` (`P_Random()%2`, info.c:594) | |
| | `sfx_slop` | `A_XScream` (info.c:599) | |
| Demon (`MT_SERGEANT`), Spectre (`MT_SHADOWS`) | `sfx_sgtsit` | `A_Look` | no draw |
| | `sfx_dmact` | `A_Chase` | |
| | `sfx_sgtatk` | `A_Chase` melee branch, before `S_SARG_ATK1` (p_enemy.c:728-729) | `A_SargAttack` itself is silent (p_enemy.c:935-947) |
| | `sfx_dmpain` | `A_Pain` (info.c:625) | |
| | `sfx_sgtdth` | `A_Scream` (info.c:627) | no gib state (info.c:1434, 1460) |
| Baron (`MT_BRUISER`, E1M8) | `sfx_brssit`, `sfx_dmact`, `sfx_dmpain`, `sfx_brsdth` | `A_Look`, `A_Chase`, `A_Pain` (info.c:677), `A_Scream` (679) | |
| | `sfx_claw` | `A_BruisAttack` in melee range (info.c:675) | else: |
| | `sfx_firsht`; `sfx_firxpl` | `MT_BRUISERSHOT` seesound and deathsound (info.c:1529, 1539) | |
| | `sfx_stnmov`, `sfx_pstop` | `A_BossDeath` (info.c:684) → `EV_DoFloor(lowerFloorToLowest)` tag 666 (p_enemy.c:1731-1734) → `T_MoveFloor` | sector origin, when the last baron dies with a player alive |
| Barrel (`MT_BARREL`) | `sfx_barexp` | `A_Scream`, `S_BEXP2` (info.c:945) | removed 30 tics later |
| Teleporter (line 97) | `sfx_telept` ×2 | `EV_Teleport` (p_telept.c:112, 118) | fog origins; monsters too |
| Doors, plats, floors, switches | see 1.2.3 | | sector or switch origins |

#### 1.2.8 Undefined, implementation-defined or layout-dependent behaviour in the call sites

1. **The `degenmobj_t` cast.** `(mobj_t *)&sec->soundorg` (all sector rows, p_switch.c:184) accesses a
   `degenmobj_t` through a `mobj_t*`. The two share a leading `thinker_t, x, y, z` (r_defs.h:88-95,
   p_mobj.h:207-215) and **(O)** the probe shows equal `x`,`y` offsets on LP64 and `-m32`. Reading
   another struct type through `mobj_t*` is outside ISO C's aliasing rules (no union), so this is
   undefined in principle and works in practice because only `x`, `y` are read and pointers are
   compared.
2. **The button release origin** (p_spec.c:1151) is a field address read as a `mobj_t`: the values
   read come from the next slot (or past the array for slot 15), differ between LP64 and `-m32`, and
   one of the LP64 values is padding (unspecified by C, zero in practice). 1.2.6.3.
3. **`EV_VerticalDoor` on a one-sided line** reads `sides[line->sidenum[side^1]]` with `sidenum[1]
   == -1` (p_doors.c:413), out of bounds; the sound then uses that garbage sector. **(I)** Not on E1:
   the M4a mover census records 0 one-sided manual doors (docs/M4A_REFERENCE_INVESTIGATION.md
   §3.0.4).
4. **`sfx_None` (0)** passed to `S_StartSound` reaches `I_Error("Bad sfx #: %d")` (s_sound.c:277-278).
   Every `info`-sound site guards it: p_mobj.c:103, 902, 975; p_enemy.c:631, 728, 771, 1541-1542, 1579.
   The unguarded `A_SkullAttack` (p_enemy.c:1431) is safe only because `MT_SKULL`'s `attacksound` is
   non-zero. The upper test `sfx_id > NUMSFX` admits `NUMSFX` itself (one past `S_sfx`); no call site
   passes it.
5. **Sector origin arithmetic.** `(bbox[BOXRIGHT]+bbox[BOXLEFT])/2` (p_setup.c:556-557) overflows
   `int` (undefined) if the sum leaves 32 bits (coordinates whose sum exceeds ±32,768 units, or a
   `BOXRIGHT`/`BOXTOP` left at `MININT` by the `else if` of `M_AddToBox` with a negative partner);
   the division of a negative odd sum truncates toward zero with gcc (implementation-defined in
   C89).
6. **A mobj removed later in the same tic.** `P_RemoveMobj` stops the first channel with that origin
   (p_mobj.c:566, s_sound.c:476-480) before the memory is freed (lazily, p_tick.c:73-77, 101-106), so
   no channel keeps a freed mobj while at most one channel carries each origin. Sector and button
   origins point into level memory that stays valid until the next level's `Z_FreeTags`, which
   follows `S_Start` (p_setup.c:606-619).
7. **The listener after the exit tic** is the old level's player mobj, valid until the next
   `P_SetupLevel` (1.2.5).

#### 1.2.9 Questions the call-site facts raise for the owner

Each question is taken up in [§5](#5-owner-decisions-t1t16-with-recommendations): 1 in T12 (the press origin); 2 in T12 (the release origin; the 64-bit reading recommended here is also T12's recommendation, while §3.4 recommends refusing); 3 in T14; 4 in T1 (the nightmare respawn fog sounds are out of scope there).

1. **The switch press origin (`buttonlist->soundorg`).** Either (a) reproduce it exactly: NULL (full
   volume, centre, any distance) while slot 0 is free, otherwise the front sector of slot 0's line;
   or (b) refuse the tic or record a gap. **Recommendation: (a).** It is well-defined C (a pointer
   value of the current level or NULL), the state is already in the button slots, and players hear
   it on every switch.
2. **The switch release origin (`&buttonlist[i].soundorg`).** Either (a) declare a fixed ABI for the
   release position (the `-m32` reading: `x = buttonlist[i+1].btexture`, `y =
   buttonlist[i+1].btimer`, or the LP64 reading: `x = buttonlist[i+1].btimer`, `y = 0`) and match the
   genuine build of that ABI; or (b) model the origin as the map point (0,0) and refuse the sound's
   start (a fidelity-gap witness) whenever slot `i+1` is busy so the two ABIs could differ; or (c)
   model it at the switch. **Recommendation: (a) with the LP64 reading**, since the default reference
   build is LP64 and `-m32` is the opt-in strict variant, plus a recorded note that `-m32` differs
   only while slot `i+1` is busy; (c) would be a silent divergence. Slot 15 cannot be reached on E1
   (at most 10 reusable switch lines); any map that would reach it should refuse.
3. **Menu-key sounds during a level** (`-`, `=`, F5, F8 and the keys that open a menu, m_menu.c:1520-1614)
   start NULL-origin sounds and so draw `M_Random`, which the face glance also uses. Either (a) keep
   them out of scope and state that the face-glance match holds only for sessions without those keys;
   or (b) model their starts (sound number and the `M_Random` draw, no menu) in the playable build.
   **Recommendation: (a)** for M5c, matching the stated scope, with the restriction recorded where
   S15 is closed; revisit when menus are ported.
4. **Nightmare respawn fog sounds** (p_mobj.c:379, 386) are E1-reachable only with skill 5 or
   `-respawn`. Either (a) include them wherever the build admits those settings, or (b) refuse
   explicitly where it does not. **Recommendation:** follow whatever M5b admits for skill and
   `-respawn` (not checked here: the Rust side is inventoried in §2).

#### 1.2.10 What this section did not do or observe

- This section did not run the genuine engine or any C scenario, so no call site's timing is *observed*; all
  timing facts are read from the source. No sound was played or traced.
- This section did not read or describe the Rust code.
- This section did not re-document `s_sound.c` internals (attenuation, channel choice, priority, pitch); it cites
  only the lines that decide what a passed origin means (s_sound.c:277-278, 304-324, 349, 471-482).
- The layout probe (`~/scratch/m5c/c-callsites/layout.c`, binaries `layout`, `layout-m32`) printed
  offsets only; this section did not run a genuine build to observe the release sound's actual position, nor
  check the padding bytes' contents at run time.
- The E1 census facts come from one local run of `tools/iwad_counts.py --things` per slot (output
  digest above) and the committed M4a census for line and sector specials; this section did not re-run
  `--specials-markdown` or `--movers-markdown`.
- This section did not determine which non-E1 Doom 1 maps use the specials marked "not E1"; "no" in the E1
  column means only "absent from Episode 1".
- Nothing was written inside the repository. `/mnt/c` had 28 GB free at 02:41 PST; the scratch
  directory is about 6 MB.

### 1.3 The mixer: i_sound.c

Written 2026-11-18 (America/Los_Angeles).
All citations are into `linuxdoom-1.10/` at a77dfb96 (repo head 7f7fd99). Every line cited
was read for this section. Labels: **[src]** read in the source, **[obs]** observed by running
something (command and output given in 1.3.10), **[inf]** inference.

Terms used below. A *frame* is one stereo output instant (one left and one right 16-bit
sample). A *buffer* is the 512 frames that one call of `I_UpdateSound` mixes. A *mixer
channel* is one of the 8 slots inside i_sound.c; it is not the same thing as an *s_sound
channel* (`channels[]` of s_sound.c, `numChannels` of them, default 3), which only does
bookkeeping. "Integrated" means the in-process mixer of i_sound.c; "sndserver" means the
separate process in `sndserv/`.

#### 1.3.0 Which code is compiled: the build configuration (read this first)

1. **The shipped build defines `SNDSERV`.** The Makefile build line is
   `CFLAGS=-g -Wall -DNORMALUNIX -DLINUX # -DUSEASM` (Makefile:8), `CC= gcc`
   (Makefile:6), `LIBS=-lXext -lX11 -lnsl -lm` (Makefile:10). It defines neither
   `SNDSERV` nor `SNDINTR` nor `USE_TIMER` on the command line. But doomdef.h:84 says
   `#define SNDSERV  1` and doomdef.h:85 is the comment `//#define SNDINTR  1` **[src]**.
   i_sound.c includes `i_sound.h` (i_sound.c:54), which includes `doomdef.h` at
   i_sound.h:26, before any `#ifdef SNDSERV` (first at i_sound.h:29, i_sound.c:62); i_sound.c
   includes doomdef.h again at i_sound.c:59. d_main.c includes doomdef.h at d_main.c:44
   (its test is at d_main.c:397); m_misc.c at m_misc.c:39 (tests at m_misc.c:211, 256).
   So in **every** translation unit that tests it, `SNDSERV` is defined and `SNDINTR` is not
   **[src]**. `USE_TIMER` does not occur anywhere in linuxdoom-1.10 (grep over the
   directory: only `SNDSERV`, `SNDINTR` and the misspelt `SNDSRV` occur) **[src]**.
2. The ChangeLog confirms this was a deliberate late switch: "i_sound.c: enabled SNDSERV, as
   SNDINTR for some reason just gives ghastly results e.g. on E4M2 ... SNDSERV is now default,
   until the internal sound driver is a bit more reliable" (ChangeLog:15-19, dated
   Dec 22 1997 at ChangeLog:12), replacing the earlier "Default is internal, synchronous"
   (ChangeLog:314-318). README.sound:75-83 says the integrated support produces "crappy"
   output with the double shotgun and plasma gun near a wall, and the same with SNDINTR
   **[src]**.
3. **Consequence for M5c [src, inf].** In the shipped linuxdoom-1.10 build, the mixer that
   produces audio is the one in `sndserv/` (driven by `fprintf(sndserver, "p%2.2x%2.2x%2.2x%2.2x\n",
   id, pitch, vol, sep)` at i_sound.c:485), not `addsfx`/`I_UpdateSound` of i_sound.c, which
   are compiled but never called (`I_UpdateSound` is called only under `#ifndef SNDSERV`,
   d_main.c:397-400; `addsfx` only from the non-SNDSERV branch of `I_StartSound`,
   i_sound.c:490-500). The task asks for the i_sound.c mixer; this section describes it
   under the **integrated synchronous configuration** (doomdef.h:84 removed, SNDINTR still
   undefined), and states for each place what the shipped (SNDSERV) build does instead.
   Because doomdef.h `#define`s the symbol, the integrated configuration cannot be obtained
   with a `-U` flag; doomdef.h:84 itself must be edited **[inf]**. This is a decision for the
   owner (question Q1).
4. Table of every conditional in the files in scope, and the branch compiled in three
   configurations: **S** = shipped (SNDSERV defined, SNDINTR not), **I** = integrated
   synchronous (neither), **T** = integrated timer (SNDINTR only). `LINUX` and `NORMALUNIX`
   come from Makefile:8; gcc on Linux does not predefine `sun`, `SGI`, `SUN`; `__GNUG__` is
   defined only by g++ and Makefile:6 uses gcc **[src, inf]**.

   | Location | Test | S | I | T |
   | --- | --- | --- | --- | --- |
   | i_sound.c:36-38 | `#ifndef LINUX` (include sys/filio.h) | skipped | skipped | skipped |
   | i_sound.c:62-65 | `#ifdef SNDSERV`: `FILE* sndserver=0; char* sndserver_filename = "./sndserver ";` | compiled | – | – |
   | i_sound.c:66-75 | `#elif SNDINTR`: `SOUND_INTERVAL 500`, timer prototypes | – | – (`#elif` of an undefined name is 0) | compiled |
   | i_sound.c:76-78 | `#else // None?` (empty) | – | compiled | – |
   | i_sound.c:482-489 / 490-500 | `I_StartSound`: pipe command / `addsfx` | 482-489 | 490-500 | 490-500 |
   | i_sound.c:541-544 | `static int misses` | – | – | compiled |
   | i_sound.c:637-653 | miss counting, `flag++` in `I_UpdateSound` | – | – | compiled |
   | i_sound.c:695-701 / 702-726 | `I_ShutdownSound`: send `"q\n"` / close device | 695-701 | 702-726 | 702-726 |
   | i_sound.c:720-722 | `I_SoundDelTimer()` (inside the 702 branch) | – | – | compiled |
   | i_sound.c:740-757 / 758-824 | `I_InitSound`: popen server / open device and load all sounds | 740-757 | 758-824 | 758-824 |
   | i_sound.c:762-765 | `I_SoundSetTimer(SOUND_INTERVAL)` (inside 758 branch) | – | – | compiled |
   | i_sound.c:901-905 | `#ifdef sun` typedef `tSigSet` | `int` branch (904) | same | same |
   | i_sound.c:956-958 | `#ifndef sun` (contains only a comment) | compiled, empty | same | same |
   | i_sound.h:29-33 | `#ifdef SNDSERV` externs | compiled | – | – |
   | d_main.c:34-41 | `#ifdef NORMALUNIX` system includes | compiled | compiled | compiled |
   | d_main.c:397-400 | `#ifndef SNDSERV` → `I_UpdateSound();` | **not called** | called | called |
   | d_main.c:402-405 | `#ifndef SNDINTR` → `I_SubmitSound();` | called | called | not called |
   | d_main.c:575-615 | `#ifdef NORMALUNIX` (DOOMWADDIR/HOME, not sound) | compiled | compiled | compiled |
   | m_misc.c:211-214, 256-259 | `#ifdef SNDSERV`: externs, defaults `"sndserver"`, `"mb_used"` | compiled | – | – |
   | m_misc.c:242-261 | `#ifdef NORMALUNIX` key defaults | compiled | compiled | compiled |
   | s_sound.c:367-381 | `#ifndef SNDSRV` (misspelt; never defined) "not pre-cached - wtf?" | compiled | compiled | compiled |
   | s_sound.c:402-405, 411-464 | `#ifdef SAWDEBUG` (never defined) | skipped | skipped | skipped |
   | i_system.c:43-45 | `#ifdef __GNUG__` pragma | skipped | skipped | skipped |
   | i_system.c:128-136 | `SGI`/`SUN`/else in `I_WaitVBL` (not sound) | `usleep` branch | same | same |
   | i_main.c | no conditionals | – | – | – |

   The timer functions `I_HandleSoundTimer`, `I_SoundSetTimer`, `I_SoundDelTimer`
   (i_sound.c:917-985) are outside any conditional and are compiled in all three
   configurations, but are called only in T (i_sound.c:764, 721) **[src]**.
5. **Shipped-build side effects of the configuration [src, inf].**
   - Under S, `I_SubmitSound` is still called every loop iteration (d_main.c:402-405) and
     executes `write(audio_fd, mixbuffer, SAMPLECOUNT*BUFMUL)` (i_sound.c:669). `audio_fd` is
     a zero-initialised global (i_sound.c:106) that only the non-SNDSERV branch of
     `I_InitSound` assigns (i_sound.c:770). So the shipped game writes 2048 bytes of zeros
     (`mixbuffer` is a zero-initialised global, i_sound.c:112, never mixed under S) to file
     descriptor 0 (standard input) once per loop iteration, and ignores the result. What that
     does depends on how fd 0 was opened (a terminal opened read-write receives NUL bytes; a
     read-only fd fails with an error) **[inf]**.
   - Under S, `I_StartSound` returns `id`, the sfx number itself (i_sound.c:489), and
     `I_SoundIsPlaying(handle)` is `gametic < handle` (i_sound.c:520). See 1.3.4 item 31 for
     what this does to s_sound.
   - Under S, `S_sfx[].data` is never set in the game process, so s_sound.c:369-380 prints
     "S_StartSoundAtVolume: 16bit and not pre-cached - wtf?" to stderr on every start that
     reaches it **[src]**. Under I and T every sound is pre-cached (1.3.2), so it never prints.

#### 1.3.1 Constants and globals (i_sound.c:84-151)

6. Constants **[src]**:

   | Name | Definition | Value | Meaning |
   | --- | --- | --- | --- |
   | `SAMPLECOUNT` | i_sound.c:93 | 512 | frames mixed per `I_UpdateSound`; also the padding unit of sound data |
   | `NUM_CHANNELS` | i_sound.c:94 | 8 | mixer channels |
   | `BUFMUL` | i_sound.c:96 | 4 | "2 for 16bit, and 2 for two channels" (i_sound.c:95): bytes per frame |
   | `MIXBUFFERSIZE` | i_sound.c:97 | `(SAMPLECOUNT*BUFMUL)` = 2048 | used as an element count of a `short` array (see 7) |
   | `SAMPLERATE` | i_sound.c:99 | 11025 | Hz, given to the device only (i_sound.c:779-781) |
   | `SAMPLESIZE` | i_sound.c:100 | 2 | defined, never used in i_sound.c |
   | `SOUND_INTERVAL` | i_sound.c:71 | 500 | microseconds (T only; comment at :68 says 30 ms) |

7. Globals (all have static storage duration, so all are zero-initialised before `main`; none
   is read before being written except as noted) **[src]**:

   | Name | Declaration | Size (x86-64 gcc, `int` 4 bytes) | Role |
   | --- | --- | --- | --- |
   | `flag` | `static int flag = 0;` (:84) | 4 | T only: mixed-but-unwritten counter |
   | `lengths` | `int lengths[NUMSFX];` (:103) | 109 ints (`NUMSFX` = 109, sounds.h:288, enum from sfx_None=0 at sounds.h:179) | padded sample count per sfx; `lengths[0]` is never written (loop starts at 1, :799) and stays 0 |
   | `audio_fd` | `int audio_fd;` (:106) | 4 | device fd; 0 until :770 |
   | `mixbuffer` | `signed short mixbuffer[MIXBUFFERSIZE];` (:112) | 2048 shorts = 4096 bytes | interleaved output. Only elements 0..1023 are ever mixed (`leftend = mixbuffer + SAMPLECOUNT*step` = +1024, :571) and only the first 2048 **bytes** (= 1024 shorts) are written (:669). Elements 1024..2047 are zeroed at :818-819 and never used. |
   | `channelstep` | `unsigned int channelstep[NUM_CHANNELS];` (:116) | 8 × 4 | 16.16 fixed-point source advance per frame |
   | `channelstepremainder` | `unsigned int [8]` (:118) | 8 × 4 | fractional position, kept in 0..65535 |
   | `channels` | `unsigned char* channels[8]` (:122) | 8 pointers | current read position; NULL = channel free |
   | `channelsend` | `unsigned char* channelsend[8]` (:123) | 8 pointers | one past the last playable sample |
   | `channelstart` | `int channelstart[8]` (:131) | 8 × 4 | `gametic` at start (oldest-channel rule) |
   | `channelhandles` | `int channelhandles[8]` (:137) | 8 × 4 | handle given at start; written, never read |
   | `channelids` | `int channelids[8]` (:141) | 8 × 4 | sfx id playing (single-instance rule) |
   | `steptable` | `int steptable[256]` (:144) | 256 × 4 | pitch → step. The comment "Pitch to stepping lookup, unused" is wrong: it is used by `I_StartSound` (:495) and the step is used by the mixer (:599) |
   | `steptablemid` | local `int* steptablemid = steptable + 128;` in `I_SetChannels` (:404) | – | not a global; lets the fill loop use indices -128..127 |
   | `vol_lookup` | `int vol_lookup[128*256]` (:147) | 32768 ints = 128 KiB | row v (0..127) maps an unsigned 8-bit sample to a scaled signed value |
   | `channelleftvol_lookup`, `channelrightvol_lookup` | `int* [8]` (:150-151) | 8 pointers each | point at row `leftvol`/`rightvol` of `vol_lookup` |
   | `sndserver`, `sndserver_filename` | :64-65 | S only | pipe to the server; `"./sndserver "` (trailing space) |
   | `looping`, `musicdies` | `static int` (:838-839) | – | music dummies (out of scope) |
   | `itimer`, `sig` | `static int` (:912, :914) | – | `ITIMER_REAL`, `SIGALRM` (T only) |

   Inside `addsfx`: `static unsigned short handlenums = 0;` (:271), 2 bytes on x86-64.

#### 1.3.2 Loading sound data: I_GetSfxLumpNum, getsfx, I_InitSound

8. **`I_GetSfxLumpNum(sfxinfo_t* sfx)`** (i_sound.c:451-456) **[src]**: `char namebuf[9];
   sprintf(namebuf, "ds%s", sfx->name); return W_GetNumForName(namebuf);`. The longest name in
   `S_sfx` is 6 characters (checked over sounds.c:114-230), so "ds" + 6 + NUL fits in 9.
   `W_GetNumForName` calls `W_CheckNumForName` and on -1 calls
   `I_Error ("W_GetNumForName: %s not found!", name)` (w_wad.c:399-409). Lookup is
   case-insensitive (`strupr`, w_wad.c:370) and scans from the last lump backward, so later
   (PWAD) lumps win (w_wad.c:376-386). **There is no fallback here.** It is called by
   `S_StartSoundAtVolume` when `sfx->lumpnum < 0` (s_sound.c:364-365), which `S_Init` sets for
   every sfx (s_sound.c:190-191). Hence, in every configuration, the first start of a sound
   whose `ds` lump is missing ends the game with I_Error, after s_sound has already consumed
   the pitch `M_Random` (s_sound.c:327-346) and allocated an s_sound channel
   (s_sound.c:352). The `dspistol` fallback in `getsfx` (next item) therefore never gets to
   matter for a sound that is actually started **[inf from the cited lines]**.
9. **`getsfx(char* sfxname, int* len)`** (i_sound.c:185-251), step by step **[src]**:
   1. `sprintf(name, "ds%s", sfxname);` into `char name[20]` (:195, :201).
   2. Fallback: `if ( W_CheckNumForName(name) == -1 ) sfxlump = W_GetNumForName("dspistol");
      else sfxlump = W_GetNumForName(name);` (:213-216). Condition: the `ds` lump is absent.
      Then the sound gets its own **copy** of the pistol data with the pistol's length. If
      `dspistol` is also absent, `I_Error` (w_wad.c:405-406).
   3. `size = W_LumpLength( sfxlump );` (:218) = `lumpinfo[lump].size` (w_wad.c:416-422).
   4. `sfx = (unsigned char*)W_CacheLumpNum( sfxlump, PU_STATIC );` (:226). This reads the
      lump into a zone block whose user is `&lumpcache[lump]` (w_wad.c:485-491), or changes
      the tag of an already cached copy (w_wad.c:493-497).
   5. `paddedsize = ((size-8 + (SAMPLECOUNT-1)) / SAMPLECOUNT) * SAMPLECOUNT;` (:230), i.e.
      `((size - 8 + 511) / 512) * 512` in `int` arithmetic with C division.
   6. `paddedsfx = (unsigned char*)Z_Malloc( paddedsize+8, PU_STATIC, 0 );` (:233). No realloc:
      the realloc is the commented-out line `// ddt: (unsigned char *) realloc(sfx, paddedsize+8);`
      (:234); the comment at :229 says "The original realloc would interfere with zone
      memory". (The sndserver still uses `realloc`, sndserv/wadread.c:249, the "libc realloc
      mixed with zone memory allocation" of README.sound:66-69.)
   7. `memcpy( paddedsfx, sfx, size );` (:239): the **whole lump including its 8 header
      bytes** is copied. (`memcpy` is declared through `<string.h>` at doomdef.h:27.)
   8. `for (i=size ; i<paddedsize+8 ; i++) paddedsfx[i] = 128;` (:240-241): bytes from
      offset `size` to `paddedsize+7` are padding with value **128**, which is the unsigned
      8-bit zero level (`vol_lookup[v*256+128] = 0` for all v, item 13).
   9. `Z_Free( sfx );` (:244). Z_Free clears the user pointer (`*block->user = 0`,
      z_zone.c:132-139), so `lumpcache[sfxlump]` becomes NULL and a later cache of the same
      lump re-reads it.
   10. `*len = paddedsize;` (:247); `return (void *) (paddedsfx + 8);` (:250).
   So the returned pointer is the first byte after the 8-byte header, and `*len` counts
   bytes from there: all `size-8` lump bytes after the header, followed by
   `paddedsize-(size-8)` bytes of 128.
10. **What getsfx does not read [src].** It never interprets the 8 header bytes: not a format
    tag, not a sample rate, not a sample count. Every byte after the first 8 is treated as an
    unsigned 8-bit sample at the device rate, including any bytes the header might say lie
    beyond the sample count. The layout of those 8 bytes is not described anywhere in
    linuxdoom-1.10 (context, not evidence: other ports describe them as format, rate and
    sample count).
11. **Edge cases of the padding arithmetic** (C `/` on `int`; values observed with the
    exact expression, 1.3.10 run A) **[src, obs]**:

    | Lump size | `size-8+511` | `paddedsize` | Allocation | Padding bytes |
    | --- | --- | --- | --- | --- |
    | 0..7 | 503..510 | 0 | 8 | offsets `size`..7 (inside the header area) |
    | 8 (zero samples) | 511 | 0 | 8 | none |
    | 9 | 512 | 512 | 520 | 511 |
    | 520 = 8+512 (exact multiple) | 1023 | 512 | 520 | none (loop at :240 runs 0 times) |
    | 521 | 1024 | 1024 | 1032 | 511 |

    For sizes below 8 the dividend is still positive, so no negative division occurs;
    `paddedsize` is never negative. A length-0 sound still plays one frame (item 21) and, for
    lumps of 8 bytes or fewer, that frame reads `paddedsfx[8]`, one byte past the 8-byte
    allocation (1.3.8).
12. **`I_InitSound`, integrated branch** (i_sound.c:758-824) **[src]**:
    1. Device setup (platform glue, 1.3.7 list B): :768-793.
    2. Load loop, `for (i=1 ; i<NUMSFX ; i++)` (:799): if `!S_sfx[i].link`,
       `S_sfx[i].data = getsfx( S_sfx[i].name, &lengths[i] );` (:802-806); else
       `S_sfx[i].data = S_sfx[i].link->data;` and
       `lengths[i] = lengths[(S_sfx[i].link - S_sfx)/sizeof(sfxinfo_t)];` (:808-812).
    3. `for ( i = 0; i< MIXBUFFERSIZE; i++ ) mixbuffer[i] = 0;` (:818-819).
    - **The linked-sound length is wrong.** `S_sfx[i].link - S_sfx` is pointer subtraction,
      already an element index. Dividing it by `sizeof(sfxinfo_t)` divides the index again.
      The only link in `S_sfx` is `{ "chgun", false, 64, &S_sfx[sfx_pistol], 150, 0, 0 }`
      (sounds.c:204); `sfx_pistol` = 1 (sounds.h:180). `sizeof(sfxinfo_t)` is 36 on a 32-bit
      build and 48 on x86-64 (struct at sounds.h:32-62; observed, 1.3.10 run A). Either way
      `1 / sizeof(sfxinfo_t)` = 0, so `lengths[sfx_chgun] = lengths[0]` = **0** (never
      written, item 7). The chaingun shares the pistol's data pointer but has length 0, so in
      the integrated mixer every chaingun shot plays exactly **one frame** (the pistol's first
      sample) and stops (observed, 1.3.10 run B: one non-zero frame, channel free after the
      buffer). The sndserver has the identical expression (sndserv/soundsrv.c:390), so the
      shipped build has the same 0 length **[src]**. The index only depends on layout through
      `sizeof`; any `sizeof(sfxinfo_t) > 1` gives 0 **[inf]**.
    - Linked sounds are loaded after their target only because the target has a lower index
      (pistol 1 < chgun 86, sounds.h:180, 265) **[src]**.
    - Init order: `I_Init` → `I_InitSound` (i_system.c:107-110) is called at d_main.c:1101,
      after `M_LoadDefaults` (d_main.c:1015) and `W_InitMultipleFiles` (d_main.c:1021), and
      before `S_Init` (d_main.c:1107), which calls `I_SetChannels` (s_sound.c:170). So the
      tables are built after the sounds are loaded and before any start **[src]**.
13. **Tables, `I_SetChannels`** (i_sound.c:396-424) **[src]**:
    - `for (i=-128 ; i<128 ; i++) steptablemid[i] = (int)(pow(2.0, (i/64.0))*65536.0);`
      (:414-415), with `steptablemid = steptable + 128` (:404). So
      `steptable[p] = (int)(pow(2.0, (p-128)/64.0) * 65536.0)` for p = 0..255: double `pow`,
      multiply in double, conversion to `int` truncates toward zero. `steptable[128]` =
      65536 (normal speed), `steptable[0]` = 16384, `steptable[64]` = 32768,
      `steptable[192]` = 131072, `steptable[255]` = 259320 **[obs]**.
    - `for (i=0 ; i<128 ; i++) for (j=0 ; j<256 ; j++) vol_lookup[i*256+j] = (i*(j-128)*256)/127;`
      (:421-423). Pure `int` arithmetic: the product is in -4,161,536..4,161,536 (no
      overflow); `/ 127` truncates toward zero (C99; implementation-defined in C89, 1.3.8).
      Range -32768 (i=127, j=0) to 32512 (i=127, j=255); row i, column 128 is 0. Example
      row 15: j=0 → -3870, j=255 → 3840 **[obs]**. Note the asymmetry from truncation toward
      zero.
    - **libm dependence [obs].** Only `steptable` uses floating point. For every i that is not
      a multiple of 64, the exact value `2^(i/64)*65536` lies at least 0.0125 from the nearest
      integer (minimum at i = -16), so any `pow` accurate to far better than 1e-7 relative gives
      the same truncated table; the multiples of 64 are exact powers of two. Observed: glibc
      2.43 x86-64 (SSE) and the same source built with `-m32` give the identical table
      (FNV-1a-64 of the 256 little-endian ints `c3b9727b3def2608`); `vol_lookup` FNV-1a-64
      `552057530633d3e9`. A port can therefore generate `steptable` with exact integer or
      high-precision arithmetic and compare with these digests **[inf]**.
    - `I_SetChannels` does not clear `channels[]` (the clearing loop is commented out,
      :407-410); they are zero from static initialisation **[src]**.
    - Under S, `I_SetChannels` still runs in the game process (s_sound.c:170) but nothing
      uses the tables there; the server builds its own copy with the same formulas, except
      that it assigns `pow(...)*65536.0` to `int` implicitly (sndserv/soundsrv.c:554-555,
      564-566) **[src]**.

#### 1.3.3 I_SetChannels, I_SetSfxVolume, I_SetMusicVolume

14. `I_SetChannels()` builds the two tables and nothing else (item 13). It takes no argument
    and does not look at `numChannels` **[src]**.
15. `I_SetSfxVolume(int volume)`: `snd_SfxVolume = volume;` (i_sound.c:427-435). No caller in
    linuxdoom-1.10 (grep over *.c finds none; s_sound.c sets `snd_SfxVolume` itself at
    s_sound.c:637). The mixer never reads `snd_SfxVolume`: volume enters only as the `vol`
    argument of `I_StartSound` **[src]**.
16. `I_SetMusicVolume(int volume)`: `snd_MusicVolume = volume;` (i_sound.c:438-444). Called by
    `S_SetMusicVolume` with 127 and then the real value (s_sound.c:624-625). No audible
    effect (no music) **[src]**.

#### 1.3.4 Starting a sound: I_StartSound and addsfx

17. **`I_StartSound(int id, int vol, int sep, int pitch, int priority)`** (i_sound.c:470-501)
    **[src]**: `priority = 0;` (:480, discarded). Integrated:
    `id = addsfx( id, vol, steptable[pitch], sep ); return id;` (:495-499). So `pitch` is used
    only as an index into `steptable` (**not** `steptablemid`), `vol` and `sep` are passed
    through unchanged, `priority` is ignored, and the return value is `addsfx`'s handle.
    Shipped (S): writes `p` + four 2-digit hex fields + newline to the pipe if `sndserver` is
    non-NULL and returns `id` (:482-489).
    The only caller is `S_StartSoundAtVolume`: `channels[cnum].handle = I_StartSound(sfx_id,
    volume, sep, pitch, priority);` (s_sound.c:389-394), reached only after `S_getChannel`
    returned a channel (s_sound.c:352-355).
18. **Argument ranges, established from s_sound.c [src]:**
    - `id`: 1..`NUMSFX` passes the check `if (sfx_id < 1 || sfx_id > NUMSFX)`
      (s_sound.c:277-278); note `NUMSFX` itself (109) would pass and index past `S_sfx`, but
      no caller is known to pass it (s_sound section's job to confirm).
    - `vol`: not scaled between s_sound and i_sound. `S_StartSound` passes `snd_SfxVolume`
      (s_sound.c:407). `snd_SfxVolume` defaults to 8 (m_misc.c:237; the initialiser 15 at
      s_sound.c:113 is overwritten by `M_LoadDefaults`), the menu keeps it in 0..15
      (m_menu.c:821-826, then `S_SetSfxVolume(snd_SfxVolume /* *8 */)` at m_menu.c:830), and
      `S_Init(snd_SfxVolume /* *8 */, ...)` (d_main.c:1107) → `S_SetSfxVolume` rejects values
      outside 0..127 with I_Error (s_sound.c:634-635), which bounds a config-file value.
      `S_MAX_VOLUME` = 127 (s_sound.c:51) is defined and never used. Adjustments: a link sound
      gets `volume += sfx->volume` (0 for chgun, sounds.c:204), returns if `< 1`, and is
      clamped to `snd_SfxVolume` (s_sound.c:283-294); a positional sound gets
      `S_AdjustSoundParams`, which sets `*vol` to `snd_SfxVolume` when close
      (s_sound.c:796-799), to `snd_SfxVolume * ((S_CLIPPING_DIST - dist)>>FRACBITS) / S_ATTENUATOR`
      (≤ `snd_SfxVolume`) normally (s_sound.c:809-815), and on `gamemap == 8` to
      `15 + ((snd_SfxVolume-15) * ((S_CLIPPING_DIST - dist)>>FRACBITS)) / S_ATTENUATOR`
      (s_sound.c:800-808), which lies between `snd_SfxVolume` and 15 and so can **exceed**
      `snd_SfxVolume` when it is below 15. Positional sounds with `*vol <= 0` are dropped
      (s_sound.c:817, 318-319). Non-positional, non-link sounds pass `snd_SfxVolume` as is,
      including 0. Therefore `vol` ∈ 0..max(`snd_SfxVolume`, 15) ⊆ 0..127: in normal play
      0..15, at the default 0..8 (0..15 on map 8) **[inf from the cited lines]**.
    - `sep`: `NORM_SEP` = 128 (s_sound.c:71) for non-positional sounds and for sources at the
      listener's exact x,y (s_sound.c:312-316, 321-324); otherwise
      `128 - (FixedMul(S_STEREO_SWING, finesine[angle])>>FRACBITS)` with
      `S_STEREO_SWING` = `96*0x10000` (s_sound.c:74, 793), roughly 32..224 (exact extremes depend
      on `finesine`, s_sound section). Always within 0..255 **[src, inf]**.
    - `pitch`: `NORM_PITCH` = 128 (s_sound.c:69), or `sfx->pitch` for a link (150 for chgun,
      sounds.c:204; s_sound.c:285). Then sfx 10..13 (`sfx_sawup`..`sfx_sawhit`, sounds.h:189-192)
      get `pitch += 8 - (M_Random()&15)` (121..136 from 128), and every other sfx except
      `sfx_itemup` (32) and `sfx_tink` (87) gets `pitch += 16 - (M_Random()&31)` (113..144 from
      128; 135..166 for chgun); both are clamped to 0..255 (s_sound.c:327-346).
      `S_AdjustSoundParams` takes `pitch` but never writes it (s_sound.c:753-818). So
      `steptable[pitch]` is always in bounds **[src]**.
    - **Pitch is audible in the integrated mixer.** The mixer advances by `channelstep`
      (i_sound.c:599), so a random pitch of 113..144 plays most sounds at
      `steptable[113]/65536` = 55709/65536 ≈ 0.850 to `steptable[144]/65536` = 77935/65536 ≈
      1.189 times normal speed **[src, obs]**. The comments "Pitching ... is set, but
      currently not used by mixing" (i_sound.c:467-468) and "I fail to see that this is
      currently used" (:413) are wrong for the code as written.
19. **`addsfx(int sfxid, int volume, int step, int seperation)`** (i_sound.c:264-381), in
    order **[src]**:
    1. Locals: `rc = -1`, `oldest = gametic`, `oldestnum = 0` (:274-277).
    2. **Single-instance sounds** (:285-306): if `sfxid` is one of `sfx_sawup` (10),
       `sfx_sawidl` (11), `sfx_sawful` (12), `sfx_sawhit` (13), `sfx_stnmov` (22),
       `sfx_pistol` (1) (numbers from sounds.h:180-201), then
       `for (i=0 ; i<NUM_CHANNELS ; i++) if ( (channels[i]) && (channelids[i] == sfxid) )
       { channels[i] = 0; break; }`. All 8 mixer channels are examined in index order; the
       **first** active channel playing the **same** sfx id is freed (its pointer set to 0;
       nothing else of it is reset), and the loop stops. Only the identical id counts: a
       sawidl does not stop a sawup, and `sfx_chgun` (86) is not in the list even though it
       uses the pistol's data. This frees a slot that the next step will usually pick.
    3. **Channel choice** (:309-325):
       `for (i=0; (i<NUM_CHANNELS) && (channels[i]); i++) { if (channelstart[i] < oldest)
       { oldestnum = i; oldest = channelstart[i]; } }`, then
       `if (i == NUM_CHANNELS) slot = oldestnum; else slot = i;`.
       - The loop stops at the **first free** channel (lowest index with `channels[i] == 0`),
         and that becomes the slot.
       - If all 8 are busy, the slot is the channel with the smallest `channelstart` that is
         **strictly** less than the current `gametic`, the lowest index among equals (strict
         `<`). If no busy channel started before the current `gametic` (all 8 started in this
         tic, or later values), `oldestnum` stays 0 and **channel 0** is overwritten.
       - No out-of-bounds read: `i<NUM_CHANNELS` is tested before `channels[i]` and `&&`
         short-circuits, so `channels[8]` is never read **[src]**. Observed: with starts at
         gametic 100..107 except slot 5 at 50, a ninth start at 200 took slot 5; with all eight
         `channelstart` = 200 and gametic 200, the new sound took slot 0 (1.3.10 run B).
    4. `channels[slot] = (unsigned char *) S_sfx[sfxid].data;` (:330) and
       `channelsend[slot] = channels[slot] + lengths[sfxid];` (:332). Since `lengths` is the
       padded length, a sound plays its padding (128 = silence) up to the next multiple of
       512 bytes.
    5. Handle: `if (!handlenums) handlenums = 100;` then
       `channelhandles[slot] = rc = handlenums++;` (:335-340). `handlenums` is
       `static unsigned short` starting at 0 (:271), so the first handle is 100, then 101, 102,
       ... up to 65535; the increment then wraps the unsigned short to 0 and the next call
       resets it to 100. The comment "limited to 0..100" (:334) is wrong. Observed: after
       65535 the next handle is 100 (1.3.10 run B).
    6. `channelstep[slot] = step; channelstepremainder[slot] = 0; channelstart[slot] = gametic;`
       (:344-348).
    7. Separation (:352-361), exact text:
       ```c
       seperation += 1;
       leftvol = volume - ((volume*seperation*seperation) >> 16);
       seperation = seperation - 257;
       rightvol = volume - ((volume*seperation*seperation) >> 16);
       ```
       With s = sep+1 ∈ 1..256: `leftvol = v - (v*s*s >> 16)`,
       `rightvol = v - (v*(s-257)^2 >> 16)`. All in `int`; for v ∈ 0..127 the products are
       ≤ 127*65536 = 8,323,072 and non-negative, so `>> 16` equals division by 65536. sep 0 is
       full left (L = v, R = 0 for v = 15), sep 255 full right. At sep 128 and v = 15,
       L = R = 12; at v = 8, L = R = 6 **[obs]**. (The sndserver writes `/(256*256)` instead
       of `>> 16`, sndserv/soundsrv.c:485, 491; identical for non-negative operands **[inf]**.)
    8. Range check: `if (rightvol < 0 || rightvol > 127) I_Error("rightvol out of bounds");`
       then the same for `leftvol` (:364-368). Observed over v 0..200 × sep 0..255: no
       failure for v ≤ 127, failures start at v = 128 (1.3.10 run A). Given item 18, neither
       I_Error can fire from s_sound's arguments **[inf]**. Note that the check happens
       **after** the slot's pointer, end, handle, step and start have already been written:
       an out-of-range call would leave a half-initialised channel, but it exits the program
       anyway.
    9. `channelleftvol_lookup[slot] = &vol_lookup[leftvol*256];`
       `channelrightvol_lookup[slot] = &vol_lookup[rightvol*256];` (:372-373), then
       `channelids[slot] = sfxid;` (:377), `return rc;` (:380).
20. **What stops a mixer channel [src].** Only three things: reaching `channelsend` in the
    mix loop (:606-607), the single-instance rule (:300), and being chosen as the slot for a
    new sound (:330). `I_StopSound` does nothing (:505-514) and `I_UpdateSoundParams` does
    nothing (:674-688). So s_sound stopping a sound (`S_StopSound`, `S_StopChannel`, the
    "kill old sound" of s_sound.c:349, the priority eviction of s_sound.c:853-865, the
    distance cutoff of s_sound.c:592-595) **never silences** the mixer, and moving sources
    are never re-panned or re-attenuated after the start. The mixer has 8 channels while
    s_sound tracks `numChannels` (default 3, m_misc.c:282), so up to 8 sounds can be audible
    while s_sound believes at most 3 are playing.
21. **Frames a sound occupies [inf, exact].** The read position after k frames is
    `floor(k*step/65536)` (the remainder is carried exactly, item 23), and the channel is
    freed after the frame in which the advanced position reaches `len`. So a sound of padded
    length `len` with step `s` contributes exactly `max(1, ceil(len*65536/s))` frames, counted
    across buffer boundaries. A length-0 sound (chgun, lumps ≤ 8 bytes) contributes 1 frame.
22. `I_SoundIsPlaying(int handle)`: `return gametic < handle;` (i_sound.c:517-521). It does not
    look at the mixer. Under I/T, handles are 100, 101, ... in start order, so s_sound treats
    a channel as playing while `gametic` is below its handle number; `gametic` is never reset
    (declared at g_game.c:123; its only writes are `gametic++` at d_main.c:384 and
    d_net.c:746).
    Under S the handle is the sfx id (1..108), so an s_sound channel counts as playing only
    while `gametic < sfx_id`, i.e. effectively never after the first 108 tics of the process.
    `S_UpdateSounds` frees s_sound channels whose handle is "not playing" (s_sound.c:558-605),
    and `S_StopChannel` only calls `I_StopSound` when it is (s_sound.c:717-724). **This makes
    s_sound's channel occupancy, and therefore whether later starts find a free s_sound
    channel or are dropped by `S_getChannel` (s_sound.c:850-860), differ between the shipped
    and the integrated configuration** **[inf from the cited lines]**. The pitch `M_Random`
    is drawn before `S_getChannel` (s_sound.c:327-352), so the number of `M_Random` calls
    does not depend on this **[src]**. (s_sound section to confirm and own.)

#### 1.3.5 Mixing: I_UpdateSound and the other entry points

23. **`I_UpdateSound(void)`** (i_sound.c:539-654) **[src]**. Locals:
    `register unsigned int sample; register int dl; register int dr;`,
    `signed short *leftout, *rightout, *leftend; int step; int chan;` (:549-561).
    ```c
    leftout = mixbuffer; rightout = mixbuffer+1; step = 2;          // :565-567
    leftend = mixbuffer + SAMPLECOUNT*step;                          // :571  (+1024)
    while (leftout != leftend) {                                     // :576  512 iterations
        dl = 0; dr = 0;                                              // :579-580
        for (chan = 0; chan < NUM_CHANNELS; chan++) {                // :585  index order 0..7
            if (channels[chan]) {                                    // :588
                sample = *channels[chan];                            // :591  unsigned char 0..255
                dl += channelleftvol_lookup[chan][sample];           // :596
                dr += channelrightvol_lookup[chan][sample];          // :597
                channelstepremainder[chan] += channelstep[chan];     // :599
                channels[chan] += channelstepremainder[chan] >> 16;  // :601
                channelstepremainder[chan] &= 65536-1;               // :603
                if (channels[chan] >= channelsend[chan])             // :606
                    channels[chan] = 0;                              // :607
            }
        }
        if (dl > 0x7fff) *leftout = 0x7fff;                          // :617-622
        else if (dl < -0x8000) *leftout = -0x8000;
        else *leftout = dl;
        /* same for dr into *rightout */                             // :625-630
        leftout += step; rightout += step;                           // :633-634
    }
    ```
    - Each frame first **reads** the sample at the current position, then advances, then
      tests the end. The sample is read before the end test, so a channel always contributes
      at least one frame.
    - Step accumulation: 16.16 fixed point in `unsigned int`; the integer part advances the
      byte pointer, the fraction stays in 0..65535. `channelstepremainder + channelstep` is at
      most 65535 + 259320, far below 2^32, so it cannot wrap **[inf]**.
    - End test `>=` against `channelsend`; on reaching it the channel is freed at once and
      contributes nothing for the rest of the buffer. Positions skipped by a step above 65536
      are never read; the test prevents reading at or beyond `channelsend` after the first
      frame.
    - Accumulation: up to 8 values in -32768..32512 summed in `int`: range -262144..260096,
      no overflow **[inf]**.
    - Clipping: to `0x7fff` = 32767 if `dl > 32767`, to `-0x8000` = -32768 if `dl < -32768`,
      otherwise stored. Conversion of the in-range `int` to `signed short` is exact.
    - Interleaving: `mixbuffer[2k]` = left, `mixbuffer[2k+1]` = right, k = 0..511. All 1024
      used elements are rewritten on every call, so no output state carries between calls;
      the carried state is only the channel arrays.
    - Output sample type: native `signed short` (two's complement 16-bit), host byte order.
      The device is asked for `AFMT_S16_LE` (:788-789), so the bytes are right only on a
      little-endian host (1.3.7, 1.3.8).
    - Under T only, after the loop: if `flag` is non-zero add it to `misses` and clear it;
      if `misses > 10` print "I_SoundUpdate: missed 10 buffer writes" and reset; then
      `flag++` (:637-653). Under I nothing else is compiled into this function.
24. **`I_SubmitSound(void)`**: `write(audio_fd, mixbuffer, SAMPLECOUNT*BUFMUL);`
    (i_sound.c:665-670): 2048 bytes = 1024 shorts = the 512 mixed frames. The return value
    is ignored (no short-write or error handling) **[src]**.
25. `I_UpdateSoundParams(handle, vol, sep, pitch)`: assigns 0 to its parameters, no effect
    (:674-688). `I_StopSound(handle)`: `handle = 0;`, no effect (:505-514) **[src]**.
26. `I_ShutdownSound` (I/T, :702-726): prints "I_ShutdownSound: NOT finishing pending sounds",
    runs `for( i=0 ; i<8 && !channels[i] ; i++);` once and sets `done=1` without waiting (the
    `if (i==8)` is commented out, :717), calls `I_SoundDelTimer()` under T, and
    `close ( audio_fd );`. Under S it sends `"q\n"` to the server (:695-701). Called from
    `I_Quit` (i_system.c:119) **[src]**.
27. **SNDINTR timer path** (T only, not compiled into the calls under S or I): `I_InitSound`
    calls `I_SoundSetTimer(500)` (:762-765), which installs `I_HandleSoundTimer` for
    `SIGALRM` with `SA_RESTART` (:955-961) and an `ITIMER_REAL` interval of 500 µs
    (:963-969). The handler writes the whole buffer if `flag` is non-zero and clears `flag`
    (:923-931). `D_DoomLoop` then skips `I_SubmitSound` (d_main.c:402-405). Under I the
    functions exist but nothing calls them **[src]**.

#### 1.3.6 Cadence: how much audio per tic

28. **The loop** (D_DoomLoop, d_main.c:354-407), per iteration, in order **[src]**:
    1. `I_StartFrame ();` (d_main.c:372), an empty function in i_video.c:183-187.
    2. Tics: if `singletics`, exactly one tic inline (d_main.c:375-386: `I_StartTic`,
       `D_ProcessEvents`, `G_BuildTiccmd`, `M_Ticker`, `G_Ticker`, `gametic++`); otherwise
       `TryRunTics ();` (d_main.c:389), which computes `counts` from wall-clock tics
       (`I_GetTime`, 35 per second from `gettimeofday`, i_system.c:88-100) and network
       availability (d_net.c:647-677, at least 1 at d_net.c:676-677), waits until that many
       tics are available (d_net.c:715-733; it returns without running a tic if it waited 20
       tics, d_net.c:728-732), then runs them back to back, each `M_Ticker (); G_Ticker ();
       gametic++;` (d_net.c:736-746). `singletics` is set by `G_TimeDemo` (g_game.c:1630).
       Gameplay sound starts happen inside `G_Ticker` here and call `addsfx` immediately.
    3. `S_UpdateSounds (players[consoleplayer].mo);` (d_main.c:392): s_sound bookkeeping; its
       calls into i_sound.c (`I_SoundIsPlaying`, `I_UpdateSoundParams`, `I_StopSound`) do not
       change the mixer (items 20, 22).
    4. `D_Display ();` (d_main.c:395). During a screen wipe, D_Display loops inside itself
       until the melt ends (d_main.c:326-344), with **no** mixing or submitting, so under I
       the mixer state is frozen and the device receives nothing for the whole wipe.
    5. `I_UpdateSound();` under `#ifndef SNDSERV` (d_main.c:397-400): mix 512 frames.
    6. `I_SubmitSound();` under `#ifndef SNDINTR` (d_main.c:402-405): blocking write of those
       512 frames.
29. **Amounts [src, inf].** One `I_UpdateSound` produces 512 frames, 512/11025 s ≈ 46.44 ms.
    One tic is 1/35 s, worth 11025/35 = 315 frames. So under I, audio is produced **per loop
    iteration** (exactly 512 frames each), not per tic. The number of tics per iteration
    is decided by `TryRunTics` from the wall clock. What paces the loop is the device:
    `SNDCTL_DSP_SETFRAGMENT` with `11 | (2<<16)` (i_sound.c:775-776) asks for 2 fragments of
    2^11 = 2048 bytes, i.e. one buffer each, and the blocking `write` returns only when there
    is room. When the game keeps up, the loop runs at about 11025/512 ≈ 21.53 iterations per
    second and `TryRunTics` runs on average 35/21.53 ≈ 1.63 tics per iteration (1 or 2,
    occasionally more) **[inf; not observed on a real device]**. With `singletics`
    (`-timedemo`), it is exactly 1 tic and 512 frames per iteration, and the blocking write
    would hold a timedemo to about 21.5 tics per second **[inf]**.
30. **Plain statement.** In linuxdoom-1.10 the relation between game tics and mixed buffers
    is a property of real-time scheduling (the wall clock in `I_GetTime`, the OSS device's
    consumption rate and buffer depth, and how long each iteration takes to draw), not of
    the game state. The same session with the same sound starts produces different audio
    on different machines and runs. A tic-synchronous port must therefore **declare** a
    schedule at each of these places:
    1. **Frames per tic / buffers per tic.** Original: 512 frames per loop iteration and a
       variable 0, 1, 2, ... tics per iteration (singletics: exactly 1). A port must choose,
       e.g. 512 frames per tic (the singletics schedule), 315 frames per tic (real time), or
       a fixed iteration pattern, and say so.
    2. **When a start becomes audible.** Original: every `addsfx` call writes the channel
       at once, and the sound begins at frame 0 of the next `I_UpdateSound`, which follows
       all the tics of that iteration. A start can never begin in the middle of a buffer, and
       all starts of one iteration begin at the same frame. The port must state the frame
       offset at which a start made in tic t begins.
    3. **Starts that interact before any mixing.** When several tics run in one iteration,
       starts from the earlier tic have not advanced at all when the later tic's starts run:
       a single-instance sound started twice in one iteration frees the first copy before
       it played a frame, and the oldest-channel rule may steal a channel filled earlier in
       the same iteration. The port's schedule fixes which starts share an iteration.
    4. **`channelstart` values.** They are `gametic` at the start (i_sound.c:348), so
       starts in different tics of one iteration get different values even though they
       begin together; this decides the oldest-channel choice (item 19.3) and must follow the
       port's tic numbering exactly.
    5. **Pauses in mixing.** Original: no mixing during a wipe (item 28.4), and mixing
       continues every iteration while the game is paused or in a menu (no tics run, the
       loop still mixes and blocks). A port must say whether its buffers stop with the
       tics, continue, or follow the wipe.
    6. **Start-up and latency.** Original: the device queue (up to 2 × 2048 bytes = 1024
       frames ≈ 93 ms) delays what is heard relative to what is mixed, and the actual
       device rate returned by `SNDCTL_DSP_SPEED` is ignored (i_sound.c:779-781). For a
       sample-exact comparison, the compared signal is the sequence of mixed buffers, not
       the sound at the speaker.
    7. **Interaction with s_sound handles.** `I_SoundIsPlaying` compares the handle counter
       with `gametic` (item 22). This is tic-domain and deterministic, but it depends on the
       configuration (S or I) chosen in Q1.

#### 1.3.7 What a port must reproduce versus linuxdoom's platform glue

**(A) Deterministic logic** (same starts in, same samples out) **[src]**:

- A1. Sound data: name `"ds" + name`, lump lookup, `dspistol` copy for a missing lump at load
  (i_sound.c:201-216), header skip of 8 bytes, `paddedsize` formula, padding with 128, `*len`
  = `paddedsize` (i_sound.c:218-250).
- A2. The link length: `lengths[sfx_chgun]` = `lengths[0]` = 0 (i_sound.c:811), and the
  shared data pointer (i_sound.c:810).
- A3. Tables: `steptable` (i_sound.c:414-415) and `vol_lookup` (i_sound.c:421-423).
- A4. `I_StartSound`: `steptable[pitch]`, priority ignored, the handle returned
  (i_sound.c:480, 495-499).
- A5. `addsfx`: single-instance list and rule, channel choice with `channelstart` and
  `gametic`, pointer/end/step/remainder/start/handle/id assignments, the separation
  arithmetic, the range check, the row pointers (i_sound.c:271-380).
- A6. `I_UpdateSound`: the per-frame loop, channel order 0..7, fetch-accumulate-advance-test,
  clipping, interleaving (i_sound.c:565-635).
- A7. `I_SoundIsPlaying` = `gametic < handle`; `I_StopSound` and `I_UpdateSoundParams` are
  no-ops (i_sound.c:505-521, 674-688). These shape s_sound's state, not the samples.

**(B) Platform glue**, with what each call does, so a headless harness can stub exactly
these **[src]**:

- B1. `open("/dev/dsp", O_WRONLY)` (i_sound.c:770): opens the OSS device; on failure prints
  "Could not open /dev/dsp" but continues with `audio_fd` = -1 (:771-772). The next
  `myioctl` then fails and `myioctl` calls `exit(-1)` (:168-174), so without a device the
  integrated build exits during `I_Init`. Stub: return a dummy fd.
- B2. `myioctl(fd, command, arg)` (i_sound.c:159-175): `ioctl`, and on `rc < 0` prints
  the command and `errno` and `exit(-1)`. It declares `extern int errno;` (:166), which does
  not link against modern glibc's thread-local `errno` (observed, 1.3.10: needed a
  `-Derrno=` adapter).
- B3. `SNDCTL_DSP_SETFRAGMENT` with `i = 11 | (2<<16)` (:775-776): 2 fragments of 2048 bytes.
- B4. `SNDCTL_DSP_RESET` with a null argument (:777).
- B5. `SNDCTL_DSP_SPEED` with `i = 11025` (:779-781); the rate the driver writes back into `i`
  is ignored.
- B6. `SNDCTL_DSP_STEREO` with `i = 1` (:783-784).
- B7. `SNDCTL_DSP_GETFMTS` into `i` (:786), then `if (i&=AFMT_S16_LE)
  myioctl(audio_fd, SNDCTL_DSP_SETFMT, &i); else fprintf(stderr, "Could not play signed 16
  data\n");` (:788-791): selects little-endian signed 16-bit if offered; otherwise prints and
  continues writing native shorts to a device in its default format.
- B8. `write(audio_fd, mixbuffer, 2048)` in `I_SubmitSound` (:669) and in the timer handler
  (:927): blocking write of one buffer, result ignored. Stub: capture the 2048 bytes; this
  is the comparison point.
- B9. `close(audio_fd)` (:725).
- B10. Timer (T only): `sigaction`, `setitimer` (:941-985).
- B11. The `fprintf(stderr, ...)` progress lines (:768, 793, 797, 815, 822, 709).
- B12. S only: `getenv("DOOMWADDIR")`, `access(X_OK)`, `popen(..., "w")`, `fprintf`/`fflush`
  to the pipe (:740-757, 483-487, 696-700).

  A headless harness for the integrated mixer stubs B1, B2 (or the five ioctls B3-B7), B8
  and B9 and nothing else; everything in (A) runs as written. That is what the probe in
  1.3.10 does.

**Context, not evidence.** README.sound:7-13 (a repo file) says DOS DOOM's sound used the
licensed DMX library and that the original DOS sound code was removed because it does not
work without DMX. So in the DOS original the equivalents of A1 (the sample format
handling), A3, A5 and A6 lived in DMX, and everything in (A) above is specific to the Linux
code (i_sound.c and its near-copy in sndserv/). Beyond that README, which DMX behaviour
differed (mixing rate, channel stealing, volume curve, pitch) is not supported by any
repository file and is not evidence here.

#### 1.3.8 Undefined, implementation-defined or layout-dependent behaviour

1. **Read past the allocation for very short lumps.** A lump of 0..8 bytes gets an 8-byte
   allocation and `paddedsize` 0 (item 11). `addsfx` sets `channelsend == channels`, and the
   first mixed frame reads `*channels`, which is `paddedsfx[8]`, one byte past the block
   (undefined behaviour; in zone memory it reads whatever follows the block). For chgun
   (length 0) the read is in bounds (the pistol's first sample). No other read past
   `channelsend` is possible: positions are tested after each advance (item 23).
2. **The `lengths[]` index for linked sounds** depends on `sizeof(sfxinfo_t)` (36 on 32-bit,
   48 on x86-64) but evaluates to 0 for any size above 1 (item 12); the read of `lengths[0]`
   is in bounds and reads the static zero. Defined behaviour, wrong intent.
3. **Uninitialised state.** All mixer arrays are static and zero-initialised (item 7); no
   uninitialised read. `channelids`, `channelstart` of never-used channels are 0.
4. **Signed overflow.** None reachable: `volume*sep*sep` ≤ 8,323,072 for volume ≤ 127;
   `vol_lookup` products ≤ 4,161,536; mix sums within ±262,144 (items 13, 19, 23). An
   arbitrary `volume` above about 32767 would overflow `volume*seperation*seperation`, but
   item 18 shows the callers cannot pass one.
5. **Negative integer division.** `(i*(j-128)*256)/127` with a negative dividend (i_sound.c:423):
   truncation toward zero is required by C99 but implementation-defined in C89 (gcc/x86
   truncates toward zero; the observed table uses it). A port must truncate toward zero.
6. **Pointer arithmetic on the volume lookup.** `&vol_lookup[leftvol*256]` with leftvol
   0..127 and index `sample` 0..255 stays inside the 32768-element array (i_sound.c:372-373,
   596-597). In range for every argument s_sound can pass (item 18).
7. **Char signedness.** Samples are read through `unsigned char*` (i_sound.c:122, 591), so
   they are 0..255 regardless of the platform's `char` signedness. Padding 128 is written
   through `unsigned char*` (:241). No signedness dependence in i_sound.c.
8. **Floating point.** Only `steptable` (`pow`, i_sound.c:415); margin 0.0125 to the nearest
   integer, so not sensitive to libm rounding (item 13, observed on two ABIs).
9. **The realloc of a zone-cached lump** is commented out in i_sound.c (:234); the
   integrated code copies into a new zone block and frees the cached lump through
   `Z_Free`, which is defined behaviour (z_zone.c:122-139). (The sndserver's
   `realloc` of memory it got from its own loader is outside this section.)
10. **Endianness.** `mixbuffer` is written as native shorts and the device is configured for
    little-endian 16-bit (i_sound.c:788-789); on a big-endian host the output bytes are
    swapped. The lump bytes are single bytes, so loading has no endianness dependence.
11. **Pointer comparison after free.** None in i_sound.c: a freed channel is set to NULL,
    never compared.
12. **`id` out of range.** `S_StartSoundAtVolume` accepts `sfx_id == NUMSFX` (s_sound.c:277),
    which would make `addsfx` read `S_sfx[109]` and `lengths[109]` out of bounds
    (i_sound.c:330-332). Whether any caller can pass 109 is for §1.2 (§1.2.8 item 4 finds that no call site passes it).
13. **Writing to fd 0 under S** (item 5): `write(0, ...)` on an fd the game never opened.
14. **`extern int errno`** (i_sound.c:166) conflicts with thread-local `errno` in modern C
    libraries (link error observed, 1.3.10).

#### 1.3.9 Implementation-ready summary

State a port keeps (per mixer channel c = 0..7): `pos[c]` (index into the sound's sample
bytes, or "free"), `end[c]` (= padded length), `data[c]` (which sound's padded bytes),
`step[c]: u32`, `frac[c]: u32` (0..65535), `start[c]: i32` (gametic), `id[c]: i32`,
`lvol[c]`, `rvol[c]` (0..127), `handle[c]` (written, never read); plus the global
`handlenums: u16` (initially 0). Per sound: padded bytes and `len`. Tables: `steptable[256]`,
`vol_lookup[128][256]`.

Table generation:
```text
for p in 0..=255: steptable[p] = trunc(2^((p-128)/64) * 65536)   // matches digest c3b9727b3def2608
for v in 0..=127, s in 0..=255: vol[v][s] = trunc_toward_zero(v*(s-128)*256 / 127)
```

Loading sound i (1..108), integrated configuration:
```text
if S_sfx[i].link: data[i] = data[link]; len[i] = 0          // lengths[0], i_sound.c:811
else:
  lump = lump "ds"+name if present else lump "dspistol" (I_Error if that is missing)
  size = lump length;  L = ((size - 8 + 511) / 512) * 512    // C int division; L >= 0
  bytes = lump bytes 8..size-1, then 128 repeated until there are L bytes
  (if size <= 8: L = 0, and the single frame such a sound plays reads one byte past the
   block in C — a port must choose and declare a value for that byte, see Q3)
len[i] = L
```

Start (`I_StartSound(id, vol, sep, pitch, priority)`), returns the handle:
```text
step = steptable[pitch]
if id in {sawup, sawidl, sawful, sawhit, stnmov, pistol}:
    first c in 0..7 with pos[c] != free and id[c] == id: pos[c] = free
c = first index in 0..7 with pos[c] == free
if none: c = lowest index whose start[c] is the minimum among start[c] < gametic; else 0
pos[c] = 0; end[c] = len[id]; data[c] = id's bytes
if handlenums == 0: handlenums = 100
handle[c] = handlenums; h = handlenums; handlenums = (handlenums + 1) mod 65536
step[c] = step; frac[c] = 0; start[c] = gametic
s = sep + 1
lv = vol - ((vol*s*s) >> 16);  s = s - 257;  rv = vol - ((vol*s*s) >> 16)
if rv < 0 or rv > 127: I_Error("rightvol out of bounds")
if lv < 0 or lv > 127: I_Error("leftvol out of bounds")
lvol[c] = lv; rvol[c] = rv; id[c] = id
return h
```

Mix one buffer (512 frames) into `out[1024]` of i16:
```text
for k in 0..512:
    dl = 0; dr = 0                                   // i32
    for c in 0..8:
        if pos[c] != free:
            b = data[c][pos[c]]                      // u8
            dl += vol[lvol[c]][b];  dr += vol[rvol[c]][b]
            frac[c] += step[c]                       // u32, no wrap possible
            pos[c] += frac[c] >> 16
            frac[c] &= 0xFFFF
            if pos[c] >= end[c]: pos[c] = free
    out[2k]   = clamp(dl, -32768, 32767)
    out[2k+1] = clamp(dr, -32768, 32767)
emit out as 2048 bytes, little-endian i16, left first
```
`I_SoundIsPlaying(h) = gametic < h`; `I_StopSound`, `I_UpdateSoundParams`: no effect. The
schedule (how many buffers per tic, where starts land) is not part of the C logic and must be
declared (item 30).

#### 1.3.10 What was run (observations)

All in `~/scratch/m5c/c-isound/` (outside the repository). gcc 15.2.0, glibc 2.43, x86-64
WSL2. No IWAD was read; all lumps are synthetic.

- **Run A, table and arithmetic probe**: `~/scratch/m5c/c-isound/tables.c` copies the
  expressions of i_sound.c:230, 352-368, 414-415, 421-423 and includes the genuine
  `sounds.h`. Built as `gcc -O0 -I.../linuxdoom-1.10 tables.c -lm` (64-bit) and with `-m32`.
  Results: `sizeof(sfxinfo_t)` 48 (64-bit) / 36 (32-bit), chgun index 1/size = 0 on both;
  steptable FNV-1a-64 `c3b9727b3def2608` on both ABIs, minimum distance to an integer
  0.012529933 (i = -16); vol_lookup FNV-1a-64 `552057530633d3e9`, min -32768, max 32512;
  separation check over volume 0..200 × sep 0..255: 15,558 failing pairs, first failing
  volume 128; paddedsize for sizes 0..10 and 519..522 as in item 11.
- **Run B, genuine i_sound.c under a declared adapter**: source exported with
  `git -C /home/copgunn/src/DOOM archive 7f7fd99 linuxdoom-1.10 | tar -x -C ~/scratch/m5c/c-isound/src`;
  in that copy only, doomdef.h:84 was commented out (integrated configuration I).
  `i_sound.c` and `sounds.c` compiled unmodified with
  `gcc -std=gnu89 -g -O0 -DNORMALUNIX -DLINUX -Derrno=probe_errno -Dopen=stub_open -Dioctl=stub_ioctl -Dwrite=stub_write`
  (gcc 15's default C23 rejects doomtype.h:34's `false` enumerator, hence `-std=gnu89`;
  `-Derrno=` because of item 1.3.8.14). Adapter `~/scratch/m5c/c-isound/probe/probe.c`
  supplies W_*/Z_*/I_Error stubs over a synthetic WAD (one lump per non-link sound with
  generated bytes, `dshtgn` deliberately missing, `dssgcock` 5 bytes, `dsdbopn` 520 bytes,
  `dsdbcls` 8 bytes) and stub open/ioctl/write. It calls the real `I_InitSound`,
  `I_SetChannels`, `I_StartSound`, `addsfx`, `I_UpdateSound`, `I_SubmitSound`. Output:
  lengths pistol 512, chgun 0, 5-byte lump 0, 520-byte lump 512 (no padding), 8-byte lump 0,
  missing lump 512 (pistol copy); chgun shares pistol data; 412 of 412 padding bytes are 128;
  a 5-byte lump keeps its 5 header bytes and gets 3 bytes of 128; 107 zone blocks live after
  init; first handle 100; pistol at vol 15 sep 128 → rows L=R=12, first frame -2128/-2128
  equals `vol_lookup[12*256+40]` computed independently; `I_SubmitSound` wrote 1 × 2048
  bytes; elements 1024..2047 stay 0; chgun (step 83168 = steptable[150]) gives exactly 2
  non-zero shorts (one frame) and frees its channel; shotgun, pistol, pistol → 2 active
  channels, the second pistol replaced the first in slot 1; oldest rule picked slot 5
  (started at 50) and, with all starts equal to gametic, slot 0; handle 65535 is followed by
  100. Because the stub ioctl does not fill `GETFMTS`, the probe printed "Could not play
  signed 16 data" (glue only).
- Files that matter for Phase 2: `~/scratch/m5c/c-isound/probe/probe.c` (adapter pattern for
  a genuine-C mixer trace generator) and `~/scratch/m5c/c-isound/tables.c` (table digests).

#### 1.3.11 Questions the mixer's facts raise for the owner

Each question is taken up in [§5](#5-owner-decisions-t1t16-with-recommendations): Q1 in T2; Q2 in T5 (this section's recommendation, one 512-frame buffer per tic, differs from §5's: the schedule measured in §3.2 supersedes it, see T5); Q3 in T11; Q4 in T13.

- **Q1. Which C mixer is the reference?** (a) The integrated mixer of i_sound.c, compiled
  with doomdef.h:84 removed (configuration I), which the ChangeLog says id/bk switched away
  from because it sounded bad; or (b) the shipped configuration S, whose audio comes from
  sndserv/ (same tables, same addsfx and mix formulas, but `channelstart` is the server's
  loop counter `mytime` (sndserv/soundsrv.c:430, 478, 629) instead of `gametic`, its own
  schedule driven by `select` on the pipe, and `I_StartSound` returning the sfx id to
  s_sound). **Recommendation: (a)** for the samples, because the brief names i_sound.c and
  its state is tic-domain (`gametic`), declaring the configuration explicitly; and record (b)
  as the shipped behaviour, particularly its effect on s_sound's handles (item 22), so §1.1
  decides which `I_StartSound` return value / `I_SoundIsPlaying` behaviour
  the tic-for-tic start sequence follows.
- **Q2. Which mixing schedule?** (a) One 512-frame buffer per tic, starts applied before
  the tic's buffer (the `-timedemo`/singletics schedule, item 29; audio then runs 512/315 ≈
  1.63 times longer than real time per tic); or (b) a real-time-shaped schedule (315 frames
  per tic, which the C never produces, or a fixed 1-2 tics per buffer pattern).
  **Recommendation: (a)**, because it is a schedule the genuine loop actually executes
  (d_main.c:375-386 followed by d_main.c:397-405), it is exactly reproducible in a C harness,
  and the port can resample or re-chunk for presentation separately without changing the
  compared samples.
- **Q3. The out-of-bounds byte for lumps of 8 bytes or fewer** (1.3.8 item 1). (a) Refuse
  such lumps at load as a declared fidelity gap; or (b) declare a value (e.g. 128, silence).
  **Recommendation: (a)** with a refuse witness, consistent with AGENTS.md's rule that a
  fidelity gap is refused, not silently diverged; no shipped IWAD question was checked.
- **Q4. Chaingun length 0.** (a) Reproduce it (one-frame chaingun, as the C does in both
  configurations); or (b) "fix" it. **Recommendation: (a)**: it is genuine behaviour of
  both the integrated code and the sndserver (i_sound.c:811, sndserv/soundsrv.c:390), and a
  fix would be a new profile requiring the owner's explicit choice.

#### 1.3.12 What this section did not do or observe

- This section did not run linuxdoom on a real or emulated OSS device, so the device-paced cadence
  (≈21.5 iterations per second, 1.63 tics per iteration) is inference, not observation.
- This section did not build or run the sndserver; the contrasts cite its source only. It did not check
  the s_sound.c call sites that start sounds (that is §1.2), the exact `finesine` extremes of `sep`, or
  whether any caller can pass `sfx_id == NUMSFX`; those belong to §1.1.
- This section did not read any IWAD, so it did not check whether any real `ds` lump is 8 bytes or
  shorter, an exact multiple of 512 after the header, or has header fields disagreeing with
  its length.
- This section did not compile the shipped configuration S, so the write to fd 0 (item 5) is read from
  the source, not observed.
- No cargo, no repository tool, no generator was run; nothing was written inside the
  repository. Disk use is a few MB in `~/scratch/m5c/c-isound/`.

### 1.4 M_Random accounting and the level-start wipe

Written 2026-11-18 (America/Los_Angeles) against linuxdoom-1.10 at the repository head 7f7fd99
(f_wipe.c sha256 `eafbc10e…2a22`, m_random.c sha256 `8d89f5cf…21ee`). Every citation below was
read in this tree. Terms: **draw** means one call of `M_Random`; **the index** is `rndindex`;
**the wipe** or **melt** is the screen transition in f_wipe.c; a **tick** in f_wipe.c is one
35 Hz clock unit passed in as `ticks`, which is not a game tic (no game tic runs during a wipe).

Three kinds of statement are kept apart: **(S)** read in the source, **(O)** observed by running a
probe, with its command and counts, and **(I)** inference.

#### 1.4.1 m_random.c in full, and every use of the index

1. (S) The file has one table, `unsigned char rndtable[256]` (m_random.c:31-51), and two indices
   `int rndindex = 0; int prndindex = 0;` (53-54).
   - `P_Random` does `prndindex = (prndindex+1)&0xff; return rndtable[prndindex];` (57-61).
   - `M_Random` does `rndindex = (rndindex+1)&0xff; return rndtable[rndindex];` (63-67). The
     index is increased **before** the read, so the first draw after a reset reads `rndtable[1]`.
   - `M_ClearRandom` does `rndindex = prndindex = 0;` (69-72).
   - The header calls `M_Random` "a number from 0 to 255" (m_random.h:31-33) and P_Random the one
     "used only by the play simulation" (35-36). `rndindex` is exported through doomstat.h:281.
2. (S) The draws are a pure counter. Each draw adds 1 to the index modulo 256 and has no other
   effect. Only the **number** of draws between two points changes the index; their order
   changes only **which value each caller receives**.
3. (S) `M_ClearRandom` has one caller, `G_InitNew` (g_game.c:1414). `G_DoLoadLevel` (445-500),
   `G_DoReborn` (924-968) and `G_DoWorldDone` (1172-1179) do not call it.
4. (S) Every `M_Random` call site and every use of `rndindex`, from a grep of all `*.c` and `*.h`
   in linuxdoom-1.10 (`grep -n "M_Random\|rndindex\|prndindex\|M_ClearRandom" *.c *.h`). A grep
   of the whole repository's `.c` and `.h` files finds no other use in sndserv/, ipx/ or sersrc/.

| Site | Draws | Condition | Reachable in a single-player level run (no menu, no intermission)? |
|---|---|---|---|
| st_stuff.c:992 `st_randomnumber = M_Random();` in `ST_Ticker` (988-996) | 1 | every tic whose `gamestate` is `GS_LEVEL`: `G_Ticker` calls `ST_Ticker` (g_game.c:731) after `P_Ticker` (730) in its `GS_LEVEL` case (727-735). It also runs while paused or in the menu, because `P_Ticker` returns early on `paused` or menu (p_tick.c:135-145) but `ST_Ticker` is called anyway | yes, once per tic |
| s_sound.c:330 `pitch += 8 - (M_Random()&15);` | 1 | a started sound with `sfx_sawup <= sfx_id <= sfx_sawhit` (327-328) that passed the early returns (§1.4.2) | yes (the chainsaw) |
| s_sound.c:340 `pitch += 16 - (M_Random()&31);` | 1 | any other started sound except `sfx_itemup` and `sfx_tink` (337-338) that passed the early returns | yes |
| f_wipe.c:159 `y[0] = -(M_Random()%16);` | 1 | `wipe_initMelt`, once per wipe | yes, one wipe at level start (§1.4.5) |
| f_wipe.c:162 `r = (M_Random()%3) - 1;` | 319 | the loop `for (i=1;i<width;i++)` (160) with `width` 320 | as above |
| wi_stuff.c:523, 525, 561 | 1 each | intermission background animations | no (intermission) |
| g_game.c:692 `consistancy[i][buf] = rndindex;` (read only) | 0 | `if (netgame && !netdemo && ...)` (681), and only for a player without `mo` (689-692) | no (netgame only) |
| g_game.c:1414 `M_ClearRandom ();` (write) | sets the index to 0 | `G_InitNew` | yes, once at the start of a new game |

5. (S) A wipe makes **exactly 320 draws**. `y[0]` takes one draw (f_wipe.c:159) and the loop
   takes one for each `i` from 1 to `width-1` (160-162). `width` is the `width` argument of
   `wipe_ScreenWipe`, passed through unchanged at f_wipe.c:285. D_Display passes `SCREENWIDTH`
   (d_main.c:339-340), which is 320 (doomdef.h:110). 320 ≡ 64 (mod 256).
6. (S) Re-verification of earlier citations:
   - docs/M5B_REFERENCE_INVESTIGATION.md §1.3.6 and §1.4.5 cite m_random.c:31-51, 53-54, 57-61,
     63-67 and 69-72, g_game.c:1414 and 692, st_stuff.c:992 and 915, f_wipe.c:159 and 162,
     wi_stuff.c:523, 525 and 561, s_sound.c:330 and 340, and m_misc.c:237. All are correct.
   - §2.5 FH1's "d_main.c:223-229, 324-353" is slightly off. The wipe decision is d_main.c:222-229,
     and the melt is 326-344, ending D_Display at 345. Lines 346-353 are D_DoomLoop's header.
   - §3.8's "d_main.c:222-226, 326-344" is correct.
   - The committed helper comments (not to be changed; historical bytes) carry two stale line
     numbers. m5b_driver.c:854 says "m_random.c:47-51", but `M_Random` is at m_random.c:63-67.
     m3h9c_stubs.c:16, m5a_stubs.c:28 and m5b_stubs.c:40 say "st_stuff.c:991", but the draw is at
     st_stuff.c:992. These are citation slips only. A new runner's comments should cite 63-67 and
     992.

#### 1.4.2 Sound's draws: the accounting in S_StartSoundAtVolume

1. (S) `S_StartSound(origin, sfx_id)` only calls `S_StartSoundAtVolume(origin, sfx_id,
   snd_SfxVolume)` (s_sound.c:398-407; the `SAWDEBUG` block is not compiled). No other code calls
   `S_StartSoundAtVolume` (grep).
2. (S) The order inside `S_StartSoundAtVolume` (s_sound.c:254-395):
   1. Bad id: `if (sfx_id < 1 || sfx_id > NUMSFX) I_Error(...)` (277-278). This is fatal, not a
      return.
   2. **Early return A, linked sounds** (283-294). `if (sfx->link)`, the code does `volume +=
      sfx->volume; if (volume < 1) return;` (287-290). The only linked entry is `chgun`, which
      links to `pistol` with link volume 0 (sounds.c:204). It therefore returns, without a draw,
      exactly when `snd_SfxVolume < 1`. No source line starts `sfx_chgun` (grep), so this branch
      is unreachable in practice.
   3. **Early return B, inaudible** (304-320). Only when `origin && origin !=
      players[consoleplayer].mo` does it call `S_AdjustSoundParams` (306-310) and `if (!rc)
      return;` (318-319). `S_AdjustSoundParams` returns 0:
      - when `gamemap != 8` and the approximate distance exceeds `S_CLIPPING_DIST` (1200 map
        units; s_sound.c:55, 771-777);
      - otherwise when the computed volume is not positive: `return (*vol > 0);` (817), with
        volume from 796-815.
      - (I) With the default `snd_SfxVolume` 8 (m_misc.c:237), the map-8 formula
        `15 + ((8-15)*d)/1040` gives at least 8 for every clipped distance d in 0..1040, so on
        map 8 no sound is ever inaudible at the default volume.
   4. **The draw** (326-346). It is one draw for every id except `sfx_itemup` and `sfx_tink`, which
      take none. The chainsaw ids `sfx_sawup..sfx_sawhit` (sounds.h:189-192) draw with `&15`. All
      others draw with `&31`.
   5. Only after the draw come `S_StopSound(origin)` (349) and `cnum = S_getChannel(origin, sfx)`
      (352), then `if (cnum<0) return;` (354-355). `S_getChannel` returns -1 when every channel
      holds a sound of higher priority (s_sound.c:850-860).
3. (S) The answers:
   - **0 or 1 draw per call**, never more.
   - A sound that **fails to get a channel still draws**: the draw precedes 352-355.
   - A sound **made inaudible by distance or volume does not draw** (318-319).
   - A sound **with no origin (`NULL`) or with the player as origin is never tested for
     audibility**. It draws even at `snd_SfxVolume` 0, except chgun.
   - `sfx_itemup` and `sfx_tink` never draw.
   - The draw does not depend on `numChannels`, on what is playing, on the sound device or on
     `I_StartSound`'s result.
4. (S) `S_UpdateSounds` (s_sound.c:519 onward), called once per loop pass before D_Display
   (d_main.c:392), contains no `M_Random`. The only `M_Random` lines in s_sound.c are 330 and 340
   (grep).
5. (S) Sound starts that a single-player level session reaches without a menu all come from the
   play simulation inside `P_Ticker`, which runs before `ST_Ticker` in the same tic (g_game.c:730-731).
   The non-playsim callers are excluded as follows:
   - hu_stuff.c:566 and 568 are chat from another player (`i != consoleplayer`, 542-543): netgame
     only.
   - g_game.c:886 is in `G_CheckSpot`, which single player reaches only for the first spawn and
     returns before the sound (855-862). The respawn callers (909, 947, 956) are deathmatch and
     netgame (`G_DoReborn` sets `ga_loadlevel` when `!netgame`, 928-932).
   - wi_stuff.c and f_finale.c are intermission and finale.
   - m_menu.c is the menu. **Note:** with the menu closed, `M_Responder` still starts sounds at
     *event* time for `-`, `=`, F1-F10 and Esc (m_menu.c:1519-1595 and 1609-1615), and F11 sets
     the palette (1597-1603). Events are dispatched inside `NetUpdate`'s build loop before each
     command (d_net.c:402-403) and in D_Display's `NetUpdate` (d_main.c:316). These draws
     therefore fall **between** tics, not inside one. They are outside M5c's scope (menus) but
     are reachable in a vanilla level session by pressing those keys.
   - So within tic t: s_t sound draws in `P_Ticker`, then 1 draw in `ST_Ticker`.

#### 1.4.3 f_wipe.c in full

All variables are `int` unless named otherwise. `boolean` is `typedef enum {false, true}`
(doomtype.h:34). `byte` is `unsigned char`.

1. (S) File statics: `static boolean go = 0;` (f_wipe.c:43); `static byte* wipe_scr_start,
   *wipe_scr_end, *wipe_scr;` (45-47); `static int* y;` (138). The `int y` parameters of
   `wipe_StartScreen`, `wipe_EndScreen` and `wipe_ScreenWipe` shadow the static `y` inside those
   functions (harmless; those functions do not touch the array).
2. (S) **`wipe_StartScreen(x, y, width, height)`** (236-246) ignores its arguments. It sets
   `wipe_scr_start = screens[2]` and calls `I_ReadScreen(wipe_scr_start)`. That is
   `memcpy(scr, screens[0], SCREENWIDTH*SCREENHEIGHT)` (i_video.c:527-530): a copy of whatever
   `screens[0]` holds at that moment.
3. (S) **`wipe_EndScreen(x, y, width, height)`** (248-259):
   - sets `wipe_scr_end = screens[3]`;
   - reads `screens[0]` into it;
   - `V_DrawBlock(x, y, 0, width, height, wipe_scr_start)` copies the start screen back into
     `screens[0]` (v_video.c:405-433: `V_MarkRect`, then row `memcpy`s).
   - So after it, `screens[0]` holds the start screen again and `screens[3]` the new frame.
4. (S) **`wipe_ScreenWipe(wipeno, x, y, width, height, ticks)`** (261-302):
   - The function-static table `wipes[]` (271-275) holds six function pointers, three per wipe
     kind, indexed `wipeno*3` (init), `wipeno*3+1` (do) and `wipeno*3+2` (exit).
   - `wipe_ColorXForm` is 0 and `wipe_Melt` is 1 (f_wipe.h:30-39), so the melt uses entries 3, 4
     and 5: `wipe_initMelt`, `wipe_doMelt` and `wipe_exitMelt`.
   - If `!go`, it sets `go = 1` and `wipe_scr = screens[0]`, then calls init (280-286).
   - Every call then runs `V_MarkRect(0,0,width,height)` (289). That only grows `dirtybox`
     (v_video.c:143-150), which no code reads.
   - It calls do (290). If do returns nonzero, it sets `go = 0` and calls exit (294-298).
   - It returns `!go` (300), which is 1 on the call that finished.
5. (S) **`wipe_shittyColMajorXform(short* array, width, height)`** (50-70) transposes a buffer in
   place:
   - it allocates `dest` with `Z_Malloc(width*height*2, PU_STATIC, 0)` (60);
   - it copies `dest[x*height+y] = array[y*width+x]` (62-64);
   - it copies back with `memcpy` (66) and frees (68).
   - It is called with `(short*)` casts and `width/2`, i.e. 160×200 `short`s (153-154). The
     screen's 320×200 bytes are treated as 160 two-pixel `short`s per row, and the result is
     **column-major**: the 200 shorts of column pair i (pixels 2i and 2i+1) sit contiguously at
     `[i*200 .. i*200+199]`. The comment says why: "makes this wipe faster (in theory) to have
     stuff in column-major format" (151-152). Each `short` is moved as a unit and never
     interpreted, so byte order inside the pair is preserved on any endianness.
6. (S) **`wipe_initMelt(width, height, ticks)`** (140-169), with `width` 320 and `height` 200:
   1. `memcpy(wipe_scr, wipe_scr_start, width*height)` (149) copies the row-major start screen
      into `screens[0]`. This is redundant after `wipe_EndScreen`'s restore.
   2. It transposes `screens[2]` (start) and `screens[3]` (end) to column-major shorts (153-154).
      They **stay** column-major after the wipe; nothing else reads them (`screens[2]` is reused
      as scratch by the screenshot code, m_misc.c:510).
   3. `y = Z_Malloc(width*sizeof(int), PU_STATIC, 0)` (158) allocates **320** ints.
   4. `y[0] = -(M_Random()%16)` (159) gives a value in -15..0.
   5. For i = 1..319 (160-166): `r = (M_Random()%3) - 1` gives -1, 0 or +1;
      `y[i] = y[i-1] + r`; then `if (y[i] > 0) y[i] = 0; else if (y[i] == -16) y[i] = -15;`.
      Because y[i-1] ≥ -15 and r ≥ -1, y[i] ≥ -16 before the clamp, so **every y[i] is in -15..0**.
   6. **Exactly 320 draws.** Only y[0..159] are ever used (item 7). The draws for y[160..319] are
      made and their values discarded.
   7. The `ticks` argument is unused.
7. (S) **`wipe_doMelt(width, height, ticks)`** (171-224):
   - `done = true` at entry (184). `width /= 2` (186) gives 160 columns of shorts.
   - `while (ticks--)` (188) runs one melt step per tick. Each step goes over i = 0..159
     (190-219):
     - If `y[i] < 0`: `y[i]++` and `done = false` (192-195). The column waits.
     - Else if `y[i] < height`:
       - `dy = (y[i] < 16) ? y[i]+1 : 8`, then `if (y[i]+dy >= height) dy = height - y[i]`
         (198-199).
       - It copies `dy` shorts of the end screen's column i, rows y[i]..y[i]+dy-1, into the same
         rows of `screens[0]`. The source is `s = &end[i*height+y[i]]` (200); the destination
         `d = &scr[y[i]*width+i]` steps `idx += width` (201-207).
       - Then `y[i] += dy` (208).
       - It copies the start screen's column i, rows 0..height-y[i]-1, into `screens[0]` rows
         y[i]..height-1 (209-216). The start screen is pushed down by y[i].
       - `done = false` (217).
     - Columns with `y[i] >= height` are finished and untouched.
   - It returns `done` (222): 1 only if **no** column was waiting or moving in **any** of this
     call's tick iterations.
   - (I) So after T cumulative ticks, `screens[0]` column i shows end rows [0, y_T[i]) at the top
     and start rows [0, 200-y_T[i]) below, while columns still waiting show the start screen.
     When every y = 200, `screens[0]` equals the end screen.
8. (S) **`wipe_exitMelt`** (226-234) does `Z_Free(y)`.
9. (S) **The colour cross-fade** `wipe_initColorXForm`, `wipe_doColorXForm` and
   `wipe_exitColorXForm` (72-135) steps each byte (a *palette index*, not a colour) toward the
   end value by `ticks`. It is **unreachable**: the only call of `wipe_ScreenWipe` passes
   `wipe_Melt` (d_main.c:339). The only callers of the three wipe entry points are d_main.c:226,
   327 and 339 (grep).
10. (S) The static `go` is 0 between wipes. D_Display's loop always calls until `done`
    (d_main.c:331-344), and that call resets `go` (f_wipe.c:296), so a new wipe always starts
    with init.
11. (I, O) **Duration.**
    - From y = 0 at height 200, the dy rule gives the steps 0→1→3→7→15→31 (5 ticks), then +8 per
      tick to 199 (21 ticks), then a clamped +1 to 200 (1 tick). That is **27 moving ticks**.
    - A column that starts at y0 needs -y0 waiting ticks plus 27 moving ticks.
    - The melt is still active on the tick where the last column reaches 200, so the call that
      returns done comes after **28 + max over i<160 of (-y0[i])** ticks with one tick per call.
    - The bound is therefore 28 to 43 ticks, because -y0 ≤ 15 (item 6).
    - Observed over all 256 possible starting indices (probe, §1.4.9): max(-y0[0..159]) ranges
      over 6..15, so the number of one-tick calls to done is **34 to 43**. Histogram
      (calls:starting indices): 34:3, 35:46, 36:4, 37:10, 38:63, 39:17, 40:16, 41:20, 42:15,
      43:62.

#### 1.4.4 D_Display's wipe path and the startup to the first frame

1. (S) `gamestate_t wipegamestate = GS_DEMOSCREEN;` (d_main.c:188). `gamestate` is a global with
   no initialiser (g_game.c:98), so it starts at `GS_LEVEL` = 0 (doomdef.h:127-133).
2. (S) **Every writer of `wipegamestate`**:
   - d_main.c:300: `oldgamestate = wipegamestate = gamestate;` on every D_Display that gets past
     `nodrawers`;
   - g_game.c:472-473: `if (wipegamestate == GS_LEVEL) wipegamestate = -1;` in `G_DoLoadLevel`;
   - f_finale.c:245 and 379: `wipegamestate = -1` (finale text stage 1 and the cast; out of
     scope).
   - No other line writes it (grep).
3. (S) **Writers of `gamestate`**:
   - g_game.c:475 (`G_DoLoadLevel`), 1133 (`G_DoCompleted` → `GS_INTERMISSION`) and 1174
     (`G_DoWorldDone`);
   - f_finale.c:99;
   - d_main.c:474, 486 and 493 (the title loop's `GS_DEMOSCREEN`).
4. (S) **The wipe in D_Display** (d_main.c:193-345):
   1. `if (nodrawers) return;` (209-210). `nodrawers` is set only by `-timedemo … -nodraw`
      (g_game.c:1627).
   2. `R_ExecuteSetViewSize` if needed (215-220). It draws nothing. `R_InitBuffer` only fills
      `ylookup` with pointers into `screens[0]` (r_draw.c:719).
   3. `if (gamestate != wipegamestate) { wipe = true; wipe_StartScreen(0, 0, SCREENWIDTH,
      SCREENHEIGHT); }` (223-227). **The start screen is `screens[0]` as left by the previous
      display, captured before anything of the new frame is drawn.**
   4. The whole new frame is then drawn into `screens[0]`:
      - `HU_Erase` (231-232);
      - `ST_Drawer(viewheight == 200, redrawsbar)` with `redrawsbar = true` because of `wipe`
        (242-246);
      - `R_RenderPlayerView` (267-268);
      - `HU_Drawer` (270-271);
      - `R_FillBackScreen` on the first `GS_LEVEL` frame (278-282);
      - the border when the view is narrower than 320 (285-295);
      - `wipegamestate = gamestate` (300);
      - the pause picture (303-311);
      - `M_Drawer` (315);
      - `NetUpdate` (316).
   5. `if (!wipe) { I_FinishUpdate (); return; }` (320-324). With a wipe, the composed frame is
      **not** shown. Instead:
   6. `wipe_EndScreen(0, 0, SCREENWIDTH, SCREENHEIGHT)` (327). The end screen is **the complete
      composed frame**: the status bar, the 3D view, the HUD message and the border or pause or
      menu if any. `screens[0]` is restored to the start screen.
   7. `wipestart = I_GetTime () - 1;` (329).
   8. The loop (331-344):
      - `do { nowtime = I_GetTime (); tics = nowtime - wipestart; } while (!tics);` (333-337) is
        a busy wait until the clock differs from `wipestart`. The `- 1` makes the first pass
        immediate, with `tics` ≥ 1.
      - `wipestart = nowtime;` (338).
      - `done = wipe_ScreenWipe(wipe_Melt, 0, 0, SCREENWIDTH, SCREENHEIGHT, tics);` (339-340).
        The first call runs `wipe_initMelt`, which makes the 320 draws.
      - `I_UpdateNoBlit ()` (341; empty), `M_Drawer ()` (342: "menu is drawn even on top of
        wipes"), `I_FinishUpdate ()` (343). One picture is presented per pass.
      - It repeats `while (!done)` (344).
5. (S) **Nothing else runs during the melt.** The loop calls no `NetUpdate`, `I_StartTic`,
   `D_ProcessEvents`, `G_Ticker`, `S_UpdateSounds` or sound submission. In `D_DoomLoop`, the sound
   calls come after D_Display returns (d_main.c:392-405). With `SNDSERV` defined in doomdef.h:84,
   only `I_SubmitSound` (404) is compiled there; `I_UpdateSound` (399) is not. Events queue up in
   X unread, no command is built, and no tic runs.
6. (S) **Game time after the melt.** `gametime` was last set by D_Display's `NetUpdate`
   (d_main.c:316, d_net.c:377-379) before the melt. The next loop pass behaves as follows:
   - **Single player, real time (with or without `-record`)**:
     - `TryRunTics` reads `entertic`. `realtics = entertic - oldentertics` (d_net.c:648-650) is at
       least the melt's length.
     - `NetUpdate` sees `newtics` ≈ the melt length (377-378) and builds commands, each preceded
       by `I_StartTic`/`D_ProcessEvents` (400-410), until `maketic - gametic >= 5` (404-405,
       `BACKUPTICS` 12 in d_net.h).
     - `gametime = nowtime` (379) has already dropped the rest. So 5 − (maketic − gametic as left
       by D_Display's `NetUpdate`) commands are built in a burst, the first of them with every
       event queued during the melt.
     - `availabletics` ≤ 5 (666). Since `realtics` ≥ `availabletics`, `counts = availabletics`
       (673-674), so **up to 5 tics run back to back** (736-766) before the next display.
     - About 28-42 tics of wall-clock time are dropped and never recovered.
   - **Under `-record`** the burst's commands are recorded as they run (`G_WriteDemoTiccmd`,
     g_game.c:668-669), so the demo replays the session exactly. The cost is presentation and
     input sampling only: no input is sampled during the melt, and the queued input lands in the
     first command after it.
   - **Under `-timedemo`** (`singletics = true`, g_game.c:1630): `D_DoomLoop` runs exactly one tic
     per pass (d_main.c:375-386) and never calls `TryRunTics`. The melt costs no tics and causes
     no burst. Commands come from the demo (g_game.c:666-667).
   - **Under `-playdemo`** (real time, `demoplayback`), the post-melt `TryRunTics` runs up to 5
     tics as above. Their commands are read from the demo, so only the displayed frames are
     skipped.
7. (S, I) **Melt frames against time.** The busy wait measures wall-clock time (`gettimeofday`,
   i_system.c:88-100), and `tics` may exceed 1 if a pass (blit, X sync) takes longer than a tick.
   - (O) The state after T cumulative ticks does not depend on how the ticks were grouped into
     calls: over all 256 starting indices, with grouping 1..5 against one tick per call, there
     were 0 digest mismatches.
   - (I) The **set of states presented**, their number, and the call that observes `done` depend
     on real time. The **final** screen, the draws and every intermediate state are a function of
     (start screen, end screen, the index at the wipe's start, cumulative ticks).
8. (S) **The startup to the first D_Display, by start mode.**
   - Common:
     - `V_Init` allocates screens 0-3 zeroed (`I_AllocLow` = `malloc` + `memset 0`,
       i_system.c:147-154; v_video.c:482-493).
     - `I_InitGraphics` is called at the top of `D_DoomLoop` (d_main.c:367) and **replaces**
       `screens[0]` with the X image buffer (i_video.c:910-913). That buffer is either:
       - the attached SysV segment when MIT-SHM is used (`grabsharedmemory`, i_video.c:595-690).
         A segment the code creates is new, and zero-filled by the kernel (I, OS behaviour). A
         stale segment of another user is reused with its old contents (641-646);
       - or a `malloc` of indeterminate contents (`XCreateImage(... malloc(X_width*X_height) ...)`,
         i_video.c:898-907);
       - or, with `-2`/`-3`/`-4`, a separate `malloc(SCREENWIDTH*SCREENHEIGHT)` (913).
     - Between then and the first `wipe_StartScreen`, nothing draws into `screens[0]`:
       - no D_Display runs before the first `TryRunTics` (d_main.c:369-395);
       - the tickers (`M_Ticker`, m_menu.c:1834-1841; `G_Ticker` → `P_Ticker`, `ST_Ticker`,
         `AM_Ticker`, `HU_Ticker`) draw nothing;
       - D_Display's first action before 226 is the non-drawing `R_ExecuteSetViewSize`.
     - **So in every start mode below, the wipe's start screen is the initial X image buffer:**
       all zeros with a fresh shared-memory segment, otherwise unspecified (I).
     - **Palette**: no `I_SetPalette` has run. The callers are d_main.c:275 (never for
       `GS_LEVEL`), st_stuff.c:1049 (only when the computed palette differs from `st_palette`,
       initially 0, 1045-1050), st_stuff.c:1461 (`ST_Stop`, skipped by the first `ST_Start`,
       1447-1448) and m_menu.c:1602 (F11).
     - The colormap is `XCreateColormap(..., AllocAll)` (i_video.c:795) with no colours stored.
       The colours on screen during the melt, and until the first damage or bonus palette, are
       whatever that colormap holds (as M5b §1.3.5 found; re-verified).
   - **`-warp` (and `-warp -record`, or `-record` alone):**
     - `D_DoomMain` calls `G_RecordDemo` for `-record` (d_main.c:1127-1133; allocation only,
       g_game.c:1530-1546), then `G_InitNew` directly (d_main.c:1163-1164).
     - That gives `M_ClearRandom` (g_game.c:1414), then `G_DoLoadLevel` (1481):
       - `wipegamestate` is `GS_DEMOSCREEN`, so it is left alone (472-473);
       - `gamestate = GS_LEVEL` (475);
       - `starttime = I_GetTime ()` (486), which is the process's **first** `I_GetTime` call, so
         `basetime` is fixed and the value returned is the sub-second phase `tv_usec*35/10^6`,
         0..34 (i_system.c:96-98).
     - Then `D_DoomLoop`: `G_BeginRecording` writes the header if recording (d_main.c:356-357;
       g_game.c:1549-1570; no draws, no drawing), then `I_InitGraphics` (367), which waits for the
       first Expose event (i_video.c:835-843).
     - The first `TryRunTics` runs k tics, then `S_UpdateSounds`, then D_Display with the wipe.
     - No draw happens between `M_ClearRandom` and tic 0: p_setup.c has no `S_StartSound`, and
       `ST_Start`/`HU_Start` do not draw.
   - **`-playdemo`:**
     - `G_DeferedPlayDemo` only sets `gameaction = ga_playdemo` (g_game.c:1576-1580; d_main.c:1135-1140).
     - `D_DoomLoop` starts, and the first `TryRunTics`'s `I_GetTime` (d_net.c:648) is the first
       call in the process.
     - Tic 0's `G_Ticker` runs `G_DoPlayDemo` (g_game.c:633-634, 1582-1620). That calls
       `G_InitNew` (`M_ClearRandom`, then `G_DoLoadLevel`, setting `GS_LEVEL`). The same
       `G_Ticker` then reads the demo command and runs `P_Ticker` and `ST_Ticker` (655-735).
     - The first D_Display wipes, because `wipegamestate` is still `GS_DEMOSCREEN`.
   - **`-timedemo`:**
     - `G_TimeDemo` sets `singletics`, `timingdemo` and `ga_playdemo` (g_game.c:1625-1634).
     - The first loop pass runs exactly tic 0, the same `G_DoPlayDemo` path, then D_Display wipes.
     - **With `-nodraw`, D_Display returns at once (d_main.c:209-210): no wipe and no draws, ever.**
     - `-noblit` is parsed (g_game.c:1628) but never read anywhere (grep `noblit`).

#### 1.4.5 Does a session started directly on a map make the wipe's draws, and where?

1. (S) **Yes, in every mode above except `-timedemo -nodraw`.** In each one, the first D_Display
   after `G_InitNew` finds `gamestate` (`GS_LEVEL`) ≠ `wipegamestate` (`GS_DEMOSCREEN`) and wipes.
2. (S, I) **Position in the M_Random sequence.**
   - Let s_t be the number of drawing sound starts in tic t. They are all in `P_Ticker`, before
     tic t's `ST_Ticker` draw (§1.4.2 item 5).
   - The index after tic t is Σ_{u≤t} (s_u + 1) mod 256.
   - The first `TryRunTics` runs tics 0..k-1, with no display in between, then the wipe's 320
     draws follow:
     **[tic 0: s_0 sound draws, 1 ST draw] … [tic k-1: s_{k-1}, 1] [S_UpdateSounds: 0] [D_Display
     frame and its NetUpdate: 0, if no menu keys are pressed] [wipe_initMelt: 320] [melt: 0]
     [tic k …].**
   - The draws come after tic 0's sound draws and ST draw in every mode, and after the k-th tic's
     draws in general.
3. (S, I) **What k is.**
   - **Real time (`-warp`, `-record`, `-playdemo`):** k = the `counts` of the first `TryRunTics`
     (d_net.c:648-677). Both `gametime` (d_net.c:366) and `oldentertics` (641) start at 0, so:
     - `realtics` = `entertic` = the clock value E at line 648;
     - `NetUpdate` builds min(N, 5) commands, where N is the clock value at line 377;
     - `availabletics` = min(N, 5);
     - k = `availabletics` if E = N (the clock did not tick between the two reads); k = N−1
       (but at least 1) if it ticked once and N ≤ 5; and k = 1 after a wait if N = 0.
     - E and N are measured from `basetime`:
       - under `-warp`/`-record`, from the first `I_GetTime` in `G_DoLoadLevel`, whose value is
         the sub-second phase, plus the time of `G_BeginRecording`, `I_InitGraphics` and its
         Expose wait;
       - under `-playdemo`, the sub-second phase at line 648 itself.
     - **k ∈ 1..5, set by the wall clock** (I: usually 5 for `-warp`, since window creation takes
       longer than 5 tics, but not guaranteed).
   - **`-timedemo` (singletics): k = 1 exactly.** The wipe's draws follow tic 0's draws.
4. (I) **What depends on k.** Since the draws only advance a counter (§1.4.1 item 2):
   - the index **after tic k-1** is the same whatever k is (Σ draws + 320);
   - only the values received during tics 1..k-1 differ between a k = 1 schedule and a k > 1 one,
     by +320 ≡ +64. Those are the `ST_Ticker` values and the **sound pitches** of tics 1..k-1.
   - The face reads `st_randomnumber` only on a tic where `st_facecount` is 0 (st_stuff.c:913-917).
     `st_facecount` is set to 17 after such a read (`ST_STRAIGHTFACECOUNT`, 121), so it is read at
     most once in tics 0..4 unless an event sets `st_facecount = 1` (768, 888, 906).
   - The pitch of every drawing sound started in tics 1..k-1 differs between the two schedules.
5. (S) **Other wipes during one level run:**
   - **Death and reborn.** `P_DeathThink`'s use press gives `PST_REBORN`. The next `G_Ticker`
     calls `G_DoReborn` (g_game.c:611-613), which sets `ga_loadlevel` in single player (928-932).
     `G_DoLoadLevel` then finds `wipegamestate == GS_LEVEL` (set by every display, d_main.c:300)
     and sets it to -1 (g_game.c:472-473). The next D_Display wipes, with **320 more draws** and
     no `M_ClearRandom`; its start screen is the last displayed frame. (M5b's S10/S11 refuse the
     reborn and make restart a new game, so this wipe is outside the port's admitted run.)
   - **Level exit** goes to `G_DoCompleted` → `gamestate = GS_INTERMISSION` (g_game.c:1133). The
     next D_Display wipes into the intermission (out of scope), and `G_DoWorldDone` (1174) wipes
     again into the next level.
   - **New game, load game, or the `idclev` cheat** (`G_DeferedInitNew`, st_stuff.c:721) go
     through `G_InitNew`: index 0, then a wipe, because `wipegamestate` was `GS_LEVEL`.
   - **Pause, menu and automap do not wipe:** none changes `gamestate` or `wipegamestate`. The
     pause is a tic button that toggles `paused` (g_game.c:702-707); the menu sets `menuactive`;
     the automap sets `automapactive`. While paused or in the single-player menu, `P_Ticker`
     returns early (p_tick.c:135-145), but `ST_Ticker` still draws once per tic (g_game.c:731).
     Menu sounds draw (m_menu.c).

#### 1.4.6 Is the list of draws complete?

1. (S) In a single-player `GS_LEVEL` session without menu keys, intermission or finale, the only
   `M_Random` draws are:
   - `ST_Ticker`, once per tic (st_stuff.c:992);
   - one per sound start passing the §1.4.2 early returns, except `sfx_itemup` and `sfx_tink`
     (s_sound.c:330, 340);
   - 320 per wipe, with one wipe at level start (f_wipe.c:159, 162).
2. (S) Checked and found draw-free: hu_stuff.c, am_map.c, d_main.c, d_net.c, g_game.c, i_*.c,
   p_*.c and r_*.c. None calls `M_Random` (grep, §1.4.1 table). Their sound starts in a single-player
   level are the playsim's, through `S_StartSound`.
3. (S) The exceptions reachable with keys a vanilla player can press outside the menu:
   `M_Responder`'s hot keys (§1.4.2 item 5), and cheats that start a new game (`idclev`).

#### 1.4.7 What a port must keep

**(a) Accounting only (a counter bump).**

1. Add **320** draws (≡ +64 mod 256) at one declared point: after the first displayed frame of
   a level started by a new game, and before the next tic.
   - Under the port's S8 schedule (one frame per tic, first frame after tic 0), the point is
     **after tic 0's sound draws and ST draw, before tic 1**. This is exactly vanilla
     `-timedemo`'s schedule (k = 1).
   - A real-time vanilla session differs only in the values handed out during tics 1..k-1
     (§1.4.5 item 4).
2. A draw counter like M5b's S16 mask (the dump prints the index without presentation draws)
   must count the 320 as presentation draws.
3. A restart that is a new game (S11) bumps again at its first frame. A reborn is refused (S10),
   so it never bumps.
4. Nothing else in the frame changes. The melt's final screen equals the end screen (§1.4.3
   item 7; observed for all 256 indices), which is the frame the port already draws after tic 0.
   R8's persistent buffer after the melt is therefore the same as without it, as long as no menu
   or `-devparm` dots drew on top (i_video.c:361-374).

**(b) Implementing the melt.**

1. State:
   - `y[0..159]` from the index at the wipe's start. The 320 draws still have to be made; y[160..319]
     are discarded.
   - The start screen. Vanilla's is the uninitialised X image; R8's zeroed initial buffer is the
     natural declared equivalent, matching a fresh shared-memory segment.
   - The end screen: the complete composed first frame, including the status bar at screenblocks
     10, the HUD line, and nothing else at 10/11.
   - The column-major transform is an implementation detail and need not be kept. Only its effect
     matters: two-pixel columns, moved as units.
2. Output: one picture per tick of wipe time, as a function of (start, end, starting index,
   cumulative ticks). With one tick per presented frame that is 34..43 pictures (§1.4.3 item 11).
   The last picture equals the one before it and equals the end screen.
3. During the melt, the status bar and HUD are not drawn separately. They are part of the end
   screen and melt in with the view. With the bar at rows 168-199, a column shows its status bar
   rows once its y passes 168. The start screen fills the rest.
4. No tic, sound update or input sampling runs for 34-43 ticks. The vanilla consequence afterwards
   is a burst of up to 5 tics (real time) or nothing (`-timedemo`). The port would have to declare
   what it does: for example, re-base its clock as for S12's pause.
5. The palette shown during the melt is undefined in vanilla (§1.4.4 item 8). The port would
   declare PLAYPAL 0 through gamma table 0 (S13).

#### 1.4.8 Undefined, implementation-defined or layout-dependent behaviour

1. **Type punning.** `byte` buffers are accessed through `short*` (f_wipe.c:153-154, 200-214) and
   then read as bytes. The buffers are `malloc`ed or shared memory with no declared type, and every
   access is either `short` or `unsigned char`/`memcpy`, so (I) gcc's strict aliasing cannot reorder
   anything observable here. Observed: the genuine file compiled with gcc 15.2 at -O2 behaved as
   the arithmetic predicts (§1.4.9).
2. **Alignment.** `short` access needs `screens[0]`, `[2]` and `[3]` to be 2-aligned.
   - `screens[2]` and `[3]` are `base + 128000` and `base + 192000` of a `malloc` (v_video.c:489-492).
   - `screens[0]` is a `malloc` or `shmat` address.
   - All are aligned in practice. x86 tolerates misalignment anyway.
3. **Endianness.** None. Shorts are only copied, never interpreted.
4. **A pointer formed out of bounds.** When a column reaches `y[i] == height`, line 210 forms
   `&((short*)wipe_scr)[200*160 + i]`, beyond the end of the 32,000-short screen for i ≥ 1 (C89
   6.3.6: undefined to form). It is never dereferenced, because the loop at 212 runs 0 times.
5. **Wall-clock steps.**
   - The wait at d_main.c:333-337 exits on any **nonzero** `tics`, including a negative one if
     `gettimeofday` steps backwards.
   - `wipe_doMelt`'s `while (ticks--)` with a negative `ticks` loops until signed wrap-around.
     Decrementing past `INT_MIN` is undefined; in practice that is about 4·10⁹ iterations, a long
     stall.
   - A forward step finishes the melt in one call.
6. **Zone allocation.**
   - `Z_Malloc` of 64,000 (twice, then freed) and 1,280 bytes (observed sizes; `width*sizeof(int)`
     with 4-byte `int`).
   - It can `I_Error` "Z_Malloc: failed on allocation" (z_zone.c:220).
   - It can purge `PU_CACHE` blocks while searching (z_zone.c:225 onward). That changes zone layout
     but no game outcome.
7. **Start screen contents.** They depend on the environment: zero for a fresh shared-memory
   segment, an earlier session's picture for a reused stale segment of another user
   (i_video.c:641-646), and indeterminate `malloc` bytes without shared memory or with `-2`..`-4`
   (i_video.c:898-913). The palette during the melt is likewise undefined.
8. **Integers.** `M_Random()%16` and `%3` act on 0..255, so there is no negative modulo. `y`
   stays in -15..200. No overflow.
9. **Out-of-range sound id.** Outside the wipe: `sfx_id == NUMSFX` passes the bad-id check
   (s_sound.c:277), an off-by-one that reads past `S_sfx` (not reachable from the playsim's ids;
   noted only).

#### 1.4.9 What was observed (probe)

- Command: in `~/scratch/m5c/c-wipe-mrandom/probe/`, the genuine `f_wipe.c` and `m_random.c`
  were compiled with `gcc -std=gnu89 -O2 -w -I…/linuxdoom-1.10 -DNORMALUNIX -DLINUX` (gcc 15.2.0),
  linked with `stubs.c` and run as `./probe` and `./ycheck`.
- `stubs.c` holds the declared adapters:
  - `Z_Malloc`/`Z_Free` become `malloc`/`free`, with sizes logged;
  - `I_ReadScreen` is i_video.c:527-530's body;
  - `V_DrawBlock` is v_video.c's copy loop without RANGECHECK;
  - `V_MarkRect` is a no-op.
- For each starting index 0..255, the probe calls the genuine `wipe_StartScreen`, `wipe_EndScreen`
  and `wipe_ScreenWipe(wipe_Melt, …)` in D_Display's order, with synthetic start and end patterns.
- Results (probe.log sha256 `73ebf3df…5161847`):
  - Allocation sizes: 64000, 64000 and 1280.
  - The index after the first call equals start+320 mod 256 for 256 of 256 starting indices.
  - The final screen equals the end screen for 256 of 256.
  - One-tick calls to done: 34 to 43 (histogram in §1.4.3 item 11).
  - States at equal cumulative ticks under a pseudo-random grouping of 1..5 ticks per call: 0
    mismatches.
  - `ycheck`, an independent recomputation of y[] from `rndtable`: every y ≥ -15, and max(-y[0..159])
    per start ranges over 6..15.
- File digests (sha256): probe.c `7618047d…dd0e`, stubs.c `dfc2b6c2…3402`, ycheck.c `d1e25305…dd1`.
  These are prototype files for Phase 2 to lift if the genuine wipe goes into the runner.

#### 1.4.10 Questions the `M_Random` and wipe facts raise for the owner

Each question is taken up in [§5](#5-owner-decisions-t1t16-with-recommendations): 1 in T6; 2 in T5; 3 in T14.

1. **The wipe: (a) account for its 320 draws only, or (b) implement the level-start melt?**
   - Facts:
     - Both close the glance gap equally from the first frame on: the draws are identical, and the
       melt's final screen is the frame the port already draws.
     - Under (a) nothing changes in the frame contract. R8 (persistent buffer, a frame after every
       tic, zeroed start) and S14 (screenblocks 10/11) stand. The declared schedule gains one line:
       "+320 draws after the frame of tic 0".
     - (b) adds 34-43 **presentation pictures** between the tic-0 frame and the tic-1 frame. They
       are not tic frames, so either the proven per-tic frame stream gains a new kind of frame (a
       schema choice) or the melt stays unproven presentation.
     - (b) also adds a gap of about 1 to 1.2 s with no tics, which needs a declared clock rule.
       Vanilla drops the time and bursts up to 5 tics; the port could re-base as for S12.
     - (b) needs a declared start screen (zeros, i.e. R8's initial buffer) and palette (S13).
     - Both can be evidenced from genuine source: the runner can compile f_wipe.c whole and call it
       after the first frame. The probe shows it builds alone with four small adapters.
   - **Recommendation: (a) for M5c**, using a new explicit configuration beside the frozen S15
     sound-free, wipe-free profile. Have the runner **run the genuine wipe** (not a bare counter),
     so the 320 draws come from source. Leave (b) for a presentation milestone if the owner wants
     the effect: it changes what is on screen, and the owner has not playtested M5b.
2. **Which first-batch schedule to declare: k = 1 (`-timedemo`/singletics, what S8's
   frame-per-tic already is) or imitate vanilla's clock-dependent k ∈ 1..5?**
   - Facts: §1.4.5 items 3-4. With k = 1, the face values and sound pitches in tics 1..k-1 differ
     from a real-time vanilla session that ran k > 1. From tic k on, everything is identical.
   - **Recommendation: k = 1, declared** ("identical to vanilla under the singletics schedule"),
     because k is wall-clock dependent and S8 already fixed one frame per tic.
3. **Menu hot-key sounds outside the menu** (`-`, `=`, F1-F10, Esc draw at event time; F11 sets
   the palette): (a) the port does not pass those keys, so it has no draws, or (b) models them.
   - Facts: §1.4.2 item 5. Menus are out of M5c's scope.
   - **Recommendation: (a)**, stated in COMPATIBILITY as part of the contract.

#### 1.4.11 What this section did not do or observe

- This section did not run the genuine engine, any X build, any repository tool, `cargo`, or any IWAD.
- This section did not measure k, the melt's real-time cost, or which presented melt states appear on a real
  machine.
- This section did not inspect the Rust code.
- The sound early-return accounting was read in the source, not traced at runtime. The M5b FH1
  measurement (§2.5) is the only runtime evidence of sound draws, and this section did not repeat it.
- The probe exercises f_wipe.c alone with adapters. It is source-equivalent for the melt
  arithmetic and the draws, not the whole D_Display path.
- The OS zero-fill of a new SysV shared-memory segment is general knowledge, not something read in
  this repository.
- Nothing was written inside the repository. Scratch use: 92 KB under ~/scratch/m5c/c-wipe-mrandom/.

## 2. What the simulation gives today

*Written 2026-11-18 (America/Los_Angeles) as a reading task. Repository head
7f7fd99 (branch `feat/m5c-sound`). Everything below was read in the source; nothing was built or run
(section 2.11 says what was not done). "C" means linuxdoom-1.10 in the repository; "Rust" means
`crates/doom-sim` as run by the playable build under wire 24 (`PlayerMonstersEpisode1V2`).*

Terms used here:

- **Sound start**: one call of `S_StartSound (origin, sfx_id)` (s_sound.c:425-431), which calls
  `S_StartSoundAtVolume` (s_sound.c:254-391).
- **Origin**: the first argument. It is a `mobj_t*` (an actor), the address of a sector's `soundorg`
  (a `degenmobj_t`, r_defs.h:88-95, 121), a button's origin, or NULL.
- **Listener**: `players[consoleplayer].mo`, the console player's actor (s_sound.c:302-309, d_main.c:392).
- **Event**: one entry of the ordered list `Simulation::events()` (crates/doom-sim/src/lib.rs:982-985)
  that the simulation fills during one tic (`Event`, lib.rs:394-487).
- **Wire**: the stable policy number of an execution profile (`OriginalExecutionProfile::wire_id`,
  crates/doom-sim/src/original_registry.rs:111-139). The playable build runs wire 24.

### 2.0 Summary

1. Every sound start that wire 24 can reach is already recorded as an event, in the order of the C calls
   as far as reading could check, with the sfx number (the `sfxenum_t` value of sounds.h) and the
   origin's identity: an actor handle, a sector index, a button slot, or "no origin". A player origin and
   a NULL origin can be told apart. One event reports the wrong sound: `A_PlayerScream` emits `sfx_pdiehi`
   below -50 health in every game mode, but the C does so only in commercial mode, and the wire-24 game is
   never commercial (2.2, item 6).
2. `S_StopSound` from `P_RemoveMobj` (p_mobj.c:565-566) has no event of its own. The existing
   `Event::RemovalMarked(id)` is emitted at each admitted `P_RemoveMobj` path, in the right place, and can
   stand in for it (2.2, item 7).
3. **No sound event carries a position.** No event marks the listener's x, y and angle at the moment of a
   call. Sector sound origins (`sector->soundorg`, p_setup.c:555-557) are not stored anywhere in Rust.
   Most positions could be rebuilt by the app from the state before the tic, the state after the tic and
   the ordered events. Some cases cannot be rebuilt without copying simulation internals into the app,
   for example the listener's angle when the player teleports in a tic in which it also turned (2.4).
4. The sound events have **never been compared with the genuine engine for wire 24**. The whole-engine
   comparisons (M3h9c, M4a, M4b) strip `events.*`, and their C stubs make `S_StartSound` a no-op. Only
   the bespoke C scenarios of M3c to M3h9b recorded `S_StartSound` calls and compared them as events
   (2.6).
5. Events are part of the canonical snapshot bytes and digest (`events.last`, snapshot.rs:1272-1279). So
   any new field or event changes frozen bytes unless it is gated to a new wire, or kept out of the
   canonical snapshot (2.5).

### 2.1 Inventory: every C call site against Rust

Command used: `grep -n "S_StartSound\|S_StopSound" linuxdoom-1.10/*.c` (s_sound.c excluded). It finds
161 call lines in 15 files.

**Out of scope for M5c (not part of the simulation)**: f_finale.c (4), hu_stuff.c (2: 566, 568, the chat
beeps), m_menu.c (38), wi_stuff.c (34). These are the finale, the HUD chat, the menus and intermission.
`hu_stuff.c:566-568` runs only in a netgame chat path, so it cannot be reached in a single-player level.

**Simulation call sites** (g_game.c and p_*.c). Event abbreviations:

- `Sound(a)`: `Event::ActorState(ActorStateEvent::Sound { actor: Some(a), sound })` (lifecycle.rs:1252-1258)
- `Sound(-)`: the same with `actor: None`
- `SecSound(S,i)`: `Event::Mover(MoverEvent::SectorSound { origin: SoundOrigin::Sector, index: i, sound })`
  (planes.rs:603-611, 629-636)
- `SecSound(B,k)`: the same with `origin: SoundOrigin::Button`, `index: k` (the button slot)

The "Missing" column means missing for `S_StartSoundAtVolume`. Every row also lacks the listener's x, y
and angle at the call (2.4); that is not repeated in the table.

#### 2.1.1 Reached by wire 24, emitted

| C site | Rust site | Event | Fields carried | Missing |
| --- | --- | --- | --- | --- |
| p_enemy.c:657/660 `A_Look` seesound; NULL for spider/cyborg | perception.rs:833-856 (`global = [19, 21]`: MT_SPIDER, MT_CYBORG) | `Sound(a)` (`Sound(-)` branch unreachable: neither type admitted) | sfx after the `P_Random` choice (posit/bgsit), actor | origin x,y |
| p_enemy.c:728-729 `A_Chase` melee attacksound | monsters.rs:286-310 (called from pursuit/chase.rs:55) | `Sound(a)` | attacksound, actor | origin x,y |
| p_enemy.c:771-775 `A_Chase` activesound (`P_Random () < 3`) | pursuit/chase.rs:67-70 | `Sound(a)` after the move | activesound, actor | origin x,y *after* `P_Move` |
| p_enemy.c:815 `A_PosAttack` pistol (after face and aim) | player_monsters.rs:422-424 (`Hitscan::Pistol`, after aim) | `Sound(a)` | sfx_pistol, actor | origin x,y |
| p_enemy.c:832 `A_SPosAttack`; p_enemy.c:855 `A_CPosAttack` (before face) | player_monsters.rs:406-408 (`kind != Pistol`) | `Sound(a)` | sfx_shotgn, actor | origin x,y |
| p_enemy.c:923 `A_TroopAttack` claw | player_monsters.rs:557-558 | `Sound(a)` | sfx_claw, actor | origin x,y |
| p_enemy.c:988 `A_BruisAttack` claw | player_monsters.rs:609-610 | `Sound(a)` | sfx_claw, actor | origin x,y |
| p_enemy.c:1568 `A_Scream` (1565 NULL for spider/cyborg) | lifecycle.rs:1709-1726 | `Sound(a)` (`Sound(-)` branch unreachable) | deathsound after the `P_Random` choice (podth/bgdth), actor | origin x,y |
| p_enemy.c:1574 `A_XScream` | lifecycle.rs:1700 | `Sound(a)` | sfx_slop, actor | origin x,y |
| p_enemy.c:1580 `A_Pain` | lifecycle.rs:1694-1699 | `Sound(a)` | painsound, actor | origin x,y |
| p_enemy.c:2007 `A_PlayerScream` | lifecycle.rs:1701-1708 | `Sound(a)` | **wrong sfx outside commercial mode** (2.2, item 6), actor | (player origin: no position needed) |
| p_mobj.c:103-104 `P_ExplodeMissile` deathsound | projectiles.rs:182-198 (`missile_sound` at 197, after state entry and draw, as in C) | `Sound(a)` | deathsound, missile | origin x,y (impact point) |
| p_mobj.c:302-307 `P_ZMovement` player hard landing `sfx_oof` | thinkers.rs:422-438 (`has_heights` and `p.mo == Some(id)`; wire 24 has heights) | `Sound(a)` before `Event::Vertical` | sfx_oof (34), player | none (player origin) |
| p_mobj.c:902-903 `P_SpawnMissile` seesound | projectiles.rs:563-608 → `finish_launch` 123-134 | `Sound(a)` after `Spawned` and `Projectile::Launch` | seesound, missile | origin x,y = spawn point (source x,y) |
| p_mobj.c:975-976 `P_SpawnPlayerMissile` seesound (rocket) | projectiles.rs:624-669 → `finish_launch` | `Sound(a)` | sfx_rlaunc, missile | origin x,y (= listener's x,y: players phase) |
| p_inter.c:659-660 `P_TouchSpecialThing` (NULL, after `P_RemoveMobj`) | world_pickups.rs:1146-1164 → `ContactEvent::Sound`, converted at 488-500 | `Sound(-)` after `RemovalMarked(item)` | pickup sfx | nothing (NULL origin) |
| p_map.c:1104 `PTR_UseTraverse` `sfx_noway` | use_lines.rs:386-395 | `Sound(player)` | sfx_noway, player | none |
| p_pspr.c:145-146 `P_BringUpWeapon` sawup | weapons.rs:386-391 (host sound → player_pistol.rs:298-305) | `Sound(player)` | sfx_sawup | none |
| p_pspr.c:296-300 `A_WeaponReady` sawidl | weapons.rs:483-490 | `Sound(player)` | sfx_sawidl | none |
| p_pspr.c:488-489 `A_Punch` (if `linetarget`) | player_pistol.rs:663-668 | `Sound(player)` before the turn | sfx_punch | none |
| p_pspr.c:518-523 `A_Saw` sawful / sawhit | player_pistol.rs:715-727 | `Sound(player)` before the turn | sfx_sawful or sfx_sawhit | none |
| p_pspr.c:651 / 675 / 737 pistol, shotgun, chaingun (sound first) | weapons.rs:531-540 | `Sound(player)` | sfx_pistol or sfx_shotgn | none |
| p_doors.c:83 / 89 `T_VerticalDoor` wait end (normal / close30ThenOpen) | movers.rs:1005-1021 | `SecSound(S,sector)` | sfx_dorcls / sfx_doropn, sector | soundorg x,y |
| p_doors.c:161 door down blocked, reopen | movers.rs:1036-1041 | `SecSound(S,…)` | sfx_doropn | soundorg x,y |
| p_doors.c:309 / 316 / 327 / 337 `EV_DoDoor` (close30ThenOpen; blazeOpen, normal and open only if top ≠ ceiling) | movers.rs:464-470 (after `spawn`, as C after `P_AddThinker`) | `SecSound(S,…)` | sfx_dorcls / sfx_bdopn / sfx_doropn | soundorg x,y |
| p_doors.c:379 / 393 / 406 `EV_VerticalDoor` locked (NULL) | movers.rs:540-550 | `Sound(-)` | sfx_oof | nothing |
| p_doors.c:449 / 453 `EV_VerticalDoor` open | movers.rs:573 | `SecSound(S,…)` | sfx_doropn | soundorg x,y |
| p_plats.c:68 / 77 / 86 / 115 / 126 `T_PlatRaise` | movers.rs:1080-1123 | `SecSound(S,…)` | stnmov (`leveltime & 7`), pstart, pstop | soundorg x,y |
| p_plats.c:194 / 217 `EV_DoPlat` (raiseToNearestAndChange; downWaitUpStay) | movers.rs:655-661 | `SecSound(S,…)` | sfx_stnmov / sfx_pstart | soundorg x,y |
| p_floor.c:219 `T_MoveFloor` stnmov; 250 pstop after `P_RemoveThinker` | movers.rs:1149-1151; 975-980 (after `MoverEvent::Removed`) | `SecSound(S,…)` | sfx_stnmov / sfx_pstop | soundorg x,y |
| p_switch.c:228 / 240 / 252 `P_ChangeSwitchTexture` at `buttonlist->soundorg` | use_lines.rs:58-70, 82-117 | `SecSound(S, buttons[0].sector)`, or `Sound(-)` when slot 0 is free | sfx_swtchn / sfx_swtchx | soundorg x,y |
| p_spec.c:1151 `P_UpdateSpecials` button end at `&buttonlist[i].soundorg` | use_lines.rs:130-145, appended in the Specials phase (demon_expanded_map.rs:1003-1027) | `SecSound(B, slot)` | sfx_swtchn, slot | origin x,y, which is **layout-dependent** (2.8) |
| p_telept.c:112 / 118 `EV_Teleport` fogs | teleport.rs:201-222 | `Spawned(fog)`, `Sound(fog)` twice, **then** `MoverEvent::Teleported` (231-236) | sfx_telept, each fog | fog x,y |

#### 2.1.2 Reached by wire 24 with nothing emitted

- **p_mobj.c:566 `S_StopSound (mobj)` in `P_RemoveMobj`.** No sound event. `Event::RemovalMarked(id)` is
  emitted at the same place on every admitted path (2.2, item 7).
- **s_sound.c:349 `S_StopSound(origin)` inside `S_StartSoundAtVolume`.** This is not a simulation call:
  the sound layer reproduces it from the start event.
- **No other** reachable simulation sound site without an event was found.

#### 2.1.3 Not reachable under wire 24

The source site needs a behaviour that is not admitted. Wire 24 refuses it at load or in the tic,
spawns no such actor, or the condition is never true in its domain.

| C site(s) | Why wire 24 cannot reach it |
| --- | --- |
| p_ceilng.c:75, 91, 118, 128 (ceilings) | No ceiling line special is admitted: `validate_movers_with` returns `PursuitError::Specials` for any line outside its list (mover_rules.rs:466-517). |
| p_doors.c:77, 132 (blazeRaise/blazeClose); 108 (raiseIn5Mins); 301 (`EV_DoDoor` blazeClose); 228, 240, 253 (`EV_DoLockedDoor`, lines 99, 133-137) | Those line specials are not in the admitted list (mover_rules.rs:466-517). `DoorKind` has only Normal, Close30ThenOpen, Open and BlazeOpen (movers.rs:1009-1021). |
| p_plats.c:204, 230, 248 (raiseAndChange, perpetual, blazeDWUS) | Not admitted. The plat kinds are only downWaitUpStay and raiseToNearestAndChange (movers.rs:597-662). |
| p_enemy.c:657, 1565 NULL branches (spider/cyborg); 1098, 1113, 1210, 1235, 1247, 1253, 1323, 1354, 1431, 1761, 1767, 1773, 1839, 1845, 1870, 1922, 1932, 1951 (Doom II and boss monsters, lost soul) | Only types 3004, 9, 65, 3001, 3002 and 58 are admitted (episode1.rs:21), plus the Baron (3003, wire 24 only). Every other monster refuses the map with `MapSpawnError::UnsupportedType` (spawning.rs:520-532). |
| p_enemy.c:1782, 1790, 1803 (super shotgun); p_pspr.c:705 (`A_FireShotgun2`); p_pspr.c:822 (BFG) | Neither weapon is an admitted pickup (episode1.rs:27-32), and `admit_weapon` refuses them (player_pistol.rs:239-249: pistol, fist, shotgun, chaingun, rocket, chainsaw only). |
| p_mobj.c:379, 386 (`P_NightmareRespawn`) | Skill is 2 or 3 only (the wire 24 constructor). |
| p_mobj.c:610 (`P_RespawnSpecials`) | Returns at once unless `deathmatch == 2` (p_mobj.c:591). |
| p_inter.c:194 (`P_GiveWeapon`, netgame branch) | Requires `netgame` (p_inter.c:175-177). |
| g_game.c:886 (`G_CheckSpot`, from `G_DoReborn`) | Netgame or deathmatch respawn only (g_game.c:930-956). Rust refuses any tic after a reborn request (`SimError::PlayerReborn`, lib.rs:1300-1302). |

**Refused in a tic.** Two C paths would make a sound but are refused in Rust with a complete-state
witness. The whole tic, including its sound events, is then not published (2.3, item 6):

- A blocked rising raiseToNearestAndChange plat (`MapScaleError::PlatBlocked`, movers.rs:1086-1088). The C
  would play `sfx_pstart` at p_plats.c:77.
- A blocked first stair step (`MapScaleError::StairBlocked`, movers.rs:1128-1143).

### 2.2 The event types

1. **`ActorStateEvent::Sound { actor: Option<ActorId>, sound: u16 }`** (lifecycle.rs:1252-1258). Its
   doc comment says "None means source boss-global origin". That is stale: `None` is also emitted for
   pickups (world_pickups.rs:498), the locked-door `sfx_oof` (movers.rs:545-548) and a switch sound while
   button slot 0 is free (use_lines.rs:68). All of these are NULL origins in the C (p_inter.c:660,
   p_doors.c:379-406, p_switch.c:228 with a zeroed slot). Canonical bytes: tag 12, sub-tag 3, optional
   handle, `u16` sound (snapshot.rs:1624-1628).
2. **`MoverEvent::SectorSound { origin: SoundOrigin, index: u16, sound: u16 }`** (planes.rs:603-611),
   DSIM tag 26 sub-tag 12 (snapshot.rs:369-378).
   - `SoundOrigin::Sector = 1` means `&sector->soundorg`.
   - `SoundOrigin::Button = 2` means "the address of `buttonlist[index].soundorg`" (planes.rs:629-636).
   - Mover sounds whose sector is unknown would be written with `index = u16::MAX`
     (`mover_sector().unwrap_or(u16::MAX)`, movers.rs:979). No admitted mover without a sector was found.
3. **Other events with "Sound" in the name are not sound starts.**
   - `PerceptionEvent::LookSound` (perception.rs:128-137) records the sector *sound target* read by
     `A_Look` (`P_NoiseAlert` / `soundtarget`).
   - `enum StagedSound` (demon_expanded_map.rs:396-412) stages those per-sector sound targets for the
     weapon noise. It holds no audio data.
   - `noise.rs` is `P_RecursiveSound`. Nothing in these three is audio.
4. **Sound naming.** `sound` is a plain `u16`. It is the index into
   `SOURCE_SOUNDS` (source_actors.rs:223 onward). That table has 109 names starting at `sfx_None`, in
   the order of `sfxenum_t` (sounds.h:179-288, 109 entries before `NUMSFX`). `generate_m3c.py:65` writes it
   from sounds.h. So the number is the C `sfx_id` itself. `source_sound_id` (lifecycle.rs:1786-1791) looks
   names up. Metadata sounds come from `ActorTypeDef::sounds()`: see, attack, pain, death, active
   (lifecycle.rs:92-95).
5. **Origin identity.**
   - **Actor**: `ActorId` = slot + generation (snapshot.rs:24-28). It is unique for the life of the actor,
     and a retired handle never resolves to a later actor. This matches pointer identity in the C, because
     `P_RemoveMobj` stops the actor's channel before its memory can be reused (p_mobj.c:565-569).
   - **Sector**: the sector index. **Button**: the slot number.
   - **NULL**: `actor: None`.
   - **Player origin**: `actor: Some(player mo)`. The app can tell it apart by comparing the handle with
     the player's body (`OriginalPlayer`/`mo`). The C makes the same comparison
     (`origin != players[consoleplayer].mo`, s_sound.c:302).
   - So NULL and player origins can be told apart.
   - A switch sound at `buttonlist->soundorg` while slot 0 is running is emitted as `SecSound(S, sector of
     slot 0)`. That is the same identity as that sector's door and lift sounds in the C (p_switch.c:184
     stores `&line->frontsector->soundorg`). The two kinds therefore share and replace each other's
     channel, which is correct.
6. **A wrong sound: `A_PlayerScream`.** The C plays `sfx_pdiehi` only if `gamemode == commercial` and
   `mo->health < -50` (p_enemy.c:1994-2007). Rust tests the health only (lifecycle.rs:1701-1708). The
   simulation has no game mode at all: `ActorGameMode` is single, coop or deathmatch (lifecycle.rs:172-181).
   The wire-24 genuine runners use `gamemode = registered`, and the playable build takes the mode from the
   IWAD name (shareware, retail or registered; COMPATIBILITY "Known gap" section and its game-mode
   paragraph). So under wire 24 the C always plays `sfx_pldeth`, while Rust emits `sfx_pdiehi` (sounds.h
   enumerator) whenever the player dies with health between -100 and -51.
   - *Inference*: this is reachable. A player death with health in [-100, -51] needs at least 52 damage in
     one `P_DamageMobj`. That can come from a Baron claw (up to 80, p_enemy.c:989), a Baron fireball, a
     barrel or rocket blast (radius damage up to 128), or a rocket hit.
   - The difference changes no state and no RNG, so the state-only whole-engine comparisons could not see
     it. COMPATIBILITY.md:921 records the high scream as unreachable for an *older* profile. It is not
     recorded for wire 24.
7. **`P_RemoveMobj`'s `S_StopSound`.** There is no dedicated event. `Event::RemovalMarked(id)` is pushed at
   the point of removal on every admitted path:
   - state `S_NULL` (lifecycle.rs:1502-1511, from p_mobj.c:63-66);
   - a pickup (`ContactEvent::Removed`, world_pickups.rs:1146-1152, before the pickup sound, as in
     p_inter.c:657-660);
   - a missile removed at the sky (projectiles.rs:274-282 → `remove_missile` → `mark`, from
     p_mobj.c:180-184);
   - a dropped item crunched by `P_ChangeSector` (planes.rs:306, from p_map.c:1284).
   The paths not admitted are p_mobj.c:408 (nightmare respawn) and p_enemy.c:1989 (boss brain).
   `RemovalMarked` therefore marks the C `S_StopSound(mobj)` in the right order. The marked actor stays
   readable until `Event::Retired`, which comes on a later traversal (lib.rs:970-973). That is enough to
   resolve the handle in the same tic.
8. **Stops for doors and plats.** linuxdoom-1.10 has no `S_StopSound` for movers: the only simulation
   `S_StopSound` is p_mobj.c:566. `P_RemoveThinker` of a door, plat or floor stops nothing. A mover's
   "stop" sound (`sfx_pstop`, `sfx_dorcls`, …) is an ordinary start, and Rust emits it as a start.

### 2.3 Order

1. **The per-tic list is built in execution order** under wire 24 (`advance_expanded_demons`,
   demon_expanded_map.rs:751-1034):
   - `Phase::Players`;
   - the `P_PlayerThink` work: command stage, `P_CalcHeight`, `P_PlayerInSpecialSector`, the weapon
     change, the use block, `P_MovePsprites`;
   - `Phase::Thinkers`, then `thinkers::run` in linked order;
   - `Phase::Specials` with the button countdown events;
   - `Phase::Respawn`.

   This matches `P_Ticker` (p_tick.c:146-155) and `P_PlayerThink` (p_user.c:249-334). Every emitter
   pushes into the one plan vector (`plan.events`) as it runs.
2. **Buffered sub-lists that are flushed in order.**
   - **Movement.** One actor's `P_XYMovement` is computed into a `MotionEvent` list and then replayed into
     the event list in order (thinkers.rs:312-369). A pickup inside a move becomes
     `MotionEvent::Contact(…)`, and is converted at its place in the list. The crossing of a special line
     is executed at its place in the list (`MotionEvent::Cross`). Under wire 24 a teleport crossing has
     already run inside the move, and its events are re-inserted in place as `MotionEvent::Outer`
     (movement.rs:588-617; teleport.rs:508).
   - **Button countdown.** It is computed after the plan commit and appended under `Phase::Specials`
     (demon_expanded_map.rs:1003-1027). That is the place of `P_UpdateSpecials` in C.
   - **Pending events.** Events produced between tics by explicit operations (`self.pending`) are put
     *before* the tic's events (demon_expanded_map.rs:1010-1021). In the playable build the only such
     events come from the level's map-spawn batch (`checked_map_spawn_batch`, demon_expanded_map.rs:
     658-697), which makes no sound, so they appear ahead of tic 1's `Phase::Players`.
   - **`StagedSound`** stages noise targets, not sounds (2.2, item 3).
3. **Possible reorderings that were looked for and not found.** For each item below, the Rust order matches the
   C order:
   - the shotgun/chaingun sound before the face turn, the pistol sound after the aim;
   - the explosion sound after the state action (`A_Explode` damage) and the draw;
   - the pickup sound after the removal;
   - `sfx_pstop` after `Removed`;
   - the oof landing sound before `Vertical`;
   - the door start sound after the thinker spawn.
   *Limit*: every emitter listed in 2.1 was read, but this section did not prove by reading that no damage plan, slide
   or crossing anywhere can emit two sounds in the opposite order. M3-era C scenarios checked order for
   their cases (2.6).
4. **Two events come after the sounds they belong with.** Their order is correct as C, but it matters for
   reconstruction.
   - `MoverEvent::Teleported` comes *after* the two fog sounds (teleport.rs:201-236). In the C the thing
     is already at the destination when the fogs sound (p_telept.c:102, then 111-118), and its angle is
     set only afterwards (p_telept.c:124).
   - `Event::Player` (P_MovePlayer ran) is pushed after the thrust and turn (player_commands.rs:224-239).
5. **Can two sounds come out in an order different from the C calls?** Not by any buffering mechanism that
   was found under wire 24. The historical weapon profile (`Profile::PlayerWeapons`) collects its own events
   in `sim.pending` (weapons.rs:915-926, lib.rs:1376-1378), but it is not wire 24.
6. **The refuse path.** A refused tic drops the whole plan and its events (`?` returns before
   `commit_demon_plan`, demon_expanded_map.rs:634-657). `events()` then still holds the *previous* tic's
   list. The app must not replay it, and the app ends the session on any refusal
   (crates/doom-game/src/app/mod.rs:757-759). No partial event list is ever published.

### 2.4 Positions at the moment of the call

What `S_StartSoundAtVolume` reads for a non-player, non-NULL origin:

- `origin->x`, `origin->y`;
- `players[consoleplayer].mo->x`, `->y`, `->angle` (s_sound.c:302-316, 761-793);
- `gamemap` for the map-8 rule (s_sound.c:773-787).

**No sound event carries any of these.** The map number is held by the simulation (`level.map`, DSIM40).

How positions change during a tic (read in the source; this decides what can be rebuilt):

- **The listener's angle** changes in `P_PlayerThink`:
  - the turn in `P_MovePlayer` (p_user.c:154), run only when `reactiontime` is 0 (p_user.c:266-270);
  - `A_Punch` and `A_Saw` face their target after their sound (p_pspr.c:489-494, 523-541);
  - `P_DeathThink` turns after `P_MovePsprites` (p_user.c:187-224);
  - a teleport sets the angle after the fogs (p_telept.c:124).
- **The listener's x,y** change only in the player's own mobj thinker: `P_XYMovement` steps, each a
  `P_TryMove`, and `P_TeleportMove`. `P_PlayerThink` only thrusts (p_user.c:159-164).
- **Any actor's x,y** change only in:
  - its own thinker (`P_XYMovement`, `P_Move` in `A_Chase`, a teleport from its own crossing);
  - at spawn (`P_SpawnMobj`);
  - in `P_CheckMissileSpawn`'s direct half step (p_mobj.c:866-871).

  Damage only adds momentum (`P_ThrustMobj`), and a telefrag does not move its victims (teleport.rs:
  183-189).

  **Correction (2026-11-18):** the list of how an actor's x,y change is incomplete. Damage can switch a monster into its chase state (p_inter.c:911-913), the state change runs its action at once (p_mobj.c:76-77) and `A_Chase` can move the victim (p_enemy.c:764-768), so a monster can move during the attacker's turn; and `A_Look`'s sight sound comes before its own chase step (p_enemy.c:660, 663). See [§4.4.2](#442-option-a-wire-24-events-plus-app-side-reconstruction), items 2 and 3.

The hard cases, in the C order (`P_PlayerThink`, then `P_RunThinkers`, then `P_UpdateSpecials`,
p_tick.c:146-152):

1. **A monster's sight, attack or pain sound**, depending on whether the player's own thinker has moved
   the player yet in this tic.
   - Before the player's mobj thinker, the listener is at the start-of-tic x,y, with the post-turn angle.
     After it, the listener is at the end-of-tic x,y.
   - `Event::Think(player mo)` (thinkers.rs:123) marks that point in the list. So "before or after the
     listener moved" *can* be recovered from the order.
   - The pre-tic x,y are not in the events. The app must copy them before calling `advance`.
2. **A monster that moves and makes a sound in the same tic.**
   - Within its own thinker, movement (`P_XYMovement`, then the action; or `P_Move`, then the active
     sound) precedes every sound it makes, so the origin is at its end-of-tic x,y.
   - A pain or death sound caused by *another* thinker's attack is made at the victim's start-of-tic x,y
     when the victim's thinker has not yet run. The victim then moves later in the same tic with the
     thrust it just got. So the end-of-tic snapshot is wrong for it.
   - The app can recover this from the `Think(victim)` order plus the start-of-tic copy.
3. **A missile's spawn sound.**
   - The sound is made at the spawn point, which equals the source's x,y (p_mobj.c:899-903, 970-976),
     *before* the half step. A missile spawned in the thinker pass also runs its own thinker in the same
     tic (it is appended at the tail), so it has moved again by tic end.
   - The spawn point equals the source's current x,y. `Projectile::Launch` names the source:
     - for a monster, its end-of-tic x,y;
     - for the player's rocket, made in the players phase, the start-of-tic x,y, which equals the
       listener's position, so `sep = NORM_SEP`, s_sound.c:310-316.
   - **Its death sound at the impact point:**
     - an explosion on spawn sounds at the half-step position, which `ProjectileEvent::HalfStep` carries
       (projectiles.rs:163-176);
     - an explosion in flight sounds at the final position (momentum is zeroed, projectiles.rs:184-186);
     - the `P_ZMovement` explosion comes after the XY move.
4. **Teleport fogs.**
   - Each fog is a new actor that never moves, so its x,y at the sound equals its end-of-tic x,y.
     `MoverEvent::Teleported` carries the old position, and the landing allows recomputing fog 2
     (`fog_offset`, teleport.rs:244-251).
   - When **the player** teleports, the listener at the two fog sounds is already at the destination but
     still has the **old angle**. The angle is assigned at p_telept.c:124, after 112 and 118. Rust follows
     this order (teleport.rs:225-230).
   - The old angle is the angle after `P_PlayerThink`. It is in no event and in neither snapshot. It can
     be computed only by redoing `P_MovePlayer`'s turn, and any `A_Punch`/`A_Saw` face turn of that tic.
     The rewritten chainsaw-lunge command of p_user.c:249-255 is also only visible through the pre-tic
     `MF_JUSTATTACKED` flag.
5. **The player's angle in general.**
   - Non-player sounds made during `P_PlayerThink` see the post-`P_MovePlayer` angle:
     - door and switch sounds from the use block (p_user.c:322-331);
     - monster pain from the player's hitscan, made before the face turn.
   - `Event::Player` tells whether `P_MovePlayer` ran (player_commands.rs:234-239), but not the angle.
   - The **dead player's** `P_DeathThink` turn comes after any sound of that tic's `P_PlayerThink`.
6. **Origins spawned and removed within one tic.** None under wire 24 was found that makes a sound.
   - Puffs and blood make none.
   - A missile that explodes on spawn stays at least 1 tic in its death state (p_mobj.c:94-97).
   - A missile removed at the sky has its seesound stopped: `RemovalMarked` stands for the C
     `S_StopSound`.
7. **Sector sound origins are not stored.**
   - Nothing in `doom-sim` holds `soundorg`: the only mentions are doc comments, planes.rs:632-634 and
     use_lines.rs.
   - The C computes it once in `P_GroupLines` (p_setup.c:536-557): `M_ClearBox`, then `M_AddToBox` with
     both vertices of every line whose front or back sector is this one. `M_AddToBox` uses `else if`, so
     a point that lowers `BOXLEFT` never raises `BOXRIGHT` (m_bbox.c:46-58). Then
     `(bbox[BOXRIGHT]+bbox[BOXLEFT])/2` and `(bbox[BOXTOP]+bbox[BOXBOTTOM])/2` (p_setup.c:556-557).
   - The app can compute this from the map's LINEDEFS, SIDEDEFS and VERTEXES. It is a pure function of the
     map, so it can also be put in the simulation's static world without touching any older wire's bytes,
     if it stays out of the snapshot.
8. **Is there an event that marks the listener's move?** Partly.
   - `Event::Think(player mo)` marks the start of the player's thinker.
   - The accepted `Event::SectorMovement { attempt }` events carry each step's target x,y
     (multisector.rs:95-118, snapshot.rs:636 onward).
   - `MoverEvent::Teleported` marks the teleport, but late (2.3, item 4).
   - There is no event with the listener's angle, and none with a mid-tic position snapshot.

**Exactly what is missing for an exact `S_AdjustSoundParams` at each start:**

- (i) the origin's x,y at the call, for actor origins;
- (ii) the sector `soundorg` x,y;
- (iii) the listener's x,y and angle at the call;
- (iv) for a button-end sound, the bytes the C reads as x,y (2.8).

(i) and (iii) can mostly be rebuilt from the start-of-tic copy, the end-of-tic state and the event order,
with the exceptions in cases 4 and 5. (ii) can be computed from the map.

### 2.5 How events leave the simulation, and what is frozen

1. **The API.** doom-game calls `sim.advance([cmd, none, none, none])` once per tic
   (crates/doom-game/src/app/mod.rs:148, 757). It then hands `&sim` to the renderer. The app can read
   `Simulation::events()` (lib.rs:982-985: "Last completed tic's events; never drained by presentation")
   and does so through doom-render today:
   - `switch_inputs` counts `sfx_swtchn` sound events (doom-render/src/state.rs:172-185);
   - `derive_message` looks for a `Sound(-)`/`sfx_oof` before a locked `EV_VerticalDoor` call
     (doom-render/src/hu_stuff.rs:127-160).
   Actor positions and angles at tic end are readable through `Simulation::actor(id)` (lib.rs:970-973;
   marked actors stay readable until retirement). That is what `S_UpdateSounds` needs (s_sound.c:519-600,
   called once per frame at d_main.c:392). Sector origins are static.
2. **Events are canonical bytes.**
   - `Snapshot` writes `events.last.N` and `events.pending.N` fields with `event_bytes`
     (snapshot.rs:1272-1279, 206 onward). These are part of `canonical_bytes` and `digest`
     (snapshot.rs:67-86).
   - `Diagnostic` validates their tags per DSIM version (diagnostic.rs:735-760: tags up to 26 for DSIM
     35-40).
   - The independent DSIM encoders of the M3 generators encode sound events (tag 12/3 and 26/12; see
     2.6).
   - So adding a field to `ActorStateEvent::Sound` or `MoverEvent::SectorSound`, or adding a new event
     to an existing wire, changes canonical bytes and digests of frozen wires. AGENTS.md forbids that.
   - The wire-24 genuine goldens strip `events.*` (2.6), but the canonical snapshot and digest of wire 24
     still contain them.
3. **How policy wires are versioned.**
   - Each wire is an `OriginalExecutionProfile` variant with a fixed `wire_id` (original_registry.rs:
     10-139).
   - Each behaviour has a predicate. For wire 24: `has_episode1_v2`, `has_teleport`, `has_baron`,
     `has_boss_death`, `has_hard_skill`, … (original_registry.rs:396-432), each `matches!(self,
     Self::PlayerMonstersEpisode1V2)`.
   - Wire 23 → 24 added behaviour only behind those predicates, with a new DSIM version (40), a context
     string (`PLAYER-MONSTERS-EPISODE1-2-v1`) and a decoder (`Diagnostic::decode_episode1_v2`).
   - *Observed*: 22 files under crates/tools/fuzz name `PlayerMonstersEpisode1V2` or `has_episode1_v2`.
4. **Where a sound-only wire 25 would attach.**
   - A new variant `PlayerMonstersEpisode1V3` (wire 25) that is a strict superset of 24.
   - New predicates such as `has_sound_positions()` and `has_mode_aware_scream()`.
   - A new event, for example `Event::SoundStart { origin, sound, origin_xy, listener }` under a new DSIM
     tag, or the old events plus a new listener-snapshot event, pushed only when the predicate holds.
   - DSIM version 41, with its independent encoder and decoder, a new context string, and the constructor
     and examples.
   - Wire 24 keeps `Sound`/`SectorSound` byte for byte.
   - Alternative without a wire: a non-canonical, opt-in observation buffer (option (b′) in 2.8).

### 2.6 What has been validated, by evidence

1. **Genuine C, bespoke scenarios (M3c to M3h9b, wires 1-21).** Their C harnesses replace `S_StartSound`
   with an observer that records origin identity and sfx in call order:
   - `void S_StartSound(void* p,int sound){observe(7,p?ident(p):-1,sound);}` in m3d1_shim.h:74,
     m3d3j_shim.h:89, m3f3_shim.h:89, m3c_driver.c:39, and 12 more shims;
   - m3h8_reference.py:257-311 adds `SO` rows for sector and button origins;
   - m3h9b_reference.py imports the M3h9 harness (line 22).
   These rows are projected by independent encoders into expected DSIM event lists. So for those
   scenarios, the sfx number, the origin identity and the order of sound starts among the other events
   were checked against the genuine source bodies.
   - m3f2_shim.h:126 *aborts* on any sound (a scenario that must make none).
   - m3b_shim.h:126 only counts sounds.
2. **Genuine C, whole engine (M3h9c wire 22, M4a wire 23, M4b wire 24, M5a, M5b).**
   - `S_StartSound` and `S_StopSound` are no-op stubs: m3h9c_stubs.c:85-88, m5a_stubs.c:101-104,
     m5b_stubs.c:113-116.
   - The comparators strip `events.*` (level_compare.py:22, 64; level_compare_m4b.py:10-13), and the
     committed fixtures carry no events (m4b_snapshot_goldens.py:31-35;
     crates/doom-sim/tests/episode1v2_mechanism_goldens.rs:16-21).
   - So **no wire-22, -23 or -24 sound event has ever been compared with the genuine engine**. This covers
     everything M4a and M4b admitted: the Episode 1 doors, plats, floors, stairs, donut, locked doors and
     `D3` switches; the teleport fogs; the Baron; the rocket; the chainsaw; the oof landing on real maps.
3. **Rust-only tests.** 15 test files mention sound events. Counted with
   `grep -rcE "ActorStateEvent::Sound|SectorSound|Sound \{"` over `crates/doom-sim/src/*_tests.rs` and
   `crates/doom-sim/tests`. They include use_lines_tests.rs (10 matches), player_episode1_movers_tests.rs
   (4), baron_tests.rs, player_episode1_weapons_tests.rs and teleport_tests.rs. These are Rust-only safety
   and consistency tests, written from cited facts. They are not genuine-engine evidence.
4. **Never validated anywhere:** positions (none are carried), the listener at the call, the button-end
   origin, the `A_PlayerScream` mode rule, and `RemovalMarked` as a stand-in for `S_StopSound`.

### 2.7 THING-loading truncation (for the owner's wire decision)

1. **C.** `P_LoadThings` sets `spawn = false` for the ten Doom II types when `gamemode != commercial`.
   `if (spawn == false) break;` then ends the whole loop (p_setup.c:317-337). The test comes before
   `P_SpawnMapThing`, so it also comes before player starts.
2. **Rust.** The loop is `Simulation::spawn_map_things`, `for thing in self.world.map().things()`
   (spawning.rs:443-462 onward). Type 65 is admitted (episode1.rs:21). The other nine fall through to
   `MapSpawnError::UnsupportedType` (spawning.rs:520-532).
3. **Size of the change** (inference). Under a new-wire predicate (for example `has_thing_truncation()`),
   one test at the top of the loop body: `if truncate && DOOM2_ONLY.contains(&thing.kind) { break; }`.
   - It must come before the start handling at spawning.rs:466-491, as in C.
   - The simulation has no game mode (lifecycle.rs:172-181). The wire would either declare "never
     commercial", which is the only mode an Episode 1 game uses, or take the mode as a constructor input.
     The same input would also fix `A_PlayerScream` (2.2, item 6).
   - `Episode1Bounds::from_things` (episode1.rs:89-150) counts the whole lump. That count stays a valid
     upper bound, so it can be left alone.
   - The `map.*` group lists the lump's THINGS (snapshot.rs:1475) and would stay unchanged.
4. **Evidence it would need.** A synthetic WAD with a type-65 THING, and one of the nine others, placed
   mid-lump before later THINGS (and one case before the player start). Run it through the existing
   wire-24-style whole-engine harness in registered mode, which already truncates (COMPATIBILITY "Known
   gap"). Also add Rust tests showing that wire 24 still spawns everything.

### 2.8 Gaps and options

**Gap list:**

- G1: no origin x,y at the call;
- G2: no sector `soundorg`;
- G3: no listener x, y and angle at the call;
- G4: the button-end origin x,y is layout-dependent and partly not in Rust state (`btexture` is not
  stored: use_lines.rs:30-42);
- G5: `A_PlayerScream` sends the wrong sfx outside commercial mode;
- G6: no genuine-engine check of any wire-22-24 sound event;
- G7: the stale doc comment on `Sound { actor: None }`.

**Options:**

- **(a) The existing events as they are: not sufficient.** G1 to G5 stand.
- **(b) App-side rebuild.** Before each `advance`, the app copies every actor's x,y and the player's
  angle. After it, the app walks the events:
  - an actor origin's x,y is the start-of-tic copy until `Think(actor)`, then the end-of-tic value. For a
    spawn sound it is the source's current x,y (`Launch`); for a spawn explosion it is `HalfStep`;
  - the listener's x,y are the start-of-tic copy until `Think(player)`, then follow each accepted
    `SectorMovement` target and the teleport landing. The landing must be read ahead of the fog sounds,
    because `Teleported` comes after them;
  - the listener's angle is the start-of-tic angle plus the executed turn when `Event::Player` is
    present, then the end angle after the teleport.

  G2 is computed from the map. G5 is rewritten as `pdiehi` → `pldeth` when the mode is not commercial;
  `A_PlayerScream` is the only emitter of `sfx_pdiehi`. G4 needs the next slot's `btimer`, which is
  DSIM-visible, and also `btexture` under ILP32, which is not. The rebuild fails exactly on the
  listener's angle at a player teleport in a tic with a turn, punch or saw face turn (2.4, case 4). It
  also copies `P_PlayerThink` and `P_TeleportMove` details into the presentation.
  **What proves it**: a genuine runner whose `S_StartSound` stub records sfx, origin kind and identity,
  `origin->x/y` and the listener's x, y and angle at the call, compared call for call with the app's
  rebuilt records on synthetic WADs and the local real-map demos.
- **(b′) An opt-in observation buffer with no new wire.** At each emitter, the simulation also pushes a
  record (sfx, origin, origin x,y and listener x, y and angle read from the staged plan at that moment)
  into a buffer:
  - kept outside `events` and outside `Snapshot`;
  - committed and dropped with the plan;
  - filled only when the app turns it on.
  Every canonical byte, digest and DSIM of every wire stays unchanged, because the buffer is not state.
  G2 can be computed at load into the static world, also outside the snapshot. G4 can be filled there
  under a declared ABI. G5 is handled app-side as in (b). Proof: the same genuine `S_StartSound`
  recorder as in (b).
- **(c) Wire 25, sound only.** A strict superset of wire 24 that adds:
  - one new event per sound start: the origin kind (none, player, actor, sector, button), the identity,
    the sfx, the origin x,y at the call, and the listener's x, y and angle at the call;
  - the guarantee that it is pushed at the C call point, so the existing `Sound`/`SectorSound` can stay
    as they are or be replaced only under wire 25;
  - the mode-aware `A_PlayerScream` (with a game-mode input);
  - optionally, the THING truncation (2.7) as the one non-sound change.

  Every older wire, including 24, keeps its bytes. It needs DSIM41, an independent encoder and decoder,
  constructor and examples, and genuine evidence from a sound-recording C runner.
- **(d) A declared limit.** Ship wire 24 events plus a rebuild that refuses or declares the
  teleport-angle case and the button-end origin.

**Recommendation** (the owner decides): **(b′) plus the genuine `S_StartSound` recorder**, with G5
corrected in the app as a declared adapter.

- It gives the sound layer exact mid-tic values straight from the simulation. It does not copy simulation
  internals into the app, and it does not change any frozen byte.
- The evidence lands where the M5c comparison needs it: a call-by-call match against the genuine engine,
  which has never been done for wire 24.

Choose (c) instead if the owner holds that every simulation output feeding a gameplay-visible outcome must
be canonical DSIM. The face glance's `M_Random` depends on how many sounds draw pitch. If the owner
chooses (c), recommendation: carry the THING truncation in the same wire (it is small, 2.7) and nothing
else.

### 2.9 Undefined, implementation-defined or layout-dependent behaviour

1. **The button-end origin (p_spec.c:1151)** passes `(mobj_t *)&buttonlist[i].soundorg`, the address of
   a *pointer field* read as a `degenmobj_t`. `S_AdjustSoundParams` then reads `->x`, `->y` at
   `offsetof(degenmobj_t, x)` = `sizeof(thinker_t)` past that field (r_defs.h:88-95, d_think.h:64-70,
   p_spec.h:227-235). The read happens before the `memset` at p_spec.c:1152, and again in every later
   `S_UpdateSounds`.
   - *Inference by layout arithmetic*, not observed:
     - **ILP32** (`-m32`): x = `buttonlist[i+1].btexture`, y = `buttonlist[i+1].btimer`.
     - **LP64**: x = `buttonlist[i+1].btimer`, y = the padding after it, which is zero.
   - Either way the "position" is a small number of fixed-point units from the map origin (0,0). For
     slot 15 (the last of 16, p_spec.h:244) the read goes past the array.
   - The reference builds default to LP64, with an opt-in `strict32` (m5b_build.py:19-22, 147-151). The
     owner must name the ABI.
2. **`soundorg`** uses signed `int` sums `bbox[BOXRIGHT]+bbox[BOXLEFT]`. In fixed point these overflow
   (undefined) when both bounds pass about ±16,384 map units, and the C89 division of a negative sum
   rounds in an implementation-defined direction (gcc truncates). The app or simulation should
   witness-check both.
3. **`R_PointToAngle2` and `finesine` in `S_AdjustSoundParams`** are covered by §1.1, not
   here.

### 2.10 Questions the simulation inventory raises for the owner

Each question is taken up in [§5](#5-owner-decisions-t1t16-with-recommendations): 1 in T3 (this section's recommendation, the non-canonical buffer, differs from §5's: T3 proposes wire 25 with canonical records); 2 in T3 (this section recommends an app adapter now and the correction in wire 25 if one is opened; T3 corrects it in wire 25); 3 in T4; 4 in T12; 5 is not an owner decision: the stale doc comment is fixed in Phase 2 step 3 with the wire.

1. Positions for sound: **(b′) an opt-in non-canonical observation buffer, or (c) a new wire 25** with a
   canonical sound event? Recommendation: (b′) (2.8).
2. `A_PlayerScream` outside commercial mode: **correct it in the app as a declared adapter (`pdiehi` →
   `pldeth` when the mode is not commercial), or only in a new wire** with a game-mode input? Recommendation:
   the app adapter now, with a COMPATIBILITY note that wire 24's event says `pdiehi`. Also fix it in
   wire 25 if one is opened.
3. If a wire 25 is opened anyway: **should it also carry the THING-loading truncation** (2.7), or should
   it stay sound-only? Recommendation: carry it; it is a small change with a ready harness, and it closes
   a known gap.
4. Button-end sound position: **LP64 (the default reference build) or ILP32 (`strict32`)** as the
   reference layout? Recommendation: LP64, matching the default genuine builds, declared as layout-dependent.
5. The stale doc comment on `ActorStateEvent::Sound` (`None` = "boss-global"): **fix it in the M5c PR**
   (documentation only, no byte change), or leave it? Recommendation: fix it.

### 2.11 What this section did not do or observe

- Nothing was built or run: no cargo, no reference tools, no genuine runner. Every statement above is
  from reading the source. The counts (161 call lines, 109 sfx names, 15 test files, 22 files naming the
  wire) are from `grep`/`awk` over the tree.
- This section did not trace every damage, slide and crossing path to prove that no two sounds can come out of
  order. Each emitter's local order was traced against its C function.
- This section did not check `S_AdjustSoundParams`, the channels, or the timing of `S_UpdateSounds` relative to tics.
  Those belong to §1.1 and §1.3.
- This section did not open any IWAD and recorded no map data. The reachability statements about high-damage player
  deaths are inference from damage formulas, not observed in any run.

## 3. Harness design, evidence first

Two scratch prototypes were built and run in Phase 1: the genuine sound logic compiled whole into a fork of the M5b runner, and the genuine mixer compiled whole with no audio device. Nothing of them is committed yet; Phase 2 steps 1 and 2 lift them.

### 3.1 Harness: the genuine sound logic in the runner

Written 2026-11-18 (America/Los_Angeles). Source authority:
linuxdoom-1.10 at a77dfb96 (repository head 7f7fd99); every `file:line` is in `linuxdoom-1.10/` unless it
names a `tools/` or `tests/` path, and every one was read for this section. Labels: **[src]** read in the
source, **[obs]** observed by running the prototype (command and counts given), **[inf]** inference.

The prototype lives in `~/scratch/m5c/proto-soundlogic/` (ext4) and is **not committed**; nothing in the
repository was edited. It was built from a private export of 7f7fd99 (`src/`). gcc 15.2.0, x86-64 (LP64).
Address runs used `ASAN_OPTIONS=detect_leaks=0`: leak detection was disabled, so they are not
LeakSanitizer coverage. No timing was measured as evidence (the durations quoted are rough wall-clock
notes only, taken while other jobs shared the machine).

#### 3.1.0 Summary

1. **The genuine `s_sound.c`, `i_sound.c` and `f_wipe.c` compile whole into an M5c fork of the M5b runner.**
   `s_sound.c` and `f_wipe.c` need no adapter; `i_sound.c` needs one two-line LP64/glibc ABI adapter for its
   own `extern int errno;` (i_sound.c:166) that does not link against glibc's thread-local `errno`. The
   M5b runner and every frozen file stay untouched (the fork imports `tools/reference/m5b_build.py` read-only).
2. **The shipped build is the sound-server build.** `doomdef.h:84` defines `SNDSERV`, which `i_sound.c`
   sees (it includes `doomdef.h` at i_sound.c:59, before `#ifdef SNDSERV` at :62 and :482). So the shipped
   `I_StartSound` returns **the sfx id** as the handle (i_sound.c:482-489), not `addsfx`'s counter. With
   `I_SoundIsPlaying` = `gametic < handle` (i_sound.c:517-521) a channel then counts as playing only while
   `gametic` is below the sfx number (1..108). The runner offers both conventions (`--handle sndserv`, the
   compiled text; `--handle integrated`, the genuine `addsfx` called exactly as the non-server branch does,
   i_sound.c:490-499), and both were measured (3.1.9).
3. **Sound never changes the game.** With sound and the melt on, the masked game dumps of all 59 synthetic
   items equal the frozen M5b goldens under both handle conventions (59 of 59 each), and on the real runs
   the screenblocks-10 render-every dump sets of all 61 real runs equal the committed M4b strict digests, under both conventions (61 of 61 each).
4. **Nothing but `rndindex`, the face index and the face's pixels changes.** Synthetic: 21,043 frames per
   configuration (M5SHORT's tracker-flagged frames excluded); at screenblocks 10 the face differs in 11,392
   frames, the frame with the face rectangle zeroed differs in **0**; at screenblocks 11 **no** frame differs.
   Real: 158,472 compared frames at screenblocks 10 (the 494 tracker-flagged frames excluded): the face differs in 68,995, the face-zeroed frame in 0; at screenblocks 11, 0 of 158,414 differ.
5. **M5b's experiment is reproduced exactly** on its 48 items: 262 sound draws, 5,882 frames whose face differs
   with sounds alone, 7,931 with the melt alone and 8,517 with both (counting M5SHORT's 136 frames in full, as
   M5b did). Real: 20,697 sound draws (4,670 on E1M8), exactly M5b's; 68,995 face-differing frames of 158,472 compared, against M5b's 69,139 of 158,966 (M5b counted the 494 tracker-flagged frames too; they are excluded here and were not re-counted); DEMO1 540 of 1,704 compared (M5b 542 of 1,710).
6. **`rndindex`, the face and every frame are the same under both handle conventions and both
   `S_UpdateSounds` schedules** (59 of 59 synthetic items; real 61 of 61), because every `M_Random` draw of
   `S_StartSoundAtVolume` comes before `S_getChannel` (s_sound.c:327-346 before 352). The conventions and the
   schedules change only the channel table, which starts reach `I_StartSound` and with which handle. On the
   synthetic items no start was ever refused for lack of a channel; on the real runs 11 starts are refused
   under the server convention and 110 under the integrated one (99 starts change outcome between the two),
   and schedule (B) refuses 15 / 17 more than (A) (3.1.9, 3.1.10).
7. **A layout-dependent origin was found:** `P_UpdateSpecials` starts the switch-return sound with origin
   `&buttonlist[i].soundorg`, the address of the pointer field itself (p_spec.c:1151), so `S_AdjustSoundParams`
   reads `x`/`y` from the bytes that follow inside `buttonlist` (3.1.11 item 6, and "Undefined ...").
8. **The sound logic does not need any sound lump in the server build**, but it needs the music lumps
   (`S_Start` → `S_ChangeMusic` → `W_GetNumForName("d_e1m1")`, s_sound.c:245, 672-676) and, at the first start
   of each sfx, `I_GetSfxLumpNum` → `W_GetNumForName("ds<name>")` (s_sound.c:364-365, i_sound.c:451-456),
   which calls `I_Error` on a missing lump. So the synthetic WAD needs one `DS` lump per sfx the items start
   and the nine `D_E1Mx` lumps (3.1.12).

#### 3.1.1 The I_ layer as seen from s_sound.c [src]

`s_sound.c` calls these `i_sound.c` functions. Under the compiled configuration (`SNDSERV` defined,
`SNDINTR` not, doomdef.h:84-85):

| Function | Genuine body (as compiled) | What s_sound.c sees | Prototype |
| --- | --- | --- | --- |
| `I_StartSound(id, vol, sep, pitch, priority)` (i_sound.c:470-501) | `priority = 0`; if `sndserver` is open, `fprintf` a command; `return id;` (482-489) | the handle = the sfx id | genuine, wrapped (records the arguments and the return); `--handle integrated` returns `addsfx(id, vol, steptable[pitch], sep)` instead, the genuine function called exactly as the non-server branch does (495) |
| `I_SoundIsPlaying(handle)` (517-521) | `return gametic < handle;` | channel "playing" while `gametic < handle` | genuine, wrapped (counts) |
| `I_StopSound(handle)` (505-514) | `handle = 0;` | nothing | genuine, wrapped (counts) |
| `I_UpdateSoundParams(handle, vol, sep, pitch)` (674-688) | assigns 0 to its parameters | nothing | genuine, wrapped (records the arguments per channel) |
| `I_GetSfxLumpNum(sfx)` (451-456) | `sprintf(namebuf, "ds%s", sfx->name); return W_GetNumForName(namebuf);` | the lump number, or `I_Error` | genuine, wrapped (records the sfx id) |
| `I_SetChannels()` (396-424) | fills `steptable` with `pow` and `vol_lookup` | nothing | genuine (called by `S_Init`, s_sound.c:170) |
| `I_SetMusicVolume`, `I_PlaySong`, `I_PauseSong`, `I_ResumeSong`, `I_StopSong`, `I_UnRegisterSong`, `I_RegisterSong` (438-444, 841-881) | trivial; `I_RegisterSong` returns 1 | nothing but `mus_playing` bookkeeping in s_sound.c | genuine |
| `I_InitSound()` (737-825) | server build: tries to `popen` `./sndserver` (740-757); integrated build: opens `/dev/dsp`, exits through `myioctl` if it cannot configure it (159-175, 770-791), then loads every sfx with `getsfx` (799-813) | `S_sfx[i].data` non-NULL only in the integrated build | **not called** (it is reached from `I_Init`, i_system.c:107-110, which the runner never calls). Under `--handle integrated` the driver runs a verbatim copy of its data loop (799-813) with the genuine `getsfx`; under `--handle sndserv` nothing, as shipped |

So every function `s_sound.c` reaches is the genuine compiled text; the only substitution is that
`I_InitSound` is not run (the server `popen` and the device open are platform I/O). In the server build
`S_sfx[].data` stays NULL, so `S_StartSoundAtVolume` takes its "not pre-cached" branch and prints
`S_StartSoundAtVolume: 16bit and not pre-cached - wtf?` on every start that reaches it: the guard is
`#ifndef SNDSRV` (s_sound.c:367-381), a misspelling of `SNDSERV`, so the branch is compiled in both builds
**[src]**; observed: 43 such lines on BARON1 under `--handle sndserv`, 0 under `--handle integrated` **[obs]**.
It writes to stderr only.

Fresh-process defaults **[src]**: `M_LoadDefaults` (d_main.c:1015) sets every default first
(m_misc.c:352-354): `sfx_volume` 8 and `music_volume` 8 (m_misc.c:237-238), `snd_channels` 3 (m_misc.c:282);
a config file read afterwards (m_misc.c:356-401) can override them. `snd_SfxVolume`'s own initialiser 15
(s_sound.c:113) is therefore overwritten before `S_Init (snd_SfxVolume, snd_MusicVolume)` (d_main.c:1107)
stores it again through `S_SetSfxVolume` (s_sound.c:631-639). The runner declares the no-config-file values
8, 8, 3 (options `--sfx-volume`, `--snd-channels` for exploration only).

#### 3.1.2 What the prototype is

Files (all under `~/scratch/m5c/proto-soundlogic/tools/`):

- `m5c_build.py`: imports `m3h9c_build`, `m5a_build`, `m5b_build` read-only from the export; sources =
  `m5b_build.SOURCES` + `s_sound.c i_sound.c f_wipe.c` (49 genuine files); every M5a/M5b adapter applied as
  `m5b_build.adapted_sources` does; the three new files genuine except `SOUND_ABI_ADAPTERS` (two lines of
  i_sound.c: add `#include <errno.h>` before line 27, and replace `    extern int\terrno;` at line 166 with a
  comment; `myioctl`, its only user, is reachable only from `I_InitSound`); variants strict, overflow,
  address (`m3h9c_build.VARIANTS`); links `-lm` (for `I_SetChannels`' `pow`); checks the stub list as M5b.
  Compiler warnings: none in the three new files in any variant.
- `m5c_stubs.c`: `m5b_stubs.c` minus its six sound stubs (`S_Start`, `S_StartSound`, `S_StopSound`,
  `S_PauseSound`, `S_ResumeSound`, `S_ChangeMusic`) plus `I_ReadScreen` (i_video.c:527-530, the genuine
  `memcpy`, which `f_wipe.c` calls at 244 and 256). 48 declared stubs. 22 changed lines against M5b.
- `m5c_driver.c`: `m5b_driver.c` with 67 changed lines, plus `m5c_sound.inc` (304 lines) included before
  the D_Display mirror. Additions: the `M_Random` wrapper counts sound, status-bar, melt and other draws
  separately (other must stay 0); the `Z_Malloc` wrapper captures the one block `S_Init` allocates (the
  channel table, s_sound.c:179-180; the driver checks its size is `numChannels * sizeof(channel_t)`); the
  `ST_Ticker` wrapper records `rndindex` after it; a `Q` frame row (frame digest with the face rectangle
  zeroed, see 3.4); the options `--sound on|off`, `--wipe melt|none`, `--handle sndserv|integrated`,
  `--update-schedule tic|buffer`, `--sfx-data load|none`, `--sfx-volume`, `--snd-channels`, `--sound-out FILE`,
  `--selftest-sound TIC`.
- Init order (d_main.c order): `V_Init`; the M_LoadDefaults values (above); `Z_Init`; `W_InitMultipleFiles`;
  `R_Init`; `P_InitSwitchList`, `P_InitPicAnims`, `R_InitSprites`; [integrated only] the `I_InitSound` data
  loop (`I_Init`, d_main.c:1101); the `D_CheckNetGame` values (d_main.c:1104); **`S_Init (8, 8)`**
  (d_main.c:1107); `HU_Init`; `ST_Init`; the demo. `S_Start` then runs inside the genuine `P_SetupLevel`
  (p_setup.c:607).
- Wrapped symbols (each records and calls the genuine function; every call is cross-object): M5b's 30 plus
  `S_StartSound`, `S_StopSound`, `S_Start` (game callers only: the calls inside `s_sound.c` are not
  redirected, which is what the recording needs), `I_StartSound`, `I_StopSound`, `I_SoundIsPlaying`,
  `I_UpdateSoundParams`, `I_GetSfxLumpNum`, `P_Ticker` (39).
- Classifying each start without touching `s_sound.c`. The wrapper of `S_StartSound` (every game call;
  `S_StartSoundAtVolume` is never called from outside s_sound.c, grep) snapshots the channel table, and,
  when `origin && origin != players[consoleplayer].mo` (the condition of s_sound.c:304), calls the genuine
  global `S_AdjustSoundParams` once as a probe. The probe is pure except that `R_PointToAngle2` stores
  `viewx`/`viewy` (r_main.c:384-385), which the genuine call repeats with the same values immediately after;
  it draws no `M_Random`. The outcome is then: `link_volume` (s_sound.c:283-290), `inaudible` (probe returned
  0, s_sound.c:318-319), `started` (`I_StartSound` was called) or `no_channel` (`S_getChannel` returned -1,
  s_sound.c:352-355). Every start is cross-checked against the source (counted as `mismatch`, observed 0 on
  every run): the number of draws (1 for started or no-channel starts except `sfx_itemup` and `sfx_tink`,
  else 0, s_sound.c:327-346), the volume and separation `I_StartSound` received against the probe's (or 8
  and 128 for local and NULL origins, s_sound.c:321-324; 128 when the origin stands on the listener's x,y,
  s_sound.c:312-316), and that the channel holding the returned handle exists afterwards. Channels stopped
  during a call are found by diffing the snapshot: `same_origin` (the `S_StopSound(origin)` at s_sound.c:349),
  `priority` (the eviction at s_sound.c:853-865), `remove` (game `S_StopSound`, only from `P_RemoveMobj`,
  p_mobj.c:566), `level_start` (`S_Start`, s_sound.c:209-211), `finished` and `inaudible` (`S_UpdateSounds`,
  s_sound.c:600-605 and 592-595).

#### 3.1.3 Where S_UpdateSounds runs [src, then a declared choice]

`D_DoomLoop` calls `S_UpdateSounds (players[consoleplayer].mo)` once per loop iteration, after the tics of
that iteration and before `D_Display` (d_main.c:369-395). With `singletics` (set by `-timedemo`,
g_game.c:1630) an iteration is exactly one tic: `G_Ticker (); gametic++; maketic++;` (d_main.c:383-385),
then `S_UpdateSounds`. In real-time play `TryRunTics` runs one or more tics per iteration (d_main.c:389), so
the call count depends on the frame rate. The runner offers both schedules the Phase 1 reports propose:

- **(A) `--update-schedule tic`** (default): after every tic, after `gametic++`, before the dump and the
  display. This is the singletics loop the frozen runners already mirror (decision S8/FH2 of M5b).
- **(B) `--update-schedule buffer`**: once per "iteration", where the iterations are the groups of tics
  ending at each 512-frame mixer buffer under the buffer rule of §3.2.4 (3): after tic t (0-based) the update runs iff
  some buffer k not yet mixed has `512*k < 315*(t+1)`; iterations {0}, {1}, {2,3}, {4}, {5,6}, ...
  (observed: 464 updates over BARON1's 753 tics, = ceil(753*315/512)).

`S_UpdateSounds` draws no `M_Random` and writes no game state (its only writes outside s_sound.c are
`R_PointToAngle2`'s `viewx`/`viewy`, which `R_SetupFrame` rewrites before any rendering) **[src]**, so both
schedules leave `rndindex`, the dumps and the frames unchanged; measured in 3.1.10.

`gametic` at the level start **[src]**: `gametic` is a zero-initialised global (g_game.c:123) whose only
writes are `gametic++` (d_main.c:384, d_net.c:746). Under `-playdemo`, `-timedemo` (d_main.c:1135-1146) or
`-warp` (`G_InitNew` before `D_DoomLoop`, d_main.c:1164) the level loads during the first `G_Ticker`, or before
it, with `gametic == 0`, exactly as in the runner (load at `gametic` 0, the update after tic t sees
`gametic == t + 1`). A level reached from the title loop or after another level starts with `gametic` equal
to the number of tics the process has run, so under the server convention (handle = sfx id ≤ 108) every
sound is "finished" at its first update once `gametic` exceeds 108, and under the integrated convention the
handle counter (100, 101, ...) carries over from earlier levels and demos. The reference therefore has to
declare "a fresh process, the level loaded at `gametic` 0" as part of its configuration [inf].

#### 3.1.4 The sound stream (schema DOOM-M5C-SOUND-v1)

Written with `--sound-out FILE`, line-buffered, independent of `--render` and of the screen size (the
synthetic streams are byte-identical at screenblocks 10 and 11 on 58 of 59 items; the 59th, M5OPEN, differs
only in the run-to-run-varying text of its `I_Error`, as in M5a) **[obs]**. Rows:

| Row | Columns | Meaning |
| --- | --- | --- |
| `S` | tic, seq, sfx, okind, oid, outcome, draws, rnd, vol, sep, pitch, priority, cnum, handle, samexy, gametic, ox, oy, lx, ly, langle | one per `S_StartSound`, in call order. okind/oid: `none,-1`; `player,<serial>`; `mobj,<serial>` (the thinker serial of the dump's T rows); `sector,<index>`; `button,<slot>` (3.1.11 item 6). outcome as 3.1.2. rnd: the `M_Random` value drawn (-1 if none). vol, sep, pitch, priority: the arguments of `I_StartSound` (inaudible starts: the probe's vol and sep). cnum: the s_sound channel; handle: as returned. gametic at the call; origin x,y (positional origins only) and listener x,y,angle |
| `K` | tic, seq, cnum, sfx, okind, oid, handle, reason, playing | every channel stopped, with the reason (3.1.2) and whether `I_SoundIsPlaying` was true (then `I_StopSound` was called, s_sound.c:717-724) |
| `C` | tic, cnum, sfx, okind, oid, handle, uvol, usep, upitch | the channel table after each `S_UpdateSounds`, with the arguments it passed to `I_UpdateSoundParams` (`-` if none) |
| `L` | tic, sfx | each `I_GetSfxLumpNum` (once per sfx id per process, since `S_Init` sets every `lumpnum` to -1, s_sound.c:190-191) |
| `R` | tic, rnd_after_p_ticker, rnd_after_st_ticker, rnd_after_display, snd_draws, st_draws, wipe_draws | the genuine `rndindex` at three points of every tic, and the cumulative draws |
| `W` | tic, draws, steps | the melt (3.1.8) |
| `E` | tic, message | an `I_Error` |
| `Z` | totals | attempts, outcomes, stops by reason, I_ call counts, max occupied, draws, updates, mismatch, unknown origins |

Size: the 59 synthetic items' streams total 2,084,402 bytes (server convention, schedule A, one
configuration); most of it is the `C` rows (three per tic) **[obs]**.

#### 3.1.5 Proof (C): sound leaves every game dump unchanged [obs]

The S16 record and mask is inherited: the H row prints `(rndindex - draws) & 0xff` where draws now counts
every presentation draw (sound, status bar, melt). Command (all items, 4 jobs):
`python3 tools/m5c_proto_ref.py --jobs 4 --out results/synth.json`. Per item, strict `--render none` with
dumps, sound and melt on:

- the masked dump set equals `tests/reference/m5b_results.json` `dumps_never_sha256` (the frozen M5b goldens,
  which themselves equal the committed M4b/M5a digests): **59 of 59 under `--handle sndserv`, 59 of 59 under
  `--handle integrated`**;
- the strict screenblocks-10 render-every dumps equal the never-render set snapshot for snapshot: 59 of 59;
- with `--sound off --wipe none` the golden rows of the frame stream (F, P, X, M, U, N, O, E, as
  `m5b_reference.normalise`) equal the committed `tests/reference/m5b_frames/<LUMP>.sb<N>.frames`: **59 of 59
  at screenblocks 10 and 59 of 59 at 11**, so the fork keeps the M5b contract exactly.

Synthetic tics: 21,088 status-bar draws in the 57 streams with a Z row (one per tic), 21,043 compared frames
per configuration.
The real runs: 3.4.2.

#### 3.1.6 Proof (D): three variants, determinism, sanitizer findings [obs]

Per item, at screenblocks 10 and 11, the strict, overflow and address builds and a second strict run:

- sound streams identical across the three variants up to the earliest end: 59 of 59 at each screen size;
  identical run to run (strict twice, and integrated twice): 59 of 59;
- frame streams (normalised as M5b, plus the new Q rows of tracker-flagged frames dropped) identical across
  variants and run to run: 59 of 59.
- **Sanitizer findings in s_sound.c, i_sound.c, f_wipe.c: none.** The only reports are M5a's two known
  limit rooms, unchanged: M5OPEN (address: `global-buffer-overflow in R_StoreWallRange`; strict and overflow
  end with the genuine `I_Error` "R_DrawPlanes: opening overflow" at tic 40) and M5SSEG (address:
  `global-buffer-overflow in R_ClipSolidWallSegment`; strict refused at tic 15). These were compared through
  the earliest end. Real runs: 3.4.2.

Endings: 55 marker, 1 exit, M5VPLAN's genuine `I_Error` ("R_FindPlane: no more visplanes"), and the two above.

#### 3.1.7 Proof (E): the sound comparison can fail [obs]

`python3 tools/m5c_proto_ref.py --selftest`: BARON1, screenblocks 10, `--selftest-sound 10` records the
volume of the first started sound at or after tic 10 with its low bit flipped. Against the plain run exactly
one row differs: `S,14,3,32,none,-1,started,...,8,...` became `...,9,...`. Pass. The inherited
`--no-mask-rnd` self-test still applies (the dump set then differs from the golden).

#### 3.1.8 The melt (F) [src, obs]

What the source does: on the first `D_Display` after a level load `gamestate != wipegamestate`
(`wipegamestate` starts `GS_DEMOSCREEN`, d_main.c:188; `G_DoLoadLevel` forces -1 when it was `GS_LEVEL`,
g_game.c:472-473), so `wipe_StartScreen` runs first (d_main.c:222-227) and after `NetUpdate` the melt loop
runs (d_main.c:326-344): `wipe_EndScreen`, then `wipe_ScreenWipe(wipe_Melt, 0, 0, 320, 200, tics)` until done.
Its first call runs `wipe_initMelt` (f_wipe.c:280-286), which draws **320** `M_Random` values: one for `y[0]`
(f_wipe.c:159) and one per column 1..319 (f_wipe.c:160-166), although `wipe_doMelt` uses only 160 columns
(f_wipe.c:186). Under singletics no tic runs during the melt. So the draws come after tic 0's
`ST_Ticker` draw and after any sound draws of tic 0, at the end of the display of tic 0.

What the prototype does: **the genuine `f_wipe.c` compiled whole and driven** by a copy of d_main.c:222-227
and 326-344 in the D_Display mirror (`--wipe melt`, default; `--wipe none` reproduces M5b). `I_GetTime` is
the declared counter stub, so each loop step gets 1 or 2 tics (41 steps observed on every item);
`I_UpdateNoBlit`, `M_Drawer` and `I_FinishUpdate` inside the loop are presentation and omitted. The frame
kept is the screen after the melt, which equals the end screen (`wipe_doMelt` reports done only after every
column has copied all 200 end rows, f_wipe.c:188-222): observed, with `--sound off`, every screenblocks-11
frame and every screenblocks-10 frame outside the face rectangle equals the frozen golden **[obs]**. With
`--render none` the runner still runs the melt at the same point (vanilla always displays), so the sound
stream does not depend on `--render`: the never-render and render-every streams are identical on 56 of 59
items; the three exceptions are the items that end in an `I_Error` or a refusal inside a display (M5OPEN,
M5SSEG, M5VPLAN), whose never-render runs simply run longer **[obs]**. Observed: 320 melt draws on every
item, `W,0,320,41` **[obs]**.

Why the genuine file rather than "bump 320 draws": it costs one stub (`I_ReadScreen`), it keeps any other
effect of the melt visible (its `Z_Malloc`/`Z_Free` of `y`, f_wipe.c:158, 232; the column-major rewrite of
`screens[2]` and `screens[3]`), and it proves the end-screen claim instead of assuming it.

#### 3.1.9 The two handle conventions, measured [obs]

Same items, schedule A, screenblocks 10 (the sound stream does not depend on the screen size):

| Synthetic, 59 items | `--handle sndserv` (shipped) | `--handle integrated` |
| --- | ---: | ---: |
| starts attempted | 602 | 602 |
| started (reached `I_StartSound`) | 573 | 573 |
| inaudible | 29 | 29 |
| refused, no channel | 0 | 0 |
| link volume | 0 | 0 |
| channels stopped: finished / same origin / priority eviction / inaudible in update / removal | 551 / 13 / 1 / 0 / 2 | 455 / 94 / 4 / 3 / 2 |
| `I_StopSound` calls (channel still "playing") | 16 | 103 |
| `I_UpdateSoundParams` calls | 409 | 1,801 |
| tics with 0 / 1 / 2 / 3 occupied channels after the update | 20,407 / 637 / 96 / 39 | 19,023 / 1,187 / 512 / 457 |
| start rows that differ (without cnum and handle) | 0 | |
| tics whose channel table differs | 1,534 | |
| `R` rows (`rndindex` at every point) identical | 59 of 59 items | |
| frames whose face differs between the two | 0 (21,043 compared at each screen size) | |

Real runs (61, 158,966 tics, schedule A; `python3 tools/m5c_proto_real.py --iwad ... --jobs 4`):

| Real, 61 runs | `--handle sndserv` | `--handle integrated` |
| --- | ---: | ---: |
| starts attempted | 26,077 | 26,077 |
| started | 21,189 | 21,090 |
| inaudible (beyond the clipping distance / computed volume 0) | 4,877 (4,319 / 558) | 4,877 |
| refused, no channel | 11 (stnmov 10, posact 1) | 110 (stnmov 47, dmact 24, posact 15, bgact 13, dmpain 4, doropn 3, sgtsit 2, posit1 1, posit2 1) |
| starts that found all three channels busy (eviction + refusal) | 809 | 1,174 |
| channels stopped: finished / same origin / priority eviction / inaudible in update / removal | 20,216 / 122 / 798 / 0 / 0 | 19,682 / 339 / 1,064 / 3 / 2 |
| `I_StopSound` calls / `I_UpdateSoundParams` calls | 144 / 1,852 | 583 / 6,947 |
| tics with 0 / 1 / 2 / 3 occupied channels after the update | 157,235 / 1,192 / 327 / 212 | 154,511 / 991 / 750 / 2,714 |
| starts whose outcome differs between the two | 99 | |
| tics whose channel table differs | 3,772 | |
| `rndindex` rows identical | 61 of 61 runs | |
| face-differing frames against the sound-free run (sb10 / sb11) | 68,995 / 68,976 | 68,995 / 68,976 |

So the convention changes how long channels stay occupied, which channel and handle a start gets, and, on
real maps, whether about 0.4% of the starts reach `I_StartSound` at all (the mixer's input), but never the
draws, `rndindex` or any frame. Under the server convention a
channel survives at most until `gametic` reaches its sfx number; under the integrated one until `gametic`
reaches 100 + the number of earlier starts, i.e. for most of a short synthetic run. Real runs: 3.4.2.

#### 3.1.10 The two S_UpdateSounds schedules, measured [obs]

`python3 tools/m5c_proto_ref.py --ab --jobs 4 --out results/ab_synth.json` (strict, screenblocks 10):

| Synthetic, 59 items | sndserv A | sndserv B | integrated A | integrated B |
| --- | ---: | ---: | ---: | ---: |
| `S_UpdateSounds` calls | 21,088 | 13,002 | 21,088 | 13,002 |
| started | 573 | 573 | 573 | 573 |
| refused, no channel | 0 | 0 | 0 | 0 |
| priority evictions | 1 | 1 | 4 | 4 |
| same-origin stops | 13 | 15 | 94 | 95 |
| finished | 551 | 549 | 455 | 454 |
| start rows that differ A/B (outcome, sfx, origin, vol, sep, pitch, priority) | 0 | | 0 | |
| starts whose cnum or handle differs A/B | 10 (in 3 items) | | 2 (in 2 items) | |
| channel tables that differ on the tics both schedules update | 0 of 13,059 | | 0 of 13,059 | |
| `rndindex` rows identical | 59 of 59 | | 59 of 59 | |
| status rows (palette, face, message, `rndindex`) identical | 59 of 59 | | 59 of 59 | |
| frame digests identical | 58 of 59 | | 58 of 59 | |

The one frame difference is M5SHORT, whose tracker-flagged frames contain the M5a undefined reads (they differ
run to run in the flagged columns whatever the schedule).

Real runs (strict, screenblocks 10):

| Real, 61 runs | sndserv A | sndserv B | integrated A | integrated B |
| --- | ---: | ---: | ---: | ---: |
| `S_UpdateSounds` calls | 158,966 | 97,826 | 158,966 | 97,826 |
| started | 21,189 | 21,174 | 21,090 | 21,073 |
| refused, no channel | 11 | 26 | 110 | 127 |
| priority evictions | 798 | 882 | 1,064 | 1,142 |
| same-origin stops | 122 | 150 | 339 | 372 |
| finished | 20,216 | 20,074 | 19,682 | 19,544 |
| start rows that differ A/B (without cnum, handle) | 15 | | 17 | |
| starts whose cnum or handle differs A/B | 1,783 (43 runs) | | 6,115 (41 runs) | |
| channel tables that differ on the tics both update | 119 of 97,826 (5 runs) | | 23 of 97,826 (4 runs) | |
| `rndindex` rows / status rows identical | 61 / 61 of 61 | | 61 / 61 of 61 | |
| frames identical, tracker-flagged frames excluded | 82,107 of 82,107 compared in the 19 runs whose raw rows differed; the other 42 runs identical in full | | the same | |

The raw frame rows differed between A and B in 19 runs, every one of them a run with tracker-flagged frames;
`tools/m5c_ab_real_frames.py` re-ran those 19 under both conventions and found 0 differing frames among the
82,107 unflagged ones (293 flagged skipped), with the face-zeroed digest also 0. So (B) changes which starts
reach the mixer (15 or 17 of 26,077) and the channel table, never the game, `rndindex` or the pixels. A different cnum or handle without any other change
means: a channel that schedule A had already freed was still occupied under B, so the same start went to
another free channel. Real runs: 3.4.2.

#### 3.1.11 What the synthetic items exercise, and what Phase 2 must add (H)

Measured from the streams of the 59 items (server convention, schedule A) **[obs]**:

1. **Sfx started (35):** pistol, shotgn, sawup, sawidl, sawful, sawhit, rlaunc, firsht, firxpl, pstop,
   doropn, stnmov, swtchn, plpain, dmpain, slop, itemup, wpnup, oof, telept, posit1, bgsit1, bgsit2, sgtsit,
   brssit, sgtatk, claw, pldeth, podth2, sgtdth, brsdth, bgact, dmact, barexp, getpow (ids 1, 2, 10-14, 16,
   17, 19, 20, 22, 23, 25, 26, 31-36, 39-41, 43, 52, 55, 57, 60, 64, 67, 76, 77, 82, 93).
2. **Early returns:** inaudible 29 times, both kinds: 21 beyond the clipping distance (`*vol` left at 8,
   s_sound.c:773-777) and 8 with a computed volume of 0 (s_sound.c:812-817); the link-volume return 0 times
   (unreachable: `sfx_chgun`, the only linked sfx, sounds.c:204, is never started; the chaingun plays
   `sfx_pistol`, p_pspr.c:737 **[src]**); `S_getChannel` returning -1 0 times.
3. **Channels:** at most 3 occupied (the configured count); 1 priority eviction under the server convention
   (4 integrated); same-origin replacement 13 (94); `S_StopSound` on removal 329 calls, of which 2 found a
   playing channel; `level_start` stops 0 (one level per run, `S_Start` at load finds no channel).
4. **Volume and separation:** started volumes 1-8 and, on map 8 (BOSS1, BOSS2), 10, 12 and 15: there the
   attenuation formula is `15 + ((snd_SfxVolume-15)*(CLIP-dist)>>16)/ATT` (s_sound.c:800-808), which with
   `snd_SfxVolume` 8 makes distant sounds **louder** (up to 15) than near ones (8) **[src, obs]**. Separation
   from 33 to 224, both theoretical extremes (`128 - (FixedMul(96<<16, finesine)>>16)`, s_sound.c:793) **[obs]**.
   The same-x,y override (s_sound.c:312-316) 20 times. The exact clipping boundary (`approx_dist ==
   S_CLIPPING_DIST`, kept, s_sound.c:774) and the close-distance boundary (`< S_CLOSE_DIST`, s_sound.c:796)
   were **not** shown to be hit exactly.
5. **Origins:** player 263, NULL 99, mobj 211 (182 started), sector 26, button 3 (M5SWTCH).
6. **The button origin.** `P_UpdateSpecials` calls `S_StartSound((mobj_t *)&buttonlist[i].soundorg,
   sfx_swtchn)` (p_spec.c:1151) — the address of the `soundorg` pointer field, not the sector's `soundorg`
   that the field points to (p_switch.c:184); `memset(&buttonlist[i], 0, ...)` follows (p_spec.c:1152)
   **[src]**. `S_AdjustSoundParams` then reads `x` and `y` at the offsets of `mobj_t.x`/`.y` from that address
   (s_sound.c:767-768): on LP64 (`button_t` 32 bytes, `soundorg` at 24, `thinker_t` 24 bytes) that is the next
   slot's `btimer` and 4 padding bytes; on the 32-bit original (`button_t` 20 bytes, `soundorg` at 16,
   `thinker_t` 12) the next slot's `btexture` and `btimer`; for slot 15 it reads past `buttonlist` (p_spec.h:244,
   p_switch.c:101) into whatever global follows [inf from the declarations, p_spec.h:227-234, d_think.h].
   Either way the "position" is a few fixed-point units from map (0,0). In the integrated convention the
   channel keeps that origin and `S_UpdateSounds` re-reads the bytes every update (76 `C`/`S`/`K` rows on
   M5SWTCH) **[obs]**. Audibility, hence the `M_Random` draw and the face, depends on the distance from the
   listener to map (0,0); the layout difference (sub-unit) can matter only at a volume or audibility boundary.
   The synthetic rooms put the player at (0,0) (M5SWTCH's starts show `samexy` 1), which hides the effect. On the
   61 real runs the button origin was started 4 times, all audible [obs].
7. Not reached at all: `pstart`, `dorcls`, `swtchx`, `popain`, `posit2`, `posit3`, `podth1`, `podth3`,
   `bgdth1`, `bgdth2`, `posact`, `noway`, `punch`, `bdopn`.

**The set the Episode 1 simulation can start** (49 sfx) **[src]**: from `info.c` for the admitted types
(MT_PLAYER: plpain, pldeth; MT_POSSESSED: posit1, pistol, popain, podth1, posact; MT_SHOTGUY: posit2,
popain, podth2, posact; MT_TROOP: bgsit1, popain, bgdth1, bgact; MT_SERGEANT/MT_SHADOWS: sgtsit, sgtatk,
dmpain, sgtdth, dmact; MT_BRUISER: brssit, dmpain, brsdth, dmact; MT_TROOPSHOT/MT_BRUISERSHOT: firsht,
firxpl; MT_ROCKET: rlaunc, and **barexp** as its death sound, not rxplod; MT_BARREL: barexp), the random
variants posit1-3, bgsit1-2 (`A_Look`), podth1-3 and bgdth1-2 (`A_Scream`), the weapons (pistol, shotgn,
punch, sawup, sawidl, sawful, sawhit, rlaunc), pickups (itemup, wpnup, getpow), oof, noway, telept, doors
(doropn, dorcls, bdopn), plats and floors (pstart, pstop, stnmov) and switches (swtchn, swtchx), claw. Not
reachable: `pdiehi` (commercial only, p_enemy.c), `itmbk` (deathmatch respawn), `bdcls` (blazeClose not
admitted), `tink` (netgame chat, hu_stuff.c:568), `chgun`, `rxplod`, `sgcock` (intermission only).
The real runs add 47 of these 49 (only `swtchx` and `bdopn` are started by neither set); real starts by okind: player 5,278, mobj 16,138, sector 3,955, NULL 702, button 4.

Correction (2026-11-18, Phase 2 step 1): the set is 47, not 49. `swtchx` cannot be started in this source (`p_switch.c:211` clears the line's special before line 221 tests it), and `bdopn` is started only by blazing doors, for which wire 24 admits no line special. The committed coverage manifest `tests/reference/m5c_coverage.json` therefore requires 47 sounds and 55 branches and records these two as unreachable with their citations.

**New rooms and scripts for Phase 2 step 1** (estimates in generator lines in the style of `m5b_wad.py`,
and tics):

| Name | What it reaches | Size |
| --- | --- | --- |
| SNDDIST | a stationary noise source (a monster that wakes at once, or a door) on a straight line from the player at distances chosen in fixed point so that `approx_dist` takes the values `S_CLOSE_DIST - 1`, `S_CLOSE_DIST`, `S_CLIPPING_DIST` (audible, vol 0 → inaudible) and `S_CLIPPING_DIST + 1`, and the volume steps 1..8; the player turns through 360° in place so separation sweeps 33..224, including the `angle == listener->angle` wrap of s_sound.c:785-788 | 250 lines, 600 tics |
| SNDBUSY | four or more simultaneous sources (three doors and the player's pistol, monsters firing together) with mixed priorities: all three channels busy, an eviction (`>=` priority, s_sound.c:853-854) and a refusal (`S_getChannel` = -1), a NULL-origin start replacing another NULL-origin channel (`S_StopSound(NULL)` matches it, s_sound.c:478) | 200 lines, 500 tics |
| SNDREMOVE | a sound whose mobj origin is removed while the channel is still occupied (a rocket's launch sound cut by its explosion's removal under the integrated convention; a picked-up item; a gibbed monster): `S_StopSound` from `P_RemoveMobj` stopping a live channel | 150 lines, 300 tics |
| SNDSAW | the chainsaw sequence (sawup, sawidl looping, sawful, sawhit) and pistol bursts: the eight-sfx single-instance set of the mixer (`addsfx`, i_sound.c:285-306) and the ±8 pitch draw (s_sound.c:327-336) | 120 lines, 400 tics |
| SNDMAP8 | an E1M8 item with a source beyond `S_CLIPPING_DIST` (audible only on map 8, clamped, s_sound.c:800-808) and one near | 150 lines, 400 tics |
| SNDPICK | every pickup sound (itemup without a draw, wpnup, getpow) and `noway`, `punch`, `oof` from a locked door | 120 lines, 300 tics |
| SNDMONST | each admitted monster's see/active/pain/death set, so that every random variant (posit1-3, bgsit1-2, podth1-3, bgdth1-2) and popain, posact are started | 250 lines, 1,200 tics |
| SNDMOVERS | pstart/pstop/stnmov plats, dorcls, bdopn blazing door, swtchx exit switch, a button whose return sound plays with the player far from map (0,0) (the button origin, item 6) and one in slot 15 if the layout question is decided for emulation | 200 lines, 600 tics |

About 1,450 generator lines and 4,300 tics in all (±40%). Each new item also needs its frame goldens under

What Phase 2 step 1 built instead (2026-11-18): four rooms and five scripts (SNDDIST, SNDBUSY, SNDMONST, and SNDBTN with the scripts SNDBTN1 and SNDBTN2; 1,599 tics), because the 59 existing items already reach much of the table. Together the 64 items reach all 47 sounds and all 55 required branches. The distance boundaries are hit exactly at the limit and half a map unit either side, which is the finest step that integer thing positions allow, not one fixed-point unit either side. All five scripts match the wire-24 Rust simulation tic for tic (`tools/reference/m5c_rust_check.py`).
the new contract (3.4).

#### 3.1.12 Lumps the runner needs, and the interface with the mixer (I) [src, obs]

- Server build (`--handle sndserv`): no sound data is loaded (`I_InitSound` is not run, and the server
  build's would not load any, i_sound.c:740-757). But `I_GetSfxLumpNum` runs at the first start of each sfx
  (s_sound.c:364-365) and calls `W_GetNumForName("ds<name>")`, which calls `I_Error` on a missing lump (no
  fallback here, unlike `getsfx`'s `dspistol` fallback at i_sound.c:213-216). So the WAD needs a `DS` lump for
  every sfx the item starts; its content is not read.
- Integrated build (`--handle integrated`): the data loop calls `getsfx` for every non-linked sfx 1..107
  (i_sound.c:799-813), which needs `dspistol` and uses it for any missing name; so the WAD needs at least
  `DSPISTOL` and, to be meaningful for the mixer, every sfx.
- Both: `S_Start` changes the music at every level load (s_sound.c:245) and `S_ChangeMusic` looks up
  `d_e1m1`..`d_e1m9` by name (s_sound.c:672-676) and caches the lump; `I_RegisterSong` ignores it
  (i_sound.c:875-881). So nine `D_E1Mx` lumps are needed (content not read; zero length is untested).
- The prototype used `tools/m5c_sfxwad.py`: a synthetic PWAD with 108 `DS` lumps in the original format
  (u16 3, u16 11025, u32 count, unsigned 8-bit samples; our own ramps) and 27 music placeholders. Phase 2
  should take the generator of §3.3 (the mixer needs real sample content) and add the nine music lumps.
- **Shared interface** with the mixer harness: the start list, one record per `I_StartSound` call in call
  order: `tic, seq, gametic, sfx, vol, sep, pitch, priority, handle`, plus the s_sound `cnum` (not needed by
  the mixer). That is exactly the `started` subset of the `S` rows. The mixer also needs the update schedule
  (when `S_UpdateSounds` and the mix run relative to tics: schedule A or B, 3.1.3) and the handle convention,
  because under the integrated build `addsfx` reads `gametic` (i_sound.c:276, 348). Since this runner can
  compile `i_sound.c` whole with the integrated data loaded, it could also call the genuine `I_UpdateSound`
  per mixing step and digest `mixbuffer` itself; this section did not try it (the harness of §3.2).

#### 3.1.13 Design for Phase 2 (J)

- Files: `tools/reference/m5c_build.py` (as the prototype, importing `m5b_build`), `m5c_driver.c` (the fork;
  the include folded in), `m5c_stubs.c`, `m5c_reference.py` (the check), `tools/local/m5c_real.py`, a synthetic
  sound PWAD `tests/reference/m5c_sfx.wad` from the generator of §3.3 plus the music lumps, new rooms in
  `tests/reference/m5c_wads/` from `m5c_wad.py`.
- Contract `tests/reference/m5c_runner_contract.json`, schema `DOOM-M5C-RUNNER-v1`, listing the 49 whole files,
  the two `SOUND_ABI_ADAPTERS`, the declared configuration (sfx volume 8, 3 channels, fresh process,
  `gametic` 0 at load, handle convention, update schedule, melt on) and the 39 wrapped symbols.
- Streams: frames `DOOM-M5C-FRAMES-v1` (the M5b rows, P carrying the genuine `rndindex` with sound and melt,
  plus the Q row), sound `DOOM-M5C-SOUND-v1` (3.1.4). Goldens: `tests/reference/m5c_frames/<LUMP>.sb<N>.frames`
  (about the size of M5b's, 59 + 8 items × 2) and `tests/reference/m5c_sound/<LUMP>.sound` (one per item, the
  screen size does not matter): about 2.1 MB for the 59 items under one convention and schedule; emitting a
  `C` row only when a channel changes would cut that to roughly a quarter [inf], and the Rust comparison does
  not lose anything by it.
- `m5c_reference.py --check`: builds strict, overflow, address; per item: masked dumps equal the frozen M5b
  goldens; `--sound off --wipe none` equals the frozen M5b frame goldens (a standing proof the fork is
  faithful); sound and frame streams identical across variants and run to run; the Q rows equal the
  sound-free run's (nothing but the face changes); the committed sound and frame goldens match; `--selftest`
  plants a sound value and the masks. CI: the Linux genuine job, as M5b's (about the M5b job's run time again
  [inf]). Local only: `tools/local/m5c_real.py` (61 real runs, counts and digests committed).
- The Rust side: per tic, the start list (outcome, sfx, origin identity, vol, sep, pitch, priority, cnum,
  handle), the channel table after each update, and `rndindex` after `P_Ticker`, after `ST_Ticker` and after
  the display, compared with the sound golden; the face/frame goldens as in M5b. The Rust model needs, for
  each start, the origin's x,y and the listener's x,y,angle at the call (now in the `S` row, so a position
  check can be done without the mixer).

### 3.2 Harness: the genuine mixer, headless

Terms used below. A **frame** is one stereo sample pair (left and right, 16 bits each, 4 bytes).
A **buffer** is what one `I_UpdateSound` call mixes and one `I_SubmitSound` call writes. A **start**
is one `I_StartSound` call. **Integrated configuration** means i_sound.c compiled with `SNDSERV`
undefined, so the in-process mixer runs; **shipped configuration** means the source exactly as it
stands, where `SNDSERV` is defined. **[src]** = read in the source, **[obs]** = observed by running
the prototype, **[inf]** = inference.

#### 3.2.1 Which mixer exists: the shipped build does not run i_sound.c's mixer

1. **[src]** The Makefile passes only `-DNORMALUNIX -DLINUX` (Makefile:8), but doomdef.h:84 is
   `#define SNDSERV  1` (doomdef.h:85 is the comment `//#define SNDINTR  1`). doomdef.h's comment
   (doomdef.h:76-83) calls the integrated sound "experimental, and unfinished". i_sound.c includes
   doomdef.h through i_sound.h:26 (i_sound.c:54) and again at i_sound.c:59, so `SNDSERV` is defined
   at every `#ifdef SNDSERV` in i_sound.c (62, 482, 695, 740).
2. **[src]** What the shipped build does instead:
   - `I_InitSound` starts the external server with `popen` if `./sndserver` is executable
     (i_sound.c:740-757). It never opens `/dev/dsp`, never calls `getsfx` and never fills
     `lengths[]` or `S_sfx[].data`.
   - `I_StartSound` writes `"p%2.2x%2.2x%2.2x%2.2x\n"` with id, pitch, vol, sep to the pipe and
     **returns the sfx id as the handle** (i_sound.c:482-489). `addsfx` is never called.
   - D_DoomLoop calls `I_UpdateSound` only under `#ifndef SNDSERV` (d_main.c:396-400), so the mixer
     never runs. It calls `I_SubmitSound` under `#ifndef SNDINTR` (d_main.c:401-405), so every loop
     iteration writes 2048 bytes of the never-mixed, zero `mixbuffer` to `audio_fd`, a zero-initialised
     global (i_sound.c:106), which is file descriptor 0 (standard input) (i_sound.c:669).
   - `I_SetChannels` still builds both tables (i_sound.c:396-424, unconditional).
3. **[obs]** Compiling i_sound.c as it stands (the first probe, without the adapter) leaves
   `access` and `popen` as undefined references of i_sound.o. Both disappear once doomdef.h:84 is
   commented out. Every finding below is therefore about the **integrated configuration**. That is
   the source's own alternative (one define changed), not what the shipped game played.
4. **How the harness selects it:** a single declared configuration adapter, applied as an exact
   one-line text replacement that must match once at its declared line (the `apply_once` rule of
   m3h9c_build.py):
   `doomdef.h:84  "#define SNDSERV  1"  ->  "//#define SNDSERV  1"`.
   No `-D`/`-U` flag can do this, because the define sits inside the header. `SNDINTR` stays
   undefined, as shipped (doomdef.h:85).
5. **[src, read only, nothing built or run] The sndserver's mixer compared with the integrated one.**
   Same: `SAMPLECOUNT` 512 (sndserv/soundsrv.h:38); the same padding expression and the value 128
   (sndserv/wadread.c:248-251); the same `steptable` and `vol_lookup` formulas (soundsrv.c:552-565,
   storing `pow(...)*65536.0` without the explicit `(int)` cast, which is the same conversion); the
   same single-instance list (soundsrv.c:438-452); the same first-free/oldest search
   (soundsrv.c:454-466); the same separation formula, written `/(256*256)` instead of `>>16`, which is
   equal for the non-negative products involved (soundsrv.c:481-498); the same linked-sound expression
   `lengths[(S_sfx[i].link - S_sfx)/sizeof(sfxinfo_t)]` (soundsrv.c:390); the same per-channel
   mixing step, unrolled for 8 channels (soundsrv.c:162-295); the same device setup
   (sndserv/linux.c:79-98); and 2048-byte writes (linux.c:102-107, soundsrv.c:409-416). Different:
   - `channelstart` is `mytime`, a counter incremented once per mixed buffer (soundsrv.c:430,
     478, 629), not `gametic`.
   - Commands arrive asynchronously through the pipe and are applied before the next buffer
     (soundsrv.c:627-726), so which buffer a start lands in depends on process scheduling.
   - The server loads its own IWAD, found by fixed file names under `DOOMWADDIR`
     (soundsrv.c:320-378, grabdata), and ignores the game's `-file` PWADs.
   - Lump lookup is a forward scan (wadread.c:203-207), where w_wad.c scans backwards.
   - A missing lump has **no pistol fallback**: `loadlump` returns 0 and leaves `size`
     unassigned (wadread.c:209-214), so `getsfx` reallocs from an uninitialised size
     (wadread.c:245-249). That is undefined behaviour, for example for Doom II names in a Doom 1 IWAD.
   - Priority is not sent, and the handle never travels back to the game.
   **[inf]** For the same starts landing in the same buffers, from the same lump bytes, with no
   missing lump, the two mixers give the same samples. The exception is the oldest-channel tie rule,
   because channels started in different tics but within one buffer period share a `mytime` and get
   different `gametic` values. "What the shipped sndserver played" is therefore not reproducible tic
   for tic in any case: its start timing is asynchronous.
6. **[src] Consequence for the sound logic (outside this harness, flagged for the s_sound side).**
   The handle returned to `S_StartSoundAtVolume` (s_sound.c:389-394) feeds
   `I_SoundIsPlaying(handle) = gametic < handle` (i_sound.c:517-521), which `S_UpdateSounds` uses to
   free channels (s_sound.c:558-605). That changes `S_getChannel`'s choices, and so the start list
   itself. The configuration therefore decides the start list, not only the samples: handles
   100, 101, ... (integrated, i_sound.c:335-340) against handle = sfx id (shipped). The sound-logic
   runner and this harness **must use the same configuration**. §1.3.0 and §1.1.0 reach the
   same conclusion. No disagreement with them was found.

#### 3.2.2 The harness (prototype): what is genuine and what is substituted

Genuine files compiled **whole**: `i_sound.c`, `w_wad.c`, `z_zone.c`, `sounds.c`, with the headers.
Adapters: the configuration adapter above (every build), and the existing LP64 adapter at
z_zone.c:440 (`(unsigned)block->user` becomes `(uintptr_t)`, copied from m3h9c_build.py; not applied
in strict32). Flags: m3h9c_build.py's `BASE` (`-std=gnu89 -O0 -fwrapv ... -DNORMALUNIX -DLINUX`) and
its variants strict, overflow (`-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow`),
address (`-fsanitize=address,undefined`, run with `ASAN_OPTIONS=detect_leaks=0`, so **leak
detection disabled**; this is not LeakSanitizer coverage) and the optional strict32 (`-m32`,
genuine text with no ABI adapter; a 32-bit toolchain exists on this machine). **[obs]** All five
builds (the four above plus the prototype-only sc315) compile with 0 warnings under the base flags
(no `-Wall`).

**Fake layer: link-time wrapping, not fake functions in headers.** Chosen because the fakes must
catch calls from the unmodified objects, and the project already records `-Wl,--wrap` symbols in
its contracts (m3h9c, m5b input). The symbols:

| Symbol | Called from | Fake behaviour (tools/reference/m5c_mixer_fakesnd.c) |
| --- | --- | --- |
| `open` | i_sound.c:770 (`/dev/dsp`), w_wad.c:163 (WAD files) | `"/dev/dsp"` returns fd 1000 and touches no device. Any other `/dev/` path aborts. Other paths go to the real `open` (WAD reading stays genuine). |
| `ioctl` | `myioctl` (i_sound.c:159-175) at :776-789 | Only fd 1000 is accepted. Records the six requests; `GETFMTS` answers `AFMT_S16_LE`, so the genuine `SETFMT` branch runs (:788-789); unknown requests abort. |
| `write` | `I_SubmitSound` (i_sound.c:669) | fd 1000: the bytes go to the driver's capture. Other fds go to the real `write`. |
| `close` | `I_ShutdownSound` (:725), w_wad.c | fd 1000 is recorded; others go to the real `close`. |
| `sigaction`, `setitimer` | `I_SoundSetTimer` (:941-976), compiled but unreachable without `SNDINTR` | abort |

- **Header.** A fake `m5c_fakesnd/linux/soundcard.h` declares only the seven names i_sound.c uses,
  with the values of the host's UAPI header. **[obs]** A probe program printed the host values,
  identical to the fake ones: SETFRAGMENT 0xc004500a, RESET 0x5000, SPEED 0xc0045002, STEREO
  0xc0045003, GETFMTS 0x8004500b, SETFMT 0xc0045005, AFMT_S16_LE 0x10.
- **Stubs** (m5c_mixer_stubs.c, in the `STUB symbol [kind] file:lines: reason` format):
  - variables: `gametic` (g_game.c:123; the driver sets it), `snd_SfxVolume` and `snd_MusicVolume`
    (s_sound.c:113, 116; written by i_sound.c:434 and 441, never read by the mixer), and `errno`;
  - functions: `I_Error` (records the message and longjmps), `I_ZoneBase` (i_system.c:76-80).
- **`errno`.** i_sound.c:166 declares `extern int errno;` without `<errno.h>`. Under glibc `errno` is
  thread-local, which §1.3.10 observed as a link error. The prototype defines a plain
  `int errno;` stub. It is read only if an ioctl fails, which the fake never does.
- **[obs]** Undefined references of i_sound.o in the integrated build: I_Error S_sfx W_CacheLumpNum
  W_CheckNumForName W_GetNumForName W_LumpLength Z_Free Z_Malloc close errno exit fflush fprintf
  fwrite gametic ioctl memcpy open pow setitimer sigaction snd_MusicVolume snd_SfxVolume sprintf
  stderr write (plus `__stack_chk_fail`).
- **[obs] The genuine device setup, as the fake recorded it:** 1 open, 6 ioctls, fragment
  0x2000b (2 fragments of 2^11 bytes, i_sound.c:775), speed 11025, stereo 1, format 0x10.
- **Lumps come through the genuine w_wad.c and z_zone.c:** `W_InitMultipleFiles` on the `--wad`
  files, then the genuine `getsfx` (`W_CheckNumForName`, `W_LumpLength`, `W_CacheLumpNum`,
  `Z_Malloc`, `Z_Free`).
- **Driver** (m5c_mixer_driver.c), start-up in the genuine order: `Z_Init`, `W_InitMultipleFiles`
  (d_main.c:1018-1021), `I_InitSound` (from `I_Init`, d_main.c:1101 and i_system.c:107-110), then
  `I_SetChannels` (from `S_Init`, d_main.c:1107 and s_sound.c:170). For each tic t: `gametic = t`;
  every start listed for tic t calls the genuine `I_StartSound(id, vol, sep, pitch, prio)` in list
  order (G_Ticker runs with `gametic == t`, d_net.c:745-746); then the schedule (3.2.4) decides
  which genuine `I_UpdateSound(); I_SubmitSound();` pairs run (d_main.c:396-405, integrated). The
  driver only **reads** i_sound.c's globals (`channels`, `channelhandles`, `lengths`, `steptable`,
  `vol_lookup`, `S_sfx[].data`) to report. It never writes them.
- **Output rows** (prototype schema name `DOOM-M5C-MIXER-v0`):
  - `i`: the fake device record;
  - `T`: SHA-256 of each table;
  - `l n len digest`: `lengths[n]` and a digest of the padded data, for all 109 entries;
  - `P`: lumps whose padded length differs from size-8, or that fell back;
  - `s tic id vol sep pitch prio -> handle slot step`: one per start;
  - `g tic id -> lump`: a probe of `I_GetSfxLumpNum`;
  - `b k tic digest16 activemask`: one per buffer;
  - `x tic message`: an `I_Error`;
  - `z nbuf sha256(stream)`.

  `--stream` writes the raw bytes exactly as the fake `write` received them (interleaved
  little-endian signed 16-bit, left first; `mixbuffer` is native shorts, i_sound.c:112, 565-567, and
  the device is set to S16_LE, :788-789).
- **Independent model** (m5c_mixer_model.py, Python). It was written from the citations and shares
  no code with the driver: an exact integer `steptable`, C-truncating division for `vol_lookup`,
  its own WAD reader with the backward scan of w_wad.c:378-389, getsfx's padding, addsfx, the
  mixing loop and both schedules. It prints the same rows (except `i`). This is the independent
  encoder for canonical expectations; the C supplies the genuine trace.

#### 3.2.3 Facts the harness depends on [src]

1. `SAMPLECOUNT` is 512 (i_sound.c:93). One `I_UpdateSound` mixes exactly 512 frames
   (`leftend = mixbuffer + SAMPLECOUNT*step`, step 2, i_sound.c:565-571). `I_SubmitSound` writes
   `SAMPLECOUNT*BUFMUL` = 2048 bytes (i_sound.c:96, 669).
2. A tic is 1/35 s, which is 11025/35 = **315 frames**.
3. Pacing in the integrated configuration **[src + inf]**: one `I_UpdateSound` and one blocking
   `I_SubmitSound` per D_DoomLoop iteration (d_main.c:370-405). The device has 2 fragments of 2048
   bytes (i_sound.c:775-776), and `open` uses `O_WRONLY` without `O_NONBLOCK` (:770). So once the
   queue is full, each write returns after about 512 frames (46.4 ms) have drained. The loop then
   runs at about 21.5 iterations per second, and `TryRunTics` runs the 1 or 2 tics that the wall
   clock has made available (d_net.c:647-677, 715-746). Tics do not pace the audio; the device paces
   the loop. This is a statement about OSS blocking semantics, not observed on a real device.
4. **The mixer feeds nothing back to the game.**
   - `I_StopSound` and `I_UpdateSoundParams` do nothing (i_sound.c:505-514, 674-688).
   - `I_SoundIsPlaying` reads only `gametic` and the handle (:517-521).
   - The handle depends only on the number of calls (:271, 335-340).
   - **[inf]** So the sample stream is a pure function of the start list (tic, id, vol, sep,
     pitch, in call order), the lumps and the schedule. The schedule cannot change gameplay or the
     `M_Random` sequence.
   - **[inf]** Stops are not needed in the interface: `S_StopSound` and `S_StopChannel` never
     silence a mixer channel. A mixer channel ends only when its data runs out (:606-607) or when
     `addsfx` reuses it (:285-305, 330).
5. The comments saying pitch is unused (i_sound.c:143, 342-343, 412-413, 467-468) are wrong.
   `steptable[pitch]` becomes `channelstep` (:495, 344), and the mixer advances by it (:599-603).
6. Only the **relative** order of `channelstart` values matters (`channelstart[i] < oldest` with
   `oldest = gametic`, :276, 309-316). Free channels are skipped, so the mixer is unchanged if every
   tic number is shifted by a constant **[inf]**.

#### 3.2.4 The mixing schedule (the open question; facts per candidate)

The schedules were run on the same start lists through the genuine code. Buffer counts are for
the `--drain` runs (the run continues until every channel is idle).

| | (1) one 512-frame buffer per tic | (2) SAMPLECOUNT 315, one buffer per tic | (3) tic-paced 512-frame buffers (**recommended**) |
| --- | --- | --- | --- |
| What stays genuine | everything | everything except i_sound.c:93 changed from 512 to 315. That changes getsfx's padding (:230) and the mix and write length (:571, 669). | everything (only the configuration adapter) |
| Frames per tic | 512 | 315 | 315 on average: buffer k is mixed after tic ⌊512k/315⌋, so **0 or 1** buffers per tic, never 2 (a tic is shorter than a buffer). Per 512 tics: 315 buffers, 197 tics with none. Pattern from tic 0: 1,1,0,1,1,0,1,0,1,1,0,1,0,1,1,0 |
| Stream against game time | 17,920 frames per game second. Played at 11,025 Hz it falls behind without limit; played at 17,920 Hz the pitch rises 512/315 = 1.625× (+8.4 semitones) and every sound lasts 315/512 of its time. Discarding 197 of every 512 frames would cut sounds. | 11,025 frames per game second | 11,025 frames per game second |
| Start latency, from tic time 315t to the start of the buffer that first contains the sound | 0 frames | 0 frames | 512⌈315t/512⌉ − 315t: every value 0..511 once per 512 tics, mean 255.5 frames (23.2 ms), maximum 511 (46.3 ms) [obs, computed] |
| Compared with the original | wrong rate | a mixing quantum the original never had | the same quantum and the same "starts take effect at the next buffer boundary" as the original's real-time loop, with its wall-clock phase replaced by a declared phase (buffer 0 starts at tic 0's time) and no jitter |
| Starts sharing a buffer | never | never | yes, as in the original when TryRunTics runs 2 tics in one iteration: [obs] in `basic`, a pistol started at tic 10 was replaced by the tic-11 pistol before any buffer was mixed (single-instance rule, i_sound.c:285-305), so it contributed no sample |
| A channel's end tied to tics? | no | no | no: a channel ends at the exact frame where its pointer reaches the padded end (:606-607), mid-buffer |
| Observable side effects | — | **[obs]** padding granularity changes channel occupancy and so the stealing. On `steal`, (3) gave slots `0 1 2 3 4 5 6 7 0 1 1 2` and (2) gave `0 1 2 3 4 5 6 7 0 1 2 3`. The padded lengths change too (pistol 512 → 630, a 1-sample lump 512 → 315). | — |

**[obs]** All three were run on `basic`, `steal`, `single`, `pad` and `clip`. The sc315 build and
the model (with `--frames 315`) agree byte for byte on all five.

**(4) Considered and rejected:**
- Real-time mixing driven by the audio device, as the original. Deterministic only if the device's
  timing were, so it cannot be compared.
- An adapter making the mix length a runtime variable of 315 frames while padding stays at 512. It
  needs a source change at :571 and :669 and gives a quantum the original never had.

**Recommendation: (3)**, declared as decision text tied to tics, in the manner of R8:
> "After the starts of tic t (counted from the first tic the session runs), mix every buffer k not
> yet mixed with 512·k < 315·(t+1), each by one genuine I_UpdateSound and I_SubmitSound. The
> compared signal is the sequence of mixed buffers, not the device output."

The app can play this stream live, because it produces exactly 11,025 frames per 35 tics. It needs
a FIFO of at least one buffer plus device latency **[inf]**. Two refinements need owner decisions
(see Questions):
- **The wipe.** The original does not mix during D_Display's melt loop (d_main.c:326-344; tics do not
  run either, and they catch up afterwards). Proposal: keep M5b's wipe-free contract (S15), so the
  schedule applies only to wipe-free sessions.
- **The phase origin.** Declared as the first tic of the session.

#### 3.2.5 Edge cases, run through the genuine code (all [obs])

Conditions for every run: strict, overflow, address and strict32 builds plus the Python model;
synthetic WADs from 3.3; `--drain`; command `python3 src/tools/reference/m5c_proto_run.py <names>`
in ~/scratch/m5c/proto-mixer. 21 scenarios. In 18 the four builds and the model were byte-identical
(rows other than `i`). The exceptions:
- the three short-lump scenarios: layout-dependent, item 3;
- `oob_pitch`: outside the domain s_sound can reach, item 12.

| # | Scenario (start file) | Result |
| --- | --- | --- |
| 1 | Padding (`pad`): lumps of 1, 511, 512, 513 and 1024 samples | lengths 512, 512, 512, 1024, 1024. Every pad byte is 128 (`P` rows count the 128s; the model's padding is 128 and its digests match), and 128 mixes to 0 (`vol_lookup[i*256+128]` = 0, :423). Padding is silent but keeps the channel busy: a 1-sample sound occupies its channel for 512 frames at pitch 128. |
| 2 | Exact multiple (`boundary`, `basic`) | A 512-sample sound started at tic 0 ends on the last frame of buffer 0 (`activemask` 00 after `b 0`). The tic-1 start reuses slot 0. A 513-sample sound started at tic 3 keeps its channel to the end of the second buffer (mask 01 after `b 2`, 00 after `b 3`). |
| 3 | Lumps of 0, 5 and 8 bytes (`short0`, `short5`, `short8`) | Init accepts them: `paddedsize` 0, `lengths` 0, an 8-byte zone block (:230-247). A start reads `*channels` = `paddedsfx[8]`, **one byte past the block** (:591 before the end test at :606), mixes one frame and frees the channel. The byte is whatever zone memory follows. First frame: strict, overflow and address gave (−15319, −15480) for the 0- and 5-byte lumps and (−14553, −14706) for the 8-byte one; strict32 gave (−18383, −18577) and (−17617, −17803). **AddressSanitizer reported nothing**, because the read stays inside the zone's single `malloc` (z_zone.c Z_Init). Layout-dependent, so a refusal witness (see Questions). |
| 4 | Zero-sample lump (8-byte header only, `short8`) | as row 3 |
| 5 | Missing lump (`missing`, dspstart absent) | getsfx substitutes `dspistol` (:213-216) in its own allocation with the pistol's length. Starting sfx 18 plays the pistol's samples (the same buffer digest as a pistol start). `I_GetSfxLumpNum` for the same sfx (the s_sound path, s_sound.c:359-360) ends in `I_Error "W_GetNumForName: dspstart not found!"` (i_sound.c:451-456, w_wad.c:399-409). So a session that starts a missing sound dies in the genuine game; the fallback only matters at init. |
| 6 | Linked sound (`chgun`, id 86) | `lengths[86]` = 0: `(S_sfx[86].link - S_sfx)` = 1 element, divided by `sizeof(sfxinfo_t)` (48 under LP64, 36 under ILP32) is 0, and `lengths[0]` is a zero-initialised global the init loop never writes (loop from 1, :799; declaration :103). [obs] `l 86 0` in all builds, strict32 included. One start mixes **exactly one frame**, the pistol's first sample, in bounds (`data` = the pistol's data, :810), at step 83168 (pitch 150), then frees the channel. The `dschgun` lump in the edge WAD is never read. No game code starts sfx_chgun: `grep sfx_chgun` matches only sounds.h and sounds.c. |
| 7 | Single-instance sounds (`single`) | ids: `sawup` 10, `sawidl` 11, `sawful` 12, `sawhit` 13, `stnmov` 22, `pistol` 1 (:285-290). A second sawful while the first plays: the first is cleared, the new one takes the first free slot (0). Two sawup starts in one tic: both in slot 0, so only the second is heard. Two `oof` (34, not single-instance): two channels. |
| 8 | More than eight (`nine`, `steal`) | The search loop tests `i<NUM_CHANNELS` before `channels[i]` (:309), so it **cannot read out of bounds**, and ASan reported nothing. Ten starts in one tic: slots 0..7, then 0 and 0 (no `channelstart` is below the current `gametic`, so `oldestnum` stays 0, :322-323). `steal`: all busy at tic 6 takes the oldest, slot 1 (start 0, lowest index among the ties). At tic 7, after slot 1 freed, the next start takes slot 1 and the one after takes slot 2. |
| 9 | Volume (`volsep`) | 0 (silent, but the channel is occupied), 15, 127. 128 at separation 128 is accepted (left 96, right 96). |
| 10 | Separation (`volsep`, `err_*`, `ok_vol15_sep257`) | At volume 127: 0 gives left 127, right 0; 1 gives left 127, right 1; 128 gives left 95, right 96 (asymmetric); 255 and 256 give left 0, right 127; −1 gives left 127, right 0. 257 → `I_Error "leftvol out of bounds"`; −2 → `"rightvol out of bounds"`; volume 128 at separation 0 → leftvol; at 255 → rightvol. The range check depends on the volume: volume 15 at separation 257 passes. Order: rightvol is checked first (:364-368). s_sound passes separations of 128 or `128 - (FixedMul(96<<16, finesine)>>16)` (s_sound.c:315, 323, 793) and volumes ≤ `snd_SfxVolume` ≤ 127 (s_sound.c:292-293, 634); the menu range is 0..15 (m_menu.c:821-826), the default 8 (m_misc.c:237). |
| 11 | Pitch 0, 128, 255 (`volsep`) | steps 16384, 65536, 259320 (one sample per 4 frames, per frame, ≈3.96 per frame). |
| 12 | Pitch 256 (`oob_pitch`; s_sound clamps to 0..255, s_sound.c:332-335, 342-345, so unreachable) | address: **UBSan "index 256 out of bounds for type 'int [256]'" at i_sound.c:495:36**. strict, overflow and strict32 read the following global (here a step of 0), so the channel never advances and the run filled 1 << 20 tics (645,120 buffers; the 1.3 GB streams were deleted). Recorded as out of domain. |
| 13 | Clipping (`clip`): 8 channels of all-255 and all-0 lumps at volume 127 | The sums clamp to 0x7fff and −0x8000 (:617-630). Model identical. |
| 14 | Start on the tic another ends | Not a tic event in the mixer: a channel is free at a start only if it ran out in an earlier buffer (`boundary`: the 512-sample sound ended in buffer 0, and the tic-1 start reused slot 0). |
| 15 | Handle wrap (`wrap`, 65,540 starts in tic 0, strict and address only) | handles 100 … 65535, then 100 again (unsigned short wraps to 0, reset to 100, :335-336). Model identical; ASan nothing. |
| 16 | Header fields (`hdr`) | A lump whose header says 22,050 Hz and one with a wrong header count play their `size-8` bytes exactly like any other: getsfx reads no header field (:218-250). |

**Sanitizer findings, complete list:** one, i_sound.c:495:36 (row 12, out of s_sound's domain).
The overflow build found nothing in any scenario. ASan/UBSan found nothing in-domain. ASan cannot
see row 3's read.

**Streams identical across variants:** yes for every in-domain scenario. The exception is row 3,
where strict32 differs from the LP64 builds because the byte read past the block belongs to the
neighbouring zone block header, whose layout differs between ABIs.

#### 3.2.6 The tables [obs]

- **Digests** (SHA-256 of the little-endian int32 arrays; identical in strict, overflow, address
  and strict32, and equal to the Python model's exact computation):
  - `steptable` (256 entries): `95f2209c952180e797cf3ef5a836db333d8ca88fc3b1c7893f136272f4e96dde`
  - `vol_lookup` (32,768 entries): `f727268ad58a2714746072664e37d5c8425d1c21d83a70eabe9e2098836a0043`
- **`steptable` against libm.** Each value is `(int)(pow(2.0, i/64.0)*65536.0)` (i_sound.c:404, 414-415).
  - The multiplication by 65536 is exact and the values are positive, so the entry is
    ⌊2^(16+i/64)⌋.
  - Four entries are exact powers of two (i = −128, −64, 0, 64 → 16384, 32768, 65536, 131072); any
    libm returns `pow(2, k)` exactly for integer k.
  - For the other 252 entries, the minimum distance of 2^(i/64)·65536 to an integer is **0.01253**
    (at i = −16, value 55108.987…; computed with 60-digit decimals). That is about 1.7·10^9 ulps of
    the double, so no libm with a sane error bound can change a single entry.
  - The model computes the table as the integer 64th root of 2^(1024+i), with no floating point, and
    matches the genuine dump.
  - Min/max 16384/259320.
- **`vol_lookup`** is integer arithmetic `(i*(j-128)*256)/127` (:423), range −32768..32512. Negative
  dividends rely on truncation toward zero, which is implementation-defined in C89 (GCC truncates;
  so does Rust's `/`).
- **Recommendation:** Rust computes both tables with integer-exact arithmetic (the integer root
  for `steptable`, the truncating division for `vol_lookup`) and tests them against these two
  committed digests. That gives identical tables on all three operating systems without depending
  on any libm.

#### 3.2.7 Proof the comparison can fail, and determinism [obs]

- **Run to run:** `basic` twice gave identical rows and raw streams (`cmp`). `steal`, `clip` and
  `volsep` reruns were identical to their first runs.
- **One flipped bit:** `--perturb-byte 5000` (bit 0 of stream byte 5000) changed exactly row
  `b 2` (digest `1053e59d…` → `a20b2f9f…`) and the `z` stream digest.
- **A start moved:** moving one start from tic 3 to tic 4 changed `b 2`, `b 3` and `z`.
- **A start changed but not heard:** changing the tic-10 pistol's volume from 127 to 126 changed
  **only its `s` row**; the stream is identical, because that start is replaced before any mixing.
  The golden must therefore carry the start echo (`s` rows with handle and slot), not only buffer
  digests.

#### 3.2.8 Real IWADs, local only (counts only) [obs]

Copies of DOOM.WAD and DOOM2.WAD from /mnt/d/DoomIWADs were placed in
~/scratch/m5c/proto-mixer/iwad. **That directory was deleted after the runs** (`rm -rf` by absolute
path), together with every output derived from it. DOOM.WAD contains E4 maps (the Ultimate
edition). No shareware IWAD is available.

| | DOOM.WAD | DOOM2.WAD |
| --- | --- | --- |
| Sfx names I_InitSound loads (non-linked, 1..108) | 107 | 107 |
| Present | 67 | 107 |
| Missing (fall back to dspistol at init) | 40 | 0 |
| Present lumps whose size−8 is not a multiple of 512 | 67 of 67 | 107 of 107 |
| Header rate other than 11,025 Hz (all 22,050) | 1 | 4 |
| Header format other than 3 | 0 | 0 |
| Header count ≠ size−8 | 0 | 0 |
| Lumps of 8 bytes or fewer | 0 | 0 |
| dschgun lump | absent | absent |
| One start of every id 1..108 (20 tics apart), all four builds | identical streams (1341 buffers), model identical, no sanitizer line | identical (1346 buffers), model identical, no sanitizer line |

- **[src + inf]** The genuine code ignores the header rate. A 22,050 Hz lump is played one byte per
  frame at 11,025 Hz (step 65536 at pitch 128): an octave low and twice as long, in the integrated
  configuration as in the sndserver.
- No lump would be mishandled beyond that.
- The 40 Doom II names missing from DOOM.WAD are harmless at init. Starting one would fail in
  `I_GetSfxLumpNum` (row 5); whether Doom 1 gameplay can start one belongs to the sound-logic
  side. Note (2026-11-18): none of the 43 sounds named at the Episode 1 call sites of §1.2 is among them (a local check that day).

#### 3.2.9 Design for Phase 2 step 2

- **Files** (tools/reference/), lifted from ~/scratch/m5c/proto-mixer/proto/:
  - `m5c_mixer_build.py`: build script; contract `tests/reference/m5c_mixer_contract.json`, schema
    `DOOM-M5C-MIXER-v1`, with `--check`.
  - `m5c_mixer_driver.c`, `m5c_mixer_stubs.c`, `m5c_mixer_fakesnd.c`,
    `m5c_fakesnd/linux/soundcard.h`.
  - `m5c_mixer_model.py`: the independent encoder.
  - `m5c_mixer_reference.py`: replaces the prototype's `m5c_proto_run.py`; `--work DIR --check
    [--selftest] [--strict32] [--jobs N]`.
  - `m5c_sfx.py`: the generator, see 3.3.
- **Contract fields:** as in m5b_input_contract.json: `whole_files`, `source_sha256`,
  `header_sha256`, `config_adapters` (the doomdef.h line, new), `abi_adapters`, `flags`, `wrapped`
  (the six symbols), `fake_device` (behaviour list), `stubs`, `driver_behaviour`, `schedule` (the
  declared rule text), and `golden_row_kinds`.
- **Goldens** (tests/reference/m5c_mixer/): per scenario a text file of `T`, `l`, `P`, `s`, `b`,
  `x` and `z` rows, plus `m5c_mixer_results.json`. **Commit digests, not raw samples.**
  - A buffer row is about 30 bytes, so a minute of game (35·60·315/512 = 1,292 buffers) is about
    40 KB of rows.
  - Raw would be 2,048 bytes per buffer, 2.6 MB per minute.
  - The 21 prototype scenarios total about 22 KB of rows (the 109 `l` rows dominate; they could
    become a single `L` digest after the first file).
- **`--check` semantics:**
  - the genuine C in strict, overflow and address must be byte-identical, with no sanitizer line;
  - it must equal the model and the committed goldens;
  - the short-lump scenarios are compared as declared witnesses (genuine C reads past the block;
    the model and Rust refuse the start), never as equal streams;
  - `--selftest` plants a flipped stream bit, a moved start and a changed volume, and requires
    each to be reported at its row (the volume case at its `s` row).
- **CI placement:**
  - genuine C runs, model comparison, contract and generator `--check`: Linux job only, beside the
    M5b C steps;
  - the model and generator are Python with no compiler and could run on every OS;
  - Rust: `cargo test` on Linux, Windows and macOS compares its mixer with the committed goldens
    (table digests, `s` rows, buffer digests, `z`).
- **Interface shared with the sound-logic runner** (proposed `DOOM-M5C-STARTS-v1`):
  - a header line naming the configuration (`integrated`);
  - one line per `I_StartSound` call, in call order: `s TIC ID VOL SEP PITCH PRIO HANDLE`, where TIC
    is `gametic` at the call;
  - the mixer driver replays it and must reproduce each HANDLE (a cross-check that both used the
    integrated handle sequence);
  - stops and parameter updates are not part of it (3.2.3 item 4).

  The sound-logic runner of §3.1 had not reported when this section was finished, so its output could not be compared.
- **The edge cases needing a Rust refusal witness:** a start of a non-linked sfx whose padded length
  is 0 (lump of 8 bytes or fewer). Pitch outside 0..255 is unreachable from s_sound and should be
  refused at the mixer API.

### 3.3 Synthetic sound lumps

1. **[src] What the source reads.** getsfx: the whole lump; bytes 8.. are unsigned 8-bit samples;
   **no header field is read** (i_sound.c:218-250). I_InitSound loads every sfx 1..108 that has no
   link (i_sound.c:799-813): **107 names**, because only `chgun` is linked (sounds.c:204; sfx_None at
   index 0 is skipped). A missing name falls back to `dspistol` (:213-216); if `dspistol` is missing
   too, `W_GetNumForName` calls `I_Error` (w_wad.c:399-409).
2. **[src] The s_sound path needs each started name present** (s_sound.c:359-360 → i_sound.c:451-456).
   The synthetic WAD therefore contains all 107 names and does not rely on the fallback (the edge WAD
   tests the fallback on purpose).
3. **Conventional layout, written but not read by this source:** u16 format 3, u16 sample rate
   11025, u32 sample count, samples.
4. **Generator** `tools/reference/m5c_sfx.py`, modelled on m5b_gfx.py: deterministic, no id audio,
   integer arithmetic only. It parses the sfx names from linuxdoom-1.10/sounds.c and supports
   `--out DIR`, `--list` and `--check DIR`.
   - **main set, m5c_sfx.wad: 107 lumps, 38,969 samples, 41,549 bytes, sha256
     `c228e186aceb05529187755be20f24321b7a46d2ba59a2a18e39b587e31d9699`.**
     - Each sfx n has length 40 + (389·n mod 700) samples, except pistol 512 and shotgn 1024 (exact
       buffers).
     - Each is a triangle wave of period 6 + (n mod 23) and amplitude 20 + (37·n mod 100) around 128.
     - Samples 0 and 1 are the signature n and 255−n, so a wrong-sound bug changes the stream.
   - **edge set, m5c_sfx_edge.wad: 107 lumps, 41,707 bytes, sha256
     `8c4d001f5807098d3025f1c21d0f5b4f75e9c0fecfb9194119d893c6cd5b9e10`.** The main set with:
     - lumps of 0 bytes (dbopn), 5 bytes (dbcls) and 8 bytes, header only (dbload);
     - lumps of 1 (plasma), 511 (bfg), 512 (sawup), 513 (sawidl) and 1024 (sawful) samples;
     - 600 × 255 (rlaunc) and 600 × 0 (rxplod) for clipping;
     - header rate 22,050 (firsht) and a wrong header count (firxpl);
     - pstart missing;
     - a dschgun lump that the link must ignore.

     The names are Doom II or weapon names, so the edge cases never replace a sound the Episode 1
     synthetic sessions need. The edge WAD is a separate full set, because a PWAD cannot delete a
     name.
   - **[obs]** `--check` against the written files matches; a second generation is byte-identical.
5. **Committed sizes (estimate):** the two WADs, about 83 KB; mixer goldens, about 22 KB for the
   prototype scenarios, plus about 40 KB per minute of any session golden. No raw samples. Nothing
   derived from IWAD audio is committed: the local run in 3.2.8 publishes counts only.

#### 3.3.1 Undefined, implementation-defined or layout-dependent behaviour

1. **Lumps of 0 to 8 bytes.** One byte is read past the zone block, then one frame is mixed. The
   value depends on the zone layout and differs between LP64 and ILP32 ([obs] table row 3). ASan
   cannot see it.
2. **Pitch outside 0..255.** `steptable` is indexed out of bounds (i_sound.c:495); [obs] UBSan
   reports it. Unreachable through s_sound.
3. **`vol_lookup`'s negative division:** implementation-defined in C89. GCC truncates toward zero.
4. **The linked-sound index** is `sizeof`-dependent but 0 on every ABI: defined behaviour, the wrong
   intent.
5. **The ioctl command argument.** It passes through `int command` (i_sound.c:162); the 0xc004…
   values are converted from unsigned to int (implementation-defined; harmless, the fake records
   the low 32 bits).
6. **Native-endian `mixbuffer`.** The stream is little-endian on the x86 builds used here.
7. **The tic-to-buffer relation in the original** is a property of real-time scheduling (wall
   clock, device drain, draw time), not of the game; schedule (3) replaces it by a declaration.
8. **The shipped configuration's `write(0, ...)` of zeros** every loop iteration (i_sound.c:669 with
   `audio_fd` 0): not reproduced and not needed.
9. **The sndserver's missing-lump path** realloc's from an uninitialised size (wadread.c:209-249).
   Read only; never run.

#### 3.3.2 Questions the mixer harness and sound lump facts raise for the owner

Each question is taken up in [§5](#5-owner-decisions-t1t16-with-recommendations): 1 in T2; 2 in T5; 3 in T11; 4 in T6 (this section's recommendation, keeping the wipe-free contract, differs from §5's: T6 recommends counting the wipe's 320 draws because S15 does not close without them); 5 in T10.

1. **Reference configuration:** integrated (doomdef.h:84 commented out, the only adapter; i_sound.c's
   mixer and handles 100, 101, …) *or* shipped (SNDSERV: the handle is the sfx id, no in-process
   mixer, audio from an asynchronous external process that cannot be compared tic for tic).
   **Recommend integrated**, declared everywhere M5c claims fidelity ("identical to linuxdoom-1.10
   built with its integrated sound configuration"), and used by both the sound-logic runner and the
   mixer.
2. **Mixing schedule:** (3) tic-paced 512-frame buffers (no source change; buffer k after tic
   ⌊512k/315⌋) *or* (2) SAMPLECOUNT 315 (changes padding and stealing, [obs]) *or* (1) one buffer per
   tic (wrong rate). **Recommend (3)**, with phase 0 at the session's first tic.
3. **Short lumps (≤ 8 bytes):** refuse the tic that starts such a sound, as a witness with complete
   canonical state, *or* reject the WAD at load. The genuine code accepts the WAD, so rejecting it
   diverges at init. **Recommend refusing the start.** The synthetic edge WAD is the witness; no
   IWAD has such a lump ([obs]).
4. **Wipes:** keep M5b's wipe-free contract for sound (no mixing question arises) *or* model the
   original's no-mixing-during-melt plus tic catch-up. **Recommend keeping wipe-free** for M5c.
5. **Golden content:** buffer digests plus start echoes (`s` rows) *or* raw samples. **Recommend
   digests plus `s` rows** ([obs]: a start can change only its `s` row).

#### 3.3.3 What this section did not do or observe

- This section did not build or run the sndserver; its comparison (3.2.1 item 5) is from reading only.
- No real audio device was opened and no sound was played. The fake aborts on any `/dev/` path
  other than `/dev/dsp`, and `/dev/dsp` never reaches the real `open`.
- The OSS pacing in 3.2.3 item 3 is inferred, not observed on a device.
- The sound-logic runner of §3.1 had not reported when this section was finished: the start-list interface has not been
  exercised with real s_sound output, only with hand-written start lists.
- No shareware IWAD was available, so shareware counts are unknown.
- No fuzzing and no coverage-guided campaign. The sanitizer runs are the 21 scenarios plus the IWAD
  start-every-sfx runs (address: `ASAN_OPTIONS=detect_leaks=0`, leak detection disabled; overflow:
  default options).
- No cargo or Rust; no timing measurements.
- The sc315 build (schedule 2) was run only on 5 scenarios, in strict.
- Nothing was written inside the repository. The prototype lives in ~/scratch/m5c/proto-mixer/
  (`proto/` holds the files to lift; `src/` is a private export at 7f7fd99 with the same files
  under src/tools/reference/; `build/` and `runs/` are disposable).

### 3.4 Full frames under the new contract

The new contract: the M5b runner's frames with the genuine `s_sound.c` (sound draws), the genuine melt
(320 draws after the display of tic 0) and `S_UpdateSounds` per tic, under the declared configuration
(sfx volume 8, 3 channels, fresh process). Commands: `python3 tools/m5c_proto_ref.py --jobs 4` (synthetic),
`python3 tools/m5c_proto_real.py --iwad ~/scratch/m5c/proto-soundlogic/iwad/DOOM.WAD --jobs 4` (real). The
reference for "what changed" is the same fork run with `--sound off --wipe none`, which equals the frozen M5b
goldens (synthetic: the golden rows, 59 of 59 at each screen size; real: the committed
`genuine_stream_sha256` of `tests/reference/m5b_real_results.json`, 61 of 61 at each screen size) **[obs]**.

**Face rectangle and pixel check.** The face widget `w_faces` draws at (143,168) (`ST_FACESX`, `ST_FACESY`,
st_stuff.c:117-118) and erases with the old patch's rectangle (`STlib_updateMultIcon`, st_lib.c:212-241). The
driver takes the union of the 42 face patches' rectangles at that point (from their offsets and sizes in the
loaded WAD) and writes per frame the digest of the frame with that rectangle zeroed (Q row). A frame "differs
only in the face" when its full digest differs and its Q digest does not.

#### 3.4.1 Synthetic items (59) [obs]

| Configuration (vs `--sound off --wipe none`) | frames compared | face differs | full frame differs (sb10) | face-zeroed frame differs | other P fields differ | full frame differs (sb11) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| sound + melt, server convention | 21,043 | 11,392 (56 items) | 11,392 | 0 | 0 | 0 |
| sound + melt, integrated convention | 21,043 | 11,392 | 11,392 | 0 | 0 | 0 |
| sound only | 21,043 | 8,556 (34 items) | 8,573 | 0 | 0 | 0 |
| melt only | 21,043 | 10,640 (56 items) | 10,640 | 0 | 0 | 0 |

"Frames compared" excludes M5SHORT's 134 tracker-flagged frames (undefined reads, M5a). The 17 frames where
the full frame differs with sound only but the face index does not: the face index sequence differed earlier,
and `STlib_updateMultIcon` erases with the patch it last drew (st_lib.c:224-238), so the face area of a later
frame with an equal index can still differ; they are inside the face rectangle (Q equal) [obs, inf for the
reason]. On M5b's 48 items: 8,400 / 5,882 / 7,814 frames; adding M5SHORT's flagged frames (119 differ with
the melt, 0 with sounds alone, measured separately on M5SHORT) gives **8,517, 5,882 and 7,931, exactly M5b's
FH1 numbers**, and the sound draws are 262, also M5b's. Totals of the 59 items: 602 starts attempted, 573
started, 29 inaudible, 0 refused (S rows of all 59 items; M5VPLAN and M5OPEN start no sound); 514 sound
draws, 21,088 status-bar draws, 18,240 melt draws (Z totals of the 57 items whose stream reaches its Z row:
M5VPLAN and M5OPEN end in a genuine `I_Error` first; every item's W row shows 320 draws), 0 other draws; the largest number of simultaneously
occupied channels 3 (39 tics server, 457 integrated); starts that found all channels busy: 1 server (evicted a
channel), 4 integrated (evicted), 0 refused.

#### 3.4.2 Real Episode 1 runs (61, local only) [obs]

IWAD: a scratch copy (`~/scratch/m5c/proto-soundlogic/iwad/DOOM.WAD`, the same SHA-256 as the read-only
original, checked with `sha256sum`), the IWAD only in the lump list (its own `DS` and `D_` lumps). Per run: at
screenblocks 10 `--sound off --wipe none`, sound + melt strict (with dumps), overflow, address, strict again,
integrated (with dumps), integrated again, sound only, melt only, schedule B under both conventions; at
screenblocks 11 off, strict, strict again, integrated. Counts only.

- Tics 158,966 (the M4b count); endings 27 marker, 12 exit, 22 reborn; identical in every variant.
- Dumps: the screenblocks-10 render-every dump sets equal the committed M4b strict digests, **61 of 61 under
  the server convention and 61 of 61 under the integrated one**.
- Fork check: `--sound off --wipe none` gives the committed M5b genuine stream digest, 61 of 61 at
  screenblocks 10 and 61 of 61 at 11.
- Variants: sound streams identical across strict, overflow and address, and frame streams identical
  (normalised): 61 of 61 (screenblocks 10). **No sanitizer report in any real run**, so none in `s_sound.c`,
  `i_sound.c` or `f_wipe.c`.
- Determinism: the sound stream of the second strict run equals the first, 61 of 61, and the two integrated
  runs, 61 of 61; the stream is the same at screenblocks 10 and 11 (61 of 61 under each convention). Combined
  digest of the 61 per-run sound-stream digests (`name:sndserv:integrated` lines, file order):
  `c8eeffa93a891f9f2d68e879398ba37b6d2ee236ca1921406dc21dd69442a798`.
- Starts (server convention): 26,077 attempted, 21,189 started, 4,877 inaudible, 11 refused (no channel), 0
  link; 20,697 sound draws (= M5b's; 4,670 on E1M8, where no source is too far: 9,955 starts at volume 8 overall and
  2,651 at 9-15, which only the map-8 rule produces); 5,380 attempts without a draw (the 4,877 inaudible and
  `itemup`'s 503; every refused start in these runs had drawn). Stops: 20,216 finished, 798 evicted, 122 same origin, 0 by removal. Largest occupancy 3
  (212 tics server, 2,714 integrated). Starts that found all channels busy: 809 (798 evicted another channel, 11
  refused) server; 1,174 (1,064 / 110) integrated. Separation 33..224, both extremes reached (466 and 473
  starts).
- Frames: at screenblocks 10, **68,995 of 158,472 compared frames show a different face** (all 61 runs; the
  494 tracker-flagged frames are excluded), every one only inside the face rectangle (face-zeroed digest
  differs in 0 frames, other P fields in 0); sound only 50,505 (49 runs), melt only 66,706 (61 runs). At
  screenblocks 11, 68,976 of 158,414 have a different face index and **0 frames differ**. M5b reported 69,139
  of 158,966 with all frames counted; the count here excludes the flagged frames and they were not re-counted, so
  the comparable statements are: 20,697 sound draws (equal), DEMO1 540 of its 1,704 unflagged frames (M5b 542
  of 1,710).
- Under the integrated convention every frame count above is identical, and `rndindex` is identical at every
  point of every tic (61 of 61).

#### 3.4.3 Housekeeping [obs]

`df -h /mnt/c` before the builds and before and after the real runs: 28 GB free each time. The IWAD copy was
deleted at the end (see "What this section did not do or observe").

#### 3.4.4 Undefined, implementation-defined or layout-dependent behaviour

1. **The button origin** (3.1.11 item 6): `S_AdjustSoundParams` reads `x`,`y` from inside `buttonlist`
   (LP64 and ILP32 give different bytes; slot 15 reads past the array). It decides audibility, hence an
   `M_Random` draw and the face, and under the integrated convention it is re-read on every update.
2. **The handle convention** depends on a compile-time macro (`SNDSERV`, doomdef.h:84) and the channel
   lifetime on `gametic` since process start (i_sound.c:517-521).
3. The `S_sfx` entry `sfx_id == NUMSFX` passes the bad-id check (`sfx_id > NUMSFX`, s_sound.c:277) and would
   index past the table; unreachable (no caller passes it) **[src]**.
4. `I_InitSound`'s length for a linked sfx is `lengths[(link - S_sfx)/sizeof(sfxinfo_t)]` (i_sound.c:811), a
   pointer difference divided again by the element size: for `sfx_chgun` it reads `lengths[0]` (0). Only the
   integrated build, only the mixer, and `sfx_chgun` is never started **[src]**.
5. The prototype reads s_sound's static channel table through a copy of `channel_t`'s declaration
   (layout-dependent; checked by the allocation size at start).
6. In the server build `I_SubmitSound` still runs every loop iteration (`SNDINTR` undefined, d_main.c:402-405)
   and writes `mixbuffer` to `audio_fd`, which is 0 (never opened, i_sound.c:106, 669): 2,048 zero bytes to
   file descriptor 0 per frame **[src]**. Presentation only.

#### 3.4.5 Questions the full-frame facts raise for the owner

Each question is taken up in [§5](#5-owner-decisions-t1t16-with-recommendations): 1 in T2; 2 in T5; 3 in T12 (this section's recommendation, refusing, differs from §5's: T12 recommends the 64-bit reading); 4 in T9 (the volume and the 3 channels) and T15 (a fresh process).

1. **Handle convention.** (a) the shipped server build (`SNDSERV` as compiled: handle = sfx id), or (b) the
   integrated build (`addsfx`'s counter). Recommendation: **(b) for the channel table and the mixer**,
   because the server build's s_sound channels mean nothing to the audio (the external `sndserver` does its
   own mixing, out of scope) and the integrated build is the one whose `i_sound.c` mixer M5c ports; declare it.
   `rndindex`, the face and the frames are identical either way (measured), so the choice does not touch
   S15's closure.
2. **Update schedule.** (A) after every tic or (B) per mixer buffer. Recommendation: **(A)** for the
   reference, matching the singletics display cadence already ruled for M5b (FH2), unless the mixer's ruling
   requires (B); they differ only in channel lifetime (measured: no start refused or added on the synthetic
   set under either; real on the real runs (B) refused 15 more starts under the server convention and 17 more under the integrated one, out of 26,077).
3. **The button origin.** (a) reproduce the LP64 bytes the C reference reads, (b) emulate the 32-bit layout
   of the original binary, or (c) refuse the tic with a complete-state witness when the two layouts would give
   a different outcome (audibility, volume or separation) or when slot 15 is involved. Recommendation:
   **(c)**, with a room (SNDMOVERS) that shows the common case agrees.
4. **Configuration.** Declare `sfx_volume` 8, `snd_channels` 3, no config file, a fresh process with the level
   loaded at `gametic` 0 (`-playdemo`/`-warp`), as the reference; a session reached through menus is a
   different `gametic` history. Recommendation: yes.

#### 3.4.6 What this section did not do or observe

- No Rust code, no committed file, contract, golden or room. No new room was built; coverage is of the
  existing 59 items and the 61 real runs.
- This section did not compile `i_sound.c` without `SNDSERV`: the integrated convention calls the genuine `addsfx`
  directly and loads the data with the genuine `getsfx`, but the integrated `I_StartSound` body itself (one
  line, i_sound.c:495) is reproduced by the wrapper, not compiled.
- This section did not run the genuine mixer (`I_UpdateSound`) or digest `mixbuffer`.
- This section did not run the strict32 (`-m32`) variant, so the 32-bit reading of the button origin is inferred from
  the declarations, not observed.
- The exact clipping and close-distance boundaries were not shown to be reached by any item.
- The sound streams of the synthetic run were produced by a build whose origin labels called the button
  origin `unknown`; it was verified on M5SWTCH (the only item with unknown origins: 6 rows server, 76 integrated)
  that the relabelled build prints `button,0` for exactly those rows and nothing else changes.
- This section did not re-count the face differences in the real runs' 494 (sb10) / 552 (sb11) tracker-flagged frames,
  so the real total is not directly comparable with M5b's 69,139 (the draw count, 20,697, is).
- The real A/B frame identity in full (flagged frames included) was not checked: flagged frames differ run to
  run by construction (M5a).
- The IWAD copy (`~/scratch/m5c/proto-soundlogic/iwad/DOOM.WAD`) was deleted after the runs, with the build
  directories and run outputs; no real-map stream was kept (only counts and digests in `results/`).
- An earlier real-run pass was stopped twice (to add the button label, then the schedule option and
  the position columns); the numbers above are from the final complete pass only.
- Rule note: the first synthetic pass ran 5 parallel jobs, one more than the 4 named for the prototype (the
  task's own limit was 6); later passes used 4.

#### Files for Phase 2 to lift (all in `~/scratch/m5c/proto-soundlogic/`)

- `tools/m5c_build.py` (build, the two i_sound.c ABI adapters, 39 wrapped symbols), `tools/m5c_driver.c` and
  `tools/m5c_sound.inc` (the fork and its sound block; fold the include into the driver), `tools/m5c_stubs.c`.
- `tools/m5c_proto_ref.py` (synthetic checks: dumps, fork check, variants, determinism, face/Q comparison,
  conventions, `--ab` schedules, `--selftest`), `tools/m5c_proto_real.py` and `tools/m5c_ab_real_frames.py`
  (local real runs), `tools/m5c_summarize.py` (counts).
- `tools/m5c_sfxwad.py` (placeholder sound/music PWAD; replace by the generator of §3.3 plus the nine music
  lumps).
- Results (counts and digests only): `results/summary_items.json`, `results/summary_realitems.json`,
  `results/ab_synth.json`, `results/ab_real_frames.json`, `results/items/*.json`, `results/realitems/*.json`;
  logs in `logs/`. `src/` is the private export of 7f7fd99 the tools import from.

<!-- The prototype of sections 3.2 and 3.3 is under ~/scratch/m5c/proto-mixer/. -->

## 4. Rust design

*Written 2026-11-18 (America/Los_Angeles). Repository head 7f7fd99 (branch `feat/m5c-sound`). Nothing was built
or run for this section; every Rust fact is from reading the code at that head, and every C fact is either
re-read for this section (marked "re-read") or taken from sections 1 to 3 of this note, which are cited by
number. The report of the sound-logic runner (§3.1 and §3.4) did not exist when this section was written
(see "What this section did not do or observe").*

Terms used here, in addition to those of sections 1-3:

- **Sound logic**: the port of s_sound.c (its 3 channels, `S_StartSoundAtVolume`, `S_StopSound`,
  `S_UpdateSounds`, `S_AdjustSoundParams` and the pitch `M_Random` draw).
- **Mixer**: the port of the deterministic part of i_sound.c (lump loading and padding, `steptable`,
  `vol_lookup`, `addsfx`, `I_UpdateSound`'s loop) under the integrated configuration (doomdef.h:84
  removed, [§1.3.0](#130-which-code-is-compiled-the-build-configuration-read-this-first)).
- **Sound record**: one sound start or one removal stop as the simulation reports it to the sound
  logic, in C call order, with the positions `S_AdjustSoundParams` reads at that moment.
- **Iteration**: one pass of `D_DoomLoop` (d_main.c:369-406): some tics, then `S_UpdateSounds`, then
  `D_Display`, then one mixed buffer ([§1.1.7](#117-where-s_updatesounds-is-called), [§1.3](#13-the-mixer-i_soundc), item 28).
- **Proven artefact**: what is compared with the genuine engine. Today: the per-tic 320x200 frame and
  palette number (COMPATIBILITY "Playable build (M5b)", Presentation). M5c adds the sound record list,
  the start list given to the mixer and the stream of mixed buffers.

### 4.1 Summary

| Piece | Home | Depends on | Platform types? |
| --- | --- | --- | --- |
| The sound records (positions at each start and each removal stop), the correct `A_PlayerScream` sound, sector sound origins and (separately decided) the THING-loading truncation | **doom-sim, new wire 25 only** (§4.4) | unchanged | no |
| Sound logic (`s_sound`), mixer (`i_sound`), sound lumps, tables, the mixing schedule, the saved sound state (`DSND`) | **new crate doom-sound** | doom-types, doom-render (only for `r_main::{finesine, point_to_angle}`), doom-sim (only for `ActorId` and the record type) | no |
| The `M_Random` index (stays where it is), the wipe's 320 draws, the display contract value and `DDSP` version 2 | **doom-render**, additive (§4.3) | unchanged | no |
| Per-tic wiring of sound into the session, the audio sink trait, the recording sink, gain, mute, the optional integer resampler | **doom-game**, additive (§4.7) | + doom-sound | no |
| The audio device behind the platform seam, its thread and hand-over queue, the `--volume` and `--no-audio-device`-style flags | **doom-play** (§4.7) | + the audio library | yes (the only crate) |

The dependency direction keeps the simulation unaware of audio, and audio unaware of the window:

```
doom-types <- doom-wad <- doom-map <- doom-sim <- doom-render <- doom-sound <- doom-game <- doom-play (bin)
```

doom-sound sits beside the renderer as a presentation consumer of the simulation. Nothing in doom-sim
or doom-render depends on it. The audio library enters only doom-play, so `cargo test --workspace`,
the fuzz workspace and every headless test never link it. `unsafe_code = "forbid"` stays inherited
through `[lints] workspace = true` (Cargo.toml:16-18).

The decisions this design assumes, each with its own entry in §4.12: the integrated configuration for
both handles and samples; the declared iteration schedule of §4.3.3 (the buffer rule of [§3.2.4](#324-the-mixing-schedule-the-open-question-facts-per-candidate), with
`S_UpdateSounds` once per iteration); the wipe accounted as 320 draws after the first frame and not
drawn; the sound logic's `snd_SfxVolume` fixed at 8, the player's volume a gain after the proven stream;
and a wire 25 that only adds what sound needs.

### 4.2 Where the code lives

#### 4.2.1 What each half needs

1. **The mixer** needs only bytes: the sound lumps (by name, with `W_CheckNumForName`'s backward scan,
   w_wad.c:376-386 per [§1.3](#13-the-mixer-i_soundc), item 8), the 109-entry `S_sfx` table (name, link) and its own tables. It
   needs no game state at all ([§3.2.3](#323-facts-the-harness-depends-on-src), item 4: the stream is a pure function of the start list,
   the lumps and the schedule).
2. **The sound logic** needs, per start: the sfx id, the origin's identity (compared by equality only,
   [§1.1.2](#112-channel_t-the-globals-and-the-values-in-a-fresh-process)), the origin's x,y and the listener's x,y,angle at the call (s_sound.c:302-316,
   767-793 per [§1.1.5](#115-every-function-line-by-line)), the map number (map-8 rule, s_sound.c:773-787), the `S_sfx` priorities
   (s_sound.c:853-854), `finesine` (s_sound.c:793) and `R_PointToAngle2` (s_sound.c:780-783). Per
   `S_UpdateSounds`: the current x,y of each occupied channel's origin and the current listener.
3. **Where the trigonometry already lives.** doom-render exposes exactly what is needed, public and
   tested against tables.c: `r_main::finesine(i) -> Option<i32>` (crates/doom-render/src/r_main.rs:47-51),
   `r_main::point_to_angle(x, y, viewx, viewy) -> u32` (r_main.rs:90-130, `R_PointToAngle` relative to
   a view origin, which is `R_PointToAngle2`'s body once `viewx = x1, viewy = y1`, r_main.c:378-388
   per [§1.1.5](#115-every-function-line-by-line), item 4), `r_main::fixed_mul` (r_main.rs:61-63) and `slope_div` (r_main.rs:81-87).
   doom-sim has its own private copy of `finesine` (`static FINE_SINE`, crates/doom-sim/src/fine_sine.rs:4,
   included at lib.rs:166) and private angle code (level.rs:17-26 for `P_DeathThink`), neither public.
4. **Lump access.** doom-render's `LumpDirectory` (crates/doom-render/src/r_data.rs:117-200) is the
   genuine global lump list with `check_num_for_name` (backward scan, upper-cased query, r_data.rs:185-189).
   doom-wad's `WadSet::find` (crates/doom-wad/src/archive.rs:247) is a different lookup. The app
   already builds one `LumpDirectory` per run (crates/doom-game/src/app/mod.rs:268-271) and keeps it
   inside `Resources` (`Resources::directory()`, r_data.rs:468).

#### 4.2.2 Options

| | (A) new crate `doom-sound` (recommended) | (B) modules of doom-game | (C) modules of doom-render |
| --- | --- | --- | --- |
| Dependency direction | doom-sound -> doom-render (trigonometry, `LumpDirectory`) -> doom-sim. The simulation and the renderer stay audio-free. | doom-game already depends on everything it needs. | doom-render would gain audio code; the renderer crate's description and its `[profile.dev]` opt-level 2 (Cargo.toml:20-24) would then also cover the mixer. |
| Fuzz | Targets depend on doom-sound only (plus what it pulls in); no change to existing targets. | Targets would depend on doom-game, which is acceptable (the fuzz workspace already depends on it, fuzz/Cargo.toml:17) but mixes app-state code with sound code in one target build. | Same as (A) in practice. |
| Mutation testing | `-p doom-sound` is a separate package filter in mutants.yml's one command line (`cargo mutants -p doom-sim -p doom-render -p doom-game ...`); its tests run alone, which keeps shard time predictable. | Mutants in doom-game run doom-game's whole test suite, including the slow headless app tests (crates/doom-game/tests/app.rs, 1,355 lines). Every sound mutant would pay for them. | Mutants would run doom-render's suite (about 119,000 rendered frames in debug, Cargo.toml:19-21 comment). Worst of the three. |
| How M5a/M5b placed code | M5a made doom-render a crate of "one module per C file"; M5b put st/hu into doom-render because they share `screens[0]` and private resources ([M5B §3.2]). Neither reason applies to sound: it shares no buffer and no private resource with the renderer. | M5b kept doom-game for the loop, input and recorder: app policy, not C ports. s_sound.c and i_sound.c are C ports. | See (A). |
| Platform-free? | yes | yes | yes |

**Recommendation: (A), a new library crate `doom-sound`.** It is the only option that keeps a C-port
crate's tests and mutants separate from the app's slow tests and the renderer's frame tests, and it
does not change any existing crate's character. Its dependency on doom-render is for three public
pure functions and `LumpDirectory`; if the owner prefers doom-sound not to depend on the renderer,
the alternative is to move `finesine`, `tantoangle`, `slope_div`, `point_to_angle` and `fixed_mul`
into doom-types (a refactor of doom-render with its own mutation exposure). Not recommended: that
refactor in M5c touches proven renderer code for no behavioural gain.

#### 4.2.3 doom-sound modules and public API sketch

| Module | C | Contents |
| --- | --- | --- |
| `sounds` | sounds.h:177-289, sounds.c:114-227 | `S_SFX: [SfxInfo; 109]` (name, priority, link, link pitch, link volume; singularity kept as data, read by nothing, [§1.1.3](#113-soundsh--soundsc-the-s_sfx-table)), generated or checked by a committed script against sounds.c, and a test that the names equal doom-sim's `SOURCE_SOUNDS` order ([§2.2](#22-the-event-types), item 4) |
| `tables` | i_sound.c:396-424 | `STEPTABLE: [i32; 256]` as a committed constant, `vol_lookup` computed at construction by integer arithmetic (§4.6.2) |
| `lumps` | i_sound.c:185-251, 451-456, 799-813 | `SoundSet::load(&LumpDirectory) -> Result<SoundSet, SoundLoadError>`: padding, the pistol fallback, the linked chaingun's zero length, the short-lump and missing-lump markers |
| `mixer` | i_sound.c:264-381, 470-521, 539-654 | `Mixer`: 8 channels, `start`, `mix_buffer`, the handle counter |
| `logic` | s_sound.c:93-131, 254-484, 519-613, 708-875 | `SoundLogic`: 3 channels, `start`, `stop`, `update`, `adjust` |
| `schedule` | (declared, §4.3.3) | `Schedule`: which tics end an iteration |
| `state` | (new) | `SoundState`, the `DSND` encoding, decode and validation |
| `session` | (new) | `Sound`: one per session; runs a tic's records against logic and mixer transactionally |

```rust
pub struct SfxId(u8);                                 // 1..=108 admitted; 0 and 109 refused
pub enum OriginId { None, Actor(ActorId), Sector(u16), Button(u8) }   // pointer identity classes (§2.2, item 5)
pub struct Placed { pub origin: [i32; 2], pub listener: [i32; 2], pub listener_angle: u32 } // 16.16, angle_t

pub enum Record {                                     // from doom-sim wire 25 (§4.4), in C call order
    Start { sfx: SfxId, origin: OriginId, at: Option<Placed> }, // None: NULL or listener origin
    Stop { actor: ActorId },                          // P_RemoveMobj's S_StopSound (p_mobj.c:565-566)
}

pub struct SoundSet { /* per sfx: padded bytes (Arc<[u8]>), padded length, flags */ }
impl SoundSet {
    pub fn load(dir: &doom_render::LumpDirectory, caps: SoundCaps) -> Result<Self, SoundLoadError>;
}

pub struct Mixer { /* 8 channels, handle counter */ }
impl Mixer {
    pub fn new(set: Arc<SoundSet>) -> Self;
    pub fn start(&mut self, sfx: SfxId, vol: i32, sep: i32, pitch: i32, gametic: i32)
        -> Result<i32, MixRefusal>;                   // I_StartSound + addsfx; returns the handle
    pub fn mix_buffer(&mut self, out: &mut [[i16; 2]; 512]);    // I_UpdateSound
}

pub struct SoundLogic { /* 3 channels */ }
pub struct Listener { pub actor: ActorId, pub xy: [i32; 2], pub angle: u32 }
pub trait OriginPositions {                            // what S_UpdateSounds needs after an iteration
    fn actor_xy(&self, id: ActorId) -> Option<[i32; 2]>;
    fn sector_xy(&self, sector: u16) -> Option<[i32; 2]>;
    fn button_xy(&self, slot: u8) -> Option<[i32; 2]>;  // the declared LP64 reading (§4.4.5)
}
impl SoundLogic {
    pub fn start(&mut self, r: &StartArgs, rng: &mut DoomRng, mixer: &mut Mixer, gametic: i32)
        -> Result<Option<StartOutcome>, SoundRefusal>;  // S_StartSoundAtVolume
    pub fn stop(&mut self, origin: OriginId);          // S_StopSound
    pub fn update(&mut self, l: &Listener, p: &dyn OriginPositions, gametic: i32)
        -> Result<(), SoundRefusal>;                    // S_UpdateSounds
    pub fn level_start(&mut self);                      // S_Start's channel loop (s_sound.c:209-211)
}

pub struct Sound { /* logic, mixer, schedule, config */ }
pub struct SoundConfig { pub map: u8, pub sfx_volume: i32 /* 8, fixed */, pub channels: usize /* 3 */ }
impl Sound {
    pub fn new(set: Arc<SoundSet>, cfg: SoundConfig) -> Self;
    pub fn tic(&mut self, tic: u64, records: &[Record], rndindex: u8)
        -> Result<TicSound, SoundRefusal>;              // staged; draws counted
    pub fn end_iteration(&mut self, listener: &Listener, p: &dyn OriginPositions,
        out: &mut Vec<[i16; 2]>) -> Result<(), SoundRefusal>;   // S_UpdateSounds + one buffer
    pub fn state(&self) -> SoundState;
    pub fn restore(&mut self, s: SoundState, sim: &Simulation) -> Result<(), SoundRestoreError>;
}
pub struct TicSound { pub draws: u16, pub started: Vec<StartEcho>, /* staged state */ }
```

`StartEcho` is the genuine runner's `s` row ([§3.2.9](#329-design-for-phase-2-step-2): `s TIC ID VOL SEP PITCH PRIO HANDLE`,
plus the s_sound channel index), which §3.2 found necessary because a start can change only its
echo and not the stream ([§3.2.7](#327-proof-the-comparison-can-fail-and-determinism-obs)).

### 4.3 The `M_Random` stream and the per-tic sequence

#### 4.3.1 Who owns the index today

`rndindex` lives in the status bar's saved history: `StatusBarState::rndindex: u8`
(crates/doom-render/src/st_stuff.rs:180-182), zero at `level_start` (st_stuff.rs:251) and advanced only
by `StatusBarState::tick` through a local `DoomRng::from_indices(0, self.rndindex)` (st_stuff.rs:282-286).
It is written into `DDSP` version 1 as one byte (crates/doom-render/src/d_display.rs:708, decoded at
762). The app reports it per tic (`TicReport::rndindex`, crates/doom-game/src/app/mod.rs:108-109) and a
test pins `rndindex == (t + 1) & 0xff` after tic t (crates/doom-game/tests/app.rs:1117-1130).
`doom_types::DoomRng` keeps the gameplay and miscellaneous indices apart (crates/doom-types/src/rng.rs:6-35),
so presentation cannot consume `P_Random`.

#### 4.3.2 Options for ownership

| | (i) Display keeps the index (recommended) | (ii) the app core owns an `MRandom` |
| --- | --- | --- |
| How sound draws | doom-game reads `display.status().rndindex`, doom-sound draws on a local `DoomRng` copy and returns the number of draws; doom-game passes that count to a new `Display::observe_after(sim, fed, draws)`, which adds it before `ST_Ticker`'s own draw | doom-game passes `&mut MRandom` to sound and to the display |
| doom-render <-> doom-sound | neither depends on the other for this | neither |
| Saved where | `DDSP` (already), plus the contract byte (§4.8) | a new field in doom-game's session state; DDSP v1's byte becomes redundant |
| Frozen code touched | `observe` and `StatusBarState::tick` unchanged; one additive method | `StatusBarState::tick`'s signature or a parallel tick; more churn in proven code |

**Recommendation (i).** The values are a pure function of the index and the number of draws
(m_random.c:63-67 per [§1.4.1](#141-m_randomc-in-full-and-every-use-of-the-index), item 2), so passing a count loses nothing. The wipe's 320 draws are
made by `Display::draw` itself on the frame that wipes (the existing `DisplayState::wipe` flag,
d_display.rs:151-153, is already set by `begin_level`, d_display.rs:285, and cleared after the first
frame, d_display.rs:418), only under the new contract value.

#### 4.3.3 The per-tic sequence against the C order

C (integrated configuration, [§1.1.7](#117-where-s_updatesounds-is-called), [§1.3](#13-the-mixer-i_soundc), item 28, [§1.4.4](#144-d_displays-wipe-path-and-the-startup-to-the-first-frame)): per iteration, tics
(`G_Ticker`: `P_Ticker` with every sound start in call order, then `ST_Ticker`'s one draw, then
`HU_Ticker`; g_game.c:727-733 per [§1.2.4](#124-the-moment-within-the-tic)), then `S_UpdateSounds` (d_main.c:392, no draw), then
`D_Display` (the first one wipes: 320 draws, f_wipe.c:159-162), then `I_UpdateSound` and
`I_SubmitSound` (d_main.c:397-405).

**The declared schedule (recommended, §4.12 item 2, T5).** The buffer rule ([§3.2.4](#324-the-mixing-schedule-the-open-question-facts-per-candidate) (3)): after the
starts of tic t (t counted from the session's first tic, 0), mix every buffer k not yet mixed with
`512*k < 315*(t+1)`. That gives 0 or 1 buffer after each tic; per 512 tics, 315 buffers; buffer 0 after
tic 0. The matching iteration boundary is added: **an iteration ends after tic t exactly when a buffer is
mixed after it**, and `S_UpdateSounds` runs once at each iteration end. The iterations are then
`{0}, {1}, {2,3}, {4}, {5,6}, ...` (pattern 1,1,0,1,1,0,1,0,... of [§3.2.4](#324-the-mixing-schedule-the-open-question-facts-per-candidate)), which is a loop
vanilla can run (`TryRunTics` with counts 1 or 2, d_net.c:736-746 per [§1.3](#13-the-mixer-i_soundc), item 28).

Per tic t, the app core does:

| Step | App | C counterpart | Draws |
| --- | --- | --- | --- |
| 1 | input, `G_BuildTiccmd`, recorder (unchanged, app/mod.rs:728-756) | `G_BuildTiccmd`, demo write and re-read | 0 |
| 2 | `sim.advance` under wire 25 (app/mod.rs:757); the tic's `Record`s become readable | `P_Ticker`: starts and removal stops in call order | 0 (in the simulation) |
| 3 | `Sound::tic(t, records, rndindex)`: for each record in order, `S_StartSoundAtVolume` or `S_StopSound`; starts reaching `I_StartSound` go to the mixer with `gametic = t` | the same calls inside `P_Ticker` | one per start that passes the early returns, except ids 32 and 87 ([§1.1.5](#115-every-function-line-by-line), item 6) |
| 4 | `Display::observe_after(sim, fed, draws)` | `ST_Ticker`, `HU_Ticker` | 1 |
| 5 | if tic t ends an iteration: `Sound::end_iteration` = `S_UpdateSounds` with `gametic = t + 1` and the current listener, then one buffer of 512 frames to the sink | `S_UpdateSounds`, then (after `D_Display`) `I_UpdateSound` | 0 |
| 6 | `Display::draw` (every tic, the M5a/M5b frame contract); on the session's first frame under the new contract, +320 after composing | `D_Display` (once per iteration; the first wipes) | 320 once |
| 7 | commit the staged sound and display state; `Host::tic` with the report | `gametic++` | |

Step 5 is placed before step 6 for the C order (`S_UpdateSounds` before `D_Display`); the mix belongs
after `D_Display`, but nothing in `D_Display` touches sound state, and nothing in the mixer touches the
display, so the order of steps 5 and 6 changes no output.

**Why the frames stay per tic and are still exact.** Sound draws depend only on positions and the sfx
id, never on channel state ([§1.1.6](#116-what-playing-means-and-exactly-when-a-channel-is-freed-in-both-builds) table: "M_Random draws ... no"). `ST_Ticker` draws once per
tic. So the `M_Random` stream, and with it the face, is identical under any grouping of tics into
iterations, provided the wipe's 320 come after the same tic. With the first iteration `{0}` the wipe
comes after tic 0 under both the per-tic frame contract and the iteration schedule, which is
the k = 1 of [§1.4.5](#145-does-a-session-started-directly-on-a-map-make-the-wipes-draws-and-where), item 3 and [§1.4.7](#147-what-a-port-must-keep) (a) 1. Only the channel table and the samples depend
on the grouping, and those are declared by the schedule.

**An earlier form of the buffer rule, and why it is not used.** The rule "a buffer is mixed after tic t
exactly when `floor(315*(t+1)/512) > floor(315*t/512)`" mixes nothing after tic 0 (floor(315/512) = 0) and
the first buffer after tic 1. Then the first iteration is `{0, 1}`, the first `D_Display` (and the wipe)
would come after tic 1 (k = 2) instead of after tic 0 (k = 1, above), and the starts of tic 0 would land in
the same buffer as tic 1's. The rule adopted (the ceiling form: buffers after tic t are those with
`ceil(315*t/512) <= k < ceil(315*(t+1)/512)`) has neither problem. The earlier form also kept
`S_UpdateSounds` after every tic while mixing only after some; that hybrid is a loop no vanilla build
runs (vanilla mixes once per iteration, and `S_UpdateSounds` runs once per iteration), and it changes
which channels are free in the second tic of a two-tic iteration, which decides refusals and so the
start list ([§1.1.7](#117-where-s_updatesounds-is-called), item 3). The per-iteration placement keeps the whole sound path equal to one
genuine loop. The owner decides (T5).

#### 4.3.4 Keeping the M5b contract reproducible

The M5b frames, goldens, DDSP version 1 states and tests are frozen. A contract value is proposed
instead of changing defaults:

```rust
// doom-render
pub enum RandomContract {
    SoundFreeWipeFree,   // decision S15 (M5b): index 0 at a new game, only ST_Ticker draws; DDSP v1
    WithSound,           // M5c: sound draws counted in by the caller, 320 at the first frame; DDSP v2
}
impl Display {
    pub fn set_contract(&mut self, c: RandomContract);           // default SoundFreeWipeFree
    pub fn observe_after(&mut self, sim: &Simulation, fed: &TicCommand, draws: u16)
        -> Result<Option<Message>, DisplayObserveError>;         // WithSound only
}
// doom-game
pub enum SoundMode { Off, On(SoundOptions) }                     // SessionConfig::sound, Default Off
impl GameData { pub fn load_with_sound(..) -> Result<Self, LoadError>; } // wire 25 + SoundSet
```

- `DisplayConfig` gets **no** new field: 18 places build it with a struct literal
  (`grep -rn "DisplayConfig {"`), several in frozen M5b tests, and a new field would break them. A
  setter with the M5b default changes nothing for existing callers.
- `SessionConfig` can take a new field because every test builds it with `..SessionConfig::default()`
  (crates/doom-game/tests/app.rs:495-498, 796-801); its default stays `SoundMode::Off`, wire 24 and
  `SoundFreeWipeFree`, so `GameData::load` (app/mod.rs:248-294, wire 24 constructor at 278-283) and every
  M5b test keep their exact behaviour. doom-play opts in.
- `Display::observe` under `WithSound` is refused (a typed error), so a caller cannot silently skip
  sound draws; `observe_after` under `SoundFreeWipeFree` is refused unless `draws == 0`.

### 4.4 How sound starts get out of the simulation (owner decision, T3)

#### 4.4.1 What the simulation gives today, re-checked

1. Every reachable start is an event in C call order with sfx and origin identity (§2.0-2.3), as
   `ActorStateEvent::Sound` (central helper `sound()` in the lifecycle host,
   crates/doom-sim/src/lifecycle.rs:1528-1530, plus about ten direct pushes: movers.rs:545,
   projectiles.rs:99, weapons.rs:923, player_pistol.rs:301, 665, 718, 724, world_pickups.rs:498,
   thinkers.rs:434, use_lines.rs:68, 390) and `MoverEvent::SectorSound`.
2. **The wrong scream is confirmed.** C: `sfx_pdiehi` only `if ((gamemode == commercial) && (mo->health
   < -50))` (p_enemy.c:1994-2007, re-read). Rust: `if life.health < -50 { "sfx_pdiehi" } else
   { "sfx_pldeth" }` (lifecycle.rs:1701-1708, re-read). The playable build never runs a commercial game.
3. No event carries a position; sector `soundorg` is stored nowhere ([§2.4](#24-positions-at-the-moment-of-the-call)).
4. Events are canonical: `events.last.N` and `events.pending.N` are fields of every snapshot
   (crates/doom-sim/src/snapshot.rs:1271-1279) and so of `canonical_bytes` and `digest`
   (snapshot.rs:67-86).
5. **But no wire-22-24 genuine comparison has ever compared events**: `level_compare.py`'s `stripped`
   drops every path starting with `map.` or `events.` (tools/reference/level_compare.py:63-68), and
   `level_compare_m4b.py` uses it (tools/reference/level_compare_m4b.py:12, 50). The DSIM40 fixtures carry
   no events (tools/reference/m4b_snapshot_goldens.py:31-35).
6. **The renderer and the HUD read events.** `RenderState`'s switch input counts `sfx_swtchn` sound
   events (crates/doom-render/src/state.rs:172-185), and `derive_message` sets its `oof` flag from **the
   immediately preceding event** (crates/doom-render/src/hu_stuff.rs:127-153: `oof = matches!(e, ...
   Sound { actor: None, sound: SFX_OOF })` after each event). Any new event interleaved in `events.last`
   right after a NULL-origin `sfx_oof` would break the locked-door message derivation, which
   `Display::observe` turns into `MessageUnderivable`. This constrains option (c) below.

#### 4.4.2 Option (a): wire-24 events plus app-side reconstruction

The app copies every actor's x,y and the player's angle before `advance`, then folds the event list.
The cases found where the result is not exact or not provably exact are:

1. **The listener's angle at the teleport fog sounds** when the same tic's `P_PlayerThink` turned the
   player by `A_Punch` or `A_Saw` (p_pspr.c:489-494, 523-541 per [§1.2.4](#124-the-moment-within-the-tic)) or by `P_DeathThink`
   (p_user.c:200-221). The fogs sound with the old angle (p_telept.c:111-118, angle set at 124); that
   angle is in no event and no snapshot ([§2.4](#24-positions-at-the-moment-of-the-call), case 4). The plain `P_MovePlayer` turn is exact (the
   executed command's `angleturn << 16` when `Event::Player` is present).
2. **Positions changed outside an actor's own thinker.** [§2.4](#24-positions-at-the-moment-of-the-call) states that an actor's x,y change only
   in its own thinker, at spawn and in `P_CheckMissileSpawn`. That is incomplete: `P_DamageMobj` sets the
   see state of a monster in its spawn state (p_inter.c:911-913, re-read), `P_SetMobjState` calls the
   action at once (p_mobj.c:76-77, re-read), and that `A_Chase` can `P_Move` the victim (p_enemy.c:764-768,
   re-read) in the **damager's** phase ([§1.2.4](#124-the-moment-within-the-tic) "Moving origins"). The victim's own thinker may move it
   again later in the tic. Reconstruction must replay that step at its place in the list.
3. **`A_Look`'s sight sound precedes its own seestate `A_Chase` step** (sound at p_enemy.c:660,
   `P_SetMobjState` at 663, re-read): the origin is one step behind its end-of-tic position.
4. **Walk-over specials and teleports inside the listener's split move** (`xmove > MAXMOVE/2`,
   p_mobj.c:162-174 per [§1.2.4](#124-the-moment-within-the-tic)): the listener is between half steps at the door or plat sound.
5. **Missile spawn and explode-on-spawn positions** are recoverable (`Projectile::Launch` names the
   source; `ProjectileEvent::HalfStep` carries the half step, [§2.4](#24-positions-at-the-moment-of-the-call), case 3).
6. **The button-release origin under ILP32** needs `btexture`, which Rust does not store
   (use_lines.rs:30-42: `Button { line, timer, sector }`); under LP64 it needs the next slot's timer,
   which Rust has (§4.4.5).
7. **`A_PlayerScream`**: a rewrite `pdiehi -> pldeth` in the app (it is the only emitter of
   `sfx_pdiehi`, [§2.8](#28-gaps-and-options)).

Cases 2-4 are exact **only if** every position commit of every actor emits an event carrying the
resulting position, in order. That could not be established from the code in the time available (it needs
an audit of `ActorMovement`, `SectorMovement`, `Slide`, pursuit and teleport events against every
`P_TryMove`, `P_Move` and `P_TeleportMove` commit). Case 1 is never exact. Cost: about 500-800 lines of
reconstruction in doom-game or doom-sound that re-implements `P_PlayerThink`'s angle changes and the
movement fold outside the simulation, and every exception becomes a refusal (case 1) or an unproven
claim (cases 2-4). **Not recommended.**

#### 4.4.3 Option (b): an observation channel, kept out of the canonical snapshot

At each sound start and each `P_RemoveMobj` stop, the simulation appends a `Record` (§4.2.3) to a
buffer that is not in `Snapshot`, committed and dropped with the tic's plan (as `events` is: a refused
tic publishes nothing, [§2.3](#23-order), item 6).

- **Where it is populated.** At the same points as today's sound events, reading the origin's and the
  listener's positions from the staged actor state at that moment. In the lifecycle host the choke point
  exists: `fn sound(&mut self, actor, sound)` (lifecycle.rs:1528-1530) with `self.access` giving every
  actor's staged fields (as `teleport.rs:214` reads `h.access.get(id)`), and `fn mark`
  (lifecycle.rs:1502-1511) for the removal stop. The other emitters (perception, pursuit, monster attacks,
  projectiles, movers and planes, use lines, weapons, pickups, the oof landing) each need the listener
  in their context. Only positional starts need positions: player-origin and NULL-origin starts (weapons,
  pickups, oof, noway, saw sounds, the locked door, a switch while slot 0 is free) need the record with
  `at: None`.
- **Must it be gated to a new wire?** It changes no canonical byte of any wire, but it is a change of
  the simulation's code paths under wire 24, and the owner's rule is "the simulation change confined to
  the new wire". It also cannot fix the scream inside the simulation without changing wire 24's events.
  Ungated, the scream needs the app rewrite of case 7 above (a declared adapter outside the simulation).
  Gated to wire 25, (b) is (c) minus canonicality.
- **Missing sector and button origins.** Sector `soundorg` is a pure function of the map
  (p_setup.c:536-557, with `M_AddToBox`'s `else if`, m_bbox.c:51-58 per [§1.2.6.2](#1262-the-sector-sound-origin-and-its-arithmetic)); the simulation
  can compute it once at load into its static world, outside the snapshot. The button release is §4.4.5.
- **Cost.** In doom-sim about 450-700 lines (record type, about 25 emitter touches with their listener
  plumbing, `soundorg`, the button reading, the gate); tests 400-600. No DSIM, encoder or fixture work.
- **What it lacks.** Positions are not pinned by any DSIM digest; Rust-only regression tests need their
  own digest of the record list.

#### 4.4.4 Option (c): wire 25 with canonical sound records (recommended)

`OriginalExecutionProfile::PlayerMonstersEpisode1V3`, `wire_id` 25, a strict superset of wire 24 built
the way wire 24 was built from 23 (crates/doom-sim/src/original_registry.rs:111-139, predicates such as
`has_episode1_v2` at :402-405 and the per-behaviour predicates at :406-432; 52 references to the wire-24
variant or predicate across doom-sim's sources and 16 files).

**What wire 25 contains, exactly, and nothing else:**

1. **Sound records** as a new canonical group, **`events.sounds.N`**, after `events.pending`, in DSIM41
   only: `Start { sfx, origin, at }` and `Stop { actor }` in C call order (§4.2.3). `at` is present
   exactly for positional starts (origin not NULL and not the listener's body, s_sound.c:302 per
   [§1.1.5](#115-every-function-line-by-line), item 5) and holds the origin's x,y and the listener's x,y,angle **at the call**.
   - Not interleaved into `events.last`: the existing `Sound`/`SectorSound` events stay byte for byte as
     wire 24 emits them, so `switch_inputs` and `derive_message` (4.4.1 item 6) see the same lists.
   - The path starts with `events.`, so every frozen comparator strips it unchanged
     (level_compare.py:64); the new sound comparator (§4.9) compares it.
   - `Stop` is pushed where `Event::RemovalMarked` is pushed (`mark`, lifecycle.rs:1502-1511, and the
     pickup, sky-missile and crunch paths of [§2.2](#22-the-event-types), item 7; Phase 2 must audit that each goes through
     one push point).
2. **The correct `A_PlayerScream`**: under wire 25 `sfx_pldeth` always. Wire 25 declares a non-commercial
   game, which is exact for its domain: its maps are E1M1-E1M9, which a commercial game never loads
   (vanilla's commercial mode names maps `MAPxx`). No constructor input is needed.
3. **Sector sound origins**: computed once at load (p_setup.c:536-557, `M_AddToBox`'s `else if`,
   truncating `/2`, [§1.2.6.2](#1262-the-sector-sound-origin-and-its-arithmetic)), held in the static world, **not** in the snapshot (the `map.*` group
   is checked once against the encoder, level_compare_m4b.py:52-56, and adding to it would cost encoder
   work); exposed read-only as `Simulation::sector_sound_origin(sector) -> Option<[Fixed; 2]>`. A sum that
   overflows `int` (p_setup.c:556-557, [§1.2.8](#128-undefined-implementation-defined-or-layout-dependent-behaviour-in-the-call-sites), item 5) refuses the map at load with a typed error.
4. **The button-release origin** under a declared ABI (§4.4.5).
5. **(Separate yes/no, T4) the THING-loading truncation** (p_setup.c:317-337 per [§2.7](#27-thing-loading-truncation-for-the-owners-wire-decision)): one test at the
   top of `spawn_map_things`' loop (crates/doom-sim/src/spawning.rs:443-462), before the start handling,
   gated by a wire-25 predicate. It uses the same "non-commercial" fact as item 2.

**What changes in canonical bytes.** Nothing for wires 1-24. Wire 25 captures are DSIM41: the DSIM40
layout with `profile` 25, a new context string (for example `PLAYER-MONSTERS-EPISODE1-3-v1`), the
`pdiehi` correction inside `events.last` where it occurs, and the `events.sounds` group.

**The independent DSIM encoders.** Following M4b's method, where DSIM40 was "the frozen DSIM39 projection
... with version 40, policy 24, context ... and `level.map`" and its self-test required every script's
DSIM40 projection to equal its DSIM39 projection outside `DELTA_FIELDS`
(tools/reference/level_snapshot_m4b.py:61-68, m4b_snapshot_goldens.py:47-50): a new
`level_snapshot_m5c.py` with version 41, policy 25, the new context, `DELTA_FIELDS = ('profile',
'context.schema')`, and a self-test that every script's DSIM41 projection equals its DSIM40 projection
outside the delta. The `events.sounds` group is projected from the genuine sound runner's rows by a
separate encoder (the start rows of the sound-logic runner of §3.1, mapped to Rust identities), compared by the sound
comparator, not by `level_compare`. No existing encoder, fixture or golden changes; new DSIM41 fixtures
are added beside the M4b ones if wanted.

**How "every non-sound digest of the 61 runs is unchanged from wire 24" is proven.** Three checks:

1. Rust against Rust, on all 61 real runs and every synthetic item: the stripped digest
   (`map.*` and `events.*` removed) of each DSIM41 capture with `profile` and `context.schema` replaced
   by wire 24's values equals the stripped digest of the wire-24 capture of the same tic. This is the
   same projection the M4b self-test used.
2. Rust against Rust: `events.last` of wire 25 equals wire 24's with every `sfx_pdiehi` replaced by
   `sfx_pldeth`, and nothing else; `events.pending` equal.
3. Genuine against Rust: `level_compare` with a DSIM41 profile on the 61 runs and the synthetic items
   (`match` on every tic), plus the new sound comparator on the `events.sounds` group.

With the truncation fix included, check 1 still holds on the 61 runs, because no Episode 1 IWAD map
contains any of the ten types (COMPATIBILITY "Known gap", census paragraph); the only digest changes
would be on the new synthetic truncation WAD.

**Cost.** doom-sim about 800-1,100 lines (the variant and predicates, constructor and context, the
record type, about 25 emitter touches and their listener plumbing, the snapshot group, the DSIM41
decoder in diagnostic.rs, `soundorg` with its overflow refusal, the button reading, the scream, the
truncation); tests 700-1,000 (wire-24 unchanged checks, record order, positions at the call in the hard
cases of 4.4.2, refusals); tools about 300 lines Python (DSIM41 encoder and self-test) plus the sound
comparator (§4.9). Evidence work: one more pass of the 61 real runs and the synthetic items through the
whole-engine comparison, and the truncation WAD through the existing registered-mode harness.

**The main risk** is plumbing: some emitters run while the player's or an actor's new state is staged
in a plan rather than in the store. Each emitter must read the staged values (the lifecycle host's
`access` does; the players phase and the mover phases need checking). A wrong read is caught only by the
genuine sound comparison, so that comparison is the gate (§4.9).

#### 4.4.5 The button-release origin (p_spec.c:1151)

The C passes `(mobj_t *)&buttonlist[i].soundorg`, a field address; `S_AdjustSoundParams` reads x,y at
`mobj_t`'s offsets past it ([§1.2.6.3](#1263-the-switch-sounds-origin-and-the-button-release-origin), observed offsets): **LP64** x = `buttonlist[i+1].btimer`, y =
the padding after it (zero in practice); **ILP32** x = `buttonlist[i+1].btexture`, y =
`buttonlist[i+1].btimer`; slot 15 reads past the array. Both readings give (0,0) when slot i+1 is free.
The read happens in the start (before the `memset` at p_spec.c:1152) and again in every later
`S_UpdateSounds` while the channel is occupied ([§2.9, item 1](#29-undefined-implementation-defined-or-layout-dependent-behaviour)), with slot i+1's then-current
values.

| | (a) LP64 reading (recommended) | (b) ABI-independent: (0,0), refuse when slot i+1 is busy | (c) ILP32 reading |
| --- | --- | --- | --- |
| Data needed | slot i+1's timer (in Rust state: `Button::timer`, use_lines.rs:36-37) | slot i+1 free or busy | `btexture`, not stored |
| Matches | the default genuine build (the reference builds are LP64 with opt-in `strict32`, [§2.9, item 1](#29-undefined-implementation-defined-or-layout-dependent-behaviour)) | both ABIs where it does not refuse | the strict32 build |
| Cost | a public read accessor for the button slots (they are `pub(crate)`, use_lines.rs:34-45), about 40 lines with the slot-15 refusal | about 20 lines; a witness that fires whenever two reusable switches overlap | a new stored field: a change to wire-25 state |

Slot 15 is refused in every option (unreachable on Episode 1: at most 10 reusable switch lines per map,
[§1.2.6.3](#1263-the-switch-sounds-origin-and-the-button-release-origin)). One ordering detail for Phase 2: `P_UpdateSpecials` walks slots 0..15 and decrements as
it goes, so at slot i's release slot i+1 has **not yet** been decremented in this tic; the record must
carry that value.

#### 4.4.6 Option (d): a declared limit

Ship wire-24 events with the reconstruction of (a), and refuse the tics where it cannot be exact
(case 1; cases 2-4 until audited) and every button release with slot i+1 busy. Cost about 600-900 lines
in the app plus refusal tests; it leaves the wrong scream in the canonical events and the positions
unproven where the refusals do not fire. Not recommended.

#### 4.4.7 Recommendation

**(c), wire 25 with canonical `events.sounds` records, and the THING truncation carried (T4: yes).**
Reasons: it is the owner's own first option; it is the only one that fixes the scream in the
simulation; positions come straight from the code that moves things, at the moment of the call; the
existing comparators strip the new group without any change, so the proof of "non-sound digests
unchanged" is the M4b projection method; and the sound comparator then checks every record against
the genuine engine, which has never been done for any wire-22-24 sound ([§2.6](#26-what-has-been-validated-by-evidence)). Choose (b) gated to
wire 25 if the owner wants to avoid DSIM41: it saves about 300-500 lines of snapshot, decoder and
encoder work and loses the DSIM digest of the positions.

### 4.5 `S_UpdateSounds` in Rust

1. **What it does** (s_sound.c:519-613 per [§1.1.5](#115-every-function-line-by-line)): for each occupied channel in index order, free
   it if `gametic >= handle` (`I_SoundIsPlaying` is `gametic < handle`, i_sound.c:517-521); otherwise,
   for a channel whose origin is not NULL and not the listener, recompute audibility from the **current**
   listener and origin positions and free it if inaudible. The recomputed volume and separation go to
   `I_UpdateSoundParams`, which does nothing (i_sound.c:674-688), and `I_StopSound` does nothing
   (i_sound.c:505-514). No draw.
2. **What the app supplies at an iteration end** (through `OriginPositions` and `Listener`, §4.2.3):
   the player body's x,y,angle after the tic (`Simulation::actor`, crates/doom-sim/src/lib.rs:970-973,
   as `status_inputs` already reads it, d_display.rs:177-186); each occupied actor origin's x,y by
   `ActorId`; sector origins from `sector_sound_origin`; button-release origins from the button slots
   (§4.4.5).
3. **Removed actors.** At most one occupied channel per origin (s_sound.c:349 then 868-872, proved in
   [§1.1.10](#1110-order-sensitivity-within-a-tic), item 3), and every `P_RemoveMobj` frees its origin's channel (p_mobj.c:566). With a
   `Stop` record at every removal, no channel can hold a removed actor at an update. If `actor_xy`
   returns `None` for an occupied channel anyway, the update refuses with `SoundRefusal::OriginGone`
   (a witness of a broken invariant, never a silent free). Marked actors stay readable until retirement
   (lib.rs:970-971), so a channel freed later in the same tic by `Stop` is ordered correctly.
4. **`gametic` and the handle counter** under "a fresh process started straight on the map" ([§1.1.6](#116-what-playing-means-and-exactly-when-a-channel-is-freed-in-both-builds), item 5, [§1.4.4](#144-d_displays-wipe-path-and-the-startup-to-the-first-frame), item 8): `gametic` is 0 while tic 0 runs and t + 1 after tic t, which is
   `Simulation::tic()` after `advance` (lib.rs:959-961), so it is derived, not stored separately (the
   saved state checks it). The integrated handle counter (`static unsigned short handlenums`, starting
   0, reset to 100 when 0, i_sound.c:271, 334-340 per [§1.3](#13-the-mixer-i_soundc), item 19.5) lives in the `Mixer` and starts at
   0 with each session (restart is a new process, decision S11). Under the integrated configuration a
   channel times out only once `gametic` passes 100 + the number of earlier starts, so in practice
   channels are freed by `S_StopSound`, eviction, inaudibility and level start ([§1.1.6](#116-what-playing-means-and-exactly-when-a-channel-is-freed-in-both-builds)).
5. **What a player can hear of this table.** Only which starts reach `I_StartSound`: a start refused for
   lack of a channel (s_sound.c:850-860) is never mixed ([§1.1.10](#1110-order-sensitivity-within-a-tic), item 5). Freeing a channel never
   silences anything; the mixer has 8 channels while s_sound tracks 3 ([§1.3](#13-the-mixer-i_soundc), item 20). The channel
   contents (slot, origin, handle) are otherwise compared only against the genuine dump, as diagnostics
   and as the evidence that refusals happen at the same starts.

### 4.6 The mixer in Rust

#### 4.6.1 Data

Per channel c in 0..8 (i_sound.c:116-151 per [§1.3](#13-the-mixer-i_soundc), item 7): `sfx: Option<SfxId>` (the data pointer;
`None` is NULL), `pos: u32` and `end: u32` (byte offsets into that sfx's padded data), `step: u32`
(16.16, `steptable[pitch]`), `frac: u32` (0..=65535), `start: i32` (`gametic` at the start), `handle: i32`
(written, never read, kept for the echo), `id: i32` (single-instance rule), `lvol: u8`, `rvol: u8`
(0..=127, row indices of `vol_lookup` in place of the C row pointers). Global: `handlenums: u16`. The
"start-order age" is `start`; ties are broken by the lowest index (strict `<`, i_sound.c:309-325).

#### 4.6.2 Tables, with no floating point at run time

- `vol_lookup[v][s] = (v * (s - 128) * 256) / 127` for v 0..=127, s 0..=255 (i_sound.c:421-423). Rust's
  `/` truncates toward zero as gcc does ([§1.3.8](#138-undefined-implementation-defined-or-layout-dependent-behaviour), item 5), so a `const fn` or a constructor computes it
  exactly on every OS; tested against the genuine digest (SHA-256 `f727268a...0043` of the
  little-endian array, [§3.2.6](#326-the-tables-obs); FNV-1a-64 `552057530633d3e9`, [§1.3](#13-the-mixer-i_soundc), item 13).
- `steptable[p] = (int)(pow(2.0, (p - 128)/64.0) * 65536.0)` (i_sound.c:414-415) = floor(2^((1024 + p -
  128)/64)) for p 0..=255, because the product is positive and the multiplication by 65536 exact ([§3.2.6](#326-the-tables-obs)). Two exact options:
  - (i) **integer generation**: the integer 64th root of 2^(896+p), computed with a small big-integer
    routine (the independent model of [§3.2.6](#326-the-tables-obs) does this in Python);
  - (ii) **a committed constant table** of 256 `i32`, generated by a committed script, plus a test that
    checks each entry v with exact integer arithmetic (v^64 <= 2^(896+p) < (v+1)^64, a 30-line
    multi-word multiply in the test only) and the genuine digest (SHA-256 `95f2209c...6dde`).
  **Recommendation (ii)**: the run-time code has no root and no float; the test is exact and identical on
  Linux, Windows and macOS. Margin to the nearest integer is 0.0125 ([§1.3](#13-the-mixer-i_soundc), item 13, [§3.2.6](#326-the-tables-obs)), so
  the genuine `pow` cannot disagree with the exact value.

#### 4.6.3 Lump loading (`SoundSet::load`)

For each sfx 1..=108 in order (i_sound.c:799-813):

1. Linked (`chgun` only, sounds.c:204): the pistol's data and **length 0**, the C's `lengths[0]`
   (i_sound.c:811, `(link - S_sfx)/sizeof(sfxinfo_t)` is 0 on every ABI, [§1.3](#13-the-mixer-i_soundc), item 12). Kept exactly:
   one frame of the pistol's first byte (in bounds). `sfx_chgun` is never started ([§1.1.3](#113-soundsh--soundsc-the-s_sfx-table)).
2. Otherwise `"ds" + name` by `LumpDirectory::check_num_for_name`; if absent, a **copy of dspistol**
   (i_sound.c:213-216) and a `missing` flag; if dspistol is absent too, `SoundLoadError::PistolMissing`
   (the C's `I_Error` at start-up, w_wad.c:399-409).
3. `size` = lump length; `padded = ((size - 8 + 511) / 512) * 512` with C `int` division on a value
   that is never negative for size >= 0 ([§1.3](#13-the-mixer-i_soundc), item 11); data = bytes 8..size then 128 until `padded`
   bytes. Lumps of 8 bytes or fewer get `padded = 0` and a `short` flag (their single mixed frame reads
   one byte past the C allocation, a layout-dependent value, [§3.2.5](#325-edge-cases-run-through-the-genuine-code-all-obs), row 3).
4. Bounds: `SoundCaps { max_lump: usize, max_total: usize }` (§4.8).

#### 4.6.4 Start and mix

`Mixer::start` is `I_StartSound` + `addsfx` in the C order ([§1.3.9](#139-implementation-ready-summary) pseudocode): `steptable[pitch]`
(pitch outside 0..=255 refused before any change, `MixRefusal::Pitch`; unreachable from s_sound, [§3.2.5](#325-edge-cases-run-through-the-genuine-code-all-obs),
row 12); the single-instance rule for ids 1, 10-13, 22; first free slot, else the lowest index with the
smallest `start < gametic`, else 0; handle; step, `frac = 0`, `start = gametic`; separation
`lv = v - ((v*s*s) >> 16)` with s = sep + 1, then s - 257 for `rv`; the range checks (refused before
mutation in Rust, where the C writes half a channel and then exits; unreachable from s_sound's domain,
[§1.3](#13-the-mixer-i_soundc), item 19.8).

`Mixer::mix_buffer` is i_sound.c:565-635: 512 frames, channels 0..7 in order, read, accumulate, advance,
free at `pos >= end`, clamp to i16. **The output type is `[[i16; 2]; 512]`** (left, right), the
interleaving of `mixbuffer[2k]`, `[2k+1]`. **Per tic under the declared schedule: 0 or 512 frames**
(315 on average). The compared signal is the buffer sequence, as little-endian bytes for digests.

#### 4.6.5 Refusals for the C's fatal or undefined cases

All are typed, leave the complete sound state, display state and recorder untouched (the tic's staged
sound state is dropped), and end the session on the refusal screen as M5b's frame refusals do
(app/mod.rs:774-776: the simulation has committed, the command stays in the demo, and the replay
reproduces the refusal).

| C behaviour | Rust |
| --- | --- |
| `sfx_id < 1` or `> NUMSFX` (`I_Error`; 109 passes and reads past `S_sfx`, s_sound.c:277-278) | `SoundRefusal::SfxId` for 0 and for 109..; unreachable from wire 25's ids |
| first start of an sfx whose `ds` lump is missing, once it obtains a channel (`W_GetNumForName` `I_Error`, s_sound.c:363-365) | `SoundRefusal::LumpMissing { sfx }` at that exact start (after its draw and its channel choice, as in C; the staged state is dropped) |
| a start of a non-linked sfx with padded length 0 (read past the block) | `MixRefusal::ShortLump { sfx }` at the mixer start ([§3.3.2, question 3](#332-questions-the-mixer-harness-and-sound-lump-facts-raise-for-the-owner)) |
| `rightvol`/`leftvol` out of bounds (`I_Error`) | `MixRefusal::Separation`, before mutation |
| pitch outside 0..=255 (out-of-bounds `steptable` read) | `MixRefusal::Pitch`, before mutation |
| `S_UpdateSounds` on an origin that cannot be resolved | `SoundRefusal::OriginGone` |
| a button release from slot 15 | `SoundRefusal::ButtonPastArray` (or refused in the simulation, §4.4.5) |

### 4.7 From the per-tic stream to the device

#### 4.7.1 The seam in doom-game (platform-free)

```rust
pub trait AudioSink {                                   // receives the proven stream, nothing else
    fn buffer(&mut self, frames: &[[i16; 2]; 512]);     // one mixed buffer, in order
    fn pause(&mut self) {}                              // app pause: clear queued audio (mute)
    fn resume(&mut self) {}
    fn session_start(&mut self) {}                      // restart: clear queued audio
}
pub struct RecordingSink { pub buffers: Vec<[[i16; 2]; 512]> }  // tests: digests, determinism
```

- `App::update` takes an `&mut dyn AudioSink` beside `&mut dyn Host` (or `Host` gains the method with a
  default no-op, which keeps every existing `Host` impl compiling; recommended). `TicReport` gains
  `sound: Option<&TicSound>` (built by struct literal only inside the crate, app/mod.rs:158, 772).
- **Gain and mute are applied after the proven stream**: `Gain` (an integer numerator over 256, saturating)
  sits in doom-play's queue reader or in a wrapper sink; the recording sink sees the unscaled buffers.
- **The volume launch flag**: `--volume 0-100` (percent; default 100; 0 mutes and still runs the sound
  logic, so the face stays vanilla). The C's `snd_SfxVolume` is **not** changed (§4.12 item 4, T9).
- Every headless test uses `RecordingSink` and the fake clock (crates/doom-game/tests/app.rs already
  drives `App::update` with scripted microsecond times); no test touches a device or a thread. Required
  test: the same scripted session run twice gives identical buffer digests, starts, channel tables and
  `rndindex`; and `replay` of its demo gives the same again.

#### 4.7.2 The device in doom-play

- **Thread and hand-over without `unsafe`.** The library's callback (maudio: `FnMut(CallBackDevice,
  &mut [i16]) + Send + 'static`, [§4.14](#414-audio-output-library-a-measured-comparison)) runs on its own thread. A bounded queue
  `Arc<Mutex<VecDeque<[i16; 2]>>>` holds the frames; the loop thread pushes each mixed buffer, the
  callback pops what it needs and writes zeros for the rest. The lock is held only for a copy. A
  lock-free queue would add a crate; not needed at these rates (inference: one push of 512 frames
  every 1-2 tics).
- **Rate.** Ask the device for 11,025 Hz stereo i16. maudio resamples itself when the device runs at
  another rate ([§4.14](#414-audio-output-library-a-measured-comparison), "Resampling" line); the pure-Rust PulseAudio route lets the server resample ([§4.14](#414-audio-output-library-a-measured-comparison)).
  If neither is available, doom-game provides a pure integer resampler
  (`resample::Linear { from: 11025, to: u32 }`, 16.16 phase, linear interpolation; 44,100 is an exact 4x
  and could be sample-and-hold or linear, 48,000 is not an integer ratio). It is presentation and
  outside the proven stream either way.
- **Underrun**: silence (zeros), counted for `--tic-stats`-style diagnostics. **Overrun** (the queue
  above its cap): drop the **oldest** frames, never block the loop, because blocking would change tic
  pacing, which M5b fixed and the owner has not playtested (a vanilla OSS build is paced by its blocking
  write, [§3.2.3](#323-facts-the-harness-depends-on-src), item 3; we deliberately do not copy that).
- **Latency.** The stream arrives in 512-frame steps, 0 or 1 per tic, so the queue must hold at least one
  buffer before the device starts pulling: a start of 1 buffer (46.4 ms) plus the device period gives
  roughly 50-100 ms, close to vanilla's two 2,048-byte fragments (about 93 ms, [§1.3](#13-the-mixer-i_soundc), item 30.6), plus
  the schedule's own start latency of 0-511 frames (mean 23.2 ms, [§3.2.4](#324-the-mixing-schedule-the-open-question-facts-per-candidate)). Cap: 4 buffers (186 ms).
  Not measured.
- **Pause** (the app's own pause, S12): no tics, so no buffers; the sink's `pause` clears the queue so the
  last 50-100 ms are not heard after the pause key (mute on pause). Vanilla would keep mixing during its
  own pause ([§1.1.5](#115-every-function-line-by-line), `S_PauseSound`); the app pause is not vanilla's and this is presentation.
- **Session end and restart.** An end screen stops tics, hence buffers. Restart is a new game (S11):
  fresh `Sound` (s_sound channels empty as after `S_Init`, mixer channels empty, handle counter 0,
  schedule phase 0), and the sink's `session_start` clears the queue. Vanilla's `G_InitNew` would run
  `S_Start` but leave the 8 mixer channels playing and the handle counter running ([§1.1.5](#115-every-function-line-by-line), `S_Start`;
  [§1.3](#13-the-mixer-i_soundc), item 20); that is process history, outside the declared fresh-process reference.
- **No device.** If the backend cannot open (maudio with an explicit backend list returns
  `MA_NO_BACKEND`; the PulseAudio client returns an `io::Error`, [§4.14](#414-audio-output-library-a-measured-comparison)), the game runs with the sound
  logic and mixer still running (the face and the proven stream unchanged) into a sink that discards,
  and doom-play prints one line to standard error once. Never miniaudio's Null backend ([§4.14, question 4](#414-audio-output-library-a-measured-comparison)).
- **Library independence.** doom-play has one module, `audio.rs`, with
  `fn open(volume: Gain) -> Result<DeviceSink, String>`; `DeviceSink: AudioSink` owns the queue's
  producer side. maudio or the per-OS pure-Rust route plugs in behind that function only.

### 4.8 Saved state for restore-and-continue

#### 4.8.1 What is saved

- **`DDSP` version 2** (doom-render): version 1's layout with one byte after the version, `contract`
  (0 `SoundFreeWipeFree`, 1 `WithSound`). The `rndindex` byte (d_display.rs:708) keeps its place; under
  contract 1 it includes the sound and wipe draws. The encoder writes version 1 when the contract is 0, so
  every M5b state stays byte-identical and `crates/doom-render/tests/m5b_restore.rs` is untouched; it
  writes version 2 for contract 1.
  - A version-1 reader (M5b's `decode`, d_display.rs:742-757, which refuses any other version) refuses
    version 2 with `StateDecodeError::Version(2)`. The new decoder accepts 1 (as contract 0, the v1 field
    rules unchanged) and 2. Nothing else changes in the layout.
- **`DSND` version 1** (doom-sound, its own magic, as M5b chose a separate magic for DDSP over a DRST
  version 3, [M5B §3.7]):
  - header: magic, version, the tic, `snd_SfxVolume` (8), `numChannels` (3), the map number, the
    schedule's buffers-mixed count k;
  - s_sound channels: for each of 3, free or (sfx id u8, origin class u8 + identity: `ActorId` slot u32 and
    generation u64, sector u16 or button u8, handle i32);
  - mixer: `handlenums` u16; for each of 8, free or (sfx id u8, pos u32, end u32, step u32, frac u32,
    start i32, handle i32, id i32, lvol u8, rvol u8);
  - nothing about lumps (the sound set is reloaded from the WADs) and nothing about `M_Random` (in DDSP).
- A session snapshot is then the `Simulation` clone (doom-sim has no restore from its snapshot; M5a and
  M5b clone it, m5b_restore.rs:1-8), the DDSP v2 bytes and the DSND v1 bytes. Nothing in the app calls
  restore (as in M5b); the encodings exist for tests, fuzzing and later save/load.

#### 4.8.2 Validation on load

Typed errors, bounded sizes (DSND is fixed-size apart from the free/occupied tags: at most about 300
bytes, refused beyond), and references checked against the simulation and the sound set:
`tic == sim.tic()`; k equals the number of buffers the schedule gives for tics 0..tic-1; sfx ids in
1..=108; each actor origin resolves to a live actor (`Simulation::actor`), each sector index is below the
sector count, each button slot below 15; at most one channel per origin (the [§1.1.10](#1110-order-sensitivity-within-a-tic) invariant);
`handle >= 100` and `handlenums` consistent with the largest live handle modulo the wrap; mixer
`end == padded length` of its sfx in the loaded set (or 0 for chgun), `pos <= end`, `frac <= 65535`,
`step` one of the 256 `steptable` entries, `lvol`, `rvol` <= 127, `start <= tic`. Like
`DisplayState::check` (d_display.rs:357-376), this is a consistency check, not a proof of reachability,
and the record says what it still trusts.

#### 4.8.3 Test design (as m5b_restore.rs)

For several items with sound (a door and a lift, a monster fight with refusals for lack of a channel,
a teleport, a barrel chain, a two-tic iteration that replaces a single-instance pistol before mixing):
at tic N-k encode DDSP v2 and DSND v1 from the uninterrupted run, decode, restore into a fresh `Display`
and `Sound` next to a clone of the simulation at N-k, continue N-k+1..N. Every frame, palette, face,
message, `rndindex`, start echo, s_sound channel table and buffer digest, and both encodings at N, must
equal the uninterrupted run's, and the genuine goldens. Forged states (each validation rule above) are
refused and leave the target unchanged.

### 4.9 Resources, bounds and work per tic

- **Memory.** 107 padded sounds plus one shared pistol copy per missing name. Bounded by the lumps: the
  padded length is size - 8 rounded up to 512. Caps (an over-refusal at load, declared): 1 MiB per lump
  and 16 MiB in total, `SoundLoadError::TooLarge`. Positions and ends are `u32`; the C's lengths are `int`
  ([§1.3](#13-the-mixer-i_soundc), item 7). The real DOOM.WAD and DOOM2.WAD lumps fit easily (counts only in [§3.2.8](#328-real-iwads-local-only-counts-only-obs); this section did not
  read sizes).
- **Work per tic.** Records per tic are bounded by the simulation's own event bounds (each record comes
  from a bounded emitter). Each start is O(3 + 8) work; `S_UpdateSounds` is O(3); a buffer is 512 x 8
  channel steps. No allocation per tic beyond the reused buffer and the record slice.
- **Preflight before mutation.** `Sound::tic` works on a copy of `SoundState` (about 300 bytes) and the
  app commits it only after the display's observe and draw succeed; refusals inside it never mutate the
  live state. No clone of the simulation per tic.

### 4.10 Tests, fuzz, mutation testing and CI (outline)

- **Goldens the Rust side is compared with, all from genuine source bodies and independent encoders:**
  the per-tic sound-record list (wire 25 `events.sounds` against the genuine recorder, via the sound
  comparator); the start echo (`s` rows: tic, id, vol, sep, pitch, handle, s_sound channel); the
  s_sound channel table after each `S_UpdateSounds`; `rndindex` per tic (already a P field of the M5b
  runner); the mixer's table digests, `l` rows and buffer digests (`b` rows, `z` stream digest) from
  the genuine harness of §3.2 and its independent Python model ([§3.2.9](#329-design-for-phase-2-step-2)). Committed: digests and
  rows only, never raw samples.
- **Operating systems.** Rust goldens (tables, echoes, buffer digests) in `cargo test` on Linux, Windows
  and macOS. The genuine C runners, the independent model and the generators' `--check` on the Linux job
  beside the M5b C steps.
- **Rust-only tests.** wire-24-unchanged checks (§4.4.4); the 109-row table against sounds.c; the
  tables against their digests; every refusal of §4.6.5 with state preserved; the app's determinism and
  replay tests (§4.7.1); the `RandomContract::SoundFreeWipeFree` path reproducing M5b's goldens.
- **Fuzz targets** (fuzz workspace, never doom-play): `sound_logic` (random record streams with random
  positions, angles, sfx ids, origin identities, stops and iteration ends: no panic; at most one channel
  per origin; draws <= starts; a refusal leaves the state's encoding unchanged; the same input twice gives
  the same output); `sound_mixer` (random lumps including short and missing, random in-domain starts and
  out-of-domain ones that must refuse, random buffer counts: no panic, `pos <= end`, determinism);
  `sound_state` (DSND decode and restore against a fixed simulation); the sector-origin function over
  arbitrary maps (no panic, overflow refused).
- **Mutation testing.** Add `-p doom-sound` to mutants.yml's command (today `-p doom-sim -p doom-render
  -p doom-game`); doom-play stays out. With about 2,000 new source lines in doom-sound and doom-sim, expect
  roughly 800-1,200 mutants (M5b's runs had 1,109 to 1,237 mutants over 8 shards); keep shards under
  about 40 minutes by raising the shard count in its one place (mutants.yml:42-46). doom-sound's tests
  are fast (no frames), which is the main reason for the separate crate.

### 4.11 Size estimate (hand-written lines, ±30%)

| Component | Lines | Basis |
| --- | --- | --- |
| doom-sim wire 25 (variant, predicates, records at about 25 emitters, `events.sounds` group, DSIM41 decoder, `soundorg`, button reading, scream, truncation) | 900 | wire-24-style predicates are small; the emitter plumbing dominates (§4.4.4) |
| doom-sound `sounds` + `tables` | 250 | 109 table rows plus generation and checks |
| doom-sound `lumps` | 150 | i_sound.c:185-251, 799-813 plus caps and flags |
| doom-sound `mixer` | 250 | i_sound.c:264-381, 539-654 (about 200 C lines) |
| doom-sound `logic` | 400 | s_sound.c's used part (about 350 C lines) plus refusals |
| doom-sound `schedule`, `session` | 250 | transactional tic, iteration end, echo |
| doom-sound `state` (DSND encode, decode, validate) | 350 | DDSP's encode and decode are about 200 lines for a similar field count (d_display.rs:689-830) plus the checks |
| doom-render additions (contract, `observe_after`, wipe draws, DDSP v2) | 150 | additive |
| doom-game wiring (sound mode, sink trait, recording sink, gain, report, replay) | 400 | app/mod.rs is 965 lines for the whole loop |
| doom-game integer resampler | 100 | only if no library resamples |
| doom-play audio module and flags | 300 | depends on the library; thin by design |
| Tests (doom-sim wire 25, doom-sound unit and golden tests, restore, app determinism) | 3,200 | M5b wrote about 5,600 test lines for its src |
| Fuzz targets | 400 | four targets; M5b's two came to 556 lines |
| **Total in crates** | **≈ 7,100** | |
| Tools outside the crates (the genuine sound-logic runner and mixer harness lifted from the prototypes, the independent models, the DSIM41 encoder, the sound comparator, the synthetic sound generator) | ≈ 2,800 | proto-mixer's harness and model plus proto-soundlogic's runner (not read, see below) |
| **Grand total** | **≈ 9,900** | |

**What M5b taught.** M5b estimated about 6,700 lines in crates. At head 7f7fd99 the M5b code is about
7,100 lines of source (doom-render's six new modules 2,737, doom-game src 3,254, doom-play 1,074, all by
`wc -l`, comments included) and about 6,100 lines of tests and fuzz (doom-game tests and doom-render's
m5b tests 5,569; the two fuzz targets 556): roughly 2x overall, about 1.5x for source and 3x for tests.
Applied here: source 3,500 x 1.5 = about 5,200, tests 3,600 x 2-3 = about 7,000-10,000, so a realistic
range for the crates is **7,100-15,000**, the tests being the uncertain half.

**What can be dropped first if the cap is passed** (the brief's fallback keeps the mixer's sample-exact
comparison and audio output; the melt is already out of this design):

1. the integer resampler (rely on the library's resampling);
2. the DSND restore-and-continue (encode, decode and restore tests), keeping the transactional tic;
3. the button-release LP64 modelling, replaced by option (b) of §4.4.5 (refuse when slot i+1 is busy);
4. the THING truncation (it is a separate yes/no and can move to its own wire later);
5. DSIM41 canonicality, falling back to option (b) gated to wire 25 (§4.4.3).

### 4.12 Stop-and-ask candidates

Each is a concrete either/or with a recommendation, except the last, which is a correction. The number at the start of an item is the decision of §5 that carries it.

1. **T2 Reference configuration.** Integrated (doomdef.h:84 removed) for both the sound logic's handles
   and the samples, or the shipped `SNDSERV` build (handle = sfx id, no in-process mixer). Recommend
   **integrated**, declared everywhere ([§1.1.14, question 1](#1114-questions-the-sound-logics-facts-raise-for-the-owner), [§1.3.11, question 1](#1311-questions-the-mixers-facts-raise-for-the-owner) and [§3.3.2, question 1](#332-questions-the-mixer-harness-and-sound-lump-facts-raise-for-the-owner) agree).
2. **T5 Schedule.** (a) the buffer rule of [§3.2.4](#324-the-mixing-schedule-the-open-question-facts-per-candidate) (buffer k after tic t when `512k < 315(t+1)`; first
   buffer after tic 0) with `S_UpdateSounds` once per iteration; or (b) the floor form of that rule
   (first buffer after tic 1, wipe at k = 2) with `S_UpdateSounds` after every tic; or (c) one 512-frame
   buffer per tic (`-timedemo`; 1.625x real time, not playable). Recommend **(a)**; (b) puts the first
   display after tic 1 instead of tic 0 and is a loop no vanilla build runs (§4.3.3).
3. **T6 Wipe.** (a) 320 draws after the first frame, melt not drawn and taking no time; or (b) keep M5b's
   wipe-free contract ([§3.3.2, question 4](#332-questions-the-mixer-harness-and-sound-lump-facts-raise-for-the-owner)). Recommend **(a)**: without it the glance stays 64 indices off
   vanilla and S15 does not close ([§1.4.10, question 1](#1410-questions-the-m_random-and-wipe-facts-raise-for-the-owner)).
4. **T9 Volume.** (a) `snd_SfxVolume` fixed at 8 in the sound logic, the player's volume and mute a gain on
   the finished stream; or (b) a launch flag that sets `snd_SfxVolume` (0-15) as vanilla's menu does,
   which changes audibility and so the draws and the face ([§1.1.12](#1112-implementation-ready-summary) "Configuration flags"). Recommend
   **(a)**, with `--volume 0-100`.
5. **T3 How sound starts leave the simulation.** (c) wire 25 with canonical `events.sounds`; (b) wire 25
   with a non-canonical buffer; (a) app reconstruction; (d) a declared limit. Recommend **(c)** (§4.4.7).
6. **T4 THING truncation in wire 25.** Yes or no. Recommend **yes** (one predicate, one synthetic WAD
   through the existing registered harness, no real-run digest change).
7. **T12 Button-release origin.** (a) LP64 reading, (b) (0,0) and refuse when slot i+1 is busy, (c) ILP32.
   Recommend **(a)**, slot 15 refused.
8. **T11 Short and missing lumps.** Refuse the start (typed witness at the exact start), or refuse the WAD
   at load. Recommend **refuse the start** for both; the genuine code accepts the WAD ([§3.3.2, question 3](#332-questions-the-mixer-harness-and-sound-lump-facts-raise-for-the-owner); [§1.1.14, question 4](#1114-questions-the-sound-logics-facts-raise-for-the-owner)).
9. **T15 Restart and session start.** Fresh process (s_sound and mixer channels empty, handle counter 0,
   `gametic` 0), or carry mixer channels and handles across a restart as vanilla's `G_InitNew` would.
   Recommend **fresh process**, consistent with S11.
10. **T15 Default contract of the app.** doom-play runs `WithSound` (wire 25) by default, the M5b contract
    staying a library configuration for tests only; or a launch flag that selects the M5b contract.
    Recommend **library-only**; `--volume 0` is the way to play silently with the vanilla face.
11. **T8 Overrun policy.** Drop the oldest queued frames (never block), or block the loop as vanilla's
    OSS write does. Recommend **drop**, because blocking changes pacing, which the owner has not
    playtested.
12. **T7 Library.** maudio with an explicit backend list, or the pure-Rust per-OS route ([§4.14, questions 1-4](#414-audio-output-library-a-measured-comparison)).
    Recommend maudio; the seam of §4.7.2 makes the choice local to doom-play.
13. **A correction to §2.4, not a decision.** The damage-chase and `A_Look` step positions (4.4.2 items 2-3) contradict [§2.4](#24-positions-at-the-moment-of-the-call)'s statement
    that positions change only in an actor's own thinker. No decision, but the note should record the
    correction, because it is the reason (a) cannot be shown exact cheaply.

### 4.13 Risks

1. **Staged-state reads in the emitters** (§4.4.4). A position read from the store instead of the
   staged plan gives a plausible but wrong audibility, a wrong draw count and a face that differs only
   occasionally. Only the genuine record comparison on the 61 runs and the synthetic items catches it;
   it must run before any face claim.
2. **The never-validated wire-24 sound events** ([§2.6](#26-what-has-been-validated-by-evidence)): wire 25 inherits them; the first genuine
   comparison may find order or identity faults in wire 24 itself. Those cannot be fixed in wire 24
   (frozen) and would need a wire-25 predicate each, enlarging "only what sound needs". The owner should
   know that this is possible before ruling T3.
3. **Schedule disagreement** (T5): a later change of the rule changes every buffer digest and many
   start echoes. Decide before Phase 2 goldens are generated.
4. **Audio library maturity** (maudio's young binding, unverified MSVC `c99`, no sound heard on this
   machine, [§4.14](#414-audio-output-library-a-measured-comparison)); the device path cannot be tested in CI. Keep it thin and behind the seam.
5. **Mutation-testing time** grows with doom-sim's wire-25 diff (doom-sim's suite is slow); size shards
   early.
6. **Size**: the M5b ratio suggests the crates could reach 15,000 lines; the fallback list of §4.11
   starts with items that do not touch the proven stream.
7. **Presentation feel** (latency, underrun clicks, gain) is unmeasured and unplaytested; nothing in this
   design changes input, pacing or the picture, but the owner will hear the result for the first time.

### 4.14 Audio output library: a measured comparison

Measured 2026-11-18, 02:27 to 02:41 (host clock, America/Los_Angeles), on the owner's WSL2 Ubuntu 26.04.1 (WSLg 1.0.79), `cargo 1.100.0` (stable) and `rustc 1.85.0`. Trial projects are in `/home/copgunn/scratch/m5c/audio-libs/<candidate>/`, and the logs are in `/home/copgunn/scratch/m5c/audio-libs/logs/`. Every trial uses `edition = "2024"`, `rust-version = "1.85"`, `resolver = "3"` and `#![forbid(unsafe_code)]`. Crates were fetched into the normal `~/.cargo` registry cache. `CARGO_TARGET_DIR` was under the scratch directory and `CARGO_BUILD_JOBS=4`; every target directory was deleted afterwards (1.7 GB). `df -h /mnt/c` showed 28 GB available before and after. Release dates come from the crates.io API (`https://crates.io/api/v1/crates/<name>`, read 2026-11-18).

This part is about the playable binary's dependencies, not about the C source. It therefore cites no linuxdoom-1.10 lines. The output format it serves (signed 16-bit stereo at 11,025 Hz, 315 frames per tic) is defined by the mixer parts of this note.

**Nobody heard anything.** Every run wrote only all-zero samples. On this machine no run could have reached a loudspeaker except the pure-Rust PulseAudio trial. Even that one sent zeros to WSLg's sink, and nobody was listening. Whether any candidate produces audible, correct sound is unverified.

#### What this machine has (read-only checks)

1. `echo $PULSE_SERVER` gives `unix:/mnt/wslg/PulseServer`. `/mnt/wslg/` contains `PulseServer`, `PulseAudioRDPSink` and `PulseAudioRDPSource` sockets. `/mnt/wslg/versions.txt` names WSLg 1.0.79 with a bundled pulseaudio build.
2. `ldconfig -p | grep -i -E "asound|pulse|pipewire|jack"` printed **nothing**. `ls /usr/lib/x86_64-linux-gnu | grep -i -E "asound|pulse|pipewire|jack|sdl|openal|portaudio"` and `dpkg -l | grep ...` also printed nothing.
   - **No audio client library is installed at all**: no `libasound.so.2` (ALSA's runtime library), no `libpulse.so.0` (PulseAudio's client library), no PipeWire, no JACK and no SDL.
   - This differs from a typical desktop Ubuntu.
3. `pkg-config` and `pkgconf` are installed (`/usr/bin/pkg-config`). `pkg-config --exists alsa` exits 1, and `/usr/include/alsa` does not exist, so the ALSA development files are absent. `cmake` is absent and libclang is absent. `gcc`/`cc` are present.
4. Installed Rust: the toolchains are stable, nightly and 1.85.0. The only installed target is `x86_64-unknown-linux-gnu`. The Windows and macOS targets are **not installed**. Nothing was compiled for them: their trees and licences come from `cargo tree`/`cargo metadata` resolution, and their build needs come from reading manifests and build scripts.

#### Summary

| | maudio 0.1.15 (miniaudio 0.11.23, C, vendored) | pure-Rust `pulseaudio` 0.3.0 on Linux + `wasapi` 0.27.0 + `coreaudio-rs` 0.14.2 | cpal 0.18.2 (and rodio on it) | tinyaudio 2.0.0 | sdl2 0.38.0 audio (C SDL2) | cubeb 0.38.0 (C++ libcubeb) |
|---|---|---|---|---|---|---|
| Pure Rust or C | Rust bindings to a C library (95,649-line `miniaudio.h`), compiled by `cc` in the build script | Pure Rust (wasapi/coreaudio-rs wrap OS APIs) | Pure Rust bindings to OS APIs (ALSA linked on Linux) | Rust bindings: ALSA or libpulse on Linux, DirectSound, CoreAudio via bindgen | Rust bindings to C SDL2 | Rust bindings to C++ libcubeb plus nested Rust backends, built by cmake |
| Licence vs GPL-2.0-only | None flagged (0 of 5). miniaudio itself is `Unlicense` or `MIT-0` (its LICENSE file); stb_vorbis (header part compiled) is MIT or public domain | None flagged (0 of 51) | **cpal is `Apache-2.0` alone** (LICENSE file is the Apache text) on all 4 targets; it cannot be trimmed by features. rodio inherits it | `clang-sys` `Apache-2.0`, macOS only, **build-time only** (bindgen tool, not linked into the binary) | None flagged (0 of 7); SDL2 is Zlib | None flagged in cargo metadata (0 of 9); libcubeb ISC, speex resampler BSD-style. **cmake runs nested `cargo build`s whose crates are invisible to `cargo metadata`, so the census is incomplete** |
| Build needs on CI with no system packages | A C compiler only (gcc on ubuntu, MSVC on windows, Xcode clang on macOS); pre-generated bindings ship for all 4 targets, so no bindgen/libclang. Linux: no dev package needed (observed) | Nothing (Linux observed; Windows/macOS from manifests: no `links`, no C) | Linux: **`libasound2-dev` via pkg-config** (observed failure). Windows/macOS: nothing extra expected | Linux: `libasound2-dev` (observed failure), or with `pulse` the runtime `libpulse.so.0` at link time (observed failure). macOS: **libclang for bindgen** | `libsdl2-dev`, or cmake for `bundled` (M5b; rechecked: link fails) | **cmake** and a C++ compiler on every OS (observed failure: `is cmake not installed?`) |
| Runtime on Linux | dlopen of `libpulse.so(.0)`, `libasound.so(.2)`, `libjack.so(.0)`; binary needs only libc/libm/libgcc_s (observed with `ldd`) | Talks the PulseAudio wire protocol over the Unix socket; no library (observed `ldd`) | DT_NEEDED `libasound.so.2`: binary will not start without it (inference; could not link here) | Same as cpal (ALSA) or `libpulse.so.0` | `libSDL2-2.0.so.0` needed to start; SDL then dlopens ALSA/Pulse | static libcubeb; backends load system libs |
| MSRV 1.85.0 | Builds (declares 1.70) | `pulseaudio` **0.3.2 and 0.3.1 fail on 1.85.0** (let-chains, `reactor.rs:172`); **0.3.0 builds**. wasapi declares 1.85 | cpal declares 1.85; type-checks on 1.85.0. rodio 0.23.0 requires 1.87; resolver picks rodio 0.21.1 on cpal 0.16.0 | Declares 1.56; not built (ALSA) | Type-checks; cannot link | Not reached (cmake) |
| Crates (union of 4 targets; new vs current Cargo.lock) | 5; 3 new (maudio, maudio-sys, cc) | 51; 45 new (Linux 28, Windows 23, macOS 10 each) | 34; 27 new (rodio 0.23: 43) | 28 (macOS 25 is bindgen's build tree) | 7; 5 new | 9; 6 new |
| API for our use | Pull callback `FnMut(CallBackDevice, &mut [i16]) + Send`, i16 stereo at a requested rate; **miniaudio resamples** to the device rate; period hint in frames | Linux: push protocol (server sends `Request(length)`; we write bytes); **PulseAudio server resamples**. Windows/macOS: our own two backends | Pull callback with `&mut [i16]`; **no resampling in cpal** (WASAPI output uses AUTOCONVERTPCM; ALSA depends on the plug layer) | Pull callback **f32 only**, 11,025 Hz accepted by parameter | Pull callback `AudioCallback`, i16; SDL resamples | Pull callback; resampler inside |
| No-device behaviour | **Default backend list ends with `Null`, so open succeeds silently on a machine with no audio** (observed). With an explicit list (`preferred_backends`) it fails cleanly: `MA_NO_BACKEND (-203)` (observed) | Linux: socket connect fails with an ordinary `io::Error` | Fails at program start if libasound is missing (inference) | Same | Fails at program start without libSDL2 | Not reached |
| Ran here under WSLg | **Ran, exit 0, but only on miniaudio's Null device**: no libpulse/libasound here. 38 callbacks, 11,970 frames in 1.03 s | **Ran, exit 0: reached WSLg's PulseAudio** (server `pulseaudio 17.0-25-gc3305`, sink `RDPSink` at 44,100 Hz); 11,025 frames sent in 25 requests, drained in 0.99 s | Could not build (no ALSA dev files) | Could not build | Could not link | Could not build |
| Maintenance (latest release) | 16 releases since 2026-02-16, latest 2026-09-28; one maintainer; 23,760 downloads | pulseaudio: 2025-08-02 (0.3.0), latest 0.3.2 on 2026-10-16; wasapi 2026-11-06; coreaudio-rs 2026-04-29 | 2026-08-16; 29.5 M downloads | 2025-11-15 | 2025-07-12 | 2026-08-18 |

Other crates looked at and set aside (details below): `quad-snd` (no streaming API, fixed 44,100 Hz, `hound` `Apache-2.0`), `oxideav-sysaudio` (needs Rust 1.88, 133 downloads, failed cleanly here), `oxisound`/`oxisound-pulse` (`Apache-2.0`, Rust 1.89), `libpulse-binding` (needs `libpulse.so.0` to link), `alsa` (needs ALSA dev files), `pulse-simple` (`LGPL-3.0`), `rustaudio` (raw FFI only), miniquad (no audio API), sdl3 (does not compile on 1.85.0, M5b §3.12).

Facts that look decisive:

1. **cpal, the de facto standard, is `Apache-2.0` alone**, and so is every crate built on it (rodio, kira, awedio, oxisound's "pure" path). That puts it with winit in S2: an owner licence decision.
2. **maudio is the only measured cross-platform option that is clean against GPL-2.0-only, builds on 1.85.0 with no system package, and gives the binary no system-library dependency at start-up.** It needs a C compiler, compiles about 95,000 lines of vendored C, is young with a single maintainer, and silently falls back to a Null device unless told otherwise.
3. **On this WSL install, nothing that loads a system audio library can make sound**, because none is installed. Only a pure-Rust PulseAudio protocol client reached WSLg's sound server. With maudio, WSLg sound needs the runtime package `libpulse0` (no `-dev`); this is inference from the dlopen names and was not tried, since nothing may be installed.

#### Common method

- **Resolve:** `cargo generate-lockfile` in each trial. Resolver 3 printed "Locking N packages to highest Rust 1.85 compatible versions"; where it had to exceed 1.85 it said so (rodio 0.23.0 needs 1.87; libloading 0.9.1 needs 1.88 for oxideav). Then `cargo fetch --locked`.
- **Census:** `/home/copgunn/scratch/m5c/audio-libs/tools/analyze.py <project> /home/copgunn/src/DOOM/Cargo.lock` (output `logs/census.txt`). It counts as §3.12 did:
  - `cargo tree --locked --target T -e normal,build --prefix none --format '{p}'` for T in `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin` and `aarch64-apple-darwin`; unique name+version, trial package excluded. Normal-only uses `-e normal`.
  - "New" means not already a `name`+`version` in the repository's `Cargo.lock` at 7f7fd99 (46 packages).
  - Licences come from `cargo metadata --format-version 1 --filter-platform T --locked`. The script treats MIT, MIT-0, BSD-2/3-Clause, ISC, Zlib, 0BSD, Unlicense, CC0-1.0, BSL-1.0 and Unicode-3.0/DFS-2016 as compatible. `OR` passes if one branch does, `AND` needs every part, and `/` is the old spelling of `OR`.
  - No crate in any tree lacked a `license` field.
- **Builds:** `cargo +stable build --locked` and `cargo +1.85.0 build --locked` (logs in `logs/build/`).
  - A first pass with an empty `main` linked SDL2 and libpulse "successfully" because unused crates are not linked. Those results were discarded, and every reported link result uses a program that calls the library.
- **Type-check without the ALSA dev files (cpal only):** a scratch-only `cpal/.cargo/config.toml` overrides the `links = "alsa"` build script (`[target.x86_64-unknown-linux-gnu.alsa] rustc-link-lib = ["asound"]`), so `cargo check` runs. Linking still fails.
- **Runs:** `timeout 20 <binary>`. Every callback or write fills all-zero samples. The maudio and oxideav trials also count non-zero samples in the buffer the library hands over before filling; the maudio count was 0.
- **Threads:** after the runs no background job was left; every command was foreground with `timeout`.

#### maudio 0.1.15 (miniaudio 0.11.23)

Source: `/home/copgunn/scratch/m5c/audio-libs/maudio/` (trial `src/main.rs`, API sketch `examples/sketch.rs`); run log `logs/run-maudio.txt`.

- **What it is:** safe Rust wrappers (maudio, MIT) over raw bindings (maudio-sys, MIT, `links = "miniaudio"`).
  - The bindings ship the miniaudio C library: `native/miniaudio/`, 13 MB of sources. Only `native/miniaudio.c` and `miniaudio_version_check.c` are compiled.
  - `miniaudio.c` includes `extras/stb_vorbis.c` as a header only, unless the `vorbis` feature is on.
  - Licences:
    - The miniaudio LICENSE offers "ALTERNATIVE 1 - Public Domain (www.unlicense.org)" or "ALTERNATIVE 2 - MIT No Attribution".
    - stb_vorbis's end-of-file licence is "ALTERNATIVE A - MIT License" or public domain.
    - maudio-sys's LICENSE is MIT ("Copyright 2025 David Reid, Copyright 2026 Cristian Roman").
- **Build:**
  - `maudio-sys/build.rs` compiles miniaudio with `cc` (C99), with `MA_NO_VORBIS` and backend switches from features (`no-alsa`, `no-pulseaudio`, `no-jack` and so on).
  - It copies **pre-generated bindings** from `src/pregen_bindings/miniaudio-0.11.23/<target>.rs`, and these exist for `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin` and 12 others. bindgen is used only with the optional `generate-bindings` feature, so no libclang is needed.
  - Observed: `cargo +1.85.0 build` 8.1 s and stable 7.4 s (debug, warm). Stable `cargo build --release` took **21.2 s** clean. The release binary is 0.74 MB.
  - The README says "LLVM / Clang — required to compile the C library and to generate Rust bindings; on Windows, this means installing LLVM (clang) in addition to Visual Studio". The build script does not need clang when the bindings are pre-generated: plain `cc::Build` picks MSVC `cl.exe` on Windows. That is inference from `build.rs`; nothing was compiled for Windows.
  - Risk: `cc_builder.std("c99")`, and cc's own docs (`cc-1.11.2/src/lib.rs:1231-1232`) say MSVC does not support `c99`. Whether `cl.exe` only warns (unknown option) or fails is **not verified**. maudio's README claims `x86_64-pc-windows-msvc` "Tested".
  - The repository already compiles C on the Linux CI runner (the C reference generators), and the existing `windows-artifact.yml` would compile miniaudio with MSVC.
- **Tree:** union 5 on every target: maudio and maudio-sys (normal); cc, shlex and find-msvc-tools (build-only). 3 are new to the lock (cc 1.11.2, maudio, maudio-sys). Licences: `MIT OR Apache-2.0` 3 and `MIT` 2. **Flagged: none.** One build-script crate (maudio-sys), one `links`.
- **Runtime (observed):**
  - `ldd` of the trial lists only libgcc_s, libm and libc. miniaudio loads backends with dlopen: the strings in `miniaudio.h` are `"libasound.so.2"`, `"libasound.so"` (29972-29973), `"libpulse.so"`, `"libpulse.so.0"` (32511-32512), `"libjack.so"` and `"libjack.so.0"` (33313-33314).
  - So the binary starts on a machine without any audio library.
  - Header §2.3: "The Linux build only requires linking to -ldl, -lpthread and -lm. You do not need any development packages."
  - Header §2.1/2.2: Windows and macOS build "without the need to ... link to any libraries", and macOS frameworks are loaded at run time. Apple notarisation may then need entitlements or `MA_NO_RUNTIME_LINKING` (header lines 484-497).
- **The Null backend:**
  - `ma_context_init` iterates over all backends up to `ma_backend_null` when none is given (`miniaudio.h` 43151-43160, comment at 6739: "Must always be the last item. Lowest priority").
  - maudio has no feature to compile Null out. A context built with `ContextBuilder::preferred_backends([...])` excludes it.
- **Trial results (WSLg, `timeout 20`, both exit 0):**
  - `MODE=default`: the log printed "Failed to load library" for each of the six names, then `[Null] NULL Playback Device`, with format `16-bit Signed Integer -> 16-bit Signed Integer`, channels `2 -> 2`, sample rate `11025 -> 11025`, buffer size `315*3 (945)`, and resampling `NO` (passthrough). The run made 38 callbacks and delivered 11,970 frames in 1.031 s, with 0 non-zero samples in the pre-silenced buffers. A rerun on the 1.85.0 build gave 37 callbacks, 11,655 frames, 1.008 s.
  - `MODE=nonull` (PulseAudio, ALSA, JACK, WASAPI, CoreAudio only): `context init failed: MaudioError { native: None, ma_result: MA_NO_BACKEND, (-203) }`.
  - So a real device was never opened. The Null device is timer-driven and discards the samples.
- **API (from source, sketch type-checked on 1.85.0 with `forbid(unsafe_code)`):**
  - `DeviceBuilder::playback().i16().playback_channels(2).sample_rate(SampleRate::Sr11025).period_size_frames(315).context(&ctx).with_callback(|_, out: &mut [i16]| ...)`. The callback bound is `FnMut(CallBackDevice, &mut [i16]) + Send + 'static` (`device_builder.rs:1137-1139`).
  - `SampleRate` has `Sr11025` and `Custom(u32)`. When the device runs at another rate, miniaudio converts: the log's "Resampling" line shows whether it does.
  - The period is a hint ("may be adjusted by the backend", `device_builder.rs` doc of `period_size_frames`).
  - The output buffer is pre-silenced by default (`pre_silenced_output`, "Enabled by default").
  - Pause is `device_stop()`/`device_start()`; mute is `set_master_volume(0.0)` or writing zeros.
  - Panics in the callback are trapped and poison the callback (`data_callback_panicked()`).
  - Underrun handling is ours: the sketch writes zeros when the queue is empty.
  - Hand-over without unsafe: `Arc<Mutex<VecDeque<i16>>>` capped at eight tics (sketch), or a channel.
  - A mutex on the audio thread risks priority inversion; a lock-free single-producer queue would add a crate.
  - The doc comment on `with_callback` says callbacks "should always fully initialize their output buffer or will cause undefined behaviour". With pre-silencing on, the slice is already initialised; the wording is the crate's.
- **Risks:**
  - It is young: first release 2026-02-16, 16 releases, last 2026-09-28, one maintainer, 23,760 downloads.
  - It has 1,070 `unsafe` occurrences in its source (FFI wrappers; not audited).
  - It is locked to miniaudio 0.11.23, and its README says "miniaudio does not guarantee a stable ABI ... The same applies to maudio".
  - The miniaudio C library itself is mature and widely used (not measured here).
  - Behaviour with no device is safe only if we exclude Null. CI tests must never open a device anyway (recording sink).

#### Pure-Rust PulseAudio client (`pulseaudio` crate) plus per-OS crates

Source: `/home/copgunn/scratch/m5c/audio-libs/pulseaudio-old/` (trial `src/main.rs`), combined census in `perOS/`; run log `logs/run-pulseaudio.txt`.

- **Versions:**
  - `pulseaudio` (colinmarc, MIT, no `rust-version`). 0.3.2 (2026-10-16) and 0.3.1 fail on 1.85.0 with `error[E0658]: let expressions in this position are unstable` at `pulseaudio-0.3.2/src/client/reactor.rs:172-173`.
  - Because the crate declares no `rust-version`, resolver 3 cannot avoid that failure.
  - 0.3.0 (2025-08-02) and 0.2.1 build on 1.85.0. The trial pins `=0.3.0`.
  - cpal's own `pulseaudio` feature depends on `pulseaudio = "0.3"`, which would pick 0.3.2.
- **Tree:** `pulseaudio` 0.3.0 alone has a union of 30 (Linux 28) and none flagged. The per-OS combination (`pulseaudio` on Linux, `wasapi` 0.27.0 on Windows, `coreaudio-rs` 0.14.2 on macOS, as target-gated dependencies) has a union of **51**, 45 new: Linux 28, Windows 23, macOS 10 each. Licences: `MIT OR Apache-2.0` 33, `MIT` 8, `Zlib OR Apache-2.0 OR MIT` 4, `Apache-2.0 OR MIT` 2, `Unlicense OR MIT` 2, `(MIT OR Apache-2.0) AND Unicode-3.0` 1, `MIT/Apache-2.0` 1. **Flagged: none.** No `links`, no C.
- **Build:** 1.85.0 release build 11.4 s clean, binary 0.75 MB. `ldd`: libgcc_s, libm, libc only.
- **Trial (WSLg, exit 0):**
  - Connected to `/mnt/wslg/PulseServer`.
  - The first attempt sent an empty cookie and the server logged `protocol-native.c: protocol error, kicking client` in `/mnt/wslg/pulseaudio.log`. The second sent a 256-byte zero cookie (no cookie file exists) and authenticated: protocol version 35, server `pulseaudio` `17.0-25-gc3305`, default sink `RDPSink`, server spec S16LE 2 channels **44,100 Hz**.
  - A stream at S16LE/2/**11,025** was created on RDPSink, so the server resamples.
  - The second attempt asked for no buffer attributes and got `target_length 88200`, `pre_buffering 87320` (2 s). One second of data never reached pre-buffering, the read blocked, and `timeout` ended it (exit 124). The trial then requested a four-tic buffer: target 5,040 bytes, pre-buffer 1,260, `adjust_latency`.
  - The server granted `target_length 3780` (945 frames, about 86 ms), `pre_buffering 1260`, `minimum_request_length 884`; `Started` arrived after 16 ms.
  - 44,100 bytes (11,025 frames) were sent in 25 requests. `DrainPlaybackStream` completed after 0.987 s. No `Underflow` message.
- **API:** the sync low-level API (`protocol::write_command_message`, `read_command_message`, `write_memblock`) is safe in our code.
  - It is a push protocol clocked by the server's `Request` messages. We would own a thread that answers requests from the tic queue.
  - Pause: `CorkPlaybackStream`. Mute: zeros, or stream volume.
  - The crate has an async layer (futures, mio) that we would not need, but it is in the tree.
  - It is Linux/Unix only.
- **Windows and macOS halves (not built: targets not installed):**
  - `wasapi` 0.27.0 (MIT, `rust-version = "1.85"`, released 2026-11-06): 152 `pub fn`, 2 `pub unsafe fn`, and `StreamMode::EventsShared { autoconvert, .. }`, which uses `AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM` so Windows resamples.
  - `coreaudio-rs` 0.14.2 (MIT/Apache-2.0): `AudioUnit::set_render_callback` is safe; `set_property`/`get_property` free functions are `unsafe` (`audio_unit/mod.rs:423,461`).
  - Neither was compiled. "Our own three backends" is new code of unknown size in the binary crate, and each OS backend needs its own test on real hardware.
- **No device:** Linux fails at `UnixStream::connect` or with `socket_path_from_env() == None`. Both are ordinary errors.
- **Risk:** a small crate (324,422 downloads, one author) with an MSRV trap in its newer releases. Three code paths. Linux machines without a PulseAudio or PipeWire-pulse server (pure ALSA) would get no sound.

#### cpal 0.18.2 (and rodio)

Source: `/home/copgunn/scratch/m5c/audio-libs/cpal/`, `cpal-pulse/`, `rodio/`, `rodio-msrv/`.

- **Licence:** `license = "Apache-2.0"`, and the LICENSE file begins "Apache License Version 2.0, January 2004", on all four targets. 0.17.x and earlier on crates.io are also `Apache-2.0`. **Flagged; no feature removes cpal itself.**
- **Linux build:** `alsa` and `alsa-sys` are unconditional dependencies (`cpal-0.18.2/Cargo.toml:208-211`). `alsa-sys-0.4.0/build.rs` panics when `pkg-config --libs --cflags alsa` fails, with the hint "apt-get install libasound2-dev". Observed on stable and on 1.85.0, with and without the `pulseaudio` feature.
  - With the scratch-only links override, `cargo check` passed on stable and on 1.85.0. Linking failed: `rust-lld: error: unable to find library -lasound`.
- **Tree:** union 34 (Linux 8, Windows 17, macOS 15), 27 new. With the `pulseaudio` feature: union 56. Its `pulseaudio` 0.3.2 would not build on 1.85.0 either (above).
- **rodio:** 0.23.0 is `MIT OR Apache-2.0` but `rust-version = 1.87`. Without a pin the resolver picks 0.21.1 on cpal 0.16.0 (also `Apache-2.0`). The rodio 0.23.0 union is 43.
- **API (source):** `build_output_stream(config, FnMut(&mut [T], &OutputCallbackInfo) + Send, FnMut(Error) + Send, Option<Duration>)`, `StreamConfig { channels, sample_rate: u32, buffer_size: BufferSize::Fixed(n) }`, and `stream.play()`/`pause()`.
  - cpal does not resample.
  - The WASAPI output path sets `AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | SRC_DEFAULT_QUALITY` (`host/wasapi/device.rs:975-976`). On ALSA, 11,025 Hz works only through the `default`/plug device.
  - Type-checked sketch: `cpal/src/main.rs`.
- **Runtime:** DT_NEEDED `libasound.so.2`, so the binary would not start on a Linux machine without libasound2 (inference: not linked here).
- **Maintenance:** latest 2026-08-16, 29.5 M downloads, RustAudio organisation.

#### tinyaudio 2.0.0

Source: `/home/copgunn/scratch/m5c/audio-libs/tinyaudio/`, `tinyaudio-pulse/`.

- MIT, `rust-version = "1.56"`, released 2025-11-15.
- The callback is `FnMut(&mut [f32]) + Send + 'static` only (f32, no i16). `OutputDeviceParameters { sample_rate, channels_count, channel_sample_count }`.
- Backends: Linux ALSA (default feature, `alsa-sys` 0.3.1) or `pulse` (`libpulse-sys`); Windows DirectSound (winapi); macOS `coreaudio-sys` 0.2.18, whose `build.rs` runs **bindgen**, so libclang is needed at build time on macOS. `clang-sys` there is `Apache-2.0`, build-only.
- Observed:
  - The default build fails at `alsa-sys` pkg-config.
  - `features = ["pulse"]` fails at link with `unable to find library -l:libpulse.so.0`. libpulse-sys links the runtime soname, so the runtime package `libpulse0` suffices to link, but it is absent here.
  - Both happened on stable and on 1.85.0.
- Union 28; Linux 4 crates.

#### sdl2 0.38.0, audio subsystem

Source: `/home/copgunn/scratch/m5c/audio-libs/sdl2/`. Facts from M5b §3.12 (licences `MIT`/`MIT AND Zlib`, tree 7, `bundled` needs cmake) still hold.

- Rechecked with a program that opens playback (`AudioSpecDesired { freq: Some(11_025), channels: Some(2), samples: Some(512) }`, `AudioCallback` with `Channel = i16`). `cargo check` passes. Build fails on stable (`rust-lld: error: unable to find library -lSDL2`) and on 1.85.0 (`ld.bfd: cannot find -lSDL2`).
- SDL2's own CMake has `SDL_ALSA_SHARED` and `SDL_PULSEAUDIO_SHARED` default ON (`sdl2-sys-0.38.0/SDL/CMakeLists.txt:464,472`), so SDL dlopens ALSA and Pulse.
- SDL converts and resamples in its audio stream (`SDL_audio.c`).
- Using SDL2 only for audio would add a C library at build time and at run time next to miniquad.

#### cubeb 0.38.0

Source: `/home/copgunn/scratch/m5c/audio-libs/cubeb/`.

- ISC (Mozilla). `cubeb-sys/build.rs` builds the vendored libcubeb with **cmake**, with `-DBUILD_RUST_LIBS=ON`, which runs nested `cargo build` for `cubeb-pulse-rs` and `cubeb-coreaudio-rs` (`libcubeb/CMakeLists.txt:382,405`). Those have their own `Cargo.lock`, outside our lock and outside `cargo metadata`.
- Observed: `failed to execute command: No such file or directory ... is cmake not installed?` (stable and 1.85.0).
- Not a fit for "no system packages" or for a complete licence census.

#### Others

- **quad-snd 0.2.8** (from miniquad's author; `MIT/Apache-2.0`; last release 2024-05-21). It is not a stream API: it loads WAV/Ogg files (`Sound::load`) and mixes them at a fixed 44,100 Hz (`alsa_snd.rs:13`, `wasapi_snd.rs:63`, `coreaudio_snd.rs:19`).
  - It depends unconditionally on `audrey`, which pulls `hound` 3.5.1, `license = "Apache-2.0"` (flagged, all targets, not trimmable).
  - Linux links `-lasound` directly (`quad-alsa-sys-0.3.2/build.rs:5`).
- **miniquad 0.4.11:** no audio API. The only audio text is CoreAudio constants in `native/apple/frameworks.rs`.
- **oxideav-sysaudio 0.1.1** (MIT; first release 2026-04-19, last 2026-04-25; 133 downloads):
  - It dlopens `libasound.so.2` and `libpulse-simple.so.0` through libloading, and the callback is f32 only.
  - It needs libloading 0.9, which needs Rust 1.88, so `cargo +1.85.0 build` fails ("libloading@0.9.1 requires rustc 1.88.0").
  - On stable, here: drivers `["pipewire", "pulse", "alsa", "oss"]`, `probe []`, `open failed: no audio driver available`. It fails cleanly but is too new to rely on.
- **oxisound / oxisound-pulse 0.2.1** (pure-Rust PulseAudio backend): `Apache-2.0`, `rust-version = 1.89`. Not measured.
- **libpulse-binding 2.30.1 + libpulse-simple-binding 2.29.0** (`MIT OR Apache-2.0`): link fails, `unable to find library -l:libpulse-simple.so.0` / `-l:libpulse.so.0`. Linux only.
- **alsa 0.12.1** (`Apache-2.0/MIT`): needs ALSA dev files (`alsa-sys 0.6.1` pkg-config failure observed). Linux only.
- **pulse-simple 1.0.1:** `LGPL-3.0`. Not measured.
- **rustaudio 0.1.2** (MIT-0, raw miniaudio bindings): raw FFI, so our code would need `unsafe`. `ep-miniaudio-sys`/`miniaudio` 0.10 (ExPixel) are older bindings. Not measured.
- **kira, awedio, kittyaudio, oddio backends:** built on cpal, so they inherit `Apache-2.0`.

#### Recommendation

**Recommendation: maudio** (miniaudio), opened through a context whose backend list names only real backends (WASAPI, CoreAudio, PulseAudio, ALSA), so that a machine with no audio fails cleanly with `MA_NO_BACKEND` instead of playing into the Null device.

- It is the only measured option that is clean against GPL-2.0-only (MIT, with miniaudio Unlicense/MIT-0).
- It builds on Rust 1.85.0 and needs nothing on CI beyond the C compiler each runner already has (Linux observed; Windows/macOS from the build script).
- It produces a binary that starts on machines without any audio library.
- Its API is a safe pull callback in i16 at 11,025 Hz with resampling done by the library.
- Its tree is 3 new crates.
- Its costs are real and stated:
  - about 95,000 lines of vendored C compiled into the program;
  - a young, single-maintainer binding crate;
  - an unverified MSVC `c99` flag;
  - **on this WSL install it can make no sound until `libpulse0` (runtime only) is installed.**

**Fallback: the pure-Rust per-OS route.** That is `pulseaudio` 0.3.0 (pinned) on Linux, `wasapi` 0.27.0 on Windows and `coreaudio-rs` 0.14.2 on macOS behind our own small audio interface.

- It is licence-clean (51 crates, none flagged) and needs no C at all.
- It is the only thing that delivered samples to WSLg's sound server as the machine stands.
- It costs three backends of our own code (two of them never compiled here), a pinned crate with an MSRV trap in newer releases, and no sound on Linux systems without a PulseAudio-compatible server.

cpal is recorded but not recommended: it is the most mature option, but its `Apache-2.0`-only licence needs the same owner decision as winit in S2, and on Linux it needs `libasound2-dev` in CI and libasound at run time.

#### Undefined, implementation-defined or layout-dependent behaviour of the audio libraries

- miniaudio's default backend choice depends on which shared libraries happen to be installed (dlopen order: PulseAudio, ALSA, JACK, then Null on Linux). The same binary therefore picks different backends, buffer sizes and resampling paths on different machines. Device period sizes are hints ("may be adjusted by the backend").
- Server-side or library-side resampling (PulseAudio here, miniaudio's converter, WASAPI AUTOCONVERTPCM) is outside the declared mixing schedule. Sample-for-sample claims can only cover our 11,025 Hz output stream, never what the device plays.
- The PulseAudio server rewrote the requested buffer (asked 5,040 bytes, granted 3,780), so latency is negotiated and machine-dependent.

#### Questions the audio library facts raise for the owner

Each question is taken up in [§5](#5-owner-decisions-t1t16-with-recommendations): 1 in T7; 2 in T7 (the C dependency); 3 in T7 (`libpulse0`); 4 in T7 (the Null device); 5 in T7 (`apt-get` in CI).

1. **Which library?** Either (i) maudio (C miniaudio compiled in our build, MIT/Unlicense, 3 new crates, no system packages, ran here only on the Null device), or (ii) the pure-Rust per-OS route (51 crates, our own three backends, reached WSLg's server), or (iii) cpal (accepting `Apache-2.0` alone, plus `libasound2-dev` in the Linux CI job). **Recommendation: (i), fallback (ii).**
2. **Is a second dependency that is C acceptable**, given that S2 chose miniquad partly because it is pure Rust? Either accept vendored C compiled by `cc` on all three CI runners, or require pure Rust and take route (ii). **Recommendation: accept it**; the C is public domain/MIT-0, is compiled from the crate, and needs no system package.
3. **Sound under WSLg on this machine:** with (i), sound needs the runtime package `libpulse0`, which the owner would install himself (one `apt-get install libpulse0`, no `-dev`, and CI is untouched). Either accept that, or add a pure-Rust PulseAudio path for Linux as a maudio custom backend or as a second backend. **Recommendation: accept the one runtime package and say so in the README.** The Phase 2 run under WSLg would then be the first real-device observation.
4. **Null device policy:** either never use miniaudio's Null backend (open fails cleanly; the app runs silent and says so), or allow it as a silent fallback. **Recommendation: never** (explicit backend list). Tests use our own recording sink and never open a device.
5. If the owner prefers a C library he installs himself, as S2's fallback wording allowed for SDL2: is one `apt-get install libasound2-dev` line in the Linux CI job acceptable (for cpal or tinyaudio)? **Recommendation: not needed if (i) is chosen.**

#### What the library comparison did not do or observe

- **No sound was heard or could be heard.** Every sample written was zero. maudio ran only on the Null device. The PulseAudio trial reached WSLg's `RDPSink`, but nobody listened, and the Windows side of WSLg's audio path was not observed.
- No real audio device was opened by maudio, cpal, tinyaudio, SDL2 or cubeb on any OS.
- Nothing was compiled for Windows or macOS (targets not installed). Their trees and licences come from `cargo tree`/`cargo metadata`, and their build needs come from manifests and build scripts. The MSVC `c99` question for maudio is open.
- `libpulse0`, `libasound2`, `libasound2-dev`, SDL2 and cmake were not installed. So: maudio's PulseAudio and ALSA paths, cpal's link, tinyaudio, SDL2 audio and cubeb were never run.
- Latency, underrun behaviour, pause/resume and resampling quality were not measured; the 0.99 s drain of the PulseAudio trial is the only timing seen.
- The per-OS route's Windows and macOS code does not exist. Its size and the safety of the `wasapi`/`coreaudio-rs` calls we would need are from a short source read only.
- Licences come from SPDX fields plus first-line or header checks of the cpal, miniaudio, maudio-sys, stb_vorbis, libcubeb, speex and pulseaudio files. This section did not read every crate's LICENSE text. cubeb's nested cargo trees were not enumerated. This is not legal advice.
- Build scripts were read for maudio-sys, alsa-sys, quad-alsa-sys, cubeb-sys (partly) and coreaudio-sys (bindgen lines only), and not for the others. maudio's 1,070 `unsafe` sites were not audited.
- Build times are single, warm-download figures on a machine where other jobs were building.
- Two network reads went beyond cargo: the crates.io API pages for release dates. Nothing else was downloaded or installed. Scratch is 0.7 MB after deleting the target directories.

Files that matter for Phase 2:

- `/home/copgunn/scratch/m5c/audio-libs/maudio/examples/sketch.rs` (API sketch, type-checked on 1.85.0)
- `/home/copgunn/scratch/m5c/audio-libs/maudio/src/main.rs`
- `/home/copgunn/scratch/m5c/audio-libs/pulseaudio-old/src/main.rs`
- `/home/copgunn/scratch/m5c/audio-libs/tools/analyze.py`
- `/home/copgunn/scratch/m5c/audio-libs/logs/` (`census.txt`, `run-*.txt`, `build/*.log`)

### 4.15 Undefined, implementation-defined or layout-dependent behaviour

1. The button-release origin (p_spec.c:1151): layout-dependent; this design models the LP64 reading and
   refuses slot 15 (§4.4.5).
2. Lumps of 8 bytes or fewer: one byte read past the zone block; refused at the start (§4.6.5).
3. `soundorg`'s `int` sum can overflow and its `/2` of a negative odd sum is implementation-defined (gcc
   truncates); Rust computes in `i64` with truncating division and refuses an overflow at load.
4. `vol_lookup`'s negative division and `S_AdjustSoundParams`' right shift of a negative product are
   implementation-defined in C89; Rust's `/` truncates and `>>` on `i32` is arithmetic, matching gcc.
5. `steptable` uses `pow`; the committed table is exact and the margin makes libm irrelevant.

### 4.16 Questions the Rust design raises for the owner

Each question is taken up in [§5](#5-owner-decisions-t1t16-with-recommendations): 1 in T2; 2 in T5 (this section's recommendation, `S_UpdateSounds` once per iteration, differs from §5's, which runs it after every tic: see T5); 3 in T6; 4 in T9; 5 in T3; 6 in T4; 7 in T12; 8 in T11; 9 and 10 in T15; 11 in T8; 12 in T7; 13 is a correction to §2.4, not a decision.

The questions are the thirteen items of §4.12, in that order (the last is a correction to §2.4, not a decision); the stop-and-ask candidates were taken up as T1-T16 in §5, and T2, T3, T5 and T6 decide what the Phase 2 goldens contain.

### 4.17 What this section did not do or observe

- Nothing was built or run: no cargo, no reference tool, no C. Every Rust fact is from reading the code at
  7f7fd99; line counts are `wc -l` and include comments and in-file tests.
- **The sound-logic runner of §3.1 and §3.4 had not reported** when this section was written and when its sources were last checked
  (`ls -la ~/scratch/m5c/reports/` at the end), so this section does not use its
  measured counts, its handle-convention comparison or its runner design; the tools estimate in §4.11 is
  therefore a guess for that part. The call-site facts of §1.2 and the mixer facts of §3.2 and §3.3 arrived during the work and are
  used.
- This section did not audit every Rust emitter's staged-state context (§4.4.4 risk 1) or prove that every actor
  position commit emits an event with its result (§4.4.2), and this section did not check that every removal path
  goes through one push point.
- This section did not measure latency, underrun behaviour or resampling quality, and opened no audio device.
- This section did not read the M5a design section in full; the design follows M5B §3, whose structure M5a's
  section shares.

## 5. Owner decisions (T1–T16) with recommendations

Each decision is tested against four questions: (a) is it consistent with prior owner decisions (E1–E9, F1–F11, G1–G13, H1–H12, I1–I13, P1–P9, R1–R16, S1–S23) and with the M5c brief? (b) does it admit no silent divergence? (c) does it add no new kind of fidelity gap beyond a declared refusal or a declared presentation contract? (d) are stores and work bounded? "Yes" to all four is written "(a)–(d) yes".

**Status: all confirmed by the owner on 2026-11-18**, as recommended, with notes on T2, T3, T4, T5 and T6, T7, T12 and T13; see [Owner rulings](#owner-rulings-2026-11-18) directly below. The decisions are kept below as written in Phase 1, each with a closing line recording its confirmation. Before the rulings, only Phase 2 steps 1 and 2 (genuine-engine tools and evidence) were done, with the three choices that shape the goldens built as switches of the tools ([§6.4](#64-what-proceeds-before-the-rulings)); the owner confirmed the recommended value of each (T2, T5, T6), so the committed goldens stand as they are.

Six findings of Phase 1 contradict or go beyond what the brief took for granted, and each has its decision below: the shipped build does not run the mixer the brief names (T2); the wire-24 simulation emits a wrong sound for one death (T3); no sound event has ever been compared with the genuine engine (T3); the original runs `S_UpdateSounds` and the mixer per loop pass, not per tic (T5); one switch sound reads its position out of the neighbouring button slot, differently on 32-bit and 64-bit builds (T12); and this machine has no audio client library at all (T7).

### Owner rulings (2026-11-18)

On 2026-11-18 the owner ruled on Phase 1 (head `a8778cb`): T1–T16 are confirmed as recommended, **confirmed by owner 2026-11-18**, with the notes below. Where a note and the text of a decision differ, the note governs. Each decision's text below is the Phase 1 record, ending with a line saying it was confirmed.

- **T2 (which build is the reference for sound; (i) is the in-process build).** (i). Every claim says "linuxdoom-1.10 built with its in-process sound code (one define removed)", never the shipped binary and never what a 1997 player heard.
- **T3 (how sound starts get out of the simulation; (iii) is wire 25 with canonical sound records and the corrected scream).** (iii). The wrong scream is documented in COMPATIBILITY as a known wire-24 gap, unpatched by rule. COMPATIBILITY also says plainly that the sound events of wires 22–24 were never compared with the genuine engine. Every further fault the first genuine comparison finds is documented as a frozen-wire gap, fixed in wire 25 only, and listed in the status line of this note and in the final report. If more than about five turn up, or any is not about sound, work stops and the owner is told.
- **T4 (the THING-loading truncation).** Yes: the truncation under wire 25, and nothing else.
- **T5 and T6 (the declared schedule; the wipe).** As recommended. The melt is the optional step 7b, done only if the cap allows.
- **T7 (the audio library; (i) is maudio).** maudio, pinned to an exact version in `Cargo.toml`, and opened with an explicit list of real backends so that it fails loudly instead of falling to a silent device. Before anything is built on it, it must be proven to compile and link in CI on Windows and macOS (the Windows artifact job included). If it does not, the work switches to the pure-Rust fallback and the owner is told. The final report gives the exact command the owner would run himself to hear sound under WSLg.
- **T12 (the two switch sounds).** Press: as recommended. Release: follow the 64-bit reading, declared, and add a detector: at each release sound whose next slot is busy, check whether any value the 32-bit build could read there (any texture number, the slot's timer) would change the start's audibility, volume, separation or `M_Random` draws. If none would, nothing is recorded. If one would, the session continues on the 64-bit reading and is labelled layout-dependent from that tic, counted separately in every report, and COMPATIBILITY says so. Slot 15 is refused as recommended. The reports say how often the detector fires on the 61 real runs (expected: never).
- **T13 (audible quirks).** Reproduce every quirk and list them in COMPATIBILITY.
- **The machine.** On 2026-11-18 the owner said the machine is idle: no caps or job limits. Work stops if `/mnt/c` falls below 20 GB free.
- All other decisions (T1, T8–T11, T14–T16) are confirmed as recommended, without notes.

One reading of the T12 note is the orchestrator's and is recorded here so that the owner can correct it: the release origin is read again by every later `S_UpdateSounds` while its channel is occupied ([§4.4.5](#445-the-button-release-origin-p_specc1151)), and a different reading there could free the channel at a different tic. The detector therefore also checks those later reads (audible or not, the only value `S_UpdateSounds` uses), under the same rule and the same label.

### Owner rulings on the open points (2026-11-18, second set)

Later on 2026-11-18, after steps 3 to 13 were on the branch, the owner ruled on the points the status line had listed as open. Each is **confirmed by owner 2026-11-18**; where a ruling and an earlier text differ, the ruling governs.

1. **The start-up pause under WSLg.** Agreed, no change; the owner will judge it when he plays.
2. **Rule S9 for sound and display refusals.** The orchestrator's reading is confirmed. The simulation's tic stays committed and its command stays in the demo; sound and display stay at the last good tic; the demo replays to the same refusal. COMPATIBILITY says that in one plain sentence, and that a future save must never combine a simulation at tic N with sound or display at tic N-1.
3. **The layout-dependent label (T12).** The exit line of `doom-play` is enough; nothing is drawn on screen. `tools/local/m5c_session_real.py` and the headless session check print the same label beside the verdict.
4. **The device queue's two-buffer start.** Accepted as presentation, unmeasured; the owner will judge lag and crackle by ear.
5. **A visual for sound (overruling "no new animation").** Sound can be shown. The README gains a visual under the visuals rule: waveforms of the genuine mixer, the Rust mixer and their difference for one synthetic session, generated by a committed tool from the committed goldens, with the count of differing samples taken from the real comparison; a static SVG is fine. The caption says, as usual, that these are our own generated test sounds and not the game's, and names the declared schedule and the in-process build. The same tool also writes a short WAV (genuine, then Rust, then the difference) and, only if ffmpeg is already installed, an MP4 under 10 MB of the picture with that audio, into `docs/media`. Nothing is installed for it. The MP4 is not linked from the README: the owner will listen first and give a link.
6. **The macOS job.** Its limit is raised to 90 minutes, and ROADMAP gains a line to split or trim the job in a later milestone.
7. **The instruction-count fix.** It changed one line of an existing test (`transaction_cost_tests.rs`, the call site only). Accepted on condition that every frozen wire's goldens pass in CI on that head; the evidence record says which line, why, and that no assertion changed.
8. **The pull request body** was stale and is rewritten before the pull request is marked ready.

The owner also ruled, the same day, on housekeeping: the original sources are marked as vendored in `.gitattributes`; commit subject lines are at most 72 characters from now on, with history never rewritten; the policy table lives in COMPATIBILITY behind one link and the README stays under 150 lines, with caveat counts as links to the evidence and no claim made stronger (commit `91b018c`).

### The owner's playtest and rulings (2026-11-18, third set)

On the evening of 2026-11-18 the owner played release builds of `3c1cc5d` and reported what follows. Items 1 to 3 are his observations, recorded as observed facts; the rest are his rulings, and where they differ from an earlier text they govern. (The message came in several wordings within an hour; this is the last one. Earlier wordings had asked for a change to the sound delay, then for exit-line statistics, then for the final gates at once: all three were withdrawn.)

1. **Windows 11, the native artifact, `--bindings modern`, E1M1 on `hmp` and `uv`.** The window opens, the game plays at 35 frames a second, the picture looks right, the mouse is captured and feels good, and sound plays. He noticed a delay on every sound at first and then judged it normal for the original. Nothing is to be changed for it.
2. **WSLg, the same commit.** The window opens and sound plays through PulseAudio (device "RDP Sink"). The pointer is not captured and turning by mouse is unusable there, so WSLg is keyboard-only for now. After the window went away the terminal showed only "X connection to :0 broken (explicit kill or server shutdown)" and none of `doom-play`'s own exit lines.
3. **Not judged by the owner:** macOS, long sessions, every map, every weapon and monster, vanilla bindings.
4. **A bug, to be dealt with first.** On Windows, with the native artifact and `--bindings modern`, firing with the left mouse button while dragging the mouse left or right leaves fire stuck on, repeating. Once stuck, moving the mouse does not clear it and clicking while moving does not clear it; only a click with the mouse held still does. Firing with right Ctrl while dragging never sticks. So a left-button release that arrives together with motion is lost, while a release with no motion is delivered: the fault decision S4 of M5b was meant to exclude. The order of work: (a) start with the code that builds the one mouse event per command build when a pump holds both motion and a button change, and with the held-back release for taps, and reproduce it in the event layer's tests (press, motion and release in the same pump and in adjacent pumps, in every order, including rapid repeated clicks during continuous motion); (b) if it does not reproduce there, read the window library's Windows code for how button-up is delivered while the pointer is captured and moving; (c) if the cause is in the app's own event layer and a test reproduces it, fix it in this milestone, the failing test first and then the fix, each in its own commit, with no existing test, script or golden changed (if an existing golden encodes the faulty behaviour: stop and tell the owner); (d) if the cause is in the window library or cannot be reproduced on Linux, do not guess at a fix: add an option to `doom-play` that logs every raw input it receives and every event it sends to the game, with the pump number, to a file, and say how to use it; (e) either way, say plainly whether sessions recorded with the fault are still valid demos (expected: yes, the demo records what the game was told).
5. **Sound latency, measurement only.** The arithmetic is written out end to end for `doom-play` (schedule, queue, device) and for linuxdoom-1.10 from its source (the 512-frame mix and the two 2,048-byte fragments requested at `i_sound.c:775`), each as a range in milliseconds, with a plain statement of whether `doom-play` sits inside the original's range. The two-buffer start stays. No code change.
6. **The lost display connection under WSLg.** Find out whether `doom-play` saves the session and prints its exit lines in that case; document it as a limit; change no behaviour.
7. **The owner's session files** in `/mnt/d/DoomSessions` (played by hand, some on Windows and some under WSLg, on `hmp` and `uv`) are each checked with `tools/local/m5c_session_real.py`, and each verdict is reported with its counts.
8. **Documents.** README, COMPATIBILITY and the evidence record stop saying nobody has seen, heard or played it: they say what was observed, on which system, by the owner, and what was not (item 3). WSLg is keyboard-only. The stuck-fire fault is recorded with its cause and its fix, or as a known Windows limit if it is not fixed.
9. **Then stop.** The Windows artifact is rebuilt on the head that has the fix or the logging option, the owner is told which run to download it from, and work stops with a report. **The final gates, the pull request body and marking the pull request ready wait until the owner has retested on Windows and said so.**

The owner was using the machine that night: the in-use limits applied (at most 2 parallel C reference jobs), with the stop below 20 GB free on `/mnt/c`. Also on 2026-11-18 the owner had the Codex model routing made to mirror Claude's, with efforts on both tools (AGENTS.md, commits `abc0ece` and `5e00229`).

### The owner's Windows retest and rulings (2026-11-19, fourth set)

On the morning of 2026-11-19 the owner reported: "Retest done on Windows 11 with the b141767 artifact", with the input log at `/mnt/d/DoomSessions/input-log.txt` and the session's demo beside it; his display scaling is 125% (the window was about 1200 by 900 for a 960 by 720 request). Two fields of his message came unfilled (whether ClickLock is on, and whether fire stuck); the log and the demo answer the second (it stuck three times) and the first is not known. His rulings:

1. **Read the log and report the cause** in two or three sentences, with the summary line's left-button counts. (Done: 18 downs and 15 ups; the cause is in the [evidence record](validation/M5C.md#the-owners-windows-retest-with-the-input-log-2026-11-19).)
2. **The fault is not fixed in M5c.** The finding and the proposed fix are recorded in the evidence record and in ROADMAP as the first item of the next milestone, together with two more: measuring the real sound latency on Windows and trying the cheapest way to shorten it; and writing the session demo so that Ctrl+C or a closed console loses nothing.
3. **In that next code change** the mouse-forwarding logic moves out of `input_log.rs` into a file named for what it does, and the review's three minor points (M1 to M3) are closed. (So M5c has no further code change, and those three points stay open in M5c, recorded.)
4. **Then** the final gates on the final head, the pull request body rewritten, the Windows artifact rebuilt, and the pull request marked ready. This is the go-ahead that the third set of rulings made the final gates wait for.

The owner did not say on 2026-11-19 whether the machine was idle, so the in-use limit (at most 2 parallel C reference jobs) was kept for the final gates.

### Scope and the reference

**T1. Scope.** As the brief. In: the sound logic of `s_sound.c`, every call site a single-player Episode 1 level run reaches (54 of the 96 level-reachable sites, [§1.2](#12-every-call-that-starts-or-stops-a-sound)), the mixer of `i_sound.c`, the `M_Random` accounting of sound and of the level-start wipe, and audio output in `doom-play`. Out: music, menus and their sounds (including the menu hot keys that work during play, T14), the automap, intermission and level-to-level play, save and load, sound options screens, the external `sndserver` process, skills and modes M5b does not admit (so the nightmare respawn fog sounds), and everything S1 left out of M5b. (a)–(d) yes. **Recommendation: as stated.** **Confirmed by owner 2026-11-18.**

**T2. Which build of linuxdoom-1.10 is the reference for sound.** Fact ([§1.3](#13-the-mixer-i_soundc), [§1.1](#11-the-sound-logic-s_soundc-in-full)): `doomdef.h:84` is `#define SNDSERV 1`, so the build the source ships with sends each sound start down a pipe to the separate `sndserver` program and never runs `i_sound.c`'s mixer; the ChangeLog (15-19) says this was switched on deliberately because the in-process mixer "gives ghastly results" with the timer option. The in-process mixer the brief names is compiled only when that one line is removed. The two builds agree on every `M_Random` draw and on the volume, separation and pitch of every start, so **the face's glance and the closing of S15 do not depend on this choice**. They differ in what `I_StartSound` returns (the sound's number in the shipped build; a counter 100, 101, ... in the in-process build), and because "still playing" is `gametic < handle` in both (`i_sound.c:517-521`), they differ in when a channel of the sound logic is free and so in which starts are refused for want of a channel.

Options: (i) **the in-process build**: `doomdef.h:84` commented out as the one declared adapter, for the sound-logic runner and the mixer runner alike; (ii) the shipped build for the sound logic, with no sample claim at all (its audio comes from `sndserv/`, which is out of scope, ages its channels by its own loop counter and receives starts asynchronously, so it cannot be compared tic for tic); (iii) the shipped build's handles for the sound logic and the in-process mixer for the samples, a combination no binary ever ran.

(a) (i) is the only option under which the brief's mixer exists; the frozen runners of M3h9c to M5b compile no sound code and are unaffected. (b) and (c): yes for (i), provided every claim says "linuxdoom-1.10 built with its in-process sound code (one define removed)", never "the shipped binary". (d) yes.

**Recommendation: (i).** Measured with both conventions as a switch of the prototype runner ([§3.1.9](#319-the-two-handle-conventions-measured-obs)): on all 61 real runs (26,077 start attempts) and all 59 synthetic items the `M_Random` index, the face and every frame are identical under both; on the real runs 99 starts change outcome (11 are refused for want of a channel under the shipped convention, 110 under the in-process one), and on the synthetic items none does. What the owner should know: this is the build id's own ChangeLog calls experimental; what is matched is its code, compiled and run, not a recording of anything a 1997 player heard. **Confirmed by owner 2026-11-18, with a note: see [Owner rulings](#owner-rulings-2026-11-18).**

### The simulation

**T3. How sound starts get out of the simulation.** Facts ([§2](#2-what-the-simulation-gives-today), [§4.4](#44-how-sound-starts-get-out-of-the-simulation-owner-decision-t3)):

- Every sound start wire 24 can reach already has an event at the right place and, as far as reading shows, in the original's order. **No event carries a position**, and `S_AdjustSoundParams` needs the origin's x, y and the listener's x, y and angle at the moment of the call, which is in the middle of the tic: player-think calls see the start-of-tic position with this tic's turn applied; sector movers see the end-of-tic position; monsters and missiles see one or the other depending on their place in the thinker list; a monster can be moved during another actor's turn (damage can switch it into its chase state); teleport fog sounds see the destination with the old facing angle ([§1.2](#12-every-call-that-starts-or-stops-a-sound)). Rebuilding these in the app from the states before and after the tic is exact in most cases and cannot be exact in at least one (the listener's angle at a teleport after a turn in the same tic), and cannot be shown exact cheaply in the others.
- **Wire 24 emits a wrong sound.** `A_PlayerScream` chooses `sfx_pdiehi` for a death below -50 health only in commercial mode (`p_enemy.c:1994-2007`); the Rust code chooses it in every mode (`lifecycle.rs:1701-1708`). The event is part of the canonical snapshot, so wire 24 keeps it for ever. Played from wire 24's events, a hard death would make the wrong scream (`DOOM.WAD` does contain that lump; checked locally).
- **No wire 22–24 sound event has ever been compared with the genuine engine**: the whole-engine runners stub `S_StartSound` and the comparators strip `events.*`. The first genuine comparison may therefore find more faults in wire 24's events. Each would need its own wire-25 rule, which would widen "only what sound needs". The owner should know this before ruling.
- Sector sound origins (`p_setup.c:536-557`) are stored nowhere in Rust.

Options: (i) use the existing events and change nothing: not possible (no positions, one wrong sound); (ii) wire-24 events plus reconstruction in the app, refusing where it cannot be exact: about 600–900 lines in the app, leaves the wrong scream in the canonical events and the positions unproven; (iii) **a new policy wire 25** that adds only what sound needs: sound records (each start and each removal stop in the original's call order, with the origin's identity and, for positional starts, the origin's and the listener's position at the call) as a new canonical group `events.sounds` in a new schema DSIM41, kept apart from `events.last` so that everything that reads today's events sees the same lists; the correct scream; sector sound origins computed at load; the switch origins of T12; (iv) wire 25 with the same records in a buffer that is not part of the snapshot: saves about 300–500 lines of snapshot, decoder and encoder work and loses the digest of the positions; (v) a declared limit (sound without positions is not Doom's sound: not recommended in any form).

How (iii) keeps the promise that nothing else changes, proven by the full harness on all 61 real runs and every synthetic item: (1) each wire-25 capture, with `events.*` stripped and the profile and schema names set back to wire 24's, has the same digest as the wire-24 capture of the same tic (the method M4b used for DSIM40); (2) wire 25's `events.last` equals wire 24's with `sfx_pdiehi` replaced by `sfx_pldeth` and nothing else; (3) the genuine engine against Rust, tic for tic, as today, plus the new comparison of every sound record against the genuine `S_StartSound` recorder. Wires 1–24, their digests, DSIM bytes and every file under `tests/reference` stay untouched.

(a) (iii) is the brief's own first option. (b) yes: positions come from the code that moves things, at the call, and every record is compared with the genuine engine. (c) yes. (d) yes: at most one record per start or removal, bounded by the tic's existing event bounds.

**Recommendation: (iii), wire 25 with canonical sound records.** Cost: about 900 lines in `doom-sim`, 700–1,000 lines of tests, about 300 lines of Python for the DSIM41 encoder ([§4.11](#411-size-estimate-hand-written-lines-30)). The main risk is a position read from the wrong moment; only the genuine comparison catches that, so no face claim is made before it passes. If the cap is passed, (iv) is the fallback. **Confirmed by owner 2026-11-18, with a note: see [Owner rulings](#owner-rulings-2026-11-18).**

**T4. Whether wire 25 also carries the THING-loading truncation fix.** Fact ([§2](#2-what-the-simulation-gives-today), item 7): the fix is one test at the top of the THING-loading loop under a wire-25 predicate (`spawning.rs:443-462`; `p_setup.c:317-337`), using the same "this game is not commercial" fact as the scream; no Episode 1 map contains any of the ten Doom II types, so the 61 real runs cannot change; the evidence is one synthetic map through the existing registered-mode harness. Options: (i) carry it, and nothing else; (ii) keep wire 25 to sound alone and leave the gap for a later wire. (a)–(d) yes for both. **Recommendation: (i)**, because it is cheap, exact and closes a recorded gap; it is the first item to drop if the cap is passed ([§6.3](#63-fallback)). **Confirmed by owner 2026-11-18, with a note: see [Owner rulings](#owner-rulings-2026-11-18).**

### Timing outside the tic

**T5. The declared schedule.** Facts ([§1.1](#11-the-sound-logic-s_soundc-in-full), [§1.3](#13-the-mixer-i_soundc), [§1.4](#14-m_random-accounting-and-the-level-start-wipe)): in the original, three things in the sound path are tied to passes of the main loop and so to the wall clock, not to tics. (1) `S_UpdateSounds`, which frees channels, runs once per pass (`d_main.c:392`), and a pass holds one or more tics. (2) The mixer makes one buffer of 512 frames per pass (`d_main.c:397-405`); a tic is worth 315 frames at 11,025 Hz; the pace comes from the blocking write to the sound device. (3) How many tics the first pass runs before the first picture (and so before the wipe) depends on the clock (1 to 5). A fourth is tied to the process: "still playing" is `gametic < handle`, and both `gametic` and the handle counter count from the start of the process, not of the level.

The schedule proposed, each part tied to tics alone:

1. **One tic per pass for the game, the sound logic and the picture**, as `-timedemo` runs the loop and as R8 and S8 already fixed for frames: tic, then `S_UpdateSounds`, then the frame. So `S_UpdateSounds` runs after every tic and the wipe's draws come after the frame of tic 0.
2. **The mixer makes buffer k (k = 0, 1, 2, ...) of 512 frames after tic ⌊512k/315⌋** of the session: none or one buffer after each tic, 315 buffers every 512 tics, the first after tic 0. A start takes effect in the next buffer mixed, as in the original, so it is heard 0 to 511 frames (at most 46 ms) after its tic. The genuine mixing code runs unchanged under this rule ([§3.2](#32-harness-the-genuine-mixer-headless)).
3. **A session is a fresh process started straight on the map**: `gametic` and the handle counter are both 0 at its first tic (T15).

The alternative for part 1, which the Rust design section argues for ([§4.3.3](#433-the-per-tic-sequence-against-the-c-order)): (B) `S_UpdateSounds` only after the tics that end with a buffer, so that the passes are tics {0}, {1}, {2,3}, {4}, {5,6}, ..., which is the one loop the in-process build itself runs when the sound device paces it. It differs from part 1 in which channels are free during the second tic of a two-tic pass, and so in a few refused starts; the `M_Random` stream, the face and the frames are identical under both. Measured ([§3.1.10](#3110-the-two-s_updatesounds-schedules-measured-obs)): on the 61 real runs (B) refuses 17 more starts than part 1 under the in-process convention (15 under the shipped one), out of 26,077; on the synthetic items no start changes outcome. The other alternatives for part 2 fail on measured grounds: one buffer per tic plays 1.625 times too long, and a buffer size of 315 changes the padding and which channel is replaced ([§3.2](#32-harness-the-genuine-mixer-headless)).

(a) Part 1 is R8's and S8's rule carried over and the brief's own wording ("`S_UpdateSounds` per tic"); it also keeps the sound-logic goldens independent of the mixer's rule. (B) is closer to one real loop but makes the sound logic depend on the buffer rule. (b) yes: every part is reproduced by the genuine code in the runners under the same rule. (c) it is a declared schedule, as R8 is. (d) yes.

**Recommendation: parts 1 to 3 as stated**, written into COMPATIBILITY as "the declared sound schedule". The claim is then: identical to linuxdoom-1.10 (T2's build) under this schedule; no claim about a session paced by a real 1997 sound card. **Confirmed by owner 2026-11-18, with a note: see [Owner rulings](#owner-rulings-2026-11-18).**

**T6. The level-start wipe.** Facts ([§1.4](#14-m_random-accounting-and-the-level-start-wipe)): every start straight on a map wipes, and the wipe draws `M_Random` exactly 320 times (which moves the index by 64) after the first frame; only 160 of the values are used. The melt's pictures depend only on the two screens, the index at its start and the number of wipe ticks elapsed; it lasts 34 to 43 ticks (about 1 to 1.2 s), during which the original runs no tics, reads no input and mixes no sound; afterwards it runs a burst of up to 5 tics and drops the rest of the lost time. Its start screen in a fresh process is the untouched frame buffer (zeros in practice) with no palette yet set.

Options: (i) **account for the 320 draws only**, at the declared point (after the frame of tic 0), with the genuine `f_wipe.c` compiled into the runner so that the draws come from the source; the melt is not shown and takes no time; the frame contract (R8, S14) is unchanged, and the saved display state goes to version 2 for the contract value either way ([§4.8](#48-saved-state-for-restore-and-continue)); (ii) **implement the melt**: 34–43 extra pictures at each session start that are not tic frames (so the proven frame stream gains a second kind of picture, or the melt stays unproven presentation), about a second in which no tic runs (needs a clock rule; vanilla's burst would not be imitated, as S8 already does not), a declared start screen and palette, a new animation under the visuals rule, and about 800–1,200 more lines.

(a) Both close S15: the draws are the same. (ii) changes how every session and every restart begins, which is pacing and presentation the owner has not yet seen even without it. (b)–(d) yes for (i).

**Recommendation: (i) in this milestone**, with the genuine wipe compiled and run in the runner from step 1 on, so that the melt's pictures can be dumped as evidence at almost no cost. If the owner wants the melt, the plan carries it as a last, optional step 7b, done only if the cap allows after steps 1–7, otherwise the first item of a later presentation milestone, as S3 (ii) was handled in M5b; it is also the brief's named first fallback. **Confirmed by owner 2026-11-18, with a note: see [Owner rulings](#owner-rulings-2026-11-18).**

### Audio output

**T7. The audio output library (the project's second runtime dependency).** Measured in [§4.14](#414-audio-output-library-a-measured-comparison) on 2026-11-18. The deciding facts:

| | maudio 0.1.15 (the miniaudio C library, vendored) | pure Rust per operating system: `pulseaudio` 0.3.0 (Linux) + `wasapi` 0.27.0 + `coreaudio-rs` 0.14.2 | cpal 0.18.2 (and rodio, built on it) |
| --- | --- | --- | --- |
| Pure Rust or C | safe Rust bindings to one C header (95,649 lines), compiled by the build | pure Rust | pure Rust |
| Licences against GPL-2.0-only | nothing flagged (MIT; miniaudio is Unlicense or MIT-0) | nothing flagged (0 of 51) | **flagged: cpal is `Apache-2.0` alone**, cannot be trimmed |
| Builds with no system packages | a C compiler only (observed on Linux; Windows and macOS from the build script) | nothing needed (Linux observed; others from manifests) | no: Linux needs `libasound2-dev` |
| Rust 1.85.0 | builds | only with `pulseaudio` pinned to 0.3.0 (0.3.1 and 0.3.2 do not compile) | type-checks |
| New crates | 3 | 45 | 27 |
| Our own code | one backend | three backends, two of them never compiled or run here | one backend |
| Ran here under WSLg | ran, but **only on miniaudio's silent Null device**: this WSL install has no `libpulse`, no ALSA library | **reached WSLg's sound server** with nothing installed (one second of silence delivered and drained) | could not build |
| Other costs | a young binding crate (first release 2026-02-16, one maintainer); an unverified risk that its build flag is refused by MSVC; to make sound under WSLg the owner would have to install the runtime package `libpulse0` himself (inferred from the library names it opens; nothing was installed) | the Linux crate is stuck at an old release until the minimum Rust version moves; the Windows and macOS halves would first run on the owner's machine or not at all | the licence question, as winit in S2 |

Every trial wrote digital silence only; nobody has heard anything from any candidate. tinyaudio, SDL2, cubeb, quad-snd and three others were rejected for observed reasons (§4.14); miniquad has no audio API.

Options: (i) maudio, opened with an explicit list of real backends (its default list ends in a Null device, so opening would "succeed" silently on a machine with no audio); (ii) the pure-Rust route; (iii) cpal, accepting `Apache-2.0`. (a) (i) and (ii) keep the GPL-2.0-only rule; (iii) needs the owner to accept the licence position he declined for winit. (i) is not pure Rust, which S2 counted in miniquad's favour. (b) and (c): not affected; the library sits behind one file of the binary crate (S19) and receives the finished stream. (d) yes. `unsafe` stays forbidden in our crates under every option (each trial compiled with `#![forbid(unsafe_code)]`).

**Recommendation: (i) maudio**, and it is a closer call than S2 was. It adds 3 crates against 45, one backend of ours against three, and its Windows and macOS paths are a widely used C library's, not code of ours that nobody can run before the owner does. Its costs are stated above; the one that touches the owner directly is that **sound under WSLg needs `libpulse0` installed by him**, while the Windows artifact should need nothing (not observed). **Fallback: (ii)**, if the owner wants pure Rust or sound under WSLg with nothing installed, or if the Windows build of maudio fails in CI. The seam in `doom-play` makes the switch local to one file. **Confirmed by owner 2026-11-18, with a note: see [Owner rulings](#owner-rulings-2026-11-18).**

**T8. What is proven in audio and what is not.** Proven artefacts, each compared with the genuine engine: the per-tic list of sound records (T3); per tic, every start with its volume, separation, pitch and channel, every stop, the channel table after `S_UpdateSounds` and the `M_Random` index; and the mixer's sample stream under the declared schedule (T5), compared buffer by buffer by digest. **Presentation, outside every claim:** the hand-over to the device thread, resampling from 11,025 Hz to the device rate (by the library where it offers it), the size of the queue and the latency that results, what happens on underrun (silence) and overrun (the oldest queued frames are dropped; the game loop never blocks on audio, because blocking would change pacing, which the owner has not playtested), the player's volume and mute (T9), and the device itself. (a)–(d) yes. **Recommendation: as stated**, as the brief proposes. **Confirmed by owner 2026-11-18.**

**T9. Volume, mute and the number of channels.** Fact ([§1.1](#11-the-sound-logic-s_soundc-in-full)): the original's sound volume setting is an input to the sound logic. It scales the volume of every start, and at 0 a sound from anything but the player is inaudible, which removes its `M_Random` draw and so changes the face. The number of sound-logic channels is likewise a configuration value (3 by default; the mixer has 8). Options: (i) the sound logic always runs with a fresh process's defaults, volume 8 and 3 channels (`m_misc.c:237`, `282`); the player's volume is a gain applied to the finished stream at the device (`--volume 0-100`), with a mute key at app level (like the app's pause: never an event the game sees) and `--no-sound`, which opens no device and still runs the sound logic so that the face is the same; pause mutes; (ii) a launch flag that sets the original's volume setting (0–15), which changes audibility, the draws and the face, and would need its own evidence per value. No options screen in either. (a) (i) is the brief's recommendation. (b)–(d) yes for (i). **Recommendation: (i).** The mute key's binding is chosen in step 7 and listed in `--help`; it is an app key, not a game input. **Confirmed by owner 2026-11-18.**

**T10. What is committed.** Committed: the synthetic sound lumps (our own tones, generated by a tool with `--check`; about 42 KB for the main set and 42 KB for the edge cases) and the rooms and scripts; the sound-logic goldens (text, every start, stop, channel table and index per tic); for the mixer, the start lists and a digest per buffer (about 40 KB per game minute), never raw samples. From IWAD runs: counts, pass/fail facts and digests only, local-only; never a sound name beyond what `sounds.c` itself names, never audio. (a)–(d) yes. **Recommendation: as stated.** **Confirmed by owner 2026-11-18.**

### Fidelity contracts that need a ruling

**T11. Where the original stops with a fatal error or reads memory it does not own.** Each becomes a typed refusal at the exact start, with the state preserved, as S17 ruled for the status bar: (1) a start whose sound lump is missing (`W_GetNumForName` aborts; 41 of the 108 sound numbers have no lump of their own in `DOOM.WAD`, the linked chaingun sound among them; a local check on 2026-11-18 found a lump for each of the 43 sounds named at the Episode 1 call sites of [§1.2](#12-every-call-that-starts-or-stops-a-sound), so none of the 41 is startable there); (2) a start of a lump of 8 bytes or fewer, where the original reads one byte past its block and the value depends on memory layout (observed to differ between builds, [§3.2](#32-harness-the-genuine-mixer-headless); no IWAD has such a lump); (3) sound number 109, which passes the original's range check and reads past the table; (4) a sector whose sound-origin sum overflows, refused at load; (5) a switch release in button slot 15 (T12). The alternative for (1) and (2), rejecting the WAD at load, diverges from the original, which accepts the WAD. (a) as S17. (b)–(d) yes. **Recommendation: refuse at the start, as stated.** None is reachable on an Episode 1 IWAD map as far as Phase 1 found; each gets a synthetic witness. **Confirmed by owner 2026-11-18.**

**T12. The two switch sounds.** Facts ([§1.2](#12-every-call-that-starts-or-stops-a-sound)):

- **Press** (`p_switch.c:228-256`): the origin passed is `buttonlist->soundorg`, the value in button slot 0, read before the new button is stored. While slot 0 is free that is NULL, so the click plays at full volume, centred, from any distance; while slot 0 holds a button it is that other line's sector. This is well-defined C. **Recommendation: reproduce it exactly.** (a)–(d) yes.
- **Release** (`p_spec.c:1151`): the call passes the address of the pointer field, not its value, so the position is read out of the next button slot: on the 64-bit reference builds x is the next slot's timer and y is 0; on the 32-bit original x is the next slot's texture number and y its timer. Both are within a hundredth of a map unit of the map's origin (0,0), which is where the click therefore seems to come from: whether it is heard, and so whether it draws `M_Random`, depends on the player's distance from that point, not from the switch. The two builds differ only while the next slot is busy, and slot 15 reads past the array. The release sound occurred 4 times in the 61 real runs and 3 times in the synthetic items ([§3.1.11](#3111-what-the-synthetic-items-exercise-and-what-phase-2-must-add-h)); whether the next slot was busy at any of them was not measured. **Observed in Phase 2 step 1 (2026-11-18)** on the committed script SNDBTN2, where the next slot is busy at the release, with the genuine runner built both ways (`m5c_reference.py --strict32`): the 64-bit build reads x, y = 34, 0 and the 32-bit build reads 19, 34 (fixed-point units, so both are less than a thousandth of a map unit from the origin); that start is the only row of the 64 items' sound streams on which the two builds differ, and its volume (4), separation (35) and pitch, and every frame, are the same in both. On SNDBTN1, where the next slot is free, both builds read 0, 0, and the release is heard or not according to the player's distance from the map's origin. Options: (i) **the 64-bit reading**, which the default reference builds produce, declared, with the 32-bit difference recorded and slot 15 refused: about 40 lines; (ii) the position (0,0) and a refusal whenever the next slot is busy at a release, so that nothing layout-dependent is ever modelled: about 20 lines, but pressing two repeating switches within a second of each other would end the session, and wire 25 would then refuse a tic that wire 24 admits; (iii) the 32-bit reading through a declared adapter in the 64-bit runner (the way S11 made the two builds agree for the input arrays): needs the next slot's texture number inside the sound path, which the simulation does not hold. (a) (iii) is what S11's precedent points to; (i) departs from it by declaring a reading instead of making the builds agree. (b) yes for all three against the declared build. (c) (i) adds one declared item of a new kind (an ABI reading); (ii) adds only a refusal. (d) yes.

**Recommendation: (i)**, because the difference cannot be heard and can change a value only in rare cases (a position difference under 0.01 map units), while (ii) would end a session over it and (iii) costs a cross-layer dependency. COMPATIBILITY would say plainly that this one read follows the 64-bit build. The Phase 1 reports are split: three recommend (i) and one recommends refusing. If the owner prefers that nothing depend on a build's layout, (ii) is the clean alternative. **Confirmed by owner 2026-11-18, with a note: see [Owner rulings](#owner-rulings-2026-11-18).**

**T13. Audible quirks of this source, reproduced as they are.** None is "fixed"; a fix would be a new profile needing its own decision. The owner will hear these and should know they are the original's: (1) pitch varies at random on most sounds (about 0.85 to 1.19 times normal speed; the saw sounds less); (2) a stop in the sound logic never silences the mixer (`I_StopSound` is empty), so a sound whose origin is removed plays to its end, and volume and stereo position are fixed at the start and never follow a moving origin; (3) a sound lump that declares 22,050 Hz is played at 11,025 Hz, an octave low and twice as long (1 lump in `DOOM.WAD`, 4 in `DOOM2.WAD`); (4) the switch click from the map's origin (T12); (5) on map 8, distant sounds are louder than near ones and nothing is ever out of earshot; (6) the linked chaingun sound has length 0 and would play one frame (no game code starts it); (7) the exit switch never plays its own sound (`p_switch.c:211` against `221`); (8) only three sounds of the sound logic can be tracked at once, though the mixer plays eight. (a)–(d) yes. **Recommendation: reproduce all of them, and list them in COMPATIBILITY.** **Confirmed by owner 2026-11-18, with a note: see [Owner rulings](#owner-rulings-2026-11-18).**

**T14. The menu hot keys, and what "a vanilla session with sound" means when S15 is closed.** Fact ([§1.4](#14-m_random-accounting-and-the-level-start-wipe)): with the menu closed, `M_Responder` still starts a sound, and so draws `M_Random`, for `-`, `=`, F1–F10 and Esc, at the moment of the key, between tics (F11 changes the gamma setting and starts no sound, `m_menu.c:1597-1603`). The app already does not pass those keys to the game (`input/mod.rs:111-120`; Esc is the app's pause). Options: (i) leave them out, and state the claim as: the face matches the genuine engine replaying the session's demo, under the declared schedule (T5) and build (T2); a vanilla player who pressed one of those keys during play would have shifted the stream, and a recorded demo does not contain key presses; (ii) model the hot keys' sounds without the menus. (a) menus are out of scope (T1). (b)–(d) yes for (i). **Recommendation: (i).** With T3, T5 and T6 as recommended, S15's limit is then removed from the README, and COMPATIBILITY states the claim in the words above. What closing it changes, measured with the genuine engine ([§3.4](#34-full-frames-under-the-new-contract)): with sound's and the melt's draws in, the face differs from the M5b sound-free pictures in 68,995 of 158,472 real-map frames and 11,392 of 21,043 synthetic frames, and in no frame does anything outside the face's rectangle differ. **Confirmed by owner 2026-11-18.**

**T15. Session start, restart and the app's default.** Options: (i) every session, and every restart, is a fresh process started straight on the map, as S11 ruled: `gametic`, the handle counter, both random indices 0, no channel occupied, mixer silent; `doom-play` runs wire 25 with sound by default; the M5b sound-free, wipe-free contract and wire 24 remain selectable in the libraries, where the frozen M5b tests and goldens use them, and are not offered as a launch flag (`--no-sound` gives silence with the same face); (ii) also a launch flag for the M5b contract. One consequence to state: a chainsaw raised at level load would play its sound before tic 0 (`P_SpawnPlayer`), which a fresh game on these maps does not reach. (a) as S11. (b)–(d) yes. **Recommendation: (i).** Session files recorded by M5b builds stay valid demos; replayed under wire 25 they give the same game and a different idle glance, which is the point of the milestone. **Confirmed by owner 2026-11-18.**

### Structure, evidence and delivery

**T16. Crates, harness, CI, size and fallback.** (1) A new platform-free crate `doom-sound` for the sound logic and the mixer, between `doom-render` and `doom-game` ([§4.2](#42-where-the-code-lives)); the device code in one file of `doom-play`; the `M_Random` index stays where it is, in the status bar state, and the display state goes to version 2 only under the new contract, so M5b states stay byte-identical. (2) New harness files only (`m5c_*`); no existing runner, driver, contract, golden or file under `tests/reference` changes; the stale line citations found in one frozen driver comment and three frozen stub comments ([§1.4](#14-m_random-accounting-and-the-level-start-wipe)) are recorded here and left alone. (3) CI: the Rust sound-logic and mixer tests against committed goldens on Linux, Windows and macOS in the existing jobs; the genuine sound-logic and mixer runs in one new Linux job, **not a required check** unless the owner says so. (4) Size: a projection of 19,000–28,000 hand-written lines against a **cap of 30,000** ([§6.2](#62-size)). (5) Fallback order if the projection passes the cap ([§6.3](#63-fallback)): the melt (T6 (ii), already optional) first, then the THING truncation (T4), then canonical records fall back to T3 (iv); the mixer's sample-exact comparison and audio output stay. (a)–(d) yes. **Recommendation: as stated.** **Confirmed by owner 2026-11-18.**

### Why Phase 1 stops here

This is kept as history; the rulings arrived on 2026-11-18 (see [Owner rulings](#owner-rulings-2026-11-18)). The brief reserves these decisions for the owner, and T2, T3, T5 and T12 rest on facts that contradict what the brief took for granted. Per the unattended rule, work continues only where no ruling can invalidate it ([§6.4](#64-what-proceeds-before-the-rulings)).

## 6. Plan, size estimate and cap

### 6.1 Steps

Phase 2 follows the brief's fourteen steps, one commit per step, pushed, with the status line at the top of this note updated after each. Each step runs in a fresh named subagent on its pinned model (AGENTS.md, model routing); one writer on the tree at a time.

| Step | What | Agent | Waits for |
| --- | --- | --- | --- |
| 1 | The synthetic sound lumps (`m5c_sfx.py`); rooms and scripts for every sound the Episode 1 simulation can start, the distance and stereo boundaries, the map-8 rule, more starts than channels, an origin removed while its sound plays, the switch origins, the refusal witnesses; the genuine sound-logic runner (`s_sound.c` and `f_wipe.c` whole, a recording `I_` layer, the `S_StartSound` recorder with positions) in the strict, overflow and address variants, identical; goldens from the C only | c-evidence | nothing |
| 2 | The genuine mixer harness (`i_sound.c` whole, the device calls wrapped) and its buffer digests for the same scripts' start lists, with the independent model as a second check | c-evidence | nothing |
| 3 | Wire 25 in `doom-sim` (T3, T4), with every older wire and every non-sound digest unchanged, proven by the full harness on the 61 runs and every synthetic item | c-evidence | T3, T4, T12 |
| 4 | `doom-sound`: the sound logic, matched start for start and channel for channel against the goldens; then the mixer, matched buffer for buffer | c-evidence | T2, T5, T11 |
| 5 | Real maps, local only: all 61 runs, every tic: sound records, starts, channels and the `M_Random` index against the genuine engine; then full frames at screenblocks 10 and 11 under the new contract; every divergence explained from the C, fixed and reproduced by a synthetic script | c-evidence | step 4 |
| 6 | The saved state: `DSND`, the display state's version 2, the restore-and-continue tests extended | c-evidence | step 4 |
| 7 | The app: the audio sink and the recording sink in `doom-game`, the device in `doom-play`, mute on pause, the volume flag; headless tests with a fake clock (same session in, same samples out, twice) | c-evidence | T7, T8, T9, T15 |
| 7b | Only if the owner wants it (T6 (ii)) and the cap allows: the melt | c-evidence | T6 |
| 8 | CI: Rust goldens on three operating systems; the genuine runs on Linux in a new job that is not a required check | docs-tools | steps 1–7 |
| 9 | Docs: COMPATIBILITY, STATE_FORMAT, ARCHITECTURE, TESTING, ROADMAP, `docs/validation/M5C.md`, README; an animation only if something visual changed | docs-tools | steps 1–8 |
| 10 | Mutation testing through the `run-mutants` label, in shards of at most about 40 minutes; every survivor dispositioned; the reviewer at xhigh effort checks every equivalent and unreachable disposition | tester, reviewer | steps 3–7 |
| 11 | Fuzz: a target for the sound logic and one for the mixer; seed replay, coverage-guided and AddressSanitizer campaigns, reported separately | tester | step 4 |
| 12 | Timings from ext4 on an idle machine: the cost per tic of sound logic and mixing; recorded, not gated | docs-tools | step 7 |
| 13 | Independent review at xhigh effort: the sound logic and mixer against the C line by line, every call site accounted for, the simulation change confined to wire 25, overclaiming in the docs | reviewer | steps 3–9 |
| 14 | Full gates from clean on ext4, the PR title and body, CI green on the exact head, mark the PR ready, stop | docs-tools | everything |

### 6.2 Size

Hand-written lines, projected:

| Part | Lines | Basis |
| --- | ---: | --- |
| Rust source: `doom-sim` wire 25 (900), `doom-sound` (1,650), `doom-render` additions (150), `doom-game` and `doom-play` (800) | 3,500 | [§4.11](#411-size-estimate-hand-written-lines-30) |
| Rust tests and fuzz targets | 3,600 | §4.11 |
| Tools: the sound-logic runner, the mixer harness and its fake device layer, the two independent models, the sound-lump and room generators, the DSIM41 encoder, the comparators, the local real-map tool | 2,800–4,500 | the two prototypes ([§3](#3-harness-design-evidence-first)); M5b's tools came to about 5,000 |
| Docs: this note (about 6,000 lines, most of it the C facts), the evidence record, COMPATIBILITY, ARCHITECTURE, STATE_FORMAT, TESTING, ROADMAP, README | 7,500–8,500 | M5b's note is 5,299 lines |

M5b's Rust landed at about 1.5 times its estimate for source and 3 times for tests, so the Rust range is taken as 7,100–15,000. **Projection: about 19,000–28,000 lines against a cap of 30,000** on hand-written code, tools and docs. The melt (step 7b) would add 800–1,200 and is outside the projection. Generated data verified by `--check` tools is reported separately: the sound-logic goldens, the buffer digests and start lists, the contracts and results files, the sound-lump WADs and the room WADs.

### 6.3 Fallback

If the projection passes the cap, in this order: the melt (step 7b) is not started; the THING truncation (T4) moves to a later wire; canonical sound records fall back to the non-canonical buffer of T3 (iv); the integer resampler is dropped where the library resamples; the switch-release reading falls back to T12 (ii). The mixer's sample-exact comparison, audio output, the real-map check and the closing of S15 stay in M5c.

### 6.4 What proceeds before the rulings

Steps 1 and 2 only, and inside them only what no ruling can invalidate: the sound-lump generator, the rooms and scripts, the genuine sound-logic runner and the genuine mixer harness, with the three choices that shape the goldens built as switches of the tools, not as assumptions: the handle convention (T2), where `S_UpdateSounds` runs and the buffer rule (T5), and the wipe's draws (T6). Goldens are generated for the recommended values and committed as such; a different ruling means running the same tools with another switch and regenerating. Nothing is done to the simulation, to `Cargo.toml` or to the app until the owner has ruled. **The owner ruled on 2026-11-18 ([Owner rulings](#owner-rulings-2026-11-18)) and confirmed the recommended value of each switch, so nothing was regenerated; steps 3 onward began the same day.**

## 7. Open questions and where each is decided

| # | Question | Decided |
| --- | --- | --- |
| 1 | Which build of linuxdoom-1.10 is the reference for sound | The owner, 2026-11-18: the in-process build (T2) |
| 2 | Wire 25, canonical sound records and the corrected scream | The owner, 2026-11-18: yes (T3) |
| 3 | The THING-loading truncation in wire 25 | The owner, 2026-11-18: yes, and nothing else (T4) |
| 4 | The declared schedule, and whether `S_UpdateSounds` runs per tic or per buffer | The owner, 2026-11-18: per tic, as recommended (T5) |
| 5 | The wipe: draws only, or the melt | The owner, 2026-11-18: draws only; the melt is the optional step 7b (T6) |
| 6 | The audio library, given that nothing here could make sound | The owner, 2026-11-18: maudio, after a proof that it compiles and links in CI on Windows and macOS; otherwise the pure-Rust fallback (T7) |
| 7 | The switch-release reading | The owner, 2026-11-18: the 64-bit reading with a detector (T12) |
| 8 | Whether the first genuine comparison finds more faults in wire 24's sound events | Phase 2 steps 3 and 5; by the T3 ruling each fault found is documented as a frozen-wire gap, fixed in wire 25 only and listed in the status line and the final report; more than about five, or any not about sound, stops the work |
| 9 | Whether maudio builds with MSVC and whether anything is audible under WSLg or on Windows | Phase 2 steps 7 and 8, and the owner's ears; no agent can hear |
| 10 | How it sounds: latency, clicks, loudness, the quirks of T13 | The owner, at the keyboard |
| 11 | Whether the new CI job becomes a required check | The owner, after Phase 2 step 8 |
