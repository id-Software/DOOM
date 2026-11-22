# BLOCKMAP corner traversal evidence

Observed on 2026-11-22 Pacific time. All maps and demos were synthetic. The
reference is Linux DOOM 1.10 at `a77dfb96cb91780ca334d0d4cfd86957558007e0`,
run through the existing [M4b whole-engine driver](../../tools/reference/m4b_build.py)
and [M5c sound driver](../../tools/reference/m5c_build.py).

## Direct traces

[P_PathTraverse](../../linuxdoom-1.10/p_maputl.c) nudges starts on block
boundaries by one map unit. The loop visits the current cell and checks the
endpoint before testing the next x or y step. When both step tests fail,
neither coordinate changes. The loop repeats that cell until its
64-iteration limit, then calls `P_TraverseIntercepts`.

The C probe called the genuine function with `PT_ADDLINES` after loading a
map. Rust called its shared slide and use path on the same endpoints in a
wire-24 world. The walks agreed on all 46 inputs. This comparison covered
the nudged start, cell sequence, iteration count and collected line
intercepts as a multiset. The inputs included eight exact crossings
(slopes with magnitudes 1 and 2), 32 one-raw-unit offsets and six starts on
block boundaries. The exact crossings gave these results at the tested
corner.

| Travel direction | Observed path |
| --- | --- |
| Increasing x and y | Repeated the approach cell to the limit |
| Decreasing x and y | Stepped into the x-side cell, then repeated it to the limit |
| Increasing x, decreasing y | Entered the x-side cell, then the diagonal cell and continued |
| Decreasing x, increasing y | Entered the y-side cell, then the diagonal cell and continued |

Some near misses stalled later along the trace. Four additional axis-aligned
direct calls isolated the budget from the corner. With the first cell
numbered zero, both walks reached endpoint cells 62 and 63 in 63 and 64 visits
respectively. With endpoints in cells 64 and 70, both stopped after visiting
cell 63. Endpoint visitation in this check comes from the recorded cells.

## Whole-engine scenes

The strict and instrumented C builds produced byte-identical dumps in all
21 scenes. The unchanged overflow-checked and address-checked C builds also
matched the strict dumps on all 21 scenes, with no runtime diagnostics.
The address build used `ASAN_OPTIONS=detect_leaks=0`, so the run provides
no LeakSanitizer coverage.

- Wires 22–24 matched the strict C dumps frame by frame through their
  independent DSIM projections, 21 scenes per wire. Wire 25 matched the same
  gameplay projections and the genuine M5c sound stream in all 21 scenes,
  with zero sound disagreements. The instrumented Rust runs matched all 296
  trace walks across those 84 runs. The trace comparison included collected
  intercept counts.
- Wires 7–21 were tried on every scene, for 315 setup attempts. Of these, 135
  finished and 14 reached the shot before the position bound refused the tic
  that would step its puff. All 574 traces from these 149 runs matched.
  Gameplay RNG index, actor count and actor positions agreed on all 9,925
  compared tics. These three fields form a limited projection of the earlier
  policies. The other 166 attempts refused at setup under existing geometry,
  door or thing-roster rules. The earlier driver continued after a rejected
  barrel spawn, which compared different rosters. This rerun stops at that
  refusal. [Compatibility rules](../COMPATIBILITY.md#policies).
- The genuine D1 aim and shot each collected 63 intercepts, including 61
  repeated entries for the same barrel. In the U1 use case the walk repeated
  its cell to the limit and missed the door. The corresponding one-unit
  control opened it. The S3 shot hit the far wall from the intercepts it had
  collected before stalling. Its control, C10S, reached the endpoint and hit
  the nearer pillar.
- The mutation that advances both axes on a missing step changed the cell
  sequence and failed the new Rust regression. The level comparators still
  accepted all nine gameplay projections on H1PP, U1 and S3 with that change.
  A second mutation discarded the intercepted wall when the trace stalled.
  All three S3 level comparisons rejected that change at the shot.

The 129th intercept write and an in-game slide into an exact corner were
outside these runs. The direct line calls cover the shared walk used for
sliding and use. In-game use and pistol traces exercise their normal callers.

## Historical endpoint policy

Canonical wires 2–6 and historical profiles 2–6 use different numbering.
The [Rust regression](../../crates/doom-sim/tests/blockmap_corner.rs) checked
both numbering schemes. Canonical wires 2–6 refuse the attack command with
`UnsupportedButtons` and explicit aim and attack with `Combat(Profile)`.
Historical profiles 2–4 refuse the explicit aim and attack with
`Combat(Profile)`. Historical combat and weapon profiles 5 and 6 support the
direct calls but return `Combat(TraversalLimit)` on the stalled S3 trace.
The test compares complete canonical bytes before and after the refusals.
It also checks successful direct aim from the control position.

Historical profile 5's refusal is the documented endpoint requirement. The
original continues to the intercept callbacks for S3. The same regression
pins the original's 64-visit shot and its collected hit on wires 7 and 25.
Source-derived expectations in the test come from the C probe. The
unchanged-state assertions check Rust policy.
[Compatibility policy](../COMPATIBILITY.md#genuine-player-pistol-combat).

## Fixture provenance

[blockmap_corner.wad](../../tests/reference/blockmap_corner.wad) is S3 from
the synthetic scene generator in the
[previous investigation sources](https://github.com/Copgunn/DOOM/tree/14f4246cddcc726b4e6bfb239be15a0a75d28dad/wip/quirks-b-corner).
The generator reuses the public M4a and M4b map builders and uses blank
graphics. S3 has one room, one void pillar and one player start. C10S is
the same map with the player start one map unit west. The rerun relocated
the scratch paths and checked the driver results explicitly. The historical
runner for wires 7–16 places the player through the public body API.
