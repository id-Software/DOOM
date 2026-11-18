# M5c mixer start lists and goldens

These files are the evidence for M5c's mixer: start lists (what the sound logic hands to
`I_StartSound`) and the sample streams the genuine linuxdoom-1.10 in-process mixer makes from them.
The goldens come from the C only, never from Rust output and never from the independent model.

- `starts/*.txt` are the goldened scenarios, written by
  [`m5c_mixer_reference.py`](../../../tools/reference/m5c_mixer_reference.py) (its `scenarios()` holds
  each one and a comment saying what it shows). `x_*` scenarios lie outside the values `s_sound.c` can
  pass and end in the genuine `I_Error`; they are goldened because the genuine code is well defined there.
- `goldens/*.golden` hold one file per scenario, from the strict build under the recommended schedule
  (`ticbuffers`: buffer k of 512 frames is mixed after tic floor(512k/315)). The overflow and address
  builds must write the same bytes, and so must the optional 32-bit build. The independent model
  ([`m5c_mixer_model.py`](../../../tools/reference/m5c_mixer_model.py)) must write the same rows apart
  from the two device rows `d` and `q`.
- `witness/*.txt` are the three short-lump starts (lumps of 0, 5 and 8 bytes). The genuine code reads one
  byte past its 8-byte block there, and the byte depends on the memory layout. They are never goldened:
  the reference tool checks that the driver flags the start with an `o` row and the model refuses it
  (decision T11), and records them in [`m5c_mixer_results.json`](../m5c_mixer_results.json).
- `probes/*.txt` hold the pitch-256 probe, outside what `s_sound.c` can pass: the address build must
  report `i_sound.c:495`. It is not a golden.

- `sessions/` holds the mixer's goldens for the committed sound-logic streams
  ([`../m5c_sound/`](../m5c_sound/), one per item, from the genuine sound-logic runner under the
  recommended switches). `sessions/starts/<ITEM>.txt` is the stream's start list, exactly as
  [`m5c_starts.py`](../../../tools/reference/m5c_starts.py) converts it (the rows whose outcome is
  `started`; the header names the stream's SHA-256). `sessions/goldens/<ITEM>.golden` is the strict
  build's rows under `ticbuffers`, with the same row kinds as the scenario goldens. Every handle the
  genuine mixer hands out must equal the handle the sound-logic run recorded (no `m` row). Items whose
  sessions start no sound still have a golden: their buffers are silent. Per item,
  [`m5c_mixer_sessions_results.json`](../m5c_mixer_sessions_results.json) records the SHA-256 of the
  stream, the numbers of starts, tics and buffers, and the digests. If a stream is regenerated and
  differs, `--check` reports that item.

The schedule `pertic` (one buffer per tic) is recorded in the results files as digests only.

The driver, its contract and the design are described in
[`m5c_mixer_driver.c`](../../../tools/reference/m5c_mixer_driver.c),
[`m5c_mixer_contract.json`](../m5c_mixer_contract.json) and
[section 3.2 of the M5c investigation](../../../docs/M5C_REFERENCE_INVESTIGATION.md#32-harness-the-genuine-mixer-headless).

## Start list format (DOOM-M5C-STARTS-v1)

One directive per line; lines starting with `#` are comments.

| Directive | Meaning |
| --- | --- |
| `schema DOOM-M5C-STARTS-v1` | First directive. |
| `config integrated` | The handle convention of the run that made the list. The in-process mixer exists only in the integrated configuration, so no other value is accepted. |
| `tics N` | Run at least tics 0 to N-1. |
| `s TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO HANDLE` | One `I_StartSound` call: the session tic, the call order (TIC SEQ strictly increases), `gametic` at the call, the five arguments exactly as `s_sound.c` passes them, and the handle the sound-logic run received (or `-`). |
| `n COUNT TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO` | Harness only: COUNT identical starts (the handle wrap). |
| `g TIC SEQ SFX` | Harness only: a call of the genuine `I_GetSfxLumpNum`. |

## Golden rows (DOOM-M5C-MIXER-v1)

| Row | Meaning |
| --- | --- |
| `h` | Header: schema, schedule, frames per buffer. |
| `d`, `q` | The fake device after start-up and at the end (device glue; not written by the model). |
| `T` | SHA-256 of `steptable` and `vol_lookup` as the genuine code builds them. |
| `L`, `l`, `P` | The padded lumps: one digest of all `l` rows; with `--lumps`, every sfx's padded length and data digest, and the lumps that were padded or fell back to `dspistol`. |
| `s` | One start and what the genuine code did: handle, mixer channel, step, left and right volume rows, occupancy before the call, the sfx the channel held before (or `-`), other channels the call freed (or `-`). |
| `sn` | An `n` row: first and last handle, digests of all handles and channels. |
| `m` | The start list's handle differs from the genuine handle (never in a golden). |
| `o` | A start with no sample byte at its start (padded length 0): the layout-dependent read. |
| `g` | The lump number `I_GetSfxLumpNum` returned. |
| `b` | One buffer: index, the tic after which it was mixed, SHA-256 of its 2,048 bytes, occupancy after it. |
| `r` | The previous `b` row repeats N more times (same digest and occupancy). |
| `x` | `I_Error` (the run stops), or the buffer limit. |
| `z` | Number of buffers and SHA-256 of the whole stream. |

No raw samples are committed. `m5c_mixer_reference.py --raw` writes them to its scratch directory.
