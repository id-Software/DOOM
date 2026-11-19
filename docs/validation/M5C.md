# M5c evidence record

Observed results of the M5c milestone (sound effects matched to linuxdoom-1.10, and the closing of the status
bar face's idle glance gap). Only observations appear here. The
[M5c investigation](../M5C_REFERENCE_INVESTIGATION.md) owns the source facts, the owner decisions T1-T16 (all
confirmed on 2026-11-18) and the plan; [testing](../TESTING.md#m5c-sound-effects-the-genuine-sound-logic-and-mixer)
owns the commands; [COMPATIBILITY](../COMPATIBILITY.md#sound-effects-m5c) owns the support statement and
[ROADMAP](../ROADMAP.md) the milestone status. Exact review state and CI belong to GitHub.

This record was last extended on 2026-11-18 (America/Los_Angeles, sixth documentation pass) with the pieces of the milestone that are complete
on the branch: Phase 2 steps 1 and 2, the proof of the audio library (decision T7), step 3 (wire 25 and its genuine
comparisons), step 4 (the crate `doom-sound` with its session, detector and saved state), step 5 (the display contract
with sound, the sound session in the game loop, the 61 real runs and the scripted app sessions), step 6 (restore and
continue), step 7 (sound in `doom-play`), step 8 part 1 (the CI job and the mutation workflow), step 10 (the first mutation run's totals, the change of sharding, the 74 survivors with a disposition each and the independent
check of those dispositions, the final run at `a8abf0c`, the second round for its 14 new survivors and the confirming run at
`937cf9d`), the local verifier of `doom-play` sessions (step 5 part 3), the failure of the instruction-count
check and its code-only fix, step 11 parts 1 and 2 (the fuzz targets, including the one for `DDSP` version 2), step 12
(timings), step 13 (the independent review and the fix made for each finding), and the CI record up to `937cf9d`.
the owner's second set of rulings of 2026-11-18, the CI record of `a8abf0c`, the instruction-count fix's one changed test line,
and the README sound visual with the layout label of the session check.
**Not done and not recorded here:** the final gates from a clean clone (running at `937cf9d` when the sixth pass was written),
the pull request's new body, the Windows artifact, and CI on the final head. With steps 5 to 7 on the branch **the
face's idle glance gap (S15) is closed** under the owner's wording
([COMPATIBILITY](../COMPATIBILITY.md#the-status-bar-faces-idle-glance-is-closed-with-sound-decision-s15-t14)).

**What has and has not been heard or done.** **Nobody has heard any sound from this project, and no game audio has
reached any audio device on any system.** The only real device open was one second of digital silence written by
`doom-play --audio-probe` to PulseAudio under WSLg, using a scratch copy of the client library
([step 7](#step-7-sound-in-doom-play)); on the development machine without that library the probe finds no backend and
fails loudly. Nothing has run on Windows or macOS beyond CI builds and tests. Every statement below is about which sounds
start, on which channel, with which volume, separation and pitch, and the 16-bit samples the original mixer writes for
them. None says how anything sounds.

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
| Step 5 part 1: the display contract with sound, the sound session in the game loop, the frame gate, step 6 (restore and continue), the headless app tests | Recorded ([below](#step-5-part-1-the-frame-gate-restore-and-continue-and-the-headless-app-tests)) |
| Step 5 part 2: the 61 real runs through the whole pipeline (local only) | Recorded ([below](#step-5-part-2-the-61-real-runs-through-the-whole-pipeline-local-only)) |
| Step 5 part 3: scripted app sessions against the genuine M5c runner, and the local verifier for `doom-play` sessions | Recorded ([below](#step-5-part-3-scripted-app-sessions)) |
| Step 7: sound in `doom-play` (device sink, flags, mute) and the start-up stall | Recorded ([below](#step-7-sound-in-doom-play)) |
| Step 10: mutation testing | Recorded ([below](#step-10-mutation-testing)): the final run at `a8abf0c` (999 mutants: 865 caught, 28 missed, 3 timeouts, 103 unviable), the second survivor round (all 14 new survivors killed) and the confirming run at `937cf9d` (879 caught, 14 missed, 3 timeouts, 103 unviable; the 14 are the equivalent and unreachable mutants of the first round) |
| The instruction-count check (`Simulation instruction counts`) at `3c1cc5d`: failure, cause and code-only fix, and the owner's ruling on the one changed test line | Recorded ([below](#the-instruction-count-check)); green at `a8abf0c` |
| The README sound visual and the layout label of the session check | Recorded ([below](#the-sound-visual-and-the-session-checks-label)) |
| Step 11 part 2: the fuzz target `render_display_state_v2` and the targets changed by the review | Recorded ([below](#step-11-part-2-the-fuzz-targets-after-the-review)) |
| Step 12: timings of the sound session, the mixer and the frame step | Recorded ([below](#step-12-timings)) |
| Step 13: independent review by three reviewers and the fix made for each finding | Recorded ([below](#step-13-independent-review)) |
| The CI record | Recorded ([below](#the-ci-record)) through `937cf9d`: all nine jobs green at `a8abf0c`, `0dea135` and `937cf9d` |
| The final gates from a clean clone, the pull request's new body, the Windows artifact, CI on the final head | Not done; not recorded here |

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
command: every figure is the step agent's. The second documentation pass (steps 5 to 7 and the CI record) also re-ran no command and took each figure from the step agents' reports or from the committed results files `m5c_real_full_results.json` and `m5c_session_results.json`. **The third pass (steps 10, 11 part 2, 12 and 13) re-ran no command either:** each figure is from a reviewer's or fixing agent's report, a commit message or a committed file. Step 12's section was written by its own agent and only read, not rewritten, in this pass. **The sixth pass (the final mutation figures, the second survivor round and the CI record to `937cf9d`) re-ran no command either and built nothing:** each figure is from the lists and reports it names in [step 10](#step-10-mutation-testing), counted from the files, or from the orchestrator's observation of CI. **The fourth pass (the survivors of step 10, the instruction-count check, the session verifier and the CI record) re-ran no command either:** each figure is from `step10`, `review-mut`, `sessreal` and `fix-ir`'s reports, a commit message or a committed file; the commit hashes and the layout of the committed files were read from the repository.

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

**Self-test, 73 checks, each passing (75 since the review added the two frame-layout plants: [step 13](#step-13-independent-review)):** DSIM41 equals DSIM40 outside `profile`, `context.schema` and `events.sounds` on
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
run reaches the scream's case. (The independent review later found that the DSIM41 *reader* refused every capture of this
truncation map, which no test here decoded; the 93 captures here were compared by the Python decoder, which does not
validate. Fixed, with a test that decodes all 93 with the Rust reader: [step 13](#step-13-independent-review).) **No change to `doom-sim` source** was made by this step.

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

**Exceptions, disclosed (the first here; the complete list is in [TESTING](../TESTING.md#what-the-sound-gates-do-not-compare) and was widened by the review: the gate does not compare the `#`, `W` and `Z` rows either, and "every genuine row" above means every `S`, `T`, `K`, `C`, `L` and `R` row).** For the **381** start rows without a position (a NULL origin and the listener's own body),
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
session continues. **Over the 64 items the detector fired 0 times, so no item is a layout-dependent session.** (The 61 real runs
were run through it later: [step 5 part 2](#step-5-part-2-the-61-real-runs-through-the-whole-pipeline-local-only), 0 firings.)

**Restore and continue.** `tests/state_restore.rs`: on all 64 items the state was saved at a quarter, a half and three
quarters of the run and at the first two tics with a busy channel: **271 saves** (130 with a busy channel). Each decodes,
re-encodes to the same bytes, restores against a clone of the simulation and continues; every stream row, every buffer
digest and the final bytes equal the unbroken run's, and the continued rows equal the genuine streams (69,226 tics
continued). It fails when the looked-up flags or the handle counter are not restored (each planted and removed). Forgeries:
**37** decode or restore forgeries are each refused with their own error (every truncated prefix also refused), plus **3**
refusals that need the simulation (a tic mismatch, an actor that does not resolve, a sector past the map).

**Unit and rule tests:** `tests/session_rules.rs` 5 (7 since the review) and `tests/detector_rules.rs` 4 as above (Rust-only), the schedule's
unit test (512 tics, 315 buffers) and the mixer's regression test of the fuzz finding.

**The fuzz finding fixed.** The first fuzz campaign (step 11) found that `mixer::volumes` (i_sound.c:352-361) overflowed
in 64-bit arithmetic for a volume and separation far outside the game's range (0x7fff << 15). It now computes in 128
bits, where no pair of 32-bit arguments overflows, and the range checks of i_sound.c:364-368 then refuse. A unit test
holds the reproducing values and 100 edge pairs; with the old 64-bit form the debug test panics (checked, reverted).

**Final checks (at that commit; 43 tests since the review, [step 13](#step-13-independent-review)):** `cargo test -p doom-sound` 36 passed in debug and in release; clippy and rustdoc with warnings denied,
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
At that commit no mutation run of `doom-sound` had been made; the listing counts mutants and says nothing about how many a test
kills. The first run, and the change to 24 shards dealt round-robin, are in [step 10](#step-10-mutation-testing).

## Step 10: mutation testing

**Done. The workflow, its first run, the tests for the 74 survivors of that run, the independent check of their dispositions, the
final run at `a8abf0c`, the tests for its 14 new survivors and the confirming run at `937cf9d` are recorded below.** The totals of
the first run come from the commit messages of the workflow's two changes (`git log -2 -- .github/workflows/mutants.yml`) and its
artifacts were not opened for this record. The totals of the final and the confirming runs, and the lists of missed mutants and
timeouts, are from the lists the orchestrator saved from the runs' shard artifacts (`missed-all.txt` and `timeout-all.txt` of each
run, `known-dispositioned.txt` and `new-survivors.txt` of the final run), which this pass opened and counted. The survivors' figures
are from the reports of the `tester` agents (`step10`, `step10b`, `followup`) and of the independent reviewer (`review-mut`).

What the [Mutants workflow](../TESTING.md#the-ci-job-and-the-mutation-workflow) is: `cargo mutants` over the branch's diff
against `master`, for `doom-sim`, `doom-render`, `doom-sound` (added in step 8, [part 1](#step-8-part-1-the-ci-job-and-the-mutation-workflow))
and `doom-game`, informational, started by the label `run-mutants`; `doom-play` is out of scope.

### The first run and the change of sharding

**The first run** (label `run-mutants`, head `216f976`): cargo-mutants' default, consecutive slices of the mutant list over 16
shards. The slow `doom-sim` mutants come first in the list, so shards 1 to 5 ran 48, over 120, 97, over 120 and 105 minutes
(shards 2 and 4 were cut off by the 120-minute job limit, with partial results uploaded) while the other eleven finished in 4
to 23 minutes. **Totals: 899 outcomes, 723 caught, 74 missed, 3 timeouts, 99 unviable; the two cut-off shards are incomplete,
so the list of outcomes is not the whole list of mutants.** (723 + 74 + 3 + 99 = 899.)

**The change** (commit `a80c373`): `--sharding round-robin` (mutant i runs on shard i mod k) and 24 shards instead of 16, the
count being set in the one place the workflow sets it, with a comment that says why. Nothing else changed: trigger, packages,
caps, timeouts and job names. Checked: the file parses as YAML; cargo-mutants 27.1.0 lists `--sharding round-robin` in its help.
**The new sharding has not been run**: no run of the workflow after the change is recorded here.

### The 74 survivors: tests and dispositions

Three commits on the branch hold the tests (`de34a59` doom-sound, `ce94184` doom-sim, `e2207ff` doom-game); only test code
was added and no source file changed (`git diff --stat 3c1cc5d HEAD` of the step: 4 files, 1,069 insertions, all
hand-written: `crates/doom-sound/tests/survivors.rs` 595, `crates/doom-sim/src/wire25_tests.rs` 191,
`crates/doom-sim/tests/wire25_genuine_placements.rs` 206, `crates/doom-game/tests/survivors.rs` 77). There are 24 new tests:
14 in `doom-sound`, 5 unit and 2 integration tests in `doom-sim`, 3 in `doom-game`. No golden, results file or existing
expectation changed. The agent worked with the mutants' lines at `216f976`; each mutant was found again by function and
mutation in the current source.

| Disposition | `doom-sound` | `doom-sim` | `doom-game` | Total |
| --- | --- | --- | --- | --- |
| Killed by a new test | 45 | 12 | 3 | 60 |
| Equivalent (argued) | 9 | 3 | 0 | 12 |
| Unreachable (argued) | 2 | 0 | 0 | 2 |
| Total | 56 | 15 | 3 | 74 |

By file in `doom-sound`: `logic` 19 (10 killed, 7 equivalent, 2 unreachable), `state` 15 (all killed), `mixer` 10 (8 killed,
2 equivalent), `detector` 5, `lumps` 3, `session` 2 and `schedule` 2 (all killed). In `doom-sim`: `sound.rs` 10 (8 killed,
2 equivalent), `diagnostic.rs` 3 (2 killed, 1 equivalent), `teleport.rs` 1 and `demon_expanded_map.rs` 1 (both killed). The
three timeouts are separate from the 74 ([below](#the-three-timeouts)). **Source equivalence** for the tests that compare
with the genuine engine's rows (`wire25_genuine_placements.rs`: every placed start of the 64 items, 404 starts over 22,770
tics, and every sector origin, 414 sectors, against `P_GroupLines` recomputed from the map as the C's loop nest does); the other
tests are **Rust-only** pins of rules, bounds and messages whose expectations rest on cited C lines or documented rules (the
test files say which). Three tests take their expectation from Rust itself, a point the independent check raised
([below](#the-independent-check)).

**How each kill was confirmed.** One mutant at a time, applied by hand: a script replaced one exact text in the worktree file,
ran only the new tests, recorded KILLED (a named test fails), SURVIVED (all pass) or UNVIABLE (does not compile), and restored the
file with `git checkout`. **No cargo-mutants run was made locally** (it would cost disk and hours): the hand check applies the
same textual mutation cargo-mutants applies, but not cargo-mutants' own set, so a mutant of the same line that the list does not
name is not covered. Two things follow for the next run. cargo-mutants was run per package, so a `doom-sim` mutant is judged by
`doom-sim`'s own tests only (the `doom-sound` gate, which runs the same simulation end to end, does not count against it); the
new `doom-sim` tests are therefore in `doom-sim`. And the equivalent and unreachable mutants will survive the next run by design:
they are to be listed there as accepted, not chased.

**The 12 equivalent and 2 unreachable mutants** (lines at `216f976`; each argument was agreed by the independent check):

| Mutant | Disposition | Why no test can tell it apart |
| --- | --- | --- |
| `logic.rs:173` `<` to `<=` (`approx_dist < S_CLOSE_DIST`) | Equivalent | At exactly `S_CLOSE_DIST` the other branches give `snd * 1040 / 1040` or `15 + ((snd - 15) * 1040) / 1040`, both exact, so the volume is `snd` as well (`s_sound.c:805-814`; `S_ATTENUATOR` is 1040, `s_sound.c:64`); the separation is computed earlier |
| `logic.rs:176` `>` to `>=` (map 8 clamp) | Equivalent | At equality the body assigns the value `approx_dist` already holds (`s_sound.c:802-803`) |
| `logic.rs:514` `+=` to `-=` (`volume += link_volume`) | Equivalent | The only linked sound is `sfx_chgun`, whose link volume is 0 (`sounds.c:204`), so `snd + 0` equals `snd - 0` |
| `logic.rs:519` `>` to `<`, `==`, `>=` (3 mutants) | Equivalent | After adding 0, `volume == snd`, so the clamp's body assigns what is already there under `>`, `>=` and `==`, and `<` is false |
| `logic.rs:669` `+` to `-` (in `update`) | Equivalent | The link volume is 0 |
| `mixer.rs:276` `\|` to `^` (the mask fold) | Equivalent | The fold visits each index once, in increasing order, so bit i is still 0 when `1 << i` is combined; for disjoint bits `\|` and `^` agree |
| `mixer.rs:437` `<<` to `>>` (`& !(1u8 << slot)` in `Mixer::start`'s `freed`) | Equivalent | `after` is read once the channel is busy, so bit `slot` of `before & !after` is already 0 and the extra mask clears nothing, for `<<` and `>>` alike (the text is a copy of the genuine mixer driver's own) |
| `sound.rs:309` and `sound.rs:314` `>` to `>=` (`add_to_box` else-if, 2 mutants) | Equivalent | At equality the body assigns the edge its current value (`m_bbox.c:53-58`) |
| `diagnostic.rs:669` `&&` to `\|\|` (`mo.len() == 13 && mo[0] == 1`) | Equivalent | DSIM41 frames are read with `version` 40, so the field loop before `validate_sound_records` already requires 13 bytes and, for tag 0, an all-zero body; the mutant then reads twelve zero bytes as the listener, a handle that `validate_handle` refuses in any record (generation 0), so `positional` is the same |
| `logic.rs:614` `get_channel` guard to `false`, and `!=` to `==` (2 mutants) | Unreachable | `get_channel` is private and called only by `start`, right after `stop_first` freed the first channel with that origin (`s_sound.c:349`, then 352); the table never holds two channels of one origin (`new` is empty, `from_parts` and `DSND` decode refuse duplicates, `start` frees before it installs, the other paths only free), so the same-origin branch (unreachable from `s_sound.c:352` in the C as well) never runs |

Three pieces of code are redundant, not defective, and stay: the second mask in `Mixer::start`'s `freed`, the length test
`mo.len() == 13` (defence in depth if the field loop's "optional" set ever changes) and the dead link-volume arithmetic for
the one linked sound (the C's own text, `s_sound.c:287-293`, from a generated and checked table). The reviewer agreed with
keeping all three; the cost is the permanent equivalent mutants above.

### The three timeouts

All three change a loop's step so that its counter no longer advances, an endless loop, and a timeout is the expected way to
catch it: `doom-sound` `logic.rs:619` (`cnum += 1` to `*= 1` in `get_channel`, stuck once channel 0 is held by another origin),
`doom-sound` `mixer.rs:412` (`i += 1` to `*= 1` in `Mixer::start`, stuck once channel 0 is busy; `i_sound.c:309-325` is the
same loop) and `doom-sim` `sound.rs:156` (`i += 1` to `*= 1` in `split`). The tester observed two of them by hand (`timeout 90
cargo test` ended with exit status 124, which includes the rebuild and so shows only that nothing finished). The independent
check ran each again from a prebuilt binary under `timeout` with a control test from the same binary: exit 124 at 30 s for the two
`doom-sound` mutants (their controls exit 0) and at 60 s for both tests of `wire25_genuine_placements` for the `doom-sim` one
(without the mutation both pass in 2.6 s). One correction from that check: the tester said a list that starts with a positional
start ends through the second `i += 1`; it cannot (`events.get(0)` is the start itself, never a record), so such a list ends only
through the `Unplaced` refusal in strict mode, and in non-strict mode, or for lists that begin with a removal mark or a start without
a position, the loop pushes records forever (unbounded allocation rather than a spin). The disposition is unaffected.

### The independent check

The `reviewer` agent `review-mut` (head `e2207ff`; it read neither the other agents' reports nor this record, only the tester's
report and the code and C; it changed nothing in the repository and worked in a scratch copy, deleted afterwards; `/mnt/c` had
26 GB free before, during and after) checked the dispositions on 2026-11-18.

- **All 14 equivalent and unreachable arguments were agreed** after attempts to break them, with no distinguishing input and no
  path found. It ran scratch tests as well: 14,336 calls of the distance rule exactly at `S_CLOSE_DIST` (maps 1, 2, 8 and 9,
  volumes 0 to 127, 7 angles, 4 source and listener pairs) all return the volume `snd`, and map 8 at exactly `S_CLIPPING_DIST`
  returns 15; over 4,013 random starts the xor fold of the busy channels equals `mask()`, and bit `slot` of `before & !after`
  was always 0 (9 starts with a non-zero `freed`, 194 into a busy slot); and `m5c_sounds_table.py --check` matched ("109 entries,
  1 link").
- **All 24 new tests were read, and 15 kills were re-run** (the source patched in the scratch copy, the named test run, the file
  restored and compared): all 15 killed. The others it judged by reading only, where it lists them (for example the seven further `MAX_BYTES`
  mutants would give 76,834, 372, 292, 2,719, 1,727, 1,057 and 1,281 bytes against the 1,505 the test asserts). No test that cannot
  fail was found. The C citations of the tests it read were checked against the source.
- **No blocker and no major finding; two minor and four notes**, all about what a comment or the evidence claims, none about a
  test's ability to fail. MINOR-1: a test's comment said it finds its tic by the genuine row, while tic 128 and the position
  (34, 0) are transcribed from that row and its agreement half compares Rust with Rust. MINOR-2: the header of the `doom-sound`
  test file said no expectation comes from Rust output, while three tests take theirs from the loaded set's own sizes, from the
  simulation's own reading, or from the `DSND` layout. N1: a `doom-sim` comment claimed more than its loop checks and cited
  `p_mobj.c:675` for the player's position, which is set at `p_mobj.c:662-663`. N2: a `doom-game` test cited `i_sound.c:665-670`
  for the left-first order, which is the interleave at `i_sound.c:565-567`. N3 (evidence): with the `sound.rs:369` mutation on, the
  placed-start comparison still passed; only the sector-origin test failed, so no genuine placement among the 64 items depends on
  a sector's back-side lines and only the independent recomputation guards that rule (the file's header already says so). N4: the
  timeout point above.
- **Corrected in commit `a777a55`** (comments only, no test's code changed; checked with `cargo fmt`): the four comments
  of MINOR-1, MINOR-2, N1 and N2.
- **Not run by the reviewer:** `doom-game` (not built), a cargo-mutants run, the `doom-sound` session gate and other full suites,
  release builds, Clippy, rustfmt, the fuzz workspace, rustdoc, Rust 1.85.0, Windows or macOS, any IWAD.

**Checks of the tests** (the agent, in its worktree at `b2d7806`, whose `crates/` equal `e2207ff`'s on the branch, as `review-mut` found; `CARGO_BUILD_JOBS=4`, debug info off): `cargo test -p doom-sound -p
doom-sim -p doom-game` in debug and in release: exit 0, 1,563 passed, 0 failed, 2 ignored each; `cargo clippy -p doom-sound -p
doom-sim -p doom-game --all-targets -- -D warnings` and `cargo fmt --all -- --check`: clean. Not run: `cargo +1.85.0 check` (no new
non-test source), the fuzz workspace, rustdoc, the other crates' tests, Windows, macOS.

### The final run at `a8abf0c`

The Mutants workflow ran on `a8abf0c` (the head after the review fixes and the instruction-count fix): cargo-mutants 27.1.0 with
nextest, 24 shards dealt round-robin, every shard 21 to 40 minutes (none cut off, so the list of outcomes is the whole list).
**Totals: 999 mutants, 865 caught, 28 missed, 3 timeouts, 103 unviable.** (865 + 28 + 3 + 103 = 999.) The 28 missed mutants
(`missed-all.txt`, 28 lines) split into two lists: **14 known** (`known-dispositioned.txt`) and **14 new** (`new-survivors.txt`).
The 14 known are the first round's 12 equivalent and 2 unreachable mutants, found again at their lines in `a8abf0c` (the table above
gives the lines at `216f976`; the same mutants at `a8abf0c` are `diagnostic.rs:689`, `sound.rs:309` and `:314`, `logic.rs:204`,
`:207`, `:553`, `:558` (3), `:653` (2) and `:708`, `mixer.rs:288` and `:454`). The 14 new are: `diagnostic.rs` 9 (the DSIM41 reader's cross-check of pending sound events with the
sound records), `logic.rs:120` and `session.rs:107` (the `Display` text of two refusal types) and three in `doom-game`
(`app/mod.rs:712` twice, `:768`).

### The second round: the 14 new survivors

All 14 are killed; none was judged equivalent or unreachable in this round. Lines are those of `a8abf0c`.

| Mutant | Disposition | Test |
| --- | --- | --- |
| `diagnostic.rs:788:18` `>` to `==` and to `>=` (2) | Killed | `dsim41_reader_bounds_the_pending_event_count_by_the_field_count` (a count one above the field count must be the `Limit` refusal; a count equal to it must not) |
| `diagnostic.rs:796:68` guard to `false` and to `true`, `:796:81` `==` to `!=` (3) | Killed | `dsim41_reader_matches_a_pending_sound_event_with_its_record` (17-byte actor and NULL events are calls; an 18-byte sound event with a NULL record is the `Identity` refusal) |
| `diagnostic.rs:798:33` `==` to `!=` | Killed | the same test (an actor event against an actor record, and the NULL mix-ups) |
| `diagnostic.rs:805:17` and `:806:17` delete the arms for sector and button events (2) | Killed | `dsim41_reader_matches_sector_and_button_sound_events_with_their_records` |
| `diagnostic.rs:807:37` guard to `true` | Killed | `dsim41_reader_does_not_count_a_malformed_stop_event_as_a_call` (a 14-byte stop event with no record must not be `Identity`) |
| `logic.rs:120:9` `SoundRefusal` `fmt` to `Ok(Default::default())` | Killed | `every_sound_refusal_has_a_message_that_names_its_facts` (all 7 variants, exact text) |
| `session.rs:107:9` `SessionRefusal` `fmt` to `Ok(Default::default())` | Killed | `every_session_refusal_has_a_message_that_names_its_facts` (6 forms, exact text) |
| `app/mod.rs:712:29` `+=` to `*=` and to `-=`, `:768:9` `layout_dependent_line` to `None` (3) | Killed | `app::tests::the_app_counts_layout_dependent_sessions_across_restarts` (a unit test inside `doom-game`) |

**Eleven are killed by six new tests** (commits `0b5ced7` and `cf6e733` on the branch; 362 insertions in two
test files, `crates/doom-sim/src/wire25_tests.rs` 262 and `crates/doom-sound/tests/survivors.rs` 100, all hand-written test code,
no source file changed): four in `doom-sim` (nine mutants), two in `doom-sound` (two mutants). The forged frames start from the
genuine DSIM41 frame of a room, one rule per case; the expectations are the reader's own documented rule (the comment before its
cross-check and the DSIM41 section of [STATE_FORMAT](../STATE_FORMAT.md)) and `s_sound.c:304` (which origins are placed), not captured
output. The two `Display` tests are **Rust-only** pins of the current wording.

**The three `doom-game` mutants** first had no test: all three need an `App` whose session the layout detector has labelled, and
the app offers no way to install one, so `step10b` marked them SOURCE (a source change would be needed) and proposed a test seam.
The follow-up agent found that none was needed, and **no production line changed**: the new unit test (commit `b5bfcde` on the branch,
109 hand-written insertions in `crates/doom-game/src/app/mod.rs`, test module only) builds a real `App` from the committed synthetic
WADs, replaces the private fresh session that `App::begin` clones with one whose detector fired (the switch-release start of
`p_spec.c:1151`, sound 23, button origin, the 64-bit reading (34, 0), 34 raw units inside the clipping distance; the same
construction as `a_layout_dependent_session_gives_one_plain_line` in `tests/m5c_app.rs`, decision T12), calls `restart`, and
checks the documented text of `layout_dependent_line` for an unlabelled session (None, also after a restart), a labelled one, a
restart into an unlabelled one ("this session is not layout-dependent; 1 earlier session..."), a labelled one after that, and "2
earlier sessions...". It kills all three: `+=` to `*=` leaves the count at 0, so the "1 earlier session" line is missing; `+=` to `-=`
panics on overflow in a debug build and prints 18446744073709551615 earlier sessions in a release build; the `None` body fails
at once.

**How kills were confirmed.** As in the first round: one mutant at a time, applied by hand (a script replaces one exact text on one
line, runs the crate's tests, restores the file), in debug and, for the `doom-game` test, in release too. **No cargo-mutants run was made
locally.** The proof that the mutants are dead in the real run is the confirming run below.
Checks of the tests (the agents, `CARGO_BUILD_JOBS` 2, debug info off): `cargo test -p doom-sim -p doom-sound` in debug and in release,
exit 0, 1,446 passed, 0 failed, 2 ignored each; `cargo test -p doom-game` in debug and in release, exit 0, 124 passed, 0 failed,
0 ignored each; Clippy with warnings denied and `cargo fmt --all -- --check`, clean. Not run by them: the rest of the workspace,
Rust 1.85.0 (no non-test source changed), rustdoc, the fuzz workspace, sanitizers, Windows, macOS.

### The confirming run at `937cf9d`

The same workflow on `937cf9d`, the head that carries the second round's tests (shards 24 to 41 minutes): **999 mutants, 879 caught,
14 missed, 3 timeouts, 103 unviable.** (879 + 14 + 3 + 103 = 999.) The caught count is the final run's 865 plus the 14 new
survivors. **The 14 missed mutants (`mutants3/missed-all.txt`) equal the 14 known ones (`mutants2/known-dispositioned.txt`) line for
line**: the two files, sorted, were compared with `diff` and are identical (and the 14 known are the 14 lines of the final run's
missed list that are not in `new-survivors.txt`). They are the 12 equivalent and 2 unreachable mutants of the first round, each argued
in the table above and agreed by the independent reviewer. **The 3 timeouts are the same three endless loops** (the lists of the two
runs are identical): `doom-sim` `sound.rs:156`, `doom-sound` `logic.rs:658` and `mixer.rs:429` (the lines of `a8abf0c` and `937cf9d`;
the same loops, `get_channel` and `Mixer::start`, were at `logic.rs:619` and `mixer.rs:412` in the [first run](#the-three-timeouts)).

**State of mutation testing.** Every mutant of the final run is caught, timed out as an endless loop, unviable, or one of 14 judged
equivalent or unreachable and independently agreed.

**Not covered by this evidence.** The runs say nothing about `doom-play` (out of scope), about a mutant cargo-mutants does not
generate, or about code changed after `937cf9d` (documents and the agent definition files only, as far as this pass knows). The
equivalent mutants stand on arguments and on the reviewer's attempts to break them, not on a test.

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

## Step 11 part 2: the fuzz targets after the review

One new target and four changed ones. Every run was on the development machine, one target built at a time into a directory
outside the tree with debug info off. **Leak detection was disabled (`ASAN_OPTIONS=detect_leaks=0`) in every AddressSanitizer
run, so none of them is LeakSanitizer coverage.** The four kinds are kept apart: *target compiled*, *seed replay*,
*coverage-guided campaign* (no sanitizer) and *sanitizer campaign*.

**`render_display_state_v2`** (step 11b; the gap [step 5 part 1](#step-5-part-1-the-frame-gate-restore-and-continue-and-the-headless-app-tests)
listed): `DDSP` version 2, which the M5b target `render_display_state` does not read (it and its seeds are unchanged). Three parts
chosen by the input's first byte: **part 0**, `DisplayState::decode_any` on arbitrary bytes (never a panic; version 1 equals the
frozen decode exactly; version 2 equals the version-1 decode of the bytes with the contract byte removed and carries the
`WithSound` contract; a contract byte other than 1 is `Invalid("contract")`; accepted blocks re-encode to the same bytes);
**part 1**, `Display::restore` into a fresh display after k tics of a room (a typed refusal leaves the state as it was; a
success gives exactly the state back, re-encodes to the same bytes, and a second restore of the same state agrees); **part 2**,
`FrameSource::restore` of a display state and a `DSND` together over the committed SNDBUSY item, with five genuine saves (tics
0, 10, 40, 80 and the last), candidates that are genuine, genuine with edits, or raw: both genuine saves must restore, and any
accepted candidate re-saves to identical bytes, restores again equal and runs one more tic (an `Err` leaves the save unchanged).
The seed tool `tools/seed_display_state_v2_fuzz.py` writes **379 seeds** (max_len 70,000): the three committed M5b version-1
states, their version-2 forms **written from the documented layout** (version 1's bytes with the version changed and one contract
byte inserted; nothing taken from the Rust encoder), forged versions, contract bytes, truncations and flipped bytes from a
fixed-seed generator, and edit records over the saves the target makes. `--check` (writes nothing) reports 379 seeds, 0
problems; `--self-test` detects 3 of 3 faults. Observed by the step agent on a worktree based on `c503ede`, before the wipe-flag rule
existed: **target compiled** (address and none builds); **seed replay** under AddressSanitizer, leak detection disabled: 379 files,
386 runs, no crash (cov 13,067); **coverage-guided**, no sanitizer, 300 s: 84,487 runs (cov 6,342, ft 14,566, corpus 261), exit 0,
no crash; **AddressSanitizer campaign**, 300 s, leak detection disabled: 23,568 runs (cov 13,217, ft 31,214, corpus 219), exit 0,
no crash. A temporary instrumented build (reverted) showed the success paths are reached: of the seeds, 6 version-1 and 84
version-2 blocks were accepted by `decode_any`, 27 part-1 restores succeeded, and 16 part-2 restores succeeded (6 genuine, 10 with an
edited sound state). **Findings: none.** Size: the target 334 lines when written (345 now, after the wipe invariant), the seed tool
178, `fuzz/Cargo.toml` 7 and `fuzz/Cargo.lock` 1, all hand-written. **Not run:** a leak-detecting campaign, longer campaigns,
other sanitizers, a 1.85.0 build of the fuzz crate, a build of the whole fuzz workspace, a mutation check of the target's own
assertions.

**Changed after the review** (step 13; each target's assertions follow the code's new rule): `sound_state` restores with the
session's own configuration and requires any other to be refused with `ConfigMismatch`; `sound_logic` gains the placement
invariant (a sector or button start without positions, or a NULL origin with them, is refused); `sound_mixer`'s model checks
the C's refusal order (pitch, right and left volume, then short lump); `render_display_state_v2` part 1 gains the wipe-flag
invariant. **What was run after those changes:** `cargo +nightly check --locked` of the four binaries passed; a fuzz build
without a sanitizer, then **seed replay (`-runs=0`) without a sanitizer**: `sound_logic` 144 seeds, `sound_state` 152,
`sound_mixer` 180, `render_display_state_v2` 379, every one exiting 0 with no panic; `tools/seed_sound_fuzz.py --check` (four
lists) and `--self-test` 3 of 3; `tools/seed_display_state_v2_fuzz.py --check` (379 seeds, 0 problems) and `--self-test` 3 of 3;
`cargo fmt` of the fuzz workspace. **These are seed replays without a sanitizer: not a campaign, and not an AddressSanitizer
replay. No campaign was repeated after the changes** (the campaigns above and those of
[part 1](#step-11-part-1-the-fuzz-targets) ran on the earlier code).

## Step 5 part 1: the frame gate, restore and continue, and the headless app tests

**What was built (one agent, `step5a`, observed on its own scratch worktree):** `RandomContract::WithSound` and
`DDSP` version 2 in `doom-render`; the sound session inside `doom-game`'s loop behind the audio seam (`FrameSource`,
`AudioSink`, `RecordingSink`, `GameData::load_with_sound`); the frame gate; the restore tests; the headless app tests.
The M5b contract and everything frozen are unchanged. Per tic the order is the one in
[COMPATIBILITY](../COMPATIBILITY.md#the-display-contract-with-sound-m5c); the sound session and the display run on
staged copies and are committed together after the frame is drawn.

**The frame gate (source equivalence).** `cargo test -p doom-game --release --test m5c_frames` (22 s; it also passes in
debug): the 64 items at screenblocks 10 and 11 through the app's own per-tic pipeline, no draw count taken from a golden.
**45,540 frames (22,770 per size), all equal:** 45,170 whole-frame digests, 300 frames flagged by the genuine tracker on the
masked digest (the flagged columns equal), 70 not digest-compared (switch parity unknown, the same 70 as M5b). The face
index is equal on all 45,540 frames and the Q row's face-zeroed digest on 45,170 frames. (**Corrected by the review:** this page and a test comment said all 45,470 digest-compared frames; 300 of those, the tracker-flagged ones, have a face-zeroed digest that is `undefined` in the goldens, and the test counted them without comparing. 45,170 are compared on both digests, 300 on the masked digest only, 70 not at all.) Every `P` row
(palette, palette set, face, `message_on`, `M_Random` index), the Q row's cumulative sound draws, the counters, `fuzzpos`,
mapped lines and 200 messages are equal. Endings: 6 renderer refusals (M5VPLAN, M5OPEN, M5SSEG at both sizes), BOSS1's
exit, the demo's end otherwise. 28,082 buffers reached the recording sink (14,041 per size). Detector firings: 0.
Two planted faults were each caught and removed: sound draws applied after `ST_Ticker`'s (first reported on M5OUCH at
screenblocks 10, tic 475: the face 26 against the genuine 25, the frame differing inside the face's rectangle only, the
index equal) and a wipe of 319 draws (M5SKY, tic 0, the index 64 against 65). `crates/doom-render/tests/m5c_display.rs`
repeats the 45,540 frames with the draws taken from the goldens' Q rows (an explicit fixture projection, display only).

**Restore and continue** (`crates/doom-game/tests/m5c_restore.rs`, 4 tests; Rust against the genuine goldens; every frame and `P` row is compared with its golden except the 70 M5SWTCH frames, 35 per size, for which only the status row is, and none of the seven items has a masked frame, as the review made the test's comment say): 7 items
(SNDBUSY, SNDMONST, SNDBTN2, TELE1, BINF1, SLIDEB, M5SWTCH) at 2 sizes, saved at a quarter, a half, three quarters and the
first busy-mixer tic: **56 saves** (16 with a busy mixer channel), each `DDSP` version 2 and `DSND` version 1, re-encoding to
the same bytes, restored next to a simulation clone and continued: 7,580 tics, every frame, face-zeroed digest, palette,
face, `message_on`, message, index, sound report, channel table, mixer and final encodings equal to the unbroken run, and
frames and `P` rows equal to the goldens. Self-test: dropping one busy mixer channel, or adding 1 to the index in `DDSP`,
each restores and then fails to continue equal. Forgeries refused with nothing built: a version 1 display with sound
data, no `DSND` with sound data, a `DSND` with M5b data, the magic, truncation, a version 2 contract byte 0, `DSND`
version 2, a truncated `DSND`, a simulation or display of the next tic. An M5b save is `DDSP` version 1, decodes with
M5b's decoder, restores as the M5b contract and continues equal. `m5c_contract.rs` (3 tests, 4 since the review added the wipe-flag rule): the contract's rules, the
version 2 encoding and its forgeries, a display-only restore.

**Headless app tests** (`crates/doom-game/tests/m5c_app.rs`, 3 tests, 4 since the review added the layout-dependent line; Rust-only): the room `amp`, the M5b graphics WAD and
`m5c_sfx.wad`, M5b's walk script, a fake clock and a recording sink. 88 tics twice give identical reports and buffers
(4 sound records, 55 buffers, which is ceil(315 x 88 / 512), not silence); the index after tic 0 is the draws plus 1 plus
320 modulo 256; the recorded demo replays identically. A pause reaches the sink; 20 paused updates run no tic and no
buffer; a focus-loss pause is delivered at the next update; a restart from pause resumes, starts a fresh session (tic 0, no
channel, a silent mixer, handle counter 0) and the same script gives the same stream. A sound refusal (a later PWAD's 8-byte
`DSPISTOL`, decision T11) at tic 18 leaves the session equal to the one after tic 17, the demo replays to the same
refusal, and **the simulation's tic is not rolled back** (see the next paragraph).

**Deviation from the brief, flagged by the agent and kept.** The brief asked that a sound or display refusal leave the
whole tic undone. Sound and display are undone (a refused tic leaves the saved `DDSP` and `DSND` bytes unchanged); the
simulation's committed tic is not, which would need a per-tic simulation clone (AGENTS.md forbids one) or a staged advance
in `doom-sim`. This follows M5b's owner-approved rule S9; **the owner confirmed this reading for sound and display refusals on 2026-11-18** ([COMPATIBILITY](../COMPATIBILITY.md#the-app-with-sound-and-what-is-presentation)).

**Other checks (observed):** `cargo test -p doom-render -p doom-sound -p doom-game` in release (36 test targets, 0 failed, 1 min
51 s) and in debug (0 failed, 2 min 42 s); clippy with warnings denied, `cargo fmt --all -- --check` and the fuzz
workspace's, `cargo +1.85.0 check` of the three crates, warnings-denied rustdoc, `cargo check -p doom-play`. The M5b tools
were re-run unchanged and passed: `m5b_session.py --check` (4 sessions), `m5b_inject.py --check` (6 scripts) and
`m5b_readme_visual.py --check` (the animation identical). **Not run then:** a fuzz target for `DDSP` version 2 or
`FrameSource::restore` (none existed; [step 11 part 2](#step-11-part-2-the-fuzz-targets-after-the-review) has one), the fuzz workspace build, a workspace-wide build, mutation testing, a sanitizer
campaign.

## Step 5 part 2: the 61 real runs through the whole pipeline (local only)

`tools/local/m5c_real_full.py` runs, for each of the 61 real Episode 1 runs, the genuine M5c runner (strict, at
screenblocks 10 and 11, frames and sound), a strict 32-bit build (sound only), the genuine mixer on the start list, and the
Rust example `m5c_real_rows` (`GameData::load_with_sound` and replay, the app's own per-tic pipeline), and compares row by
row. It reads an IWAD copy in scratch; only counts and digests are committed
(`tests/reference/m5c_real_full_results.json`, `DOOM-M5C-REAL-FULL-RESULTS-v1`). **Kinds of evidence:** source equivalence
for every comparison below; the self-test is a harness check. Two runs with `--write` (12.7 and 13.2 minutes wall, other
agents running, not timings) gave identical totals.

| What | Count |
| --- | --- |
| Runs, tics | 61, 158,966 (equal to the note's section 3.4.2) |
| Sound starts: attempted, started, inaudible, refused for want of a channel, link volume | 26,077, 21,090, 4,877, 110, 0 |
| `M_Random` draws by sound | 20,697 |
| Endings | 27 marker, 12 exit, 22 reborn |
| Sound rows compared, all equal (0 differ) | S 26,077; T 10,470; K 21,090; C 10,633; L 1,042; R 158,966 |
| Listener columns not compared (starts with no position) | 5,980 |
| Frames at screenblocks 10 | 158,966: 158,472 whole-frame digest, 494 masked, 0 not compared; 0 mismatched rows |
| Frames at screenblocks 11 | 158,966: 158,414 whole, 552 masked, 0 not compared; 0 mismatched rows |
| Face index equal | 158,966 of 158,966 at each size |
| Face-zeroed digest equal | 158,472 of 158,472 (size 10), 158,414 of 158,414 (size 11) |
| Mixer, genuine mixer on the start list | 61 of 61 runs every row equal; 21,090 start echoes, 98,578 buffers |
| Genuine sound streams equal the committed `m5c_real` digests; equal across both sizes; Rust sound identical across sizes | 61 of 61; 61 of 61; 61 of 61 |

**T12.** The 61 runs hold 4 switch-release starts, 0 with a busy next slot, 0 later `S_UpdateSounds` reads of a release
origin. **Detector firings: 0; runs labelled layout-dependent: 0.** The genuine 32-bit build's sound streams equal the
64-bit ones on 61 of 61 runs, 0 differing rows (so none differs in audibility, volume, separation or draws).

**Self-tests:** 61 of 61 passed (a frame digest, the face and a mixer buffer fault planted in every run, a volume fault in
49 runs and a swapped serial in 48; the idle runs have no sound to plant in). The first full run failed only on an earlier
form of that rule (at least 4 plants) on the 12 idle runs; every comparison in it was equal as well.

**Synthetic mode** (`--synthetic --selftest`, no IWAD, 15 s): 63 items (M5CHEAT is left out, it needs `replay_adjusted`),
every row equal: S 785, T 378, K 723, C 4,311, L 176, R 21,870; 381 unplaced listener columns; 63 of 63 mixer goldens (741
starts, 13,489 buffers); 21,870 frames per size (21,685 whole, 150 masked, 35 switch parity); detector 0; planted faults
caught on BARON1, SNDMONST and SNDBUSY.

**No frozen-wire sound fault was found, no divergence, nothing contradicting the note.** **Not run:** `--check` of the results file
(two `--write` runs agreed, but `--check` itself was not run), the `doom-game` test suite for this step, a sanitizer variant on
the real runs (`m5c_real.py` covers the strict, overflow and address variants of the genuine runs), CI (local only by design).
Sizes: the example 395 lines, `tests/common/rows.rs` 354 (moved from the gate), the tool 511, and the generated results file
4,652 lines.

## Step 5 part 3: scripted app sessions

`tools/reference/m5c_session.py` plays each script of `tests/reference/m5c_sessions` (five new: `death`, `fight`, `pause`,
`restart`, `switch`, hand-written to make sound matter) and the four M5b scripts (unchanged) through the headless app under
the new contract (`crates/doom-game/examples/m5c_session.rs`) and, per part (a restart is a new part, a fresh process by
decision T15), through the genuine input driver (its demo must equal the app's) and the genuine M5c runner in the strict,
overflow and address variants, with sound on. It compares on every tic the executed command, the frame digest under M5b's
rules, the status-bar rows, the face and the index, and every sound row, then runs the genuine start list through the
genuine mixer and compares every buffer the app handed its sink. Committed results:
`tests/reference/m5c_session_results.json` (`DOOM-M5C-SESSION-RESULTS-v1`, genuine-side counts and digests).
**Source equivalence**, plus a harness self-test.

| What | Count |
| --- | --- |
| Sessions, parts, tics | 9, 10, 2,213 (all commands equal) |
| Frames compared on the whole digest | 2,213 (0 masked, 0 not comparable); face and index rows equal 2,213; face-zeroed digests equal 2,213 |
| Sound rows (genuine counts, all equal) | S 142, T 55, K 128, C 1,122, L 56, R 2,213 |
| Start attempts, started, no channel, inaudible | 142, 133, 8, 1 |
| Listener columns not compared | 43 |
| Buffers the app handed its sink, equal by digest to the genuine mixer's | 1,367 (mixer rows equal on all 10 parts) |
| Variants | 10 of 10 parts: strict = overflow = address (frames, sound streams, endings); mixer variants identical |
| Pause | 5 sessions each with one pause: no tic and no buffer between pause and resume; the restart session shows 2 session starts with a resume before the second |
| T12 detector | 0 firings; no layout-dependent session (no release had a busy next slot, so the detector was not exercised) |

The play of `pause` without its pause line gives identical streams and the genuine input driver records the same demo; only
the pause's release event (decision S6, one empty mouse event at the first tic after the resume) differs in the app's `E`
rows. With integrated handles every start after about tic 100 is finished at its first update, so the `switch` session has no
`C` rows, and the genuine engine agrees.

**Self-test** (on the `fight` session): the no-fault baseline is clean, and seven planted faults are each caught at their
place: a changed command byte, a changed frame row, a changed start volume, a dropped buffer, a dropped tic, a dropped driver
event and a tic logged during a pause. `--check` also failed on a results file with one count changed. In the job's form from a
fresh work directory: exit 0, 88 s wall including the C builds (other agents were running; not a timing). Address runs use
`ASAN_OPTIONS=detect_leaks=0`: leak detection was disabled, so this is not LeakSanitizer coverage. **Not run:** a state
comparison through the independent DSIM encoder (only the commands are read from the dumps); a strict 32-bit run; real maps;
a session with a busy next button slot, a focus-loss pause or a sound refusal (the refusal is a Rust-only test above).
CI itself had not run this step when it was written; see [the CI record](#the-ci-record).

**The local verifier for `doom-play` sessions** (`tools/local/m5c_session_real.py`, 328 hand-written lines, commit `27cc8e8`; its
two commands are in [TESTING](../TESTING.md#m5c-sound-effects-the-genuine-sound-logic-and-mixer), commit `0cd43ae`; not in CI). It is
the M5c counterpart of the M5b session check: given the IWAD and demos that `doom-play` recorded with sound, it replays each through
the Rust pipeline and through linuxdoom-1.10 built with its in-process sound code (one define removed) under the confirmed
switches (strict, overflow and address variants required identical; **address with leak detection disabled**), runs the genuine mixer
on the genuine start list, and compares every frame at the session's screen size, every sound-logic row, every mixer row and
buffer digest, the tic count and the T12 label, importing the comparison code of `m5c_real_full.py`. It prints one `VERDICT: PASS` or
`VERDICT: FAIL` line per demo, with the first differing tic and kind on a failure. **Source equivalence** per demo it is given.
- **Self-check without an IWAD** (`--synthetic`, about 44 seconds including the C builds, run twice with the same output): the
  committed synthetic demos SNDBTN2 (248 tics; frames 248 whole; sound rows C 11, K 11, L 3, R 248, S 11; starts 11; buffers 153;
  not layout-dependent, 2 release starts with the next slot busy in 1; the 32-bit run had 1 differing row, 0 in behaviour) and
  SNDMONST (452 tics; frames 452 whole; C 318, K 115, L 28, R 452, S 116, T 48; starts 115; buffers 279) both PASS. Three **planted
  faults** on SNDMONST are each caught at their place: a turn byte of tic 226 changed in the Rust side's demo only (first
  difference at tic 226, a frame), the volume of the start at tic 21 changed in the Rust sound rows (tic 21, sound), and one
  buffer digest after tic 3 changed in the Rust mixer rows (tic 3, mixer): 3 of 3. These demos are the committed item demos, not
  demos recorded by the headless app.
- **Two real-map demos** (a scratch copy of the IWAD, deleted afterwards; the committed demos `M6DIE.lmp` and `M8FIGHT3.lmp`):
  M6DIE at screenblocks 10 PASS (350 tics; frames 349 whole and 1 masked, face equal on 350; sound rows C 177, K 30, L 9, R 350,
  S 32, T 32 with 5 listeners unchecked; starts 30; buffers 221; no release; the genuine ending reborn) and at screenblocks 11 PASS
  (348 whole, 2 masked, the rest as above); M8FIGHT3 at screenblocks 10 PASS (3,822 tics; frames 3,793 whole and 29 masked, face
  equal on 3,822; C 327, K 651, L 23, R 3,822, S 656, T 401 with 173 unchecked; starts 651; buffers 2,354; 1 release start with
  the next slot not busy; detector 0 firings; the 32-bit run identical). Every figure equals the demo's record in
  `tests/reference/m5c_real_full_results.json`, checked by printing the committed records.
- **Not run:** a session of the owner's (none exists to read); a demo with a busy-next-slot release on a real map (none of the 61
  has one; SNDBTN2 covers the 32-bit path); a restarted (multi-part) `doom-play` session; Windows or macOS; the planted-fault check
  covers the synthetic path only. One observation, not a fault: `m5c_real_rows` prints the replay's ending as `None` when the demo
  runs out where the genuine side says reborn or marker; the tool prints both and does not compare them.

## Step 7: sound in `doom-play`

**What was built** (`crates/doom-play` only; Rust-only tests, no genuine comparison): `DeviceSink` (a queue the device
callback pulls from and the loop pushes one buffer to; a lock that the loop holds only to copy one buffer in and the callback only to copy one period out: the loop never waits for the device to play anything, but it can wait for one such copy, which "never a wait" overstated), `Discard` (counts and drops
buffers: no device, or `--no-sound`), `--volume N`, `--no-sound`, the mute key **End** (the original's `xlatekey` leaves it
as a keysym the game never sees, i_video.c:152-157, g_game.c:566-571; it is not a menu hot key or an app key), mute on
pause, one stderr line when no device opens, and a launch that loads wire 25 with sound. Numbers (presentation, not
claims): 512-frame buffers, a start threshold of one buffer (**two buffers, 1,024 frames, 92.9 ms, since the review**: [step 13](#step-13-independent-review)), a cap of 2,048 frames; latency by arithmetic, not measured.
Underrun fills the rest of a pull with zeros, counts it and re-arms the threshold; overrun drops the oldest frames and counts
them. `cargo test -p doom-play`: 20 passed (21 since the review), 9 of them new (the numbers, underrun and overrun, pause and session start, the
integer saturating gain, the sink and the no-device line; the options; the mute key not forwarded; and
`session_tests.rs`, which runs the app on eight variants of the sound options: all 88 tics equal in command, frame, index,
sound report, channel table, layout label and demo, the unscaled stream and every sink call equal, and the demo replaying the
same; on a device sink volume 0 and a muted run give 0 non-zero samples, volume 37 equals volume 100 scaled sample by sample;
a planted fault that forwards End as the up arrow fails it). `cargo test -p doom-play -p doom-game --release` and in debug all
pass (118 tests in `doom-game`, 20 in `doom-play`); clippy, fmt (including the fuzz workspace), 1.85.0 and rustdoc clean.
**Not run:** workspace-wide build, fuzz, sanitizers, mutation testing, CI, Windows and macOS builds.

**On this machine** (WSLg, no audio client library installed): `doom-play --audio-probe` exits 1 and prints that PulseAudio
and ALSA gave `MA_NO_BACKEND`. A session of 105 tics with the AMP room, `m5b_gfx.wad` and `m5c_sfx.wad` printed exactly one sound
line (no device opened, the game runs without sound output) and the tic statistics line, exit 0, a 434-byte demo; with
`--no-sound` the same demo and no sound line.

**The first real device open (digital silence only).** With a scratch copy of the PulseAudio client library and its
dependencies (downloaded with `apt-get download` into a scratch directory and unpacked, not installed), run through
`LD_LIBRARY_PATH`, `doom-play --audio-probe` opened PulseAudio's "RDP Sink" and requested 11,776 frames of silence in about
a second, exit 0. **No game session was run with that library path. Nobody heard anything, and no game audio has reached any
device.** The scratch directory was deleted.

**The start-up stall (a finding from the M5b code, not from sound).** In the step's runs the window sometimes opened and no
tic ran, the demo stayed at 14 bytes (header and end marker) and the run hit its timeout: 4 of 24 runs with the new binary and
1 of 25 with the M5b-contract binary built from `216f976`. A separate diagnosis (agent `stall`, 260 runs of the two
binaries, each `timeout 20`, `--quit-after-tics 105 --tic-stats`, one at a time, under WSLg) gives:

| Batch | Binary | Runs | Stalled |
| --- | --- | --- | --- |
| idle, `--no-sound` | new (`7f34f2c`) | 40 | 0 |
| idle, with sound (the discarding sink: no device could open) | new | 20 | 0 |
| idle | M5b contract (`216f976`) | 40 | 1 |
| idle, `--no-sound`, X events logged by a scratch copy of the window library | new | 40 | 0 |
| loaded (8 busy shell loops on 12 threads), `--no-sound`, X events logged | new | 40 | 2 |
| loaded, `--no-sound` | new | 40 | 0 |
| loaded | M5b contract | 40 | 0 |
| all of the diagnosis | new / M5b contract | 180 / 80 | 2 / 1 |
| the step's earlier runs | new / M5b contract | 24 / 25 | 4 / 1 |

Two-sided Fisher exact tests: 0 of 40 against 1 of 40 (same conditions, interleaved) p = 1.0; 2 of 180 against 1 of 80
p = 1.0; with the earlier runs 6 of 204 against 2 of 105 p = 0.72; the earlier runs alone 4 of 24 against 1 of 25 p = 0.19;
loaded 2 of 120 against idle 1 of 140 p = 0.60. **No difference between the binaries or with load is distinguishable from
chance.** The rate in the diagnosis (3 of 260) is lower than in the earlier set (5 of 49); what differed in the earlier
session (machine load, the pointer's position) is unknown, and the load of other agents during the "idle" batches was not
recorded per run.

*Observed:* the backend was X11 (both `DISPLAY` and `WAYLAND_DISPLAY` were set and the app chooses X11 when `DISPLAY` is set);
in every logged stall the window received an X `FocusOut` in its first 132 ms (all three logged stalls: at tic 1), the M5b
focus-loss rule paused the app, and the `FocusIn` that followed 0 ms later is turned by the window library into a
"restored" event that `doom-play` does not use (only Esc or Pause resume), so the loop kept running (about 30 updates and
draws a second) with the clock stopped and `--quit-after-tics` never reached. In the X-level log (80 runs): 78 normal runs
began with `FocusIn` (mode normal, detail nonlinear) and then an `EnterNotify` caused by the app's own pointer grab; both
runs that stalled in that log began with the `EnterNotify` from the grab, then a `FocusOut` (mode normal, detail
`NotifyPointer`), then at once a `FocusIn`; one normal run also had the grab first, so that order is necessary in every stall
seen but not sufficient. *Inferred, not observed:* that this is a race between the app's immediate pointer grab and
XWayland's first focus assignment (the detail's meaning is from the X protocol); what the screen shows (from the code, the
first frame at half brightness with the PAUSED box, the mouse released) and that Esc or Pause resumes. **Nobody saw the screen
and no key was pressed in a stalled window.** The sound work is not involved (the stalled runs were `--no-sound`, and the M5b
binary stalls the same way with the same event order). Not run: the Wayland backend, Windows, macOS; which process picks the
focus order. **Nothing was changed for it**; the owner can decide whether to ignore a focus loss that arrives before the first
focus gain (a change of M5b's input behaviour, left for the owner's own play of the focus rules).

## Step 12: timings

Recorded, never gated (no threshold and no CI step; decision R15 of M5a carries over). Observed 2026-11-18
(America/Los_Angeles): the three runs started at 14:03:23, 14:03:34 and 14:03:45 and took 10.7 to 10.8 s each. Commit:
`d43aa6f` (the new example on top of `ad13de5`; nothing else uncommitted). Machine: Intel Core i7-8750H @ 2.20 GHz, WSL2
(kernel 6.18.58.1-microsoft-standard-WSL2); rustc 1.100.0 (0ef1bc386 2026-11-09); the repository's release profile
(`cargo build -p doom-game --release --example m5c_bench`, no `CARGO_PROFILE_RELEASE_*` variable set). The binary was
built in and run from the Linux filesystem (ext4): `CARGO_TARGET_DIR=~/scratch/m5c/step12/target`, binary
`~/scratch/m5c/step12/target/release/examples/m5c_bench`; the committed synthetic WADs and demos it reads are in the
worktree under `/home` (ext4); nothing ran from `/mnt/*`. The orchestrator started the step with no other agent running.
After the build's own load (1-minute load average 2.16 at 14:02:36) the step waited 40 s (bounded at 10 minutes) until
the load was below 1.2. Before each run `uptime` showed a 1-minute load average of 0.93, 0.94 and 1.03 (5-minute 1.76,
1.73, 1.73, still decaying from the build) and the busiest processes were `claude` (3.0 %, the orchestrating session)
and `rust-analyzer` (0.1 %, idle); nothing else was started while a run was going. No IWAD was read, no audio device was
opened and no sound was played: every case is a committed synthetic room with the committed synthetic sound lumps
(`m5c_sfx.wad`, `m5c_music.wad`) and `m5b_gfx.wad`.

New example `m5c_bench` ([command](../TESTING.md#m5c-timings)), beside `m5b_bench`. For each room and screenblocks one
unmeasured warm-up replay of each kind, then one measured replay of each kind, each on a freshly loaded simulation, to the
end of the room's script; the simulation advance is outside every timer. The whole sequence ran 3 times and each cell is
the three runs separated by `/`, in microseconds. Every replay ran to the end of its script with no refusal (stderr was
empty). The four series:

- **sound tic**: `Sound::tic_from_simulation` alone: the tic's sound records, `S_UpdateSounds`, the session's internal
  copies of its logic and mixer (it works on copies and commits at the end), and the buffer when the schedule makes one due
  (so a tic that mixes includes the mix). It runs on a session of its own, fed the `M_Random` index of the replay's real
  frame source, which runs untimed between the timers; the example stops if the two sessions' tic reports ever differ
  (none did).
- **mix buffer**: `Mixer::mix_buffer` alone, per buffer. The mixer the buffer is mixed from is rebuilt outside the timer
  (the mixer before the tic plus the tic's started sounds); the example stops if a rebuilt start or the rebuilt buffer
  differs from the session's (none did). The count is buffers, not tics.
- **frame step, sound**: `FrameSource::tic` under the M5c contract: the sound session, then the display's observe and draw
  with the draws, including the per-tic staging copies of the display and the session. It excludes the simulation advance,
  the app's input and event layer, the host report and the sink call that hands a buffer on.
- **frame step, M5b**: the same call on the same room loaded without sound (the M5b contract: the display's observe and
  draw), in a separate replay. Both frame series were timed in replays of their own, so a few microseconds between them is
  inside the run-to-run spread seen in the same table.

Rooms: `SNDBUSY` (many starts at once, evictions), `SNDMONST` (a fight), `SNDBTN2` (switches, with a lift),
`SLIDEB` (a sliding door), `M5AMP` (the quiet room: one started sound in 124 tics) and `M5STDIG` (M5b's room, for
comparison with its record; its replay here is the app's frame source with the M5c data, not M5b's `Display` calls, so its
numbers are close to but not those of the M5b table).

| Room | Screenblocks | Count | Series | Median | p95 | Max |
| --- | --- | --- | --- | --- | --- | --- |
| SNDBUSY | 10 | 125 | sound tic | 2.9/3.0/2.7 | 5.7/4.8/4.4 | 33.7/17.8/31.6 |
| SNDBUSY | 10 | 77 | mix buffer | 2.3/2.3/2.2 | 3.5/3.5/3.4 | 13.5/13.6/13.3 |
| SNDBUSY | 10 | 125 | frame step, sound | 409/410/395 | 479/451/495 | 531/544/668 |
| SNDBUSY | 10 | 125 | frame step, M5b | 392/386/377 | 475/512/427 | 656/829/558 |
| SNDBUSY | 11 | 125 | sound tic | 2.9/2.9/2.8 | 4.8/4.3/4.2 | 18.2/16.9/17.8 |
| SNDBUSY | 11 | 77 | mix buffer | 2.3/2.2/2.3 | 3.5/3.4/3.5 | 13.6/13.2/13.2 |
| SNDBUSY | 11 | 125 | frame step, sound | 427/422/419 | 475/567/480 | 532/846/806 |
| SNDBUSY | 11 | 125 | frame step, M5b | 405/400/400 | 487/455/488 | 594/657/562 |
| SNDMONST | 10 | 452 | sound tic | 2.8/2.8/2.9 | 6.0/6.2/6.2 | 28.2/47.8/49.1 |
| SNDMONST | 10 | 279 | mix buffer | 2.3/2.3/2.4 | 4.7/5.6/4.8 | 21.3/25.9/10.4 |
| SNDMONST | 10 | 452 | frame step, sound | 623/631/621 | 766/788/772 | 999/1064/1176 |
| SNDMONST | 10 | 452 | frame step, M5b | 609/633/599 | 801/1127/772 | 1097/2526/1217 |
| SNDMONST | 11 | 452 | sound tic | 2.8/2.9/2.9 | 5.9/6.8/6.4 | 17.6/63.6/26.8 |
| SNDMONST | 11 | 279 | mix buffer | 2.3/2.3/2.3 | 4.7/6.1/6.0 | 10.4/22.9/31.2 |
| SNDMONST | 11 | 452 | frame step, sound | 640/657/645 | 793/804/817 | 1058/1154/1030 |
| SNDMONST | 11 | 452 | frame step, M5b | 627/634/627 | 786/792/781 | 1117/1270/1590 |
| SNDBTN2 | 10 | 248 | sound tic | 2.7/2.7/2.7 | 3.5/4.2/3.6 | 8.8/9.0/14.6 |
| SNDBTN2 | 10 | 153 | mix buffer | 2.2/2.3/2.2 | 3.4/3.5/3.4 | 19.8/6.3/7.1 |
| SNDBTN2 | 10 | 248 | frame step, sound | 327/340/327 | 419/414/436 | 498/491/598 |
| SNDBTN2 | 10 | 248 | frame step, M5b | 309/320/317 | 360/366/418 | 552/475/545 |
| SNDBTN2 | 11 | 248 | sound tic | 2.7/2.8/2.7 | 3.9/4.1/3.8 | 8.5/24.3/8.9 |
| SNDBTN2 | 11 | 153 | mix buffer | 2.2/2.3/2.2 | 3.4/3.5/3.4 | 6.1/6.3/6.1 |
| SNDBTN2 | 11 | 248 | frame step, sound | 344/357/344 | 455/415/422 | 712/596/552 |
| SNDBTN2 | 11 | 248 | frame step, M5b | 324/340/326 | 415/387/416 | 500/508/673 |
| SLIDEB | 10 | 100 | sound tic | 2.8/2.9/2.8 | 4.1/4.5/4.1 | 13.1/18.8/12.5 |
| SLIDEB | 10 | 62 | mix buffer | 2.2/2.3/2.2 | 3.3/3.5/3.6 | 3.6/3.6/289.0 |
| SLIDEB | 10 | 100 | frame step, sound | 363/368/354 | 479/430/406 | 611/549/522 |
| SLIDEB | 10 | 100 | frame step, M5b | 336/348/336 | 414/406/406 | 541/454/612 |
| SLIDEB | 11 | 100 | sound tic | 2.8/2.8/2.8 | 4.2/4.3/4.3 | 5.0/15.8/19.1 |
| SLIDEB | 11 | 62 | mix buffer | 2.2/2.2/2.2 | 3.4/3.3/3.2 | 3.6/3.6/3.6 |
| SLIDEB | 11 | 100 | frame step, sound | 374/372/369 | 450/462/456 | 524/530/582 |
| SLIDEB | 11 | 100 | frame step, M5b | 353/356/352 | 450/487/451 | 521/740/578 |
| M5AMP | 10 | 124 | sound tic | 2.7/2.7/2.7 | 3.0/3.0/2.9 | 5.3/5.6/31.7 |
| M5AMP | 10 | 77 | mix buffer | 2.2/2.2/2.2 | 2.3/2.3/2.3 | 3.4/3.6/3.4 |
| M5AMP | 10 | 124 | frame step, sound | 336/336/344 | 423/411/495 | 509/489/782 |
| M5AMP | 10 | 124 | frame step, M5b | 316/315/326 | 391/383/443 | 482/546/654 |
| M5AMP | 11 | 124 | sound tic | 2.7/2.7/2.7 | 4.0/3.0/3.8 | 12.8/20.1/33.1 |
| M5AMP | 11 | 77 | mix buffer | 2.2/2.2/2.3 | 3.4/2.3/2.5 | 19.9/3.4/15.4 |
| M5AMP | 11 | 124 | frame step, sound | 354/358/364 | 434/449/488 | 501/537/805 |
| M5AMP | 11 | 124 | frame step, M5b | 337/337/348 | 430/401/458 | 529/587/836 |
| M5STDIG | 10 | 877 | sound tic | 2.8/2.8/2.8 | 4.5/4.7/4.6 | 33.8/24.8/13.7 |
| M5STDIG | 10 | 540 | mix buffer | 2.3/2.3/2.3 | 3.5/3.5/3.5 | 32.7/41.4/19.5 |
| M5STDIG | 10 | 877 | frame step, sound | 388/383/373 | 518/483/470 | 905/752/878 |
| M5STDIG | 10 | 877 | frame step, M5b | 372/356/360 | 534/468/457 | 1371/775/740 |
| M5STDIG | 11 | 877 | sound tic | 2.7/2.8/2.7 | 4.5/5.0/4.6 | 34.8/26.4/50.3 |
| M5STDIG | 11 | 540 | mix buffer | 2.3/2.3/2.3 | 3.5/3.5/3.5 | 26.5/34.1/24.8 |
| M5STDIG | 11 | 877 | frame step, sound | 396/420/392 | 494/570/504 | 811/929/715 |
| M5STDIG | 11 | 877 | frame step, M5b | 375/382/374 | 469/513/477 | 626/752/955 |

Workload of each room (identical at both screenblocks and in all three runs): started sounds (the mixer's start echoes),
buffers mixed and busy mixer channels per mix.

| Room | Tics | Started sounds | Buffers | Busy channels per mix: mean | max |
| --- | --- | --- | --- | --- | --- |
| SNDBUSY | 125 | 15 | 77 | 0.30 | 8 |
| SNDMONST | 452 | 115 | 279 | 0.72 | 6 |
| SNDBTN2 | 248 | 11 | 153 | 0.14 | 3 |
| SLIDEB | 100 | 4 | 62 | 0.11 | 2 |
| M5AMP | 124 | 1 | 77 | 0.03 | 1 |
| M5STDIG | 877 | 137 | 540 | 0.41 | 1 |

**Outliers.** Maxima are single-sample outliers of the shared host; the series' medians and 95th percentiles agree
between runs. The largest are the 289 us `mix buffer` maximum of `SLIDEB` at screenblocks 10 in run 3 (the same case's
maxima are 4 us in runs 1 and 2; one buffer of 62; the cause was not investigated), the 64 us `sound tic` maximum
(`SNDMONST`, screenblocks 11, run 2) and the 2,526 us `frame step, M5b` maximum of `SNDMONST` at screenblocks 10 in run 2.
Medians are printed to a tenth of a microsecond for the first two series, whose values are a few microseconds; the clock's
own resolution was not measured.

**Reading.** A tic of the game is 28,571 us. The sound session's tic had a median of 2.7 to 3.0 us, a 95th percentile of
at most 6.8 us and a worst single tic of 64 us here; a mixed buffer had a median of 2.2 to 2.4 us, a 95th percentile of at
most 6.1 us and a worst single buffer of 41 us apart from the one 289 us outlier above. The whole frame step with sound had
medians of 327 to 657 us and a worst single step of 1,176 us. Against the same rooms under the M5b contract the median of
the frame step with sound was between 1.4 us lower and 37.9 us higher (median difference 18.3 us over the 36 case-runs; the
spread between runs of the same case is of similar size). In arithmetic against the tic: the sound tic's median is about
0.01 % of 28,571 us, its worst single tic 0.22 %, the worst single mixed buffer 1.0 % (the outlier) and the worst single frame
step with sound 4.1 %.

**Limits.** Not measured: any real audio device or backend, the device thread and its queue lock, latency, underruns or
overruns (this machine has no audio client library installed, so even the audio probe finds no backend there); the
windowed loop's tic jitter with sound (the M5b jitter runs were not repeated with sound); the app's event layer and the
host's own report; the layout detector's cost beyond the starts these rooms make; Windows and macOS; any
real map (the rooms are small synthetic maps whose mixer channels are idle at most mixes, as the table above shows: a
mix with all eight channels busy occurred in `SNDBUSY` only, and which sample it was is not known); anything on a cold
cache or an unquiet machine. One run of each case per sequence on a shared host; nobody judged anything by ear.


## Step 13: independent review

**Who and what.** Three reviewer agents, each given one part of the milestone, reviewed the branch head `c503ede` on
2026-11-18 (America/Los_Angeles). **They were independent of the authors:** none read the authors' reports (`review-app` also
did not rely on commit messages; `review-sound` checked commit messages and doc comments against the code and took none as
fact), and besides the code, the C and their briefs each read only section 5 of the investigation (the owner's rulings and
T1-T16) of the milestone's documents. Each changed nothing in the repository and worked in scratch copies; the builds and
copies were deleted (`review-sound` kept its probe file and test log in scratch), and the free space on `/mnt/c` was 26 GB
before and after in each.

| Reviewer | Part | Reviewed |
| --- | --- | --- |
| `review-sim` | wire 25 in `doom-sim` | `git diff origin/master...c503ede -- crates/doom-sim` (merge base `639407f`; 22 files, +2,264 and -57) against linuxdoom-1.10, and the four Python tools (`level_snapshot_m5c.py`, `level_compare_m5c.py`, `m5c_wire25_check.py`, `m5c_t4.py`) |
| `review-sound` | `crates/doom-sound` | every file of the crate, its tests and `tools/reference/m5c_sounds_table.py`, against `s_sound.c`, `i_sound.c` and the other C it names |
| `review-app` | display contract, game loop, app | the diff of `crates/doom-render`, `crates/doom-game` and `crates/doom-play` (merge base `639407f`) against `d_main.c`, `g_game.c`, `st_stuff.c`, `m_random.c`, `f_wipe.c`, `s_sound.c` and `i_video.c` |

**Result: no BLOCKER; one MAJOR; 11 MINOR and 19 NOTE findings (31 in all: `review-sim` 1 MAJOR, 3 MINOR, 4 NOTE; `review-sound` 4
MINOR, 6 NOTE; `review-app` 4 MINOR, 9 NOTE).** `review-sound` found no behaviour that differs from linuxdoom-1.10 (built with
its in-process sound code, one define removed) for any input the game can produce; its findings are validation gaps, test and
documentation precision and unreachable edge cases. The fixes were made by two agents (`fix-sim`, `fix-app`), **each finding was
agreed after they read the C and the code, and none was disputed**; the commits are on the branch. Each fix's test was shown to
fail before the change and pass after, except where the table says otherwise.

### Findings and what was done

| Finding (reviewer) | What it was | Disposition and commit |
| --- | --- | --- |
| **MAJOR 1** (`review-sim`) | The DSIM41 reader refused every capture of a map the THING truncation cuts short: its bounds and totals were counted over every `map.things` record, the original over the records before the first Doom II type (p_setup.c:317-337; p_mobj.c:790, 792). Confirmed by running: all 93 captures of the T4 map refused with `Identity`; with one equality disabled all decoded. No test decoded a truncated-map capture | Fixed: the reader counts the loaded prefix (`2d447f3`); test `every_t4_capture_decodes_with_the_dsim41_reader` (`b942c67`), which failed at tic 0 before the fix and decodes 93 of 93 after it |
| MINOR 2 (`review-sim`) | The constructor's admission bounds and the transient allowance counted the unspawned tail; a map the original plays could be refused with `Capacity` at load. Not confirmed by running by the reviewer | Fixed (`591c793`), and `fix-sim` found a second site of the same cause: the world-pickup cross-check refused such a map at setup with `PlayerBodyError::Invariant`. Test `an_unloaded_thing_tail_is_not_counted_under_wire_25` (`1808c3f`): before, `Store(Capacity)`, and with only the bounds fixed `PlayerBody(Invariant)` |
| MINOR 3 (`review-sim`) | The DSIM41 reader accepted frames the writer never produces: a surplus or unknown `events.sounds.*` field, a button start in slot 15, a pending stop with no removal (confirmed by running) | Fixed (`d7104bf`): the group holds exactly its counts and records, slot below 15, and the pending list equals `events.pending`'s sound calls (stronger than the reviewer's suggestion: it also refuses a removal without its stop, a stop of another actor and mismatched starts). Test `dsim41_reader_refuses_forged_frames` (`459f2a6`). The reviewer's "group moved to the front" is not a reader rule (field order is free in every DSIM reader); it is covered by NOTE 7 |
| MINOR 4 (`review-sim`) | The test case "unplaced button" kept `placed = 1` and was refused by the length check, not the rule it names | Fixed (`eddf9b4`): it now writes `placed` 0 on a whole record; with the positional rule removed from the reader in a scratch edit the old case passed and the new one fails |
| NOTE 5 (`review-sim`) | Paths the 64 synthetic items never exercise: `P_CheckMissileSpawn`'s immediate explosion (0 occurrences), a sector sound from a crossing by a non-player mover (0), the hard-death scream (0 replacements) | Agreed; nothing to change (a coverage observation). Their only evidence is the local real runs, which the reviewer could not see |
| NOTE 6 (`review-sim`) | The genuine record comparison is in a job that is not required | Agreed: this is the owner's ruling T16(3) |
| NOTE 7 (`review-sim`) | The Rust writer and the independent encoder place the group differently when `events.pending` is not empty, and the comparator never compared the frame as a whole | Done (`a7e51de`, test `87385a8`): the comparator checks every capture's frame layout; the encoder places the group after the last `events.pending` field. A correction to the review: the load capture (tic 0) does hold a non-empty `events.pending` (seen on BARON1 and SNDBTN2), so the two places differ there. Shown: on a scratch copy of the T4 captures with the tic-6 group moved to the front, the new comparator reports a layout difference at tic 6 and the old one reported a match. The self-test plants 2 faults (75 checks, were 73) |
| NOTE 8 (`review-sim`) | A fresh record vector is allocated every tic | Agreed; not a full-simulation clone; passed to the timing step ([step 12](#step-12-timings)) |
| Rustdoc | | `Diagnostic::decode_episode1_v3`'s documentation states the reader's rules (`0fa53df`) |
| M1 (`review-sound`) | A start whose origin can never be the listener (a sector or button origin, or another actor), given without a placement, was played as non-positional, and a release then skipped the T12 detector. Confirmed by running | Fixed (`cc67f11`): `SoundLogic::start` refuses a sector or button origin without a placement and a NULL origin with one; `Sound::tic` refuses an actor start without a placement whose actor is not the listener (`SoundRefusal::Placement`; the sound number is checked first). Tests `refusals_preserve_the_state` (new block) and `an_unplaced_start_of_another_actor_is_refused` failed before. **The reverse case,** a start of the listener's own body that carries a placement, which the C never measures (s_sound.c:304) and the simulation never records, was named by `fix-app` as left open and is refused by a follow-up commit (`d2be47d`, same refusal; its assertion fails without the change: the start was accepted with arguments [8, 129, 116, 64]) |
| M2 (`review-sound`) | Restoring `DSND` trusted the configuration in the bytes (map number, volume, channel count, texture count). Confirmed by running: map 8, volume 0 and texture count 0 all restored | Fixed (`5832e4e`): `Sound::from_state` and `Sound::restore` take the caller's configuration and refuse another (`StateError::ConfigMismatch`); the detector's texture count must equal it. `FrameSource::restore` passes the data's configuration. Tests `restore_refuses_a_configuration_other_than_the_callers` and a map-8 forgery in `m5c_restore` (compile failure before: the API changed) |
| M3 (`review-sound`) | The self-test of `m5c_sounds_table.py` could not fail (it only checked an in-memory string replacement) | Fixed (`9764956`): the tool's real comparison and digest check run on planted inputs, 8 of 8 cases. With the comparison always "matches" 4 of 8 exit 1; with the digest check always passing 7 of 8 exit 1 |
| M4 (`review-sound`) | The gate's module documentation said every row is compared; the `#`, `W` and `Z` rows, 381 of 785 start rows' listener columns and the device rows are not | Documentation fixed (`c141cb9`); the complete list is [in TESTING](../TESTING.md#what-the-sound-gates-do-not-compare) |
| N1 (`review-sound`) | A state saved while a linked `sfx_chgun` is waiting to be mixed could not be decoded (`Position { slot: 0 }`, confirmed by running) | Fixed (`24007da`): exactly a linked sound with `pos`, `end` and `frac` 0 is accepted (i_sound.c:103, 799, 808-811, 330-332, 591-607). Test `a_linked_sound_saved_before_its_frame_is_mixed_restores` failed before. No source file starts `sfx_chgun`, so no game reaches it |
| N2 (`review-sound`) | `DSND` does not check its fields against each other (handles against tics and the counter, a busy channel against its looked-up bit, a mixer position against its start tic) | **Not changed; disclosed** in [STATE_FORMAT](../STATE_FORMAT.md#sound-session-state-dsnd-version-1-m5c) |
| N3 (`review-sound`) | The mixer checked `ShortLump` before the volume ranges; the C reads the short lump only at the mix | Fixed (`229e0eb`): pitch, then right and left volume, then the short lump. Test `the_mixer_refuses_in_the_c_order` failed before; the fuzz target's model follows. Only the refusal named could differ, and the C's volumes are always in range |
| N4 (`review-sound`) | The three short-lump witnesses and the pitch-256 probe compare only the stop position and counts; four items run only to the length of their stream | **No code change; documented** ([TESTING](../TESTING.md#what-the-sound-gates-do-not-compare)) |
| N5 (`review-sound`) | A comment overstated the step table's margin | Fixed (`5700cb5`, comments and the tool's docstring only): 0.0125 holds for the 252 non-integer entries; p = 0, 64, 128 and 192 are exact powers of two (checked with Python's `decimal` at 60 digits) |
| N6 (`review-sound`) | Public functions did not guard their input ranges: `adjust_sound_params` overflowed `i32` for a volume outside 0..=127 (panic confirmed, volume `i32::MAX`); `SoundState::encode` truncated an out-of-range volume or channel count with `as u8` | Fixed (`9ddfb8c`): `adjust_sound_params` and the detector's checks return `SoundRefusal::Config`; `encode` writes 255, which `decode` refuses; the padded lump length uses `u32::try_from`. Tests `adjust_sound_params_refuses_a_volume_outside_the_range` (compile failure before) and `encode_never_wraps_an_out_of_range_value_into_an_accepted_one` (300 decoded as 44 before). The 4 GiB lump conversion has no test (not constructible) |
| M1 (`review-app`) | A forged `DDSP` version 2 state with the wipe flag set after the first frame was accepted and made the 320 wipe draws again (index 149 against 85 after tic 20, confirmed by running) | Fixed (`1bb153e`): version 2 requires the flag set at tic 0 and clear from tic 2, either at tic 1; version 1 unchanged (M5b frozen). Test `a_version_2_wipe_flag_the_original_cannot_hold_is_refused` failed before; the fuzz target gained the invariant. The citation in the brief, d_main.c:188, is :187 |
| M2 (`review-app`) | The layout-dependent label (ruling T12: "counted separately in every report") reached no `doom-play` output | Done (`014d7b9`): one plain line in `doom-play`'s exit lines ([text](../COMPATIBILITY.md#the-two-switch-sounds-decision-t12)); nothing on screen. **The owner confirmed on 2026-11-18 that the exit line is enough**, and the session check now prints the same label ([below](#the-sound-visual-and-the-session-checks-label)). Tests `a_layout_dependent_session_gives_one_plain_line` (the exact text, from a session whose detector fires at tic 1) and `the_exit_lines_hold_no_layout_line_for_an_unlabelled_session`. No committed input makes the detector fire inside a played session, so no real window session has produced the line |
| M3 (`review-app`) | Two sentences of `doom-play --help` lacked the reference's qualifiers | Fixed (`3e8ded6`): the header names "the sound effects of linuxdoom-1.10 built with its in-process sound code (one define removed)", and the face sentence adds "under the declared schedule and build". `sound_options` failed with the old text |
| M4 (`review-app`) | Two test comments claimed more golden comparison than the code makes | Fixed (`5c6003c`, comments only): 45,170 compared, 300 masked, 70 not comparable; `m5c_restore`'s 70 M5SWTCH frames compare the status row only |
| N1 (`review-app`) | On a sound or display refusal the simulation's committed tic is kept and its command stays in the demo (M5b's rule S9 for frame refusals) while sound and display roll back; the genuine engine would have played that tic | **Confirmed by the owner on 2026-11-18**: the reading of S9's "saved up to the last good tic" for sound refusals, and the rule that a future save must never combine a simulation at tic N with sound or display at tic N-1 ([COMPATIBILITY](../COMPATIBILITY.md#the-app-with-sound-and-what-is-presentation)); no code change |
| N2 (`review-app`) | The port runs `S_UpdateSounds` before the status bar's ticker; the C runs it after (d_main.c:392 after `G_Ticker`) | No code change; no value flows between them. Only which refusal is named could differ. Documented ([COMPATIBILITY](../COMPATIBILITY.md#the-display-contract-with-sound-m5c)) |
| N3 (`review-app`) | Nothing tied the display's index to the session's, and the tic's draw count was a `saturating_add` into a `u16` | Fixed (`52a9e11`): `FrameRefusal::SoundIndex` when the two disagree and `SessionRefusal::DrawCount` when the count leaves 16 bits (65,535 exact, 65,536 refused). Tests `the_two_views_of_the_random_index_must_agree` and `the_draw_count_is_exact_or_refused`: compile failure before |
| N4 (`review-app`) | The app's sink stream is compared with the genuine mixer only outside the required tests | **No code change; documented** ([TESTING](../TESTING.md#what-the-sound-gates-do-not-compare)). `m5c_session.py --check` passed for the reviewer: 9 sessions |
| N5 (`review-app`) | (a) The queue's start threshold was one buffer, with a margin near zero; (b) a panic in the device callback is caught by the audio library, which then writes silence with no message; (c) "never waits" was not literally true; (d) a real backend can open a silent sink | (a) and (c) fixed (`95cbe2d`): the threshold is two buffers, 1,024 frames, 92.9 ms (the cap stays four buffers), with the arithmetic in a comment and the wording corrected; with the threshold set back to one buffer 2 audio tests fail. **(b) and (d) were not changed** and stay open (no panic path exists in `Queue::pull`; `--audio-probe` prints the device name). Presentation, unmeasured |
| N6, N7 (`review-app`) | `Witness::Sound` was shown on the end screen as debug text; a comment still said "outside wire 24" | Fixed (`ad13de5`): `SoundRefusal` and `SessionRefusal` implement `Display` and `Error`, and the end screen shows `sound: tic 18, sound call 0: mixer: refused: sfx 1 has no sample byte at its start (i_sound.c:230-247, 591)`; the comment names wire 24 or 25. The test failed before (the text was the debug form) |
| N8 (`review-app`) | Each tic with sound clones the whole `Display` (about 340 KB) and the `Sound` | Bounded; timed as part of the frame step in [step 12](#step-12-timings) (the staging copies are inside the timer) |
| N9 (`review-app`) | `doom-play`'s sound code postdates the last CI run (`216f976`) | The reviewer ran its tests and Clippy on Linux only. A CI run on the final head is **still to come** |

**Corrections to earlier statements that a fix made false** (each corrected in its authoritative home; the earlier text
of this record is corrected inline where it stands):

- the device queue's start threshold was one buffer and is **two** (1,024 frames, 92.9 ms), and the latency "roughly 50 to 100
  ms" is replaced by "at least the 92.9 ms queued at the start" (COMPATIBILITY, ARCHITECTURE, [step 7](#step-7-sound-in-doom-play));
- "never blocks" / "never waits" / "never a wait in the loop" said of the audio sink: the loop never waits for the device to play
  anything, but takes a lock held only for a copy (COMPATIBILITY, ARCHITECTURE, ROADMAP, step 7);
- "every row" of the gates and "all 45,470 digest-compared frames": see the exact lists
  ([TESTING](../TESTING.md#what-the-sound-gates-do-not-compare); [step 5 part 1](#step-5-part-1-the-frame-gate-restore-and-continue-and-the-headless-app-tests): 45,170);
- "every other DSIM40 field keeps its meaning" and "read by `Diagnostic::decode_episode1_v3`": true for a map the truncation does
  not cut, false for one it does until `2d447f3` ([STATE_FORMAT](../STATE_FORMAT.md#what-the-v41-reader-checks));
- the DSIM41 reader "checks `< 16`" for a button slot: it is `< 15` ([STATE_FORMAT](../STATE_FORMAT.md#the-sound-record));
- DSND restore "does not check the map number against the simulation" and trusted the bytes' configuration: it checks the caller's
  configuration ([STATE_FORMAT](../STATE_FORMAT.md#sound-session-state-dsnd-version-1-m5c));
- "In the C the update and the mix come after the frame's draw" (COMPATIBILITY): the update comes before `D_Display`, only the mix
  after it (d_main.c:392-400); the difference from the port is that both run before `ST_Ticker` here;
- "Every report counts layout-dependent sessions separately" was true of the headless examples only; `doom-play` now prints the
  line;
- the test counts of [TESTING](../TESTING.md#m5c-sound-effects-the-genuine-sound-logic-and-mixer) (`doom-sound`, `m5c_app`, `m5c_contract`, `doom-play`, the wire-25 unit tests and self-test) rose with the new tests; TESTING owns them.

**What each reviewer ran.**

| Reviewer | Ran | Not run |
| --- | --- | --- |
| `review-sim` | `cargo test -p doom-sim --no-fail-fast`: 1,374 passed, 0 failed, 2 ignored (one pass is the reviewer's own probe); `wire25_projection --nocapture`: 64 items, 22,976 captures, 785 starts (404 placed), 378 stops, 0 screams replaced, 1 refused run (BOSS1, `LevelExited`, the same under both wires), equal to the committed results; scratch probes for MAJOR 1 and MINOR 3; path instrumentation of a scratch copy (NOTE 5) | the genuine C runners, `m5c_wire25_check.py`, `m5c_t4.py`, `doom-sound`'s gate with a mutated simulation, any IWAD or real run, Miri, fuzzing |
| `review-sound` | `cargo test --locked -p doom-sound -- --nocapture`: every test passed (18.8 s; totals: mixer 65,878 starts of which 4 refused, 490 buffers, 1,440 rows, and 64 sessions with 741 starts, 14,043 buffers, 3,012 rows; session gate 22,770 tics, S 785, T 378, K 723, C 4,311, L 176, R 22,770, 0 detector firings; restore 271 saves, 69,226 tics continued; stream replay 37 of 64 streams to their end; `adjust_streams` 404 rows, 32 on map 8); five probes (M1, N1, M2, N2, N6); one planted fault (the off-by-one of `S_AdjustSoundParams` replaced by a plain subtraction: four test files failed), then restored; `m5c_sounds_table.py --check` passed and `--self-test` printed "2 of 2" | the genuine C runners, the 61 real-map runs, the fuzz targets, the mutation campaign, Clippy, rustdoc, any other OS or the minimum Rust version |
| `review-app` | the same M5b outputs from `master`'s and the head's sources, in separate target directories: all 59 M5b items at screenblocks 10 and 11 (118 runs, 42,352 tics), a running digest of every frame, `P` field, message and the `DDSP` bytes: identical (a first attempt with a shared target directory was discarded and redone); two wipe-flag tests (M1); two planted faults, each caught (the wipe draws before `ST_Ticker`: `m5c_display` failed at tic 0 of SNDDIST, face index 0 against the genuine 1; the mute key forwarded to the game: `session_tests` failed); `cargo test -p doom-play` (debug): 20 passed; clippy of the three crates, all targets, warnings denied: clean; `cargo test --release -p doom-game`: 118 passed; the render tests (`m5c_display` 5, `m5c_contract` 3, `m5b_display` 7, `m5b_restore` 17); `m5c_session.py --check`: 9 sessions pass, address variants with leak detection disabled (not LeakSanitizer coverage) | the windowed app, any audio device, `--audio-probe`, Windows and macOS builds, fuzzing, mutation testing, the local real-IWAD runs |

**What each read.** `review-sim`: fully, the diffs of `sound.rs`, `demon_expanded_map.rs`, `lib.rs`, `snapshot.rs`,
`diagnostic.rs`, `use_lines.rs`, `movers.rs`, `teleport.rs`, `lifecycle.rs`, `projectiles.rs`, `spawning.rs`,
`original_registry.rs`, `player_monsters.rs` and the small files, the wire-25 tests and the four Python tools, and the C it
cites; in part, the thinker, movement, perception, pickup and episode-1 code around the diff, the C harness's `S_StartSound`
wrapper, the gate's `compare_stream` and the job list of `rust.yml`; not at all, the rest of the note, the rest of `doom-sound`, `doom-render` and
`doom-game`, the mixer, and the C harness beyond that wrapper. `review-sound`: fully, every file of `doom-sound` and its
tests, the table tool, `s_sound.c`, `i_sound.c` lines 60-880, the RNG part of `m_random.c`, the sound table, three lump
functions of `w_wad.c`, `p_spec.c:1127-1155`, `p_switch.c:220-256` and section 5 of the note; in part, `sound.rs` of
`doom-sim` (the record types), parts of `doom-render` and `doom-game`, `d_main.c:350-410`, `m_misc.c` and a few goldens;
not at all, the C reference tools, the generators of the goldens and WADs, the fuzz targets, the documents (other than
section 5), and the authors' reports. `review-app`: fully, the render, game and play diffs named above, six test files,
`examples/m5c_session.rs` and the C it names; in part, `m5c_real_rows.rs`, `window.rs`, parts of `d_display.rs`,
`doom-sound`'s `session.rs`, `schedule.rs` and `logic.rs`, the audio library's callback and `ma_context_init`, and
the STATE_FORMAT and COMPATIBILITY text on `DDSP` and T12; not at all, the bodies of `doom-sound`'s logic and mixer, wire 25 in
`doom-sim`, the Python tools (run, not read), the CI workflows and the other documents. **So no reviewer read the pages of
documentation as a whole, the C reference harness beyond one wrapper and one row writer, or the generators.**

**Open points after the review, and the owner's rulings of 2026-11-18** (second set; [note, section 5](../M5C_REFERENCE_INVESTIGATION.md#owner-rulings-on-the-open-points-2026-11-18-second-set)): the rule for a sound refusal is confirmed, the layout label stays in the exit line, and the two-buffer start is accepted as presentation. **Still open:** the audio presentation (the queue's numbers, the silent-after-a-callback-panic behaviour and silent backends such as PulseAudio's dummy output) is unmeasured and unplayed, and the owner will judge lag and crackle by ear. The request that the simulation carry the listener columns of starts without a position was not a question for the owner; those columns stay uncompared (see [Findings](#findings)). **What the review could not observe and this record does not claim:** nobody has heard any
sound, no game audio has reached any device, and nothing has run on Windows or macOS beyond CI builds and tests. The 61
real-map runs and the genuine C comparisons were not repeated after the fixes (the fixes did not touch a golden, a results file
or a fixture: `git diff --name-only 6d02210..HEAD -- tests/reference` lists none, per `fix-sim`).

**Checks run by the fixing agents, at their own worktree heads (the same commits are on the branch under the hashes above; all exit 0 unless said; not the final gates):** `fix-sim` at `e9630fd` (on
a base of `6d02210`): `cargo test -p doom-sim --no-fail-fast` 1,376 passed, 0 failed, 2 ignored, in debug and in release;
`wire25_projection --nocapture`: 3 passed, the same totals as before; `m5c_wire25_check.py --check --selftest --jobs 4`: 64 of
64 match, 22,976 tics, 75 checks; `m5c_t4.py --check` passed (wire 25 matches on 93 tics, wire 24 diverges at tic 0); clippy,
fmt, `cargo +1.85.0 check -p doom-sim` and warnings-denied rustdoc clean; 10 files, +490 and -22, all hand-written.
`fix-app` at `17066e2`: `cargo test -p doom-sound -p doom-render -p doom-game -p doom-play` 420 passed, 0 failed (43, 236, 120,
21), in debug and in release; clippy, fmt (including the fuzz workspace), `cargo +1.85.0 check`, rustdoc, the table tool's
`--check --self-test` (8 of 8), `m5c_session.py --check --selftest --jobs 4` (9 sessions, 8 self-test cases, leak detection
disabled) and the fuzz seed replays of [step 11 part 2](#step-11-part-2-the-fuzz-targets-after-the-review); 29 files, +1,145 and -181,
all hand-written. Not run by either: a workspace-wide test, a sanitizer or fuzz campaign, mutation testing, the 61 real runs,
Windows or macOS builds, any audio device.

## The instruction-count check

**What failed.** At `3c1cc5d` the job `Simulation instruction counts` (`simulation-ir`, `python3 -I tools/bench_advance.py ir
--check`, [TESTING](../TESTING.md#advance-benchmarks-m4p)) failed: case `M4a_corpse_continuation`, `memcpy_calls` 8,407 in the
baseline against 9,207 measured, **+9.52% against the threshold of 3.0%**. `malloc` and
`realloc` counts were equal to the baseline. The agent `fix-ir` reproduced it on 2026-11-18 with CI's own command (Rust 1.85.0,
the release profile, no overrides; `cargo +1.85.0 build -p doom-sim --examples --release --locked`, then `ir --check`): the same
nine results, `threshold 3.0%: FAILED` on that case only. Commits `d1f7b3f` (the status line) and `91b018c` (housekeeping) changed no
code and carried the same failure: at `d1f7b3f` the run failed on this job alone (the other eight jobs passed), and at `91b018c` this
job had failed before the next push cancelled the run's three `Stable` jobs.

**Cause.** A bisect on the one case: `6d02210` 8,407; `2d447f3` 8,407; `591c793` 9,207 (the first bad commit, a review fix of step 13);
`d7104bf` and `3c1cc5d` 9,207; the `world_pickups.rs` hunk of `591c793` alone on `2d447f3` gives 9,207. That hunk (counting
the map's THINGS through a helper) is **not executed by this case** (the preflight returns early for the older Demon profile),
but it changes how rustc 1.85.0 splits `doom-sim` into its 16 release code-generation units (no link-time optimisation): about 20
functions differ in the binary's symbol list. The effect lands on one vector append that runs once per tic (400 tics): in
`demon_expanded_map.rs`, the `None` arm after `commit_demon_plan` appended the closing `Specials` and `Respawn` phases with
`self.events.extend([..; 2])`, which calls an out-of-line `Vec::spec_extend` for `array::IntoIter<Event, 2>`. At `6d02210` that
body copied both 152-byte events with one fused `memcpy`; at `3c1cc5d` the same instance first copies the whole 320-byte iterator
to the stack and then each event: three `memcpy` calls per tic, 1,200 instead of 400, and the 800 extra are the whole
difference. It is a code-generation effect of the pinned toolchain: behaviour is unchanged. `fix-ir` tried seven source shapes
that left the count at 8,807 (two plain pushes, helpers with and without `#[inline(never)]`) or 9,207 (moving or annotating the
unexecuted helper). With 1.85.0 every by-value write of a 152-byte `Event` is one `memcpy` call, and no safe form found makes
the fused copy certain.

**The fix** (code only, two commits, `git log --oneline -2 --grep='Instruction-count fix'`; 28 insertions and 15 deletions in
`crates/doom-sim/src/demon_expanded_map.rs` and `crates/doom-sim/src/transaction_cost_tests.rs`, hand-written):

1. `1b2ecc4`: two `self.events.push(Event::Phase(..))` instead of `extend` over the array, with a comment giving the cause. The
   corpse case alone then reads 8,807 (+4.76%, still over the threshold); the other eight cases equal `3c1cc5d` on every counter.
2. `a8abf0c`: `commit_demon_plan` takes the tic's plan by mutable borrow instead of by value, so the 792-byte `DemonPlan` is no
   longer copied into an argument temporary before each commit (one `memcpy` per tic in `advance_expanded_demons`, plus one per
   explicit checked operation). Each field is taken out of the borrowed plan and assigned exactly as before (the actor stores
   swapped and the replaced store's sector slots kept as spare slots; spatial, specials, writes and events taken, their defaults
   being empty vectors that do not allocate; the sound targets replaced and matched as before; the `Copy` fields copied). Same
   values, same order of every operation, events, RNG draws and allocations. The one difference is when the replaced actor store
   is freed: when the caller's plan goes out of scope, a few statements later (`doom-sim` has no `Drop` implementation, found by
   searching for `impl.*Drop for`, and nothing reads the store).

**Every case before and after** (`ir --check`, 1.85.0, the repository's release profile; `memcpy` calls, then instructions inside
`advance` outside libc as a change against the baseline; baseline, then `3c1cc5d`, then `a8abf0c`; `malloc` and `realloc` equal
the baseline in every case at both heads; `threshold 3.0%: ok` at `a8abf0c`; measured at the commit of the second fix, whose
`crates/` are `a8abf0c`'s):

| Case | `memcpy`: baseline | `3c1cc5d` | `a8abf0c` | Ir: `3c1cc5d` | `a8abf0c` |
| --- | --- | --- | --- | --- | --- |
| `SYNCHAOS_wire22` | 1,763,633 | 1,763,628 | 1,758,628 | -0.20% | -0.17% |
| `SCALE2_episode1v2` | 392,111 | 392,092 | 391,492 | -0.17% | -0.16% |
| `MTELE1_wire24` | 9,113 | 9,113 | 8,953 | +0.31% | +0.56% |
| `TFRAG1_wire24` | 14,947 | 14,944 | 14,662 | +0.25% | +0.50% |
| `BVB1_wire24` | 34,377 | 34,369 | 33,634 | +0.61% | +0.92% |
| `M4a_dense64_lift` | 1,363,680 | 1,363,680 | 1,361,280 | -0.12% | -0.11% |
| `M4a_dense64_fight` | 1,415,616 | 1,415,538 | 1,411,938 | -0.23% | -0.20% |
| `M4a_gallery_patrol` | 258,136 | 258,130 | 254,530 | +0.18% | +0.34% |
| `M4a_corpse_continuation` | 8,407 | **9,207** | 8,407 | -0.11% | +0.23% |

**What was not changed.** The baseline `tests/reference/m4p_ir_baseline.json`, the threshold (3.0%), the nine cases and the tool
`tools/bench_advance.py`. **What is still true after the fix:** `memcpy` is lower than at `3c1cc5d` in every case, but the
instruction counts are **0.01 to 0.34 percentage points of the baseline higher than at `3c1cc5d` in all nine cases** (the
largest rise, the corpse case, is +11,200 instructions over 400 tics, about 28 per tic), all inside the threshold. The likely
sources are the empty defaults that `mem::take` writes back into the borrowed plan, the swap of the actor store and code-generation
shifts; counts move this much between unrelated builds (the build of `591c793` alone was +1.2% on the corpse case). Meeting "no
counter worse" for instructions as well would mean searching more source shapes, which is again code-generation dependent.

**An alternative, not taken.** The failure came from an unexecuted change shifting code generation, not from a
behaviour change; rewriting the baseline from a pinned build instead of changing code was an option. It was not taken: the brief forbade
changing the baseline, threshold, cases or tool. The owner accepted the code change on 2026-11-18 (ruling 7, below).

**The one changed line of an existing test (owner's ruling 7, 2026-11-18).** The second fix changed one line of an existing test, in
`crates/doom-sim/src/transaction_cost_tests.rs` (line 133, the call site only, in `plans_stage_sector_writes_in_a_lazy_copy`):
`let _ = sim.commit_demon_plan(plan);` became `let _ = sim.commit_demon_plan(&mut plan);`, because the function now takes the tic's plan
by mutable borrow (`git show a8abf0c -- crates/doom-sim/src/transaction_cost_tests.rs`: 1 insertion, 1 deletion). **No assertion changed.**
The test, read on this branch, asserts after that call that sector 0 holds the moved value and the other sectors are unchanged, that
the spare write buffer is empty with a capacity of at least the table's size and no line slots, that the next plan starts empty, that
it stages a write without growing the buffer's capacity, and, after a refused plan, that the committed table and the simulation's
snapshot are intact. The owner accepted the fix **on condition that every frozen wire's goldens pass in CI on that head.** The
`Stable` jobs on ubuntu-latest, windows-latest and macos-latest run every frozen wire's goldens, and all three were green at
`a8abf0c` (the observation of 2026-11-18 in [the CI record](#the-ci-record)), so the condition is met at that head.

**Checks of the fix** (`fix-ir`, at the second commit, `CARGO_BUILD_JOBS=2`): `cargo test -p doom-sim` in debug and in release:
1,376 passed, 0 failed, 2 ignored each; `tests/wire25_projection`: 3 passed; the Rust steps of the `Stable` job as CI runs them:
the six M3h9c scripts and `check_player_level.py` (all checks passed), the M4a episode-1 check (all passed) and the M4a and M4b
wire-24 check (52 of 52 scripts matched over 20,205 tics, 0 pending); `cargo clippy -p doom-sim --all-targets -- -D warnings`,
`cargo fmt --all -- --check` (workspace and fuzz), `cargo +1.85.0 check -p doom-sim`, `RUSTDOCFLAGS=-D warnings cargo doc -p
doom-sim --no-deps` and `cargo test -p doom-sound --test session_gate` (1 passed): all clean. **Not run:** the genuine-engine C
steps of the `Stable` job (they do not build or run Rust), the whole-workspace tests and Clippy, the fuzz builds, sanitizers,
Windows and macOS. CI on `a8abf0c`: the job `Simulation instruction counts` passed (the whole run is in [the CI record](#the-ci-record)).

## The sound visual and the session check's label

**The visual** (owner's ruling 5 of 2026-11-18, under the visuals rule; the figures are from the author's report and the committed
files). `tools/media/m5c_readme_visual.py` draws the waveforms of the genuine mixer (linuxdoom-1.10 built with its in-process
sound code, one define removed; the strict build of `m5c_mixer_build.py`), of the Rust mixer (the release example `m5c_mixer_raw`)
and of their difference, for one item, **`SNDDIST`**: it starts 14 sounds of four effects in 1.393 s, up to three channels at once,
at separations 33, 61, 128, 129 and 224 and volumes 1 to 8. The other items either start one sound, stay centred or run much
longer. All sessions use volumes of 8 or less of 127, so every stream is quiet (the genuine peak is 2,224 of 32,767). The sounds are
our own generated test sounds (`m5c_sfx.wad`), no IWAD. **Source equivalence against the genuine stream, sample by sample, under
the declared schedule:** 15,360 frames, 30,720 samples, **0 differing**. The tool refuses to draw unless the genuine rows equal the
committed golden `tests/reference/m5c_mixer/sessions/goldens/SNDDIST.golden` byte for byte (the report found that file's and the
start list's digests equal to `m5c_mixer_sessions_results.json`, and the raw stream's digest equal to the golden's `z` row, 30 buffers);
the Rust example's stream of the same list has the same digest. Self-test (`--selftest`): one changed Rust sample is caught as 1
differing; a Rust stream one frame short is caught; one changed genuine row is refused; one changed genuine raw byte is refused.
`--check` fails, naming the file, when one byte is appended to the SVG or to the WAV, and writes nothing in that mode.

**Files** (generated, in `docs/media`): `m5c-mixer-waveforms.svg`, 76,450 bytes, 960 by 522, three panels (genuine, Rust, Rust
minus genuine), left and right lanes, one vertical scale, a mark for each of the 14 starts; `m5c-mixer-session.wav`, 184,364 bytes,
4.180 s, 11,025 Hz, stereo, 16-bit PCM, three segments of 15,360 frames in this order and without gaps: the genuine stream (from
frame 0), the Rust stream (from frame 15,360) and the difference, Rust minus genuine, which is all zero (from frame 30,720). The
tool's hand-written part is 432 lines and the example's 189 (`git diff --stat` of the author's work); the CI job `M5c genuine
sound (not required)` runs `--check --selftest`. The README links the picture, not the WAV (the owner will listen first and
give a link).

**What was not done or seen.** **Nobody has listened to the WAV, and nobody has heard any sound.** **The SVG could not be rendered on
the development machine** (no rasterizer): the author viewed the tool's own raster of the same geometry instead, so how a browser
draws the SVG is unobserved. **No MP4 was written**, because ffmpeg is not installed and nothing was installed for it (the tool
printed "ffmpeg not found: no MP4 written" and exited 0), **so the MP4 code path has never run.**

**The label in the session check** (owner's ruling 3). `tools/reference/m5c_session.py` now prints the layout-dependent label beside
each part's verdict, as `tools/local/m5c_session_real.py` already did. Run as `--check --selftest --jobs 2` with leak detection
disabled, exit 0, the results file byte-identical before and after, 8 of 8 self-test cases passed: **all ten parts (nine
sessions, `restart` having two) print "not layout-dependent"**; the detector fired 0 times. `m5c_session_real.py` was read, not run
(it needs an IWAD).

## The CI record

At `216f976` every required job was green (the macOS job took 42 minutes of its 60-minute limit) and the non-required job
`M5c genuine sound (not required)` **failed at its T4 check** (`m5c_t4.py --check`: "differs or missing") for two generated files
that had never been committed: `tests/reference/m5c_t4/truncate.wad` and `TRUNC.lmp`. The cause: `.gitignore` excludes `*.wad`
and `*.lmp` (the rule that keeps commercial IWADs out), so a plain `git add` skipped them without a message, and the check
passed only where the files already lay. **Fixed in `a6f3b3b`** by adding both files explicitly (86,537 bytes and 382 bytes,
our own synthetic data, no IWAD content); a plain run of `m5c_t4.py` then regenerated them and left `TRUNC.txt` and the results
unchanged, and `--check` passed. The step that runs `m5c_session.py --check --selftest` was added to the same job
(`dad83eb`) and run only locally in the job's form (exit 0, 88 s). CI results for later heads are on GitHub; the run after
`a6f3b3b` had not been read when this was written.

At `3c1cc5d` the job **`Simulation instruction counts` failed** (case `M4a_corpse_continuation`, `memcpy_calls` +9.52% against
3.0%); cause, fix and counts are in [the instruction-count check](#the-instruction-count-check). `d1f7b3f` (the status line) and
`91b018c` (housekeeping: vendored-source attributes, the commit-subject rule, the README's 237 lines cut to 149 with its policy table
moved to [COMPATIBILITY](../COMPATIBILITY.md#policies)) changed no code and carried the same failure: the run on `d1f7b3f` failed on that job alone, and the run on `91b018c` was cancelled
by the next push after that job had failed. **`a8abf0c` is the head with the fix** (`1b2ecc4` and `a8abf0c`). Its run, as observed by the orchestrator on 2026-11-18,
was **green on all nine jobs**: `Detect changes`; `Stable` on ubuntu-latest, windows-latest and macos-latest (the macOS job took
39 minutes); `Minimum Rust / 1.85.0`; `M5a genuine renderer`; `M5b genuine status bar, HUD and input`; `Simulation instruction counts`;
and `M5c genuine sound (not required)`. Because the `Stable` jobs run every frozen wire's goldens, the owner's condition on the
test-line change is met at that head. **The macOS job's limit is now 90 minutes** (`4989fcc`, from 60; the job took 38 to 50
minutes on the M5c heads, and ROADMAP records the follow-up to split or trim it). **The Mutants workflow's runs on `a8abf0c` and `937cf9d`
are in [step 10](#step-10-mutation-testing).** The orchestrator observed on 2026-11-18 that CI was **green on all nine jobs at
`0dea135` and at `937cf9d`**, `937cf9d` being the head that carries the final code and tests (the macOS job took 37 minutes there).
`c976db4` (documents only) and `abc0ece` (`AGENTS.md` and the agent definition files: the Codex model routing now mirrors Claude's,
with efforts on both; no code) follow it; **their runs were not read for this record.** CI on the heads after that (this pass) has
not been read either.

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
- **Layout-dependent sessions: 0.** The T12 detector fired 0 times over the 64 synthetic items, the 9 scripted app
  sessions and the 61 real runs (4 release sounds, none with a busy next slot); the genuine 32-bit build's sound streams equal
  the 64-bit ones on all 61 ([step 5 part 2](#step-5-part-2-the-61-real-runs-through-the-whole-pipeline-local-only)). Such
  sessions are counted separately in every report.
- **381 of 785 start rows lack the listener columns in Rust.** For starts with a NULL origin or the player's own body
  the genuine row writes the listener's position and angle, which the wire-25 record does not carry; those three columns
  are compared nowhere (neither in step 3b's comparison, nor in the end-to-end gate, nor in the later steps), and the original
  does not read them there. The same holds for 5,980 starts of the 61 real runs (26,077 less 20,097) and 43 of the 142 starts
  of the 9 sessions; a request to the simulation to carry them is open.
- **A simulation tic is not rolled back when sound or display refuses.** See step 5 part 1; **confirmed by the owner on
  2026-11-18** ([step 13](#step-13-independent-review)), with the rule that a future save never combines a simulation at tic N with sound or display at tic N-1.
- **The independent review found one MAJOR (the DSIM41 reader and truncated maps), 11 MINOR and 19 NOTE findings, all agreed and
  fixed or documented; no BLOCKER.** The list, with the commits, is in [step 13](#step-13-independent-review).
- **Gates compare less than "every row".** The exact exclusions (header, `W` and `Z` rows, the listener columns of unplaced
  starts, the device rows, four items shorter than their demo, 300 frames on the masked digest only, 70 not comparable) are in
  [TESTING](../TESTING.md#what-the-sound-gates-do-not-compare).
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
- **A slip of the orchestrator, recorded.** The tester's worktree (`~/scratch/m5c/wt-step10`) was removed before the tester
  had handed back, while its first full debug test run was going; that run failed (25 failed lines in `doom-sim`'s teleport
  tests) and was discarded, and the agent recreated the worktree at its last commit and ran every check again from scratch. Its
  three commits and its results were not affected (the commits reached the branch by cherry-pick, so the hashes in the agent's
  report, `9ca0cf2`, `5a62716` and `b2d7806`, differ from the branch's `de34a59`, `ce94184` and `e2207ff`).
- **The survivors' dispositions: no source change was needed and the equivalent mutants stay.** 60 of the 74 survivors were
  killed by 24 new tests, 12 shown equivalent and 2 unreachable; the independent check agreed with every argument and re-ran 15
  kills. The lesson for the next run: cargo-mutants judges a `doom-sim` mutant by `doom-sim`'s own tests only
  ([step 10](#step-10-mutation-testing)).
- **A review fix changed instruction counts in code it never runs.** `591c793` made one case of the instruction-count check fail
  by +9.52% through code generation alone; the code-only fix leaves instruction counts 0.01 to 0.34 percentage points above
  `3c1cc5d` in all nine cases, inside the threshold, and the owner accepted the code change on 2026-11-18
  ([the check](#the-instruction-count-check)).
- **The README no longer holds the policy table.** The housekeeping commit moved it to
  [COMPATIBILITY](../COMPATIBILITY.md#policies); this record and the other pages that name it link there (the documents were
  searched for a remaining statement that the table is in the README: none was found).
- **A documentation slip found on the way.** The M5b section of TESTING listed a `--strict32` option for
  `m5b_reference.py` that the tool does not have; it was corrected on 2026-11-18.

- **Clarified non-finding: the DSIM reader refuses the frame of a barrel forced into its explosion state.** While writing the
  second round's tests the `step10b` agent saw the reader refuse (`Identity`) the genuine frame of a barrel put into state 809 by the
  crate-private `enter_demon_state`, and suspected a reader and writer gap. The follow-up agent traced it (a temporary experiment,
  restored afterwards): the refusal comes from `Diagnostic::validate_barrels` (`diagnostic.rs:1846`, the field witness ending at
  `:1946`; one failing condition is `(health > 0) != alive` at `:1921`). The explicit entry leaves a live barrel's body (health 20,
  the live flags, full height 42) in a dying state, while in the C a barrel reaches the death chain only through `P_DamageMobj`
  and `P_KillMobj` (`p_inter.c:887-890`, `:675`, `:680-681`, `:725`), which change health, flags and height first; the witness
  documents exactly that (STATE_FORMAT, DSIM v37). It is a **rule of the frozen readers since DSIM37** (commit `ac886d4`, 2026-11-04,
  M3h9b, merged by PR #55; the simulation's mirror `barrels::validate_barrel` dates from `42e9c79`, wire 21): the same frame is refused
  under wire 24 too, DSIM41 being read internally as version 40. It is **reachable only through a crate-private test call**, not in a
  demo run, and **has nothing to do with sound**: the simulation's own next tic refuses the same frame (`PlayerBody(Invariant)`, state
  unchanged). **Nothing was changed**, and no frozen wire was touched. Whether `enter_demon_state` should refuse a barrel's death-chain
  entry up front is an owner's decision, not asked for. One doc-comment sentence (the test `an_explicit_operations_sound_is_kept_as_a_pending_record`
  in `wire25_tests.rs` says the next tic's preflight publishes the records) describes the rule, not something that
  test's frame can reach; it was not edited.

## What a human still has to judge

The owner will judge by ear, when he plays, the lag and crackle of the device queue (its two-buffer start was accepted on 2026-11-18 as
presentation, unmeasured) and the start-up pause that a game under WSLg occasionally shows (it stays as it is).

Nobody has heard any sound from this project: every audio trial wrote digital silence, the one real device open (the probe
under WSLg) wrote silence only, and no game audio has reached any device. Nothing has run on Windows or macOS beyond CI
builds and tests. The evidence above compares which sounds start, on which channel, with which volume, separation and
pitch, and the 16-bit samples the original mixer writes for them; it cannot say how any of it sounds, whether the
synthetic or the real samples are pleasant, or whether a backend will play them without gaps. The owner has also
not playtested M5b, so input, pacing and presentation are not to change on a guess about feel.

## Still to come in this record

The final
gates from a clean clone (running at `937cf9d` when this was written; build, format, Clippy, workspace tests, release tests, rustdoc, the original-C generators and
encoders with `--check`, the fuzz builds, Rust 1.85.0, and a bounded sanitizer campaign) the pull request's new body, the Windows artifact, and the CI record of the final head, which
also covers `doom-play`'s sound code on Windows and macOS (not observed since `216f976`); a repeat of the fuzz campaigns on the
changed targets (only seed replays without a sanitizer have been run since the review); the optional melt (not built). The
optional MP4 of the sound visual is not written (ffmpeg is not installed).
