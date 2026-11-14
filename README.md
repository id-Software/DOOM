# Rust DOOM

A rewrite of the original DOOM engine in Rust that aims to behave exactly like id
Software's code, down to every single step of game logic. (Rust DOOM is a working title.)

> **Machine-Maintained, Human-Directed Project**
>
> This project is developed and maintained primarily by AI coding agents under
> human direction and oversight. Agents implement code, write tests, maintain
> documentation, and assist with development workflows.
>
> Changes are subject to automated validation and human-defined review
> requirements. AI-generated code can contain mistakes, and passing tests does
> not guarantee correctness. This project is not affiliated with id Software or
> Bethesda.

## Why this exists

Most source ports (modern rewrites of a game's released source code) aim to be playable
first and accurate second. This project works the other way round. It only accepts
behavior that has been checked against the original Linux DOOM 1.10 C code (id Software's
1997 source release), comparing the entire game state after every tic, the 1/35-second
step in which DOOM updates its world. Anything not yet proven identical is refused: the
engine stops with an error instead of approximating.

That is what makes it fit for exact-behavior work once the ground is covered: replaying
recorded demos (files that replay a player's inputs, which only stay in sync if every tic
matches), speedrun and tool-assisted (TAS) research, and studying the original engine. Vanilla behavior, meaning
that of the original unmodified game, is the default here, quirks included, and game math
stays in Doom's original whole-number arithmetic, with no decimals, except for the one
division where the original itself used them. Once a behavior is admitted, nothing quietly
rounds it differently.

**It is not playable yet.** If you want to play Doom today, use an established source
port such as [Chocolate Doom](https://www.chocolate-doom.org/). Like every Doom engine,
this one needs your own game files, such as those from a copy of Doom you bought.

## Where it stands

The engine runs DOOM's game logic without graphics. It is driven by tests and small
example programs, on small test maps built for this project and, locally, on all nine
real Episode 1 maps (E1M5, E1M8 and E1M9 are work in progress, still under review). Today it can simulate:

- one player who moves, uses switches and doors, takes damage and dies, with the fist,
  pistol, shotgun and chaingun;
- the Demon, Zombieman, Shotgun Guy, Chaingunner and Imp, which see, chase, attack and
  fight each other, plus ammunition, health, armor and bonus pickups;
- walls at any angle, steps and ledges, windows, sky, damaging floors, secret areas,
  flickering lights and scrolling walls;
- the doors, lifts, moving floors, switches and exit, lamps, pillars, corpses and
  exploding barrels (with chain reactions) of E1M1, the first map;
- every Episode 1 door, floor, lift, stair, donut, light and exit special, keys and
  locked doors, powerups, the chainsaw, the rocket launcher and the Spectre;
- the real E1M1, E1M2, E1M3, E1M4, E1M6 and E1M7, which run in sync with the genuine
  engine (the original C code, built and run alongside the Rust code) for all 30 scripted
  runs: idle, walking to the exit, fighting, dying and random commands on each map (on
  E1M1, a barrels run instead of dying);
- in progress on a feature branch: teleporters, the Baron of Hell, the E1M8 boss floor, hard
  skill, and with them the real E1M5, E1M8 and E1M9, plus the original's own `DEMO1` as run
  by the original code. 61 scripted runs over all nine maps match the genuine engine; the
  milestone has not been reviewed or merged yet. The comparison harness is local-only.

What it cannot do yet:

- show graphics, play sound or offer menus, or play back the original's recorded demos
  as the original program did (only the one local `DEMO1`, as run by the original code,
  is covered);
- claim a tic-for-tic match of any map for every input: only the scripted runs recorded
  so far are matched;
- handle the other episodes, or easy or nightmare skill.

This is a long-term project, not a quick rewrite. Every behavior is checked against the
original before it's admitted, so progress is measured in proven features, not weeks.

## Which kind of Doom?

Four broad ways to play classic Doom; each non-vanilla one would be a separate optional mode.

- **Vanilla.** The original 1993 game, limits and quirks included: what this project builds.
- **Limit-removing.** Same game, but the original's hard limits on map size are lifted (Track 1 below).
- **Boom, MBF21 and ID24.** Extensions adding new doors, lifts and floors: a possible later mode.
- **Advanced, ZDoom-style.** Scripting, slopes and big mods: this project would go its own way (Track 3 below).

## Where it's (maybe) going

Ideas, not promises, one line each; the [roadmap](docs/ROADMAP.md#ideas-under-consideration) has more.

**Track 1: Faithful.** Vanilla stays exact; anything that changes it lives in a separate optional mode.

- **Original demo playback.** Replay vanilla demos to the last tic, the strongest proof of exactness.
- **Faster progress.** Port whole systems at once, checked on real maps and demos.
- **One vanilla mode, plus a limit-removing one.** Fold the policies into one; another lifts limits.
- **An optional "fixed" mode.** Off unless chosen, it would correct known original bugs.
- **The full game and Doom II.** After Episode 1, the other episodes and Doom II, just as exact.

**Track 2: Presentation.** The renderer only reads game state, so it can never change gameplay.

- **A faithful software renderer first.** It would look exactly like the original Linux DOOM 1.10.
- **Then a modern renderer.** Smooth high frame rates between tics, plus sector-based lighting.
- **VR.** A headset handles looking around while the mouse turns and aims.
- **A fully offline engine.** Your own game files, no internet, account or launcher.

**Track 3: Beyond vanilla.** A new engine on the verified core, in separate optional modes.

- **Modern source-port features, done cleanly.** ZDoom-style features on a fresh core.
- **Things the original never simulated.** Flowing water, blood and nukage; huge monster crowds.
- **Scripting and more platforms.** A scripting layer for new game logic; platforms such as mobile.
- **A live editor with rewind.** Change anything while the game runs, then rewind and replay from any moment.
- **"Why did that happen?"** Click a monster and see the exact decisions and random rolls behind what it did.
- **TAS tools that are vanilla-legal by construction.** Runs built frame by frame here play back on the original game.
- **Map and demo checks.** See exactly which line breaks vanilla limits, and which demos a map change desyncs.

## How it's verified

The original C source is the ground truth. Each supported behavior is run in both the
original code and the Rust code, and the whole game state is compared after every tic.
The expected results are recorded by a separate program written from the C output alone,
so Rust is never checked against its own output. Sanitizers catch hidden memory and
arithmetic errors while the original C code runs. Fuzzing feeds in huge numbers of random
inputs to find anything that slips through. Mutation testing, which deliberately breaks
the code, checks that the tests themselves notice. When something is not proven identical,
the engine refuses it rather than guessing.

## Try it

You need Rust via [rustup](https://rustup.rs/) (stable, or at least 1.85.0) and a C compiler
(on Windows, the Visual Studio build tools) for a small test-only compression library. No
game files are needed; it builds on Linux, Windows and macOS. From the cloned top folder:

```sh
cargo build --workspace --locked
cargo test --workspace --locked
```

Three examples worth running:

```sh
cargo run -p doom-sim --example player_barrels --locked  # exploding barrels, chain reactions, monsters
cargo run -p doom-sim --example player_movers --locked   # a player, monsters, doors, lifts and the exit
cargo run -p doom-map --example parse_map --locked       # reading DOOM's map format
```

Each runs a short scripted scene on a test map and prints a one-line summary. None needs
an IWAD (the main game data file, such as `doom.wad`), a graphics card, audio or a network.

## Policies (for developers)

New gameplay is switched on through optional policies, each selected explicitly and adding
one slice of Doom to the one before. Older policies never change, so earlier results stay
reproducible, much like compatibility levels. The number in brackets is the policy's
identifier in saved test snapshots; earlier Demon profiles are under
[versioned gameplay profiles](docs/COMPATIBILITY.md#versioned-gameplay-profiles).

| Policy (wire ID) | Adds | Contract |
| --- | --- | --- |
| `DemonNormalV1` (1) | The bounded Demon closure with original actor types and states stored directly | [Original-ID Demon](docs/COMPATIBILITY.md#original-id-demon-profile) |
| `PlayerBodyLifecycleV1` (2) | One genuine original player body: direct damage, pain, death and a retained corpse | [Player body](docs/COMPATIBILITY.md#original-player-body-lifecycle) |
| `PlayerBodyKnockbackV1` (3) | Physical knockback and grounded wall collision for that player | [Knockback](docs/COMPATIBILITY.md#original-player-grounded-knockback) |
| `PlayerCommandMovementV1` (4) | Player commands (turning, thrust) and the source reaction-time delay | [Command movement](docs/COMPATIBILITY.md#original-player-command-movement) |
| `PlayerDemonAcquisitionV1` (5) | One Demon that sees and pursues the active player, without melee | [Acquisition](docs/COMPATIBILITY.md#active-original-player-and-demon-acquisition) |
| `PlayerDemonMeleeV1` (6) | Source-timed Demon melee against the player, with damage, pain and death | [Demon melee](docs/COMPATIBILITY.md#autonomous-original-demon-player-melee) |
| `PlayerDemonPistolV1` (7) | The genuine pistol: psprites, ammunition, refire spread, hitscan and noise alerts | [Pistol](docs/COMPATIBILITY.md#genuine-player-pistol-combat) |
| `PlayerDemonWeaponsV1` (8) | Fists, shotgun, chaingun, weapon fallback and manual switching | [Weapons](docs/COMPATIBILITY.md#original-player-weapon-expansion) |
| `PlayerDemonPickupsV1` (9) | World ammunition, stimpacks, medikits and shotgun/chaingun pickups | [Pickups](docs/COMPATIBILITY.md#original-world-pickups) |
| `PlayerDemonArmorV1` (10) | Green and blue armor with source absorption and depletion | [Armor](docs/COMPATIBILITY.md#original-armor-and-damage-absorption) |
| `PlayerDemonBonusesV1` (11) | Health and armor bonuses (+1 up to 200) and counted-item statistics | [Bonuses](docs/COMPATIBILITY.md#original-health-and-armor-bonuses) |
| `PlayerMonstersV1` (12) | Former humans beside the Demon, up to eight monsters, infighting and item drops | [Former humans](docs/COMPATIBILITY.md#original-former-humans-and-multi-monster-encounters) |
| `PlayerMonstersImpV1` (13) | The Imp and its dodgeable fireball | [Imps](docs/COMPATIBILITY.md#original-imps-and-fireballs) |
| `PlayerMonstersMapScaleV1` (14) | Travel bounded by the loaded walls in walled rooms up to 2,048 units | [Map scale](docs/COMPATIBILITY.md#original-monsters-at-real-map-scale) |
| `PlayerMonstersHeightsV1` (15) | Static floor/ceiling heights joined by two-sided lines: steps, ledges, windows, falls | [Static heights](docs/COMPATIBILITY.md#original-monsters-with-static-heights) |
| `PlayerMonstersDiagonalV1` (16) | Linedefs of any direction inside a closed boundary; one corner jump refused | [Diagonal walls](docs/COMPATIBILITY.md#original-monsters-among-diagonal-walls) |
| `PlayerMonstersRealMapV1` (17) | WAD-format geometry limits (E1M1-sized) under a per-map arithmetic certificate | [Classic-map scale](docs/COMPATIBILITY.md#original-monsters-at-classic-map-scale) |
| `PlayerMonstersSpecialsV1` (18) | Sky ceilings, light thinkers, damage floors, secrets and one-sided scrolling walls | [Sky and specials](docs/COMPATIBILITY.md#original-monsters-with-sky-and-static-specials) |
| `PlayerMonstersMoversV1` (19) | E1M1's seven door, lift, floor, switch and exit line types; the tic after the exit is refused | [Movers](docs/COMPATIBILITY.md#original-monsters-with-doors-lifts-floors-switches-and-the-exit) |
| `PlayerMonstersE1M1V1` (20) | Two-sided scrolling walls and E1M1's seven static decoration types (lamps, pillars, candelabras, corpses and gore) | [E1M1 decorations](docs/COMPATIBILITY.md#original-monsters-with-two-sided-scrolling-walls-and-static-decorations) |
| `PlayerMonstersBarrelsV1` (21) | Exploding barrels with the original's radius attack and chain reactions, 64 pickups, every map-thing angle and the relaxed spawn rule | [Barrels](docs/COMPATIBILITY.md#original-monsters-with-exploding-barrels-pickup-scale-and-every-thing-angle) |
| `PlayerMonstersLevelV1` (22) | The player start at its place in the map's thing list, with its angle; the full player think, including damage-flash decay and the dead player's turn toward its attacker: the real E1M1 run | [Level runs](docs/COMPATIBILITY.md#original-level-runs-the-player-start-in-lump-order-and-the-full-player-think-wire-22) |
| `PlayerMonstersEpisode1V1` (23) | Episode 1, part 1: every door, floor, lift, stair, donut, light and exit special, keys, powerups, chainsaw, rocket launcher and Spectre: the real E1M2-E1M4, E1M6 and E1M7 runs | [Episode 1, part 1](docs/COMPATIBILITY.md#original-episode-1-part-1-every-door-floor-plat-stair-donut-light-and-exit-special-keys-items-powers-chainsaw-rocket-launcher-and-spectre-wire-23) |
| `PlayerMonstersEpisode1V2` (24), in progress | Episode 1, part 2: teleporters, the Baron of Hell and the E1M8 boss floor, hard skill: the real E1M5, E1M8, E1M9 and `DEMO1` runs | [Episode 1, part 2](docs/COMPATIBILITY.md#original-episode-1-part-2-teleporters-the-baron-of-hell-a_bossdeath-and-hard-skill-wire-24) |

More developer reading:

- [Compatibility](docs/COMPATIBILITY.md) states the limits of every policy, the
  [validation guide](docs/VALIDATION.md) the method, and
  [Testing](docs/TESTING.md#asserted-headless-examples) every example and check.
- [Architecture](docs/ARCHITECTURE.md): the small crates and how
  [actors and their ordering](docs/ARCHITECTURE.md#actors-lifecycle-and-ordering) work.
- [Stepping contract](docs/COMPATIBILITY.md#stepping-and-diagnostics),
  [actor identities](docs/ACTOR_IDENTITIES.md) and the [roadmap](docs/ROADMAP.md).
- The [documentation index](docs/README.md) and the
  [historical evidence archive](docs/validation/archive/README.md).

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md): each change needs evidence from the original
source, tests, documentation and owner review. The Python tools under `tools/` are a gentle
place to start. Standing rules for AI agents are in [AGENTS.md](AGENTS.md).

## License and attribution

The project preserves id Software's original source, history and attribution. The Rust
workspace is GPL-2.0-only; see the [license notice](LICENSE.TXT) and the
[original release README](README.TXT). Commercial game data stays external and is never
distributed here; see the [external asset guide](docs/ASSETS.md).
