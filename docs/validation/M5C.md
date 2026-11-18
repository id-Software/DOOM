# M5c evidence record

Observed results of the M5c milestone (sound effects matched to linuxdoom-1.10, and the closing of the status
bar face's idle glance gap). Only observations appear here. The
[M5c investigation](../M5C_REFERENCE_INVESTIGATION.md) owns the source facts, the owner decisions T1-T16 (all
confirmed on 2026-11-18) and the plan; [testing](../TESTING.md#m5c-sound-effects-the-genuine-sound-logic-and-mixer)
owns the commands; [COMPATIBILITY](../COMPATIBILITY.md#sound-effects-m5c) owns the support statement and
[ROADMAP](../ROADMAP.md) the milestone status. Exact review state and CI belong to GitHub.

This record was last extended on 2026-11-18 (America/Los_Angeles) with the pieces of the milestone that are complete
on the branch: Phase 2 steps 1 and 2, the proof of the audio library (decision T7), step 3 (wire 25 and its genuine
comparisons), step 4 (the crate `doom-sound` with its session, detector and saved state), step 8 part 1 (the CI job
and the mutation workflow) and step 11 part 1 (the fuzz targets). **Still in progress and not recorded here:** the
display contract with sound, the game loop, the device and launch flags, the real-map runs of the full sound logic and
of frames, so **the face's idle glance gap (S15) is not closed**; also mutation testing, timings and the reviews.

**What has and has not been heard or done.** Nobody has heard any sound from this project. No audio device has been
opened on any system: the one program that can open one, `doom-play --audio-probe`, failed loudly on the development
machine (it found no backend). Every statement below is about which sounds start, on which channel, with which volume,
separation and pitch, and the 16-bit samples the original mixer writes for them. None says how anything sounds.

**Wording rule.** The reference is **linuxdoom-1.10 built with its in-process sound code (one define removed)**
(decision T2: `doomdef.h:84` removed, the integrated handle convention); it is never the binary the source ships
(which sends sound to an external `sndserver` program) and never what a 1997 player heard. The owner confirmed on
2026-11-18 the values the tools were built with: T2 (that build), T5 (`S_UpdateSounds` after every tic, and mixer
buffer k mixed after tic floor(512k/315)) and T6 (the genuine wipe's 320 `M_Random` draws after the frame of tic 0), so
no golden was regenerated. The tools still take the other values as switches.

**Kinds of evidence, kept apart.** Four kinds appear below, and each result says which it is:
**source equivalence** (the Rust result equals what the genuine C engine wrote, for the same inputs, tic for tic);
**independent-encoder consistency** (the expected bytes come from a Python encoder that shares no code with Rust, fed
by the genuine engine's own output, never by Rust's); **Rust against Rust** (two Rust configurations compared with
each other, which shows a change did nothing else, not that either is right); and **Rust-only safety tests** (bounds,
refusals and atomicity, built from cited C facts, not equivalence). For fuzzing the record separates: the target
compiled; seed replay; a coverage-guided campaign; a sanitizer campaign.

**Scope of the record.**

| Piece | State on 2026-11-18 |
| --- | --- |
| Step 1: genuine sound-logic runner, rooms, goldens | Recorded ([below](#step-1-the-synthetic-sound-lumps-the-rooms-and-the-genuine-sound-logic-runner)) |
| Step 2: genuine mixer harness and goldens | Recorded ([below](#step-2-the-genuine-mixer-harness-and-its-goldens)) |
| The audio library proof (T7) | Recorded ([below](#the-audio-library-proof-decision-t7)) |
| Step 3a: wire 25 in `doom-sim` | Recorded ([below](#step-3a-wire-25-in-doom-sim)) |
| Step 3b: independent DSIM41 encoder, comparators, genuine comparisons, T4 | Recorded ([below](#step-3b-the-independent-dsim41-encoder-the-comparators-and-the-genuine-comparisons)) |
| Step 4 part 1: tables, lumps, mixer, sound logic | Recorded ([below](#step-4-part-1-the-sound-tables-the-mixer-and-the-sound-logic)) |
| Step 4 part 2: schedule, session, detector, saved state, end-to-end gate | Recorded ([below](#step-4-part-2-the-session-the-detector-the-saved-state-and-the-end-to-end-gate)) |
| Step 8 part 1: the CI job and the mutation workflow | Recorded ([below](#step-8-part-1-the-ci-job-and-the-mutation-workflow)) |
| Step 11 part 1: fuzz targets | Recorded ([below](#step-11-part-1-the-fuzz-targets)) |
| Display contract, game loop, device and flags, real-map runs of the full logic and frames, mutation testing, timings, reviews, final gates | Not done; not recorded here |

No IWAD was read in step 1 except by `tools/local/m5c_real.py`, in step 2 except by `tools/local/m5c_mixer_real.py`,
and in step 3 except by `tools/local/m5c_wire25_real.py`. Each read a scratch copy outside the repository; only counts,
pass/fail facts and digests are recorded. Every address-checked run used `ASAN_OPTIONS=detect_leaks=0`: LeakSanitizer
was disabled, not passed. Compiler for the C tools: gcc 15.2.0, x86-64, glibc. Every result was observed on one machine.

**Who observed what.** The results of each step below were produced by the step's agent in its own scratch worktree,
before the work was integrated on the branch; the counts are taken from the agents' reports and, wherever one holds the
figure, from the committed results files (the places where a report and a file differ in wording are listed under
[Findings](#findings)). The note's [status line](../M5C_REFERENCE_INVESTIGATION.md) lists what was run again on the
branch at the head it names for steps 1 and 2: the sound PWAD generator and the verifier's self-test (9 of 9 planted
defects), the runner contract and the room generator (`--check`), `m5c_reference.py --check --selftest` (the results
file, the coverage manifest and 192 golden files for 64 items; 4 of 4 planted faults), `m5c_reference.py --strict32`
(63 of 64 items identical), `m5c_rust_check.py` (5 scripts, 1,599 tics), the mixer contract (`--check`), the bridge's
self-test, and `m5c_mixer_reference.py --check --selftest` (23 scenarios and 64 sessions, 741 session starts, 14,043
session buffers, 5 of 5 planted faults). All exited 0. **Not re-run on the branch:** `tools/local/m5c_real.py`, whose
totals equal the note's section 3.4.2, the mixer's `--strict32` (its last run was step 2f's, below) and
`tools/local/m5c_wire25_real.py --check`. The documentation pass that wrote the sections for step 3 onward re-ran no
command: every figure is the step agent's.

## Step 1: the synthetic sound lumps, the rooms and the genuine sound-logic runner

What was built, all new files (hand-written lines by `wc -l`; generated data separately):

- **1a, the sound PWADs.** `tools/reference/m5c_sfx.py` (207 lines) writes `tests/reference/m5c_sfx.wad` (107
  lumps, one distinct lump for every unlinked `S_sfx` name, 38,969 samples, 41,549 bytes), `m5c_sfx_edge.wad`
  (the edge cases of the note's section 3.3: lumps of 0, 5 and 8 bytes, 1, 511, 512, 513 and 1,024 samples,
  clipping lumps, a 22,050 Hz header, a wrong header count, a missing `pstart`, a `dschgun` the link ignores;
  41,707 bytes) and `m5c_music.wad` (the nine `D_E1M1`-`D_E1M9` placeholders `S_Start` looks up, our own ASCII,
  939 bytes). The tones are computed from integers: no id audio. The music lumps are a separate WAD so that
  `m5c_sfx.wad` keeps the digest the note's section 3.3 recorded. `m5c_check_sfx.py` (213 lines) is an independent
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

Hand-written: 4,090 lines in 1a to 1e (the nine files above; 4,085 as the step ended, plus 5 added to `m5c_check_sfx.py` on 2026-11-18 so that `--help` prints its usage and an unknown option is refused), of which about 2,370 are new against the forked M5b
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

**Not run in this step** (later pieces are recorded below): `m5c_real.py --check` after the write; the mixer (step 2); any
fuzz or coverage-guided campaign; any run on Windows or macOS; any CI job; any timing.

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

**Not run in this step** (later pieces are recorded below): `m5c_mixer_real.py` after the branch integration; any fuzz or
coverage-guided campaign; any CI job; any timing; any audio device. The fresh `--strict32` run of step 2f (23 of 23 scenarios and 64 of 64 sessions
identical) was not repeated on the branch.

## The audio library proof (decision T7)

The owner's ruling on T7 required that the library (maudio, the miniaudio C library behind safe Rust bindings) be shown to
compile and link in CI on Windows and macOS before anything is built on it. This piece did that and added the one device
file and a diagnostic. **It opened no device and played nothing.**

**What was built.** `crates/doom-play/Cargo.toml` depends on `maudio = "=0.1.15"` with its default features (with
`default-features = false` maudio 0.1.15 does not compile: E0433 at `context.rs:53` and `device/device_builder.rs:83`,
which import `crate::engine`, gated behind the `engine` feature at `lib.rs:145-146`). `Cargo.lock` gained two package
entries (maudio 0.1.15, maudio-sys 0.1.9) and one dependency line; no existing entry changed, and the build crate `cc`
was already in the lock at 1.10.0. `crates/doom-play/src/audio.rs` names the backends per system (Windows: WASAPI,
DirectSound, WinMM; macOS: Core Audio; Linux: PulseAudio, ALSA; JACK left out; any other system: none, so opening fails
with a typed error). It tries each backend alone in its own context, so the opened backend is known, with 16-bit stereo
at 11,025 Hz and a period hint of 512 frames (a hint only; the note names no period). `doom-play --audio-probe` opens a
device, writes about one second of zeros, prints the backend and device, and exits 0; on failure it prints every backend
tried and exits 1; with any other argument it exits 2. The workspace's `unsafe_code = "forbid"` lint applies to it.
Hand-written: 287 insertions and 3 deletions (`audio.rs` 208, of which about 60 are tests; `main.rs` 59 and 3; manifests).
Nothing is generated.

**What was observed on the development machine (Linux, WSLg).** `cargo build -p doom-play` (debug, and with the 1.85.0
toolchain), `cargo clippy -p doom-play --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `cargo test -p
doom-play` (11 passed, 0 failed, 3 of them new) and `cargo doc` with warnings denied all passed; the release build links
192 `ma_*` symbols and depends on libgcc_s, libm and libc only. The probe, with `PULSE_SERVER` pointing at the WSLg socket and
no PulseAudio or ALSA client library installed (release and debug, under `timeout 20`) printed
`doom-play: audio probe: no audio device opened; backends tried: PulseAudio (MA_NO_BACKEND (-203)), ALSA (MA_NO_BACKEND (-203))`
and exited 1. `--audio-probe --iwad x` exited 2. **So the probe failed loudly, as designed, and no device has been opened.**

**CI.** The note's status line records that CI at the head with this change was green on all eight jobs, that the
Windows artifact job was green on the same head, and that the logs show `maudio-sys` and `maudio` compiled and
`doom-play` built and tested on `macos-26-intel` and `windows-latest`, and built in release with the static C runtime in
the artifact job. Exact CI is GitHub's to show. What CI cannot show: that a device opens on any system; the probe is
never run there (no runner has an audio device).

**Licences of the new crates** (the SPDX fields and the LICENSE files, read by the agent; not legal advice): maudio
0.1.15 MIT; maudio-sys 0.1.9 MIT; the vendored miniaudio Unlicense or MIT-0; nothing incompatible with GPL-2.0-only was
flagged.

**Not run:** a workspace-wide build or test, fuzzing, sanitizers, a Windows or macOS build on this machine, or any
playback. A newer maudio on crates.io was not checked online (`cargo info` showed 0.1.15).

## Step 3a: wire 25 in doom-sim

What was built (crates/doom-sim only; support and layout are in
[COMPATIBILITY](../COMPATIBILITY.md#original-episode-1-part-3-canonical-sound-records-wire-25) and
[STATE_FORMAT](../STATE_FORMAT.md#dsim-v41--wire-25-canonical-sound-records)): the profile
`PlayerMonstersEpisode1V3`, the public record types at `doom_sim::sound` (the crate root already exports a different
`SoundOrigin`, the frozen mover-event enum, which could not be renamed), the DSIM41 writer and reader, sector sound
origins computed once at load, the two switch origins, the corrected scream, the THING-loading truncation, read-only
accessors for the sound logic, and the example `sound_records_run`. Each positional emitter pushes, right after its
unchanged wire-24 event, a transient record carrying the placement read at the call; the records are taken out in event
order before the tic commits. The frozen wires take no new branch.

**What was run and observed** (by the step's agent; Rust against Rust unless said):

- **Checks 1 and 2 on the 64 synthetic items** (committed test `wire25_projection`): 64 items, 22,976 captures; every
  wire-25 capture with the sound group and header fields stripped equals the wire-24 capture, wire 25's events equal
  wire 24's except the scream, and the pending lists are equal; 785 starts (404 with positions) and 378 stops; the
  scream replaced 0 times; the one item that leaves the level (BOSS1) ends in `LevelExited` under both wires. Every wire-25
  capture decodes with `decode_episode1_v3` and is refused by `decode_episode1_v2`. A second test checks the item table
  equals the 64 committed genuine streams.
- **The same checks on the 61 real Episode 1 runs** (local only, a scratch IWAD copy, deleted afterwards): 61 of 61 pass;
  159,027 captures (158,966 tics plus the 61 loads); 26,077 starts (20,097 with positions) and 10,470 stops, so the
  26,077 starts equal the note's count of genuine start attempts on these runs (section 5, T2); the scream replaced 0
  times; 11 runs end in a `PlayerReborn` refusal under both wires (the demos' deaths) and 50 run to the end.
- **A first comparison with the genuine streams** (scratch tool, never committed; superseded by step 3b): all 64
  items agreed on the 785 starts, the 404 positions and the 378 stops; it failed as it should on two planted faults (a
  flipped angle bit in SNDBTN2, and the listener read from the stale store at in-move crossings in SNDBUSY, which the
  fix reads from the mover's current record), both reverted.
- **Nine unit tests** (`src/wire25_tests.rs`; Rust-only, built from cited C facts): the scream (it fails with the fix
  removed: checked), the bounding-box arithmetic by hand values, every sector origin against the C loop nest, overflow
  refusal on a room moved 17,000 units east (a shift of 16,000 loads), release placement and slot 15, the stop record of
  an explicit removal, the split order and three `Unplaced` refusals, the truncation (each of the ten types, before and
  after the start), and the DSIM41 round trip with four forged-record refusals.
- **Final checks**: `cargo test -p doom-sim` in debug and in release, each 79 test binaries, 1,373 passed, 0 failed, 2
  ignored; clippy with warnings denied and `cargo fmt --check` clean; `cargo +1.85.0 check -p doom-sim` builds, with one
  warning (`MAX_LINKED_THINGS` never used, `player_map_scale.rs:74`) that also exists before this change; rustdoc with
  warnings denied clean; `cargo test --release -p doom-render -p doom-game` 20 binaries, 333 passed; and
  `git diff --stat 78d9717 -- tests/reference` was empty at the time (no file there changed or was added).

**Sizes** (`git diff --stat` from the base): 22 files, 2,151 insertions and 57 deletions, all hand-written except the
64-row item table in `tests/wire25/mod.rs` (66 lines, generated once from the M5b item list and the M5c manifest).

**Frozen-wire fault found:** one, the known hard-death scream (reading, not a run). **Not done in this step:** the
committed genuine comparison and the independent encoder (step 3b), the T12 detector and any sound logic, a fuzz target,
a sanitizer run, mutation testing, a timing.

## Step 3b: the independent DSIM41 encoder, the comparators and the genuine comparisons

What was built, all new files except where said: `tools/reference/level_snapshot_m5c.py` (the DSIM41 encoder: it imports
the frozen DSIM40 projection and adds the version word, `profile`, the context and the sound group, built from the genuine
`S` and `T` rows, never from Rust output; a rows-to-records mapping through the encoder's own actor store);
`level_compare_m5c.py` (the per-tic comparator of a wire-25 run); `m5c_wire25_check.py` (every synthetic item, genuine
against wire 25); `m5c_t4.py` (the T4 map, script and results); the local `tools/local/m5c_wire25_real.py`; and two
extensions of the step 3a example (`--captures`, `--alone`, `--capture-stream`). Results files:
`tests/reference/m5c_wire25_results.json` (schema `DOOM-M5C-WIRE25-RESULTS-v1`),
`m5c_wire25_real_results.json` (`DOOM-M5C-WIRE25-REAL-RESULTS-v1`) and `m5c_t4_results.json`
(`DOOM-M5C-T4-RESULTS-v1`), plus the map `tests/reference/m5c_t4/truncate.wad` (86,537 bytes), `TRUNC.txt` and
`TRUNC.lmp` (382 bytes). Hand-written: 1,238 lines (Python 1,111 in five tools; Rust 123; CI 9). Generated: the three
results files (1,816, about 1,917 and 51 lines), `TRUNC.txt` 7 lines and the two binaries above.

### The 64 synthetic items (independent-encoder consistency and source equivalence)

`m5c_wire25_check.py --check --selftest --jobs 4` (29 seconds with the binary built). Against the frozen M4b runner's
dumps and the M5c runner's fresh sound stream, projected by the independent encoder, the wire-25 captures match on
**64 of 64 items** (`tests/reference/m5c_wire25_results.json`): 22,976 compared captures (the file calls them `tics`;
each item's count equals its snapshots, so it includes the load capture), 785 starts (404 with positions), 378 stops
and 0 sound disagreements. The map group, the whole engine outside `map.*` and `events.*`, every sound record's order,
tic, sound number, origin kind and identity, every placed start's positions and every stop are compared. Each fresh
genuine stream's `S` and `T` rows equal the committed golden's on 64 of 64 (the cheat item M5CHEAT is recorded only: its
golden was made with cheat keys that the frozen runner and the simulation do not see, and its sound rows are empty,
0 starts; the file records it equal to its golden). The M4b scripts' dumps equal their committed digests. Items that end at
a renderer error in the golden (M5VPLAN, M5OPEN) are compared through that tic. One item (BOSS1) ends at the level exit:
the genuine engine exits at its tic 545 and wire 25 refuses that tic (`LevelExited`, DSIM tic 546, which numbers the
capture after the tic); the other 63 run to their last capture on both sides.

**Self-test, 73 checks, each passing:** DSIM41 equals DSIM40 outside `profile`, `context.schema` and `events.sounds` on
every capture of all 64 items; a changed sound number makes the group differ at that tic only; an unknown actor, a mobj
start without a position, a stop of a sector origin and a stream cut 10 tics short are each refused at the right tic;
and four planted faults on BARON1 (a flipped position bit, a swapped pair, a dropped stop, an actor origin made NULL)
are each reported at exactly that tic and no other.

### The 61 real runs (local only, wire 25)

`m5c_wire25_real.py --jobs 6 --write` on a scratch copy of DOOM.WAD (deleted afterwards), about 50 minutes
(`tests/reference/m5c_wire25_real_results.json`, which holds counts, pass/fail facts and digests only): **61 of 61 runs
match tic for tic**; 159,027 captures compared (158,966 tics plus the 61 loads); 26,077 starts, 20,097 with positions,
10,470 stops; 0 sound disagreements; every dump set equals its M4b strict digest (61 of 61) and every fresh genuine
stream gives its committed `m5c_real_results.json` sound digest (61 of 61); the genuine engine's endings are 27 by marker,
12 by exit and 22 by the demo's death (reborn), and on the Rust side 11 of those 22 end in a `PlayerReborn` refusal and
the other 50 runs reach their last capture. The 31 runs driven by the M4b driver and 30 by the M4a driver are all in the
file. **`m5c_wire25_real.py --check` has not been run since the file was written** (about an hour).

### The THING-loading truncation (decision T4)

`m5c_t4.py --check`: one synthetic E1M1 map whose things in lump order are the player start, a zombieman, a type-65
Former Human Commando, a second zombieman and a stimpack, and a script that fires the pistol and waits (skill 2, 93
tics). The genuine engine under the whole-engine runner's registered mode loads **2 mobjs**, and its 93 snapshots hold 4
starts (3 with positions) and 1 stop. Wire 25 matches on 93 of 93 captures with 2 actors at load and 0 sound
disagreements. Wire 24, run by the frozen `player_level_run --policy episode1v2` against the same dumps with the frozen
DSIM40 comparator, **diverges at the load capture** (DSIM tic 0, first at `rng.gameplay`, 5 actors at load); its three
extra spawns each draw a random number for their spawn state's duration (p_mobj.c:787-788). `--check` reproduces the map,
the script and the results file.

**Frozen-wire faults:** only the known scream; no further fault was found, in the synthetic items or the real runs. No
run reaches the scream's case. **No change to `doom-sim` source** was made by this step.

**Checks run by the agent:** clippy on `doom-sim` (release, all targets, warnings denied) and `cargo fmt --check` clean,
`wire25_projection` 2 passed, `cargo +1.85.0 check -p doom-sim --examples` builds with the same one pre-existing warning;
the CI steps run locally in the job's form (below). **Not run:** `m5c_wire25_real.py --check`, the full workspace test,
rustdoc, fuzz, sanitizers, Windows or macOS CI.

## Step 4 part 1: the sound tables, the mixer and the sound logic

The new platform-free crate `crates/doom-sound` joins the workspace between `doom-render` and `doom-game`
([architecture](../ARCHITECTURE.md#sound-effects-m5c)). Part 1 wrote the modules `sounds`, `tables`, `lumps`, `mixer`
and `logic`, with no new third-party dependency and no floating point at run time. The tool
`tools/reference/m5c_sounds_table.py` generates `sounds_table.rs` (the 109-entry `S_sfx` table of sounds.c:114-227 with
the enumerators of sounds.h:177-289) and `steptable.rs` (the 256 steps as an integer 64th root, its SHA-256 checked
against the genuine digest `95f2209c...6dde`); its `--check` and its `--self-test` (2 of 2) passed.

**Observed gates** (the Rust against the genuine goldens; source equivalence unless said):

- **The mixer** (`tests/mixer_goldens.rs`): every row of every golden except the two device rows on the **23 scenarios**
  (65,878 starts mixed, 490 buffers, 1,440 rows; 4 further listed starts of the refused `x_*` scenarios are refused where
  the C ends in `I_Error`, so the results file's 65,882 counts them) and on the **64 sessions** (741 starts, 14,043
  buffers, 3,012 rows, every handle equal to the stream's). The three short-lump witnesses are refused at the flagged
  start with the mixer unchanged, and the pitch-256 probe is refused. Two planted faults were caught and removed: the
  pistol dropped from the single-instance list, and the clamp `0x7fff` changed to `0x7ffe`.
- **`S_AdjustSoundParams`** (`tests/adjust_streams.rs`): 404 of 404 positional start rows of the 64 streams (37
  inaudible, 24 clipped, 32 rows on map 8). Three planted faults (no off-by-one angle, a floor on map 8, a shift of the
  minimum) each fail it.
- **The sound logic's rules** (`tests/logic_rules.rs`, 8 tests): the allocation, priority and draw cases as the genuine
  runs observed them, pitches from the drawn values, the same-position rule, every refusal leaving logic, mixer and
  generator unchanged, one lump lookup per sound, updates freeing channels, level start and removal.
- **Stream replay** (`tests/stream_replay.rs`): 37 of 64 streams replayed to their end (16,069 of 22,770 tics) and the
  27 others as far as the first update where a playing channel has a positional origin. Rows compared: S 369, K 309, T
  225, L 87, C 1,275, R 16,069. Planted faults (a draw for `itemup`, the eviction comparison, the finished comparison)
  each fail it. Part 2's gate replaced the partial replay with the whole.
- **Tables and lumps** (`tests/tables_and_lumps.rs`, 6 tests): the names against the simulation's list, the step table
  by integer inequality, both digests, edge lumps and load refusals (the digests are source equivalence of the tables;
  the refusals are Rust-only safety tests).

**Refusals implemented** (each leaves the state unchanged; the list with citations is in
[COMPATIBILITY](../COMPATIBILITY.md#refusals-instead-of-the-originals-errors-decision-t11)): sound number 0 and 109 and
above, a missing lump at a start that obtains a channel, a short lump, a pitch outside 0..255, a right or left volume
outside 0..127, a switch release in slot 15, a NULL origin with a placement (Rust-only), an origin that cannot be read
at an update (Rust-only), a configuration out of bounds (Rust-only), a missing `dspistol` at load and a sound lump over
the size caps of 1 MiB and 16 MiB (Rust-only bounds).

**Sizes:** 20 files, +3,523 and -1 (including 11 lines of `Cargo.lock`); generated 158 lines (`sounds_table.rs` 116,
`steptable.rs` 42); hand-written: `src` 1,347 (logic 659, mixer 350, lumps 184, sounds 78, tables 46, lib 30), tests
1,761, the tool 153 and manifests 23; plus 71 lines of the `doom-sim` types commit, cherry-picked from step 3a.

**Final checks:** `cargo test -p doom-sound` 21 passed in debug and in release; clippy with warnings denied, `cargo fmt
--check` and rustdoc with warnings denied clean; `cargo +1.85.0 check -p doom-sound --all-targets` clean; the sounds-table
`--check` matched both files. **Not run:** a workspace-wide build or test, fuzzing, sanitizers, mutation testing, the
genuine C runners, the independent Python model, CI. No frozen-wire sound fault was found (no simulation output was
compared in this part).

## Step 4 part 2: the session, the detector, the saved state and the end-to-end gate

What was built (the crate `doom-sound`, plus the fuzz target and seed tool of
[step 11](#step-11-part-1-the-fuzz-targets)): the fix of the fuzz finding in the mixer (below), the schedule and the
session (`Sound`, one per fresh process, transactional), the T12 layout detector, the saved state `DSND`
([layout](../STATE_FORMAT.md#sound-session-state-dsnd-version-1-m5c)) and its restore, and the end-to-end gate.

**The gate (source equivalence).** `cargo test -p doom-sound --release --test session_gate` (it also passes in debug, in
6.9 seconds): the 64 items, 22,770 tics, run from the Rust wire-25 simulation through the session. **Every genuine row is
equal**: S 785, T 378, K 723, C 4,311, L 176 and R 22,770, the row counts of the 64 files; and all 64 mixer session
goldens (3,012 rows: 741 starts, 14,043 buffers; the two device rows excluded as in part 1). The `R` rows (the
`M_Random` index) are the session's returned index after `P_Ticker`, plus one for `ST_Ticker` (st_stuff.c:992, after
`P_Ticker`, g_game.c:731-732), plus 320 after the frame of tic 0 (f_wipe.c:159-162; decision T6). Item endings: M5OPEN
(40 of 82 demo tics), M5SSEG (15) and M5VPLAN (49) end where the genuine process ended at the renderer's `I_Error`; BOSS1
(545 of 600) ends at the level exit, where the gate checks that the simulation refuses tic 545 with `LevelExited`.
Three planted faults were each caught and removed: records run after the update (M5PEG tic 128, row 2); an actor
position one unit east at the update (M5VIEWZ tic 85, a volume of 7 against 6); and the first buffer one tic late (M5PEG
mixer golden row 5).

**One exception, disclosed.** For the **381** start rows without a position (a NULL origin and the listener's own body),
the genuine runner writes the listener's x, y and angle, and the wire-25 record does not carry them. Those three columns
are not compared on those rows (the test counts them); nothing in the sound logic reads them there (s_sound.c:304-324).
Every other column of those rows and every column of the 404 placed rows are compared. The gate harness includes
`doom-sim`'s `tests/wire25/mod.rs` by path for the item table, so a change of that table moves the gate.

**The detector (Rust-only tests built from cited C facts).** Four tests in `tests/detector_rules.rs`: it fires on an
audibility flip at the clipping edge (a declared volume of 1 exactly 130 units inside `S_CLIPPING_DIST`, where texture 0
gives volume 0), at a start and at an update; it fires on a separation-only change (an angle where (34,0) and (19,34)
differ only in separation; the same listener does not fire at an update); it does **not** fire on SNDBTN2's busy-slot
release (the genuine row S,128,6, read (34,0), listener 32643278,7238990, angle 1073741824; all 80 textures of the
room's WAD tried); and through a session the label is the session tic, the same tic's update read also fires, and the
session continues. **Over the 64 items the detector fired 0 times, so no item is a layout-dependent session. The 61 real
runs have not been run through the Rust sound logic, so nothing is known for them.**

**Restore and continue.** `tests/state_restore.rs`: on all 64 items the state was saved at a quarter, a half and three
quarters of the run and at the first two tics with a busy channel: **271 saves** (130 with a busy channel). Each decodes,
re-encodes to the same bytes, restores against a clone of the simulation and continues; every stream row, every buffer
digest and the final bytes equal the unbroken run's, and the continued rows equal the genuine streams (69,226 tics
continued). It fails when the looked-up flags or the handle counter are not restored (each planted and removed). Forgeries:
**37** decode or restore forgeries are each refused with their own error (every truncated prefix also refused), plus **3**
refusals that need the simulation (a tic mismatch, an actor that does not resolve, a sector past the map).

**Unit and rule tests:** `tests/session_rules.rs` 5 and `tests/detector_rules.rs` 4 as above (Rust-only), the schedule's
unit test (512 tics, 315 buffers) and the mixer's regression test of the fuzz finding.

**The fuzz finding fixed.** The first fuzz campaign (step 11) found that `mixer::volumes` (i_sound.c:352-361) overflowed
in 64-bit arithmetic for a volume and separation far outside the game's range (0x7fff << 15). It now computes in 128
bits, where no pair of 32-bit arguments overflows, and the range checks of i_sound.c:364-368 then refuse. A unit test
holds the reproducing values and 100 edge pairs; with the old 64-bit form the debug test panics (checked, reverted).

**Final checks:** `cargo test -p doom-sound` 36 passed in debug and in release; clippy and rustdoc with warnings denied,
`cargo fmt --all -- --check` and the same check on the fuzz workspace clean; `cargo +1.85.0 check -p doom-sound
--all-targets` builds with one warning in `doom-sim` (the pre-existing `MAX_LINKED_THINGS`); `cargo +nightly check` of the
three fuzz targets, the seed tool's `--check` and `--self-test` pass. **Not run:** a workspace-wide build or test,
`doom-sim`, `doom-render` or `doom-game` tests (not touched), the 61 real runs, the genuine C runners, a leak-detecting or
longer fuzz campaign, mutation testing, CI.

**Sizes:** 19 files, +3,274 and -32, all hand-written: `doom-sound` `src` +1,375 (session 312, state 469, detector 172,
schedule 144, mixer +229 including a 60-line test, logic +46, lib +10), its tests +1,556 (gate harness 627, state_restore
407, session_rules 221, detector_rules 164, session_gate 112, common +25), fuzz and tools +341 and -25, manifests 2.

**Deviations from the note's sketch, and what the part needs from the simulation.** `DSND` adds the looked-up bits, the
detector fields and the texture count to the note's section 4.8.1 sketch and drops the `channelids`; the note's text of
`S_UpdateSounds` "per iteration" in section 4.3.3 is superseded by the owner's "after every tic", which the goldens
agree with. The part asks of the simulation (a request, not yet done): the listener's x, y and angle at the call for
every start without a position, so that the 381 rows' columns can be compared.

## Step 8 part 1: the CI job and the mutation workflow

One job was added to `.github/workflows/rust.yml` and its steps were checked locally in the job's form (the agent ran
each step's command, with a local target directory, on 2026-11-18; all exited 0): job id `m5c-genuine-sound`, name
`M5c genuine sound (not required)`, Linux, 45 minutes, the documentation-only fast path, placed before
`simulation-ir`, nothing else in the workflow changed. Its first version had six steps, and these timings:

| Step | Local time |
| --- | --- |
| generator and contract checks (`m5c_sfx`, `m5c_check_sfx` with its self-test, `m5c_wad`, `m5c_build`, `m5c_mixer_build`, `m5c_sounds_table` with its self-test) | 1 s |
| `m5c_starts.py --selftest`, then each of the 64 streams against its committed start list | 4 s |
| `m5c_reference.py --check --selftest --jobs 4` (the address variant with leak detection disabled) | 628 s (a cold run, while another build ran) |
| `m5c_mixer_reference.py --check --selftest --jobs 4` | 19 s |
| `cargo +stable build -p doom-sim --release --locked --example player_level_run` | 27 s |
| `m5c_rust_check.py --binary .../player_level_run` | 6 s |

Step 3b then added three steps after the room check: the release build of `sound_records_run`, `m5c_wire25_check.py
--check --selftest --jobs 4` and `m5c_t4.py --check`. Checked locally in the job's form (a release build with
`cargo +stable build --release --locked`, work under a temporary directory): 64 of 64 items match, the results file
matches, the self-test passed (73 checks), and the T4 check passed; the workflow parses as YAML. The note's status line
records that the job was green at the head it names, together with the other eight jobs. The real-map tools, the
`--strict32` option and every audio command are not in the job.

**The mutation workflow.** The one `cargo mutants` command of `.github/workflows/mutants.yml` gained `-p doom-sound`
beside `doom-sim`, `doom-render` and `doom-game` (`doom-play` stays out); the trigger, the 16 shards, the caps and the
job names are unchanged. Checked: the file parses as YAML, and `cargo mutants --list` over the branch's diff against
master listed 264 mutants in `doom-sim` and 556 in `doom-sound` (about 51 per shard) at the head the commit names.
**No mutation run of `doom-sound` has been made; the listing counts mutants and says nothing about how many a test
kills.**

## Step 11 part 1: the fuzz targets

Four targets in the fuzz workspace, none needing an IWAD or any id data, with one deterministic seed tool
(`tools/seed_sound_fuzz.py`, which has `--check` and a `--self-test`; no corpus is committed, as for the render
targets). All were run on the development machine with one target built at a time into a directory outside the
tree, `CARGO_PROFILE_*_DEBUG=0`, and the figures are the step agents'. **Leak detection was disabled in every
sanitizer run (`ASAN_OPTIONS=detect_leaks=0`), so none of them is LeakSanitizer coverage.**

| Target | Input | Invariants (Rust-only safety tests) |
| --- | --- | --- |
| `sound_lumps` | a WAD image (max_len 4096) | `SoundSet::load` equals an independent model of `I_InitSound`'s loop and `getsfx` (i_sound.c:185-251, 796-813): the same `Ok` or typed `Err` in sound order, the padded length, the data, the linked entry's length 0, the lump number, and both size caps |
| `sound_mixer` | start parameters (max_len 2050) | the refusal order (pitch, short lump, right volume, left volume) with volumes recomputed in 128 bits; a refused start leaves the mixer equal to a clone; the handle counter wraps as i_sound.c:335-340; two runs give one digest |
| `sound_logic` | records, positions and updates (max_len 1060) | an `Err` leaves logic, mixer and generator equal to clones; one channel per origin; at most one draw, by the exact rule; a started sound occupies the channel it reports, with the mixer's handle; updates free only occupied channels |
| `sound_state` | mode 0: `DSND` bytes; mode 1: a session on arbitrary records (max_len 2048) | the decoder never panics, an accepted encoding is canonical and round-trips, and `from_state` accepts or refuses with a typed error and gives the state back; a refused tic leaves the session equal to its clone, an accepted tic advances the schedule by exactly the tic and the buffers the rule gives and returns the index advanced by the reported draws, the detector's count never falls, and every save round-trips within 1,505 bytes |

**Observed** (target compiled; seed replay; coverage-guided campaign; sanitizer campaign are four different things;
`cov` and `ft` are libFuzzer's counts of coverage points and features):

| Target | Seed replay (address build) | Coverage-guided, 300 s, no sanitizer | Address campaign, 300 s, leaks disabled |
| --- | --- | --- | --- |
| `sound_lumps` | 22 seeds (23 runs), no failure | 13,084 runs, cov 270, ft 1,090, corpus 151 | 6,924 runs, cov 561, ft 1,943, corpus 156 |
| `sound_mixer` | 180 seeds (182 runs), no failure after the skip below | 122,697 runs, cov 217, ft 681, corpus 170 | 31,714 runs, cov 561, ft 1,585, corpus 167 |
| `sound_logic` | 144 seeds (146 runs), no failure | 1,058,469 runs, cov 453, ft 2,464, corpus 521 | 191,051 runs, cov 1,052, ft 5,269, corpus 505 |

All exited 0 without a crash. The `sound_state` target (the step 4 part 2 commit message records these figures; the step
agent's report does not): 152 seeds (153 runs) replayed with no sanitizer and 154 runs with AddressSanitizer, no
failure; a 180-second coverage-guided campaign of 364,535 runs (cov 691, ft 3,627, corpus 545); a 180-second
AddressSanitizer campaign with leak detection disabled of 66,579 runs (cov 1,600, ft 8,609, corpus 479); and, after the
overflow skip below was removed, `sound_mixer` again under AddressSanitizer: 180 seeds replayed (182 runs) and a 180-second
campaign of 12,959 runs (cov 662, ft 2,100, corpus 169), leak detection disabled; no crash and no finding. The state
seeds are written by the tool's own independent `DSND` writer; its five valid states decode and restore in Rust (a
temporary test, not committed).

**One real finding** (step11a-1): `Mixer::start` panicked with "attempt to multiply with overflow" in `volumes`
(`v * s * s` in 64-bit) for a volume and separation far outside the game's range (the 10-byte input `00 00 10 01 ff 7f
ff 7f 80 ff`, volume = separation = `0x7fff << 15`). It cannot happen in the game (volume 0..127, separation 0..255);
in a release build without overflow checks it would have wrapped and could have mis-decided the refusal. The target
skipped those inputs until the fix ([step 4 part 2](#step-4-part-2-the-session-the-detector-the-saved-state-and-the-end-to-end-gate)),
which removed the skip. **Not run:** a leak-detecting campaign, longer campaigns, other sanitizers, a 1.85.0 build of the
fuzz crate, the whole fuzz workspace build. Sizes: three targets and the tool 949 insertions (`sound_logic` 270,
`sound_lumps` 210, `sound_mixer` 180, the seed tool 257, `fuzz/Cargo.toml` 22, `fuzz/Cargo.lock` 10), all hand-written; the
fourth target about 190 lines and the tool's `state` kind about 131 lines more.

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
  of step 2c is right; nothing was amended.
- **Two more commit messages carry a wrong count; the true figures are these.** The message of the schedule and session
  commit of step 4 part 2 says `cargo test -p doom-sound --release: 33 passed`; the count at that commit was **28**. The
  message of the saved-state commit says "40 forgeries are each refused with their error"; the true split is **37**
  decode or restore forgeries (each prefix also refused) plus **3** refusals that need the simulation, 40 in all.
  Nothing was amended (the later messages state 36 passed, which is right).
- **Frozen-wire sound faults: one, and the count stands there.** (T3 ruling: left unpatched in wires 22-24, fixed in
  wire 25 only, listed.) The hard-death scream. What: wires 22-24 choose `sfx_pdiehi` in every mode. C:
  `p_enemy.c:1994-2007` (`sfx_pdiehi` only when `gamemode == commercial` and health is below -50). Rust: the
  `PlayerScream` arm of `crates/doom-sim/src/lifecycle.rs` (lines 1701-1708 when it was found; wire 25's branch tests
  `!wire25`). How found: by reading, not by a run (note section 2.2, item 6). Fixed in wire 25 by step 3a. The first
  genuine comparison of the sound events, wire 25 over DSIM41 on the 64 synthetic items and the 61 real runs, found no
  further fault, and no item or run reaches the scream's case. The owner's limit before stopping (about five, or any fault
  that is not about sound) was not approached.
- **Layout-dependent sessions: none seen, and only the synthetic items were checked.** The T12 detector fired 0 times
  over the 64 synthetic items. The 61 real runs have not been run through the Rust sound logic and the detector, so
  the count for them is not known (the owner's expectation is 0). Such sessions are counted separately in every report.
- **381 of 785 start rows lack the listener columns in Rust.** For starts with a NULL origin or the player's own body
  the genuine row writes the listener's position and angle, which the wire-25 record does not carry; those three columns
  are compared nowhere (neither in step 3b's comparison nor in the end-to-end gate), and the original does not read them
  there. On the real runs the same holds for 5,980 starts (26,077 less 20,097; by subtraction from the results file).
- **A report and a committed file differ in wording only.** The synthetic results file counts 22,976 as `tics`; each
  item's value equals its snapshots, so it counts captures including the load, which is what step 3a's report and the
  real-run commit message call them (the real file's 159,027 is 158,966 tics plus the 61 loads). No figure disagrees.
- **Stale or loose statements of the simulation, not changed here.** The doc comment of `ActorStateEvent::Sound`
  still says a missing origin means boss-global (note section 2.10, item 5); step 3a's module comment says the listener
  body is told apart by its handle, whereas `doom-sound` decides positional starts by the presence of a placement
  (s_sound.c:304) and uses the listener's handle only at updates.
- **Where the note's text needed adjusting.** The library comparison counted 3 new crates; there are 2 new ones (`cc` was
  in the lock already at 1.10.0) and maudio cannot drop its `engine` feature; the note's backend sketch used one context
  with several backends, and the code uses one context per backend so that the opened backend is known. The note's
  Test 4 ("chgun 1 draw") was run with the lump functions stubbed so that every lump is found; on `m5c_sfx.wad`, which has
  no `dschgun`, a chaingun start that obtains a channel is a `LumpMissing` refusal, and on `m5c_sfx_edge.wad` it draws 1
  as the note says (both tested).
- **A documentation slip found on the way.** The M5b section of TESTING listed a `--strict32` option for
  `m5b_reference.py` that the tool does not have; it was corrected on 2026-11-18.

## What a human still has to judge

Nobody has heard any sound from this project: every audio trial in Phase 1 wrote digital silence, the probe wrote
silence at most and failed to find a backend on the development machine, and no device has been opened on any system.
The evidence above compares which sounds start, on which channel, with which volume, separation and
pitch, and the 16-bit samples the original mixer writes for them; it cannot say how any of it sounds, whether the
synthetic or the real samples are pleasant, or whether a backend will play them without gaps. The owner has also
not playtested M5b, so input, pacing and presentation are not to change on a guess about feel.

## Still to come in this record

The display contract with sound (the status bar's `M_Random` index taking the sound draws and the wipe's 320, and the
display state version 2), the game loop, the device in `doom-play` and the launch flags (volume, mute, no sound), the
real-map runs of the full sound logic and of the frames under the new contract (which decide whether the face's glance
limit S15 can be called closed, and how often the T12 detector fires on the 61 runs), the optional melt, mutation testing
of `doom-sound`, timings, the visuals, the reviews and the final gates. The second documentation pass records them.
