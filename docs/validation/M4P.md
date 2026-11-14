# M4p evidence record

Observed results of the M4p milestone (simulation performance with no behaviour change). Only observations
appear here. The [M4p investigation](../M4P_REFERENCE_INVESTIGATION.md) owns the profile, the decisions P1 to P9
and the plan; [testing](../TESTING.md#advance-benchmarks-m4p) owns the commands. Exact review state and CI belong
to GitHub.

**Scope of the record.** Phase 2 step 1 (2026-11-14, America/Los_Angeles): the committed benchmark harness and
the master baseline, in [Baseline (master)](#baseline-master). Later steps append their own sections.

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
