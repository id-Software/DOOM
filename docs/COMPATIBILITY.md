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
| Respawn/raise | Explicitly unsupported; no Nightmare respawn or arch-vile resurrection |
| Complete original actor | Deferred: native type 136 identity, state aliases and disabled raise remain adaptations |

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
sight only. Player binding, typed actors, map spawning, AI/hearing, combat/hitscan,
sliding, dynamic sectors, specials and complete game/map admission remain excluded.
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

Full player damage/inventory/armor/powers/scoring, multiplayer, Fast/Nightmare
execution, respawning, resurrection, line and sector specials beyond the static set of
the [specials policy](#original-monsters-with-sky-and-static-specials) and the seven
door, lift, floor, switch and exit types of the
[movers policy](#original-monsters-with-doors-lifts-floors-switches-and-the-exit) (with
two-sided scrolling walls under the
[E1M1 policy](#original-monsters-with-two-sided-scrolling-walls-and-static-decorations)), complete
map/actor spawning, generalized monster interactions, demos and presentation remain
unsupported. Spectre metadata recognition is not Spectre gameplay compatibility.
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
gameplay permission. Spectre gameplay, real
players, infighting, resurrection and Nightmare/respawn remain deferred.

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
not raw original IDs in ActorLife/DSIM. Full actor compatibility, real players,
generalized infighting, Nightmare/respawn, resurrection, dynamic sectors, new
hitscan/projectiles and rendering remain deferred. See the
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
Automatic targeting and physical Demon melee against the player remain future work.

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
C observations, reused history and Rust-only safety policies. Full player controls,
PlayerThink, roster/automatic Demon attacks, view/camera, weapons, inventory,
armor/powers, air movement, respawn and multiplayer remain unsupported.


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
PlayerThink, P_Ticker, P_DeathThink or player roster execution. View/camera/bob,
weapons/buttons, inventory/powers/cheats, special sectors, airborne controls,
respawn/multiplayer, arbitrary angled/two-sided walls and automatic Demon
perception/acquisition remain unsupported. Dying/corpse Demon origins retain
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

Full P_PlayerThink/P_Ticker/P_DeathThink, view/camera, weapons, inventory, powers,
armor/cheats, roster changes, respawn/multiplayer, generalized targeting/infighting,
other monster species, sound propagation, arbitrary geometry and autonomous combat
remain unsupported. [Source findings](M3G5_REFERENCE.md) and
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

Other weapons, switching, pickups, armor/powers/cheats, respawn, multiplayer,
other monsters, infighting, full player thinkers, airborne/general geometry,
dynamic sectors, generalized sound propagation and rendering remain unsupported.
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
records pickup fields/events; decoding never restores a game. Armor, powers,
keys, backpacks, other items/weapons, player rebirth, multiplayer, generalized
specials and dynamic geometry remain excluded. Source evidence, explicit harness
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

Soulspheres, MegaSpheres, powers, keys, backpacks, item respawn, dropped bonuses,
new weapons/monsters, multiplayer and dynamic geometry remain excluded. Physical
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

Raise/resurrection, MT_VILE and every other monster type, projectiles, nightmare
and fast monsters, respawn, multiplayer and dynamic geometry remain excluded.
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
needs overlapping THINGs. Every other projectile and monster, raise, nightmare and fast
monsters, respawn, multiplayer and dynamic geometry remain excluded. See
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
argument does not hold for them. Two-sided lines, height changes, moving sectors,
other monsters and projectiles remain excluded. See
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
`P_KillMobj` drop that would not fit fails its checked tic. Moving sectors, doors,
lifts, crushers, line and sector specials, damaging floors, teleporters, sky ceilings,
non-axis walls, `P_CalcHeight`/view bobbing and every other monster or weapon stay
excluded. See [source investigation](M3H4_REFERENCE_INVESTIGATION.md) and
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
or whose center lies outside rejects with unchanged state. Moving sectors, doors, lifts,
crushers, line and sector specials, damaging floors, teleporters, sky ceilings,
`P_CalcHeight`/view bobbing, cap increases and every other monster, weapon or item stay
excluded. See [source investigation](M3H5_REFERENCE_INVESTIGATION.md) and
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

No admitted line is a crusher; crushers are deferred, not emulated. The Stage A sector
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
  out of scope.
- **Reborn.** Under this policy a dead player may hold `BT_USE` alone (the only dead
  command besides the empty one). That tic completes, the player state becomes
  `PST_REBORN` (DSIM35 `original_player.0.state` 2), and every later `advance` or `step`
  returns `SimError::PlayerReborn` with the state unchanged (decision N3). The level
  restart is out of scope. Both refusals are checked before the pause test.
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

Still excluded: every other line special (locked doors and keys, the other door, lift,
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
the MAP30 stomp (teleporters out of scope). Known limitation of the C evidence: the
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

**Status: not merged; see [ROADMAP](ROADMAP.md#m4a-episode-1-part-1-proven-against-the-genuine-engine).** The command-domain
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

**Status: in progress (M4b, plan steps 1-14 done, step 15, the full gates, next; see the
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
tic on 61 scripted runs over E1M1 to E1M9 on skills 2 and 3, and on `DEMO1` run by linuxdoom-1.10
(a match with that engine, not with the DOS 1.9 executable that recorded the demo); other inputs are
not claimed ([evidence record](validation/M4B.md#real-maps-and-demo1)).
The observations (the 30 M4a synthetic scripts and the M4b synthetic scripts compared with the
genuine engine under wire 24, and the local real-map runs) are in
[the evidence record](validation/M4B.md); the commands are in
[TESTING](TESTING.md#m4b-synthetic-mechanism-wads). Rust-only tests are in
`crates/doom-sim/src/player_episode1_v2_tests.rs`, `crates/doom-sim/src/teleport_tests.rs` and
`crates/doom-sim/src/baron_tests.rs` and `crates/doom-sim/src/hard_skill_tests.rs`; `crates/doom-sim/tests/episode1v2_mechanism_goldens.rs` replays the admitted M4b scripts against
the goldens and fixtures in `cargo test`.
