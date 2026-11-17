# M5b evidence record

Observed results of the M5b milestone (a limited playable build: a window, live keyboard and mouse input, the
35 tics-a-second loop, the status bar, HUD messages and palette flashes, on the wire-24 simulation and the M5a
renderer). Only observations appear here. The [M5b investigation](../M5B_REFERENCE_INVESTIGATION.md) owns the
source facts, the decisions S1-S23 (confirmed by the owner on 2026-11-17) and the plan;
[testing](../TESTING.md#m5b-playable-status-bar-hud-input-and-the-app) owns the commands and
[COMPATIBILITY](../COMPATIBILITY.md#playable-build-m5b) owns the support statement. The status-bar face, the
render state and the `Display` state are saved as described in
[STATE_FORMAT](../STATE_FORMAT.md#display-state-ddsp-version-1-m5b). Exact review state and CI belong to GitHub.

This is a first pass, written on 2026-11-17 (America/Los_Angeles) while Phase 2 was under way. Where a step's
section is missing, that step had not been run or its result had not been written up on that date.

**Wording rule.** A frame is described as identical to the genuine engine under the declared sound-free,
wipe-free contract (decision S15), never as identical to a vanilla session without that qualifier.

**Scope of the record.**

| Step | Date | State on 2026-11-17 |
| --- | --- | --- |
| 1 | 2026-11-17 | Recorded: synthetic graphics, eleven new rooms and the genuine full-frame runner ([below](#step-1-the-genuine-full-frame-runner-and-its-goldens)) |
| 2 | 2026-11-17 | Recorded: the genuine input driver and its scripts ([below](#step-2-the-genuine-input-driver)) |
| 3 | 2026-11-17 | Recorded: the status bar, HUD, palette choice and `Display` in `doom-render` ([below](#step-3-the-rust-status-bar-hud-and-display)) |
| 4 | 2026-11-17 | Recorded: the `doom-game` input core and event layer ([below](#step-4-the-rust-input-core-and-event-layer)) |
| 5 | not yet recorded | Real maps through the app's frames: in progress on 2026-11-17, not written up |
| 6 | 2026-11-17 | Recorded: the app core and the `doom-play` window ([below](#step-6-the-app-core-and-the-window)) |
| 7 | 2026-11-17 | Recorded: session verification ([below](#step-7-session-verification)) |
| 7b | not yet recorded | The unrecorded full-resolution turning mode (S3 (ii)): in progress, not written up |
| 8 | 2026-11-17 | Recorded: the first play check ([below](#step-8-the-first-play-check)) |
| 9 | 2026-11-17 | Recorded: CI ([below](#step-9-ci)) |
| 10 | 2026-11-17 | Documentation: this first pass; a second pass adds the results of the steps still to come and the new animation |
| 11 | not yet run | Mutation testing of `doom-game` (the scope is in place, [below](#step-9-ci)) |
| 12 | first half recorded | The input fuzz target ([below](#step-12-fuzz-target-for-the-input-code-first-half)); the second target is in progress |
| 13 | not yet run | Timings |
| 14 | not yet run | Independent review |

No IWAD was read in steps 1 to 4, 6, 7 (except the one local real-map run named there) and 9. Step 8 and the
local real-map run of step 7 read a scratch copy of the IWAD outside the repository; only counts, pass/fail
facts and digests are recorded. Address-checked runs used `ASAN_OPTIONS=detect_leaks=0`: LeakSanitizer was
disabled, not passed. Test totals below were reported by the step that ran them, on that step's tree; they are
not re-run on a later head, and CI on the current head is in GitHub, not here.

## Step 1: the genuine full-frame runner and its goldens

What was run: `python3 tools/reference/m5b_reference.py --work DIR --check --pngs --selftest --jobs 6` (a full
`--check` took 4 minutes 18 seconds at 6 jobs). `m5b_gfx.py --check`, `m5b_check_gfx.py`, `m5b_wad.py --check`
and `m5b_build.py --check` passed, as did the M5a tools' `--check`.

- `tools/reference/m5b_gfx.py` writes `tests/reference/m5b_gfx.wad` (1,048 lumps, 1.3 MB): the M5a set unchanged
  (every M5a lump keeps its bytes and number) plus our own status bar, face and HUD font lumps. No id art.
- `tools/reference/m5b_wad.py` writes 11 new rooms (STDIG, STARM, EVIL, OUCH, LOOK, FACED, PALRD, PALBN,
  PALST, MSG and the bespoke scenario CHEAT) in `tests/reference/m5b_wads` (11 WADs) with 22 files in
  `tests/reference/m5b_scripts`; the new rooms total 7,785 tics.
- `tools/reference/m5b_build.py` builds the runner (the M5a runner plus the genuine status bar and HUD
  compiled whole, in the strict, overflow and address variants). Goldens: `tests/reference/m5b_frames`,
  118 files, 5.1 MB, schema `DOOM-M5B-FRAMES-v1`.
- 59 items: the 26 M5a rooms, the 22 M4b scripts and the 11 new rooms, each at screenblocks 10 and 11, every
  tic: 21,176 frames per configuration.

**Proofs (all observed):**

- The strict, overflow and address streams are identical on 59 of 59 items at each configuration, and two
  strict runs are identical on 59 of 59. M5a's normalisation applies: tracker-flagged frames are compared on
  their masked digest, and nothing from a limit witness on is compared (M5OPEN, M5SSEG).
- Masked never-render game dumps equal the committed digests on 48 of 48 existing items (22 M4b, 26 M5a) and
  equal the frozen M4b runner's on 10 of 10 new wire-24 rooms. The bespoke scenario's dumps are recorded only.
  Render-every dumps equal never-render dumps snapshot for snapshot everywhere. The two fields that change
  when the genuine status bar and HUD run (the `M_Random` index and the message on the tic it is taken) are
  recorded and masked in this runner only, and compared separately (decision S16).
- Message-free 3D views equal the M5a goldens on 11,791 frames per configuration, 0 differ; 1,600 were skipped
  per configuration (1,450 with a message showing, 150 tracker-flagged).
- Sanitizers: `ASAN_OPTIONS=detect_leaks=0`, `UBSAN_OPTIONS=print_stacktrace=1`; leak detection was off, so
  this is not LeakSanitizer coverage. The only reports are M5a's known address-build stops in
  `R_StoreWallRange` (M5OPEN) and `R_ClipSolidWallSegment` (M5SSEG). None in the status bar, HUD or
  `v_video.c` code. The 2D tracker counted 0 undefined background reads, 0 posts outside a lump, 0 writes
  below the screen and 0 skipped patches. `M_Random` was never drawn outside `ST_Ticker`, and no palette
  pointer fell outside PLAYPAL.

**Coverage under wire 24:** palettes 12 of 14 (0, 2-8, 10-13; palettes 1 and 9 are never chosen by the
source); faces 40 of 42 (37 and 40 are not reachable), 41 of 42 with the bespoke scenario (the god face);
messages 29 kinds, every reachable one (code 6 is unreachable); widgets: every number and icon changed except
the plasma and BFG arms icons, the skull-key icons 3-5 and the minus sign, none of which Episode 1 can show.

**Self-test, six planted faults, each found at the planted place:** a status bar pixel at tic 20; a palette
number at tic 25; an `M_Random` index at tic 30; the turbo message recorded one tic late; `--no-mask-rnd` (the
dump set no longer equals the frozen digest, first difference at snapshot 0); `--no-hu-record` (first
difference at snapshot 59, the first message).

**Corrections to the Phase 1 note** (in its [section 2.7](../M5B_REFERENCE_INVESTIGATION.md#27-corrections-found-in-phase-2)):
40 of 42 faces, message code 6, the radiation-suit bit, the 4-pixel space and the 7,785-tic total.

**Not run:** `--strict32`, any fuzz campaign, real maps (no IWAD read). The step 7b command-injection mode has
only its hook in the driver. A scratch Rust check loaded all 11 new rooms under wire 24 with no refusal; it did
not compare Rust state with the C dumps.

## Step 2: the genuine input driver

What was run: `python3 tools/reference/m5b_input_reference.py --work DIR --check --selftest` (jobs 4), plus the
`m5b_input_scripts.py --check` and `m5b_input_build.py --check` generators. The driver compiles 23 genuine
linuxdoom-1.10 files whole, with linuxdoom's X translation (`i_video.c`) against a declared fake Xlib. No WAD is
read.

- 251 scripts, 36,453 tics (`demo_limit` alone is 32,761 tics; the other 250 scripts have 3,692). 2 witness
  scripts, which are not goldens. 175 app-domain scripts with 35,861 tics. 1,343 built turns have a nonzero low
  byte. Generated data is 546,060 bytes.
- The overflow and address builds are byte-identical to strict on 251 of 251 goldens and demo files; the
  opt-in `--strict32` build (`gcc -m32`) is also identical on 251 of 251, with the genuine text and no adapter.
- Sanitizers: `ASAN_OPTIONS=detect_leaks=0`, `UBSAN_OPTIONS=print_stacktrace=1`; leak detection disabled. No
  finding in the 251 scripts. Witness `ev_mouse_int_overflow` exits 0 in strict and the overflow build stops at
  `g_game.c:579` (signed integer overflow); `ev_mouse_int_edge` passes in both.
- Chain: `chain_walk` records a 254-byte demo, which the genuine M5a runner replays (60 tics, end by marker):
  all 60 replayed commands equal the golden's executed commands. Every `t` row's demo bytes equal the encoding
  of the built command and decode to the executed one.
- Wire-24 check (`--summary`): 10 rows fall outside, all `BT_SPECIAL` (the pause scripts and one key-code
  script); the app domain has 0 such rows (buttons 0-60, forward and side -50 to 50, turn +-14,288 built and
  +-14,336 executed).

**Self-test, four planted faults, each found:** a flipped command bit (tic 3), a dropped key-up (tic 4), a
changed binding (56 of 251 scripts differ) and a changed turn-table entry (51 of 251 differ).

**Findings beyond the Phase 1 note** (in its section 2.7): typing `iddqd` one key per tic while recording ends
the demo at tic 4; a turn at the positive edge of a `short` executes as -32768 after the demo re-read; a second
press lasting 2 tics after a short gap gives no double-click use; `G_DoLoadLevel` leaves the double-click state.

**Not run:** the historical `run_reference.py` suite, `m5a_reference.py --check`, a gnu99/ubsan diagnostic
build, any real X server, CI.

## Step 3: the Rust status bar, HUD and `Display`

What was run: `cargo test -p doom-render --test m5b_display --test m5b_restore` and the gates below. About
1,770 hand-written source lines and 1,250 test lines (`v_video`, `st_lib`, `st_stuff`, `hu_lib`, `hu_stuff`,
`d_display`). `doom-sim` is unchanged: no new wire, no DSIM change, no digest change.

- **Golden test (`tests/m5b_display.rs`):** all 59 items at screenblocks 10 and 11, every tic: 42,352 frames
  (2 x 21,176). 41,982 compared on the whole-frame digest; 300 on the masked digest where the C tracker flagged
  columns (M5MSKPAD and M5SHORT; the Rust flagged columns must also equal the genuine ones); 70 not
  comparable (M5SWTCH's parity-unknown frames, as in M5a). Also compared on every frame: the renderer counters,
  `fuzzpos` and the newly mapped lines. 211,760 status fields (palette, palette set, face, message flag,
  `M_Random` index) and all 190 message rows match. The 6 limit endings are refused on the next tic with the
  matching witness. No item or row is excluded. About 55 seconds in debug.
- **M5CHEAT:** the cheats are applied to the status bar inputs through `observe_with`; all 900 frames per size
  match.
- **The test can fail:** two planted mutations were each caught (radiation bit `&8` to `&16`; HUD space
  advance 4 to 5).
- **Unit and refusal tests:** 12 new unit tests in the crate and 4 new integration tests in
  `tests/m5b_restore.rs`: restore and continue on five rooms at both sizes after an encode and decode round
  trip equals the uninterrupted run and the goldens; each witness (message underivable, attacker removed,
  message not taken, an observe gap, undefined background, before first tic) is produced from a constructed
  state and leaves the encoded state unchanged; restore and decode refuse bad states.
- **Gates at the step 3 head:** build, both fmt forms, Clippy with warnings denied, warnings-denied rustdoc,
  `cargo test --workspace --locked` (1,600 passed, 0 failed, 4 ignored), `cargo test -p doom-render --release`
  (161 passed) and `cargo +1.85.0 test -p doom-render` (161 passed) passed. After cherry-picking the later
  commits the full workspace test, release, rustdoc and MSRV gates were not re-run by that step.

Two consequences of the C, reproduced and tested: at a level start `st_oldhealth` is -1, so the first tic with
damage shows the ouch face whenever health is 20 or more; after switching from screenblocks 11 to 10, icons
that were never drawn have no old box to erase (two pixels remain in M5STARM).

**Not run:** any C or generator `--check`, fuzz build, sanitizer campaign, mutation testing, IWAD or real map.

## Step 4: the Rust input core and event layer

What was run: `cargo test -p doom-game` and the gates below. 1,407 source lines and 1,860 test lines in
`crates/doom-game` (no third-party dependency).

- **Golden test:** all 175 app-domain scripts, 35,861 tics, 656,396 compared fields (18 per tic: the built
  command at full 16-bit turn, the executed command, demo bytes, `turnheld`, both double-click triples, the
  mouse-button mask, the held keys), the `e` rows, the `q` rows (queue losses reproduced), `L`, `s`, `z` and
  `x`. `beyond_app_mouse_short_edge` is exact (13 tics, 297 fields). The `x_` and `s5_` scripts at the event
  level: 60 of 75 exact (463 tics, 8,334 fields).
- **Refused or excluded scripts (the test asserts each fails in exactly this way):** refused as `WithheldKey`:
  the Pause-key scripts, `q`, the cheat-typing scripts and the F-key scripts (14 scripts); `s5_cheat_idclev12`
  differs at tic 8 (cheats are not ported). The witness `ev_mouse_int_overflow` is refused as `MouseOverflow`.
- **Wire-24 admission:** every script command (36,208 commands, 53 refused events skipped), 60,000 random core
  commands (19,297 refused events, each shown to leave state unchanged) and 60,000 random platform-through-layer
  commands (never refused) are checked against a restated wire-24 predicate and against the simulation's demo
  writer. The proof that the build is infallible is in the doc comment of `doom_game::input`.
- **Gates:** build, both fmt forms, Clippy, `cargo test --workspace --locked` (1,579 passed, 0 failed, 4
  ignored), `cargo test -p doom-game --release` (38 passed), warnings-denied rustdoc, `cargo +1.85.0 test -p
  doom-game` (38 passed), and `m5b_input_reference.py --check` passed.

Decisions of this step, now documented in COMPATIBILITY: `load_level` clears only `mousebuttons[0]`; the same-pump
deferral applies to mouse buttons; a mouse event is posted only when there is motion or a button change;
`release_all` keeps always-run's speed key; at most 63 events per build and 1,024 pending raw inputs.

**Not run:** fuzz (added in step 12), mutation testing, sanitizers, `--strict32`, CI.

## Step 6: the app core and the window

What was run: `cargo test -p doom-game -p doom-play` and the gates below. `doom-game` app: 1,258 source lines
and 725 test lines (20 headless tests); `doom-play`: 655 source lines and 155 test lines (5 unit tests).

- **New dependency:** miniquad `=0.4.11`, pinned in `Cargo.lock`. The lock gains 7 packages (miniquad,
  malloc_buf, ndk-sys (Android only), objc-rs, winapi and its two gnu helper crates); `cargo tree -p doom-play
  -e normal` on Linux shows 13 unique lines (11 workspace crates, libc and miniquad). Every licence offers MIT,
  compatible with GPL-2.0-only. `doom-play` compiled first time with no system packages and builds with
  `cargo +1.85.0`.
- **Headless scenarios (a fake clock, scripted events, synthetic rooms):** determinism (two runs give the same
  frames, palettes, state digests and demo bytes); the saved demo replays through a plain `Simulation` to the
  app's digest at every tic; slow frames (5 due run 5; 6 due run 5 and drop 1; 30 late run 5 and drop 25);
  no drift (350 tics in 10 seconds of 7 ms updates, at most 1 per update); pause (tics frozen, input ignored,
  the game frame untouched, the tic after resume carries forward 0 and buttons 0); level end (BOSS1's script
  reaches the exit screen, the 545-tic demo equals the committed `BOSS1.lmp` commands); refusal (SPECH9 is
  refused at tic 75 and the demo equals `SPECH9.lmp`); death; restart (the old session is saved, the new one has
  a new name and replays identically to a fresh app); the tic cap (a test cap of 12; the constant is pinned at
  1,048,576 and a recorder test pushes exactly that many tics and refuses the next); saving (rewritten at
  exactly 1,050 tics, a failed write reported, name collisions get `-2`, `-3`).
- **Commands cannot be altered after recording:** a `RecordedTic` whose executed command differs from the
  re-read of its bytes is refused and nothing is stored, as are side -128, buttons 0x80 or 0x40, and weapon
  bits without the change bit; a 400-tic pseudo-random stream of every input kind gave only admitted commands,
  each replaying to the same digest.
- **Window trial under WSLg** (release binary from ext4, synthetic room, `--quit-after-tics 105`): X11 exit 0
  after 3.65 s, Wayland exit 0 after 3.43 s, both saved the demo (105 tics, 434 bytes). The first Wayland try
  panicked inside miniquad (`XOpenDisplay() failed!`); the backend choice now follows the environment (X11 when
  `DISPLAY` is set, otherwise Wayland with X11 fallback). A run with options (fullscreen, screenblocks 11,
  square aspect, gamma 2, modern bindings) exited 0 and its demo replayed through `player_level_run`
  (70 admitted, none refused).
- **Gates:** build, both fmt forms, Clippy, rustdoc with warnings denied, `cargo test --workspace` (1,604
  passed, 0 failed, 4 ignored), `cargo test -p doom-game --release` (58 passed), `cargo +1.85.0 test -p
  doom-game` (58 passed) and `cargo +1.85.0 build -p doom-play`.

**Not observed:** any picture (no screenshot tool on the machine), fullscreen, scaling, colours, a key press, a
mouse movement or mouse capture (the trial sessions were idle); nothing on Windows or macOS beyond CI builds.
**Not run:** the C reference suite, DSIM encoders, fuzz, sanitizers, mutation testing, timings.

## Step 7: session verification

What was run: `python3 tools/reference/m5b_session.py --check --selftest` (`--work DIR`). The headless app
(`crates/doom-game/examples/m5b_session.rs`, a release build) plays four scripted sessions of timestamped
platform events on a fake clock through the app's event layer, input path, recorder, simulation and `Display`.
The app's posted events become a script for the genuine input driver, whose recorded demo must equal the
app's byte for byte (header included). The app's demo is replayed by the genuine M5b runner in the strict,
overflow and address variants with every frame rendered; on every tic the executed command, the frame digest,
the status fields, the messages and the masked game dump (by the independent DSIM40 encoder) are compared.
The scripts were written by hand, not played by a person.

| Session | Room, screenblocks | Tics | Shows | Genuine end |
| --- | --- | --- | --- | --- |
| amp_walk | amp, 10 | 75 | keyboard and mouse turns, fire, a pickup with palette 10, an Esc pause | marker |
| palbonus_pickups | palbonus, 11 | 145 | four pickups (palettes 10-12), mouse turns dropped at demo resolution, a pause on focus loss | marker |
| palred_damage | palred, 10 | 155 | soulsphere, barrel damage with red palettes 2-8, faces 0, 2 and 7, a pause | marker |
| exits_secret | m4a exits, 10 | 61 | the secret-exit walk made with mouse and keyboard | exit |

Totals: 436 tics, 436 commands, 436 frames (436 whole-frame digests, 0 masked, 0 non-comparable), 2,180 status
fields, 6 messages and 440 game-dump snapshots matched; the three variants were identical and the input
driver's demo equalled the app's in every session. No session reaches a refusal, a death or the tic cap, so
the masked-frame branch of the comparator did not run on a session.

**Self-test, four planted faults, each found at its place:** a demo command byte at tic 22, the app's frame row
at tic 22, tic 22 dropped from the demo, and the first keydown left out of the driver's script.

**Real-map local run** (scratch IWAD copy; script not committed): E1M1, skill 2, screenblocks 10, a scripted
session with movement, mouse and keyboard turns and strafes, fire, a pause and use: 334 tics, 334 commands, 334
frames (all whole-frame digests), 1,670 status fields, 0 messages, 335 game dumps; the variants identical and the
driver's demo identical. Verdict PASS, and the same session checked in `--lmp` mode gave the same counts.

**Window trial under WSLg** (debug binary, synthetic room, `--quit-after-tics 175`): X11 exit 0 after 5.8 s;
Wayland exit 0 after 5.6 s with the compositor and Mesa warnings listed under step 8; both saved a 175-tic,
714-byte demo; the X11 demo checked with `tools/local/m5b_session_real.py` gave PASS in all three variants.

**Gates:** build, both fmt forms, Clippy, rustdoc with warnings denied, `cargo test --workspace` (1,626 passed, 0
failed, 4 ignored), `cargo +1.85.0` tests of `doom-game` and a build of `doom-play`, `m5b_session.py --check
--selftest` and `check_docs.py` passed. Sanitizers: the address and overflow variants of these runs, with leak
detection disabled.

**Not run:** step 7b; a second strict run for determinism; a sanitizer campaign beyond these runs; anything on
Windows or macOS; any session a person played.

## Step 8: the first play check

Run on 2026-11-17 at about 15:20 PST on the WSL2 host (the genuine checks finished a few minutes later), from the ext4 worktree, with the release
binary built by `cargo build --release --locked -p doom-play -j 4` and a scratch copy of the registered-mode
IWAD (local only; nothing derived from it is committed). Each run was
`timeout 60 doom-play --iwad ... --sessions DIR --quit-after-tics 350`; the saved session was then checked with
`python3 tools/local/m5b_session_real.py --iwad ... --lmp FILE` (with `--screenblocks 11` for the third).

| Run | Backend | Map, skill, screenblocks | Exit | Elapsed | stderr | Saved file | Genuine check |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | X11 (`WAYLAND_DISPLAY` unset) | E1M1, 2, 10 | 0 | 10.41 s | only the "saved ... (350 tics)" line | 1,414 bytes, 350 tics | PASS (68 s) |
| 2 | Wayland (`DISPLAY` unset) | E1M1, 2, 10 | 0 | 10.49 s | "saved" line plus "Wayland compositor does not support cursor shape", libEGL warnings ("failed to get driver name", "MESA-LOADER: failed to retrieve device information", "failed to create dri2 screen") and "MESA: error: ZINK: failed to choose pdev" | 1,414 bytes, 350 tics | PASS (40 s) |
| 3 | X11 | E1M3, 3, 11 | 0 | 10.20 s | only the "saved" line | 1,414 bytes, 350 tics | PASS |

Every check printed `tics 350; commands 350; frames 350 (digests 350, masked 0, non-comparable 0); status fields
1750; messages 0; game dumps 351; variants strict,overflow,address identical True; genuine end marker` and then
`VERDICT: PASS: every tic and frame identical to the genuine engine under the declared sound-free, wipe-free
contract (decision S15)`. 1,414 bytes is the 13-byte header, 350 tics of 4 bytes and the end marker. No window
or process was left running.

**What this showed:** the app starts under WSLg on both X11 and Wayland, loads the real IWAD and map, runs to
the tic count in the time 350 tics take at 35 tics a second (10.2 to 10.5 s), saves a session, and the idle
session replays in the genuine engine with every command, frame, palette, face, message flag, `M_Random` index
and masked game dump identical under the declared contract, at both screen sizes and on skill 3.

**What it did not show:** the picture on a screen (nobody looked, and there is no screenshot tool on the
machine), any key or mouse input (the sessions are idle: no input reached the app), mouse capture, fullscreen,
scaling, latency, frame pacing felt by a person, or anything on Windows or macOS. The Wayland run probably ran
on a software OpenGL fallback (an inference from the warnings, not observed). A human still has to judge these.

## Step 9: CI

What was run: the six `--check` commands of the new job and both reference tools locally
(`m5b_reference.py --check --pngs --selftest --jobs 3` took 376 s; `m5b_input_reference.py --check --selftest`
15 s), all three workflow YAML files parsed with `yaml.safe_load`, and `cargo build --release --locked -p
doom-play -j 4` (18 s on a warm target). `actionlint` was not installed and was not run.

- **`M5b genuine status bar, HUD and input`** (rust.yml, Linux, 45-minute timeout, same change detection and
  docs-only fast path as the M5a job; `--strict32` is not used). Not a required check name; no required check
  was added, renamed or removed, and `macos-26-intel` is untouched. The Stable and Minimum Rust jobs already
  run `--workspace`, so `doom-game` and `doom-play` build on Linux, Windows and macOS and test on 1.85.0 with no
  new step.
- **`Windows build artifact (doom-play)`** ([windows-artifact.yml](../../.github/workflows/windows-artifact.yml)):
  `windows-latest`, triggered by the label `windows-artifact` on a pull request or by a manual run once the file
  is on `master`; informational, not required. It builds with a statically linked C runtime and uploads the
  `doom-play-windows-x86_64` artifact (30 days): `doom-play.exe`, `LICENSE.TXT` and a generated `README.txt`; no
  game data. The repository label `windows-artifact` has to be created by the owner. The job had not run on
  GitHub when this was recorded.
- **Mutation scope:** `mutants.yml` gains `-p doom-game` (`doom-play` stays out: it is a window shell with no
  logic a test could pin); the shard count is a single value (16) in the new `Mutants plan` job, an estimate of
  the diff size, to be revised from real shard times. `cargo mutants --list` gave 7,916 mutants for
  `doom-render`, 569 for `doom-game` and 11,555 for `doom-sim` (whole-crate counts; `--in-diff` selects fewer).
  No mutation testing was run.

**Not run:** the Windows build and its packaging, the static-CRT build, the Stable, Minimum Rust and macOS jobs
for these changes, anything on GitHub; timings of the new job on a hosted runner.

## Step 12: fuzz target for the input code (first half)

Target `game_input_events` (nightly `cargo 1.101.0-nightly` of 2026-10-27, cargo-fuzz 0.13.2, build output and
corpora outside the tree). It drives the event layer (any key code including withheld ones, mouse motion up to
+-2^30, buttons, `release_all`, drain and tic builds) and the core (events sent straight to
`InputState::respond`, deltas up to +-2^31 so `MouseOverflow` is reachable). Properties: no panic or overflow;
the layer never emits a withheld keydown and never more than 63 events; every built and executed command is
wire-24 admitted, equals the demo round trip and has a zero low turn byte; a refused core event or
`Controls::set` leaves state equal; two runs from fresh state are identical. Five hand-made seed files are
committed.

- **Target compiled:** with the default (address) sanitizer and with `--sanitizer none`; `cargo fmt
  --manifest-path fuzz/Cargo.toml --all -- --check` passed.
- **Seed replay** (address build, `ASAN_OPTIONS=detect_leaks=0`, `-runs=0 -max_len=1017`): 5 files, 6 runs, no
  failure.
- **Coverage-guided campaign, no sanitizer** (`-fork=4 -max_total_time=600 -seed=1`): 613 s, last progress line
  33,720,595 executions, corpus 530; oom, timeout and crash counts 0/0/0.
- **AddressSanitizer campaign** (`ASAN_OPTIONS=detect_leaks=0`, same flags, 300 s): 304 s, last progress line
  3,823,719 executions, corpus 463; 0/0/0. Leak detection was disabled: no LeakSanitizer coverage.
- **Findings:** none. Bounded campaigns, not exhaustive; execution counts are the last periodic line of the
  fork-mode output, not a final total.

**Not run:** the other fuzz targets, libFuzzer minimisation, other sanitizers, workspace tests or Clippy, CI.
The second target (step 12b) is in progress and not recorded.

## What a human still has to judge

Nobody has seen the picture on a screen, pressed a real key, moved a real mouse, checked mouse capture under
WSLg, or run the program on Windows or macOS beyond the CI builds. The evidence above shows that the commands
the app makes are the genuine engine's and that the frames it draws equal the genuine engine's under the
declared contract; it cannot show:

- how the game feels: latency, smoothness at 35 tics a second, and the turning resolution (mouse turns below
  256 units are dropped because every session is recorded at demo resolution);
- how it looks: colours through the gamma table, nearest-neighbour scaling, the 4:3 stretch, fullscreen,
  the pause overlay and the app's end and refusal screens;
- whether the mouse is captured and released properly under WSLg, on Windows and on macOS;
- whether the key held across a window switch behaves acceptably (the library reports no focus loss);
- whether an OpenGL driver is found on a given machine.

## Still to come in this record

Steps 5 (real maps), 7b, 11 (mutation testing and its dispositions), the second half of step 12, 13 (timings
run alone on an idle machine from ext4), 14 (independent review) and the final gates are not yet recorded; the
second documentation pass adds them, the README animation and the CI observations.
