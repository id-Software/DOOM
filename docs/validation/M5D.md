# M5d evidence record

**Status (2026-11-20 05:23 PST):** **stopped at the owner's Windows session (A6); the pull request stays a draft.** Code
complete: the DPI request on Windows (A1), the sound statistics and trial options (A2), the session file kept current tic by
tic (A3), the mouse forwarding moved into its own file with the input log's review points M1 to M3 closed, and the CI job
renamed (A4). The gates passed (A5: two independent reviews, three mutation runs, the full gates from a clean clone, CI green
on all nine jobs at the last code head). CI built and tested the code on Windows and macOS; nobody has run it by hand there.
Next: the owner's session with the Windows artifact (A6), then his rulings.

Observed results of milestone M5d (play fixes from the owner's playtest of M5c). Only observations and the
library-source readings behind each change appear here. The facts this milestone starts from are in the
[M5c record](M5C.md#the-owners-windows-retest-with-the-input-log-2026-11-19) and the
[roadmap item](../ROADMAP.md) that named this work; no separate investigation note was written, by the owner's
instruction. [Testing](../TESTING.md) owns the commands, [COMPATIBILITY](../COMPATIBILITY.md) the support
statement and [ROADMAP](../ROADMAP.md) the milestone status. Exact review state and CI belong to GitHub.

Scope, by the owner's brief: app and presentation work only. Nothing in `doom-sim`, `doom-render` or
`doom-sound` changes, and no wire, schema, golden or existing test changes.

## A1. Stuck fire on Windows: the DPI request

**The change** (commit `0958257`): `crates/doom-play/src/window.rs` sets miniquad's `Conf::high_dpi` to true
on Windows only (`HIGH_DPI = cfg!(target_os = "windows")`). Linux and macOS ask for nothing, as before, so
their behaviour is unchanged by construction. `--input-log` is kept.

**What the flag does, read from miniquad 0.4.11's source** (`src/native/windows.rs` in the cargo registry;
line numbers of that file; nothing here was run on Windows):

| Effect | Source |
| --- | --- |
| The process calls `SetProcessDPIAware` before the window is created: it becomes *system* DPI-aware | `run`, 1158-1160 |
| The client rectangle, the clip rectangle built from it with `ClientToScreen` and `ClipCursor`, and the cursor are then all in physical pixels | `update_clip_rect`, 327-353 |
| The requested size (960×720, or 960×600 with square pixels) is passed to `AdjustWindowRectEx` and `CreateWindowExW` unscaled, so it is taken as physical pixels | `create_window`, 894-964 |
| The window scale is read from the primary monitor's effective DPI (`GetDpiForMonitor` on the monitor at point 1,1); the content scale becomes that factor and the mouse scale 1 | `init_dpi`, 1093-1116 |
| `screen_size` becomes the physical client size, computed as `trunc(trunc(client / scale) × scale)`, so it can be short of the real client size by less than the scale plus one pixel (table below) | `update_dimensions`, 1059-1066; the scales, `init_dpi`, 1109 and 1112 |
| Raw mouse motion is multiplied by the mouse scale, which is 1 with the flag and also 1 without it (without the flag the window scale stays 1) | `WM_INPUT`, 526-556; `init_dpi` |

**What that means for the player on Windows (inference from the source):**

- **Window size.** At 100% nothing changes. At 125% the window is 960×720 physical pixels, about 768×576 in
  Windows' scaled units; before, Windows scaled the 960×720 window up to 1,200×900 physical pixels (the owner
  saw "about 1200 by 900"). At 150% it is about 640×480 scaled units where it was 1,440×1,080. The window
  stays resizable, and `--fullscreen` covers the screen either way (`GetSystemMetrics` returns physical
  pixels to a DPI-aware process). Asking for a larger window after start (`window::set_window_size`) was not
  done: it would add a `resize` line to every log, which the acceptance test reads as a fault.
- **Picture fit.** The picture is fitted to `screen_size` at every draw (`fit` in `window.rs`). When the
  truncation above makes `screen_size` smaller than the real client area, the picture can be narrower or shorter
  than it could be, with a black line at an edge. The shortfall is less than the scale plus one pixel. Worked
  through in `f32` for client widths 1 to 7,999, the worst case is: none at 100%; one pixel at 125%, 150% and
  200%; two at 175%, 225%, 250% and 300%; three at 275% and 350%; and four at 450% and 500% (the review
  assumed one pixel up to 200%, which does not hold at 175%). Cosmetic only.
- **Mouse.** Turning speed is unchanged (mouse scale 1 either way). The positions on `raw ... cursor` and
  button lines of the input log become physical pixels.
- **Sharpness.** Windows no longer stretches the window's bitmap, so the picture is drawn at the screen's
  pixels; with nearest-neighbour scaling of a 320×200 picture this is not expected to look different.

**On Linux and macOS, if the flag were set** (it is not): on X11 only the reported `dpi_scale` and
`high_dpi()` change (`src/native/linux_x11.rs`, 488-489 and 598-599; nothing else reads them), under Wayland
with an output scale above 1 the buffer and `screen_size` grow by the integer scale
(`src/native/linux_wayland.rs`, 785-798), and on macOS the view asks for the Retina backing store and
`screen_size` doubles on a Retina display (`src/native/macos.rs`, 210-232 and 1056-1060). `doom-play` fits
the picture to `screen_size` in all of these, so the picture would still fit; no fault was seen there and
nothing is changed.

**CI builds.** The flag is a compile-time constant read by miniquad's start-up; the Windows, macOS and Linux
CI jobs compile it on the exact head (GitHub's record). The test `dpi_awareness_is_asked_for_on_windows_only` pins
the constant on each operating system, and, since the review (N2), the test
`the_window_configuration_asks_for_dpi_awareness_on_windows_only` checks the window configuration that `play` passes to
miniquad (`window_conf`: `high_dpi`, size, fullscreen, resizable, Linux backend). Deleting the `high_dpi` line from the
configuration is caught only on Windows, where miniquad's default (`false`) differs from the constant; on Linux and
macOS the two are equal, so the same behaviour results (inference; Windows was not run). A build proves only that it compiles and that the constant is
right; what the window does on Windows is the owner's to observe.

### Can a release still be lost? Yes, in four cases, by the source

The flag addresses the case the owner's log points to (the hidden cursor coming to rest on the window's
border, where a button-up goes to the frame). It does not give the library a mouse capture. Reading
`src/native/windows.rs` and `doom-play`, a left-button release can still miss `doom-play` when, at the moment
of the release, the cursor is not over the game's client area or something else receives the button-up:

1. **Another monitor with a different scale.** `SetProcessDPIAware` makes the process *system* DPI-aware, not
   per-monitor aware: on a second monitor whose scale differs from the primary monitor's, Windows stretches the
   window's bitmap again, which brings back the mismatch the flag removes. The library handles no
   `WM_DPICHANGED` (no match in the file). One monitor, or monitors at the same scale, are not affected.
2. **The clip rectangle is cleared or replaced without a focus change.** The clip is a system-wide setting.
   The library sets it in `set_cursor_grab` (124-133) and again only on `WM_SIZE` (439-442) and `WM_MOVE`
   (522-524), not on `WM_ACTIVATE` (763-769). `doom-play` pauses on focus loss and grabs again on resume, which
   re-applies it; a program or system component that changes the clip while `doom-play` keeps the focus
   leaves the cursor free until the next pause.
3. **A topmost window over the client area.** A notification, a game overlay or any always-on-top window
   inside the window's area receives the button-up when the cursor is over it, because there is no capture.
4. **Windows holding the button itself** (ClickLock, or a mouse vendor's remapping program), the review's
   candidate (2) of M5c: not affected by the flag either way.

Whether any of these happens on the owner's machine is not known. The acceptance test (drag-firing against
every edge at 100%, 125% and 150% scaling, `--input-log`, left downs equal to ups and no `resize` line) tests
the flag against the cause the log points to; it would show case 1 only with a second monitor at another scale.

