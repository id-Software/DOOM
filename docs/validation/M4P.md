# M4p evidence record

Observed results of the M4p milestone (simulation performance with no behaviour change). Only observations
appear here. The [M4p investigation](../M4P_REFERENCE_INVESTIGATION.md) owns the profile, the decisions P1 to P9
and the plan; [testing](../TESTING.md#advance-benchmarks-m4p) owns the commands. Exact review state and CI belong
to GitHub.

**Scope of the record.** Phase 2 step 1 (2026-11-14, America/Los_Angeles): the committed benchmark harness and
the master baseline, in [Baseline (master)](#baseline-master). Step 2 (O1), in [O1 (owner callback)](#o1-owner-callback).
Step 3 (O3, O4) and the first group check, in [O3 (pickup revalidation)](#o3-pickup-revalidation),
[O4 (owner-loop monster_actor)](#o4-owner-loop-monster_actor) and [Group check 1](#group-check-1). Steps 4 to 6
(O2, O5, O7) and the second group check, in [O2 (sound-target references)](#o2-sound-target-references),
[O5 (event buffer)](#o5-event-buffer), [O7 (linked bitmap)](#o7-linked-bitmap) and [Group check 2](#group-check-2).
Steps 7 and 8 (O6; O8, dropped) and the third group check, in [O6 (single-pass for_things)](#o6-single-pass-for_things),
[O8 (location memo): dropped](#o8-location-memo-dropped) and [Group check 3](#group-check-3). Later steps append their
own sections.

## Baseline (master)

Master `4021d60` behaviour (this branch differs from it only by documentation, the harness and the baseline file),
recorded with the committed harness `tools/bench_advance.py` on 2026-11-14 (America/Los_Angeles).

**Host and build.** Intel Core i7-8750H @ 2.20 GHz (6 cores, 12 logical), 7.7 GiB, Linux 6.18.58.1 WSL2, valgrind
3.26.0. Examples built with the pinned `cargo +1.85.0 build -p doom-sim --examples --release --locked` (rustc
1.85.0, 4d91de4e4 2025-02-17) into `~/scratch/m4p/target-185` (ext4). The binary, the scratch IWAD copy
(`~/scratch/m4p/iwad/DOOM.WAD`) and the output directory were all on ext4 (the harness checks `/proc/mounts`
and found no path under `/mnt/*`); nothing was built or run from `/mnt/*`. `/mnt/c` free: 71 GiB after the
builds and at the end (at least 20 GiB throughout).

**Timing method.** `python3 -I tools/bench_advance.py timing --reps 5` (see
[TESTING](../TESTING.md#advance-benchmarks-m4p)): `player_level_advance --policy episode1v2` over the 61
real-map cases of `tests/reference/m4b_real_results.json`, five repetitions in order rotated by 12 per
repetition, one run at a time. All 305 runs ended in the recorded final stripped SHA-256 and the recorded
admitted-tic count (0 mismatches; 158,966 admitted tics per repetition). Columns: median of the per-script
medians of advance-only microseconds per admitted tic; the worst single run; whole-process wall (parse, setup,
advance, final digest) as a secondary figure.

**Idle record.** Before the timing run no cargo, rustc, nextest, Valgrind, fuzz or C process was running (the
only listed CPU users were this session's own tool host and an idle editor `rust-analyzer` daemon at 0.2% CPU,
which the harness does not treat as stray). Gate readings at the start of each repetition: all-CPU busy fraction
0.10% to 0.20% over a 5 s window, no stray process, 1-minute load 0.52 (rep 0) and 0.92 to 1.01 afterwards
(the single-threaded timing process itself contributes about 1.0, so the load limit of the gate is 1.5 and the
busy fraction is the real test). Runs 12:33 to 12:39 PST. Windows-side load is invisible from WSL.

**Advance-only timing per map (wire 24, `episode1v2`).**

| Map | Cases | Admitted tics | Advance-only median us/tic (median of script medians) | Worst run us/tic | Whole-process wall median us/tic (secondary) |
|---|--:|--:|--:|--:|--:|
| E1M1 | 5 | 10,029 | 63.5 | 71.0 | 73.2 |
| E1M2 | 5 | 9,094 | 145.1 | 162.9 | 157.7 |
| E1M3 | 5 | 13,750 | 351.7 | 360.5 | 361.0 |
| E1M4 | 5 | 9,386 | 256.7 | 300.6 | 286.5 |
| E1M5 | 10 | 36,783 | 484.5 | 769.5 | 502.3 |
| E1M6 | 5 | 12,742 | 830.1 | 892.1 | 851.1 |
| E1M7 | 5 | 12,939 | 361.8 | 411.6 | 377.0 |
| E1M8 | 10 | 30,840 | 344.1 | 488.0 | 357.5 |
| E1M9 | 10 | 21,693 | 667.6 | 1056.1 | 684.2 |
| DEMO1 | 1 | 1,710 | 703.2 | 733.0 | 713.5 |

These figures are 14% (E1M2) to 34% (E1M8) above the Phase 1 table of the
[investigation](../M4P_REFERENCE_INVESTIGATION.md) (for example E1M1 53.0, E1M6 633.2 us/tic), which was timed
with a stable rustc 1.100.0 build; this table uses the pinned 1.85.0 build, the toolchain the CI guard pins.
Compare later timings only with a table built by the same toolchain; the cause of the difference was not
isolated (no rustc 1.100.0 re-run was made).

**CI-set instruction counts (`ir`).** `python3 -I tools/bench_advance.py ir --update-baseline` with the same
1.85.0 build; the numbers are the committed `tests/reference/m4p_ir_baseline.json`. Advance Ir is counted
inside `Simulation::advance` (Callgrind `--toggle-collect`) outside libc, ld.so, libm, libgcc_s and unknown
objects; the call counts are calls to `malloc`, `realloc` and `memcpy` variants from callers outside those
objects.

| Workload | Advance Ir excl. libc | malloc | realloc | memcpy | Advance Ir total (information) |
|---|--:|--:|--:|--:|--:|
| `SYNCHAOS_wire22` | 2,417,221,845 | 537,034 | 40,302 | 3,726,424 | 2,817,871,837 |
| `SCALE2_episode1v2` | 607,264,606 | 121,778 | 32,962 | 787,156 | 685,446,699 |
| `MTELE1_wire24` | 9,657,230 | 3,883 | 1,141 | 18,813 | 10,954,763 |
| `TFRAG1_wire24` | 12,167,172 | 5,560 | 1,646 | 25,971 | 14,285,193 |
| `BVB1_wire24` | 19,876,640 | 9,716 | 3,168 | 45,342 | 24,787,687 |
| `M4a_dense64_lift` | 2,458,894,726 | 634,539 | 340,572 | 2,525,763 | 2,829,702,660 |
| `M4a_dense64_fight` | 1,894,182,959 | 779,089 | 177,944 | 2,973,051 | 2,224,314,539 |
| `M4a_gallery_patrol` | 254,615,183 | 357,331 | 95,649 | 422,489 | 293,362,995 |
| `M4a_corpse_continuation` | 5,462,483 | 4,933 | 1,123 | 13,858 | 6,824,249 |

**Run-to-run noise of the CI metrics.** Two further `ir --check --jobs 1` runs on the same machine and build
reproduced every one of the 36 metric values exactly (0.00% on all workloads); the digests and outcomes of all
nine workloads equal the baseline's. Runner-to-runner stability on hosted CI is not yet measured.

## O1 (owner callback)

Code commit `3ca7623`, recorded on 2026-11-14 (America/Los_Angeles) with the same host, toolchain (rustc 1.85.0),
valgrind and ext4 paths as the baseline; `/mnt/c` had 63 GiB free at the end.

**CI-set instruction counts.** `python3 -I tools/bench_advance.py ir --check`: all nine outcomes (admitted tics,
refusal tic, final stripped SHA-256) equal the baseline's; `threshold 3.0%: ok`. The baseline was not lowered.
malloc and realloc call counts are unchanged on all nine workloads.

| Workload | Advance Ir excl. libc, baseline | After O1 | Change | memcpy calls, baseline | After O1 |
|---|--:|--:|--:|--:|--:|
| `SYNCHAOS_wire22` | 2,417,221,845 | 2,417,221,845 | 0.00% | 3,726,424 | 3,726,424 |
| `SCALE2_episode1v2` | 607,264,606 | 607,264,606 | 0.00% | 787,156 | 787,156 |
| `MTELE1_wire24` | 9,657,230 | 6,338,588 | −34.36% | 18,813 | 13,767 |
| `TFRAG1_wire24` | 12,167,172 | 10,822,358 | −11.05% | 25,971 | 23,671 |
| `BVB1_wire24` | 19,876,640 | 19,876,640 | 0.00% | 45,342 | 45,342 |
| `M4a_dense64_lift` | 2,458,894,726 | 2,458,894,726 | 0.00% | 2,525,763 | 2,525,763 |
| `M4a_dense64_fight` | 1,894,182,959 | 1,894,182,959 | 0.00% | 2,973,051 | 2,973,051 |
| `M4a_gallery_patrol` | 254,615,183 | 254,615,183 | 0.00% | 422,489 | 422,489 |
| `M4a_corpse_continuation` | 5,462,483 | 4,497,073 | −17.67% | 13,858 | 12,668 |

Only the three workloads with a live Demon change, as expected: O1 runs only for an owner-type actor.

**Indicative timing (one repetition, not performance evidence of the milestone).**
`python3 -I tools/bench_advance.py timing --reps 1` on the 61 real-map cases, ending 13:21 PST, with no local build, test,
fuzz or Valgrind process running (the CI run on GitHub is remote). All 61 runs ended in the recorded final digest and
admitted-tic count (0 mismatches). One repetition only, so these medians are single-run values.

| Map | Advance-only us/tic, baseline (5 reps) | After O1 (1 rep) |
|---|--:|--:|
| E1M1 | 63.5 | 64.0 |
| E1M2 | 145.1 | 147.2 |
| E1M3 | 351.7 | 246.5 |
| E1M4 | 256.7 | 150.5 |
| E1M5 | 484.5 | 206.0 |
| E1M6 | 830.1 | 299.7 |
| E1M7 | 361.8 | 228.0 |
| E1M8 | 344.1 | 100.5 |
| E1M9 | 667.6 | 230.9 |
| DEMO1 | 703.2 | 252.7 |

**Gates run for O1.** Build, both format checks, Clippy with warnings denied, `cargo test --workspace` (1,383
passed, 4 ignored), the release tests (1,350 passed, 2 ignored), `cargo +1.85.0 test --workspace` (1,383 passed,
4 ignored), warnings-denied rustdoc; `run_reference.py --suite --jobs 2` (65 commands passed); the 95 Python
commands of the CI `stable` job other than the three genuine-engine runs (`m3h9c_reference`, `m4a_reference`,
`m4b_reference` twice) and the four projections that read their dumps, all passed (the 22 headless debug/release
checks ran with `python3`, as `python` does not exist on this host), including `check_player_level.py`,
`check_player_episode1.py --require` and `check_player_episode1v2.py --require`; and
`check_player_level_advance.py`. Not run for O1: the genuine-engine runs and their projections, fuzz builds and the
sanitizer campaign (due at step 11, or after step 3 for the genuine-engine checks).

## O3 (pickup revalidation)

Code commit `69c1a3a`, recorded on 2026-11-14 (America/Los_Angeles) with the same host, toolchain (rustc 1.85.0),
valgrind and ext4 paths as the baseline; `/mnt/c` had 61 GiB free after the gates.

**CI-set instruction counts.** `python3 -I tools/bench_advance.py ir --check`: all nine outcomes (admitted tics,
refusal tic, final stripped SHA-256) equal the baseline's; `threshold 3.0%: ok`. The baseline was not lowered.
malloc, realloc and memcpy call counts equal the O1 counts on all nine workloads.

| Workload | Advance Ir excl. libc, after O1 | After O3 | Change vs O1 | Change vs baseline |
|---|--:|--:|--:|--:|
| `SYNCHAOS_wire22` | 2,417,221,845 | 2,237,634,810 | −7.43% | −7.43% |
| `SCALE2_episode1v2` | 607,264,606 | 572,762,739 | −5.68% | −5.68% |
| `MTELE1_wire24` | 6,338,588 | 6,336,384 | −0.03% | −34.39% |
| `TFRAG1_wire24` | 10,822,358 | 10,808,097 | −0.13% | −11.17% |
| `BVB1_wire24` | 19,876,640 | 19,846,714 | −0.15% | −0.15% |
| `M4a_dense64_lift` | 2,458,894,726 | 2,355,200,198 | −4.22% | −4.22% |
| `M4a_dense64_fight` | 1,894,182,959 | 1,738,739,771 | −8.21% | −8.21% |
| `M4a_gallery_patrol` | 254,615,183 | 254,208,344 | −0.16% | −0.16% |
| `M4a_corpse_continuation` | 4,497,073 | 4,495,473 | −0.04% | −17.70% |

**Gates run for O3.** Build, both format checks, Clippy with warnings denied, `cargo test --workspace` (1,383
passed, 4 ignored), the release tests (1,350 passed, 2 ignored), `cargo +1.85.0 test --workspace` (1,383 passed,
4 ignored), warnings-denied rustdoc; `run_reference.py --suite --jobs 2` (65 commands passed); the same 95 Python
commands of the CI `stable` job as for O1 (all passed, run with `python3`), including `check_player_level.py`,
`check_player_episode1.py --require`, `check_player_episode1v2.py --require` and the mechanism goldens; and
`check_player_level_advance.py` (36 final states and boundaries passed).

## O4 (owner-loop monster_actor)

Code commit `c439d91`, recorded on 2026-11-14 (America/Los_Angeles), same host, toolchain, valgrind and ext4 paths;
`/mnt/c` had 60 GiB free after the gates.

**CI-set instruction counts.** `python3 -I tools/bench_advance.py ir --check`: all nine outcomes equal the
baseline's; `threshold 3.0%: ok`. The baseline was not lowered. The cumulative change (O1, O3 and O4) against the
baseline is in the last column.

| Workload | After O3 | After O4 | Change vs O3 | Change vs O1 | Change vs baseline | malloc calls, baseline | After O4 | memcpy calls, after O3 | After O4 |
|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| `SYNCHAOS_wire22` | 2,237,634,810 | 1,906,381,824 | −14.80% | −21.13% | −21.13% | 537,034 | 402,007 | 3,726,424 | 3,256,284 |
| `SCALE2_episode1v2` | 572,762,739 | 503,782,893 | −12.04% | −17.04% | −17.04% | 121,778 | 97,738 | 787,156 | 689,795 |
| `MTELE1_wire24` | 6,336,384 | 5,383,366 | −15.04% | −15.07% | −44.26% | 3,883 | 3,883 | 13,767 | 12,337 |
| `TFRAG1_wire24` | 10,808,097 | 9,278,761 | −14.15% | −14.26% | −23.74% | 5,560 | 5,560 | 23,671 | 21,128 |
| `BVB1_wire24` | 19,846,714 | 18,187,449 | −8.36% | −8.50% | −8.50% | 9,716 | 9,716 | 45,342 | 42,459 |
| `M4a_dense64_lift` | 2,355,200,198 | 2,154,296,070 | −8.53% | −12.39% | −12.39% | 634,539 | 557,707 | 2,525,763 | 2,235,242 |
| `M4a_dense64_fight` | 1,738,739,771 | 1,474,105,514 | −15.22% | −22.18% | −22.18% | 779,089 | 681,862 | 2,973,051 | 2,595,639 |
| `M4a_gallery_patrol` | 254,208,344 | 218,704,391 | −13.97% | −14.10% | −14.10% | 357,331 | 321,321 | 422,489 | 376,271 |
| `M4a_corpse_continuation` | 4,495,473 | 3,922,512 | −12.75% | −12.78% | −28.19% | 4,933 | 4,933 | 12,668 | 11,468 |

realloc calls are unchanged except `SCALE2_episode1v2` (32,962 to 28,154). O4 is larger than the §3 projection
on the workloads without a Demon: under `uses_demon_map` the skipped `monster_actor` ran for every actor, not only
for owners, and its type and state-role validation allocates.

**Gates run for O4.** The same list as for O3, all passed with the same counts: build, both format checks, Clippy,
`cargo test --workspace` (1,383 passed, 4 ignored), release tests (1,350 passed, 2 ignored), `cargo +1.85.0 test
--workspace` (1,383 passed, 4 ignored), rustdoc; `run_reference.py --suite --jobs 2` (65 passed); the 95 CI
`stable` Python commands; `check_player_level_advance.py` (36 passed).

## Group check 1

After O1, O3 and O4 (head `c439d91`), on 2026-11-14 between 15:14 and 15:20 PST (America/Los_Angeles).

**Real-map digests and indicative timing (one repetition, not performance evidence of the milestone).**
`python3 -I tools/bench_advance.py timing --reps 1` with the rustc 1.85.0 release build in
`~/scratch/m4p/target-185` and the scratch IWAD copy (both ext4), with no local build, test, fuzz or Valgrind
process running; the 1-minute load average was 2.65 at the start. All 61 runs ended in the recorded final stripped
SHA-256 and admitted-tic count (0 mismatches; 158,966 admitted tics). Single-run medians:

| Map | Advance-only us/tic, baseline (5 reps) | After O1 (1 rep) | After O4 (1 rep) |
|---|--:|--:|--:|
| E1M1 | 63.5 | 64.0 | 55.2 |
| E1M2 | 145.1 | 147.2 | 127.0 |
| E1M3 | 351.7 | 246.5 | 213.3 |
| E1M4 | 256.7 | 150.5 | 140.7 |
| E1M5 | 484.5 | 206.0 | 185.6 |
| E1M6 | 830.1 | 299.7 | 260.0 |
| E1M7 | 361.8 | 228.0 | 205.3 |
| E1M8 | 344.1 | 100.5 | 91.3 |
| E1M9 | 667.6 | 230.9 | 216.2 |
| DEMO1 | 703.2 | 252.7 | 230.9 |

**Synthetic genuine-engine checks.** The CI `stable` job's commands, run one at a time (one C job at a time), with
dumps under `~/scratch/m4p/ci-g1` (ext4); the address variants run with `ASAN_OPTIONS=detect_leaks=0` set by the
runners (leak detection disabled). All passed:

- `m3h9c_reference.py --check --keep-dumps`: 6 scripts, strict, overflow-checked and address-checked dumps
  identical for all 6, results file matched; `m3h9c_snapshot_goldens.py --dumps … --check` (11 files);
  `check_player_level.py` on the six Rust runs (all checks passed).
- `m4a_reference.py --check --keep-dumps --selftest 300`: 30 scripts, variants identical for all 30, results file
  matched, selftest passed; `m4a_snapshot_goldens.py --check` (19 files); `check_player_episode1.py --example-args
  "--policy episode1" --require` (all checks passed); `m4b_m4a_goldens.py --check` (30 scripts, 15,116 snapshots
  projected).
- `m4b_reference.py --check --keep-dumps --selftest 30`: 18 scripts, variants identical for all 18, the `SPECH9`
  overrun probe reported `p_map.c:242` after tic 74 as required, results file matched, selftest passed;
  `m4b_snapshot_goldens.py --check --selftest` (20 files, 8 selftest checks).
- `m4b_reference.py --set 2 --check --keep-dumps`: 4 scripts, variants identical, results file matched;
  `m4b_snapshot_goldens.py --set 2 --check` (5 files).
- `check_player_episode1v2.py --require`: 52 of 52 scripts matched (20,205 tics), 0 pending.

No digest or byte differed, so no optimisation of the group was reverted. Not run for the group: the opt-in
`strict32` variants, fuzz builds and the sanitizer campaign (due at step 11).

## O2 (sound-target references)

Code commit `e8586a1`, recorded on 2026-11-14 (America/Los_Angeles), same host, toolchain (rustc 1.85.0), valgrind
and ext4 paths; `/mnt/c` had 56 GiB free after the gates.

**CI-set instruction counts.** `python3 -I tools/bench_advance.py ir --check`: all nine outcomes equal the
baseline's; `threshold 3.0%: ok`. The baseline was not lowered.

| Workload | After O4 | After O2 | Change vs O4 | Change vs baseline | malloc calls | realloc calls | memcpy calls |
|---|--:|--:|--:|--:|--:|--:|--:|
| `SYNCHAOS_wire22` | 1,906,381,824 | 1,849,252,635 | −3.00% | −23.50% | 402,007 | 40,302 | 3,201,449 |
| `SCALE2_episode1v2` | 503,782,893 | 500,296,515 | −0.69% | −17.61% | 97,738 | 28,154 | 687,591 |
| `MTELE1_wire24` | 5,383,366 | 5,344,416 | −0.72% | −44.66% | 3,883 | 1,141 | 12,337 |
| `TFRAG1_wire24` | 9,278,761 | 9,241,798 | −0.40% | −24.04% | 5,560 | 1,646 | 21,128 |
| `BVB1_wire24` | 18,187,449 | 18,152,761 | −0.19% | −8.67% | 9,716 | 3,168 | 42,459 |
| `M4a_dense64_lift` | 2,154,296,070 | 2,149,892,070 | −0.20% | −12.57% | 557,707 | 340,572 | 2,235,242 |
| `M4a_dense64_fight` | 1,474,105,514 | 1,468,459,943 | −0.38% | −22.48% | 681,862 | 177,944 | 2,595,639 |
| `M4a_gallery_patrol` | 218,704,391 | 208,446,443 | −4.69% | −18.13% | 321,321 | 95,649 | 365,732 |
| `M4a_corpse_continuation` | 3,922,512 | 3,889,973 | −0.83% | −28.79% | 4,933 | 1,123 | 11,468 |

A first form of O2 (an eight-entry seen-set of accepted ids, also inside the owner walk) passed the same gates but
measured above O4 on five workloads (`TFRAG1_wire24` +0.06%, `BVB1_wire24` +0.13%, `M4a_dense64_lift` +0.02%,
`M4a_dense64_fight` +0.03%, `M4a_corpse_continuation` +0.32%), so it was not committed; the committed form is
described in the [investigation](../M4P_REFERENCE_INVESTIGATION.md) under O2.

**Gates run for O2 (committed form).** Build, both format checks, Clippy with warnings denied, `cargo test --workspace` (1,388 passed, 4 ignored; the five
more than at O4 are the P8 tests of commit `1239d4c`), the release tests (1,355 passed, 2 ignored), `cargo +1.85.0
test --workspace` (1,388 passed, 4 ignored), warnings-denied rustdoc; `run_reference.py --suite --jobs 2` (65
passed); the 95 CI `stable` Python commands (95 passed, 0 failed, run with `python3`), including
`check_player_level.py`, `check_player_episode1.py --require`, `check_player_episode1v2.py --require`, the mechanism
goldens and `check_player_level_advance.py`.

## O5 (event buffer)

Code commit `320053c`, recorded on 2026-11-14 (America/Los_Angeles), same host, toolchain, valgrind and ext4 paths;
`/mnt/c` had 52 GiB free after the gates.

**CI-set instruction counts.** `python3 -I tools/bench_advance.py ir --check`: all nine outcomes equal the
baseline's; `threshold 3.0%: ok`. The baseline was not lowered.

| Workload | After O2 | After O5 | Change vs O2 | Change vs O4 | Change vs baseline | malloc calls | realloc calls | memcpy calls |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| `SYNCHAOS_wire22` | 1,849,252,635 | 1,846,585,623 | −0.14% | −3.14% | −23.61% | 392,007 | 9,350 | 3,196,448 |
| `SCALE2_episode1v2` | 500,296,515 | 500,101,668 | −0.04% | −0.73% | −17.65% | 96,538 | 23,941 | 686,990 |
| `MTELE1_wire24` | 5,344,416 | 5,244,565 | −1.87% | −2.58% | −45.69% | 3,563 | 502 | 12,176 |
| `TFRAG1_wire24` | 9,241,798 | 9,082,882 | −1.72% | −2.11% | −25.35% | 4,996 | 712 | 20,845 |
| `BVB1_wire24` | 18,152,761 | 17,718,281 | −2.39% | −2.58% | −10.86% | 8,246 | 590 | 41,723 |
| `M4a_dense64_lift` | 2,149,892,070 | 2,148,563,820 | −0.06% | −0.27% | −12.62% | 552,909 | 323,313 | 2,232,842 |
| `M4a_dense64_fight` | 1,468,459,943 | 1,466,584,150 | −0.13% | −0.51% | −22.57% | 674,666 | 154,670 | 2,592,040 |
| `M4a_gallery_patrol` | 208,446,443 | 206,267,101 | −1.05% | −5.69% | −18.99% | 314,125 | 81,327 | 362,133 |
| `M4a_corpse_continuation` | 3,889,973 | 3,696,843 | −4.96% | −5.75% | −32.32% | 4,135 | 242 | 11,069 |

**Gates run for O5.** Build, both format checks, Clippy with warnings denied, `cargo test --workspace` (1,388 passed, 4 ignored; the five
more than at O4 are the P8 tests of commit `1239d4c`), the release tests (1,355 passed, 2 ignored), `cargo +1.85.0
test --workspace` (1,388 passed, 4 ignored), warnings-denied rustdoc; `run_reference.py --suite --jobs 2` (65
passed); the 95 CI `stable` Python commands (95 passed, 0 failed, run with `python3`), including
`check_player_level.py`, `check_player_episode1.py --require`, `check_player_episode1v2.py --require`, the mechanism
goldens and `check_player_level_advance.py`.

## O7 (linked bitmap)

Code commit `46f4b4e`, recorded on 2026-11-14 (America/Los_Angeles), same host, toolchain, valgrind and ext4 paths;
`/mnt/c` had 56 GiB free after the gates. From this step on the machine was in use by its owner, and every build and
test ran with `CARGO_BUILD_JOBS=6`, `NEXTEST_TEST_THREADS=4` and `RUST_TEST_THREADS=4`, with at most two C jobs.
Instruction counts do not depend on load.

**CI-set instruction counts.** `python3 -I tools/bench_advance.py ir --check`: all nine outcomes equal the
baseline's; `threshold 3.0%: ok`. The baseline was not lowered.

| Workload | After O5 | After O7 | Change vs O5 | Change vs O4 | Change vs baseline | malloc calls | realloc calls | memcpy calls |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| `SYNCHAOS_wire22` | 1,846,585,623 | 1,673,950,274 | −9.35% | −12.19% | −30.75% | 311,904 | 9,350 | 2,564,614 |
| `SCALE2_episode1v2` | 500,101,668 | 456,387,352 | −8.74% | −9.41% | −24.85% | 80,206 | 23,941 | 554,672 |
| `MTELE1_wire24` | 5,244,565 | 5,028,005 | −4.13% | −6.60% | −47.94% | 3,402 | 502 | 11,460 |
| `TFRAG1_wire24` | 9,082,882 | 8,569,796 | −5.65% | −7.64% | −29.57% | 4,713 | 712 | 18,902 |
| `BVB1_wire24` | 17,718,281 | 17,300,504 | −2.36% | −4.88% | −12.96% | 7,510 | 590 | 40,431 |
| `M4a_dense64_lift` | 2,148,563,820 | 2,037,221,536 | −5.18% | −5.43% | −17.15% | 504,889 | 323,313 | 1,829,474 |
| `M4a_dense64_fight` | 1,466,584,150 | 1,326,920,948 | −9.52% | −9.98% | −29.95% | 610,387 | 154,670 | 2,072,198 |
| `M4a_gallery_patrol` | 206,267,101 | 195,316,887 | −5.31% | −10.69% | −23.29% | 303,322 | 81,327 | 318,741 |
| `M4a_corpse_continuation` | 3,696,843 | 3,621,350 | −2.04% | −7.68% | −33.71% | 3,735 | 242 | 10,787 |

**Gates run for O7.** Build, both format checks, Clippy with warnings denied, `cargo test --workspace` (1,388 passed, 4 ignored; the five
more than at O4 are the P8 tests of commit `1239d4c`), the release tests (1,355 passed, 2 ignored), `cargo +1.85.0
test --workspace` (1,388 passed, 4 ignored), warnings-denied rustdoc; `run_reference.py --suite --jobs 2` (65
passed); the 95 CI `stable` Python commands (95 passed, 0 failed, run with `python3`), including
`check_player_level.py`, `check_player_episode1.py --require`, `check_player_episode1v2.py --require`, the mechanism
goldens and `check_player_level_advance.py`.

## Group check 2

After O2, O5 and O7 (head `46f4b4e`), on 2026-11-14 (America/Los_Angeles), with the same caps as O7.

**Real-map digests (no timing).** `python3 -I tools/bench_advance.py timing --reps 1` with the rustc 1.85.0 release
build in `~/scratch/m4p/target-185` and the scratch IWAD copy (both ext4), run only as a digest check: the machine
was in use, so the idle gate was relaxed (`--max-load 1000 --max-busy 1.0`) and **no timing figure from this run is
recorded** (the tool's "performance evidence" flag reflects only the relaxed limits). All 61 runs ended in the
recorded final stripped SHA-256 and admitted-tic count (0 mismatches; 158,966 admitted tics).

**Synthetic genuine-engine checks.** The same commands as group check 1, run one at a time (one C job at a time),
with dumps under `~/scratch/m4p/ci-g2` (ext4); the address variants run with `ASAN_OPTIONS=detect_leaks=0` set by the
runners (leak detection disabled). All passed:

- `m3h9c_reference.py --check --keep-dumps`: 6 scripts, variants identical for all 6, results file matched;
  `m3h9c_snapshot_goldens.py --dumps … --check` (11 files); `check_player_level.py` on the six Rust runs (all checks
  passed).
- `m4a_reference.py --check --keep-dumps --selftest 300`: 30 scripts, variants identical for all 30, results file
  matched, selftest passed; `m4a_snapshot_goldens.py --check` (19 files); `check_player_episode1.py --example-args
  "--policy episode1" --require` (all checks passed); `m4b_m4a_goldens.py --check` (30 scripts, 15,116 snapshots
  projected).
- `m4b_reference.py --check --keep-dumps --selftest 30`: 18 scripts, variants identical for all 18, the `SPECH9`
  overrun probe reported `p_map.c:242` after tic 74 as required, results file matched, selftest passed;
  `m4b_snapshot_goldens.py --check --selftest` (20 files, 8 selftest checks).
- `m4b_reference.py --set 2 --check --keep-dumps`: 4 scripts, variants identical, results file matched;
  `m4b_snapshot_goldens.py --set 2 --check` (5 files).
- `check_player_episode1v2.py --require`: 52 of 52 scripts matched (20,205 tics), 0 pending.

No digest or byte differed, so no optimisation of the group was reverted. Not run for the group: the opt-in
`strict32` variants, fuzz builds and the sanitizer campaign (due at step 11).

## O6 (single-pass for_things)

Code commit `fda9230`, its differential test in `3bfaf45`, recorded on 2026-11-14 (America/Los_Angeles), same host,
toolchain, valgrind and ext4 paths; `/mnt/c` had 51 GiB free after the gates. From this step on the owner had
lifted the caps (2026-11-14 evening PST) and the machine was idle: builds and tests ran uncapped, with two C jobs for
the reference suite. The caps returned at about 23:00 PST, after these gates. Instruction counts do not depend on load.

**CI-set instruction counts.** `python3 -I tools/bench_advance.py ir --check`: all nine outcomes equal the
baseline's; `threshold 3.0%: ok`. The baseline was not lowered. The thread-local scratch adds one `malloc` per
process and at most one `realloc` per growth of the spatial order. A first form with a local `Vec` per call lowered
Ir too, but raised `M4a_dense64_lift`'s `realloc` calls from 340,572 (baseline) to 578,233 (+69.78%), failed the
check and was discarded before any commit.

| Workload | After O7 | After O6 | Change vs O7 | Change vs baseline | malloc calls | realloc calls | memcpy calls |
|---|--:|--:|--:|--:|--:|--:|--:|
| `SYNCHAOS_wire22` | 1,673,950,274 | 1,613,824,201 | −3.59% | −33.24% | 311,905 | 9,351 | 2,564,614 |
| `SCALE2_episode1v2` | 456,387,352 | 369,062,175 | −19.13% | −39.23% | 80,207 | 23,942 | 554,672 |
| `MTELE1_wire24` | 5,028,005 | 4,864,943 | −3.24% | −49.62% | 3,403 | 503 | 11,460 |
| `TFRAG1_wire24` | 8,569,796 | 8,378,591 | −2.23% | −31.14% | 4,714 | 713 | 18,902 |
| `BVB1_wire24` | 17,300,504 | 16,755,693 | −3.15% | −15.70% | 7,511 | 590 | 40,431 |
| `M4a_dense64_lift` | 2,037,221,536 | 1,645,910,068 | −19.21% | −33.06% | 504,890 | 323,313 | 1,829,474 |
| `M4a_dense64_fight` | 1,326,920,948 | 1,231,860,255 | −7.16% | −34.97% | 610,388 | 154,671 | 2,072,198 |
| `M4a_gallery_patrol` | 195,316,887 | 189,743,220 | −2.85% | −25.48% | 303,323 | 81,328 | 318,741 |
| `M4a_corpse_continuation` | 3,621,350 | 3,570,560 | −1.40% | −34.63% | 3,736 | 242 | 10,787 |

**Gates run for O6** (on `fda9230`). Build, both format checks, Clippy with warnings denied, `cargo test
--workspace` (1,393 passed, 4 ignored), the release tests (1,360 passed, 2 ignored), `cargo +1.85.0 test --workspace`
(1,393 passed, 4 ignored), warnings-denied rustdoc; `run_reference.py --suite --jobs 2` (65 passed); the 95 CI
`stable` Python commands (95 passed, 0 failed, run with `python3`).

**Differential test** (`3bfaf45`, module `o6` of `crates/doom-sim/src/m4p_elision_tests.rs`). It keeps the pre-O6
per-cell scan as an oracle and compares visit sequences and panics on six crowded synthetic layouts (12 to 62
actors on a 4×2-cell blockmap, ids out of slot order, repeated ids, `MF_NOBLOCKMAP` actors, actors off the
blockmap, the mover's own copy differing from its stored one). It also tries every early-exit point, and a stale
handle on four further layouts. It made 2,332 full-walk visits. It was shown to fail with each of three mutations of
`things_in_box`: y-outer loops (both tests failed), visiting every kept entry before the missing-handle panic (the
panic test failed), and no substitution of the mover's own copy (both failed). `cargo test -p doom-sim` passed
(1,356, 2 ignored), and the module passed under the release profile and under rustc 1.85.0, with Clippy and format
clean. The full gate list was not rerun for this test-only commit.

## O8 (location memo): dropped

Not committed (2026-11-14, America/Los_Angeles). The memo was built and measured in six forms against the O6 build
with `ir --check` (each passed the 3% threshold). Each lowered excl-libc Ir on eight CI workloads (best form:
`SYNCHAOS_wire22` −5.38%, `M4a_gallery_patrol` −3.00%). Each raised `M4a_corpse_continuation` (+0.11% to +0.38%),
whose map has no BSP nodes. §5's zero-or-negative rule therefore drops it; the working tree was reset, so no O8 gate
list ran. The forms, their measurements and the cause are in the
[investigation, §3 O8](../M4P_REFERENCE_INVESTIGATION.md#3-inventory).

## Group check 3

After O6 (head `3bfaf45`; O8 dropped), on 2026-11-14 between 22:24 and 22:31 PST (America/Los_Angeles), while the
machine was idle with the caps lifted. The machine was in use by the owner from about 23:00 PST, with the caps back
on, and timings are held.

**Real-map digests (no timing recorded).** `python3 -I tools/bench_advance.py timing --reps 1` ran with the rustc
1.85.0 release build in `~/scratch/m4p/target-185` and the scratch IWAD copy (both ext4), with no build, test, fuzz or
Valgrind process running. The idle gate passed at its default limits. After the run, the owner's instruction to hold
timings applied to this group check, so the run counts only as a digest check and **no timing figure from it is
recorded**. All 61 runs ended in the recorded final stripped SHA-256 and admitted-tic count (0 mismatches; 158,966
admitted tics).

**Synthetic genuine-engine checks.** The same commands as group check 1, run one at a time (one C job at a time),
with dumps under `~/scratch/m4p/ci-g3` (ext4); the address variants run with `ASAN_OPTIONS=detect_leaks=0` set by the
runners (leak detection disabled). All passed:

- `m3h9c_reference.py --check --keep-dumps`: 6 scripts, variants identical for all 6, results file matched;
  `m3h9c_snapshot_goldens.py --dumps … --check` (11 files); `check_player_level.py` on the six Rust runs (all checks
  passed).
- `m4a_reference.py --check --keep-dumps --selftest 300`: 30 scripts, variants identical for all 30, results file
  matched, selftest passed; `m4a_snapshot_goldens.py --check` (19 files); `check_player_episode1.py --example-args
  "--policy episode1" --require` (all checks passed); `m4b_m4a_goldens.py --check` (30 scripts, 15,116 snapshots
  projected).
- `m4b_reference.py --check --keep-dumps --selftest 30`: 18 scripts, variants identical for all 18, the `SPECH9`
  overrun probe reported `p_map.c:242` after tic 74 as required, results file matched, selftest passed;
  `m4b_snapshot_goldens.py --check --selftest` (20 files, 8 selftest checks).
- `m4b_reference.py --set 2 --check --keep-dumps`: 4 scripts, variants identical, results file matched;
  `m4b_snapshot_goldens.py --set 2 --check` (5 files).
- `check_player_episode1v2.py --require`: 52 of 52 scripts matched (20,205 tics), 0 pending.

No digest or byte differed, so O6 was not reverted. Not run for the group: the opt-in `strict32` variants, fuzz
builds and the sanitizer campaign (due at step 11). `/mnt/c` had 49 GiB free after the group check.

## O9a (descriptor tables)

Code commit `1408ee3`, recorded on 2026-11-15 (America/Los_Angeles), same host, toolchain, valgrind and ext4 paths;
the machine was in use by the owner, so only instruction counts were taken. `/mnt/c` had 42 GiB free after the gates.

**CI-set instruction counts.** `python3 -I tools/bench_advance.py ir --check` on the rustc 1.85.0 build: all nine
outcomes equal the baseline's; `threshold 3.0%: ok`. The baseline was not lowered.

| Workload | After O6 | After O9A | Change vs O6 | Change vs baseline | malloc calls | realloc calls | memcpy calls |
|---|--:|--:|--:|--:|--:|--:|--:|
| `SYNCHAOS_wire22` | 1,613,824,201 | 1,321,947,849 | −18.09% | −45.31% | 311,905 | 9,351 | 2,564,614 |
| `SCALE2_episode1v2` | 369,062,175 | 301,820,357 | −18.22% | −50.30% | 80,207 | 23,942 | 554,672 |
| `MTELE1_wire24` | 4,864,943 | 4,463,208 | −8.26% | −53.78% | 3,403 | 503 | 11,460 |
| `TFRAG1_wire24` | 8,378,591 | 7,766,939 | −7.30% | −36.16% | 4,714 | 713 | 18,902 |
| `BVB1_wire24` | 16,755,693 | 15,774,682 | −5.85% | −20.64% | 7,511 | 590 | 40,431 |
| `M4a_dense64_lift` | 1,645,910,068 | 1,470,761,268 | −10.64% | −40.19% | 504,890 | 323,313 | 1,829,474 |
| `M4a_dense64_fight` | 1,231,860,255 | 997,969,891 | −18.99% | −47.31% | 610,388 | 154,671 | 2,072,198 |
| `M4a_gallery_patrol` | 189,743,220 | 155,772,424 | −17.90% | −38.82% | 303,323 | 81,328 | 318,741 |
| `M4a_corpse_continuation` | 3,570,560 | 3,485,757 | −2.38% | −36.19% | 3,736 | 242 | 10,787 |

**Gates run for O9a** (on the tree of `1408ee3`). Build, both format checks, Clippy with warnings denied, `cargo test --workspace` (1,395 passed, 4 ignored), the release tests (1,362 passed, 2 ignored), `cargo +1.85.0 test --workspace` (1,395 passed, 4 ignored), warnings-denied rustdoc; `run_reference.py --suite --jobs 2` (65 passed); the 95 CI `stable` Python commands (95 passed, 0 failed, run with `python3`). All under the owner's caps (`CARGO_BUILD_JOBS=6`, `NEXTEST_TEST_THREADS=4`, `RUST_TEST_THREADS=4`, two C jobs).

## O9b (decoration states inline)

Code commit `251ada7`, recorded on 2026-11-15 (America/Los_Angeles), under the same conditions. `/mnt/c` had 45 GiB
free after the gates.

**CI-set instruction counts.** `ir --check` as above: all nine outcomes equal the baseline's; `threshold 3.0%: ok`;
no workload rose. The three wire-24 workloads and `M4a_corpse_continuation` hold no decoration and are unchanged.

| Workload | After O9a | After O9B | Change vs O9a | Change vs baseline | malloc calls | realloc calls | memcpy calls |
|---|--:|--:|--:|--:|--:|--:|--:|
| `SYNCHAOS_wire22` | 1,321,947,849 | 1,309,257,849 | −0.96% | −45.84% | 41,851 | 9,351 | 2,564,614 |
| `SCALE2_episode1v2` | 301,820,357 | 297,831,537 | −1.32% | −50.96% | 27,318 | 9,517 | 554,672 |
| `MTELE1_wire24` | 4,463,208 | 4,463,208 | 0.00% | −53.78% | 3,403 | 503 | 11,460 |
| `TFRAG1_wire24` | 7,766,939 | 7,766,939 | 0.00% | −36.16% | 4,714 | 713 | 18,902 |
| `BVB1_wire24` | 15,774,682 | 15,774,682 | 0.00% | −20.64% | 7,511 | 590 | 40,431 |
| `M4a_dense64_lift` | 1,470,761,268 | 1,463,542,068 | −0.49% | −40.48% | 351,226 | 323,313 | 1,829,474 |
| `M4a_dense64_fight` | 997,969,891 | 988,833,091 | −0.92% | −47.80% | 415,934 | 154,671 | 2,072,198 |
| `M4a_gallery_patrol` | 155,772,424 | 152,388,424 | −2.17% | −40.15% | 231,303 | 81,328 | 318,741 |
| `M4a_corpse_continuation` | 3,485,757 | 3,485,757 | 0.00% | −36.19% | 3,736 | 242 | 10,787 |

**Gates run for O9b** (code of `251ada7`; its proof text was added to the commit after the gates). Build, both format checks, Clippy with warnings denied, `cargo test --workspace` (1,395 passed, 4 ignored), the release tests (1,362 passed, 2 ignored), `cargo +1.85.0 test --workspace` (1,395 passed, 4 ignored), warnings-denied rustdoc; `run_reference.py --suite --jobs 2` (65 passed); the 95 CI `stable` Python commands (95 passed, 0 failed, run with `python3`). All under the owner's caps (`CARGO_BUILD_JOBS=6`, `NEXTEST_TEST_THREADS=4`, `RUST_TEST_THREADS=4`, two C jobs).

## O9c (state-role memo): not committed

Paused on 2026-11-15 at 02:16 PST at the owner's request, with its gates incomplete; the work is a patch in scratch,
not on the branch. Observed so far, on the build before formatting: `ir --check` passed with all nine outcomes equal
and every workload lower than after O9b (−2.22% to −4.50%). Of the full gate list, build, both format checks,
Clippy, the workspace tests (1,395 passed, 4 ignored), the release tests (1,362 passed, 2 ignored), the rustc 1.85.0
tests (1,395 passed, 4 ignored), rustdoc and the reference suite (65 passed) passed; the 95 CI commands and the gate
run's own `ir --check` were stopped before they finished. O9d, O10 and group check 4 were not started.
