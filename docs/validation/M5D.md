# M5d evidence record

**Status (2026-11-20 00:30 PST):** in progress on branch `feat/m5d-play-fixes`. Done: the DPI request on
Windows (A1, untested on Windows), the mouse forwarding moved into its own file and the input log's review points
M1 to M3 closed, the CI job renamed (A4). In progress: the sound statistics and trial options (A2), the session
file that cannot be lost (A3). To come: the gates (A5) and the owner's Windows session (A6). Nothing in this
milestone has been run on Windows or macOS.

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
| `screen_size` becomes the physical client size, computed as `trunc(trunc(client / scale) × scale)`, so it can be one pixel short of the real client size | `update_dimensions`, 1050-1090 |
| Raw mouse motion is multiplied by the mouse scale, which is 1 with the flag and also 1 without it (without the flag the window scale stays 1) | `WM_INPUT`, 526-556; `init_dpi` |

**What that means for the player on Windows (inference from the source):**

- **Window size.** At 100% nothing changes. At 125% the window is 960×720 physical pixels, about 768×576 in
  Windows' scaled units; before, Windows scaled the 960×720 window up to 1,200×900 physical pixels (the owner
  saw "about 1200 by 900"). At 150% it is about 640×480 scaled units where it was 1,440×1,080. The window
  stays resizable, and `--fullscreen` covers the screen either way (`GetSystemMetrics` returns physical
  pixels to a DPI-aware process). Asking for a larger window after start (`window::set_window_size`) was not
  done: it would add a `resize` line to every log, which the acceptance test reads as a fault.
- **Picture fit.** The picture is fitted to `screen_size` at every draw (`fit` in `window.rs`). When the
  truncation above makes `screen_size` one pixel smaller than the real client area, the picture can be one
  pixel narrower or shorter than it could be, with a one-pixel black line at an edge. Cosmetic only.
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
CI jobs compile it on the exact head (GitHub's record), and the test `dpi_awareness_is_asked_for_on_windows_only`
pins the constant on each operating system. A build proves only that it compiles and that the constant is
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
the flag against the observed cause, case 0, and would show 1 only with a second monitor at another scale.

### The options if it still happens (none taken; the owner's ruling)

| Option | What it is | Cost |
| --- | --- | --- |
| Library upgrade | A newer miniquad with a mouse capture | **None exists.** 0.4.11 is the newest release on crates.io (`cargo info miniquad`, 2026-11-20), and a code search of the upstream repository finds no `SetCapture` (2026-11-20). A git revision would be an unreleased, unreviewed dependency, and it would still lack the capture. |
| A patch to the library | A copy of miniquad 0.4.11 in the repository (or a fork), used through `[patch.crates-io]`, with about ten lines in `src/native/windows.rs`: `SetCapture(hwnd)` on a button-down and `ReleaseCapture()` when no button is down, so every button-up comes to the window wherever the cursor is; or reading the button-up flags of the raw input the library already receives (`WM_INPUT`, 526-556, ignores them) | The unsafe calls live inside the library, which is all FFI already, so the workspace's `unsafe_code = "forbid"` lint (it applies to our crates) is untouched. But it is code we carry: about 0.9 MB of vendored source or a fork to keep in step, its own review, and the Windows CI jobs building it. Licence: miniquad is MIT or Apache-2.0; the MIT choice is compatible with GPL-2.0-only (Apache-2.0 alone is not). |
| A reviewed exception to the no-unsafe rule | `doom-play` calls `SetCapture` and `ReleaseCapture` itself from its button handlers through a Windows API crate (miniquad exposes no window handle, so the handle would come from `GetActiveWindow` on the window's own thread) | A new dependency (`windows-sys` or similar), a separately reviewed policy and safety proof, as AGENTS.md requires, and a small amount of Windows-only code that no Linux test can run. |

## A2. Sound latency on Windows: statistics and trial options

Commits `5421b5c`, `8d40286`, `b6d6196` on the agent's worktree, cherry-picked as the three commits ending at
`fd442ee`. Everything is in `crates/doom-play`; the declared mixing schedule, the mixer and every golden are
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
the callbacks line) led to a stronger test in `b6d6196`, and one is equivalent (`resume` clearing a flag `pause`
already cleared). Not run: anything on Windows or macOS, a windowed run with a device.

**Open for the owner:** keep miniaudio's INFO report at every launch (about 18 lines on PulseAudio here) or put it
behind an option, and limit it to the open (remove the INFO callback after start) in case DirectSound stalls repeat;
whether to pursue the WASAPI period count by any of the three routes above.

## A3. A session that cannot be lost

To come (agent running).

## A4. Leftovers by the owner's earlier rulings

Commit `30e535b`:

- **The mouse forwarding** moved unchanged from `crates/doom-play/src/input_log.rs` into
  `crates/doom-play/src/mouse_forward.rs` (`Forward`; the log's button names became `pub(crate)` so the new
  module can use them). The window and the log's test use it from there; the test module of `input_log.rs` is
  textually unchanged (it reaches `Forward` through a test-only import in its parent module).
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

To come.

## A6. The owner's Windows session

To come.
