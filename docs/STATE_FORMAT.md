# Canonical diagnostic state format: DSIM versions 1–40

This is a diagnostic/replay comparison format for explicit simulation profiles,
not a DOOM savegame or network/demo format. A field schema change requires an
intentional format-version change and reviewed golden updates. All integers
below are little endian, with explicit widths and two's-complement signed values.
No Rust layout, padding, pointers, addresses, timestamps, paths, or GPU state is
encoded. Shared immutable map storage uses Arc internally; reference counts and
addresses are excluded.

M3f8's execution-scope brands and derived target availability are API observations,
not persistent fields. V20/v21, profile/registry/context schema identifiers, stored
target handles and event meanings remain unchanged. An inert target identity may
remain encoded after retirement; readers must not redirect it to a reused slot.
Decoded diagnostics do not restore a world or grant a context token. See the
[ownership contract](COMPATIBILITY.md#context-and-target-lifetime-contract).

M3f9's process-local InstanceId and qualified-reference envelopes are also excluded.
They have no numeric/byte encoding, and neither DSIM version can restore them or
an executable world. A decoded local ActorId carries no provenance and rejects
qualified import unless a caller explicitly supplies trusted local routing policy.
That provenance milestone introduced no v22, field change or reinterpretation; see the
[provenance contract](COMPATIBILITY.md#game-instance-provenance).

## Framing and field order

Stepping observation is optional: `advance` records the same gameplay/events but
does not capture a Snapshot; `step` captures after that core succeeds. Explicit
capture and encoding retain every historical byte and interpretation, including
v20/v21. No format version or field changes accompany this API split. See the
[stepping contract](COMPATIBILITY.md#stepping-and-diagnostics).

Header: ASCII `DSIM`, u32 schema version (`1` for M2, `2` for M3a, `3` for M3b, `4` for M3c, `5` for M3d1, `6` for M3d2, `7` for M3d2b, `8` for M3d3a, `9` for M3d3b, `10` for M3d3c, `11` for M3d3d, `12` for M3d3e, `13` for M3d3f, `14` for M3d3g, `15` for M3d3h, `16` for M3d3i, `17` for M3d3j, `18` for M3f1, `19` for M3f2, `20` for M3f3, `21` for the original-ID profile; `22` to `40` are the policy
sections below, each naming the policy and context its version selects),
u64 field count. Each ordered field is
u32 path-byte length, UTF-8 path bytes, u64 value-byte length, then value bytes.
Generated schema paths are shorter than u32 capacity. Value/count lengths fit
u64 on supported Linux/Windows/macOS hosts. No native usize bytes are emitted.

Runtime fields precede static map fields. Runtime order:

| Field/group | Value encoding |
| --- | --- |
| `tic` | u64 completed logical tics |
| `tic_rate`, `profile` | u32 35; u32 profile 1 (M2 flat non-player XY), 2 (M3a player/solid XY), 3 (M3b static-sector XY/Z), 4 (M3c typed actor lifecycle), 5 (M3d1 bounded damage/hitscan), 6 (M3d2 player weapons/psprites), 7 (M3d2b projectiles), 8 (M3d3a perception), 9 (M3d3b pursuit), 10 (M3d3c bounded attacks), 11 (M3d3d shotgunner attacks), 12 (M3d3e chaingunner attack/refire), or 13 (M3d3f Demon melee); profile-specific additions are specified below |
| `paused` | u8 boolean |
| `rng.gameplay`, `rng.miscellaneous` | u8 indexes, in that order |
| `store.max_slots`, `store.slot_count` | u32 capacity policy; u64 logical allocated slot count |
| `store.head`, `store.tail` | optional actor identities |
| `thinker.order` | consecutive actor identities in processing order |
| `actor.N.generation`, `.retired`, `.occupied` | u64, u8 boolean, u8 boolean, for every logical slot including vacant/retired slots |
| Occupied slot `.pending_removal`, `.previous`, `.next` | u8 boolean, optional identities |
| Occupied slot `.x`, `.y`, `.z`, `.momx`, `.momy`, `.radius`, `.height`, `.floor`, `.ceiling` | i32 raw 16.16, in this order |
| Occupied slot `.angle`, `.lastlook` | u32 binary angle; u8 spawn RNG modulo four |
| `player.N.actor`, `.forward`, `.side`, `.angle_turn`, `.buttons` for slots 0..3 | optional identity, i8, i8, i16, u8 |
| `events.last.count`, then `events.last.N` | u64 count, individually framed event records |
| `events.pending.count`, then `events.pending.N` | u64 count, individually framed event records |

An actor identity is u32 slot + u64 generation (12 bytes). An optional identity
is u8 presence + those 12 bytes; absent identities have all-zero payloads. Slots
and generations are logical state, not allocator metadata. Heap capacity is
excluded. The lowest reusable slot is chosen deterministically; processing order
remains a separate append order. Generations never wrap; exhausted slots retire.
Handles belong to a store/instance, not a global namespace. Fresh equivalent
instances intentionally produce equivalent handle values.

## Events

| Tag | Payload |
| --- | --- |
| 0, phase | u8 Players=0, Thinkers=1, Specials=2, Respawn=3 |
| 1, spawned | actor identity |
| 2, removal marked | actor identity |
| 3, retired | actor identity |
| 4, player-command phase | u8 slot + actor identity |
| 5, thinker update | actor identity |
| 6, movement attempt | actor identity; i32 target X/Y; u16 blocker or 65535 for none; u8 fits; u32 candidate count; ordered u16 candidate indexes |
| 7, explicit gameplay draw | u8 lookup result |

Between-tic spawning, removal marks, and explicit gameplay draws queue pending
observations. At the next unpaused step they become the start of the last-event
list, before Players/Thinkers/Specials/Respawn observations. Spawn consumes one
gameplay lookup for lastlook; its event is tag 1, not an additional tag-7 draw.
Specials and respawn are currently phase markers, not implemented gameplay.
Paused step calls leave commands, time, RNG, and both event lists unchanged.
Presentation reads event lists without draining or mutating them.

## Static map fields

After runtime fields: `map.things`, `.linedefs`, `.sidedefs`, `.vertices`, `.segs`,
`.subsectors`, `.nodes`, `.sectors`, `.reject`, `.blockmap.words`,
`.blockmap.cell_ranges`, in that order. The first eight tables encode the exact
classic record fields in original order and widths (10/14/30/4/12/4/28/26 bytes),
including raw eight-byte names and -1 sidedef sentinels. BSP children encode the
original u16 high-bit leaf flag. REJECT retains all parsed bytes. BLOCKMAP words
are all decoded u16 words, including header/table/unused words; row-major cell
ranges are u32 start/end word indexes, excluding terminators. Shared lists are
stored once, so canonicalization is linear rather than expanding every cell list.
Derived line slopes/bboxes are reproducible from these fields and profile 1.

## DSIM-v2 and migration

`Simulation::new` / `with_actor_limit` remain profile 1, version 1, with unchanged
framing, fields, events and committed v1 expectations above. Select M3a explicitly
with `player_collision` / `with_player_collision_limit`; its header version and
`profile` value are both 2. `Snapshot::version()` exposes the version;
first-difference reports `schema.version` before comparing incompatible schemas.
There is no savegame loader or automatic migration. Re-run the same setup/commands
under the chosen profile, retaining version-specific expected traces. Do not
replace v1 regression expectations with v2 bytes.

V2 retains v1 field order with these precise additions:

- Immediately after each occupied `actor.N.lastlook`: `actor.N.flags`, u32 0 or
  MF_SOLID=2. No other actor flags are supported. Vacant slots omit this field.
- After all logical actor slots and before `player.0.actor`: `spatial.link_order`,
  concatenated 12-byte identities, most recent link first. Includes off-map live
  actors; excludes removal-marked actors. Filtering by center cell reconstructs
  cell-head order. This order can differ from thinker/slot order and is required
  state. Allocation capacity is excluded.
- Existing player bindings and last commands encode all implemented player XY
  state. Ground Z/floor/ceiling are already explicit; no hidden player timers,
  view/animation/weapon state is implemented. Movement candidates are observations,
  not persistent pointer relationships.

Two new event tags use the same individually framed event fields:

| Tag | Payload, in order |
| --- | --- |
| 8, actor movement | 12-byte mover; i32 target X/Y; 13-byte optional blocking actor; u32 actor-candidate count; ordered 12-byte candidate IDs; u16 blocking line or 65535; u8 fits; u32 line-candidate count; ordered u16 line IDs |
| 9, slide projection | 12-byte mover; u16 selected linedef |

Profile 2 emits tag 8 instead of tag 6, including ordinary non-player moves.
Actor candidates include self/non-solid observations and stop at the first solid
blocker. A thing blocker leaves the line-candidate list empty. Tag 9 occurs when
P_HitSlideLine projects remaining motion; a failed flush or pure stair fallback
has no projection event. All phase/spawn/removal/RNG tags retain v1 encodings.
Static map fields are identical in layout; derived cell/line caches are omitted.

`m3a_snapshot_goldens.py` independently constructs explicit profile-2 coasting
state from Python v1 encoding rules, source C XY values, and explicit clockwise
synthetic map records. It reads no Rust output. `m3a_snapshot_provenance.json`
records input/encoder/output SHA-256 hashes. New first-tic bytes and twelve digests
are versioned separately; they are project schema goldens, not reference saves.

## DSIM-v3 and static-sector state

M3b constructors explicitly select header version/profile 3. V1/v2 constructors,
fields, framing and fixtures remain available unchanged; there is no savegame
migration/loader. Reinitialize a selected profile and replay its commands, retaining
versioned expectations. Cross-version comparison reports `schema.version`.

V3 retains v2 field order with these exact changes/additions:

- Occupied `actor.N.flags` is u32 combination of MF_SOLID=2, MF_NOGRAVITY=512 and
  MF_DROPOFF=1024; no other bits are supported. After it: `.momz` i32 raw fixed,
  `.subsector` u16, `.sector` u16. Existing `.z`, `.floor`, `.ceiling` retain their
  positions; constraints can differ from the actor center's sector heights.
- After `spatial.link_order`, before player fields: `world.blockmap_policy` u8=1
  (supplied reference lists); `world.conservative_omissions` u32 diagnostic bbox
  cell-pair count; `sectors.count` u32.
- For each runtime sector in numeric order: `sector.N.floor` i32 raw fixed,
  `.ceiling` i32 raw fixed, `.actors` concatenated 12-byte identities in current
  sector-head order. The sector vector is distinct from static map definitions;
  M3b exposes no runtime-height mutation API. Removal marks exclude list membership
  immediately even while actor slot storage remains occupied.
- Spatial order includes all live actors, including off-grid centers; filtering
  by source BSP membership reconstructs sector-head lists. Derived topology,
  line bounds/slopes and compiled cell caches are reproducible and omitted.

New events (all integers little endian):

| Tag | Payload |
| --- | --- |
| 10, sector movement | The complete tag-8 payload without its tag byte, followed by u16 proposed subsector, u16 sector, i32 floor/ceiling/dropoff, u8 floatok, u8 first failure, u32 opening count; each opening is u16 line + i32 top/bottom/range/lowfloor |
| 11, Z update | 12-byte actor; i32 before Z/momz, after Z/momz, floor/ceiling (six i32s); u8 floor-contact, ceiling-contact, gravity-applied booleans |

Failure values: 0 allowed, 1 actor, 2 line, 3 interval fit, 4 headroom, 5 step,
6 dropoff. They follow source evaluation order. `fits` in the tag-8 subpayload is
false only after an evaluated interval-fit failure; an earlier actor/line blocker
leaves it true (not evaluated). `floatok` becomes true after interval fit passes.
Opening records include only actual crossing two-sided candidates evaluated before
rejection, preserving order. SectorMovement emits for each attempted destination,
including slide retries/fallback; tag 9 slide projections retain v2 encoding and
appear at their original chronological position. Z emits only when its source
guard (z!=floor or nonzero momz) runs, after that actor's XY events. No camera,
viewheight, audio or state-machine data is implied.

`m3b_snapshot_goldens.py` independently encodes 96 states from explicit setup,
synthetic map bytes, C per-attempt counts/order and physical XY/Z results. It reads
no Rust output. Contact bits derive from pre-Z C values and original predicates.
Five complete byte fixtures cover first fall, step-up clipping, center-sector
transition, airborne drop and landing; tests compare named fields as well as full
bytes. Input/encoder/output hashes are in `m3b_snapshot_provenance.json`. These are
project schema goldens with independent physical evidence, not engine saves.

## Digests, comparison, and evidence

FNV-1a-64: initial value `0xcbf29ce484222325`; for each canonical byte, XOR it
then multiply by `0x100000001b3`, wrapping modulo 2^64. It is noncryptographic.
`Snapshot::first_difference` compares named values; `compare_traces` compares
captures in order and reports the first differing tic/field or trace length.
Compare bytes/fields even when hashes agree. Default divergence display omits
value contents; callers can inspect encoded expected/actual values privately.

`tools/reference/snapshot_goldens.py` independently encodes an explicit one-actor
setup plus recorded C coasting values. It reads no Rust output. Tests compare the
full first snapshot bytes and twelve per-tic digests/position/momentum vectors.
Longer 768-call scenarios test fresh instances, RNG wrapping, lifecycle/reuse,
pause, presentation schedules, and injected first-divergence detection. Those
long scenarios are project consistency checks, not full-engine reference traces.
Both kinds run in debug/release CI across supported platforms.

Snapshots include map/resource bytes. Traces made with private assets must remain
private and must never become public fixtures, logs, artifacts, or screenshots.
All current fixtures and CI scenarios use only original synthetic map data.

## DSIM-v4 and typed actor state

Profile 4 extends v3 with the following explicitly ordered fields. Older profiles
retain their original bytes and fixtures. Registry definitions/options are immutable.
All integers use little-endian widths; booleans are u8 0/1. Optional actor IDs use
the existing 13-byte presence/u32-slot/u64-generation representation.

- After `rng.miscellaneous`: `actors.options` = skill u8 (baby=0 through nightmare=4),
  mode u8 (only single-player=0 admitted), nomonsters u8.
- Existing per-actor `flags` now records the full admitted source u32 flags.
- After actor flags, before v3 momz/subsector/sector: `type` u16, `state` u16
  (65535 = entered null pointer; zero = direct `&states[0]` spawn assignment),
  `tics`, `health`, `reaction_time` i32 each, `sprite` u16, `frame` u32, `target`
  optional ID, `death_requested` u8, `spawnpoint` presence u8 followed, when present,
  by signed x/y/angle/type i16 and options u16 (10 bytes). Prefix remains `actor.N.`.
- Spatial link order retains all logical relinks. Block queries filter NOBLOCKMAP;
  sector list fields filter NOSECTOR. Immediate removal excludes both lists.
- After immutable map fields: `registry.states.count` u64, then `registry.state.N`;
  `registry.types.count` u64, then `registry.type.N`. State payload is name length
  u8/name UTF-8 bytes, sprite u16, frame u32, tics i32, action encoding below,
  next u16, misc1/misc2 i32. Type payload is name length/name followed by the 23
  source-order i32 properties documented by ActorTypeDef and m3c_types.csv.

Action payload: None = tag 0; original = tag 1/u16 action ID; probe = tag 2/u8
probe ID (Random=0, Enter=1, SpawnTarget=2, RemoveSelf=3, RemoveTarget=4,
TargetSelf=5). Enter/SpawnTarget additionally carry a u16 state/type operand.
Original IDs follow the generated symbolic table: zero None, remaining 74 names
in lexicographic order. Sprite/sound IDs follow original source enum order.

Event tag 12 introduces actor-state observations. Its second u8 is Initialized=0
(actor ID/type u16), Entered=1 (actor/state u16), Action=2 (actor/action payload),
Sound=3 (optional actor/sound u16), Target=4 (actor/optional target), Death=5
(actor), TargetUnavailable=6 (actor), Random=7 (actor/value u8). Actor IDs here
are the existing nonoptional 12-byte slot/generation encoding unless specified.
Last/pending observations include these events in execution order. Initialized
records direct assignment without invoking an action; Death records the explicit
controlled request, not damage processing. No sound channel/device state exists.

`m3c_snapshot_goldens.py` independently encodes C values and explicit setups:
99 captures and 24 full states cover death, null/retirement, map placement,
recursive callbacks/targets/spawning, five actions and initial zero-tic behavior.
Names and immutable definitions are serialized so different native registries
cannot accidentally produce equal canonical bytes merely by sharing indexes.

## DSIM-v5 and bounded combat

`Simulation::combat` / `with_combat_registry` select header version/profile 5.
Versions 1 through 4 remain byte-for-byte unchanged; no savegame migration exists.
V5 retains v4 field order and adds `actor.N.threshold` (i32) immediately after
`.spawnpoint` and before `.momz`. All other implemented persistent combat state
already has explicit fields: health, flags, current state/timer, momentum, height,
reaction, targets, slot generations, linked orders and gameplay RNG. No persistent
inflictor, weapon state, attack queue or artificial combat counter is added.

Event tag 13 adds a combat subtag u8. `ID` below means the existing 12-byte actor
identity, `OptID` the 13-byte optional identity. Fixed values are raw i32 16.16;
all counts/integers have explicit little-endian encoding.

| Subtag | Ordered payload |
| --- | --- |
| 0 Attack | shooter ID, angle u32, range/slope/damage i32 |
| 1 Trace | origin X/Y then delta X/Y i32; u32 cell count then i32 X/Y pairs; u32 candidate count then objects; u32 intercept count then (fraction i32, object); hit presence u8 then optional object; position presence u8 then optional X/Y/Z i32; traversed count u32 |
| 2 Damage | target ID, inflictor OptID, source OptID, amount i32 |
| 3 Random | kind u8, value u8 |
| 4 Knockback | target ID, angle u32, thrust i32, resulting momx/momy i32 |
| 5 Health | target ID, before/after i32 |
| 6 Pain | target ID, state u16 |
| 7 Kill | target ID, state u16, extreme u8 |
| 8 DeathTimer | target ID, tics i32 |
| 9 Reaction | target ID |
| 10 Retarget | target ID, source ID |
| 11 Impact | actor ID |
| 12 Ignored | target ID |

Trace object = tag 0 + line u16, or tag 1 + actor ID. Intercepts are stable sorted
by signed fraction, preserving candidate insertion ties; candidates remain in
source callback order. Trace records precede impact/damage and include no later
result.effect/result.damage fields. Random kinds: 0 forward-fall, 1 pain, 2 kill
timer, 3/4 impact Z left/right, 5 impact timer. P_SpawnMobj lastlook remains an
implicit draw, observable in actor/RNG state and initialization order. Actor-state
entries/actions/targets still use tag 12 at their actual chronological positions.
There is no sound device, renderer state or memory address in these records.

`m3d1_snapshot_goldens.py` reads C captures, explicit inputs and source metadata,
never Rust output. It encodes 48 initial, nine damage API, three shot API and three
settled death captures (63 total), with 17 full byte fixtures and provenance hashes.
The selected due-east damage events use source branch predicates, C RNG/momentum/
state rows and original input thrust; complete shot traces use C geometry/cell/
intercept/hit rows. Settled final snapshots assert no movement/actions and encode
phase/thinker events; C link/removal observations reconstruct spatial/store order.
Field comparison precedes digest comparison for stored byte fixtures. Digests are
noncryptographic schema diagnostics, not proof of complete gameplay equivalence.

## DSIM-v6 and player weapons/psprites

`player_weapons` / `with_weapon_registry` explicitly select version/profile 6.
Versions 1..5 retain their exact field order, bytes and goldens. V6 retains v5
actor fields, including threshold, and adds the following ordered runtime fields:

- Immediately after each `sector.N.actors`: `sector.N.soundtarget`, the existing
  13-byte optional actor identity. Per-call sound traversal caches are derived.
- After all four player command groups, before events: `weapons.player.present`
  u8. When present, append fields under `weapons.player.` in this order:
  `slot` u8, `actor` 12-byte ID, `health` i32, `ready` u8, `pending` u8 (10 means
  source wp_nochange; 9 is NUMWEAPONS), `owned` u16, `ammo` four i32s, `max_ammo`
  four i32s, `attack_down` u8, `refire` i32, `bob` raw fixed i32, `extra_light` i32.
- Then `weapons.player.psprite.N.state` u16 (65535 null), `.tics` i32, `.x`/`.y`
  raw fixed i32, for N=0 weapon then N=1 flash. Sprite/frame derive from the
  immutable registry state; no ordinary thinker identity is invented for psprites.

The profile invariant admits only one LIVE player with protected health. Inventory
setup is explicit; no armor/power/pickup state or attack queue is invented. Slot,
body generation, source ownership/ammo, timers/positions, light, commands, RNG,
actor effects and sound targets capture all implemented persistent consequences.

Event tag 14 adds a u8 weapon subtag. All integer values use explicit little-endian
widths. ID below is 12-byte slot/generation; action IDs reuse the source symbolic
registry table, but dispatch eligibility is separate from actor actions.

| Subtag | Ordered payload |
| --- | --- |
| 0 Entered | slot u8, psprite u8, state u16 (zero records null entry) |
| 1 Action | slot u8, psprite u8, action u16 |
| 2 Ammo | slot u8, ammo u8, before/after i32 |
| 3 Light | slot u8, value i32 |
| 4 Fire | slot u8, weapon u8, refire i32 |
| 5 Bullet | actor ID, angle u32, slope/damage i32 |
| 6 Noise | sector u16, blocking-lines-crossed u8, target ID |
| 7 Initialized | slot u8, actor ID |
| 8 Random | kind u8 (damage=0, spread left=1, spread right=2), value u8 |

Source weapon sounds use existing tag-12 Sound observations; combat retains tag
13 exactly. Entries/actions/effects appear in chronological order. Setup queues
initialization/psprite events, which prefix the first completed tic just like actor
spawns. Read-only queries never drain observations or consume RNG.

`m3d2_snapshot_goldens.py` reads C/setup/schema inputs only and reuses historical
v5 field assembly without editing it. It explicitly adds v6 fields/events, source
player bindings/commands/color angles and sector sound targets. Selected states
are initial, raise/ready/attack-start and settled captures with no shot effects or
motion on that tic; their complete source weapon/lifecycle events are encoded.
Single-burst captures avoid slot reuse assumptions; the per-tic C test separately
checks reused identities and transient firing fields/order. 69 captures and 24 byte
fixtures have independent input/encoder/output hashes and first-field diagnostics.
Hashes remain noncryptographic schema evidence, not full gameplay synchronization.

## DSIM-v7 — Projectile Foundations

Profile/version 7 explicitly selects `Simulation::projectiles` or
`with_projectile_registry`. It composes **v5** field order: typed actor fields,
threshold, XYZ/momentum, generation/link/membership, sector geometry and immutable
registry already represent every persistent plasma property. `ActorLife.target`
is the source owner. Intended launch victim/aim is transient; no tracer or runtime
damage multiplier is introduced. Registry type 34 contains immutable damage/speed
and state definitions; flags/state/tics encode flight versus explosion.

V6-only `sector.N.soundtarget` and `weapons.player` fields are absent in v7. V6 is
a separate player-weapon profile, not a required schema prefix. No implicit profile
conversion or savegame migration exists. Versions 1..6 retain their exact old
fields/bytes/helpers. First comparison across versions reports `schema.version`.

Shared state, combat, sector-movement and Z events retain tags 12, 13, 10 and 11.
New event tag **15** is followed by one u8 discriminator:

| Discriminator | Payload in order |
| --- | --- |
| 0 explosion starts | 12-byte missile identity, before momentum/state changes |
| 1 launch velocity | missile identity; u32 BAM angle; three i32 raw momenta XYZ |
| 2 projectile RNG | u8 kind (0 spawn tics, 1/2 shadow left/right, 3 contact damage, 4 explosion timer); u8 result |
| 4 initial half-step | missile identity; three i32 raw XYZ; i32 randomized spawn tics |
| 5 launch request | source identity; u8 aim mode (0 actor, 1 autoaim); destination identity only for mode 0; u16 projectile type |

Discriminator 3 is unused/reserved and is never emitted. Launch state is queued
between tics; successful effects retain source order. Damage precedes movement
result/explosion; in-thinker explosion precedes the same actor's state countdown.
Actor and candidate references retain their original generation after retirement.

`m3d2b_snapshot_goldens.py` independently encodes 129 C/setup-derived captures
and stores 35 full streams. It covers all 41 setups, complete wall/dropoff
flight/explosion/removal, settled lethal state and both RNG-wrap batches with
slot reuse. No Rust output is read. Transient damage fields/order are tested
against C separately; canonical bytes are project schema evidence, not DOOM
savegames or full synchronization. Rust checks named fields before full bytes
and digest, preserving first-field divergence diagnostics.

## DSIM-v8 — Monster Perception and Target Acquisition

Schema/profile 8 deliberately composes v5 persistent typed actors/static-sector
state, the existing sector sound-target representation and a separate minimal
perception roster. It has no v6 weapon player/psprites or v7 projectile API/event15.
Old schemas 1..7 and their expected bytes/encoders remain unchanged. Existing
actor lastlook, target, threshold, reaction time, state, tics, health and angle
already encode perception consequences; no duplicate fields are added.

In field order, add `sector.N.soundtarget` immediately after each `sector.N.actors`
using the existing fixed 13-byte optional generation identity (u8 presence, u32
slot, u64 generation; absent is 13 zero bytes). Legacy `player.N` bindings/commands
remain absent/zero. Immediately after those four legacy slot groups and before
events, write for N=0..3:

| Path | Value |
| --- | --- |
| `perception.player.N.active` | u8 boolean |
| `perception.player.N.health` | signed i32 little-endian player-record health |
| `perception.player.N.actor` | fixed 13-byte optional generation identity |

Inactive retained IDs remain encoded even after retirement; active IDs must pass
preflight. Sound targets persist independently of assignment/wake. Local REJECT
indices, slope scratch, traversal stack, duplicate masks, validcount, sightcounts
and scan plans are not authoritative persistent fields. Diagnostic events carry
observations of completed operations and do not become inputs to later detection.
Pure sight queries append no events and preserve the entire snapshot.

Existing event tags/phases retain their layouts. Shared noise uses only tag 14,
discriminator 6 (u16 sector, u8 blocks, 12-byte target); other weapon events are not
admitted. New tag 16 is followed by the u8 discriminator below. All integers are
little-endian. `ID` is u32 slot+u64 generation; `OptID` is the fixed 13-byte form.

| Discriminator | Payload in order |
| --- | --- |
| 0 LookStart | ID actor, i32 threshold before reset |
| 1 LookSound | ID actor, OptID sound target, u8 shootable, u8 ambush |
| 2 ScanStart | ID actor, u8 allaround, u8 lastlook, u8 stop |
| 3 Slot | ID actor, u8 slot, u8 active, u8 prior active count |
| 4 Health | ID actor, u8 slot, i32 player-record health |
| 5 Facing | ID actor, u8 slot, u32 relative BAM angle, u8 distance-present, i32 distance only if present, u8 accepted |
| 6 EmptyRoster | ID actor; explicit Rust safety-policy observation |
| 7 ScanEnd | ID actor, u8 found, u8 final lastlook |
| 8 Wake | ID actor, u16 see state |
| 9 Random | ID actor, u8 gameplay result |
| 10 Sight | ID observer, ID target, u8 visible, u32 REJECT pair, u8 rejected, u8 geometry-present; if present i32 eye/bottom/top; u32 visit count then visits |

Each sight visit starts with a u8 code:

| Code | Payload in order |
| --- | --- |
| 1 NodeStart | u16 node, u8 raw side |
| 2 NodeEnd | u16 node, u8 raw side |
| 3 Subsector | u16 subsector |
| 4 Seg | u16 subsector, u32 seg, u16 line, u8 duplicate |
| 5 Sides | u16 line, u8 stage, u8 side0, u8 side1 |
| 6 Opening | u16 line, i32 top, i32 bottom |
| 7 Clip | u16 line, i32 fraction, i32 bottom, i32 top |
| 8 Blocked | u16 line, u8 reason (0 one-sided, 1 closed opening, 2 closed slopes) |

Geometry is absent on REJECT rather than serializing stale C global scratch.
Node side 2 is recorded before normalization. Action/target/sound/state-entry
observations still use established tag 12.

`m3d3a_snapshot_goldens.py` independently encodes 420 C/setup-derived captures and
45 full streams, including pending/transient sight/scan/A_Look/noise, source
thinker phases and RNG wrap. Named fields, full bytes and FNV-1a64 digest are
compared by Rust. Native fixture registry edits are encoded explicitly. This
schema is a project comparison format, not an original DOOM savegame or evidence
of complete vanilla/demo synchronization.

## DSIM-v9 — Monster Pursuit and Movement Decisions

Schema/profile 9 composes v8 typed actor/static-sector state, sector sound targets,
legacy absent/zero player bindings and the distinct perception roster. It adds
only two persistent actor fields immediately after `actor.N.threshold` and before
`actor.N.momz`, for each occupied typed actor:

| Path | Value |
| --- | --- |
| `actor.N.movedir` | u8 source direction 0..7 or NODIR (8); spawn initializes EAST (0) |
| `actor.N.movecount` | signed i32 little-endian; spawn initializes 0 |

Existing angle, flags, health, target, reactiontime, threshold, state/tics and
spatial/sector order are reused. Native registry edits (including missile-state
removal, integer speed, radius, sounds and positive Chase duration) are explicitly
encoded in existing immutable registry fields. Source skills use the existing
actors.options field. fastparm/netgame remain fixed false; no hidden option is
added. v6 weapon player, v7 projectile events/API and player damage are absent.
All schemas 1..8 retain their original fields/expected bytes/encoders.

Temporary axis candidates, movement results, query stamps and local plans are
not persistent authority. Existing events 10 (ordered sector movement),11 (Z),12
(actor state/action/target/sound),14/6 (shared noise) and 16 (perception) retain
their exact layouts. Observations do not become gameplay inputs.

New event tag 17 has a u8 discriminator, then 12-byte actor ID (u32 slot, u64
generation) and payload below, all integers little-endian:

| Discriminator | Remaining payload |
| --- | --- |
| 0 Start | u8 operation |
| 1 Return | u8 operation, u8 boolean result |
| 2 Direction | u8 assigned movedir |
| 3 Random | u8 kind, u8 original lookup value |

Operation 0=Move, 1=TryWalk, 2=NewChaseDir, 3=Chase. Void operations encode false in
Return; this does not invent a source result. Random kind 0=successful walk count,
1=axis priority, 2=fallback ordering, 3=active sound. Source call nesting, assignments,
query results, RNG, state/sound events and returns retain chronological order.
Move NODIR has no collision event. Failed direct moves leave actor constraints/
links untouched. Successful walking relinks immediately before its movement
observation; ordinary momentum XY/Z still precedes the state action.

`m3d3b_snapshot_goldens.py` independently encodes 835 C/setup-derived captures and
stores 49 full byte streams. All captures compare digests; selected streams first
compare named fields, then bytes. Captures include pending/transient callbacks,
source thinker phases, counter/angle/target changes, relinking and RNG wrap. C
ordinals map to generation 1 in these source cases; retirement/reuse is separately
verified by Rust policy tests. This is a project diagnostic comparison schema,
not a DOOM savegame/demo or evidence of complete vanilla synchronization.

## DSIM-v10 — Monster Attack Selection and Bounded Actions

Schema/profile 10 composes v9 fields in the same order: typed actors, health,
states/tics/flags, source target generations, threshold/movedir/movecount, physical
geometry, thinker/spatial/sector links, sound targets and distinct four-slot roster.
It includes shared damage/impact state and exact named registry adaptations. No
new persistent attack result, queue or cached shot plan is serialized. Every
v1-v9 schema/fixture byte remains unchanged. fastparm/netgame stay fixed false.

Existing event tags 10/11/12/13/14/16/17 retain their layouts. New tag 18 has a u8
variant and 12-byte actor identity (u32 slot,u64 generation), followed by:

| Variant | Payload after actor identity |
| --- | --- |
|0 Range |u8 kind (0 melee,1 missile),u8 accepted,u8 reason,optional i32 distance,optional i32 threshold |
|1 Random |u8 kind,u8 lookup value |
|2 Face |13-byte optional target,u32 old/new angle,u32 old/new flags |
|3 Selection |u8 kind (0 melee,1 missile),u16 state |
|4 JustAttacked |u32 resulting flags |
|5 Aim |shared combat Trace payload beginning with its variant1 byte,optional target,i32 slope,i32 bottom/top window |

Optional range integers use a presence byte followed by i32 only when present.
Melee distance/threshold are raw 16.16; missile distance is signed integer map
units after the reached adjustments (final capped value when probability runs).
Reasons 0 accepted,1 null,2 melee distance,3 sight,4 JUSTHIT,5 reaction,6 VILE far,
7 UNDEAD near,8 random rejection. Random kinds 0/1 shadow,2/3 spread,4 damage,
5 missile probability. Source sight uses tag 16 before range; pure public melee
returns evidence without adding events. Selection precedes melee sound/state;
JustAttacked follows missile state actions. Internal PosAttack facing has its own
Face/Random observations without inventing another state-action event.

Aim's reused Trace layout contains origin/delta, ordered cells/candidates, stable
fraction-sorted intercepts, optional stopped object, absent position and traversed
count, then aim target/slope/window. PosAttack uses existing tag 13 Attack/Trace,
real effect and damage observations in source order. No observation drives AI.

The independent Python encoder reads C traces, public inputs, original metadata
and this schema. It maps source birth ordinals to the lowest free slot/current
generation, retaining old references after retirement, and reproduces full events
and canonical link order.1290 captures and 72 full streams include retirement and
reuse; first differing named field precedes full bytes/digest checks. DSIM remains
a project diagnostic format, not a vanilla demo, savegame or network protocol.

## DSIM-v11 — Shotgunner Multi-Pellet Attack Foundations

Version/profile 11 composes the accepted v10 actor, health/state/frame/timer/flags,
target/generation, reaction/threshold/movedir/movecount, geometry, thinker/spatial/
sector links, sound targets and four-slot perception roster. Exact labelled SPOS
registry adaptations replace the fixture animation in this separate profile.
No weapon-player record or persistent pellet/cache/plan is added. V1-v10 fields,
expected bytes and helper schemas remain unchanged; a higher profile number does
not grant all previous gameplay capabilities.

Existing tags 10/11/12/13/14/16/17/18 keep their layouts. Source body state/action,
shotgun sound, facing/shared aim/RNG and real combat/effect events retain their
existing chronological forms. New tag 19 is u8 variant, 12-byte actor identity
(u32 slot,u64 generation), then:

| Variant | Remaining payload |
| --- | --- |
|0 Begin |None; emitted only after the source non-null check, before sound |
|1 PelletStart |u8 source ordinal 1..3, before its three attack RNG draws |
|2 Pellet |u8 ordinal,u32 BAM angle,i32 shared slope,i32 independent damage |
|3 PelletDone |u8 ordinal,u8 gameplay RNG index,13-byte optional real effect ID |
|4 Completed |u8 final gameplay RNG index |

Optional IDs retain presence+u32 slot+u64 generation, zeros when absent. All
integers are explicitly little-endian. A true miss has no effect; null emits no
shotgun event (the existing action-entry observation still records invocation).

Each pellet's tag 18 spread/damage draws precede its tag 19 parameters and existing
tag 13 Attack/Trace/impact/damage. PelletDone follows all those effects and records
the exact source RNG index before the next PelletStart. Shared aim appears once
before the loop. Earlier standalone FaceTarget and internal facing remain separate
observations; internal facing does not invent an extra state-action entry.

The independent Python encoder reads C/public inputs and documented schema only.
Aim and each of three line traces have distinct scopes, including identical miss
results, so candidates/intercepts cannot accidentally be mixed across pellets.
C birth ordinals map to lowest-free slots/current generations; full thinker and
spatial order, effects, retirements and old identities are encoded. 949 captures
and 92 full streams are compared with first-field diagnostics and byte/digest
checks. DSIM is a project diagnostic, not a DOOM savegame/demo/network protocol.

## DSIM-v12 — Chaingunner Attack and Refire Foundations

Version/profile 12 composes the v11 field order: actor state/tics/sprite/frame,
health/flags/target generations, movedir/movecount/reaction/threshold, RNG, geometry,
thinker/spatial/sector order, roster health and sound targets. Exact labelled CPOS
states replace the attack fixtures in this distinct profile. No persistent firing
counter, aim cache or preflight plan is serialized. V1-v11 formats and expected
bytes remain unchanged; profile numbers do not grant cumulative gameplay support.

Existing tags and layouts are retained. Actor-state tag 12 records CPosAttack/
CPosRefire entries using original action IDs 13/14, shotgun sound, immediate see/
Chase/Look actions and physical state changes. Tag 18 retains facing, fresh aim,
spread/damage draws and range selection; its Random kind 6 adds the refire draw
(0/1 shadow, 2/3 spread, 4 damage, 5 missile chance stay unchanged). Tag 16 carries
only reached source sight queries, and tag 13 carries actual shot/damage/effects.

New tag 20 is u8 decision followed by a 12-byte actor ID (u32 slot,u64 generation):

| Decision | Meaning |
| --- | --- |
| 0 | Random below 40: early continuation without target/health/sight evaluation |
| 1 | Null target: immediate see-state handoff |
| 2 | Mobj health <=0: handoff without refire sight |
| 3 | Healthy target failed original sight: handoff |
| 4 | Healthy visible target: natural continuation |

Chronology is action entry, facing/shadow RNG, refire draw, optional sight, decision,
then any immediate see/Chase/Look actions. Nested queries after a handoff remain
separately observable. CPOS's 10/4/4/1 phase is already represented by state/timer;
shots execute on distinct entries with current aim and source RNG.

The independent encoder consumes original-C rows/public setup/schema only and
checks 1,544 captures/110 full streams, real lowest-free-slot/generation mapping,
retirement, fields, bytes and digests. The separate injected-40 dependency probe
is not a gameplay capture and adds no schema field. DSIM remains a project
diagnostic, not a DOOM savegame/demo/network protocol.

## DSIM-v13 — Demon Melee Attack Foundations

Header/version and profile are 13. V13 composes the existing typed actors,
position/momentum/flags/health/state/timer/sprite/frame, target/threshold/reaction,
movedir/movecount, RNG indices, roster/sound targets, store generations and
thinker/spatial order, map/sector and complete registry fields. No persistent
melee result/counter or cached sight/aim is added. V1-v12 definitions and expected
bytes/generators/encoders/helpers remain unchanged.

Native 136 has named SARG Look/Chase 1/2 and copied attack 4/5/6 (sprite 39,
frames 4/5/6,tics 8/8/8,next 5/6/2). Its explicit source-player-body adaptation sets
health 150,mass 400,melee 4,missile 0,attacksound 52. Target 135 retains labelled source
body pain/death/see states 149..173. Full original Demon 12/Spectre 13 execution,
Nightmare timing and actual player binding/damage are excluded.

No new transient tag is necessary:

| Existing tag/layout | Meaning used by v13 |
| --- | --- |
| Actor-state action/layout | Original action 56 A_SargAttack; separate action 20 entries for ATK1/ATK2 |
| 18 Monster Range, kind 0 | Current source melee decision; optional distance/threshold, strict short circuit |
| 18 Monster Random | kind 0/1 separate SHADOW draws; kind 4 one damage draw |
| 18 Monster Face / Selection | Real facing and Chase melee choice before sound/state entry |
| 16 Sight/perception | Existing ordered REJECT/BSP result/visits and awareness/roster effects |
| 13 Combat | Direct Damage request, source/inflictor IDs, knockback, health, pain/kill/timer RNG, reaction/retarget/ignored |
| Existing state/sound/movement/store events | Selection sound 52 before entry 4, real Chase return and source thinker/spatial order |

Null Sarg invocation records the action but no Face/range/damage draw. A miss
records Face/shadow and evaluated range/sight, no damage draw. A hit records
Face, sight/range, kind 4, then immediate shared damage effects. ATK1/ATK2 and
internal Sarg facing remain distinct; source state already expresses 8/8/8 progress.
No hitscan Attack/Trace/Impact is generated by direct melee. The scripted reuse
fixture has a genuine explicit body spawn/retirement; it is not a melee effect.

The independent Python encoder reads original-C/public fixture/schema data only.
Its 1,309 captures/169 complete streams cover multi-tic entries, changed range,
pain/death/RNG and real lowest-free-slot generation reuse. First-field diagnostics
complement full bytes and digests. The 36 radius component vectors are outside
these capture totals. DSIM remains a project diagnostic, not a vanilla savegame,
network format or demo; hashes do not establish complete game compatibility.

## M3d3g DSIM-v14: Demon pain, death and corpse foundations

Version/profile 14 composes the same field families as v13, with explicit native
Demon lifecycle metadata and copied state aliases. All earlier headers, field
order, encoders and expected bytes remain frozen. The existing version 8+ roster
and sector sound fields are present; root profile and DSIM header both encode 14.
No persistent corpse boolean, pain result or duplicate death timer is added.

Native136 copies source Demon properties with aliases: 1 Look, 2 Chase, 4/5/6 SARG
attack, 7/8 pain, 9..14 ordinary death. PAIN2 returns to fixture Chase 2; DIE6 remains
-1 with next0. Doom number is-1, flags 6 omit COUNTKILL, raise/xdeath/missile are0.
All source pain/death sounds, mass, height, chance and other copied fields remain
visible in canonical registry records. Native135 and original blood/puff records
retain their explicit source-body/effect identities; full types 12/13 stay blocked.

Health, flags, height, XY/Z momentum/location, state/tics/sprite/frame, target,
reaction/threshold, generations, thinker/spatial/sector order and RNG already
encode all persistent outcomes. Kill sets CORPSE/DROPOFF and height 14 while SOLID
remains; Fall later clears SOLID; terminal state 14 has tics -1 and retains storage.
Old target identities may remain unresolved after retirement and never alias reuse.

Existing transient tags suffice: ActorState12 Entered/Action/Sound; Combat13
Damage/Random/Knockback/Health/Pain/Kill/DeathTimer/Reaction/Retarget/Impact/Ignored;
existing Monster/Perception/Sight and movement observations retain chronology.
No new event tag is needed to diagnose pain, scream, Fall or terminal entry.

`m3d3g_snapshot_goldens.py` independently derives3,972 captures and 727 complete
byte streams from C/public inputs/schema, including per-tic digests and first-field
comparison paths. The three injected pain-boundary components are excluded from
canonical gameplay captures. DSIM remains project diagnostic data, not a vanilla
savegame, network protocol or proof of full demo synchronization.

## DSIM-v15 — Original Demon Stand/Run State Closure

Profile 15 composes the v14 persistent layout and observation tags. Header schema
and profile numbers are15; every v1–v14 encoder/fixture byte remains unchanged.
No additional persistent actor flag, AI clock or redundant corpse state is added.
Actor state/tics/sprite/frame, original metadata, health/flags/height/momentum,
identity/target, reaction/threshold/directions, thinker/spatial/sector/roster/sound
membership and RNG already describe this behavior.

The immutable fixture registry copies source states 475/476 into1/3, and477–484
into2/15–21. Sprite39, frames0/1 and10/10 stand timing, frames0/0/1/1/2/2/3/3 and
2-tic run timing, actions/misc and every mapped next-state link are retained.
Native 136 is named FIXTURE_DEMON_STAND_RUN_136. Its property aliases, excluded
COUNTKILL/raise and admitted body/effects match the documented restricted slice.
Attack4–6 and pain7–8 return to RUN1/2; death9–14 preserves terminal-1.

Existing Entered/Action/Sound, perception/sight/movement and combat observations
record immediate nested chronology. Repeated frames still have different state
IDs and separate Chase actions. No transient tag is added merely to label a frame.
`m3d3h_snapshot_goldens.py` independently derives 7,186 captures and 1,395 complete
byte fixtures from original-C traces/public inputs/schema, including all loop,
wake/combat/lifecycle transitions and C-observed RNG-wrap boundaries. It never
reads Rust output. DSIM is project diagnostic data, not a DOOM savegame or demo.

## DSIM-v16 — Original Demon Map-Spawn Foundations

Version/profile 16 preserves every v1–v15 field layout and expected byte stream.
It composes the v15 schema and existing persistent fields: raw map THINGS,
ActorOptions, registry metadata, actor type/state/tics/sprite/frame/health/flags,
position/momentum/angle/lastlook, floor/ceiling/sector, target generation, thinker
and spatial order, RNG indexes and existing transient events. No duplicate map-
spawn, original-type, scoring or corpse flag is introduced.

Each map native has the existing `.spawnpoint`: u8 presence1 followed by exactly
10 little-endian bytes (i16 x,y,angle,type; u16 options). Unbound/effect spawns
retain presence0. Original kind3002 remains in spawnpoint/map fields, while the
explicit executable actor is136 and the registry declares the adaptation. Source
filter/THING-index observations are recorded in reference rows/report, not hidden
persistent state. Header and profile carry 16; sound/roster fields compose exactly
as in 15. Raw spawn has no entered-state action event.

`m3d3i_snapshot_goldens.py` independently composes C rows/public input/schema into
3,440 captures and 784 complete byte fixtures, including filtered/multi-THING setup,
angles/floors, source RNG wrap, ordered shared-target strikes and spawn-to-corpse.
Rust checks full field order/value/bytes and per-tic FNV1a64, with first-field
errors. The asserted 135-tic lifecycle digest is `e327c3fbf88cb132`. DSIM remains
project diagnostic evidence, not a vanilla savegame/demo or full synchronization.

## DSIM-v17 — Demon Map Context and Statistics

Profile17 composes v16 fields/ordering with a distinct registry name and source
COUNTKILL flags. Schemas1–16, their encoders, fixtures and Rust helpers remain
byte-identical. This is project diagnostic state, not an original savegame/demo.

Immediately after `actors.options`, before `store.max_slots`, encode four named
fields, each signed32-bit little-endian:

| Field | Meaning |
| --- | --- |
| `level.total_kills` | Available admitted map COUNTKILL placements |
| `level.total_items` | Source initialized item total; remains0 in this slice |
| `level.total_secrets` | Source initialized secret total; remains0 in this slice |
| `level.single_player_kill_count` | Genuine unbound single-player players[0] fallback credit |

All counters begin0. Spawn total increments after timer RNG and before angle/
ambush assignment; death credit occurs after corpse flags/quarter height and
before death-state entry/timer RNG. Neither death nor removal decrements available
kills. No redundant corpse, respawn-disabled or initial-seed field is serialized:
actor flags/state and actual owned RNG indexes already represent admitted state.
Profile17 fixes single-player normal timing and respawn disabled by admission.

Transient event tag21 has a one-byte subtype:0 map available-kill update,1 fallback
death credit. Both then encode actor identity (u32 slot,u64 generation) and signed
32-bit updated total. Events observe the source positions above. They are not
additional persistent statistics. Existing event tags/order retain their meanings.
Head-actor retirement is inside the thinker phase, before any later live Think.

Fresh restart constructs a new instance through the same validated setup path,
clears counters/tic/actors and retains actual RNG streams. Handles remain scoped
to their instance. Checked overflow and recoverable rejection are Rust policies;
unsupported player scoring, respawn, raise and full game setup reject explicitly.

The [independent encoder](../tools/reference/m3d3j_snapshot_goldens.py) derives
complete bytes/digests from C rows and documented schema, never Rust output.
See [source findings](M3D3J_REFERENCE.md), [validation](validation/M3D3J.md) and
[reproduction](TESTING.md).

## DSIM-v18 — Map Scale Foundations

Profile18 deliberately composes the **DSIM-v3 untyped static-sector field layout**,
with header version and `profile` field both u32 value18. It does not inherit typed
lifecycle fields merely because its version is numerically higher. All v1–v17
encoders, fixtures, helpers and encoded meanings remain unchanged.

Field order, integer widths and event tags are those of v3: tic/rate/profile/pause,
owned RNG indexes, store limit/slots/list links and thinker order, untyped physical
actor fields and flags, XY/Z/momentum/constraints/location, spatial link order,
BLOCKMAP policy/omission diagnostic, indexed runtime sector heights/head lists,
four unbound command slots, ordered last/pending events, then original map tables
and source BLOCKMAP words/ranges. No registry, actor-state, perception roster,
weapon or map-statistics fields are added. Existing count/index widths cover the
[declared capacities](COMPATIBILITY.md#map-scale-profile) without widening classic records.

Prepared-list caches, membership words, setup reference budget, timing measurements
and derived storage metrics are excluded. The map is immutable; a different
successful setup reference budget cannot affect later behavior. Actor slot limits
remain encoded by the existing `store.max_slots` field.

The [independent encoder](../tools/reference/m3f1_snapshot_goldens.py) consumes
original-C traces/public fixture inputs and this schema, never Rust output. The
[record](validation/M3F1.md) identifies full-byte and per-tic coverage and its
explicit exclusions. Full fixtures diagnose the first differing named field;
digests are noncryptographic diagnostics, not source-equivalence proof. This is
neither an original savegame nor a vanilla demo format.

## DSIM-v19 — Expanded Map Extents and Spatial Work

Profile19 uses the same untyped static-sector field layout described for v18 and
v3, with header version and `profile` both u32 value19. It adds no persistent fields
or new event tags. Existing physical actors, flags, XYZ/momentum/constraints,
location, thinker/spatial/sector order, owned RNG, command/event fields and original
map tables represent the expanded inputs. Numeric version order does not admit
typed lifecycle, perception, weapons or statistics.

Preparation budgets, membership, root-expansion cache, timing/storage observations
and read-only SightResult diagnostics are excluded. Actor slot limits remain in
`store.max_slots`; successful differences in other setup budgets do not alter
canonical state. All v1–v18 schemas, encoders, helpers and expected bytes remain
unchanged. [The independent encoder](../tools/reference/m3f2_snapshot_goldens.py)
uses original-C physical/order traces, public classic bytes and this schema;
expected snapshots are never generated from Rust. Full fixtures identify the first
different named field; per-tic FNV digests are diagnostics, not vanilla savegames,
demos or independent proof of complete source equivalence. Coverage and exclusions
belong in [the evidence record](validation/M3F2.md).

## DSIM-v20 — Expanded-Map Demon Gameplay Composition

Header version and the `profile` u32 are both 20. This explicitly selects the
v17 typed actor, perception, lifecycle and statistics field layout with the new
profile's admitted larger classic-map tables. It does not inherit v19's untyped
layout, nor add redundant copies of geometry, actors, targets or counters.

Field order remains: tic/rate/profile/pause/RNG; actor options and level counters;
store/slot/actor/thinker state; spatial and static-sector state with ordered sector
actors/sound targets; command bindings/commands and perception roster; last and
pending events; original map tables and registry metadata. Existing v17 widths,
optional identity encoding, spawnpoints, state/tics/sprite/frame, health/flags,
threshold/reaction/movement, XYZ/momentum and statistics/event tags are unchanged.
Sector/line capacities change admitted field counts, not integer wire widths.

No new persistent field or event tag is introduced. The actor execution plan,
transient sight failure/query budgets, root-expansion cache, prepared allocations
and timing/RSS observations are excluded. Failed operations commit no diagnostic
state. All v1–v19 schemas, encoders, helper bytes and expectations remain frozen.

The [main independent encoder](../tools/reference/m3f3_snapshot_goldens.py) and
[additional clipping encoder](../tools/reference/m3f3_clipping_snapshot_goldens.py)
use genuine C rows, public input bytes and documented native identity/state
projection. Expected bytes never come from Rust. Full fixtures diagnose the first
named field; every capture also compares complete ordered event-field bytes with
first-byte diagnostics. Event projections are separate from full DSIM fixtures.
Per-tic FNV digests provide compact checks. DSIM is not a vanilla
savegame/demo. [Evidence](validation/M3F3.md) distinguishes C-derived captures from
separate independently specified Rust-only capacity states.

## Original identity metadata observations

M3f4 changes no DSIM version, profile, field or historical fixture. Canonical
identity views are read-only metadata, not additional simulation state. Existing
registry-local identifiers remain encoded exactly as before. The separate
`DOOM-IDENTITY-v1` text observer begins with its schema tag and counts, followed
by ordered type/state/sprite records; it is not a DSIM capture or demo format.
The [mapping guide](ACTOR_IDENTITIES.md) documents interpretation, and the
[source observer](../tools/reference/m3f4_reference.py) defines emitted columns.

## Canonical Demon execution observations

M3f5 adds no DSIM schema, profile, persistent field or event tag. Raw ActorLife
and DSIM-v20 continue encoding native type 136 and historical state aliases.
Canonical definition queries are derived observations, excluded from snapshots.

The separate text protocol `DOOM-DEMON-EXEC-v1` prefixes the original-C trace and
spawn files with that schema line. Its tagged rows inherit the column definitions
in the unchanged [M3f3 observer](../tools/reference/m3f3_reference.py) and
[source extractor](../tools/reference/generate_m3d3j.py), but retain raw original
type 12/SARG identities. The [new encoder](../tools/reference/m3f5_snapshot_goldens.py)
explicitly projects only identity-bearing A/E/K/J/G fields into native Profile20
slots before invoking the unchanged independent DSIM encoder. Its provenance lists
every mapping and input/output hash. The raw C files are retained separately.
This is source-derived projected evidence, not a claim that native IDs equal
original ordinals. Historical captures remain unchanged.

## Typed native identity observations

M3f6 changes no DSIM version, profile tag, registry metadata, field path, width,
value or event encoding. NativeDemonType/NativeDemonState are derived views, excluded
from serialization. ActorLife.actor_type/state and state-bearing events retain the v20
native slots. M3f6 itself introduced no v21 stream or decoder. Historical
encoders/readers and fixtures keep their existing interpretation. The
[runtime audit](M3F6_REFERENCE.md) identifies the exact observable fields that a
opt-in original-ID contract must version rather than silently reinterpret.

## DSIM-v21 — Original-ID Demon diagnostics

Version21 changes actual definition/state/event values, supplies explicit registry
and context identifiers, and omits redundant immutable registry rows. It is an
opt-in diagnostic representation; it is not a savegame or executable world loader.
No historical version is redefined. Existing readers/helpers for 1–20 remain valid;
a v20-only reader rejects21.

### Identifiers and field layout

Framing is unchanged: DSIM + u32 version 21 + u64 field count, followed by ordered
u32 path length/UTF-8 path/u64 value length/value records. Runtime fields compose
the version 17 families, as v20 does, with these explicit differences:

| Field or family | V21 contract |
| --- | --- |
| `tic`, `tic_rate` | u64 completed tic, u32 value 35; unchanged |
| `profile` | u32 execution policy 1 = `DemonNormalV1`; independent of format21 and Rust enum ordering |
| `registry.schema` | Exact UTF-8 bytes `DOOM-INFO-1.10-v1`, immediately after profile |
| `context.schema` | Exact UTF-8 bytes `UNBOUND-BODY-v1`, immediately after registry schema |
| `.type` and initialized-state event type | u16 definition slot: source ordinals0–136 or explicit context 137; only admitted roles may occur |
| `.state` and state-bearing events | u16 actual original state ordinal; normal SARG475–495, not native1–21 |
| Absent current state | u16 65535; distinct from original S_NULL0 |
| All actor handles | u32 slot + u64 generation (12 bytes); optional handles have one presence byte plus12 bytes |
| Registry counts/records/actions | Not emitted; derived from identified immutable source/context packages |
| Remaining runtime/map/event fields | Same explicit widths and order as v20, complete map fields retained |

The registry identifier fixes all original actor/state/sprite/action metadata of
the preserved Linux DOOM1.10 package. `OriginalExecutionRegistry::admit` checks the
complete records; changed metadata requires a new schema. Action IDs in event
payloads retain the existing Rust dispatch encoding fixed by this package, not
invented original-C action ordinals. There are no serialized function pointers.
The context package fixes one synthetic unbound body, its definition slot derived
from original actor count, and the descriptor specified in
[identity mapping](ACTOR_IDENTITIES.md#original-id-execution-registry). It does not
claim original player identity. Source135/136 remain original decorations.

State-entry event subtype1, perception SeeState subtype8, monster attack selection
subtype3 and combat Pain/Killed subtypes6/7 carry source states. Initialized type
subtype0 carries the original/context definition slot. Other event payloads,
chronology, raw RNG observations/indexes, statistics and map/physics fields retain
the shared schema. Referenced handles may be stale because an event describes
history; they must never resolve to a different generation. No second canonical
identity field is added to runtime storage or diagnostics.

### Bounded encoding and reading

`Snapshot::diagnostic_bytes(DiagnosticLimits)` preflights byte/field/path/value
counts with checked arithmetic and uses a fallible reservation before encoding.
The existing trusted `canonical_bytes()` convenience API keeps its prior process
allocation policy. `Diagnostic::decode` explicitly accepts20/21;
`decode_v20` accepts only20. Unknown versions and incompatible profile/registry/
context identifiers reject, with no fallback reinterpretation. Versions1–19 use
their unchanged historical readers and are not accepted by this new reader.

Default budgets are 64 MiB total/value,262144 fields and192 path bytes. These are
diagnostic input/output budgets, not increased simulation capacities. Lengths,
UTF-8, duplicate paths, truncation/trailing data, integer overflow and allocation
failure reject. Field-count reservations require enough remaining framing bytes.
The reader borrows values and sorts only a bounded lookup index; encoded actor,
thinker and event order is preserved. Raw nested sight/collision payload counts
are not used to allocate or traverse untrusted arrays.

The read-only view validates required version/profile/schema fields, source/context
role and current-state closure, current generations/occupied slots, handle widths,
bounded handle slots, event counts/tags and the identity-bearing event fields.
V20 retains native interpretation and validates its complete embedded registry
against the legacy package. `actor(handle)` resolves only an occupied slot with
matching generation. Optional unavailable/stale target references do not redirect.
`encode` rechecks caller budgets and preserves exact input bytes/order.

This is deliberately a diagnostic reader: non-identity event bodies and physical/
map values are opaque, and successful decode does not prove canonical full-field
shape, executable state, coherent geometry or restorable simulation. Unknown field
paths can be retained for inspection; unknown format/profile/schema identifiers
cannot. It never constructs a Simulation. Error-policy, malformed-input and
namespace tests are Rust-only safety evidence, separate from C compatibility.

### Independent evidence and historical reading

[M3f7 encoding](../tools/reference/m3f7_snapshot_goldens.py) consumes unchanged
M3f5 compiled-C rows, preserving actual type 12/source SARG states. Only the
observer's explicitly synthetic body 135 binds to the context definition. Immutable
metadata is omitted by schema; profile/schema/framing are independently encoded.
A compact digest index and two complete streams avoid duplicating old raw traces
or EV20 data. Ordered event tests reuse old EV20 bytes and replace only identity
payloads from actual C rows, not native-to-original runtime tables. These are new
encodings of reused inputs, not new C behavior coverage. [Observed results](validation/M3F7.md)
own exact counts and size. All v20 expectations/readers remain byte-for-byte intact.

## DSIM-v22 — Original player body

V22 was introduced for PlayerBodyLifecycleV1 (wire policy2), because genuine
player ownership/attacker/health/state observations change the schema. The original
policy2 contract below remains unchanged; separate grounded and command policies follow. It preserves
DSIM framing and the immutable original registry ID `DOOM-INFO-1.10-v1`; context ID
is exactly `PLAYER-BODY-1-MEDIUM-NOTHRUST-v1`. Historical v20/v21 bytes, identities,
encoders and readers remain unchanged. No reinterpretation or world restoration.

Runtime fields compose the v21/version17 families with these changes:

| Ordered location | V22 representation |
| --- | --- |
| Header / profile | u32 version22 / u32 execution policy2 |
| registry.schema, context.schema | Exact IDs above, in the existing positions |
| Occupied actor after movecount, before momz | actor.N.player: u8 source player index0 if its full handle equals mo, otherwise255; derived, not another stored identity |
| Synthetic perception.player.* family | Omitted; this policy has an unlisted genuine body, no synthetic AI roster |
| After four historical command-slot families, before events.last | original_player.0.mo (optional handle), health (i32), state (u8 PST_LIVE0/PST_DEAD1), attacker (optional handle), damage_count (i32), frags (i32), in that order |
| Existing level.single_player_kill_count | Sole source players[0].killcount, no duplicate player counter |
| Definitions / states | Actual original0/12/37/38 and root-admitted source states; context137 is not executable here |

OriginalPlayer fields are present even before binding, initially mo/attacker absent,
health100, Live, damagecount/frags0. The four old `player.N.actor` command binding
families remain distinct and empty, with zero commands under policy 2. mo and attacker retain
stored handles when unavailable. The derived actor association requires exact slot
AND generation; no stale handle points at a reused body. InstanceId remains excluded.
Immutable metadata is identified, never serialized redundantly.

Event tag22 was introduced in v22 and is retained in v23. Player slot0 is fixed by the policy:

| Subtype | Ordered payload after tag/subtype |
| --- | --- |
| 0 Bound | 12-byte body handle, after existing Spawned/Initialized observations |
| 1 Damaged | i32 clamped player health, 13-byte optional attacker, i32 damagecount, i32 tactile amplitude; before existing mobj Health event; tactile period40/duration10 fixed by schema |
| 2 Died | i32 frags[0], u16 original requested S_PISTOLDOWN state; after source player-dead/solidity changes and before existing Kill/state entry |

Other ordered event bytes and identity meanings remain as previously specified.
The psprite request state is metadata for a service boundary, never permission to
execute that state as a mobj. Tactile is an observation, not a device implementation.

`Diagnostic::decode` retains its v20/v21-only contract and rejects22. Explicit
`Diagnostic::decode_player` accepts only22 through the same bounded framing parser,
validating policy/schema, admitted identities/states, handle widths/generations,
player health/state/association, available attacker roles and new event payloads.
Malformed/truncated/duplicate/over-budget data reject. Stale references remain
opaque, not redirected. `Snapshot::diagnostic_bytes` supports20–22 with existing
fallible size budgets. Trusted canonical_bytes remains available. As before,
physical/map and unrelated event data are not executable restoration proof.

[Independent encoding](../tools/reference/m3g2_snapshot_goldens.py) uses new genuine
C player/body observations and separately declared Rust birth/generation schema
policy. Complete event bytes cover every tic; selected state captures, compact
indexes and four complete fixtures avoid broad raw-trace growth. Expected bytes
never come from Rust. [The evidence record](validation/M3G2.md) owns exact counts.

## DSIM-v22 grounded-player execution policy

PlayerBodyKnockbackV1 selects wire policy 3 with exact context schema
`PLAYER-BODY-1-MEDIUM-GROUNDED-v1`. It uses the same version 22 framing, ordered
fields, source player/mo/attacker association and event widths as stationary
policy 2. The original registry schema remains `DOOM-INFO-1.10-v1`. Position,
momentum, existing movement/slide and CombatEvent::Knockback payloads already
represent every required observable; no version 23 or new event tag is necessary.

Policy 2 must pair with `PLAYER-BODY-1-MEDIUM-NOTHRUST-v1`, and policy 3 with the
new grounded schema. Mixed/unknown pairs reject. `Diagnostic::decode_player`
explicitly validates these two policy/context pairs; the historical v20/v21 decode entry
point still rejects v22. Neither old context definitions nor native identities are
reinterpreted. Existing policy 2 full bytes remain immutable.

The [new independent encoder](../tools/reference/m3g3_snapshot_goldens.py) uses
C physical/relationship/collision observations plus explicit schema policy. Its
reusable [player projection](../tools/reference/player_snapshot.py) retains full
DSIM bytes for selected fixtures. Test-only `PBE1` event evidence is compact:
4-byte magic, repeated u32 case/op/last-count/pending-count, followed by each
last then pending event's u32 length and exact payload. Field names are derived
from family/index; both counts and every ordered payload are compared. This is
a separately versioned test-fixture encoding, not a DSIM format change or a demo.

Decoding stays read-only, bounded and distinct from restoring an executable world.
Physical/map fields and unrelated event payloads retain their documented opaque
validation limits. Source equivalence is established by the declared comparisons,
not merely by successful decoding or canonical projection.


## DSIM-v22 player-command execution policy

PlayerCommandMovementV1 selects wire policy 4 and exact context schema
`PLAYER-COMMAND-1-MEDIUM-GROUNDED-v1`, with registry `DOOM-INFO-1.10-v1`. Framing,
field order/widths and original player association remain unchanged. Existing
player.0.forward/side/angle_turn/buttons contain admitted input; the remaining
three slots stay default, and historical player.N.actor bindings stay empty.
Existing actor angle/momentum/reaction/state/timer fields describe the outcome.
No persistent field, DSIM version or event tag is added.

The Players phase now precedes command work and the Thinkers phase follows it.
Existing tag4 Player(slot0,body handle) observes completed P_MovePlayer, after any
immediate RUN entry; reaction-suppressed or inactive bodies emit no such event.
All ordinary thinker/movement/combat events retain their meanings. Old policies
2/3 retain their previous bytes, phase order and exact context pairing; v20/v21
retain their historical interpretation. Mixed/unknown policy/context pairs reject
through explicit decode_player; the historical decode entry point still rejects22.
The reader validates bounded structural/identity/player contracts, not complete
command arithmetic, map physics or executable-world restoration.

The [independent command encoder](../tools/reference/m3g4_snapshot_goldens.py)
combines actual C command/phase observations with frozen field/event primitives
through a reusable player execution projection. It preserves PBE1 test framing,
reports birth/generation projection separately from C pointer behavior and never
uses Rust outcomes as expectations. See [observations](validation/M3G4.md).

## DSIM-v22 active-player Demon policy

`PlayerDemonAcquisitionV1` selects wire policy5 and exact context
`PLAYER-DEMON-1-MEDIUM-GROUNDED-NOMELEE-v1`, paired with `DOOM-INFO-1.10-v1`.
Framing, widths, field order and event tags remain DSIM22. Existing original_player
mo/health/state/attacker fields and derived actor.player association describe the
same canonical record. Fixed active source slot0 and inactive slots1–3 are part
of this immutable context contract; no independently mutable roster health/actor
copy or redundant immutable activity field is serialized. The historical
perception fixture array is not encoded in player policies.

Existing actor target, lastlook, angle/momentum, state/tics, reaction/threshold,
command, RNG, statistics and ordered events describe acquisition and nonmelee
pursuit. Players/Thinkers phase ordering and Player command events retain policy4
meanings; the existing Perception, Pursuit, Monster range, state/sound and movement
events retain their source chronology. No version increment or event reinterpretation
is required. Old v20/v21/v22 policies2/3/4 retain their bytes and identifiers.

The explicit decode_player reader recognizes only matching policy/context pairs
and validates original identities against this policy's admitted states. Unknown
or mixed pairs reject. SARG attack metadata exists but has no executable permission
under this policy. Structural reading still does not establish executable geometry,
roster liveness, provenance or a restored world. Setup and retained-reference
observations remain distinct from permission to advance.

The [independent encoder](../tools/reference/m3g5_snapshot_goldens.py) verifies
actual C roster R rows against P/AP ownership and health before using frozen field/
event primitives. Complete PBE1 event frames and selected full fixtures preserve
all ordered payloads. Birth/generation projection is independent Rust schema policy,
not C allocator identity. See [evidence](validation/M3G5.md) and
[source/harness qualifications](M3G5_REFERENCE.md).

## DSIM-v22 autonomous Demon-player melee policy

`PlayerDemonMeleeV1` selects wire policy6 and exact context
`PLAYER-DEMON-1-MEDIUM-GROUNDED-MELEE-v1`, paired with `DOOM-INFO-1.10-v1`.
Framing, field order, widths and event tags remain DSIM22. Existing player/mo,
attacker, actor target/lastlook, angle/momentum, health/state/tics, flags, reaction,
statistics, RNG and ordered Monster/Combat/PlayerBody events describe the admitted
melee without new persistent fields or redundant immutable metadata.

Fixed source activity remains player0 active and slots1–3 inactive, declared by
this context rather than serialized as another mutable roster. Death does not
change activity or mo ownership. Actor source identities and SARG states remain
original ordinals. Policy6 permits normal attack states485–487; policy5 retains its
no-melee context and rejection. All previous v20/v21/v22 bytes and meanings remain.

`decode_player` validates exact policy/context pairs and their admitted definition/
state roles. Mixed or unknown pairs reject; replacing both identifiers with the
no-melee pair still rejects a captured SARG attack state. The old decode entry
continues rejecting22. Structural decoding supplies neither safe execution,
geometry admission, incarnation provenance nor executable-world restoration.

The [independent encoder](../tools/reference/m3g6_snapshot_goldens.py) checks source
roster/player/association rows and uses frozen generic field/event primitives.
The separately named source capture `DOOM-PLAYER-DEMON-MELEE-v1` and new PBE1 event
file preserve complete event order without rewriting older captures. Birth/generation
projection and invalid-input rejection remain Rust schema/safety policy. See
[source qualification](M3G6_REFERENCE.md), [reproduction](TESTING.md#autonomous-demon-player-melee)
and [observations](validation/M3G6.md).


## DSIM-v23 genuine-player pistol

V23 is required by player-owned persistent weapon/psprite observations. It retains
the v22 genuine-player field/event families and framing, selects wire policy7 and
exact context `PLAYER-DEMON-1-MEDIUM-GROUNDED-PISTOL-v1`, with unchanged registry
`DOOM-INFO-1.10-v1`. Earlier20/21/22 layouts, readers and policy meanings are
immutable. `Diagnostic::decode_pistol` reads23 explicitly; the historical decode
and decode_player entry points continue rejecting it.

After `original_player.0.frags`, before `events.last.count`, append these fields
in the stated order. Prefix is `original_player.0.pistol`:

| Suffix | Representation |
| --- | --- |
| present | u8 bool; if0, no remaining pistol fields follow |
| bullets | u16 source am_clip in0..200 |
| attack_down | u8 source bool |
| refire | i32 source counter in0..200 |
| bob | i32 raw source fixed-point value in0..0x100000 |
| extra_light | i32 source light0/1 |
| psprite.0.state, .tics, .x, .y | u16 original state or65535 forNULL; i32 countdown and raw fixed-point XY |
| psprite.1.state, .tics, .x, .y | Same fields, flash after weapon |

Immutable ready=Pistol, pending=nochange, owned=fist+pistol and original ammo
capacity derive from policy7 rather than another serialized player record.
Original body0, Demon12 and effects37/38 retain actual source IDs. Player mobj
attack states154/155 are executable under this capability; policy8 reuses them separately. Weapon overlays
10..17 and immediate LightDone1/Null0 are not actor states or thinker instances.

Persistent weapon states are source pistol ready/up/down/attack; flash is17 or
NULL. Timers are checked against canonical source descriptors. Null flash retains
a zero timer and source-maintained coordinates; it does not restore another
weapon. Decoding bounds bullets, refire, bob, light, coordinates, channel-specific
states and exact schema/policy pairs. Death retains the body and weapon ownership,
with a lowering pistol. Setup snapshots may have present0 before initialization.

Existing event tag14 weapon subtypes and payload widths remain unchanged. V23
validates the admitted pistol state/action identities, ammo changes, source
5/10/15 bullet damage, ordered draw categories and one-sector noise payload.
Source initialization and psprite entry are real events, not synthetic firing
requests. Tag22 Died remains the same boundary, now followed immediately by
actual tag14 lowering entries/actions before existing Kill/death-state events.
An ActorState Entered S_NULL event can mark an impact thinker for removal; v23
explicitly recognizes that sentinel without granting live S_NULL execution.

Read-only source aim queries are compared separately; no monster-aim event is
invented for them. Existing Attack/Trace/Impact/Damage and player/movement/AI events
encode actual source chronology. PBE1 remains compact independent test-evidence
framing, not a savegame format. Instance provenance and context tokens remain
excluded. Diagnostic decoding observes fields and identity boundaries; map,
physical values and other nested payloads retain their declared opaque limits.
It never restores executable worlds or grants actor ownership.

[Support](COMPATIBILITY.md#genuine-player-pistol-combat),
[source qualification](M3G7_REFERENCE.md), [reproduction](TESTING.md#genuine-player-pistol)
and [evidence](validation/M3G7.md) identify the independent bytes and limits.

## DSIM-v24 original player weapon expansion

Version24 selects policy8 (`PlayerDemonWeaponsV1`) and context
`PLAYER-DEMON-1-MEDIUM-GROUNDED-WEAPONS-v1`, with the original registry schema.
This separate layout is necessary because ready/pending/owned and shell counts
are dynamic; v23's immutable pistol assumptions remain unchanged. The explicit
`Diagnostic::decode_weapons` reader handles24; older readers reject it. V20–23
bytes, readers, event meanings and execution-policy/context pairs are preserved.

Replace the v23 pistol-only group with prefix `original_player.0.weapons`, after
`original_player.0.frags` and before `events.last.count`, in this order:

| Suffix | Representation |
| --- | --- |
| present | u8 bool;0 means no other weapon fields follow |
| ready, pending | u8 original weapon ordinal0..3; pending10 means wp_nochange |
| owned | u16 original weapon bits0..3; fist/pistol required; ready/pending must be owned |
| bullets, shells | u16 am_clip0..200, am_shell0..50 |
| attack_down | u8 bool |
| refire | nonnegative i32, checked on source increment |
| bob, extra_light | i32 raw fixed0..0x100000; i32 light0..2 |
| psprite.0.state, .tics, .x, .y | u16 original state or65535 NULL; i32 countdown/raw XY |
| psprite.1.state, .tics, .x, .y | Same, flash after weapon |

There are18 fields when present. Capacity and excluded cell/rocket ammo remain
immutable policy data. Original overlays are fist2–9, pistol10–17, shotgun18–31,
chaingun49–56, with immediate Null0/LightDone1. Persisted weapon states must match
ready and canonical positive timers; flash identity/light and coordinate bounds
are checked. Source Chain3 is zero-tic and appears only as an entry/action event.
Dead-player weapon state must be the actual ready downstate. No health, actor
binding, immutable original metadata or process-local provenance is duplicated.

Existing tag14 widths/order remain. Version24/policy8 admits the four named weapon
state/action closures, ammo0/1 decrements, lights0..2, nonnegative refire and fist
or bullet damage domains. Tag22 Died keeps its width and records the actual
ready weapon's downstate. V23 retains pistol-only validation. Other nested payloads
retain their documented bounded/opaque handling. Decoding validates diagnostic
observations, never restores an executable world or grants ownership.


## DSIM-v25 original world pickups

Version25 selects policy9 (`PlayerDemonPickupsV1`), original registry schema and
context `PLAYER-DEMON-1-MEDIUM-GROUNDED-PICKUPS-v1`. The explicit
`Diagnostic::decode_pickups` reader validates25; earlier readers reject it.
V20–24 bytes, policy/context meanings and readers are preserved. New persistent
pickup fields and ordered touch/resource observations justify this separate
layout; a milestone counter alone does not justify a version.

The existing18-field v24 weapon group remains. Immediately after it and before
`events.last.count`, encode prefix `original_player.0.pickups`:

| Suffix | Representation |
| --- | --- |
| present | u8=1 under the pickup policy |
| bonus_count | i32,0..96 from the bounded16-item budget |
| item_count | i32=0 for these non-COUNTITEM definitions |
| message | u8:0 none,1 clip,2 bullet box,3 shells,4 shell box,5 stimpack,6 medikit,8 shotgun,9 chaingun |

Message7 identifies source GOTMEDINEED but is unreachable in the positive-health
admitted slice and rejects. Immutable metadata, capacities, item kind, active
roster and provenance are not duplicated. Actors retain original type/state,
flags, spawnpoint and generation fields. The reader checks static pickup
state/sprite/frame, health, dimensions, flags, origin shape and physics fields.
It does not grant executable world restoration.

New event tag23 has a u8 subtype, followed by these little-endian fields:

| Subtype | Payload |
| --- | --- |
|0 Touch|item handle12, body handle12, sprite u16, delta/height/health i32 each |
|1 Ammo|ammo u8, accepted u8, before/after i32, ready/pending u8 |
|2 Body|accepted u8, before/after/mobj health i32 |
|3 Weapon|weapon/accepted u8, owned before/after u16, pending before/after u8 |
|4 Message|message u8 |
|5 Bonus|bonus_count/item_count i32 |
|6 Result|item handle12, outcome u8:0 unreachable,1 dead,2 not needed,3 collected |

Pending10 retains original wp_nochange; handles retain the existing slot u32 and
generation u64 representation. Widths, bool/enum domains and bounded handles are
checked before interpretation. Existing removal tag2 and source sound tag12/3
retain their meanings and occur between Message, Bonus and Result at source
boundaries. Source-only inactive item-respawn queue PQ rows are excluded from
DSIM, explicitly retained in original evidence. No respawn equivalence is claimed.
PBE1/DREFLZ1 remain test-evidence framing/storage, not another runtime format.

## Reference storage is outside DSIM

Selected raw C trace CSVs use standard zstd framing at the test-loading boundary;
this does not compress or redefine any DSIM payload. Every historical version,
policy, reader and complete-byte fixture retains its old meaning. Existing
DREFLZ1 evidence is unchanged. See [storage contract](EVIDENCE_STORAGE.md).


## DSIM-v26 original player armor

Version26 selects policy10 (`PlayerDemonArmorV1`), unchanged original registry
schema and context `PLAYER-DEMON-1-MEDIUM-GROUNDED-ARMOR-v1`.
`Diagnostic::decode_armor` explicitly reads26; all prior version-specific readers
retain their historical contracts and reject26. V20–25 bytes, encoders, fixtures
and policy/context meanings are unchanged. Diagnostic decoding is not executable
world restoration or actor ownership.

After the v25 pickup group and before events, add exactly two persistent fields:
`original_player.0.armor.type` u8 (0 none,1 green,2 blue) and `.points` i32.
None requires0, green1..100, blue1..200. No redundant presence, immutable definition,
body identity or health is added. Pickup message10=GOTARMOR and11=GOTMEGA are valid
only here. Animated item records validate current original state/sprite/frame and
positive tics bounded by that descriptor; older static-item validation remains.

Existing event23 gains subtype7 only under26: requested type u8, accepted u8,
before type u8/points i32, after type u8/points i32 (14 bytes including tag/subtype).
The reader checks typed bounds and source grant acceptance/replacement. Event24
has no subtype: target handle12, incoming/saved/effective i32 each, before type
u8/points i32, after type u8/points i32 (35 bytes including tag). It occurs after
raw thrust and before player health observations, only for live player armor work,
including None/0 or raw0. The reader checks the owned body, raw0..1000, exact
integer absorption, depletion class and resulting points. It grants no lifetime
lease and does not recompute past gameplay.

Independent C rows AX/AR and grant observations feed the Python
[armor projection](../tools/reference/armor_snapshot.py); independent full bytes
and PBE1 ordered events precede matching Rust changes. Source-only inactive item
respawn queue observations remain excluded. DREFLZ1/PBE1 are evidence storage,
not another runtime diagnostic version.

## DSIM-v27 original health and armor bonuses

Version27 selects policy11 (`PlayerDemonBonusesV1`), the unchanged original registry
schema and context `PLAYER-DEMON-1-MEDIUM-GROUNDED-BONUSES-v1`.
`Diagnostic::decode_bonuses` explicitly reads27; every earlier version-specific
reader keeps its historical contract and rejects27. V20–26 bytes, encoders,
fixtures and policy/context meanings are unchanged.

V27 adds no persistent field. It widens the meaning of two existing fields:
`level.total_items` may be0..16 and `original_player.0.pickups.item_count`
0..total. A nonzero total must equal the placed DoomEd2014/2015 THINGs that pass
the Medium single-player filter (`options & 2` set, `options & 16` clear); zero also
describes a constructed world before its THING batch. Live player health may be
1..200 and green armor1..200, only under27. Pickup message12=GOTHTHBONUS and
13=GOTARMBONUS are valid only here. Bonus item records validate their current
state within the six-state loop, sprite/frame and positive tics.

New transient events, valid only under27:

| Tag/subtype | Payload | Meaning |
| --- | --- | --- |
| 23/8 | before i32, after i32, body i32 | Health bonus; after = min(before+1, 200) = body |
| 23/9 | before type u8/points i32, after type u8/points i32 | Armor bonus; +1 capped at200, type0 becomes1 |
| 23/10 | before i32, after i32 | Player item_count increment, after the message and before removal |
| 21/2 | actor (u32 slot, u64 generation), total i32 | Map totalitems increment at counted spawn |

Independent C rows feed the Python
[bonus projection](../tools/reference/bonus_snapshot.py); independent full bytes
and PBE1 ordered events preceded the matching Rust changes. Diagnostic decoding
remains observation, not executable world restoration.

## DSIM-v28 original former humans and mixed rosters

Version28 selects policy12 (`PlayerMonstersV1`), the unchanged original registry
schema and context `PLAYER-MONSTERS-1-MEDIUM-GROUNDED-v1`.
`Diagnostic::decode_monsters` explicitly reads28; every earlier version-specific
reader keeps its historical contract and rejects28, and the v28 reader rejects
every earlier version. V20–27 bytes, encoders, fixtures and policy/context
meanings are unchanged.

V28 uses the DSIM27 field layout and adds no persistent field or event tag. It
widens meanings only under28:

- Actor `type`/`state` and state-bearing event payloads may name MT_POSSESSED=1,
  MT_SHOTGUY=2 and MT_CHAINGUY=10 within their admitted normal closures.
- `original_player.0.attacker` may name any roster monster.
- World items may number at most 16 placed items plus one MF_DROPPED
  MT_CLIP/MT_SHOTGUN/MT_CHAINGUN per roster monster; drops have no spawnpoint.
- `level.total_kills` is 0..8. A nonzero total must equal the placed DoomEd
  3002/3004/9/65 THINGs that pass the Medium single-player filter (`options & 2`
  set, `options & 16` clear); `level.single_player_kill_count` is 0..total.
  The same filter function counts DoomEd2014/2015 for `level.total_items`.

Existing event encodings carry the new behavior: shotgunner Begin/PelletStart/
Pellet/PelletDone/Completed (tag19), MonsterEvent Aim/Random/Face (tag18), refire
decisions (tag20), combat retarget/kill/death-timer (tag13), the grouped
`A_Scream` draw as ActorStateEvent Random (12/7) and drop spawns (tags1 and 12/0).

Independent C rows feed the Python
[former-human projection](../tools/reference/monsters_snapshot.py); its full bytes
and PBE1 ordered events preceded the matching Rust changes. Decoding remains
observation, not executable world restoration.

## DSIM-v29 original Imps and fireballs

Version29 selects policy13 (`PlayerMonstersImpV1`), the unchanged original registry
schema and context `PLAYER-MONSTERS-IMPS-1-MEDIUM-GROUNDED-v1`.
`Diagnostic::decode_imps` explicitly reads29; every earlier version-specific reader
keeps its historical contract and rejects29, and the v29 reader rejects every earlier
version. V20–28 bytes, encoders, fixtures and policy/context meanings are unchanged.

V29 uses the DSIM28 field layout and adds no persistent field or event tag. A
fireball is an ordinary actor record. It widens meanings only under29:

- Actor `type`/`state` and state-bearing event payloads may name MT_TROOP=11 and
  MT_TROOPSHOT=31 within their admitted normal closures.
- `original_player.0.attacker` may name an Imp.
- At most 24 nonpending actors may have type 31, and every `pending_removal` is 0 or 1.
- A nonzero `level.total_kills` must equal the placed DoomEd
  3001/3002/3004/9/65 THINGs that pass the Medium single-player filter. The v28 set
  stays without 3001.

Tag15 projectile events, which earlier original-ID versions never carried, are
validated under29:

| Sub-tag | Meaning | Payload after tag and sub-tag |
| --- | --- | --- |
| 0 | Explosion, before momentum clears | fireball handle |
| 1 | Launch velocity | fireball handle, u32 angle, three i32 momentum |
| 2 | Draw | u8 kind (0 spawn, 1/2 shadow, 3 contact, 4 explosion), u8 value |
| 4 | Half-step, before the spawn move | fireball handle, three i32 position, i32 tics ≥1 |
| 5 | Validated launch | Imp handle, u8 aim 0, target handle, u16 type 31 |

Independent C rows feed the Python
[Imp projection](../tools/reference/imps_snapshot.py); its full bytes and PBE1 ordered
events preceded the matching Rust changes. Decoding remains observation, not
executable world restoration.

## DSIM-v30 original monsters at real-map scale

Version30 selects policy14 (`PlayerMonstersMapScaleV1`), the unchanged original
registry schema and context `PLAYER-MONSTERS-MAP-SCALE-1-MEDIUM-GROUNDED-v1`.
`Diagnostic::decode_map_scale` explicitly reads30; every earlier version-specific
reader keeps its historical contract and rejects30, and the v30 reader rejects every
earlier version. V20–29 bytes, encoders, fixtures and policy/context meanings are
unchanged, including their 64-slot bounds.

V30 uses the DSIM29 field layout, event vocabulary and tag15 projectile events, and
adds no persistent field or event tag. It widens bounds only under30:

- `store.max_slots` and `store.slot_count` may reach 320, and handle slots in every
  field and event payload are bounded by 320 instead of 64.
- At most 208 nonpending actors may have type 31; every `pending_removal` is 0 or 1.
- Actor coordinates are bounded by the registry's 2,048-unit domain; the walled
  rectangle is derived from `map.vertices` by the runtime and is not a stored field.
- `level.total_kills` and the counted DoomEd set are those of v29.

Independent C rows feed the Python
[map-scale projection](../tools/reference/map_scale_snapshot.py); its full bytes and
PBE1 ordered events preceded the matching Rust changes. Decoding remains observation,
not executable world restoration.

## DSIM-v31 original monsters with static heights

Version31 selects policy15 (`PlayerMonstersHeightsV1`), the unchanged original
registry schema and context `PLAYER-MONSTERS-HEIGHTS-1-MEDIUM-v1`.
`Diagnostic::decode_heights` explicitly reads31; every earlier version-specific
reader keeps its historical contract and rejects31, and the v31 reader rejects every
earlier version. V20–30 bytes, encoders, fixtures and policy/context meanings are
unchanged.

V31 uses the DSIM30 field layout and event vocabulary and adds no persistent field or
event tag: the source state that heights need (`z`, `momz`, `floor`, `ceiling`, the
subsector/sector location and every `sector.{i}.soundtarget`) has been in the layout
since v3 and v6. It changes the item witness only under31:

- A world item's `z` equals its `floor`, and its `floor`/`ceiling` equal the
  `sector.{i}.floor`/`sector.{i}.ceiling` of the sector whose `sector.{i}.actors`
  list holds the item's handle, instead of `0`/`128`.
- The 320-slot store, the 208-fireball bound and the 2,048-unit coordinate domain of
  v30 apply unchanged.

Newly reachable event payloads under31 use existing tags: a `Sound` actor-state
event with sound 34 (`sfx_oof`) immediately before the `Vertical` event of a landing
player, `Noise` weapon events for every sector the `P_RecursiveSound` flood visits (in
source order, with `blocks` 0 or 1), `Vertical` events with floor or ceiling contact
for falling, stepping and exploding actors, and `SectorMovement` attempts refused by
the fit, headroom, step and dropoff rules.

Independent C rows feed the Python
[heights projection](../tools/reference/heights_snapshot.py); its full bytes and
PBE1 ordered events preceded the matching Rust changes. Decoding remains observation,
not executable world restoration.

## DSIM-v32 original monsters among diagonal walls

Version32 selects policy16 (`PlayerMonstersDiagonalV1`), the unchanged original
registry schema and context `PLAYER-MONSTERS-DIAGONAL-1-MEDIUM-v1`.
`Diagnostic::decode_diagonal` explicitly reads32; every earlier version-specific
reader keeps its historical contract and rejects32, and the v32 reader rejects every
earlier version. V20–31 bytes, encoders, fixtures and policy/context meanings are
unchanged.

V32 uses the DSIM31 field layout, event vocabulary, item witness, store and fireball
bounds and coordinate domain and adds no persistent field or event tag: a slide's
outcome is already carried by the mover's `x`, `y`, `momx`, `momy`, `subsector` and
`sector` fields and the `Slide` event's line index, and the boundary and its parity
test are derived from the immutable map at load, never state. `MapScaleError::Escape`
is a checked error of the stepping call, not an event. Newly reachable payloads use
existing tags: `Slide` events naming diagonal one- and two-sided lines, `SectorMovement`
attempts refused at diagonal lines, hitscan impacts and fireball explosions at diagonal
walls, and `Noise` floods through diagonal openings.

Independent C rows feed the Python
[diagonal projection](../tools/reference/diagonal_snapshot.py). Because the M3h5 rooms
carry a real BSP with several subsectors per sector, that layer derives each
`actor.{i}.sector` and `sector.{i}.actors` from the subsector's first seg through the
map lumps instead of equating sector and subsector indices; `actor.{i}.subsector` keeps
the C leaf. Its full bytes and PBE1 ordered events preceded the matching Rust changes.
Decoding remains observation, not executable world restoration.

## DSIM-v33 original monsters at classic-map scale

Version 33 selects policy 17 (`PlayerMonstersRealMapV1`), the unchanged original
registry schema and context `PLAYER-MONSTERS-REALMAP-1-MEDIUM-v1`.
`Diagnostic::decode_real_map` explicitly reads 33; every earlier version-specific
reader keeps its historical contract and rejects 33, and the v33 reader rejects every
earlier version. V20–32 bytes, encoders, fixtures and policy/context meanings are
unchanged.

V33 uses the DSIM32 field layout, event vocabulary, item witness, store and fireball
bounds and adds no persistent field or event tag: raising the static-geometry caps adds
no actor or sector state, the arithmetic and slide-window certificates and the
per-sector line lists are derived from the immutable map at load, and
`MapScaleError::{Arithmetic, Intercepts}` are checked errors of the constructor and the
stepping call, not events. What grows is the data already carried: `sectors.count` and
the `sector.{i}.*` tables of a 74-sector room, `map.*` lumps of hundreds of linedefs,
positions anywhere inside a rectangle up to the format's range. The reader checks those
against the stream's own tables instead of the DSIM32 constants: the THINGS lump is
bounded by the format's 32,768 records instead of 256, the slot bound stays 320, and
item sectors are read from `sectors.count` as in v31 and v32.

Independent C rows feed the Python
[real-map projection](../tools/reference/real_map_snapshot.py), which extends the
diagonal projection without editing it and drops the `TI` observer rows (intercept
counts) before the chain. Its full bytes and PBE1 ordered events preceded the matching
Rust changes. Decoding remains observation, not executable world restoration.

## DSIM v34 — original monsters with sky and static specials

Version 34 selects policy 18 (`PlayerMonstersSpecialsV1`), the unchanged original
registry schema and context `PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1`. A
`Diagnostic::decode_specials` reader reads 34 only; every earlier version-specific
reader keeps its historical contract and rejects 34. V20–33 bytes, encoders, fixtures
and policy/context meanings are unchanged.

V34 is the DSIM33 layout, event vocabulary, item witness, store and fireball bounds
plus exactly the state of the sky and static sector specials
([investigation](M3H7_REFERENCE_INVESTIGATION.md#42-stage-a-m3h7-sky-and-static-specials)).
Every DSIM33 field keeps its path, width, position and meaning; in particular
`thinker.order`, `store.head`, `store.tail` and each actor's `previous`/`next` still
list **actors only** (the actor subsequence of the shared order below). The new fields
are inserted at four places, in this order of the stream:

| Position | Field | Value encoding |
| --- | --- | --- |
| right after `thinker.order` | `thinker.shared_order` | the one thinker list in processing order: 13-byte entries, u8 kind (0 actor, 1 sector thinker) + u32 slot + u64 generation |
| right after the last `actor.N.*` record, before `spatial.link_order` | `sector_thinkers.slot_count` | u64 logical sector-thinker slot count |
| | for each slot `j`: `sector_thinker.{j}.generation`, `.occupied` | u64, u8 boolean |
| | occupied slot: `.kind`, `.sector` | u8 1 flash, 2 strobe, 3 glow, 4 flicker; u16 sector index |
| | occupied slot except a glow: `.count` | i32 tics left before the next change |
| | occupied slot: `.minlight`, `.maxlight` | i32, i32 |
| | flash: `.maxtime`, `.mintime`; strobe: `.darktime`, `.brighttime`; glow: `.direction`; flicker: nothing more | i32 64, 7; i32 15 or 35, 5; i32 −1 down, 1 up |
| after each `sector.{i}.soundtarget` | `sector.{i}.light`, `sector.{i}.special` | i16 runtime `lightlevel`, i16 runtime `special` |
| right after `original_player.0.armor.points` | `original_player.0.secret_count` | i32 `player->secretcount` |

So a sector group reads `floor`, `ceiling`, `actors`, `soundtarget`, `light`,
`special`, and the player block ends `… armor.type`, `armor.points`, `secret_count`,
then `events.last.count`. Field paths of the sector-thinker store use the same decimal
slot numbering as `actor.N`. No other field is added, removed or moved.

Meanings:

- **Shared order.** `thinker.shared_order` lists every actor of `thinker.order`
  (pending removals included) and every occupied sector thinker exactly once, in the
  order `P_RunThinkers` visits them. A sector thinker is appended when it is created,
  like an actor; in this policy all of them are created by the one load operation that
  follows every THING spawn and the player body (`P_SpawnSpecials` in sector-index
  order), so the list is the load actors, then the sector thinkers, then every later
  actor. The kind byte selects the handle namespace: kind 0 entries are actor
  identities, kind 1 entries are sector-thinker identities (slot, generation) in their
  own store. Neither namespace is reused for the other.
- **Sector-thinker store.** A separate slot class, bounded at load: its slots are
  allocated in creation order, so slot `j` is the `j`-th thinker `P_SpawnSpecials`
  creates, with generation 1. No sector thinker is created after load or removed in
  this policy, so `slot_count` is the number of light sectors (at most `sectors.count`),
  every slot is occupied and every generation stays 1. The fields are the source
  structures of [p_spec.h](../linuxdoom-1.10/p_spec.h): `lightflash_t` (flash, sector
  special 1), `strobe_t` (strobe, specials 2, 3, 4, 12, 13), `glow_t` (glow, 8; it has no
  count) and `fireflicker_t` (flicker, 17). `minlight`/`maxlight` are the values the
  spawner computed (a strobe whose darkest neighbor equals its own light has
  `minlight` 0).
- **Sector light and special** are the runtime values; the `map.sectors` lump keeps the
  original ones. A light sector's special is 0 after load (the spawners clear it, special
  4 is set back to 4), and a secret sector's special becomes 0 at its entry.
- **Secrets.** `level.total_secrets` is `P_SpawnSpecials`' `totalsecret`: the number of
  `map.sectors` records whose special is 9; it does not change when a secret is found.
  `original_player.0.secret_count` is 0 to `level.total_secrets`.
- **Time.** `tic` equals the source `leveltime` after the tic. During the step that
  completes tic *t* the source runs with `leveltime` = *t* − 1, so the damage floors hurt
  in the steps completing tics 1, 33, 65, … (`!(leveltime & 0x1f)`).

Events. One new tag, 25 (specials), with three sub-tags:

| Sub-tag | Meaning | Payload after tag and sub-tag | Position |
| --- | --- | --- | --- |
| 0 | Secret entry | u16 sector, i32 `secret_count` after the increment | in the player phase, after the player's tag-4 command event and before its weapon (tag 14) events, where `P_PlayerInSpecialSector` runs |
| 1 | Fireball removed at a sky ceiling | fireball identity, u16 `ceilingline` | right after the refused movement attempt (tag 10) and before the removal mark (tag 2); there is no explosion event and no draw |
| 2 | Hitscan ended at a sky ceiling, no puff | u8 reason (0 `z` above the front sector's ceiling, 1 back sector also sky), u16 line, i32 `z` (16.16) | right after the trace record (tag 13 sub-tag 1) of the hitscan it ended, whose hit is absent |

Environmental damage needs no new tag: it is the existing combat damage event (tag 13
sub-tag 2) with absent inflictor and source, followed by the existing armor, player-body
and death events, in the player phase at the position of sub-tag 0. An environmental
death credits the player's own frag through the existing player event. There is **no**
event for a light change (it is visible in `sector.{i}.light`, owner decision 5), for a
sector thinker's run (only actors emit tag 5), for a light draw or a light spawn draw
(they advance `rng.gameplay` only, never tag 7), for a call of `P_PlayerInSpecialSector`
that does nothing, for a missile refusal that explodes, or for a scrolling wall (its
offset is derived from `tic`). This policy has no sector sound. Without its tag-25
records the event stream is exactly what the DSIM33 vocabulary gives for the same
source behavior; the independent projection asserts this.

Independent C rows feed the Python
[specials projection](../tools/reference/specials_snapshot.py), which extends the
real-map projection without editing it. It takes the shared order and the sector-thinker
store from the `TK` rows, the runtime light, special and secret count from the `SX`
rows, and the tag-25 events from the `SE`, `KR` (with its `KY` line) and `KS` rows; the
other new observer rows are not events. Its full bytes and PBE1 ordered events preceded
the matching Rust changes. Decoding remains observation, not executable world
restoration.

## DSIM v35 — original doors, lifts, floors, switches and the exit

`PlayerMonstersMoversV1` captures are version 35; `Diagnostic::decode_movers` is its
explicit reader. The Rust writer reproduces the independent Python projection
([movers projection](../tools/reference/movers_snapshot.py) and
[its generator](../tools/reference/m3h8_snapshot_goldens.py)) byte for byte at every
committed capture, and its event stream frame for frame
([full replay](TESTING.md#original-doors-lifts-floors-switches-and-the-exit-c-evidence)).

Version 35 selects policy 19 (`PlayerMonstersMoversV1`, wire 19), the unchanged original
registry schema and context `PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1`. Every earlier
version-specific reader, `decode_specials` included, keeps its historical contract and
rejects 35; the v35 reader reads 35 only. V20–34 bytes, encoders, fixtures and
policy/context meanings are unchanged.

V35 is the DSIM34 layout, event vocabulary, store and fireball bounds plus exactly the
Stage B state of the
[investigation](M3H8_REFERENCE_INVESTIGATION.md#10-dsim35--wire-19); the item witness a
v35 reader applies is the relaxed one of
[§4.2](M3H8_REFERENCE_INVESTIGATION.md#42-b3-the-relaxed-item-witness), since sector
heights now move. Every DSIM34 field
keeps its path, width and relative position; `thinker.order`, `store.*` and each actor's
`previous`/`next` still list actors only. Changes, in stream order:

| Position | Field | Value encoding |
| --- | --- | --- |
| right after `level.single_player_kill_count` | `level.exit` | u8 boolean: `G_ExitLevel` ran (`gameaction == ga_completed`; `secretexit` is always false) |
| `thinker.shared_order` (same place) | now also lists movers | 13-byte entries as in v34; doors, plats, floors and removed movers are kind 1 (sector-thinker) entries |
| sector-thinker store (same place) | `sector_thinkers.slot_count` and every slot `j` < count | see below; a slot may now be free |
| `sector.{i}.floor`, `sector.{i}.ceiling` (same place) | the runtime heights | i32 16.16, moved by `T_MovePlane` |
| right after the last `sector.{i}.special`, before `player.0.actor` | `line.{i}.special` for every line whose `map.linedefs` special is nonzero, in line order | i16 runtime `line->special` (W1 and S1 lines read 0 once cleared) |
| right after the line specials, before `player.0.actor` | `button.{k}.btimer` for every `buttonlist` slot k = 0..15 (`MAXBUTTONS`), each followed, when nonzero, by `button.{k}.line` and `button.{k}.soundorg` | i32 `btimer` (0: a free, zeroed slot; else 1..35); u16 line; u16 the sector whose `soundorg` the slot names (`P_StartButton`: the line's front sector) |
| right after `original_player.0.secret_count` | `original_player.0.use_down` | u8 boolean `player->usedown` |

The sector-thinker store:

| Slot state | Fields after `.generation` (u64) and `.occupied` (u8) |
| --- | --- |
| free | none; `.generation` is the generation its next occupant receives |
| light (kinds 1–4) | exactly the v34 record |
| door (kind 5) | `.sector` u16, `.type` u8 (`vldoor_e`: 0 `normal`, 6 `blazeOpen`), `.direction` i32 (−1, 0, 1), `.topheight` i32 16.16, `.topcountdown` i32 |
| plat (kind 6) | `.sector` u16, `.status` u8 (`plat_e`: 0 up, 1 down, 2 waiting), `.count` i32, `.low` i32, `.high` i32 (16.16) |
| floor (kind 7) | `.sector` u16, `.type` u8 (`floor_e`: 1 `lowerFloorToLowest`, 2 `turboLower`), `.floordestheight` i32 16.16 |
| removed mover (kind 8) | `.sector` u16 (the sector it moved) |

Implied and omitted mover fields (decision N6, mover allocations zero-filled): a door's
`speed` (2 units for `normal`, 8 for `blazeOpen`) and `topwait` (150); a plat's `type`
(`downWaitUpStay`), `speed` (4), `wait` (105), `crush` (false), `tag` (its sector's tag)
and `oldstatus` (never written, 0); a floor's `direction` (−1), `speed` (1 for
`lowerFloorToLowest`, 4 for `turboLower`), `crush` (false), `newspecial` and `texture`
(never written, 0). `topcountdown` reads 0 until the door first waits. The projection
asserts every implied value against the C record. Not encoded: `sector->specialdata`
(derived: the one live door, plat or floor record naming the sector, or none; lights
never set it; the projection checks it against the C value of every sector), the
`activeplats` slots, a button's `where` and `btexture` and every switch texture
(presentation, M3h7 decision 5; the button timers, lines and sound origins are encoded
above, decision N9) and the `P_ChangeSector` working flags.

Thinker records and their order:

- **Creation.** A mover is appended to `thinker.shared_order` when its `EV_*` body
  creates it, after every thinker created before it (`P_AddThinker`): a door opened by a
  use joins in the players phase and steps in the same tic; a mover created by a crossing
  or a monster's use joins during the thinker pass and steps later in the same pass.
- **Slots.** Light thinkers keep the load slots 0..L−1, generation 1, for the whole
  level. A new mover takes the **lowest free slot**, or slot `slot_count` (which then
  grows by one, generation 1) when none is free; `slot_count` never shrinks.
- **Removal.** When a mover finishes (`pastdest`, or `P_RemoveActivePlat` for a lift),
  `P_RemoveThinker` marks it: its slot stays occupied as kind 8, its entry keeps its place
  in the shared order and the sector's `specialdata` is cleared at once, so a later thinker
  of the same pass may create a replacement for the same sector (a sector then has two
  entries, the removed one first). When the next pass reaches the marked entry it is
  unlinked: the entry leaves the shared order, the slot becomes free and its generation
  grows by one.
- At most one live (kind 5–7) record names a sector at any time.

Reborn request (decision N3): `BT_USE` held by a dead player sets the source
`playerstate` to `PST_REBORN`, encoded as the existing `original_player.0.state` with the
newly reachable value 2. The exit and the reborn request both refuse the next `advance`;
the capture after the requesting tic is the last of the level.

Events. One new tag, 26 (movers). Identities are 12-byte actor IDs (u32 slot + u64
generation) for things and 12-byte sector-thinker IDs (u32 slot + u64 generation of the
store above) for movers; heights, `frac` and speeds are i32 16.16.

| Sub-tag | Meaning | Payload after tag and sub-tag | Position |
| --- | --- | --- | --- |
| 0 | Use started (`P_UseLines`) | player actor ID | players phase, after the weapon events, where the use block of `P_PlayerThink` runs |
| 1 | Use intercept (`PTR_UseTraverse` on a line) | u16 line, i32 `frac`, u8 continues (0 stops the trace: a special line, or `sfx_noway` at a closed line) | after the records of what that line's use did |
| 2 | Special line used (`P_UseSpecialLine`) | user actor ID, u16 line, u8 side, i16 special before, u8 result, i16 special after | after the records of the line action (mover starts and calls, sounds, the exit); a monster's use follows sub-tag 4 |
| 3 | Line crossed (`P_CrossSpecialLine`) | crosser actor ID, u16 line, u8 old side, i16 special before, i16 special after | right after the movement-attempt event of the `P_TryMove` that crossed, after the records of the line action; several crossings of one move follow in the source's reverse `spechit` order |
| 4 | Blocked with special lines (`P_Move` failure branch, `movedir = DI_NODIR`) | actor ID, u8 `numspechit` (1–8) | after the failed movement attempt, before that monster's sub-tag 2 uses |
| 5 | Mover started | mover ID, u8 kind (5 door, 6 plat, 7 floor), u16 sector | at the end of its `EV_*` call, one per created mover in creation order, after that call's sounds |
| 6 | Mover call (`EV_DoDoor` 1, `EV_VerticalDoor` 2, `EV_DoPlat` 3, `EV_DoFloor` 4) | u8 function, u16 line, i32 argument (door/plat/floor type; the line special for 2), i32 result (−1 for 2), u8 movers created | right after its sub-tag 5 records; a door reversed by `EV_VerticalDoor` creates none |
| 7 | Plane step (`T_MovePlane`) | u16 sector, u8 plane (0 floor, 1 ceiling), i32 direction, i32 speed, i32 destination, i32 height before, i32 height after, u8 result (0 ok, 1 crushed, 2 pastdest) | in the thinker pass at the mover's entry, after that step's sub-tag 8 records |
| 8 | Sector changed (`P_ChangeSector` exit) | u16 sector, u8 `nofit` | after the sub-tag 9 records of its things; a crushed step that moves back has two |
| 9 | Thing clipped (`PIT_ChangeSector`) | actor ID, u8 outcome (0 fits, 1 corpse to `S_GIBS`, 2 dropped item removed, 3 non-shootable left as is, 4 `nofit`), i32 `z`, i32 `floorz`, i32 `ceilingz` after `P_ThingHeightClip` | after the existing pickup, state-entry or removal events the clip caused, in the source's blockbox and link order |
| 10 | Mover removed (`P_RemoveThinker`) | mover ID | where the mover finishes |
| 11 | Mover unlinked | mover ID | in the thinker pass where its marked entry is reached |
| 12 | Sector sound | u8 origin (1 sector `soundorg`, 2 the address of `buttonlist[index].soundorg`), u16 index, u16 sound | where `S_StartSound` runs; a button's restore sound (origin 2, only `P_UpdateSpecials` makes it) is in the Specials phase, after that phase's record |
| 13 | Exit (`G_ExitLevel`) | u8 secret exit (always 0) | inside the S1 exit switch's use, after the switch sound and before that use's sub-tag 2 record |
| 14 | Reborn requested | u8 player slot (0) | last record of the players phase of the tic whose dead player held `BT_USE` |

A height clip's own `P_CheckPosition` candidates and openings are not events; pickups,
`S_GIBS` state entries and removals it causes keep their existing events. Sounds with a
thing or no origin (`sfx_noway` at the player, the global switch sound when the button
slot has none) keep the existing actor-state sound event. There is no event for a mover's
run without a step (a waiting door or plat), for a `spechit` append, for `activeplats` or
button bookkeeping (the slots are state), or for a switch texture change. Without its tag-26 records the event
stream is exactly what the DSIM34 vocabulary gives for the same source behavior; the
independent projection asserts this.

Independent C rows feed the Python
[movers projection](../tools/reference/movers_snapshot.py), which extends the specials
projection without editing it. It takes the heights from `MH`, the line specials from
`ML`, the button slots from `MB`, `use_down`, the exit and the player state from `MZ` and `P`, the shared order and
the records from `TK` with the slots from `MM` and `SF`, and the tag-26 events from `MA`,
`MI`, `MU`, `MX`/`MY`, `MP`, `MM`, `ME`, `MV`, `MC`, `HC`/`MT`, `SD`, `SF`, `SO` and `MG`
rows. Its full bytes and PBE1 ordered events were committed before the matching Rust
changes. Decoding remains observation, not executable world restoration.

The v35 reader applies every DSIM34 check that still holds (identity, schema, handles,
pickups with the relaxed item witness of
[§4.2](M3H8_REFERENCE_INVESTIGATION.md#42-b3-the-relaxed-item-witness), weapons, armor,
bonus statistics, live fireballs, light records, sector lights and specials, the secret
count) and checks the v35 fields against the encoded map: store slots at most three per
sector; light records at generation 1, one per light sector of its kind; door, plat
and floor records within their field domains with at most one live mover per sector;
every occupied slot once in `thinker.shared_order` at its generation and the actor
subsequence equal to `thinker.order`; `line.{i}.special` present exactly for the lines
with a nonzero map special, equal to it or 0 on a W1 (36, 38, 109) or S1 (11) line; the
16 button slots (a running one on a distinct SR 63 line, timer 1..35, `soundorg` the
line's front sector; a free one with no further fields); booleans `use_down` and
`level.exit`; `original_player.0.state` 2 only with health 0; tag-26 payload widths and
their lines, sectors, handles and small domains; and `S_GIBS` actors (monsters or the
player, admitted only by this reader) with height and radius 0, `MF_SOLID` clear and
health at most 0.

## DSIM v36 — original two-sided scrollers and static decorations

`PlayerMonstersE1M1V1` captures are version 36; `Diagnostic::decode_e1m1` is its
explicit reader. The independent Python projection
([E1M1 projection](../tools/reference/e1m1_snapshot.py) and
[its generator](../tools/reference/m3h9_snapshot_goldens.py)) fixed every committed
capture and the PBE1 event stream from the C rows alone before any Rust writer existed
([plan](M3H9_REFERENCE_INVESTIGATION.md#9-implementation-plan-one-commit-per-step-pushed-evidence-before-runtime),
step 3). The Rust writer needed no change to produce them: the full replay matches every
capture's digest, size and field count, the 15 pinned fixtures byte for byte and every
event frame ([observations](validation/M3H9.md)).

Version 36 selects policy 20 (`PlayerMonstersE1M1V1`, wire 20), the unchanged original
registry schema and context `PLAYER-MONSTERS-E1M1-1-MEDIUM-v1`. Every earlier
version-specific reader, `decode_movers` included, keeps its historical contract and
rejects 36; the v36 reader reads 36 only. V20–35 bytes, encoders, fixtures and
policy/context meanings are unchanged.

V36 is the DSIM35 layout, event vocabulary, item witness, store and fireball bounds with
**no new field and no new event**
([decision E8](M3H9_REFERENCE_INVESTIGATION.md#6-new-owner-decisions-e1e8)). Every
DSIM35 field keeps its path, width, position and meaning, and every event keeps its tag,
payload and position. A v36 capture is byte for byte the DSIM35 layout of the same state
except the version word, the `profile` value (20) and the `context.schema` bytes; the
generator asserts this for every capture of the M3h9 rooms, so a capture of a map
without decorations differs from a DSIM35 capture in nothing else. What changes is what
the layout may contain.

**Static decorations** are ordinary actor records. The policy admits the seven E1M1
decoration types by DoomEd number with the pristine `info.c` values
([investigation §2.5](M3H9_REFERENCE_INVESTIGATION.md#25-the-seven-e1m1-decoration-types-infoc));
the u16 `type` and `state` values are the `mobjtype_t` and `statenum_t` indices of
[info.h](../linuxdoom-1.10/info.h), as for every actor:

| DoomEd | `type` | Spawn `state` | `radius` / `height` (units) | info `flags` |
| --- | --- | --- | --- | --- |
| 2028 | `MT_MISC31` (81), floor lamp | `S_COLU` (886) | 16 / 16 | `MF_SOLID` |
| 48 | `MT_MISC48` (98), tall techno pillar | `S_TECHPILLAR` (916) | 16 / 16 | `MF_SOLID` |
| 35 | `MT_MISC50` (100), candelabra | `S_CANDELABRA` (912) | 16 / 16 | `MF_SOLID` |
| 24 | `MT_MISC71` (121), pool of blood and flesh | `S_GIBS` (895) | 20 / 16 | 0 |
| 15 | `MT_MISC62` (112), dead player | `S_PLAY_DIE7` (164) | 20 / 16 | 0 |
| 10 | `MT_MISC68` (118), bloody mess | `S_PLAY_XDIE9` (173) | 20 / 16 | 0 |
| 12 | `MT_MISC69` (119), bloody mess | `S_PLAY_XDIE9` (173) | 20 / 16 | 0 |

A decoration record reads `type` and `state` from the table (the state never changes:
`tics` −1, no action), `health` 1000, `reaction_time` 8, `radius` and `height` from the
table (16.16), `flags` equal to the info flags, plus `MF_AMBUSH` (32) when the THING
record carries `MTF_AMBUSH` (the bit survives into the flags word), `lastlook` from the
one spawn draw (0..3), `spawnpoint` present with the THING record (x, y, angle, DoomEd
type, options), `target` absent, `momx`/`momy`/`momz` 0, `z` equal to `floor` (spawned
`ONFLOORZ`; a mover's clip keeps it on the floor, and `floor` may exceed `ceiling` while
a door is shut on a decoration whose box straddles a higher floor, as
`P_ThingHeightClip` leaves it; no reader orders the two), `threshold`, `movedir` and `movecount`
0, `player` 255 and `pending_removal` 0: a decoration is never removed, damaged, gibbed,
pushed, targeted or sight-checked. It holds a slot of the one actor store (320 slots),
appears once in `thinker.order`, `thinker.shared_order` (kind 0), `spatial.link_order`
and its sector's `actors`, and runs each tic (the existing tag-5 thinker update) doing
nothing. At most 32 decorations per map (`MAX_DECORATIONS`; E1M1 has 27 on every skill).

**Order.** `P_SpawnMapThing` spawns one actor per admitted THING record in lump order
(a record filtered by skill or bit 16 spawns nothing and draws nothing, as before), the
player body is spawned (by the harness after the lump in the M3h9 rooms, whose lumps hold
no player start; a lump's own player start spawns it where the lump lists it), then
`P_SpawnSpecials` creates the sector thinkers: `thinker.order` at load is the spawned
records in lump order followed by the player body, decorations interleaved with monsters
and pickups exactly as the lump lists them. A decoration is never removed, so the decoration subsequence of
`thinker.order` stays the lump order for the whole level.

**`S_GIBS`.** A decoration of type `MT_MISC71` spawns in `S_GIBS` with height 16, radius
20, health 1000 and flags 0 (or `MF_AMBUSH`), unlike a corpse crunched by a mover
(`PIT_ChangeSector`: `S_GIBS` with height and radius 0, `MF_SOLID` clear, health at most
0). The v36 reader keys the `S_GIBS` rule by type: `MT_MISC71` keeps its info values; a
monster or the player in `S_GIBS` keeps the DSIM35 crunch rule.

**Two-sided scrolling walls (48).** A type-48 special on a two-sided linedef (blocking or
not) is `line.{i}.special` = 48, exactly as any nonzero map special in DSIM35; it is never
cleared (no W1/S1 path) and its texture offset (`leveltime × FRACUNIT`, `P_UpdateSpecials`)
is derived, presentation only and never encoded. Nothing new is encoded for scrollers:
a thing crossing the line gives the existing tag-26 sub-tag 3 record with special 48
before and after; a monster's failed move with 48 lines in `spechit` gives sub-tag 4 and,
per line, sub-tag 2 with result 0; a use trace stopped at the line gives sub-tag 1
(continues 0) and, for the player, sub-tag 2 with result 1 and the special unchanged;
hitscans cross it with no record of their own. A fireball crossing it (blocking or not)
gives the same sub-tag 3 crossing record as any thing, special 48 before and after: the
source's crossing loop calls `P_CrossSpecialLine`, which returns at once for a missile
(p_spec.c:503–514). A blocking (`ML_BLOCKING`) scroller refuses a player's or monster's
move before the special test, so for them it appears only as the blocking line of the
movement attempt (tag 10) and never in `spechit`; a missile skips that test
(p_map.c:214–221), so a fireball's position checks put it in `spechit` and its crossing
gives the sub-tag 3 record (review scenario `fireball_through_blocking_scroller`).

**Collisions with decorations** use existing events: a movement attempt (tag 10) names a
solid decoration as its blocking actor (walkers of any height: no height test); a missile
exploding on a solid decoration gives the existing explosion record and no damage; a
hitscan trace record (tag 13) lists the decorations the traverser skipped among its
thing intercepts; a decoration under a mover gives the tag-26 sub-tag 9 clip record with
outcome 0 (fits, riding the plane) or 3 (non-shootable left as is, no `nofit`).

**Skill.** The policy is medium, single player, as `actors.options` has encoded since
v20; the C rows evidence the spawn filter (skill bits, bit 16, ambush) per record and
nothing new is encoded. The v36 reader rejects any skill byte but 2 (see below).

Independent C rows feed the Python [E1M1 projection](../tools/reference/e1m1_snapshot.py),
which extends the movers projection without editing it. It renames the identity, drops
the M3h9 observer rows `DT` (`PIT_CheckThing` flag tests), `XT` (aim and shoot traverser
skips) and `NS` (`numspechit` before each movement result) before every chain, and checks
each capture: the decoration records against the declared table (enum indices read from
info.h), their lump order in `thinker.order`, the bound of 32, `line.{i}.special` 48 on
every two-sided scroller and, in the generator, the byte identity with the DSIM35
projection of the same rows. Its full bytes and PBE1 ordered events were committed
before the matching Rust changes. Decoding remains observation, not executable world
restoration.

The v36 reader applies every DSIM35 check (the relaxed item witness of
[§4.2](M3H8_REFERENCE_INVESTIGATION.md#42-b3-the-relaxed-item-witness) included) and
admits in addition actor records whose `type` is one of the seven decoration types. Each
must hold the spawn state of its type with that state's `sprite` and `frame`, `tics` −1,
`health` 1000, the `radius` and `height` of the table, `flags` equal to the info flags
plus `MF_AMBUSH` exactly when the spawnpoint's options carry `MTF_AMBUSH` (8;
`P_SpawnMapThing`, p_mobj.c:795–796), no target and no momentum, `z` equal to `floor`, the
type's `reaction_time` (8), `threshold`, `movedir` and `movecount` 0, `player` 255, not
pending removal, `death_requested` 0, `lastlook` below 4 (`P_Random () % MAXPLAYERS`,
p_mobj.c:506), and a present spawnpoint naming the type's DoomEd number at the actor's
position, with `angle` equal to `ANG45 * (spawnpoint angle / 45)` (p_mobj.c:794: the
`short` divided by 45 truncating toward zero, times 0x20000000 in wrapping unsigned
32-bit arithmetic); any other value is `DiagnosticError::Identity`. The reader also
requires the skill byte of `actors.options` to be 2 (`sk_medium`), the only skill of
the policy (decision E4); any other skill is `DiagnosticError::Identity`. A 33rd decoration in one capture is
`DiagnosticError::Limit`. The decorations of `thinker.order` must read their spawnpoints
(x, y, angle, type, options) as a subsequence of `map.things`, that is in lump order.
`S_GIBS` follows the type-keyed rule above. Every other actor type (barrels among them)
is rejected as before.

Reader limitations. Like the v35 reader, the v36 reader does not re-run the load rules
on the encoded map: a `line.{i}.special` of 48 is checked as any nonzero map special
(present, equal to the map's value), whether its line is one-sided or two-sided, so it is
the loader that keeps two-sided 48 lines out of DSIM35 captures. The v36 reader checks only
the skill byte of `actors.options` (medium); its other two bytes (mode and
`nomonsters`) stay unchecked, as in every reader. Every reader before DSIM36 keeps its
historical contract and leaves `actors.options` entirely unchecked: a DSIM20–35 capture
forged with another skill decodes under its reader, although no player policy can
produce one (investigation [§12](M3H9_REFERENCE_INVESTIGATION.md#12-follow-ups-risks-status),
step 9). Rejecting the skill in `decode_e1m1` only is the owner's decision of the
independent review.

## DSIM v37 — original exploding barrels, pickup scale and every thing angle

`PlayerMonstersBarrelsV1` captures are version 37; `Diagnostic::decode_barrels` is its
explicit reader. The independent Python projection
([barrels projection](../tools/reference/barrels_snapshot.py) and
[its generator](../tools/reference/m3h9b_snapshot_goldens.py)) fixed every committed
capture and the PBE1 event stream from the C rows alone before any Rust writer existed
([plan](M3H9B_REFERENCE_INVESTIGATION.md#9-implementation-plan-one-commit-per-step-pushed-evidence-before-runtime),
step 3); the Rust replay matches every capture's digest, size and field count, the 16
pinned fixtures byte for byte and every event frame
([observations](validation/M3H9B.md)).

Version 37 selects policy 21 (`PlayerMonstersBarrelsV1`, wire 21), the unchanged original
registry schema and context `PLAYER-MONSTERS-BARRELS-1-MEDIUM-v1`. Every earlier
version-specific reader, `decode_e1m1` included, keeps its historical contract and rejects
37; the v37 reader reads 37 only. V20–36 bytes, encoders, fixtures and policy/context
meanings are unchanged.

V37 is the DSIM36 layout and event vocabulary with **no new field and no new event**
(decision [F9](M3H9B_REFERENCE_INVESTIGATION.md#6-new-owner-decisions-f1f11-with-recommendations)).
Every DSIM36 field keeps its path, width, position and meaning, and every event its tag,
payload and position. A v37 capture is byte for byte the DSIM36 layout of the same state
except the version word, the `profile` value (21), the `context.schema` bytes and
**`store.max_slots`, which is 384** (the policy's actor slots; 320 before); the generator
asserts this for every capture of the M3h9b rooms (454 captures; a capture is 3 bytes
longer and has the same field count). What changes is what the layout may contain: up to
64 pickups (`level.total_items`, `pickups.item_count` and the pickup events range to 64,
`bonus_count` to 384), spawnpoint angles of any `short` value with `angle` equal to their
wrapped product, and barrel actor records.

**Barrels** are ordinary actor records with the pristine `info.c` values: `type`
`MT_BARREL` (the `mobjtype_t` index), `state` one of `S_BAR1`, `S_BAR2`, `S_BEXP`,
`S_BEXP2`, `S_BEXP3`, `S_BEXP4`, `S_BEXP5` (the `statenum_t` indices), or `S_GIBS` (895)
after a crunch, or none (65535) on the tic of its removal; `health` 20 down to ≤ 0;
`radius` 10 and `height` 42 alive, `height` 10.5 (`42 >> 2`, raw `688128`) dying, both 0
gibbed; `flags` `MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD` (+`MF_AMBUSH` from the record) alive,
and dying `MF_SHOOTABLE` clear with `MF_CORPSE|MF_DROPOFF` set and `MF_SOLID` kept (clear
only when gibbed); `reaction_time` 8 until the first wound it survives, then 0
(`P_DamageMobj`, p_inter.c:902); `threshold` 0, or 100 with `target` present once a
non-lethal wound with a non-NULL source fixed it (never cleared); `momx`/`momy` from
thrust, `momz` from a fall; `lastlook` below 4; a present spawnpoint holding the THING
record (DoomEd 2035 at the record's position, which a thrust barrel leaves); `player` 255. A barrel holds a slot of the one actor store (384
slots), appears once in `thinker.order`, `thinker.shared_order` (kind 0), `spatial.link_order`
(while linked) and its sector's `actors`. At most 16 barrels per capture. The barrels of
`thinker.order` read their spawnpoints as a subsequence of `map.things` (lump order).
The tic a barrel enters `S_NULL` its record is still present with `state` none, `tics` 0,
`pending_removal` set, the dying flags and height; the next pass retires it.

**Explosions use existing events only.** The barrel's `S_BEXP4` entry gives the tag-12
`Entered` record and the `Action` record of `A_Explode` (action index 19); then, per
candidate of the 3 × 3 block walk in walk order: nothing for a candidate that is not
shootable or out of range, the existing perception sight record (tag 16 sub-tag 10,
observer the candidate, target the dying barrel) for every candidate that reaches
`P_CheckSight`, and for a damaged candidate the existing tag-13 `Damage` record (inflictor
the barrel, source the barrel's target or none) followed by its usual `Knockback`,
`Health`, `Pain`/`Kill`/`DeathTimer`, `Reaction` and `Retarget` records. The walk order is
therefore pinned by the order of these records and by the RNG stream; the candidates
skipped by range or sight are C comparison rows (`RA`, `RC`), not state.

Independent C rows feed the Python [barrels projection](../tools/reference/barrels_snapshot.py),
which extends the E1M1 projection without editing it: it renames the identity, sets the
slot bound to 384 and the item ranges to 64 and 384 for this version only, drops the
M3h9b observer rows `RA` (each `P_RadiusAttack`) and `RC` (each candidate) before every
chain, and checks each barrel record against the declared table, the lump order and the
bound of 16. Its full bytes and PBE1 events were committed before the matching Rust
changes. Decoding remains observation, not executable world restoration.

The v37 reader applies every DSIM36 check (the decoration rules, the medium skill byte,
the relaxed item witness) and admits in addition actor records of type `MT_BARREL`
holding the values above; any other value is `DiagnosticError::Identity`, a 17th barrel
`DiagnosticError::Limit`, `store.max_slots` above 384 `DiagnosticError::Limit`. `S_GIBS`
follows the type-keyed rule (a barrel in `S_GIBS` has the crunch values). Every other
actor type is rejected as before.

Reader limitations. As the v36 reader, the v37 reader does not re-run the load rules on
the encoded map (the spawn rule S′, the barrel and pickup bounds are checked only as
counts and ranges of the encoded state), checks only the skill byte of `actors.options`,
and does not verify that a barrel's `target` is its first wounder (that is a C-evidence and
replay fact, not a per-capture invariant). Every reader before DSIM37 keeps its contract;
`store.max_slots` 384 is rejected by all of them (`Limit`).

## DSIM v38 — the whole level: the body at its THINGS record, damage-count decay and `P_DeathThink`

`PlayerMonstersLevelV1` captures are version 38; `Diagnostic::decode_level` is its explicit
reader. Version 38 selects policy 22 (`PlayerMonstersLevelV1`, wire 22), the unchanged
original registry schema and context `PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1`. Every earlier
version-specific reader, `decode_barrels` included, keeps its historical contract and
rejects 38; the v38 reader reads 38 only. V20–37 bytes, encoders, fixtures and
policy/context meanings are unchanged.

V38 is the DSIM37 layout, event vocabulary and bounds with **no new field and no new
event**: every field keeps its path, width, position and meaning, and every event its tag,
payload and position. Only the version word, the `profile` value (22) and the
`context.schema` bytes name the policy. What changes is what the layout may contain,
following the three behaviours of the policy (decision G4):

- **The player body sits at its THINGS record.** `P_LoadThings` spawns the lump in order
  and `P_SpawnMapThing` calls `P_SpawnPlayer` when it meets the type-1 record
  (p_setup.c:313–346, p_mobj.c:729–736), so the body's slot and its place in
  `thinker.order`, `thinker.shared_order` and `spatial.link_order` follow the records
  before it, not every THING. The body has no spawnpoint (`spawnpoint` is the one byte
  `0`: `P_SpawnPlayer` sets none) and its `angle` starts at `ANG45 * (angle/45)` of the
  record, wrapped as for any THING (p_mobj.c:671).
- **`original_player.0.damage_count` decays** by one per live tic (p_user.c:355–356) and,
  while dead, by one per tic in which the body faces its attacker or has none
  (p_user.c:200–224). Its domain stays 0–100; `bonus_count` keeps its DSIM37 domain and
  decay.
- **A dead body turns toward its attacker** by `ANG5` steps, or exactly when within `ANG5`
  (p_user.c:200–222), so its `angle` is any value. `bob` follows `P_CalcHeight` while dead
  (p_user.c:198).

The v38 reader applies every DSIM37 check and in addition requires: exactly one type-1
record in `map.things`; `original_player.0.mo` present and naming an occupied slot of the
same generation that appears in `thinker.order` and has no spawnpoint; every actor before
the body in `thinker.order` carrying a spawnpoint that matches, in lump order, a record
before the type-1 record; and the body's slot at least the number of those actors and at
most the number of those records. A failure is `DiagnosticError::Identity`. No check is
made on the player body's `angle`.

Reader limitations. As the v37 reader, the v38 reader does not re-run the load rules or
the skill filter on the encoded map, so it checks the body's lump position only as the
ordering above (it cannot tell a removed early THING from a filtered one), and it does not
recompute the turn toward the attacker or the decay from the previous tic (those are
replay and C-comparison facts, not per-capture invariants).

## DSIM v39 — Episode 1, part 1: every special, keys, items, powers and the rocket launcher

`PlayerMonstersEpisode1V1` captures are version 39; `Diagnostic::decode_episode1` is its
explicit reader. Version 39 selects policy 23 (`PlayerMonstersEpisode1V1`, wire 23), the
unchanged original registry schema, layout selector 17 (as versions 20–38) and context
`PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1`. **Frozen-reader rule:** every earlier
version-specific reader, `decode_level` included, keeps its historical contract and rejects
39; the v39 reader reads 39 only. V20–38 bytes, encoders, fixtures and policy/context
meanings are unchanged. The Rust writer is checked against the independent Python
projection of the genuine engine's per-tic dump; the field list below is the contract
between the two. Support and limits are in
[COMPATIBILITY](COMPATIBILITY.md#original-episode-1-part-1-every-door-floor-plat-stair-donut-light-and-exit-special-keys-items-powers-chainsaw-rocket-launcher-and-spectre-wire-23).

V39 is the DSIM38 layout and event vocabulary plus the following, in emission order
("after X" means immediately after field X). Every DSIM38 field keeps its path, width,
position and meaning.

| Position | Field | Value encoding |
| --- | --- | --- |
| after `level.exit` | `level.secret_exit` | u8 boolean: the exit request was `G_SecretExitLevel` (special 51); `level.exit` is then also 1 |
| `store.max_slots` | same field | u32 `1024` (it was 384) |
| after each `sector.N.special` | `sector.N.floorpic` | 8 bytes, the normalized flat name (the same normalization as the donut texture), for every sector; it changes when a donut copies a floor texture |
| after `original_player.0.weapons.shells` | `original_player.0.weapons.cells`, `.rockets` | u16 each |
| after those | `original_player.0.weapons.max_ammo.0` … `.3` | u16 each (bullets, shells, cells, rockets): 200, 50, 300, 50, each doubled after the first backpack |
| after `original_player.0.use_down` | `original_player.0.powers.0` … `.5` | i32 each, in `powertype_t` order: invulnerability, strength, invisibility, ironfeet, allmap, infrared |
| after the powers | `original_player.0.cards.0` … `.5` | u8 0/1, in `card_t` order: blue card, yellow card, red card, blue skull, yellow skull, red skull |
| after the cards | `original_player.0.backpack` | u8 0/1 |

Domains the v39 reader enforces: `store.max_slots` is exactly 1,024 (decision D1; the
older readers check only their upper bound); `powers.0` and `powers.1` are 0 (no
invulnerability or berserk in Episode 1 single player), `powers.2` and `powers.3` are 0–2100, `powers.4` is
0–1, `powers.5` is 0–4200; `cards.3` … `cards.5` are 0; `weapons.max_ammo` equals the base
capacity shifted left by `backpack`, and each ammunition count is at most its maximum;
`weapons.owned` uses only bits 0–4 and 7 (fist, pistol, shotgun, chaingun, rocket launcher,
chainsaw), `weapons.ready` is one of 0–4 and 7, and `weapons.pending` is one of 0–4, 7 and
10 (`wp_nochange`); `MF_SHADOW` is set on the player body exactly when `powers.2` is
positive; `MF_JUSTATTACKED` appears only on the body and only while the chainsaw is ready;
`level.secret_exit` implies `level.exit`; every `floorpic` is a normalized name. The
player-command fields (`player.0.forward`, `.side`, `.angle_turn`, `.buttons`) hold the
command after `P_PlayerThink`'s rewrite: on a chainsaw lunge tic `forward` is 100 and `side`
and `angle_turn` are 0.

**Sector-thinker records.** The kind codes are unchanged (5 door, 6 plat, 7 floor, 8
removed mover); stairs and the donut are ordinary floor records, in thinker order.

| Record | Fields after `.generation` and `.occupied` |
| --- | --- |
| door (kind 5) | unchanged (`sector, type, direction, topheight, topcountdown`); `type` is now 0 `normal`, 1 `close30ThenOpen`, 3 `open` or 6 `blazeOpen` |
| plat (kind 6) | `sector, status, count, low, high` as before, then `type` u8 (`plattype_e`: 1 `downWaitUpStay`, 3 `raiseToNearestAndChange`). For type 3 `low` is never written by the original and is encoded as 0 (decision H7) |
| floor (kind 7) | `sector, type, floordestheight` as before, then `direction` i32 (−1 or 1) and `speed` i32 (raw 16.16); **only when `type` is 11 (`donutRaise`)** also `texture` (8 bytes, normalized flat name) and `newspecial` i16. `type` is 0 `lowerFloor` (the donut hole and every stair step), 1 `lowerFloorToLowest`, 2 `turboLower`, 3 `raiseFloor`, 4 `raiseFloorToNearest` or 11 |
| removed (kind 8) | unchanged |

Implied and omitted: a door's `speed` and `topwait`, a plat's `speed`, `wait`, `crush` and
`tag`, a floor's `crush`, and `texture` and `newspecial` for every floor that is not a
donut. At most one live mover names a sector. The light trigger (special 35) creates no
record; its effect is the `sector.N.light` values, and the shot and switch triggers change
`line.N.special` as before.

**Pickup messages** (`original_player.0.pickups.message`, codes 0–13 unchanged):

| Code | Message | Code | Message |
| --- | --- | --- | --- |
| 14 | `GOTSUPER` | 22 | `GOTROCKET` |
| 15 | `GOTBLUECARD` | 23 | `GOTROCKBOX` |
| 16 | `GOTYELWCARD` | 24 | `GOTBACKPACK` |
| 17 | `GOTREDCARD` | 25 | `GOTCHAINSAW` |
| 18 | `GOTINVIS` | 26 | `GOTLAUNCHER` |
| 19 | `GOTSUIT` | 27 | `PD_BLUEK` |
| 20 | `GOTMAP` | 28 | `PD_YELLOWK` |
| 21 | `GOTVISOR` | 29 | `PD_REDK` |
| | | 30 | turbo message |

The three `PD_*` locked-door messages (blue, yellow and red key needed) are the genuine
driver's message indices 38–40; the projection maps them to DSIM codes 27–29, and the
other new driver indices to 14–26. Code 30 is `G_Ticker`'s turbo message (`"%s is turbo!"`
with player 0's name, g_game.c:671–679): it is assigned before `P_Ticker` when the tic's
`forwardmove` exceeds 50 on every 128th gametic (`gametic` a multiple of 128), so a pickup
or locked door later in the same tic replaces it; the genuine driver reports it as index 41.
Codes 14–30 are admitted by the v39 reader only.

**Events.** No new tag. Pickup tag 23 gains four sub-tags and widens two:

| Sub-tag | Meaning | Payload after tag and sub-tag |
| --- | --- | --- |
| 11 | Key card touched | u8 card (0–2), u8 held before |
| 12 | Power touched | u8 power, u8 accepted, i32 counter before, i32 counter after |
| 13 | Soulsphere | i32 health before, i32 after, i32 body health after |
| 14 | Backpack | u8 first (1 when it was the first backpack) |

The existing Ammo (sub-tag 1) payload admits cells and rockets and the doubled maxima.
Mover event records gain the builder functions `BuildStairs` (5) and `DoDonut` (6), and the
exit event carries a secret flag for special 51. The projection strips the event stream on
both sides; the events remain canonical in the Rust capture.

The v39 reader applies every DSIM38 check (with `store.max_slots` 1024 in place of 384) and
the domains above. Reader limitations as for v38: it does not re-run the load rules or
replay the engine, so it cannot check a floor's destination or a power's history.

## DSIM v40 — Episode 1, part 2: teleporters, the Baron of Hell, `A_BossDeath` and hard skill

`PlayerMonstersEpisode1V2` captures are version 40. Version 40 selects policy 24
(`PlayerMonstersEpisode1V2`, wire 24; [M4b decision I2](M4B_REFERENCE_INVESTIGATION.md#5-owner-decisions-i1i13-with-recommendations)),
the unchanged original registry schema, layout selector 17 (as versions 20–39) and context
`PLAYER-MONSTERS-EPISODE1-2-v1`. The context carries no skill word because the skill is a field.
**Frozen-reader rule:** every earlier version-specific reader, the v39 reader included, keeps
its historical contract and rejects 40; the v40 reader reads 40 only. V20–39 bytes, encoders,
fixtures and policy/context meanings are unchanged.

The Rust writer is checked against the independent Python projection of the genuine engine's
per-tic dump (`tools/reference/level_snapshot_m4b.py`, profile `episode1v2`) and its committed
goldens (`tools/reference/m4b_snapshot_goldens.py`; commands in
[TESTING](TESTING.md#m4b-synthetic-mechanism-wads)). The text below is the contract between
the two. Expected bytes come from the C dumps through that projection, never from Rust.

### Layout: DSIM39 plus one field

V40 is the DSIM39 layout and event vocabulary with the following differences, and no others.
Every DSIM39 field keeps its path, width, position, source and meaning.

| Position | Field | Value encoding | Why |
| --- | --- | --- | --- |
| header | version word | u32 `40` | selects the policy |
| `profile` (same place) | same field | u32 `24` | wire 24 |
| `context.schema` (same place) | same field | the 29 bytes `PLAYER-MONSTERS-EPISODE1-2-v1` | names the policy, with no skill word |
| `actors.options` (same place) | same 3 bytes | `[skill, 0, 0]`; `skill` is now 2 (medium) **or 3 (hard)** | decision I4: the constructor takes the skill; it is `gameskill` |
| right after `level.secret_exit` | **`level.map`** (new) | u8, 1–9 | the map number of the Episode 1 slot (`gamemap`). `A_BossDeath` (p_enemy.c:1629–1634) and `PIT_StompThing` (p_map.c:102) test it, so it is canonical state. The constructor takes it (decision I2); it is constant for a run |

The episode is not encoded. The policy admits episode 1 only, so `gameepisode` is 1 by
construction. The independent projection reads `level.map` and the skill from bytes 3 and 1 of
the demo header, which `G_DoPlayDemo` hands to `G_InitNew`. The skill must equal the dump's
`gameskill`. The genuine driver has no `gamemap` column; nothing else reads it.

**Compatibility rule.** A state that wire 23 can reach is encoded by v40 exactly as by v39,
except in the four places above: the version word, `profile`, `context.schema` and the
inserted `level.map`. (`actors.options` is unchanged at skill 2.) The projection checks this
rule on every snapshot it projects with both profiles
([evidence](validation/M4B.md#independent-projection)).

**No new per-actor or per-player field.** Teleport, telefrag, the Baron and `A_BossDeath` change
only values that DSIM39 already carries:

- `x`, `y`, `z`, `floor`, `ceiling`, `angle` and the momenta;
- `reaction_time`, `health`, `state`, `tics`, `flags` and `target`;
- the store, `thinker.order`, `thinker.shared_order` and `spatial.link_order`;
- the sector heights and the sector-thinker records;
- `rng.gameplay`, the level totals and the kill count.

`tracer` stays unencoded and must be NULL: no admitted action writes it. `player->viewz` is not
encoded. The teleport writes it (p_telept.c:107–108), but it is presentation: `P_CalcHeight`
recomputes it every tic, and no gameplay code reads it.

### New actor types and states

Numbers are `mobjtype_t` and `statenum_t` ordinals (info.h), as in every original-ID capture.

| Type | Doomednum | States allowed in `type`/`state` (the normal closure) | Notes |
| --- | --- | --- | --- |
| `MT_BRUISER` (15), Baron of Hell | 3003 | `S_BOSS_STND` (527) … `S_BOSS_DIE7` (548) | health 1,000, mass 1,000, radius 24, height 64; no xdeath. `S_BOSS_RAISE1`–`7` (549–555) are not allowed |
| `MT_BRUISERSHOT` (16), Baron fireball | — | `S_BRBALL1` (522) … `S_BRBALLX3` (526) | an ordinary missile actor, like the Imp fireball (type 31); damage 8 |
| `MT_TFOG` (39), teleport fog | — | `S_TFOG` (130) … `S_TFOG10` (141) | NOGRAVITY\|NOBLOCKMAP; removed at the end of its chain (`S_NULL`), like a puff |
| `MT_TELEPORTMAN` (41), teleport landing | 14 | `state` **0** only | see below |

The normal closure of a type is every state reached through `nextstate` from its info.c
`spawnstate`, `seestate`, `painstate`, `meleestate`, `missilestate`, `deathstate` and
`xdeathstate`. `raisestate` is excluded because Episode 1 has no Archvile. Every type wire 23
admits keeps its own closure, plus two extras:

- the player's `S_PLAY_ATK2`, which p_pspr.c sets by name;
- `S_GIBS` for any corpse that a moving sector crushes.

A record pending removal may also hold the removal sentinel 65535.

**The landing (`MT_TELEPORTMAN`).** Its spawn state is `S_NULL`. A live landing is encoded with:

- `state` **0**: the index of `S_NULL`, which is not the removal sentinel 65535;
- `tics` −1, `sprite` 0, `frame` 0 and `health` 1,000;
- flags 24 (NOBLOCKMAP|NOSECTOR) and zero momentum.

It never thinks, moves or is removed. It occupies a slot and appears in `thinker.order` and
`thinker.shared_order` like any actor. It also appears in `spatial.link_order`, because
`P_SpawnMobj` ran `P_SetThingPosition` for it. It is in no `sector.N.actors` list, because of
NOSECTOR. Its `.sector` is its subsector's sector, as for every actor. Landings spawn on skill 2
and skill 3 alike, subject only to the THING option bits.

### What the new behaviour writes into existing fields

**Teleport** (line special 97, `EV_Teleport`, p_telept.c:47–131; the M4b note §2 T1–T7). When a
crossing teleports a thing:

- `x` and `y` take the landing's position. `floor` and `ceiling` come from `P_TeleportMove`
  (p_map.c:115–177), and `z` is the new `floor`.
- `angle` takes the landing's angle, and `momx`, `momy` and `momz` become 0.
- The thing is relinked, so it moves to the front of `spatial.link_order`.
- Two `MT_TFOG` actors are then spawned, in this order:
  1. at the thing's old x, y and z;
  2. 20 units from the landing along the landing's angle, at the new z.

  Each spawn draws once for `lastlook`, so `rng.gameplay` advances by 2. Both fogs take the
  lowest free slots and come after the relinked thing in `spatial.link_order`.
- A player's body gets `reaction_time` 18. `P_PlayerThink` counts it down by one per tic
  (p_user.c:267–268) and skips `P_MovePlayer` while it is nonzero. The `player.0.*` command
  fields still hold the command that was received. A non-player's `reaction_time` is not changed.
- A teleport that fails changes nothing. It fails on a crossing from the back side, when no
  sector with the line's tag holds a landing, or when `P_TeleportMove` refuses.

The teleport runs inside the move, as in `P_TryMove`. A split move or a slide continues from
the landing (decision I7).

**Telefrag** (`PIT_StompThing`, p_map.c:81–106; decision I8). Only a player stomps: on maps 1–9
a non-player's teleport fails when any shootable thing overlaps the landing (p_map.c:102). Each
overlapping shootable thing is hit by `P_DamageMobj(thing, player, player, 10000)`:

- Its `health` drops by 10,000, so a telefragged monster's health lies between −9,999 and its
  spawn health minus 10,000 (−9,000 for a Baron). It dies through the ordinary `P_KillMobj`, which takes its usual
  draws.
- Its momentum gains the knockback, unless the player's ready weapon is the chainsaw
  (p_inter.c:806–810). The thrust is the **wrapped** 32-bit value of the source expression
  `damage*(FRACUNIT>>3)*100/mass` (p_inter.c:817; decision I3): the product 8,192,000,000 wraps
  to −397,934,592 and is then divided by the mass, truncating toward zero:

  | Mass | Thrust |
  | --- | --- |
  | 100 | −3,979,345 |
  | 400 | −994,836 |
  | 1,000 | −397,934 |

  It is applied with `FixedMul` along the angle from the player's position before the teleport
  to the victim, so the corpse is pushed toward that position. The "fall forwards" branch never
  applies, because the damage is not below 40.
- `level.single_player_kill_count` counts the kill as usual.

**The Baron.** `A_BruisAttack` (p_enemy.c:979–996) either claws or fires:

- In melee range it claws: `(P_Random()%8+1)*10` damage through `P_DamageMobj`, with no actor
  spawned.
- Otherwise it fires an `MT_BRUISERSHOT` through the shared `P_SpawnMissile`, with the same
  record shape as an Imp fireball (`target` is the Baron).

Pain (`S_BOSS_PAIN`, painchance 50), infighting and death follow the shared services. A Baron's
fireball that hits another Baron explodes without damage (the same-species rule, p_map.c:299–313);
the Hell Knight half of that rule does not arise, because the Knight is not admitted.

**`A_BossDeath`** (p_enemy.c:1609–1757; decision I9) runs when a Baron enters `S_BOSS_DIE7`.
That state has `tics` −1, so the action runs on entry. It acts only when all of these hold:

- `level.map` is 8;
- the player is alive (`original_player.0.health` > 0);
- no other `MT_BRUISER` in the thinker list has `health` > 0.

It then calls `EV_DoFloor(lowerFloorToLowest)` on every sector tagged 666. Each floor started is
an existing kind-7 record (`type` 1, `direction` −1, `speed` 65536). It is appended to the
shared order and steps once in the same tic. A sector that already has a mover starts nothing.
In every other case the action returns and changes nothing. On episode 1, `G_ExitLevel` is never
reached from here. E1M8 ends through sector special 11, as in wire 23, and every tic after the
exit tic is refused with `LevelExited`.

**Hard skill.** Under skill 3 the THING filter tests option bit 4 instead of bit 2
(p_mobj.c:743–751). That is the only single-player difference (M4b note §3 part C).
`level.total_kills` and `level.total_items` count what spawned.

### Events

Every DSIM39 tag keeps its layout. Mover tag 26 gains three sub-tags, admitted by the v40
reader only:

| Sub-tag | Meaning | Payload after tag and sub-tag | Position |
| --- | --- | --- | --- |
| 15 | Teleported (`EV_Teleport` returned 1) | thing actor ID, u16 line, landing actor ID, i32 old x, i32 old y, i32 old z (16.16) | after the records of what the teleport caused (the stomp's combat records, then the two fog spawn records, tag 1, each followed by the fog's state-initialisation record and its `sfx_telept` sound record), and before the sub-tag 3 record of the same crossing |
| 16 | Teleport not taken (`EV_Teleport` returned 0) | thing actor ID, u16 line, u8 reason: 0 crossed from the back side, 1 no landing in any sector with the line's tag, 2 `P_TeleportMove` refused (a non-player met a shootable thing) | the same position |
| 17 | `A_BossDeath` ran | Baron actor ID, u8 outcome: 0 `level.map` is not 8, 1 no live player, 2 another Baron alive, 3 the tag-666 floor call was made | where the action runs, in the `S_BOSS_DIE7` state entry. Outcome 3 comes after the call's sub-tag 5 records and its sub-tag 6 record. That record has function 4 (`EV_DoFloor`), line 65535 (no line: the source passes a local line whose only field set is tag 666), argument 1 (`lowerFloorToLowest`), the call's result and the number of movers created |

Notes on the events:

- Missiles never reach sub-tags 15 or 16. `P_CrossSpecialLine` returns before the special for
  every Episode 1 missile type.
- `A_BossDeath`'s type test cannot fail for an admitted actor, because only the Baron's
  `S_BOSS_DIE7` names it.
- A telefrag uses the existing combat records (tag 13): damage (sub-tag 2) with amount 10,000,
  inflictor and source the teleporting player; knockback (sub-tag 4) with the wrapped thrust
  above; health, kill and death-timer records as for any kill. Like the v39 reader, the v40
  reader does not check combat record amounts; it bounds the result, each actor's `health`
  being at least −9,999.
- The Baron's attacks use the existing combat and projectile records (tags 13 and 15) with
  `MT_BRUISERSHOT` (16) wherever the v39 forms name `MT_TROOPSHOT` (31).

The projection strips the event stream on both sides; the events remain canonical in the Rust
capture.

### What the v40 reader checks

The v40 reader applies every v39 check, with these changes:

- version 40, `profile` 24 and the v40 context;
- `level.map` is 1–9;
- `actors.options` is `[2, 0, 0]` or `[3, 0, 0]`;
- every actor's `type`/`state` lies in the closures above. `MT_TELEPORTMAN` must be in state 0
  with `tics` −1 and is never pending;
- every actor's `health` is at least −9,999 (one telefrag of a live thing; the player is never
  stomped, so this bounds it too);
- the tag-26 sub-tags 15–17 and the widened damage form are admitted.

**Admission so far (M4b step 6).** The v40 reader admits the `MT_TFOG`, `MT_TELEPORTMAN`,
`MT_BRUISER` and `MT_BRUISERSHOT` closures (the Baron's raise states refused), sub-tags 15, 16
and 17, the sub-tag 6 record of `A_BossDeath`'s floor call (function 4, line 65535, argument 1),
`MT_BRUISERSHOT` in the projectile launch record and the sky removal, and the telefrag's health
domain, and, since step 6, `actors.options[0]` 2 or 3 (any other value is refused with `Identity`;
the DSIM36-39 readers admit 2 only; M4b note §7, finding 10, closed).

The per-map bounds, and the placed totals (`level.total_kills`, `level.total_items`) the reader
re-derives, come from `map.things` with the skill bit of `actors.options[0]` (2 for skill 2, 4 for
skill 3, p_mobj.c:743-751), after the single-player filter, so a frame whose skill byte
disagrees with its totals is refused. Doomednum 3003 counts as a monster
that drops nothing, and doomednum 14 as a landing. The bound formulas and the store-full witness
are wire 24's and are documented in
[COMPATIBILITY](COMPATIBILITY.md) with the policy.

Reader limitations, as for v39: the reader does not re-run the load rules or replay the engine.
So it cannot tell whether a teleport, a stomp or an `A_BossDeath` outcome matches the map. Those
are replay and C-comparison facts, not per-capture invariants.
