# M5b evidence record

Observed results of the M5b milestone (a limited playable build: a window, live keyboard and mouse input, the
35 tics-a-second loop, the status bar, HUD messages and palette flashes, on the wire-24 simulation and the M5a
renderer). Only observations appear here. The [M5b investigation](../M5B_REFERENCE_INVESTIGATION.md) owns the
source facts, the decisions S1-S23 (confirmed by the owner on 2026-11-17) and the plan;
[testing](../TESTING.md#m5b-playable-status-bar-hud-input-and-the-app) owns the commands and
[COMPATIBILITY](../COMPATIBILITY.md#playable-build-m5b) owns the support statement. The status-bar face, the
render state and the `Display` state are saved as described in
[STATE_FORMAT](../STATE_FORMAT.md#display-state-ddsp-version-1-m5b). Exact review state and CI belong to GitHub.

This is the second documentation pass, written on 2026-11-17 (America/Los_Angeles). A third pass adds the final
mutation and gate numbers; where a step's section says so, or is missing, that step had not been run or
its result had not been written up on that date.

**Wording rule.** A frame is described as identical to the genuine engine under the declared sound-free,
wipe-free contract (decision S15), never as identical to a vanilla session without that qualifier.

**Scope of the record.**

| Step | Date | State on 2026-11-17 |
| --- | --- | --- |
| 1 | 2026-11-17 | Recorded: synthetic graphics, eleven new rooms and the genuine full-frame runner ([below](#step-1-the-genuine-full-frame-runner-and-its-goldens)) |
| 2 | 2026-11-17 | Recorded: the genuine input driver and its scripts ([below](#step-2-the-genuine-input-driver)) |
| 3 | 2026-11-17 | Recorded: the status bar, HUD, palette choice and `Display` in `doom-render` ([below](#step-3-the-rust-status-bar-hud-and-display)) |
| 4 | 2026-11-17 | Recorded: the `doom-game` input core and event layer ([below](#step-4-the-rust-input-core-and-event-layer)) |
| 5 | 2026-11-17 | Recorded: all 61 real Episode 1 runs through the Rust display ([below](#step-5-the-real-map-check-local-only)) |
| 6 | 2026-11-17 | Recorded: the app core and the `doom-play` window ([below](#step-6-the-app-core-and-the-window)) |
| 7 | 2026-11-17 | Recorded: session verification ([below](#step-7-session-verification)) |
| 7b | 2026-11-17 | Recorded: the opt-in unrecorded full-resolution turning mode ([below](#step-7b-full-resolution-unrecorded-turning)) |
| 8 | 2026-11-17 | Recorded: the first play check and the focus-loss check ([below](#step-8-the-first-play-check)) |
| 9 | 2026-11-17 | Recorded: CI ([below](#step-9-ci)) |
| 10 | 2026-11-17 | Documentation: the first pass, this second pass and the status bar animation ([below](#step-10-the-status-bar-animation)); a third pass adds the final numbers |
| 11 | 2026-11-17 | Interim: the first mutation run is recorded ([below](#step-11-mutation-testing-interim-the-first-run)); dispositions and the second run are pending |
| 12 | 2026-11-17 | Recorded: both fuzz targets ([below](#step-12-fuzz-targets)) |
| 13 | 2026-11-17 | Recorded: frame time and tic jitter, WSLg only ([below](#step-13-timings)) |
| 14 | 2026-11-17 | Recorded: two independent reviews and their fixes ([below](#independent-reviews-and-their-fixes)) |

No IWAD was read in steps 1 to 4, 6, 7 (except the one local real-map run named there), 7b, 9, 11 and 12. Steps
5 and 8 and the local real-map run of step 7 read a scratch copy of the IWAD outside the repository; only counts,
pass/fail facts and digests are recorded. Address-checked runs used `ASAN_OPTIONS=detect_leaks=0`: LeakSanitizer was
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

Two consequences of the C, reproduced and tested: at a level start `st_oldhealth` is -1, so damage on the first
tic of the level shows the ouch face whenever health is 20 or more (`ST_Ticker` sets `st_oldhealth` at the end of
every tic, so later tics are unaffected); after switching from screenblocks 11 to 10, icons that were never drawn
have no old box to erase (two pixels remain in M5STARM). The independent review later asked for the golden test
to assert its counts; it now asserts (whole digest, masked, not compared, refused by a 2D drawer) =
(41,982, 300, 70, 0).

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

## Step 5: the real-map check (local only)

What was run (2026-11-17): `python3 tools/local/m5b_real.py` over the 61 runs of `tests/reference/m4b_real_results.json`
(the committed E1M1-E1M9 scripts and `DEMO1`, extracted from a scratch IWAD copy and patched from demo version
109 to 110) with 4 runs at a time. For each run it runs the genuine M5b runner (strict, with the S16 record and
mask) with no rendering and requires the dump set to equal the committed M4b digest; then at screenblocks 10 and
11, every tic, the strict (with dumps), overflow and address variants, whose frame streams must be byte-identical
and end alike; then the Rust display (`crates/doom-render/examples/m5b_real.rs`: the Rust simulation replays the
demo, the display observes and draws every tic) against the strict stream. The IWAD supplies only its own status
bar, font and palette lumps. Only counts, pass/fail facts and digests are committed
(`tests/reference/m5b_real_results.json`, 61 records); frame streams, which name IWAD lumps and carry message
texts, stay in the scratch directory.

| | Screenblocks 10 | Screenblocks 11 |
| --- | ---: | ---: |
| Runs | 61 | 61 |
| Tics = genuine frames = Rust frames | 158,966 | 158,966 |
| Compared on the whole-frame digest | 158,472 | 158,414 |
| Compared on the masked digest (columns the genuine tracker flagged) | 494 | 552 |
| Not compared (genuine `whole` / Rust switch parity unknown) | 0 / 0 | 0 / 0 |
| Flagged columns in the masked frames | 1,255 | 1,510 |
| Mismatched frames; renderer counter or flag disagreements | 0; 0 | 0; 0 |
| Status fields compared (5 per frame), mismatches | 794,830; 0 | 794,830; 0 |
| Tics compared for the message taken, mismatches | 158,966; 0 | 158,966; 0 |
| Messages taken (genuine = Rust) | 698 | 698 |
| Frames with a message showing; with a non-zero palette | 51,508; 13,660 | 51,508; 13,660 |
| Genuine stream digest = Rust stream digest | 61 of 61 | 61 of 61 |
| Strict, overflow and address streams identical, same endings | 61 of 61 | 61 of 61 |
| Render-every dumps = never-render dumps, snapshot for snapshot | 61 of 61 | 61 of 61 |
| Sanitizer reports; 2D tracker counts (undefined background, skipped patch, below screen, lump overrun) | 0; 0 | 0; 0 |
| Palette pointer outside PLAYPAL; `M_Random` outside `ST_Ticker` | 0; 0 | 0; 0 |
| Rust refusals, observe errors, not reached | 0; 0; 0 | 0; 0; 0 |

- Never-render masked dump sets equal to the committed M4b digest: 61 of 61 (159,027 snapshots, the load
  included). Endings, all variants alike: marker 27, reborn 22, exit 12 per screen size.
- Seen across the runs: 12 distinct palettes (palettes 1 and 9 never occur), 38 distinct face indices (the
  unreachable 37 and 40, and 13 and 21, never appear), and 24 distinct message codes. These agree with the Phase 1
  prototype's record in the investigation note.
- **No divergence, no S17 stop** (no witness, refusal or sanitizer report), so no fix, new room or golden change
  was needed. The full run was done twice, and all 61 records, including both stream digests per size, were
  identical. The 494 and 552 masked frames are the exclusion described in the support statement; there are no
  not-compared frames on the real maps.
- **The check can fail:** `--selftest` (on one run, E1M1 BARRELS at screenblocks 10) changed one frame digest, one
  face and one message tic in a copy of the strict stream and got exactly 1 mismatched frame, 1 face mismatch
  and 2 message-tic mismatches, the first at the earliest planted tic. The result is printed, not committed.
- Address runs had `ASAN_OPTIONS=detect_leaks=0`: leak detection was disabled, so this is not LeakSanitizer
  coverage. Run 1 took 48 min 03 s and run 2 44 min 36 s with other builds going on the same machine: these are
  not performance measurements.

**Not run:** a third full run to use `--check` on all 61 records (it was run on one run, E1M1 BARRELS, and equalled
the committed file); the M5a tools' `--check`; `--strict32`; sound and level-start wipe experiments.

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

Behaviour in this step that the independent reviews later changed (session files, focus loss, the recorder's
types, `quit`) is described under [Independent reviews and their fixes](#independent-reviews-and-their-fixes).

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

## Step 7b: full-resolution unrecorded turning

The opt-in `--unrecorded-full-turning` mode (decision S3 (ii)); the support statement is
[Unrecorded full-turning mode](../COMPATIBILITY.md#unrecorded-full-turning-mode-opt-in). A demo carries one byte of
`angleturn`, so the genuine engine had to be shown to agree with Rust on commands whose turn has a nonzero low
byte before the app could execute them. The Rust simulation matched the genuine engine on every tic of every
script, and wire-24 admission refused none, so nothing under `doom-sim`, no wire, DSIM format or digest changed.

- **The injection runner.** `tools/reference/m5b_inject.c` compiles `m5b_driver.c` byte for byte unchanged through
  `#include` with two renames (`main` and the driver's one `G_Ticker` call). Without `--commands` it is the demo
  runner. With a command file it loads the level through the genuine demo-play path from a header-only demo, then
  before each genuine `G_Ticker` clears `demoplayback` and writes the tic's command where `G_BuildTiccmd` writes
  it under `singletics`, so `G_Ticker` copies it and `P_MovePlayer` applies `angleturn << 16` whole. A separate
  contract (`tests/reference/m5b_inject_contract.json`) records the base contract and the two renames; the
  existing contract, `m5b_results.json` and `m5b_session_results.json` are unchanged.
- **Four injected scripts** on the synthetic rooms palred and palbonus (screenblocks 10, skill 2): the genuine
  input driver's built commands (keyboard 320, 640, 480; mouse values from -1184 to 160; walk, strafe, fire,
  weapon change, use), the same golden's executed commands as the control, literal keyboard turns with 1280 in
  both signs and a run over pickups, and mouse turns (small, clamp edges, every multiple of 8 from -504 to 504).
  630 tics, 373 commands with a nonzero low byte (16, 0, 99 and 258 in the four scripts).
- **Three genuine variants** (strict, overflow, address) give identical frame streams, dumps and endings on
  every script. The control (zero low bytes) equals plain demo replay through the unchanged runner.
- **The turn matters.** The strict runner also plays each script's demo-resolution twin (every turn rounded as the
  demo writer and reader do): chain_palred diverges at dump tic 25; keys_palbonus at tic 0 (2 pickup messages at
  full resolution, 0 in the twin); mouse_palred at tic 20, where a barrel keeps health 20 at full resolution
  (the accurate shot misses) and drops to 5 in the twin.
- **Rust against the genuine goldens** (`m5b_session --commands`, through `app::replay`): 630 tics, 630 commands,
  630 frames (630 on the whole digest), 3,150 status fields, X rows equal, 634 game-dump tics through the
  independent DSIM40 encoder. All equal.
- **Two unrecorded app sessions** (84 and 115 tics, 12 and 63 commands with a nonzero low byte): the app plays
  each with the mode on and saves nothing; the replay of its executed commands equals its live rows; the genuine
  input driver, fed the app's posted events with recording off, builds and executes the same commands; those
  commands injected into the genuine runner (three variants identical) match the app's live rows on every tic
  (199 tics, frames, status fields, X rows and 201 dump tics). With these: 6 scripts, 829 tics, 448 commands with
  a nonzero low byte.
- **App tests:** the default session is recorded and unlabelled; an unrecorded session executes the built
  commands (on `amp.wad` the executed nonzero turns are exactly 320 x5, 640 x3, 1184, -2880 and -96), writes
  nothing and shows the label on its pause picture; every ending (level end, a refusal at tic 75, the tic cap,
  death) writes no file and shows the label. `doom-play`'s argument test: the mode is off by default and no other
  option turns it on.
- **Self-test** (`m5b_inject.py --selftest`): a changed low byte (tic 25 of chain_palred) is found at
  `(25, 'command')`, and a changed Rust frame digest at `(25, 'frame')`. CI runs the check and self-test as one
  step of the M5b genuine job.

**Not covered:** real maps, and any person playing the mode; the label on screen is checked by tests, not seen;
the input-driver comparison inside the unrecorded-session check has no planted fault; no fuzzing or sanitizer
campaign beyond the address and overflow variants of these runs (address runs had leak detection disabled).

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

**Focus-loss check (2026-11-17, under WSLg, after the independent review).** The review read the window library's
source and found that it reports focus loss through its minimise event (the documents had said it did not), and
that `doom-play` turns that event into a pause. To observe it: a release `doom-play` built and run from ext4, on
the synthetic room `amp.wad` with `m5b_gfx.wad`, `--quit-after-tics 350` under `timeout 30`, and a second
`doom-play` started 3 seconds later.

- Alone, X11 exited 0 at 10.70 s and Wayland (no `DISPLAY`) at 10.46 s.
- With the second window, on both X11 and Wayland the first instance did not reach its 350 tics and was killed by
  the timeout (exit 124 at 30.02 s and 30.03 s), while the second exited 0.
- With a scratch probe build that printed when the minimise event arrived (the source was restored at once and the
  probe deleted): on X11 the first instance logged the event at tic 106 while playing and again while paused; on
  Wayland once, at tic 112, while playing; the second instance logged none.

So under WSLg a new window taking the focus pauses the running game, on X11 and on Wayland. **Not checked:**
Alt-Tab, minimising by hand, Windows, macOS, which native event (focus-out or keyboard leave) fired, and whether the
held keys are seen released on screen (there is no input injection tool on the machine). Killing a session leaves
its empty 14-byte demo, which is created at session start.

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
  game data. The repository label `windows-artifact` now exists, and the job ran and succeeded on the pull request
  on 2026-11-17 (run time 1 min 15 s). Nobody has downloaded or run the artifact on Windows.
- **Mutation scope:** `mutants.yml` gains `-p doom-game` (`doom-play` stays out: it is a window shell with no
  logic a test could pin); the shard count is a single value (16) in the new `Mutants plan` job, an estimate of
  the diff size, to be revised from real shard times. `cargo mutants --list` gave 7,916 mutants for
  `doom-render`, 569 for `doom-game` and 11,555 for `doom-sim` (whole-crate counts; `--in-diff` selects fewer).
  No mutation testing had been run at this step; the first run is under [step 11](#step-11-mutation-testing-interim-the-first-run).

**Not run:** the Windows build and its packaging, the static-CRT build, the Stable, Minimum Rust and macOS jobs
for these changes, anything on GitHub; timings of the new job on a hosted runner.

## Step 10: the status bar animation

`docs/media/m5b-status-bar-side-by-side.gif` (968 x 298 pixels, 155 frames, 10.36 s, 1,333,776 bytes) is made by
`tools/media/m5b_readme_visual.py` ([commands](../TESTING.md#readme-animations)). It plays the committed session
`palred_damage` (155 tics on the synthetic room `palred.wad`, screenblocks 10, our own generated test art, no
IWAD) and shows, side by side, the genuine engine's frame, the Rust display's frame and their difference: tics
0-11 walk to a soulsphere, tic 12 shows the pickup message with palette 10, and from about tic 54 a barrel shot at
point-blank range gives red palettes 8 down to 2 and the faces 0, 2 and 7.

- **Compared:** all 155 tics on palette indices and palette numbers: 0 differing pixels (0 of 155 x 64,000) and 0
  palette mismatches. On any difference the script exits with the first differing tic, pixel and palette and writes
  nothing.
- **Asserted before it writes:** the session script and the app's demo match the committed
  `tests/reference/m5b_session_results.json`; the genuine strict runner's normalised frame stream matches the
  committed frame-stream digest; every raw genuine frame equals its row's digest; no frame is non-comparable; the
  PLAYPAL read from `m5b_gfx.wad` equals the renderer's; the script decodes its own GIF and compares every frame.
- **Reproducible:** a first write and a later `--check` regeneration gave byte-identical files (about 45 s a run).
  The GIF has no timestamp or path.

**Not run:** a browser or GitHub render check (the script's own decoder was used, and two decoded frames were
viewed as images).

## Step 11: mutation testing (interim: the first run)

This is an interim record: only the first run is described. The dispositions of its survivors and the second run
are pending and will be added by a later pass.

- **The run** (the `Mutants` workflow with the `run-mutants` label, 16 shards, at 334cd4b): `cargo mutants --in-diff`
  over the 9,839-line diff of `crates/` against `origin/master` listed **1,109 mutants** (70 per shard, 59 in the
  last). The 15 shards that finished gave **812 caught, 154 missed, 7 timeouts and 66 unviable** (1,039 mutants;
  their lists equal the result files exactly).
- **Shard 3** (70 mutants, all in `doom-game`: 23 in `app/recorder.rs`, 47 in `app/screens.rs`) lost its runner. It
  was re-run locally from the same patch under a memory cap. A first attempt with a 3 GB cap per test process and
  the default test parallelism was stopped by a watchdog when available memory fell to 692 MB (five tests at once,
  so the per-process cap did not protect the 7 GB host); the second, with a 1.2 GB cap and two test threads,
  finished in 3 minutes: **38 caught, 31 missed, 0 timeouts, 1 unviable**.
- **The culprit** was `screens.rs:34:40: replace / with %` in `COLUMNS = SCREENWIDTH / ADVANCE - 2`, which makes
  `COLUMNS` 0. `wrap()` then never shrank its input and pushed an empty string on every pass: an endless loop with
  unbounded allocation, which took about 4.4 GB in 25 seconds locally and ended the hosted runner. It was also a
  real test gap: nothing pinned `COLUMNS` at 51, and three sibling mutants survived. Fixed in 4fe0936: every
  pass of `wrap()` takes at least one character (0 columns count as 1), and a unit test pins the constants and
  checks line widths and that the text is kept, at 51, 7, 1 and 0 columns, with an empty text and a 130-character
  word.
- **The workflow guard** (a2e0b94): the shard step runs test executables through a cargo runner that applies
  `ulimit -v 1500000` and `timeout 240` (per process, test executables only, not `rustc`) and passes
  `--cargo-test-arg=--test-threads=2`, so a mutant that loops and allocates is caught instead of starving the
  runner. The unmutated suite passed under a 1.2 GB cap locally; 1.5 GB and the hosted two-job setup were not run
  locally.
- **Survivors so far:** for `doom-render`, the 72 survivors of the CI run were given tests in
  `crates/doom-render/tests/m5b_mutants.rs` (18 tests, each from a C fact already cited next to the code or from
  the DDSP bounds). As recorded in that commit (7b605d7): cargo-mutants running only these tests catches 55 of
  the 72, 5 are equivalent, and 12 are unreachable under wire 24 and need inline tests of private items. The
  survivors of `doom-sim` and `doom-game` have no dispositions yet.

**Not run:** the second run after the new tests; uncapped execution of the culprit (deliberately); the
CI-equivalent two-job run locally; shards other than 3 locally.

## Step 12: fuzz targets

Both targets are in `fuzz/fuzz_targets/` ([`game_input_events.rs`](../../fuzz/fuzz_targets/game_input_events.rs),
[`render_display_state.rs`](../../fuzz/fuzz_targets/render_display_state.rs)), built with nightly `cargo
1.101.0-nightly` of 2026-10-27 and cargo-fuzz 0.13.2, with build output and corpora outside the tree. Each run
kind is listed separately; all are bounded campaigns, not exhaustive, and execution counts are the last periodic
line of the fork-mode output, not a final total.

**`game_input_events`** (five hand-made seeds committed). It drives the event layer (any key code including
withheld ones, mouse motion up to +-2^30, buttons, `release_all`, drain and tic builds) and the core (events sent
straight to `InputState::respond`, deltas up to +-2^31 so `MouseOverflow` is reachable). Properties: no panic or
overflow; the layer never emits a withheld keydown and never more than 63 events; every built and executed
command is wire-24 admitted, equals the demo round trip and has a zero low turn byte; a refused core event or
`Controls::set` leaves state equal; two runs from fresh state are identical.

- **Target compiled:** with the default (address) sanitizer and with `--sanitizer none`.
- **Seed replay** (address build, `ASAN_OPTIONS=detect_leaks=0`, `-runs=0 -max_len=1017`): 5 files, 6 runs, no
  failure.
- **Coverage-guided campaign, no sanitizer** (`-fork=4 -max_total_time=600 -seed=1`): 613 s, 33,720,595
  executions, corpus 530; oom, timeout and crash counts 0/0/0.
- **AddressSanitizer campaign** (`ASAN_OPTIONS=detect_leaks=0`, same flags, 300 s): 304 s, 3,823,719 executions,
  corpus 463; 0/0/0. Leak detection was disabled: no LeakSanitizer coverage.
- **Findings:** none. After the review fixes changed the recorder's accessors the target was updated to compile,
  and its five seeds replayed again with exit 0 (address build); no campaign was re-run after the fixes.

**`render_display_state`** (seven seeds committed: three real `DDSP` states of about 64 KB and four hand-made
driven-run streams). Byte 0 picks the part and byte 1 one of the 11 M5b rooms. Part 0 decodes arbitrary bytes as
`DisplayState` (canonical re-encode equal to the input, decode again equal) and restores into a display at tic
`k`: a typed refusal leaves `state()` unchanged, or four tics of `observe` and `draw` on two clones agree.
Part 1 runs 16, 32 or 64 tics with wire-24 commands at screenblocks 10 or 11 and `observe_with` perturbations
(health, armour, ammunition, weapons, keys, counts, powers, attacker including a removed one, position, angle,
message none, short, empty or over 80 characters). It checks palette 0-13 or a typed refusal; a refused observe or
draw leaves `state()` equal; the state encoded after every tic decodes to an equal state with identical
re-encoding; and a state saved at a chosen tic, restored into a fresh display beside a simulation clone, continues
with identical observe results, messages, palette, face, frame hash and final encoded state. The properties are
self-consistency only; nothing is compared with the genuine engine.

- **Target compiled:** `cargo +nightly fuzz build render_display_state`, and `cargo fmt` of the fuzz workspace.
- **Seed replay** (a scratch copy of the seeds, `-runs=0 -max_len=70000 -timeout=60`): 7 files, 13 runs, no failure
  after the finding below was handled.
- **Coverage-guided campaign, no sanitizer** (`--sanitizer none`, `-fork=2 -max_total_time=600 -max_len=70000
  -timeout=60 -rss_limit_mb=2048`): 614 s and only about **2,320 executions** (2 to 3 a second: a part 0 input is
  64 KB and a part 1 run encodes the state every tic), cov 5,932, corpus 466; 0 oom, timeout or crash.
- **AddressSanitizer campaign** (`ASAN_OPTIONS=detect_leaks=1`, `--sanitizer address`, `-fork=2
  -max_total_time=300`, same limits): 316 s, 641 executions, cov 11,646, corpus 103; 0/0/0. Leak detection was
  enabled explicitly and no leak was reported; that the platform's LeakSanitizer actually ran was not verified. (A
  first run of 630 s made before noticing that cargo-fuzz defaults to the address sanitizer was an
  address-sanitizer build with default options, about 877 executions and no crash; it is not counted as the
  no-sanitizer campaign.)
- **Campaign after the review fixes** (address sanitizer, `ASAN_OPTIONS=detect_leaks=0`: leak detection
  disabled; `-seed=1 -max_len=391`, 3 minutes): the first campaign stopped at a **harness bug** (below); after the
  fix, 462 executions in 181 s, cov 11,598, 196 corpus entries, no finding. With `-max_len=391` inputs longer than
  that were not generated, and the report does not say what that did to part 0.
- **Findings.** (1) A hook finding, not a crash: `observe_with` with a health perturbation of -512 gave a display
  whose own encoded state `DisplayState::decode` refuses (`oldhealth`), a value the simulation never produces
  (health is clamped at 0) but the hook's type allowed. The target first narrowed its perturbation to 0..=2040; the
  review fixes made `observe_with` refuse out-of-range adjusted inputs (`AdjustedInputOutOfRange`, before any state
  change), and the target now perturbs health by any value. (2) A **harness bug** found by the later campaign:
  the target saved a state after a refused `observe`, which gave an inconsistent tic pair; it now saves only after an
  accepted observe. By the fixing step's reading this was reachable earlier through the attacker-removed
  perturbation; that was not shown.

**Not run:** sanitizers other than address; a long campaign for either target (the largest is 614 s); minimisation;
Clippy on the fuzz workspace; timings or Windows and macOS.

## Step 13: timings

Recorded, never gated (decision R15 of M5a carries over; no threshold and no CI step was added). Observed
2026-11-17 (America/Los_Angeles), 18:31 to 18:42 for the measurements, at commit `3e8865b` plus the two measurement
aids described below (uncommitted when measured, committed with this record). Machine: Intel Core i7-8750H @ 2.20 GHz,
WSL2 (kernel 6.18.58.1-microsoft-standard-WSL2, WSLg 1.0.79); rustc 1.100.0 (0ef1bc386 2026-11-09); release profile.
Binaries were built in and run from the Linux filesystem (ext4): `CARGO_TARGET_DIR=~/scratch/m5b/step13/target`; the
repository is on ext4 under `/home`; nothing ran from `/mnt/*`. The owner had said the machine was idle. Before the
frame runs `uptime` showed load average 0.96, 1.47, 3.52 (decaying from the release build a few minutes earlier;
0.88 at the first run) and the busiest processes were `claude --rc` (2.5 %) and rust-analyzer (0.6 %, resident and
idle); before the loop runs 0.87, 1.37, 3.37 with the same two. Nothing else was started while a measurement ran;
the load averages logged at the start of each loop run were 0.48 to 1.17. No IWAD was read; every case is a committed
synthetic room with `tests/reference/m5b_gfx.wad`. No real-map number was taken.

### Frame time with the status bar and HUD

New example `m5b_bench` ([command](../TESTING.md#m5b-timings)). One unmeasured warm-up replay, then one measured
replay per room and screenblocks, each on a freshly loaded simulation; the simulation advance is outside every timer.
Per frame it times `Display::observe` plus `Display::draw`, `Display::draw` alone, and, on the same frames, the M5a
`Renderer::observe` plus `render` (the 3D view alone, no status bar or HUD; the M5a example `m5a_bench` times
`render` only and could not be used because it has no `Display`). The whole sequence ran 3 times; each cell is the
three runs separated by `/`, in microseconds. Rooms: `M5MSG` (message showing, by the room's design), `M5PALRD` and
`M5PALBN` (red and bonus palette flashes), `M5STDIG` and `M5EVIL` (plain views with the status bar's numbers and
face changing). Every replay ran to the end of its script with no refusal.

| Room | Screenblocks | Frames | Series | Median | p95 | Max |
| --- | --- | --- | --- | --- | --- | --- |
| M5MSG | 10 | 696 | Display observe+draw | 549/551/552 | 622/602/624 | 964/1116/1396 |
| M5MSG | 10 | 696 | Display draw | 392/393/392 | 439/440/441 | 742/801/876 |
| M5MSG | 10 | 696 | Renderer observe+render | 534/536/539 | 603/589/610 | 1065/907/1287 |
| M5MSG | 11 | 696 | Display observe+draw | 583/579/571 | 693/672/651 | 1485/913/1088 |
| M5MSG | 11 | 696 | Display draw | 420/416/412 | 476/461/459 | 1089/701/869 |
| M5MSG | 11 | 696 | Renderer observe+render | 575/575/566 | 688/623/631 | 1226/1089/1259 |
| M5PALRD | 10 | 175 | Display observe+draw | 303/303/301 | 360/345/354 | 525/435/682 |
| M5PALRD | 10 | 175 | Display draw | 263/261/261 | 304/294/314 | 467/397/624 |
| M5PALRD | 10 | 175 | Renderer observe+render | 293/290/288 | 342/329/346 | 533/514/558 |
| M5PALRD | 11 | 175 | Display observe+draw | 329/326/325 | 372/394/374 | 476/661/591 |
| M5PALRD | 11 | 175 | Display draw | 288/287/285 | 314/349/320 | 438/516/516 |
| M5PALRD | 11 | 175 | Renderer observe+render | 325/321/322 | 373/358/362 | 561/474/601 |
| M5PALBN | 10 | 130 | Display observe+draw | 352/350/396 | 400/451/976 | 548/554/1358 |
| M5PALBN | 10 | 130 | Display draw | 290/288/318 | 352/343/758 | 474/487/1062 |
| M5PALBN | 10 | 130 | Renderer observe+render | 340/335/372 | 374/398/1052 | 706/674/1672 |
| M5PALBN | 11 | 130 | Display observe+draw | 377/371/385 | 438/449/470 | 966/713/601 |
| M5PALBN | 11 | 130 | Display draw | 311/309/318 | 368/349/396 | 638/589/486 |
| M5PALBN | 11 | 130 | Renderer observe+render | 364/363/386 | 422/413/467 | 611/530/664 |
| M5STDIG | 10 | 877 | Display observe+draw | 366/359/360 | 434/448/435 | 938/803/3770 |
| M5STDIG | 10 | 877 | Display draw | 298/294/294 | 342/353/342 | 723/732/3696 |
| M5STDIG | 10 | 877 | Renderer observe+render | 352/346/348 | 418/442/421 | 729/801/779 |
| M5STDIG | 11 | 877 | Display observe+draw | 375/378/399 | 456/460/507 | 978/823/854 |
| M5STDIG | 11 | 877 | Display draw | 310/313/328 | 365/370/404 | 670/662/644 |
| M5STDIG | 11 | 877 | Renderer observe+render | 369/374/397 | 450/455/502 | 698/697/749 |
| M5EVIL | 10 | 413 | Display observe+draw | 345/346/340 | 387/390/397 | 579/587/612 |
| M5EVIL | 10 | 413 | Display draw | 290/290/285 | 323/326/331 | 522/516/543 |
| M5EVIL | 10 | 413 | Renderer observe+render | 330/332/327 | 369/382/381 | 653/738/664 |
| M5EVIL | 11 | 413 | Display observe+draw | 378/367/373 | 457/488/497 | 1157/983/895 |
| M5EVIL | 11 | 413 | Display draw | 321/311/315 | 380/413/421 | 1038/928/801 |
| M5EVIL | 11 | 413 | Renderer observe+render | 369/363/370 | 461/473/540 | 4741/748/4529 |

Run 3's `M5PALBN` at screenblocks 10 (median 396 against 352 and 350) and its 3,770 us maxima in `M5STDIG` are
scheduler noise: the same run's other cases agree with runs 1 and 2. Maxima are single-frame outliers. `Renderer
observe+render` includes the renderer's `observe`, which the `Display draw` series does not, so the two are not
like-for-like; `Display observe+draw` is the matching series.

### Tic jitter of the real loop

New opt-in flag `doom-play --tic-stats` ([command](../TESTING.md#m5b-timings)): it keeps in memory the
monotonic-clock time (the loop's own, microseconds) at which each `App::update` ran its tics, counts the dropped tics
`App::update` reports, and prints a summary to stderr on exit. A tic executed in the same `update` as another has the
same time (interval 0); this never occurred (see the minima). The window ran on the synthetic room `M5STDIG`
(`stdigit.wad` as the IWAD with `m5b_gfx.wad` as the extra WAD), idle (no key or mouse input), `--quit-after-tics
3500`, release build, three runs each under X11 (`DISPLAY` set, `WAYLAND_DISPLAY` unset) and Wayland (`DISPLAY`
unset), alternated, about 100 s each (the process ran 100 s from `date` stamps: 18:31:50-18:33:30 and so on). Each run
executed all 3,500 tics and dropped none. Intervals are between consecutive executed tics, in microseconds; 1/35 s
is 28,571.4 us.

| Backend | Run | Executed | Dropped | Mean | SD | Min | Median | p95 | p99 | Max | >1 ms off | >5 ms off |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| X11 | run | 3500 | 0 | 28571.4 | 61.3 | 27540 | 28571 | 28670 | 28719 | 29608 | 2 | 0 |
| Wayland | run | 3500 | 0 | 28571.4 | 166.6 | 24247 | 28571 | 28695 | 28740 | 35703 | 3 | 1 |
| X11 | run | 3500 | 0 | 28571.5 | 63.7 | 27988 | 28571 | 28691 | 28728 | 29159 | 0 | 0 |
| Wayland | run | 3500 | 0 | 28571.4 | 67.6 | 27473 | 28572 | 28681 | 28723 | 29659 | 2 | 0 |
| X11 | run | 3500 | 0 | 28571.4 | 53.7 | 28285 | 28571 | 28665 | 28705 | 28949 | 0 | 0 |
| Wayland | run | 3500 | 0 | 28571.4 | 91.1 | 25459 | 28571 | 28667 | 28705 | 31701 | 2 | 0 |

The wall time from the first to the last tic was 99.971 to 99.972 s in every run, equal to the expected 3,499/35 s
by construction: the tic clock is computed from its origin ("no accumulated rounding"), so a late tic is followed by
a shorter interval and the mean cannot drift; the spread is in the intervals. Stderr under X11 showed no driver
message; under Wayland it showed `Wayland compositor does not support cursor shape`, libEGL warnings (`failed to get
driver name for fd -1`, `MESA-LOADER: failed to retrieve device information`, `egl: failed to create dri2 screen`)
and `MESA: error: ZINK: failed to choose pdev`; the GL renderer was not printed, so which software or virtual GPU
path drew is not known.

### Reading

A frame with the status bar, HUD and 3D view cost a median of 0.30 to 0.58 ms and a p95 of at most about 1.0 ms here,
against the 28.6 ms tic, so drawing used about 1 to 2 % of a tic and the status bar and HUD added nothing measurable
over the view alone. The loop ran all 3,500 tics at 35 Hz with no drops: 99 % of intervals were within about 170 us
of 1/35 s, and the worst interval was 35.7 ms (a 7 ms late tic, once, under Wayland); at most 3 of 3,499 intervals per
run were more than 1 ms off and 1 of the 21,000 more than 5 ms.

Limits: WSL2 under WSLg only, on a shared host that was idle as described, with a software or virtual GPU path that
was not identified; no input during the loop runs (key and mouse events were not measured, and the update's own
cost is inside the interval); `draw` time is not window presentation time (the texture upload, the GL draw and the
buffer swap are outside the example's timers, and the loop runs did not time them); nobody judged smoothness by eye;
one synthetic room for the loop and five for the frames; nothing was measured on Windows or macOS, with a real
GPU, or on a real map.

## Independent reviews and their fixes

Two independent reviews read the code at 334cd4b (their snapshot had one docs-only commit on top) on 2026-11-17,
without edits to any tree: one of the input code, the app and the session tools (19 findings), one of the status
bar, HUD, palette, frame composition and the genuine full-frame harness (12 findings; it did not read the authors'
scratch reports). Neither found a defect or a silent divergence on any state the app can reach: the first found no
path through the app that changes a command after it is recorded, and the second found every table, offset,
threshold, counter, priority and update condition it compared equal to the C.

**Findings by class and what was done** (fixes in e3dd509 for the input code, app and session tools, f3183b3 for
the status bar, HUD and display, and 4fe0936 for `wrap()`; the status paragraph of the investigation note is
e078e2c; this record's wording findings are fixed in this pass):

- **Wording (9 in the first review; 4 in the second, counting its documentation mismatch):** the claim that every command is the genuine result left out cheat completion
  (`idclev` differs; other cheats are silently inert) and is now qualified, with the golden pinned at tic 8; the
  documents said the library reports no focus loss (it does; see the focus-loss check above); a display-refused tic
  is in the demo; universal session claims; `--skill` numbering against vanilla's; "window pixels" (raw device
  units); a wrong C citation; the end screen shows only the file name; frame identity stated without its
  exclusions (300 masked and 70 not compared on the synthetic set); the ouch-face statement; the `BeyondBuffer`
  refusal for foreground posts being conservative; a citation line in `hu_lib.rs`.
- **Robustness (7 in the first review, 2 in the second):** a session name could overwrite an existing file after 998 collisions (files are now
  created exclusively and names run to -9999); a failed final save was not retried and not shown during play
  (retried on quit and restart, shown on the pause box, temporary file synced); a sub-tic tap of the run key under
  always-run was lost (the same deferral as an ordinary tap, with a test that fails without the fix); `App::picture`
  panicked on a buffer longer than 64,000 pixels; a refused event wedged the input queue (now consumed);
  `RecordedTic` had public fields (now private with accessors, and `replay` takes demo commands only the demo reader
  or the recorder can make, with a separately named `replay_full_resolution` for injected commands); `quit` let up
  to 5 tics run (none now); `observe_with` could make `draw` panic and decoded states could be forged (the hook now
  refuses out-of-range inputs, and `restore` checks the fields against each other and the simulation: see the
  support statement for what is still trusted).
- **Test and harness gaps (3 in the first review, 2 in the second):** the stale-binary risk in `m5b_session.py` (now takes the binary from cargo's
  JSON output); the cheat exclusion accepted a difference at any tic (pinned at tic 8); the missing fuzz and mutation
  run (both since added); the digest counts of the golden test (now asserted); witnesses without end-to-end evidence
  (`AttackerRemoved` is now reached through the real simulation path in a test; the other witnesses and
  `MessageNotTaken` still come from constructed states).

The second review also gave 4 notes: conservative refusals at load (documented in the support statement), the
mid-session size change and `observe` trusting its command (both left as they are, below), and a wrong citation
line (fixed).

**Left as it is, and why:** the directory is not synced after the rename (a process crash is covered, an operating-
system crash or power loss is not); the whole-demo save is synchronous on the loop thread (determinism is
unaffected; responsiveness on a slow path was not measured); the saved-state check still takes on trust the line
text, `showMessages`, the palette and widget old values, `oldweaponsowned`, `firsttime` and `bg_drawn`, `rndindex`
and the pain band; a mid-session screenblocks change has no genuine evidence (the app fixes the size per session);
`observe` trusts the command it is given (the app passes the executed command, and its clamp keeps the turbo
message out of reach); the review's alternative of withholding cheat-code keys was not taken, since forwarding
them follows decision S5 and changes no command except after `idclev` (it is the owner's call to change); and that
wire 24 never removes a live monster is argued in a test comment, not shown.

**Observed by the fixing step** (on its own tree, not re-run on a later head): build, both format checks, Clippy with
warnings denied, warnings-denied rustdoc, `cargo test --workspace` (1,638 passed, 0 failed, 4 ignored, over 112
test binaries, before the last commit), `cargo test -p doom-render -p doom-game --release` (232 passed) and the
same two crates on Rust 1.85.0 (232 passed, and `doom-play` builds); `m5b_session.py --check --selftest` (4
sessions match), `m5b_inject.py --check --selftest` (6 goldens match), `m5b_input_reference.py --check --jobs 2`
(251 goldens match, 36,453 tics; leak detection disabled) and `check_docs.py`. Not run by that step: the C
regeneration checks that no change touched, the real-map tool, Windows or macOS, sanitizer campaigns beyond seed
replay for the input target, and mutation testing. CI on the pull request is in GitHub.

## What a human still has to judge

Nobody has seen the picture on a screen, pressed a real key, moved a real mouse, checked mouse capture under
WSLg, or run the program on Windows or macOS beyond the CI builds. The evidence above shows that the commands
the app makes are the genuine engine's (except for cheat codes, which the app does not act on) and that the
frames it draws equal the genuine engine's under the declared contract; it cannot show:

- how the game feels: latency, smoothness at 35 tics a second, how the raw mouse clamp feels, and the turning
  resolution (mouse turns below 256 units are dropped in a recorded session; the unrecorded mode keeps them, and
  nobody has played it);
- how it looks: colours through the gamma table, nearest-neighbour scaling, the 4:3 stretch, fullscreen,
  the pause overlay and the app's end and refusal screens;
- whether the mouse is captured and released properly under WSLg, on Windows and on macOS;
- whether switching windows behaves acceptably: the game was seen to pause when a second window took the focus
  (X11 and Wayland under WSLg), but Alt-Tab, minimising by hand, Windows and macOS were not checked, nor whether
  held keys are seen released;
- whether an OpenGL driver is found on a given machine; and whether the Windows artifact runs on Windows.

## Still to come in this record

A third pass adds the dispositions of the mutation survivors and the second mutation run (step 11) and the final
gates; the timings (step 13) are recorded above.
