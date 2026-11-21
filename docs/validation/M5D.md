# M5d evidence record

**Status (2026-11-20 23:55 PST):** **the owner retested the second round on Windows and accepted the stuck-fire fix and the slow-disk fix;
the pull request is ready for his review ([A9](#a9-the-owners-retest-of-2026-11-20-his-rulings-and-the-close)).** The first round (A1 to A5;
the owner's session plan is A6) is unchanged below. On 2026-11-20 the owner ran the `b374aaa` artifact on Windows
([A7](#a7-the-owners-windows-session-of-2026-11-20-and-the-second-round)): the launch capture failed in two of five launches, the 100% run
lost three releases, and every stall in his logs came from file writes on a slow disk. In answer the code gained a re-grab of the cursor
and a cursor-left report on Windows, writer threads for the session file and the input log (with a declared weakening of the kill
guarantee), a settle step at a session's end, and a bound on the log's queue; reviews 3, 4 and 5 followed. His retest of the `c9f54ee`
artifact the same day ([plan](#a8-the-owners-retest), [results](#the-retest)) had no lost release in nine launches and no hitch on the
slow disk. **What is not shown stays stated: the launch-capture cause was never reproduced, and nine clean launches cannot show whether
the first grab would have failed without the re-grab.** The close added an exit report that names the sessions not saved after a
writer-thread panic, review 6, and a local mutation run on `doom-play`. The final code head is `0f26023`; the full gates ran at
`3868b2c`, two test-only commits before it. Four review items stay open as documented limits, by his ruling.

Observed results of milestone M5d (play fixes from the owner's playtest of M5c). Only observations and the
library-source readings behind each change appear here. The facts this milestone starts from are in the
[M5c record](M5C.md#the-owners-windows-retest-with-the-input-log-2026-11-19) and the
[roadmap item](../ROADMAP.md) that named this work; no separate investigation note was written, by the owner's
instruction. [Testing](../TESTING.md) owns the commands, [COMPATIBILITY](../COMPATIBILITY.md) the support
statement and [ROADMAP](../ROADMAP.md) the milestone status. Exact review state and CI belong to GitHub.

Scope, by the owner's brief: app and presentation work only. Nothing in `doom-sim`, `doom-render` or
`doom-sound` changes, and no wire, schema, golden or existing test changes. *(Note, 2026-11-20: the second round added one public
method to `doom-game`, `App::settle_save`, with its own new test file; no existing test of any crate changed. See [A7](#a7-the-owners-windows-session-of-2026-11-20-and-the-second-round).)*

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

> **Note, 2026-11-20 (second round).** This section is the record of the first build, in which the loop thread wrote the file
> itself (`FsHost`) and a kill left M or M - 1 tics. The owner's Windows session led to writer threads, which weakened that
> guarantee on stated conditions; the design, the guarantee and the evidence now are in [A7](#a7-the-owners-windows-session-of-2026-11-20-and-the-second-round). Statements below that
> are in the present tense (the design, the write schedule, the kill test's rule, "what a player could notice") describe
> the first build and are kept as the record of it; where they differ, A7 is current.

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

*(First build. Since 2026-11-20 `doom-play` runs `WriterHost`: the same operations are made in order on a writer thread, and the page-boundary rewrite is unsynced; see [A7](#the-writer-threads).)*

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
| Ctrl+C, SIGKILL, a lost X connection, a closed console window | Nothing at that moment, but the file already holds every tic run, or every tic but the last *(first build only; since the writer threads the file holds every tic the writer had finished, which can be fewer: [A7](#the-kill-guarantee-old-and-new))* |

### Evidence (release builds unless stated)

**A. The cargo kill test** (`kill_tests::sigkill_at_random_points_leaves_valid_demos`, `crates/doom-play/src/kill_tests.rs`,
Unix only, so it runs in CI on Linux and macOS; commands in [Testing](../TESTING.md#m5d-kill-tests-and-trial-options)).
*(First build; since 2026-11-20 the child plays the real `WriterHost` and the tic rule is between W and M with a lag bound, [A7](#kill-evidence). The next sentences and tables describe the first build.)* The parent spawns its own executable, which plays the real `App` with the real `FsHost` (sound discarded) on the M5c
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

*(First build. The notes of 2026-11-20 are in [A7](#the-writer-threads): the writes no longer run on the game thread, and leftover temporary files older than an hour are removed at the next start.)*

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

1. **Retry after a failure (the orchestrator chose; the owner ruled on 2026-11-20: once a second is accepted).** After a failed write the app retries a whole
   rewrite at the next tic, then at most once every 35 tics (`FAILED_WRITE_RETRY_TICS`) while it keeps failing, in place of
   the first version's every tic (review MIN-1: up to 35 synced rewrites a second under a persistent failure). The first
   retry is immediate because the existing `app_append` test pins an immediate retry, and no existing test may change in this
   milestone. Is one second right, should even the first retry wait, or should a failure retry only at `SAVE_INTERVAL` and the end?
2. **Fallback claim** *(ruled 2026-11-20: keep it, say so in COMPATIBILITY and print one line on standard error when it is taken)*. A hard-link error other than "exists" falls back to `create_new`, whose kill window can leave an
   empty `.lmp`. NTFS supports hard links (inference); FAT and exFAT do not.
3. **Leftover temporary files** (`*.claim.tmp` and `*.tmp`) accumulate after kills or a failed removal, and no later run
   removes them (review N8). Should a later run remove stale ones it did not create? A failed removal of the claim's
   temporary is still ignored, now harmless to the rewrite (MIN-5). *(Ruled 2026-11-20: remove them at the next start; done, [A7](#the-owners-rulings-and-what-was-done).)*
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

## A7. The owner's Windows session of 2026-11-20 and the second round

**Scope of this section.** It records what the owner saw and ruled on the `b374aaa` artifact, and what was built and checked in
answer (`b374aaa..f7b45a2`, 17 commits). It supersedes the present-tense statements about the
session file in A3 (see the note at A3's head). Observations come from the owner's report and from the orchestrator's reading of
his files at `/mnt/d/DoomSessions/m5d` (read only; counts and verdicts only are recorded). Everything built was tested on Linux only;
**nothing from this round has run on Windows.**

### The session

- **When and on what.** 2026-11-20, 06:36 to 07:06 PST, Windows, the `doom-play-windows-x86_64` artifact built at `b374aaa`
  (the code of A1 to A5, before any writer thread). Sessions and logs went to `D:` (an HDD), as A6 asked. An addendum run at 07:11
  PST used `C:` (an SSD) instead.
- **A1, stuck fire** (`--input-log`; the summary line's `raw downs/ups: Left D/U`):

| Scale | Log | Left downs/ups | What the log shows |
| --- | --- | --- | --- |
| 125% | `dpi125/input-log.txt`, `launch1.txt`, `launch2.txt` | 46/46, 12/12, 12/12 | Pass; no `resize` line. The cursor rested at x=959 from the first drag in all three (first drags at pumps 439, 37 and 28) |
| 150% | `dpi150/input-log.txt` | 34/34 | No `resize` line. Capture went off at pump 118 and on at 182, before the first drag. The first reading said the launch capture was not exercised here; **that was wrong, see [the second round](#stuck-fire-the-second-round)** |
| 100% | `dpi100/input-log.txt` | **41/38 (fail)** | `capture on` was logged at pump 1, yet the cursor was not confined: after the down at pump 42 it ran to x=936 at pump 47, with no cursor event until pump 87 at x=950. Downs at pumps 42, 148 and 283 have no up. Esc at pump 389 (capture off) and Esc at pump 840 (capture on); from there every up lands at an edge (x=0, x=959, y=0 or y=719) and 38 of 38 match. No focus-lost or `resize` line in that window |

- **A2, sound trials.** All on "Headphones (JBL TUNE670NC)", Bluetooth, WASAPI, stream 11,025 Hz. Windows granted period times three
  exactly as asked in every run. One run each, by ear; lengths differ and pauses are not counted. **All were recorded to `D:` (the
  HDD), so the underrun figures are contaminated by disk stalls and are not evidence that a period of 128 drops out less.** **The
  trial was by ear on Bluetooth headphones and is not a latency measurement.** The figures, as reported:

| Run | Options | Tics | Underruns (frames) | Queue depth, frames (mean, max) | By ear |
| --- | --- | ---: | --- | --- | --- |
| a | default | 3,435 | 5 (2,560) | 1,443.4, 2,048 | |
| b | start buffers 1 | 3,425 | 9 (4,608) | 1,360.2, 2,048 | gaps after unpausing |
| c | period 256 | 2,308 | 8 (2,048), overruns 1 (256) | 1,157.7, 2,048 | shorter delay than a |
| d | period 128 | 1,917 | 3 (384) | 1,108.5, 1,920 | best |
| e | period 128, start buffers 1 | 3,556 | 22 (2,816) | 1,033.9, 1,920 | |

- **A3, kills.** Ctrl+C left 3 files (682, 1,130 and 4,766 bytes) and closing the console 4 files (378, 1,106, 3,694 and
  7,946 bytes). Every size is 14 + 4n (a header, n commands and the marker). No `.tmp` file was left. **End task was not run.**
  The report does not state how long each run lasted against its file's tic count, so whether each file held every tic run is not
  recorded here.
- **The hitch.** About every 30 s the pump lines show `ran=4` or `ran=5` right after the synced whole rewrites: at tics 1,025 and
  1,026, 1,054 and 1,055, 2,049, 2,104 and 3,154, across `dpi100`, `dpi125`, `dpi150` and `launch2` (4 to 5 tics after tics
  1,021, 1,050, 2,045, 2,100 and 3,150). All six 1,050-tic rewrites in the logs show it; four of seven page-boundary rewrites do.
  About 30 further pumps with `ran` of 3 or more are not aligned with any rewrite (roughly every 250 to 350 tics), and one falls
  near tic 58 to 73 in every run.
- **The addendum (07:11 PST, `C:`).** The same build with the default sound and `--sessions` and `--input-log` on `C:` instead of `D:`:
  1,754 tics, **no pump with `ran` of 2 or more** (the `D:` runs had 12 to 15 each), underruns 0, queue depth mean 848.1, max 1,024.
  No hitch felt. **The owner's conclusion:** every stall, aligned with a rewrite or not, comes from file writes on the game thread
  hitting a slow disk, and the same stalls cause the underruns and the deeper queue.
- **The session check of his 17 files** (the orchestrator, 2026-11-20; a worktree at `b374aaa`, `tools/local/m5c_session_real.py`,
  a scratch copy of the IWAD, the genuine engine in its strict, overflow and address variants and the genuine mixer): **17 of 17
  `VERDICT: PASS`, 0 FAIL**, 29,187 tics in all (audio runs 3,435, 3,425, 2,308, 1,917 and 3,556; Ctrl+C 167, 279 and 1,188; close 91,
  273, 920 and 1,983; `dpi100` 2,398; `dpi125` 3,574, 1,021 and 1,044; `dpi150` 1,608). **The address variant ran with
  `ASAN_OPTIONS=detect_leaks=0`: leak detection was disabled, so it is not LeakSanitizer coverage.** The check shows the files replay
  equal to the genuine engine; it does not show what he saw or heard. The log is in scratch (not committed).

### The owner's rulings and what was done

The numbers are those of the owner's messages (the brief for the writer threads numbered its own rulings 1 to 3 differently, so
each row also says what it is about).

| Ruling | What was done | Commits |
| --- | --- | --- |
| Stuck fire at 100% (ruling 1): ask for the grab again on Windows; report when the cursor has left the window | `Forward::grab_again`, the detector `cursor_leave.rs`, `GRAB_AGAIN` (Windows only); [below](#stuck-fire-the-second-round) | `e6b9f76`, `11ebbe0`, `b7b5ca3`, `06acf97` |
| The two cursor-left reports in `dpi150` (pumps 94 and 117) are real exits; the threshold stays 2 pumps | Nothing changed | none |
| The hitch: file writes off the game thread; a bounded queue (5 s of play, 175 tics); quit writes everything; the weakening of the kill guarantee accepted on stated conditions | Writer threads for the session file and the input log; [below](#the-writer-threads) and [the guarantee](#the-kill-guarantee-old-and-new) | `6899d97`, `157048a`, `47874ce`; then `dce493d`, `b073255`, `d4c4642`, `c14b3a7`, `b8bd558`, `fd87e57`, `ceec231`, `7af7799`, `f7b45a2` after review 3 |
| Ruling 3, a default sound period of 128 | **Withdrawn for this pull request.** The default stays 512, the master test on the constants is untouched, both experimental switches stay, and the owner reruns trials a, d and e on `C:`. The orchestrator's margin and queue-depth arithmetic is kept as a report ([below](#ruling-3-withdrawn-the-arithmetic-as-a-report)) | none |
| 4: a failed write is retried once a second | Accepted as it was (`FAILED_WRITE_RETRY_TICS` = 35); the writer thread keeps the cadence, with a test that counts the disk attempts | none new (the cadence is `a62458d`) |
| 5: where hard links are refused, the claim falls back | Kept; COMPATIBILITY says so, and one line on standard error is printed each time the fallback is taken | `6899d97`, `157048a`; the documentation in the commit of this record |
| 6: remove stale leftover `.tmp` files at start | At every start, files named `NAME.PID.tmp` or `NAME.PID.claim.tmp` (names of this program's own form) more than an hour old are removed from the sessions directory, and the count is printed at every launch, zero included | `6899d97`, `157048a`, `3957cf0`, `7af7799` |
| 7: the audio library's `[INFO]` report behind an option | `--audio-report` (off by default); with it off the settings line says the granted period is not shown | `6899d97`, `157048a` |
| 8: the one changed import line in `session_tests.rs` | Accepted as disclosed in A2 | none |
| README and COMPATIBILITY keep stuck fire as a known fault until he confirms a retest; COMPATIBILITY says recording to a slow disk causes hitches and dropouts and to keep the sessions folder on an SSD; COMPATIBILITY states the new kill limit in one plain sentence | Done | the commit of this record |
| The reporting corrections | [Below](#gates-ci-and-the-corrections-to-the-earlier-summary) | the commit of this record |

### Stuck fire: the second round

**What the five launches show.** The launch capture failed in **two of five launches**, `dpi100` and `dpi150` (not "one of four", as
the first summary of the session said, and the note that the 150% run had not exercised the launch capture was wrong: it had, and the
capture did not hold). Both failures were the **first launch after the owner changed the display scale**, and the three launches
where the clip held (`dpi125/input-log`, `launch1`, `launch2`) were all at 125%. **That is an observation from five launches, not a
cause.** In `dpi150` the focus loss at pump 118 followed the cursor leaving the window (x=889 at pump 115). The M5c retest log of
2026-11-19 (`D:\DoomSessions\input-log.txt`, a pre-M5d build) is scored below.

**The cause: read from the library's source, not reproduced.** Nothing was run on Windows and the failure was not reproduced.
miniquad 0.4.11 (`src/native/windows.rs`; each line re-read):

- `set_cursor_grab`, 124-133, sets `cursor_grabbed` (125) and then calls `update_clip_rect` (128) or `ClipCursor(NULL)` (130);
  `process_request` calls it for `SetCursorGrab`, 1120 and 1126.
- `update_clip_rect`, 327-353: `GetClientRect`, two `ClientToScreen`, `SetRect`, then `ClipCursor` at 352 **with its return value
  discarded**.
- The clip is re-applied only on `WM_SIZE` (439-442) and on `WM_MOVE` while grabbed (522-524), not on `WM_ACTIVATE` (763-769, which
  only reports restored or minimised), `WM_SETFOCUS` (770-797, IME only) or `WM_KILLFOCUS` (799-801).
- `run`: `ShowWindow` at 1222, raw input registered with `hwndTarget = NULL` and no flags at 1225-1237, the event handler set at
  1239 and the window's user data at 1242. `win32_wndproc` returns `DefWindowProcW` while the user data is 0 (392-394), so the
  activation `ShowWindow` causes at launch cannot reach doom-play.
- The loop drains requests at the top of each iteration (1248-1250), then dispatches messages, then runs `update` and `draw`; a
  grab asked for in `update` is made at the top of the next iteration.
- There is no leave event: the `TrackMouseEvent` block in `WM_MOUSEMOVE` is commented out (497-520, block 500-514) and
  `WM_MOUSELEAVE` (558-565) is empty. `WM_INPUT` (526-556) posts raw motion wherever the cursor is. There is no call to read the
  cursor position or the clip rectangle.
- Other systems, which is why the new code is gated to Windows: X11 ungrabs and grabs on every call (`linux_x11.rs` 345-370); macOS
  returns early on a repeated request and freezes the cursor with `CGAssociateMouseAndMouseCursorPosition(false)` (`macos.rs` 51-65),
  which the detector's silence rule would misread.

So the library applies the clip once per `set_cursor_grab(true)`, ignores the result, and nothing re-applies it if Windows or another
program clears or replaces it after pump 1 without a size or move message; its own flag and doom-play's `captured` stay true, so
doom-play believes the grab holds. **What cleared or prevented the clip on the owner's machine cannot be shown from this source**
(Windows' own behaviour is not readable here). One observation bears on it, and its link to the foreground state is an inference, not
shown: in `dpi100` and `dpi150` no raw motion arrived before the first click (the first raw motion was at pump 42, the first down, in
`dpi100`; at pump 55, first down 52, in `dpi150`) while cursor events did; in all three `dpi125` logs raw motion came before any click.
Raw input registered with a null target follows the keyboard focus by Windows' documentation, which suggests the window was not
foreground at launch in those two runs. No log has a `restored` line at launch (0 of 5).

**The fix** (Windows only: `GRAB_AGAIN = cfg!(target_os = "windows")` gates the detector and the request, so Linux and macOS get no
call, no detector and no new log line, by construction; no unsafe code, no library patch, no forced button release; what reaches the
game is identical, tested). The grab is asked for again, at the end of a pump and after that pump's capture change, when any of
three reasons holds while the app holds the capture: a mouse button went down, focus was gained, or the detector reports the cursor
left. Each request logs `app P T capture again (REASONS)` under `--input-log`. The first click therefore asks for the clip again; that
covers a clip that never took hold or was lost before the first click. A grab asked for in `update` is made at the top of the next
loop iteration, so one more message batch is handled first.

**The cursor-left detector is an inference.** The library cannot say where the cursor is, so `cursor_leave.rs` infers an exit: while
captured, after a cursor event at (x, y), it sums raw motion per axis; a pump is silent if raw motion arrived and no cursor event did;
once **2 silent pumps** have passed and the summed motion on some axis points at an edge more than **2.0 px** away and is at least
that distance (factor 1), it logs `app P T cursor-left from x=X y=Y dx=DX dy=DY pumps=N` once and asks for the grab; the next cursor
event re-arms it. It reports nothing before the first cursor event of a grab. Basis: healthy logs rest the cursor at x=0, x=w-1,
y=0 or y=h-1 (largest x 959 and y 719 in every log, at 960 by 720) and post no cursor event while pinned. Silence is counted over
whole pumps because a pump's raw lines precede its cursor event: a prototype that counted the pump in progress gave false reports in
all three `dpi125` logs at 2 pumps, and the pump-end version gave none at any count. A sweep of silent counts 1 to 4 and distance
factors 0.5 to 3 reported the same episodes with none in the `dpi125` logs; factor 0.1 or 0.25 at count 1 gave one in
`dpi125/input-log`, so the margin on the factor is about 4. **Blind spots, documented in the module:** a cursor resting at an edge
whose clip is then cleared, or one leaving through an edge it already rested on, is not reported.

**Its score on the owner's logs** (the Rust replay `replay_local_input_logs`, debug build; counts only):

| Log | Reports | At pumps | Left downs without an up |
| --- | --- | --- | --- |
| `dpi100/input-log.txt` | 3 | 49, 151, 291 | 42, 148, 283 |
| `dpi125/input-log.txt` | 0 | none | none |
| `dpi125/launch1.txt` | 0 | none | none |
| `dpi125/launch2.txt` | 0 | none | none |
| `dpi150/input-log.txt` | 2 | 94, 117 | none |
| M5c retest log, 2026-11-19 (pre-M5d build; scored apart) | 4 | 323, 801, 1264, 1770 | 234, 1303, 1763 |

- `dpi100`: one report after each lost release and before the next down (42 to 49, 148 to 151, 283 to 291), all between 42 and 389;
  none after the second Esc at pump 840 to the end of the log (pump 2,829).
- `dpi150`: the two reports are before the focus loss at 118; none after the capture came back at 182. The first acceptance bar
  ("none at 150%") was not met, and the thresholds were not tuned to meet it. By the log itself they are real exits: after a cursor
  event 90 px from the left edge at pump 92, about 210 raw units of leftward motion over pumps 93 to 97 with no cursor event, then a
  cursor event 7 px from the left edge at pump 108; and after an event 70 px from the right edge at pump 115, about 190 raw units
  rightward with no cursor event until the focus loss at 118. **The owner accepted both as real exits; the threshold stays 2 pumps.**
  (For information only: 8 or more silent pumps would give none in `dpi150` and still flag all three `dpi100` lost releases, at the
  cost of reports about 6 pumps, about 170 ms, later; not chosen.)
- M5c retest log: **4 reports, covering 2 of the 3 lost releases** (234 then 323; 1763 then 1770). The third, at 1303, has the
  cursor resting on the border at x=959 with cursor events along y (pumps 1306 to 1337): pinned with the clip holding. The detector
  does not report that case by design; it is the case the DPI request and the re-grab on button-down address, and whether they do is
  untested. Reports 801 and 1264 have no lost release beside them.

**Tests and plants.** `cursor_leave` has 6 tests and one ignored local replay; `mouse_forward` 7 tests of the decision (a 32-case
truth table), the reasons and the lines; `window` one test pinning `GRAB_AGAIN`. 19 planted faults, each applied alone, 19 caught at
`06acf97`; one (a pump with a cursor event counted silent) survived at `b7b5ca3` and led to `06acf97`. Not covered by a test: the
window glue in `Stage` (the `set_cursor_grab(true)` call and the calls that feed the follower), which needs a miniquad display; it
is compile-checked only. Review 3 later added one test (idle pumps do not count as silent). **Not run:** anything on Windows or macOS.

**Status (as written before the retest; superseded by [A9](#a9-the-owners-retest-of-2026-11-20-his-rulings-and-the-close), where the
owner accepted the fix).** Stuck fire remains a **known fault** until the owner confirms a retest ([A8](#a8-the-owners-retest)). A release can still be
lost in the four cases listed under A1; the re-grab narrows the window in which the clip is missing, it is not a capture.

### The writer threads

**What the owner found.** Every stall came from file writes on the game thread on a slow disk. **Which writer caused which stall is
not yet measured.** The stalls that follow the synced rewrites coincide with them; the unaligned ones were not attributed to a
write; and the local runs below on an SSD show no stall before or after the change, so they can attribute nothing. The new exit lines
and the `--tic-stats` list exist so that his retest on `D:` measures it ([A8](#a8-the-owners-retest)).

**The design** (`crates/doom-play/src/session_writer.rs`, `input_log.rs`, `write_stats.rs`, `closing.rs`). `FsHost` stays synchronous
(master tests build it by a struct literal and read the file while it is alive); `WriterHost` wraps it and sends every write to one
writer thread, which makes the same operations of `session_file.rs` in order. `LoggedHost` became generic over its inner host.

- **Claim.** Answered at once (`Ok(false)` if the name exists, a stat on the loop thread, or this process already handed it out);
  otherwise queued and `Ok(true)`. The writer then runs the unchanged claim (synced temporary file, hard link, fallback). If another
  process took the name in between, the session is marked lost and never written; every later call about it returns the error and
  the pause box and exit line show a failed save (the earlier synchronous claim would have moved to `-2`; a lost race of two
  instances starting in the same second in the same directory costs the session visibly).
- **A tic.** Queued as its five bytes. The writer keeps its own copy of the demo. For the tics waiting when it takes a batch it makes
  one open and one five-byte write per tic, each inside one 4,096-byte page; if one would straddle a boundary it rewrites whole,
  **unsynced**, through `NAME.PID.tmp` and a rename. (Several in-page appends cost the number of tics waiting; a whole rewrite per
  batch could itself take longer than a tic on a slow disk.)
- **A whole demo.** The app's every-1,050-tic, end, restart and quit saves, its retries, and the hand-over after a decline at the
  bound are kept as the session's newest snapshot (a newer one replaces the pending one in place); the writer rewrites whole from it,
  **synced** for the app's own saves and unsynced for a catch-up snapshot. Queued appends a later snapshot already holds are skipped.
- **Where `sync_all` remains:** the claim's temporary file and the app's own whole saves. Not the page-boundary or catch-up
  rewrites, which were synced before.
- **The bound.** `QUEUE_TICS` = 175 queued tics (5 s of play). At the bound `append` returns `Ok(false)` and the app (unchanged)
  hands the whole demo over, which replaces the pending snapshot in place. With the disk stalled the loop goes on at full speed;
  memory is at most 175 queued tics plus one demo-sized snapshot per session whose snapshot is pending (one, unless the player
  restarts during the stall); no tic is dropped, because the app's recorder holds every tic; the file stops growing, so **its lag
  in tics grows without bound while the disk is stalled; only memory is bounded**. When the disk answers, the writer finishes the
  queued appends and writes the newest snapshot as one unsynced rewrite holding every tic.
- **Errors return.** A failed write is recorded per session and returned by the next call about that session; after a failure the
  writer writes nothing for that session until the app hands a whole demo over (its retry at the 35-tic cadence), so a failing disk
  sees one attempt per retry (a test counts 4 rewrites in 110 tics with the real app).
- **Where the loop thread still touches the disk or waits:** the `Path::exists` stat at each claim (a read; on a stalled network
  share it could block), `eprintln!` lines, the settle wait at a session's end (next item), and quit.
- **The input log** is formatted on the loop thread, handed over at each pump's end in a bounded queue (`LOG_QUEUE_PUMPS` = 1,024
  pumps, `try_send`) and written and flushed by a log-writer thread. When the queue is full, a pump's lines are dropped whole and
  counted, and the next accepted hand-over starts with `dropped PUMP TIC N lines of P pumps were not written: the log's writer was
  behind`; after the summary line, hand-overs block instead. The header is unchanged, so the bytes equal the earlier log's while the
  disk keeps up (a test writes one session through both writers).
- **The end of a session (the settle).** Review 3 found that a failure of a session's last writes never reached the app (it said
  DEMO SAVED). Now, after every update in which the app is not playing, the window asks the writer about the current session's file
  and tells the app (`App::settle_save`, the one new `doom-game` method). A failure makes the status Failed; a file holding every
  recorded tic makes it Saved; a still-pending write after a wait turns Saved into Failed with "not written yet". **At the update in
  which a session ends, the wait is up to 2 s** (`END_WAIT`; no tic runs then). **What the player sees with a stalled disk at a
  session's end:** (1) for up to 2 s the last game frame stays on screen and the window does not respond (there is no draw and no
  event handling in that update); (2) if the disk answers in time, the end screen shows DEMO SAVED or DEMO NOT SAVED with the reason
  at once; (3) if not, it shows "DEMO NOT SAVED: NOT WRITTEN YET: THE DISK HAS NOT ANSWERED IN 2 S; STILL TRYING"; (4) the writer goes
  on and at the first update after it finishes (they come every 1/35 s on an end screen) the screen changes to DEMO SAVED or to the
  error; (5) pressing R starts the next session at once, with no wait; the earlier session is written when the disk answers,
  rewritten again at exit if that failed, and named in an exit line; (6) pressing Q or closing the window waits for the disk
  without a bound, as M5b's synchronous quit did.
- **Quit drains.** At quit the log's summary is written, `finish` drains every write handed over (no time bound), the settle runs,
  the app's own `quit` rewrites whole and synchronously if the file is behind or failed, and the writer closes. The exit report
  gives one true line per session (earlier sessions one line each: saved at exit, not saved, or not written).
- **At restart** the window does not wait (it cannot see the restart coming without duplicating the app's key map, and a 2 s wait on
  every key press while paused would freeze resuming); when the writer moves to the next claim it rewrites the previous session
  whole once more if its file does not hold its copy, unless its last write was already a failed retry of that same copy (a whole
  demo written after an earlier failure, with no tic added since; review 4, R2).

**Exact new texts** (each after `doom-play: `). On exit: `session file writes: N (claims C, appends A of T tics[, D appends declined],
whole rewrites S synced and U unsynced), F failed; longest X.Y ms (KIND at tic K); at most W tics waiting`; with `--input-log`:
`input log writes: N, F failed; longest X.Y ms (input log at tic K)`; with `--tic-stats`: `writes over 20 ms: KIND at tic K X.Y ms; ...` or
`writes over 20 ms: none` (at most 40 listed). At start: `removed N leftover temporary file(s) more than an hour old from DIR`
(every launch) and, per failure, `a leftover temporary file could not be removed: E`. When the fallback claim is taken:
`PATH: the file system refused a hard link, so the session file was created by the fallback claim (created empty, then written)`.

### The kill guarantee, old and new

| | The statement |
| --- | --- |
| **Old** (first build, A3; M5b before it had none between writes) | At every instant the session's file is a valid demo and holds M or M - 1 tics of the M the session has run |
| **New** (since the writer threads, `6899d97`) | At every instant the file is a valid demo holding **at least every tic the writer had finished writing (W) and at most M**. A kill (Ctrl+C, a closed console, a lost display connection, End task, SIGKILL) loses the tics still queued: normally none or a few, at most 175 queued appends, and while the disk is stalled **the lag in tics is not bounded** (past the queue bound the app hands whole demos over and the file stops growing; only memory is bounded) |

**This is a weakening, and the owner accepted it** on stated conditions, each met as follows:

| Condition | How it is met |
| --- | --- |
| The old and the new statement side by side | The table above, and COMPATIBILITY's one sentence |
| The lag distribution measured: ext4, and `/mnt/d` random and aimed | [Kill evidence](#kill-evidence) below (at `3957cf0`, `f7b45a2` and the final code head `d9cd771`) |
| A visible exit line | `session file writes: ... at most W tics waiting`, and the `--tic-stats` list of writes over 20 ms |
| Quit drains | `finish` drains every write handed over; the session-end settle and quit are tested with a real `App` |
| A bounded queue | 175 tics for the session file, 1,024 pumps for the log; a test with the disk stalled checks the queue and the one snapshot |
| COMPATIBILITY states the limit plainly | Done in the commit of this record |

**The atomicity argument, new.** Every write is still one of the same all-or-nothing operations (a five-byte write inside one page,
by the reading of Linux's `generic_perform_write` in A3, or a new temporary file and a rename), now made in order on one thread; a
multi-tic append is a sequence of such writes. So at every instant the name holds a valid demo that is a prefix of the session. The
queue can only make that prefix shorter. A kill before the claim completes leaves no file for that session (and perhaps a
`NAME.PID.claim.tmp`), never an empty or partial `.lmp`, except in the no-hard-link fallback window.

**What an OS crash or power loss can lose.** It was never covered (the directory is never synced), and it covers less now: the
page-boundary and catch-up rewrites are **unsynced**, where the page-boundary rewrite was synced before. Only the claim's temporary
file and the app's own whole saves are synced. A crash can lose any tics not yet on the disk.

### Ruling 3 withdrawn: the arithmetic as a report

The owner's default of 128 was withdrawn for this pull request; the figures stay as a report, and nothing changed on their basis. The
model: buffer k is handed over at tic floor(512k/315); the device pulls P frames per callback at 11,025 Hz from the instant the start
threshold is met plus a phase of 0 to P-1; it ignores any pre-fill of the three-period device buffer.

| Period P | Hand-over slack before a pull finds too few frames, two-buffer start | One-buffer start | Model queue depth at a callback, two-buffer start (min, mean, max) | One-buffer start |
| --- | --- | --- | --- | --- |
| 512 | 315 to 826 frames (28.6 to 74.9 ms) | 0 to 511 | 512, 980.5, 1,536 | 512, 665.5, 1,024 |
| 256 | 315 to 570 (28.6 to 51.7 ms) | 0 to 255 | 512, 854.5, 1,280 | 256, 539.5, 1,024 |
| 128 | 315 to 442 (28.6 to 40.1 ms) | 0 to 127 | 384, 791.5 (71.8 ms), 1,152 | 128, 476.5, 896 |

The worst case is equal at every period (one tic). **Reconciliation with the owner's runs:** on `D:` run d (period 128) had mean 1,108.5
and max 1,920, above the model's maximum of 1,152, so the 63.4 to 121.3 ms total did not hold in that run; his `C:` run at the default
(mean 848.1, max 1,024) lies inside the model's 512 to 1,536. That is consistent with stalls inflating the `D:` depths (each underrun's
silence leaves that many more frames queued, up to the 2,048 cap) but is not a full account (384 frames of silence in d does not reach
1,920). The `C:` reruns of a, d and e are to settle it ([A8](#a8-the-owners-retest)).

### Kill evidence

**What this tests.** `kill_tests::sigkill_at_random_points_leaves_valid_demos`, now playing the real `WriterHost`, SIGKILLs the child
and checks every `.lmp`: it parses, replays with no refusal, its digest equals the child's printed digest at its tic count, and its
tic count lies **between W and M** (exactly M once a later session's claim completed; a missing file only before its claim
completed); and the session running at the kill has lost at most the **lag bound** of 525 tics (3 times the queue bound, once its
claim completed; `DOOM_PLAY_KILL_MAX_LOST` sets or lifts it). The summary prints the distribution of tics lost (M minus the tics in
the file) of the session running at each kill and the bound applied. Three self-tests must each fail their plant: `torn` (a torn
append), `early` (W printed 3 too high; the W rule) and `stall` (a writer that stops; the lag bound). **The test child runs far faster
than 35 tics a second, so the lag it shows is worse than play would give.** Commands are in [Testing](../TESTING.md#m5d-kill-tests-and-trial-options).

**Preliminary runs at `3957cf0`** (before the fixes of review 3; release build, `--nocapture`, one machine, run in sequence, 2026-11-20
08:29 to 09:03 PST). **These went through the Rust checker (the kill test) only; none went through the genuine engine.** "Lost" is M
minus the tics in the file for the session running at each kill. The `/mnt/d` directories were kept (`DOOM_PLAY_KILL_DIR` under
`/mnt/d/DoomSessions/m5d-kills`; nothing is deleted under `/mnt`). `/mnt/d` is this machine's view of the Windows `D:` drive through
WSL; the owner calls `D:` an HDD; this record does not state what it is.

| File system | Aim | Kills | Files / tics | Lost 0 | 1 | 2 to 5 | 6 to 20 | More | Max | Kills with no `.lmp` |
| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ext4 | none (random) | 2,000 | 2,154 / 869,273 | 1,711 | 259 | 23 | 4 | 3 | 77 | 416 |
| ext4 | tic 1,021, 0 to 3 ms | 200 | 397 / 298,970 | 179 | 21 | 0 | 0 | 0 | 1 | 0 |
| ext4 | tic 1,050, 0 to 3 ms | 200 | 409 / 314,428 | 0 | 33 | 167 | 0 | 0 | 5 | 0 |
| ext4 | tic 1, 0 to 3 ms | 200 | 2 / 2 | 2 | 61 | 137 | 0 | 0 | 5 | 198 |
| ext4 | tic 1,021, 0 to 60 ms | 150 | 297 / 221,961 | 107 | 19 | 16 | 4 | 4 | 54 | 0 |
| ext4 | tic 1,050, 0 to 60 ms | 150 | 303 / 233,781 | 112 | 18 | 9 | 7 | 4 | 63 | 0 |
| `/mnt/d` | none (random) | 300 | 249 / 89,556 | 13 | 1 | 16 | 124 | 101 | 424 | 97 |
| `/mnt/d` | tic 1,021, 0 to 60 ms | 100 | 188 / 144,877 | 0 | 0 | 4 | 56 | 40 | 89 | 0 |
| `/mnt/d` | tic 1,050, 0 to 60 ms | 100 | 196 / 152,628 | 0 | 0 | 2 | 19 | 79 | 174 | 0 |

- All nine runs passed. In the `/mnt/d` random run the lost counts add up to 255, not 300: the other 45 kills landed in a claim
  (before any file existed), where no tic count applies. The 3 ms aims at tics 1,021 and 1,050 on ext4 also left 0 and 200
  leftover `.tmp` files; the aim at tic 1 left 198 kills with no `.lmp` (the claim had not finished, so nothing was lost that existed).
- **Reading.** On this machine's ext4 the file was within 0 or 1 tic of the last printed tic in most kills (1,970 of 2,000 random kills
  within 1) and at most 77 behind. Through `/mnt/d` it was typically 6 to 20 tics behind and up to 424 (random) or 174 (aimed): the
  lag the owner accepted is of that order for a slow disk. The lag bound of 525 tics did not exist at `3957cf0`; the largest of these, 424, is under it by 101 tics, so a `/mnt/d` run needs `DOOM_PLAY_KILL_MAX_LOST` set or lifted if it is to pass on a slower moment. Measured here only for the test child, not for play.

**Runs on the code of `f7b45a2`** (after the fixes of review 3; the tree was at `489f912`, which adds documentation only; release build,
one machine, in sequence, 2026-11-20 09:49 to 11:36 PST; the fix and review agents were building and testing on the machine during
it). First the kill test (the Rust checker), with the same seeds and aims as the preliminary runs; `/mnt/d` under
`/mnt/d/DoomSessions/m5d-kills2`, with the lag bound lifted (`DOOM_PLAY_KILL_MAX_LOST=none`); the ext4 runs kept the default bound of
525 tics. All nine passed.

| File system | Aim | Kills | Files / tics | Lost 0 | 1 | 2 to 5 | 6 to 20 | More | Max | Kills with no `.lmp` |
| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ext4 | none (random) | 2,000 | 2,096 / 798,593 | 1,683 | 278 | 22 | 1 | 12 | 244 | 421 |
| ext4 | tic 1,021, 0 to 3 ms | 200 | 397 / 299,019 | 178 | 22 | 0 | 0 | 0 | 1 | 0 |
| ext4 | tic 1,050, 0 to 3 ms | 200 | 409 / 314,426 | 0 | 36 | 164 | 0 | 0 | 5 | 0 |
| ext4 | tic 1, 0 to 3 ms | 200 | 51 / 182 | 36 | 54 | 110 | 0 | 0 | 5 | 149 |
| ext4 | tic 1,021, 0 to 60 ms | 150 | 297 / 223,120 | 109 | 29 | 6 | 4 | 2 | 55 | 0 |
| ext4 | tic 1,050, 0 to 60 ms | 150 | 305 / 234,854 | 120 | 19 | 9 | 2 | 0 | 7 | 0 |
| `/mnt/d` | none (random) | 300 | 250 / 90,066 | 9 | 1 | 16 | 113 | 113 | **620** | 98 |
| `/mnt/d` | tic 1,021, 0 to 60 ms | 100 | 188 / 144,692 | 0 | 0 | 4 | 49 | 47 | 128 | 0 |
| `/mnt/d` | tic 1,050, 0 to 60 ms | 100 | 196 / 152,738 | 0 | 0 | 2 | 18 | 80 | 121 | 0 |

Then **the genuine engine** (`tools/reference/m5d_kill_check.py`, two jobs, ext4): every demo that survived a kill went through the
session check (the genuine engine against Rust, tic for tic; strict, overflow and address variants, the last with leak detection
disabled, so it is not LeakSanitizer coverage).

| Set | Kills | Surviving demos / tics | Genuine session check |
| --- | ---: | --- | --- |
| Synthetic, random, with the self-check | 200 | 221 / 101,156 | 221 PASS, 0 FAIL; `SELF-CHECK: PASS` |
| Synthetic, aimed at tic 1,021 (0 to 60 ms) | 30 | 63 / 46,958 | 63 PASS, 0 FAIL |
| IWAD copy, random, with the self-check | 50 | 49 / 10,356 | 49 PASS, 0 FAIL; `SELF-CHECK: PASS` |
| IWAD copy, random, sessions up to 8 s | 200 | 250 / 120,539 | 250 PASS, 0 FAIL |
| IWAD copy, aimed at tic 1,021 (0 to 60 ms) | 30 | 55 / 40,813 | 55 PASS, 0 FAIL |
| IWAD copy, aimed at tic 1,050 (0 to 60 ms) | 30 | 55 / 42,171 | 55 PASS, 0 FAIL |
| **Total** | **540** | **693 / 361,993** | **693 PASS, 0 FAIL** |

**Runs on the final code head, `d9cd771`** (after the follow-ups of review 4; the kill test only, the Rust checker; release build, in
sequence, alone on the machine apart from the orchestrator's own light work, 2026-11-20 11:37 to 12:08 PST; `/mnt/d` under
`/mnt/d/DoomSessions/m5d-kills3`, bound lifted). All nine passed. **The genuine-engine sets above were not repeated on this head**;
the genuine-engine kill check on this head is the gates' own (100 synthetic and 50 IWAD kills, in the gates table).

| File system | Aim | Kills | Files / tics | Lost 0 | 1 | 2 to 5 | 6 to 20 | More | Max | Kills with no `.lmp` |
| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ext4 | none (random) | 2,000 | 2,196 / 919,679 | 1,692 | 282 | 24 | 1 | 1 | 117 | 412 |
| ext4 | tic 1,021, 0 to 3 ms | 200 | 397 / 299,052 | 174 | 26 | 0 | 0 | 0 | 1 | 0 |
| ext4 | tic 1,050, 0 to 3 ms | 200 | 409 / 314,428 | 0 | 30 | 170 | 0 | 0 | 5 | 0 |
| ext4 | tic 1, 0 to 3 ms | 200 | 53 / 203 | 39 | 60 | 101 | 0 | 0 | 4 | 147 |
| ext4 | tic 1,021, 0 to 60 ms | 150 | 299 / 223,602 | 121 | 20 | 7 | 2 | 0 | 6 | 0 |
| ext4 | tic 1,050, 0 to 60 ms | 150 | 305 / 234,944 | 118 | 22 | 9 | 1 | 0 | 7 | 0 |
| `/mnt/d` | none (random) | 300 | 262 / 97,059 | 10 | 2 | 7 | 125 | 109 | 206 | 94 |
| `/mnt/d` | tic 1,021, 0 to 60 ms | 100 | 188 / 144,508 | 0 | 0 | 1 | 53 | 46 | 197 | 0 |
| `/mnt/d` | tic 1,050, 0 to 60 ms | 100 | 196 / 152,448 | 0 | 0 | 2 | 19 | 79 | 211 | 0 |

- Where a row's lost counts add up to fewer than its kills, the rest landed in a claim, before any file existed (4 of the ext4
  random kills at `f7b45a2`; 48 and 47 of the `/mnt/d` random kills).
- **Reading.** On ext4 the file was within one tic of the last printed tic in 1,974 of 2,000 random kills at the final head (1,961
  at `f7b45a2`), and at most 117 behind (244 at `f7b45a2`, measured while other jobs loaded the machine). Aimed inside the synced
  whole save at tic 1,050 the file is 1 to 5 tics behind: the appends wait behind that save on the writer thread, which is the
  weakening in a small form. Through `/mnt/d` the file was typically 6 to 20 tics behind and up to 211 at the final head.
- **One `/mnt/d` kill at `f7b45a2` found the file 620 tics behind, past the kill test's default lag bound of 525.** That run had the
  bound lifted, so it passed; with the default bound it would have failed. It is the unbounded lag the new statement declares, seen
  once in the 1,500 `/mnt/d` kills of the three runs (500 at each head): on a slow disk the bound of 525 is a test default,
  not a guarantee. The file was still a valid demo and a prefix of the session.
- The test child runs far faster than 35 tics a second, so every lag here is worse than play would give at the same disk speed;
  **no lag was measured in play.** The owner's retest exit line (`at most W tics waiting`) gives the figure for his disk.
- The `/mnt/d` directories `m5d-kills`, `m5d-kills2` and `m5d-kills3` under `/mnt/d/DoomSessions` were kept (nothing is deleted
  under `/mnt`); they are the owner's to delete.

### Review 3

- **Reviewer:** Claude Opus 5.5 (`claude-opus-5-5`), effort **xhigh** (set by the orchestrator at spawn), a fresh agent that wrote none
  of the code. **Range:** `b374aaa..3957cf0` (`crates/doom-play`), in a scratch worktree, planted faults in the scratch copy only (all
  reverted). Another kill-test job ran on the machine throughout, so its lag figures were measured under load.
- **Verdict:** **not ready as it stands: one major finding, no blocker.** The writer's ordering, atomicity, queue bound and draining
  on quit held up under every check.
- **What it ran:** doom-play tests in debug and release and on 1.85.0 (85 passed, 2 ignored each), format, Clippy and rustdoc on
  doom-play, 23 planted faults, scratch probes, a comparison of 1,792 master test bodies (none changed within the range). **Not run:**
  the other crates' tests, anything on Windows or macOS, the window code paths (read only), a real audio device, the genuine-engine
  kill check, sanitizers, fuzzing, mutation testing, CI, any timing.

| Finding | What it said | What was done (commits) |
| --- | --- | --- |
| M1 (major) | A failure of a session's last writes never reached the app: the end screen said DEMO SAVED and a restart did not retry (probe: 29 of 30 tics in the first file) | `App::settle_save` and the settle step in the window (`dce493d`, `b073255`, `c14b3a7`); the writer re-tries an earlier session once at the next claim and again at exit; tests with a real `App` (`d4c4642`, `fd87e57`) |
| m1 | The kill test could not detect a writer that stops; the W lower bound was nearly vacuous on a healthy disk (plants: W+3 capped at M-1 passed; a writer sleeping forever passed) | The lag bound of 525 tics, `DOOM_PLAY_KILL_MAX_LOST`, and the `early` and `stall` self-tests (`ceec231`); the stale sentence in `session_file.rs` (`f7b45a2`) |
| m2 | The exit lines could contradict each other ("saved" followed by "failed again") | One true line per session, built after the settle; earlier sessions one line each (`c14b3a7`) |
| m3 | The input log's last pump line was lost, and its queue had no bound | The log is ended after the last pump line (`c14b3a7`); a bounded queue that drops whole pumps and says how many (`b8bd558`); tests (`fd87e57`) |
| m4 | Stale lines: `session_file.rs` 54-58, `session_writer.rs` 33; COMPATIBILITY "loses at most the last tic"; this record's "every tic run, or every tic but the last"; the kill checker's docstring | Code and tool comments in `f7b45a2`; the documentation lines in the commit of this record |
| m5 | Plants that survived: 10 of 23 | Tests added (table below) |
| n1 to n11 | Notes (a stat on the loop thread, weaker OS-crash durability, Windows clip timing, detector window size, an unreachable reason, a lost claim race loses the session, synchronous claim after `finish`, write counts, the log's session line, the fallback line on Windows, the master-test rule) | n8 done (write counts, `c14b3a7`); n7 left (a case that does not occur); n1 to n6 and n9 to n11 left as they were, and n1, n2 and n6 are stated in this section |

**Plants:** 23 planted, **13 killed, 10 survived** at `3957cf0` (killed: P3, P4, P5, P6, P9, P10, P12, P13, P17, P26, P28, P29,
P30; survived: P1, P7, P8, P14, P19, P21, P23, P24, P25, P27). P7 and P8 belonged to M1; P14 is equivalent under the current call
patterns; P1 (a skip rule `<=` against `<`) is close to an equivalent mutant and was left.

| Plant | Test added | Result |
| --- | --- | --- |
| P19 (the `is_file` guard of the clean-up) | `the_start_removes_regular_files_only_whatever_their_age` (a directory and a symlink of leftover names stay) | caught |
| P21 (a foreign snapshot dropped) | `a_demo_of_another_session_is_not_written_and_is_reported` | caught |
| P23 (an in-place replacement drops a synced flag) | `a_synced_snapshot_replaced_in_place_stays_synced` | caught |
| P24 (a length mismatch not marked stale) | `a_tic_out_of_step_marks_the_file_stale` | caught |
| P25 (`finish` neither drains nor goes synchronous) | `after_finish_every_call_waits_and_returns_its_real_outcome` | caught |
| P27 (idle pumps counted as silent) | `idle_pumps_do_not_count_as_silent` | caught |
| P8 (`close` drops a failure the app never heard of) | `a_failure_the_app_never_heard_of_is_in_the_report` | caught |
| P7 (a superseded failure) | the path no longer exists; its two successors (a stray failure dropped, an earlier session left behind dropped) were planted | both caught |
| the writer-died check in `outcome` | `a_writer_that_died_is_never_reported_as_having_written` | caught |

The window's own call of the settle step survived a plant (removing it): `Stage` needs a GL context, so no test drives it; the logic it
calls is tested through `Play`. In `doom-game`, plants of no redraw, `retry_from` kept, and the claimed guard dropped were each caught
by the five tests of `app_settle.rs`; a guard on the recorded demo proved redundant and was removed.

### Review 4 and its follow-ups

- **Reviewer:** Claude Opus 5.5 (`claude-opus-5-5`), effort **xhigh** (set by the orchestrator at spawn), a fresh agent that wrote none
  of the code. **Range:** `3957cf0..f7b45a2` (`crates/doom-play`, `App::settle_save`, the kill checker's docstring), in a scratch
  worktree; probes and plants reverted. A kill-test job ran on the machine throughout, so nothing it timed is a measurement.
- **Verdict:** **ready, with minor follow-ups; no blocker and no major finding.** Review 3's M1 is resolved: a failure of a session's
  last writes reaches the app at the session's end, after a pause and at quit; no path blocks the loop while tics run; the reviewer
  found no way to show DEMO SAVED for a file that does not hold every tic; no write goes over a name the process did not claim, and
  no older demo over a newer one.
- **What it ran:** format, Clippy, rustdoc and the 1.85.0 check on `doom-play` and `doom-game`; their tests in debug and release
  (`doom-play` 105 passed, 2 ignored; `doom-game` 146 passed); the three self-tests, each failing its plant; three scratch probes;
  plants; a comparison of 1,793 master test bodies (one differs, the comment change of A4, outside this range). **Not run:** the
  other crates, Windows, macOS, the window code paths (read only), the genuine-engine kill check, sanitizers, fuzzing, mutation
  testing, CI, any timing.

| Finding | What it said | What was done (commits) |
| --- | --- | --- |
| R1 (minor) | A quit after the run had ended (the window's drop path) replaced the real failure reason in the exit line with "the session writer was closed" | `Play::quit` returns at once after the run's end (`b5c2b5f`); the reviewer's probe as a test (`c8b6edd`) |
| R2 (minor) | The writer skipped its rewrite at the next claim after any failed whole demo, not only after the app's retry: a catch-up that failed after R with no poll between left the earlier file at 21 of 260 tics until exit | After a first failure the writer makes one more whole write of that session at the next claim; only a failed retry of the same copy is not repeated (`a81636b`). Review 5 found the limit: when two whole writes are already in flight and both fail, the second counts as that one write (N-a below); the probe as a test, both outcomes (`af1c359`) |
| R3 (minor) | Three guards survived plants: the check that keeps a short file from being called saved, the zero-wait check of a paused poll, and the hand-over of an earlier session's failure | Three tests (`af1c359`); each plant again: 104 passed, 1 failed |
| R4 (minor) | Two documentation lines stale at `f7b45a2` | Both reworded in `489f912` |
| N2 (note) | The `stall` self-test's margin narrowed when the first write was slow | The plant now stops before the file would hold more than 200 tics, whatever the operations cost (`90f4710`) |
| N5 (note) | A 50 ms wait in the log-bound test could flake | The test waits on a gate and asserts the dropped count exactly (`31f3706`) |
| N4 (note) | After a retry whose later tics were not handed over, the pause box says LAST DEMO SAVE FAILED while the last write succeeded; the file is behind, and quit or restart writes it all | Left; the new R3 test pins it |
| N6 (note) | If the writer thread panics, earlier sessions left behind are not rewritten at exit | **Not fixed** *(later the same day the exit was made to name them; they are still not rewritten: [A9](#a-writer-thread-panic-names-the-sessions-not-saved))* |

### Review 5 (the follow-ups)

- **Reviewer:** Claude Opus 5.5 (`claude-opus-5-5`), effort **xhigh** (set by the orchestrator at spawn), a fresh agent that wrote none
  of the code. **Range:** `f7b45a2..d9cd771`, `crates/doom-play`, in a scratch worktree; plants reverted. The gates ran on the machine
  at the same time; nothing was timed.
- **Verdict:** **the code in the range is ready, with notes only; the head was not**, because the Windows leg of CI failed on
  `d9cd771` in a test from before the range (W1). R1 and R2 are fixed; review 4's invariants still hold (the loop never waits on the
  disk, no DEMO SAVED for a short file, no rename over a name not claimed, no older demo over a newer one, the queue bound).
- **What it ran:** format (both workspaces) and Clippy; `doom-play` (109 passed, 2 ignored) and `doom-game` (146 passed) in debug and
  release; the three self-tests, each failing its plant; plants C1, D, K, R1, four on R2, the revert of both fixes and N5, **each
  killed by its test**; the five changed tests 100 times each (500 passes, 0 failures); 1,795 master test bodies compared (the one
  known difference). **Not run:** the other crates, rustdoc, the 1.85.0 check, Windows or macOS locally, the window code, sanitizers,
  fuzzing, mutation testing, the C generators.

| Finding | What it said | What was done |
| --- | --- | --- |
| W1 (major, outside the range) | `each_earlier_session_left_behind_gets_one_line` (from `fd87e57`) expected `/s/a.lmp`; on Windows the joined path has a backslash, so the Windows test step failed on `d9cd771` (`doom-play` 100 passed, 1 failed) and every later Windows step did not run. The product code was right | The test builds its expected path from the platform's join (`7431dd2`) |
| N-a | The rewrite at the next claim is one write, not a guarantee: with two whole writes in flight that both fail before the app hears of either, the earlier session's file stayed short (21 of 210 tics in a probe) through the next session and was saved only at exit | **Left as it is**; stated here and in the R2 row above. A kill during that next session would leave the earlier file short, still a valid demo |
| N-b | Three plants of the new retry bookkeeping survive; they change how many write attempts are made, never the data. "Not rewritten again at once" has no test in the claim case | Left |
| N-c | One wall-clock assertion in a new test (under 500 ms); it does not decide the outcome | Left |
| N-d | The `stall` self-test would miss if the first claim took longer than the child's run to tic 1,000 (missed all 10 kills in release with a planted 1.5 s claim; caught in debug). Not a regression | Left |

**`7431dd2` itself (one test's expected strings) was read by no reviewer.** Before it, Windows Clippy had stopped CI on `5855b36`
(an unused `mut` off Unix in a test, `d9cd771`). **So no test of the second round's code had run on Windows until the CI run on
`7431dd2`.**

### Local timing

Linux (WSL2, kernel 6.18, WSLg), release binaries built on and run from ext4 under `~/scratch/m5d` (the `before` build is `b374aaa`
extracted with `git archive`; the `after` build is the writer code at `26de27d`, without the cursor commits). `--no-sound --tic-stats
--quit-after-tics 2200` on the synthetic set with nothing pressed. The machine was idle apart from the run: no build was running and
the load average at each start was 0.10 to 0.96 (the first `before` run began at 1.83 just after a build and opened paused, and is not
counted). It is not claimed to be a controlled benchmark.

| Run | Tics | Dropped | Largest interval (us) | Intervals more than 1 ms from 1/35 s | Pumps with `ran` of 2 or more (log) |
| --- | ---: | ---: | --- | ---: | --- |
| before, no log | 2,200 | 0 | 28,827 | 0 | n/a |
| after, no log (2 runs) | 2,200, 2,200 | 0, 0 | 28,853, 28,984 | 0, 0 | n/a |
| before, `--input-log` (2 runs) | 2,200, 2,200 | 0, 0 | 28,766, 28,861 | 0, 0 | 0, 0 |
| after, `--input-log` (2 runs) | 2,200, 2,200 | 0, 0 | 28,813, 28,852 | 0, 0 | 0, 0 |

The four `after` exit lines read `session file writes: 2201 (claims 1, appends 2196 of 2196 tics, whole rewrites 2 synced and 2
unsynced), 0 failed; longest 6.8 to 7.0 ms (synced rewrite at tic 1050 or 2100); at most 1 tics waiting`, and with the log `input log
writes: 2201, 0 failed; longest 0.1 ms`, `writes over 20 ms: none`. **Reading:** on this SSD neither build stalls, so these runs cannot
show an improvement; they show that the change costs nothing here and that the exit lines work. The seam tests (host calls under 50 ms
with every operation 200 ms slow; a loop that stays prompt with the disk stalled) are the evidence that the loop no longer waits. The
owner's retest on `D:` measures his case. These figures predate the review-3 fixes and were not repeated at `f7b45a2`.

### Gates of the second round

**Full workspace gates at `d9cd77194c5bcd2a6479f9ba95dd124a62795481`**, from a clean clone under `~/scratch/m5d/gates2` (ext4), by a
gates agent (Claude Sonnet, effort medium; the docs-tools role), 2026-11-20 12:09 to 14:28 PST, with the commands of
[A5](#full-workspace-gates). `HEAD` and a clean `git status` were checked at the start and end of every phase. Build jobs capped at 4
and C reference jobs at 2; review 5 ran on the machine for the first part. `/mnt/c` showed 29 GB free at every check. The
address-sanitized genuine C runs used `detect_leaks=0` (**not LeakSanitizer coverage**); the fuzz replays and campaigns used
`detect_leaks=1`. **Every gate that ran passed.** No timing is claimed.

| Gate | Result |
| --- | --- |
| Build, format (both workspaces), Clippy and rustdoc with warnings denied | passed, 0 warnings |
| `cargo test --workspace --locked` | **1,976 passed, 0 failed, 6 ignored** (139 result lines) |
| Release `doom-sim`, `doom-types` | **1,393 passed, 0 failed, 2 ignored** |
| Release `doom-render`, `doom-sound`, `doom-game`, `doom-play` | **550 passed, 0 failed, 2 ignored** |
| Rust 1.85.0, `cargo +1.85.0 test --workspace --locked` | **1,976 passed, 0 failed, 6 ignored**; the known dead-code warning only |
| Job `stable`, Linux steps after the Rust gates | 64 steps passed; 3 non-Linux steps skipped as on CI |
| Jobs `M5a genuine renderer`, `M5b genuine status bar, HUD and input`, `M5c genuine sound` | all steps passed, self-tests included |
| README animations, `--check`; synthetic local tools; fuzz seed tools | identical; self-checks pass, planted faults caught |
| Fuzz build, AddressSanitizer, target compiled only | 60 of 60 built |
| Fuzz seed replays | 7 targets, all exit 0 |
| Fuzz AddressSanitizer campaigns, 300 s each | 7 targets, no findings, 0 artifact files; **the other 53 targets were compiled only**; no fuzz target exists for `doom-play` |
| Kill check through the genuine engine, synthetic (100 kills) | 122 files, 57,494 tics, **122 PASS, 0 FAIL**; `SELF-CHECK: PASS` |
| Kill check through the genuine engine, IWAD copy (50 kills) | 48 files, 6,913 tics, **48 PASS, 0 FAIL**; `SELF-CHECK: PASS` |
| Lost display, windowed `doom-play --no-sound` (display, SIGINT, SIGKILL; 4, 3 and 3 runs) | 10 of 10 PASS, no run without a demo |
| Kill test, 500 kills, release, ext4 (the Rust checker) | passed; 501 files, 202,160 tics; lost 0 in 436 kills, 1 in 58, 2 or 3 in 5, 40 in 1; 120 kills left no `.lmp`; the three self-tests each failed their plant in the debug and release test runs |
| The owner's 13 earlier session demos through the genuine engine | **13 of 13 `VERDICT: PASS`**, 27,146 tics |

**After the gates, two commits changed test code only** (`7431dd2`, a path separator in one test's expected strings; `47cdd02`, a gate
in place of a sleep in one test; see CI below). **The full gates were not repeated on them.** At the final code head
`47cdd02df6dead5fe63b58c3b698cf787d2e10d9`, from a clean clone on ext4 (`~/scratch/m5d/gates2/ir`), the orchestrator ran:

- **Instruction counts** (job `Simulation instruction counts`; the gates agent had left it for an idle machine): alone, load average
  0.84 at the start, 14:30 PST: `threshold 3.0%: ok`, 9 workloads, largest increase +0.92%, malloc and realloc counts equal to the
  baseline. It counts instructions, not time.
- `cargo test --locked -p doom-play -p doom-game`, debug and release: **255 passed, 0 failed, 2 ignored** each; Clippy on the
  workspace with warnings denied, format, and the 1.85.0 check of the two crates: clean. The changed test, 60 runs: 60 passed.

**Not run in this round:** Windows and macOS locally (CI below is the only evidence); any audio device or played sound; any timing or
benchmark of play (the Local timing table above predates the review fixes); local mutation testing at any head of this round (CI's workflow ran, below); LeakSanitizer
coverage of the genuine C runs; `DOOM2.WAD`; the real-IWAD M5c tools beyond the 13 demos. Slips of the gates agent's harness (a wrong
path for four fuzz seed-tool commands, re-run and passed) were not product failures.

### Gates, CI and the corrections to the earlier summary

The full gates of A5 ran at `6e61fc8`; the second round's code has its own gates ([above](#gates-of-the-second-round)) and CI
(below). No sentence of A5 is a claim about this round's code. The author-side checks the writer
and fix agents ran (format, Clippy, doom-play and doom-game tests, MSRV `cargo check`, rustdoc, plants) were run on `doom-play` and
`doom-game` only; the other crates are untouched by the round.

**Corrections to the earlier summary, stated here by the owner's demand:**

- **The genuine engine checked 540 kills, plus the 150 in the gates** (A5, table C: 230 synthetic and 310 IWAD kills; the gates'
  100 synthetic and 50 IWAD kills). **Every other kill went through the Rust checker only** (the kill test's own check: parse,
  replay, digest, tic count). An earlier summary's larger figure of kills checked implied more than that.
- **Fuzzing:** 60 targets built (AddressSanitizer, compiled only for 53 of them), 7 seed-replayed and 7 run as 300 s AddressSanitizer
  campaigns, 53 compiled only; no fuzz target exists for `doom-play`. This is A5's table; it ran at `6e61fc8`.
- **The CI mutation workflow** (`mutants.yml`) ran on `e25388d` and `cdb11cc` (A5) and, in this round, on `47cdd02` (below); it does
  not mutate `doom-play`.

### CI, the mutation workflow and the Windows artifact of the second round

**CI on the final code head, `47cdd02df6dead5fe63b58c3b698cf787d2e10d9`:** all nine jobs of
[run 44829468285](https://github.com/Copgunn/DOOM/actions/runs/44829468285) passed: Detect changes, Stable on ubuntu-latest,
windows-latest and macos-latest, Minimum Rust 1.85.0, Simulation instruction counts, and the M5a, M5b and M5c genuine-engine jobs.
**It is the first run in which the second round's tests ran to the end on Windows.** The three runs before it failed, each on test code:

| Head | What failed | Fix |
| --- | --- | --- |
| `5855b36` | Windows, Clippy: an unused `mut` off Unix in a test; the Windows build and tests did not run | `d9cd771` |
| `d9cd771` | Windows, tests: a test expected `/s/a.lmp` where Windows joins with a backslash (`doom-play` 100 passed, 1 failed); later Windows steps did not run | `7431dd2` |
| `7431dd2` | macOS, tests: `tics_queued_behind_a_failure_wait_for_the_retry` slept 15 ms to land inside a planted 60 ms delay, and the runner was slower than that (`doom-play` 108 passed, 1 failed). The test dated from the writer commits and had passed on macOS at `5855b36` and `d9cd771` | `47cdd02`: the writer is held at a gate instead |

Runs on the heads between `b374aaa` and `5855b36` were cancelled by later pushes. Commits after `47cdd02` change documentation only and
take CI's documentation fast path; the run on the final head is GitHub's record, linked from the pull request. **Other sleeps remain in
the new tests** (review 5's N-c, and the timed opener in the quit tests of `closing_tests.rs`); none failed in these runs, and none was
run as a flake campaign.

**The CI mutation workflow** ran on `47cdd02` ([run 44830321031](https://github.com/Copgunn/DOOM/actions/runs/44830321031), started by
re-adding the label): 24 shards, **37 mutants on the changed lines of `doom-game` (the workflow does not mutate `doom-play`): 36 caught,
1 missed, 0 timed out, 0 unviable.** The missed one is `App::append`'s `n > 0` to `n >= 0`, the equivalent mutant of A5. Before this it
had run on `e25388d` and `cdb11cc` only. **No mutation run of any kind covered `doom-play`'s second-round code** (the writer threads,
the cursor code, the settle step); the reviewers' hand plants are the only fault-injection evidence for it.

**The Windows artifact** for the owner's retest is built by the `Windows artifact` workflow on the final head of the pull request,
after this record's commit; the run and the artifact are GitHub's record, linked from pull request
[#64](https://github.com/Copgunn/DOOM/pull/64).

## A8. The owner's retest

*(The retest was carried out on 2026-11-20; its results and the owner's rulings are in
[A9](#a9-the-owners-retest-of-2026-11-20-his-rulings-and-the-close). The plan is kept as it was written.)*

Nothing is marked fixed until the owner rules. This is the plan for one sitting with the artifact built on the final head of pull
request [#64](https://github.com/Copgunn/DOOM/pull/64) (the run is linked in the PR). Use **Command Prompt** (`cmd.exe`). Replace
`C:\path\to\DOOM.WAD` with the IWAD's path and run from the folder the artifact is unzipped into. Make the folders first:

```
mkdir D:\DoomSessions\m5d-retest\launch D:\DoomSessions\m5d-retest\run2min D:\DoomSessions\m5d-retest\kills
mkdir C:\DoomSessions\m5d-retest\audio
```

Do not open, rename or delete files in them. The orchestrator reads `D:` at `/mnt/d/DoomSessions/m5d-retest`; `C:` files are to be
copied there afterwards or sent as named.

### (i) Launch capture (A1)

Change the display scale (Settings, System, Display, Scale) **immediately before each launch**, then launch, and from the first
second hold the left button and drag hard towards each edge in turn (at least ten drags to each edge, releasing while moving) for
about a minute. Do not resize, maximise or move the window. Quit with Esc, then Q. Run `NAME` and the scale in this order, with the
scale set just before the run:

1. `ch100`: set 100%, launch.
2. `ch150`: set 150%, launch.
3. `ch125`: set 125%, launch.
4. `r100-1`, `r100-2`, `r100-3`: set 100% once, then launch three times without changing it.
5. `r125-1`, `r125-2`, `r125-3`: set 125% once, then launch three times.

```
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions D:\DoomSessions\m5d-retest\launch --input-log D:\DoomSessions\m5d-retest\launch\NAME.txt 2> D:\DoomSessions\m5d-retest\launch\NAME-terminal.txt
```

**Accepted for a run when** the last line of its log, `summary ... raw downs/ups: Left D/U`, has D equal to U **and** the log has no
line containing ` resize `. Also report, per run, any line containing `capture again` or `cursor-left`, and if fire sticks, the time
and what you did to clear it.

### (ii) One two-minute run on D: on purpose (the hitch, A7)

Recording to `D:` is the point; leave the sessions folder and the log on the slow disk. Play for two minutes without pausing (about
4,200 tics), then quit with Esc, then Q:

```
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions D:\DoomSessions\m5d-retest\run2min --input-log D:\DoomSessions\m5d-retest\run2min\input-log.txt --tic-stats 2> D:\DoomSessions\m5d-retest\run2min\terminal.txt
```

Looked for in the log: **no pump with `ran` of 3 or more at a rewrite tic** (tics 1,021, 1,050, 2,045 and 2,100). **Send back, from
`terminal.txt`, the exit lines** starting `doom-play: session file writes` and `doom-play: input log writes` (the number of writes, the
longest write in milliseconds, the deepest queue as "at most W tics waiting") and the line starting `doom-play: writes over 20 ms`.

### (iii) Sound runs a, d and e again, sessions on C:

By ear, about a minute each with plenty of shooting, door and switch sounds; quit with Esc, then Q. Sessions and logs on `C:`
(an SSD), so that disk stalls do not contaminate the underruns:

```
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions C:\DoomSessions\m5d-retest\audio 2> C:\DoomSessions\m5d-retest\audio\a-default.txt
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions C:\DoomSessions\m5d-retest\audio --experimental-audio-period-frames 128 2> C:\DoomSessions\m5d-retest\audio\d-period128.txt
doom-play.exe --iwad C:\path\to\DOOM.WAD --bindings modern --sessions C:\DoomSessions\m5d-retest\audio --experimental-audio-period-frames 128 --experimental-audio-start-buffers 1 2> C:\DoomSessions\m5d-retest\audio\e-period128-start1.txt
```

One line per run in a `notes.txt` (the letter, the delay against `a`: shorter, same or longer, and any crackle or gaps). **Send back
the `audio` exit lines** (`audio device`, `audio settings`, `audio callbacks`, `audio queue`) of each. The default period stays 512
whatever the result; the result only informs a later choice.

### (iv) Optional: Ctrl+C and console close again

As in A6, part 3, with the console visible (no `2>`), three runs each of about 2 s, 10 s and 40 s or more, into
`D:\DoomSessions\m5d-retest\kills`. Expect a file that may lack the last tics the disk had not yet taken (the
[new limit](../COMPATIBILITY.md#a-session-is-safe-on-windows-and-macos-only-when-doom-play-ends-it-a-limit-not-yet-lifted-there));
the files are checked with the session check, each must replay.

### What the orchestrator does with it

Reads each launch log for downs and ups, `resize`, `capture again` and `cursor-left` lines; reads the `ran` column at the four rewrite
tics; reads the exit lines; runs every `.lmp` through the session check. **Until the owner has ruled, nothing is marked fixed:**
stuck fire stays a known fault, recording to a slow disk stays documented as causing hitches and dropouts, and the pull request stays
a draft.

## A9. The owner's retest of 2026-11-20, his rulings and the close

**Scope of this section.** It records the owner's retest of [A8](#a8-the-owners-retest), his rulings on it, and what was done before the
pull request was marked ready. The figures come from his report and from the orchestrator's reading of his files (read only; counts and
verdicts only are recorded): `/mnt/d/DoomSessions/m5d-retest` for the launch runs and the two-minute run, and the three exit-line files
in `C:\DoomSessions\m5d-retest\audio` for the sound runs. Where the files show more than the report said, that is stated.

### The retest

- **When and on what.** 2026-11-20, 16:34 to 16:57 PST, Windows 11, the `doom-play-windows-x86_64` artifact built at `c9f54ee`
  ([run 44838097212](https://github.com/Copgunn/DOOM/actions/runs/44838097212)). By the files' times the order was: the two-minute
  run, the three sound runs, then the launches `ch100`, `ch150`, `ch125`, `r125-1` to `r125-3`, `r100-1` to `r100-3` (the plan listed
  the 100% repeats before the 125% ones). **Part (iv), Ctrl+C and console close, was not run:** the `kills` folder is empty, so no
  kill of the writer-thread build has been made on Windows.
- **Smart App Control.** Windows 11's Smart App Control blocked this build outright ("blocked by your organization's Device Guard
  policy"), while the `b374aaa` build still ran on the same machine. The owner turned Smart App Control off to test. The artifact is
  unsigned. Why one unsigned build ran and the next did not was not investigated.

**The two-minute run on `D:`** (`run2min`, `--input-log --tic-stats`; the sound device was the Bluetooth headphones of the first
session):

| Measure | Figure |
| --- | --- |
| Tics | 4,587 executed, 0 dropped; every pump that ran a tic ran exactly one (no `ran` of 2 or more) |
| Tic intervals | mean 28,571.4 us, largest 31,783 us; 22 more than 1 ms from 1/35 s, **0 more than 5 ms** |
| Sound | underruns 0, overruns 0; queue depth mean 1,216.7, max 1,536 frames |
| Session file writes | 4,539 (1 claim, 4,530 appends of 4,579 tics, whole rewrites 4 synced and 4 unsynced), 0 failed; longest 481.9 ms (the claim, at tic 0); **at most 11 tics waiting** |
| Input log writes | 4,593, 0 failed; longest 303.5 ms (at tic 0) |
| Writes over 20 ms | 26: the claim; 6 of the input log (50.7 to 303.5 ms); 15 appends (24.4 to 116.9 ms; 13 of them between 74 and 117 ms), at tics 97, 411, 612, 670, 1,002, 1,242, 1,594, 1,815, 2,387, 2,580, 2,954, 3,330, 3,341, 3,532 and 4,105; and all 4 synced rewrites, at tics 1,050, 2,100, 3,150 and 4,200 (128.2, 122.7, 144.2 and 131.3 ms). No unsynced rewrite is on the list |
| Left downs/ups | 27/27; 27 `capture again (button-down)` lines; no `cursor-left`, `resize` or focus line |

The owner felt no hitch. **His reading, which the list supports:** the appends of about 100 ms that recur every few hundred tics were
the stalls of the first session that no rewrite explained, and the synced rewrites were the ones that followed tics 1,050, 2,100 and so
on. Those writes still take as long on his disk; they no longer hold the loop. This is one run on one disk.

**The nine launches** (`--input-log`, sessions and logs on `D:`, window 960 by 720 in every log):

| Run | Tics | Left downs/ups | `capture again` lines | Releases at an edge | First raw motion, first press (pump) | Longest session-file write | Most tics waiting |
| --- | ---: | --- | ---: | ---: | --- | --- | ---: |
| `ch100` | 2,329 | 57/57 | 57 | 41 | 44, 47 | 634.2 ms (claim) | 17 |
| `ch150` | 1,871 | 40/40 | 40 | 35 | 1, 31 | 147.5 ms (claim) | 5 |
| `ch125` | 1,899 | 37/37 | 37 | 33 | 1, 32 | 146.9 ms (claim) | 4 |
| `r125-1` | 1,108 | 6/6 | 6 | 1 | 31, 125 | not available | not available |
| `r125-2` | 1,384 | 6/6 | 6 | 0 | 6, 130 | 178.7 ms (claim) | 5 |
| `r125-3` | 1,371 | 15/15 | 15 | 3 | 20, 173 | 151.8 ms (claim) | 5 |
| `r100-1` | 1,318 | 7/7 | 7 | 1 | 12, 257 | 146.8 ms (claim) | 5 |
| `r100-2` | 948 | 29/29 | 29 | 3 | 16, 71 | 114.6 ms (an append) | 5 |
| `r100-3` | 1,154 | 35/35 | 35 | 4 | 14, 30 | 140.6 ms (an append) | 5 |

- In all nine: downs equal ups (232 in all), no `cursor-left` line, no `resize` line, no focus line, no pump with `ran` of 2 or more and no
  dropped log line; in the eight whose exit lines exist, 0 failed writes and underruns 0. All 232 `capture again` lines give the reason `button-down`, one per press; **the
  focus-gained and cursor-left reasons never occurred, so those two paths and the detector did not act in this retest.** Fire never
  stuck, by the owner's report.
- `r125-1-terminal.txt` is empty (it was written again 12 s after its log; an empty file with a mistyped name beside it suggests a
  repeated command), so that run's exit lines are not available. Its input log and demo are complete.
- **The most tics waiting on `D:` was 17, in `ch100`,** not the 11 of the two-minute run; the owner's report gave the two-minute run's
  figure. The documentation cites 17.
- "Releases at an edge" counts button-up lines whose position is on the window's first or last row or column. The three launches after
  a scale change were dragged hard against the edges; the six repeats had far fewer edge releases than the plan asked.

**What the nine launches show, and what they cannot.** No release was lost in 232 presses across nine launches, three of them the first
launch after a change of display scale, the condition of both failures in the first session. **The cause of the launch-capture failure was
never reproduced, on this machine or on his, and nine clean launches cannot show whether the launch grab would have failed without the
re-grab:** no log line says whether the first clip held, and the build asks for the grab again at every press, so a launch whose first
clip failed and one whose first clip held look alike once a button has gone down. One reading bears on it and settles nothing: in both failed launches of the first session no raw motion arrived before the
first press; here raw motion arrived before the first press in all nine, in `ch100` only three pumps before it (pump 44, press at 47). Both
failures of the first session were first launches after a scale change (2 of 2 such launches then); here 3 of 3 such launches were
clean. Five launches then and nine now are too few to give a rate, and the two builds were not run side by side.

**The sound runs on `C:`** (wired, "Speakers/Headphones (Realtek(R) Audio)", WASAPI, stream 11,025 Hz; `--audio-report` was on, and by the
owner's reading of it Windows granted the period times three as asked; one run each, by ear; **not a latency measurement**):

| Run | Options | Tics | Callbacks (frames each) | Queue depth, frames (mean, max) | Underruns (frames) | Longest session-file write; most tics waiting | By ear |
| --- | --- | ---: | --- | --- | --- | --- | --- |
| a | default | 2,843 | 1,800 (512) | 1,019.0, 1,024 | 0 | 7.6 ms; 1 | quicker than on Bluetooth |
| d | period 128 | 2,362 | 5,885 (128) | 786.0, 1,152 | 0 | 6.6 ms; 1 | quicker than a |
| e | period 128, start buffers 1 | 2,379 | 5,914 (128) | 566.3, 1,024 | 1 (128) | 6.0 ms; 1 | quicker than a; not distinguishable from d |

Against [the arithmetic](#ruling-3-withdrawn-the-arithmetic-as-a-report): run a's maximum of 1,024 and run d's mean of 786.0 and maximum
of 1,152 lie inside the model's ranges for their periods (512 to 1,536; 384 to 1,152, mean 791.5); run e's maximum of 1,024 is above the
model's 896 for a one-buffer start, in the run that had an underrun. So with the sessions on an SSD the depths of the first session's
`D:` runs (maxima of 1,920 and 2,048) did not recur, which fits the owner's conclusion that disk stalls had inflated them. One run each.

### The session check of his files

The orchestrator, 2026-11-20 (PST), with `tools/local/m5c_session_real.py` from a scratch worktree at `c9f54ee`, a scratch copy of the
IWAD, the genuine engine in its strict, overflow and address variants and the genuine mixer:

- **Every `.lmp` under `/mnt/d/DoomSessions/m5d-retest`: 10 files, 10 of 10 `VERDICT: PASS`, 0 FAIL, 17,969 tics** (the nine launch
  sessions, 2,329, 1,871, 1,899, 1,108, 1,384, 1,371, 1,318, 948 and 1,154 tics, and the two-minute run, 4,587). The three variants
  were identical in all ten. Each file's tic count equals the last tic of its input log.
- **Beyond what the owner asked:** the three sound-run demos in `C:\DoomSessions\m5d-retest\audio`, from scratch copies: **3 of 3
  `VERDICT: PASS`**, 7,584 tics (2,843, 2,362 and 2,379).
- **The address variant ran with `ASAN_OPTIONS=detect_leaks=0`: leak detection was disabled, so it is not LeakSanitizer coverage.** The
  check shows that the files replay equal to the genuine engine; it does not show what he saw or heard. Every session was ended by
  Esc then Q, so these files say nothing about a kill. The logs are in scratch (not committed).

### The owner's rulings (2026-11-20, after the retest)

| Ruling | What was done |
| --- | --- |
| **Stuck fire (A1): accepted as fixed** | README and COMPATIBILITY no longer list it as a known fault; the limits stay stated (the cause never reproduced; the re-grab is not a mouse capture) |
| **The hitch and the slow-disk sound dropouts: accepted as fixed** | The "keep sessions on an SSD" advice is replaced by the limit that remains: a kill can lose the tics still queued (at most 17 in his ten runs on `D:`; 620 seen once through WSL's view of that drive, in the kill test, whose child runs far faster than play) |
| **The 2-second wait at a session's end on a stalled disk: accepted as built** | Nothing changed |
| **The open review items: accepted for this pull request as documented limits** | Listed [below](#the-four-open-review-items-accepted-as-documented-limits); the writer-panic one is on the [roadmap](../ROADMAP.md) |
| **A writer-thread panic must print a clear error at exit naming the sessions not written** | It was not there: the exit said only that the writer had stopped and named no session. Added, with tests ([below](#a-writer-thread-panic-names-the-sessions-not-saved)) |
| **The default period: 128 frames with a two-buffer start is the owner's choice, on run d. Not in this pull request** | Nothing changed here: the default stays 512 and both experimental switches stay. It goes in its own small pull request after this one merges, which edits the master test `the_numbers_are_those_of_the_specification` openly |
| The leftover worktree of the review-4 fix agent and its local branch may be removed; nothing else | Removed (`.claude/worktrees/agent-aac9fb778d3d3aff3`, `fix4-work`, whose commits are all on this branch) |
| Smart App Control | One paragraph in [COMPATIBILITY](../COMPATIBILITY.md#windows-smart-app-control-blocks-the-unsigned-build); code signing is on the [roadmap](../ROADMAP.md) for the first release |

### The four open review items, accepted as documented limits

| Item | From | The limit |
| --- | --- | --- |
| A writer-thread panic | Review 4, N6 | If the session writer's thread panics, the earlier sessions it had left behind (a failed last write) are not rewritten at exit, and the current session's file holds only its last completed write. **Since this section's change the exit names each such session and what its file holds; nothing is rewritten after the panic** ([below](#a-writer-thread-panic-names-the-sessions-not-saved)) |
| The rewrite of an earlier session at the next claim is one write, not a guarantee | Review 5, N-a | With two whole writes in flight that both fail before the app hears of either, the earlier session's file stays short through the next session and is saved only at exit; a kill during that next session leaves it short, still a valid demo |
| Three surviving plants in the retry bookkeeping | Review 5, N-b | They change how many write attempts are made, never the data; "not rewritten again at once" has no test in the claim case |
| Sleeps and one wall-clock assertion in new tests | Review 5, N-c, and the CI note | One assertion of under 500 ms that does not decide a test's outcome, and the timed opener in the quit tests of `closing_tests.rs`; none failed in CI and none was run as a flake campaign |

Two further notes of review 5 stay as recorded there and were not among the four: N-d (the `stall` self-test would miss a plant if the
first claim took longer than the child's run to tic 1,000) and review 4's N4 (the pause box's wording after a retry).

### A writer-thread panic names the sessions not saved

**What was there at `c9f54ee`** (read from the code): after a panic of the session writer's thread the exit printed one line, "the
session writer stopped unexpectedly (a panic); the session file holds its last completed write". It named no session; the earlier
sessions the writer still owed a rewrite were known only to the dead thread and were not mentioned; and the app's own line for the
current session said "not saved" with no path or tic count, also when its file was whole.

**What was built** (`crates/doom-play` only: `session_writer.rs`, `closing.rs`, `window.rs`; no existing test edited). The loop side keeps
every session name it handed over as a claim, in order, with the most tics the app recorded in it (one entry per session, not per
tic); the writer notes in the shared state what its last completed write of each session left in the file, or that another file took
the name. Only after a panic, at close, the two are compared. Nothing changes when the writer does not panic. Each line follows
`doom-play: `; `{who}` is "the session" for the session current at exit and "an earlier session" otherwise:

1. Nothing lost: `the session writer stopped unexpectedly (a panic); every session file it was handed holds its whole demo`
2. Something lost: `the session writer stopped unexpectedly (a panic); the sessions below were not saved, and no rewrite at exit saved them`
3. A short file: `{who} was not saved: the session writer panicked; {PATH} holds at least {T} of its {N} tics (its last completed write)`
4. No file: `{who} was not saved: the session writer panicked; {PATH} was not created (no write of it completed)`
5. The name taken: `{who} was not written: {PATH}: another file took the name before this session's file was created; it was left alone and this session is not written`
6. Sessions begun after the writer stopped: `the session was not saved, nor was any other session begun after the session writer stopped: no file was created for them` (or, if the current session has a name, `no session begun after the session writer stopped was saved: no file was created for them`)
7. The current session, when its file is whole: `saved {PATH} ({N} tics)`

Each session gets one line. A small ordering fault was fixed with it: the writer was marked stopped before its receiving end was
dropped, so a claim could be queued for no one; the receiver is now dropped first. The input log's writer was read and left as it
was: a panic there already prints "the input log stopped: ..." when it is found.

**Review 6** (Claude Opus 5.5, `claude-opus-5-5`; the reviewer definition sets effort xhigh, which the agent could not see from inside its
run; a fresh agent that wrote none of the code; range `c9f54ee..23dc8b4`, in a scratch worktree, plants reverted; a cargo-mutants run
shared the machine, so nothing was timed). **Verdict: ready, with follow-ups; no blocker and no major finding in the range.** It ran
format (both workspaces), Clippy (debug and release) and rustdoc with warnings denied and the 1.85.0 check on `doom-play` and
`doom-game`, their tests in debug and release (`doom-play` 116 passed, 2 ignored; `doom-game` 146 passed), the 7 new tests 100 times
each in debug and release (1,400 passes, 0 failures), 23 plants (15 caught, 8 survived: 2 equivalent, 1 a race no test can hit, 5 test
gaps), and compared 1,795 master test bodies (the one known difference). **Not run:** Windows, macOS, CI, the other crates, sanitizers,
fuzzing, mutation testing, the window code under a display.

| Finding | What it said | What was done (commits on the branch) |
| --- | --- | --- |
| X1 (major, outside the range) | `the_logs_queue_holds_1024_pumps`, a test of this pull request from `7af7799`, failed 2 of 24 full runs on the loaded machine: if the log thread took its first hand-over during the push loop, the dropped pumps were split over two lines | The test holds the log thread at its gate and asserts 1,025 pumps kept and one `dropped` line of 75 exactly (`ba50628`). With the reviewer's late first receive planted the old test failed 5 of 5 and the new one passed 5 of 5; 400 of 400 repeat runs passed |
| m1 (minor) | After a panic, an unheard failure of a session whose file is whole still printed "was not fully written" | No such line for a whole session (`4d6b4e1`), with a test |
| m2 (minor) | "holds T of its N tics" was a lower bound: an append batch that failed after its first write gave "2 of 4" with 3 in the file | The line says "at least T" (`9dd8d95`), with the probe as a test |
| m3 (minor) | Five plants survived (the taken-name and holds notes on two paths; the lines before the session line after a panic) | Four tests (`7cfdd77`, `d9122a5`); each plant again: caught |
| n1 (note) | A claim made after `finish` that fails could give one session two lines; unreachable through the window's run | The failed claim's name is used as the current session's after a panic (`af5dc90`, `814ba1b`), three tests. **Limit:** the argument rests on the window's call order, which the writer's interface alone does not enforce |
| n3 (note) | Doc-comment wordings | Corrected (`98bb0ea`) |
| n4, n5 (notes) | After a panic an earlier session's own error text is not repeated; while play goes on after a panic, a session begun after the stop reserves a new name at each retry and the pause box can be wrong until exit (both from before the range) | Left; on the [roadmap](../ROADMAP.md) with the rewrite at exit |

The fix agent's plants on the follow-ups: 14 planted, 13 caught by the final tests, and the timing plant of X1 passed as intended.
**The follow-up commits themselves were read by no reviewer.** Not tested: a panic inside the rewrite at exit beyond the one planted case, a real panic in the running program, and anything on Windows or macOS outside CI.

### The local mutation run on `doom-play`

CI's mutation workflow does not mutate `doom-play`, and no mutation run of any kind had covered the second round's code. The owner
asked for a local one. cargo-mutants 27.1.0, `-p doom-play --in-diff` on the diff of `crates/doom-play` against master (`d637f7d`), so
the writer threads, the re-grab and the cursor-left detector are all in it, with `kill_tests.rs` excluded (a test harness; 216 of the
777 listed mutants were in it); each test run under a 4 GB address-space cap and cargo-mutants' 300 s timeout; from a clean scratch
worktree on ext4 ([command](../TESTING.md#local-mutation-run-on-doom-play-cis-workflow-does-not-mutate-it)). Other jobs shared the
machine, so nothing here is a timing.

**First run, at `c9f54ee`** (2026-11-20, 17:05 to 18:13 PST, 3 jobs): **561 mutants: 463 caught, 61 missed, 7 timed out, 30 unviable.**

**Every survivor, by disposition** (a tester agent, Claude Sonnet, effort medium; 19 new tests in `3868b2c`'s commit, no product code and no
existing test line changed; it re-ran cargo-mutants on the 68 at its own head and the outcome matched each row):

| Disposition | Count | Which |
| --- | ---: | --- |
| Killed by a new test | 34 | 24 in `session_writer.rs` (the host's guards on a second session's name, `settle`, `stopped_short`, the forwarding of the clock, the sound and the mute key, and the writer's counters, `take_append`, `flush` and `fail`), 5 in `write_stats.rs`, 2 in `input_log.rs` (`LogThread`'s drop, `LoggedHost::appends`), 2 in `main.rs` (`FsHost::claim`'s fallback line, `run`), 1 in `window.rs` (`print`) |
| Timed out because a test hangs: detected | 6 | The three mutants of `WriterHost::wait_idle`, `Alive`'s drop, `Writer::run` replaced by nothing, and the `done` counter in `Writer::run`: with each, tests wait forever and the timeout ends the run |
| Timed out, yet failing existing tests | 1 | `main.rs:350`, `!=` to `==` in `FsHost::claim`: existing tests fail by assertion at once, but other tests then spin through 9,999 claim retries (about 42 s each), so the tool reports a timeout, not a catch. Slow, not a hang |
| Equivalent | 4 | `cursor_leave::crossed`, `>` to `>=` and `<` to `<=` (they differ only at a motion of exactly zero, where both return false); `write_stats::slow_line`, `>` to `>=` (at exactly 40 entries the branch leaves out none); `window_conf`'s `high_dpi` field, **equivalent on Linux only**: on Windows the existing test `the_window_configuration_asks_for_dpi_awareness_on_windows_only` fails it, and that was not run here |
| Not testable without a display or a real audio device | 23 | `window.rs`: `play` (it opens a window), `Stage`'s drop and the 17 mutants of `Stage`'s `update`, `draw` and event handlers, which need a miniquad GL context (among them the settle call, the re-grab request and the feeding of the cursor-left detector: **the window's own calls of the tested logic remain compile-checked only**); `audio.rs`: 4 in `probe`, which opens a real device (a probe test was removed earlier for that reason) |

- **So no survivor is left that a test could catch and does not**, by this reading; 27 survive by nature (4 equivalent, 23 behind a
  display or a device), and the 23 are a real gap: the glue between the window library and the tested logic has no test and was
  exercised only by the owner's sessions.
- One new test pins a rule no document states: of two equally long writes the exit line names the first. It is kept and flagged for
  the owner.
- The re-grab decision (`mouse_forward.rs`) and the cursor-left detector (`cursor_leave.rs`) had no survivor beyond the two equivalent
  ones: all 33 mutants of the first were caught, and of the second's 37, 34 were caught, 1 was unviable and 2 are the equivalent pair.

**Second run, at `3868b2c`** (the panic change, review 6's follow-ups and the 19 tests included; 2026-11-20, 19:35 to 21:12 PST, 2
jobs, the gates running beside it): **591 mutants: 521 caught, 31 missed, 7 timed out, 32 unviable.** The 7 timeouts are the 7 of the
table. Of the 31 missed, 27 are the table's 4 equivalent and 23 untestable ones, and **4 were new, all in the exit-line code added
today** (`closing.rs`: the condition of `Play::settle_after_update`, two conditions of `after_panic`, one of `writer_lines`). A tester
agent wrote four tests for them (`08946d5`, `0f26023`; no product code changed) and re-ran cargo-mutants on `closing.rs` alone at
`08946d5`: **47 mutants: 46 caught, 1 unviable, 0 missed, 0 timed out.** `0f26023` changes one line of one of those tests for Clippy;
the mutation run was not repeated on it, and **no whole-crate mutation run was made at the final code head `0f26023`.**

### Gates and CI of the close

**The commits of the close** (`c9f54ee..0f26023`, 16 commits, `crates/doom-play` only): the panic lines and their tests (5), review 6's
follow-ups (8), the tests for the mutation survivors (3). The last product-code change is `814ba1b`; `7cfdd77` to `0f26023` are tests
only. No file outside `crates/doom-play/src` changed, and no master test, fixture, golden or trace.

**Full workspace gates at `3868b2c94342d8d4370b7aa548888bf495f40d96`**, from a clean clone under `~/scratch/m5d/gates3` (ext4), by a gates
agent (Claude Sonnet, effort medium; the docs-tools role), 2026-11-20 19:35 to 22:51 PST, with the commands of
[A5](#full-workspace-gates). `HEAD` and a clean `git status` were checked at the start and end of every phase. Build jobs capped at 4
and C reference jobs at 2; the second mutation run shared the machine for the first hour and a half. `/mnt/c` showed 28 to 29 GB free.
The address-sanitized genuine C runs used `detect_leaks=0` (**not LeakSanitizer coverage**); the fuzz replays and campaigns used
`detect_leaks=1`. **Every gate that ran passed; no test failed and nothing was re-run.** No timing is claimed.

| Gate | Result |
| --- | --- |
| Build, format (both workspaces), Clippy and rustdoc with warnings denied | passed, 0 warnings |
| `cargo test --workspace --locked` | **2,010 passed, 0 failed, 6 ignored** |
| Release `doom-sim`, `doom-types` | **1,393 passed, 0 failed, 2 ignored** |
| Release `doom-render`, `doom-sound`, `doom-game`, `doom-play` | **584 passed, 0 failed, 2 ignored** |
| Rust 1.85.0, `cargo +1.85.0 test --workspace --locked` | **2,010 passed, 0 failed, 6 ignored**; the known dead-code warning only |
| Job `stable`, Linux steps after the Rust gates | 64 steps passed; 3 non-Linux steps skipped as on CI |
| Jobs `M5a genuine renderer`, `M5b genuine status bar, HUD and input`, `M5c genuine sound` | all steps passed, self-tests included |
| README animations, `--check`; synthetic local tools; fuzz seed tools; documentation check | identical; self-checks pass |
| Fuzz build, AddressSanitizer, target compiled only | 60 of 60 built |
| Fuzz seed replays | 7 targets, all exit 0 |
| Fuzz AddressSanitizer campaigns, 300 s each | 7 targets, no findings, 0 artifact files; **the other 53 targets were compiled only**; no fuzz target exists for `doom-play` |
| Kill check through the genuine engine, synthetic (100 kills) | 120 files, 55,787 tics, **120 PASS, 0 FAIL**; `SELF-CHECK: PASS` |
| Kill check through the genuine engine, IWAD copy (50 kills) | 48 files, 6,778 tics, **48 PASS, 0 FAIL**; `SELF-CHECK: PASS` |
| Lost display, windowed `doom-play --no-sound` (display, SIGINT, SIGKILL; 4, 3 and 3 runs) | 10 of 10 PASS, no run without a demo |
| Kill test, 500 kills, release, ext4 (the Rust checker) | passed; 494 files, 196,513 tics; lost 0 in 435 kills, 1 in 58, 2 or 3 in 5, 6 in 1, 9 in 1; the torn-append self-test passed (its planted torn file was refused) |
| The owner's 13 earlier session demos through the genuine engine | **13 of 13 `VERDICT: PASS`**, 27,146 tics |
| Every `.lmp` then under `/mnt/d/DoomSessions/m5d-retest` through the genuine engine | **14 of 14 `VERDICT: PASS`**, 25,844 tics: the 10 of [the session check above](#the-session-check-of-his-files) again, and 4 more (7,875 tics) that had appeared in a new `lag` folder between 17:28 and 18:46 PST, after the owner's report and outside it |

**After the gates, two commits changed test code only** (`08946d5`, four tests; `0f26023`, one line of one of them). **The full gates were
not repeated on them.** At the final code head `0f26023a8a13de95f3acbe80c74ebdc46d2d67b1`, from the same clean clone moved to that head
(`git status` clean), alone on the machine, 2026-11-20 22:52 to 23:30 PST, the orchestrator ran:

| Gate at `0f26023` | Result |
| --- | --- |
| Instruction counts (job `Simulation instruction counts`), load average 0.55 at the start | `threshold 3.0%: ok`, 9 workloads, largest increase +0.92%, malloc and realloc counts equal to the baseline. It counts instructions, not time |
| Build, format (both workspaces), Clippy and rustdoc with warnings denied | passed, 0 warnings |
| `cargo test --workspace --locked` | **2,014 passed, 0 failed, 6 ignored** |
| Release `doom-sim`, `doom-types`; release `doom-render`, `doom-sound`, `doom-game`, `doom-play` | **1,393 passed, 0 failed, 2 ignored; 588 passed, 0 failed, 2 ignored** |
| Rust 1.85.0, `cargo +1.85.0 test --workspace --locked` | **2,014 passed, 0 failed, 6 ignored**; the known dead-code warning only |

**Not run at `0f26023`:** the C generators and genuine-engine jobs, the fuzz builds and campaigns, the kill checks and the session checks
(all at `3868b2c` above; the two later commits add tests in `closing_tests.rs` only).

**CI.** Three runs, each with all nine jobs green (Detect changes, Stable on ubuntu-latest, windows-latest and macos-latest, Minimum
Rust 1.85.0, Simulation instruction counts, and the M5a, M5b and M5c genuine-engine jobs):

| Head | Run | What it was the first run of |
| --- | --- | --- |
| `23dc8b4` | [44851862388](https://github.com/Copgunn/DOOM/actions/runs/44851862388) | the panic lines and their tests on Windows and macOS |
| `3868b2c` | [44861791868](https://github.com/Copgunn/DOOM/actions/runs/44861791868) | review 6's follow-ups and the 19 survivor tests (two of them start the test program again as a child process) |
| `0f26023` | [44871340008](https://github.com/Copgunn/DOOM/actions/runs/44871340008) | **the final code head** |

No run failed in the close. Commits after `0f26023` change documentation only. They do not take CI's documentation fast path, because
they change this record and the workflow sends every change under `docs/validation/` down the full path; so the final head gets a full
run, which is GitHub's record, linked from the pull request. (The same holds for the record commits of the earlier rounds, where this
record said "fast path".) The CI mutation workflow and the Windows artifact workflow were not
started again: the artifact the owner tested is `c9f54ee`'s, and **no Windows artifact of the final head was built or run by hand.**

**Not done in the close:** nothing ran on Windows or macOS outside CI; no kill of the writer-thread build on Windows; no sound was
played here; no timing or benchmark of play; LeakSanitizer coverage of the genuine C runs; `DOOM2.WAD`; a review of review 6's
follow-up commits or of the survivor tests.

**Scratch.** `~/scratch/m5d` is kept until the owner has merged (reports, logs, the mutation outputs); the build outputs and worktrees of
the close's agents were deleted as each finished. `/mnt/d/DoomSessions/m5d-kills`, `m5d-kills2` and `m5d-kills3` remain the owner's to
delete.
