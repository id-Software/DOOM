# Documentation

Start with the [project README](../README.md). These guides explain the engine
without requiring familiarity with milestone names or earlier agent sessions.

## Documentation authority

Every information category has one authoritative home. Update that home, then
summarize briefly and link from other pages. Do not maintain parallel milestone
status tables, test-count lists, commit histories or schema definitions.

| Information | Authoritative home |
| --- | --- |
| Purpose, quickstart and high-level capabilities | [README](../README.md) |
| Current milestone summary and planned work | [ROADMAP](ROADMAP.md) |
| Exact supported behavior and runtime/parser limits | [COMPATIBILITY](COMPATIBILITY.md) |
| Architecture and subsystem ownership | [ARCHITECTURE](ARCHITECTURE.md) |
| Original actor/state and runtime identity mapping | [ACTOR_IDENTITIES](ACTOR_IDENTITIES.md) |
| Execution-context ownership and reference lifetime limits | [COMPATIBILITY](COMPATIBILITY.md#context-and-target-lifetime-contract), with ownership architecture in [ARCHITECTURE](ARCHITECTURE.md) |
| Runtime instance provenance and boundary trust | [COMPATIBILITY](COMPATIBILITY.md#game-instance-provenance), with design in [ARCHITECTURE](ARCHITECTURE.md#game-instance-provenance) |
| Canonical simulation snapshot schemas | [STATE_FORMAT](STATE_FORMAT.md) |
| Build, test and reproduction commands | [TESTING](TESTING.md) |
| Validation methodology and evidence navigation | [VALIDATION](VALIDATION.md) |
| Original-C investigation and technical findings | The corresponding REFERENCE note below |
| Detailed milestone test observations | The corresponding `docs/validation/` record, such as [M3d3j](validation/M3D3J.md) |
| Exact review commits, CI, reviews and merge history | [GitHub PRs](https://github.com/Copgunn/DOOM/pulls) and [Actions](https://github.com/Copgunn/DOOM/actions) |
| Standing agent engineering rules | [AGENTS](../AGENTS.md) |
| Human contributor workflow | [CONTRIBUTING](../CONTRIBUTING.md) |
| Historical observations | [Frozen records](validation/archive/README.md) |

The [documentation checklist](DOC_REVIEW_CHECKLIST.md) applies this ownership model.
[External asset guidance](ASSETS.md) covers separately authorized local data;
public examples and checks use synthetic or redistributable inputs.

## Short glossary

- **Deterministic simulation:** the same inputs and initial state produce the same results.
- **Fixed-point arithmetic:** integer-based fractions with explicit rounding and overflow rules.
- **RNG:** random-number generator; gameplay owns its sequence rather than using global randomness.
- **BSP / BLOCKMAP:** the map's binary space partition tree / collision grid.
- **Profile:** an explicitly selected simulation configuration with documented support limits.
- **Admission:** the checks deciding which behavior a configuration actually supports.
- **Fixture / fixture projection:** a controlled test setup / a documented adaptation
  between original game data and that setup, not proof of complete game support.
- **DSIM:** this project's deterministic simulation snapshot format; its versions
  are defined in STATE_FORMAT. It is not an original demo or savegame.
- **Original-C equivalence:** matching specific behavior observed from genuine
  original source functions under the stated inputs and adapters, not whole-game compatibility.

## Original-source investigations

- [M3g1 stepping and transaction-cost audit](M3G1_REFERENCE.md), with
  [observed evidence](validation/M3G1.md).

These technical notes retain their original investigation and observation context.
They are not live milestone-status pages; consult ROADMAP and the linked GitHub PR
for present status. Historical review statements are not new claims that work is pending.

| Foundation | Investigation |
| --- | --- |
| Numeric/headless simulation | [M2](M2_REFERENCE.md) |
| Collision and static sectors | [M3a](M3A_REFERENCE.md), [M3b](M3B_REFERENCE.md) |
| Actor lifecycle | [M3c](M3C_REFERENCE.md) |
| Shared damage and tracing | [M3d1](M3D1_REFERENCE.md) |
| Weapons and projectiles | [M3d2](M3D2_REFERENCE.md), [M3d2b](M3D2B_REFERENCE.md) |
| Perception and pursuit | [M3d3a](M3D3A_REFERENCE.md), [M3d3b](M3D3B_REFERENCE.md) |
| Selected monster attacks | [M3d3c](M3D3C_REFERENCE.md), [M3d3d](M3D3D_REFERENCE.md), [M3d3e](M3D3E_REFERENCE.md) |
| Demon melee/lifecycle | [M3d3f](M3D3F_REFERENCE.md), [M3d3g](M3D3G_REFERENCE.md) |
| Demon stand/run and map spawning | [M3d3h](M3D3H_REFERENCE.md), [M3d3i](M3D3I_REFERENCE.md) |
| Demon map totals and bounded credit | [M3d3j](M3D3J_REFERENCE.md) |
| Bounded larger static geometry | [M3f1](M3F1_REFERENCE.md), [measured evidence](validation/M3F1.md) |
| Expanded coordinates and spatial work | [M3f2](M3F2_REFERENCE.md), [measured evidence](validation/M3F2.md) |
| Demon gameplay on larger static maps | [M3f3](M3F3_REFERENCE.md), [measured evidence](validation/M3F3.md) |
| Canonical actor/state identity | [M3f4](M3F4_REFERENCE.md), [mapping](ACTOR_IDENTITIES.md), [observations](validation/M3F4.md) |
| Canonical Demon execution binding | [M3f5](M3F5_REFERENCE.md), [mapping](ACTOR_IDENTITIES.md#canonical-demon-execution-binding), [observations](validation/M3F5.md) |
| Original-ID registry and diagnostic contract | [M3f7](M3F7_REFERENCE.md), [registry](ACTOR_IDENTITIES.md#original-id-execution-registry), [observations](validation/M3F7.md) |
| Runtime identity storage audit | [M3f6](M3F6_REFERENCE.md), [native boundary](ACTOR_IDENTITIES.md#native-runtime-identity-boundary), [observations](validation/M3F6.md) |
| Context and target lifetimes | [M3f8](M3F8_REFERENCE.md), [contract](COMPATIBILITY.md#context-and-target-lifetime-contract), [observations](validation/M3F8.md) |
| Game-instance provenance | [M3f9](M3F9_REFERENCE.md), [contract](COMPATIBILITY.md#game-instance-provenance), [observations](validation/M3F9.md) |
| Original player body lifecycle | [M3g2](M3G2_REFERENCE.md), [body contract](COMPATIBILITY.md#original-player-body-lifecycle), [observations](validation/M3G2.md) |

[Reference tooling](../tools/reference/README.md) describes harness layout and
provenance. [Historical records](validation/archive/README.md) retain complete
original evidence and former guide contents with exact-byte copies and digests.

- [M3g3 original-player knockback and grounded movement](M3G3_REFERENCE.md).

- [M3g4 original-player commands and reaction-time execution](M3G4_REFERENCE.md), with [observations](validation/M3G4.md).

- [M3g5 active original-player roster and canonical Demon acquisition](M3G5_REFERENCE.md), with [observations](validation/M3G5.md).

- [M3g6 autonomous canonical Demon-to-player melee](M3G6_REFERENCE.md), with
  [observations](validation/M3G6.md) and [reproduction](TESTING.md#autonomous-demon-player-melee).


- [M3g7 genuine-player pistol combat](M3G7_REFERENCE.md), with
  [observations](validation/M3G7.md), [DSIM23](STATE_FORMAT.md#dsim-v23-genuine-player-pistol)
  and [reproduction](TESTING.md#genuine-player-pistol).

- [M3g8 original weapon expansion investigation](M3G8_REFERENCE_INVESTIGATION.md),
  [observations](validation/M3G8.md),
  [DSIM24](STATE_FORMAT.md#dsim-v24-original-player-weapon-expansion) and
  [reproduction](TESTING.md#original-player-weapon-expansion).

- [M3g9 original world pickup investigation](M3G9_REFERENCE_INVESTIGATION.md),
  [observations](validation/M3G9.md), [support](COMPATIBILITY.md#original-world-pickups)
  and [DSIM25](STATE_FORMAT.md#dsim-v25-original-world-pickups).

- [Historical evidence storage](EVIDENCE_STORAGE.md) and
  [shared weapon arithmetic](WEAPON_FORMULAS.md), with
  [maintenance observations](validation/ZSTD_WEAPON_MAINTENANCE.md).

- [M3g10 original armor investigation](M3G10_REFERENCE_INVESTIGATION.md),
  [observations](validation/M3G10.md), [support](COMPATIBILITY.md#original-armor-and-damage-absorption).
- [M3g11 original bonus investigation](M3G11_REFERENCE_INVESTIGATION.md),
  [observations](validation/M3G11.md), [support](COMPATIBILITY.md#original-health-and-armor-bonuses).
- [M3h1 original former-human investigation](M3H1_REFERENCE_INVESTIGATION.md),
  [observations](validation/M3H1.md), [support](COMPATIBILITY.md#original-former-humans-and-multi-monster-encounters)
  and [DSIM28](STATE_FORMAT.md#dsim-v28-original-former-humans-and-mixed-rosters).
- [M3h2 original Imp and fireball investigation](M3H2_REFERENCE_INVESTIGATION.md),
  [observations](validation/M3H2.md), [support](COMPATIBILITY.md#original-imps-and-fireballs)
  and [DSIM29](STATE_FORMAT.md#dsim-v29-original-imps-and-fireballs).
- [M3h3 wall-aware travel and real-map-scale investigation](M3H3_REFERENCE_INVESTIGATION.md),
  [observations](validation/M3H3.md), [support](COMPATIBILITY.md#original-monsters-at-real-map-scale)
  and [DSIM30](STATE_FORMAT.md#dsim-v30-original-monsters-at-real-map-scale).
- [M3h4 two-sided lines, static heights and vertical movement investigation](M3H4_REFERENCE_INVESTIGATION.md),
  [observations](validation/M3H4.md), [support](COMPATIBILITY.md#original-monsters-with-static-heights)
  and [DSIM31](STATE_FORMAT.md#dsim-v31-original-monsters-with-static-heights).
- [M3h5 non-axis (diagonal) walls investigation](M3H5_REFERENCE_INVESTIGATION.md),
  [observations](validation/M3H5.md), [support](COMPATIBILITY.md#original-monsters-among-diagonal-walls)
  and [DSIM32](STATE_FORMAT.md#dsim-v32-original-monsters-among-diagonal-walls).
- [M3h6 real-map scale investigation](M3H6_REFERENCE_INVESTIGATION.md),
  [observations](validation/M3H6.md), [IWAD counts](validation/M3H6_IWAD_COUNTS.md),
  [support](COMPATIBILITY.md#original-monsters-at-classic-map-scale)
  and [DSIM33](STATE_FORMAT.md#dsim-v33-original-monsters-at-classic-map-scale).
- [M3h7 doors, lifts, line and sector specials and sky investigation](M3H7_REFERENCE_INVESTIGATION.md)
  (Stage A, sky and static specials, implemented in M3h7; Stage B implemented in M3h8),
  [observations](validation/M3H7.md), [support](COMPATIBILITY.md#original-monsters-with-sky-and-static-specials)
  and [DSIM34](STATE_FORMAT.md#dsim-v34--original-monsters-with-sky-and-static-specials).
- [M3h8 doors, lifts, floors, switches and exit investigation](M3H8_REFERENCE_INVESTIGATION.md)
  (Stage B), [observations](validation/M3H8.md),
  [IWAD mover counts](validation/M3H8_IWAD_COUNTS.md),
  [support](COMPATIBILITY.md#original-monsters-with-doors-lifts-floors-switches-and-the-exit)
  and [DSIM35](STATE_FORMAT.md#dsim-v35--original-doors-lifts-floors-switches-and-the-exit).
- [M3h9 two-sided scrolling walls, static decorations and the E1M1 run investigation](M3H9_REFERENCE_INVESTIGATION.md)
  (part M3h9a implemented; barrels, pickup scale and the real-E1M1 harness are later parts),
  [observations](validation/M3H9.md), [IWAD THING counts](validation/M3H9_IWAD_COUNTS.md),
  [support](COMPATIBILITY.md#original-monsters-with-two-sided-scrolling-walls-and-static-decorations)
  and [DSIM36](STATE_FORMAT.md#dsim-v36--original-two-sided-scrollers-and-static-decorations).
- [M3h9b barrels, pickup scale and full thing angles investigation](M3H9B_REFERENCE_INVESTIGATION.md)
  (the last E1M1 load blockers, with the spawn-rule, height-fit and budget census),
  [observations](validation/M3H9B.md),
  [support](COMPATIBILITY.md#original-monsters-with-exploding-barrels-pickup-scale-and-every-thing-angle)
  and [DSIM37](STATE_FORMAT.md#dsim-v37--original-exploding-barrels-pickup-scale-and-every-thing-angle).
- [M3h9c real E1M1 against the genuine engine investigation](M3H9C_REFERENCE_INVESTIGATION.md)
  (the headless genuine build, the demo-format input, the per-tic comparison and decisions G1–G13;
  merged; [observations](validation/M3H9C.md)).
- [M4a Episode 1, map by map, against the genuine engine investigation](M4A_REFERENCE_INVESTIGATION.md)
  (the Episode 1 census, the original C facts for every mechanism E1M2–E1M9 add, the command
  domain, the original demos, the synthetic-WAD CI path, decisions H1–H12 and the part 1 /
  M4b / M4c order of work; Phase 2 in progress).
- [M4b the rest of Episode 1 (teleporters, the Baron, A_BossDeath, hard skill, DEMO1) investigation](M4B_REFERENCE_INVESTIGATION.md)
  (the E1M5/E1M8/E1M9 census on skills 2 and 3, the original C facts, the wire 23 Rust inventory, decisions
  I1–I13 and the plan; Phase 1 done, decisions pending the owner).