### The options if it still happens (none taken; the owner's ruling)

| Option | What it is | Cost |
| --- | --- | --- |
| Library upgrade | A newer miniquad with a mouse capture | **None exists.** 0.4.11 is the newest release on crates.io (`cargo info miniquad`, 2026-11-20), and a code search of the upstream repository finds no `SetCapture` (2026-11-20). A git revision would be an unreleased, unreviewed dependency, and it would still lack the capture. |
| A patch to the library | A copy of miniquad 0.4.11 in the repository (or a fork), used through `[patch.crates-io]`, with about ten lines in `src/native/windows.rs`: `SetCapture(hwnd)` on a button-down and `ReleaseCapture()` when no button is down, so every button-up comes to the window wherever the cursor is; or reading the button-up flags of the raw input the library already receives (`WM_INPUT`, 526-556, ignores them) | The unsafe calls live inside the library, which is all FFI already, so the workspace's `unsafe_code = "forbid"` lint (it applies to our crates) is untouched. But it is code we carry: about 0.9 MB of vendored source or a fork to keep in step, its own review, and the Windows CI jobs building it. Licence: miniquad is MIT or Apache-2.0; the MIT choice is compatible with GPL-2.0-only (Apache-2.0 alone is not). |
| A reviewed exception to the no-unsafe rule | `doom-play` calls `SetCapture` and `ReleaseCapture` itself from its button handlers through a Windows API crate (miniquad exposes no window handle, so the handle would come from `GetActiveWindow` on the window's own thread) | A new dependency (`windows-sys` or similar), a separately reviewed policy and safety proof, as AGENTS.md requires, and a small amount of Windows-only code that no Linux test can run. |

## A2. Sound latency on Windows: statistics and trial options

Commits `5ffb210`, `6b024a7` and `fd442ee` on the branch (cherry-picked from the agent's worktree, where they were
`5421b5c`, `8d40286`, `b6d6196`). Everything is in `crates/doom-play`; the declared mixing schedule, the mixer and every golden are
untouched, and the defaults are unchanged. One shared line of the existing test file `session_tests.rs` changed:
its `use crate::audio::{...}` import gained `Trial` for the new tests; no existing test's body or assertion changed.

### (a) The exit lines

Whenever a device is open, with or without `--tic-stats`, four lines follow `doom-play: ` on exit (they replace the
counters line `--tic-stats` printed before). With no device (`--no-sound`, or no backend opened) nothing new is
printed. The text below is what a unit test asserts for a headless sink; the backend and figures are test inputs,
not a measurement:

```
audio device: backend WASAPI; name "Speakers (Test)"; native sample rates 44100 Hz, 48000 Hz; stream 11025 Hz
audio settings: period asked 512 frames; start threshold 1024 frames (2 buffers); granted period and buffer in miniaudio's [INFO] report at open
audio callbacks: 40 calls; frames per call min 441, max 512
audio queue: depth in frames when a callback arrived (not paused) min 0, mean 825.2, max 1536 (38 samples); underruns 2 (99 frames of silence); overruns 1 (512 frames dropped)
```

- **Backend, device name, native sample rates:** the backend that opened; `get_name` and `get_info` of the device,
  read after it is built and before it starts (maudio 0.1.15 `src/device.rs:241-256`).
- **The period and buffer the device granted cannot be read in safe Rust through maudio 0.1.15.** It has no accessor
  for miniaudio's internal period, period count or device rate, and no way to register a log callback of our own
  (`private_device_b` is `pub(crate)`, `src/device/device_builder.rs:301`, `563`; the log FFI is private,
  `src/logging.rs:198-257`). So `doom-play` turns on miniaudio's own INFO messages before building the device
  (`print_level(LogLevel::Info)`, `src/logging.rs:151-188`), and miniaudio's device report is printed once on standard
  error at open, as lines starting `[INFO]`, among them `Sample Rate: <stream> -> <device>` and
  `Buffer Size: <period>*<periods> (<buffer>)` (`miniaudio.h:43887-43937`). By the miniaudio source nothing else is
  posted at INFO during play on WASAPI, PulseAudio or ALSA; on DirectSound (the second Windows backend, used only if
  WASAPI fails) one line can appear per playback stall (`miniaudio.h:26724-26736`).
- **Callbacks:** every call counted, with the fewest and most frames in one. **Queue depth:** sampled at each
  callback before it takes frames, while not paused; min, mean (rounded to a tenth) and max in frames.
  **Underruns and overruns:** the counters that existed.

### (b) The trial options

In a separate block of `--help` headed as experimental, presentation only, for the owner's M5d trials; a setting
away from its default is marked `EXPERIMENTAL` on the `audio settings` line. Values outside the listed sets are
refused with exit code 2 and a message. They reach only the device sink; a test shows a one-buffer, 128-frame trial
gives the same tics, frames, sound reports, stream and demo bytes as `--no-sound`.

| Option | Values (default) | What it changes |
| --- | --- | --- |
| `--experimental-audio-start-buffers N` | 1, 2 (2) | The queue's start threshold (after the start, every gap and every resume) |
| `--experimental-audio-period-frames N` | 128, 256, 512 (512) | The device period asked for. With miniaudio's fixed-size callback (maudio's default) the callback then always gets exactly N frames (`miniaudio.h:43798-43836`, `20271-20302`) |
| WASAPI period count, 2 or 3 | **left out** | Not reachable through maudio 0.1.15's safe API: `ma_device_config.periods` (`miniaudio.h:7108`) is written only through the crate-private `inner`, no builder method sets it, and at 0 miniaudio uses `MA_DEFAULT_PERIODS` = 3 (`miniaudio.h:12202-12204`). |

