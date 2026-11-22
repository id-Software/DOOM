# M3h5 — Non-axis (diagonal) walls

This note records the original-C facts, the admitted domain, the wall invariant and its
proof for arbitrary wall angles, the evidence plan and the implementation plan of
`PlayerMonstersDiagonalV1`. It was written and committed before any runtime change.
Support and limits belong to [COMPATIBILITY](COMPATIBILITY.md); DSIM32 belongs to
[STATE_FORMAT](STATE_FORMAT.md); measured observations belong to
[the evidence record](validation/M3H5.md). All references are to the preserved
`linuxdoom-1.10/` sources at `a77dfb96`. Every number is in map units unless it says raw
(16.16 bits). Line numbers name the first line of the cited statement.

## 1. Starting point

`PlayerMonstersHeightsV1` ([M3h4](M3H4_REFERENCE_INVESTIGATION.md)) admits two-sided
lines, static heights and vertical movement inside a walled vertex rectangle `R` whose
four sides are covered by one-sided lines, with **every linedef axis-aligned**
(`validate_heights_geometry`, `player_map_scale.rs:212`). The reason was the slide
projection: M3h3 noted that `P_HitSlideLine` on a non-axis wall can move a player more
than its 32-unit box in one `P_TryMove` (`P_AproxDistance(30,30) = 45`, 22.5° wall:
38.4 units), which breaks the M3h3 argument that a destination box leaving the room
overlaps the start box and therefore contains a boundary point.

The engine below the policies does not share that restriction. `LineGeometry::new`
computes the source `slopetype` (`geometry.rs:57`), `point_side` is `P_PointOnLineSide`
with its shifted products (`geometry.rs:80`), `box_side_bounds` is `P_BoxOnLineSide`
for all four slope types (`geometry.rs:104`), `movement::project` is the full
`P_HitSlideLine` including the angle branch (`movement.rs:441`), `movement::path` is
`P_PathTraverse` with `P_PointOnDivlineSide`, `P_InterceptVector` and the 16-unit
predicate switch (`movement.rs:288–410`), `combat_trace.rs:160` carries the same DDA for
hitscan, `multisector::node_side` is `R_PointOnSide` (`multisector.rs:241`), and
`sight::side`/`fraction` are `P_DivlineSide`/`P_InterceptVector2` (`sight.rs:89,123`).
The M3b fixtures already contain a diagonal two-sided portal with a diagonal node
partition (`generate_m3b.py`, `fixture(..., diagonal=True)`, maps 10–11) and the M3d3a
sight evidence has `sight_diagonal`. What no policy has ever admitted, and what no
evidence has ever observed, is a one-sided diagonal wall, a slide along it, a slide
into a diagonal two-sided line, hitscan and fireballs against diagonal faces, and
monsters moving around diagonals.

M3h5 therefore adds a policy, an admission with a new definition of "closed boundary",
a proof for arbitrary angles with an explicitly stated hole and its exact runtime
witness, and the evidence. It adds no mover, collision, traversal or scheduler code.

## 2. Original C facts

### 2.1 Line setup

- **`P_LoadLineDefs`** ([p_setup.c:357](../linuxdoom-1.10/p_setup.c)): `dx = v2->x -
  v1->x; dy = v2->y - v1->y` (380–381); `slopetype` is `ST_VERTICAL` when `dx == 0`,
  `ST_HORIZONTAL` when `dy == 0`, otherwise `ST_POSITIVE` when `FixedDiv(dy, dx) > 0`
  and `ST_NEGATIVE` otherwise (383–393); the bbox is the min/max of the endpoints
  (395–415). `FixedDiv` saturates by operand sign, so the slope sign is exact.

### 2.2 Side predicates and their truncation

- **`P_PointOnLineSide`** ([p_maputl.c:66](../linuxdoom-1.10/p_maputl.c)): the axis
  cases compare coordinates exactly (76–89). Otherwise `left = FixedMul(line->dy >>
  FRACBITS, x - v1->x)`, `right = FixedMul(y - v1->y, line->dx >> FRACBITS)` and `right
  < left → 0` (front), else 1 (back) (91–99). With integer vertices `dy >> FRACBITS` is
  exact, and `FixedMul` is a 64-bit product shifted right by 16, i.e. a floor. Writing
  `a = dy_u·Δx` and `b = dx_u·Δy` in unit² (whole-unit line delta times the real
  point offset), the code returns front iff `floor(b) < floor(a)`. The exact cross
  product `cross = a − b` is `L` times the signed perpendicular distance in front of
  the line, with `L` the Euclidean line length. Consequences, used by the proof:
  - a point strictly behind or exactly on the line (`cross ≤ 0`) always reads back;
  - a point in front with `cross ≥ 1` unit² always reads front;
  - a point in front with `0 < cross < 1` reads front or back depending on the
    fractional parts (the "sliver", perpendicular width `1/L` units, at most 0.71 for
    a `(1,1)` wall and 0.00065 for a 1,536-unit wall).
- **`P_BoxOnLineSide`** ([p_maputl.c:110](../linuxdoom-1.10/p_maputl.c)): horizontal and
  vertical lines compare the box edges exactly (119–137): a box touching an axis wall
  is on one side. `ST_POSITIVE` tests the corners (left, top) and (right, bottom),
  `ST_NEGATIVE` (right, top) and (left, bottom) (139–147): the two corners extreme along
  the line's normal. −1 iff they read differently (150–152). The line is infinite.
- **`PIT_CheckLine`** ([p_map.c:189](../linuxdoom-1.10/p_map.c)): skip unless the
  destination box strictly overlaps the line's closed bbox (191–195, `<=`/`>=`); skip
  unless `P_BoxOnLineSide == −1` (197); then `!ld->backsector → false` (211). So a
  one-sided line blocks a destination exactly when the open box strictly overlaps its
  bbox **and** the infinite line is read as crossing the box. Two observable quirks of
  diagonal walls follow from §2.2 and are reproduced, never corrected: a box whose
  corner lies exactly on a diagonal wall is blocked from the wall's front side (the
  corner reads back, the opposite corner front) but not from its back side; and a box
  approaching from the back side may be accepted with its leading corner up to `1/L`
  units past the line (`cross < 1`). Axis walls have neither effect.
