# M4p evidence record

Observed results of the M4p milestone (simulation performance with no behaviour change). Only observations
appear here. The [M4p investigation](../M4P_REFERENCE_INVESTIGATION.md) owns the profile, the decisions P1 to P9
and the plan; [testing](../TESTING.md#advance-benchmarks-m4p) owns the commands. Exact review state and CI belong
to GitHub.

**Scope of the record.** Phase 2 step 1 (2026-11-14, America/Los_Angeles): the committed benchmark harness and
the master baseline, in [Baseline (master)](#baseline-master). Step 2 (O1), in [O1 (owner callback)](#o1-owner-callback).
Step 3 (O3, O4) and the first group check, in [O3 (pickup revalidation)](#o3-pickup-revalidation),
[O4 (owner-loop monster_actor)](#o4-owner-loop-monster_actor) and [Group check 1](#group-check-1).
Later steps append their own sections.

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
