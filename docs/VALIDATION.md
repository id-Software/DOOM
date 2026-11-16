# Validation guide

Compatibility claims are limited to independently demonstrated input slices.
A passing test suite or matching digest does not prove a complete game, original
savegame compatibility or vanilla demo synchronization.

## Evidence families

| Evidence | What it establishes | Main records |
| --- | --- | --- |
| Genuine original C | Source decisions, fields, RNG and ordering under documented adapters | Milestone REFERENCE notes and `tests/reference/*provenance.json` and versioned `*contract.json` |
| Fixture projection | Explicit native identity/state adaptations compared with original C | Source investigations and adapter assumptions |
| Independent DSIM | Complete bytes/per-tic digests from C observations plus documented schema; no Rust expected-output generation | [State format](STATE_FORMAT.md), independent Python encoders |
| Rust safety policies | Checked arithmetic, bounds, atomic rejection, stale handles and independent instances | Policy/unit tests, [compatibility](COMPATIBILITY.md) |
| Platform/toolchain | Recorded checks execute on supported hosts and Rust versions | Exact-head GitHub Actions, [testing](TESTING.md) |
| Instruction counts (M4p) | `Simulation::advance` did not get costlier, in Callgrind instructions and allocator/copy calls, on nine fixed synthetic workloads, and their final digests did not change. Not time, not real maps, not other hosts' speed | `simulation-ir` CI job, [M4p record](validation/M4P.md), [testing](TESTING.md#advance-benchmarks-m4p) |

Compiled source-function inventory is not branch coverage. Setup fragments are
reported separately from whole functions. Unadapted C, declared fixture projection,
canonical consistency and Rust-specific rejection are never interchangeable claims.

## Reproduce

[TESTING](TESTING.md) owns build, format, Clippy, workspace/release, rustdoc/MSRV,
reference/encoder, example, documentation and fuzz reproduction commands.
Cross-platform CI requires Linux, Windows, native macOS and
Rust 1.85.0. Original-C regeneration runs on Linux; other hosts compare recorded
observations and independent canonical fixtures.

Sanitizer reports name the actual target, corpus/input model, execution count,
limits, findings and leak setting. Disabled leak detection does not provide
LeakSanitizer coverage; no bounded campaign is exhaustive.

## Detailed records

- [M3g1 stepping and transaction costs](validation/M3G1.md): complete-state API
  refinement, capture-boundary regression hooks, reused C observations, and
  separately instrumented timing/allocation measurements. No new C coverage.

- [M3f9 game-instance provenance](validation/M3F9.md): Rust-specific incarnation,
  import/export, restart and trust policy; the selected C input is reused evidence,
  not new original-source coverage. Historical DSIM and gameplay remain separate.

- [M3f8 context/target ownership](validation/M3F8.md): new compact C inputs,
  retained references and logical retirement, independently encoded v21 captures,
  and separately labeled lifetime-brand/generation/atomicity policy checks.

- [M3f7 original-ID registry/profile and bounded diagnostics](validation/M3F7.md):
  new encoding of reused C inputs; actual runtime storage and exact ordered event
  comparisons remain distinct from metadata, digests and Rust-only reader policy.

- [M3f6 runtime identity audit and boundary](validation/M3F6.md), reusing independent
  C observations without claiming new capture or gameplay coverage.

- [M3f5 canonical Demon execution binding](validation/M3F5.md), including raw
  canonical C observations and separately projected DSIM-v20 comparisons.
- [M3f4 canonical identity metadata](validation/M3F4.md).

- [M3f3 combined Demon/expanded-map evidence](validation/M3F3.md).

- [M3f2 expanded-coordinate/spatial-work evidence](validation/M3F2.md).
- [M3f1 map-scale source comparisons and host measurements](validation/M3F1.md).

- [M3d3j map-context implementation and delivery evidence](validation/M3D3J.md).
- [M3d3j original-source investigation](M3D3J_REFERENCE.md).
- [Reference-tool reproduction and provenance](../tools/reference/README.md).
- [All historical engineering records](validation/archive/README.md), including
  the complete [pre-M3d3j ledger](validation/archive/pre-m3d3j-ledger.md).

## Historical evidence preservation

The old validation ledger is retained byte-for-byte in the
[archive](validation/archive/README.md). Its readable Markdown copy rebases links. The
[manifest](validation/archive/manifest.json) records source commit, hashes, sizes,
archive destinations and every original heading. Other former guides are also
archived intact, preserving detailed restrictions and evidence.

Historical SHA/CI references, source hashes, qualified counts and sanitizer limits
remain historical observations. Moving them does not assert they were rerun on a
new commit. Actual generators, encoders, fixtures and historical helper bytes remain
at their existing paths. The [documentation checklist](DOC_REVIEW_CHECKLIST.md)
governs evidence and link review.