- **`P_CheckPosition`** ([p_map.c:375](../linuxdoom-1.10/p_map.c)): lines are visited in
  the cells of the exact box, `xl = (tmbbox[BOXLEFT] − bmaporgx) >> MAPBLOCKSHIFT` …
  (431–439), without the `MAXRADIUS` extension used for things (420–423).
  `P_BlockLinesIterator` ([p_maputl.c:472](../linuxdoom-1.10/p_maputl.c)) returns true
  for a cell outside the grid (481–486) and visits each line once per `validcount`
  (497–500).

### 2.3 Sliding

- **`P_XYMovement`** ([p_mobj.c:114](../linuxdoom-1.10/p_mobj.c)): clamp to `±MAXMOVE`
  (137–146); `xmove > MAXMOVE/2 || ymove > MAXMOVE/2` halves the attempt (152–161, only
  positive components); `P_TryMove` (166); a blocked player calls `P_SlideMove(mo)`
  (171) with the **full clamped momentum**, not the halved attempt; the loop then
  continues with the remaining halves (191).
- **`P_SlideMove`** ([p_map.c:695](../linuxdoom-1.10/p_map.c)): three corner traces of
  length `mom` (714–743), each stopped by its own first blocking line
  (`PTR_SlideTraverse` returns false, 681); `bestslidefrac` is the minimum over the three
  traces (673–678). No blocker → stairstep (746–752, `y` then `x`). Flush move by
  `bestslidefrac − 0x800` when positive (756–764). Remainder `FRACUNIT − (bestslidefrac
  + 0x800)` capped at `FRACUNIT` (768–774); `tmxmove = FixedMul(momx, remainder)`
  (776–777); `P_HitSlideLine` (779); `mo->momx = tmxmove` (781–782); `P_TryMove`
  (784), on failure `retry` with the projected momentum, the third entry goes to the
  stairstep (709).
- **`PTR_SlideTraverse`** ([p_map.c:636](../linuxdoom-1.10/p_map.c)): a one-sided line
  is ignored when the mover is on its back side by `P_PointOnLineSide` (645–651),
  otherwise blocks; a two-sided line blocks on `openrange < height`, `opentop − z <
  height`, `openbottom − z > 24` (656–665) regardless of `ML_BLOCKING`.
- **`P_HitSlideLine`** ([p_map.c:584](../linuxdoom-1.10/p_map.c)): `ST_HORIZONTAL →
  tmymove = 0`, `ST_VERTICAL → tmxmove = 0` (596–606). Otherwise `side =
  P_PointOnLineSide(slidemo)`; `lineangle = R_PointToAngle2(0,0, dx, dy)`, plus `ANG180`
  on the back side (608–613); `moveangle = R_PointToAngle2(0,0, tmxmove, tmymove)`;
  `deltaangle = moveangle − lineangle`, plus `ANG180` when above `ANG180` (615–619);
  both shifted to fine angles (622–623); `movelen = P_AproxDistance(tmxmove, tmymove)`;
  `newlen = FixedMul(movelen, finecosine[deltaangle])`; `tmxmove = FixedMul(newlen,
  finecosine[lineangle])`, `tmymove = FixedMul(newlen, finesine[lineangle])` (625–629).
  `newlen` may be negative (delta beyond 90°), which reverses the direction along the
  line. `P_AproxDistance` ([p_maputl.c:49](../linuxdoom-1.10/p_maputl.c)) is `dx + dy −
  min/2`, so `(30,30) → 45`. `finesine[2048] = 65535` ([tables.c:841](../linuxdoom-1.10/tables.c)),
  so `|newlen| < 45` strictly. `R_PointToAngle`
  ([r_main.c:292](../linuxdoom-1.10/r_main.c)) uses `SlopeDiv`
  ([tables.c:50](../linuxdoom-1.10/tables.c)), `(num << 3) / (den >> 8)` in `unsigned`.
- **Exact bound of the projected step.** Over the clamped domain `|tmxmove|, |tmymove|
  ≤ 30` and every fine line angle, a replica of the three bodies above in exact 32-bit
  fixed point gives a largest per-axis component of **38.409 units** (line direction
  fine angle 2557, i.e. a 22.5°-class line, `tmxmove = 30, tmymove = −30`) and a
  largest length of 44.999 units (fine angle 3066, a 45°-class line). Per-axis
  components exceed 30 for every diagonal line direction except those within about
  0.3° of an axis or of 45°, and exceed 32 for most of them. This is the quantity the
  M3h3 note estimated; §4.4 shows that it is a real hole, not a slack bound.

### 2.4 Traversal against diagonal lines

- **`PIT_AddLineIntercepts`** ([p_maputl.c:562](../linuxdoom-1.10/p_maputl.c)): when any
  trace component exceeds 16 units, the line's endpoints are classified against the
  trace with `P_PointOnDivlineSide` ([p_maputl.c:161](../linuxdoom-1.10/p_maputl.c)),
  whose products are `FixedMul(dy >> 8, dx >> 8)` (197–198) with a sign-bit shortcut
  (190–195); otherwise the trace's endpoints are classified against the line with
  `P_PointOnLineSide` (570–582). Equal sides → not crossed (584); `frac =
  P_InterceptVector(&trace, &dl)` (588–589, [p_maputl.c:231](../linuxdoom-1.10/p_maputl.c):
  `den = FixedMul(v1.dy >> 8, v2.dx) − FixedMul(v1.dx >> 8, v2.dy)`, parallel → 0);
  `frac < 0 → behind` (591); `earlyout` on a one-sided line below `FRACUNIT` (595–599).
  A slide corner trace that starts in a diagonal wall's sliver can therefore read its
  two ends on different sides of a line it runs parallel to, and the parallel intercept
  returns `frac = 0`: the wall is then the slide line with fraction 0. The Rust
  `path`/`intercept` reproduce this.
