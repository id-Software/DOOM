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
  Episode 1 need more actor slots (the M3h9b note's §5.3 formula) and their own evidence;
  easy and hard skill (E4 option ii, deferred by decision F10).

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

- **M4b (in progress on `feat/m4b-episode-1-complete`; decisions I1–I13 confirmed by the owner on 2026-11-10; see the
  [investigation note](M4B_REFERENCE_INVESTIGATION.md)).** Teleportation (line 97 and the landing thing), the
  Baron and `A_BossDeath`, hard skill, and with them E1M5, E1M8 and E1M9; the `DEMO1`
  (E1M5, skill 3) of the four-episode `DOOM.WAD`, which the genuine engine runs as registered because of its file
  name, becomes runnable here. Plan steps 1–14 are done (the step 14 independent review found one behavior difference, a walking monster's teleport fog z, fixed; see the [review record](validation/M4B.md#independent-review-step-14)): wire 24 (`PlayerMonstersEpisode1V2`, DSIM40) admits
  teleportation, the Baron, `A_BossDeath` and hard skill, and E1M5, E1M8 and E1M9 load on skills 2 and 3; the
  real-map runs (61, all matched against the genuine engine, local-only) and `DEMO1` are recorded in the
  [evidence record](validation/M4B.md#real-maps-and-demo1); the Baron fireball wall tunnel is a declared
  refuse-the-tic witness ([COMPATIBILITY](COMPATIBILITY.md#the-baron-fireball-wall-tunnel-wire-24)); mutation
  testing, fuzzing and matched timings are recorded in the evidence record (the timings show an unexplained
  slowdown of short workloads, carried to the performance milestone below). Next: step 15, the full gates,
  then step 16, the pull request.
- **M4c (proposed, pending the owner).** Pause, easy skill and the reborn reload as an
  admitted transition; level transitions stay a later direction.
- **Simulation performance (proposed by the owner, after M4b).** Profile
  `Simulation::advance`, which costs 55–746 µs per tic on the real Episode 1 maps against
  the genuine C engine's 3.7–13.4 µs ([measurements](validation/M4A.md#simulation-only-real-map-timings)),
  and reduce it without changing any digest or DSIM byte. This includes the shared-validation
  costs deferred from M4a.
- Complete player/gameplay integration and broader monster/THING support.
- The remaining moving sectors and map specials (the established M3e direction) beyond
  the doors, lifts, floors, switches and exit of M3h8: crushers, ceilings, teleporters
  and the other triggers (stairs, locked doors and the other Episode 1 door, floor and plat
  kinds belong to M4a).
- **M4 — Demo compatibility:** version-specific playback and divergence diagnostics;
  keep DOS and Linux behavior claims separate.
- **M5 — Rendering and platform integration:** presentation/input/audio and a
  custom Vulkan-first rendering direction across supported platforms. Presentation
  must not change authoritative simulation or RNG; no renderer exists yet.
- Multiplayer and later experiments, after their source/game-context dependencies
  can be independently validated.

The established historical labels are retained; this order expresses dependencies,
not old document append order. Current exclusions remain in [COMPATIBILITY](COMPATIBILITY.md#deferred-systems).

## Ideas under consideration

These are ideas, not commitments. None is scheduled, and each would need its own
investigation, owner decision and evidence before it became a milestone. They are listed
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
  kept separate.
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
  Doom II, each with the same source-first evidence and exactness.

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