## Keep observations separate from current status

Detailed measured results belong in milestone records, not this guide or other
entry points. ROADMAP owns the development summary; GitHub PRs/Actions own exact
review commits, statuses and CI history. Source investigations and frozen archives
describe their original observation context. Follow the
[authority map](README.md#documentation-authority) rather than copying those facts.

M3f4 adds a separate compiled-C [identity metadata observer](M3F4_REFERENCE.md),
without changing historical DSIM captures. [Measured observations](validation/M3F4.md)
separate complete metadata recognition from unchanged executable admission.

## Original-player evidence boundary

[M3g2](validation/M3G2.md) adds genuine original-player damage/death inputs, C field
and ordered-event comparisons, and independently encoded DSIM-v22 fixtures. It
reuses the existing source extraction services while separately counting executed
function entries, inherited inventory and the newly compiled P_DropWeapon body.
The association fragment and tactile/psprite request boundaries are explicit;
controls, weapons, death thinker and automatic roster/monster acquisition are not
inferred. Rust generation/provenance/atomicity tests remain policy evidence.
Historical observers, fixtures and encoders continue to run unchanged. The
[source matrix](M3G2_REFERENCE.md) and [support contract](COMPATIBILITY.md#original-player-body-lifecycle)
define what each comparison proves.

[M3g6 autonomous melee](validation/M3G6.md) adds genuine player-versus-Demon
selection, delayed attack, range miss, physical damage, pain/death and corpse
observations through the existing source command/thinker harness. Compiled-body
inventory, actually exercised entries, newly exercised input families, C fields,
independent event/schema projections and Rust-only ownership/rollback checks are
reported separately. Existing source bodies and previous evidence are reused,
not counted again as globally new coverage. The [source matrix](M3G6_REFERENCE.md)
qualifies unreachable reverse thrust/extreme death and excluded full player thinkers.

[M3g3](validation/M3G3.md) adds original-player physical impulse and actual grounded
movement, friction, axis sliding and moving-corpse observations. Genuine map-spawn
origins retain previous ownership/RNG semantics. Independent DSIM22 policy3 captures
and compact complete event frames remain distinct from Rust-only forward bounds,
provenance and rollback guarantees. Three source slide bodies extend this observer
but already occur in historical movement evidence; their inventory is not globally
new function or branch coverage. All old observers, schemas and fixtures remain
regression gates.


[M3g4](validation/M3G4.md) adds command-gate/P_MovePlayer observations, compact
per-tic motion/command rows and independently encoded DSIM22 policy4 fixtures.
Selected full captures overlap these rows; do not sum them as unique coverage.
The unsigned negative-turn adapter, literal defined cases and deliberate UB
expression probe have distinct claims. Rust admission, generation/provenance,
zero-command continuation and late transaction checks remain separate safety
policy. Every older observer, fixture and decoder remains a regression gate.


[M3g5](validation/M3G5.md) adds genuine active-player/MT_SERGEANT source scenarios.
The new observer executes original scan/Look/Chase bodies against a genuine player
association, with the explicitly bounded command gate before the thinker pass.
This is neither full PlayerThink nor full P_Ticker execution. Source roster rows
are independently checked against player/mo fields before policy5 DSIM22 encoding.
Per-tic motion/Demon/decision rows overlap selected full captures and are counted
separately. All-inactive C scanning is never run. Retired source identity uses the
declared quarantine observation; stale/foreign generations, fixed-body removal
rejection and transactional melee refusal are Rust policy. Historical evidence,
source extraction and encoders remain unchanged; new expected values never come
from Rust output. See the [source matrix](M3G5_REFERENCE.md).


## Genuine-player pistol evidence

[M3g7](validation/M3G7.md) establishes a new source-backed composition of genuine
player weapon setup, command-triggered psprites, ammo/refire, aim/hitscan,
Puff/Blood thinkers, one-sector noise, Demon pain/death/retaliation and existing
autonomous melee. It reuses historical source bodies and shared Rust services;
new scenarios are not globally new function or branch coverage. The instrumented
entry inventory explicitly separates uninstrumented linked fixed-math bodies.

Expected C rows and independent DSIM23 bytes were recorded before matching Rust
integration. Full ordered-event comparisons and separate actual aim-query
comparisons complement actor/player/weapon field checks. Source allocations are
projected into the separately specified lowest-vacant generational schema, not
claimed equivalent to C pointer lifetime. No stale/freed C dereference or empty
active-roster scan is executed. Ownership, atomic failure, invalid inputs and
corrupt diagnostics are Rust policy tests. The [source investigation](M3G7_REFERENCE.md)
and [commands](TESTING.md#genuine-player-pistol) declare bounded harness sequencing,
source arithmetic qualifications and unsupported complete player-thinker behavior.

## Original four-weapon composition

[M3g8 evidence](validation/M3G8.md) distinguishes original-C observations,
independent DSIM24 encoding, actual admitted Rust comparisons and Rust-only
resource/lifetime/rollback policies. The same shared source services execute
fallback, fist, switching, shotgun and chaingun in one canonical player/Demon
world. Source-only inputs may exceed the unchanged Rust admission domain; such
prefix comparisons and safety rejection are identified explicitly, without
altering the C expectations. No whole-ticker or portable undefined-C claim is made.
See [source matrix](M3G8_REFERENCE_INVESTIGATION.md) and
[reproduction](TESTING.md#original-player-weapon-expansion).


## Original world pickup composition

[M3g9 observations](validation/M3G9.md) distinguish new source world contacts,
resource/weapon grants, independent DSIM25 encoding and Rust-only handle/error
policies. Actual source things-before-lines collection is compared even when a
later wall or solid actor blocks movement. No Rust output generates expectations.
The source's inactive item-respawn queue is retained as C-only PQ evidence and is
outside the executable projection; no whole RemoveMobj or respawn claim is made.
Eight unticked components separately establish reach/overlap/full/dead boundaries.
The fixed command/weapon/bonus fragment before original thinkers is not complete
P_Ticker/PlayerThink/DeathThink. Sustained winning and losing encounters use normal
initial pistol ammo and real autonomous damage, then actual world resources.
See [source qualification](M3G9_REFERENCE_INVESTIGATION.md) and
[reproduction](TESTING.md#original-world-pickups).

- [Zstd and weapon-formula maintenance](validation/ZSTD_WEAPON_MAINTENANCE.md):
  actual baseline-byte migration proof, bounded loader policy, compact inherited
  source-body component observations and matched before/after measurements.
  [Storage](EVIDENCE_STORAGE.md) and [formula](WEAPON_FORMULAS.md) contracts
  separate new safety/component tests from reused gameplay evidence.


- [M3g10 armor and sustained combat](validation/M3G10.md): real animated armor,
  source grants/absorption, independent DSIM26 and separately labeled Rust safety.
- [M3g11 health/armor bonuses](validation/M3G11.md): animated bonuses, COUNTITEM
  statistics, independent DSIM27, matched timings and bounded ASan fuzz campaigns.
- [M3h1 former humans and mixed rosters](validation/M3H1.md): original attacks,
  infighting, drops and kill statistics, independent DSIM28, matched timings and
  bounded ASan fuzz campaigns.
- [M3h2 Imps and fireballs](validation/M3H2.md): original melee/missile choice,
  fireball flight and contact, same-species rule, DSIM29, dispatch and measurements.
- [M3h3 wall-aware travel and real-map scale](validation/M3H3.md): the reproduced
  M3h2 stalls and their continuation in C, rooms to 2,048 units, wall slides and
  impulses, DSIM30, the no-stall soaks, caps and the `sixteen` fix.
- [M3h4 two-sided lines, static heights and vertical movement](validation/M3H4.md):
  steps, falls, landings, ledges, windows, fences, noise floods and fireball
  floor/ceiling explosions in C, DSIM31, the vertical-invariant soaks and fuzzing.
- [M3h5 non-axis (diagonal) walls](validation/M3H5.md): wall slides at three slopes,
  both corner kinds, a diagonal riser, window, pillar and corridor and the corner jump
  in C, DSIM32, the wall-invariant soaks, fuzzing and the refused escape.
- [M3h6 real-map scale](validation/M3H6.md): a 516-linedef, 74-sector town, a toothed
  gallery with 73-intercept traces, a 3,850-norm wall at the arithmetic limit, a comb
  of 123 lines in one window and a 60 × 60 cell map in C, DSIM33, the admission
  rejections, the intercept witness, soaks, fuzzing and IWAD counts.
- [M3h7 sky and static specials](validation/M3H7.md): light thinkers, damage floors,
  a secret, scrolling walls, sky removals and sky shots in C, DSIM34 with the shared
  thinker order, the admission rejections, the load order, soaks and fuzzing.
- [M3h8 doors, lifts, floors, switches and the exit](validation/M3H8.md): doors, lifts,
  floors, switches, use, crossing, monster door use, giblets, the exit and the reborn
  request in C, DSIM35 with the movers and button slots, the full replay, the load-rule
  rejections, in-tic rollback, soaks and fuzzing.
- [M3h9 two-sided scrolling walls and static decorations](validation/M3H9.md): two-sided
  scrollers, the seven E1M1 decoration types blocking, passed through and riding movers,
  and the skill and bit-16 spawn filter in C, DSIM36, the full replay, the load
  rejections, rollback, soaks and the fuzz seed replay, with the
  [IWAD THING counts](validation/M3H9_IWAD_COUNTS.md).
