# M4p: simulation performance with no behaviour change: reference investigation

Phase 1 note for milestone M4p, which makes `Simulation::advance` faster on the real Episode 1 maps without
any observable change: no new wire, policy or DSIM version, and no change to any digest, DSIM byte, refusal,
refusal reason or event. This note summarises the master baseline, attributes the profile to causes, lists the
candidate optimisations with an equivalence argument for each, sets out the owner decisions P1–P9 with one
recommendation each, and gives the Phase 2 plan.

Code line numbers are of master `4021d60` (the merge of PR #58), which is also the measured tree; this branch
adds only status commits on top. The behavioural authority is unchanged: every optimisation here must reproduce
what the code does today, byte for byte, and the original C is reached only through the existing goldens and
genuine-engine checks. Milestone status lives in [ROADMAP](ROADMAP.md), support in
[COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md) and commands in
[TESTING](TESTING.md) (see the [documentation authority map](README.md#documentation-authority)). Observations
of this milestone will go to `docs/validation/M4P.md`, created in Phase 2. This note records only counts,
timings, instruction counts and digests: no IWAD-derived coordinate, lump byte or per-object datum.

**Status / next step (2026-11-14 PST).** Phase 1 (profile, inventory, decisions, plan) is done. Phase 2 step 1
(the committed harness) is done on branch `feat/m4p-performance`: `tools/bench_advance.py` (code commits `ea507c4`
and `debb09f`; the docs commit after `debb09f` adds [validation/M4P.md](validation/M4P.md), the
[TESTING](TESTING.md#advance-benchmarks-m4p) commands and this line), and the master baseline is committed as
`tests/reference/m4p_ir_baseline.json` (rustc 1.85.0, valgrind 3.26.0). The modes are the subcommands `timing`
and `ir` (not `--local`/`--ir`); no new Rust example was needed, `player_level_advance` and the four M4a examples
suffice. **Next step: step 2, O1 (§3 and §5)**; the decisions P1–P9 (§4) still await the owner, so confirm them
first (O1 itself needs none of them). Half-done work: none. Failing checks: none known (CI on the pushed head
is checked by GitHub; the new `simulation-ir` CI job of P6 is not yet written and its hosted-runner stability is
unmeasured). Resume commands (build on ext4, never `/mnt/*`; `/mnt/c` had 71 GiB free):
`export CARGO_TARGET_DIR=~/scratch/m4p/target-185; cargo +1.85.0 build -p doom-sim --examples --release --locked;
python3 -I tools/bench_advance.py ir --check` (expect all 0.00% before any optimisation, 9 workloads), then after
a change the same command shows the delta; timing: `python3 -I tools/bench_advance.py timing --binary
$CARGO_TARGET_DIR/release/examples/player_level_advance --iwad ~/scratch/m4p/iwad/DOOM.WAD --out-dir
~/scratch/m4p/harness --reps 5` on an idle machine (the scratch IWAD copy and `target-185` exist locally only).
Compare timings only between builds of the same toolchain. **Open owner questions: P1–P9.**

## Summary

- The per-tic **preflight** (`preflight_expanded_demons_for`) costs 61–87% of advance instructions on the 19
  real-map runs. `thinkers::run`, the work the tic actually does, costs 10–26% (§2.2).
- The largest single cause is a **quadratic repeat**. `preflight_monster_owner` runs `preflight_map_demons`
  (a full validation of every actor) once. It then calls `preflight_monster_callback` once for each live
  Demon (`MT_SERGEANT`, the historical "owner" type), and each call runs the same full validation again on
  the same immutable `&self`. That costs 37–74% of advance instructions on every map with Demons (E1M3–E1M9,
  DEMO1; up to 18 live Demons per tic on E1M8) and 0% on E1M1 and E1M2 (§2.3).
- The other measured causes: a second `validate_item` of every pickup in the same pass; sound-target
  references validated once per sector rather than once per distinct actor; the location witness (a BSP
  descent per actor per tic); a `BTreeSet` built per tic; event vectors regrown from empty and copied at
  commit; `for_things` scanning the whole spatial order once per blockmap cell; and per-call decoding of
  static descriptors with `Vec` allocation.
- **Rejected as primary causes:** cloning state for refuse-the-tic (the staged `demon_plan` copy plus its
  commit is 0.2–1.7% on real maps), checked arithmetic (at most about 1% self), sight queries (0.3–3.3%) and
  `BTreeMap` lookups (only the per-tic `linked` set, 0.8–6.3%) (§2.4).
- Every recommended optimisation is argued to give the identical outcome. Most elide a check whose identical
  call already succeeded earlier in the same `&self` borrow. The rest are memos of pure functions keyed by
  their full input, or reuse of non-canonical buffers. Nothing changes which tic refuses, the refusal reason
  or the order of the checks that decide it (§3).
- **Projection from the Callgrind data** (an estimate, not a measurement): the worst median-length script
  (E1M6 FIGHT) goes from 6.30 M to about 1.44 M instructions per tic, about 4.4 times fewer. The projected
  gain is 1.5–7.8 times by script. **Recommended target (P1):** every map's advance-only median at or below
  150 µs per tic, with 100 µs as a stretch; 5 times the C engine is out of reach without changing the
  validation design (§4).

## 1. Baseline

The baseline was measured on master `4021d60` on 2026-11-14 (PST), from ext4 scratch only. Raw data,
scripts and the detailed method stay in scratch (`~/scratch/m4p/baseline.md`, `timing/`, `cg/`) and are not
committed. Phase 2 step 1 re-records the baseline with the committed harness (§5), and the evidence record
`docs/validation/M4P.md` will hold that measurement. This section only summarises it.

**Host.** Intel Core i7-8750H at 2.20 GHz, 6 cores / 12 logical CPUs, 7.7 GiB, Linux 6.18.58.1 under WSL2,
rustc 1.100.0, valgrind 3.26.0, default release profile, no CPU isolation. The IWAD was a scratch copy; DEMO1
was patched at byte 0 as in M4b, with the patched digest matching the recorded one.

**Idle caveat.** Before each repetition the harness waited for a 1-minute load below 1.0. The
single-threaded timing process itself contributes about 1.0, so that gate was marginal after the first
repetition (0.95–0.99 at the starts). No other process was observed. Windows-side load is invisible from WSL.

**Advance-only timing** (wire 24, `player_level_advance --policy episode1v2`, 61 cases, 5 repetitions,
sequential, order rotated). All 305 final stripped SHA-256 values equal `tests/reference/m4b_real_results.json`,
and the admitted tics equal the recorded tics (158,966 in total). The median is the median of script medians.

| Map | Cases | Admitted tics | Advance median µs/tic | Worst run µs/tic | Genuine C median µs/tic (M4a) |
| --- | ---: | ---: | ---: | ---: | ---: |
| E1M1 | 5 | 10,029 | 53.0 | 60.9 | 3.7 |
| E1M2 | 5 | 9,094 | 127.1 | 140.8 | 5.8 |
| E1M3 | 5 | 13,750 | 281.7 | 300.9 | 12.1 |
| E1M4 | 5 | 9,386 | 218.6 | 238.8 | 6.8 |
| E1M5 | 10 | 36,783 | 386.1 | 611.1 | not measured |
| E1M6 | 5 | 12,742 | 633.2 | 674.5 | 13.4 |
| E1M7 | 5 | 12,939 | 290.2 | 342.5 | 11.5 |
| E1M8 | 10 | 30,840 | 256.2 | 388.0 | not measured |
| E1M9 | 10 | 21,693 | 518.0 | 792.2 | not measured |
| DEMO1 | 1 | 1,710 | 565.2 | 566.2 | not measured |

The C column is the fresh strict32 median of [M4a](validation/M4A.md#simulation-only-real-map-timings),
measured with a different timer boundary. It is descriptive, not an equivalent-work ratio.

**Callgrind.** `valgrind --tool=callgrind --toggle-collect='<doom_sim::Simulation>::advance'` on a release
build with line tables only (`CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`, same opt-level). Per map it
ran the median-length script and the longest script (19 real-map runs, all final digests equal to the
recorded ones), plus four synthetic M4a example workloads. Instructions per advance call:

| Map | Median script Ir/tic | Longest script Ir/tic |
| --- | ---: | ---: |
| E1M1 | 506,625 (FIGHT) | 493,310 (CHAOS) |
| E1M2 | 1,017,143 (M2FIGHT) | 1,070,937 (M2WALK) |
| E1M3 | 2,287,783 (M3IDLE) | 2,727,146 (M3WALK) |
| E1M4 | 2,209,569 (M4FIGHT) | 2,255,282 (M4WALK) |
| E1M5 | 2,088,465 (M5IDLE) | 5,660,268 (M5WALK3) |
| E1M6 | 6,302,335 (M6FIGHT) | 4,895,605 (M6WALK) |
| E1M7 | 2,540,774 (M7CHAOS) | 2,651,319 (M7WALK) |
| E1M8 | 4,035,195 (M8CHAOS3) | 2,001,538 (M8WALK) |
| E1M9 | 5,316,169 (M9FIGHT) | 5,903,325 (M9WALK3) |
| DEMO1 | 5,736,049 (M5DEMO1) | (one script) |

The M4a workloads give: `player_e1m1` gallery_patrol 196,008; `player_barrels` dense64_fight 1,412,788;
dense64_lift 1,954,360; `player_demon_melee` corpse_continuation 15,826. These are within 6% of the M4a
master numbers.

Where a timed median and a Callgrind median script coincide in kind, time and instructions agree at about
0.10 µs per thousand instructions (E1M1 0.105, E1M4 0.099, E1M6 0.100, E1M9 0.097, DEMO1 0.099). The
projections below use that factor. The factor is not a constant of the machine, and E1M5 and E1M8 depart
from it because their median timed script is not the median-length script.

## 2. Profile

### 2.1 Method

The data are self costs from `callgrind_annotate --inclusive=no --threshold=100`, summed per function over
the inlined-file entries. The cause table instead sums, from the raw call graph (`calls=` records), the
inclusive instructions of named caller→callee edges. Edge sums are exact for non-recursive functions.
The `--inclusive=yes` view of `callgrind_annotate` splits a function across inlined files and is not used
for shares. Call counts per tic are `calls=` counts divided by the advance calls. Scratch scripts:
`~/scratch/m4p/scripts/{agg,calls,causes}.py`.

### 2.2 Self cost by function (union of each run's top 30, top 30 by median shown)

The columns are the median-length script of each map, as a percentage of advance instructions. The range
covers all 19 real-map runs.

| Function | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 | DEMO1 | 19-run range |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `Simulation::monster_actor` | 9.1 | 7.7 | 9.8 | 11.7 | 11.2 | 12.1 | 10.3 | 14.7 | 14.9 | 13.8 | 7.7–14.9 |
| `__memcpy_avx_unaligned_erms` | 13.4 | 15.3 | 13.7 | 11.4 | 13.1 | 10.7 | 11.8 | 8.8 | 9.0 | 10.9 | 8.8–16.3 |
| `Simulation::preflight_map_demons` | 3.4 | 3.6 | 7.3 | 6.4 | 9.3 | 9.3 | 7.5 | 9.6 | 7.8 | 10.1 | 3.4–10.1 |
| `world_pickups::validate_item` | 4.1 | 4.0 | 8.9 | 6.2 | 8.6 | 11.5 | 8.8 | 6.5 | 6.8 | 7.5 | 3.9–11.5 |
| `original_registry::state_role_for` | 3.5 | 2.8 | 3.4 | 4.9 | 4.3 | 4.6 | 3.9 | 6.2 | 6.7 | 6.0 | 2.8–6.7 |
| `Simulation::validate_monster_type` | 2.6 | 2.4 | 3.6 | 3.8 | 4.3 | 4.5 | 3.8 | 5.1 | 4.7 | 5.0 | 2.4–5.1 |
| `OriginalActorType::descriptor` | 2.7 | 2.1 | 3.9 | 3.1 | 3.3 | 4.2 | 3.6 | 4.0 | 3.4 | 2.8 | 1.8–4.2 |
| `Simulation::preflight_expanded_demons_for` | 5.6 | 6.2 | 3.9 | 2.9 | 3.0 | 1.9 | 3.2 | 0.8 | 1.2 | 1.4 | 0.8–6.2 |
| `thinkers::run` | 5.8 | 6.3 | 4.1 | 2.8 | 3.2 | 1.8 | 3.2 | 0.9 | 1.1 | 1.4 | 0.9–6.3 |
| `multisector::Topology::locate` | 5.5 | 5.9 | 4.2 | 2.6 | 3.0 | 1.9 | 3.4 | 0.8 | 1.1 | 1.3 | 0.8–5.9 |
| `Simulation::preflight_world_pickups` | 1.2 | 1.3 | 2.6 | 2.2 | 3.2 | 3.3 | 2.7 | 3.3 | 2.6 | 3.4 | 1.2–3.4 |
| `movement::check_things` (pursuit) | 1.5 | 1.0 | 0.0 | 6.4 | 0.0 | 1.0 | 3.2 | 4.9 | 6.7 | 4.2 | 0.0–10.9 |
| `Mover<teleport::Motion>::attempt` | 4.2 | 4.4 | 0.0 | 3.1 | 0.0 | 2.0 | 2.0 | 0.6 | 1.6 | 1.3 | 0.0–4.4 |
| `<i16 as SliceContains>::slice_contains` | 1.1 | 1.1 | 2.0 | 1.9 | 3.1 | 2.6 | 2.3 | 2.8 | 2.2 | 2.9 | 1.0–3.1 |
| `Simulation::validate_perception_reference` | 1.9 | 0.5 | 0.0 | 1.8 | 0.0 | 0.8 | 0.5 | 2.1 | 3.1 | 1.4 | 0.0–3.1 |
| `Simulation::monster_state_role` | 1.4 | 1.2 | 1.5 | 1.8 | 1.7 | 1.9 | 1.6 | 2.2 | 2.3 | 2.1 | 1.2–2.3 |
| `original_registry::require_role_for` | 1.1 | 1.0 | 1.3 | 1.6 | 1.7 | 1.5 | 1.3 | 2.1 | 1.9 | 1.9 | 1.0–2.1 |
| `Simulation::monster_reference` | 1.3 | 0.3 | 0.0 | 1.9 | 0.0 | 0.9 | 0.5 | 2.2 | 3.3 | 1.4 | 0.0–3.3 |
| `world_pickups::kind_for` | 0.8 | 0.8 | 1.7 | 1.3 | 1.9 | 2.2 | 1.8 | 1.5 | 1.5 | 1.9 | 0.7–2.2 |
| `BTreeSet<ActorId>` search (`linked`) | 2.2 | 2.6 | 2.0 | 1.2 | 1.4 | 0.9 | 1.5 | 0.4 | 0.5 | 0.6 | 0.4–2.6 |
| `world_pickups::state_allowed` | 0.8 | 0.9 | 1.6 | 1.1 | 1.6 | 2.0 | 1.6 | 1.3 | 1.1 | 1.4 | 0.8–2.0 |
| `Simulation::original_actor_identity` closure | 0.8 | 0.7 | 0.9 | 1.1 | 1.0 | 1.1 | 0.9 | 1.4 | 1.3 | 1.2 | 0.7–1.4 |
| `Simulation::validate_combat_type` | 0.5 | 0.5 | 1.0 | 0.9 | 1.3 | 1.3 | 1.0 | 1.3 | 1.1 | 1.4 | 0.5–1.4 |
| `world_pickups::validate_item` closure | 0.6 | 0.7 | 1.5 | 1.0 | 1.4 | 1.9 | 1.4 | 1.1 | 1.1 | 1.2 | 0.6–1.9 |
| `Simulation::preflight_monster_owner` | 1.9 | 2.0 | 1.4 | 0.9 | 1.1 | 0.6 | 1.1 | 0.3 | 0.4 | 0.4 | 0.3–2.0 |
| `ThinkerStore<Actor>::next_visit` | 1.8 | 2.0 | 1.3 | 0.9 | 1.0 | 0.6 | 1.0 | 0.3 | 0.3 | 0.4 | 0.3–2.0 |
| `world_pickups::counted_map_things` | 0.5 | 0.4 | 0.8 | 0.7 | 1.1 | 1.0 | 0.9 | 1.0 | 0.8 | 1.1 | 0.4–1.1 |
| `BTreeSet` node insertion | 1.4 | 1.6 | 1.1 | 0.7 | 0.8 | 0.5 | 0.8 | 0.2 | 0.3 | 0.3 | 0.2–1.6 |
| `player_map_scale::position_admitted` | 0.4 | 0.4 | 0.9 | 0.7 | 0.8 | 1.0 | 0.8 | 0.8 | 0.7 | 0.7 | 0.4–1.0 |
| `Simulation::demon_plan` | 1.3 | 1.4 | 0.9 | 0.6 | 0.7 | 0.4 | 0.7 | 0.2 | 0.3 | 0.3 | 0.2–1.4 |

The `slice_contains` row is the THINGS rescan in `counted_map_things` together with the small constant-list
membership tests in the validators.

### 2.3 Cost by cause (inclusive edge sums)

Shares overlap where one cause sits inside another, for example sound-target references inside the
repeated validation, or the realloc copies inside the event growth.

| Cause (share of advance Ir unless a count) | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 | DEMO1 | 19-run range |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| whole per-tic preflight (`preflight_expanded_demons_for`) | 65.1 | 60.6 | 76.9 | 73.3 | 80.5 | 83.6 | 75.5 | 87.4 | 82.6 | 82.2 | 60.6–87.4 |
| of which `preflight_map_demons` repeated inside `preflight_monster_callback` | 0.0 | 0.0 | 36.6 | 41.1 | 51.0 | 63.1 | 43.0 | 73.6 | 62.8 | 66.5 | 0.0–73.6 |
| all `preflight_map_demons` calls | 36.0 | 28.9 | 54.8 | 54.8 | 63.8 | 71.3 | 57.3 | 77.7 | 68.7 | 72.0 | 28.9–77.7 |
| sound-target references in `preflight_map_demons` | 8.6 | 2.0 | 0.0 | 8.2 | 0.0 | 3.5 | 2.1 | 9.4 | 13.6 | 6.2 | 0.0–13.9 |
| target and sound-target references in `preflight_monster_callback` | 0.0 | 0.0 | 0.0 | 3.8 | 0.0 | 2.0 | 0.8 | 4.4 | 7.1 | 2.6 | 0.0–7.1 |
| `validate_item`, all calls (half are the duplicate in the `preflight_map_demons` loop) | 6.7 | 6.7 | 14.9 | 10.2 | 14.5 | 19.2 | 14.7 | 11.0 | 11.2 | 12.6 | 6.4–19.2 |
| `monster_actor` re-run by `preflight_monster_owner` and the callback | 8.1 | 8.3 | 5.7 | 3.9 | 4.4 | 2.5 | 4.4 | 1.5 | 1.7 | 2.0 | 1.5–8.3 |
| `counted_map_things` (THINGS rescan) | 1.6 | 1.5 | 2.8 | 2.7 | 4.2 | 3.6 | 3.2 | 3.7 | 3.0 | 4.0 | 1.4–4.2 |
| location witness (`locate_validated` in the preflight) | 5.5 | 6.0 | 4.3 | 2.6 | 3.1 | 2.0 | 3.4 | 0.8 | 1.1 | 1.3 | 0.8–6.0 |
| per-tic `linked` `BTreeSet` | 5.3 | 6.3 | 4.8 | 2.9 | 3.5 | 2.1 | 3.6 | 0.8 | 1.2 | 1.4 | 0.8–6.3 |
| event vector growth (`RawVec<Event>::grow_one`) | 5.4 | 5.9 | 4.4 | 3.6 | 3.5 | 3.1 | 3.4 | 1.5 | 2.1 | 2.7 | 1.5–7.3 |
| event vector append at commit | 3.7 | 4.5 | 3.2 | 2.3 | 3.0 | 1.5 | 2.8 | 1.2 | 1.2 | 1.5 | 1.2–4.5 |
| `thinkers::run` (the tic's own work) | 21.5 | 26.1 | 14.7 | 20.0 | 12.0 | 11.1 | 17.1 | 9.7 | 13.7 | 13.4 | 9.7–26.1 |
| of which `movement::check_things` (all callers) | 1.9 | 2.0 | 0.0 | 6.6 | 0.0 | 1.4 | 3.6 | 5.1 | 7.0 | 4.8 | 0.0–14.0 |
| `demon_plan` + `commit_demon_plan` (the staged copy) | 1.7 | 1.7 | 1.0 | 0.7 | 0.8 | 0.4 | 0.8 | 0.2 | 0.3 | 0.3 | 0.2–1.7 |
| `memcpy`, total self | 13.4 | 15.3 | 13.7 | 11.4 | 13.1 | 10.7 | 11.8 | 8.8 | 9.0 | 10.9 | 8.8–16.3 |
| `memcpy` called directly by actor-by-value readers | 4.3 | 4.2 | 5.3 | 4.9 | 5.9 | 5.6 | 5.2 | 5.8 | 5.4 | 6.1 | 4.2–6.1 |
| malloc + free + realloc (inclusive) | 7.2 | 8.0 | 6.7 | 5.2 | 5.2 | 4.3 | 5.0 | 3.7 | 3.6 | 3.9 | 3.6–9.5 |
| live Demons (callback calls per tic) | 0 | 0 | 2.0 | 3.0 | 4.0 | 7.7 | 3.0 | 18 | 11 | 12 | 0–18 |
| `malloc` calls per tic | 120 | 155 | 323 | 301 | 102 | 420 | 280 | 417 | 615 | 182 | 102–615 |
| `realloc` calls per tic | 13 | 26 | 31 | 27 | 32 | 50 | 37 | 382 | 187 | 53 | 9.3–382 |
| `memcpy` calls per tic | 894 | 1,846 | 4,697 | 4,061 | 4,656 | 12,719 | 5,001 | 8,320 | 10,115 | 12,324 | 885–12,719 |

The four synthetic M4a workloads: the preflight is 34–61% (corpse_continuation 50.3%, with 13.7% repeated
`preflight_map_demons` from its single Demon). `check_things` is 43.7% of dense64_lift. Event growth plus
append is 5.0–11.5%. `decoration_states` is 6.3% of gallery_patrol and 2.4% of dense64_fight, and
`PlayerWeaponState::validate_for` is 1.7% of gallery_patrol. `demon_plan` plus commit is 1.0–5.8%.

### 2.4 Each expected candidate, confirmed or rejected

**C1. Whole-state copies for the refuse-the-tic preflight: rejected as a primary cause.** The checked tic
stages a plan. `demon_plan` copies the actor slot vector (`slots.extend_from_slice`,
`crates/doom-sim/src/demon_expanded_map.rs:440–441`), the sector-thinker slots, the spatial order (455), the
sound targets, the specials and small values. `commit_demon_plan` (620–650) swaps the copies in and keeps the
old buffers as spares. There is no full-simulation clone ([architecture](ARCHITECTURE.md#validation-and-failure-atomicity)).
The size probe on this build gives `Slot<Actor>` 208 bytes, `Actor` 136, `ActorLife` 80, `Event` 152 and
`ActorId` 16. With about 373 live actors on E1M6 FIGHT, the slot copy is therefore at least about 78 KB per
tic, plus 16 bytes per `max_slots` entry of order. The copy and commit together cost 0.2–1.7% of advance
instructions on real maps (corpse_continuation 5.8%). The `memcpy` that leads every self profile has
other callers:

- actor-by-value reads, 4.2–6.1% directly. `monster_actor` returns `Actor` by value
  (`crates/doom-sim/src/monsters.rs:534`), `preflight_world_pickups` copies each actor
  (`crates/doom-sim/src/world_pickups.rs:1208`), and `for_things` copies every candidate
  (`crates/doom-sim/src/movement.rs:261–265`);
- `realloc` copies during event growth, 1.5–7.2%;
- the event append at commit (`crates/doom-sim/src/demon_expanded_map.rs:993–994`), 1.2–4.5%.

**C2. Per-tic validation of every actor: confirmed, and the dominant cause.** Its largest part is
repetition, not the per-actor checks themselves:

- `preflight_monster_owner` (`crates/doom-sim/src/monsters.rs:719–749`) calls `preflight_map_demons` (720) and
  `preflight_perception_roster` (721). For each live owner it then calls `preflight_monster_callback` (731),
  which repeats both (580, 588) on the same `&self`. On these policies the owner type is `MT_SERGEANT`
  (`monster_is_owner_type`, 437–443; `owner_slot`, `crates/doom-sim/src/original_registry.rs:1010–1012`).
  The repeated calls cost 36.6–73.6% on the eight maps with Demons. This is the quadratic term: per tic it
  is the number of live Demons times the number of actors plus the number of THINGS.
- `preflight_map_demons` (`crates/doom-sim/src/demon_map_spawn.rs:77–169`) runs `preflight_world_pickups` (88),
  which calls `validate_item` for every pickup (`crates/doom-sim/src/world_pickups.rs:1214`). The loop then
  calls `validate_item` again for every pickup with the same arguments (`demon_map_spawn.rs:126–129`). The
  duplicate is half of 6.4–19.2%.
- `preflight_map_demons` calls `monster_actor(id, 1008)` for every actor (97). `preflight_monster_owner`
  calls `monster_actor(id, 978)` for every actor again (`monsters.rs:727`), and the callback calls it a
  third time for the owner (584). The re-runs cost 1.5–8.3%.
- Sound targets: `preflight_map_demons` validates one reference per sector with a target
  (`demon_map_spawn.rs:85–87`), 0–1,281 calls per tic, nearly all the same player body. The callback does
  the same for every owner (`monsters.rs:589–598`). Together 0–20.9%.
- The M4a-deferred items, as inclusive shares on real maps: `require_role_for` 1.0–2.1%
  (`original_registry.rs:1210`); `OriginalActorType::descriptor` 1.8–4.2%, which decodes 23 properties with
  checked conversions on every call (`crates/doom-sim/src/identity.rs:185–207`); `decoration_states` 0.1–3.4%,
  which builds a `Vec` (`crates/doom-sim/src/e1m1_rules.rs:153–162`, called through `spawn_state`, 328–330;
  843,901 `Vec<u16>` growths on E1M8 CHAOS3, the main source of its 382 reallocs per tic);
  `PlayerWeaponState::validate_for` at most 0.2% on real maps (1.7% of gallery_patrol); `validate_item`
  (`world_pickups.rs:586–651`) as above; `state_role_for` 5.4–10.4% inclusive (`original_registry.rs:1279`),
  reached from `monster_state_role` for every `monster_actor` call (`monsters.rs:532`).

**C3. Checked arithmetic and witness checks: rejected as a cost.** The checked integer intrinsics inside
`monster_actor` are about 0.9% self, and fixed-point helpers inside `validate_item` about 0.5%. The witnesses
are part of the refuse-the-tic contract and stay. Only their repetition (C2) is a cost.

**C4. `BTreeMap` lookups on hot paths: confirmed, but only one.** The per-tic `linked` set
(`demon_expanded_map.rs:539–544`), with one insertion and one lookup per actor and node allocations, costs
0.8–6.3%. No other `BTreeMap` appears in the top functions.

**C5. Per-tic allocation: confirmed and moderate.** There are 102–615 `malloc` and 9–382 `realloc` calls per
tic, and allocation costs 3.6–9.5% inclusive. The sources:

- the event vector starts empty every tic (`events: Vec::new()`, `demon_expanded_map.rs:506`) and the event
  vectors make about 42 growth calls per tic on E1M6 FIGHT;
- `linked` set nodes;
- `decoration_states` vectors;
- `check_things` candidate vectors (`movement.rs:208–217`);
- the sight scratch vectors.

**C6. Repeated blockmap or line scans: confirmed in two places.**

- The location witness. `actor.location != Some(self.world.locate_validated(actor.position()))`
  (`demon_expanded_map.rs:587`) descends the BSP for every actor every tic. All of `Topology::locate` comes
  from here, 0.8–6.0%.
- `for_things` (`movement.rs:245–277`). For each blockmap cell of the box it scans the whole
  `spatial.order` and copies each candidate to test its cell. That is O(cells × actors) per move attempt:
  `check_things` costs up to 14.0% (E1M5 M5WALK3) and 43.7% of dense64_lift.

**C7. Sight queries: rejected for M4p.** `action_sight` and `try_extent_query` cost 0.3–3.3%.

## 3. Inventory

"Shared" means that the code also runs under older wires. An optimisation of shared code is allowed only if
every older-wire golden, digest and fixture reproduces byte for byte. The arguments below are profile
independent unless stated. The gains are projected kIr per tic on the median-length script, taken from the
edge sums. The guards for every item are the full Phase 2 gate list (§5); item-specific guards are named.

Three facts recur in the arguments. (F1) Every validation named here takes `&self` and draws no RNG. (F2)
The only interior mutability these validations touch is the `AdmittedTypes` memo
(`crates/doom-sim/src/demon_lifecycle.rs:5–24`), a pure-function cache whose contract is that a stale read
only repeats a validation whose result cannot change. (F3) The level-run work counters
(`crates/doom-sim/src/level_stats.rs`) are maxima of trace work, and none of these validations traces. So two
calls of the same validation in the same borrow return the same result, and dropping the second call when
the first returned `Ok` changes no outcome, no error, no counter and no state.

**O1. Do not repeat the pass inside the owner callback.** Shared (`monsters.rs`). Gain 0 on E1M1/E1M2, then
836–3,977 kIr per tic (36.6–73.6%). From the owner loop, call a private variant of the callback that omits
`preflight_map_demons` and `preflight_perception_roster`, both already `Ok` at 720–721 (F1–F3). It also
omits the owner test (the branch condition at 729 established it), `monster_actor(id, 1008)` and the
health test (the branch condition excludes `health <= 0` under the lifecycle profile). For `monster_actor`:
under `uses_demon_map` the loop at `demon_map_spawn.rs:97` already ran `monster_actor(id, 1008)`. Otherwise,
`monster_actor(id, 978)` at `monsters.rs:727` passed, and the only margin-dependent check
(`position_admitted`, 517–519) is either room-based or `within(v, margin)`, which is monotone in the margin
(`crates/doom-sim/src/sight.rs:163–165`). So the 1008 call passes too. The other callers of
`preflight_monster_callback` (older profiles, `prepare_monster_shot`) keep the full function unchanged.
Item-specific guards: corpse_continuation (one Demon), the canonical-demon and demon-map goldens, and M4b
MTELE1/TFRAG1 (synthetic, with Demons). Risk: low.

**O2. Validate each distinct sound target once per pass.** Shared. Gain 0–420 kIr per tic. In
`preflight_map_demons` (85–87), skip a target equal to the last target validated `Ok`. In the owner callback,
skip the sound targets for every owner after the first in the same pass, and skip repeats within the first
owner's list. `validate_perception_reference(t)` and `monster_reference(t)` depend only on `(self, t)` (F1),
so a skipped call would have returned the `Ok` already observed. The first failing target is still checked
at the same position in the same order, so the error and its position are unchanged. No store is needed:
one `Option<ActorId>` per loop and one flag per pass. Risk: low.

**O3. Drop the second `validate_item` in the same pass.** Shared. Gain 12–69 kIr per tic. The call at
`demon_map_spawn.rs:129` runs only under `has_world_pickups()` with `kind_for(..).is_some()`, on a
non-pending actor. `preflight_world_pickups` (`world_pickups.rs:1190–1247`) returns early only when that
policy flag is false, and otherwise calls `validate_item` with the identical arguments `(self.room(),
self.item_sectors(), a)` for exactly the same set (same order, same filter) before the loop starts. The
second call therefore returns `Ok`. The suffix and drop checks at 131–135 stay. Risk: low.

**O4. Replace the second `monster_actor` of the owner loop by its one margin-dependent check.** Shared,
applied only when `uses_demon_map()`. Gain 37–142 kIr per tic. In `preflight_monster_owner` (727), every
actor already passed `monster_actor(id, 1008)` in the loop at `demon_map_spawn.rs:97`, over the same
non-pending order (F1). In `monster_actor`, the only check that depends on the margin is
`position_admitted(pos, margin)` for owner types (`monsters.rs:517–519`), and it comes after the flag
checks, which already passed. A 978 margin is stricter than 1008 when there is no room, so the replacement
keeps exactly `if owner_type && !position_admitted(pos, 978) { Err(MonsterError::Arithmetic) }` and reuses
the actor value. Without `uses_demon_map` the full call stays. Risk: low; it needs a unit test that fails if
the residual check is removed.

**O5. Reuse the event buffers.** Shared by every checked-demon wire. Gain 37–235 kIr per tic. Take
`plan.events` from a spare `Vec` with retained capacity, as the code already does for `spare_sound_targets`,
`spare_specials` and `spare_writes` (`demon_expanded_map.rs:443–495`, `crates/doom-sim/src/lib.rs:587–605`). At
commit, when `self.pending` is empty, move the plan's vector into `self.events` instead of appending it,
and keep the previous one as the spare. The event sequence and its contents are unchanged. Capacity is not
observable: `Vec` equality, `Debug` and the snapshot fields read contents only. On a refused tic the plan,
and with it the spare, is dropped and `self.events` is untouched, as today. The store is bounded by the
largest admitted tic's event count. No new fallible allocation is added, so no new `StoreError::Capacity`
point appears. Risk: low.

**O6. Scan the spatial order once in `for_things`.** Shared by every wire with actor movement. Gain up to 555
kIr per tic (E1M5 M5WALK3), and about 0.7 of dense64_lift's 43.7%. Today the function visits, for x then y
over the box, every candidate of `spatial.order` whose cell is `[x, y]`, in order, and stops when `visit`
returns false (`movement.rs:258–276`). The same sequence comes from one pass over `spatial.order` that keeps
the in-box candidates with their cells (in order), followed by the same x/y loops over that short list. The
candidate order inside each cell is the spatial order, which is not a sort and does not rebuild the
BLOCKMAP. The candidate is read by reference, not copied. The early return and the `id` substitution are
unchanged. The pass runs only when the box is non-empty, so the "valid spatial handle" panic is reached in
exactly the same cases. Guards: dense64 workloads, the movement, barrels and multisector goldens, and the
real-map digests. Risk: medium (the order must match exactly); it needs a test comparing visit sequences on
crowded synthetic layouts.

**O7. Replace the `linked` `BTreeSet` with a slot bitmap.** Wire-24/23/22 shared preflight. Gain 24–120 kIr
per tic. Every id inserted has passed `is_pending(id) == Some(false)`, so it is the current generation of
its slot. Two equal ids therefore share a slot index, and two ids with the same slot index are equal. The
later `contains` queries use current ids from `actors.order()`. So membership by slot index is equivalent
to membership by `ActorId`, and duplicate detection gives the same `Ownership` error at the same position.
The bitmap holds `max_slots` bits, at most 8 KiB at the store's 65,536-slot bound (`crates/doom-sim/src/store.rs:298`). It is allocated infallibly,
like the `BTreeSet` today, or taken from a spare. Risk: low.

**O8. Memoise the location witness.** Gain 19–99 kIr per tic. `locate_validated(position)` is a pure function
of the position over the immutable `StaticWorld` BSP. Keep, per actor slot, the last
`(position, location)` pair that was actually computed. When the actor's position equals the stored
position, the stored location is exactly the function's value, so the comparison at
`demon_expanded_map.rs:587` gives the same boolean. A memo keyed by the full input of a pure function cannot
be stale, so no invalidation proof is needed. The store is bounded by `max_slots`, is non-canonical, and is
updated either through the `AdmittedTypes` atomic pattern or by passing it mutably from
`advance_expanded_demons`, which holds `&mut self`. Risk: low. It needs a debug assertion or test that the
memo and the direct call agree.

**O9. Static tables and pure memos for the per-actor validators.** Shared. Gain 8–24 kIr per tic from the
descriptor and `decoration_states` alone, plus the `state_role_for` memo (5.4–10.4% inclusive today, mostly
in the repeated passes; not included in the projection):

- `OriginalActorType::descriptor`: a once-built static table holds the same values. The `expect`s cannot
  fire on verified static data.
- `decoration_states`: a fixed array of at most 8 states plus a length gives the same membership.
- `state_role_for`, and `monster_state_role` under the expanded profile: an `Ok`-only memo per
  `(type, state)`, like `AdmittedTypes`. A miss recomputes and returns the same error.
- The `counted_map_things` total: computed once per `Simulation` from the immutable THINGS and the
  constant skill bit (`world_pickups.rs:1234–1238`), not once per pass.

Every result is identical by construction. Risk: low.

**O10. Read actors by reference in the validators.** Shared. Gain: most of the 4.2–6.1% direct `memcpy` that
remains after O1–O4. Internal `&Actor` variants of `monster_actor` and `perception_actor` replace the copies,
and `preflight_world_pickups` borrows. Pure refactor, same checks in the same order. Risk: low; the size is
mechanical.

**O11. Replace the staged plan copy by an undo log or copy-on-write: not recommended.** Gain at most 1.7% on
real maps. The risk is high: every write path would have to log, and a refused tic would have to restore
byte-identical slots, generations, free lists, order links and spare buffers. See P3.

**O12. Merge the separate per-actor loops into one pass: rejected.** When two invariants fail in different
loops, the first loop's error is reported today. A merged pass would report whichever actor comes first,
which changes the refusal reason. That is observable.

**O13. Remove or cheapen checked arithmetic and witnesses: rejected.** They cost about 1% and are contract.

**O14. Cache sight results: deferred.** 0.3–3.3%. Caching would need a proof over the mutable actor positions
and sector heights for little gain.

**Projection.** Applying O1, then O2–O4, then O5–O9 to the edge sums gives the following. The
`state_role_for` memo and O10 are excluded, so the projection is conservative for them. It is an estimate
for planning, not a measurement.

| Workload | Ir/tic now (k) | after O1 (k) | after O1–O4 (k) | after O1–O9 (k) | factor |
| --- | ---: | ---: | ---: | ---: | ---: |
| E1M1 FIGHT median | 507 | 507 | 411 | 306 | 1.7 |
| E1M2 M2FIGHT median | 1,017 | 1,017 | 888 | 663 | 1.5 |
| E1M3 M3IDLE median | 2,288 | 1,451 | 1,277 | 937 | 2.4 |
| E1M4 M4FIGHT median | 2,210 | 1,301 | 1,074 | 749 | 2.9 |
| E1M5 M5IDLE median | 2,088 | 1,023 | 911 | 677 | 3.1 |
| E1M5 M5WALK3 longest | 5,660 | 2,754 | 2,255 | 1,363 | 4.2 |
| E1M6 M6FIGHT median | 6,302 | 2,325 | 1,970 | 1,436 | 4.4 |
| E1M7 M7CHAOS median | 2,539 | 1,448 | 1,268 | 910 | 2.8 |
| E1M8 M8CHAOS3 median | 4,034 | 1,065 | 809 | 514 | 7.8 |
| E1M9 M9FIGHT median | 5,316 | 1,978 | 1,449 | 930 | 5.7 |
| E1M9 M9WALK3 longest | 5,903 | 2,595 | 2,011 | 1,178 | 5.0 |
| DEMO1 M5DEMO1 | 5,736 | 1,924 | 1,626 | 1,104 | 5.2 |

At about 0.10 µs per thousand instructions, the worst projected median-length script is about 145 µs per
tic (E1M6). What remains is the single necessary preflight pass and `thinkers::run`.

## 4. Owner decisions (P1–P9) with recommendations

Each decision answers four questions: (a) consistent with prior owner decisions, (b) no observable change on
any wire, (c) refuse-the-tic contract kept exactly, (d) bounded stores.

**P1. Target.** Recommendation: **every map's advance-only median (median of script medians, the committed
harness of §5) at or below 150 µs per tic, with 100 µs as a stretch.** Report the factor against the
M4a C medians but do not make it a gate. Basis: the projection (§3) puts the worst median-length script at
about 145 µs after O1–O9. The stretch needs the `state_role_for` memo, O10 and further constant-factor work
inside the single pass. Five times the C engine (18–67 µs) would need incremental validation that drops
per-tic witnesses, a design change outside M4p. (a) Yes: the owner asked to "reduce it without changing any
digest" with no fixed number. (b)–(d) Not applicable to a target.

**P2. Allowed equivalence arguments.** Recommendation: **admit exactly three kinds of change, each
documented per item in this note:**

1. eliding a check whose identical call already returned `Ok` in the same `&self` borrow (F1–F3);
2. a memo of a pure function keyed by its full input, or an `Ok`-only memo (the `AdmittedTypes` precedent);
3. reuse of non-canonical buffers (the spare-buffer precedent).

Reject any change that reorders checks across loops (O12). (a) Yes: these are the M3–M4 precedents. (b) Yes
by construction. (c) Yes: no state is written before the existing checks. (d) Yes: each store is bounded
(O5, O7, O8, O9).

**P3. Refusal preflight: keep the staged plan copy; no undo log or copy-on-write in M4p.**
Recommendation: **keep it.** The measured gain is at most 1.7% on real maps, and the equivalence proof
(byte-identical restoration of slots, generations, free lists, order links and spares on every refusal path)
is a large new risk. Reconsider only if the post-O10 profile shows the copy above 10%. (a) Yes: "no
full-simulation clone per tic" stays true. (b)–(d) Yes: nothing changes.

**P4. No new dependency and no new `unsafe`.** Recommendation: **keep both defaults.** Nothing in the data
needs either. The bitmap, the memo and the buffer reuse are all safe standard Rust. (a)–(d) Yes.

**P5. Shared older-wire code may be optimised.** Recommendation: **allow O1–O10 in shared code, under the
full older-wire gate set after each item** (every historical golden, digest and fixture byte for byte; see
§5). Most of the gain sits in functions shared with wires 22–24 and older monster profiles, and duplicating
them per wire would add maintenance risk for no benefit. (a) Yes: the brief permits it under that
condition. (b) Yes, if the gates pass. (c) Yes. (d) Yes.

**P6. CI regression guard.** Recommendation: **a new Linux-only CI job, `simulation-ir`, that runs Callgrind
(valgrind from the Ubuntu runner's packages, no Rust dependency) on a fixed set of synthetic workloads and
fails if a workload exceeds its committed baseline by more than 3%.** The workloads, all CI-available and
IWAD-free:

| Workload | Coverage | Advance Ir today |
| --- | --- | ---: |
| `player_level_advance` on `m3h9c_synthetic.wad` SYNCHAOS (wire 22) | older wire, events | 2,170,632,865 |
| `m4a_wads/scale.wad` SCALE2 under `--policy episode1v2` | pickups, decorations, barrels | 538,551,700 |
| `m4b_wads/montele.wad` MTELE1 (wire 24) | two Demons, the O1 path | 8,994,509 |
| `m4b_wads/tfrag.wad` TFRAG1 (wire 24) | one Demon | 12,348,284 |
| `m4b_wads/bvb.wad` BVB1 (wire 24) | Barons, infighting | 22,081,901 |
| `player_barrels --measure dense64_lift advance` | `check_things`, O6 | 2,345,232,039 |
| `player_barrels --measure dense64_fight advance` | events, decorations | 1,695,346,189 |
| `player_e1m1 --measure gallery_patrol advance` | decorations, weapon state | 235,209,643 |
| `player_demon_melee --measure corpse_continuation advance` | older Demon profile | 6,330,406 |

All values were measured here on master (rustc 1.100.0, valgrind 3.26.0). Stable instruction counts across
runners need two choices. First, build with a pinned toolchain (1.85.0, which CI already installs for the
MSRV job), so that a stable-channel update does not move the baseline. Second, gate on instructions
outside `libc` and `ld.so`, plus the `malloc`, `realloc` and `memcpy` call counts. glibc chooses its
`memcpy` variant by CPU, so libc instruction counts vary between runner hosts, while the call counts and the
Rust code's own instructions (built for baseline x86-64) do not. Report total Ir for information. A decrease
over 3% prints a request to lower the baseline. Whether `simulation-ir` becomes a required check is the
owner's call; existing required check names and steps are unchanged. (a) Yes: the brief proposed it. (b)–(d)
Not applicable. Not yet established: the run-to-run stability on hosted runners, to be measured in Phase 2
step 1 before the threshold is fixed.

**P7. AGENTS.md rule: timings only from the Linux disk.** Recommendation: **add to "Session
conventions":** "Run every timing, benchmark and Callgrind measurement from binaries built on and run from
the Linux filesystem (ext4, for example `~/scratch`), never from `/mnt/*`; record the path in the evidence."
Basis: the M4b 9p test, where the same binaries were +146.7% (median) slower on 9p with about 50 major
faults per process and +1.0% on ext4 ([M4b matched performance](validation/M4B.md#matched-performance)).
(a) Yes: it is the M4b conclusion. (b)–(d) Not applicable.

**P8. Fixed regression tests for each elision.** Recommendation: **each of O1–O4, O7 and O8 adds a
Rust-only test that would fail if the remaining check were removed or misplaced.** Example: an actor that
violates only the 978 margin under a non-room policy must still be refused with `MonsterError::Arithmetic`
in the same tic. These tests restate existing behaviour and need no new C citation. The existing goldens
remain the equivalence evidence. (a)–(d) Yes.

**P9. Scope cap and stop rule.** Recommendation: **cap M4p at O1–O10 plus the harness and the CI guard,
about 1.5k–3k lines including tests and tools.** Stop at P1's 150 µs target, or after O10, whichever comes
first, and report. Further constant-factor work becomes a follow-up. (a) Yes: one focused PR per milestone.
(b)–(d) Not applicable.

**Decisions without a single clear recommendation: none.** One item needs a Phase 2 measurement before its
number is final: P6's 3% threshold depends on runner stability.

## 5. Plan

Ordered by measured gain against risk. Sizes are code plus tests, excluding generated data.

| Step | Item | Gain (median scripts, kIr/tic) | Size | Risk |
| --- | --- | --- | --- | --- |
| 1 | Benchmark harness and CI Ir guard (P6) | none (measurement) | 500–900 lines | none to behaviour |
| 2 | O1: no repeated pass in the owner callback | 0–3,977 (up to 73.6%) | 60–120 | low |
| 3 | O3 + O4: duplicate `validate_item`, owner-loop `monster_actor` | 54–211 | 60–120 | low |
| 4 | O2: distinct sound targets | 0–420 | 40–80 | low |
| 5 | O5: event buffer reuse | 37–235 | 40–80 | low |
| 6 | O7: `linked` bitmap | 24–120 | 40–80 | low |
| 7 | O6: single-pass `for_things` | 0–555 | 100–200 | medium |
| 8 | O8: location memo | 19–99 | 60–120 | low |
| 9 | O9: descriptor table, `decoration_states`, `state_role_for` memo, THINGS total | 8–24 plus the memo | 120–250 | low |
| 10 | O10: actor reads by reference | most of the remaining direct `memcpy` | 150–300 | low |
| 11 | Re-record, evidence record, docs, review | none | docs | none |

**Gates per item (steps 2–10).** Each step runs all of the following:

- build, format and Clippy with warnings denied;
- the full workspace tests;
- every `--check` tool;
- `check_player_level*`, `check_player_episode1` and `check_player_episode1v2`;
- the mechanism goldens;
- the CI-set instruction delta of the harness.

The delta must be zero or negative on every CI workload, and it is recorded per step. **After every two or
three steps** (after 3, 6, 8 and 10) the following also run: all 61 real-map digests (local, IWAD) and the
M3h9c, M4a and M4b synthetic genuine-engine checks. Step 11 runs the full local gate set of
[TESTING](TESTING.md) (every original-C generator and DSIM encoder with `--check`, debug and release
examples, fuzz builds, a bounded sanitizer campaign) and the exact-head CI matrix.

**Step 1, the committed harness.**

- (Not needed: `player_level_advance` already prints these.) An example, `crates/doom-sim/examples/advance_bench.rs`, with no new dependency. Given a WAD, a map, a
  script and a policy, it prints the admitted tics, the successful-advance nanoseconds and the final stripped
  SHA-256. It reuses `player_level_advance`'s setup; that example may itself serve, if its output is enough.
- A tool, `tools/bench_advance.py`, with two modes:
  - `--local` (IWAD, local only) times the 61 real-map cases with N repetitions in rotated order. It refuses
    to run when the binary or target directory is on `/mnt/*` (P7) and waits for the idle gate. It checks
    every final digest against `tests/reference/m4b_real_results.json` and prints counts and µs per tic only
    (the per-map table of §1).
  - `--ir` (CI) runs the §4 P6 workload set under `valgrind --tool=callgrind --toggle-collect` and compares
    against `tests/reference/m4p_ir_baseline.json` (instructions outside libc and ld.so, call counts, total
    Ir, toolchain and valgrind version). `--check` exits non-zero above the threshold, and `--update` writes
    the baseline.
- TESTING gains the commands, and VALIDATION states what the guard does and does not prove (instruction
  counts, not time).

## 6. What this note could not establish

- The projection assumes that the removed work costs what the edge sums say and that nothing else shifts.
  Layout and inlining will move costs. Each step's measured delta replaces the estimate.
- The µs-per-instruction factor (about 0.10) is an observation from five maps, not a model. E1M5 and E1M8
  depart from it because their median timed script differs from their median-length script.
- Runner-to-runner stability of the CI instruction counts is untested (P6).
- C timings for E1M5, E1M8, E1M9 and DEMO1 were not measured (M4a measured six maps).
- Bytes copied per tic were derived from type sizes (a scratch probe on this build) and live-actor counts,
  not traced. Valgrind's `memcpy` instruction counts do not give byte counts.
- The baseline timing ran on a host whose 1-minute load gate was marginal (§1). The committed harness
  re-records it.
