# Architecture

Rust DOOM separates validated input, owned authoritative simulation and eventual
presentation. Compatibility comes from original-source investigation and independent
comparisons within explicitly selected simulation configurations, rather than
mechanically copying C globals. The [glossary](README.md#short-glossary) explains
terms such as profile, fixture and admission.

## Crate boundaries

| Crate | Responsibility |
| --- | --- |
| [`doom-types`](../crates/doom-types/src/lib.rs) | Fixed-point arithmetic, original RNG table and owned stream indexes |
| [`doom-wad`](../crates/doom-wad/src/lib.rs) | Bounded WAD directory/resource access without assuming assets are executable |
| [`doom-map`](../crates/doom-map/src/lib.rs) | Validated classic map records, BSP, REJECT and source BLOCKMAP data |
| [`doom-sim`](../crates/doom-sim/src/lib.rs) | Headless deterministic gameplay profiles, actors, collision and canonical diagnostics |
| [`doom-render`](../crates/doom-render/src/lib.rs) | The faithful software renderer for the 3D view (M5a); reads a simulation, never writes it ([below](#software-renderer-m5a)) |

The preserved [original source](../linuxdoom-1.10/) is behavioral reference, not a
runtime dependency. The Rust workspace forbids unsafe code and global mutable game
state. Each simulation owns its RNG, actors, counters and ordered observations.

## Deterministic time and numbers

Simulation advances one logical tic per successful unpaused call at 35 tics/second.
There is no wall-clock pacing in authoritative gameplay. Fixed-point operations
and binary angle measurements (BAM) use explicit integer semantics. Source-defined
wrapping and checked Rust rejection are documented separately. Gameplay and
miscellaneous RNG streams have independent owned indexes. Presentation cannot consume gameplay draws.

## Stepping and observation

`Simulation::advance` owns the single stepping implementation and returns unit.
`step` delegates to it, then captures one Snapshot. `snapshot` captures explicitly;
encoding and digesting are further explicit work. Paused advance calls do no tic
work or capture; paused step calls retain their historical capture. Events remain
recorded and observable on both paths. `run_commands` still collects snapshots.
`ExecutionContext::advance` delegates to that core, also through GameInstance.

Dynamic snapshot fields allocate names/value buffers and serialize events; map
fields and historical registry fields are cached and shared. Checked-Demon staging
separately copies bounded actor slots/spatial order and small owned state while
borrowing geometry/definitions. Recoverable late failures still discard that
projection in release. There is no unchecked fast mode or allocation-free claim.
The [cost inventory](M3G1_REFERENCE.md) separates these operations; measured costs
and limitations belong in [the evidence record](validation/M3G1.md).

## Actors, lifecycle and ordering

A generational actor handle identifies a slot within one simulation. Reusing a slot
cannot redirect an old target/source to its new occupant. Thinker insertion order,
slot identity and spatial head-insertion order are separate contracts. Source
BLOCKMAP ordering is retained; sorting handles or rebuilding BLOCKMAP would alter
collision decisions.

The original-ID profile also exposes an exclusive `ExecutionContext` callback.
Invariant lifetime-branded `ContextActor` values cannot escape or mix between
callbacks. They carry the existing generational handle, not another definition or
owner ID. Every access still validates current storage; tokens are not liveness
leases. Raw handle admission explicitly interprets a candidate locally and cannot
infer foreign provenance from identical numbers. This preserves legacy APIs and
diagnostics while providing a checked typed boundary for new callers.

`ActorAvailability` derives current, pending, retired, stale and invalid states
from existing slots. Pure target observation keeps stored identity distinct from
availability, health and role. Target mutation, damage, perception repair, spawn,
state entry and stepping delegate to the existing services and target-role guard.
No actor store, scheduler, reference sweep or persistent identity field is added.
Each operation retains its atomicity; the callback is not an aggregate transaction.
See the [contract](COMPATIBILITY.md#context-and-target-lifetime-contract) and
[source investigation](M3F8_REFERENCE.md).

### Game-instance provenance

`GameInstance` owns one unchanged Simulation and one opaque `InstanceId`. Its
key compares private Arc allocation identity; retained tokens prevent aliasing
with a newly allocated incarnation. Moves preserve identity. Wrapping cloned or
extracted simulation state creates a fresh identity; the owner itself is not
Clone and exposes no mutable Simulation escape. Successful restart installs the
existing fresh level state and a new key together. Setup failure retains both.

`InstanceActorRef` carries optional opaque provenance and the existing local
handle, without retaining actor/world storage. Explicit checked export/import use
the same `ExecutionContext` and invariant `ContextActor` brand. Bound scopes borrow
the owner's key; raw Simulation scopes remain unbound. Import rejects missing or
foreign provenance before existing generation/state/expected-definition checks.
Actors, relationships, collision, scheduling and ordinary tics still use local
handles; no per-actor identity mirror or per-tic provenance lookup is introduced.

This is in-process uniqueness and consistency, not authentication or persistent
identity. Explicitly attaching a trusted local key to an erased candidate is local
readmission, not recovery of its origin. There is no byte/numeric InstanceId
constructor, savegame loader or transport resolver. Keys/refcounts are excluded
from snapshots, events and RNG. See [support/trust rules](COMPATIBILITY.md#game-instance-provenance)
and [the complete audit](M3F9_REFERENCE.md).

State entry installs frame/timer and executes its action immediately, including
nested Look-to-Chase or Chase-to-melee transitions. Raw spawning initializes state
without executing its action. Damage, death state entry, delayed sounds, Fall and
terminal corpses compose the same lifecycle machinery. A corpse retains its actor
and thinker identity; completion of its animation does not imply retirement.

## Map context

Each simulation owns its level statistics alongside actors and RNG. Map placement
and shared damage/lifecycle services update those counters; there is no separate
statistics world, actor store or scheduler. Available map kills and credited deaths
have distinct ownership and meaning.

Fresh restart builds a new simulation through the existing constructor and THING
batch, keeping the old instance intact on failure. Handles remain scoped to their
own simulation. [COMPATIBILITY](COMPATIBILITY.md) defines the supported reset and
statistics behavior; the [source investigation](M3D3J_REFERENCE.md) explains exact
chronology, and [STATE_FORMAT](STATE_FORMAT.md) owns the encoding.

## Validation and failure atomicity

Immutable map/registry data and small, bounded plans validate roles, references,
capacity, reachable actions, arithmetic and source dependencies before changing state.
Damage uses one shared plan/execution service. Multi-Demon melee evaluates the
current body health and RNG after earlier thinkers; it does not reuse stale plans.
There is no full-simulation clone per tic. Setup may construct a bounded fresh
instance, and the generic user thinker budget has a separate nontransactional
contract from built-in simulation operations.

[DSIM](STATE_FORMAT.md) serializes canonical fields and ordered diagnostics in
versioned schemas. Historical profiles remain frozen. Source-C equivalence,
independent encoding and Rust-specific safety tests serve different purposes;
see [validation](VALIDATION.md).

## Bounded larger geometry

The opt-in map-scale configuration reuses `StaticWorld`, sector state, the actor
store and existing movement/sight services. Its capacities and excluded gameplay
are owned by [COMPATIBILITY](COMPATIBILITY.md#map-scale-profile).

Preparation compiles each supplied cell into a vector of linedefs in original
first-occurrence order. Reverse word-position lookup handles shared lists/suffixes
without repeated raw scans. Sorting preparation metadata by original word position
does not sort gameplay candidates by line ID. Checked total-reference accounting
and fallible reservations precede installation; source BLOCKMAP is never rebuilt.

Preparation membership is a multiword bit matrix sized from actual cells and
lines. Query-local deduplication uses a bounded multiword stack set; membership
bits never choose traversal order. Topology retains source first-seg sector
ownership and indexed sector vectors. The existing spatial index preserves
head insertion independently from thinker order.

Preparation has separate storage and CPU bounds. Let L be linedefs, C cells, W raw
words, K distinct list starts, T terminators and E compiled references. Reverse
lookup, line-marker resets, per-start ordering and coverage diagnostics take
`O(W log C + T*L + K*L log L + E + L*C)` work in the conservative worst case.
Temporary membership/storage is `O(C + L + E + C*ceil(L/64))`. The historical
algorithm already reset/scanned its fixed line table; increasing the admitted line
count raises that work ceiling. All dimensions are separately capped in
[COMPATIBILITY](COMPATIBILITY.md#map-scale-profile). The reference budget bounds
list expansion/copying, while raw-word and line caps also bound unused-terminator
work. Avoid treating one budget as a bound on every stage. A query's membership
check is constant-time and never chooses candidate order. These bounds describe
collision/topology preparation; parsing and canonical-map cache construction add
their separately bounded decoded-table work.

Arithmetic admission runs before authoritative movement. The new untyped profile
explicitly selects the earlier static-sector canonical field layout under a new
version tag; numeric version order cannot implicitly enable typed actors. Immutable
preparation budgets and derived storage metrics are not redundant persistent actor
state. Snapshot fields remain defined only in [STATE_FORMAT](STATE_FORMAT.md).

## Expanded arithmetic and spatial work

The separate map-extent profile adds immutable admission analysis to the same
world, movement, spatial and sight services. [COMPATIBILITY](COMPATIBILITY.md#map-extent-profile)
owns its exact domains and resource ceilings. [M3F2_REFERENCE](M3F2_REFERENCE.md)
derives them from original source expressions, including intercept sums; representable
classic coordinates alone do not establish arithmetic safety.

World coordinates, pairwise differences, line-delta norm, node deltas, heights,
actor margins and momentum have separate checks. Those checks make overflow in
reachable original arithmetic unnecessary; wider temporaries are not used to
invent results for overflowing C. Existing wrapping numeric primitives retain
historical contracts. Original FixedDiv guard/rounding, BSP ties and XY/Z action
order remain unchanged. A per-query check additionally rejects a saturated
INT_MIN intercept before the original clipping division could evaluate abs(INT_MIN).
The source-side quantization trigger is recorded as a defined C component; its
undefined continuation has no fabricated equivalence result. Historical numeric
policies remain unchanged.

Setup conservatively charges native-size preparation requests and a collision
work proof before constructing a simulation. Supplied BLOCKMAP cells still compile
into source first-occurrence lists; membership bits never choose traversal order.
Cells, raw words, expanded references and memory remain independent dimensions.
The preparation complexity above still applies to the expanded cell domain.

Sight memoizes subtree costs but adds both child references, so a shared DAG counts
its full expansion. A derived root cost is cached in the immutable world and used
by the query work assertions and fallible scratch reservations together. Its
observation bound is root cost plus five records per unique linedef; the task
stack is bounded by node count. REJECT needs neither scratch nor traversal. These
caches and limits are not canonical actor state.

Spawn/control/query validation and an all-live-actor tic preflight precede RNG,
event consumption and movement. Public world BSP queries enforce the new domain
too. No full simulation copy or second physics/actor/spatial service is introduced.
Historical profile selection keeps its old limits, and the new untyped canonical
tag cannot implicitly enable gameplay fields. Allocation exhaustion for existing
tic diagnostics/snapshots retains its documented process-level policy.

## Expanded-map Demon composition

Profile 20 selects the existing geometry preparation and the native map-context
Demon registry together. Capability helpers express map statistics, expanded
geometry and the established perception/monster/lifecycle closures. Numeric
profile order never selects these services implicitly. The constructor validates
their arithmetic intersection and complete registry before returning an instance;
[COMPATIBILITY](COMPATIBILITY.md#expanded-map-demon-profile) owns the exact bounds.

There is one authoritative actor store and one scheduler. The original built-in
thinker pass is shared in `thinkers.rs`; both historical execution and the new
profile call the same movement and LifeHost services in the same linked order.
A new operation stages at most 64 slot records (including generations and links),
64 spatial handles, RNG, statistics, bindings and new observations. It borrows the
immutable world, sectors, registry, roster, sound references and canonical cache.
It does not copy a Simulation, large geometry or previous event queues.

This small actor-state plan runs once and commits only after success. The same
ThinkerStore type supplies traversal and retirement within the temporary plan;
there is no second persistent arena or AI scheduler. Later native strikes plan
against the current staged target health/RNG, including earlier owners' damage.
Public direct Look, Chase, state entry, facing, melee and damage share the boundary.
Damage needs it because source retargeting can enter Chase immediately after a
failed pain decision. Pure player scanning gathers checked results before mutation.

Every new AI sight path uses fallible expanded sight, including the INT_MIN guard.
A transient error latch unwinds void source-style callbacks and stops later work;
it is always checked before commit. No fabricated false visibility result is
committed. A later actor's failure discards staged movement, RNG, retirement,
damage, counters and observations. Original commands and both event queues remain
untouched. Layout 20 serializes actual typed state/statistics, not the plan, latch,
query budget or derived preparation data.

Batch spawning still resolves original DoomEd metadata before the native identity
adaptation and appends actors in THING order. Spatial heads remain most-recent-link
order; generational identity never selects a blocker. Restart remains a setup-time
new-instance operation, retaining RNG and rebuilding through the validated constructor.
[The investigation](M3F3_REFERENCE.md) records arithmetic/closure assumptions and
[the evidence](validation/M3F3.md) records their independently exercised limits.

## Rendering direction

A custom Vulkan-first presentation architecture remains a future direction (see
[ROADMAP](ROADMAP.md)); no Vulkan or wgpu runtime exists. What exists is the faithful
software renderer below. Any presentation must consume simulation results without
changing time, ordering or RNG.

### Software renderer (M5a)

`doom-render` depends on `doom-types`, `doom-wad`, `doom-map` and `doom-sim`; nothing
depends on it, so the simulation cannot reach presentation code. It reads the simulation
only through its public `&self` API and `Simulation::snapshot()` (the runtime floors,
line specials and button slots the simulation keeps only in its canonical snapshot), so
`doom-sim` was not edited for it. A test compares a run with and without rendering tic by tic.

- **Shape.** `Renderer::begin_level` at tic 0, then after every committed
  `Simulation::advance` call `observe` and `render` into a `Frame` (320x200 palette
  indices). Resources (`r_data::Resources`) come from the caller's lump list with the
  original's lookup semantics. `RenderConfig` selects screenblocks 9-11 at high detail;
  everything else is a typed `RenderRefusal`, which leaves state unchanged.
- **`RenderState` and `DRST`.** What the original renderer carries between tics and the
  simulation does not keep (the view shadow that re-derives `viewz`, the switch tracker,
  the mapped-line set, `fuzzpos`, the r_segs.c wall-step globals, the persistent framebuffer
  and the undefined-read taint)
  lives in one self-contained value that serializes as [`DRST`](STATE_FORMAT.md#render-state-drst-version-2-m5a),
  separate from DSIM. Saved next to a DSIM snapshot, it lets a rewind render any tic directly.
- **Layout.** One module per C file, named for it: `r_main`, `r_bsp`, `r_segs`, `r_plane`,
  `r_sky`, `r_things`, `r_draw`, `r_data` and `p_spec` (the switch list and scrolling-wall offset, from p_spec.c and p_switch.c), plus
  `tables` (generated from `tables.c` and checked), `view` (the view shadow), `state`, and
  hand-written `png` and `gif` writers. Renderer arithmetic wraps as the original's does
  (a presentation-only exception to the simulation's checked arithmetic).
- **Harness and tools.** The genuine renderer is compiled whole into a separate runner
  (`tools/reference/m5a_*`) that dumps a frame stream; goldens and PNGs for the synthetic rooms
  are committed, and `tests/frames.rs` compares the Rust frames in CI. The real-map comparison
  (`tools/local/m5a_real.py`), the GIF viewer example and two fuzz targets are developer tools,
  not part of the library. See [support](COMPATIBILITY.md#software-renderer-m5a) and
  [commands](TESTING.md#m5a-synthetic-renderer-rooms).

For detailed historical design constraints see the complete
[pre-M3d3j architecture](validation/archive/pre-m3d3j-architecture.md).

## Canonical metadata and registry identity

The [identity API](ACTOR_IDENTITIES.md) borrows existing immutable source tables.
Private bounded canonical IDs are distinct from registry-local candidates and
from generational live actor handles. Explicit conversions compare a complete
record at its original slot. They certify that record only; downstream registry
records and executable game context still require their existing admission.
No alias is silently promoted, no second registry/store is introduced, and queries
allocate nothing and do not mutate simulation state. Profiles 1–20 retain their
existing identity encodings, state chronology and RNG contracts.

## Canonical Demon execution binding

`DemonExecutionBinding` is a borrowed proof connecting canonical MT_SERGEANT and
its normal SARG closure to Profile20's existing native definition. Admission
checks full canonical records through M3f4 resolution, then checks every bound
native descriptor and translated next edge. An explicit bounded bijection keeps
canonical definition identity distinct from registry slot identity. No second
actor store, persistent identity field or scheduler is added.

The map-context fixture now derives its native records directly from canonical
metadata, preserving its previous bytes. Profile20 construction certifies the
binding. `canonical_demon_map`, `spawn_original_demons` and
`enter_original_demon_state` delegate to existing constructor, batch and state-entry
services; existing role/health/arithmetic checks still apply. Definition queries
are immutable, allocate nothing and consume no RNG. Their bounded validation cost
is paid only by explicit requests, outside ordinary per-tic execution.

Native actor/state aliases remain serialized. Canonical observation explicitly
projects a current owner record through the verified binding; M3f4's same-record
resolver continues rejecting native136 as pristine source metadata. Source raise
metadata remains intact while native raise is suppressed. See the
[mapping](ACTOR_IDENTITIES.md#canonical-demon-execution-binding) and
[investigation](M3F5_REFERENCE.md) for the narrower integration boundary.

## Runtime identity representation boundary

M3f6 keeps ActorLife's registry-local type/state fields authoritative. The
[complete audit](M3F6_REFERENCE.md) distinguishes definition IDs, execution slots,
DoomEd numbers and generational object handles. Persisting both canonical and
native IDs would introduce synchronization invariants without solving historical
observation compatibility; no such fields are added.

Sealed native Demon views make the existing representation explicit. The borrowed
binding still proves metadata; scalar views do not certify another registry, a
live handle or state-entry permission. Existing canonical observations delegate
through the native view. One mapping supplies projection and a derived bounded
reverse lookup; queries allocate nothing. Profile20's entry/death/Fall guards use
named states through constant-time reverse lookup, without full registry validation
per tic. Earlier fixture profiles retain their policies. Thinker, spatial, RNG,
transactional planning and serialization services remain shared and unchanged.

## Original-ID registry and shared execution

The [original-ID registry](ACTOR_IDENTITIES.md#original-id-execution-registry)
certifies an immutable source metadata package and a separate context package.
Source ordinals are direct indexes; a typed definition key prevents the context
slot from becoming a fake original identity. ActorLife remains the sole persistent
identity storage. Only explicitly admitted roles execute, through existing
LifeHost, DamagePlan, ThinkerStore, movement/perception and checked Demon plans.
No second scheduler, actor store or per-monster native translation table is added.

Original-root state sets and action policies are derived once with bounded work.
They preserve damage-controlled entry and exclude raise without modifying its
metadata. Construction certifies all records; state reads do not repeat full table
validation. Historical profiles retain native definitions and guards through a
small explicit profile/role boundary. The [pre-implementation investigation](M3F7_REFERENCE.md)
records alternatives, consumer responsibilities and adapter removal criteria.

Immutable source/context schema IDs replace redundant registry rows in new
DSIM-v21 diagnostics. Complete runtime/map/event order remains observable.
The bounded decoder borrows values, validates identity namespaces and generations,
and never restores a Simulation. It explicitly dispatches v20/v21 and preserves
legacy interpretation. [STATE_FORMAT](STATE_FORMAT.md#dsim-v21--original-id-demon-diagnostics)
defines the wire contract and the limits of validation.

## Original player body ownership and damage

The [player-body policy](COMPATIBILITY.md#original-player-body-lifecycle) extends
the original-ID service family explicitly. OriginalExecutionProfile distinguishes
DemonNormalV1 from PlayerBodyLifecycleV1; the historical public Profile enum is
unchanged for exhaustive callers. The registry carries the selected capability
until construction. Simulation's optional OriginalPlayer record then identifies
the policy without another persistent actor-type or profile mirror.

PlayerSlot identifies the sole source player, independently of ActorId slots.
OriginalPlayer.mo references the existing generational actor owner; reverse
association derives from exact handle equality. A checked initial canonical0 body
binds once. ExecutionContext checks the current branded token and GameInstance
checks incarnation provenance before import. Existing command bindings, synthetic
perception records and historical WeaponPlayer remain separate contracts.

Source-required player health clamps0 while mobj health stays signed. Shared
DamagePlan/LifeHost updates player fields before actor health; death immediately
clears solidity, sets PST_DEAD and records the source weapon-lowering request
before body death-state entry. The small player record is staged and committed
with actors/RNG/statistics/events, so late failure rolls everything back. No actor
store, scheduler, damage engine, player-specific tic loop or per-tic diagnostic
capture is added. Existing contexts hold no player heap allocation; measured
scalar/branch and allocation effects belong in [the record](validation/M3G2.md).

Death leaves the actor allocated and bound; removal is explicit and deferred.
Stored mo/attacker retain their original handles after retirement, and availability
comes from the existing store. Generations prevent rebinding to a new occupant.
C quarantine/opaque-pointer observations and Rust generation safety are distinct.
In the stationary capability, physical inflictors and player inputs reject;
live Demon AI remains outside the historical unlisted player capabilities. Their
fixed unlisted context avoids pretending that PlayerThink, a roster,
weapon lowering animation or a death camera is implemented.

## Grounded original-player physical execution

The [knockback policy](COMPATIBILITY.md#original-player-grounded-knockback) reuses
the same body, shared damage service, XY/Z movement and thinker pass. A private
PlayerRuntime stores the immutable selected capability once beside OriginalPlayer
source fields. Public player observations are unchanged; ActorLife remains the
sole definition/state storage. There is no extra per-actor identity or owner.
Historical stationary policy remains independently selected.

Physical input validates source/inflictor independently and proves checked source
impulse accumulation plus a forward coordinate envelope before mutation. Actual
thrust still executes in LifeHost::apply_damage before player/mobj health changes.
The bounded actor/player/spatial/RNG/statistics/event plan preserves late-error
rollback. The proof uses wider temporaries for rejection, not wider gameplay
arithmetic or an alternative physics predictor.

In the knockback capability, exact player/mo association supplies a zero command
to the existing player slide
branch, independently of historical command bindings. The existing stop-state
hook handles RUN-to-PLAY before countdown. Grounded z/momz keep player view/landing
branches outside execution. Flat axis-wall restrictions make the conservative
travel proof forward invariant; unsupported geometry and direct XY control reject.
Map-spawned Demon origins retain their original admission, spawn RNG and statistics.
Physical hits require an already executable dying/corpse thinker closure; no active
player roster or automatic Demon AI is added. See [source/design](M3G3_REFERENCE.md).


## Original-player command stage

[Command capability](COMPATIBILITY.md#original-player-command-movement) admission
resolves the existing player/mo handle once before the shared checked actor plan.
The stage runs between Players and Thinkers phases before any linked actor. The
existing weapon interpreter and this capability share one checked motion/reaction
helper and root command_thrust integer implementation. LifeHost performs immediate
RUN entry; the same thinker pass receives actual commands for friction/sliding and
stopping. No second player loop, timer, actor store, health source or identity field
is introduced. The capability stays beside OriginalPlayer in PlayerRuntime.

The existing bounded DemonPlan stages actor slots, spatial order, RNG, player
fields, statistics and events while borrowing world/registry/sector data. Exact
motion runs once; a late travel postcondition rejects by discarding that plan.
Old policies retain their original phase construction and command guards. All
historical capacities, direct-state/physics controls and transaction guarantees
remain. Simple command interpretation adds no collection; measured complete-path
allocations, including existing staging/preflight work, are documented in
[the evidence record](validation/M3G4.md). Diagnostic decoding remains read-only.


## Active original-player roster and shared Demon AI

`PlayerDemonAcquisitionV1` is a distinct opt-in capability. Its execution context
fixes `playeringame` to active slot0 and inactive slots1–3. The existing
`OriginalPlayer` record and actor store supply current health, state and mo; no
second player record, mutable roster health cache or actor manager exists. A
transient roster selector borrows historical fixture records in old profiles or
derives the genuine player view from the current transaction at each Look/Chase
scan. Both paths invoke the same bounded source scanner. The derived public
`original_player_roster` observation does not grant permission to execute.

Setup appends the map-THING Demon before binding the genuine player. The existing
command/reaction stage precedes their shared thinker traversal; the earlier Demon
therefore sees the player's pre-XY position, even after command angle/momentum
updates. Immediate Look-to-see-to-Chase entry, target assignment, movement and
sounds use existing lifecycle, perception, sight and pursuit services. No separate
player-versus-monster loop, store, RNG or spatial scheduler is introduced.

Death preserves fixed activity, mo association, corpse allocation and retained
target/attacker handles. This capability rejects removal of its owned player body;
roster deactivation/respawn is unavailable. Other actor retirement/reuse still uses
existing generations. Current ownership and nonempty-roster support validate
before action/tic mutation. The same bounded transaction discards all staged
commands, movement, player fields, targets, statistics, RNG and events if chase
reaches the unsupported autonomous melee branch. There is no implicit Snapshot in
advance and no release-only rollback exception. [Support](COMPATIBILITY.md#active-original-player-and-demon-acquisition)
and [source findings](M3G5_REFERENCE.md) distinguish original behavior from Rust
lifetime/admission policy; [observations](validation/M3G5.md) report measured costs.

## Autonomous Demon-to-player combat

`PlayerDemonMeleeV1` composes the same fixed roster, command stage, original state
machine and thinker pass. The authoritative capability remains in PlayerRuntime;
its immutable value travels through the existing transient perception selector.
OriginalPlayer still contains only source player fields. No persistent actor or
player field, roster cache, lifetime manager or alternate simulation loop is added.

Chase selects the original attack state and sound. Two normal state timers later,
SargAttack faces and rechecks current range before drawing damage. Its checked
wrapper resolves the current owned body and live Demon from the staged actor store,
uses the shared physical impulse/travel proof, then the existing current-body
DamagePlan and LifeHost damage executor. No future hit, health or RNG is cached.
Range misses and absent targets never build a damage plan. A dead allocated target
can pass range and consume an attack draw before shared damage ignores it.

The existing bounded transaction carries late combat errors as well as sight and
command errors. Failure discards actor/player/RNG/statistics/spatial/event changes
before commit, including partially executed commands or damage; release retains
the same protection. Live Demon follow-through, player friction and corpse movement
use existing services. The acquisition-only capability keeps its old rejection.
See [support](COMPATIBILITY.md#autonomous-original-demon-player-melee),
[source matrix](M3G6_REFERENCE.md) and [observations](validation/M3G6.md).


## Canonical player pistol execution

OriginalPlayer remains the sole source player owner for mo, health, state,
attacker and lifetime. Under the pistol-only capability, PlayerRuntime selects its PistolState variant, containing only
bullets, attackdown, refire, bob, extralight and two psprites. Immutable ownership,
ready/pending pistol and capacities derive from the explicit capability. There is
no second actor handle, health record, player manager or persistent WeaponPlayer.

A transient adapter supplies the existing weapon Machine with current ownership
and health and returns only checked weapon fields. Historical WeaponPlayer and
its profile remain available unchanged. The shared interpreter now reads source
health/death through its host, so original lowering runs immediately at the
existing player-death boundary. Pistol damage/spread generation is shared with
historical weapons; immutable state metadata is borrowed rather than cloning its
String on each entry. No alternate weapon loop or RNG is added.

The existing Players phase runs the command/reaction gate once, then the shared
weapon interpreter; dead continuation runs only MovePsprites. The same actor
thinker traversal follows, including impact thinkers born during firing. Pure
trace collection/aim/hit algorithms accept borrowed current transaction actors,
spatial order, world and sectors. Historical wrappers retain endpoint rejection;
policy7 admits the original finite64-cell collection result even without endpoint
completion. No map or actor bounds expand.

Current hits use the shared PreparedAttack, DamagePlan, impact spawning and
LifeHost executor. There is no cached future damage, health, target position or
RNG outcome. The one-sector soundtarget is an inline transaction value and is
visible to the following thinker pass; it is committed with ammo, psprites,
actors, player, RNG, statistics, spatial order and both event queues. Earlier
profiles continue borrowing their unchanged sound-target representation. Late
errors discard the existing bounded plan, including initialization/fallback and
impact-capacity errors. No full Simulation clone or implicit Snapshot is added.

[Support](COMPATIBILITY.md#genuine-player-pistol-combat),
[source boundaries](M3G7_REFERENCE.md), [DSIM23](STATE_FORMAT.md#dsim-v23-genuine-player-pistol)
and [observations](validation/M3G7.md) separate source execution, schema projection
and Rust safety. Camera, other weapons, inventory gameplay and full player
thinking remain separate work.

## Shared original weapon expansion

PlayerRuntime holds one exclusive weapon-state variant: historical PistolState or
PlayerWeaponState for the four-weapon capability. The latter adds ready, pending,
owned and shell fields without duplicating mo, health, attacker or provenance.
A transient WeaponPlayer adapter invokes the same Machine and returns checked
weapon-only fields. There is no second persistent player, weapon interpreter,
actor store, combat engine or scheduler. Existing synthetic WeaponPlayer remains
its historical profile's representation.

The shared genuine-player host admits four weapons only under the new capability.
Original selection and psprites execute in the existing Players stage; original
actor thinkers follow. Current aim, line-hit, effect and DamagePlan services are
shared. Shotgun repeats the complete current bullet operation seven times after
one aim calculation, so earlier pellets can affect later damage/trace/RNG state.
Fist uses one melee aim and faces a struck target only after shared damage returns.
Current weapon state supplies death lowering. The transaction commits weapon,
player, actor, spatial, sound, RNG, statistics and events together. No Snapshot or
full Simulation clone is added to production stepping. See the
[support boundary](COMPATIBILITY.md#original-player-weapon-expansion) and
[source matrix](M3G8_REFERENCE_INVESTIGATION.md).


## Original world contacts and resources

The pickup capability stores canonical item type/state identities in existing
ActorLife records. Item kind derives from those definitions and validated source
sprites; no per-item kind mirror or new lifetime manager exists. Map admission
uses the same THING batch, with separate Demon/item budgets. Controlled dropped
setup uses the existing spawn and MF_DROPPED flag, with no map spawnpoint.

The shared Mover uses a statically dispatched MotionAccess contact hook. Historical
lookup closures use Infallible with an empty hook; their movement signatures and
algorithm remain. Pickup access borrows the current transaction's actor access
and PlayerRuntime. It uses the existing thing-candidate prefix before the existing
sector line check. Collection removes only nonsolid items and changes resources,
so it cannot invalidate the already ordered blocker/candidate decision. Contacts,
removal and sound events precede their own movement-result event even on a normal
blocked move. Shared slide/stairstep retries use that same hook; there is no
second pickup collision or physics loop.

OriginalPlayer remains authoritative for health/mo/state/attacker; the moving
actor copy and its stored mobj health are synchronized on a grant. The existing
weapon-only Pack record owns ammo/ownership/pending fields. PlayerRuntime adds
only optional bonuscount/itemcount/message fields, with no body/health duplicate.
One accepted item uses the same generational mark and spatial unlink operations;
existing thinker traversal retires it. The existing bounded DemonPlan stages these
fields together with actors, spatial order, RNG, sound, statistics and events.
Late failure discards all of them. No full Simulation clone, implicit Snapshot,
new allocator, global counter or second scheduler is introduced.

See [support](COMPATIBILITY.md#original-world-pickups),
[source matrix](M3G9_REFERENCE_INVESTIGATION.md) and
[DSIM25](STATE_FORMAT.md#dsim-v25-original-world-pickups).

## Test evidence and shared weapon arithmetic

The isolated `doom-test-evidence` crate is a test/tool dependency; ordinary
simulation stepping has no codec dependency or evidence-loading work. Its bounded
immutable cache replaces selected embedded raw trace text at test boundaries.
Frozen observers run unchanged in a copied decoded workspace. See
[evidence storage](EVIDENCE_STORAGE.md). Historical preview/commit and genuine
weapon execution reuse the same source-derived damage/spread helpers and their
existing RNG adapters; player ownership, state-machine services and transactions
remain authoritative. See [weapon formulas](WEAPON_FORMULAS.md).


## Original player armor

PlayerRuntime owns one optional armor-only record under PlayerDemonArmorV1.
OriginalPlayer and ActorLife retain their existing health/mo/lifetime ownership.
Armor item kind derives from canonical MT_MISC0/MT_MISC1, with the original
positive two-state animation clocks in the shared thinker pass. Profile-aware
role checks keep those actors out of historical pickup execution.

The existing contact hook calls the source points-threshold grant and stages its
message, removal, bonus and sound. DamagePlan retains the raw DamageRequest and
computes a single optional armor consequence from the current staged player at
actual damage time. Shared physical thrust still uses raw damage. The executor
then commits armor and uses effective damage for player/mobj health, damagecount,
tactile response and kill/pain decisions. Ignored damage has no armor consequence.
This is one damage planner/executor, not a player-specific combat engine or a
future-hit cache. All fields remain in the existing bounded transaction, including
late-error rollback; advance adds no Snapshot or world clone.

See [support](COMPATIBILITY.md#original-armor-and-damage-absorption),
[source matrix](M3G10_REFERENCE_INVESTIGATION.md) and
[DSIM26](STATE_FORMAT.md#dsim-v26-original-player-armor).

## Original health and armor bonuses

PlayerDemonBonusesV1 extends the armor capability rather than adding a new owner.
Bonus kinds derive from canonical MT_MISC2/MT_MISC3 and animate through the shared
thinker pass. The existing contact hook applies the +1 effects directly to the
OriginalPlayer health (synchronized to the moving actor) and the existing armor
record, then increments the existing pickup `item_count` before the shared
removal. Map totalitems lives in the existing LevelStatistics and is incremented
inside the atomic THING spawn batch. All of it stays in the bounded DemonPlan
transaction, so late failures discard RNG, statistics, player and spatial changes
together. No second statistics store, inventory or scheduler is introduced.

See [support](COMPATIBILITY.md#original-health-and-armor-bonuses),
[source matrix](M3G11_REFERENCE_INVESTIGATION.md) and
[DSIM27](STATE_FORMAT.md#dsim-v27-original-health-and-armor-bonuses).

## Original former humans and mixed rosters

PlayerMonstersV1 extends the bonus capability; no new owner, store or scheduler
is added. Former-human identities are ordinary original-ID registry roles whose
closures are derived from source roots. The single `LifeHost::action` table
routes A_PosAttack, A_SPosAttack, A_CPosAttack and
A_CPosRefire to one current-state implementation under this policy. Historical
fixture profiles keep their preflighted single-owner plans in the same table.

A current attack faces, aims and traces against the staged transaction at action
time, uses the shared `weapons::monster_shot` formula, plans damage with
`DamagePlan::current_original` from the target's current health, armor and
roles, and executes through the shared impact/damage executor. Several shooters
per tic therefore see each other's effects and RNG. Demon bites and player
bullets against monsters use the same plan. `LifeHost::apply_damage` already owns
P_DamageMobj retargeting; its kill branch now spawns the P_KillMobj drop through
`LifeHost::spawn`. Capacity for impacts and drops is proved before spawning, and
any late error is latched and discards the whole staged tic, as for every other
DemonPlan operation.

The owner-prefix preflight treats every roster monster as an owner and admits
drops after the player/impact suffix. Monster-to-monster target references are
validated as monster actors. See
[support](COMPATIBILITY.md#original-former-humans-and-multi-monster-encounters),
[source matrix](M3H1_REFERENCE_INVESTIGATION.md) and
[DSIM28](STATE_FORMAT.md#dsim-v28-original-former-humans-and-mixed-rosters).

## Original missiles and typed action dispatch

`projectiles.rs` holds one source missile algorithm: launch tail, spawn check,
explosion, contact query, XY and Z movement. It runs over the small `MissileWorld`
trait, which supplies actor access, RNG, events, damage and relinking. `Simulation`
implements it for the frozen plasma profile; `LifeHost` implements it for the checked
player world, so no missile code is duplicated and no second scheduler exists.
Historical request validation and the profile7 thinker loop stay plasma-only.

`thinkers::run` gives a fireball its own branch inside the shared pass: movement,
which owns contact and explosion, then the ordinary state countdown. Contact damage
uses `DamagePlan::current_original` from the staged transaction, with an impulse proof
that accepts an airborne inflictor. A failure is latched and discards the staged tic.
A blocked spawn keeps the half-stepped position without relinking, as the source does,
so preflight accepts that one location mismatch for an exploding fireball.

Canonical dispatch is typed. `actions::OriginalAction` is resolved at compile time from
the original action table, and `LifeHost::action` and the weapon `act` table match it
instead of comparing names, for the Demon, former-human, Imp and player-weapon family.
Admission keeps its explicit per-profile tables: a typed identity grants nothing. See
[support](COMPATIBILITY.md#original-imps-and-fireballs),
[source matrix](M3H2_REFERENCE_INVESTIGATION.md) and
[DSIM29](STATE_FORMAT.md#dsim-v29-original-imps-and-fireballs).

## Wall-derived travel bounds and real-map scale

`player_map_scale.rs` owns the map-scale policy's admission and bounds without a second
movement, collision or scheduler path. `RoomBounds::derive` runs once in
`with_original_registry`: it computes the vertex rectangle, checks that its four sides are
covered by one-sided walls, that the BLOCKMAP covers the rectangle and lists every wall in
every cell its bbox touches, and that the line count leaves room for the linked things
under `MAXINTERCEPTS`. The result is stored once in `PlayerRuntime`, beside the policy,
and is not canonical state.

Every coordinate check of the checked player world now goes through one policy-aware
helper: `position_admitted`, `spawn_admitted` and `travel_admitted` return the historical
constant margins and the `160M/3` envelope for every older policy, byte for byte, and the
derived rectangle plus the 4,096-unit momentum bound for the map-scale policy. The
callers are the same preflights as before (`monster_actor`, the map-Demon and
world-pickup preflights, the player body and command stage, every damage plan's impulse
proof and the post-tic monster check), so no check was removed, only its bound changed
for one policy. The sight domain, momentum domain and live-fireball bound are selected
the same way.

The slot budget is derived rather than measured: `MAP_SCALE_ACTOR_SLOTS` (320) exceeds
the worst-case live actors of any admitted roster plus the spawns of one pass, so the
store never fills and the thinker visit budget is never reached; the recoverable-refill
latch stays as defense. `MAP_SCALE_MAX_LIVE_PROJECTILES` (208) follows from the Imp's
23-tic minimum launch interval (its attack state cut short by pain, then the JUSTHIT
shortcut) and the longest flight in the largest room.

Immutable definitions are validated once per registry: `validate_demon_lifecycle_type`
remembers each admitted type ordinal in a per-simulation bitmask (the registry, static
sectors and policy never change after construction), and `world_pickups::validate_definition`,
a pure function of the compile-time identity tables, is memoized per process. The checks
themselves are unchanged; they simply stop repeating for every actor every tic. See
[support](COMPATIBILITY.md#original-monsters-at-real-map-scale),
[source matrix](M3H3_REFERENCE_INVESTIGATION.md) and
[DSIM30](STATE_FORMAT.md#dsim-v30-original-monsters-at-real-map-scale).

## Static heights and vertical movement

The heights policy adds no mover, collision path or scheduler: the M3b bodies that
`movement`, `multisector`, `pursuit`, `sight`, `combat_trace` and `projectiles` already
implement (`P_LineOpening`, `PIT_CheckLine`, `P_TryMove`, `P_SlideMove`, `P_ZMovement`,
`P_Move`, `P_CrossSubsector`, `PTR_AimTraverse`, `PTR_ShootTraverse`, `P_SpawnMissile`)
simply run on maps with several sectors. `player_map_scale.rs` owns the additional
admission (`validate_heights_geometry`, the one-sided boundary covering) and the
Rust-only witnesses (`spawn_fits`, `vertical_admitted`, `vertical_witness`,
`HEIGHTS_MOMZ_UNITS`); every flat-world witness of the older policies is kept and
bypassed only when `has_heights()` holds. Type dimensions are admitted against the
tallest sector (`Simulation::type_envelope`, set before the shared impact-type
validation) and fitting is proved per spawn, so windows and closed sectors load.

Two behaviors the flat world could not reach are new code: `thinkers.rs` emits the
`sfx_oof` sound request before the `Vertical` event when the canonical player lands
with `momz < -8`, and `noise.rs` is `P_RecursiveSound` as a pure flood over the static
sectors, used by the staged weapon noise through a per-sector target vector in the
tic plan (`Simulation::emit_noise` keeps its own copy for the historical profiles). The
type-admission memo is atomic so `Simulation` stays `Send + Sync`. See
[support](COMPATIBILITY.md#original-monsters-with-static-heights),
[source matrix](M3H4_REFERENCE_INVESTIGATION.md) and
[DSIM31](STATE_FORMAT.md#dsim-v31-original-monsters-with-static-heights).

## Diagonal walls

The diagonal policy adds no mover, collision, traversal or scheduler code either: the
angle-general bodies (`LineGeometry::point_side`/`box_side`, `movement::project`,
`movement::path` and the intercepts, `combat_trace`, `sight`, `multisector::node_side`,
`pursuit`) run on linedefs of any direction once the admission lets them in.
`player_map_scale.rs` owns the new admission: `Boundary::derive` collects the
one-sided walls, checks the closed-boundary rules C1–C3 in exact integer arithmetic and
provides the ray-parity and closed-box tests; `RoomBounds::derive_diagonal` keeps the
vertex rectangle with the closed-cell BLOCKMAP rule; `validate_diagonal_geometry` is
the heights linedef check without the axis rule. The boundary lives in
`Simulation::boundary` as derived data (never canonical state) and flows into the
thinker inputs and `world_pickups::Contacts`, whose `MotionAccess::after_slide` hook is
the escape witness: `Mover::slide` calls it once a projected slide move has been
accepted, and under this policy an even-parity center fails the tic with
`MapScaleError::Escape` through the same fallible path the pickup contacts use, so the
transaction is dropped whole. Spawn admission (`Simulation::spawn_admitted`) adds the
parity-and-box rule to the rectangle rule. `Simulation::set_momentum` admits the player
body under this policy as the declared mirror of the harness push. See
[support](COMPATIBILITY.md#original-monsters-among-diagonal-walls),
[source matrix](M3H5_REFERENCE_INVESTIGATION.md) and
[DSIM32](STATE_FORMAT.md#dsim-v32-original-monsters-among-diagonal-walls).

The real-map policy changes no mover, collision, traversal, sight or scheduler body
either; it changes what the admission lets in and how the tables behind those bodies
are sized. `GeometryCapacity` (lib.rs) carries the table bounds a construction prepares
against: the M3f2/M3h3 constants for every historical profile, the WAD format's own
bounds (`REAL_MAP_FORMAT_RECORDS`, 32,768 BLOCKMAP words) under this policy, through
`movement::prepare_extent_with` and `Topology::new_with_capacity`.
`map_extent::validate_real_map` computes, instead of checking against constants, the
sight expansion (bounded by the tree size `2 · nodes + subsectors + segs`), its
per-operation budget `64 · (expanded + 5 · lines)` stored in `StaticWorld::
sight_work_budget`, the compiled cell references and the preparation bytes (guarded
by `REAL_MAP_MAX_PREPARED_REFERENCES` and `REAL_MAP_PREPARATION_BYTES`).
`player_map_scale.rs` owns the new admission: `ArithmeticCertificate::derive` evaluates
(i)–(viii) of the M3h6 note in exact integers, `slide_window_lines` the 2 × 2 window
rule, `RoomBounds::derive_real_map` the closed-cell rule without the 95-line bound,
`sector_lines` the `P_GroupLines` lists the noise flood reads from
`StaticWorld::sector_lines`. `line_work::SeenLines` is the `validcount` of a trace: an
inline bitset for worlds of at most 1,024 lines (every older policy, unchanged) and a
heap bitset sized by the line count above it; `combat_trace::collect_limited` uses it
instead of the line-indexed `seen[128]`, which failed every trace that met a line of
index 128 or more. The intercept witness is one comparison in each collector: `movement::path`
takes the policy's limit through `MotionAccess::intercept_limit` (`Some(128)` from
`world_pickups::Contacts` under this policy, `None` elsewhere) and `combat_trace::add`
keeps its 128; both surface `CombatError::InterceptLimit`, which the transaction
boundaries of `demon_expanded_map.rs` name `MapScaleError::Intercepts` under this policy
so the tic is dropped whole. `spawn_map_things` keeps starts as records under this
policy and `set_momentum` refuses pushes to items. See
[support](COMPATIBILITY.md#original-monsters-at-classic-map-scale),
[source matrix](M3H6_REFERENCE_INVESTIGATION.md) and
[DSIM33](STATE_FORMAT.md#dsim-v33-original-monsters-at-classic-map-scale).

## Sky and static specials

The specials policy adds one thinker kind and one piece of mutable sector state; it
changes no mover, collision, traversal or sight body. `ThinkerStore` (store.rs) has two
slot classes, actors (`ActorId`) and sector thinkers (`SectorThinkerId`), and one linked
processing order of `ThinkerId::{Actor, Sector}` entries, which is `P_RunThinkers`' list:
appends run in the same pass and removal stays lazy. The sector-thinker class has a
capacity fixed at construction, zero for every other policy and the light-sector count
under this one (`install_specials` in `demon_expanded_map.rs`), so the 320-slot actor
proof is untouched and the visit budget is the actor slots plus that capacity. The actor
order, `store.head`/`tail` and each actor's neighbors are derived as the actor
subsequence of the shared order, so every historical fixture and digest is unchanged.
`thinkers::run` hands a sector visit to `sector_thinkers::visit`, which refuses
explicitly when the policy has no runtime sectors.

`sector_thinkers.rs` owns the four `p_lights.c` thinkers, `P_FindMinSurroundingLight`,
the `P_SpawnSpecials` sector scan behind `Simulation::spawn_level_specials` and
`P_PlayerInSpecialSector`. `Simulation::specials` holds the runtime light and special of
every sector beside the static `SectorState` heights, which stay read-only. The checked
transaction stages them like the RNG and the actors: `demon_plan` copies them into the
plan's buffer, taken from `spare_specials` (the previous committed plan's vector, like the
sound targets), and stages the sector-thinker slot class the same way in
`spare_sector_slots`; the thinker pass and the player phase write only those buffers, and
the commit swaps them in, keeping the replaced vectors as the next plan's buffers. After a
committed tic neither needs an allocation once its capacity matches the sector or
sector-thinker count; a refused tic drops them with the rest of the plan, so the next plan
allocates each once. Other policies stage both as empty vectors that never allocate. The
actor slots and the spatial order are still copied into fresh buffers every tic, as on
every policy. The player phase is split
for this policy, as `P_PlayerThink` is: the command stage, then `P_CalcHeight`
(`PlayerWeaponOperation::CalcHeight`), then `player_in_special_sector`, then the weapon
change and psprites (`AfterSpecials`); older weapon policies keep the single
`player_pistol::stage`. Environmental damage is an ordinary `DamagePlan` with no
inflictor and no source.

Sky identity is a per-sector boolean in `StaticWorld::sky_ceilings`, present only under
this policy. `projectiles::ceiling_line` derives the source's `ceilingline` from the
refused attempt's openings (no new array), and the projectile mover removes a fireball
whose `ceilingline` has a sky back sector; `combat_trace::hit_trace_sky` ends a trace at a
front-sky line before the puff. Both emit the tag-25 events of DSIM34. See
[support](COMPATIBILITY.md#original-monsters-with-sky-and-static-specials),
[source matrix](M3H7_REFERENCE_INVESTIGATION.md) and
[DSIM34](STATE_FORMAT.md#dsim-v34--original-monsters-with-sky-and-static-specials).

## Doors, lifts, floors, switches and the exit

The movers policy makes sector heights and line specials mutable inside the checked
tic; no collision, sight or traversal body changes, they read the heights through a view.
Six modules own the new work, all under `crates/doom-sim/src/`:

| Module | Owns |
| --- | --- |
| `staging.rs` | `StagedWrites` and the `Sectors` read-through view: the plan's staged sector heights and runtime line specials |
| `mover_rules.rs` | `validate_movers`: the load rules, the reachable-height fixpoint, `R_min`, `R_max`, `Δ` and the mover-sector count (`MoverAdmission`, kept in `Simulation::movers`); the public `mover_bounds` and `special_window_lines` |
| `planes.rs` | `T_MovePlane` (`PlaneHost::move_plane`), `P_ChangeSector`, `PIT_ChangeSector` (giblets, dropped-item removal, `nofit`) and the box-wide `P_ThingHeightClip`; the tag-26 `MoverEvent` vocabulary |
| `movers.rs` | the `Door`, `Plat` and `FloorMove` records, their `EV_*` starters (`Starter`) and `T_*` thinkers, the derived `specialdata` and the live-lift count |
| `line_triggers.rs` | `spechit` as the special subsequence of a position check's `openings`, the reverse crossing loop and `P_CrossSpecialLine` |
| `use_lines.rs` | `P_UseLines` over `movement::path`, `PTR_UseTraverse`, `P_UseSpecialLine`, `P_ChangeSwitchTexture`, the button slots and their countdown, `G_ExitLevel` |

**Sector thinker kinds.** `SectorThinker` gains `Door`, `Plat` and `Floor` beside the
four lights, in the same slot class and the same shared order (`ThinkerStore`, see
[sky and static specials](#sky-and-static-specials)). The class capacity is the light
sectors plus twice the mover sectors (a finished mover keeps its slot until the next
pass unlinks it while a replacement may already run); the 320-slot actor proof is
untouched. A mover is appended at the tail when its `EV_*` runs and takes the lowest
free slot; `thinkers::run` hands its visit to `movers::think` with a fresh `PlaneHost`.
`specialdata` is not stored: it is the one live mover record naming the sector.

**Per-sector block box.** `multisector::sector_block_boxes` computes `sector_t::blockbox`
at load exactly as `P_GroupLines` does (the line bounding box ± `MAXRADIUS`, clamped to
the grid) into `StaticWorld::sector_block_boxes`. `P_ChangeSector` iterates its cells
`x` outer, `y` inner, in link order, skipping things an earlier clip removed.

**How the transaction stages sector and line writes.** A plan never writes
`Simulation::sectors` or `Simulation::line_specials`. `StagedWrites` holds two lazy full
copies: the plan's first write of a table copies the committed table into a buffer taken
from `Simulation::spare_writes`, later writes change the copy, and every later collision,
sight, noise and clip query of the same pass reads it through the `Sectors` view (which
reads the committed table until the first write). Heights are written only through
`StagedWrites::write_sector`, and each clip after a write takes a fresh view, so later
thinkers see the moved plane. A committed plan swaps the copies in and keeps the
replaced tables as the next plan's buffers; a refused tic drops them with the rest of the
plan, so the committed tables stay byte-identical. A tic that writes nothing copies
nothing, so older policies never allocate these buffers. The M3h8 note's
[§9](M3H8_REFERENCE_INVESTIGATION.md#9-sector-staging) records the measurement that chose
the lazy copy over the approved sparse overlay. The button slots, `level_exit`,
`use_down` and the reborn request are staged with the plan's player and level state.

**Where the triggers run.** Every position check's `openings` already lists the
two-sided lines visited before a blocker, so `spechit` needs no new array; a view without
line specials (every older policy) collects nothing, and a ninth entry is the
`SpecialCross` witness. The player's and knocked-back things' crossings are found during
the motion and applied after it; a monster's `P_Move` applies them inline and, on a
failed move, runs the `DI_NODIR` reset and the reverse use loop in `pursuit::move_actor`.
The player phase runs the weapon change, then the use block, then `P_MovePsprites`, as
`P_PlayerThink` does; the button countdown of `P_UpdateSpecials` runs in the Specials
phase after the thinker pass. Under the movers policy the tic-boundary preflight checks
V′ (`player_map_scale::v_prime_admitted`) instead of the M3h4 vertical invariant. See
[support](COMPATIBILITY.md#original-monsters-with-doors-lifts-floors-switches-and-the-exit),
[source matrix](M3H8_REFERENCE_INVESTIGATION.md) and
[DSIM35](STATE_FORMAT.md#dsim-v35--original-doors-lifts-floors-switches-and-the-exit).

## Two-sided scrolling walls and static decorations

The E1M1 policy adds no runtime body: the collision, missile, hitscan, clip and thinker
paths already treated a non-shootable solid thing and a flags-0 thing as the source does
(the M3h9 note's step 1 audit in
[§12](M3H9_REFERENCE_INVESTIGATION.md#12-follow-ups-risks-status)). What it adds is
admission. `mover_rules::validate_e1m1` runs the movers load rules with a two-sided 48
admitted (`e1m1_mover_bounds` is its public query), and one new module,
`crates/doom-sim/src/e1m1_rules.rs`, owns the decorations: the explicit seven-type list
`E1M1_DECORATIONS`, the static predicate every listed type satisfies, `MAX_DECORATIONS`,
the linked-thing budget `E1M1_LINKED_THINGS` with the note's §5.3 table, and the
per-actor witness the tic preflight applies (the DSIM36 reader checks the same fields
from the encoded record). The policy gate is
`OriginalExecutionProfile::has_e1m1`; the role, state, spawn and API-refusal sites ask
`e1m1_rules::is_decoration`, so every older policy keeps its admission. Decorations are
ordinary actors in the one store, the spatial links and the thinker order; no store or
staging buffer changes. See
[support](COMPATIBILITY.md#original-monsters-with-two-sided-scrolling-walls-and-static-decorations),
[source matrix](M3H9_REFERENCE_INVESTIGATION.md) and
[DSIM36](STATE_FORMAT.md#dsim-v36--original-two-sided-scrollers-and-static-decorations).

## Exploding barrels, pickup scale and every thing angle

`PlayerMonstersBarrelsV1` (wire 21, DSIM37; the M3h9b note's
[§5](M3H9B_REFERENCE_INVESTIGATION.md#5-admitted-domain-of-playermonstersbarrelsv1-and-its-budgets))
adds one actor role and one body set on top of the E1M1 policy. `crates/doom-sim/src/barrels.rs`
owns the barrel: the role predicate (`is_barrel`, gated by `has_barrels`, never a policy
monster), the `info.c` definition and state set, the live witness `validate_barrel`, and
the explosion. The explosion is `A_Explode` → `P_RadiusAttack` → `PIT_RadiusAttack` run on
the staged `LifeHost` of the tic, with the source's wrapped 3 × 3 block window
(`BLAST_WINDOW`, statically asserted equal to `128 << 16`), the `y` outer, `x` inner walk,
and each cell's things in link order: the spatial index keeps one newest-first order
(`SpatialIndex::link` prepends), a cell's list is that order filtered by cell, and the
walk re-reads the next link in the victim's *current* cell after each callback, so a
monster relinked by its own `A_Chase` inside the walk changes the walk as the source's
`bnext` chain does. Sight checks go through a per-explosion `ExplosionSight` control
(independent of the per-tic monster budget); every victim is damaged through the shared
`DamagePlan::current_original` → `reserve` → `apply_damage` path, which now accepts a
barrel target (no armor, no credit, no drop) and refuses a stale bomb source with
`PlayerBodyError::Invariant` before any mutation. Barrels move through the ordinary
thinker path (thrust momentum, friction, the corpse and dropoff rules) and are gibbed by
`planes.rs` like any `health <= 0` thing; `OriginalAction::Explode` is admitted for
`MT_BARREL`'s `S_BEXP4` only. The per-policy bounds live beside the role: `actor_slots`
(384), `world_pickups::max_pickups` and `max_bonus` (64, 384), `MAX_BARRELS` (16) and the
spawn rule S′ (`Boundary::spawn_admitted_relaxed`, a sibling of the closed-box rule, used
only under `has_barrels`). See
[support](COMPATIBILITY.md#original-monsters-with-exploding-barrels-pickup-scale-and-every-thing-angle)
and [DSIM37](STATE_FORMAT.md#dsim-v37--original-exploding-barrels-pickup-scale-and-every-thing-angle).

## Whole levels: wire 22 and the whole-engine runner

`PlayerMonstersLevelV1` (wire 22, DSIM38; the M3h9c note's
[§10](M3H9C_REFERENCE_INVESTIGATION.md#10-policy-proposal)) adds no actor role, store or
field to the barrels policy. Its three behaviours, gated by the policy predicate
`has_level`, live with the code they extend: the body spawned by `spawn_map_things` at its
type-1 record in `spawning.rs`, the live `damagecount` decay beside the bonus decay in
`world_pickups.rs`, and `P_DeathThink`'s turn toward the attacker (`death_turn`) in
`level.rs`; every older policy keeps its
admission. `doom_sim::demo` reads and writes version-110 demos for the `player_level_run`
example. The genuine side is a separate program, not part of the simulation: the C runner
compiles the original files whole with declared stubs and adapters and dumps the state after
every tic, and an independent Python projector and comparator turn a dump tic into a DSIM38
capture to compare with Rust's. See [support](COMPATIBILITY.md#original-level-runs-the-player-start-in-lump-order-and-the-full-player-think-wire-22),
[DSIM38](STATE_FORMAT.md#dsim-v38--the-whole-level-the-body-at-its-things-record-damage-count-decay-and-p_deaththink)
and [commands](TESTING.md#m3h9c-whole-engine-runner-and-the-real-e1m1-comparison).
