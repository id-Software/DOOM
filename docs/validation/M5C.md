# M5c evidence record

Observed results of the M5c milestone (sound effects matched to linuxdoom-1.10, and the closing of the status
bar face's idle glance gap). Only observations appear here. The
[M5c investigation](../M5C_REFERENCE_INVESTIGATION.md) owns the source facts, the owner decisions T1-T16 (all
open on 2026-11-18) and the plan; [testing](../TESTING.md#m5c-sound-effects-the-genuine-sound-logic-and-mixer)
owns the commands; [COMPATIBILITY](../COMPATIBILITY.md) owns the support statement, which has not changed:
no build makes sound yet. Exact review state and CI belong to GitHub.

This record was last extended on 2026-11-18 (America/Los_Angeles) with Phase 2 steps 1 and 2, the only steps that
may be done before the owner rules ([note, section 6.4](../M5C_REFERENCE_INVESTIGATION.md#64-what-proceeds-before-the-rulings)).
Nothing in the simulation, in `Cargo.toml` or in the app has changed. No CI job runs any M5c tool yet (step 8).

**Wording rule.** Every result below was produced under the **recommended** values of three decisions the owner
has not ruled on: T2 (the in-process mixer build, with `doomdef.h:84` removed, called the integrated handle
convention), T5 (`S_UpdateSounds` after every tic, and mixer buffer k mixed after tic floor(512k/315)) and T6
(the genuine wipe's 320 `M_Random` draws after the frame of tic 0). The tools take the other values as switches,
and the results files record what those give, as digests and counts; a different ruling means regenerating the
goldens. "Identical to linuxdoom-1.10" in this record always means the genuine source compiled into the
in-process build of T2, never the binary the source ships (which sends sound to an external `sndserver`
program). Nothing here says how anything sounds: no tool opens an audio device.

**Scope of the record.**

| Step | Date | State on 2026-11-18 |
| --- | --- | --- |
| 1 | 2026-11-18 | Recorded: the synthetic sound lumps, four rooms and five scripts, and the genuine sound-logic runner with its goldens ([below](#step-1-the-synthetic-sound-lumps-the-rooms-and-the-genuine-sound-logic-runner)) |
| 2 | 2026-11-18 | Recorded: the genuine mixer harness, the independent model, the goldens of 23 scenarios and of the 64 sound streams ([below](#step-2-the-genuine-mixer-harness-and-its-goldens)) |
| 3-7 | not started | Waits for the owner's rulings (wire 25 and the Rust sound logic, mixer, schedule and wipe) |
| 8-14 | not started | Waits for the owner's rulings (CI, commands, the audio backend, the app, visuals, reviews, final gates) |

No IWAD was read in step 1 except by `tools/local/m5c_real.py`, and in step 2 except by
`tools/local/m5c_mixer_real.py`. Both read a scratch copy outside the repository; only counts, pass/fail facts and
digests are recorded. Every address-checked run used `ASAN_OPTIONS=detect_leaks=0`: LeakSanitizer was disabled,
not passed. **No fuzzing and no coverage-guided campaign was run, no timing or benchmark was taken, and no audio
device was opened.** Compiler: gcc 15.2.0, x86-64, glibc. Every result was observed on one machine.

**Who observed what.** The results of each step below were produced by the step's agent in its own scratch
worktree, before the work was integrated on the branch. The note's
[status line](../M5C_REFERENCE_INVESTIGATION.md) lists what was run again on the branch afterwards, at the head
it names: the sound PWAD generator and the verifier's self-test (9 of 9 planted defects), the runner contract and
the room generator (`--check`), `m5c_reference.py --check --selftest` (the results file, the coverage manifest and
192 golden files for 64 items; 4 of 4 planted faults), `m5c_reference.py --strict32` (63 of 64 items identical),
`m5c_rust_check.py` (5 scripts, 1,599 tics), the mixer contract (`--check`), the bridge's self-test, and
`m5c_mixer_reference.py --check --selftest` (23 scenarios and 64 sessions, 741 session starts, 14,043 session
buffers, 5 of 5 planted faults). All exited 0. **Not re-run on the branch:** `tools/local/m5c_real.py`, whose
totals equal the note's section 3.4.2, and the mixer's `--strict32` (its last run was step 2f's, below). The
status line does not list `tools/local/m5c_mixer_real.py` among the re-runs either. The counts quoted here are
taken from the committed results files wherever one holds them, and otherwise from the step reports.

## Step 1: the synthetic sound lumps, the rooms and the genuine sound-logic runner

What was built, all new files (hand-written lines by `wc -l`; generated data separately):

- **1a, the sound PWADs.** `tools/reference/m5c_sfx.py` (207 lines) writes `tests/reference/m5c_sfx.wad` (107
  lumps, one distinct lump for every unlinked `S_sfx` name, 38,969 samples, 41,549 bytes), `m5c_sfx_edge.wad`
  (the edge cases of the note's section 3.3: lumps of 0, 5 and 8 bytes, 1, 511, 512, 513 and 1,024 samples,
  clipping lumps, a 22,050 Hz header, a wrong header count, a missing `pstart`, a `dschgun` the link ignores;
  41,707 bytes) and `m5c_music.wad` (the nine `D_E1M1`-`D_E1M9` placeholders `S_Start` looks up, our own ASCII,
  939 bytes). The tones are computed from integers: no id audio. The music lumps are a separate WAD so that
  `m5c_sfx.wad` keeps the digest the note's section 3.3 recorded. `m5c_check_sfx.py` (208 lines) is an independent
  verifier with its own parse of `sounds.c`.
- **1b, the runner.** `m5c_build.py` (274 lines) builds the M5b runner fork with `s_sound.c` and `f_wipe.c`
  compiled whole and unmodified and `i_sound.c` whole with two declared errno lines: 49 genuine files, 39 wrapped
  symbols, 32 function and 16 variable stubs; contract `tests/reference/m5c_runner_contract.json`
  (`DOOM-M5C-RUNNER-v1`), which declares the `doomdef.h:84` configuration adapter for the integrated handle
  convention. `m5c_driver.c` (1,983 lines; 481 added and 7 removed against the M5b driver) records the sound
  stream `DOOM-M5C-SOUND-v1` (rows S, T, K, C, L, R, W, E, Z) and adds Q and V rows to the frame stream;
  `m5c_stubs.c` (214 lines; 22 differ from the M5b stubs); `m5c_reference.py` (638 lines) runs the proofs. The
  switches are `--handle integrated|sndserv`, `--update-schedule tic|buffer`, `--wipe genuine|none` and
  `--sound on|off`. The compiler printed no warning for `s_sound.c`, `i_sound.c` or `f_wipe.c`.
- **1c, the rooms.** `m5c_wad.py` (285 lines) writes four rooms (`tests/reference/m5c_wads`, M5a framing, E1M1
  slot, skill 2) and five scripts (`tests/reference/m5c_scripts`: SNDDIST 48 tics, SNDBUSY 125, SNDMONST 452,
  SNDBTN1 721, SNDBTN2 248) with the manifest `m5c_manifest.json`. Between them and the 59 existing items every
  required sound and branch is reached; the measured record is `tests/reference/m5c_coverage.json`
  (`DOOM-M5C-COVERAGE-v1`). The 59 existing items alone lacked 13 sounds (`pstart`, `dorcls`, `popain`, `posit2`,
  `posit3`, `podth1`, `podth3`, `bgdth1`, `bgdth2`, `posact`, `noway`, `punch`, `bdopn`) and 10 branches
  (including no channel, the clip and close boundaries, a release out of earshot, a release with the next
  button slot busy and nine starts in four tics). `m5c_rust_check.py` (79 lines) is described under
  [the Rust room check](#the-rust-room-check).
- **1d, the goldens.** `tests/reference/m5c_sound` (64 files, 1,000,059 bytes), `tests/reference/m5c_frames`
  (128 files, 10,091,933 bytes) and `tests/reference/m5c_results.json` (882,926 bytes; switch values, contract and
  tool digests, per item and screen size the counts, endings, digests and the face check, and the three
  alternates as digests and counts), 64 items each at screenblocks 10 and 11.
- **1e, the real maps (local only).** `tools/local/m5c_real.py` (197 lines) and its record
  `tests/reference/m5c_real_results.json` (298,725 bytes; counts and digests only).

Hand-written: 4,085 lines in 1a to 1e (the nine files above), of which about 2,370 are new against the forked M5b
text. Generated data: about 12.85 MB, of which the frames are 10.09 MB.

**What was run** (by the step's agent):

```sh
python3 tools/reference/m5c_sfx.py --check
python3 tools/reference/m5c_check_sfx.py --self-test
python3 tools/reference/m5c_reference.py --work DIR --jobs 6                    # writes the goldens
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5c_reference.py --work DIR --check --selftest --jobs 6
python3 tools/reference/m5c_reference.py --work DIR --strict32                  # local opt-in
python3 tools/local/m5c_real.py --iwad SCRATCH/DOOM.WAD --work DIR --jobs 6 --write
```

**Proofs observed on the 64 items** (`tests/reference/m5c_results.json`; `--check` matched all 194 files, the 64
sound files, the 128 frame files, the results file and the coverage manifest):

- Masked never-render dumps equal the frozen M5b golden (59 items) or the frozen M5b runner (the 5 new items)
  on 64 of 64. With `--sound off --wipe none` the frame streams equal the frozen M5b frame goldens (or runner) on 64
  of 64 at screenblocks 10 and 11.
- Strict, overflow, address and a second strict run give identical frame and sound streams on 64 of 64 at both
  sizes. The sound stream is identical at the two sizes on 64 of 64, and, rendering every tic or never, on 61 of
  64 (M5OPEN, M5SSEG and M5VPLAN are the three that differ, as the note's section 3.1.8 records).
- The face check (only the face rectangle differs from the frame without sound; at screenblocks 11 no frame
  differs) passes on 64 of 64 at both sizes. Over the compared frames (150 flagged frames per size skipped): 22,620
  frames at each size; at screenblocks 10 the whole-frame digest differs in 12,183 of them and the frame outside the
  face in none, and at 11 the whole-frame digest differs in none (the face index changes in 12,183 frames at both
  sizes).
- The independent Python model of the audibility rules agrees with all 404 positional starts, with no
  mismatch. No `Z` row mismatch, no unknown origin and no draw of `M_Random` outside the status bar, the sound
  starts and the wipe.
- Totals: 45,540 frames (22,770 tics at each of two sizes); 785 starts, of which 741 started, 37 inaudible and 7
  found no channel (refused); 688 sound draws.
- Alternates at screenblocks 10: `--handle sndserv` 747 started and 1 refused; `--update-schedule buffer` 741 and 7;
  `--wipe none` 741 and 7. Their `rndindex` rows equal the recommended run on 64 of 64 for `sndserv` and `buffer`
  (and on 0 of 64 for `--wipe none`, as expected, because the wipe's 320 draws are absent), and their face rows
  equal it on 128 of 128 for the first two.
- Sanitizers (`ASAN_OPTIONS=detect_leaks=0`; leak detection disabled): no report in any sound file. The only reports
  are M5a's and M5b's known address-build stops in `R_StoreWallRange` (M5OPEN) and `R_ClipSolidWallSegment`
  (M5SSEG).
- Endings at screenblocks 10: 60 items end by marker, 1 by exit, and three end at a limit or engine error (M5OPEN
  `refused:OpeningOverflow`, M5SSEG `refused:SolidSegOverflow`, M5VPLAN `error:R_FindPlane: no more visplanes`).

**Self-test, four planted faults, each found** (`--selftest`): a volume changed in a copy of one stream changes
exactly one `S` row; the unmasked `rndindex` (`--no-mask-rnd`) changes the dump set; a pixel planted outside
the face is reported at tic 20 only; removing one item's coverage loses 8 required entries.

**Coverage.** The coverage manifest requires 47 sounds and 55 branches and lists none missing over the 64 items.
Two sounds and six branches are recorded as unreachable, each with its citation (`swtchx`: `p_switch.c:211`
zeroes the special before `:221` tests it; `bdopn`: only blazing doors start it, and wire 24 admits none of the
line specials that create one; the branches are the linked `chgun` volume, the same-origin channel case, the pitch
clamp, a bad sfx, a linked update and a stop at level start). The exact boundaries (distance equal to
`S_CLIPPING_DIST` and `S_CLOSE_DIST`, and half a map unit either side) were hit; half a unit is the smallest step
integer thing positions give, not one fixed-point unit.

### The 32-bit build

`m5c_reference.py --strict32` (a local opt-in that builds `-m32` and runs only the comparison, not `--check`)
found 63 of 64 items with sound and frame streams identical to the 64-bit strict build. The one differing row is
the switch release in SNDBTN2 with the next button slot busy: the 64-bit runner reads the position x, y = 34, 0 and
the 32-bit runner 19, 34; volume (4), separation (35), outcome, pitch and every frame are the same. It is the
layout-dependent read of decision T12: the release sound's position comes out of the next button slot, and the
fields in it sit at different offsets. All other button rows are identical (M5SWTCH 73 rows, SNDBTN1, and SNDBTN2's
other release with the following slot free: 0, 0).

### The Rust room check

`m5c_rust_check.py` runs the existing wire-24 comparison unchanged on the five scripts: the frozen M4b runner,
`player_level_run --policy episode1v2 --captures` (built by the caller) and `level_compare_m4b.py`. All 5 scripts
`match`, 1,599 tics compared. The five scripts have 1,594 tics in the manifest; the one more per script is what the
comparator counts, and this record does not verify why. This shows the rooms stay inside what the wire-24
simulation admits. It compares no sound.

### The real Episode 1 runs (local only)

`tools/local/m5c_real.py` ran once on a scratch copy of DOOM.WAD (its SHA-256 equal to the original's; deleted
afterwards) at 6 jobs, on the 61 runs of `tests/reference/m4b_real_results.json`, with the IWAD only in the lump list.
Observed (`tests/reference/m5c_real_results.json`): masked never-render dumps equal the M4b strict digests on 61 of
61; with `--sound off --wipe none` the frame stream equals the M5b genuine stream digest on 61 of 61 at both
sizes; strict, overflow and address and a second strict run are identical on 61 of 61 at both sizes; the sound
stream is identical at both sizes on 61 of 61; no sanitizer report. Totals: 158,966 tics; 26,077 starts, 21,090
started, 4,877 inaudible, 110 refused; 20,697 sound draws; the face rectangle differs in 68,995 of 158,472 compared
frames at screenblocks 10 and no frame differs of 158,414 at 11; endings 27 marker, 12 exit, 22 reborn. The
alternates give 21,189 started and 11 refused (`sndserv`) and 21,073 and 127 (buffer schedule). The model agrees
with all 20,097 positional starts. All equal the note's sections 3.4.2 and 3.1.9-3.1.10. `m5c_real.py --check` was
not run afterwards by the step, and not on the branch.

**Not run:** `m5c_real.py --check` after the write; the mixer (step 2); any fuzz or coverage-guided campaign;
any run on Windows or macOS; any CI job; any timing.

## Step 2: the genuine mixer harness and its goldens

What was built, all new files:

- **2a, the harness.** `tools/reference/m5c_mixer_build.py` (339 lines) builds `m5c_mixer_driver.c` (451 lines)
  with the genuine `i_sound.c`, `w_wad.c`, `z_zone.c` and `sounds.c` compiled whole, behind a declared fake OSS
  device (`m5c_mixer_fakesnd.c`, 90 lines, with the fake header `m5c_fakesnd/linux/soundcard.h`, 16 lines; `open`,
  `close`, `ioctl`, `write`, `sigaction` and `setitimer` are wrapped at link time) and five declared stubs
  (`m5c_mixer_stubs.c`, 37 lines). The adapters are one-line replacements at declared lines: `doomdef.h:84` commented
  out (every build), the two errno lines of `i_sound.c:27` and `:166` (every build) and `z_zone.c:440` (the LP64
  builds). The contract is `tests/reference/m5c_mixer_contract.json` (`DOOM-M5C-MIXER-v1`; 22,884 bytes): 4
  whole files, 1 configuration, 2 errno and 1 ABI adapter, 2 function and 3 variable stubs, 6 wrapped symbols. The
  driver reads a start list (`DOOM-M5C-STARTS-v1`) and runs it under `--schedule ticbuffers` (default: buffer k
  after tic floor(512k/315)) or `pertic`; per start it writes an echo row (handle, mixer channel, step, volume rows,
  occupancy, the previous sfx, channels freed) and per buffer the SHA-256 of the 2,048 bytes `I_SubmitSound` wrote.
- **2b, the independent model.** `m5c_mixer_model.py` (361 lines, standard library only, no code shared with the C
  driver; its own parse of `sounds.c` and its own WAD reader). It writes the same rows as the driver except the two
  device rows, and refuses (an `x` row) a start where the genuine code reads past its block or past the
  step table. It is a second check, never a source of goldens.
- **2c, the scenarios.** `m5c_mixer_reference.py` (817 lines now; 650 at step 2c) writes the 23 start lists of
  `tests/reference/m5c_mixer/starts` (volumes 0, 1, 8, 15 and 127; separations 33, 128 and 224 and the wider values
  the mixer accepts; pitches 0, 113, 121, 128, 135, 136, 144, 166 and 255; the single-instance sounds; ten starts
  in one tic; the oldest-channel rule with its ties; clipping; buffer boundaries; the linked chaingun; the header
  fields; padding; the missing lump and the pistol fallback; the handle wrap; a tic-phase list; a 300-start busy
  list; four `I_Error` probes outside the argument domain of `s_sound.c`), 3 short-lump witnesses, 1 probe, the
  goldens (strict build, `ticbuffers`) and `tests/reference/m5c_mixer_results.json` (17,305 bytes).
- **2d, the bridge.** `m5c_starts.py` (199 lines) converts a `DOOM-M5C-SOUND-v1` stream into a start list; it
  refuses `sndserv` streams (their handles are sound ids and their audio comes from the external program).
- **2e, the IWAD run (local only).** `tools/local/m5c_mixer_real.py` (172 lines) and
  `tests/reference/m5c_mixer_real_results.json` (3,368 bytes).
- **2f, the sessions.** For each of the 64 sound streams of step 1: the start list
  `tests/reference/m5c_mixer/sessions/starts/<ITEM>.txt` (45,407 bytes in 64 files) and the strict build's rows
  `sessions/goldens/<ITEM>.golden` (189,115 bytes in 64 files), with
  `tests/reference/m5c_mixer_sessions_results.json` (`DOOM-M5C-MIXER-SESSIONS-RESULTS-v1`, 48,639 bytes). It is a
  separate file so that `m5c_mixer_results.json` keeps meaning the scenario set alone. `m5c_mixer_reference.py`
  gained the group and the README of `tests/reference/m5c_mixer` describes it.

Hand-written: 2,557 lines now (2,378 at step 2e), including the README (73 lines). Generated data: the 23 goldens are
86,550 bytes, the 27 start lists 20,856 bytes (23, 3 and 1), the contract 22,884, the scenario results 17,305, the
real results 3,368, and the session files above.

**What was run** (by the step's agent, from fresh work directories):

```sh
python3 tools/reference/m5c_mixer_build.py --check
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5c_mixer_reference.py --work DIR --check --selftest --strict32 --jobs 6
python3 tools/reference/m5c_starts.py --selftest
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5c_mixer_reference.py --work DIR --sessions tests/reference/m5c_sound --strict32 --jobs 4
python3 tools/local/m5c_mixer_real.py --iwad SCRATCH/DOOM.WAD --iwad SCRATCH/DOOM2.WAD --work DIR --write   # then --check
```

**Observed on the scenarios** (`tests/reference/m5c_mixer_results.json`):

- The strict, overflow and address builds write byte-identical rows under both schedules on 23 of 23 scenarios,
  and the independent model's rows equal them (device rows aside) on 23 of 23; no `m` row. The opt-in `--strict32`
  build is identical on 23 of 23, and on the 3 witnesses it gives a different stream.
- 490 buffers, 65,882 starts (65,438 of them in the `handle_wrap` scenario), 86,550 bytes of goldens.
- The tables equal the integer reconstruction, entry by entry: the step table to floor(2^(16+(p-128)/64)) by an
  integer root, the volume table to truncating division. Their SHA-256 digests start `95f2209c` and `f727268a`, equal
  to the note's section 3.2.6. The smallest distance of a step table value to an integer is 12,529,933 billionths
  (0.012529933), at pitch 112.
- **Witnesses of decision T11.** The three lumps of 8 bytes or fewer (`w_short0`, `w_short5`, `w_short8`) are each
  flagged by an `o` row at the same start (tic 2, call 0) in the strict, overflow and address builds, and the model
  refuses that start with every earlier row equal. They are never goldened.
- **Sanitizer.** The only report is the declared probe `p_pitch256`: UBSan "index 256 out of bounds for type
  'int [256]'" at `i_sound.c:495:36` (line 496 in the compiled file, which one errno line lengthens; the tool maps it
  back). The overflow build reported nothing anywhere. Nothing is reported in the argument domain of `s_sound.c`.
- **Self-test, 3 of 3** (before the sessions): a flipped bit in byte 5000 of a stream changes only buffer row `b 2`
  and `z`; a start moved from tic 3 to tic 4 makes `s 3 0` the first difference with 4 buffer rows differing; the
  volume of the tic-10 pistol changed from 127 to 126, replaced before any buffer is mixed, changes only `s 10 0`.
- The bridge's self-test passes 12 of 12.

**Observed on the 64 sessions** (`tests/reference/m5c_mixer_sessions_results.json`): all 64 streams convert (741 starts,
none refused); no `m` row, so every handle the genuine mixer returned equals the handle the sound-logic run
recorded; strict, overflow, address and strict32 identical and the model identical under both schedules on 64 of 64;
no sanitizer line; 14,043 buffers under `ticbuffers` over 22,770 tics. Two further self-test cases bring it to 5 of
5: with the separation of the start at tic 0, call 1 of SNDDIST changed from 33 to 224 in a copy of the streams,
the comparison fails for SNDDIST only (the list and the digest), and the genuine run first differs from the
committed golden at row `s 0 1`, then one buffer row and `z`. A hand-altered session golden fails `--check` at its
first buffer row (SNDBUSY, row `b 0`); the file was restored and compared with `cmp`.

**The IWADs (local only; counts only).** On scratch copies of DOOM.WAD and DOOM2.WAD (deleted afterwards), the
counts of the note's section 3.2.8 (`tests/reference/m5c_mixer_real_results.json`):

| | present of 107 | fall back to `dspistol` | sample counts that are multiples of 512 | header rate not 11,025 | 8 bytes or fewer | `dschgun` |
| --- | --- | --- | --- | --- | --- | --- |
| DOOM.WAD | 67 | 40 | 0 | 1 | 0 | absent |
| DOOM2.WAD | 107 | 0 | 0 | 4 | 0 | absent |

Starting every sound 1 to 108 once, 20 tics apart, gave 1,329 buffers (DOOM.WAD) and 1,334 (DOOM2.WAD) under
`ticbuffers` and 2,152 and 2,149 under `pertic`; the four builds were identical, the model was identical on both
IWADs and both schedules, and there was no sanitizer line. Without an IWAD the tool prints one skip line and exits
0.

**Not run:** `m5c_mixer_real.py` after the branch integration; any fuzz or coverage-guided campaign; any CI job;
any timing; any audio device. The fresh `--strict32` run of step 2f (23 of 23 scenarios and 64 of 64 sessions
identical) was not repeated on the branch.

## Findings

- **The 32-bit build.** 63 of 64 sound-logic items are identical; the one row that differs is the
  SNDBTN2 switch release (above). This is decision T12's layout-dependent read, observed.
- **The reachable sound set is 47, not 49.** The note first listed 49. `swtchx` cannot be started in this source
  (`p_switch.c:211` against `:221`) and wire 24 admits no blazing-door special, so `bdopn` cannot be started. 55
  required branches; all 47 sounds and all 55 branches are reached by the 64 items. Step 1 built four rooms and
  five scripts rather than the eight the note proposed, because the existing items already reach map-8 near and far
  sounds, the saw sounds, removal stops and pickups.
- **The three short lumps (decision T11).** Lumps of 8 bytes or fewer are flagged in every contract build and refused
  by the model, and are not goldened. Changing the zone size does not change the byte the genuine code reads (zone
  sizes of 6, 7, 8 and 16 MB gave the same stream): the byte lies in the next block's header, not in free space.
  Only the 32-bit build reads a different one. So in CI without `-m32` the witnesses will show as flagged, not as
  differing between builds.
- **The pitch-256 probe.** The only sanitizer report, and outside what `s_sound.c` can pass: the coverage manifest
  records that the pitch it computes is 128 plus at most 16 either way (`s_sound.c:331-345`), so never outside 0
  to 255.
- **M5OPEN and M5VPLAN.** Both end in an engine or limit stop (tics 39 and 48) with no starts and no `E` or `Z`
  row in the committed stream. The error text of M5OPEN varies from run to run, which made the alternates' raw
  sound digest unstable; the digest now leaves out `E` rows. It is not in the committed streams.
- **Silent items.** 23 of the 64 items start no sound (listed in `items_without_starts` of the sessions results,
  including M5OPEN and M5VPLAN) and are goldened as silent buffers; for example M5OPEN has 25 buffers under
  `ticbuffers` and 40 under `pertic`.
- **Handles.** Every handle the genuine mixer returned equals the handle the sound-logic run recorded, on all 64
  streams.
- **Frame goldens are about twice the size of M5b's** (10.09 MB in 128 files against 5.1 MB in 118): the Q rows add
  about 80 bytes a frame and the V rows 41 bytes a run.
- **The edge WAD is mixer-only.** `m5c_sfx_edge.wad` lacks `DSPSTART`, which plats start, so it cannot be used with
  the Episode 1 sessions; the note's section 3.3 said its edge names never replace a sound the synthetic sessions
  need.
- **One commit message is wrong.** The message of step 2b says the Phase 1 prototype had 21 scenarios; it had 20
  (12 in-domain and 4 `I_Error` probes identical to the model, 3 short-lump and 1 pitch probe refused). The message
  of step 2c and the later ones are right; nothing was amended.
- **A documentation slip found on the way.** The M5b section of TESTING listed a `--strict32` option for
  `m5b_reference.py` that the tool does not have; it was corrected on 2026-11-18.

## What a human still has to judge

Nobody has heard any sound from this project: every audio trial in Phase 1 wrote digital silence and no tool opens
an audio device. The evidence above compares which sounds start, on which channel, with which volume, separation and
pitch, and the 16-bit samples the original mixer writes for them; it cannot say how any of it sounds, whether the
synthetic or the real samples are pleasant, or whether a backend will play them without gaps. The owner has also
not playtested M5b, so input, pacing and presentation are not to change on a guess about feel.

## Still to come in this record

Steps 3 to 14, after the owner rules on decisions T1-T16: wire 25 and the Rust sound logic, the mixer, the schedule
and the wipe, CI (step 8), the audio backend, the app, the visuals, the reviews and the final gates.
