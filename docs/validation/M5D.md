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

To come (agent running).

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
