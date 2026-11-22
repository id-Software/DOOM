# Compatibility boundaries

The behavioral authority is original id Software Linux DOOM 1.10. This is an early
headless rewrite supporting specific, limited behavior, not a complete source port.
This describes the code in this checkout, not its merge/review status. This
document owns support restrictions and limits; [ROADMAP](ROADMAP.md) owns
development status and plans. The [glossary](README.md#short-glossary) explains
profile, admission and fixture terminology.
Linux 1.10 comparisons do not prove DOS 1.9 demo synchronization.

## What a compatibility claim means

| Level | Meaning |
| --- | --- |
| Recognition | An original type, state or resource is represented as metadata |
| Validated parsing | Input records and references pass structural/safety checks |
| Supported execution | An explicitly selected configuration accepts specified actors and reachable actions |
| Original-C evidence | Specific inputs match genuine source bodies under declared adapters |
| Full-game compatibility | All enabled dependencies compose in the original game context; not established |

Loading a WAD, recognizing MT_SERGEANT or matching an isolated hash does not
establish the last level. [Reference notes](README.md) describe each admitted
slice and the [validation guide](VALIDATION.md) distinguishes evidence families.

## Stepping and diagnostics

`Simulation::advance([TicCommand; 4]) -> Result<(), SimError>` completes one tic
when unpaused. It constructs no Snapshot. `step` runs that same core and returns
one Snapshot on success; explicit `snapshot()` remains available in release.
`Snapshot::canonical_bytes` and `digest` additionally encode bytes. Opening an
execution scope or using GameInstance does not select a second stepping engine.

Both APIs preserve commands, ordered events, pending observations, thinker/slot
ordering, RNG, actor/state admission and per-operation recoverable-error atomicity.
Paused calls are no-ops before command/overflow checks: advance returns unit;
step captures unchanged state. Errors capture nothing and preserve canonical state,
including queues and RNG. This is per call, not rollback of a whole command stream
or context callback. Allocation failure outside fallible reservations remains the
existing process-level Rust policy, not a promise to recover from every OOM.

Events and gameplay/preflight/staging allocations remain enabled. This API makes
full diagnostic capture optional; it adds no gameplay profile, identity storage,
DSIM version, actor or geometry capacity. `run_commands` retains its diagnostic
collection contract. Use an ordinary loop over advance when snapshots are unwanted.
See [architecture](ARCHITECTURE.md#stepping-and-observation),
[source/cost audit](M3G1_REFERENCE.md) and [measurements](validation/M3G1.md).

## Policies

New gameplay is switched on through optional policies, each selected explicitly and adding one slice of Doom to the
one before. Older policies never change, so earlier results stay reproducible, much like compatibility levels. The
number in brackets is the policy's identifier in saved test snapshots; the earlier Demon profiles are under
[versioned profiles](#versioned-gameplay-profiles). This table is the one list of policies; the README links here.

| Policy (wire ID) | Adds | Contract |
| --- | --- | --- |
| `DemonNormalV1` (1) | The bounded Demon closure with original actor types and states stored directly | [Original-ID Demon](#original-id-demon-profile) |
| `PlayerBodyLifecycleV1` (2) | One genuine original player body: direct damage, pain, death and a retained corpse | [Player body](#original-player-body-lifecycle) |
| `PlayerBodyKnockbackV1` (3) | Physical knockback and grounded wall collision for that player | [Knockback](#original-player-grounded-knockback) |
| `PlayerCommandMovementV1` (4) | Player commands (turning, thrust) and the source reaction-time delay | [Command movement](#original-player-command-movement) |
| `PlayerDemonAcquisitionV1` (5) | One Demon that sees and pursues the active player, without melee | [Acquisition](#active-original-player-and-demon-acquisition) |
| `PlayerDemonMeleeV1` (6) | Source-timed Demon melee against the player, with damage, pain and death | [Demon melee](#autonomous-original-demon-player-melee) |
| `PlayerDemonPistolV1` (7) | The genuine pistol: psprites, ammunition, refire spread, hitscan and noise alerts | [Pistol](#genuine-player-pistol-combat) |
| `PlayerDemonWeaponsV1` (8) | Fists, shotgun, chaingun, weapon fallback and manual switching | [Weapons](#original-player-weapon-expansion) |
| `PlayerDemonPickupsV1` (9) | World ammunition, stimpacks, medikits and shotgun/chaingun pickups | [Pickups](#original-world-pickups) |
| `PlayerDemonArmorV1` (10) | Green and blue armor with source absorption and depletion | [Armor](#original-armor-and-damage-absorption) |
| `PlayerDemonBonusesV1` (11) | Health and armor bonuses (+1 up to 200) and counted-item statistics | [Bonuses](#original-health-and-armor-bonuses) |
| `PlayerMonstersV1` (12) | Former humans beside the Demon, up to eight monsters, infighting and item drops | [Former humans](#original-former-humans-and-multi-monster-encounters) |
| `PlayerMonstersImpV1` (13) | The Imp and its dodgeable fireball | [Imps](#original-imps-and-fireballs) |
| `PlayerMonstersMapScaleV1` (14) | Travel bounded by the loaded walls in walled rooms up to 2,048 units | [Map scale](#original-monsters-at-real-map-scale) |
| `PlayerMonstersHeightsV1` (15) | Static floor/ceiling heights joined by two-sided lines: steps, ledges, windows, falls | [Static heights](#original-monsters-with-static-heights) |
| `PlayerMonstersDiagonalV1` (16) | Linedefs of any direction inside a closed boundary; one corner jump refused | [Diagonal walls](#original-monsters-among-diagonal-walls) |
| `PlayerMonstersRealMapV1` (17) | WAD-format geometry limits (E1M1-sized) under a per-map arithmetic certificate | [Classic-map scale](#original-monsters-at-classic-map-scale) |
| `PlayerMonstersSpecialsV1` (18) | Sky ceilings, light thinkers, damage floors, secrets and one-sided scrolling walls | [Sky and specials](#original-monsters-with-sky-and-static-specials) |
| `PlayerMonstersMoversV1` (19) | E1M1's seven door, lift, floor, switch and exit line types; the tic after the exit is refused | [Movers](#original-monsters-with-doors-lifts-floors-switches-and-the-exit) |
| `PlayerMonstersE1M1V1` (20) | Two-sided scrolling walls and E1M1's seven static decoration types (lamps, pillars, candelabras, corpses and gore) | [E1M1 decorations](#original-monsters-with-two-sided-scrolling-walls-and-static-decorations) |
| `PlayerMonstersBarrelsV1` (21) | Exploding barrels with the original's radius attack and chain reactions, 64 pickups, every map-thing angle and the relaxed spawn rule | [Barrels](#original-monsters-with-exploding-barrels-pickup-scale-and-every-thing-angle) |
| `PlayerMonstersLevelV1` (22) | The player start at its place in the map's thing list, with its angle; the full player think, including damage-flash decay and the dead player's turn toward its attacker: the real E1M1 run | [Level runs](#original-level-runs-the-player-start-in-lump-order-and-the-full-player-think-wire-22) |
| `PlayerMonstersEpisode1V1` (23) | Episode 1, part 1: every door, floor, lift, stair, donut, light and exit special, keys, powerups, chainsaw, rocket launcher and Spectre: the real E1M2-E1M4, E1M6 and E1M7 runs | [Episode 1, part 1](#original-episode-1-part-1-every-door-floor-plat-stair-donut-light-and-exit-special-keys-items-powers-chainsaw-rocket-launcher-and-spectre-wire-23) |
| `PlayerMonstersEpisode1V2` (24) | Episode 1, part 2: teleporters, the Baron of Hell and the E1M8 boss floor, hard skill: the real E1M5, E1M8, E1M9 and `DEMO1` runs | [Episode 1, part 2](#original-episode-1-part-2-teleporters-the-baron-of-hell-a_bossdeath-and-hard-skill-wire-24) |
| `PlayerMonstersEpisode1V3` (25) | Episode 1, part 3: wire 24 plus canonical sound records, the sound origins, the corrected hard-death scream and the original's THING-loading stop; it makes no sound itself, and `doom-play` runs on it | [Wire 25](#original-episode-1-part-3-canonical-sound-records-wire-25) |

## Versioned gameplay profiles

Historical profile behavior and DSIM bytes remain frozen. New behavior is opt-in.

| Configuration (profile) | Supported foundation |
| --- | --- |
| 1 | Non-solid fixed-point headless movement and command foundations |
| 2 | Grounded player XY sliding and solid collision |
| 3 | Static multi-sector BSP/openings, XY/Z and gravity/dropoff |
| 4 | Explicit type/state lifecycle and bounded synthetic probes |
| 5 | Unbound damage, aim and hitscan |
| 6 | Bounded player weapons and psprites; complete player damage excluded |
| 7 | Bounded original plasma projectile foundations |
| 8 | Sight, ordered player scanning, sound propagation and Look |
| 9 | Non-attacking source pursuit |
| 10–12 | Separately admitted attack-selection, shotgunner and chaingunner fixtures |
| 13 | Demon melee with simplified fixture stand/run loops |
| 14 | Demon pain, ordinary death and persistent corpse lifecycle |
| 15 | Original two-state stand and eight-state run closure |
| 16 | Bounded classic Demon THING spawning, retaining the prior fixture flags |
| 17 | Map kill totals, original COUNTKILL flags and bounded death-credit context |
| 18 | Expanded bounded static geometry, unbound XY/Z and read-only original sight |
| 19 | Checked larger coordinate envelope and spatial work for unbound static actors |
| 20 | Explicit intersection of larger static geometry and adapted Demon gameplay |
| OriginalDemon | Same bounded Demon closure with actual original IDs and explicit context; DSIM21/profile wire ID 1 |

These are separate contracts, not modes that automatically admit all original
actors or map features. Detailed older limits remain available in the complete
[historical compatibility document](validation/archive/pre-m3d3j-compatibility.md).

## Demon support: remaining dependencies

| Dependency | Map-context configuration (profile 17) support |
| --- | --- |
| Source metadata | Original `MT_SERGEANT` type 12 / editor type 3002 is recognized unchanged |
| Map THING admission | Commercial single-player, normal timing, bounded Demon-only execution |
| Spawn compatibility | Signed fixed coordinates, BSP floor, quantized angle, raw state, original RNG/order |
| Original flags | SOLID/SHOOTABLE/COUNTKILL = 4194310 restored; ambush applied from THING |
| Level statistics | Kills/items/secrets reset to 0; admitted map COUNTKILL placements add available kills |
| Player statistics | Only genuine players[0].killcount fallback for absent/unbound sources; no player-pointer scoring |
| Damage/death | Existing shared services, pain chance 180, delayed sounds, ordinary death/Fall/quarter height |
| Terminal corpse | COUNTKILL retained; respawn disabled, no extra terminal RNG/movecount work |
| Respawn/raise | Explicitly unsupported under this profile; no Nightmare respawn or arch-vile resurrection (original behaviour: Nightmare respawn is in M6b by the owner's ruling of 2026-11-21, arch-vile resurrection has no milestone named yet: [checklist](ROADMAP.md#vanilla-completeness-checklist)) |
| Complete original actor | Deferred as of this profile: native type 136 identity, state aliases and disabled raise remain adaptations (the original identities came with the [original-ID profile](#original-id-demon-profile)) |

Only the old COUNTKILL flag projection is removed. Profile 17 uses
`ActorRegistry::demon_map_context_fixture`, not complete original actor admission.
Available `total_kills` never decreases on death/removal. The separate
`single_player_kill_count` represents source's unbound single-player fallback,
including absent sources, self damage and admitted monster-caused lethal damage.
It is not points, frags or a complete player/intermission subsystem. Other level
totals remain zero because items and sector specials are excluded.

Fresh restart resets the bounded level/actors/statistics while continuing RNG.
It returns a new simulation and does not perform complete game/input/player reload.
No API promises globally unique handles across separate simulation instances.

## Map size and processing limits

A valid parsed map may still be rejected by a simulation configuration. The bounds
below come from the linked implementation, not a promise of real-map gameplay or
unlimited geometry. Four different kinds of limits must be considered separately.

### 1. Original encoding and source semantics

The classic records have fixed-width fields. [Map types](../crates/doom-map/src/types.rs)
preserve signed 16-bit coordinates/heights, unsigned flag bit patterns and
16-bit references. The [parser](../crates/doom-map/src/parse.rs) additionally
validates references according to the selected Linux 1.10 interpretation:

- Ordinary vertex, line, sidedef and sector references must address existing
  records and fit the nonnegative signed 16-bit range. A Rust `u16` storage type
  does not automatically authorize larger source indexes.
- Node children use the high bit to mark a subsector and the lower bits for its
  index. Sidedef `0xffff` means absent; subsector first-seg/count fields have
  signed-range and contiguous-range checks.
- BLOCKMAP dimensions must be positive signed 16-bit values; referenced list
  offsets and line indexes follow signed source constraints. `0xffff` terminates
  a list. These rules can reject data well below allocation budgets.

Changing a budget cannot widen an on-disk field or remove a reserved marker.
Supporting another source interpretation or an extended map format requires an
explicit compatibility decision and, where needed, different representations.

### 2. Parser safety budgets

[MapLimits and Budget](../crates/doom-map/src/parse.rs) provide these defaults:

| Setting | Default | What it bounds |
| --- | --- | --- |
| `max_records` | 32,768 | Decoded records per table |
| `max_lump_bytes` | 64 MiB | Bytes in one map lump |
| `max_blockmap_cells` | 262,144 | Decoded collision-grid cells |
| `max_blockmap_words` | 4 Mi words | Raw BLOCKMAP header/table/list storage |
| `max_decoded_bytes` | 128 MiB | Cumulative requested decoded and scratch allocation |

These are resource ceilings, not guaranteed supported map sizes. Checked
multiplication/addition and fallible reservations enforce the allocation budget.
The parser also hard-caps every record table at 32,768 independently of the
configurable setting. Its uniform application needs table-by-table investigation:
for example, the THINGS record count comes from lump length, not a 16-bit count
field. Neither that policy nor every classic reference is interchangeable with a
universal format limit. Increasing `max_records` alone does not remove the cap.

### 3. Temporary simulation support restrictions

These narrower bounds support the existing arithmetic, ownership and preflight
proofs. They are not classic-map format limits; changing one requires checking
all dependent representations and services.

| Area | Current bound | Implementation |
| --- | --- | --- |
| Historical collision profiles 2–17 | 128 lines; 4,096 BLOCKMAP cells; 65,536 BLOCKMAP words | [prepare_cells](../crates/doom-sim/src/movement.rs) |
| Historical topology profiles 3–17 | 128 sectors; 4,096 subsectors; 4,096 nodes | [Topology::new](../crates/doom-sim/src/multisector.rs) |
| Historical sight-supported geometry through profile 18 | Absolute vertex coordinates, node origins/deltas and sector heights at most 512 map units | [validate_map](../crates/doom-sim/src/sight.rs) |
| Map Demon setup | 256 THINGS, four native Demon test actors, 64 total actor slots | [map spawning](../crates/doom-sim/src/demon_map_spawn.rs), [constructor](../crates/doom-sim/src/demon_lifecycle.rs) |
| Earlier general actor-state setup | 4,096 THINGS and 1,024 actor slots | [actor-state constructor](../crates/doom-sim/src/spawning.rs) |
| Underlying actor store | Configurable, at most 65,536 slots; individual configurations are stricter | [ThinkerStore::new](../crates/doom-sim/src/store.rs) |
| Map Demon arithmetic margins | Absolute spawn coordinates 978, owner query margin 1,008, other actor coordinates 994, general actor/query range 1,024 map units; body initial XY momentum and admitted Z momentum at most 64 units | [map preflight](../crates/doom-sim/src/demon_map_spawn.rs), [spawn checks](../crates/doom-sim/src/spawning.rs), [sight](../crates/doom-sim/src/sight.rs) |
| Real-map policy (wire 17) | The WAD format's own bounds (32,767 records per table, 32,768 BLOCKMAP words, fewer cells than words); sight and collision work computed from the map; compiled cell references at most 2²⁴ and preparation requests at most 64 MiB as allocation guards; see [the classic-map-scale section](#original-monsters-at-classic-map-scale) | [validate_real_map](../crates/doom-sim/src/map_extent.rs), [GeometryCapacity](../crates/doom-sim/src/lib.rs) |
| Specials policy (wire 18) | The real-map bounds plus sector specials 1–5, 7–9, 12, 13, 16, 17 only; line special 48 on at most 64 one-sided lines; sector lights within `i16::MIN + 64 ..= i16::MAX - 64`; a sector-thinker slot class sized by the light-sector count at load; see [the sky and static-specials section](#original-monsters-with-sky-and-static-specials) | [validate_specials](../crates/doom-sim/src/demon_expanded_map.rs), [sector thinkers](../crates/doom-sim/src/sector_thinkers.rs) |
| Movers policy (wire 19) | The specials bounds plus line specials 1, 63, 109, 88, 36, 38, 11 (no two-sided 48); at most 30 lift sectors, 16 SR switch lines and 8 two-sided special lines per 2 × 2 BLOCKMAP window; reachable heights within ±512; a sector-thinker slot class of light sectors + 2 × mover sectors; see [the movers section](#original-monsters-with-doors-lifts-floors-switches-and-the-exit) | [validate_movers](../crates/doom-sim/src/mover_rules.rs), [movers](../crates/doom-sim/src/movers.rs) |
| E1M1 policy (wire 20) | The movers bounds with line special 48 also on two-sided lines (still at most 64 scrollers and 8 two-sided special lines per window); at most 32 decorations of seven listed types; see [the E1M1 section](#original-monsters-with-two-sided-scrolling-walls-and-static-decorations) | [validate_e1m1](../crates/doom-sim/src/mover_rules.rs), [decorations](../crates/doom-sim/src/e1m1_rules.rs) |

For example, collision and hitscan services contain fixed line-work buffers tied
to the line bound; raising only the rejection constant would not be sufficient.
The [future map-scale milestone](ROADMAP.md#map-scale-limits-and-real-map-readiness)
describes the broader direction. The separate geometry configurations below
expand their declared slices while retaining historical admission.

### 4. Performance and traversal work limits

| Service | Current work bound | Meaning |
| --- | --- | --- |
| Historical sight through profile 18 | 8,192 units of fully expanded BSP traversal work, including repeated children/segs | [sight work validation](../crates/doom-sim/src/sight.rs) |
| Hitscan | 64 grid-cell iterations; candidate count bounded by `64 * (128 + 1024)` | [combat tracing](../crates/doom-sim/src/combat_trace.rs) |
| State actions | 128 immediate entry work units, 8,192 planned step work units, and bounded child spawning | [lifecycle planning](../crates/doom-sim/src/lifecycle.rs) |
| Thinker pass | One visit per slot of either class: the actor slots plus, under the specials policy, the light-sector count, and under the movers policy the light-sector count plus twice the mover-sector count | [ThinkerStore](../crates/doom-sim/src/store.rs) |
| Moving planes (movers policy) | At most two steps per mover sector per tic, two `P_ChangeSector` calls per step, each re-clipping the linked things of the sector's block box; one use trace per press | [planes](../crates/doom-sim/src/planes.rs), [use](../crates/doom-sim/src/use_lines.rs) |

Some work bounds also preserve source semantics: the hitscan cell loop follows
[original P_PathTraverse](../linuxdoom-1.10/p_maputl.c), so expanding it may change
vanilla behavior. Other bounds prevent pathological CPU/memory growth. The future
inventory must distinguish these reasons rather than replace every value with a
large constant. Keep deterministic traversal and checked resource use throughout.

## Map-scale profile

`Simulation::map_scale` / `with_map_scale_limits` select profile 18, not a Demon or
player profile. Parsing capacities/formats are unchanged. The new configuration
supports at most **1,024 linedefs, 256 static sectors, 64 unbound actor slots,
4,096 BLOCKMAP cells, 65,536 raw BLOCKMAP words, and 262,144 compiled first-occurrence
references summed over cells**. Shared lists count once per cell in the last budget.
Nodes and subsectors remain at most 4,096 each. Smaller actor/reference budgets
are available through `MapScaleLimits`; higher budgets reject.

Vertex coordinates, node origins/deltas, sector heights and BLOCKMAP origins stay within
512 absolute map units. Original sight expansion remains bounded at 8,192 work
units, including repeated node children and segs. Spawn positions/absolute Z and
current positions/Z before a step stay within 994 units; XY/Z momentum is at most
64. Source XY movement clamps to 30. Read-only destination/sight actor coordinates
are bounded at 1,024 units. A source step can leave the 994-unit step-admission
margin; the next step then rejects atomically. This is an explicit conservative
Rust boundary, not unrestricted off-map movement compatibility.

Use parameterized unbound bodies and existing XY/Z, gravity, openings, collision,
sector/BLOCKMAP head insertion and thinker storage. THINGS remain parsed data.
Read-only `check_sight` preserves REJECT indexing and source BSP/seg ordering without
admitting perception rosters or AI. Unsupported player binding, typed spawning,
map spawning, damage/hitscan, noise/Look/Chase, map specials and dynamic sectors
reject; no unsupported callback becomes a no-op. Real player, Demon/Spectre and
whole-map/game admission are not implied.

Source BLOCKMAP words, cell traversal and first encounters determine candidates.
Lists are never rebuilt or globally sorted by line ID. Recoverable rejected setup,
controls, identity queries and stepping preserve canonical state. Allocation for
new geometry preparation is fallible and bounded; existing runtime diagnostic
allocation exhaustion retains the process-level Rust policy described by `step`.
The source 64-cell slide/hitscan traversal and 128-intercept restriction are not
expanded or admitted by this configuration. Historical constructors retain their
old capacities and DSIM expectations.

[Source findings](M3F1_REFERENCE.md) explain the representation and adapter;
[measured evidence](validation/M3F1.md) separates original-C equivalence,
independent DSIM consistency, safety tests and host measurements. This supports
larger controlled classic fixtures, not all real DOOM maps or vanilla demos.

## Map-extent profile

`Simulation::map_extent` / `with_map_extent_limits` select **profile 19**, a separate
unbound static-geometry configuration. Historical profiles, including profile 18,
retain their existing coordinate/work rejection thresholds and canonical bytes.
Classic formats and parser budgets are unchanged.

| Domain or resource | Profile19 admission |
| --- | --- |
| Vertex components, node origins and node deltas | Absolute 2,048 map units each |
| Linedef delta | `abs(dx) + abs(dy) <= 1,536` units, checked from endpoints |
| Sector floor/ceiling | Absolute 1,024 units; noninverted |
| BLOCKMAP origin components | Absolute 2,048 units |
| Actor spawn/current XY before a tic | Absolute 2,018 units |
| Destination and sight XY | Absolute 2,048 units |
| Absolute spawn/current Z before a tic | Absolute 1,984 units; floor/ceiling placement uses the sector |
| Sight Z | Absolute 2,048 units |
| Actor radius / height | Radius 0–32; positive height fitting the spawn sector, at most 2,048 |
| XY/Z momentum | Absolute 64; original XY clamp remains 30 |
| Linedefs / sectors / unbound actor slots | 1,024 / 256 / 64 |
| Nodes / subsectors | 4,096 each |
| BLOCKMAP cells / raw words | 8,192 / 65,536 |
| Prepared first-occurrence references | 262,144, counting each shared list once per cell |
| Conservative preparation allocation requests | 4 MiB; independently configurable downward |
| Collision work proof | At most 16,384 units; setup checks `nodes + 4*lines + 4*actor_limit` |
| Expanded sight work | 16,384, counting both repeated children, leaves and segs |

`MapExtentLimits` allows smaller actor, reference, preparation-memory and collision-
work budgets. Larger budgets reject. Preparation charges reserved reference limits
and native-size cell/line/membership/topology/work scratch conservatively, including
scratch whose lifetimes do not overlap. Parsed Map storage, canonical caches,
actors, snapshots and allocator overhead are outside that request budget and retain
their separately bounded/process-level allocation policies. Geometry preparation
and new sight-query scratch reservations are fallible. Existing tic diagnostics
and canonical allocation exhaustion remain process-level Rust failures, not `SimError`.

World-space bounds and pairwise differences are distinct: admitted query actors
can differ by 4,096 units. The line-delta norm additionally protects the original
shifted intercept sum/difference; blindly enlarging all coordinates would not.
This is not a strict superset of profile 18: some long old diagonals require the
old narrower interaction domain. A successful tic can leave the next-tic XY/Z or
momentum margin; a subsequent tic rejects before any earlier actor moves.

A radius-bounded destination visits at most four line cells and four thing cells:
at most 4,096 compiled references, 1,024 unique linedef callbacks and 64 unique
thing candidates. BSP lookup adds at most the admitted node count. Source supplied
list/cell order, duplicate suppression, leading-zero behavior, things-before-lines,
head insertion and thinker order remain authoritative. Conventional empty cells
still contain the required zero word and therefore visit line0. Unused grid padding
need not fit world coordinates; runtime queries cannot leave their proven domain.

Coordinate bounds are necessary, but not sufficient for every source sight input.
Quantized side tests can produce an intercept saturated to `INT_MIN`; original
FixedDiv would then evaluate undefined `abs(INT_MIN)` during clipping. Profile19
returns a recoverable arithmetic error before that division, preserving canonical
state. Historical profiles retain their existing explicit Rust numeric policy.

Sight caches a proved full-root expansion cost; query execution uses that same
bound and fallibly reserves bounded tasks/observations. REJECT returns before BSP
work or scratch reservation. No query consumes RNG or changes canonical state.

Use existing unbound XY/Z, gravity, static openings, spatial storage and read-only
sight only. As of this profile, player binding, typed actors, map spawning, AI/hearing,
combat/hitscan, sliding, dynamic sectors, specials and complete game/map admission remain
excluded ([what came later](#deferred-systems)).
Original 64-cell `P_PathTraverse` and 128-intercept behavior is unchanged.
[Source analysis](M3F2_REFERENCE.md) owns the arithmetic derivation;
[observations](validation/M3F2.md) distinguish genuine C comparisons, independent
canonical encoding, Rust safety tests and host measurements.

## Expanded-map Demon profile

`Simulation::with_demon_expanded_map_registry` selects **Profile 20**. Supply the
existing `ActorRegistry::demon_map_context_fixture`, explicit actor options and
map context; then call `spawn_map_things` before installing unbound targets.
This composes the admitted Demon closure with expanded static preparation. It
retains native type 136/state aliases and does not admit complete MT_SERGEANT.

| Area | Profile 20 limit or behavior |
| --- | --- |
| Tables | 1,024 lines, 256 static sectors; 4,096 nodes and subsectors each |
| Geometry | Vertex, node origin/delta and BLOCKMAP-origin components within ±2,048 units |
| Heights and line deltas | Sector heights within ±512, noninverted; linedef `abs(dx)+abs(dy) <= 1536` |
| Collision preparation | 8,192 cells, 65,536 raw words, 262,144 prepared references; separate conservative 4 MiB preparation-request ceiling |
| Collision work | Existing 16,384-unit setup proof; source list order, leading zero, duplicates and suffix semantics retained |
| Sight | 16,384 fully expanded node/leaf/seg work; checked scratch allocation and original INT_MIN clipping exclusion |
| Per-operation sight budget | At most 64 action queries and 1,048,576 conservatively charged diagnostic work units, including reserved extra per-line observations |
| Actor count | At most 64 slots; at most four native Demon owners before the stable unbound suffix |
| Actor coordinates | Read-only actor XY/Z within ±1,024; pre-tic owner XY within ±978 and suffix XY within ±994; callback/spawn XY within ±1,008, map Demons within ±978 |
| Vertical setup | Explicit absolute spawn Z within ±1,018; pre-tic vertical momentum within ±64 and Z plus momentum within ±1,024 |
| Momentum and dimensions | XY momentum within ±64 in the new profile; original XY clamp 30 and Demon Chase speed 10. Demon radius/height 30/56; body target radius/height 16/56; exact registry/effect dimensions must fit static sectors |
| THINGS | At most 256 records; original lookup/filter/nomonsters order, DoomEd 3002 only after filtering; source raw state, floor/BSP, spawnpoint, RNG and insertion order |
| Selected THING angles | Truncated `angle/45` in `[-4,3]`, encoded degrees `[-224,179]`. Outside values reject atomically because the original signed ANG45 product overflows. All eight BAM octants can use signed equivalent encodings |
| Gameplay | Original normal stand/run, Look/hearing/roster, Chase/movement, melee, shared direct damage, pain/death/Fall and persistent corpses |
| Targets and sources | Stable unbound body 135 for live AI/sound/roster targets; existing effect lifecycles 37/38 remain distinct. No real player pointers or native infighting beyond the inherited synthetic lethal/ignored slice |
| Statistics/restart | Separate available map kills and single-player fallback credit; retained COUNTKILL, disabled respawn; fresh restart resets level state and continues RNG |

Geometry admission is broader than actor admission. This does not allow Demon
execution at ±2,048 actor coordinates. The line-norm and signed-angle restrictions
also mean this is not an unconditional superset of older profiles. Historical
Profiles 17 and 19 retain their contracts and byte fixtures.

Direct hitscan/autoaim, other attacks, player bindings/damage, multiplayer,
Fast/Nightmare, resurrection, dynamic sectors and specials reject. The source's
64-cell path-traversal and 128-intercept limits are unchanged. Unsupported types
never become generic actors; unknown enabled types remain recoverable batch errors.

Whole-tic and nested direct actions use a bounded actor-state plan. Any recoverable
error preserves actors, identities, RNG, lists, statistics, commands and both event
queues. Checked reservations/work limits are distinct from inherited process-level
Rust allocation failure for ordinary observation/snapshot vectors. Preparation
accounting excludes parsed Map storage, canonical caches, the small actor plan,
observations and allocator overhead; it is not a process RSS limit.

[Source findings](M3F3_REFERENCE.md) explain the arithmetic and original signed-angle
failure. [Architecture](ARCHITECTURE.md#expanded-map-demon-composition) explains
planning; [evidence](validation/M3F3.md) separates genuine C, independent canonical
encoding, safety policies and host measurements.

## Demon runtime restrictions

Map-spawned Demon thinkers precede stable unbound type 135 test targets/effects.
Live Demon, perception-roster and sound targets must use that target role. Real
player bindings reject. Demon actors can enter only through the atomic map setup
batch, not ordinary in-tic spawning. Source identifiers/state aliases remain the
documented adaptation in the support matrix above. The [source investigation](M3D3I_REFERENCE.md)
explains its ownership proof. Capacity/work/checked-statistics overflow and stale
identities reject before changing canonical state.

Synthetic cross-native lethal or already-ignored damage is admitted because source
returns before retargeting. Nonlethal cross-native damage rejects; autonomous AI
still cannot acquire native targets. No generalized infighting or projectile
interaction is implied. Source/inflictor identity is validated even for ignored
requests as a Rust safety policy. Unsupported actions never silently no-op.

## Deferred systems

Every policy is frozen: its own section says what it admits and refuses, and it keeps
refusing what it refused when it was written, even where a later policy admits it. A
sentence in an older section that begins "as of this policy" or "as of this profile" is
true of that policy only. This section says where the newest ones stand
([wire 25](#original-episode-1-part-3-canonical-sound-records-wire-25) and the
[playable build](#playable-build-m5b) with [sound effects](#sound-effects-m5c)).

The [quirk register](QUIRKS.md) lists quirks of the original found by the 2026-11-21 sweep, some of which these systems will have to meet; it is not claimed to be complete.

**Admitted since the older sections were written, under the later policies only:**

- real players with damage, armor, pickups and weapons (the player policies from
  [the player body lifecycle](#original-player-body-lifecycle) on);
- every line and sector special Episode 1 uses, keys, the Spectre and most powers, not
  invulnerability or berserk
  ([wire 23](#original-episode-1-part-1-every-door-floor-plat-stair-donut-light-and-exit-special-keys-items-powers-chainsaw-rocket-launcher-and-spectre-wire-23));
- teleporters, the Baron, hard skill and the one local `DEMO1`
  ([wire 24](#original-episode-1-part-2-teleporters-the-baron-of-hell-a_bossdeath-and-hard-skill-wire-24));
- the 3D view ([M5a](#software-renderer-m5a)), the status bar, HUD messages, palette
  flashes, window and input ([M5b](#playable-build-m5b)) and sound effects
  ([M5c](#sound-effects-m5c)).

**Still unsupported under every policy:** scoring and the intermission, multiplayer,
Fast/Nightmare execution, respawning, resurrection, ceilings and crushers and every other
line and sector special no Episode 1 map uses, things and monsters outside Episode 1,
general demo playback, and the presentation the playable build does not have (music,
menus, the automap among it). All of it is original behaviour, so none of it is optional:
the [vanilla completeness checklist](ROADMAP.md#vanilla-completeness-checklist) lists every such item with the
milestone named for it, or says that none is named yet. The only exceptions are the items the owner declined by
ruling on 2026-11-21, each marked there with its reason.

Spectre metadata recognition is not Spectre gameplay compatibility.
Source undefined behavior is not a compatibility target; checked Rust rejection
and adapter-defined evaluation order are recorded separately.

Read [M3D3J_REFERENCE](M3D3J_REFERENCE.md) for exact statistics chronology,
[STATE_FORMAT](STATE_FORMAT.md) for schemas, and [TESTING](TESTING.md) to reproduce.

## Original identity metadata

M3f4 represents all 137 original actor, 967 state and 138 sprite identities and
all original actor/state descriptor records. The [mapping guide](ACTOR_IDENTITIES.md)
defines checked conversions and named properties. This is complete metadata
recognition, not complete actor/state execution. M3f4 admitted no new gameplay
profile or behavior. Existing original decorative/effect/lifecycle
slices retain their existing checks; the focused integration uses MT_TFOG through
the same typed owner, raw spawn, state entry and thinker services.

Canonical-to-registry resolution compares only the requested complete record at
its original ordinal. It does not certify every referenced state or live actor
context. Returned slots must still pass existing action/flag/role/map checks.
Altered records, reordered names and fixture replacements reject. Native Demon
136 and state aliases remain explicit fixture adaptations. The separate binding
below connects MT_SERGEANT to its already admitted subset. MT_SHADOWS, players,
raise states and other unimplemented dependencies remain metadata without new
gameplay permission. As of M3f4, Spectre gameplay, real
players, infighting, resurrection and Nightmare/respawn remain deferred
([what came later](#deferred-systems)).

## Canonical Demon execution requests

The M3f5 binding connects original MT_SERGEANT (Demon/Pinky, not Shotgun Guy) to
the existing [Profile20 closure](#expanded-map-demon-profile). It derives native
metadata from canonical records and admits typed canonical batch/state requests
through existing services. All original ordinals are preserved in the source
registry. All runtime limits, native identity bytes, event order and game-context
restrictions remain the Profile20 contract; there is no Profile21.

The binding covers normal stand, run, melee, pain and death definitions. Only
stand/run/attack have direct entry permission; pain/death remain damage-controlled.
Raise references are represented but have no binding. Invalid ordinal conversion,
unsupported identity/state requests, wrong owners and stale handles reject safely.
MT_SHADOWS (Spectre) and MT_SHOTGUY (Shotgun Guy) are explicitly excluded.

Canonical definition queries are projections through a validated correspondence,
not raw original IDs in ActorLife/DSIM. As of this binding, full actor compatibility,
real players, generalized infighting, Nightmare/respawn, resurrection, dynamic sectors, new
hitscan/projectiles and rendering remain deferred ([what came later](#deferred-systems)). See the
[mapping](ACTOR_IDENTITIES.md#canonical-demon-execution-binding),
[source investigation](M3F5_REFERENCE.md) and [observations](validation/M3F5.md).

## Native runtime identity boundary

M3f6 retains all existing stored IDs and admission bounds. The typed native Demon
view covers only the validated Profile20 type 136/21-state execution correspondence.
It is separate from original identities and live handles; it grants no additional
state entry, actor, raise, Spectre or player behavior. The complete original registry
remains metadata, not executable support. Earlier profiles retain their distinct
native fixtures, including unrelated uses of136.

Raw canonical type 12 or SARG475–501 requests are not silently interpreted as native
owner/state requests. Use the explicit canonical APIs or checked native projection.
Pain/death observations are representable native values but direct entry still
rejects. All historical public raw observations, event payloads and DSIM-v20 remain
unchanged. See [mapping](ACTOR_IDENTITIES.md#native-runtime-identity-boundary),
[audit](M3F6_REFERENCE.md) and [evidence](validation/M3F6.md).

## Original-ID Demon profile

`Profile::OriginalDemon` is opt-in. `OriginalExecutionProfile::DemonNormalV1`
identifies its role contract independently of diagnostic version. The
[registry](ACTOR_IDENTITIES.md#original-id-execution-registry) keeps complete
original metadata pristine and stores admitted Demon type 12/states 475–495 directly.
Normal gameplay is the existing Profile20 closure, not broader Demon support.
Direct stand/run/melee entry is guarded; pain/death remain damage-controlled and
raise 496–501 remains metadata only. MT_SHADOWS, MT_SHOTGUY, MT_PLAYER and original
MT_MISC85/86 have no executable actor role. Existing puff/blood effects and an
explicit synthetic unbound body context retain their bounded roles.

All [expanded-map restrictions](#expanded-map-demon-profile), four-owner/64-slot
limits, geometry/work bounds, THING filter order, normal timing, signed-angle
proof, checked operation atomicity and unbound statistics policy apply. Source
DoomEd lookup precedes admission; source decorations at 135/136 now retain their
true metadata and reject if selected. This is distinct from their overwritten
fixture metadata in old profiles. Source-ID metadata does not authorize new map
placements, callbacks, players, infighting, hitscan, Fast, respawn or resurrection.

This profile reuses the immutable M3f5 independent C scenarios. New comparisons
exercise actual source-ID storage and diagnostic interpretation; they are not new
C-input or whole-game coverage. Profiles 1–20, native aliases and DSIM-v20 bytes
remain unchanged. V21 is a read-only diagnostic contract, not a savegame; see
[format and validation limits](STATE_FORMAT.md#dsim-v21--original-id-demon-diagnostics).

## Context and target-lifetime contract

The original-ID profile admits an additive `with_execution_context` interface.
Its lifetime-branded instance tokens belong to one exclusive callback; they cannot
escape or mix with another scope. Explicit local raw-handle admission checks
generation, actual definition and the caller's expected definition. This is an
access contract over the existing Simulation, not a second lifetime manager.
Metadata recognition and token possession alone do not grant gameplay permission.
Historical profiles reject this scope before invoking its callback; their raw APIs
and diagnostics retain their prior meanings.

| Reference condition | Observation and mutation policy |
| --- | --- |
| Current nonpending storage | `Live`, including corpses; health and shootability remain separate |
| Pending removal | Readable until traversal; spatial membership already unlinked; acquisition rejects |
| Latest retired occupant, slot still vacant | `Retired`; actor access rejects; stored references remain inert |
| Slot reused / older generation | `StaleGeneration`; never resolves to the new occupant |
| Unissued current generation in vacant slot | `Vacant`, not a live actor |
| Generation zero / unallocated slot | `InvalidGeneration` / `OutOfRange`; admission rejects |
| No stored target | `TargetReference::Absent`; no implicit mutation |
| Current target with incompatible role | `Incompatible` observation; acquisition remains rejected |

Generation exhaustion seals a slot instead of wrapping. `Retired` above describes
the most recent occupant; the existing DSIM slot `.retired` flag means permanent
generation exhaustion. Availability is derived without adding a history table.

Target observation reuses the shared definition-role guard, consumes no
RNG and never clears/rebinds a reference. An available body corpse is not proof of
combat permission. The Demon cannot target itself because its target must be a
context body; the existing synthetic-body self policy is retained. Shared damage
can retain a source on an inert body even when that source is not an admissible
Demon target. Observation does not authorize a new autonomous relationship.

Assignment/replacement validate before mutation; clearing does not resolve the
old target. Removal is spatially immediate and thinker-deferred; duplicate marks
are idempotent. Death preserves instance identity and does not universally clear
targets. There is no incoming-reference sweep. A living Demon with an unavailable
target retains the established checked-step rejection until explicit caller repair;
roster/sound references have their own guards and are not automatically repaired.
Dead-owner references can remain inert and observable. Each existing operation
retains its atomicity; successful earlier operations inside a callback remain
applied if a later operation fails.

Raw handles remain store-scoped. Their equal values across worlds/restarts do not
establish provenance; the origin of an erased/re-admitted ID cannot be recovered
from its raw value. No persistent global context ID or cross-world transport is admitted.
No actor/geometry limit, execution profile, registry schema or DSIM version changes.
Real player mo/attacker ownership, general species relationships, tracer/projectile
ownership and resurrection require separate source/capability investigations.
The new boundary grants none of those behaviors. See [mapping](ACTOR_IDENTITIES.md#context-scoped-actor-instances),
[investigation](M3F8_REFERENCE.md) and [qualified evidence](validation/M3F8.md).

## Game-instance provenance

`GameInstance` adds an owning in-process boundary over one Simulation. Fresh
wrapping always creates a distinct opaque InstanceId, even for equal map/registry/
state or cloned Simulation data. Moving the owner preserves its identity. The
wrapper is not Clone; its read-only Simulation view cannot replace state behind
the key. Consuming extraction preserves raw state, while rewrapping creates a new
incarnation. Existing raw APIs remain explicitly local, without inferred provenance.

`GameInstance::with_execution_context` reuses the current original-ID scope and
all role/state/action guards. Wrapping a historical profile does not admit that
scope. An exported InstanceActorRef holds only an opaque key and local handle,
not a world/actor lease. Import checks these conditions in order:

| Condition | Result |
| --- | --- |
| Raw Simulation scope, without an owner key | UnboundContext |
| Candidate without provenance, including decoded local IDs | MissingProvenance |
| Different instance or earlier incarnation | InstanceMismatch, before actor lookup |
| Matching key, invalid/stale/vacant/retired local identity | Existing ContextError::Unavailable classification |
| Matching current actor, wrong expected definition/state/role | Existing identity/definition error |
| Matching current actor, including pending storage or corpse | Scoped observation; existing gameplay guards still apply |

Import/export are pure: no game-state mutation, event, RNG draw or actor allocation.
Generation exhaustion, lowest-slot reuse and scheduling remain unchanged. A pending
actor can be observed but cannot be newly acquired through a rejected target role.
Successful earlier operations remain applied when a later callback import fails;
the scope is not an aggregate transaction.

`restart()` delegates to existing fresh level setup/spawning with continuing RNG.
Only success replaces both state and key. References to the previous incarnation
reject even if new actors have equal slots/generations/definitions. Failure leaves
the prior identity and complete simulation state intact. There is no global live-
instance directory or history index; foreign and pre-restart keys share the explicit
InstanceMismatch classification.

### Trust and unsupported boundaries

Opaque keys compare runtime allocation identity, not numeric contents, map hashes
or snapshot equality. A retained key prevents reuse aliasing but retains neither
the world nor actor. Keys are process-local, with no persistent/serialized meaning,
byte constructor, numeric conversion, remote resolver or authentication guarantee.
There is no savegame restoration or networking implementation. DSIM decoding
remains diagnostic observation and cannot manufacture provenance.

`InstanceActorRef::candidate` is deliberately explicit local routing/readmission.
A caller possessing a trusted local key can assert that a raw candidate should be
interpreted there; an erased origin cannot be recovered or authenticated. Preserved
checked exports reject foreign worlds automatically, whereas deliberately replacing
their provenance changes the caller's assertion. Untrusted remote data would need
a separately trusted resolver/transport policy, not a numeric tag alone.

One small Arc allocation is made per new wrapper/successful restart, under the
existing process-level OOM policy. Import borrows the key; export/reference cloning
shares it without a new allocation. Allocation counts were not instrumented.
No per-actor or Simulation identity field, global counter, wall clock, OS randomness,
RNG consumption, capacity expansion, gameplay profile or DSIM schema is added.
Real-player mo/attacker, new species relationships, persistence and transport remain
separate work. See [architecture](ARCHITECTURE.md#game-instance-provenance),
[audit/design](M3F9_REFERENCE.md) and [observations](validation/M3F9.md).

## Original player body lifecycle

Select `Simulation::original_player_map` or an OriginalExecutionRegistry with
`OriginalExecutionProfile::PlayerBodyLifecycleV1` explicitly. The legacy
`Profile::OriginalDemon` names the shared original-ID service family;
`original_execution_profile()` identifies the selected policy. Historical
DemonNormalV1, ContextRole::UnboundBody137 and all old profiles keep their behavior.
No synthetic body is reclassified as a player.

This policy admits one genuine MT_PLAYER=0, with original S_PLAY/RUN/pain/normal
and extreme-death states149–153/156–173. Fixed settings are Medium, commercial,
single-player, no armor/powers/cheats, pistol presentation request, automap false,
no respawn, static one-sector floor 0 / ceiling 128, existing64 actor-slot maximum and
existing geometry/work budgets. Input movement/buttons, physical inflictors,
manual player state/health/pursuit/momentum changes and unsupported settings reject.
The body stays grounded and stationary; XY must already fit the shared pre-tic
±994-unit proof when spawned or bound. This is a Rust support bound, not an
original-C coordinate limit. Real player starts, spawn-angle loading,
HUD, weapons, inventory and view initialization are not claimed.

`spawn_player_body(PlayerSlot::One, XY)` atomically spawns at the floor with
angle0 and health100, then establishes source mo ownership. A separately raw-spawned
initial canonical body can be bound through checked local/scoped APIs. Bind validates
current generation, nonpending storage, pristine state/health/flags/physics and
initial counters/orientation. Double binding, wrong definition, stale/pending or
invalid handles reject atomically. Player index0 can own a different actor slot;
raw slot equality never establishes ownership. GameInstance import still rejects
foreign or old incarnation references. Raw admission remains explicitly store-local.

Supported direct damage is `P_DamageMobj(target, NULL, source, amount)` for bounded
amount0–1000 and admitted current player/Demon sources or NULL. Source and inflictor
are distinct; physical thrust is explicitly excluded. Player health subtracts and
clamps0; mobj health stays signed. Attacker assigns source including NULL, damagecount
caps100 and the console tactile request is observed before mobj health subtraction.
Nonlethal pain/reaction/retargeting and lethal state selection/jitter use the shared
source-tested damage service and RNG. Self damage may store self as attacker;
NULL clears player.attacker without clearing the distinct mobj.target.

Death preserves the mo binding, sets PST_DEAD, clears MF_SOLID immediately, keeps
source corpse/dropoff flags, quarters height and retains the allocated corpse.
Normal versus extreme death uses the strict health < -100 boundary. Commercial
player scream uses the source -50 threshold. P_DropWeapon's requested pistol-down
state is observed; psprite lowering/weapon execution is excluded. Source environment/
self frags and sole-player monster kill credit are admitted. No score/intermission,
respawn, automatic retirement or PlayerThink/DeathThink damagecount/camera update
is implied. Repeated damage to a dead body is source-ignored after Rust reference
validation; malformed sources still reject without changes.

The player body is unlisted (`playeringame=false`), not an active AI roster or
complete game-loop player. Original all-inactive LookForPlayers does not terminate.
Live Demon AI stepping/manual AI callbacks therefore reject in this policy. A live
canonical Demon can be a controlled direct damage source between tics, then must
be explicitly killed or marked for removal before stepping. Dead-but-allocated
sources and Demon death/corpse actions remain supported. Player-to-Demon damage is
restricted to lethal input, avoiding unsupported nonlethal retarget/infighting.
As of this policy, automatic targeting and physical Demon melee against the player remain
future work ([what came later](#deferred-systems)).

mo/attacker retain their original opaque handles after pending removal, retirement
and reuse. New damage/binding cannot use pending or unavailable actors. Death alone
keeps a current actor, including a possible attacker corpse. No incoming-reference
sweep clears identities. Generational availability queries never resolve an old
reference to a new occupant. These guarantees are Rust policy, separate from safe
C logical-retirement observations. Restart/respawn reject rather than silently
losing or transferring the player binding.

DSIM-v22 supplies the genuinely new player observations; v20/v21 and their readers
retain their old interpretations. `advance` still captures no Snapshot, `step`
captures after the same shared core, and every supported/rejected operation keeps
its existing transaction boundary. See [schema](STATE_FORMAT.md#dsim-v22--original-player-body),
[source investigation](M3G2_REFERENCE.md), [tests](TESTING.md#original-player-body-validation)
and [measured evidence](validation/M3G2.md). No complete player gameplay claim follows.

## Original player grounded knockback

Select `Simulation::original_player_knockback_map` or explicit
`OriginalExecutionProfile::PlayerBodyKnockbackV1`. This is a separate capability;
PlayerBodyLifecycleV1 stays stationary and continues rejecting physical inflictors.
The constructor and new capability-enum variant are explicit API additions;
exhaustive matches on OriginalExecutionProfile need the new arm. The historical
Profile enum and existing signatures/variant meanings stay unchanged.
Both use one genuine original MT_PLAYER=0 with the same checked mo/player ownership,
source health, pain/death states and allocated corpse semantics. Canonical identity
representation still does not imply general executable support.

Settings remain Medium/commercial/single-player/pistol, no armor/powers/cheats,
automap/respawn false, unlisted player, zero commands, no PlayerThink/DeathThink.
Geometry is one static sector floor 0 / ceiling 128 with axis-aligned, one-sided walls
whose flags are ML_BLOCKING. Existing actor/geometry/work limits are unchanged.
Angled/two-sided walls, steps/dropoffs, airborne player motion, dynamic sectors and
unsupported settings reject during construction. Player/body spawn/binding keeps
the existing ±994-unit initial XY boundary and initial zero momentum.

A controlled DamageRequest may supply a distinct optional attacker source and
physical inflictor. Both references must be current and nonpending original player
or Demon instances; a dead-but-allocated source remains valid. The physical origin
and player target must be at floor 0 with zero vertical momentum. New momentum may
only enter through shared damage; direct XY momentum control in this policy rejects.
Physical damage to another actor is outside this new admission. No new weapon,
projectile, infighting or automatic melee/acquisition system is implied.

Eligible hits require an already executable thinker closure: all unlisted Demon
owners must be dying/corpses or pending removal before the hit. Their creation
retains the existing map-THING admission; raw Demon spawning remains rejected.
Old no-inflictor controlled source interactions keep their lifecycle boundaries.
A valid definition or available handle alone never grants permission to execute AI.

Damage 0..1000 is accepted only when its original ordered thrust arithmetic,
checked momentum addition and forward travel proof succeed. Let M=max(abs XY
momentum) in raw 16.16 bits after impulse: M<=30U and each
abs(position)+ceil(160M/3)<=994U. This conservative support bound proves later
axis-wall/friction tics remain executable; it is not an original-C coordinate
limit or a gameplay clamp. Additional hits must establish it again. Even a small
hit near the old coordinate boundary may reject atomically instead of committing
an unsupported future position. See the [derivation](M3G3_REFERENCE.md#forward-coordinate-proof).

Actual source thrust uses point-to-angle, mass 100, integer damage*(FRACUNIT>>3)*100/
mass and fine cosine/sine FixedMul additions before player-specific health,
attacker/damagecount/tactile and signed mobj health. The reverse-thrust branch is
unreachable because both Z values are 0; no thrust RNG draw or reverse-thrust
coverage is claimed. Zero damage still follows source pain processing. Existing
pain/death RNG, flags, statistics and ordered event semantics remain shared.

Subsequent tics perform real shared XY movement, source positive-only subdivision,
things-before-lines collision, source BLOCKMAP candidate order, spatial relinking,
axis-wall sliding/stairstep, grounded friction 0xe800 and strict stop speed 0x1000.
A stopped RUN frame enters S_PLAY before state countdown. Lethal hits preserve
momentum: the allocated player corpse can move, decay and stop while mo/attacker
stay bound. Death, explicit removal and generational reuse remain distinct. There
is no automatic respawn or reference rebinding; stale handles never select a reused
occupant. Rejected operations preserve complete actor/player/RNG/statistics/spatial/
event state, including late checked failures.

[DSIM-v22](STATE_FORMAT.md#dsim-v22-grounded-player-execution-policy) already
represents these observations. Wire policy 3 and a distinct grounded context schema
select the new behavior with unchanged field/event layout. Old v20/v21/v22 policy 2
fixtures/readers retain their meanings. advance remains diagnostic-free, and step
captures only after the same core. [Evidence](validation/M3G3.md) distinguishes new
C observations, reused history and Rust-only safety policies. As of this policy, full
player controls, PlayerThink, roster/automatic Demon attacks, view/camera, weapons,
inventory, armor/powers, air movement, respawn and multiplayer remain unsupported
([what came later](#deferred-systems)).


## Original player command movement

Select `Simulation::original_player_command_map` or
`OriginalExecutionProfile::PlayerCommandMovementV1`. This is a new explicit
capability, not a change to PlayerBodyLifecycleV1 or PlayerBodyKnockbackV1. Their
old command rejection, diagnostics and inputs remain unchanged. All three use
one genuine MT_PLAYER=0 and the existing player/mo ownership, damage, pain/death,
corpse retention, scoped handles and game-instance provenance. Context UnboundBody
remains synthetic. An exhaustive match on OriginalExecutionProfile needs the new
variant; historical Profile variants and existing API signatures are unchanged.

Only source player slot One executes. TicCommand forward/side each range -50..50
(g_game MAXPLMOVE), angle_turn accepts all signed i16 values, and buttons must be
zero. Other command slots must be default. Medium/commercial/pistol, no armor,
powers, cheats or active roster, and the grounded flat axis-wall geometry/limits
of the knockback capability remain required. MF_JUSTATTACKED rewriting is excluded.
The bound live body's existing reaction_time must be in 0..255. Controlled
`set_initial_player_reaction_time` accepts u8 on a current live bound body only
before the first successful tic; default spawn is unchanged. This setup operation
is not teleport gameplay or a second timer.

The single player stage runs before every actor thinker. If reaction_time is
nonzero it decrements once and suppresses all P_MovePlayer work for that tic,
including turning; movement resumes on the next tic that begins with zero.
Commands still reach XY/friction's stop condition. Source damage can reset the
counter before the stage. Otherwise angle updates first, forward thrust uses that
new angle, then sideways thrust uses angle minus ANG90. Integer fine tables and
FixedMul accumulate existing momentum. Standing directional input immediately
enters S_PLAY_RUN1; ordinary thinker countdown continues its source graph. When
existing XY movement stops a RUN frame with zero directional input, S_PLAY entry
precedes countdown. An already zero-momentum RUN state is not forcibly reset.
Turning alone does not enter RUN. Pain does not disable an otherwise live command.

Negative original signed left shifts are undefined ISO C. Supported turns use the
explicit unsigned two's-complement bit-pattern interpretation of the existing
historical observer adapter. Defined literal C cases are compared separately;
there is no portable ISO-C equivalence claim for negative signed shifts. See
[arithmetic investigation](M3G4_REFERENCE.md).

Shared physical damage/knockback composes without replacing momentum. New command
attempts require M=max(abs XY momentum)<=30U and
abs(position)+4M+8 raw bits<=994U for nonzero M; zero M requires only the coordinate
bound. After actual shared movement, the staged body must satisfy the existing
abs(position)+ceil(160M/3)<=994U travel envelope. These conservative Rust support
checks preserve a safe zero-command continuation, not original coordinate limits
or clamps. A source-valid command can reject near this bounded domain's edges.
Any early or late recoverable error preserves complete authoritative state,
including angle, reaction, commands, RNG, both event queues and spatial order.

Unbound, pending-removal, retired/stale or dead bodies accept only default inactive
input through the ordinary thinker/removal pass; nonzero input rejects. Pending
and stale handles never redirect to a reused slot. Dead allocated corpses keep
physical motion and ownership; no command revives them. Malformed current body
identity/health/state rejects. Pause retains its historical precedence. `advance`
remains diagnostic-free; `step` uses the same core and captures once on success.

This is a declared P_PlayerThink movement conditional plus P_MovePlayer, not full
PlayerThink, P_Ticker, P_DeathThink or player roster execution. As of this policy,
view/camera/bob, weapons/buttons, inventory/powers/cheats, special sectors, airborne
controls, respawn/multiplayer, arbitrary angled/two-sided walls and automatic Demon
perception/acquisition remain unsupported ([what came later](#deferred-systems)). Dying/corpse Demon origins retain
existing controlled interaction semantics; no live AI/roster is silently enabled.

## Active original player and Demon acquisition

Select `Simulation::original_player_demon_map` or the separate
`OriginalExecutionProfile::PlayerDemonAcquisitionV1`. It composes the existing
command/knockback/player lifecycle with canonical Demon visual acquisition and
bounded nonmelee pursuit. Historical DemonNormalV1 and player policies are unchanged.
Metadata still represents every original actor/state; only admitted roles execute.

The fixed source roster is `[true,false,false,false]`: only PlayerSlot::One
(source index0) is active, regardless of its actor storage slot or health.
`spawn_player_body` or checked initial binding associates the genuine MT_PLAYER=0
with the existing OriginalPlayer. `original_player_roster()` derives current
health/mo from that record; it is an observation, not a separate mutable roster.
`perception_players()` remains the historical fixture-array observation and does
not describe this capability's active player. Its fixture registration setter
rejects here, including attempts to deactivate or fabricate players.

Before any unpaused tic or search, the player must have a current, nonpending,
correct-generation owned body with consistent definition/health/state. Incomplete
setup may be observed but cannot execute. Player activity persists when health
becomes zero; the allocated corpse remains owned but source search ignores its
player health. Removing this fixed active body rejects, including after death;
roster deactivation, body replacement and respawn require a future capability.
Other actor retirement/reuse retains existing generation safety. Foreign instance
references reject at existing provenance boundaries; raw ActorId remains local.
Damage can also retain a source in the player's mobj target, separately from
OriginalPlayer.attacker. Neither reference registers a player or enables player AI.
The scoped target view classifies compatibility with the perception-target role:
a retained Demon source can be Incompatible with that role while remaining a valid
stored identity. Actor fields/availability and source lifetime are separate observations.

At most one MT_SERGEANT=12 may enter through the original Demon THING batch,
before the player/effect suffix. Zero Demons is supported. This narrows the new
capability without changing historical four-owner or 64-total-slot bounds. All
existing Medium/commercial, grounded one-flat-sector, static axis-wall, geometry,
coordinate/travel and command bounds still apply. Player commands remain slot0,
forward/side -50..50, signed i16 turns under the declared bit interpretation,
buttons0 and default other slots. Physical damage composes through existing shared
services; it is not proof of autonomous melee. No sector soundtarget/noise
propagation is admitted; AMBUSH can use the source visual fallback.

The existing scanner preserves source lastlook, stopping slot and active encounter
count, then player-health/sight/facing checks. It does not sort by distance. Rear
angle strictly between ANG90/ANG270 permits acquisition only within approximate
64-unit distance. Initial lastlook1 with only player0 active stops at that slot
before eligibility on its first scan. Other initial lastlook values and repeated
scans keep original progression. New execution never feeds an empty active roster
to source scanning: original all-inactive search may not terminate. The historical
Rust EmptyRoster safety policy remains isolated in its historical configuration.

A_Look resets threshold, scans, assigns the real player's current ActorId, emits
the single Demon wake sound without sound-selection RNG, and immediately enters
SARG see/Chase chronology. Shared A_Chase supports target/reaction/threshold/facing,
movement direction, original collision/sight and RNG. Commands update player
angle/momentum before thinkers; XY moves at the player's own thinker position.
A Demon inserted first therefore initially sees pre-player-XY coordinates.

SARG attack states and autonomous melee are explicitly unexecutable here. If the
actual source melee check succeeds, the complete operation rejects before attack
sound/state/damage commit; chase does not silently skip the branch. Controlled
initial direction/state setup is distinct from automatic gameplay. Death or loss
of target shootability invokes source allaround fallback; failed scanning and
spawn entry retain an allocated dead target handle. Pending, retired, stale or
otherwise incompatible target/body references reject before execution and never
resolve to a different slot occupant. Nonshootable living player corruption rejects
as Rust invariant policy, not a claim about a separately admitted source branch.

Both stepping APIs share the same transaction; late rejection preserves player,
actor, targets, RNG, statistics, spatial/thinker order, commands, tic and both event
queues. Paused input retains historical no-op precedence. `advance` captures no
Snapshot; `step` captures once after successful advancement. DSIM22 policy5/context
select the fixed active contract with unchanged layout; [STATE_FORMAT](STATE_FORMAT.md#dsim-v22-active-player-demon-policy)
owns its precise identifiers.

As of this policy, full P_PlayerThink/P_Ticker/P_DeathThink, view/camera, weapons,
inventory, powers, armor/cheats, roster changes, respawn/multiplayer, generalized
targeting/infighting, other monster species, sound propagation, arbitrary geometry and
autonomous combat remain unsupported ([what came later](#deferred-systems)). [Source findings](M3G5_REFERENCE.md) and
[observed evidence](validation/M3G5.md) qualify this bounded gameplay composition.

## Autonomous original Demon-player melee

Select `Simulation::original_player_demon_melee_map` or the separate
`OriginalExecutionProfile::PlayerDemonMeleeV1`. This adds a public capability
variant and constructor; callers with exhaustive policy matches need a new arm.
It composes the existing fixed active player0, genuine MT_PLAYER=0 ownership,
map-THING MT_SERGEANT=12, commands, acquisition and pursuit. Historical policies,
including acquisition-only PlayerDemonAcquisitionV1, keep their original inputs,
state permissions and rejection behavior. No actor or geometry bound expands.

The admitted domain retains Medium/commercial single-player settings, at most one
Demon, one active owned player, floor0/ceiling128, static one-sector axis-aligned
one-sided walls, source BLOCKMAP/sight order and existing arithmetic/work limits.
Zero Demons is allowed. The same player slot0 commands execute before thinkers:
forward/side -50..50, signed i16 turns under the declared bit interpretation,
buttons0 and default other slots. No armor, powers, cheats, soundtarget propagation,
Fast/Nightmare, inventory, weapon or respawn configuration is implied.

Original A_Chase chooses melee using current approximate distance and sight, emits
the attack sound and immediately enters S_SARG_ATK1. ATK1/2/3 retain original
ordinals485/486/487, durations8/8/8 and Face/Face/SargAttack actions. The damaging
action occurs16 tics after selection, and returns to run chronology8 tics later.
The player can move during windup. SargAttack returns on absent target; otherwise
it faces and checks range again. A pristine player requires distance strictly
below60 units plus successful sight. A range miss consumes no damage draw.

An accepted range check draws exactly `((P_Random()%10)+1)*4`, giving4–40 damage
in increments of4. The current Demon is explicitly both source and inflictor in
DamageRequest. The existing shared damage service applies mass-scaled fixed-point
thrust before player health/attacker/damagecount and signed mobj health. Incoming
momentum includes the command stage and prior impulses; the player's later thinker
then performs existing XY/friction/sliding. The live inflictor remains executable
afterward and need not be killed or marked for removal.

Current ownership, generation, definition, state, health and grounded assumptions
are checked. The shared impulse/travel proof must accept the actual staged result:
component magnitude<=30 units and the existing conservative `160/3` travel envelope
inside±994. These are Rust admission limits, not original world limits or a clamp.
A well-defined C impulse outside that proof is an atomic unsupported operation.
Player SHADOW/power flags are outside the accepted source settings. Both z values
are0, so reverse-thrust height>64 and its conditional random draw are unreachable.

Nonlethal damage preserves the source pain draw, reaction reset and conditional
mobj retargeting. Player chance255 does not trigger pain on draw255. Player health
clamps0 while mobj health can become negative. Lethal damage enters the existing
player death path, clears solidity/shootability, retains momentum, sets PST_DEAD
and leaves an allocated corpse. Kill returns before the nonlethal reaction reset.
Death does not deactivate the roster, clear mo/attacker or erase the Demon's target.
Subsequent Chase handles nonshootability through the existing source scan/spawn
fallback; no actor-slot reuse can redirect retained generations.

Melee from positive health cannot produce mobj health below-39. Autonomous extreme
death(<-100) and the commercial below-50 high scream are therefore unreachable.
Existing controlled-damage extreme-death behavior remains separate coverage. If a
target dies during windup but remains in range, SargAttack can still draw damage;
P_DamageMobj then ignores the dead/nonshootable body without thrust, health or new
pain/death effects. This differs from a range miss and from invalid storage.

Existing live-owner controlled original state-entry restrictions still apply;
player pain/death remain damage-controlled and raise is excluded. The legacy
synthetic `demon_melee_attack` API remains unavailable with a genuine player.
Normal autonomous execution needs only advance/step, not that helper or injected
damage. Fixed player removal/rebinding, all-inactive roster transitions and
foreign/stale/pending relationship admission retain their checked boundaries.

All late failures discard the bounded existing transaction, including command,
actor/player fields, impulse, references, RNG, statistics, spatial/thinker order,
tic and pending/last events. No release exception or full-world clone is added.
`advance` constructs no implicit Snapshot; `step` uses the same core and captures
once after success. [DSIM22 policy6](STATE_FORMAT.md#dsim-v22-autonomous-demon-player-melee-policy)
uses the existing layout with a distinct context; policy5 remains no-melee.

This is a bounded command gate plus original actor thinkers, not complete
P_Ticker/P_PlayerThink/P_DeathThink. View/camera, damagecount decay, weapons,
pickups, armor/powers/cheats, roster changes, respawn/multiplayer, other monsters,
infighting, dynamic sectors, airborne/general geometry and rendering remain
excluded. [Source findings](M3G6_REFERENCE.md) and [observed evidence](validation/M3G6.md)
separate source behavior, reusable historical evidence and Rust safety policy.


## Genuine player pistol combat

Select `Simulation::original_player_demon_pistol_map` or the explicit
`OriginalExecutionProfile::PlayerDemonPistolV1` (wire7). Exhaustive public policy
matches need a new arm. It composes the existing genuine MT_PLAYER0, at most one
map-spawned MT_SERGEANT12, fixed active player0, commands, acquisition, pursuit and
autonomous melee. Older policies, including the no-weapon melee policy6, retain
all prior inputs, diagnostics and rejection behavior. No synthetic body is
reclassified as a player.

After spawning/binding the fresh live health100 body, call
`initialize_player_pistol(PlayerSlot::One, INITIAL_PISTOL_BULLETS)` before any tic.
G_PlayerReborn uses50; explicit initial values0..200 are controlled inventory
setup, not pickups. Double setup, wrong capability, absent/stale/pending body and
noninitial setup reject. PistolState observations are copies of weapon-only
fields; mo/health/state/attacker remain authoritative in the existing player.
Fixed player retirement/rebinding and foreign-instance admission remain checked.

Settings are Medium/commercial/single-player, floor0/ceiling128, one static flat
sector, axis-aligned one-sided walls, at most128 trace lines and existing64 actor,
coordinate/travel, BLOCKMAP, sight and work bounds. Turning and forward/side
-50..50 retain the prior arithmetic qualification. Only BT_ATTACK is newly
admitted; other player slots must remain default. Dead-player commands must be
default, while existing corpse physics and source weapon lowering continue.
There is no full PlayerThink/DeathThink, view/camera or damagecount decay.

Original pistol overlays use10..17 plus immediate LightDone1/Null0, distinct from
mobj states. Body attack states154/155 execute under this capability and the separately selected
weapon expansion capability. Setup
immediately raises128 to122; continuous attack can select at tic15 and first
fires at19. Pistol attack durations4/6/4/5 and immediate ReFire yield a14-tic held
shot interval. Ammo decrements at FirePistol, after sound/body-state and before
flash/aim/shot. Release does not cancel a selected windup. Refire resets when the
source ReFire action observes release, not at arbitrary button-change time.

P_GunShot draws5/10/15 damage first. An accurate refire0 shot has no spread draws;
inaccurate shots draw two ordered spread values. Autoaim tries the current angle,
+2^26 and-2^26 at1024 units; it changes slope, not the actual bullet angle. Bullet
range is2048. Both use the same source BLOCKMAP/intercept service. Original
P_PathTraverse processes collected intercepts after its bounded64-cell loop even
when the endpoint was not reached; this newly observed pistol domain admits that
finite result. Historical trace APIs retain their stricter endpoint rejection.
Puff37 and Blood38 are real allocated thinkers with source RNG and same-tic
thinker insertion, not presentation placeholders. Walls and misses cannot
fabricate Demon damage.

One-sector P_NoiseAlert sets the genuine player's sector soundtarget at
FireWeapon selection, before the later bullet action. Nonambush Look may wake
from sound despite rear facing or a wall; ambush retains its sight check. This is
not generalized sound propagation. Direct damage retaliation is separately
performed by the shared damage service. Old acquisition/melee policies still
reject sector soundtargets as before.

A real hit uses the current owned player as both source and inflictor, applies
checked mass400 Demon thrust, then original health/pain/reaction/retarget/death
processing. The existing single-player kill counter is the source player kill
count; no duplicate statistic is stored. Surviving AI and autonomous Demon melee
continue through the same core. Player death immediately lowers the weapon,
retains mo/attacker and active roster membership, and leaves an allocated moving
corpse. A dead player never fires or automatically respawns.

Empty ammo may idle, but source CheckAmmo fallback requires an unadmitted weapon.
That continuation rejects atomically, including a late ReFire check after
release. There is no infinite ammo or silently executed fist. Trace, capacity,
impulse, ownership, unsupported-action and later AI failures preserve ammo,
psprites, health, references, RNG, statistics, soundtarget, ordering, queues and
tic. `advance` constructs no implicit Snapshot; `step` captures once from the
same execution. [DSIM23](STATE_FORMAT.md#dsim-v23-genuine-player-pistol) adds only
the required dynamic weapon fields; historical20/21/22 remain intact.

As of this policy, other weapons, switching, pickups, armor/powers/cheats, respawn,
multiplayer, other monsters, infighting, full player thinkers, airborne/general geometry,
dynamic sectors, generalized sound propagation and rendering remain unsupported
([what came later](#deferred-systems)).
New source scenarios use defined map angles0/-180 and nonnegative turns; inherited
negative-turn and ordered spread/jitter qualifications are not new portable
ISO-C evidence. [Investigation](M3G7_REFERENCE.md),
[reproduction](TESTING.md#genuine-player-pistol) and [observations](validation/M3G7.md)
state exactly what source comparisons and Rust safety tests establish.

## Original player weapon expansion

Opt in with `original_player_demon_weapons_map` / `PlayerDemonWeaponsV1`.
The fixed active slot0 owns genuine MT_PLAYER0; at most one map-spawned
MT_SERGEANT12 shares the original command, acquisition, pursuit, melee and damage
core. Initialize once with `initialize_player_weapons(PlayerSlot::One,
WeaponInventory::default())` after fresh body binding, before any tic. Default
means fist+pistol, pistol ready,50 bullets. Alternative ready/owned/loadouts are
explicit controlled source-compatible setup: only fist/pistol/shotgun/chaingun,
fist+pistol always owned, bullets0..200, shells0..50, cells/rockets0. This is not
pickup gameplay. Wrong ready ownership, foreign/stale/pending body, double or
noninitial setup rejects. `original_player_weapons` returns a copy of weapon-only
observations; OriginalPlayer still owns body, health, attacker and lifetime.

Medium/commercial settings, grounded movement, one static floor0/ceiling128 sector,
axis-aligned one-sided walls,128 trace lines,64 actors and all earlier coordinate,
travel, sight and work proofs remain. Forward/side commands are -50..50; turning
retains the original arithmetic qualification. Player0 accepts BT_ATTACK and
BT_CHANGE with only the four supported weapon selections. Unsupported button bits,
weapon-mask bits without CHANGE and commands in other slots reject. A request for
an unowned supported weapon follows source behavior and does nothing; it never
grants ownership or ammunition. Dead-player commands must be default.

Empty ammunition selects an owned usable chaingun, shotgun, pistol or fist in
source priority. Pending selection enters the actual ready weapon's downstate;
ready changes at bottom, followed by raise and ready actions. Requests are not
instantaneous. A pending manual selection can be superseded by the source
CheckAmmo branch. Attackdown/refire, held attack, release and selected windup
retain source chronology. Empty chaingun still makes the source sound request
before returning without bullet/flash/ammo/damage draws. No infinite ammo exists.

Unpowered fists draw damage2..20 then two spread draws, aim/attack at64 units,
and use real current intercepts. An actual actor hit produces the source punch
sound and faces the target's **post-damage** position. Shotgun consumes one shell,
aims once, then resolves seven independent inaccurate bullets sequentially through
effects, damage, pain/death and retaliation. Chaingun uses pistol bullet damage but
its own cadence, two initial accurate shots, refire spread and state-dependent
flashes. No shotgun aggregation or precomputed future hits/health is used.
Pistol behavior retains the earlier source algorithm within this new capability.

Immediate attack actions can execute before P_FireWeapon's NoiseAlert: chaingun
can damage/retarget the Demon first. The same admitted one-sector soundtarget then
feeds existing AI. Fist/pistol/shotgun windups place noise before their later attack
action. All effects are real Puff/Blood thinkers; the same Demon can hurt or kill
the genuine player while weapons execute. Death lowers the actual ready weapon,
retains pending/ownership/roster/body/attacker, and continues supported corpse
physics. No death camera, respawn or dead-player firing is implemented.

Every recoverable failure discards the existing bounded transaction, including
partial pellet effects/damage, ammo/psprites, commands, soundtarget, references,
RNG, statistics, ordering, events and tic. `advance` constructs no implicit
Snapshot; `step` runs that core and captures once. Historical pistol-only and
synthetic weapon policies retain all prior rejection and diagnostic contracts.
[DSIM24](STATE_FORMAT.md#dsim-v24-original-player-weapon-expansion) records the
additional dynamic ownership, ready/pending and shell fields explicitly.

No Berserk, other weapons, weapon pickups, armor/powers/cheats, inventory expansion,
full PlayerThink/DeathThink/view, respawn, multiplayer, additional monsters,
infighting, general sound propagation, arbitrary geometry, dynamic sectors or
rendering is admitted. Source scenarios are a qualified bounded command/weapon
stage before original thinkers, not full ticker equivalence. The source-only
out-of-enclosing-wall scenario eventually exceeds the unchanged Rust978-unit
Demon admission proof; Rust compares its supported prefix and rejects atomically
rather than following that later C path. See [source qualifications](M3G8_REFERENCE_INVESTIGATION.md),
[reproduction](TESTING.md#original-player-weapon-expansion) and
[observations](validation/M3G8.md).


## Original world pickups

`PlayerDemonPickupsV1` (wire policy9) composes the four-weapon/player/Demon
capability with one actual original player0, fixed active roster, at most one
map Demon and at most16 placed plus controlled dropped items within64 actor slots.
Medium/commercial settings and existing grounded one-sector, flat0/128,
axis-aligned one-sided wall, coordinate/travel, sight and trace budgets remain.
There is one player, actor store, inventory, movement service and thinker pass.

| Original item | DoomEd | Executable grant |
| --- | ---: | --- |
| MT_CLIP |2007|10 bullets; controlled dropped Clip gives5 |
| MT_MISC17 |2048|50 bullets |
| MT_MISC22 |2008|4 shells |
| MT_MISC23 |2049|20 shells |
| MT_MISC10 |2011|10 health |
| MT_MISC11 |2012|25 health |
| MT_SHOTGUN |2001|Shotgun ownership and8 shells; dropped4 |
| MT_CHAINGUN |2002|Chaingun ownership and20 bullets; dropped10 |

`spawn_map_things` admits this subset in the existing atomic THING pass, retaining
single-player/skill filtering, angle admission and input order. Demon count and
item count are separate. Map kill totals follow MF_COUNTKILL; none of these items
has MF_COUNTITEM, so level.total_items and source player itemcount remain0.
Items may surround the map Demon in placement order; all precede the bound body
and effect suffix. Original monster identities remain distinct from weapon items.
Raw `spawn_type` does not grant item execution. `spawn_dropped_pickup` admits only
Clip/Shotgun/Chaingun before body binding and tic0, through actual raw spawn plus
MF_DROPPED. This is controlled setup, not monster loot or runtime respawn.

Collection occurs only through the existing moving player's thing checks. Strict
square overlap precedes line checks; source vertical bounds are inclusive body
height/−8, and nonpositive toucher health refuses collection. The grounded policy
does not grant airborne item access. A successful pickup remains collected if a
later wall/solid actor normally blocks movement. It is immediately unlinked from
spatial queries, marked pending and retired by existing thinker traversal. Its old
handle never resolves to a later effect reusing that slot. A recoverable error
instead discards the complete transaction, including earlier grants/removals.

Ammo caps at200 bullets/50 shells. Full resources refuse without removing the
item. Zero-to-nonzero ammo selection uses current readyweapon: bullets select
owned chaingun or pistol from fist; shells select owned shotgun from fist/pistol.
GiveWeapon grants ammo before new ownership/pending selection. Already-owned
weapons can grant ammo; new ownership still collects with full ammo. Pending
selection uses existing source lowering/raising, never instantaneous replacement.
Health caps at100, synchronizes the same player/mo and preserves pain, attacker
and damagecount. Dead bodies cannot heal or collect. Medikit message selection
uses post-heal health; GOTMEDINEED is unreachable for positive admitted health.

Accepted grants assign a typed persistent source message, mark removal, add6 to
bonuscount, then request source item/weapon pickup sound without an actor origin.
Only the live bonuscount decrement fragment after the weapon stage is admitted;
no HUD message clearing or full PlayerThink/DeathThink is implied. The16-item
one-time budget bounds bonuscount at96; this is Rust admission policy, not a
claimed original source cap. Original P_RemoveMobj also writes an inactive item
respawn queue; those C-only PQ observations are retained, while queue storage and
respawning are outside this executable/diagnostic contract. Audio requests do not
implement playback or active-sound ownership.

Historical no-pickup policies and DSIM20–24 remain unchanged. DSIM25 separately
records pickup fields/events; decoding never restores a game. As of this policy, armor,
powers, keys, backpacks, other items/weapons, player rebirth, multiplayer, generalized
specials and dynamic geometry remain excluded ([what came later](#deferred-systems)). Source evidence, explicit harness
adapters, generation/provenance safety and rollback are separate claims; see
[investigation](M3G9_REFERENCE_INVESTIGATION.md), [schema](STATE_FORMAT.md#dsim-v25-original-world-pickups)
and [observations](validation/M3G9.md).

## Maintenance compatibility boundary

Reference compression and shared weapon formulas change no supported gameplay
profile, actor admission, capacities, error contract or diagnostic schema. The
historical raw bytes remain in Git and are supplied exactly to existing test
parsers. Source arithmetic retains its qualified ordered-draw/BAM interpretation.
See [storage limits](EVIDENCE_STORAGE.md) and [formula scope](WEAPON_FORMULAS.md).


## Original armor and damage absorption

`PlayerDemonArmorV1` is a separate opt-in composition of the existing grounded
single-player/four-weapon/autonomous-Demon world. Player slot0 owns canonical
MT_PLAYER=0; roster remains [true,false,false,false], including after death.
At most one map MT_SERGEANT=12 and16 total world items share64 actor slots.
Medium/commercial, static flat one-sector/axis-aligned one-sided wall, existing
command/coordinate/travel/sight/trace bounds and transaction guarantees remain.
Earlier policies and their no-armor contracts are unchanged.

New executable items are MT_MISC0=43/DoomEd2018 (green) and MT_MISC1=44/DoomEd2019
(blue), with real states802/803 and804/805, sprites55/56 and fullbright frame bits.
Map spawning consumes the original lastlook and positive timer draws. Shared
thinkers execute green6/7 and blue6/6-tic animation loops. Armor is map-spawned;
controlled dropped armor, raw arbitrary item spawn/state/target/damage operations
remain rejected. These are SPECIAL, non-COUNTITEM actors.

One player-owned armor record starts None/0. Ordinary green is type1 with1..100
points; blue is type2 with1..200. GiveArmor compares current points to100/200,
not class ranking: blue80 accepts green100, blue100/180 refuses green. Successful
contacts use source message GOTARMOR/GOTMEGA, removal, BONUSADD6 and itemup sound.
A normally blocked later wall move retains collection; recoverable failures roll
back the whole operation. Dead bodies cannot collect or resurrect.

For live shootable player damage, grounded physical thrust uses incoming raw
damage first. Green saves floor(raw/3), blue floor(raw/2), capped by remaining
points. Equality depletes and clears type. Effective damage updates clamped player
health, signed mobj health, attacker, damagecount and tactile response before
existing pain/death logic. Zero damage still follows the admitted source path;
ignored dead/nonshootable targets consume no armor or new RNG. Demons never gain
player armor. Autonomous SARG strikes and controlled damage share that same plan.
Dead players retain armor, body, attacker and active status; corpse physics remain
separate from retirement. Stale/foreign handles reject under Rust safety policy,
not unsafe C pointer emulation.

Real source-derived encounters collect armor before autonomous melee, fire actual
weapon commands, collect healing/ammo/weapons under pressure, deplete armor and
reach player or Demon death. The harness is the declared bounded command/weapon
stage before original thinkers, not complete PlayerThink/Ticker/DeathThink.
This policy admits no armor/health bonuses (see the separate
[bonus capability](#original-health-and-armor-bonuses)), powers, god mode, Baby
scaling, special-sector damage, telefrag system, new weapons/monsters, respawn,
multiplayer, general airborne or dynamic geometry. Existing reverse-thrust RNG is
unreachable in this grounded domain. See
[source qualifications](M3G10_REFERENCE_INVESTIGATION.md) and
[observed evidence](validation/M3G10.md).

## Original health and armor bonuses

`PlayerDemonBonusesV1` (wire11, context
`PLAYER-DEMON-1-MEDIUM-GROUNDED-BONUSES-v1`, DSIM-v27) is a separate opt-in
superset of the armor world. Select it with `Simulation::original_player_bonuses_map`.
The same single player0, at most one map MT_SERGEANT=12, four weapons, 16 world
items, 64 actor slots, Medium/commercial settings, grounded static geometry and
transaction guarantees apply. Older policies, wires and DSIM20–26 are unchanged and
continue to reject bonus items.

New executable items are MT_MISC2=45/DoomEd2014 (health bonus, SPR_BON1) and
MT_MISC3=46/DoomEd2015 (armor bonus, SPR_BON2). Both use their real six-state
loops (816–821 and 822–827): six tics per state, frames 0,1,2,3,2,1, no actions,
flags SPECIAL|COUNTITEM. Map spawning draws the source lastlook and one positive
timer randomization per placed item, in THING order; animation draws no RNG.

On contact, a health bonus adds 1 health up to 200 and syncs the body's health; an
armor bonus adds 1 point up to 200 and sets green type only when the player had
none. Bonuses are always collected, even at the cap. Ordinary items keep their
rules: stimpacks/medikits still refuse at 100+, and green/blue grants still compare
current points to 100/200. Under this policy only, green armor may exceed 100 and
still absorbs one third.

Bonus items are the only admitted COUNTITEM actors. `level.total_items` counts
filtered placed bonuses at spawn and never decreases; the player's `item_count`
increases after the pickup message and before removal, matching
`P_TouchSpecialThing`. Recoverable failures roll back RNG, statistics, player
state, spatial links and events together.

As of this policy, soulspheres, MegaSpheres, powers, keys, backpacks, item respawn, dropped
bonuses, new weapons/monsters, multiplayer and dynamic geometry remain excluded
([what came later](#deferred-systems)). Physical
component T40's post-hit result (raw 300 damage exceeding the 30-unit momentum
admission) stays source-only; its pickup is compared and the hit is atomically
rejected. See [source investigation](M3G11_REFERENCE_INVESTIGATION.md) and
[observed evidence](validation/M3G11.md).

## Original former humans and multi-monster encounters

`PlayerMonstersV1` (wire12, context `PLAYER-MONSTERS-1-MEDIUM-GROUNDED-v1`,
DSIM-v28) is a separate opt-in superset of the bonus world. Select it with
`Simulation::original_player_monsters_map`. The same single player0, four weapons,
16 placed world items, 64 actor slots, Medium/commercial single-player settings,
grounded static axis-wall geometry and transaction guarantees apply. Older policies,
wires and DSIM20–27 are unchanged and continue to reject former humans.

Admitted monsters are map THINGs of MT_POSSESSED=1/DoomEd3004,
MT_SHOTGUY=2/DoomEd9, MT_CHAINGUY=10/DoomEd65 and MT_SERGEANT=12/DoomEd3002, in
any mix, at most `MAX_PLAYER_MONSTERS` = 8. The cap keeps one player, 16 items, at
most eight drops and transient impacts inside 64 slots and each tic's checked sight
budget (64 queries). THING angles keep the earlier admission of `angle / 45` quotients in
[-4, 3], which avoids the original signed `ANG45` product overflow. Monsters spawn in THING order
with source timer randomization and `total_kills`.

Former humans use their original spawn/see/missile/pain/death/xdeath closures,
sounds and flags. They wake through `A_Look`, chase and choose missile attacks
through the original `A_Chase`/`P_CheckMissileRange`, and fire `A_PosAttack`
(one bullet), `A_SPosAttack` (one aim, three complete bullets) and
`A_CPosAttack`/`A_CPosRefire` with the original draw order. Each bullet is traced
and damaged against the current state of the tic, so earlier shooters' kills, pain
and RNG are visible to later ones. Monster bullets and Demon bites damage the
player (including armor), other monsters, or nothing.

`P_DamageMobj` infighting is the source rule: a damaged actor with zero threshold
retargets to any different source, sets threshold 100 and enters its see state if
it was in its spawn state. There is no same-species exception and no MT_VILE.
Every COUNTKILL death is credited to `single_player_kill_count`, whoever killed it.
A dead former human drops MT_CLIP / MT_SHOTGUN / MT_CHAINGUN at its position with
MF_DROPPED; dropped ammo is halved by the existing pickup rules. Drops are
appended after the player in thinker order and are not counted items.

As of this policy, raise/resurrection, MT_VILE and every other monster type, projectiles,
nightmare and fast monsters, respawn, multiplayer and dynamic geometry remain excluded
([what came later](#deferred-systems)).
The original attack actions store BAM angles in `int` locals; `angle += spread`
is signed overflow near 180°. The default original-C build keeps that text under
the declared GNU wrapping interpretation, and strict audits retype only those
locals to `angle_t` and must reproduce the identical trace. Rust uses wrapping
BAM addition. Recoverable failures, including impact or drop capacity exhaustion,
roll back actors, player, RNG, statistics, spatial links and events together. See
[source investigation](M3H1_REFERENCE_INVESTIGATION.md) and
[observed evidence](validation/M3H1.md).

## Original Imps and fireballs

`PlayerMonstersImpV1` (wire13, context `PLAYER-MONSTERS-IMPS-1-MEDIUM-GROUNDED-v1`,
DSIM-v29) is a separate opt-in superset of `PlayerMonstersV1`. Select it with
`Simulation::original_player_imps_map`. Everything that section states still applies.
Older policies, wires and DSIM20–28 are unchanged and continue to reject the Imp and
the fireball.

MT_TROOP=11/DoomEd3001 joins the mixed roster under the unchanged
`MAX_PLAYER_MONSTERS` = 8. Eight stays right because a monster costs at most a few
sight queries per tic against the budget of 64, and fireballs cost none. Imps use their
original spawn/see/pain/attack/death/xdeath closures, sounds and flags. The one attack
state chooses at action time: `A_TroopAttack` faces the target, then claws for
`(P_Random()%8+1)*3` when `P_CheckMeleeRange` passes, otherwise launches. Imps drop
nothing. XDeath needs a single hit above 60, more than any admitted in-world attack;
it stays reachable through `damage_actor`.

MT_TROOPSHOT=31 is spawned only by `A_TroopAttack`, 32 units above the Imp, at speed
10. It runs in the same thinker pass that launched it, deals `((P_Random()%8)+1)*3`
on contact, and explodes on walls and solid things. Items and corpses do not stop it.
The inflictor is the fireball and the source is the Imp, even if the Imp has since
died, so `P_DamageMobj` infighting needs no new rule.

The source same-species rule applies to missiles only: a fireball explodes on another
Imp with no draw, damage or retarget, and passes through its own owner. It still hits
the player and other species. Bullets and claws have no such exemption.

| Bound | Value | Why |
| --- | ---: | --- |
| Actor slots | 64 | Unchanged store |
| Monsters | 8 | Unchanged; 1 player + 8 + 16 items + 8 drops = 33 slots |
| Live fireballs, flying or exploding | `MAX_LIVE_PROJECTILES` = 24 | Measured margin, not a proof: unscripted soaks of eight Imps peaked at 19 |
| Remaining impact slots | 7 | 64 - 33 - 24, when every other budget is full |

The original has no fireball cap; this bound is this project's. A launch past the cap
(`ProjectileError::WorkLimit`) or past the store (`StoreError::Capacity`) and a full
store that refills a slot within one pass (`StoreError::VisitLimit`) each reject with
the whole tic rolled back. That tic keeps rejecting until the caller changes the state,
and player commands may not clear it; removing fireballs with `mark_remove` can. There
is no partial substitute. `mark_remove` rejects the owner of a flying fireball. The
existing `|position| + 160M/3 <= 994` travel envelope also applies to fireball knockback,
so a hit near a wall can reject its tic in the same way, even with no escaping input.
The earlier promise of a safe zero-command continuation does not hold for that case.
Fireballs cannot be spawned, entered, retargeted or pushed through the
public API, and no actor may name one as a target or damage source.

The geometry stays grounded: one floor height, one-sided axis-aligned blocking walls,
no sky. Every shootable actor stands at z 0, so a fireball flies level with zero
vertical momentum. Floor and ceiling contact, the sky hack, line openings and the
SHADOW spread are unreachable here; this policy makes no observation claim for them.
Actor coordinates keep the existing 994-unit preflight bound, so a map whose walls
lie beyond it rejects a fireball that reaches that distance. A blocked spawn check
needs overlapping THINGs. As of this policy, every other projectile and monster, raise,
nightmare and fast monsters, respawn, multiplayer and dynamic geometry remain excluded
([what came later](#deferred-systems)). See
[source investigation](M3H2_REFERENCE_INVESTIGATION.md) and
[observed evidence](validation/M3H2.md).

## Original monsters at real-map scale

`PlayerMonstersMapScaleV1` (wire14, context
`PLAYER-MONSTERS-MAP-SCALE-1-MEDIUM-GROUNDED-v1`, DSIM-v30) is a separate opt-in
superset of `PlayerMonstersImpV1`. Select it with
`Simulation::original_player_map_scale_map`, which takes no slot limit: the
capacity is part of the proof. The same roster, weapons, items, drops, fireballs,
infighting, statistics and whole-tic rollback apply. Older policies, wires and
DSIM20–29 are unchanged and keep every margin and envelope listed in the sections
above, including the stalls they can produce.

What changes is the coordinate admission. The historical 978/994/1,008-unit actor
margins, the `|position| + 160M/3 <= 994` travel envelope, its `M <= 30` rejection and
the command stage's `4M + 8` margin do not apply to this policy. In their place the
loaded map itself bounds every actor: the vertex rectangle `R` is derived once at
construction and the walls keep every mover inside it, as the original's
`P_TryMove` does. The proof is in the
[investigation](M3H3_REFERENCE_INVESTIGATION.md#4-the-new-admitted-domain-and-its-proof).

| Admitted at load | Value | Why |
| --- | ---: | --- |
| Vertices, node origins/deltas, BLOCKMAP origin | within 2,048 | M3f2 intercept proof (`G = A = 2048`) |
| Line norm `abs(dx)+abs(dy)` | ≤ 1,536 | same proof |
| Linedefs | ≤ `MAP_SCALE_WALL_LINES` = 95 | with at most 33 BLOCKMAP-linked things, one trace never exceeds `MAXINTERCEPTS` 128 |
| Walls | one-sided, axis-aligned, `ML_BLOCKING`; each side of `R` fully covered by wall segments | `P_HitSlideLine` axis cases; a box can only leave `R` through a wall point |
| BLOCKMAP | origin at or below `R`'s minimum, grid strictly past its maximum, every wall listed in every cell its bbox touches | the source checks only the cells of the destination box |
| Sector | one, heights `[0,128]`, no specials, no sky | unchanged grounded domain |
| Spawns | closed bounding box inside `R` (THINGs, player body, dropped-pickup setup) | the invariant must hold from the start |
| Actor slots | `MAP_SCALE_ACTOR_SLOTS` = 320 | ≤ 241 live actors for any admitted roster, ≤ 256 visits per pass |
| Live fireballs | `MAP_SCALE_MAX_LIVE_PROJECTILES` = 208 | 26 per Imp from a 23-tic minimum launch interval (attack cut short by pain, then the JUSTHIT shortcut) and a 598-pass maximum lifetime in the largest room |
| Monsters, items, drops, sight queries | 8, 16, 8, 64 per tic | unchanged |

| Checked before every tic | Value | Status |
| --- | ---: | --- |
| Actor center | inside `R` | unreachable witness of the wall invariant |
| XY momentum per component | ≤ `MAP_SCALE_MOMENTUM_UNITS` = 4,096 | in-world impulses add at most about 650 units per pass and `P_XYMovement` clamps to 30 each tic; reachable only by repeated controlled `damage_actor` calls between tics, which reject with rollback |
| Z, vertical momentum | as before (`z + momz` within 1,024, `momz` within 64) | flat domain |

Legal source moves, wall slides, corner pinches and hits at walls are never rejected:
`advance` fails for this policy only on invalid inputs (stale or foreign handles,
commands outside the source range, non-zero commands to a dead player, unsupported
buttons, pause), on the two witnesses above, or on a controlled launch past 208
fireballs. A map that fails any load check is rejected by the constructor with
`MapScaleError::{Walls, Blockmap, Lines}`, `GeometryError::CoordinateOverflow` or the
inherited grounded-geometry rejection, never during play. Spawns whose box crosses a
wall reject with the historical spawn errors and unchanged state.

E1M1-sized rectangles (about 3,900 × 3,200 units) fit when their long walls are
split at vertices. Non-axis walls stay excluded: with `P_AproxDistance(30,30) = 45`,
a 22.5° slide can move a player 38 units, more than its 32-unit box, so the wall
argument does not hold for them. As of this policy, two-sided lines, height changes,
moving sectors, other monsters and projectiles remain excluded ([what came later](#deferred-systems)). See
[source investigation](M3H3_REFERENCE_INVESTIGATION.md) and
[observed evidence](validation/M3H3.md).

## Original monsters with static heights

`PlayerMonstersHeightsV1` (wire15, context `PLAYER-MONSTERS-HEIGHTS-1-MEDIUM-v1`,
DSIM-v31) is a separate opt-in superset of `PlayerMonstersMapScaleV1`. Select it with
`Simulation::original_player_heights_map`, which takes no slot limit. The same roster,
weapons, items, drops, fireballs, infighting, statistics, walled rectangle, slot and
fireball budgets, momentum bound and whole-tic rollback apply. Older policies, wires
and DSIM20–30 are unchanged and keep the flat single sector, the `z == floor`,
`momz == 0` and 64-unit witnesses and every margin listed above.

What changes is the vertical domain. The map may hold up to 256 axis-aligned sectors
with different static floor and ceiling heights joined by two-sided linedefs: steps,
ledges, drop-offs, windows, closed sectors and fences. Every mover follows the
original bodies that the engine already carried for the M3b profile: `P_LineOpening`,
`PIT_CheckLine`'s `tmfloorz`/`tmceilingz`/`tmdropoffz`, `P_TryMove`'s fit, headroom,
24-unit step and dropoff rules, `P_SlideMove` against blocking two-sided lines,
`P_ZMovement` gravity, landing and ceiling clipping, `P_Move` snapping a walking monster
to its floor, `P_CheckSight`/`P_CrossSubsector` slopes with REJECT, `P_AimLineAttack`
and `PTR_ShootTraverse` through openings and onto upper and lower walls, fireball
`momz` chords and floor/ceiling explosions, and `P_RecursiveSound` through open
two-sided lines with `ML_SOUNDBLOCK`. Three behaviors the flat world could never reach
become reachable: the player's landing sound request (`sfx_oof` when `momz < -8`),
weapon noise flooding every connected sector, and the `P_DamageMobj` fall-forward
branch for a lethal weak hit from more than 64 units below (one extra gameplay draw).
The proof is in the [investigation](M3H4_REFERENCE_INVESTIGATION.md#4-the-admitted-domain).

| Admitted at load | Value | Why |
| --- | ---: | --- |
| Vertices, node origins/deltas, BLOCKMAP origin, line norm, linedef count, slots, fireballs, roster, items | as the map-scale policy | unchanged proofs |
| Sectors | ≤ 256 (in practice far fewer: the 95-linedef cap bounds a walled strip to about 30), heights within ±512, floor ≤ ceiling, no specials, no `F_SKY1`; spans may be shorter than any actor (windows, crawlspaces, closed sectors) | source geometry; a closed door loads as a static wall |
| Linedefs | axis-aligned, not degenerate, a front side, `ML_TWOSIDED` exactly when a back side exists; flags within `{1, 2, 8, 16, 32, 64, 128, 256}` plus 4 | `P_HitSlideLine` axis cases; bits 1, 2, 4 and 64 are read by the admitted bodies, the rest only by the renderer and automap |
| Walls | each side of `R` fully covered by **one-sided** linedefs of any admitted flags | `PIT_CheckLine` blocks a one-sided line regardless of flags |
| BLOCKMAP | every linedef, one- or two-sided, listed in every cell its bbox touches | the source checks only the destination box's cells |
| Type dimensions | each type fits the tallest sector | Rust-only; fitting is proved per spawn instead of map-wide |
| Spawns | closed box inside `R` **and** the type's height fits the spawn sector's span (THINGs, player body, controlled spawns, dropped pickups, `P_KillMobj` drops) | Rust-only: the source would stand a too-tall thing through the ceiling; rejecting it costs no legal play |

| Checked before every tic | Value | Status |
| --- | ---: | --- |
| Vertical invariant | every non-impact actor has `floor <= z <= ceiling - height` with its own `floorz`/`ceilingz` | unreachable witness (`MapScaleError::Vertical`); puffs and blood spawn at absolute `z` and are clipped by `P_ZMovement`, as in the source |
| `z`, `momz` | within 2,048 and `HEIGHTS_MOMZ_UNITS` = 1,024 | a fireball chord is within 968, gravity within 46; unreachable witness |
| Items | `z == floor`, `floor`/`ceiling` equal to the item's own sector | replaces the flat `0/128` |

Legal source moves are never rejected. A map that fails a load check is rejected by
the constructor with `MapScaleError::{Geometry, Walls, Blockmap, Lines}`, the inherited
specials, sky, height-range and inverted-sector rejections, never during play; a spawn
that does not fit rejects with `MapScaleError::Fit` and unchanged state, and a
`P_KillMobj` drop that would not fit fails its checked tic. As of this policy, moving
sectors, doors, lifts, crushers, line and sector specials, damaging floors, teleporters,
sky ceilings, non-axis walls, `P_CalcHeight`/view bobbing and every other monster or
weapon stay excluded ([what came later](#deferred-systems)). See [source investigation](M3H4_REFERENCE_INVESTIGATION.md) and
[observed evidence](validation/M3H4.md).

## Original monsters among diagonal walls

`PlayerMonstersDiagonalV1` (wire16, context `PLAYER-MONSTERS-DIAGONAL-1-MEDIUM-v1`,
DSIM-v32) is a separate opt-in superset of `PlayerMonstersHeightsV1`. Select it with
`Simulation::original_player_diagonal_map`, which takes no slot limit. The same roster,
weapons, items, drops, fireballs, infighting, statistics, slot and fireball budgets,
momentum bound, vertical domain, witnesses and whole-tic rollback apply. Older policies,
wires and DSIM20–31 are unchanged and keep their axis-aligned admission.

What changes is the linedef admission. Linedefs may have any direction, one- or
two-sided, and the walled rectangle is replaced by a **closed boundary** of one-sided
walls: wall slides at any angle run through the full `P_HitSlideLine` projection
(`R_PointToAngle2`, `finecosine`/`finesine`, `P_AproxDistance`), `P_BoxOnLineSide`
reads diagonal walls through its corner tests, intercepts, sight, hitscan, autoaim,
fireballs and `P_NewChaseDir` meet diagonal lines through the bodies the engine already
carried. The source's reading quirks are reproduced, never corrected: a box whose corner
lies exactly on a diagonal wall is blocked from the wall's front side but not from its
back, a box approaching from the back may stop up to `1/L` units past the line, a slide
trace starting in that sliver can pick a wall it runs parallel to with fraction 0, a
player running straight into a convex 45°/45° tip slides in place, and a diagonal trace
through a BLOCKMAP cell corner visits two of the four cells. The proof is in the
[investigation](M3H5_REFERENCE_INVESTIGATION.md#4-the-admitted-domain-and-the-proof).

| Admitted at load | Value | Why |
| --- | ---: | --- |
| Vertices, node origins/deltas, BLOCKMAP origin, line norm `abs(dx)+abs(dy) <= 1536`, linedef count, slots, fireballs, roster, items, sectors, heights, flags | as the heights policy | unchanged proofs |
| Linedefs | any direction, not degenerate, a front side, `ML_TWOSIDED` exactly when a back side exists; the heights flag set | every `P_HitSlideLine` branch is admitted; the projected move's bound is the escape witness below |
| Boundary | the one-sided walls meet only at shared endpoints (C1), every vertex ends an even number of walls (C2), every wall's front side has odd ray parity (C3) | the walls form a closed chain `S` whose odd-parity region `Ω` is the room; C3 rejects a wall facing away from its room, which a real map could carry (to revisit with real-map loading) |
| BLOCKMAP | the grid covers the vertex rectangle; every linedef, one- or two-sided, is listed in every cell whose closed 128-unit square meets the closed segment | the cell holding any wall point lists that wall; real node builders list a line only in the cells it crosses, which real-map loading must revisit |
| Spawns | center with odd parity **and** the closed box meeting no wall (THINGs, the player body, controlled spawns and controlled dropped pickups), plus the heights fit | Rust-only; the open box then lies in `Ω`. A `P_KillMobj` drop keeps only the heights fit: it spawns at the corpse's center with a box inside the corpse's, which the invariant already keeps inside `Ω` (and a corpse may legally stand with its box edge exactly on an axis wall) |

| Checked during a tic | Value | Status |
| --- | ---: | --- |
| Escape witness | after every projected slide move `P_TryMove` accepts, the player's center still has odd parity | the one refused legal source move: a projected slide of more than 30 units per axis can jump a corner (the `corner_jump` observation); the tic fails with `MapScaleError::Escape` and the state before it is kept. Every other move, spawn and controlled operation keeps Invariant Ω by Theorem B of the investigation, so the witness never fires for them. A known fidelity gap to revisit for demo compatibility |
| Rectangle, momentum, vertical, item witnesses | as the heights policy | `Ω` lies inside the vertex rectangle, so they never fire |

`Simulation::set_momentum`, refused for every actor under the older physical-player
policies, is admitted under this policy for the player body (the mirror of the C
harness's declared push input), monsters, corpses and items, bounded by the 4,096-unit
momentum admission; fireballs stay refused by the projectile control, so every pushed
mover is one the proof's clamp covers. A map that fails a load check is
rejected by the constructor with `MapScaleError::{Geometry, Boundary, Walls, Blockmap,
Lines}` or the inherited rejections, never during play; a spawn whose box meets a wall
or whose center lies outside rejects with unchanged state. As of this policy, moving
sectors, doors, lifts, crushers, line and sector specials, damaging floors, teleporters,
sky ceilings, `P_CalcHeight`/view bobbing, cap increases and every other monster, weapon
or item stay excluded ([what came later](#deferred-systems)). See [source investigation](M3H5_REFERENCE_INVESTIGATION.md) and
[observed evidence](validation/M3H5.md).

## Original monsters at classic-map scale

`PlayerMonstersRealMapV1` (wire 17, context `PLAYER-MONSTERS-REALMAP-1-MEDIUM-v1`,
DSIM-v33) is a separate opt-in superset of `PlayerMonstersDiagonalV1`. Select it with
`Simulation::original_player_real_map_map`, which takes no slot limit. The same roster
(eight monsters, sixteen pickups, eight drops), weapons, items, fireballs, infighting,
statistics, 320 slots, 208 live fireballs, momentum bound, vertical domain (heights
within ±512), linedef flag set, boundary rules C1–C3, closed-cell BLOCKMAP rule, spawn
rule, escape witness and whole-tic rollback apply; every committed M3h5 room loads
unchanged. Older policies, wires and DSIM20–32 are unchanged and keep their caps.

What changes is the static-geometry admission (M3h6 note
[§4.1](M3H6_REFERENCE_INVESTIGATION.md#41-every-cap-before--after-with-its-derivation)).
Every cap of the diagonal world that stood in for an allocation or CPU budget becomes
the WAD format's own bound or a value computed from the loaded map; the cap that stood
in for the M3f2/M3h3 arithmetic envelope becomes a certificate evaluated on the map
against the same source expressions; the cap that stood in for `MAXINTERCEPTS` becomes a
runtime witness with an admission rule for slides.

| Cap | Diagonal policy | Real-map policy | Derivation |
| --- | ---: | ---: | --- |
| Linedefs | 95 | 32,767 | `short` references (`mapseg_t::linedef`, `line_t::sidenum`); the parser's own bound |
| Line-index buffers | 1,024 (`SeenLines`), 128 (hitscan `seen[]`) | sized by the line count | one bitset per trace, inline up to 1,024 lines, heap above (4 KiB at the format maximum) |
| Sidedefs, vertices, segs | coordinates only | 32,767 each | `short` references; no new check |
| Sectors | 256 | 32,767 | `mapsidedef_t::sector` is `short`; REJECT is `ceil(n²/8)` bytes by the parser |
| Subsectors, nodes | 4,096 each | 32,767 each | `unsigned short children[2]` with `NF_SUBSECTOR` as bit 15 |
| BLOCKMAP cells, words | 8,192 cells, 65,536 words | cells < words ≤ 32,768 | list offsets are `short` word indices (`P_BlockLinesIterator`) |
| Prepared references, preparation bytes | 262,144 refs, 4 MiB | computed and recorded; refused above 2²⁴ refs or 64 MiB | allocation guards no `short`-indexed IWAD map approaches (at most 13,435 words) |
| Coordinates, line norm | ±2,048; `\|dx\| + \|dy\| ≤ 1,536` | the arithmetic certificate (i)–(vii) below | §4.2 of the note, computed from the map |
| Sector heights | ±512 | ±512 | kept (decision 7); 58 of 68 IWAD maps fit |
| THING records | 256 | 32,768 | lump length by the parser; spawned actors stay bounded by the roster |
| Player starts 1–4, 11 | rejected | kept as records | `P_SpawnMapThing` 718–736; the player body is spawned by the caller at a position of its choosing (the source spawns it at start 1 in single player; the record's position is not used), a type-0 THING is `ActorError::InvalidType` |
| Sight work per query, per-tic budget | 16,384; 64 queries and 1,048,576 work | the map's expansion (bounded by `2 · nodes + subsectors + segs`); 64 queries and `64 · (expansion + 5 · lines)` | the 64-query cap is kept (at most 6 per tic observed in C) and is the binding one, since one query charges exactly `expansion + 5 · lines`; E1M1: 1,462 and 249,088 |
| Collision work per query | ≤ 16,384 | computed and recorded | `nodes + 4 · lines + 4 · 320` (E1M1 3,462) |
| Roster, slots, fireballs, pickups, drops, momentum, z witnesses, 64-cell DDA | 8 / 320 / 208 / 16 / 8, 4,096 / 2,048 / 1,024, 64 | unchanged | decisions 3 and 10; the DDA bound is the source's |

**The arithmetic certificate** (`ArithmeticCertificate::derive`, `MapScaleError::Arithmetic`
on failure), with `R` the vertex rectangle, `L₁ = max(|dx| + |dy|)` and `L₂² = max(dx² +
dy²)` over the linedefs, `C` the largest line component, `N` the largest node partition
norm: (i) `256 · 2,177 · L₁ + L₁ + 2 ≤ 2³¹ − 1` and `256 · L₁ · 2,049 ≤ 2³¹ − 1`
(hitscan `P_InterceptVector`; `L₁ ≤ 3,853`); (ii) `65,536 · diag(R)² · L₂² ≤ (2³¹ − 1 −
L₁ − 2)²` (sight `P_InterceptVector2`); (iii) `span_x + span_y < 32,768`
(`P_AproxDistance`); (iv) `max(span) < 8,192` and `C < 8,192` (`SlopeDiv`); (v) the
BLOCKMAP origin at or below the rectangle, the rectangle inside the grid and within
32,768 units of the origin; (vi) `(L₁ + N) · max(span) < 2³¹` (side tests); (vii) heights
within ±512; (viii) reach: `max |R| + 2,177 < 32,768` per axis, `max(R) − org + 2,177 <
32,768` and every node partition origin inside `R`, so hitscan endpoints, collision
boxes and BLOCKMAP indexes formed from a center in `R` stay within the `short` range.
`ArithmeticCertificate::evaluate` reports each clause's verdict. E1M1 (`L₁ = L₂ = 832`,
`diag = 5,373`) holds; across the 68 IWAD maps (i) holds for 64, (ii) for 46, (vii) for
58, (viii) for 68, all for 39 ([counts](validation/M3H6_IWAD_COUNTS.md)).
The slide-window rule (`slide_window_lines`, `MapScaleError::Lines` above 128) bounds the
distinct lines listed in any 2 × 2 window of BLOCKMAP cells, which is every line a slide
trace can meet, so the slide half of the intercept witness never fires (E1M1 28, IWAD
maximum 101).

| Checked during a tic | Value | Status |
| --- | ---: | --- |
| Intercept witness | a hitscan, autoaim or slide trace of this tic would write the source's 129th intercept | the second refused legal source behavior after the corner jump: the original writes it past `intercepts[MAXINTERCEPTS]` (undefined; demo-compatible ports emulate the DOS layout, deferred to M4). The tic fails with `MapScaleError::Intercepts` and the state before it is kept. Reached only by construction (the Rust-only witness hall); the largest trace observed in C holds 73 intercepts and E1M1's replica maximum is 67. A known fidelity gap to revisit for demo compatibility |
| Escape, rectangle, momentum, vertical, item witnesses | as the diagonal policy | unchanged |

Things are added to a trace once per visit of their cell and never deduplicated, as
`PIT_AddThingIntercepts` does; no re-add was observed. `Simulation::set_momentum` keeps
the diagonal admission for the player body, monsters and corpses and refuses a push to
an item (`PlayerBodyError::Unsupported`), whose momentum no source path produces and
whose state the item witness would refuse. A map that fails a load check is rejected by
the constructor with `MapScaleError::{Arithmetic, Geometry, Boundary, Walls, Blockmap,
Lines}`, `GeometryError::WorkLimit` (a format bound or an allocation guard) or the
inherited specials, sky, height-range, inverted-sector and short-REJECT rejections, never
during play. Admission time is linear in the lump for real maps (E1M1 about 1.6 ms in
release) but the cell preparation visits every line per cell start, so a hostile
format-maximum lump (32,764 cells of 32,767 lines) would take seconds; it is not an
admitted target. A THING of an unadmitted type still rejects the whole batch
(`MapSpawnError::UnsupportedType`), which is what keeps E1M1 itself out: after this
policy E1M1 needs the roster scale (29 monsters, 53 pickups on UV), eight decoration and
barrel types, sky, eight line-special and five sector-special types and the movers behind
them ([readiness](M3H6_REFERENCE_INVESTIGATION.md#9-e1m1-readiness-report)). Moving
sectors, doors, lifts, crushers, line and sector specials, damaging floors, teleporters,
sky ceilings, the view-height part of `P_CalcHeight` (`viewheight`, `deltaviewheight`,
`viewz` and its bobbing; the momentum bob `player->bob` is computed), intercepts-overflow
emulation and every other monster, weapon or item stay excluded under this policy (the load check rejects only
`F_SKY1` ceilings; a sky floor has no game-code effect, M3h7 note
[§2.9](M3H7_REFERENCE_INVESTIGATION.md#29-sky-in-game-code-re-read-for-this-note));
the [specials policy](#original-monsters-with-sky-and-static-specials) admits sky
ceilings and the static specials. See
[source investigation](M3H6_REFERENCE_INVESTIGATION.md) and
[observed evidence](validation/M3H6.md).

## Original monsters with sky and static specials

`PlayerMonstersSpecialsV1` (wire 18, context `PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1`,
DSIM-v34) is a separate opt-in superset of `PlayerMonstersRealMapV1`. Select it with
`Simulation::original_player_specials_map`, which takes no slot limit, spawn the THINGs
and the player body, then call `Simulation::spawn_level_specials` once before the first
tic. The same roster (eight monsters, sixteen pickups, eight drops), weapons, items,
fireballs, infighting, 320 actor slots, 208 live fireballs, heights within ±512,
arithmetic certificate (i)–(viii), slide-window rule, boundary rules C1–C3, closed-cell
BLOCKMAP rule, spawn rule, escape, intercept and vertical witnesses and whole-tic
rollback apply; every committed M3h6 room loads unchanged. Older policies, wires and
DSIM20–33 are unchanged; the real-map policy still rejects every special and every sky
ceiling. Every sector height stays static: nothing in this policy moves a floor or a
ceiling.

What changes is that sector lights and specials become runtime state, four light
thinkers join the thinker order, the player's sector is tested every tic, and sky
ceilings are admitted with their two game-code effects (M3h7 note
[§4.2](M3H7_REFERENCE_INVESTIGATION.md#42-stage-a-m3h7-sky-and-static-specials)).

| Admitted at load | Value | Source behavior |
| --- | --- | --- |
| Light sector specials | 1 (blinking), 2 and 3 (fast and slow strobe), 8 (glow), 12 and 13 (synchronized slow and fast strobe), 17 (fire flicker) | `P_SpawnSpecials` spawns `T_LightFlash`, `T_StrobeFlash`, `T_Glow` or `T_FireFlicker` and clears the special; the flash and unsynchronized-strobe spawn draws come in sector order after every THING draw, and the flash and flicker draw again while running |
| Damage sector specials | 4 (fast strobe plus 20%), 5 (10%), 7 (5%), 16 (20%) | `P_PlayerInSpecialSector` hurts a live player on the 32-tic gate `!(leveltime & 31)`; special 4 also spawns its strobe and keeps its special |
| Secret sector special | 9 | counted into `level.total_secrets` at load; entered once, it adds one to the player's `secret_count` and clears the sector special |
| Line special 48 (scrolling wall) | one-sided lines only, at most 64 (`MAXLINEANIMS`) | presentation only: the offset is derived from the tic and never stored |
| `F_SKY1` ceilings | a ceiling whose flat name equals `F_SKY1` ignoring ASCII case, up to the first NUL | the fireball removal of `P_XYMovement` and the two sky returns of `PTR_ShootTraverse` below |
| Sector light levels | within `i16::MIN + 64 ..= i16::MAX - 64` | Rust-only: keeps the light arithmetic inside `short lightlevel`; otherwise `MapScaleError::Light` |
| Tags | kept | no admitted special reads them |

Behavior during play:

- **Thinker order.** Light thinkers live in the same processing order as actors
  (`P_RunThinkers`), in their own sector-thinker slot class whose capacity is the number
  of light sectors, fixed at load; the visit budget is the 320 actor slots plus that
  number. `spawn_level_specials` appends them after every map actor and the player
  body, in sector order, so the order is the load actors, then the lights, then every
  later actor. Light changes emit no event; they are visible in the runtime sector
  light of DSIM34.
- **Player sector.** `P_PlayerInSpecialSector` runs in the player phase after
  `P_CalcHeight` and before the weapon change, for a live player whose center lies in a
  sector with a nonzero runtime special. It does nothing unless the player's `z` equals
  that **sector's** floor (not `floorz`), so a player on a ledge overhanging nukage is
  not hurt. `leveltime` is the completed-tic count before the tic.
- **Environmental damage and death** go through the shared damage path with no
  inflictor and no source: no thrust, armor absorbs as usual, the pain draw and the
  death draw are made, and an environmental death counts as one of the player's own
  frags, as `P_KillMobj` does. Only the player reads damage floors, so the source's
  `players[0]` kill credit for a null source is unreachable.
- **Radiation suit.** No admitted roster carries it, so the `P_Random() < 5` of specials
  4 and 16 is short-circuited away as in the source; the branch that would draw it is
  an explicit refusal, never a silent no-op.
- **Sky removal.** A fireball whose move is refused, and whose refused check's
  `ceilingline` (the last visited two-sided line that strictly lowered the ceiling,
  starting from the destination sector's ceiling) has a sky **back** sector, is removed
  without exploding: no explosion draw, no radius or contact damage. A fireball refused
  by a thing, refused at its spawn half-step, or reaching a sky ceiling vertically still
  explodes.
- **Sky shot.** A hitscan reaching a line whose **front** sector has a sky ceiling ends
  without a puff (and without the puff's two draws) when its height is above that
  ceiling or the back sector is sky too. A shot exactly at the front ceiling is not sky
  ended.
- **Load order.** `spawn_level_specials` is refused with the state unchanged under any
  other policy, before the player body exists, a second time or after a tic;
  `advance` and `step` are refused until it has run, and `spawn_map_things` and
  `spawn_player_body` are refused after it, so no light thinker can precede a map actor
  or the body.

Still rejected at load with `PursuitError::Specials` (or `MapScaleError::Light` for a
light outside the range): every line special other than 48, including every tag-using
one; a two-sided 48 (it would enter `spechit`, which belongs to the movers); a 65th
scroller; sector specials 6, 10, 11, 14, 15, 18 and every other unknown value. `BT_USE`
stays refused with `SimError::UnsupportedButtons` and the state unchanged. Sky floors
have no game-code effect and are not tested. A map that fails a load check is rejected
by the constructor, never during play; the inherited rejections of the real-map policy
still apply. Moving sectors, doors, lifts, floors, switches, the level exit, crushers,
teleporters, light-switching lines, the view-height part of `P_CalcHeight`
(`viewheight`, `deltaviewheight`, `viewz` and its bobbing; the explicit `CalcHeight` step
computes only the momentum bob `player->bob`) and every other monster, weapon or item stay
excluded under this policy; the
[movers policy](#original-monsters-with-doors-lifts-floors-switches-and-the-exit) admits
E1M1's doors, lifts, floors, switches and exit. E1M1 is still not admitted: it needs the
roster scale and eight decoration and barrel types. See [source investigation](M3H7_REFERENCE_INVESTIGATION.md),
[DSIM34](STATE_FORMAT.md#dsim-v34--original-monsters-with-sky-and-static-specials) and
[observed evidence](validation/M3H7.md).

## Original monsters with doors, lifts, floors, switches and the exit

`PlayerMonstersMoversV1` (wire 19, context `PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1`,
DSIM-v35) is a separate opt-in superset of `PlayerMonstersSpecialsV1`. Select it with
`Simulation::original_player_movers_map`, which takes no slot limit, spawn the THINGs and
the player body, then call `Simulation::spawn_level_specials` once before the first tic,
exactly as for the specials policy. The roster (eight monsters, sixteen pickups, eight
drops), weapons, items, fireballs, 320 actor slots, 208 live fireballs, arithmetic
certificate, slide-window rule, boundary rules, spawn rule, escape and intercept
witnesses, sky, lights, damage floors, secrets and whole-tic rollback are the specials
policy's. Older policies, wires and DSIM20–34 are unchanged: under every earlier policy
the line specials below, `BT_USE` and moving planes stay rejected and no sector height
changes after construction.

What changes is that sector floors and ceilings move, line specials become runtime state,
the player can use lines and cross them, monsters open doors, and the exit switch ends
the level (M3h8 note [§4.0](M3H8_REFERENCE_INVESTIGATION.md#40-the-admitted-domain)).

| Admitted line special | Trigger | Source behavior |
| --- | --- | --- |
| 1 (DR door) | use, player or monster, front side | `EV_VerticalDoor` on the line's back sector: opens a closed door (speed 2 units, top at the lowest surrounding ceiling − 4, waits 150 tics, closes); a player's use reverses a door that is going up or waiting into closing and a closing door back up; a monster only sends a closing door up. Kept (repeatable) |
| 63 (SR door) | use, player, front side | `EV_DoDoor(normal)` on every tagged sector without a mover; the switch sound and a 35-tic button only when a door was created. Kept |
| 109 (W1 blazing door) | player crossing | `EV_DoDoor(blazeOpen)` (speed 8, stays open). Cleared |
| 88 (WR lift) | player or monster crossing | `EV_DoPlat(downWaitUpStay)`: lowers at speed 4 to the lowest surrounding floor, waits 105 tics, rises to the height it started from. Kept |
| 36 (W1 turbo floor) | player crossing | `EV_DoFloor(turboLower)`: speed 4 to the highest surrounding floor + 8 (in one tic if that is above). Cleared |
| 38 (W1 floor to lowest) | player crossing | `EV_DoFloor(lowerFloorToLowest)`: speed 1 to the lowest surrounding floor. Cleared |
| 11 (S1 exit switch) | use, player, front side | the special is cleared, the switch sound plays, `G_ExitLevel` runs |

A W1 line is cleared even when its `EV_*` started nothing; tag 0 acts on every untagged
sector, in sector order, as in the source. A use from the back side of any special line,
and a monster's use of anything but 1 (or of an `ML_SECRET` line), does nothing but still
stops the use trace. A shot never triggers an admitted line. Two-sided lines with no
admitted special are crossed and used exactly as before.

Load rules (M3h8 note [§7](M3H8_REFERENCE_INVESTIGATION.md#7-mover-load-rules-refining-m3h7-44)),
all computed once from the static map in exact integers; a failing map is rejected by the
constructor with a field-less error and no state is built, never during play:

| Rule | Rejected with |
| --- | --- |
| Every line special is 0, a one-sided 48 or one of 1, 63, 109, 88, 36, 38, 11 (every other special, on either side, including a **two-sided 48**, decision N1 option iii) | `PursuitError::Specials` |
| Every DR line (1) has a back side (no `sides[-1]` read) | `MapScaleError::DoorLine` |
| No sector belongs to two mover families (doors: back sectors of 1 and targets of 63 and 109; lifts: targets of 88; floors: targets of 36 and 38); door kinds 1, 63 and 109 may share a sector | `MapScaleError::MoverFamily` |
| Every mover sector has a two-sided neighbor (no `MAXINT − 4` door top; a turbo destination still folds from −500, so neighbor floors all below −500 give −492, as in the source) | `MapScaleError::MoverNeighbor` |
| Every reachable floor and ceiling height (the note's §5 fixpoint over the admitted lines) stays within ±512 | `MapScaleError::MoverHeights` |
| No door's smallest possible top lies below its floor (no inverting door) | `MapScaleError::InvertingDoor` |
| No door sector has a two-sided line with the sector on both sides | `MapScaleError::SelfReferencingDoor` |
| At most 30 lift sectors (`MAXPLATS`) and 16 SR switch lines (`MAXBUTTONS`) | `MapScaleError::MoverCount` |
| At most 64 one-sided scrollers (`MAXLINEANIMS`, the Stage A rule) | `PursuitError::Specials` |
| At most 8 distinct two-sided special lines in any 2 × 2 window of BLOCKMAP cells, so no position check can append a ninth `spechit` entry (`MAXSPECIALCROSS`) | `MapScaleError::SpecialWindow` |
| No mover sector has an `F_SKY1` ceiling (decision N4) | `MapScaleError::SkyMover` |
| Every 11 and 63 line's front side carries one of the 38 episode-1 `alphSwitchList` names as its upper, lower or middle texture, ignoring ASCII case (decision N9a) | `MapScaleError::SwitchTexture` |
| Every mover sector has a defined `blockbox`: the `P_GroupLines` box arithmetic stays inside `fixed_t`, so every `P_ChangeSector` of a plane step walks defined BLOCKMAP cells (rule 10, added by the independent review; defensive, since the real-map arithmetic certificate already refuses every constructed counterexample) | `MapScaleError::MoverBlockBox` |

No admitted line is a crusher: under this policy crushers are deferred, not emulated. No
later policy admits one either (none of wires 20 to 25 adds a crusher line), and ceilings
and crushers are original behaviour with no milestone named yet
([checklist](ROADMAP.md#simulation-systems)). The Stage A sector
special, light and inherited real-map rejections still apply.

Behavior during play:

- **Movers.** Doors, lifts and floors are sector thinkers in the shared thinker order. A
  mover joins at the tail of the order when it is created (in the players phase for a
  use, so a door opened by a use steps in the same tic; during the thinker pass for a
  crossing or a monster's use) and takes the lowest free sector-thinker slot. A finished
  mover keeps its slot and its place until the next pass reaches it, while its sector
  can already receive a replacement, so a sector can step twice in one tic. No mover,
  trigger or plane path draws `P_Random`.
- **Moving planes.** Each step re-clips every BLOCKMAP-linked thing in the sector's
  block box (`P_ChangeSector`). A thing that still fits follows its floor or is pushed
  under the ceiling; a shootable thing that does not fit refuses the step, which is
  undone (a closing door reopens, a rising lift goes back down; a refused last step
  stops the plane short of its destination and the mover ends there);
  a corpse that does not fit becomes giblets (state `S_GIBS`, height and radius 0,
  `MF_SOLID` cleared, final) and the step stands; a dropped item that does not fit is
  removed; other non-shootable things stay as they are. A re-clipped player touches
  overlapping pickups and collects them inside the clip. `crush` is false for every
  admitted mover, so no crush damage is dealt.
- **Crossing and `spechit`.** The two-sided special lines a position check visits form
  the source's `spechit`; after an accepted move the crossed ones are triggered in the
  source's reverse order. A monster's crossing acts only on 88; a fireball's crossings
  do nothing.
- **Monster door use.** A monster's failed move with special lines in `spechit` resets
  its direction (`DI_NODIR`) and uses each line in reverse; the move counts as
  successful if a use did, which is how monsters open DR doors.
- **Use.** `BT_USE` is admitted. `use_down` starts true (`G_PlayerReborn`), so a use held
  from the first tic does nothing until released; a use runs one 64-unit trace per press
  after the weapon change; a closed non-special line plays `sfx_noway` and stops it.
- **Switches and buttons.** The switch sound plays at the first button slot's sector (a
  global sound when that slot is free); an
  SR switch holds one of the 16 button slots for 35 tics, after which a restore sound
  plays in the Specials phase. The slots are DSIM35 state; switch textures are not
  simulated (the load rule above makes every admitted switch take the vanilla
  texture-found path).
- **Exit.** The tic in which the exit switch is used completes (thinkers, specials,
  `leveltime`), `level.exit` is set, and every later `advance` or `step` returns
  `SimError::LevelExited` with the state unchanged. The intermission and the next map are
  not part of this policy, nor of any later one so far; they are original behaviour, and
  the [checklist](ROADMAP.md#intermission-and-finale) says where they stand.
- **Reborn.** Under this policy a dead player may hold `BT_USE` alone (the only dead
  command besides the empty one). That tic completes, the player state becomes
  `PST_REBORN` (DSIM35 `original_player.0.state` 2), and every later `advance` or `step`
  returns `SimError::PlayerReborn` with the state unchanged (decision N3). The level
  restart is not part of this policy, nor of any later one so far; it is original
  behaviour, and the [checklist](ROADMAP.md#playing-a-single-player-game) says where the
  reborn reload stands. Both refusals are checked before the pause test.
- **Vertical invariant V′.** The M3h4 vertical invariant is replaced, under this policy
  only, by V′ (note [§4.1](M3H8_REFERENCE_INVESTIGATION.md#41-b2-the-vertical-invariant-v)):
  besides a thing between its floor and ceiling, it admits a non-shootable thing stuck
  under a closed door on its floor, a thing sunk by at most one step `Δ` below its floor
  (landed by its next move), a shootable thing stuck on its floor from a spawn
  placement, and a corpse or giblets down to the lowest reachable height minus 56. A
  state outside V′ refuses the tic with `MapScaleError::Vertical`, state kept.
- **Item witness.** Items require `z == floorz` with their own stored heights instead of
  equality with the center sector's heights (note
  [§4.2](M3H8_REFERENCE_INVESTIGATION.md#42-b3-the-relaxed-item-witness)), since a clip
  may lift an item onto a step its box overlaps.
- **Staging.** Sector heights, runtime line specials, the button slots, `level.exit` and
  the use latch are staged with the checked tic: a refused tic leaves them, the thinker
  store and order, the RNG and the events byte-identical.

| Limit (movers policy) | Value |
| --- | --- |
| Admitted line specials | 1, 63, 109, 88, 36, 38, 11, plus one-sided 48 (at most 64) |
| Sector-thinker slots | light sectors + 2 × mover sectors, fixed at load; visits per pass 320 + that number |
| Lift sectors / SR switch lines | 30 (`MAXPLATS`) / 16 (`MAXBUTTONS`) |
| `spechit` per position check | 8 (`MAXSPECIALCROSS`), guaranteed by the window rule |
| Reachable heights | within ±512 |
| Use traces | one per press, 64 units, under the 128-intercept witness |

The public `doom_sim::mover_bounds(&Map)` runs these line and sector load rules (not the
geometry rules) and returns `MoverBounds { reach, delta, movers }`: the smallest and
largest reachable height, the `Δ` of V′ and the number of sectors an admitted line can
move; `doom_sim::special_window_lines(&Map)` returns the largest window count of the
`spechit` rule. Both are admission queries; neither builds a simulation.

The [quirk register](QUIRKS.md) lists quirks of the original found behind the fidelity gaps and refusals below; it is not claimed to be complete.

Fidelity gaps and refusals:

- **`SpecialCross` witness.** A position check that would append a ninth `spechit` entry
  refuses the tic with `MapScaleError::SpecialCross`, state kept. The source writes past
  `spechit[8]` (undefined; demo-compatible ports emulate the DOS layout). The window rule
  makes it unreachable on admitted maps; it is reached only by a Rust-only test lane with
  the rule bypassed. Like the corner jump and the 129th intercept, it is a known fidelity
  gap to revisit for demo compatibility (M4).
- **Undefined behavior refused rather than emulated.** The ninth `spechit` write, the
  `sides[-1]` read of a one-sided DR line, the door cast of a non-door `specialdata`
  (shared families), the 31st lift and the 17th button (`I_Error` in the source) and
  inverting doors are rejected at load, never emulated.

Known limitation (Rust-only API, kept by owner decision so the policy stays frozen):
`set_vertical_momentum` accepts nonzero vertical momentum on an item under this policy.
The source never gives an item vertical momentum; the next tic is refused
(`PlayerBodyError::Invariant`, state preserved) and the capture fails `decode_movers`.
The [E1M1 policy](#original-monsters-with-two-sided-scrolling-walls-and-static-decorations)
refuses the call itself.

Still excluded as of this policy ([what came later](#deferred-systems)): every other line special (locked doors and keys, the other door, lift,
floor, stair, donut and ceiling types, crushers, teleporters, `raiseToTexture`,
perpetual and stopping lifts, light-switching and gun lines, the other exits),
two-sided scrolling walls, sector specials 10, 11 and 14, sky ceilings over movers, the
intermission, the next map and the level restart, the view-height part of
`P_CalcHeight` and every other monster, weapon or item. E1M1 is still not admitted: it
needs the roster scale (29 monsters and 53 pickups on UV against 8 and 16), eight
decoration and barrel types and, under decision N1, its eight two-sided scrollers (note
[§13](M3H8_REFERENCE_INVESTIGATION.md#13-e1m1-readiness-under-the-movers-policy)); the
[E1M1 policy](#original-monsters-with-two-sided-scrolling-walls-and-static-decorations)
admits the scrollers and seven of those decoration types, and E1M1 still does not load
under it; the [barrels policy](#original-monsters-with-exploding-barrels-pickup-scale-and-every-thing-angle) completes its load admission. See
[source investigation](M3H8_REFERENCE_INVESTIGATION.md),
[DSIM35](STATE_FORMAT.md#dsim-v35--original-doors-lifts-floors-switches-and-the-exit) and
[observed evidence](validation/M3H8.md).

## Original monsters with two-sided scrolling walls and static decorations

`PlayerMonstersE1M1V1` (wire 20, context `PLAYER-MONSTERS-E1M1-1-MEDIUM-v1`, DSIM-v36) is
a separate opt-in superset of `PlayerMonstersMoversV1`. Select it with
`Simulation::original_player_e1m1_map`, which takes no slot limit and no skill, spawn the
THINGs and the player body, then call `Simulation::spawn_level_specials` once before the
first tic, exactly as for the movers policy. Everything the movers policy admits, with its
roster (eight monsters, sixteen pickups, eight drops), 320 actor slots, 208 live fireballs,
arithmetic certificate, slide-window and boundary rules, spawn rule, load rules, V′, the
relaxed item witness, the exit and reborn refusals and whole-tic rollback, applies
unchanged. Older policies, wires and DSIM20–35 are unchanged: under every earlier policy a
two-sided 48 stays `PursuitError::Specials` and every decoration type stays
`MapSpawnError::UnsupportedType`.

The name is chosen for the target map, not as a claim: **E1M1 itself still does not load
under this policy** (see the end of this section). What changes is that two more parts of
E1M1 are admitted, faithfully and with no new state (M3h9 note
[§5.1](M3H9_REFERENCE_INVESTIGATION.md#51-the-domain), decisions E1 and E2):

**Two-sided scrolling walls.** Line special 48 is admitted on two-sided lines, blocking or
not. Its texture offset is the tic count times `FRACUNIT`, derived and never stored, and
it has exactly the three game-code effects of the source (note
[§2.4](M3H9_REFERENCE_INVESTIGATION.md#24-p_spawnspecials-and-p_updatespecials-p_specc)):

- a non-blocking 48 enters `spechit` when a position check's box crosses it, and so
  counts toward the eight-entry `spechit` window;
- a monster whose move fails with a 48 line in `spechit` resets its direction
  (`DI_NODIR`) and its use of the line returns false, so the move stays failed and the
  next chase picks a new direction with its usual draws;
- a player's use reaching a 48 line does nothing, plays no `sfx_noway` and stops the
  use trace, so a door behind it is not opened.

Crossing a 48 line does nothing, and shots and fireballs pass it as any two-sided line. A
48 line with `ML_BLOCKING` blocks the player and monsters like any blocking line and
never enters their `spechit`. A fireball is not blocked by it (`PIT_CheckLine` skips the
`ML_BLOCKING` test for missiles, p_map.c:214–221): the line enters the fireball's
`spechit` and counts toward its window, and a crossing calls `P_CrossSpecialLine`, which
returns at once for a fireball, so the crossing is recorded and does nothing. A fireball
whose box touches the line only on a step that does not cross it, and not on the step
that does, crosses with no crossing call at all, as in the source.

**Static decorations.** The seven decoration types E1M1 spawns are admitted, by an
explicit list (`E1M1_DECORATIONS`); every other decoration type stays rejected:

| DoomEd | `mobjinfo` | Spawn state | Radius / height | Solid |
| --- | --- | --- | --- | --- |
| 2028 | `MT_MISC31`, floor lamp | `S_COLU` | 16 / 16 | yes (`MF_SOLID`) |
| 48 | `MT_MISC48`, tall techno pillar | `S_TECHPILLAR` | 16 / 16 | yes (`MF_SOLID`) |
| 35 | `MT_MISC50`, candelabra | `S_CANDELABRA` | 16 / 16 | yes (`MF_SOLID`) |
| 24 | `MT_MISC71`, pool of blood and flesh | `S_GIBS` | 20 / 16 | no (flags 0) |
| 15 | `MT_MISC62`, dead player | `S_PLAY_DIE7` | 20 / 16 | no (flags 0) |
| 10 | `MT_MISC68`, bloody mess | `S_PLAY_XDIE9` | 20 / 16 | no (flags 0) |
| 12 | `MT_MISC69`, bloody mess | `S_PLAY_XDIE9` | 20 / 16 | no (flags 0) |

Every spawn state has `tics` −1 and no action, every spawn health is 1000, and none of the
seven is shootable, special, counted or spawned on the ceiling. A decoration is spawned by
`P_SpawnMapThing` like any THING, in lump order: one `P_Random` draw for `lastlook` and no
tics draw, health 1000, the THING's angle, `MF_AMBUSH` added to its flags when the THING
carries the ambush bit (it has no effect on a decoration but is part of the flags word), on
its floor, linked to the BLOCKMAP and its sector, and appended to the thinker order, where
it does nothing each tic. A record filtered by skill or by the not-in-single-player bit
spawns nothing and draws nothing, before its type is checked. Decorations do not change
`total_kills` or `total_items`.

During play (note [§2.6](M3H9_REFERENCE_INVESTIGATION.md#26-collision-with-decorations)
and [§2.7](M3H9_REFERENCE_INVESTIGATION.md#27-decorations-under-movers-pit_changesector-p_mapc12571314)):

- **Solid decorations** block the player and monsters with **no height test**: a 16-unit
  lamp blocks like an infinitely tall pillar, and a monster blocked by one picks a new
  direction as it would at a wall. A fireball that hits one explodes, dealing no damage
  to it.
- **Flags-0 decorations** (the corpses and gore) are passed through by everything,
  fireballs included.
- **Hitscans and autoaim** pass through every decoration; decorations still enter the
  trace's intercept list like any linked thing. Sight is unaffected, and no monster ever
  targets a decoration.
- **Under movers** a decoration rides a lift or floor on its sector. A door closing onto
  one simply stops its ceiling below the decoration's top, with no `nofit`, no damage and
  no blood; a decoration is never gibbed, removed or damaged. When its box straddles the
  door and a higher neighbouring floor, the door still closes fully and the decoration
  is left with its floor (the higher one, which it stands on) above its ceiling until
  the door reopens, as in the source; the per-tic checks admit that state for a
  decoration only (M3h9 note [§12](M3H9_REFERENCE_INVESTIGATION.md#12-follow-ups-risks-status)).

| Limit (E1M1 policy) | Value |
| --- | --- |
| Decorations per map | 32 (`MAX_DECORATIONS`; E1M1 spawns 27 on every skill) |
| Linked things | 65 (`E1M1_LINKED_THINGS`: player, 8 monsters, 16 pickups, 8 drops, 32 decorations) |
| Live actors | at most 273 of the 320 actor slots |
| Scrollers | 64 (`MAXLINEANIMS`), one-sided and two-sided together |
| `spechit` per position check | 8, guaranteed by the window rule, which counts two-sided 48 lines |

The budget formulas behind these values are in the note's
[§5.3](M3H9_REFERENCE_INVESTIGATION.md#53-budgets-as-formulas-with-the-values-of-this-policy);
no fixed-capacity store grows. The public `doom_sim::e1m1_mover_bounds(&Map)` is
`mover_bounds` under this policy's load rules (a two-sided 48 admitted); it builds no
simulation.

Load rules: every movers load rule, with the line-special rule relaxed for 48 on a
two-sided line. The scroller count (at most 64) and the `spechit` window (at most 8
two-sided special lines in any 2 × 2 BLOCKMAP window, `MapScaleError::SpecialWindow`)
are unchanged and count the newly admitted lines. On top of them, at load, with the state
unchanged and nothing spawned from a failing batch:

| Rejected | With |
| --- | --- |
| A 33rd decoration | `MapSpawnError::DecorationLimit` |
| Barrels (2035), hanging corpses and every other THING type outside the admitted monsters, pickups and the seven decorations | `MapSpawnError::UnsupportedType` |
| A ninth monster or a seventeenth pickup | `MapSpawnError::WorkLimit` |
| A THING, decoration or not, whose closed box touches a wall (the inherited diagonal-policy spawn rule; `P_SpawnMapThing` checks no position) | `MonsterError::Arithmetic` |

The setup options are fixed at medium skill, single player, as for every player policy;
the constructor takes none, so other skills, netgame and Nightmare cannot be selected.
The DSIM36 reader `decode_e1m1` rejects a capture whose `actors.options` skill byte is not
medium. Known limitation: every reader before DSIM36 keeps its historical contract and
leaves that skill byte unchecked, so a DSIM20–35 capture forged with another skill
decodes although no player policy can produce one; the mode and `nomonsters` bytes are
unchecked by every reader ([DSIM v36](STATE_FORMAT.md#dsim-v36--original-two-sided-scrollers-and-static-decorations)).

Rust-only API calls that the source never makes on a decoration are refused with the
state unchanged:

| Call on a decoration (wire 20) | Result |
| --- | --- |
| A nonzero push (`set_momentum`) or nonzero vertical momentum (`set_vertical_momentum`), within the shared 64-unit input bound | `PlayerBodyError::Unsupported` (past the bound: that bound's error, checked first) |
| A target (`set_actor_target`), a removal (`mark_remove`), a state entry (`enter_actor_state`, and `enter_original_state` through it) | `PlayerBodyError::Unsupported` |
| A direct spawn of one of the seven types (`spawn_type`) | `MapSpawnError::DirectSpawn`, as for a pickup |
| A death request (`request_actor_death`) | `MonsterError::Profile`, as for every actor under the player policies |
| Damage with an inflictor (`damage_actor`) | `PlayerBodyError::Unsupported`, the physical-player rule for any target but the player |
| Damage with no inflictor and no source | accepted and ignored, as `P_DamageMobj` returns for a thing without `MF_SHOOTABLE` (p_inter.c:787–788): no gameplay value changes; only the request and its `Ignored` record are written |
| Making it a monster's target | `PerceptionError::Roster` |

Under this policy nonzero vertical momentum on an item is refused the same way. The
older policies are unchanged: under the movers policy (wire 19, observed in a committed
M3h8 room) `set_vertical_momentum` still accepts nonzero momentum on an item, after
which the next tic is refused (`PlayerBodyError::Invariant`) and the capture fails
`decode_movers`. Known limitation of the movers policy, kept by owner decision so that
the older policies keep their exact behavior (M3h9 [evidence record](validation/M3H9.md#independent-review)).
Damage with no inflictor and no source stays accepted and ignored, as in the source, also
by owner decision.

Fidelity gaps: none new. The `SpecialCross`, escape, intercept and vertical witnesses and
the exit and reborn refusals are the movers policy's, unchanged; the window rule keeps
`SpecialCross` unreachable on admitted maps, two-sided 48 lines included. An item or a
corpse whose floor would end above its ceiling (a door closing on one that straddles a
higher floor) keeps the movers policy's refusal of the next tic
(`MapSpawnError::Ownership`, state preserved); that case has no C evidence yet.

**E1M1 does not load yet.** On skill 3 ("Hurt Me Plenty", the only skill the player
policies accept) E1M1 spawns 6 exploding barrels, which stay `UnsupportedType`, and 52
pickups against the bound of 16, and 45 of its 91 spawned THINGs have an angle octant
(`angle/45`) of 4..7, which the inherited spawn-angle rule refuses
(`MapSpawnError::Angle`, the "Selected THING angles" row of the
[expanded-map Demon profile](#expanded-map-demon-profile); the
[census](validation/M3H9_IWAD_COUNTS.md)). The note's
[§8](M3H9_REFERENCE_INVESTIGATION.md#8-size-estimate-and-split) splits the remaining work:
M3h9b adds barrels and the pickup scale, and decision
[E9](M3H9_REFERENCE_INVESTIGATION.md#6-new-owner-decisions-e1e8) recommends admitting
every THING angle there as a declared wrap quirk; M3h9c adds a local-only harness that
runs the real E1M1 against the original C without committing any IWAD content
([§12](M3H9_REFERENCE_INVESTIGATION.md#12-follow-ups-risks-status)). The M3h9b census
measured the spawn-box rule: 16 E1M1 THINGs fail it at medium, and the
[barrels policy](#original-monsters-with-exploding-barrels-pickup-scale-and-every-thing-angle) replaces it under wire 21 with the rule S′ that they pass. See
[source investigation](M3H9_REFERENCE_INVESTIGATION.md),
[DSIM36](STATE_FORMAT.md#dsim-v36--original-two-sided-scrollers-and-static-decorations)
and [observed evidence](validation/M3H9.md).

## Original monsters with exploding barrels, pickup scale and every thing angle

`PlayerMonstersBarrelsV1` (wire 21, context `PLAYER-MONSTERS-BARRELS-1-MEDIUM-v1`, DSIM-v37)
is a separate optional superset of `PlayerMonstersE1M1V1`. Select it with
`Simulation::original_player_barrels_map`, which takes no slot limit and no skill, spawn
the THINGs and the player body, then call `Simulation::spawn_level_specials` once before
the first tic, exactly as for the E1M1 policy. Everything the E1M1 policy admits (the
movers world, two-sided scrollers, the seven decoration types, the arithmetic certificate,
the slide-window and boundary rules, V′, the relaxed item witness, the exit and reborn
refusals and whole-tic rollback) applies unchanged, with four changes that together let
E1M1 on medium skill pass load admission (M3h9b note
[§10](M3H9B_REFERENCE_INVESTIGATION.md#10-e1m1-medium-load-admission-under-the-new-policy);
the real-map run itself is the next milestone). Older policies, wires and DSIM20–36 are
unchanged: under every earlier policy the barrel stays `MapSpawnError::UnsupportedType`, a
17th pickup `WorkLimit`, an angle octant outside −4..3 `MapSpawnError::Angle`, and a THING
whose closed box touches a wall `MonsterError::Arithmetic`.

**Exploding barrels** (`MT_BARREL`, DoomEd 2035; note
[§2](M3H9B_REFERENCE_INVESTIGATION.md#2-original-c-facts)) are admitted in full, at most
16 per map (`MAX_BARRELS`; E1M1 spawns 6 on every skill). A barrel is spawned by
`P_SpawnMapThing` like any THING, in lump order, with its `info.c` record (health 20,
radius 10, height 42, mass 100, `MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD`, `MF_AMBUSH` from the
record), the `lastlook` draw and the tics draw (`1 + P_Random() % 6`), and cycles
`S_BAR1`/`S_BAR2` every six tics with no draw. It is never a monster: it is not counted,
never targeted by a monster, never drops anything and never credits a kill. During play:

- **It blocks** the player, monsters and other barrels like any solid thing, and a live
  barrel is an autoaim target; **it is damaged** by player and monster hitscans (a puff,
  not blood, since `MF_NOBLOOD`), by fireballs and by other explosions, through the one
  `P_DamageMobj` body: thrust away from the inflictor (mass 100: damage/8 units per tic),
  the pain draw that every surviving target makes even with `painchance` 0, `reaction_time`
  0 after a wound, and the retarget block, which fixes the barrel's `target` to its **first
  non-lethal wounder** with a non-NULL source (threshold 100, never lowered) for the rest of
  its life. A single bullet or pellet (5, 10 or 15) never kills a full-health barrel; a
  fist blow of 20 does.
- **It dies** at health ≤ 0 into `S_BEXP` with `tics -= P_Random() & 3` (so 2..5 tics),
  `MF_SHOOTABLE` cleared, **`MF_SOLID` kept**, `MF_CORPSE|MF_DROPOFF` set and its height
  quartered to 10.5. While dying it still blocks movers, absorbs a fireball that overlaps
  its 10.5 units with no damage (a fireball on a common floor flies over it), lets bullets
  and autoaim through, slides with corpse friction and can now be pushed over a drop.
  `A_Scream` (`sfx_barexp`, no draw) at `S_BEXP2`, **`A_Explode` at `S_BEXP4`**, 11 to 14
  tics after the killing tic (10 after `A_Scream`: `S_BEXP` lasts 2..5 tics, then 5 and 5),
  in the barrel's own thinker, and removal at `S_NULL` 20 tics after the explosion; a barrel killed by another barrel's explosion or by a fireball whose
  thinker runs before the killer's explodes one tic later (`d` = 1).
- **The explosion** is `P_RadiusAttack(barrel, barrel->target, 128)` with the source's
  arithmetic: `dist = (damage + MAXRADIUS) << FRACBITS` overflows and wraps to exactly
  128 units, so the scan is always the **3 × 3 BLOCKMAP blocks around the barrel's block**
  (a declared reproduced quirk, decision F5): a thing linked in a fourth block is never
  visited even when its Chebyshev distance minus its radius is below 128. Blocks are
  walked `y` outer, `x` inner, and each block's things newest-linked first, **re-reading
  the next link after every victim**: a monster hit in its spawn state whose pain draw
  fails enters `A_Chase` at once inside the walk, may move and relink, and so may be
  revisited or make another thing be revisited, exactly as the source's list walk does.
  Each shootable candidate (the dying barrel itself is skipped since `MF_SHOOTABLE` is
  clear) within `max(|dx|, |dy|) − radius < 128` is sight-checked from its own eye (¾
  height) to the dead barrel's **10.5-unit** window and damaged `128 − dist` with inflictor
  the barrel and source the barrel's target: the player (armor, thrust, `attacker` = the
  source or none, pain or death, the reborn tic refused as before), monsters (pain or
  death with their drops; the kill is credited to the player when the source is the
  player and to the single-player count when the source is NULL or a monster; a monster
  hurt by an explosion whose source is another monster retargets that monster) and other
  barrels (chain explosions with their own timers). There is no self-exclusion: a monster
  that wounded the barrel is hurt by its own explosion. Nothing new is spawned by an
  explosion and no new DSIM event is recorded (decision F9).
- **Under movers** a live barrel holds a closing door like a monster (`nofit`, the door
  reverses); a dying barrel that no longer fits becomes `S_GIBS` (radius and height 0,
  not solid), never explodes and is never removed, as in the source. A sliding barrel
  crossing a WR 88 line lowers the lift (`P_CrossSpecialLine` admits non-players on 88).

| Limit (barrels policy) | Value |
| --- | --- |
| Barrels per map | 16 (`MAX_BARRELS`; `MapSpawnError::BarrelLimit`) |
| Pickups per map | **64** (`world_pickups::max_pickups`; bonuses up to 6 × 64 = 384) |
| Actor slots | **384** (`actor_slots`; encoded as `store.max_slots`) |
| Live actors | at most 337 of the 384 slots, plus at most 39 spawns in one pass |
| Linked things | 129 (`BARRELS_LINKED_THINGS`: player, 8 monsters, 64 pickups, 8 drops, 32 decorations, 16 barrels) |
| Explosions per tic | at most 16; each at most 24 × (1 + 8) sight queries, its own sight control |
| Hitscan intercepts | the witness (128, `MapScaleError::Intercepts`) unchanged; E1M1's 18-cell estimate is 103 |

The budget formulas are in the note's
[§5.3](M3H9B_REFERENCE_INVESTIGATION.md#53-budgets-as-formulas-values-of-this-policy).

**Pickups up to 64.** The bound alone changes; every pickup kind E1M1 spawns was already
admitted. `PlayerPickupState` ranges, the item witness and the drops (8) follow the
policy's bound; older policies keep 16 and 96.

**Every THING angle** (decision E9, confirmed). The spawn angle is
`ANG45 * (mthing->angle / 45)` with the `short` quotient truncating toward zero and the
`int` product wrapping as the original executable does, which equals
`(q as u32).wrapping_mul(0x2000_0000)` for every `short`: 180 gives `0x80000000`, −45 gives
`0xE0000000`, 400, 32767 and −32768 give 0. A declared reproduced quirk of the original
build (the product is undefined in ISO C); the C evidence compiles the genuine statement
under `-fwrapv` and a declared unsigned rewrite in the sanitizer builds, byte-identical.

**The spawn rule S′** (decision
[F3](M3H9B_REFERENCE_INVESTIGATION.md#6-new-owner-decisions-f1f11-with-recommendations)).
`P_SpawnMapThing` checks no position, and 16 of E1M1's medium THINGs touch a one-sided
wall with their closed box (six lamps and candelabras exactly on the edge, nine bonuses
and one Imp overlapping it). Under this policy every spawn (THINGs, the player body,
dropped pickups) requires only what the M3h5 invariant needs: the centre has odd parity
against the one-sided walls and lies inside the vertex rectangle; a static (pickup or
decoration) needs nothing more; an XY mover (monster, barrel, the player) whose open box
meets a wall must have its centre at least `(31 − r)·(|cos θ| + |sin θ|)` units in front
of that wall's line, in exact integer arithmetic, so that no single `MAXMOVE` step can
carry its box through the wall and the `Escape`/`Position` witnesses stay unreachable. An
embedded barrel (radius 10) or a player embedded by more than 1 unit is therefore still
refused (`MonsterError::Arithmetic`; a player body `PlayerBodyError::Binding`), an Imp
embedded up to 9 units is admitted, and edge contact is admitted for every kind. E1M1 has
0 violations of S′ at every skill ([evidence](validation/M3H9B.md#iwad-counts)).

Load rules: every E1M1-policy rule, with the four changes above. Rejected at load with the
state unchanged and nothing spawned from a failing batch:

| Rejected | With |
| --- | --- |
| A 17th barrel | `MapSpawnError::BarrelLimit` |
| A 65th pickup, a ninth monster | `MapSpawnError::WorkLimit` |
| A 33rd decoration | `MapSpawnError::DecorationLimit` |
| Every THING type outside the monsters, pickups, decorations and the barrel (rockets and every other `A_Explode` caller among them) | `MapSpawnError::UnsupportedType` |
| A spawn failing S′ | `MonsterError::Arithmetic` |

Refused per tic with the state preserved (fidelity-gap witnesses): the inherited ones
(the 129th intercept, the sight budgets, `SpecialCross`, `Vertical`, the exit and reborn
tics) and two new witnesses. **The barrel corner jump**: a barrel (radius 10), live or
dying, thrown more than 20 units per axis can clear a wall corner in one `P_TryMove`,
because the source tests only the destination box and does not halve negative moves
(p_mobj.c:150–164, p_map.c:451–517); the Rust move does the same, and then, as for the
player's corner jump of the diagonal policy, an exact parity test after every accepted
barrel step (halved steps included) refuses the tic with `MapScaleError::Escape` when the
barrel's centre has left the room (state preserved). It
needs momentum above 20 per axis: an API push or hit, or the 4× fall-forward thrust of a
lethal hit on a barrel more than 64 units above its attacker; monsters (radius 20) and the
player (radius 16, with its own witness) cannot clear a corner this way. **A stale bomb
source**: The barrel's `target` is a
generational handle; when `A_Explode` finds it unreadable (its actor was removed), the
source would read freed memory, so the tic is refused with `PlayerBodyError::Invariant`
instead (decision F4; unreachable in the admitted domain, where monster corpses and the
player are never removed). The per-explosion sight control refuses the tic with
`PerceptionError::WorkLimit` only on a map outside the sight certificate.

Rust-only API calls on a barrel under wire 21:

| Call on a barrel | Result |
| --- | --- |
| A target (`set_actor_target`), a removal (`mark_remove`), a state entry (`enter_actor_state`, `enter_original_state`), a death request (`request_actor_death`) | refused, state unchanged (`PlayerBodyError::Unsupported`; the death request as for every actor) |
| A direct spawn of 2035 (`spawn_type`) | `MapSpawnError::DirectSpawn` |
| Damage with an inflictor (`damage_actor`) | admitted: it wounds, thrusts and may kill the barrel as a shot would |
| A barrel named as the *source* of a damage request | `PlayerBodyError::Unsupported`, state unchanged: the source only ever passes a barrel as the inflictor (p_enemy.c:1600, p_map.c:1194), never as a source |
| A push (`set_momentum`) or vertical momentum within the shared bounds | admitted: a thrust barrel is ordinary play |

Quirks reproduced, each with C evidence: the wrapped 3 × 3 window, the first-wounder bomb
source, the unconditional pain draw, the conditional fall-forward draw, dying barrels
solid and non-shootable, the 10.5-unit sight window, the `S_GIBS` crunch of a dying
barrel, the positive-only halving of `P_XYMovement` steps, the lift line crossed by a
barrel, the angle wrap. Recorded, not emulated (unreachable here): crusher blood on a
`MF_NOBLOOD` barrel (crushers rejected), the chainsaw thrust exception (saw not admitted),
the MAP30 stomp (no teleporters under this policy; Episode 1's came with
[wire 24](#original-episode-1-part-2-teleporters-the-baron-of-hell-a_bossdeath-and-hard-skill-wire-24),
and MAP30 is Doom II, which is on the [checklist](ROADMAP.md#doom-ii)). Known limitation of the C evidence: the
harness's `intercepts[]` carries an abort before a 129th entry, so the unchecked overflow
of the source is never observed in C; the Rust-only lane refuses the tic.

## Original level runs: the player start in lump order and the full player think (wire 22)

`PlayerMonstersLevelV1` (wire 22, context `PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1`, DSIM-v38) is a
separate optional superset of `PlayerMonstersBarrelsV1`. Select it with
`Simulation::original_player_level_map`, which takes no slot limit and no skill (medium only),
then call `spawn_map_things`, `initialize_player_weapons` and `spawn_level_specials` once each
before the first tic. Its admitted domain, bounds, load rules, witnesses and refusals are those
of the barrels policy, unchanged; it differs in exactly three behaviours (M3h9c note
[§6](M3H9C_REFERENCE_INVESTIGATION.md#6-new-owner-decisions-g1g13-with-recommendations), decision G4;
the owner confirmed decisions G1-G13 on 2026-11-05). Older policies,
wires and DSIM20-37 are unchanged, and `spawn_player_body` is refused under this policy.

- **The player start is a THING in lump order.** `spawn_map_things` spawns the body when it
  meets the type-1 record, with that record's position and `ANG45 * (angle/45)`, so the
  body's actor slot and its place in the thinker and link orders follow the records before
  it, and its single `lastlook` draw happens at that point of the RNG sequence
  (`P_LoadThings`, `P_SpawnMapThing`, `P_SpawnPlayer`). A level without exactly one type-1
  record is refused at load.
- **The damage flash decays.** `P_PlayerThink` decrements `damagecount` by one per live tic
  (`bonuscount` already decayed under wire 21).
- **A dead player runs `P_DeathThink`.** It updates the bob and the psprites, turns toward the
  attacker in `ANG5` steps (or exactly when within `ANG5`), decrements `damagecount` in the
  tics it faces the attacker or has none, and sets `PST_REBORN` on `BT_USE`; the reborn tic is
  still refused. A dead player accepts any admitted command, of which only `BT_USE` is read.
  A stale attacker handle is refused with `PlayerBodyError::Invariant` and the state
  preserved. The view height, its delta and the on-ground flag are not modelled (they never
  reach a captured field).

The capture is the DSIM37 layout with version 38, profile 22 and the level context; the
[STATE_FORMAT section](STATE_FORMAT.md#dsim-v38--the-whole-level-the-body-at-its-things-record-damage-count-decay-and-p_deaththink)
owns the schema and the reader's checks. Rust-only tests of the three behaviours are in
`crates/doom-sim/src/player_level_tests.rs`. The demo-format reader and writer
`doom_sim::demo` (`parse_demo`, `write_demo`, version-110 demos) and the `player_level_run`
example drive a whole level from demo ticcmds; `crates/doom-sim/tests/private_level.rs` is an
ignored test that needs `DOOM_IWAD_DIR`.

**Status of the real E1M1.** Under this policy the owner's E1M1 on medium skill loads, and
all five scripted real-E1M1 runs (idle, walking to the exit, fighting, barrels and random
commands) matched tic for tic the genuine engine built whole from the preserved sources
(see the [observations](validation/M3H9C.md) and the [roadmap entry](ROADMAP.md#m3h9c-the-real-e1m1-run-tic-for-tic)). That is evidence for those scripted inputs only: it is not a claim for
every input, every skill or every map. The comparison needs
the commercial IWAD, so it is local-only; CI uses a synthetic game WAD instead
([testing](TESTING.md#m3h9c-whole-engine-runner-and-the-real-e1m1-comparison)).

Limitations: the policy admits no new THING type, monster count, skill or special beyond the
barrels policy, so a map outside those bounds is refused at load exactly as under wire 21;
demos are limited to the admitted command domain (no turbo, pause, save or weapons 4-7); the
events groups are Rust's own records and the static map is compared separately, so the
per-tic comparison covers the rest of the capture.

## Original Episode 1, part 1: every door, floor, plat, stair, donut, light and exit special, keys, items, powers, chainsaw, rocket launcher and Spectre (wire 23)

**Status: merged (M4a, PR #57, 2026-11-10; see [ROADMAP](ROADMAP.md#m4a-episode-1-part-1-proven-against-the-genuine-engine)).** The command-domain
lift, the version-110 demo header for maps 1-9 and the real-map Rust runs are done. Against the genuine
engine, Rust matches tic for tic on the 25 authored scripts of E1M2, E1M3, E1M4, E1M6 and E1M7 and on the
five E1M1 scripts re-run under wire 23, with no refuse-the-tic witness firing (see
[the evidence record](validation/M4A.md#real-episode-1-maps-genuine-engine-runs-local-only)). That is evidence for those scripted
inputs only. None of DOOM.WAD's original demos can run under wire 23: all are skill 3, which is M4b, and
DEMO1 is E1M5, whose teleporters are also M4b; none was attempted.

`OriginalExecutionProfile::PlayerMonstersEpisode1V1` (wire 23, DSIM39, context
`PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1`) is a separate optional superset of
`PlayerMonstersLevelV1` (wire 22). Select it with `Simulation::original_player_episode1_map`
(`crates/doom-sim/src/player_monsters.rs`), which takes no slot limit and no skill (medium
only), then call `spawn_map_things`, `initialize_player_weapons` and `spawn_level_specials`
once each before the first tic, exactly as for wire 22. The purpose is part 1 of Episode 1 of
the original game: the mechanisms that E1M2, E1M3, E1M4, E1M6 and E1M7 need, proven against
the genuine linuxdoom-1.10 engine. The owner confirmed decisions H1-H12 on 2026-11-05 (note
[§10](M4A_REFERENCE_INVESTIGATION.md#10-new-owner-decisions-h1h12-with-recommendations)), and
the Phase 2 rulings and plan decisions D1-D10 are in
[§14](M4A_REFERENCE_INVESTIGATION.md#14-corrections-and-findings-from-phase-2). Wires 1-22,
their goldens and DSIM20-38 are unchanged: every older policy still refuses each thing,
special and bound below, and the wire-22 runs under the new tree match byte for byte
(`check_player_level.py`, [testing](TESTING.md)). The canonical layout is the
[STATE_FORMAT DSIM39 section](STATE_FORMAT.md#dsim-v39--episode-1-part-1-every-special-keys-items-powers-and-the-rocket-launcher).
Metadata recognition, executable admission and compatibility are distinct claims: this
section describes executable admission, with source equivalence shown only for the scripted
inputs of the evidence record.

### Admitted line specials

Wire 22's eight specials (1, 11, 36, 38, 48, 63, 88, 109, with the behaviour of the
[wire-19 table](#original-monsters-with-doors-lifts-floors-switches-and-the-exit)) are kept.
Wire 23 adds these 29 (`mover_rules::validate_movers_with`, `use_lines.rs`, `line_triggers.rs`,
`movers.rs`); W1 lines are cleared after the call whatever it started, S1 switches only when
the source's call returned non-zero.

| Special | Trigger | Source behavior |
| --- | --- | --- |
| 26, 27, 28 | use (DR), player | locked door: blue, yellow, red card or skull (`EV_VerticalDoor`, p_doors.c:354-499); without the key message 27/28/29 (`PD_BLUEK`, `PD_YELLOWK`, `PD_REDK`, a state field of DSIM39) and `sfx_oof`, no mover |
| 31 | use (D1), player | open door (stays), special cleared only when a door thinker is created |
| 32, 33, 34 | use (D1), player or monster | locked open door (32 blue, 33 red, 34 yellow); a monster's use returns with no change |
| 2 | W1 | `EV_DoDoor(open)` |
| 16 | W1 | `EV_DoDoor(close30ThenOpen)` |
| 76 | WR | `EV_DoDoor(close30ThenOpen)` |
| 86 | WR | `EV_DoDoor(open)` |
| 90 | WR | `EV_DoDoor(normal)` |
| 103 | S1 | `EV_DoDoor(open)` |
| 5 | W1 | `EV_DoFloor(raiseFloor)` (to the lowest surrounding ceiling, clamped to the sector's own) |
| 91 | WR | `EV_DoFloor(raiseFloor)` |
| 18 | S1 | `EV_DoFloor(raiseFloorToNearest)`, with `P_FindNextHighestFloor`'s 20-neighbour cut-off reproduced |
| 23 | S1 | `EV_DoFloor(lowerFloorToLowest)` |
| 82 | WR | `EV_DoFloor(lowerFloorToLowest)` |
| 70 | SR | `EV_DoFloor(turboLower)` |
| 98 | WR | `EV_DoFloor(turboLower)` |
| 20 | S1 | `EV_DoPlat(raiseToNearestAndChange)`: copies the trigger line's front flat and clears the sector special |
| 22 | W1 | `EV_DoPlat(raiseToNearestAndChange)` |
| 62 | SR | `EV_DoPlat(downWaitUpStay)` (a reusable lift switch) |
| 7 | S1 | `EV_BuildStairs(build8)`: stair sectors, speed 16,384 raw, step 8 |
| 8 | W1 | `EV_BuildStairs(build8)` |
| 9 | S1 | `EV_DoDonut`: ring then hole, the flat and special applied at the end |
| 35 | W1 | `EV_LightTurnOn(35)` on every tagged sector (no thinker; a glow thinker in the sector rewrites the level from the next tic) |
| 46 | shot (GR) | `P_ShootSpecialLine`: `EV_DoDoor(open)` and a switch-texture button restart on every hit, by the player or a monster hitscan, the pistol and the chainsaw included |
| 51 | use (S1) | the secret exit: special cleared, switch sound, `G_SecretExitLevel` (`level.secret_exit` set) |

Unknown specials are still refused at load (`PursuitError::Specials`), among them 97 (the
teleporter, M4b), 3, 4, 10, 24 and 47. A monster crossing or using any of the new lines
does nothing (the source's own filter in `P_CrossSpecialLine` and `P_UseSpecialLine`). The
switch-texture rule is generalised (decision D3): a switch line whose front texture matches
no switch is a silent no-op in the source (no sound, no button, S1 cleared) and is admitted;
a match only in another episode's switch list is still refused
(`MapScaleError::SwitchTexture`).

### Admitted sector specials, things and weapons

| Set | Admitted under wire 23 (beyond wire 22) |
| --- | --- |
| Sector specials | 11 (`P_PlayerInSpecialSector`: 20 damage on the damage gate that ignores the suit, the one-tic cap that leaves the player at 1 health for any source in a special-11 sector, the exit at health 10 or less). Wire 22's 1, 2, 3, 4, 5, 7, 8, 9, 12, 13, 16 and 17 stay; a radiation suit now cancels 5, 7, 4 and 16 as in the source (4 and 16 draw `P_Random() < 5` every tic with the suit) |
| Keys | 5 blue card, 6 yellow card, 13 red card (`P_GiveCard`; a second card of a colour adds only its `BONUSADD`) |
| Items and powers | 2013 soulsphere (200 cap), 2024 partial invisibility (powers[2] = 2100, `MF_SHADOW` set on the body and cleared only when it expires), 2025 radiation suit (2100), 2026 computer map (refused while held), 2045 light amplification visor (4200), 8 backpack (maxima double once, then one clip of each ammunition in order) |
| Ammunition | 2010 rocket, 2046 box of rockets; `P_GiveAmmo` for all four types with `maxammo` |
| Weapons | 2005 chainsaw and 2003 rocket launcher; weapon selections 0-7 under `BT_CHANGE` (unowned selections ignored, 0 chooses the chainsaw once owned, 7 is the chainsaw, 5 and 6 are never owned) |
| Statics | 34 candle, 46 tall red firestick (four-state loop); 14 (the teleport landing) is **not** admitted |
| Monster | 58 Spectre: a Demon with `MF_SHADOW` and `MF_COUNTKILL`, with the Demon's exact state and action roles; the shadow aim fuzz applies when a monster attacks an invisible player |
| Rocket | `MT_ROCKET` with `P_SpawnPlayerMissile` (three autoaim tries), `A_Explode` through the generalised `P_RadiusAttack` source and the sky removal of a missile; the chainsaw `A_Saw` lunge rewrite (p_user.c:249-255) and the no-thrust rule that keys on the stored ready weapon |

Wire 22's things (zombieman, sergeant, chaingunner, Imp and Demon, the wire-22 pickups, the
barrel, the seven E1M1 decorations and the player start in lump order) are unchanged.
New events use DSIM39 sub-tags 11-14 and message codes 14-29.

### Bounds

Wire 22's fixed caps (8 monsters, 64 pickups, 16 barrels, 32 decorations, 384 actor slots)
are replaced by values derived once at load from the map's THINGS lump with the
single-player, medium-skill filter of `P_SpawnMapThing`
(`crates/doom-sim/src/episode1.rs`, `Episode1Bounds::from_map`; decision H4). Immutable and
never canonical state; the DSIM39 reader derives the same values from `map.things`.

| Bound | Wire 23 value |
| --- | --- |
| Monsters `K`, droppers `R`, pickups `P`, keycards `C`, barrels `B`, statics `D` | counted from THINGS (monsters 3004, 9, 65, 3001, 3002, 58; droppers 3004, 9, 65; the 25 pickup numbers; barrel 2035; statics) |
| World items | `P + R` |
| `bonuscount` | `6 * (P + R + C)` |
| Actor slots | 1,024 (`store.max_slots`, decision D1; the store-full witness replaces the live-actor proof). The v39 reader refuses any other value of the field (found by the mutation-testing step). Readers before v39 check only an upper bound, so the v38 reader accepts values below its fixed 384: a documented older-schema gap, not patched |
| Per-tic monster sight queries | `max(64, 8 * (K + 1))` |
| Queries per explosion | `max(216, (1 + K + B) * (1 + K))`; the visit-bound roster term is `max(8, K)` |
| Live missiles | no per-type cap; the full store is the witness |
| Sector heights | still within +-1,024 map units: the static heights at load (`map_extent::validate_real_map`), and every height a mover writes, through the `Arithmetic` witness below |

Heights above 512 now load under wire 23 and wire 22 still refuses them.

### Load rules

The mover rules of the wire-19 section apply as follows
(`mover_rules::validate_episode1`). **Kept:** a DR or D1 line needs a back side
(`MapScaleError::DoorLine`), `MapScaleError::MoverNeighbor`, `SelfReferencingDoor`, the
plat and reusable-switch counts (`MoverCount`), a defined `blockbox` for **every** sector
(`MoverBlockBox`), the +-1,024 sector heights, the switch-texture rule above, the light
range, and `DonutGeometry` (decision H8): every sector tagged by a special-9 line must have a
two-sided first line and an outer sector that `EV_DoDonut` finds without a null side or
sector, because the source's `!flags & ML_TWOSIDED` test never skips a one-sided line. The
boundary rules C1/C2 are **optional**: a map that fails them (E1M3's) loads with the Escape
witness inactive and the Position witness kept (decision D4). **Bypassed, with a witness in
their place** (H3, wire 23 only): one family per sector (`MoverFamily`), reachable heights
(`MoverHeights`, `InvertingDoor`, the +-512 check), the 2 x 2 special-window (`SpecialWindow`)
and the arithmetic certificate's clauses (i), (ii) and (vii). Rule 8, sky movers
(`SkyMover`), no longer applies at all. The reachable-height computation is clamped to
+-32,767 units per round so that it terminates; it now feeds only the vertical-position rule, and
the heights movers actually write are bounded by the `Arithmetic` witness.

### Runtime witnesses

Each refuses the tic and preserves the complete canonical state (the whole-tic rollback
of earlier wires); the next tic can be attempted. All but the store-full one are
`MapScaleError` variants (`crates/doom-sim/src/player_map_scale.rs`) reported through
`SimError`.

| Variant | When |
| --- | --- |
| `SecondDoor` | a D1 line (31-34) is used on a sector that already holds a live mover: the source falls through and overwrites `specialdata` |
| `MoverFamily` | at use time, a DR line (1, 26-28) used on a sector whose `specialdata` is not a door (the source casts the plat or floor to a door and reads an aliased field that differs between the 32-bit original and LP64) |
| `StairBlocked` | a stair step's first `P_ChangeSector` is blocked: the source reads the uninitialised `crush` of the `EV_BuildStairs` thinker (decision H7) |
| `PlatBlocked` | a `raiseToNearestAndChange` plat is blocked: it never initialised `low` (H7) |
| `DonutBusy` | `EV_DoDonut`'s outer sector already holds a live mover (H8) |
| `Arithmetic` | a checked `i64` recomputation of the source's `P_InterceptVector`/`P_InterceptVector2` sums, `FixedMul` narrowing, sight slope differences and trace heights would overflow or leave `fixed_t`; or a mover would write a sector height beyond +-1,024 units (a repeatable turbo floor or door can ratchet heights past any bound; C runs on until `fixed_t` overflows), or its `T_MovePlane` step or `EV_*` destination offset would overflow `fixed_t`. Replaces the certificate clauses and the +-512 checks (the same `Arithmetic` variant, now raised at the tic) |
| `SpecialCross` | a `P_CheckPosition` would store a ninth `spechit`; it replaces the 2 x 2 window load rule that E1M5 and E1M9 fail (neither map is admitted) |
| `StoreError::Capacity` | a spawn finds the 1,024-slot store full; nothing is spawned |
| `Escape`, `Intercepts`, `Position`, `Vertical` | the wire-22 witnesses, unchanged |

The blocked stair and the blocked raise-and-change plat are cases where the genuine C runner
reads zero-filled zone memory (the declared driver behaviour) and runs on; Rust refuses the
tic instead, because vanilla's behaviour there depends on zone contents no model can know.
Observed at tic 91 (PLATS2) and tic 153 (STAIRS2) in the
[evidence record](validation/M4A.md); the earlier tics match. The three genuine C builds
(strict `-fwrapv`, overflow-checked, address-checked) remain the independent check that no
overflow occurs on any matched run.

### Explicitly not admitted

The [quirk register](QUIRKS.md) lists quirks of the original found behind this list; it is not claimed to be complete.

- **Teleportation:** special 97 and thing 14 (the landing) - M4b with E1M5, E1M8 and E1M9.
- **The Baron and `A_BossDeath`** (E1M8) and every thing outside the lists above; an
  unsupported thing is `MapSpawnError::UnsupportedType`, an unsupported action is an explicit
  rejection, never a silent no-op.
- **Skills other than medium**, multiplayer, and the intermission or any level transition:
  a level exit ends the run (`LevelExited` on the next tic), and the dead player's `BT_USE`
  reborn tic is refused (`PlayerReborn`).
- **Known gap: a Demon or Spectre bite on a body `A_Saw` flagged.** When a chainsaw hit sets
  `MF_JUSTATTACKED` on the player's body (p_pspr.c:542) and a Demon or Spectre bites in the same
  tic, the genuine engine runs the bite as on any live body, but wire 23 refuses the tic with
  `PlayerBody(Invariant)`. It is an explicit refusal, never a silent difference. Found in M4b on
  E1M5 (see [the evidence record](validation/M4B.md#the-m5walk3-divergence-and-its-fix)); it is fixed
  under wire 24 only and stays unpatched in wire 23 by rule, because historical profiles keep
  their behaviour. Wire 23 is the only older wire that admits the chainsaw, so no earlier wire
  can reach it.
- **Commands:** `BT_SPECIAL` and button bit 0x40 are refused (`UnsupportedButtons`).
  `forward` and `side` are admitted in -127..127 (-128 is the demo end marker and is refused) and
  weapon indices 0-7 follow vanilla's rules; the chainsaw lunge rewrite runs with a forward value of
  100 inside the engine, as the source does. The demo reader parses the version-110 header
  (`parse_demo_episode1(bytes, map)`), so demos for maps 1-9 of episode 1 are readable; a map outside
  the five above is still refused at load.
- **The turbo message is modelled.** `G_Ticker` (g_game.c:671-679) assigns `"<name> is turbo!"` to the
  player's message when `forwardmove` exceeds 50 on every 128th gametic, during demo playback too. Under
  wire 23 this is message code 30 ([format](STATE_FORMAT.md#dsim-v39--episode-1-part-1-every-special-keys-items-powers-and-the-rocket-launcher)),
  assigned before `P_Ticker` from the command as read, so a later pickup or door message in the same
  tic replaces it. Older wires never see a forward value above 50.
- **Presentation and cheats:** the view height, colormaps, the map display and `cheats` are not
  modelled; they never reach a captured field.
- Rust-only departures: the checked-arithmetic, store-full and mover witnesses and the work
  budgets above are safety bounds, not source behaviour. A mover or missile outside them
  refuses the tic; none changes a tic that is admitted.

### Comparison contract

The per-tic comparison of [validation/M4A.md](validation/M4A.md) is the wire-22 contract
with DSIM39: the genuine engine (`tools/reference/m4a_driver.c`, schema
`DOOM-M4A-RUNNER-v1`, three variants and the opt-in 32-bit build) is projected by the
independent encoder (`tools/reference/level_snapshot.py --profile episode1`) and compared
with the Rust capture without the `map.*` and `events.*` groups
(`level_compare.py --profile episode1`). `player.0.*` holds the command after the chainsaw
rewrite (decision D5). The 30 synthetic scripts of the nine generated WADs run in CI with
no IWAD; real-map runs of E1M2-E1M4, E1M6 and E1M7 are local-only. Evidence for scripted
inputs is not a claim for every input, every skill or every map.

Limitations: no map beyond the five above is claimed; E1M6 and E1M7 are secondary targets
that are admitted only if every script authored for them matches. The sources of truth for
the commands are [TESTING](TESTING.md#m4a-synthetic-mechanism-wads-and-the-episode-1-comparison)
and for the observations [validation/M4A.md](validation/M4A.md); Rust-only tests are in
`crates/doom-sim/src/player_episode1_*_tests.rs` and `crates/doom-sim/tests/episode1_synthetic.rs`.

## Original Episode 1, part 2: teleporters, the Baron of Hell, `A_BossDeath` and hard skill (wire 24)

**Status: merged (M4b, PR #58, 2026-11-14; see the
[note](M4B_REFERENCE_INVESTIGATION.md#6-plan-size-estimate-and-cap) and [ROADMAP](ROADMAP.md)).** Wire 24 is wire 23 plus teleportation, the Baron of Hell, `A_BossDeath`
and hard skill (skills 2 and 3, decision I4). Skills 1, 4 and 5 are refused at load, as listed below.

`OriginalExecutionProfile::PlayerMonstersEpisode1V2` (wire 24, DSIM40, context
`PLAYER-MONSTERS-EPISODE1-2-v1`) is a separate optional superset of `PlayerMonstersEpisode1V1`
(wire 23), decision [I2](M4B_REFERENCE_INVESTIGATION.md#5-owner-decisions-i1i13-with-recommendations).
Select it with `Simulation::original_player_episode1_v2_map(map, map_number, skill, context)`
(`crates/doom-sim/src/player_monsters.rs`). `map_number` is the Episode 1 slot, 1-9; the
simulation holds it for the run, and DSIM40 records it as `level.map`. `skill` (medium or hard) becomes
`actors.options`. Then call `spawn_map_things`, `initialize_player_weapons` and
`spawn_level_specials` once each before the first tic, as for wire 23. The canonical layout is
the [STATE_FORMAT DSIM40 section](STATE_FORMAT.md#dsim-v40--episode-1-part-2-teleporters-the-baron-of-hell-a_bossdeath-and-hard-skill),
read by `Diagnostic::decode_episode1_v2`. Wires 1-23, their goldens and DSIM20-39 are unchanged.

**Admitted now.** Everything wire 23 admits, with wire 23's bounds, load rules, runtime witnesses
and comparison contract (the section above). Under the same inputs, a wire-24 capture is the
wire-23 capture with `profile` 24, the new context and `level.map` added. The demo reader
`parse_demo_episode1_v2(bytes, map)` reads a version-110 header with skill 2 or 3, episode 1 and
maps 1-9. The runner `player_level_run --policy episode1v2` takes the map from `--map` and the
skill from the header. Plus teleportation (plan step 4; decisions I3, I7 and I8):

- **Line special 97** (WR teleport, p_spec.c:904–907) for the player and for every non-missile
  thing that crosses it: walking monsters (`P_Move`) and anything sliding with momentum
  (monsters, corpses, barrels). `EV_Teleport` (p_telept.c:47–131) runs as in C: the back side
  does nothing; the landing is the first `MT_TELEPORTMAN` in thinker order inside the
  lowest-numbered sector with the line's tag (no tag-0 guard); `P_TeleportMove` takes the
  centre sector's floor and ceiling with no line pass; the thing gets `z = floorz`, the landing's
  angle and zero momentum; two `MT_TFOG` are spawned (old position, then 20 units along the
  landing's angle by the fine-sine table, which is not exactly 20); a player gets
  `reactiontime` 18, which skips the next 18 `P_MovePlayer` calls.
- **Inside the move.** The teleport runs in `P_TryMove`'s crossing loop. Once `P_TeleportMove` was
  reached (success or failure) the loop ends; the second half of a split move and the rest of a
  slide carry on from the landing, so a thing can teleport twice in one tic. Under wire 24 every
  crossing of `P_XYMovement` runs inside the move at its place in the loop (wire 23 keeps running
  them after the move); the events and the shared thinker order come out as C orders them.
- **The landing** (doomednum 14): spawned by `spawn_map_things` like any THING (one `lastlook`
  draw, no tics draw), in state 0 (`S_NULL`, tics −1) forever, in `spatial.link_order` but in
  no sector list and no block cell.
- **Stomping** (`PIT_StompThing`, p_map.c:81–108; decision I8). A player telefrags every
  shootable thing whose box overlaps the landing (no z test) with `P_DamageMobj(thing, player,
  player, 10000)`. A non-player's teleport fails if any shootable thing overlaps (maps 1–9),
  which changes nothing and spawns no fog. The telefrag's thrust is the wrapped 32-bit value
  of `damage*(FRACUNIT>>3)*100/mass` that the genuine strict builds compute (−397,934,592/mass;
  decision I3); no knockback when the player's ready weapon is the chainsaw. Damage above 1,000
  stays refused everywhere else and on every older wire. A second player-1 start (a voodoo
  doll) is refused at spawn, as under wire 22 and 23, so a victim is never a player.

Plus the Baron of Hell and `A_BossDeath` (plan step 5; decision I9):

- **The Baron** (doomednum 3003, `MT_BRUISER`) with its whole normal state closure
  (`S_BOSS_STND` … `S_BOSS_DIE7`; the raise states stay metadata) and the shared look, chase,
  face, pain, scream and fall actions. `A_Chase` takes its melee branch without a `movecount`
  test and without setting `MF_JUSTATTACKED`, its missile branch only at `movecount` 0 and with
  the flag (p_enemy.c:725–749). **`A_BruisAttack`** (p_enemy.c:979–996) does not face the target;
  in melee range it claws (`sfx_claw`, one draw, `(P_Random()%8+1)*10` damage with inflictor and
  source the Baron), otherwise it fires an `MT_BRUISERSHOT` through the same `P_SpawnMissile` as
  the Imp (one launcher parametrised by type; the Imp's path is unchanged). The fireball is speed
  15, damage `((P_Random()%8)+1)*8`, no splash; it never splits a step. A contact during the
  spawn half step draws the damage, runs `P_DamageMobj` and then explodes (with its draw). A
  Baron fireball explodes on another Baron without damage (the species rule); it hurts every
  other shootable thing, which may then fight back (infighting), and the shadow fuzz applies at
  an `MF_SHADOW` target in both `A_FaceTarget` and `P_SpawnMissile` (left draw first, as observed
  in the genuine engine). Death: `S_BOSS_DIE1` with its tics draw, `A_Scream`, `A_Fall`, no drop.
- **`A_BossDeath`** runs on entry into `S_BOSS_DIE7` and makes the source's checks in order: the
  map number must be 8, the player's health above 0, and no other Baron in the thinker list may
  have health above 0. Then `EV_DoFloor(lowerFloorToLowest)` on every sector tagged 666, in index
  order; a sector that already has a mover is skipped; each new floor is appended to the thinker
  order and moves in the same tic. The call names no line (the source passes a local line whose
  only field is the tag). On every other map, or when a check fails, nothing happens; it never
  exits the level. E1M8 ends through sector special 11, as wire 23 already models: the exit tic
  completes and every later tic is refused with `LevelExited`. The ±1,024 sector-height witness
  covers the boss floor (it is an ordinary floor step); it cannot fire there, because the floor
  only moves down to a current neighbour height.
- **Per-map bounds.** Wire 24 counts doomednum 3003 in `K` (placed monsters) as a monster that
  drops nothing; everything derived from `K` follows (the roster, the sight allowances, the
  transient allowance). Wire 23's bounds and the DSIM39 reader's count are unchanged.

**Witnesses of the teleport (each refuses the tic; the checked transaction keeps the whole
canonical state, as for every wire-23 mid-tic refusal):**

- the ninth `spechit` entry of any position check (`MapScaleError::SpecialCross`, inherited
  from wire 23; it covers every `P_TryMove`, `P_Move`, `P_ThingHeightClip` and missile move);
- a full actor store when the drops of a telefrag and the two fogs need their slots
  (`StoreError::Capacity`), checked after the landing search and the damage plans and before the
  teleport's first mutation. The load rule behind it: wire 24 counts the landings `L` with the
  same THING filter as the per-map bounds and refuses at load (`StoreError::Capacity`) a map whose
  **transient allowance** `1024 − (1 + K + R + P + B + D + L)` is below 2, so that one teleport's
  fogs fit when every placed thing and every drop is live (`Episode1Bounds::transient_allowance`);
  wire 23's bounds are unchanged;
- the 129th intercept of a trace (inherited);
- the landing search is bounded by construction (one pass over the actor order, then at most
  sectors × landings) and runs before any mutation; it refuses nothing;
- **the Baron fireball wall tunnel** (below), refused on the tic after the ball leaves the map.

#### The Baron fireball wall tunnel (wire 24)

A Baron fireball (`MT_BRUISERSHOT`, speed 15, radius 6; info.c:1524-1548) moves up to 15 units in one
step and is never split. `P_CheckPosition` tests only the box at the destination (p_map.c:191-195,
374-442), so a step can carry the ball wholly past a one-sided wall thinner than the step. The step bound
of the M3h5 position proof (2r - 2 = 10 units for radius 6) does not hold for this ball: 15 is more than 10,
and the earlier wording that assumed it was corrected in the
[note](M4B_REFERENCE_INVESTIGATION.md#3-c-facts-the-baron-a_bossdeath-hard-skill-and-demo-version)
(finding 29). The genuine engine keeps such a ball flying in the void.

Under wire 24 Rust admits the tunnelling tic. If the ball has left the map's vertex rectangle, the next
tic is refused with `SimError::Monster(MonsterError::Arithmetic)` by the position check on the spawned
actor (`demon_map_spawn.rs:155-157`) and the whole canonical state is kept, like every other refused tic.
While the ball stays inside the vertex rectangle, Rust applies the same destination-box checks and the same
BSP location as the genuine engine, so it matches or another witness refuses; BTUNNEL (a ball past an inner
one-sided wall) is the observed instance, and a ball that passes an inner wall may still leave the rectangle
later and be refused. This is a declared witness, not a difference from the
original: for this case Rust either matches the genuine engine or refuses the tic (owner ruling,
2026-11-13). Wire 23 cannot reach it, since every wire-23 missile meets the step bound. It is reachable on
E1M8 in principle; none of the 61 real-map runs hit it. The soak and fuzz target accept exactly this
refusal and no other `Monster(Arithmetic)`. Evidence:
[the evidence record](validation/M4B.md#the-baron-fireball-wall-tunnel).

**Refused now, each explicitly at load:**

- every skill but medium (2) and hard (3), and any other non-default option (such as
  `no_monsters`): `SimError::PlayerBody(PlayerBodyError::Unsupported)` from the constructor; the
  runner reports such a demo header before any tic. Wires 23 and below still refuse hard skill
  (`has_hard_skill()` is wire 24 only);
- a map number outside 1-9: the same error;
- line specials 39, 125 and 126 (the other teleports): `PursuitError::Specials`, as under every
  wire (decision I1);
- the DSIM40 reader refuses `actors.options[0]` other than 2 or 3 with `Identity`; DSIM36-39 admit 2
  only.

The Baron's fireball launch, its claw and `A_BossDeath`'s floor use the witnesses above (the
store-full witness before a spawn, the height witness on every floor step); `A_BossDeath`'s
thinker walk is one read per actor and runs before any mutation.

**Hard skill (plan step 6).** In single player nothing but `P_SpawnMapThing`'s option bit differs
between skills 2 and 3: bit 2 for medium, bit 4 for hard (p_mobj.c:743-751). Wire 24 therefore
counts the per-map bounds (`K`, the landings, the transient allowance) and the item and kill
totals with the bit of the run's skill (`Episode1Bounds::from_map_v2`, `landings`,
`world_pickups::counted_things_with_bit`); wires 23 and below keep bit 2. The demo reader already
read skill 3, and the runners now run a skill-3 demo.

So E1M5, E1M8 and E1M9 now load and run under wire 24 on skills 2 and 3. The evidence for skill 3 is
the SKILL3 synthetic script ([evidence record](validation/M4B.md#rust-replay-hard-skill)) and the
scripted real-map runs of plan step 7.

**A bite on a body `A_Saw` flagged (plan step 7).** `A_Saw` sets `MF_JUSTATTACKED` on the player's
body on a hit (p_pspr.c:542), and `P_PlayerThink` runs before the thinkers (p_tick.c:150-152), so a
Demon or Spectre can bite a body that carries that flag in the same tic. Nothing on the bite path
reads it. Wire 24 admits the bite on such a body (`has_bite_on_lunging_body()`), and any other
unadmitted body flag still refuses under wire 24. Wire 23 still refuses the bite with
`PlayerBody(Invariant)`: a known wire-23 gap, left unpatched by rule because historical profiles
keep their behaviour (listed under wire 23's [explicitly not admitted](#explicitly-not-admitted)).
Wires below 23 do not admit the chainsaw, so they cannot reach it. The case found a real divergence on E1M5 (skill 3 walk), reproduced by the synthetic scene
SAWBITE; see [the evidence record](validation/M4B.md#the-m5walk3-divergence-and-its-fix). The bite that
kills the body while the flag is set was reasoned from the source and has not been observed.

**Real-map evidence (plan step 7, local-only).** Rust under wire 24 matched the genuine engine tic for
tic on 61 scripted runs over E1M1 to E1M9 (skill 2 on all nine maps, skill 3 on E1M5, E1M8 and E1M9; `DEMO1` is E1M5 on skill 3), and on `DEMO1` run by linuxdoom-1.10
(a match with that engine, not with the DOS 1.9 executable that recorded the demo); other inputs are
not claimed ([evidence record](validation/M4B.md#real-maps-and-demo1)).
The observations (the 30 M4a synthetic scripts and the M4b synthetic scripts compared with the
genuine engine under wire 24, and the local real-map runs) are in
[the evidence record](validation/M4B.md); the commands are in
[TESTING](TESTING.md#m4b-synthetic-mechanism-wads). Rust-only tests are in
`crates/doom-sim/src/player_episode1_v2_tests.rs`, `crates/doom-sim/src/teleport_tests.rs` and
`crates/doom-sim/src/baron_tests.rs` and `crates/doom-sim/src/hard_skill_tests.rs`; `crates/doom-sim/tests/episode1v2_mechanism_goldens.rs` replays the admitted M4b scripts against
the goldens and fixtures in `cargo test`.

## Original Episode 1, part 3: canonical sound records (wire 25)

**Status: merged (M5c, [PR #63](https://github.com/Copgunn/DOOM/pull/63), 2026-11-19); milestone status is in [ROADMAP](ROADMAP.md).** Wire 25 is wire 24 plus
what sound needs, and **nothing else** (decisions T3 and T4, confirmed by the owner on 2026-11-18). It makes no
sound: it lets the simulation say, tic by tic, which sounds the original would start and stop and from where, so
that [the sound logic](#sound-effects-m5c) can run on them.

`OriginalExecutionProfile::PlayerMonstersEpisode1V3` (wire 25, DSIM41, context `PLAYER-MONSTERS-EPISODE1-3-v1`)
is a separate optional superset of `PlayerMonstersEpisode1V2` (wire 24). Select it with
`Simulation::original_player_episode1_v3_map(map, map_number, skill, context)` (same arguments and same setup
calls as wire 24). The canonical layout is the
[STATE_FORMAT DSIM41 section](STATE_FORMAT.md#dsim-v41--wire-25-canonical-sound-records), read by
`Diagnostic::decode_episode1_v3`. Wires 1-24, their goldens and DSIM1-40 are unchanged, and every earlier reader
refuses 41.

**What wire 25 adds to wire 24, and no more.**

| Addition | What it is |
| --- | --- |
| Canonical sound records | Every `S_StartSound` the original makes (one record per call, in call order) and every `S_StopSound` that `P_RemoveMobj` makes, in the group `events.sounds`. A start whose origin is neither NULL nor the player's own body also carries the positions of the origin and the listener at the moment of the call, which the audibility rule needs and which the wire-24 events do not hold |
| Sector sound origins | The point a door, lift or floor sound comes from is the middle of the sector's bounding box, computed once at load as `P_GroupLines` does (p_setup.c:536-557). A sector whose arithmetic would overflow refuses the map at load (`SoundError::SectorOriginOverflow`) |
| The two switch origins | Press: the origin the original passes, `buttonlist[0].soundorg` read before the new button is stored (p_switch.c:228-256). Release: the declared 64-bit reading and the slot-15 refusal ([below](#the-two-switch-sounds-decision-t12)) |
| The corrected scream | A player's hard death plays `sfx_pldeth`, as the original does outside commercial mode (p_enemy.c:1994-2007). Wires 22-24 name `sfx_pdiehi` ([known gap](#known-gap-the-players-hard-death-scream-in-the-sound-events-of-wires-22-24)) |
| The THING-loading truncation | `P_LoadThings` stops at the first of ten Doom II types outside commercial mode (p_setup.c:317-337); wire 25 does the same ([known gap](#known-gap-thing-loading-stops-at-the-first-doom-ii-type-outside-commercial-mode)) |
| New typed refusals | `SimError::Sound` with `SectorOriginOverflow`, `ButtonSlot15` and `Unplaced` (a start that should carry positions and does not, which no admitted path makes); each leaves the simulation's state as it was. The sound session's own refusals are [listed below](#what-the-sound-code-refuses) |

**Admission counts only what the original loads.** Under wire 25 the constructor's per-map bounds (`Episode1Bounds`), the
landing count, the transient-actor allowance and the world-pickup cross-check count only the THINGS before the first of
the ten Doom II types, as `P_LoadThings` does (p_setup.c:317-337; the totals `totalkills` and `totalitems` count only
spawned things, p_mobj.c:790, 792). A map with a long unloaded tail (for example a type-65 THING followed by 1,100
zombiemen) is therefore admitted under wire 25, where wire 24 refuses it with `Capacity` at load; and the DSIM41 reader
counts the same prefix, so every capture of a truncated map decodes
([layout and reader](STATE_FORMAT.md#what-the-v41-reader-checks)). An independent review found that both had counted
the whole lump; the fix and its tests are in the [evidence record](validation/M5C.md#step-13-independent-review).

**How "nothing else changes" was shown** (details in the [evidence record](validation/M5C.md#step-3a-wire-25-in-doom-sim)):
every wire-25 capture, with the sound group and the three header fields removed, equals the wire-24 capture of the
same run, and wire 25's events equal wire 24's except the scream. Rust against Rust, on the 64 synthetic items:
22,976 captures, all equal. On the 61 real Episode 1 runs (local): 159,027 captures, all equal. No real or
synthetic run reaches the scream's case, so that replacement is checked by a unit test, not by a run.

**How the records were compared with the genuine engine.** A second encoder, written independently in Python
(`tools/reference/level_snapshot_m5c.py`), builds the DSIM41 bytes from the genuine engine's dumps and from the
genuine sound stream (linuxdoom-1.10 built with its in-process sound code, one define removed), never from Rust
output. Wire 25 matches it on every capture of the 64 synthetic items (785 starts, 404 of them with positions, and
378 stops) and of the 61 real runs (26,077 starts, 20,097 with positions, 10,470 stops), with no disagreement
([synthetic](validation/M5C.md#step-3b-the-independent-dsim41-encoder-the-comparators-and-the-genuine-comparisons),
[real maps, local only](validation/M5C.md#the-61-real-runs-local-only-wire-25)).

**What this does not show.** Matching is a statement about those 64 items and 61 runs, not a proof for every
possible input. It says nothing about what the sound logic does with the records (that is the next section) and
nothing about how anything sounds.

## Software renderer (M5a)

Milestone status is in [ROADMAP](ROADMAP.md). The crate
`doom-render` draws the 3D view that linuxdoom-1.10's `R_RenderPlayerView` draws, as a 320x200 buffer of
palette indices, from a wire-24 simulation. It is a presentation layer: it reads the simulation and never
changes it or the gameplay RNG, and nothing in the simulation, DSIM or any frozen fixture changed for it.
The facts are in the [investigation note](M5A_REFERENCE_INVESTIGATION.md) (decisions R1-R16, §5, owner
rulings 2026-11-15), the observations in the [evidence record](validation/M5A.md) and the commands in
[TESTING](TESTING.md#m5a-synthetic-renderer-rooms). The render state is saved in
[`DRST`](STATE_FORMAT.md#render-state-drst-version-2-m5a), not in DSIM.

| Claim | What is and is not shown |
| --- | --- |
| Source equivalence of the frame | The Rust frame equals the genuine renderer's frame, byte for byte, on every comparable synthetic frame in CI (the committed rooms at screenblocks 9, 10 and 11) and on every frame of the 61 scripted Episode 1 runs at screenblocks 11 and 10 (local only; frame counts in the evidence record; DEMO1 as run by the original code, not the DOS 1.9 executable). Two exclusions apply (below): screen columns the genuine read tracker flags as reading undefined memory (R4) are left out of a frame's comparison, and frames with an unknown switch parity (R7) are not compared at all; those are 105 synthetic frames (M5SWTCH, 35 per configuration) and no real-map frame. It is not a claim for other maps, scripts or settings. |
| Configurations (R2) | Screenblocks 9 (synthetic rooms only), 10 and 11, **high detail**. **Low detail is refused** (the original's `R_DrawColumnLow` is broken in this source); other screenblocks are refused. |
| Not part of M5a (R1) | Status bar, HUD, automap, menus, palette flashes (the palette is chosen by the caller), window, input, sound; the border is never drawn. The status bar, HUD, palette flashes, window and input came with [M5b](#playable-build-m5b) and sound effects with [M5c](#sound-effects-m5c); the automap, the menus and the view border are original behaviour with no milestone named yet ([checklist](ROADMAP.md#vanilla-completeness-checklist)). Player colour translation (players 2-4) is refused (`PlayerTranslation`). |
| Invulnerability | Not admitted by the simulation (DoomEd 2022, `INVERSECOLORMAP`), so no frame uses fixed colour map 32; it is out of M5a, and it is original behaviour with no milestone named yet ([checklist](ROADMAP.md#simulation-systems)). Light amplification (fixed colour map 1) is covered. |

**Vanilla limits (R3).** Reproduced exactly because they are deterministic: `MAXDRAWSEGS` (256) and
`MAXVISSPRITES` (128) drop silently, as the original does. Refused with a named witness, because the
original writes past an array before any error or its result depends on memory layout:
`VisplaneOverflow` (`MAXVISPLANES` 128), `OpeningOverflow` (`MAXOPENINGS` 20,480), `SolidSegOverflow`
(`MAXSEGS` 32; the refusal can come one frame before the genuine write, as the owner accepted), the
`RANGECHECK` errors (`RangeCheck`), `FixedDiv`, `TableRead` (a table read outside the genuine contiguous
tables, R12), `StaleGlobal` (the uninitialised `best` of `R_SortVisSprites`), `UnallocatedColumn` (a column
of a zero-width texture, or a composite column of a negative-height texture, whose genuine read is outside
any allocation) and, by the orchestrator's application of R3 (2026-11-16), `MaskedCompositeColumn` (a
masked midtexture column that is a multi-patch composite: the genuine post walk reads composite texels as
post headers, so its end depends on texel values and, once it leaves the composite, on memory layout; a
declared refusal, accepted by the owner 2026-11-16, [investigation §7](M5A_REFERENCE_INVESTIGATION.md#7-open-questions-and-where-each-is-decided)
item 43) and `MaskedColumnUndefined` (a post walk that leaves its patch lump). A `pixhigh` or `pixlow` left
over from an earlier wall is reproduced, not refused: the render state carries it (item 42). A refusal is typed, repeatable and leaves the render state unchanged. Only the limit
refusals are reached by committed rooms; the others are guards, covered by unit tests where the evidence
record says so and not reached by any real or synthetic frame.

**Undefined texels (R4).** Wall columns index `& 127` whatever the texture height and composites are never
cleared, so some genuine pixels depend on memory layout. The Rust renderer renders them deterministically
and reports the affected columns (`FrameReport::flagged_columns`); the genuine runner's read tracker names
the same columns, and the comparison excludes only those columns. A frame is non-comparable as a whole only
where the tracker cannot name the columns: none occurred on any real or synthetic frame. Flagged-column
counts are in the evidence record. A flagged column is a statement about the original's undefined memory
reads, not a Rust defect.

**`ML_MAPPED` (R5).** The genuine renderer sets bit 256 of a line's flags
(r_segs.c:398); the M5a runner masks that bit in its game dump (a projection of the new runner only,
frozen runners untouched), and the renderer keeps its own mapped-line set as presentation state. The simulation
never has the bit.

**The view shadow (R6).** The simulation keeps no `viewz`, `viewheight` or `deltaviewheight`. A
renderer-owned shadow observes every committed tic from level start and re-derives them; it was validated per
tic against the genuine runner. It means `Renderer::observe` must see every tic in order, from `begin_level`
(a gap is refused, `NotObserved`).

**Reusable switches (R7).** After a re-press of a reusable (SR/GR) switch line, no canonical field records
which texture shows. The renderer reports `switch_parity_unknown` for such frames instead of guessing, and
they are not compared. The synthetic room M5SWTCH, built to reach the case, has 35 such frames per
configuration (105 over screenblocks 9, 10 and 11); no frame of the 61 real-map runs has one (counts in the
evidence record).

**Frame sequence (R8).** `fuzzpos` is never reset and the view rectangle is never cleared in the original, so
spectre pixels and uncovered pixels depend on every earlier frame. The contract: start at level load with a
zeroed buffer and `fuzzpos` 0, render after every tic that is displayed, keep one persistent buffer. The
render state (`DRST`) carries all of it, and a test restores a state at tic N-k and continues to the same
frames as an uninterrupted run (rewind).

Not claimed: any renderer or presentation behaviour beyond these, the DOS executable's picture, speed (no
timing gate), or a modern renderer. The window, status bar, HUD and input of M5b are under
[Playable build (M5b)](#playable-build-m5b) and sound effects under [Sound effects (M5c)](#sound-effects-m5c);
the automap is not built: it is original behaviour with no milestone named yet
([checklist](ROADMAP.md#in-game-screen)).

## Playable build (M5b)

Milestone status is in [ROADMAP](ROADMAP.md). M5b makes the engine playable in a limited way: the binary
`doom-play` opens a window and plays one Episode 1 map (E1M1-E1M9, skill 2 or 3) with keyboard and mouse at 35
tics a second, on the wire-24 simulation and the M5a renderer, with the status bar, HUD messages and palette
flashes. The simulation did not change: no new wire, no DSIM change, no digest change. The facts are in the
[investigation note](M5B_REFERENCE_INVESTIGATION.md) (decisions S1-S23, owner rulings 2026-11-17), the
observations in the [evidence record](validation/M5B.md) and the commands in
[TESTING](TESTING.md#m5b-playable-status-bar-hud-input-and-the-app). The saved display state is
[`DDSP`](STATE_FORMAT.md#display-state-ddsp-version-1-m5b), not part of DSIM.

**M5c has since added sound to the same app** ([below](#sound-effects-m5c)). This section describes the M5b contract
(no sound, no level-start wipe, qualified below as the declared sound-free, wipe-free contract): it is what the M5b
profile and its frozen evidence use, and it is still selectable in the libraries. The app `doom-play` now runs under
the [contract with sound](#the-display-contract-with-sound-m5c), under which the status bar face's idle glance matches
the genuine engine ([closed](#the-status-bar-faces-idle-glance-is-closed-with-sound-decision-s15-t14)).

**What is claimed, in one table.**

| Claim | What is and is not shown |
| --- | --- |
| The composed frame | The 320x200 frame (status bar, 3D view, HUD message) and the palette number of every tic are **identical to the genuine engine under the declared sound-free, wipe-free contract** (below), with two exclusions. On the 59 synthetic items at screenblocks 10 and 11, checked in CI: 42,352 frames, of which 41,982 are compared on the whole-frame digest, 300 on a masked digest (whole columns that the genuine tracker flags as read from undefined memory are zeroed on both sides, with the status bar and HUD pixels in them) and 70 are not compared (the switch state is not recorded, as in M5a). On all 61 real Episode 1 runs at screenblocks 10 and 11, locally (158,966 frames each): at 10, 158,472 on the whole digest, 494 masked, 0 not compared; at 11, 158,414 whole, 552 masked, 0 not compared. Also checked through the app on scripted sessions (next rows). It is never claimed identical to a vanilla session without the qualifier, nor to the DOS executable's picture ([synthetic counts](validation/M5B.md#step-3-the-rust-status-bar-hud-and-display), [real maps](validation/M5B.md#step-5-the-real-map-check-local-only)). |
| The commands | **Except for cheat codes**, every tic command the app makes is the genuine `G_Responder` / `G_BuildTiccmd` result for the stream of events the app posts, and is admitted by wire 24; in a recorded session each is also re-read from the demo buffer as vanilla does (the unrecorded mode executes the built command instead). Shown against the genuine input driver on 251 scripts (the 175 in the app's domain tic for tic in Rust). The exception: the app forwards cheat-code keys as ordinary keys and has no cheat matcher, so typing a code does nothing where the genuine engine would act on it, and `idclev` would also make the genuine engine reload the level and clear the held keys, so its later commands would differ (the golden `s5_cheat_idclev12` differs at tic 8, and the test pins that tic). |
| Sessions | Each session is recorded as a version-110 demo (unless the opt-in unrecorded mode is used, below). What was checked: the 4 committed scripted sessions (75 + 61 + 145 + 155 = 436 tics) in CI; and, locally, a 334-tic scripted E1M1 session and three idle first-play sessions of 350 tics each (E1M1 on X11 and on Wayland; E1M3 on skill 3 at screenblocks 11). In each, the demo replays in the genuine engine with every command, frame, palette, face, message and masked game dump equal under the contract. Those are the sessions checked, not a guarantee about every other: any saved session can be checked with the one command in [TESTING](TESTING.md#m5b-playable-status-bar-hud-input-and-the-app), which prints the first tic that differs. |
| Unrecorded mode | The opt-in `--unrecorded-full-turning` mode writes no demo and cannot be replayed ([below](#unrecorded-full-turning-mode-opt-in)). Its evidence is limited to 4 injected scripts and 2 scripted sessions on synthetic rooms (829 tics): nothing on real maps, and no person has played it. |
| What a person experiences | **Observed once, by the owner, not shown by any test.** On the evening of 2026-11-18 the owner played release builds of `3c1cc5d` (the M5c branch, with sound); what he reported is under [what the owner observed](#what-the-owner-observed-2026-11-18) (Windows 11: window, 35 frames a second, picture, captured mouse; WSLg: window, but the pointer is not captured). Not judged by anyone: macOS, long sessions, every map, every weapon and monster, vanilla bindings. Feel, latency and appearance remain for a human to judge ([evidence](validation/M5B.md#what-a-human-still-has-to-judge); [the playtest record](validation/M5C.md#the-owners-playtest-2026-11-18)). |

### The status bar face's idle glance under the M5b contract (declared, closed by M5c)

In plain words: under the M5b contract, **the status bar face's idle left, ahead or right glance differs from a
vanilla session with sound, and nothing else on screen is affected.** The reason: the glance is chosen from
`M_Random` (st_stuff.c:992, 913-919), the same random stream that every started sound and the level-start screen melt
also draw from. The M5b contract has neither sound nor the melt, so its `M_Random` index starts at 0 at a new game and
only the status bar advances it. `M_Random` never feeds game logic, so the game, the 3D view, the numbers and every
message are unaffected. This is the declared **sound-free, wipe-free contract** (decision S15). It is what every M5b
frame claim, golden and saved `DDSP` version 1 state means, and it is unchanged. **The limit is closed for the app by
M5c:** `doom-play` runs under the contract with sound, and
[the glance matches the genuine engine there](#the-status-bar-faces-idle-glance-is-closed-with-sound-decision-s15-t14).

### Status bar, HUD and palette choice

The status bar (`st_stuff.c`, `st_lib.c`), the HUD message line (`hu_stuff.c`, `hu_lib.c`), the 2D patch
drawing (`v_video.c`) and the palette choice are new modules of `doom-render`, composed with the 3D view by
`Display` in the order of `D_Display` for a single-player level at screenblocks 10 and 11. The drawing code
treats its lumps by the original's rules; the committed tests use our own generated graphics (no id art).

- **Matched, and on what evidence.** The genuine status bar and HUD are compiled whole into the M5b runner and
  compared with the Rust `Display` on every tic of 59 items (the 26 M5a rooms, the 22 M4b scripts and 11 new
  rooms) at screenblocks 10 and 11: the whole 320x200 frame digest (masked digest where the read tracker flags
  columns, as in M5a), the palette number and the palette set during the frame, the face, whether the message
  shows, the `M_Random` index and every message the HUD took. Everything compared matches, apart from the 300 masked
  and 70 not-compared frames of the table above ([evidence](validation/M5B.md#step-3-the-rust-status-bar-hud-and-display)).
  The state can be saved and restored (`DDSP`) and continued to the same frames.
- **Real maps (local-only).** The same comparison on all 61 real Episode 1 runs (E1M1-E1M9 scripts and `DEMO1`),
  every tic, at screenblocks 10 and 11: no mismatched frame, no renderer counter or flag disagreement, 794,830
  status fields per size (5 per frame) and the message taken on every tic equal, no witness or refusal. The
  committed record keeps counts, pass/fail facts and digests only ([evidence](validation/M5B.md#step-5-the-real-map-check-local-only)).
- **The 3D view is unchanged.** Message-free frames equal the M5a goldens on 11,791 frames per configuration.
- **Reachable under wire 24** (what the synthetic evidence covers): 12 of the 14 palettes (palettes 1 and 9 are
  never chosen by the source); 40 of the 42 faces (the ouch face at pain level 4 needs a health change the game
  cannot produce) plus the god face in one bespoke scenario; 29 message kinds, every reachable one (the
  "medikit you really need" message, code 6, is unreachable); every number and icon the status bar can change
  in Episode 1.
- **Unreachable under wire 24, so not exercised by any level run:** the berserk palette term, the god-mode
  face (covered by unit tests and the one bespoke cheat scenario only, whose cheats are applied through a test
  hook, not by keys), the plasma and BFG arms icons, the skull-key icons 3-5 and the minus sign. The simulation
  does not admit invulnerability or berserk (original behaviour with no milestone named yet:
  [checklist](ROADMAP.md#simulation-systems)).
- **Screenblocks 10 and 11 only**, chosen at launch and fixed for the session. At these sizes no border is
  drawn and the HUD erase does nothing, which keeps the frame contract small. Screenblocks 9 and below and low
  detail are refused (M5a decisions R1, R2, S14).
- **HUD messages** are the ones the simulation's events produce (pickups, key messages, locked doors); the
  message line cuts a string off where the original's `HUlib_drawTextLine` stops (it does not wrap). `showMessages`
  stays 1 (there is no F8 or menu). The cheat matcher is not ported, so no key can change player fields.
- **Saved display state.** `DDSP` decode checks each field's range. `restore` also runs `DisplayState::check`,
  which refuses (as `Inconsistent`, leaving the display unchanged) a state whose fields disagree with each other
  or with the simulation: a message flag without its counter or the reverse, a menu-only flag, `oldhealth` other
  than -1 at tic 0 or the simulation's health afterwards, key boxes and ready ammunition that differ from the
  simulation's, an impossible attack-direction counter, and a face that its priority and health cannot produce. It still accepts some states that no run produces
  (at tic 0 the fresh values of the face, key boxes and message are not pinned, and the priority 4 to 8 rows do not
  check every precondition), so it is a consistency check, not a proof that a state is reachable.
  `restore` still **trusts** the line text, `showMessages`, the palette and the widgets' old values,
  `oldweaponsowned`, `firsttime` and `bg_drawn` against the embedded screen, `rndindex`, and the pain band
  against the current health. The golden test runs `check` on all 42,352 states (all pass), and 12 forged states
  are each refused. Nothing in the app calls `restore`.
- **Admission bounds at load.** A malformed lump the single-player game never draws (the plasma and BFG arms icons, skull keys
  3-5, the minus sign, the multiplayer face background) refuses the whole display, and a PLAYPAL shorter than 14 palettes of 768 bytes is refused; the original would
  only cache those lumps. This is an over-refusal, not a divergence. The 2D drawer's `BeyondBuffer` refusal of a
  foreground post below row 199 is likewise conservative (the original would write into the next screen buffer);
  for the background buffer the overrun is real.
- **The `observe_with` hook** (used by the cheat scenario of the golden test and by fuzzing, never by the app)
  refuses adjusted inputs outside what the simulation can produce (health and armour 0-200, ammunition within its
  maximum, a ready weapon below 9, damage count 0-100, a bonus count that keeps the +7 in range, powers not
  negative) with `AdjustedInputOutOfRange`, before any state changes.
- **Not evidenced.** A mid-session change of screenblocks (`Display::set_screenblocks`) is checked only Rust
  against Rust; the app fixes the size for the session. Only `AttackerRemoved` is reached through the real
  simulation path (a test removes the attacker after its damage); the other 2D and message witnesses come from
  constructed states, and that wire 24 never removes a live monster is argued, not shown. In one state (the dead
  player with its attacker retired) the Rust refusal is wider than the original's read, so the app would refuse a
  tic the genuine code survives; it refuses rather than diverges.

### Declared contracts, witnesses and refusals

The [quirk register](QUIRKS.md) lists quirks of the original found behind this list; it is not claimed to be complete.

- **The sound-free, wipe-free `M_Random` contract** (decision S15, above): `M_Random` is 0 at a new game and
  only the status bar draws it. Every frame claim in this section carries that qualifier; the app under M5c runs
  under [another contract](#the-display-contract-with-sound-m5c) instead.
- **The masked game-dump fields (decision S16).** When the genuine status bar and HUD run, two game-dump fields
  change (the `M_Random` index, and the message on the tic it is taken). The M5b runner records and masks them;
  the masked dump equals the frozen digests on every existing item and the frozen M4b runner's on the new rooms,
  and the recorded fields are compared separately. No frozen runner, driver, contract or digest changed.
- **Display witnesses** (typed, repeatable, leave the display state unchanged; decision S17). Observing a tic
  is refused with `MessageUnderivable` (the message assignment cannot be derived from the tic's events and
  command, or disagrees with the simulation's stored code), `MessageNotTaken` (a message would stay pending in
  `player->message`), `AttackerRemoved` (the face would read a freed attacker) or an observe error (`NoPlayer`,
  or a gap: the display must see every tic from level start in order). Drawing is refused with a 2D drawer
  error (a post outside its lump, a write below the background buffer, a copy-rect check) or
  `BeforeFirstTic`, besides the M5a render refusals. None of the 2D witnesses is reached by any synthetic or
  real item. In the app a refused frame reaches the refusal screen (below).
- **Wire-24 admission of commands.** `forward` and `side` are clamped to +-50, the buttons are only attack, use
  and a weapon change with its change bit, and `BT_SPECIAL` cannot be produced (no pause or save command), so
  every recorded command is admitted by wire 24.

### Input

Linux DOOM 1.10's own mouse layer is not playable (no mouse without `-grabmouse`; only the last motion event of
a tic counts; purely horizontal moves are dropped; fire can stick), so the app chooses the **stream of events**
and keeps the genuine path from there on (decision S4).

- **The event layer** (the app's choice, not linuxdoom's X layer). Key repeats are dropped; a key release that
  arrives in the same pump as its press is held back to the next pump, so a tap shorter than a tic still lasts
  one tic (the same for mouse buttons, and for the run key under always-run); the mouse sends one event per command
  build with the motion accumulated since the last build, each axis clamped to +-319 raw device units (the window
  library's raw motion, which is not in pixels; how the clamp feels is unmeasured) and scaled `<< 2` as linuxdoom does, and
  only when there is motion or a button change; at most 63 events per build and at most 1,024 pending raw
  inputs (past that everything is released). A press is passed at most once per build (decision S6): **clicks faster than one per
  build (35 a second) are played out one per build, with fire staying on meanwhile; nothing is lost, the queue empties by itself, with
  or without motion** (the confirmed rule's behaviour, pinned by the test `clicks_faster_than_one_per_build_drain_at_one_per_build`;
  it is not the Windows [stuck-fire report](#stuck-fire-on-windows-accepted-as-fixed-in-m5d-the-cause-was-never-reproduced)). Every stream it makes is one the genuine `G_Responder` could have
  received. Sensitivity arithmetic, turning, strafing, forward movement, double-click use and the clamps are the
  genuine code ported.
- **Never forwarded** (decision S5): Esc and Pause (the app's own pause), Enter, Tab, `-`, `=`, `q` and F1-F12.
  The genuine effects of these (menus, automap, screen size, gamma, message refresh, ending a recording) are out
  of scope, and a withheld key cannot be bound. The cheat matcher is not ported: the letters of a cheat code
  reach the game as ordinary keys and do nothing, whereas the genuine engine would act on a completed code
  (see the exception under "The commands" above). Keys linuxdoom's translation
  leaves as a code of 256 or more (keypad digits, Insert, Home and so on) are not forwarded either.
- **Bindings** (decision S7); no configuration file is read. `vanilla` (the default): arrow keys move and turn,
  `,` and `.` strafe, right Ctrl fires, Space uses, right Alt strafes with the turn keys, right Shift runs,
  `1`-`8` choose weapons; mouse left fires, middle strafes, right moves forward; sensitivity 5. `modern`
  (`--bindings modern`): W, A, S, D move and strafe, E uses, always run, left button fires, no vertical mouse
  motion, no middle or right button action; the other keys as vanilla.
- **Pause and focus.** Esc or Pause pauses the app (below). The window library (miniquad 0.4.11) reports the
  window being minimised or losing focus through its minimise event, and `doom-play` turns that event into a pause.
  Every pause releases every held key and button through the event layer. Observed on 2026-11-17 under WSLg: when a
  second window took the focus, the running game paused, on X11 and on Wayland. **Not checked:** Alt-Tab,
  minimising by hand, Windows, macOS, and whether the keys are seen released on screen. From the library's source,
  not observed: on X11 any focus-out pauses (including one caused by another program's keyboard grab), and on macOS
  moving the window pauses it. Pressing Esc before switching windows remains the safe habit.
- **Recording round trip, and what it costs** (decision S3, owner-confirmed). Every session is recorded as a
  version-110 demo, and, as in vanilla recording, the game executes the command it has just **re-read from the
  demo buffer**. In plain words: while recording, turning happens in steps of 256 units (about 1.4 degrees), so
  a mouse movement or key turn that is smaller than that is rounded (the demo byte is `(angleturn+128)>>8`,
  g_game.c:1512): a turn of -128 to 127 units a tic is dropped without being carried to the next tic, and 128 to 255
  becomes 256. This costs fine mouse turning, and it is what makes every
  session a valid vanilla demo. The opt-in mode that executes full-resolution turns without recording is
  [below](#unrecorded-full-turning-mode-opt-in).
- **Input evidence.** The genuine input path (event queue, responders, `G_BuildTiccmd`, the demo write and
  re-read, linuxdoom's X translation against a declared fake Xlib) is compiled whole and run on 251 scripts
  (36,453 tics); the Rust core matches the 175 app-domain scripts tic for tic and refuses the scripts that
  use a withheld key (`WithheldKey`) or a mouse value that would overflow (`MouseOverflow`). One script of
  forwarded keys is declared not exact: typing `idclev12`, after which the genuine engine reloads the level
  at tic 8 and clears the held keys (the app has no cheat matcher); the other cheat scripts (`idkfa`, `idbeholds`
  and so on) leave the commands equal, but the genuine engine would change player state where the app does
  nothing. Findings that bear
  on behaviour (the app clamps or withholds before any of them is reachable): a built turn at the edge of a
  `short` re-reads as -32768; typing a cheat while recording would end the recording; the level load leaves the
  double-click state as it was (the app's `load_level` clears only `mousebuttons[0]`, as the declared 32-bit
  adapter does).

### Unrecorded full-turning mode (opt-in)

`--unrecorded-full-turning` (decision S3 (ii), owner-confirmed) is never the default and never implied by another
option. **What it does:** mouse and keyboard turns are executed at full resolution (the built command, after
the wire-24 admission check) instead of being rounded to multiples of 256 by the demo re-read. **What is lost:**
nothing is recorded and no file is written under any ending, so the session cannot be replayed or checked against
the genuine engine afterwards. **What is gained:** small mouse turns are no longer dropped. The window title, the
pause overlay and every end, death, cap and refusal screen say exactly "not recorded: no demo, cannot be
replayed", and the exit message prints the label instead of a save line. So that the label is seen even in
fullscreen, where the window title is not visible, the same text is also drawn in a band over the top of the
presented picture (never in the game frame) before the first tic and for tics 0 to 174 (175 tics, five seconds) of
every unrecorded session, in every window mode and after every restart; a recorded session never shows it.

Evidence, as far as it goes: the M5b runner's driver, compiled unchanged with a declared command-injection mode
(`tools/reference/m5b_inject.c`, contract `tests/reference/m5b_inject_contract.json`), runs such commands in the
genuine engine. Rust matches it on every tic (command, frame under the declared sound-free, wipe-free contract, status
fields, messages and game dump through the independent DSIM40 encoder) on 4 injected scripts on the synthetic rooms palred and palbonus (630 tics, 373
commands with a nonzero low byte of the turn) and on 2 scripted unrecorded app sessions (199 tics, 75 such
commands); the three genuine variants are identical, and the demo-resolution rounding of each script gives a
different game, so the low byte matters. **Not covered:** real maps, and any person playing it; the label on
screen is checked by tests, not seen; the input-driver step of the unrecorded-session check has no planted
fault ([evidence](validation/M5B.md#step-7b-full-resolution-unrecorded-turning)).

### The app's own behaviour (not vanilla)

- **The loop.** The first update only starts the clock; tic 1 falls due 1/35 s later. Each update runs the tics
  that are due, at most 5; the rest of the lost time is dropped, as `NetUpdate`'s bound does (decision S8).
  Unlike vanilla, which draws one picture per loop pass, the app draws **after every tic** (the M5a render
  contract), and shows the last. No clock or file type reaches the simulation: a session's tics are a function
  of its events and of how many tics each update ran.
- **Pause** (decision S12). Esc or Pause stops tics; the simulation, the render state and the demo are
  untouched; the clock is re-based on resume; the "paused" overlay is drawn only on the presented picture, never
  into the proven frame. From the pause: Esc or Pause resumes, R restarts, Q quits. This is not vanilla's pause
  (a `BT_SPECIAL` command, not admitted).
- **End and refusal screens** (decisions S9, S10). At the level's exit, or when the dead player presses use, the
  app shows its own end screen ("level complete" or "you died", the tic count and the demo's file name; the full
  path goes to the console only), in its own font and five-colour palette, with R to restart and Q or Esc to quit.
  There is no intermission, tally or next level. When the simulation or the display refuses a tic (a witness
  fires), the app stops on a refusal screen that names the witness and the tic, saves the demo, and offers restart
  or quit; nothing is approximated. **Which tics are in the demo:** a tic the simulation refuses is not (its
  command is dropped); a tic whose frame is refused after the simulation committed it **is**, so the demo
  reproduces the refusal and its replay ends the same way (the test refuses tic 15 and saves a 16-tic demo).
  **A dead player gets no hint.** After a death the game keeps running with the view on the floor until the
  player presses use; nothing on screen says so, and the owner took it for a freeze (his report of 2026-11-21). The
  original shows no prompt there either (`P_DeathThink` only tests the use button, p_user.c:227-228), so nothing
  is changed; what use does after a death is taken up by milestone M6a (decision U6 of its investigation note, which
  is on pull request #65 and not yet on master).
- **Restart is a new game** (decision S11): a fresh simulation at tic 0, both random indices 0, every status bar,
  HUD and input variable as at process start, a new demo file; the old session is saved first. This differs from
  vanilla's reload after death, which keeps process state and is not modelled.
- **Session files** (decision S21). `--sessions DIR` (default `doom-sessions/`) receives a version-110 demo named
  `E1M<map>-skill<skill>-<yyyymmdd-hhmmss>.lmp`; **the time in the name is UTC** (there is no safe local-time
  call in the standard library). The file is created when the session starts, as an empty demo (the 13-byte header
  and the end marker: 14 bytes), so a session killed at once leaves that file and an unwritable directory shows
  at the first pause. An existing file is **never replaced**: the file is created exclusively, and a collision
  appends `-2`, `-3` and so on up to `-9999`, after which the save fails visibly. The demo is rewritten whole
  every 30 seconds of play (1,050 tics) and when the session ends, through a temporary file
  `<name>.<pid>.tmp` (created new) that is synced and renamed; the new file itself appears through
  `<name>.<pid>.claim.tmp`. Since M5d the file is also extended after every tic by a writer thread, so
  **if doom-play is killed, the session file is still a valid demo but may lack the last tics the disk had not yet taken: normally none
  or one, more on a slow disk** (see the session-limit section below). The directory itself is not synced, and the page-boundary
  rewrites are not synced either, so an operating-system crash or power loss is not covered. If a write fails, the
  pause box says "LAST DEMO SAVE FAILED", the end screen and the exit message give the reason, and the write is
  tried again on quit and on restart; with the per-tic file (`doom-play`), also at the next tic and then at most once
  every 35 tics (`FAILED_WRITE_RETRY_TICS`, accepted by the owner on 2026-11-20) while it keeps failing. Since the second round of M5d every write runs on a writer thread, not on the loop thread. Before that, recording to a slow disk
  caused hitches and sound dropouts on the owner's Windows machine (the sessions folder on an HDD); with the writer threads his retest on
  that disk had none, and he accepted it as fixed ([record](validation/M5D.md#a9-the-owners-retest-of-2026-11-20-his-rulings-and-the-close)).
  **What a slow disk still costs is the kill limit: a kill can lose the tics still queued for the disk** (see the session-limit section below). At a session's end the game waits up to 2 s for the file, and Q or closing the window waits
  for the disk without a bound. A failed last write shows on the end screen and the pause box, and is written again on restart and quit. The session ends at the **tic cap of 1,048,576 tics** (about 8 hours 19 minutes), a bound
  that keeps a demo within a bounded size. A demo holds only the player's tic commands and a 13-byte header,
  nothing from the IWAD. The window's close button saves and quits, and no tic runs after a quit.
- **Game mode comes from the IWAD file name**, as linuxdoom's `IdentifyVersion` does: `doom1.wad` is shareware,
  `doomu.wad` retail, anything else registered (the mode of the M5a and M4b real-map runs). A registered-mode
  game cannot load a shareware IWAD's switch textures.
- **Skills and maps.** Only skills 2 and 3 are admitted, and not on every
  map: `--skill 2` or `hmp` is Hurt Me Plenty and `--skill 3` or `uv` is Ultra-Violence. These are the demo-header
  values; vanilla's own `-skill` counts from 1, so they are vanilla's `-skill 3` and `-skill 4`. `--map` outside
  1-9 and other skills are refused at load, as is a missing map or a truncated WAD. Observed by the orchestrator on
  2026-11-21 (local, read-only): a scratch probe of `GameData::load_with_sound` at master cc78cc3 (wire 25,
  registered mode, the owner's four-episode `DOOM.WAD`), 36 map and skill pairs; this table holds counts and rule
  names only. Skill 4 (nightmare) was not probed and is not admitted (milestone M6b).
  The map-by-skill [load table](#skills-and-maps-load-table) follows.

### Skills and maps load table

| Map | Skill 0 | Skill 1 | Skill 2 (`hmp`) | Skill 3 (`uv`) |
| --- | --- | --- | --- | --- |
| E1M1 | refused at load: `Domain` | refused at load: `Domain` | loads | loads |
| E1M2 | refused at load: `Domain` | refused at load: `Domain` | loads | loads |
| E1M3 | refused at load: `Domain` | refused at load: `Domain` | loads | loads |
| E1M4 | refused at load: `Domain` | refused at load: `Domain` | loads | loads |
| E1M5 | refused at load: `Domain` | refused at load: `Domain` | loads | loads |
| E1M6 | refused at load: `Domain` | refused at load: `Domain` | loads | loads |
| E1M7 | refused at load: `Domain` | refused at load: `Domain` | loads | refused at load: `Admission(Monster(Arithmetic))` |
| E1M8 | refused at load: `Domain` | refused at load: `Domain` | loads | loads |
| E1M9 | refused at load: `Domain` | refused at load: `Domain` | loads | loads |

Which thing on E1M7 and which exact bound refuses it is not yet known (the spawn-time sites, read but not
traced, are in `crates/doom-sim/src/demon_map_spawn.rs:164-174`: a position-admission margin and a vertical
witness; this page does not say which fires). The refusal is the existing `MonsterError::Arithmetic` ("outside
the existing bounds", `crates/doom-sim/src/monsters.rs:21-22`). "Loads" means admitted, not "compared with the
genuine engine": the comparison runs are listed in the
[real-map evidence](#original-episode-1-part-2-teleporters-the-baron-of-hell-a_bossdeath-and-hard-skill-wire-24)
(the figures stay there; E1M7 on skill 3 has no comparison run).

### Presentation (outside the proven frame)

The proven artefact is the 320x200 indexed frame and the palette number of each tic; everything below is outside
that claim (decision S13). The picture is the palette applied through the original's gamma table (table 0 by
default, which is not the identity; `--gamma 0-4` selects the other four genuine tables), scaled by nearest
neighbour, with 4:3 aspect correction on by default (the picture shown as 320x240; `--aspect square` for square
pixels), letterboxed, in a window or in borderless fullscreen (`--fullscreen`). The window loop sleeps until
the next tic is due and keeps the library's default swap interval. The mouse is captured only while playing
(not while paused or on an end or refusal screen).

### Platform limits

- **OpenGL is required**: an OpenGL 2.1 (or OpenGL ES 2) driver (the wording of `doom-play --help`). A missing
  driver is not checked beforehand and may still end in a panic inside the window library. Without `DISPLAY` or
  `WAYLAND_DISPLAY` the app gives a plain error.
- **Focus loss.** The window library (miniquad 0.4.11) reports focus loss and minimising through one minimise
  event, and the app pauses and releases every held key and button on it (see "Pause and focus" above for what
  was observed under WSLg on 2026-11-17 and what was not checked: Alt-Tab, minimising by hand, Windows, macOS
  and whether keys are seen released). Pressing Esc or Pause before switching windows is the safe habit.
  Raw mouse motion is delivered by the library whatever the focus, and the app uses it only while playing.
- **Linux under WSLg** is the platform the program was run on (X11 and Wayland, each ran to the tic count with
  an idle session; Wayland printed compositor and Mesa warnings and probably used a software fallback).
  **Windows and macOS** build in CI (the Stable job covers `doom-play`), and a label-triggered CI job builds a
  `doom-play.exe` artifact (not a required check; it ran successfully on the pull request on 2026-11-17, and
  [TESTING](TESTING.md#windows-build-artifact) says how to download and run it); **no person has run it on macOS**. The owner ran release builds of `3c1cc5d` on Windows 11 and under WSLg on 2026-11-18
  ([what he observed](#what-the-owner-observed-2026-11-18)). **Mouse capture under WSLg does not work: the
  pointer is not captured there, so WSLg is keyboard-only for now.**
- Miniquad is the only windowing dependency (pure Rust, MIT-licensed, GPL-2.0-only compatible). Since M5c the
  audio library `maudio` is a second dependency of `doom-play` only. `doom-sim`, `doom-sound` and `doom-game` have no
  windowing, audio-device, thread or wall-clock dependency.

### Not in M5b (later milestones)

(Sound effects came with [M5c](#sound-effects-m5c); the glance fix and the level-start wipe's draws too.) Music, menus, the automap, save and load, the intermission and level-to-level carry-over, the title
screen and demo loop, the screen wipe's picture, cheats, vanilla's in-game pause, easy and nightmare skill, low
detail, screenblocks below 10, multiplayer and joystick input. All of these are original behaviour, so none
is optional except what the owner declined by ruling on 2026-11-21: the [vanilla completeness checklist](ROADMAP.md#vanilla-completeness-checklist) gives each one's milestone, says
that none is named yet, or says that it was declined (music and the joystick are in, in later milestones the owner will name). The modern renderer, which is not original behaviour, is listed in
[ROADMAP](ROADMAP.md).

## Sound effects (M5c)

Milestone status is in [ROADMAP](ROADMAP.md). **As of 2026-11-18 (America/Los_Angeles) the playable build
`doom-play` plays Doom's sound effects** (no music, no menus) through the first audio backend that opens on the machine,
and the status bar face's idle glance gap is closed. The crate `doom-sound`
([architecture](ARCHITECTURE.md#sound-effects-m5c)) decides which sounds start, on which channel, at what volume, stereo
position and pitch, and computes the samples the original mixer would write; `doom-game` runs it inside the game loop
and `doom-play` hands its finished stream to the device. **What has been observed, and by whom:** on 2026-11-18 the owner played release builds of `3c1cc5d` on Windows 11 and under WSLg and reported that sound plays on both; the details, and what he did not judge, are [below](#what-the-owner-observed-2026-11-18). It is one person on one evening, on two systems: nothing was recorded of what he heard, and nobody has run it on macOS. Earlier, the only real device open was `doom-play --audio-probe`, which wrote one second of digital silence to PulseAudio under WSLg using a scratch copy of the client library ([record](validation/M5C.md#step-7-sound-in-doom-play)). Every claim below is about which sounds start, when, with which values, and the 16-bit samples the original mixer writes; none says how anything sounds.

The facts are in the [investigation note](M5C_REFERENCE_INVESTIGATION.md) (decisions T1-T16, owner rulings
2026-11-18), the observations in the [evidence record](validation/M5C.md) and the commands in
[TESTING](TESTING.md#m5c-sound-effects-the-genuine-sound-logic-and-mixer).

### The reference

Every claim about sound means: identical to **linuxdoom-1.10 built with its in-process sound code (one define
removed)** (decision T2). The one define is `SNDSERV` at `doomdef.h:84`: the source as shipped sends sound to an
external `sndserver` program, and its own mixer in `i_sound.c` exists only with that define removed. So this is not a
claim about the shipped binary and not a claim about what a 1997 player heard.

### The declared sound schedule (decision T5)

In the original, three things in the sound path follow passes of the main loop, and so the clock, not tics. This
port ties all of them to tics, as the frame contract of M5a and M5b already does:

1. One tic per pass: the tic's sound records run with `gametic` equal to the tic number `t`, then `S_UpdateSounds`
   runs with `gametic` `t + 1` (d_main.c:392), after **every** tic.
2. The mixer makes buffer k (k = 0, 1, 2, ...) of 512 frames after tic floor(512k / 315): none or one buffer per
   tic, 315 buffers every 512 tics, the first after tic 0. A tic is worth 315 frames at 11,025 Hz. A start takes
   effect in the next buffer mixed, so it is heard 0 to 511 frames (at most 46 ms) after its tic.
3. A session is a fresh process started straight on the map (decision T15): `gametic` and the handle counter are 0
   at its first tic, no channel is busy, and the mixer is silent.

The claim is therefore: identical to the reference **under this schedule**. It is not a claim about a session paced
by a real sound card. Every tool carries the other values as switches, and the committed results record what they
give.

### The level-start wipe (decision T6)

Every start straight on a map wipes, and the wipe draws `M_Random` exactly 320 times after the first frame (f_wipe.c).
The port accounts for the 320 draws at the declared point, after the frame of tic 0: no melt picture is shown and the
wipe takes no time. The accounting is part of the [display contract with sound](#the-display-contract-with-sound-m5c)
below. Where the `M_Random` index lives is in [ARCHITECTURE](ARCHITECTURE.md#where-the-m_random-index-lives).

### The display contract with sound (M5c)

The M5b contract (the declared sound-free, wipe-free contract) is kept as it was for the M5b profile; the app runs under a
second, explicit contract, `RandomContract::WithSound` (decisions T6, T15 and T16). Under it the one `M_Random` index
advances, **within each tic and in this order** (the C's order for the tic the simulation has just committed):

1. the simulation's tic (`P_Ticker`, g_game.c:730) has made its sound records;
2. the sound session runs every record through the sound logic at `gametic` equal to the tic, in call order, each draw
   advancing the index as `M_Random` does (m_random.c:63-67); then `S_UpdateSounds` with `gametic` plus one (d_main.c:392),
   which draws nothing; then the mixer's buffer if one is due (d_main.c:397-400, the [schedule](#the-declared-sound-schedule-decision-t5));
3. `ST_Ticker` draws once (g_game.c:731, st_stuff.c:992), then `HU_Ticker` runs (g_game.c:733);
4. the frame is drawn (`D_Display`, d_main.c:395); on the one frame that wipes, the first of the level (`gamestate !=
   wipegamestate`, d_main.c:223), the index then advances by 320 (`wipe_EndScreen` and the melt loop after the composed frame,
   d_main.c:327-340; the first `wipe_ScreenWipe` call runs `wipe_initMelt`, which draws for the first column and for
   each of the other 319, f_wipe.c:159-166, 280-286). No melt picture is produced.

**Where the port's order differs from the C's.** In the C, `G_Ticker` runs `P_Ticker` and then `ST_Ticker` and
`HU_Ticker` (g_game.c:730-733), `S_UpdateSounds` follows (d_main.c:392), then `D_Display` (d_main.c:395), and the mixer's
buffer comes last (d_main.c:397-400). The port runs `S_UpdateSounds` and the mix **before** the status bar's ticker, where
the original runs `S_UpdateSounds` after it and the mix after the frame. No value flows between them (the `M_Random`
index, which both touch, keeps the original's order: the sound draws, then the status bar's one), so every output is equal and
the frame gate (below) found none that changes ([note section 4.3.3](M5C_REFERENCE_INVESTIGATION.md)). The one observable difference is
which refusal is named if two fall in the same tic: an `S_UpdateSounds` refusal (`OriginGone`) and an `ST_Ticker` refusal
(`AttackerRemoved`) in one tic report the sound one here and the status bar one in the original's order. An earlier version of
this paragraph said the update, too, came after the frame's draw in the C; that was wrong, and only the mix does.
The saved display state under this contract is [`DDSP` version 2](STATE_FORMAT.md#display-state-ddsp-version-2-m5c).

### The status bar face's idle glance is closed with sound (decision S15, T14)

**What is now true, in the owner's words (decision T14):** the status bar face matches the genuine engine replaying the
session's demo, under the declared schedule ([T5](#the-declared-sound-schedule-decision-t5)) and the declared build
([T2](#the-reference)). A vanilla player's menu hot keys, `-`, `=`, F1 to F10 and Esc, start menu sounds and so draw
`M_Random` between tics; they are not part of a recorded demo, the app does not pass them to the game, and they are
**not modelled**: a vanilla player who pressed one during play would have shifted the stream. Nothing else on screen
changes: with sound's and the wipe's draws in, the genuine engine's face differs from the M5b pictures in 68,995 of
158,472 real-map frames and 11,392 of 21,043 synthetic frames (the note's [section 5, T14](M5C_REFERENCE_INVESTIGATION.md),
measured before the Rust side was built), and in no frame does anything outside the face's rectangle differ.

**Evidence** (details and every count in the [evidence record](validation/M5C.md#step-5-part-1-the-frame-gate-restore-and-continue-and-the-headless-app-tests)):
on the 64 synthetic items at screenblocks 10 and 11, all 45,540 genuine frames equal and the face index equal on every
frame (45,170 whole-frame digests, 300 compared on the masked digest, 70 not digest-compared as under M5b); locally, on
the 61 real Episode 1 runs (158,966 tics) through the whole Rust pipeline, every frame at both sizes, every status-bar
row and every sound row equal, with the face index equal on 158,966 of 158,966 frames at each size; and 9 scripted app
sessions (2,213 tics) replayed through the genuine input driver and the genuine engine, every command, frame and face
equal. These are the cases checked, not a proof for every possible session; the app's own sessions can be checked as
under M5b. The claim covers what the demo records: the exceptions of the M5b frame claim (masked columns, the 70
switch-parity frames, the cheat codes the app does not forward) still apply as they did, and the glance is also
subject to everything listed [below](#the-app-with-sound-and-what-is-presentation).

### Refusals instead of the original's errors (decision T11)

The [quirk register](QUIRKS.md) lists quirks of the original found behind this list; it is not claimed to be complete.

Where the original stops with `I_Error`, or reads memory it does not own, this port refuses with a typed error **at
the exact start, with every part of the state unchanged**. None of these is reachable on an Episode 1 map of the
IWADs as far as the investigation found, and each has a synthetic witness in the tests.

| Case | What the original does | Refusal | Source |
| --- | --- | --- | --- |
| Sound number 0, or 110 and above | `I_Error("Bad sfx #")` | `SfxId` | s_sound.c:277-278 |
| Sound number 109 | passes that check and reads one entry past the `S_sfx` table | `SfxId` | s_sound.c:277-280 |
| A start that obtains a channel, whose `ds` lump is missing | `W_GetNumForName` ends in `I_Error` | `LumpMissing` (a start that finds no channel never looks the lump up, as in the original) | s_sound.c:363-365, i_sound.c:451-456, w_wad.c:399-409 |
| A lump of 8 bytes or fewer | the original reads one byte past its block, and the value depends on memory layout (it differed between builds) | `Mix(ShortLump)` | i_sound.c:230-247, 591, 606 |
| A pitch outside 0..255 | indexes past the 256-entry step table | `Mix(Pitch)` (the sound logic cannot make one: it computes 128 plus at most 16 either way) | i_sound.c:495, s_sound.c:324-345 |
| A volume or separation that makes a left or right volume outside 0..127 | `I_Error("rightvol out of bounds")`, then `leftvol` | `Mix(RightVolume)`, `Mix(LeftVolume)` | i_sound.c:364-368 |
| A switch release in button slot 15 | reads one slot past `buttonlist` | the wire-25 tic is refused (`ButtonSlot15`); the sound session refuses too | p_spec.c:1151 |
| A sector whose sound-origin arithmetic overflows | signed overflow in `P_GroupLines` | the map is refused at load (`SectorOriginOverflow`), wire 25 only | p_setup.c:536-557 |
| No `dspistol` lump when the sound set loads | `I_InitSound` replaces every missing lump with it, then `W_GetNumForName` ends in `I_Error` | the load is refused (`PistolMissing`) | i_sound.c:213-214, w_wad.c:399-409 |

The crate has a few further refusals that are not the original's behaviour: a start with positions but a NULL
origin, an origin that no longer resolves when `S_UpdateSounds` needs its position, a volume or channel count
outside the configuration bounds, and the limits on the size of sound data. They guard the Rust code's own
invariants and are labelled Rust-only safety checks in the [evidence record](validation/M5C.md).

### The two switch sounds (decision T12)

- **Press.** The original passes `buttonlist->soundorg`, the value in button slot 0, **read before the new button is
  stored** (p_switch.c:228-256). While slot 0 is free that value is NULL, so the click plays at full volume, centred,
  from any distance; while slot 0 holds a button it is that other line's sector. Wire 25 does exactly this.
- **Release.** `p_spec.c:1151` passes the address of the pointer field, not its value, so the position is read out
  of the **next** button slot: on a 64-bit build x is that slot's timer and y is 0; on the original 32-bit layout x is
  that slot's texture number and y is its timer. Both are within a hundredth of a map unit of the map's origin (0, 0),
  so the click seems to come from the map's origin, not the switch, and whether it is heard (and so whether it draws a
  random number) depends on how far the player is from that point. **This port follows the 64-bit reading, declared**:
  x is the next slot's timer and y is 0. The two readings differ only while the next slot is busy.
- **Slot 15 is refused**, because it reads past the array.
- **The layout-dependent label.** At every switch release whose next slot is busy, and at every later
  `S_UpdateSounds` read of that origin, the session works out what the 32-bit build could have read there (any
  texture number of the loaded WADs as x, the next slot's timer as y) and compares it with the declared reading. If
  any of them would change whether the sound is audible, its volume, its separation or the `M_Random` draws (at the
  start), or whether it is audible (at a later read), the session is **labelled layout-dependent from that tic**
  and the firing is counted. The session always continues on the 64-bit reading. **Every report counts
  layout-dependent sessions separately.** The label and the count are part of the saved sound state.
  **Where `doom-play` says so.** When the detector has labelled a session of the run, the exit lines on standard error
  (printed with or without `--tic-stats`, after the tic statistics and before the line saying where the demo went)
  carry one plain line: `doom-play: layout-dependent session from tic T (N detector firings): a 32-bit build could
  have played a switch-release sound differently; this one used the 64-bit reading`, with `firing` for N = 1, and
  `; K earlier session(s) of this run were labelled too` appended when a restart left earlier labelled sessions (a
  current session that is not labelled then reads `this session is not layout-dependent; ...`). A run with no labelled
  session prints no such line. **Nothing is drawn on the screen and the end screen is unchanged**, and the saved demo
  holds no label (a replay recomputes it). The exit line alone is enough: **the owner confirmed on 2026-11-18** that nothing is
  drawn on the screen. The headless session check (`tools/reference/m5c_session.py`) and `tools/local/m5c_session_real.py`
  print the same label beside each part's verdict. No committed input makes the detector fire inside a played
  session, so a real window session has never produced this line; the exact text is tested by feeding a session whose
  detector fires at tic 1.
  **Layout-dependent sessions counted so far: 0.** The detector fired 0 times over the 64 synthetic items, the 9 scripted
  app sessions and the 61 real Episode 1 runs, so none is a layout-dependent session. The 61 real runs hold 4 switch-release
  starts, none with a busy next slot, and no later `S_UpdateSounds` read of a release origin; the 9 sessions had no release
  with a busy next slot, so the detector was never exercised there. The genuine 32-bit build's sound streams equal the 64-bit
  ones on all 61 real runs (0 differing rows). On the one release where the two genuine builds read different values (the
  committed script SNDBTN2, where the next slot is busy), the detector does not fire: no texture number of that room's WAD
  changes anything audible. The owner's expectation, that it never fires on the real runs, held.

### Audible quirks of this source, reproduced as they are (decision T13)

None is "fixed": a fix would be a new profile needing its own decision. This is what a listener will hear that the
original does, and each line is as the original's code behaves.

| # | What the player will hear | Source |
| --- | --- | --- |
| 1 | Pitch varies at random on most sounds, by about 0.85 to 1.19 times normal speed (the chainsaw sounds vary less; `itemup` and `tink` not at all). Each variation is a random draw | s_sound.c:327-345 |
| 2 | A sound keeps playing to its end after its source is gone, and its volume and stereo position are fixed when it starts: `I_StopSound` and `I_UpdateSoundParams` do nothing. "Playing" for the sound logic means `gametic` has not reached the sound's handle, which has nothing to do with its length | i_sound.c:505-513, 675-686, 517-521 |
| 3 | A sound lump whose header declares a rate other than 11,025 Hz is still played at 11,025 Hz (a lump declaring 22,050 Hz is an octave low and twice as long; 1 such lump in `DOOM.WAD` and 4 in `DOOM2.WAD`, [counts](validation/M5C.md#step-2-the-genuine-mixer-harness-and-its-goldens)) | i_sound.c:99, 185-251 |
| 4 | The switch-release click comes from near the map's origin, not the switch, so it can be silent or loud depending on where the player is. The press click comes from the origin of slot 0 (above) | p_spec.c:1151, p_switch.c:228-256 |
| 5 | On map 8, nothing is ever out of earshot, and with the default sound volume of 8 a distant sound is **louder** (up to 15) than a near one (8) | s_sound.c:773, 800-806 |
| 6 | The chaingun sound is linked to the pistol's and the link gives it length 0, so it would play one frame; no game code starts it | sounds.c:204, i_sound.c:801-811 |
| 7 | The exit switch never plays its own sound, only the ordinary switch sound: the line's special is cleared at line 211-212, before line 221 tests it | p_switch.c:211-222 |
| 8 | Only three sounds can be tracked by the sound logic at once (`snd_channels`), though the mixer plays eight, so a fourth sound either replaces a tracked one or, when none may be replaced, is not started (it still draws its random number) | m_misc.c:282, i_sound.c:94 |
| 9 | **Early in a session a sound can fail to start for want of a channel, and later it cannot.** The sound logic counts a channel as busy while `gametic < handle` (`I_SoundIsPlaying`, "Ouch."), and in this build the handle is a counter that starts at 100 and takes the next value (101, 102, ...) at each start that reaches the mixer. So in the first 100 or so tics of a session, a started sound's channel stays busy until `gametic` reaches its handle, however short the sound, and with only three tracked channels (row 8) starts can be refused (one that finds no channel is not played; it still draws). Once `gametic` is past the handle, a channel is freed at the first `S_UpdateSounds` after its start (here, the end of that tic), even though the mixer goes on playing the sound to its end. A started sound is never cut short by this. 110 of the 26,077 starts of the 61 real runs found no channel | i_sound.c:517-521, 334-340; [note section 1.1.6](M5C_REFERENCE_INVESTIGATION.md) |

### What the sound logic has and has not been compared with

Against the genuine engine (the reference above, under the declared schedule):

- **The 64 synthetic items**, end to end from the Rust wire-25 simulation: every start, stop, channel stop,
  per-update volume, separation and pitch, lump lookup and the `M_Random` index after each tic, and the mixer's every
  buffer on those items and on 23 scenarios ([step 4](validation/M5C.md#step-4-part-2-the-session-the-detector-the-saved-state-and-the-end-to-end-gate)).
  Through the display and the game loop as well: 45,540 frames, with the face index equal on all of them (45,170 compared on the whole-frame digest, 300 on the masked digest, 70 not comparable, the same 70 as under M5b; [step 5](validation/M5C.md#step-5-part-1-the-frame-gate-restore-and-continue-and-the-headless-app-tests)).
- **The 61 real Episode 1 runs** (local only, counts and digests committed): 158,966 tics through the whole Rust pipeline,
  every sound row, every frame at screenblocks 10 and 11 and all 98,578 mixer buffers equal
  ([step 5 part 2](validation/M5C.md#step-5-part-2-the-61-real-runs-through-the-whole-pipeline-local-only)).
- **The 9 scripted app sessions** (2,213 tics, 133 sounds started, 1,367 buffers handed to the sink), through the app
  and against the genuine engine ([step 5 part 3](validation/M5C.md#step-5-part-3-scripted-app-sessions)).
- **Not compared: the listener columns of starts with no position.** For a start with a NULL origin or the listener's
  own body the genuine row carries the listener's position and angle, and a wire-25 record carries only what the sound
  logic reads, so those three columns are not compared on 381 of 785 start rows of the 64 items, 5,980 of 26,077 of
  the 61 real runs and 43 of 142 of the 9 sessions; every other column of those rows is compared, and the sound logic
  does not read them there (s_sound.c:304-324). A request to the simulation to carry them (so that they could be
  compared) is open.
- **Other things the gates do not compare** (the headers and totals rows, the wipe's rows, the fake device's rows, four
  items shorter than their demo, two buffers before each short-lump refusal) are listed with the tests in
  [TESTING](TESTING.md#what-the-sound-gates-do-not-compare); they are evidence-completeness points, not known
  differences.

### What the sound code refuses

Every refusal below is typed, happens before anything is committed and leaves the session, the mixer and the display
exactly as they were. The first six were added after the independent review
([record](validation/M5C.md#step-13-independent-review)); the others are older and are listed here once, for completeness.

- **A start with the wrong placement** (`SoundRefusal::Placement`). The C measures the distance and stereo position of every
  start whose origin is neither NULL nor the listener's own body (s_sound.c:304-320), and never those two. So a start of a
  sector's or a switch button's origin, or of an actor other than the listener's body, **must carry a placement and is
  refused if it does not**; a start of a NULL origin or of the listener's own body **must not carry one and is refused if
  it does**. (The simulation makes neither; the session no longer trusts its producer. The sound number is checked first.)
- **A saved sound state of another configuration** (`StateError::ConfigMismatch`): see [DSND](STATE_FORMAT.md#sound-session-state-dsnd-version-1-m5c).
- **The two views of the `M_Random` index disagreeing** (`FrameRefusal::SoundIndex`): the display advances its index by the
  draws the sound session reports, and the session returns its own index; the frame source requires that the two agree.
- **A draw count out of range** (`SessionRefusal::DrawCount`): the draws of one tic are counted in 16 bits; 65,535 are
  exact, 65,536 are refused (before, the count saturated). No real tic comes near it.
- **A sound volume outside 0..=127** given to `adjust_sound_params` or the layout detector (`SoundRefusal::Config`): before,
  such a value overflowed and panicked in a debug build. The session's own configuration is always checked, so no game
  reaches it.
- **The mixer's refusals are checked in the C's order**: pitch (i_sound.c:495), then the right and left volume rows
  (i_sound.c:364-368), then a short lump, which the C reads only when the next buffer is mixed (i_sound.c:591). Only which
  refusal is named changes; the C's volumes are always in range.
- **A lump of 8 bytes or fewer, a missing lump, a button slot 15, a sector-origin overflow, a sound number outside 1..=108**:
  as in the sections above (decisions T11, T12).
- **A forged display state** whose wipe flag the original could not hold at its tic (`DDSP` version 2 only):
  [STATE_FORMAT](STATE_FORMAT.md#display-state-ddsp-version-2-m5c).

### The app with sound, and what is presentation

**Proven (compared with the genuine engine, under the declared schedule and build):** the sound records of wire 25;
every start with its volume, separation, pitch and channel; every stop; the channel table after `S_UpdateSounds`; the
`M_Random` index; and the mixer's sample stream, buffer by buffer by digest (decision T8). **Presentation, outside every
claim:** the hand-over of the finished stream to the device thread, resampling from 11,025 Hz to the device's rate (by
the audio library), the size of the queue and the latency that results, what happens on underrun (silence) and overrun
(the oldest queued frames are dropped; the game loop never waits for the device to play anything, because waiting would change pacing: it takes a lock on the queue that the device's callback holds only while it copies one period out, and that the loop holds only while it copies one buffer in, so it can wait for at most one such copy), the
player's volume and mute, and the device itself. Of these the app's code is Rust-only safety tests, not equivalence.

- **Per tic** the app runs the simulation, the sound session, the status bar's draw and the frame, in that order
  ([architecture](ARCHITECTURE.md#sound-effects-m5c)); the session's zero or one buffer of 512 stereo 16-bit frames goes
  to the audio sink before the tic's host step.
- **Options (decision T9).** `--volume N` (0 to 100, default 100) is a gain applied to the finished stream at the
  device; the sound logic always runs with a fresh process's defaults (the original's volume setting 8, 3 channels), so
  the game, the face and the demo do not depend on it. `--no-sound` opens no device and prints nothing; the sound logic
  and the mixer still run, so the face and the demo are the same. **End** mutes and unmutes (the player's own key, never
  seen by the game; it is not one of the original's keys); **pausing silences the sound** (the queue is cleared and
  frames pushed while paused are ignored), and the game's pause releases it again on resume.
- **A session and a restart are a fresh process** (decision T15): `gametic` and the handle counter are 0 at the first
  tic, no channel is busy, the mixer is silent and the queue is cleared. `doom-play` runs wire 25 with sound; wire 24 and the
  M5b contract remain selectable in the libraries, are not offered as a launch flag, and `--no-sound` gives silence with
  the same face. Demos recorded by M5b builds stay valid; replayed here they give the same game and the genuine
  glance of this contract.
- **The device.** The first audio backend that opens plays the stream (Linux: PulseAudio, then ALSA, which need the
  system's `libpulse.so.0` or `libasound.so.2`; Windows: WASAPI, DirectSound, then WinMM; macOS: Core Audio); nothing
  falls back to a silent device. If none opens, **one line on standard error** says so and names every backend tried, and
  the game runs exactly the same without sound output. The queue (512-frame buffers; the device takes nothing until **two
  buffers, 1,024 frames, 92.9 ms at 11,025 Hz, are queued**, and again after every underrun and every pause; at most four
  buffers, 2,048 frames, 185.8 ms, are kept, the oldest frames dropped beyond that) is presentation and **not measured**,
  and the latency that follows (at least the 92.9 ms queued at the start, plus the device's own buffering) is by
  arithmetic only; none of it is part of any claim. **Why two buffers:** under the declared schedule buffer k is handed
  over at tic floor(512k/315) while a device at exactly 11,025 Hz needs it (512k/315) tics after it starts, so with one buffer as the
  threshold the margin falls to nothing at every k that is a multiple of 315 (every 14.6 s) and ordinary sleep jitter gives an
  underrun (a gap); two buffers keep a margin in hand, for more latency (the margin is 315 frames, 28.6 ms, by the arithmetic in [sound latency](#sound-latency-arithmetic-not-measured); the code comment, which said one whole buffer and 46.4 ms, now says the same). (An earlier version of this page, and
  the first implementation, started at one buffer; a review of the device code found the margin.) `doom-play --audio-probe` writes about one second of silence to check the
  device. **The owner accepted the two-buffer start on 2026-11-18 as presentation, unmeasured; he will judge lag and
  crackle by ear.**
- **No music, no menus.** The menu hot keys are not modelled (see the glance above).
- **When the sound session refuses a tic** (a typed refusal: for example a PWAD's sound lump of 8 bytes or fewer, decision
  T11), the app stops on its refusal screen, which names the witness and the tic. **When the sound session or the display
  refuses a tic, the simulation's tic stays committed and its command stays in the demo, sound and display stay at the last
  good tic, and the demo replays to the same refusal (as under M5b's rule S9 for frame refusals; confirmed by the owner,
  2026-11-18).** A future save must never combine a simulation at tic N with sound or display at tic N-1. Rolling the simulation back too would need a staged
  advance in `doom-sim` that does not exist, and a full-simulation clone per tic is not allowed. The refusal screen
  shows the witness as a plain sentence (for example `sound: tic 18, sound call 0: mixer: refused: sfx 1 has no sample
  byte at its start (i_sound.c:230-247, 591)`), not as debug text. What a person sees is that the refused tic stays in the
  demo and the replay refuses at the same tic; the genuine engine would have played that tic (for a short lump it reads past
  the block, for a missing lump it calls `I_Error`).
- **Under WSLg the game occasionally opens already paused.** The window can report a focus loss in its first moments,
  and the M5b focus-loss rule pauses the game (the same cause and effect as M5b's pause, [above](#platform-limits)). The
  PAUSED box shows, the mouse is released, and **Esc or Pause resumes**. It was seen in 3 of 260 runs in the
  diagnosis and in 5 of 49 in an earlier set, with the M5b build too (the sound work is not involved:
  [record](validation/M5C.md#step-7-sound-in-doom-play)). Whether the owner met it in his playtest was not reported.
  **The owner agreed on 2026-11-18 that the start-up pause stays as it is; he will judge it when he plays.**

### What the owner observed (2026-11-18)

**The owner observed** the following on the evening of 2026-11-18 with release builds of `3c1cc5d`. These are one
person's observations on one evening; no test, recording or measurement stands behind them.

- **Windows 11, the native artifact, `--bindings modern`, E1M1 on `hmp` and `uv`.** The window opens, the game plays at
  35 frames a second, the picture looks right, the mouse is captured and feels good, and sound plays. He noticed a
  delay on every sound at first and then judged it normal for the original (see [sound latency](#sound-latency-arithmetic-not-measured)).
- **WSLg, the same commit.** The window opens and sound plays through PulseAudio (device "RDP Sink"). **The pointer is
  not captured and turning by mouse is unusable there, so WSLg is keyboard-only for now.** What is asked of the window
  library is a pointer grab confined to the window plus raw mouse motion; whether the grab succeeds under WSLg's
  Wayland compositor and remote-desktop connection, and whether raw motion arrives there, is not known without
  measuring (the library's source ignores the grab's result).
- **Not judged:** macOS, long sessions, every map, every weapon and monster, vanilla bindings. He also recorded
  sessions on E1M2 and E1M3; what he saw there he did not report.
- **His 12 session files** replay equal to linuxdoom-1.10 built with its in-process sound code (one define removed):
  12 of 12 pass the genuine comparison (8 on E1M1 at `hmp`, 1 on E1M1 at `uv`, 2 on E1M2 at `uv`, 1 on E1M3 at `uv`;
  25,309 tics in all, one file holding none). This shows that the files replay equal to the genuine engine; it does not show what
  was on his screen or in his ears ([counts](validation/M5C.md#the-owners-playtest-2026-11-18)).
- **A fault was reported on Windows** (accepted as fixed in M5d): [stuck fire](#stuck-fire-on-windows-accepted-as-fixed-in-m5d-the-cause-was-never-reproduced).

### A killed session: two kills shown on Windows, nothing shown on macOS (a limit)

**Since M5d the session file is kept current tic by tic. This is shown on Linux by tests and on Windows by two kills of the final build
made by the owner's hand (one machine); on macOS nothing is shown and the limit stays.** The demo is created whole when a session starts and then extended by one 5-byte write
per tic, made by a writer thread. **If doom-play is killed, the file is still a valid demo but may lack the last tics the disk had not
yet taken: normally none or one, more on a slow disk.** At every instant it is a valid demo holding at least every tic the writer had
finished and at most every tic run; the tics still queued are lost by a kill, at most 175 of them, and while a disk is stalled the lag in
tics is not bounded (only the memory is). This weakens the first build's "every tic run, or every tic but the last"; the owner accepted
it on stated conditions ([old and new side by side](validation/M5D.md#a7-the-owners-windows-session-of-2026-11-20-and-the-second-round), [the write schedule of the first build](validation/M5D.md#when-the-demo-is-written-fshost-as-doom-play-runs-it)). It is also rewritten whole
every 30 seconds of play, when the session ends, and when you restart or quit (Esc, then R or Q; R, Q or Esc on an end screen)
or close the window with its close button; pausing writes nothing.

- **Shown, on Linux (this machine, under WSL2):** SIGKILL at random points (a test that runs in CI on Linux and macOS, and
  2,000 kills locally; its lag distribution on ext4 and on `/mnt/d` is in the [record](validation/M5D.md#kill-evidence)), SIGINT (what Ctrl+C sends) and a lost display connection under WSLg (reproduced through a proxy,
  with the same "X connection to ... broken" message the owner saw). Every surviving file passed the genuine-engine session
  check ([counts](validation/M5D.md#evidence-release-builds-unless-stated)). That the single write cannot be cut short
  rests on a reading of how the Linux kernel writes to a file, not on a test: deterministic tests pin that the code makes one
  five-byte write, and the kill tests are evidence for the steady state; neither is a guarantee for every file system.
- **Tried on Windows, first on the first build:** on 2026-11-20 the owner ended `b374aaa` runs by Ctrl+C three times and by
  closing the console four times (before the writer threads existed); the seven files each had a valid size (14 + 4n bytes), no
  temporary file was left, and all passed the genuine session check ([record](validation/M5D.md#the-session)). That is history:
  the writer-thread build was played the same day and every session was ended by Esc then Q
  ([record](validation/M5D.md#a9-the-owners-retest-of-2026-11-20-his-rulings-and-the-close)).
- **Shown on Windows, on the final build (2026-11-21): two kills.** The owner killed the Windows artifact of the final head
  (`9610b51`) twice, once with Ctrl+C in the console and once by closing the console, on one machine, with the sessions folder on a `D:` disk
  that he states is NTFS (not checked from here). Both files are valid demos (1,461 and 1,581 tics, with the end marker, no temporary file left)
  and both passed the genuine session check ([counts](validation/M5D.md#a10-addendum-the-session-check-of-the-owners-hand-played-demos-2026-11-21)).
  Two kills are not a rate.
- **Not shown: End task on Windows; any other machine, disk or file system; how many tics had been played beyond what each of those two
  files holds (a killed process prints no exit line, and those runs have no input log); and anything on macOS.** For these
  the limit stays as it was: **a session ended any other way than by doom-play may lose its last tics, and a session may be
  left as an empty demo of 14 bytes**, until the owner has tried them. For End task the design is expected to hold
  (inference from the system's documented behaviour, untested). **On macOS, to keep a session, end it with Esc then Q before closing
  anything.**
- Two caveats that remain even where it is shown: **where hard links are refused** (FAT, exFAT, some network shares) the claim
  falls back to M5b's way (create, then write), a kill in that window can leave an empty `.lmp`, and one line on standard error says
  the fallback was taken; and a kill can leave a harmless `<name>.<pid>.claim.tmp` (during a claim) or `<name>.<pid>.tmp` (during a
  whole rewrite) file in the sessions directory. At every start doom-play removes such leftovers of its own naming that are more than an
  hour old from the sessions directory and prints how many.
- **Slow disks: a kill can lose the tics still queued.** Recording to a slow disk (an HDD or a network share) no longer holds the game
  up: in the owner's retest on his HDD single writes took up to 634 ms, the game dropped no tic and no sound underrun was counted
  ([figures](validation/M5D.md#the-retest)). What remains is that the file lags the game by the tics waiting for the disk, and a kill
  loses them: [at most 17 tics in his ten runs on that disk](validation/M5D.md#the-retest), and
  [620 once in the kill test through WSL's view of the same drive](validation/M5D.md#kill-evidence), where the test's child runs far
  faster than play. Neither figure is a bound. On exit, `session file writes` and (with `--input-log`) `input log writes` lines give
  the longest write and the deepest queue ("at most W tics waiting"), and `--tic-stats` lists every write over 20 ms.
- **If the session writer's thread panics** (a defect in doom-play; none has been seen), the files stop at its last completed write
  and earlier sessions it still owed a rewrite are not rewritten. On exit, standard error says that the writer stopped and names every
  session that was not saved, with its path and at least how many of its tics the file holds (or that no file was created); a session whose
  file is whole is reported as saved. Nothing is rewritten after the panic ([roadmap](ROADMAP.md),
  [record](validation/M5D.md#a-writer-thread-panic-names-the-sessions-not-saved)).
- Unchanged: under WSLg, when the display connection is lost, Xlib prints "X connection to :0 broken (explicit kill or
  server shutdown)." and ends the process before any of doom-play's code runs, so no exit line is printed (the window
  library installs no handler, and installing one needs unsafe code, which the workspace forbids); the demo is now
  already current when that happens. Earlier history of this limit is in the
  [M5c record](validation/M5C.md#the-lost-display-connection-under-wslg-what-is-saved).

### Sound latency (arithmetic, not measured)

Nothing here was measured. Each figure is arithmetic from our code or from the linuxdoom-1.10 source, or a default read
from a library's source; what depends on the operating system's audio engine, the drivers, WSLg's remote-desktop audio,
clocks and timer resolution is not known without measuring on a machine.

**The table is for the 128-frame device period, the default since 2026-11-21, with the two-buffer start. It was recomputed on
2026-11-21 by the link model of the [M5c record](validation/M5C.md#sound-latency-the-arithmetic-in-full) with the period changed from
512 to 128 frames. It is arithmetic, not measured, and assumes the back end grants the period asked for.**

| From the call that starts a sound to | linuxdoom-1.10 built with its in-process sound code | doom-play under PulseAudio (WSLg) | doom-play under WASAPI (Windows) |
|---|---|---|---|
| the hand-over to the operating system | above 0 to 46.4 ms (about 46.4 ms on a fast machine) | 28.6 to 86.4 ms | 40.2 to 98.0 ms |
| the end of the device buffer it asks for | above 0 to 92.9 ms (about 92.9 ms) | 40.2 to 98.0 ms | 63.4 to 121.3 ms |

How the doom-play figures are made, link by link (frames at 11,025 Hz, where one frame is 0.0907 ms; arithmetic, not measured):

| Link, at a period of 128 frames | Least | Most |
|---|---|---|
| The declared schedule and the two-buffer start (`315 + d`; the period does not change it) | 315 frames, 28.6 ms | 826 frames, 74.9 ms |
| The callback's phase: 0 to one frame less than a period | 0 | 127 frames, 11.5 ms |
| WASAPI only: a pulled chunk waits for one period of room | 128 frames, 11.6 ms | 128 frames, 11.6 ms |
| **Sum to the hand-over, PulseAudio** | **315 frames, 28.6 ms** | **953 frames, 86.4 ms** |
| **Sum to the hand-over, WASAPI** | **443 frames, 40.2 ms** | **1,081 frames, 98.0 ms** |
| Buffer ahead, PulseAudio: one period (`tlength`) | 128 frames, 11.6 ms | 128 frames, 11.6 ms |
| Buffer ahead, WASAPI: two of the three periods asked for | 256 frames, 23.2 ms | 256 frames, 23.2 ms |
| **Total to the end of the buffer asked for, PulseAudio** | **443 frames, 40.2 ms** | **1,081 frames, 98.0 ms** |
| **Total to the end of the buffer asked for, WASAPI** | **699 frames, 63.4 ms** | **1,337 frames, 121.3 ms** |

**The plain answer: at this default doom-play's ranges overlap the original's and reach above it; they do not sit inside it.**
On both back ends the low end is inside the original's range and the top is above it: under PulseAudio by 40.0 ms (441 frames) to
the hand-over and 5.2 ms (57 frames) to the end of the buffer; under WASAPI by 51.6 ms (569 frames) to the hand-over and 28.4 ms
(313 frames) to the end of the buffer. So the arithmetic no longer says that every sound on Windows is later than the original's
(it said so for the 512-frame period, below); it says a sound can be later: by up to those amounts measured top against top, and by more
against an original on a machine slow enough to sit low in its own range. **What accounts for it:**
in the original the mix happens in the same pass of the main loop as the call, and a blocking write paces the loop, so no
queue lies between game and device. doom-play adds a queue between a loop paced by tics and a callback paced by the device:
the two-buffer start plus the wait for the next buffer boundary of the declared schedule (up to 74.9 ms, the largest link, which
the period does not change), and the callback's phase (up to one 128-frame callback); on WASAPI, miniaudio pulls a chunk one period
before the engine has room for it and the three requested periods keep two ahead (11.6 ms and 23.2 ms at this period).

**What this arithmetic leaves out.** It assumes the device's callbacks arrive evenly and the back end grants what is asked. Neither
is shown: in one scratch probe WSLg's sink granted three quarters of the length asked for
([record](validation/M5D.md#observed-on-this-machine-linux-under-wslg-silence-only)), and the model's queue depth at a callback for
this period (two-buffer start: minimum 384, mean 791.5, maximum 1,152 frames;
[M5d record](validation/M5D.md#ruling-3-withdrawn-the-arithmetic-as-a-report)) was exceeded in four of the owner's five later runs
at 128, whose deepest queue was above that maximum, which is not explained; a deeper queue means more delay than the table gives.
The owner heard 128 as quicker than 512, by ear, and no number of milliseconds is claimed.

**The 512-frame period, the default until 2026-11-21** (the same arithmetic, in full in the
[M5c record](validation/M5C.md#sound-latency-the-arithmetic-in-full); not measured; it can still be chosen with
`--experimental-audio-period-frames 512`):

| From the call that starts a sound to | doom-play under PulseAudio (WSLg) | doom-play under WASAPI (Windows) |
|---|---|---|
| the hand-over to the operating system | 28.6 to 121.3 ms | 75.0 to 167.7 ms |
| the end of the device buffer it asks for | 75.0 to 167.7 ms | 167.9 to 260.6 ms |

At that period the whole WASAPI range was above the original's (by 28.6 ms at the low end and 121.3 ms at the top, to the
hand-over), and under PulseAudio the top was above it by 74.8 ms.

**The owner's judgement stands beside this and is not reconciled with it.** He heard a delay on every sound, judged it
normal for the original, and ruled on 2026-11-18 (M5c) that nothing changes (the two-buffer start stays; the period became 128
frames later, below). He judged by ear, on one machine, on one evening; the arithmetic for the 512-frame period then in use says
the Windows range lies above the original's. Neither is measured against the other.
**Latency is presentation and outside every claim** (above). The tables, the sources, the links that could be shortened
later and the unknowns are in the [evidence record](validation/M5C.md#sound-latency-the-arithmetic-in-full).

**M5d added measurements of the device, not a fix.** On exit `doom-play` prints the audio backend, device, callback counts and
queue depth, and offers two experimental trial options, `--experimental-audio-start-buffers` and
`--experimental-audio-period-frames`, which change only how finished sound reaches the device (presentation only; M5d left the defaults
unchanged, and nothing is promised to be shorter). The audio library's own `[INFO]` device report is printed only with `--audio-report`.
Since 2026-11-21, when at least one underrun was counted, one more exit line lists each of the first 32: the session of the launch, the
session's tics at the loop's last update before it, the frames of silence in that one callback, and how many mixer buffers had been
queued since the queue last began to fill, after the session's start, a resume or the underrun before. It is a diagnostic so that a run
shows whether its underruns are the start-up fill running dry (while playing, a few buffers after a start or a resume). A session that
has ended sends no more sound and its queue runs out once: that has always been counted as an underrun, is no gap in play, and the line
marks it. No run with the diagnostic has been reported yet, and it changes nothing that is played or counted.
The owner's first trial on 2026-11-20 was by ear on Bluetooth headphones with the sessions recorded to a slow disk, so its underrun counts
are contaminated by disk stalls. He reran three trials the same day on wired output with the sessions on an SSD, one run each, by ear
(not a latency measurement): a period of 128 frames sounded quicker than the default with no underrun, and a one-buffer start added
nothing he could hear and gave one underrun ([figures](validation/M5D.md#the-retest)). **He chose a period of 128 frames with the
two-buffer start as the future default; that change was not in M5d, which left the default at 512 frames.** Since 2026-11-21 (its own
pull request, [#67](https://github.com/Copgunn/DOOM/pull/67), which he merged that day) **the default period is 128 frames with the two-buffer start kept**; 256 and 512
remain available through the experimental option and are marked `EXPERIMENTAL` on the exit line. The figures, from six runs on one machine
and one device, by ear and by counters, are in the [M5d record](validation/M5D.md#the-default-period-becomes-128-frames-2026-11-21-its-own-pull-request). The WASAPI period count is not offered: the audio library's safe API
cannot set it. Their arithmetic and the one run on this machine are in the [M5d record](validation/M5D.md#a2-sound-latency-on-windows-statistics-and-trial-options);
the Windows runs are the owner's two trials of 2026-11-20 and his later runs at 128 linked above.

### Windows Smart App Control blocks the unsigned build

The Windows build from this repository's CI is not signed. On 2026-11-20 Smart App Control on the owner's Windows 11 machine refused
to start one build outright ("blocked by your organization's Device Guard policy"), although the build before it had started on the
same machine; he turned Smart App Control off to test. So on a machine with Smart App Control on, the downloaded `doom-play.exe` may
not start, and nothing in the program can change that. Why one unsigned build was allowed and the next was not is not known. Signing
the Windows build is on the [roadmap](ROADMAP.md) for the first release
([record](validation/M5D.md#a9-the-owners-retest-of-2026-11-20-his-rulings-and-the-close)).

### Stuck fire on Windows: accepted as fixed in M5d (the cause was never reproduced)

**The owner's report (2026-11-18):** on Windows, with the native artifact and `--bindings modern`, firing with the left mouse
button while dragging the mouse leaves fire stuck on; a click with the mouse held still clears it; firing with right Ctrl never
sticks.

**On Linux it does not reproduce** in the app's event layer or pump: every order of press, motion and release tried (an exhaustive
run of 2 x 50,625 four-pump sequences among them) delivered every release to the game, because motion never enters the queue of
button changes (`crates/doom-game/src/input/layer.rs:187-192`, `259-336`, `248-256`).

**The cause, from the owner's retest log (2026-11-19, `doom-play --input-log`, the `b141767` artifact, display scaling 125%;
other scalings were not tried).** *Shown by the log:* the summary line read `raw downs/ups: Left 18/15`, so three releases never
reached doom-play; its own layer passed on everything it received; each loss followed a rightward drag during which the window
library stopped reporting cursor positions while raw motion continued, and in one case the next press resized the window.
*Inferred, not shown:* at 125% scaling the hidden cursor is not held inside the client area and comes to rest on the window's right
border; the window library never captures the mouse, so a release made there goes to the window's frame, which the library does not
handle ([evidence record](validation/M5C.md#the-owners-windows-retest-with-the-input-log-2026-11-19)).

**Second round (2026-11-20):** the owner's session on the `b374aaa` artifact showed the launch capture failing in two of five launches
(the first launch after a change of display scale, in both) and three releases lost at 100%; the clip, read from the window library's
source and not reproduced, is applied once per grab and nothing re-applies it. On Windows only, doom-play now asks for the grab again on
a button-down, on gaining focus and when it infers from the logs that the cursor has left the window (a detector that is an inference,
scored against his logs in the [record](validation/M5D.md#a7-the-owners-windows-session-of-2026-11-20-and-the-second-round)).

**The retest (2026-11-20, the `c9f54ee` artifact):** nine launches at 100%, 125% and 150%, three of them the first launch after a
change of display scale, and one two-minute run; 259 presses, every release delivered, no resize line, and fire never stuck
([figures](validation/M5D.md#the-retest)). **The owner accepted the fault as fixed.**

**Status: accepted as fixed by the owner on 2026-11-20; no longer listed as a known fault. What is not shown:** the cause of the
launch-capture failure was never reproduced, here or on his machine, and nine clean launches cannot show whether the first grab would
have failed without the re-grab. doom-play asks for DPI awareness on Windows (`Conf::high_dpi` in `crates/doom-play/src/window.rs`) and
asks for the grab again; it still has no mouse capture, so a release can still be lost in the cases listed in the
[M5d record](validation/M5D.md#a1-stuck-fire-on-windows-the-dpi-request). It was tried on one machine. If fire does stick, a click with the
mouse still and the cursor inside the window clears it, right Ctrl fires without the fault, and a `--input-log` file of the session
shows what happened. **Sessions recorded with
the fault are still valid demos:** the demo records the commands the game executed, and the game was told the button was still
down, so a replay shows the stuck fire as it was played (a valid record of what happened in the game, not of what the hand did); the
owner's session passes the genuine comparison.

**`doom-play --input-log FILE`** (off by default, listed in `--help`) writes one line for every raw input the window library
delivers, for what doom-play passes on and for every event sent to the game, each with the loop's pump number and the tic, and a
summary of raw button downs and ups on quit. A test shows it changes neither the game, the demo nor the frames.

## Known gap: THING loading stops at the first Doom II type outside commercial mode

`P_LoadThings` (linuxdoom-1.10/p_setup.c:317-337) tests, when `gamemode != commercial`, each THING's type
against ten Doom II types (64 Archvile, 65 Former Human Commando (the Chaingunner), 66 Revenant, 67 Mancubus,
68 Arachnotron, 69 Hell Knight, 71 Pain Elemental, 84 Wolf SS, 88 Boss Brain, 89 Boss Shooter). A match sets
`spawn = false`, and the `break` at line 337 then ends the **whole** loop: that THING and every later THING
of the map are not loaded. It is a truncation, not a skip. Rust spawns every admitted THING in the lump.

**Census (observed in the code; no IWAD read for it).**

| Of the ten types | Admitted at load by |
| --- | --- |
| 65 | Wires 12-24 (every policy with former humans: `PlayerMonstersV1` to `PlayerMonstersEpisode1V2`; `EPISODE1_MONSTERS` in `episode1.rs`, `is_policy_monster` in `original_registry.rs`) |
| 64, 66, 67, 68, 69, 71, 84, 88, 89 | No wire. They are not former humans, Imps, Demons or Spectres, pickups, decorations, barrels or the landing, so map loading refuses them with `MapSpawnError::UnsupportedType`. (Type 69 is not the Baron, which is 3003.) |

Wires 1-11 admit no former human; the pre-wire profile 16 admits only the Demon. Other numbers in the table
above (for example 2001 or 14) are not among the ten.

**Game mode of each wire's genuine harness.** Wires 22, 23 and 24 are compared with the whole-engine
runners (`m3h9c_driver.c`, `m4a_driver.c`, the latter reused by M4b), which set `gamemode = registered`
(d_main.c:697-701, `doom.wad`) and load through `P_SetupLevel` and `P_LoadThings`: there the genuine engine
**does** truncate at a type-65 THING, and Rust would not. The earlier wires' genuine harnesses
(`m3c_driver.c` and the generators built on it, wires 1-21) default to `gamemode = commercial` and call
`P_SpawnMapThing` per THING (or `P_LoadThings`, in M3d3j, M3g2 and M3g7, under commercial), where the check does not
apply; their evidence therefore does not exercise the gap, and a registered or shareware engine would
truncate where they spawn. I read the drivers and the harness generators' function lists, not every case of
every earlier harness.

**Status: unpatched by rule** for every affected frozen wire, 12-24 (the wires that admit type 65), because
historical profiles keep their behaviour. **No Episode 1 IWAD map contains any of the ten types.** The M3h9 note
([§2.2](M3H9_REFERENCE_INVESTIGATION.md#22-p_setuplevel-and-p_loadthings-p_setupc)) recorded the census for
E1M1; for E1M2-E1M9 it follows from the wire 22-24 evidence rather than a new count: wires 23 and 24 refuse
types 64, 66-69, 71, 84, 88 and 89 at load, wire 24 loads all nine maps, and every scripted real-map run matched the genuine
registered engine tic for tic, which a truncation at a type-65 THING would have broken
([M4a census](M4A_REFERENCE_INVESTIGATION.md#2-episode-1-census-medium-skill) for the admitted monsters). So no recorded
comparison is affected. A map with a type-65 THING under a registered or shareware engine, for example a
synthetic one, would diverge. The fix is
closed under wire 25 (below); wires 12-24 keep the gap.

**Closed under wire 25 (decision T4, 2026-11-18).** Wire 25 breaks at the first of the ten types before the start
handling, as the original does, and for no other change of behaviour. One synthetic map (`tests/reference/m5c_t4`,
E1M1, things in lump order: the player start, a zombieman, a type-65 Former Human Commando, a second zombieman and a
stimpack) shows it against the genuine engine, which loads 2 mobjs: wire 25 matches on all 93 tics, and wire 24
diverges at the load capture (DSIM tic 0, first at `rng.gameplay`, with 5 actors at load, because its three extra
spawns each draw a random number for their first state's duration, p_mobj.c:787-788)
([observations](validation/M5C.md#the-thing-loading-truncation-decision-t4)). The wire 25 simulation is the only one
that truncates; for wires 12-24 the gap above stands unchanged.

## Known gap: the player's hard-death scream in the sound events of wires 22-24

In the original, `A_PlayerScream` plays `sfx_pdiehi` for a player death below -50 health only in commercial mode
(linuxdoom-1.10/p_enemy.c:1994-2007); otherwise it plays `sfx_pldeth`. The Rust simulation's sound event, in wires
22-24, names `sfx_pdiehi` in every mode (`PlayerScream` arm of `crates/doom-sim/src/lifecycle.rs`). Only that event's
sound number is affected: the game state, positions, random numbers and every frame are as the original's. The app
plays sound only from wire 25, which corrects it, so no build plays the wrong scream (wires 22-24 produce events but no
app turns them into sound).

**Status: a known gap, unpatched by rule.** The event is part of the canonical bytes of wires 22-24 and stays as it is,
because historical profiles keep their behaviour. It was found by reading the code on 2026-11-18. **Wire 25 corrects
it** (it plays `sfx_pldeth`, as an Episode 1 game does; decision T3). No run reaches the case: the 61 real Episode 1
runs and the 64 synthetic items contain no hard player death, so the replacement count is 0 on all of them and the
correction is tested by a unit test.

### The sound events of wires 22-24 were never compared with the genuine engine

Said plainly, as the owner asked: **until wire 25, no whole-engine run had ever compared the simulation's sound events
with the genuine engine, and the events of wires 22, 23 and 24 themselves still have not been compared directly.** What
was done is this. The first genuine comparison was made on **wire 25's records**, over 64 synthetic items and 61 real
Episode 1 runs, and wire 25's events equal wire 24's apart from the scream on every one of those captures (Rust against
Rust, [above](#original-episode-1-part-3-canonical-sound-records-wire-25)). So for those inputs the comparison is evidence
about wire 24's events as well, and **it found no fault beyond the scream: the count of frozen-wire sound faults stands
at one.** Wire 22 and 23 events have not been checked against wire 25 or the genuine engine on sound. Every further fault
a later comparison finds is to be listed here, left unpatched in the frozen wires and fixed in wire 25 only.

What the comparison covered, and did not:

- **Compared:** for every start, its order, its tic, its sound number, its origin's kind and identity, and, for the
  starts that carry positions, the origin and listener positions and the listener's angle; for every stop, its tic and
  identity. Every capture of the run was also compared outside sound.
- **Not compared:** for the 381 starts of the 64 synthetic items that carry no position (a NULL origin or the player's
  own body: 785 starts, of which 404 carry positions), the genuine engine writes the listener's position and angle,
  and the wire-25 record does not carry them. Those three columns are not compared, and the original does not read them
  there (s_sound.c:304-324). The same holds for the 5,980 such starts of the 61 real runs (26,077 starts less 20,097 with
  positions). Every other column of those starts is compared.