The WASAPI period count could be had only by (i) a maudio change or fork with a `periods` setter, (ii) unsafe FFI on
`maudio-sys` (an exception to the workspace's unsafe ban), or (iii) a build-time route **found but not tried**:
`MA_DEFAULT_PERIODS` is guarded by `#ifndef` (`miniaudio.h:12202`) and the `cc` crate passes `CFLAGS` to the C build,
so a build with `CFLAGS=/DMA_DEFAULT_PERIODS=2` would use two periods for every backend in that binary. Each is the
owner's decision; none is taken.

**What each option is expected to change, by arithmetic** (the link model of the
[M5c latency record](M5C.md#sound-latency-the-arithmetic-in-full), assuming the backend grants what is asked; not
measured). From the call that starts a sound to the end of the buffer WASAPI is asked for:

| Start buffers, period | WASAPI, to the end of the requested buffer |
| --- | --- |
| 2, 512 (default) | 167.9 to 260.6 ms |
| 1, 512 | 139.3 to 232.0 ms |
| 2, 256 | 98.2 to 167.7 ms |
| 1, 256 | 69.7 to 139.1 ms |
| 2, 128 | 63.4 to 121.3 ms |
| 1, 128 | 34.8 to 92.7 ms |

The original built with its in-process sound code: above 0 to 92.9 ms on the same basis. A one-buffer start leaves no
margin (gaps are likely early in every session and after every pause); smaller periods mean two or four times as many
callbacks and less room against Windows' engine; under WASAPI's automatic conversion miniaudio's low-latency
IAudioClient3 path stays off (`miniaudio.h:23885`), so Windows may grant more than asked. The two-buffer margin was
computed for 512-frame callbacks; it was not recomputed for 128 or 256.

### Observed on this machine (Linux under WSLg; silence only)

`doom-play --audio-probe` (digital silence for about 1 s): backend PulseAudio, device "RDP Sink", native rate
44,100 Hz, stream opened at 11,025 Hz with no resampling in miniaudio (`Sample Rate: 11025 -> 11025`), 23 callbacks
of exactly 512 frames. **The RDP sink granted three quarters of the requested length:** `Buffer Size: 384*4 (1536)`
for a 512-frame request. With a scratch build of the probe at 256 and 128 frames (not the committed binary; reverted):
44 callbacks of 256 and 88 of 128 frames, granted 192 and 96 frames. The M5c arithmetic assumed 512 frames on
PulseAudio; with 384 the default PulseAudio total would be 63.4 to 156.1 ms (arithmetic, one probe run).

### Checks (the agent's run, Linux)

fmt (workspace and fuzz), clippy with warnings denied on `doom-game` and `doom-play` (debug and release), tests in
debug and release (166 passed, 0 failed; `doom-play` 35, from 26), `cargo +1.85.0 check`, rustdoc with warnings denied
(with and without private items). Fault plants: 22 planted, 20 caught at first; one survivor (min and max swapped in
the callbacks line) led to a stronger test in `fd442ee`, and one is equivalent (`resume` clearing a flag `pause`
already cleared). Not run: anything on Windows or macOS, a windowed run with a device.

**Open for the owner:** keep miniaudio's INFO report at every launch (about 18 lines on PulseAudio here) or put it
behind an option, and limit it to the open (remove the INFO callback after start) in case DirectSound stalls repeat;
whether to pursue the WASAPI period count by any of the three routes above.

## A3. A session that cannot be lost

**The change.** Commits on the branch: `1727f91` (doom-game: opt-in per-tic append of the session demo), `364290e`
(its test), `4274723` (doom-play `session_file.rs`: the session demo kept valid at every instant), `0d250a5` (the kill
test and the unit tests of the session file), `eceb6f6` (aimed kills in the kill test), `e25388d` (the two tools,
`tools/reference/m5d_kill_check.py` and `tools/local/m5d_lost_display.py`). Nothing in `doom-sim`, `doom-render` or
`doom-sound` changed, and no existing test line, wire, schema or golden. Every figure below was observed on this machine
(Linux 6.18 under WSL2, x86-64) unless it is marked as inference.

### The C citations for the marker (linuxdoom-1.10)

- `#define DEMOMARKER 0x80`: `g_game.c:1488`. `G_ReadDemoTiccmd` (`g_game.c:1491-1503`) tests only the byte at the start of
  a tic for the marker (1493), then calls `G_CheckDemoStatus` (1496), which ends the playback (1647-1673); otherwise it
  reads four bytes (1499-1502).
- The recording engine writes the marker once, when recording ends (`*demo_p++ = DEMOMARKER;` and `M_WriteFile`,
  `g_game.c:1679-1680`).
- Without the marker the original cannot replay the file safely: `G_DoPlayDemo` takes the lump from `W_CacheLumpName`
  (`g_game.c:1588`) and never checks a length, and `W_CacheLumpNum` allocates exactly `W_LumpLength (lump)` bytes
  (`w_wad.c:490`), so the reader would run past the lump's allocation.
- The header is 13 bytes (`G_BeginRecording`, `g_game.c:1555-1566`), so tic N's command starts at 13 + 4N and the
  marker of a file of N tics is at 13 + 4N. A tic's first byte is `forwardmove`, and wire 24 refuses forward -128
  (`wire24_admits`), so no recorded tic starts with 0x80 and the reader never stops early.
- doom-sim's frozen reader (`parse_demo_episode1_v2`, `crates/doom-sim/src/demo.rs:179-233`, unchanged) refuses a missing
  marker and any byte after it, so the file must be a complete demo at every instant, not only at the end.

### The design

- **One 5-byte write per tic.** After each committed, recorded tic the file is extended by one `write` of five bytes at
  the old marker's offset: the tic's four command bytes and a new marker. The bytes come from doom-sim's own writer
  (`Recorder::last_tic_tail`), so they equal what the whole writer would produce. The file grows by 4 bytes per tic and
  is a valid demo before and after the write.
- **Page-boundary rule.** Five bytes that would straddle a 4,096-byte boundary are not appended: `append` declines
  (`append_fits_one_page`) and the app rewrites the whole file through a synced temporary file and a rename. The marker
  offset 13 + 4N is 1 modulo 4, so this happens exactly after 1,020 + 1,024k tics.
- **A claim that appears whole.** A new session's file (14 bytes: header and marker) is written to `<name>.<pid>.claim.tmp`
  (created new), synced, and hard-linked to the name (`std::fs::hard_link` fails if the name exists, so an existing file
  is never replaced); the temporary is then removed. The whole rewrite uses its own name, `<name>.<pid>.tmp`, also
  created new (review MIN-5: if the claim's removal failed, that name is a second link to the session's file, and a
  rewrite through the same name would truncate it in place). **The fallback window:** if the link fails for any reason but the name
  existing (no hard links: FAT, exFAT, some network shares), the claim falls back to M5b's `create_new`, write and sync,
  and a kill between the create and the write can leave an empty `.lmp`.
- **Hosts that do not opt in are unchanged.** `Host::appends` defaults to false and `Host::append` is never called for such
  a host; the test hosts and every example keep M5b's cadence exactly. `FsHost` opts in; `LoggedHost` delegates.
  The whole synced rewrite stays every `SAVE_INTERVAL` (1,050) tics, at the session's end, and on restart or quit when
  the file is behind. A declined append is repaired by a whole rewrite on the next tic.
- **After a failed write** (review MIN-1; commit `a62458d`, constant `FAILED_WRITE_RETRY_TICS` = 35 in
  `crates/doom-game/src/app/mod.rs`): the app retries a whole rewrite at the next recorded tic; while retries keep
  failing, the next one waits 35 recorded tics (one second of play). The `SAVE_INTERVAL` write, the end, restart and quit
  are unconditional as before, any successful whole rewrite ends the wait, and appends resume only after one. The value
  and the immediate first retry are the orchestrator's choice and can be changed (open question 1).

### The atomicity argument (it rests on a reading of the kernel's behaviour, not on a test)

The agent read Linux v6.18's source (not committed): a buffered write to a regular file on ext4 or tmpfs goes through
`generic_perform_write` (`mm/filemap.c:4227-4318`), which checks for a fatal signal only at the top of each folio
iteration, before the copy; the copy and the size update finish before the next check. Folios are page-aligned and at
least one page, so five bytes inside one aligned 4,096-byte block are one iteration: SIGKILL either prevents the write
or lets it finish. That is why the page-boundary rule exists. The line numbers are the agent's reading of a source that
is not committed and that the independent review could not check. **A residual case:** a short copy from the five source
bytes (a fault on the caller's buffer, just written and resident) could stop inside a folio; it is not expected and was
not observed. This is an argument about one kernel's implementation, not a guarantee of every file system, and not covered by a test.

**What the tests pin instead** (`crates/doom-play/src/session_file.rs`, commit `d653975`; each was planted to fail):
`an_append_is_one_write_call_of_the_five_bytes_at_the_marker` (one `write` call of five bytes at the marker's offset,
through a recording stand-in; a short write is not followed by a second call), `a_whole_rewrite_replaces_the_file_and_never_writes_the_old_one`
(the inode under the name changes and a second link to the old file keeps its bytes), `a_claimed_name_is_the_temporary_file_or_the_fallback_says_so`
(the name is the temporary file's inode, or an injected link failure takes the fallback) and
`a_failed_removal_of_the_claims_temporary_never_lets_a_rewrite_write_in_place`. A refactor that bypassed the function
the first test inspects inside `append` would not be caught by it. The kill test is evidence for the steady state only (no
gross fault: a file that falls behind, loses its marker or replays differently); it cannot land a kill inside a window of
microseconds. Its self-test, `a_torn_append_fails_the_kill_test`, shows that it catches a torn append whose two writes are a
millisecond apart (see Evidence A).

**Windows and macOS (inference, not tested).** On Windows, Ctrl+C and the console's close button reach a process with no
console handler as `CTRL_C_EVENT` and `CTRL_CLOSE_EVENT`, whose default handler calls `ExitProcess`; End task calls
`TerminateProcess`. A thread inside a synchronous, cached `WriteFile` of five bytes on NTFS is expected to finish the
copy into the cache before it is terminated, so the five bytes should land whole; the page rule and the whole-rewrite
path are the same there. On macOS only the CI run of the kill test (`#[cfg(unix)]`) speaks for it; it had not run when
this was written.

### When the demo is written (FsHost, as `doom-play` runs it)

| Moment | Write |
| --- | --- |
| Session start | The file appears with its content (14 bytes): a synced temporary file hard-linked to the name (fallback without hard links: `create_new`, then the write) |
| Every committed, recorded tic | One write of 5 bytes at the old marker's offset; the file grows by 4 |
| A tic whose 5 bytes would straddle a 4,096-byte boundary (the tic after 1,020, 2,044, 3,068, ... tics) | Rewritten whole through a synced temporary file and a rename |
| Every 1,050 recorded tics | Rewritten whole the same way (unchanged) |
| The tic after a failed write, then at most once every 35 recorded tics while writes keep failing | Rewritten whole (repairs the file) |
| Session end (exit, death, a refusal, the tic cap) | Rewritten whole (unchanged) |
| Restart, quit | Nothing when the file is current (the normal case); rewritten whole when behind |
| Pause (Esc, Pause, focus loss, minimise) | Nothing (no tic runs) |
| Ctrl+C, SIGKILL, a lost X connection, a closed console window | Nothing at that moment, but the file already holds every tic run, or every tic but the last |

### Evidence (release builds unless stated)

**A. The cargo kill test** (`kill_tests::sigkill_at_random_points_leaves_valid_demos`, `crates/doom-play/src/kill_tests.rs`,
Unix only, so it runs in CI on Linux and macOS; commands in [Testing](../TESTING.md#m5d-kill-tests-and-trial-options)).
The parent spawns its own executable, which plays the real `App` with the real `FsHost` (sound discarded) on the M5c
session-check synthetic set with seeded keys, mouse motion, buttons, pauses and restarts on a fake clock, and SIGKILLs it
after a random delay. The parent requires that the child died by `SIGKILL` itself (`ExitStatusExt::signal() == Some(9)`; a child that exits or
panics by itself is a harness failure carrying the tail of its standard error, review MIN-2). A checker requires of every `.lmp`: not empty, parses, replays with no refusal, its command digest
equal to the child's printed one at its tic count, and M - 1 or M tics for the current session (earlier sessions exactly
their last count), with at most one file whose claim was never printed, holding 0 tics. The default run is 50 kills: 15.0
to 15.3 s for the whole `doom-play` test binary in debug (31 passed, 1 ignored), 14.6 s in release with 3 sessions past
the first page-boundary rewrite. Two planted faults were caught (a two-step append, at kill 8 of 30; a claim with no
content, at kill 7 of 50). On this machine's tmpfs `/tmp` no kill landed before a claim line; with `TMPDIR` on ext4, 8 of
50 did.

*What a run reaches depends on the file system and the build (review MIN-4).* The review's observations: a debug build
on tmpfs (CI's configuration) reached no claim and no rewrite (0 of 50) and ended before the first page boundary; a release
build on tmpfs had 3 files past it; a debug build on ext4 had 8 to 12 claim kills in 50 (each left a temporary file and no `.lmp`); aimed kills on tmpfs landed inside no rewrite (0 leftover temporaries in 80), and on ext4 with a
8 ms window 24 of 30 kills landed in one. CI's default run therefore mainly exercises the steady-state append; the claim and
the rewrite are exercised by a disk-backed `TMPDIR` and by aimed runs (the counts below carry their file system, build and
aim). **The self-test:** `DOOM_PLAY_KILL_PLANT=torn` (read only by the test child; self-test only) makes the child append in
two writes 1 ms apart; `a_torn_append_fails_the_kill_test` (runs by default in `cargo test -p doom-play`, aimed at tic 30,
at most 40 kills) must see a check failure "is not a valid demo", and did at the second kill in every run
(`UnterminatedCommands`, 133 bytes = 13 + 4 x 30).

**B. Many kills locally** (`DOOM_PLAY_KILLS=2000`, seeds 1000 to 2999, ext4, release build as in A, no aim): pass; 2,000 kills, 2,126 files holding
819,826 tics (longest 3,315; 206 files past the first page-boundary rewrite); 414 kills landed before a claim line, 4 after
a whole-rewrite tic, 1,582 mid-session; 0 failures. Aimed runs (same build and file system as B; `DOOM_PLAY_KILL_AT_TIC` and `_AT_US` as in the table), 0 failures in all:

| Aim | Kills | Window after the tic's print | Files / tics | Current file 1 behind / even | `.tmp` |
| --- | --- | --- | --- | --- | --- |
| tic 1,021 (first page-boundary rewrite) | 200 | 0 to 3 ms | 411 / 306,048 | 200 / 0 | 200 |
| tic 1,050 (first periodic rewrite) | 200 | 0 to 3 ms | 424 / 319,597 | 200 / 0 | 200 |
| tic 1 (first append) | 200 | 0 to 3 ms | 201 / 486 | 18 / 182 | 0 |
| tic 1,021 | 150 | 0 to 60 ms | 294 / 220,796 | 38 / 112 | 33 |
| tic 1,050 | 150 | 0 to 60 ms | 299 / 227,759 | 33 / 117 | 24 |

Every kill within 3 ms of tic 1,021 or 1,050 found the synced whole rewrite unfinished and the old file whole; with a 60 ms
window both states occur and pass.

**C. The session check against the genuine engine** (`tools/reference/m5d_kill_check.py`): every surviving `.lmp` goes
through the M5c session check (the tool builds the test executable in release, the aim is in the table's first column, and the temporary file system was not recorded; `m5c_session_real.py`'s `verify`: linuxdoom-1.10 built with its in-process sound code in
the strict, overflow and address variants, and the genuine mixer, against the Rust replay; every frame, sound row, mixer
buffer and the tic count compared). **The address variant ran with `ASAN_OPTIONS=detect_leaks=0`: leak detection was
disabled, so it is not LeakSanitizer coverage.** The strict32 variant was not needed in any run.

| Run (the tool builds release; the temporary file system was not recorded) | Kills | Surviving demos | Tics | Verdicts |
| --- | --- | --- | --- | --- |
| synthetic, 200 kills, seed 100, `--selftest` | 200 | 195 | 66,633 | 195 PASS, 0 FAIL |
| synthetic, aimed tic 1,021 (0 to 60 ms), seed 900 | 30 | 63 | 46,373 | 63 PASS, 0 FAIL |
| IWAD E1M1 (scratch copy), 50 kills, seed 300, `--selftest` | 50 | 46 | 6,176 | 46 PASS, 0 FAIL |
| IWAD, 200 kills, seed 400, longer delays | 200 | 242 | 106,093 | 242 PASS, 0 FAIL |
| IWAD, aimed tic 1,021, seed 700 | 30 | 55 | 40,632 | 55 PASS, 0 FAIL |
| IWAD, aimed tic 1,050, seed 800 | 30 | 55 | 42,112 | 55 PASS, 0 FAIL |

Totals: synthetic 230 kills, 258 surviving demos, 113,006 tics; IWAD 310 kills, 398 surviving demos, 195,013 tics; all
PASS. In the first synthetic run the frames compared were 66,633 (0 masked, 0 not comparable) and 43 kills left no `.lmp`
(they came before the claim). At the head of those runs, `--selftest` planted a truncated last tic with no marker, a missing marker and a byte
after the marker: each failed the session check, and the self-check passed both times. Review MIN-3 found that those
three are refused by the header check, so the genuine comparison never ran on them. Since commit `cdb11cc`, the three file plants are labelled "refused by the header check" and a fourth plant changes one command
byte (the turn of the middle tic) in the demo fed to the Rust side only, passes the header and must fail the genuine
comparison at the planted tic or later; a run with that plant made vacuous fails the self-check (`SELF-CHECK: FAIL`). The
tables above were observed before this change. A 0-tic file (header and marker only)
also passed by hand. Only counts and verdicts are recorded; the IWAD was a scratch copy.

**D. The real binary under WSLg** (`tools/local/m5d_lost_display.py`, IWAD scratch copy, `--no-sound`, nothing pressed):

| Mode | Runs | Exit | Surviving demos | Tics | Verdicts |
| --- | --- | --- | --- | --- | --- |
| lost display connection (X proxy shut), 0.5 to 8 s | 20 | 1 in all 20, each with "X connection to :42 broken (explicit kill or server shutdown)." | 20 | 3,109 | 20 PASS |
| lost display connection, 32 to 75 s | 5 | 1, same message | 5 | 8,878 (all past tics 1,021 and 1,050) | 5 PASS |
| SIGINT (what Ctrl+C sends), 0.5 to 8 s | 20 | killed by signal 2 in all 20 | 20 | 3,116 | 20 PASS |
| SIGKILL, 0.5 to 8 s | 20 | killed by signal 9 in all 20 | 20 | 3,117 | 20 PASS |

The proxy: WSLg mounts `/tmp/.X11-unix` read-only, so the proxy listens on the abstract socket `@/tmp/.X11-unix/X42`,
which libxcb tries first for `DISPLAY=:42`, and forwards byte for byte to the real socket; shutting it reproduces the
owner's message. A manual SIGINT run with `--input-log` left a file holding exactly the 135 tics the log listed.

### Which of the owner's three cases were run here

| Case | Run here? |
| --- | --- |
| Lost display connection under WSLg | **Yes, through a proxy** (table D), not by the owner's own session losing its display |
| Ctrl+C | **SIGINT on Linux: yes** (table D). **Ctrl+C in a Windows console: no** |
| Closed console window on Windows | **No; only the owner can** (End task was not run either) |

The kill test on macOS was not run (CI's job), and nothing was run on Windows. The Windows steps for the owner's session
belong to A6.

### What a player could notice

- Disk activity: 35 small writes a second while playing (open, seek, 5-byte write, close), instead of one rewrite every 30 s.
- One more synced whole rewrite every 1,024 tics (about every 29 s), beside the existing one every 1,050. On this
  machine's ext4 it took longer than 3 ms in all 200 aimed kills at tic 1,021 and 1,050 (the kills found it unfinished); an
  fsync stall of that size already happened every 1,050 tics before. Not measured as a frame-time effect.
- After a failed write the app retries a whole rewrite at the next tic and then at most once every 35 tics while writes keep
  failing (M5b retried at 1,050 tics, the end, restart and quit, which still happen).
- Leftover files can appear in the sessions directory and nothing removes them: `<name>.<pid>.claim.tmp` (after a kill
  during a claim, or a removal that failed) or
  `<name>.<pid>.tmp` (after a kill during a whole rewrite). Never a `.lmp` that fails.
- The game, the frames, the sound and the demo bytes do not change: the existing session and input-log tests, which compare
  demo bytes and frames, pass unchanged.

### Checks the agent ran (Linux; before the review fixes)

Format (workspace and fuzz) clean; Clippy with warnings denied on `doom-game` and `doom-play`, debug and release, clean;
`cargo test -p doom-game -p doom-play` 165 passed, 0 failed, 1 ignored, in debug and in release; `cargo +1.85.0 check` on the
two crates passes; rustdoc with warnings denied (with and without private items) clean. Fault plants: 11 of 11 caught (the
two above and nine more: page rule always true, append without the length check, a claim by `create_new` then write, a claim
leaving its temporary, always rewriting whole, an append offered to a host that did not opt in, an append while behind, the
offset of n tics instead of n - 1, a declined append marked saved). **Not run in that step:** the workspace-wide gates, the
fuzz builds, sanitizer campaigns of Rust code, CI on any operating system, Windows and native macOS. Those are A5.

### Open questions for the owner

1. **Retry after a failure (the orchestrator chose; the owner to rule).** After a failed write the app retries a whole
   rewrite at the next tic, then at most once every 35 tics (`FAILED_WRITE_RETRY_TICS`) while it keeps failing, in place of
   the first version's every tic (review MIN-1: up to 35 synced rewrites a second under a persistent failure). The first
   retry is immediate because the existing `app_append` test pins an immediate retry, and no existing test may change in this
   milestone. Is one second right, should even the first retry wait, or should a failure retry only at `SAVE_INTERVAL` and the end?
2. **Fallback claim.** A hard-link error other than "exists" falls back to `create_new`, whose kill window can leave an
   empty `.lmp`. NTFS supports hard links (inference); FAT and exFAT do not.
3. **Leftover temporary files** (`*.claim.tmp` and `*.tmp`) accumulate after kills or a failed removal, and no later run
   removes them (review N8). Should a later run remove stale ones it did not create? A failed removal of the claim's
   temporary is still ignored, now harmless to the rewrite (MIN-5).
4. **The residual short-copy case** above, and the kernel-dependence of the whole argument, stay open until Windows and
   macOS are shown by hand.

## A4. Leftovers by the owner's earlier rulings

Commit `30e535b`:

- **The mouse forwarding** moved unchanged from `crates/doom-play/src/input_log.rs` into
  `crates/doom-play/src/mouse_forward.rs` (`Forward`; the log's button names became `pub(crate)` so the new
  module can use them). The window and the log's test use it from there; no existing test of `input_log.rs` changed (they
  reach `Forward` through a test-only import in the parent module); one new test was added (M1, below).
- **M1:** the log names the session's demo file on a new `session` line when the file is created, and gives
  each committed tic's executed command and its fire bit on a new `tic` line (`tic PUMP TIC cmd forward=F
  side=S turn=T buttons=B fire=0|1`), after that tic's `game` lines. A new test checks the lines against the
  demo's bytes, tic by tic. An unrecorded session creates no file and has no `session` line.
- **M2:** the test comment in `crates/doom-play/src/audio.rs` (the schedule loop of the constants test) now
  says two buffers leave at least one tic, 315 frames, as the constant's own comment does since `b141767`. Only
  the comment changed.
- **M3:** the log's module comment states the exceptions to its pump rule (a `capture` line carries the pump of
  the update in which it was written; a release held back by decision S6 reaches the game in a later pump; inputs
  before an update that runs no tic have no `game` line) and that on Windows the position on a button line is
  that of the last cursor-motion message.
- **The CI job** "M5c genuine sound (not required)" is now named "M5c genuine sound"
  (`.github/workflows/rust.yml`; job id `m5c-genuine-sound` unchanged), so that the owner can make it a required
  check. TESTING was updated to the new name.
- **Reviews:** from this milestone on, this record states the model and effort of every independent review
  (below, under A5).

Checks run on this commit (Linux, debug, `CARGO_PROFILE_DEV_DEBUG=0`): `cargo fmt --all`, `cargo clippy -p
doom-play --all-targets -- -D warnings` clean, `cargo test -p doom-play` 26 passed, 0 failed. The full gates
follow under A5.

## A5. Gates

### Independent review

- **Reviewer:** Claude Opus 5.5 (`claude-opus-5-5`), effort **xhigh**. The orchestrator set the effort at spawn, as the
  [model routing table](../../AGENTS.md#model-routing-for-subagents) requires for the reviewer role; the reviewer itself could
  not see the setting from inside and said so. A fresh agent that wrote none of the code and saw no author reports (it read
  the diff, linuxdoom-1.10, the library sources in the cargo registry and this record).
- **Reviewed:** `d637f7d..e25388d`, in a scratch worktree, read-only on the branch, with planted faults in the scratch
  copy only (all reverted). The branch head had since moved to `e1d73dd` (a docs commit), which it did not review.
- **Verdict:** no blockers; one major and six minor findings, and nine notes. The review's own commands and counts are in
  its report (scratch; not committed). It ran the doom-play unit tests (40 passed, 1 ignored), `app_append` (3 passed),
  `cargo test --release -p doom-game` (134 passed), format, Clippy and rustdoc on the two crates, and the kill test in
  several configurations (all passed). It ran nothing on Windows or macOS and no MSRV build, full workspace test, fuzz build
  or sanitizer campaign.
- **The fixes were re-reviewed** by a second fresh reviewer (next subsection).

| Finding | What it said | What was done |
| --- | --- | --- |
| MAJ-1 | No test failed when the all-or-nothing write was broken (planted: the append as two writes, the rewrite done in place, the hard link never used); the kill test cannot land a kill inside a microsecond window and had no self-test | Fixed in `72a7ebb` (seams `write_tail` and `claim_with`, no behaviour change), `d653975` (deterministic tests: one five-byte write call, a rewrite that replaces the file, the claimed name is the temporary's inode) and `bcba352` (the kill test's self-test `a_torn_append_fails_the_kill_test`, `DOOM_PLAY_KILL_PLANT=torn`; the wording of `session_file.rs` now says the argument rests on the kernel's source). Re-planted by the fixing agent: the two-write append is caught by the one-write test, the in-place rewrite by the replace test, the missing link by the inode test (and the default kill test caught the 200 microsecond variant once, on ext4). Limit: a refactor that bypassed `write_tail` inside `append` would not be caught |
| MIN-1 | After a failed write every tic attempted a synced whole rewrite | Fixed in `a62458d` (`FAILED_WRITE_RETRY_TICS` = 35; first retry at the next tic), tests in `a1d4a87` (`tests/app_retry.rs`, 4 tests; plants of no throttle, interval 34, a throttle inside `save`, a success not clearing it, and the old every-tic retry were each caught). The value is the orchestrator's choice: open question 1 in A3 |
| MIN-2 | The kill test counted a child that died by itself as killed; stderr discarded | Fixed in `bcba352`: the status must be `SIGKILL` (9), the child's stderr tail goes into every failure. A planted child panic is caught by the default kill test (and the old `status.success()` check let it pass) |
| MIN-3 | `m5d_kill_check.py --selftest` plants never reached the genuine engine | Fixed in `cdb11cc`: the three file plants are labelled as refused by the header check, and a fourth plant (one command byte, Rust side only) passes the header and must fail the genuine comparison at the planted tic or later; with the plant made vacuous the self-check fails |
| MIN-4 | What the kill test reaches depends on the file system and the build; CI's default run reaches only the steady-state append | The kill test's module comment now says so (`bcba352`), and the counts in A3 carry their build and file system where they are known. A CI step with aimed kills on a disk-backed temporary directory was **not added** (left for the owner) |
| MIN-5 | A failed removal of the claim's temporary made every later whole rewrite non-atomic | Fixed in `72a7ebb` (the claim's temporary is `NAME.PID.claim.tmp`, the rewrite's `NAME.PID.tmp`, both created new), tested in `d653975` (`a_failed_removal_of_the_claims_temporary_never_lets_a_rewrite_write_in_place`; the names-equal plants and `File::create` were caught) |
| MIN-6 | The granted rate, period and buffer are not on the exit lines (a library limit) | **Left for the owner.** Requirement A2(a) is met for these three values at launch (miniaudio's `[INFO]` report on stderr), not at exit |
| N1 | The kill test's bound is sound | No action |
| N2 | The DPI test pins only the constant | Partly fixed in `b608542`: a test of the window configuration (`window_conf`). Deleting the `high_dpi` line is caught only by the Windows run (the default equals the constant elsewhere); not run on Windows |
| N3 | "At most a pixel short" holds only up to 200% | Fixed in `b608542` (the comment) and in A1 above: the shortfall is less than the scale plus one pixel; the review's own figures were also off at 175% |
| N4 | The module doc's opening promise was wider than its argument | Fixed: the opening of `session_file.rs` names the file systems it covers and those it does not |
| N5 | Windows console close: with no handler the default ends the process at once (the "after its handler times out" wording) | **Left, unverified.** `session_file.rs` still says the console's close "ends the process the same way after its handler times out"; not checked on Windows. For the owner's Windows session (A6) |
| N6 | Commit hashes not on the branch, "textually unchanged", "case 0" | Fixed in `95d27b7` and, for one more hash, in this commit |
| N7 | COMPATIBILITY stale (30 seconds; 14-byte demo) | The session-file paragraph that still said "loses at most 30 seconds" now points to the per-tic file; the limits section was already current |
| N8 | Leftover `.tmp` files pile up | Documented here, in COMPATIBILITY and in the module doc; no cleanup code (open question 3 in A3) |
| N9 | ROADMAP still says "M5c genuine sound (not required)" in the M5c history | Left as history |

### Second independent review (after the fixes)

- **Reviewer:** Claude Opus 5.5 (`claude-opus-5-5`), effort **xhigh**, set by the orchestrator at spawn (the reviewer cannot
  see the setting from inside). A fresh agent that wrote none of the code. **Reviewed:** `e1d73dd..cdb11cc`, in a scratch
  worktree. Its report is in scratch (not committed).
- **Verdict:** one blocker, introduced by the fixes (B-1), and one minor gap (M-1), both fixed since; three notes.

| Item | Status or what was done |
| --- | --- |
| MAJ-1, MIN-1, MIN-2, MIN-3, MIN-5 | Resolved (planted faults in the scratch copy were caught; the reviewer lists them). MAJ-1 had the one gap M-1 below |
| MIN-4 | Resolved in the code's documentation; the optional CI step with aimed kills was not added |
| N2 | Resolved on Windows CI only, which is all the design allows (a deleted `high_dpi` line cannot fail on Linux or macOS) |
| **B-1** (blocker) | A claim test compared inode numbers after the temporary had been removed; ext4 reuses the number, so the test failed 19 of 20 runs locally on ext4 and failed the Linux CI jobs `Stable / ubuntu-latest` and `Minimum Rust / 1.85.0` at `14a9cc6`. **Fixed by the orchestrator in `6e61fc8`:** a second handle keeps the inode allocated across the assertion; 25 of 25 runs on ext4 passed |
| **M-1** | The real hard link in `claim_new` was unpinned: replacing it by an error left every test passing on tmpfs. **Fixed in `6e61fc8`** by a new test, which was caught on ext4 and on tmpfs when the link is replaced by an error |
| N-A | No test covered `begin`'s reset of the retry counter (the planted fault survived). Covered by the tester (below): `a_restart_resets_the_failed_write_retry_counter` |
| N-B | The kill test prints harness failures with `{e:?}`, so the child's stderr tail shows `\n` escapes on one line. **Left:** it affects the failure message's readability only |
| N-C | The kill checker's fourth plant can report NOT CAUGHT if the player is dead at the middle tic. **Left:** it fails in the safe direction (a false alarm, never a hidden fault), and the player is unlikely to be dead there |

The fixes of the second review (`6e61fc8` and after) were not reviewed again by a fresh reviewer; the gates and CI below
are the evidence for them.

### Mutation runs

Three runs, stated separately. cargo-mutants finds changed lines by diff; none of these is a coverage claim for unchanged code.

1. **CI's mutation workflow** (`mutants.yml`, started by the label `run-mutants`). It mutates `doom-sim`, `doom-render`,
   `doom-sound` and `doom-game` on the changed lines, not `doom-play`. Result on GitHub: **success** on `e25388d` (run
   44713873716) and on `cdb11cc` (run 44723050975). Summary counts were not read from the logs.
2. **Local cargo-mutants 27.1.0 on `doom-play` and `doom-game`**, `--in-diff` against `d637f7d`, `-j 2`, 300 s timeout per
   mutant, a memory-capped test runner, on this machine (Linux under WSL2). Survivor list: scratch
   (`mutants/survivors-b.txt`; not committed).

| Run | Head | Mutants | Caught | Missed | Unviable | Timeouts | Time |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | `e25388d` | 327 | 194 | 118 | 13 | 2 | 44 min |
| 2 (kill-test harness excluded) | `cdb11cc` | 206 | 182 | 12 | 12 | 0 | 30 min |

   Of run 1's 118 missed, 95 were in `crates/doom-play/src/kill_tests.rs` (the kill test's own harness, test code, so run 2
   excludes it) and 23 in product code.
3. **The 12 survivors of run 2.** A tester agent (Claude Sonnet, effort medium, per the routing table) planted each fault by
   hand, wrote tests, and reverted. Its commits are `7249065`, `0e985b7` and `dfa1f37` (new test code only; the new file is
   `crates/doom-game/tests/app_append_survivors.rs`, and new test modules in `mouse_forward.rs`, `input_log.rs` and
   `audio.rs`). One change by the orchestrator: the tester's test of `probe()` opened a real audio device when one exists,
   against the milestone's rule that tests open no device (and the test depended on the machine), so it was dropped in `e6b17b2`. The
   dispositions:

| Survivor | Disposition |
| --- | --- |
| `Host::append` replaced by `Ok(true)` | Killed by `the_default_append_declines_so_the_file_is_rewritten_whole` |
| `App::append`: `&&` to `\|\|` (two places) | Killed by `a_host_that_opts_in_late_gets_a_whole_rewrite_before_its_first_append` |
| `App::append`: match guard `current` to `true` | Killed by the same test |
| `LoggedHost::append` replaced by `Ok(false)` | Killed by `the_wrapper_delegates_append_to_the_host` |
| `Forward::captured` replaced by `false` | Killed by `captured_follows_the_apps_wish` |
| `Forward::raw_motion`: `+` to `-`, and `-` to `+` | Killed by `whole_units_are_passed_on_and_the_fraction_is_kept` |
| `App::append`: `n > 0` to `n >= 0` | **Equivalent:** `append` is called only after a committed recorded tic, so `n` is at least 1; with the plant all tests still pass |
| `probe` returning `Ok(String::new())` or `Ok("xyzzy")` | **Unreachable in tests without a device** (the tester's test, which killed both, was dropped in `e6b17b2`; see above) |
| `probe`: `/` to `*` | **Unreachable in tests:** it is inside the device callback, which runs only with a real device |

   That makes 8 killed by new tests, 1 equivalent and 3 unreachable in tests. The tester's tests also cover N-A. After
   `e6b17b2` the tester's mutants were not re-run by cargo-mutants (the plants were by hand), so the three probe mutants
   stay surviving in a re-run.

### Full workspace gates

From a clean clone (`git clone` of the working repository, then a detached checkout) at
`6e61fc8af570b899b076ec195d10dcbaf4534cf3`, by a gates agent (Claude Sonnet, effort medium; the docs-tools role in the
routing table). Every gate passed. Everything below was observed there; the agent's report is in scratch.

**Conditions.** 2026-11-20, 03:17 to 05:19 PST, one machine (Linux under WSL2), all on ext4 under `~/scratch`. The owner said
the machine was idle, so no job caps (`CARGO_BUILD_JOBS=6`); a cargo-mutants run was still ending at the start and was gone
before the first build finished. `HEAD` and a clean `git status` were checked at the start and end of every phase. rustc 1.100.0,
`cargo +1.85.0`, nightly with cargo-fuzz 0.13.2, gcc 15.2.0, Python 3.14.4, valgrind 3.26.0. Debug information off for the
build profiles (output size only), on for the instruction counts as in CI. A scratch copy of `DOOM.WAD` and of the owner's 13
session demos; their digests matched the originals; `/mnt/d` was only read; only counts and verdicts are recorded.
**Disk:** `/mnt/c` showed 27 GB free at every check (floor 20 GB). **Leak detection:** every address-sanitized genuine C run
used `detect_leaks=0`, so those runs are **not** LeakSanitizer coverage; the fuzz replays and campaigns used
`detect_leaks=1` (a probe run on each binary printed "checking for leaks" once and exited 0).

| Gate | Result | Wall |
| --- | --- | ---: |
| Build, `--workspace --all-targets --locked` | passed, 0 warnings | 102 s |
| Format, both workspaces | passed | 5 s, 6 s |
| Clippy, warnings denied | passed, 0 warnings | 28 s |
| `cargo test --workspace --locked` | **1,906 passed, 0 failed, 5 ignored** (137 result lines) | 502 s |
| Rustdoc, warnings denied | passed, 0 warnings | 7 s |
| Release `doom-sim`, `doom-types` | **1,393 passed, 0 failed, 2 ignored** | 232 s |
| Release `doom-render`, `doom-sound`, `doom-game`, `doom-play` | **480 passed, 0 failed, 1 ignored** | 176 s |
| Rust 1.85.0, `cargo +1.85.0 test --workspace --locked` | **1,906 passed, 0 failed, 5 ignored**; one warning, the known dead-code `MAX_LINKED_THINGS` in `doom-sim` | 606 s |
| Instruction counts (job `Simulation instruction counts`) | `threshold 3.0%: ok`, 9 workloads; largest increase +0.92%; malloc and realloc counts equal to the baseline | 57 s build, 13 s check |
| Job `stable`, Linux steps after the Rust gates (the workflow's steps run in order) | 64 steps passed; 3 steps gated `runner.os != 'Linux'` skipped, as on CI's Linux leg; documentation check: 147 Markdown files, 3,086 relative links, 1,222 anchors, 20 exact archives | |
| Jobs `M5a genuine renderer`, `M5b genuine status bar, HUD and input`, `M5c genuine sound` | all steps passed, self-tests included (leak detection disabled in the address variants) | 198 s, 235 s plus 52 s, not timed for M5c |
| README animations, `--check` | both identical to the regenerated animations | 9 s, 25 s |
| Synthetic local tools (`m5c_real_full.py`, `m5c_session_real.py`) | self-checks pass; planted faults caught | 14 s, 27 s |
| Fuzz seed tools | checks and self-tests pass | about 1 s each |
| Fuzz build, AddressSanitizer, target compiled only | 60 of 60 targets built, 0 warnings | 113 s |
| Fuzz seed replays (7 targets) | all exit 0 | 13 s in all |
| Fuzz AddressSanitizer campaigns, 300 s each, 7 targets | no findings; 0 artifact files | 35 min |
| Kill check, synthetic (`m5d_kill_check.py`, 100 kills) | 118 files checked, 51,319 tics, **118 PASS, 0 FAIL** in the genuine session check; `SELF-CHECK: PASS` | |
| Kill check, IWAD copy (50 kills) | 48 files, 6,444 tics, **48 PASS, 0 FAIL**; `SELF-CHECK: PASS` | 138 s |
| Lost display, windowed `doom-play --no-sound` (display, SIGINT, SIGKILL; 4, 3 and 3 runs) | 10 of 10 PASS, 0 FAIL, no run without a demo | 43 s, 195 s, 37 s |
| Kill test, 500 kills, release, `TMPDIR` on ext4 | 502 files checked, 202,303 tics; landed in the claim 122 times and in the session 378; 119 kills left no `.lmp`; passed; torn-append self-test passed | |
| The owner's 13 session demos through the genuine engine | **13 of 13 `VERDICT: PASS`**, 27,146 tics (every tic's frame compared; the same totals M5c recorded) | about 730 s |

The fuzz targets chosen were those nearest the diff (`game_input_events`, `render_display_state`,
`render_display_state_v2` and the four sound targets). **No fuzz target exists for `doom-play`, its session file or its
`--input-log`, and none was invented.** The other 53 targets were built only.

**Slips (harness mistakes, none a product failure).** The first instruction-count run failed because the agent's script
unset the target directory the tool needs; fixed and re-run alone. The four fuzz seed-tool commands were first called at the
wrong path and re-run at the right one. A `pkill -f` killed the agent's own shell and left one orphan build, which it killed by PID.

**Not run:** Windows and macOS; the exact-head CI of GitHub (below) and the Windows artifact; any audio device or played
sound (the windowed runs used `--no-sound`); any timing or benchmark; mutation testing (above); `m5b_session_real.py`,
`DOOM2.WAD` runs, and the real-IWAD `m5c_real_full.py`, `m5c_real.py`, `m5c_wire25_real.py` and `m5c_mixer_real.py` (not
changed by M5d); LeakSanitizer coverage of the genuine C runs; any sanitizer other than AddressSanitizer for the fuzz
campaigns, any campaign without a sanitizer, and a 1.85.0 build of the fuzz crate. The campaigns are bounded, not exhaustive.

**Commits after `6e61fc8`.** The tester's tests and `e6b17b2` add or drop test code only. They were checked by `cargo test -p
doom-game -p doom-play` (192 passed, 0 failed, 1 ignored), format and Clippy locally; the whole workspace on that head is
CI's run (below).

### CI on the exact head

**The last code head, `e6b17b2`:** all nine jobs of [run 44729364458](https://github.com/Copgunn/DOOM/actions/runs/44729364458)
passed: Detect changes, Stable on ubuntu-latest, windows-latest and macos-latest, Minimum Rust 1.85.0, Simulation
instruction counts, and the M5a, M5b and M5c genuine-engine jobs (the last under its new name, "M5c genuine sound"). Earlier
heads: `14a9cc6` failed the Stable ubuntu and Minimum Rust jobs on the flaky claim test (B-1, fixed in `6e61fc8`); the other
runs were cancelled by later pushes. Commits after `e6b17b2` change documentation only and take CI's documentation fast
path; the run on the final head is GitHub's record, linked from the pull request.

### The Windows artifact

Built by the `Windows artifact` workflow on the final head of the pull request; the run and the artifact are GitHub's record,
linked from the pull request [#64](https://github.com/Copgunn/DOOM/pull/64). It is the build for the owner's session (A6).

## A6. The owner's Windows session

Nothing in this milestone was run on Windows. Acceptance of A1 and the choice of A2's default are the owner's. A3 has been
shown on Linux only, and on Windows only he can show it. This is the plan for one sitting, with the build named below.

**The build.** The `doom-play-windows-x86_64` artifact of the `Windows artifact` workflow on the final head of pull request
[#64](https://github.com/Copgunn/DOOM/pull/64) (the run is linked in the PR). Unzip it into a folder of its own, for example
`D:\DoomM5d`. Use **Command Prompt** (`cmd.exe`), not PowerShell: the redirections below are cmd's, and A3 needs a console
window to close. In every command, replace `C:\path\to\DOOM.WAD` with the IWAD's path. Make the folders first:

```
mkdir D:\DoomSessions\m5d\dpi100 D:\DoomSessions\m5d\dpi125 D:\DoomSessions\m5d\dpi150 D:\DoomSessions\m5d\audio
mkdir D:\DoomSessions\m5d\ctrlc D:\DoomSessions\m5d\close D:\DoomSessions\m5d\endtask
cd /d D:\DoomM5d
```

Everything goes into `D:\DoomSessions\m5d`, which the orchestrator reads at `/mnt/d/DoomSessions/m5d`. Do not open, rename
or delete the files there. Leftover `.tmp` files are expected and harmless.

### Part 1: stuck fire at 100%, 125% and 150% (A1, about 15 minutes)

For each scale: set it in Settings, System, Display, Scale. Sign out and in again if Windows asks you to. Then run, with
`125` replaced by the scale:

```
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions D:\DoomSessions\m5d\dpi125 --input-log D:\DoomSessions\m5d\dpi125\input-log.txt 2> D:\DoomSessions\m5d\dpi125\terminal.txt
```

1. The window will look **smaller** at 125% and 150% than before. That is expected from the fix (the window is now 960×720
   real pixels). **Do not resize, maximise or move the window:** each of those writes a `resize` line, and the test reads
   that line as a fault.
2. Play for about three minutes. Again and again, hold the left button, drag the mouse hard towards one edge, and release
   while still moving. Do it towards the right, left, top and bottom, at least ten times each, as you did when it stuck.
3. If fire sticks, keep your hands off the mouse for 2 seconds, then click once with the mouse still. Note the time.
4. Quit with Esc, then Q.
5. Open `input-log.txt` and look at the last line: `summary ... raw downs/ups: Left D/U`. **Accepted for this scale if D
   equals U and the file has no line containing ` resize `.**

If you have a second monitor set to a **different** scale, do one extra run with the window on it, in a folder `dpi-second`.
That is the one case the fix is not expected to cover (record, A1).

### Part 2: sound latency trials (A2, about 15 minutes)

At any one scale. For each line below, run it, play about a minute with plenty of shooting, door and switch sounds, then
quit with Esc then Q. Each run writes its `[INFO]` device report and its `audio` exit lines into its text file.

```
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions D:\DoomSessions\m5d\audio 2> D:\DoomSessions\m5d\audio\a-default.txt
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions D:\DoomSessions\m5d\audio --experimental-audio-start-buffers 1 2> D:\DoomSessions\m5d\audio\b-start1.txt
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions D:\DoomSessions\m5d\audio --experimental-audio-period-frames 256 2> D:\DoomSessions\m5d\audio\c-period256.txt
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions D:\DoomSessions\m5d\audio --experimental-audio-period-frames 128 2> D:\DoomSessions\m5d\audio\d-period128.txt
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions D:\DoomSessions\m5d\audio --experimental-audio-period-frames 128 --experimental-audio-start-buffers 1 2> D:\DoomSessions\m5d\audio\e-period128-start1.txt
```

For each, write one line in `D:\DoomSessions\m5d\notes.txt`: the letter, then the delay compared with `a` (shorter, same,
longer), and any crackle, gaps or stutter. Then say which, if any, should become the default. The WASAPI period count (2 or
3) is not offered: it cannot be set through the pinned audio library (record, A2), and the routes to it are your decision.

### Part 3: a session that cannot be lost (A3, about 10 minutes)

Without `2>` this time, so that the console stays visible.

```
doom-play.exe --iwad C:\path\to\DOOM.WAD --sessions D:\DoomSessions\m5d\ctrlc
```

1. **Ctrl+C:** play and move around, then click the console window (the game pauses when it loses focus, which is fine)
   and press Ctrl+C. Do this three times, each a new run: after about 2 seconds, after about 10 seconds, and after at least
   40 seconds of play.
2. **Closing the console:** the same with `--sessions D:\DoomSessions\m5d\close`. Instead of Ctrl+C, click the console
   window's close button (X). Three times again, at about 2 s, 10 s and 40 s or more.
3. **Optional, End task:** the same with `--sessions D:\DoomSessions\m5d\endtask`. Open Task Manager, select doom-play.exe
   and choose End task.

Note in `notes.txt` roughly how long each run lasted, in order.

### What to send back

Tell the orchestrator when `D:\DoomSessions\m5d` is complete, with the scale names you used and anything you saw. The
orchestrator will then:

- check each input log for downs and ups and for `resize` lines;
- read the audio lines and device reports;
- run every `.lmp` through the session check (`tools/local/m5c_session_real.py`), where each must PASS. Each A3 file must
  hold about 35 tics per second of unpaused play.

**Until the owner has ruled, nothing is marked fixed.** README and COMPATIBILITY keep the stuck fire as a known fault and the
lost session as a limit on Windows. The pull request stays a draft.