- **`P_PathTraverse`** ([p_maputl.c:743](../linuxdoom-1.10/p_maputl.c)): the `FRACUNIT`
  nudge off cell boundaries (777–781), the DDA that steps one axis per iteration
  (866–875) and the 64-iteration cap (846). A line listed only in a skipped
  cell is missed. The exact-corner probes on 2026-11-22 corrected the earlier
  two-cell account. Travel direction and the fixed-point intercepts govern the
  next step. If neither axis test holds, the loop revisits the same cell to the
  cap and processes the intercepts already collected.
  [Genuine-engine corner evidence](validation/BLOCKMAP_CORNER.md#direct-traces).
- **`PIT_AddThingIntercepts`** ([p_maputl.c:616](../linuxdoom-1.10/p_maputl.c)) and
  **`P_TraverseIntercepts`** ([p_maputl.c:683](../linuxdoom-1.10/p_maputl.c)) do not read
  line angles. `PTR_AimTraverse` ([p_map.c:816](../linuxdoom-1.10/p_map.c)) and
  `PTR_ShootTraverse` ([p_map.c:899](../linuxdoom-1.10/p_map.c)) use only `frac`,
  `P_LineOpening` and the front/back sectors (M3h4 §2.6); the puff sits 4 units before
  the intercept along the trace, which for a diagonal wall is a point inside the room at
  a perpendicular distance of `4·|sin(angle between trace and wall)|`.

### 2.5 Sight, missiles, monsters

- **`P_CrossSubsector`** ([p_sight.c:135](../linuxdoom-1.10/p_sight.c)) classifies seg
  endpoints with `P_DivlineSide` ([p_sight.c:55](../linuxdoom-1.10/p_sight.c)) and
  `P_InterceptVector2` ([p_sight.c:109](../linuxdoom-1.10/p_sight.c)); `R_PointOnSide`
  ([r_main.c:162](../linuxdoom-1.10/r_main.c)) descends the BSP by the node partition's
  side. All are angle-general and already in Rust with diagonal evidence (M3b, M3d3a).
  With diagonal walls the BSP must carry diagonal partitions so that
  `R_PointInSubsector` is right; the harness builder makes them (§6.1).
- **Missiles**: `P_XYMovement` explodes a blocked missile at its unchanged position
  (p_mobj.c 186); the impact position is the position before the refused step, so a
  fireball's blast against a diagonal wall sits wherever its last accepted 10-unit step
  ended, up to 10 units from the wall along its path. `P_RadiusAttack`
  ([p_map.c:1206](../linuxdoom-1.10/p_map.c)) and `PIT_RadiusAttack` use XY distance only,
  without line of sight (the Imp fireball has no explosion damage; `MT_TROOPSHOT`
  explodes with `P_ExplodeMissile` only).
- **`P_Move`** ([p_enemy.c:272](../linuxdoom-1.10/p_enemy.c)), `P_TryWalk` (349),
  `P_NewChaseDir` (363): one `P_TryMove` per attempt of at most `speed` per axis, and no
  reading of line angles. A monster blocked by a diagonal wall cycles directions as it
  does at an axis wall; with eight compass directions it walks along a 45° wall as a
  staircase of diagonal and axis steps. `P_NewChaseDir` reads `numspechit == 0` after a
  blocked move (p_enemy.c 310), unchanged.

### 2.6 Constants

`MAXMOVE 30`, `MAXRADIUS 32`, `FRACUNIT 65536` (p_local.h), `ANGLETOFINESHIFT 19`,
`FINEANGLES 8192` (tables.h), `MAPBLOCKSHIFT 23`, `MAPBLOCKSIZE 128·FRACUNIT`
(p_local.h). `P_InterceptVector` and `P_PointOnLineSide` operate on `fixed_t`; the M3h3
arithmetic table (`G = A = 2048`, line norm `|dx| + |dy| ≤ 1536`) does not depend on
the line angle and is reused unchanged (§4.6).

## 3. What exists and what changes (Rust inventory)

| Source body | Rust | Status for M3h5 |
| --- | --- | --- |
| `P_LoadLineDefs` slopetype, bbox | `LineGeometry::new` (`geometry.rs:47`) | Present. No change. |
| `P_PointOnLineSide`, `P_BoxOnLineSide`, bbox skip | `point_side`, `box_side_bounds`, `crosses` (`geometry.rs:80–125`) | Present, all slope types. No change. |
| `PIT_CheckLine`, `P_CheckPosition` cells | `check_validated_position`, `multisector::check_move_from_things` | Present. No change. |
| `P_HitSlideLine` (angle branch), `R_PointToAngle2`, `SlopeDiv`, `P_AproxDistance` | `movement::project`, `point_angle`, `slope_div` (`movement.rs:412–464`) | Present, never admitted. **Add** the parity witness after the projected move (§4.5). |
| `P_SlideMove`, `PTR_SlideTraverse` | `Mover::slide`, `multisector::slide_blocks` | Present. No change. |
| `P_PathTraverse`, `PIT_AddLineIntercepts`, `P_PointOnDivlineSide`, `P_InterceptVector` | `movement::path`, `div_side`, `intercept`; `combat_trace.rs` DDA | Present. No change. |
| `PTR_AimTraverse`, `PTR_ShootTraverse`, puff placement | `combat_trace::aim_trace`, `hit_trace` | Present. No change. |
| `P_CrossSubsector`, `P_DivlineSide`, `R_PointOnSide` | `sight.rs`, `multisector::node_side` | Present with diagonal evidence. No change. |
| `P_Move`, `P_TryWalk`, `P_NewChaseDir` | `pursuit.rs` | Present. No change. |
| Admission | `validate_heights_geometry`, `RoomBounds::derive_for` (`player_map_scale.rs:122,212`) | **Add** `validate_diagonal_geometry`, `Boundary` (the one-sided chain), the parity test, closed-cell BLOCKMAP coverage, the spawn box test (§4.1). Older policies keep both existing functions unchanged. |
| Spawn admission | `spawn_admitted` (`player_map_scale.rs:294`) | **Widen** for this policy: parity plus "closed box meets no one-sided segment" instead of "closed box inside `R`". |
| Per-tic witness | `travel_admitted`, `position_admitted` | Unchanged (`R` is still implied, §4.3). |
| Policy plumbing | `original_registry.rs` (variant, wire 16, every `matches!` list, `with_original_registry`), `player_monsters.rs` (schema, `original_player_diagonal_map`) | **Add** the variant; `has_heights` includes it; new `has_diagonals`. |
| DSIM | `snapshot.rs`, `diagnostic.rs` | **Add** version 32 (§5). |

## 4. The admitted domain and the proof

`PlayerMonstersDiagonalV1` (wire 16, context `PLAYER-MONSTERS-DIAGONAL-1-MEDIUM-v1`,
DSIM32) is a superset of `PlayerMonstersHeightsV1`: same roster, weapons, items, drops,
fireballs, infighting, statistics, 320 slots, 208 live fireballs, 95 linedefs, 256
sectors, ±512 heights, the 2,048-unit extent and the 4,096-unit momentum bound. It
changes only the linedef admission, the boundary definition, the spawn admission and one
witness, for itself. Every older policy keeps its admission, behavior, digests and
bytes: `validate_heights_geometry` and `RoomBounds::derive_for` are not touched.

### 4.1 Geometry admitted at load (checked once, before construction)

Let `S` be the set of one-sided linedefs taken as closed segments with integer
endpoints, and let `R` be the closed vertex rectangle as before.

1. Inherited: everything of M3h4 §4.1 items 1, 3, 5 (caps, flag sets, front side
   present, `ML_TWOSIDED` consistency, no specials, no sky, heights within ±512, line
   norm `|dx| + |dy| ≤ 1,536`), with two replacements below. Lines may have any
   direction; degenerate lines (`dx = dy = 0`) reject.
2. **Closed boundary** (replaces the walled rectangle, M3h4 item 4), three checks on `S`:
   - **C1, endpoint-only contacts**: two segments of `S` intersect only at a shared
     endpoint (exact integer tests; overlapping collinear segments, crossings and
     T-junctions reject).
   - **C2, even degree**: every vertex is an endpoint of an even number of segments of
     `S` (two at a corner, four where two loops touch). With C1 this makes `S` a mod-2
     cycle, so the **parity** of a point `p` not on `S` — the number of segments of `S`
     crossed by the horizontal ray `{(x, p.y) : x > p.x}` under the half-open rule
     (`a.y > p.y ≠ b.y > p.y`, crossing abscissa `> p.x`, computed exactly in `i64`) —
     is constant on every connected component of the plane minus `S`, flips across every
     segment, and is even far away. Define `Ω = {p : parity(p) odd}`. `Ω ⊂ R`.
   - **C3, orientation**: for every segment of `S`, the points just in front of its
     midpoint (the sidedef-0 side, `P_PointOnLineSide = 0`) have odd parity. Tested
     exactly in doubled coordinates: the horizontal ray from the midpoint counted over
     the other segments as the limit toward the front side (a vertex exactly at the
     ray's height counts as above it when `dx > 0`, below it otherwise), plus the
     segment itself when that ray crosses it (`dy < 0`). By C1 no other segment passes
     through the midpoint.
     A one-sided wall whose sector side is its back side rejects. Real maps satisfy C3
     because a one-sided line's only sidedef faces its sector.
3. **BLOCKMAP** (replaces M3h4 item 5): the grid covers `R` (origin at or below the
   minimum, maximum strictly inside) and every linedef, one- or two-sided, is listed in
   every cell whose **closed 128-unit square meets the segment** (exact Liang–Barsky
   clipping in `i64`). This is weaker than the M3h4 bbox rule, which stays for the heights
   policy, and is what the proof needs: for every point `q` of a wall, the cell containing
   `q` lists the wall. Real BLOCKMAPs list a line in the cells it passes through, so a
   long 45° wall no longer needs the `n²` cells of its bbox.
4. **Spawns** (replaces M3h4 item 7's box test): for every THING, the player body, every
   controlled dropped pickup and every controlled spawn, the center has odd parity and
   the closed box `[x−r, x+r] × [y−r, y+r]` meets no segment of `S`; the height fit of
   M3h4 §4.3 is unchanged.
5. Two-sided lines are unconstrained in direction and may touch, cross or end on walls;
   nothing in the proof reads them.

### 4.2 The reading lemma

For a one-sided segment `W` with Euclidean length `L` and a destination open box `B`
that contains a point of `W` in its interior:

- the bbox test passes (the point is in the open box and in the closed bbox);
- the infinite line of `W` passes through the interior of `B`, so the two corners
  tested by `P_BoxOnLineSide` lie strictly on opposite sides;
- the corner behind the line reads back; the corner in front reads front iff its
  `cross ≥ 1` (§2.2);

hence **`PIT_CheckLine` blocks `B` unless the front corner's `cross` is in `(0, 1)`**, in
which case `B` is accepted and every point of `B` lies within `1/L` units of the closed
back half-plane of `W` ("`B` is almost entirely behind `W`"). For an axis-aligned `W`
the test is exact and `B` is always blocked.

### 4.3 Invariant and proof

**Invariant Ω**: every XY mover (player, monsters, corpses, fireballs) has its center in
`Ω`, and its open box contains no point of `S`. Every other actor (items, impacts,
exploded fireballs) has its center in `R`.

**Lemma A (no partial penetration).** If a box `B` with center `c ∈ Ω` is accepted by
`P_CheckPosition` (or is a spawn box under item 4), then `B` contains no point of `S`,
hence `B ⊂ Ω`.
*Proof.* Spawns: item 4 directly. Suppose `q ∈ W ∩ B`, `W ∈ S`; choose `q` not a vertex
and not collinear with `c` and any vertex (`W ∩ B` is a relatively open piece of `W`, the
excluded directions are finitely many). Let `q′` be a point of `B` just behind `q`:
`q′ ∉ Ω` by C3. The segment `c → q′` lies in `B` (convex) and changes parity, so it has
a first point `q₂` of `S`, on some `W₂ ∈ S`, with `c` strictly on the front side of `W₂`
(C3 and the choice of `q`). `q₂ ∈ B`, so by the reading lemma `B` is almost entirely
behind `W₂`: `cross_{W₂}(c) < 1`. But the front corner of `B` with respect to `W₂` is
`r·(|cos θ| + |sin θ|) ≥ r ≥ 6` units further in front, so its `cross ≥ 6·L ≥ 6 > 1`
and it reads front; the back corner reads back; `P_BoxOnLineSide = −1` and `B` was
blocked. Contradiction. Connectedness then gives `B ⊂ Ω`. ∎

**Theorem B (step bound).** Let a mover with center `c₀ ∈ Ω` attempt the destination
`c₁ = c₀ + Δ` with `|Δx|, |Δy| ≤ 2r − 2` and let `P_TryMove` accept it. Then `c₁ ∈ Ω`.
*Proof.* Let `B₀`, `B₁` be the open boxes. By Lemma A, `B₀ ⊂ Ω`. The overlap `P = B₀ ∩
B₁` is an open box of half-widths `r − |Δx|/2 ≥ 1` and `r − |Δy|/2 ≥ 1` around the
midpoint `m`, and `P ⊂ Ω`.
*Step 1: `B₁` contains no point of `S`.* Suppose `q ∈ W ∩ B₁`, `q′` just behind `q`
(`q′ ∈ B₁`, `q′ ∉ Ω`). For any generic point `y ∈ B₁ ∩ Ω`, the segment `y → q′` lies in
`B₁` and has a first point of `S` on some `W_y ∈ S` meeting `B₁`, with `y` strictly in
front of `W_y`; by the reading lemma `B₁` is almost entirely behind `W_y`, so `0 <
cross_{W_y}(y) < 1`, i.e. `y` lies in the front sliver of `W_y`, at perpendicular
distance below `1/L_{W_y} ≤ 1/√2` from its line. So `P ⊂ Ω` is covered, up to a null
set, by the front slivers of walls meeting `B₁`. Take a generic `ỹ ∈ P` within 0.1 of
`m`, in the sliver of some `W_i`, and `n_i` the unit front normal of `W_i`. The point
`ỹ + 0.75·n_i` is still in `P` (`|n_i|_∞ ≤ 1`, `0.1 + 0.75 < 1`) and lies `0.75 > 1/√2
≥ 1/L_i` in front of `W_i`'s line, contradicting "`B₁` is almost entirely behind `W_i`".
*Step 2.* `B₀ ∪ B₁` is connected (they overlap), contains no point of `S`, and contains
`c₀ ∈ Ω`; `c₁ ∈ B₁`, so `c₁ ∈ Ω`. ∎

**Blocked moves** leave the position unchanged (`P_TryMove` returns before `x`/`y`
are set); a blocked missile explodes where it is; a blocked non-player zeroes momentum.
So Invariant Ω is preserved by every `P_TryMove` whose step satisfies the bound:

| Mover | Radius, `2r − 2` | Step per attempt | Source |
| --- | ---: | ---: | --- |
| Player, ordinary attempt | 16, 30 | ≤ 30 (clamp; positive halves ≤ 15, whole negative components ≤ 30) | `P_XYMovement` 137–166 |
| Player, slide flush move | 16, 30 | `abs(mom)·(bestslidefrac − 0x800) ≤ 30` | `P_SlideMove` 756–764 |
| Player, stairstep | 16, 30 | ≤ 30 per axis, one axis at a time | 749–752 |
| **Player, projected slide move** | 16, 30 | **up to 38.41 per axis (§2.3)** | 776–784 |
| Former humans, Imp (walk) | 20, 38 | ≤ 8 | `P_Move` |
| Demon (walk) | 30, 58 | ≤ 10 | `P_Move` |
| Monsters, corpses (thrust) | 20/30, 38/58 | ≤ 30 after the clamp | `P_XYMovement` |
| Fireball | 6, 10 | 5 (half-step), 10 per tic | `P_CheckMissileSpawn`, `P_XYMovement` |

Spawns: the fireball is spawned at the Imp's center with radius 6 inside the Imp's box
(radius 20), so its box contains no point of `S`; a drop at a corpse's center with radius
20 inside the corpse's box (radius 20 or 30); the half-stepped exploded fireball keeps a
center within 5 units of the Imp's center, inside `R`. Puffs sit on the trace between the
shooter and a wall point (both in `R`, convex), blood at a thing hit, as in M3h3. Items
never move. Invariant Ω therefore holds after every spawn, every public controlled
operation (damage adds momentum, not position) and every `P_TryMove` **except the
projected slide move of a player when `|tmxmove|` or `|tmymove|` exceeds 30**, which
Theorem B does not cover.

### 4.4 The hole is real: a corner jump

The projected move is not a slack bound. Take a horizontal one-sided wall `W_h` along
`y = 0` from `x = −1024` to `0` with the room below it, and a one-sided wall `W_d` from
`(0, 0)` toward `(−383, −924)` (67.5° from the x axis) with the room on its right; the
room is the 67.5° wedge between them. A player with momentum `(30, 30)` whose box
nearly touches `W_d` (center `(−31, −20)`, right edge 0.08 units from the wall at the
box's bottom) attempts the halved step `(15, 15)`, is blocked by both walls, and slides:
the third corner trace meets `W_d` at fraction 0.0045, so `W_d` is the slide line, the
flush move is skipped (`0.0045 − 0x800/FRACUNIT < 0`), the remainder is 0.9955 and
`P_HitSlideLine` projects `(29.87, 29.87)` onto `W_d`'s direction: `lineangle = 247.5°`,
`deltaangle = 157.5°`, `newlen = −40.5`, giving the step `(15.82, 38.24)`. The destination
center `(−15.18, 18.24)` has its box bottom at `y = 2.24`, above `W_h`; neither wall's
bbox strictly overlaps the destination box, so `P_TryMove` accepts it and the player
stands outside the room (checked with an exact replica of the four bodies). Natural play
of the admitted roster tops out near 24 units of momentum (running plus strafing) plus a
few units per hit, so this needs the controlled impulse API or stacked knockback; it is
reachable in the admitted domain and is observed in C in scenario `corner_jump` (§6.1).
In the source the player then roams the void: one-sided lines block from both sides and
`PTR_SlideTraverse` ignores them from the back, so the player cannot return except by the
same jump.

### 4.5 What the policy does about it

The policy keeps every move of the source. After each projected slide move that
`P_TryMove` accepts (the `MotionEvent::Slide` path of `Mover::slide`), it evaluates the
player's center parity against `S` exactly (at most 95 segments, a few hundred `i64`
operations, only when a slide projection happens). If the parity is even — the center
crossed the boundary — the tic fails with a new checked error `MapScaleError::Escape`
and the complete canonical state is preserved, like every other recoverable error. By
Theorem B this witness never fires for a projected step within 30 units per axis, and by
Lemma A, whenever it does not fire the box is entirely inside `Ω`, so Invariant Ω is true
of every committed state. The exact statement:

- **Proven**: Invariant Ω for every spawn, every controlled operation, and every
  `P_TryMove` with a step of at most `2r − 2` per axis, which is every move of every
  mover except a player's projected slide move above 30 units per axis.
- **Rejected at runtime**: a player's projected slide move that leaves `Ω`, detected
  exactly (never a false positive: the parity test is exact integer arithmetic; never a
  miss: leaving `Ω` is a parity change). The source would accept that move; this is the
  one legal source move the policy refuses, and it is the `corner_jump` observation.
- **Not rejected**: a projected slide move above 30 units per axis that stays in `Ω`
  (the common case, a long slide along a shallow wall); and a jump across a wall into
  another odd region (through a fin thinner than the step), which Lemma A shows still
  satisfies Invariant Ω.

The alternative — admitting the move and relying on the rectangle witness when the
player finally walks out of `R` — would also keep every arithmetic bound (the void inside
`R` is within the sight and intercept domains), but every later observation would be of a
player in the void, which no evidence can validate; it is listed as an owner option
(§8.1).

### 4.6 Arithmetic

The M3h3 table bounds `P_InterceptVector`, `P_PointOnLineSide`, `P_DivlineSide`,
`P_AproxDistance`, `SlopeDiv` and the DDA with `G = A = 2048` and `|dx| + |dy| ≤ 1,536`
without using the line angle. `P_HitSlideLine` adds `R_PointToAngle2(0, 0, dx, dy)` with
`|dx|, |dy| ≤ 1,536` units (`SlopeDiv` of raw values `< 2^27 · 8 < 2^31`) and
`R_PointToAngle2(0, 0, tmxmove, tmymove)` with components ≤ 30; the `FixedMul`s are
within `45 × 65536`. The new admission checks (parity rays, segment intersections,
cell clipping) run in `i64` on integer vertices and raw fixed-point centers; the largest
product is a 2,048-unit coordinate difference in raw units (`2^27`) times a 1,536-unit
delta (`2^11`), far inside `i64`. `Ω ⊂ R`, so the per-tic witnesses of M3h3 (center in
`R`, momentum ≤ 4,096, the M3h4 vertical witnesses) stay as they are and never fail.

### 4.7 What stays rejected at load

Everything of M3h4 §4.6 except the axis-alignment rule; plus: a one-sided wall facing
away from its room (C3), a dangling or T-junction wall (C1, C2), crossing or overlapping
one-sided walls (C1), a BLOCKMAP missing a line in a cell it passes, a spawn whose closed
box meets a wall or whose center is outside `Ω`, degenerate lines. Doors, lifts,
crushers, moving sectors, specials, teleporters, damaging floors, sky, view bob and every
cap increase stay out.

## 5. DSIM32 and wire 16

DSIM32 is the DSIM31 field layout and event vocabulary under version 32, policy 16 and
the new context. Diagonal walls add no actor or sector state: the mover's position,
momentum and location fields already encode everything the slide produces, and the
slide event (`Event::Slide`, tag 9) already carries the line index. **No
persistent field is added; nothing about the wire changes except the version and policy
numbers**, which is what "wire 16" means here. Readers: `Diagnostic::decode_diagonal`
reads 32 only; every earlier reader rejects 32; under 32 the item and vertical checks are
those of 31. `MapScaleError::Escape` is a checked error, not an event.

## 6. Evidence plan

### 6.1 C harness

`tools/reference/m3h5_reference.py` inherits the M3h4 harness (M3h2 bodies, observers,
controlled inputs) and replaces the geometry and cases. `m3h5_geometry.py` builds rooms
from explicit polygons: each sector is a list of integer vertex rings (the outer ring
clockwise, holes counterclockwise, so its walls have it on their right/front side); an
edge shared by two sectors in opposite directions becomes one two-sided linedef with the
caller's extra flags (front = the sector listed first); every other edge is a one-sided
`ML_BLOCKING` wall split at `segment` units (1,024 by default) at exact integer points;
segs follow the source side convention; a BSP is built by a recursive splitter over an
**explicit partition tree** naming linedefs by their endpoints (seg splits are computed
exactly and asserted to land on integer coordinates; collinear segs go to the side their
direction faces); every leaf is asserted non-empty, convex and single-sector, which is why
the concave wedge carries one partition on its 45° wall and the pillar room seven on the
pillar's faces; REJECT is zero; the BLOCKMAP lists every line in every cell whose closed
square meets it (an exact Liang–Barsky clip, the admission rule of §4.1 item 3), with
the origin at the vertex minimum and line 0 listed only by each cell's leading 0 as in
every earlier builder. The adapter installs sector line lists, sidedef sectors, segs,
node partitions and REJECT as in M3h4, plus the leaf-to-sector table (the inherited
loader gives every sector one subsector; the M3h5 harness patches the generated C so
`numsubsectors` and `subsectors[k].sector` follow the BSP leaves). The push input is one
driver line before the tic's command stage: `if (t == push_at) { body->momx = push_x *
FRACUNIT; body->momy = push_y * FRACUNIT; }`, declared harness-only in the contract.

Rooms (`m3h5_cases.py`; floor 0, ceiling 128 unless stated; coordinates in map units):

| Room | Geometry | Purpose |
| --- | --- | --- |
| wedge | one sector, eleven vertices from `(−1024, 0)` clockwise: axis walls, a 45° wall `(−256, 768)→(256, 256)`, a horizontal wall meeting it at the concave corner `(256, 256)`, a convex 45°/45° tip at `(1024, 0)`, a 12:5 (22.6°) wall `(768, −512)→(288, −712)` and a 5:1 (11.3°) wall `(288, −712)→(−672, −904)` split in two | wall slides at 45°, 22.6°, 11.3°, into the convex tip and along the concave corner; fireball into the 45° wall; hitscan puff on the 12:5 wall |
| pillar | 1,024-square room with an octagonal one-sided pillar of half-width 128 (axis faces 128 long, 45° faces) | sight past the pillar; zombie and Imp woken by noise and pursuing around it |
| corridor | rooms `[−1024, −384] × [−768, −128]` and `[384, 1024] × [128, 768]` joined by a 45° corridor between `y = x + 96` and `y = x − 96` (192 units along the axes), three sectors, two vertical two-sided lines | Demon pursuit along the corridor, player slides along both corridor walls |
| step | `[−384, 384]²` cut by the diagonal `(−384, −384)→(384, 384)` into floor 0 and floor 48 | slide along a diagonal riser (reason 5 refusals), sight and hitscan across it (lower-wall puff) |
| window | `[−384, 384]²` cut by `y = x − 192` and `y = x + 192` into a floor-0 room, a `[40, 88]` band and a 64-unit step | slides into both window lines (reason 3), upper-wall puff from the step, autoaim up and down across the opening, noise waking a monster facing away |
| corner | the 67.5°/horizontal wedge of §4.4, closed by walls at `y = −924` and `x = −1024` (segment 1,536 so the 67.5° wall is one line) | the corner jump with the controlled `(30, 30)` push, and the same start running into the corner without it |

Scenarios (22): `slide_45`, `slide_22`, `slide_shallow`, `convex_corner`,
`concave_corner`, `fireball_diagonal_wall`, `puff_diagonal_wall`, `sight_past_pillar`,
`zombie_around_pillar`, `imp_around_pillar`, `corridor_pursuit`, `corridor_slide_east`,
`corridor_slide_west`, `riser_slide_puff`, `window_slide_low`, `window_slide_high`,
`window_upper_puff`, `autoaim_up_window`, `autoaim_down_window`,
`noise_diagonal_window`, `corner_jump`, `corner_no_jump`. Captures as in M3h4 plus every
slide row (`SL`), every blocked move and the push tic. The contract also counts
`player_void_captures`: captured player positions whose center has even ray parity
against the room rings, computed by the script from the trace (7, all in `corner_jump`
after its first tic). The Rust replay applies the push with `Simulation::set_momentum`.

### 6.2 Independent DSIM32

`tools/reference/diagonal_snapshot.py` extends `heights_snapshot.py` without editing it:
version 32, policy 16, the new context. `m3h5_snapshot_goldens.py` writes
`m3h5_snapshots.csv`, `m3h5_events.dref` and selected full-byte fixtures before the Rust
change.

### 6.3 Rust-only evidence

A test-only `diagonal_room.rs` patches lumps of the committed rooms so rejection variants
exist without new bytes: a one-sided wall reversed (C3), a wall removed (C2), a wall
moved to cross another (C1), a BLOCKMAP entry removed from a crossed cell, a THING
whose box crosses a diagonal wall, a THING in the void, a diagonal line with a forbidden
flag, and the M3h4 variants (specials, sky, heights, inconsistent `ML_TWOSIDED`). Every
rejection is tested for unchanged state. The sliver quirk (touching blocked from the
front, sub-unit penetration accepted from the back) is asserted against `LineGeometry`
directly with the §2.2 numbers. The `corner_jump` replay asserts the exact tic at which
`MapScaleError::Escape` is returned and that the state before it is byte-identical after
the failure. Soaks: wedge and pillar with the full roster for 1,200 tics on four seeds
asserting Invariant Ω (parity of every mover) after every tic. Fuzz target
`player_diagonal` on the committed rooms with the M3h3 progress property plus the parity
assertion; seeds from `tools/seed_player_diagonal_fuzz.py`.

### 6.4 Measurements

Matched release timings of `5e711be` and the branch head for every existing `--measure`
workload (digests compared) plus `wedge_slide`, `pillar_chase` and `corridor_pursuit`
from the new example, eleven interleaved process pairs per workload, alone on the idle
host.

## 7. Implementation plan

Each step is a separate commit. Commits 1–3 land before the runtime change.

1. **This note.**
2. **C evidence**: `tools/reference/m3h5_geometry.py`, `m3h5_cases.py`,
   `m3h5_reference.py`; generated `tests/reference/m3h5_trace.dref`, `m3h5_cases.csv`,
   `m3h5_geometry_*.bin`, `m3h5_maps.rs`, `m3h5_contract.json`; strict and ASan/UBSan
   builds compared.
3. **DSIM32 projection**: `tools/reference/diagonal_snapshot.py`,
   `m3h5_snapshot_goldens.py` and their outputs.
4. **Admission** (`crates/doom-sim/src/player_map_scale.rs`): `Boundary` (segments of
   `S`, C1–C3 checks, `parity`, `box_meets_wall`), `validate_diagonal_geometry`,
   `RoomBounds::derive_diagonal` (vertex rectangle, closed-cell BLOCKMAP rule),
   `MapScaleError::{Boundary, Escape}`; `original_registry.rs` (variant, wire 16,
   `has_diagonals`, `has_heights` includes it, `with_original_registry` branches);
   `player_monsters.rs` (schema, `original_player_diagonal_map`); `spawning.rs`,
   `world_pickups.rs`, `original_player.rs` spawn admission through
   `Simulation::spawn_admitted`; `lib.rs` exports.
5. **Escape witness**: `movement.rs` (`MotionAccess` gains a post-slide hook, or the
   `Mover` carries the boundary) so the projected move's acceptance is followed by the
   parity test under this policy; the error propagates through the fallible
   `move_xy_profile` path that the pickup contacts already use.
6. **DSIM32**: `snapshot.rs` (version 32, every `21..=31` range), `diagnostic.rs`
   (`decode_diagonal`, `policy`, bounds), `STATE_FORMAT.md`.
7. **Tests and tools**: `diagonal_room.rs`, `player_diagonal_tests.rs`,
   `tests/support/m3h5.rs`, `examples/player_diagonal.rs` (asserted replay plus
   `--measure`), `tools/reference/check_player_diagonal.py`,
   `fuzz/fuzz_targets/player_diagonal.rs`, `tools/seed_player_diagonal_fuzz.py`, CI steps.
8. **Follow-up 1 (CI headroom)**, §8.2.
9. **Follow-up 2 (allocation)**, §8.3.
10. **Docs**: README, ROADMAP, COMPATIBILITY, STATE_FORMAT, ARCHITECTURE, TESTING,
    docs/README, VALIDATION, `tools/reference/README.md`, `docs/validation/M3H5.md`.
11. **Gates, review, PR.**

## 8. Follow-ups, risks and owner decisions

### 8.1 Owner decisions requested before Phase 2

1. **Policy identity**: `PlayerMonstersDiagonalV1`, wire 16, DSIM32, context
   `PLAYER-MONSTERS-DIAGONAL-1-MEDIUM-v1`.
2. **The corner jump (§4.4–4.5)**: (a) recommended — exact parity witness after every
   accepted projected slide move, `MapScaleError::Escape`, the one refused legal source
   move; (b) admit it and rely on the rectangle witness (faithful longer, unvalidatable
   afterwards); (c) parity witness for every mover every tic (about 3,000 extra `i64`
   operations per tic, catches nothing the proof allows). The proof does not change
   between (a) and (c).
3. **Boundary definition (§4.1 item 2)**: C1 (endpoint-only contacts), C2 (even
   degree), C3 (front side faces the room). C3 rejects "backwards" one-sided walls,
   which are map errors but would be loadable by the source.
4. **BLOCKMAP rule**: closed-cell intersection for this policy (bbox rule kept for
   heights). It is the rule real maps satisfy; it costs an exact clipper (about 40
   lines).
5. **Harness input**: a controlled `push` (set momentum) in the C harness and its Rust
   mirror for `corner_jump`. Without it the jump cannot be observed in C with this
   roster.
6. **CI headroom** (§8.2) and the allocation change (§8.3).

**Owner decisions (recorded 2026-11-01, before any runtime change):**

1. Approved: `PlayerMonstersDiagonalV1`, wire 16, DSIM32, context
   `PLAYER-MONSTERS-DIAGONAL-1-MEDIUM-v1`.
2. Approved: option (a). The exact parity witness runs after every accepted projected
   slide move; the corner jump fails the tic with `MapScaleError::Escape`. It is
   documented as the one refused legal source move and as a known fidelity gap to
   revisit for M4 demo compatibility. It is tested (`corner_jump` replay).
3. Approved: boundary rules C1–C3 for this policy. Real-map loading must revisit
   backwards and dangling one-sided lines (a later milestone).
4. Approved: the closed-cell BLOCKMAP rule for this policy only. Real-map loading must
   revisit BLOCKMAPs produced by real node builders (a later milestone).
5. Approved: a controlled push input in the C harness, clearly harness-only, so that
   `corner_jump` is observed in C.
6. Approved: the parallel reference suite with `--jobs 4` (outputs verified once to be
   byte-identical to the serial run, recorded in the evidence record), the macOS cap of
   40 minutes, and the spare-buffer swap.

### 8.2 Follow-up 1: CI headroom (measured)

Run `41378833526` (master `5e711be`, 2026-11-02): Ubuntu 30 min of 45, macOS 17 of 30,
Windows 12 of 30, MSRV 6 of 10. Steps over 100 s:

| Job | Step | Seconds |
| --- | --- | ---: |
| Ubuntu | Reproduce isolated reference vectors and snapshot goldens | 1,023 |
| Ubuntu | Test | 228 |
| Ubuntu | Release deterministic checks | 114 |
| macOS | Test | 491 |
| macOS | Release deterministic checks | 243 |
| macOS | Headless debug/release equality | 112 |
| Windows | Test | 300 |
| Windows | Release deterministic checks | 212 |
| MSRV | Test minimum supported Rust | 368 |

Inside the 1,023-second suite step (65 commands, one private workspace): the
single-threaded Python encoders dominate — `m3f3_snapshot_goldens.py` 215 s,
`m3d3h` 84 s, `m3d3c` 83 s, `m3d3j` 72 s, `m3f5_reference.py` (ASan) 93 s, then
eleven commands of 16–48 s; the remaining 44 commands sum to 117 s. Nothing is
dropped or weakened by running them concurrently: each command only compares against
committed bytes. Plan: `run_reference.py --suite --jobs N` creates `N` private
workspaces (the copied inputs are 512 MB, dominated by `tests/reference`; four copies
fit the runner), schedules the commands longest-first from a committed duration table
with a sequential fallback for `--jobs 1`, and CI passes `--jobs 4` on the 4-vCPU
Ubuntu runner: expected about 5 minutes instead of 17, a saving of about 12 minutes
on the Ubuntu job. The macOS Intel job's long steps are the test suites themselves;
nothing there can be split without dropping checks, so the plan raises its cap from 30
to 40 minutes with the measurement recorded in the workflow comment. Ubuntu's 45 is
kept. New M3h5 steps add about 2 minutes on Ubuntu (ASan generator plus goldens) and
about 1 minute elsewhere.

### 8.3 Follow-up 2: the per-tic sector target allocation

`Simulation::demon_plan` (`demon_expanded_map.rs:217`) copies every sector's sound
target into a fresh `Vec` for the heights policy (`StagedSound::Many`), the third
allocation per tic that the M3h4 record notes. Plan: `Simulation` keeps a spare
`Vec<Option<ActorId>>` of the sector count (not canonical state, not encoded, excluded
from equality); `demon_plan` takes it with `mem::take`, clears and extends it from the
live targets (no allocation once its capacity is right), and `commit_demon_plan` swaps
it with the live vector and keeps the old live one as the next spare. On a failed tic
the plan is dropped with the spare, the live vector is untouched (atomicity unchanged),
and the next tic allocates once to replace it. About 25 lines; provably safe because the
spare is never read. Verified with the allocation probe of `measure_stepping.py` on a
heights workload if the stepping example can select that policy (to be checked in Phase
2); if it cannot, the count is reported from the `player_heights` example under the probe
and the smoke assertion is left as it is.

### 8.4 Risks

- The harness node builder is new; its correctness is checked only by the C bodies
  agreeing with the Rust bodies on the same lumps and by an explicit convexity/side
  assertion per leaf. A wrong BSP would make both sides wrong together; the pillar
  room's sight cases (known answers by construction) guard against that.
- The parity witness depends on C1–C3; a map that passes them but whose intended
  sector side disagrees with the ray parity cannot exist (parity is the sector side by
  C3 at every wall, and constant between walls).
- `Ω` may contain more than one component (a map of two separate rooms); the proof
  holds per mover and never needs connectivity between movers.
- The `finesine[2048] = 65535` and `SlopeDiv` details make the replica's 38.409 exact
  for sampled momenta; the policy does not rely on the number, only on Theorem B's
  threshold of 30 and the exact witness above it.

## 9. Status / next step

Status: **investigation complete; owner decisions recorded (§8.1); Phase 2 in
progress; step 2 (C harness, polygon room builder and node splitter) steps 3 (DSIM32 projection) and 4–6 (admission, escape witness, DSIM32; one commit,
since the replay tests need all three), 7 (tools, example, fuzz, CI), 9 (allocation)
and 10 (docs) and 8 (CI headroom) committed, review recorded and applied, gates passed; step 11 ends
with the draft PR and its CI run**. Phase 2 corrections to the plan: the "existing controlled impulse" assumed for
the Rust replay of the push did not exist (`set_momentum` refused the player body under
every physical-player policy), so `set_momentum` admits the player body, monsters, corpses
and items (never fireballs) under this policy only; the DSIM32 projection had to derive actor sectors from the BSP leaves (every earlier
builder had one subsector per sector); the wedge is concave and carries one partition.
