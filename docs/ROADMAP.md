# Roadmap

This is the authoritative development summary and plan. The [documentation index](README.md#documentation-authority)
identifies the other information owners. Exact review state, commits, CI and merge
history belong to GitHub; supported behavior and limits belong to [COMPATIBILITY](COMPATIBILITY.md).

## Done

Method since M4a: whole systems, proven by the whole-engine harness on the real maps
(local-only) and by synthetic WADs through the same harness in CI; bespoke C scenarios
only for quirks the maps do not reach.

The merged baseline provides:

- WAD and classic-map parsing with checked references and resource budgets.
- Deterministic simulation, fixed-point arithmetic and original random-number streams.
- Ordered actors, state changes, static-sector movement and collision.
- Selected weapon/monster combat, sight, hearing and pursuit.
- Demon stand/run, melee, pain/death and persistent corpses.
- Bounded classic THING spawning (M3d3i, merged), independently checked against
  original C and versioned simulation snapshots.
- Demon map context and statistics (M3d3j): original kill-count flags, separate
  map and credited-kill totals, and fresh level reset with RNG continuity.
  See the [source investigation](M3D3J_REFERENCE.md) and
  [observed tests](validation/M3D3J.md) for technical evidence.
- M3f1 bounded line/sector scale, regularly merged: reusable source-ordered static
  collision and read-only sight preparation. See [source findings](M3F1_REFERENCE.md)
  and [observations](validation/M3F1.md).

- M3f2 expanded coordinates and spatial work, regularly merged: checked arithmetic,
  bounded collision preparation and fallible sight. See [source findings](M3F2_REFERENCE.md)
  and [observations](validation/M3F2.md).

- M3f3 expanded-map Demon composition is merged. Its opt-in profile reuses
  larger static geometry with bounded map-spawned Demon perception, pursuit,
  melee and lifecycle. See [support](COMPATIBILITY.md#expanded-map-demon-profile),
  [source findings](M3F3_REFERENCE.md) and [observations](validation/M3F3.md).

- M3f4 original actor/state identity admission is merged. Complete original
  identity metadata and checked registry resolution preserve existing execution.
  See [mapping](ACTOR_IDENTITIES.md), [source findings](M3F4_REFERENCE.md) and
  [observations](validation/M3F4.md).

- M3f5 canonical MT_SERGEANT execution binding is merged. Typed source requests
  use the existing native closure, with independent canonical C observations and
  explicit DSIM projection. See [investigation](M3F5_REFERENCE.md) and
  [observations](validation/M3F5.md).

- M3f6 original-ID runtime storage audit is merged. Its native representation
  guards preserve all historical aliases and diagnostics; see
  [audit](M3F6_REFERENCE.md) and [observations](validation/M3F6.md).

- M3f7 original-ID registry/profile and diagnostic contract is merged. Actual
  source IDs, explicit context definitions and root-derived state permissions use
  shared execution services; bounded v20/v21 readers preserve historical meaning.
  See [investigation](M3F7_REFERENCE.md) and [observations](validation/M3F7.md).

- M3f8 context and target-lifetime ownership is merged. Scoped access, derived
  reference availability and compact independent lifetime inputs preserve existing
  gameplay. See [design](M3F8_REFERENCE.md) and [observations](validation/M3F8.md).

M3f9 game-instance provenance is merged. Opaque incarnation keys qualify boundary
references while the same actor store and execution context remain authoritative;
see [design](M3F9_REFERENCE.md) and [observations](validation/M3F9.md).

These foundations support controlled headless tests, not a complete playable game.
Earlier findings and milestones are retained in the [historical records](validation/archive/README.md).

### M3h9c: The real E1M1 run, tic for tic

Merged 2026-11-05 (PR #56). The separately selected `PlayerMonstersLevelV1` (wire 22,
DSIM38) spawns the player at its place in the THINGS lump, runs the full player think
(damage-count decay, `P_DeathThink`), and the local-only whole-engine harness runs the
owner's E1M1 under the genuine linuxdoom-1.10 engine and the Rust simulation from one
version-110 demo, comparing every tic and committing only counts, pass/fail and digests.
All five scripted runs (idle, walking to the exit, fighting, barrels, random commands)
matched tic for tic. See the [source investigation](M3H9C_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-level-runs-the-player-start-in-lump-order-and-the-full-player-think-wire-22),
[observations](validation/M3H9C.md) and
[commands](TESTING.md#m3h9c-whole-engine-runner-and-the-real-e1m1-comparison). The
timing regression and the still-open intercept and `P_ChangeSector` observations are
carried in the M3h9c follow-ups below.

### M3g5: Active original-player roster and canonical Demon acquisition

Opt-in PlayerDemonAcquisitionV1 composes one active genuine player and at most one
map-THING Demon. Existing commands precede shared thinkers; source visual search,
lastlook/stop-slot behavior, wake sound/state entry and bounded autonomous pursuit
use the same simulation. Death retains activity and corpse ownership. The actual
melee branch rejects transactionally; full PlayerThink, roster deactivation and
autonomous combat are not claimed by that capability. M3g5 is merged. See
[source investigation](M3G5_REFERENCE.md), [support](COMPATIBILITY.md#active-original-player-and-demon-acquisition)
and [observations](validation/M3G5.md). Exact review/CI state belongs to GitHub.

M3g4 command/reaction movement, M3g3 physical knockback/corpses, M3g2 genuine-player
lifecycle and M3g1 diagnostic-free stepping are merged. Their investigations and
observations remain linked from the [evidence index](README.md).

## Current milestone

### M4a: Episode 1, part 1, proven against the genuine engine

Status: done; merged into master as
[PR #57](https://github.com/Copgunn/DOOM/pull/57) on 2026-11-10. Facts are in the
[investigation note](M4A_REFERENCE_INVESTIGATION.md), observations in
[validation/M4A.md](validation/M4A.md). The owner confirmed decisions H1-H12 of the note on
2026-11-05.

**Strategy change.** Earlier mover, monster and weapon milestones proved each mechanism with
bespoke C scenarios, one small mechanism at a time. With the whole-engine harness of M3h9c
in place, M4a ports whole systems (every Episode 1 door, floor and plat kind, not the
census-minimal subset), proves them with the whole-engine harness on the real maps
(local-only; genuine engine against Rust, tic for tic), and covers CI with synthetic
WADs run through the same harness. Bespoke C scenarios remain only for quirks the maps do
not reach, and the self-tests must prove the harness can fail.

**Targets.** A new separately selected `PlayerMonstersEpisode1V1` (wire 23, DSIM39, context
`PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1`; wires 22 and below stay frozen) for medium skill,
single player:

- the 29 Episode 1 line specials as whole systems (every door kind including locked,
  D1 and the shot door, every floor kind, the three plat kinds, stairs, the donut, the light
  trigger, the secret exit) and sector special 11;
- the Spectre, the chainsaw and the rocket launcher, with the generalised missile and
  explosion paths;
- keys and locked doors, backpack, soulsphere, invisibility, radiation suit, computer map and
  light amplifier, and the candle and tall red firestick;
- load-time proofs that real maps fail become per-tic witnesses, bounds are derived per
  map, and the command domain widens to the full byte range with weapon indices 0-7 (H3-H5);
- primary real-map targets E1M2, E1M3 and E1M4; E1M6 and E1M7 are secondary, admitted if
  they load and every script authored for them matches.

Verified so far in Phase 2 (steps 1-8 done on 2026-11-05): the movers, items, keys, powers,
chainsaw, rocket launcher, Spectre, command domain and demo header under wire 23, and Rust matching
the genuine engine tic for tic on all 30 real scripts (E1M2-E1M4, E1M6, E1M7 and E1M1 re-run), with
the `strict32` build identical; see the [evidence record](validation/M4A.md). The fuzz campaigns were
re-run clean on the final crates. Mutation survivor work, idle matched timings and simulation-only
per-tic measurements are recorded. Next: independent review, full gates from clean and the text
of the existing draft PR. Support is in
[COMPATIBILITY](COMPATIBILITY.md#original-episode-1-part-1-every-door-floor-plat-stair-donut-light-and-exit-special-keys-items-powers-chainsaw-rocket-launcher-and-spectre-wire-23);
the schema is [DSIM v39](STATE_FORMAT.md#dsim-v39--episode-1-part-1-every-special-keys-items-powers-and-the-rocket-launcher).
Remaining: the owner's review and merge.

### Previous gameplay foundation — M3h9b: Exploding barrels, pickup scale and every thing angle

The separately selected `PlayerMonstersBarrelsV1` (wire 21, DSIM37) is the E1M1 world plus
the last four things that kept E1M1 from loading: exploding barrels in full (the source's
damage, death chain, `A_Explode` and `P_RadiusAttack` with its wrapped 3 × 3 block window,
link-order walk, chain explosions, thrust, the first-wounder bomb source and the kill
credit it implies), pickups raised to 64 with 384 actor slots under this policy only, every
THING angle admitted as the original build's wrapped product (decision E9), and the
spawn rule S′, which admits the 16 E1M1 THINGs whose closed box touches a wall while
keeping the M3h5 invariant (decision F3). DSIM37 is the DSIM36 layout with barrels as
ordinary actor records and `store.max_slots` 384, no new field or event. **E1M1 on medium
skill passes every load rule it is measured against** (the note's
[§10](M3H9B_REFERENCE_INVESTIGATION.md#10-e1m1-medium-load-admission-under-the-new-policy));
the real-map run that confirms it is M3h9c (above). Two new fidelity-gap witnesses: an explosion
whose bomb source handle is stale refuses the tic, and so does a barrel whose step would
carry its centre through a wall corner (the barrel counterpart of the player's corner-jump
witness). Older policies keep their admission,
digests and DSIM bytes. Maintenance in the same milestone (owner request, 2026-11-04): the
docs-only CI fast path, a detect-changes job that lets a documentation-only push skip the
heavy steps inside each required job so every required check still reports.

Status on `feat/m3h9b-barrels`: implemented and in owner review (draft pull request #55,
not merged); every local gate, the matched timings, the fuzz campaigns and the independent
review are recorded. See [source investigation](M3H9B_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-with-exploding-barrels-pickup-scale-and-every-thing-angle)
and [observations](validation/M3H9B.md). Decisions F1–F11 are adopted; the owner
confirmed them on 2026-11-05.

### Previous gameplay foundation — M3h9a: Two-sided scrolling walls and static decorations

The separately selected PlayerMonstersE1M1V1 (wire 20, DSIM36) is the movers world plus
two parts of E1M1 that need no new state: the scrolling wall 48 on two-sided lines,
blocking or not, with the source's three game-code effects (a `spechit` entry, the
monster's direction reset with a failed use, the player's use stopped with no effect),
and the seven static decoration types E1M1 spawns (floor lamps, tech pillars and
candelabras that block with no height test; pools of blood, dead players and bloody
messes that everything passes through), at most 32 per map. Decorations spawn with
their single `P_Random` draw in lump order, ride movers, stop fireballs when solid,
let hitscans through and are never damaged, gibbed, removed or targeted. DSIM36 is the
DSIM35 layout with no new field or event. Barrels, more than 8 monsters or 16 pickups,
every other THING type and THING angles outside the inherited `[-4,3]` octants stay
rejected at load, so **E1M1 itself still does not load**: on skill 3 its blockers are 6
barrels, 52 pickups and 45 THINGs with angle octants 4..7, and its spawn-box rule is
unmeasured. No
new fidelity gap is added. Older policies keep their admission, digests and DSIM bytes.

Merged as PR #54. The C evidence (22 scenarios), the
independent DSIM36 projection, the policy, the full replay, the Rust-only tests, the
example, the tools, the fuzz target with its seed replay and this documentation are
complete, the independent review's findings are fixed, and the bounded fuzz campaigns,
the matched timings and the local gates are recorded. The owner confirmed decisions E1–E9
(E9 applies to M3h9b), kept wire 19 frozen with its pickup vertical-momentum gap as a
known limitation, and kept damage with no inflictor and no source faithful (accepted
and ignored). This branch stops at owner review; exact commits and CI belong to
GitHub. See [source investigation](M3H9_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-with-two-sided-scrolling-walls-and-static-decorations)
and [observations](validation/M3H9.md).

### Previous gameplay foundation — M3h8: Doors, lifts, floors, switches and the exit (Stage B)

The separately selected PlayerMonstersMoversV1 (wire 19, DSIM35) is the specials world
plus E1M1's seven tag, use and crossing line types: the DR door 1, the SR door switch 63,
the W1 blazing door 109, the WR lift 88, the W1 floors 36 and 38 and the S1 exit switch
11. It adds `BT_USE` with the source's use latch, moving floors and ceilings staged with
the checked tic, `P_ChangeSector` (re-clipping, giblets, dropped-item removal, pickups
inside a clip), crossing through the source's `spechit`, monster door use, switches with
their button slots and the exit. The exit tic completes and the next `advance` is refused
with `SimError::LevelExited`; `BT_USE` held while dead completes its tic and the next is
refused with `SimError::PlayerReborn`. Load rules reject what the source leaves undefined
or what is deferred: one-sided DR lines, shared mover families, doors without neighbors,
inverting and self-referencing doors, reachable heights beyond ±512, more than 30 lifts,
16 SR switches or 8 special lines in a BLOCKMAP window, sky ceilings over movers,
switches without a switch texture, crushers and every other line type; a two-sided
scrolling wall stays rejected under this policy (decision N1, option iii). V′ and a relaxed
item witness replace the vertical and item witnesses under this policy only, and the
ninth `spechit` write is a refuse-the-tic witness (an M4 fidelity gap like the corner
jump and the intercepts). Older policies keep their admission, digests and DSIM bytes.

M3h8 is merged. See [source investigation](M3H8_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-with-doors-lifts-floors-switches-and-the-exit)
and [observations](validation/M3H8.md).

### Previous gameplay foundation — M3h7: Sky and static specials

The separately selected PlayerMonstersSpecialsV1 is the real-map world plus the parts of
E1M1's specials that move no plane: sky ceilings and the static sector specials. It
admits the light sector specials 1, 2, 3, 8, 12, 13 and 17 (blinking, strobing, glowing
and flickering lights, with their random draws), the damage floors 4, 5, 7 and 16 on the
source's 32-tic gate, the secret 9, the scrolling wall 48 on at most 64 one-sided lines,
and `F_SKY1` ceilings with their two game-code effects: a fireball refused at a sky
ceiling line is removed without exploding, and a hitscan reaching a sky ceiling ends
without a puff. Light thinkers share one processing order with the actors, in their own
slot class bounded at load and spawned by an explicit `spawn_level_specials` after the
THINGs and the player body; environmental damage and death go through the shared damage
path with no source. Every other line special, a two-sided 48, sector specials 10, 11,
14 and unknown values, `BT_USE` and every moving plane stay rejected; sector lights
outside the range the light arithmetic needs are refused at load. DSIM34 adds the
shared thinker order, the light thinkers, the runtime sector light and special and the
player's secret count. E1M1 still needs the roster scale, eight decoration and barrel
types and the doors, lifts, floors and exit switch of Stage B. M3h7 is merged. See
[source investigation](M3H7_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-with-sky-and-static-specials) and
[observations](validation/M3H7.md).

### Previous gameplay foundation — M3h6: Real-map scale — classic map limits toward E1M1

The separately selected PlayerMonstersRealMapV1 raises the static-geometry limits of the
diagonal world from test-room size to the scale of classic maps. Every cap that stood in
for an allocation or CPU budget (95 linedefs, 256 sectors, 4,096 nodes and subsectors,
8,192 BLOCKMAP cells, 256 THINGs, ±2,048 coordinates, 1,536-unit lines, fixed sight and
collision work) becomes the WAD format's own `short` bound or a value derived from the
loaded map; the arithmetic envelope becomes a certificate computed from the map against
the same source expressions; `MAXINTERCEPTS` becomes a runtime witness that refuses the
tic which would write the source's 129th intercept (`MapScaleError::Intercepts`, the
second refused legal source behavior after the corner jump, a known fidelity gap for
demo compatibility), with a slide-window admission rule that proves slides never reach
it. Player starts are kept as records, the hitscan collector's 128-line buffer is fixed
so maps above 128 lines can fire, and the noise flood reads per-sector line lists.
Roster, slots, fireballs, heights, specials, sky and the boundary rules are unchanged;
E1M1's geometry, BLOCKMAP, REJECT, boundary and arithmetic are inside the new policy,
its THING roster, decorations, sky and specials are not. DSIM33 adds no persistent
field. M3h6 is merged. See
[source investigation](M3H6_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-at-classic-map-scale) and
[observations](validation/M3H6.md).

### Previous gameplay foundation — M3h5: Non-axis (diagonal) walls

The separately selected PlayerMonstersDiagonalV1 takes the heights world off the
axis-aligned grid: one- and two-sided linedefs of any direction inside a closed
boundary of one-sided walls, so wall slides at any angle, convex and concave diagonal
corners, diagonal risers and windows, an octagonal pillar and a 45° corridor execute
through the angle-general bodies the engine already carried (`P_HitSlideLine`'s
projection, `P_BoxOnLineSide`'s corner tests, the intercepts, sight, hitscan, autoaim,
fireballs and `P_NewChaseDir`). The closed boundary is defined by three exact integer
rules (endpoint-only contacts, even vertex degree, front sides facing odd parity) and
the wall invariant is proved for every move of at most `2r − 2` units per axis; the one
move outside that bound, a player's projected slide of more than 30 units per axis, can
jump a corner in the source and is the one legal source move the policy refuses
(`MapScaleError::Escape`, a known fidelity gap to revisit for demo compatibility).
DSIM32 adds no persistent field. Moving sectors, specials, sky ceilings and cap
increases stay rejected at load. M3h5 is merged. See
[source investigation](M3H5_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-among-diagonal-walls) and
[observations](validation/M3H5.md).

### Previous gameplay foundation — M3h4: Two-sided lines, static floor/ceiling heights and vertical movement

The separately selected PlayerMonstersHeightsV1 takes the canonical player and monster
world off flat floors inside the M3h3 walled rectangle: axis-aligned sectors with
different static heights joined by two-sided lines, so steps, ledges, drop-offs,
windows, closed sectors and fences execute through the original bodies the engine
already carried (`P_LineOpening`, `P_TryMove`'s fit, step and dropoff rules,
`P_ZMovement`, sight and hitscan slopes, fireball chords, `P_RecursiveSound`). The
vertical invariant extends the wall invariant to openings and is checked before every
tic; spawns must fit their sector. The landing sound, cross-sector weapon noise and the
fall-forward damage branch become reachable. DSIM31 adds no persistent field. Moving
sectors, specials, sky ceilings and non-axis walls stay rejected at load. This branch
stops at owner review; exact commits and CI belong to GitHub. See
[source investigation](M3H4_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-with-static-heights) and
[observations](validation/M3H4.md).

### Previous gameplay foundation — M3h3: Wall-aware travel bounds, no-stall guarantees and real-map scale

The separately selected PlayerMonstersMapScaleV1 replaces the conservative `160M/3`
travel envelope and the 994-unit actor margins with bounds derived from the loaded
map: a walled rectangle of up to 2,048 units whose one-sided axis walls keep every
mover inside, exactly as the original's `P_TryMove` does. Both M3h2 stalls (a hit
near a wall, and a fireball past 994, which was confirmed rather than refuted) are
reproduced on the historical policies and progress on the new one. Slot and
fireball budgets (320 slots and 208 fireballs) are derived from flight time and attack cadence, so
ordinary play cannot reach a cap, and immutable definitions are validated once
instead of every tic. DSIM30 adds no persistent field. This branch stops at owner
review; exact commits and CI belong to GitHub. See
[source investigation](M3H3_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-at-real-map-scale) and
[observations](validation/M3H3.md).

### Previous gameplay foundation — M3h2: Original Imp, fireballs and roster maintenance

The separately selected PlayerMonstersImpV1 brings projectiles into the canonical
player world: the original MT_TROOP chooses melee or missile, and MT_TROOPSHOT flies,
collides and explodes through one shared missile service, so fights gain dodgeable
shots and projectile infighting with the source same-species rule. Imps join the
eight-monster roster, at most 24 fireballs are live, and exceeding a bound rolls back
the whole tic. Canonical action dispatch is now typed. DSIM29 adds no persistent
field. This branch stops at owner review; exact commits and CI belong to GitHub. See
[source investigation](M3H2_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-imps-and-fireballs) and
[observations](validation/M3H2.md).

### Previous gameplay foundation — M3h1: Original former humans and multi-monster encounters

PlayerMonstersV1 adds original MT_POSSESSED, MT_SHOTGUY and MT_CHAINGUY beside the
Demon, up to eight monsters, source infighting and P_KillMobj drops with DSIM28.
M3h1 is merged; exact review/CI state belongs to GitHub. See
[source investigation](M3H1_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-former-humans-and-multi-monster-encounters) and
[observations](validation/M3H1.md).

### Previous gameplay foundation — M3g11: Original health/armor bonuses and counted items

PlayerDemonBonusesV1 adds genuine animated health and armor bonuses (+1 up to 200)
and COUNTITEM statistics to the armor world with DSIM27. M3g11 is merged; exact
review/CI state belongs to GitHub. See
[source investigation](M3G11_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-health-and-armor-bonuses) and
[observations](validation/M3G11.md).

### Previous gameplay foundation — M3g10: Original armor, damage absorption and sustained combat

PlayerDemonArmorV1 composes genuine animated green/blue world armor with the
existing player, weapons, contacts and autonomous Demon, using shared damage
planning for absorption, depletion and rollback with DSIM26. M3g10 is merged;
exact review/CI state belongs to GitHub. See
[source investigation](M3G10_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-armor-and-damage-absorption) and
[observations](validation/M3G10.md).

The zstd/formula maintenance milestone is merged. Its independent migration and
formula proofs, unchanged gameplay and preserved DSIM contracts remain in
[storage](EVIDENCE_STORAGE.md), [formulas](WEAPON_FORMULAS.md) and
[observations](validation/ZSTD_WEAPON_MAINTENANCE.md).

### Previous gameplay foundation — M3g9: Original world pickups and combat sustain

PlayerDemonPickupsV1 adds bounded original world items to the existing genuine
player/four-weapon/Demon world. Ammo, healing and source-gated shotgun/chaingun
acquisition share actor storage, collision, inventory, retirement and transactions.
Source-first observations and DSIM25 make pickup ordering and player resource
changes explicit. M3g9 is merged; exact review/CI state belongs to GitHub. See [source investigation](M3G9_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-world-pickups) and
[observations](validation/M3G9.md).

### Previous gameplay foundation — M3g8: Original weapon expansion

M3g8 is merged. Its separately selected no-pickup four-weapon capability and
DSIM24 retain their original contract. See
[investigation](M3G8_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-player-weapon-expansion) and
[observations](validation/M3G8.md).

### Previous gameplay foundation — M3g7: Genuine canonical player pistol combat

M3g7 is merged. PlayerDemonPistolV1 retains its separately bounded no-switching,
no-fallback contract and DSIM23. See [source investigation](M3G7_REFERENCE.md),
[support](COMPATIBILITY.md#genuine-player-pistol-combat) and
[observations](validation/M3G7.md).

### Previous gameplay foundation — M3g6: Autonomous canonical Demon-to-player melee

Opt-in PlayerDemonMeleeV1 composes the same genuine player, fixed active roster,
commands, sight, pursuit and thinker transaction with original delayed SARG melee.
The attack rechecks current range, then applies damage/physical impulse through
the shared DamagePlan, including pain, death, target retention and persistent
corpses. Misses and dead-body ignored damage retain distinct RNG behavior. Older
policies keep their admission and rejection contracts. M3g6 is merged; exact
commits and CI belong to GitHub. See
[source investigation](M3G6_REFERENCE.md),
[support](COMPATIBILITY.md#autonomous-original-demon-player-melee) and
[observations](validation/M3G6.md).

### Next narrow gameplay milestone

- **M3h9c follow-ups.** The real-E1M1 harness is merged (M3h9c, above). The
  matched timings found a codegen-split regression of 6-13% in time on the older Demon-profile
  workloads (`canonical_demon`, `demon_expanded_map`, `diagnostic_free`; identical digests),
  accepted as explained in decision G13 and to be reconsidered with the owner in a maintenance
  milestone. Still open after it: the actual intercept maximum, the `P_ChangeSector` work at 129 linked
  things and whether any per-tic witness fires on the real map, to be observed by the
  matched runs and recorded in its evidence record.
- **Later bounds and skills.** Barrels beyond 16 and pickups beyond 64 for the rest of
  Episode 1 need more actor slots (the M3h9b note's §5.3 formula) and their own evidence.
  Easy and hard skill (E4 option ii) were deferred by decision F10, not dropped: hard skill came
  with M4b, and easy skill moves to M4c (proposed, under [Later](#later)).

Also revisit the rules the 14 other IWAD maps break (13 with one-sided walls of odd
degree, crossing or facing away from their sector, one BLOCKMAP with missing listings)
and the 22 maps that fail the sight certificate. Require source-first evidence and
unchanged earlier contracts. This is future work, not admission in M3h9b.

### Focused maintenance acceptance criteria

- Inventory production panic paths reachable from public inputs; replace recoverable
  failures with checked errors and preserve complete-state rejection evidence.
  Name remaining gameplay flags/constants with source references, without broad
  unrelated refactoring or changing historical profile semantics.
- Measure reference bytes against independently useful input/event coverage before
  adding large traces. Prefer compact uniquely useful evidence while retaining
  immutable historical fixtures/tools/archives. Any storage migration needs its own
  preservation proof and reviewed scope.

- Inventory repeated preflight walks and transaction staging against the measured
  component/allocation baseline. Any removal must preserve late-error rollback,
  bounded resources and complete source/event/RNG observations, with matched
  three-process timings and actual allocation counters. Do not add an unchecked
  release mode or fragile timing gate. Prioritize a separately reviewed bounded
  scratch reuse or one proven redundant validation over broad redesign.

### Map Scale, Limits, and Real-Map Readiness

**Broader readiness remains planned.** M3f1 supplies the merged line/sector
foundation; M3f2 supplies bounded coordinates/work and M3f3 composes a controlled
Demon gameplay subset; M3h6 replaces the static-geometry caps of the player and monster
world with the format's bounds, a per-map arithmetic certificate and derived work
values, so E1M1-sized geometry is admitted. Further stages
require separate owner direction. The longer-term
purpose is practical support for increasingly complex classic maps,
not unlimited geometry or arbitrarily larger constants.

Start from the [code-backed limits inventory](COMPATIBILITY.md#map-size-and-processing-limits).
It distinguishes original encoding/source constraints, parser allocation budgets,
temporary simulation restrictions, and CPU/traversal work bounds. Parsing a map
successfully does not mean a simulation configuration can execute it.

Acceptance for that future milestone:

- Inventory line, sidedef, sector, vertex, seg, node, subsector, BLOCKMAP and THING
  limits, including interactions between individual table and aggregate budgets.
- Identify unnecessarily restrictive bounds for valid classic maps; document
  encoding/source constraints that must remain and where wider internal indexes,
  dynamic work buffers or other representation changes are required.
- Keep classic compatibility distinct from extended map formats. Specify the
  intended supported slice and compatibility implications before changing limits.
- Preserve checked indexing/arithmetic/allocation, bounded memory and recoverable
  error atomicity; choose justified capacities and work budgets.
- Benchmark larger synthetic or legally redistributable public maps. Verify BSP,
  collision, sight, thinker/spatial ordering and deterministic runtime capacities.
- Add boundary, rejection and regression tests; preserve original-C evidence and
  historical canonical fixtures. Publish measurements and remaining restrictions.

This plan does not authorize commercial-asset access, new map formats or broader
monster/player behavior. Further stages require separate owner direction.

## Later

- **M4b (merged as PR #58, 2026-11-14; decisions I1–I13 confirmed by the owner on 2026-11-10; see the
  [investigation note](M4B_REFERENCE_INVESTIGATION.md)).** Teleportation (line 97 and the landing thing), the
  Baron and `A_BossDeath`, hard skill, and with them E1M5, E1M8 and E1M9; the `DEMO1`
  (E1M5, skill 3) of the four-episode `DOOM.WAD`, which the genuine engine runs as registered because of its file
  name, is runnable here. Wire 24 (`PlayerMonstersEpisode1V2`, DSIM40) admits
  teleportation, the Baron, `A_BossDeath` and hard skill, and E1M5, E1M8 and E1M9 load on skills 2 and 3; the
  real-map runs (61, all matched against the genuine engine, local-only) and `DEMO1` are recorded in the
  [evidence record](validation/M4B.md#real-maps-and-demo1); the independent review is in the
  [review record](validation/M4B.md#independent-review-step-14); the Baron fireball wall tunnel is a declared
  refuse-the-tic witness ([COMPATIBILITY](COMPATIBILITY.md#the-baron-fireball-wall-tunnel-wire-24)); mutation
  testing, fuzzing and matched timings are recorded in the evidence record (the short-workload slowdown
  reproduces only from the 9p mount, not on ext4, so it is not a code regression; it is carried to the
  performance milestone below).
- **M4c (proposed, pending the owner).** Pause, easy skill and the reborn reload as an
  admitted transition; level transitions stay a later direction.
- **M4p, simulation performance (merged as PR #60, merge commit 82e2207, 2026-11-15; decisions P1–P9 confirmed
  by the owner on 2026-11-15; see the [investigation note](M4P_REFERENCE_INVESTIGATION.md)).** Make
  `Simulation::advance` faster on the real Episode 1 maps without changing any digest, DSIM byte, refusal or
  event. Plan steps 1–10 are done: eleven optimisations (O1–O7, O9a–O9c, O10; O8 and O9d dropped because they made
  some workload costlier), each with its equivalence proof, a Callgrind regression guard in CI (`simulation-ir`)
  and its baseline. In a matched run on rustc 1.85.0, advance-only medians fell from 64–829 µs per tic on master
  to 33–150 µs (1.8× to 7.1× faster), with every real-map digest unchanged; P1's 150 µs target is met there and
  missed by 0.9 µs on E1M6 in an earlier run
  ([evidence record](validation/M4P.md#matched-timing)). The independent review found no behaviour difference.
  Mutation testing, fuzzing and the full gates from clean passed. Whether `simulation-ir` becomes a required
  check is the owner's call.
- Complete player/gameplay integration and broader monster/THING support.
- The remaining moving sectors and map specials (the established M3e direction) beyond
  the doors, lifts, floors, switches and exit of M3h8: crushers, ceilings, teleporters
  and the other triggers (stairs, locked doors and the other Episode 1 door, floor and plat
  kinds belong to M4a).
- **M4 — Demo compatibility:** version-specific playback and divergence diagnostics;
  keep DOS and Linux behavior claims separate.
- **M5 — Rendering and platform integration:** presentation/input/audio, with a
  custom Vulkan-first rendering direction across supported platforms as a future
  direction, not a commitment. Presentation must not change authoritative
  simulation or RNG.
  - **M5a (faithful software renderer; merged as [PR #61](https://github.com/Copgunn/DOOM/pull/61), merge commit 9a598b8, 2026-11-17; the post-merge CI run on master is green).**
    Phase 2 is implemented: the `doom-render` crate draws `R_RenderPlayerView`'s 3D view at high
    detail (screenblocks 9-11) and matches linuxdoom-1.10 on every synthetic frame in CI and on every
    frame of the scripted real Episode 1 runs (local-only); a local GIF viewer, a CI job for the
    genuine renderer, unit and contract tests and two fuzz targets are in. Owner decisions R1-R16 were
    confirmed 2026-11-15. Mutation testing (with an independent review of its dispositions), the
    independent code review, fuzz campaigns and timings are recorded. See the
    [investigation note](M5A_REFERENCE_INVESTIGATION.md) status
    line, [support](COMPATIBILITY.md#software-renderer-m5a) and the
    [evidence record](validation/M5A.md). Not part of M5a (M5b and later): status bar, HUD, automap,
    menus, palette flashes, window, input, sound, low detail, a modern renderer.
  - **M5b (playable: window, input, status bar, HUD messages, palette flashes and the real-time loop on
    the wire-24 simulation and the M5a renderer).** Phase 1, the investigation, is done (2026-11-17); owner
    decisions S1–S23 were confirmed by the owner on 2026-11-17, and Phase 2 is at the review stage (draft
    [PR #62](https://github.com/Copgunn/DOOM/pull/62); status on 2026-11-17). Done and recorded in the
    [evidence record](validation/M5B.md): the genuine status bar, HUD and input harnesses, the Rust status bar,
    HUD and `Display`, the input core, the app core and the `doom-play` window (a first play check and a
    focus-loss check ran under WSLg), session verification against the genuine engine, all 61 real Episode 1
    runs through the Rust display, the opt-in unrecorded full-resolution turning mode (S3 (ii)), the CI jobs
    and the Windows artifact job (it ran successfully on the pull request), both fuzz targets, the README
    animation, two pairs of independent reviews with their fixes, the mutation runs (every survivor of the first
    three is killed by a test or argued equivalent), and frame-time and tic-jitter timings (WSLg only, recorded, not
    gated). The final gates were all run from a clean clone at `d5a4c4a` (2026-11-17/18; every one passed, including the
    fourth mutation run; see [step 15](validation/M5B.md#step-15-final-gates-from-clean)). Remaining: CI on the exact final head, the
    Windows artifact rebuilt on it, the pull request marked ready and the owner's review. [Support](COMPATIBILITY.md#playable-build-m5b) is written. The owner played the
    M5c build (M5b's input and picture included) on 2026-11-18 on Windows 11 and under WSLg (see [the owner's playtest](validation/M5C.md#the-owners-playtest-2026-11-18)); beyond that, a human has to judge feel, latency and looks. See the
    [investigation note](M5B_REFERENCE_INVESTIGATION.md#4-owner-decisions-s1s23-with-recommendations).
  - **M5c (sound effects matched to linuxdoom-1.10, and the closing of the status bar face's idle glance
    gap, S15).** In progress on the branch `feat/m5c-sound` (draft [PR #63](https://github.com/Copgunn/DOOM/pull/63);
    status on 2026-11-18). Sound starts and the level-start wipe draw `M_Random`, so modelling both makes the idle
    face's left, ahead or right glance match the genuine engine replaying the session's demo. The owner confirmed
    decisions T1-T16 on 2026-11-18 (the reference is "linuxdoom-1.10 built with its in-process sound code (one define
    removed)"). **Done and recorded** in the [evidence record](validation/M5C.md): the genuine sound-logic and mixer
    harnesses with their goldens (steps 1 and 2); the proof that the audio library compiles and links in CI on all
    three systems (T7); wire 25 with canonical sound records, the corrected scream and the THING-loading truncation,
    compared with the genuine engine on the 64 synthetic items and the 61 real runs (step 3); the crate `doom-sound`
    (step 4); **the display contract with sound, `DDSP` version 2 and the sound session in the game loop, with every
    frame of the 64 items (45,540) equal and the face equal on every frame (step 5 part 1); restore and continue
    (step 6); the 61 real runs through the whole pipeline locally (158,966 tics: every sound row, every frame at both
    sizes and 98,578 mixer buffers equal, step 5 part 2); nine scripted app sessions against the genuine engine (step 5
    part 3); and sound in `doom-play` (a device sink that never waits for the device to play anything, `--volume`,
    `--no-sound`, End to mute, step 7)**; the CI job `M5c genuine sound (not required)` and the mutation workflow's coverage of `doom-sound`
    (step 8, part 1); five fuzz targets, the fifth for `DDSP` version 2 (step 11); timings (step 12); **and the independent
    review (step 13): three reviewers, no blocker, one major finding (the DSIM41 reader refused every capture of a map the
    THING truncation cuts short), 11 minor and 19 notes, each fixed or documented, with a test that failed before the change wherever a behaviour was fixed** ([record](validation/M5C.md#step-13-independent-review)). **The status bar face's idle glance gap (S15) is closed** in the
    owner's T14 wording ([COMPATIBILITY](COMPATIBILITY.md#the-status-bar-faces-idle-glance-is-closed-with-sound-decision-s15-t14)).
    **Mutation testing is done**: the first run's 74 survivors each have a disposition (60 killed by new tests, 12 equivalent,
    2 unreachable) and an independent check agreed; the final run at `a8abf0c` (999 mutants: 865 caught, 28 missed, 3 timeouts, 103
    unviable) had 14 new survivors, all killed by new tests; the confirming run at `937cf9d` (879 caught, 14 missed, 3 timeouts, 103
    unviable) missed only the 14 equivalent or unreachable mutants ([step 10](validation/M5C.md#step-10-mutation-testing)); a local verifier for
    `doom-play` sessions with sound exists (not in CI). One check failed on the branch: `Simulation instruction counts` at
    `3c1cc5d`, a code-generation effect of a review fix on one case; a code-only fix is on the branch (instruction counts end up
    0.01 to 0.34 percentage points higher in all nine cases, inside the threshold; the baseline is unchanged). **The job is green
    at `a8abf0c`, with all nine jobs of that run**, and the owner accepted the fix on 2026-11-18, which changed one call-site line of
    an existing test and no assertion, on the condition that every frozen wire's goldens pass in CI on that head, which they did
    ([record](validation/M5C.md#the-instruction-count-check)). The README gains a sound visual (waveforms of the genuine and Rust
    mixers for one synthetic session, 0 of 30,720 samples differing; [record](validation/M5C.md#the-sound-visual-and-the-session-checks-label)).
    CI was green on all nine jobs at `937cf9d` ([record](validation/M5C.md#the-ci-record)). **The final gates** were all run from a clean clone at `a15e0dc` on 2026-11-19 and every one passed (1,876 workspace tests; the Linux steps of the Stable job;
    the M5a, M5b and M5c genuine-engine jobs; all 60 fuzz targets built, 7 seed replays and 10 bounded campaigns on the five M5c targets with no finding; the 61 real runs
    through every local tool; the owner's 13 session demos, all passing) ([record](validation/M5C.md#step-14-final-gates-from-clean); a first run at `937cf9d` was
    stopped and is superseded). **What remains:** the pull request marked ready with its rewritten body, the Windows artifact and CI on the final head (GitHub's
    record), and the owner's review. The melt picture is not built: it is original behaviour, so not optional, and no milestone is named for it yet ([checklist](#vanilla-completeness-checklist)). **Observed by the owner on 2026-11-18** (one person, one evening, release builds of
    `3c1cc5d`; [the owner's playtest](validation/M5C.md#the-owners-playtest-2026-11-18)): on Windows 11 the window opens, the game
    plays at 35 frames a second, the mouse is captured and sound plays, with a delay on every sound that he judged normal for the
    original and left unchanged (the arithmetic puts the Windows range above the original's; both are in the
    [record](validation/M5C.md#sound-latency-the-arithmetic-in-full)); under WSLg sound plays but the pointer is not captured, so WSLg is
    keyboard-only for now; his 12 session files replay equal to the genuine in-process build (12 of 12). **Not judged:** macOS, long
    sessions, every map, every weapon and monster, vanilla bindings. **A fault he reported on Windows,** left mouse fire sticking
    while the mouse moves, did not reproduce in the app's event layer on Linux. He retested on Windows 11 on 2026-11-19 with the logging
    option `doom-play --input-log FILE`: the log shows three left-button releases that never reached doom-play (summary line Left 18/15) and
    points, as an inference, to the hidden cursor resting on the window's border at 125% display scaling. **By his ruling it is not fixed in M5c;**
    the fix is the first item of the next milestone, below ([record](validation/M5C.md#the-owners-windows-retest-with-the-input-log-2026-11-19)). Nobody has listened to the visual's WAV file, the SVG was not rendered on
    the development machine, and no MP4 was written (ffmpeg is not installed, so that code path has never run). **Owner rulings
    of 2026-11-18 (second set):** a game under WSLg that occasionally opens already paused stays as it is, and the owner will judge it
    when he plays; when sound or display refuses a tic the simulation's tic stays committed and its command stays in the demo while
    sound and display stay at the last good tic (a future save must never combine a simulation at tic N with sound or display at
    tic N-1); the layout-dependent label in `doom-play`'s exit line is enough, with nothing drawn on screen; the device queue's two-buffer
    start is accepted as presentation, unmeasured, and the owner will judge lag and crackle by ear; the macOS job's limit is 90
    minutes (the follow-up is below). Support is in [COMPATIBILITY](COMPATIBILITY.md#sound-effects-m5c). See the
    [investigation note](M5C_REFERENCE_INVESTIGATION.md) and its
    [owner decisions](M5C_REFERENCE_INVESTIGATION.md#5-owner-decisions-t1t16-with-recommendations).
  - **M5d (play fixes), in progress on the branch `feat/m5d-play-fixes` ([evidence record](validation/M5D.md)); the first item of the next milestone, by the owner's ruling of 2026-11-19.** *Status 2026-11-20: after the owner's Windows session a second round followed (re-grab and cursor-left report, writer threads, settle at a session's end; [record](validation/M5D.md#a7-the-owners-windows-session-of-2026-11-20-and-the-second-round)); on his retest the same day he accepted the stuck-fire fix and the slow-disk fix ([record](validation/M5D.md#a9-the-owners-retest-of-2026-11-20-his-rulings-and-the-close)); the pull request is at his review gate. Follow-ups by his rulings are listed after item (d).* In this order: (a) **the stuck-fire fix** (proposed, untested):
    ask the window library for DPI awareness (`Conf { high_dpi: true, .. }` in `crates/doom-play/src/window.rs`) so that the client rectangle, the clip
    rectangle and the cursor share physical pixels and the clip holds the cursor inside the window; the requested size and the picture's fit are then in
    physical pixels and must be checked. Acceptance: on Windows at 100%, 125% and 150% scaling, a session of drag-firing against every edge with
    `--input-log` ends with left downs equal to ups and no resize line. If a release can still be lost, a real mouse capture is needed, which miniquad
    0.4.11 does not offer: a library change or upgrade, or a reviewed exception to the no-unsafe rule, which is the owner's decision
    ([record](validation/M5C.md#the-owners-windows-retest-with-the-input-log-2026-11-19)). (b) **Measuring the real sound latency on Windows and trying the cheapest way to shorten it**
    (the candidates and their costs are in the [evidence record](validation/M5C.md#sound-latency-the-arithmetic-in-full): the start threshold, a smaller callback
    or period, WASAPI with two periods, WASAPI at the mix rate with the library's resampler, drift control); which is cheapest is decided after measuring, and
    the declared schedule itself is part of the claim, not a presentation knob. (c) **Writing the session demo so that Ctrl+C or a closed console loses
    nothing** (options in the [lost-connection record](validation/M5C.md#the-lost-display-connection-under-wslg-what-is-saved): writing the demo incrementally,
    one tic at a time, is the safe-Rust candidate; the X connection loss under WSLg is the same class of exit). (d) **In that code change, by the owner's
    ruling:** the mouse-forwarding logic moves out of `crates/doom-play/src/input_log.rs` into a file named for what it does, and the review's three minor
    points are closed (M1: the log gains a line for each committed tic's command or fire bit and names the session; M2: the stale test comment at
    `crates/doom-play/src/audio.rs:601`; M3: the log's module comment states the exceptions to its pump rule and what position a button line carries on Windows).
    **Follow-ups, by the owner's rulings of 2026-11-20 ([record](validation/M5D.md#a9-the-owners-retest-of-2026-11-20-his-rulings-and-the-close)):**
    (1) **the default sound period becomes 128 frames, with the two-buffer start kept**: done in its own small pull request after M5d merged (branch
    `feat/audio-period-128`, 2026-11-21, awaiting the owner's review); it edited the master test `the_numbers_are_those_of_the_specification` openly
    ([record](validation/M5D.md#the-default-period-becomes-128-frames-2026-11-21-its-own-pull-request)). (2) **A panic of the session writer's thread** (accepted as a documented limit in
    M5d): the exit names every session whose file is short and what it holds, but nothing is rewritten after the panic, so the writes still queued and
    the earlier sessions awaiting a rewrite are lost. The candidate fix is a synchronous rewrite by the loop thread at exit, from the app's own recording
    and from copies of the earlier sessions kept where both threads can reach them; two smaller points go with it (while play continues after a panic the
    pause box can still be wrong until exit, and a panic inside the rewrite at exit has no test). (3) **Code signing of the Windows build, for the first
    release:** Windows Smart App Control refused to start the unsigned build on the owner's machine
    ([limit](COMPATIBILITY.md#windows-smart-app-control-blocks-the-unsigned-build)).
- **Simulation follow-up (decided: the THING-loading truncation is closed under wire 25 only).** `P_LoadThings` ends the
  whole THINGS loop at the first Doom II type (doomednums 64-69, 71, 84, 88, 89) when `gamemode != commercial`
  (p_setup.c:317-337). Wire 25 (decision T4, confirmed 2026-11-18) stops loading there, with a synthetic map and
  genuine-engine evidence; wires 12-24 still spawn a type-65 THING and everything after it, and stay as they are. The
  gap is recorded in
  [COMPATIBILITY](COMPATIBILITY.md#known-gap-thing-loading-stops-at-the-first-doom-ii-type-outside-commercial-mode);
  no Episode 1 map is affected. Wire 25 is part of the M5c pull request.
- **CI follow-up (owner ruling, 2026-11-18): split or trim the `Stable / macos-latest` job.** It took 38 to 50 minutes on the
  M5c heads; its limit was raised from 60 to 90 minutes in M5c as a stopgap. A later milestone splits the job or trims what
  it repeats, without dropping a validation step or changing a required check's name.
- Multiplayer and deathmatch, after their source/game-context dependencies can be
  independently validated. They are original behaviour, so not optional; no milestone is named
  for them yet ([checklist](#vanilla-completeness-checklist)). Later experiments follow the same
  validation rule.

The established historical labels are retained; this order expresses dependencies,
not old document append order. Current exclusions remain in [COMPATIBILITY](COMPATIBILITY.md#deferred-systems).
An exclusion of original behaviour means "not yet", never "optional": the
[vanilla completeness checklist](#vanilla-completeness-checklist) below lists every feature of the original and
where it stands.

## Vanilla completeness checklist

**The owner's standing rule (2026-11-21): nothing linuxdoom-1.10 does is optional.** A size cap decides which
milestone something lands in, never whether it lands.

The list was built file by file from the original source at the repository's upstream commit
`a77dfb96cb91780ca334d0d4cfd86957558007e0` (the directories `linuxdoom-1.10/`, `sndserv/`, `ipx/` and `sersrc/`),
so the original defines it, and every `.c` file of those four directories (62, 4, 3 and 3 files) is cited by at
least one line. A file name without a directory is in `linuxdoom-1.10/`; `file:line` points at the function,
option or table meant. One line per item: what it is, where it lives, and its status as recorded in this roadmap
and in [COMPATIBILITY](COMPATIBILITY.md) on 2026-11-21:

- **done (M…)**: delivered by that milestone, within the support and limits COMPATIBILITY gives it. "Baseline"
  means the merged foundations at the top of [Done](#done).
- **partly done (M…)**: what is done, then what is not.
- **in M4c (proposed, pending the owner)**: the one milestone this roadmap already names for unfinished
  original behaviour (pause, easy skill and the reborn reload, under [Later](#later)). Two broader directions
  under Later are also named where they apply: "M4 — Demo compatibility" and the M3e direction for moving
  sectors.
- **not started**: no milestone is named for it yet. It is not optional; it waits for the owner to name one.
- **needs the owner's ruling**: code in the source that may not belong in a port (a DOS program, a separate
  process, a build option, id's in-house paths). It is listed so nothing is lost; the reason is given.

Work still on a branch is not counted until it is merged: M6a (Episode 1 from start to finish,
[PR #65](https://github.com/Copgunn/DOOM/pull/65)) covers the intermission, the next map and the finale of Episode 1,
which stay "not started" here until then. Not items: code the source compiles out or never calls (the sliding
doors at `p_doors.c:557-764` are inside `#if 0`; the colour-fade wipe at `f_wipe.c:73-129` is never chosen,
because `D_Display` asks only for the melt at `d_main.c:339`), and an end-of-game text screen, which
linuxdoom-1.10 does not have (no file in the source refers to one).

### Start-up, game modes and files

| What | C files | Status |
| --- | --- | --- |
| Game mode from the IWAD's file name: shareware, registered, retail, commercial (Doom II, Plutonia and TNT all run as commercial) | `d_main.c:563` (`IdentifyVersion`), `doomdef.c`, `doomstat.c` | partly done (M5b): `doom-play` takes shareware, registered or retail from the file name; the Doom II, Plutonia, TNT and French names not started |
| French Doom II: its file name sets the language, which changes only which quit message is shown | `d_main.c:658-667`, `m_menu.c:1100-1105` | not started |
| French text, compiled only with a `-DFRENCH` define | `dstrings.h:34`, `d_french.h` | needs the owner's ruling: the shipped `Makefile:9` does not set the define |
| Finding the IWAD through `DOOMWADDIR`, and the configuration file `~/.doomrc` through `HOME` | `d_main.c:575-615` | not started |
| WAD files: the directory, lump lookup and caching | `w_wad.c:292`, `w_wad.c:325`, `w_wad.c:351` | done (baseline) |
| Extra WAD files (`-file`), the "modified game" notice that waits for Enter, the shareware refusal and the registered-lump check | `d_main.c:938-946`, `d_main.c:1024-1061`, `w_wad.c:141` | not started |
| Response files (`@file` on the command line) | `d_main.c:722` (`FindResponseFile`) | not started |
| Start-up text: the version banner, the shareware or commercial notice, the start-up messages | `d_main.c:817-875`, `d_main.c:1064-1113` | not started |
| The command line itself | `i_main.c`, `m_argv.c:41` (`M_CheckParm`) | not started: `doom-play` has its own options; the next table says where one stands in for an original option |
| The configuration file: the defaults table (keys, mouse, volumes, screen size, detail, gamma, chat macros, sound server, memory), read at start, written at quit | `m_misc.c:234`, `m_misc.c:340`, `m_misc.c:308`, `i_system.c:116` | not started (`doom-play` reads no configuration file, M5b decision S7) |
| The memory zone and its size (`mb_used`) | `z_zone.c`, `i_system.c:71-76` | needs the owner's ruling: Rust ownership replaces it; only its errors and its memory layout can be observed |
| Error exits (`I_Error`) | `i_system.c:162` | partly done (M5c): the errors the admitted features reach are refusals that keep the state (decision T11) |
| The clock and quitting: `I_GetTime` at 35 tics a second; `I_Quit` leaves the net game, stops sound and writes the configuration | `i_system.c:88`, `i_system.c:116` | partly done (M5b): the app's own loop runs 35 tics a second; the net and configuration steps of quitting not started |

### Command-line options

Every `M_CheckParm` call in `linuxdoom-1.10/`; the DOS drivers' own options are under
[multiplayer](#multiplayer-and-deathmatch).

| Option | C files | Status |
| --- | --- | --- |
| `-nomonsters` | `d_main.c:808` | not started |
| `-respawn` | `d_main.c:809` | not started |
| `-fast` | `d_main.c:810` | not started |
| `-devparm`: the development banner, F1 screenshots, the tic dots | `d_main.c:811`, `m_menu.c:1511`, `i_video.c:361` | not started |
| `-deathmatch`, `-altdeath` | `d_main.c:812-815` | not started |
| `-skill` | `d_main.c:967` | partly done (M5b): `doom-play --skill` picks medium or hard; the option itself not started |
| `-episode` | `d_main.c:974` | not started |
| `-warp` | `d_main.c:997` | partly done (M5b): `doom-play --map` starts E1M1-E1M9 directly; the option itself not started |
| `-turbo` | `d_main.c:885` | not started (the turbo message is done; see [single-player](#playing-a-single-player-game)) |
| `-file` | `d_main.c:938` | not started |
| `-playdemo` | `d_main.c:948`, `d_main.c:1135` | partly done (M4b): see [demos](#demos); the option itself not started |
| `-timedemo` | `d_main.c:951`, `d_main.c:1143`, `g_game.c:1625` | not started |
| `-record` | `d_main.c:1127`, `g_game.c:1530` | partly done (M5b): every recorded `doom-play` session is a version-110 demo; the option itself not started |
| `-maxdemo` | `g_game.c:1539` | not started |
| `-loadgame` | `d_main.c:1150` | not started |
| `-timer`, `-avg` | `d_main.c:982-995`, `p_spec.c:1253-1264` | not started |
| `-config` | `m_misc.c:357` | not started |
| `-debugfile` | `d_main.c:359`, `d_net.c:166` | not started |
| `-nodraw`, `-noblit` | `g_game.c:1627-1628`, `d_main.c:209` | not started |
| `-net`, `-port`, `-dup`, `-extratic` | `i_net.c:255-316` | not started |
| `-statcopy` | `d_main.c:1116`, `g_game.c:1137` | needs the owner's ruling: it passes a memory address to an external DOS statistics program |
| `-cdrom` | `d_main.c:877`, `g_game.c:1278`, `m_menu.c:520`, `m_menu.c:583` | needs the owner's ruling: it writes to the DOS directory `c:\doomdata` |
| `-shdev`, `-regdev`, `-comdev`, `-wart` (with the reloadable `~` WAD files) | `d_main.c:617-656`, `d_main.c:909-936`, `w_wad.c:236` | needs the owner's ruling: they load from id Software's in-house development directories |
| `-2`, `-3`, `-4`, `-disp`, `-geom`, `-grabmouse` | `i_video.c:719-744` | needs the owner's ruling: options of the X11 window (see [platform](#platform)) |

### Title screen, demo loop and menus

| What | C files | Status |
| --- | --- | --- |
| The title and demo loop: title, credits and help pages by game mode, the demos in turn, their timing, the title music | `d_main.c:423-530` | not started |
| Any key during the demo loop opens the menu | `g_game.c:521-532` | not started |
| The main menu (new game, options, load, save, read this, quit), the skull cursor and the menu sounds | `m_menu.c:856`, `m_menu.c:1740` (`M_Drawer`), `m_menu.c:1834` (`M_Ticker`), `m_menu.c:1847` (`M_Init`) | not started |
| The episode menu, with the shareware and registered limits | `m_menu.c:893-948`, `m_menu.c:1866-1888` | not started |
| The skill menu, with the nightmare confirmation | `m_menu.c:867-917` | not started |
| The options menu: end game, messages on or off, detail, screen size, mouse sensitivity, sound volumes | `m_menu.c:800-850`, `m_menu.c:951-1180` | not started |
| The load and save menus: six slots with descriptions | `m_menu.c:511-680` | not started |
| Quick save and quick load | `m_menu.c:681-750` | not started |
| The help pages ("read this"), by game mode | `m_menu.c:752-800`, `m_menu.c:1030-1046` | not started |
| Quitting: the confirmation, a random quit message, a quit sound | `m_menu.c:1054-1110`, `dstrings.c:35` | not started |
| The function keys F1-F11 and the `-` and `=` screen-size keys | `m_menu.c:1511-1610` | not started |
| Message boxes and the thermometer bars | `m_menu.c:1183`, `m_menu.c:1228` | not started |
| Menu control by joystick and mouse | `m_menu.c:1362-1440` | not started |
| The open menu pauses a single-player game | `p_tick.c:138-145` | not started |

### In-game screen

| What | C files | Status |
| --- | --- | --- |
| The 3D view: walls, floors and ceilings, sky, sprites, weapon sprites, the Spectre's fuzz, light levels, the light-amplification colormap | `r_main.c:870` (`R_RenderPlayerView`), `r_bsp.c`, `r_segs.c`, `r_plane.c`, `r_things.c`, `r_draw.c`, `r_sky.c`, `r_data.c` | done (M5a): high detail, screen sizes 9-11 |
| Screen sizes 3-8, with the view border | `r_main.c:658` (`R_SetViewSize`), `r_draw.c:731`, `r_draw.c:843` | not started (M5a draws sizes 9-11 and never the border) |
| Low detail | `r_draw.c:211`, `r_draw.c:643`, `r_main.c:654` | not started (M5a refuses it: this source's low-detail column drawer is broken, decision R2) |
| The invulnerability colormap | `p_user.c:362-369` | not started |
| Player colours 2-4 | `r_draw.c:385`, `r_draw.c:459` | not started (M5a refuses it) |
| The status bar: numbers, keys, weapons owned, the face and its idle glance | `st_stuff.c:1108` (`ST_Drawer`), `st_stuff.c:988` (`ST_Ticker`), `st_lib.c` | done (M5b; the glance with sound in M5c) for single player |
| Palette flashes for damage, pickups, berserk and the radiation suit | `st_stuff.c:1000` | partly done (M5b): every palette wire 24 can reach; the berserk term waits for berserk |
| The HUD message line | `hu_stuff.c:486`, `hu_stuff.c:505`, `hu_lib.c` | done (M5b) |
| Messages on or off, and Enter to show the last message again | `hu_stuff.c:659`, `m_menu.c:978`, `m_menu.c:1582` | not started |
| Gamma correction | `v_video.c:51`, `m_menu.c:109`, `m_menu.c:1597` | partly done (M5b): `--gamma 0-4` picks the five genuine tables; F11 and its message not started |
| Drawing 2D pictures | `v_video.c:204`, `v_video.c:271-493` | partly done (M5b): everything the status bar and HUD draw; flipped pictures (the cast call) and the rest not started |
| The automap: follow, pan, zoom, grid, marks, the map title, the computer map's display | `am_map.c:614` (`AM_Responder`), `am_map.c:805`, `am_map.c:1332`, `hu_stuff.c:442-470` | not started |
| Map names for the automap title, for each game | `hu_stuff.c:115-286` | not started |
| Screenshots (PCX files) | `m_misc.c:442`, `m_misc.c:503`, `g_game.c:970` | not started |
| The screen melt between screens | `f_wipe.c:141-227`, `d_main.c:222-344` | partly done (M5c): the melt's 320 `M_Random` draws at the level start are accounted for (decision T6); the melt picture and its timing not started |

### Sound and music

| What | C files | Status |
| --- | --- | --- |
| Sound effects: which sound starts, on which channel, with which priority, volume, stereo position and pitch | `s_sound.c:255`, `s_sound.c:519`, `s_sound.c:753`, `sounds.c` | done (M5c) |
| The in-process mixer at 11,025 Hz, and handing its buffers to the device | `i_sound.c:539`, `i_sound.c:666`, `i_sound.c:738` | done (M5c): the reference is linuxdoom-1.10 with its in-process sound code (decision T2); what the device adds is presentation |
| The separate sound server the shipped build uses | `sndserv/soundsrv.c`, `sndserv/linux.c`, `sndserv/sounds.c`, `sndserv/wadread.c`, `i_sound.c:62-65`, `doomdef.h:84` | needs the owner's ruling: a separate process; M5c's reference is the in-process mixer (decision T2) |
| Timer-driven sound output (`SNDINTR`) | `i_sound.c:917-985`, `doomdef.h:85` | needs the owner's ruling: a build option the shipped build leaves off |
| The sound-effect and music volume settings | `s_sound.c:616`, `s_sound.c:631`, `m_menu.c:816-850` | not started (`doom-play --volume` is the app's own option) |
| Music selection: the song of each map, the intermission, the finale and the title; pausing; `idmus` | `s_sound.c:497-515`, `s_sound.c:644-706`, `sounds.c` | not started |
| Music playback | `i_sound.c:830-890`, `sndserv/linux.c:67` | needs the owner's ruling: linuxdoom-1.10 plays no music (these functions are empty), so there is no reference to compare a music player with |

### Playing a single-player game

| What | C files | Status |
| --- | --- | --- |
| Skill: Hurt Me Plenty (medium) | `p_mobj.c:743-751` | done (M3h9c, M4a) |
| Skill: Ultra-Violence (hard) | `p_mobj.c:743-751` | done (M4b) |
| Skills: I'm Too Young To Die (double ammunition, half damage) and Hey, Not Too Rough | `p_mobj.c:743-751`, `p_inter.c:95-101`, `p_inter.c:799-800` | in M4c (proposed, pending the owner), which names "easy skill" |
| Skill: Nightmare: fast monsters, respawning monsters, double ammunition, its own chase rules | `g_game.c:1416-1436`, `p_mobj.c:357` (`P_NightmareRespawn`), `p_enemy.c:672` (`A_Chase`), `m_menu.c:898` | not started |
| A new game: the skill and episode limits of each game mode, the random reset, each episode's sky | `g_game.c:1345`, `g_game.c:1365` (`G_InitNew`) | partly done (M4b, M5b): a new game straight onto an Episode 1 map on medium or hard; the rest not started |
| The level exit and the secret exit | `g_game.c:1002`, `g_game.c:1009` | done (M3h8, M4a): the exit tic completes and is recorded; the next tic is refused until the intermission exists |
| Finishing a level: the players finish, statistics and par times go to the intermission | `g_game.c:779`, `g_game.c:978-998`, `g_game.c:1020` | not started |
| The next map: the normal order, the secret level and the way back, Doom II's maps 15, 31 and 32 | `g_game.c:1064-1110`, `g_game.c:1147-1179` | not started |
| What the player carries to the next map (health, armour, weapons, ammunition, backpack) and loses (keys, powers) | `g_game.c:779-792`, `p_mobj.c:642` | not started |
| Death and the reborn reload | `p_user.c:182` (`P_DeathThink`), `g_game.c:800`, `g_game.c:924` (`G_DoReborn`) | partly done (M3h9c): the death view and the reborn request; the reload is in M4c (proposed, pending the owner) |
| The original's pause: the Pause key, the pause command in the demo, the stopped game, the pause picture, paused music | `g_game.c:426-430`, `g_game.c:561-565`, `g_game.c:698-712`, `p_tick.c:135-136`, `d_main.c:303-311`, `s_sound.c:497` | in M4c (proposed, pending the owner); `doom-play`'s Esc pause is the app's own (M5b) |
| The turbo message | `g_game.c:671-679` | done (M4a) |
| The game states and what each draws (level, intermission, finale, demo screen) | `g_game.c:605` (`G_Ticker`), `d_main.c:193` (`D_Display`) | partly done (M5b): the level; the others not started |

### Intermission and finale

| What | C files | Status |
| --- | --- | --- |
| The single-player tally: kills, items, secrets, time and par, with the counting sounds and the skip on a press | `wi_stuff.c:1318-1468`, `wi_stuff.c:1470` | not started |
| The maps of episodes 1-3: "you are here", the marks on finished levels, the animated backgrounds (whose drawing never runs, `wi_stuff.c:588`, while their random draws do) | `wi_stuff.c:421-602`, `wi_stuff.c:752-816` | not started |
| The intermission of episode 4 and Doom II (no map) | `wi_stuff.c:771-811`, `wi_stuff.c:1538-1706` | not started |
| The intermission music | `wi_stuff.c:1503-1515` | not started |
| The Doom 1 finale: each episode's text crawl | `f_finale.c:96-191`, `f_finale.c:261` | not started |
| The Doom 1 end screens: episode 1's picture (by game mode), episode 2's, episode 3's bunny scroll, episode 4's | `f_finale.c:644`, `f_finale.c:700-738` | not started |
| The Doom II text screens after maps 6, 11, 20 and 30 and the secret maps 15 and 31 | `f_finale.c:142-191`, `g_game.c:1154-1168` | not started |
| The Doom II cast call, with its sounds | `f_finale.c:377-643` | not started |
| The finale music | `f_finale.c:96-191` | not started |

### Saving and loading

| What | C files | Status |
| --- | --- | --- |
| Saving a game: six slots, a description, a version string, the 0x2c000-byte buffer limit | `g_game.c:74-75`, `g_game.c:1261-1320` | not started |
| What a save holds: players, the world, thinkers and specials | `p_saveg.c:47-586` | not started |
| Loading a game, with the version check | `g_game.c:1192-1252` | not started |
| Saving through the game command, so a network game saves on every machine | `g_game.c:435`, `g_game.c:714-720` | not started |

### Demos

| What | C files | Status |
| --- | --- | --- |
| Recording a demo (version 110) | `g_game.c:1506`, `g_game.c:1549` | partly done (M5b): every recorded `doom-play` session; `q` to stop (`g_game.c:1508-1509`) and the buffer limit not started |
| Playing a demo back | `g_game.c:1491`, `g_game.c:1582-1622` | partly done (M4b): Episode 1 demos within what wire 24 admits, `DEMO1` among them; general playback is the "M4 — Demo compatibility" direction under [Later](#later) |
| The end of a demo, the `-timedemo` result and quitting after `-playdemo` | `g_game.c:1647` | not started |

### Cheats

| What | C files | Status |
| --- | --- | --- |
| The cheat matcher (scrambled key sequences) | `m_cheat.c` | not started (`doom-play` passes cheat keys on as ordinary keys, M5b) |
| `iddqd`: god mode | `st_stuff.c:549-562` | not started |
| `idfa` and `idkfa`: weapons, ammunition, armour and keys | `st_stuff.c:564-593` | not started |
| `idmus`: change the music | `st_stuff.c:595-623` | not started |
| `idspispopd` and `idclip`: walk through walls | `st_stuff.c:625-634`, `p_user.c:242` | not started |
| `idbehold` and its six powers | `st_stuff.c:636-655` | not started |
| `idchoppers` | `st_stuff.c:657-662` | not started |
| `idmypos` | `st_stuff.c:664-674` | not started |
| `idclev`: change level | `st_stuff.c:676-721` | not started |
| `iddt`: the whole map and its things on the automap | `am_map.c:287`, `am_map.c:701-706` | not started |

### Multiplayer and deathmatch

| What | C files | Status |
| --- | --- | --- |
| Starting a network game and keeping the tics in step: packets, resending, `ticdup` | `d_net.c:368` (`NetUpdate`), `d_net.c:476`, `d_net.c:555`, `d_net.c:602`, `d_net.c:636` (`TryRunTics`) | not started |
| The Linux UDP network driver | `i_net.c:90-348` | not started |
| The DOS IPX driver (`-nodes`, `-port`, `-vector`) | `ipx/IPXSETUP.C`, `ipx/IPXNET.C`, `ipx/DOOMNET.C` | needs the owner's ruling: a separate DOS program that starts DOOM and talks to it through an interrupt vector |
| The DOS serial and modem driver (`-com2` to `-com4`, `-port`, `-irq`, `-8250`, `-dial`, `-answer`, the baud rates) | `sersrc/SERSETUP.C`, `sersrc/PORT.C`, `sersrc/DOOMNET.C` | needs the owner's ruling: a separate DOS program that drives the serial port's hardware |
| The consistency check between players | `g_game.c:681-695` | not started |
| Co-operative play: up to four players, their starts, respawning at a start, the body queue | `g_game.c:210`, `g_game.c:844` (`G_CheckSpot`), `g_game.c:924-966`, `p_mobj.c:740` | not started |
| Deathmatch and altdeath: random starts, weapons that stay, items that come back, frags | `g_game.c:897`, `p_setup.c:109`, `p_mobj.c:578`, `p_inter.c:168`, `p_inter.c:668` | not started |
| The level timer (`-timer`, `-avg`) | `p_spec.c:1092-1097`, `p_spec.c:1253-1264` | not started |
| Chat, with its macros and destinations | `hu_stuff.c:65`, `hu_stuff.c:587`, `hu_stuff.c:617` | not started |
| Watching another player (F12) | `g_game.c:506-518` | not started |
| The multiplayer status bar: frags and the face's background colour | `st_stuff.c:499-507`, `st_stuff.c:971` | not started |
| The multiplayer intermission: the co-operative and deathmatch tables | `wi_stuff.c:818-1316` | not started |
| Network demos | `g_game.c:1582-1622` | not started |

### Episodes 2 to 4

| What | C files | Status |
| --- | --- | --- |
| The Lost Soul | `p_enemy.c:1419` (`A_SkullAttack`) | not started |
| The Cacodemon | `p_enemy.c:950` (`A_HeadAttack`) | not started |
| The Cyberdemon | `p_enemy.c:969`, `p_enemy.c:1759`, `p_enemy.c:1765` | not started |
| The Spider Mastermind | `p_enemy.c:821`, `p_enemy.c:882`, `p_enemy.c:1765` | not started |
| The boss deaths of E2M8, E3M8, E4M6 and E4M8 | `p_enemy.c:1609` (`A_BossDeath`) | partly done (M4b): E1M8's; the rest not started |
| The plasma rifle and the BFG 9000 | `p_pspr.c:564`, `p_pspr.c:578`, `p_pspr.c:781`, `p_pspr.c:818` | not started |
| Items wire 24 does not admit: berserk, invulnerability, skull keys, cells | `p_inter.c:339` (`P_TouchSpecialThing`) | not started |
| The skies of episodes 2-4 | `g_game.c:1464-1479` | not started |
| Switch textures by game mode (registered: episodes 1-2; retail: episode 1 only; commercial: all) | `p_switch.c:107-131` | partly done (M4a): the episode 1 list; a texture only in another list is refused (decision D3) |
| Episode 4 (retail only) and the shareware limit of one episode | `g_game.c:1389-1403`, `m_menu.c:919-948` | not started |

### Doom II

| What | C files | Status |
| --- | --- | --- |
| The Heavy Weapon Dude (Chaingunner) | `p_enemy.c:845`, `p_enemy.c:865` | done (M3h1) |
| The Hell Knight | `p_enemy.c:979` (`A_BruisAttack`), `info.c` | not started |
| The Revenant | `p_enemy.c:1002-1120` | not started |
| The Mancubus | `p_enemy.c:1351-1418` | not started |
| The Arachnotron | `p_enemy.c:898` (`A_BspiAttack`), `p_enemy.c:1771` | not started |
| The Pain Elemental | `p_enemy.c:1512`, `p_enemy.c:1522` | not started |
| The Arch-vile and resurrection | `p_enemy.c:1129-1350` | not started |
| The Wolfenstein SS | `info.c` (`MT_WOLFSS`) | not started |
| Commander Keen and its door | `p_enemy.c:566` (`A_KeenDie`) | not started |
| The boss brain and the cube spawner | `p_enemy.c:1813-1993` | not started |
| The super shotgun and its weapon-change rule | `p_pspr.c:366`, `p_pspr.c:696`, `p_enemy.c:1778-1810`, `p_user.c:298-304` | not started |
| The megasphere | `p_inter.c:409` | not started |
| MAP07's boss deaths | `p_enemy.c:1609` (`A_BossDeath`) | not started |
| The Doom II line specials (blazing doors and lifts, fast stairs and the rest); the W1 blazing door 109 is done (M3h8) | `p_spec.c:492`, `p_switch.c:276`, `p_doors.c:264`, `p_plats.c:139`, `p_floor.c:453` | not started |
| The Doom II skies by map range | `g_game.c:1455-1463` | not started |
| The Doom II map names (Plutonia's and TNT's are compiled out, so those games show Doom II's) | `hu_stuff.c:169-286`, `hu_stuff.c:453-460` | not started |
| THING loading stops at the first Doom II type outside commercial mode | `p_setup.c:317-337` | done (M5c, wire 25) |

### Simulation systems

| What | C files | Status |
| --- | --- | --- |
| Fixed-point arithmetic, the trigonometry tables, the two random streams | `m_fixed.c`, `tables.c`, `m_random.c:57-72` | done (baseline; `M_Random` with M5b and M5c) |
| Bounding boxes and byte order | `m_bbox.c`, `m_swap.c` | done (baseline) |
| Thinkers and the order of a tic | `p_tick.c:53-157` | done (baseline, M3h7) |
| Loading a map | `p_setup.c:122-700` | done (M3h6, M3h9c, M4a) for single player |
| Spawning map things: the skill bits and the multiplayer-only things | `p_mobj.c:708` (`P_SpawnMapThing`), `p_mobj.c:740-751` | partly done (M3h9c, M4b): single player on medium and hard |
| Movement, collision, sliding, use, hitscans, radius damage and the crush check | `p_map.c`, `p_maputl.c` | done (baseline-M4b) for every admitted thing, with the refusals listed under [limits](#limits-overflows-and-quirks) |
| Sight | `p_sight.c:135`, `p_sight.c:257` | done (baseline-M3h6) |
| Thing physics, missiles, puffs and blood | `p_mobj.c:90-546`, `p_mobj.c:812-988` | done (baseline-M4b) for every admitted thing |
| The player think: movement, the view, powers, the weapon change | `p_user.c:77-386` | partly done (M3g4, M3h9c): everything wire 24 admits; walking through walls, invulnerability, berserk and the super shotgun rule not started |
| The weapons of Episode 1: fist, chainsaw, pistol, shotgun, chaingun, rocket launcher | `p_pspr.c:282-760`, `d_items.c` | done (M3g7, M3g8, M4a) |
| Monster thinking: looking, chasing, attacking, pain and death | `p_enemy.c:174-782` | done (baseline-M4b) for the seven monsters wire 24 admits |
| The monsters of Episode 1: Zombieman, Shotgun Guy, Imp, Demon, Spectre, Baron of Hell (with E1M8's boss death) | `p_enemy.c:802`, `p_enemy.c:821`, `p_enemy.c:913`, `p_enemy.c:935`, `p_enemy.c:979`, `p_enemy.c:1609`, `info.c` | done (M3g6, M3h1, M3h2, M4a, M4b) |
| Pickups, damage, death and drops for Episode 1 | `p_inter.c:74-918` | done (M3g9-M4a) for what wire 24 admits |
| Exploding barrels | `p_enemy.c:1598` (`A_Explode`), `p_map.c:1164` | done (M3h9b) |
| Doors | `p_doors.c:63-499` | partly done (M3h8, M4a): every Episode 1 kind; the rest not started |
| Floors, stairs and the donut | `p_floor.c:209-555`, `p_spec.c:1163` | partly done (M3h8, M4a): every Episode 1 kind; the rest not started |
| Lifts and platforms | `p_plats.c:52-314` | partly done (M3h8, M4a): the three Episode 1 kinds; the rest not started |
| Ceilings and crushers | `p_ceilng.c:52-335` | not started (listed under [Later](#later) as part of the M3e direction, which has no milestone yet) |
| Lights | `p_lights.c:46-357` | partly done (M3h7, M4a): every sector light and line 35; the other light lines not started |
| Switches and buttons | `p_switch.c:155-654` | partly done (M3h8, M4a): Episode 1's switches; the rest not started |
| Teleporters | `p_telept.c:48` (`EV_Teleport`) | done (M4b) |
| Line specials by crossing, use and shot | `p_spec.c:492`, `p_switch.c:276`, `p_spec.c:959` | partly done (M3h8-M4b): the 38 specials Episode 1 uses; the rest not started |
| Sector specials | `p_spec.c:1009`, `p_spec.c:1239` | partly done (M3h7, M4a): all but 10 (a door that closes after 30 seconds) and 14 (one that opens after 5 minutes) |
| Animated walls and floors, the scrolling wall | `p_spec.c:148`, `p_spec.c:1083-1160` | partly done (M3h7, M5a): what the Episode 1 maps show; the rest not checked |

### Platform

| What | C files | Status |
| --- | --- | --- |
| The X11 window with shared memory, the palette upload, 2x to 4x scaling | `i_video.c:352`, `i_video.c:538-594`, `i_video.c:692`, `i_video.c:920-930` | needs the owner's ruling: `doom-play` has its own window (M5b); the frame and palette it shows are the original's |
| Keyboard and mouse events from X11: key translation, mouse grab and motion | `i_video.c:97`, `i_video.c:194`, `i_video.c:309` | partly done (M5b): the key translation was checked against a declared stand-in for Xlib; the app feeds its own event stream (decision S4) |
| Building each tic's command from keys and mouse, double-click use included | `g_game.c:237` (`G_BuildTiccmd`), `g_game.c:504` (`G_Responder`) | done (M5b), cheat keys excepted |
| The joystick | `g_game.c:583-590`, `m_menu.c:1362` | needs the owner's ruling: the game and the menus handle joystick events, but no linuxdoom-1.10 driver produces them |

### Limits, overflows and quirks

| What | C files | Status |
| --- | --- | --- |
| 128 intercepts | `p_local.h:154`, `p_maputl.c:562` | partly done (M3h6): the tic that would write the 129th is refused (a declared fidelity gap) |
| 8 special lines crossed in one move | `p_map.c:67` | partly done (M3h8): the ninth is refused |
| A long player slide that jumps a wall corner | `p_map.c:695` (`P_SlideMove`) | partly done (M3h5): refused |
| A Baron fireball that passes through a thin wall | `p_map.c:191-195` | partly done (M4b): refused ([COMPATIBILITY](COMPATIBILITY.md#the-baron-fireball-wall-tunnel-wire-24)) |
| 128 visplanes, 20,480 openings, 32 solid segments | `r_plane.c:52`, `r_plane.c:59`, `r_bsp.c:88` | partly done (M5a): refused with a named witness |
| 256 drawsegs and 128 vissprites, dropped silently | `r_defs.h:55`, `r_things.h:31` | done (M5a) |
| Tables that stop the original when full: 30 lifts, 30 ceilings, 16 buttons | `p_spec.h:244`, `p_spec.h:306`, `p_spec.h:518` | partly done (M3h8): more than 30 lifts or 16 switches are refused at load; ceilings not started |

## Ideas under consideration

These are ideas, not commitments. None is scheduled, and each would need its own
investigation, owner decision and evidence before it became a milestone. The exception is
anything the original does (original demo playback, the full original game and Doom II): by the
owner's standing rule of 2026-11-21 it is not optional, only its milestone is still open (see the
[vanilla completeness checklist](#vanilla-completeness-checklist)). They are listed
here so that the README can summarize them in one place; the README's "Where it's (maybe)
going" list changes only with the owner's approval. They fall into three tracks.

The README's "Which kind of Doom?" section places the project among the four ways classic
Doom is played today, and the tracks below map onto it: **vanilla** (the original's
behavior, limits and quirks) is what the per-milestone policies implement and what Track
1 keeps exact; **limit-removing** is Track 1's separate profile; **Boom, MBF21 and ID24**
(the successive extensions of the original map format that most new community maps
target, ID24 being id Software's 2024 extension) is a possible later profile under Track
3, not a promise; and **advanced, ZDoom-style** engines are what Track 3 would build its
own way rather than copy. Every kind is a separate opt-in mode, so vanilla never changes.

### Track 1: Faithful

Vanilla behavior stays exact forever; everything here either proves it or builds on it.

- **Original demo playback.** Play back recorded vanilla demos (`.lmp` files, the
  original game's recordings of player input) without desyncing. A demo replays only if
  every tic matches, so it is the strongest end-to-end proof of exactness. It builds on
  the M4 demo-compatibility direction under [Later](#later), with DOS and Linux claims
  kept separate. Playing back the original's demos is original behaviour, so not optional.
- **Porting whole systems at once.** Port complete systems in one step (all monsters, all
  line types, all weapons) and check them by running real maps and demos against the
  original C. The detailed per-mechanic C evidence would be kept for the tricky quirks
  rather than written for every behavior.
- **One complete vanilla profile, plus a limit-removing profile.** Eventually fold the
  per-milestone policies into one complete vanilla profile. A separate limit-removing
  profile would lift the original's hard limits (visplanes, intercepts, things and the
  like) without otherwise changing gameplay. The per-milestone policies stay reproducible.
- **An optional "fixed" mode.** A separate opt-in mode would correct known original bugs.
  The fidelity gaps and quirks documented in [compatibility](COMPATIBILITY.md) become its
  to-do list, much like the compatibility levels of other source ports.
- **The full original game and Doom II.** After Episode 1, the remaining episodes and then
  Doom II, each with the same source-first evidence and exactness. Not optional: they are what
  the original does; no milestone is named for them yet.

### Track 2: Presentation

How it looks and feels. None of this can change gameplay, because the renderer only
reads game state, which the architecture already requires.

- **A 1:1 vanilla-faithful software renderer first**, so the game looks exactly like the
  original Linux DOOM 1.10 renderer, the same source reference the simulation follows. DOS
  presentation claims would need their own reference.
- **Then a modern renderer.** The simulation still runs at 35 tics per second while the
  renderer interpolates between tics, giving smooth high frame rates on any display, plus
  sector-based dynamic lighting. Other ports already flood-fill exactly which sectors a
  light touches, then shade every pixel in them anyway; this idea stops at the sector
  step, changing each touched sector's light, which costs almost nothing.
- **VR.** A headset handles looking around while the mouse turns and aims. This suits
  Doom, whose original has no vertical aiming, so head movement need not change gameplay.
- **A fully offline engine.** Play your own copy of Doom with your own game files, with
  no internet connection, account, launcher or login ever required. (In 2019 the official
  console and mobile re-releases briefly required an online account login to play; it was
  reversed within days after player backlash.)

### Track 3: Beyond vanilla

A new engine built on the verified core, in separate opt-in modes so that vanilla is
never affected. This is the reason for the painstaking verification: an exact Doom core
that wild new things can be built on without losing what makes Doom feel like Doom.

- **Boom, MBF21 and ID24 map features** (new door, lift and scrolling-floor types, deep
  water and the rest of those map-format extensions) as a possible later profile, not a
  promise; the owner decides whether and when.
- **ZDoom-style features done cleanly**, without decades-old code underneath, going the
  project's own way rather than copying ZDoom.
- **Simulation features the original never had**, such as fluid simulation (water, blood
  and nukage that flow) and very large monster crowds.
- **A scripting layer** for new game logic, and **other platforms** such as mobile.
- **A live editor with rewind.** Change anything while the game runs (move a monster, edit
  a sector, alter a rule), then rewind and replay from any moment. Because the simulation
  is deterministic with a small, fully canonical state, a rewind is a restore plus a replay
  of recorded commands.
- **"Why did that happen?"** Click a monster and see the exact decisions and random rolls
  behind what it did: which sound woke it, which state it chose, which draw of the random
  stream set its damage. The canonical state and event streams already record much of this.
- **TAS tools that are vanilla-legal by construction.** Build a run frame by frame here, with
  every input restricted to what the original game accepts, so the result plays back on the
  original executable as a demo.
- **Map and demo checks.** Report exactly which line, sector or thing breaks a vanilla limit,
  and which recorded demos a map change would desync, by replaying them before and after.
- **Headless bot playthroughs.** Thousands of automated runs of a map, aggregated into
  encounter heatmaps (where players die, which fights are decisive, which areas go unseen).
- **Edits as replayable operations, with bisect.** Record each edit as an operation applied
  to a timeline, so a set of edits can be replayed, reordered and bisected to find the one
  that broke a demo or a run.
- **A divergence view.** Compare a modded timeline against verified vanilla and show the
  first tic and the first state that differ, and why.

Edits made through any of these tools always live in a mod layer or a branched timeline,
never in vanilla: the verified vanilla core stays unchanged and every branch can be
compared with it. On prior art, deterministic rewind editors already exist inside
proprietary engines; what is proposed here is an open-source one built on a Doom core whose
vanilla behavior is verified against the original.
