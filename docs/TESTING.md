# Testing and reproduction

Run commands from the repository root. Public checks use synthetic or legally
redistributable fixtures; no commercial IWAD, renderer, audio device or network
service is required after toolchain/dependency installation.

## Required Rust checks

Full workspace builds/tests require a host C compiler (MSVC on Windows, or a
supported C compiler on Linux/macOS) for bundled test/tool zstd. No system zstd
installation, Clang/bindgen runtime, or zstd executable is required. Normal
`cargo build -p doom-sim --locked` retains its Rust-only production dependency
graph; the native codec is a dev/tool dependency. Original-C observers below
separately require their documented GNU C host.

```sh
cargo build --workspace --locked
cargo fmt --all -- --check
cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p doom-sim -p doom-types --release --locked
cargo +1.85.0 test --workspace --locked
```

Set `RUSTDOCFLAGS="-D warnings"` in your shell, then run:

```sh
cargo doc --workspace --no-deps --locked
```

Stable toolchain configuration is in [rust-toolchain.toml](../rust-toolchain.toml).
MSRV is Rust 1.85.0. Warnings-denied Clippy and rustdoc are CI requirements.

## Original C and independent canonical bytes

On a supported GNU C host (Linux, 32-bit int/fixed/angle, 16-bit short,
64-bit pointers, little-endian):

```sh
python3 tools/reference/run_reference.py check_references.py
python3 tools/reference/run_reference.py m3f1_reference.py --check
python3 tools/reference/run_reference.py m3f1_snapshot_goldens.py --check
```

The frozen historical helper covers the original reference pairs. Run the
separate milestone commands below as well; it does not discover newer tools.
It does not regenerate expectations to match Rust. For the expanded geometry slice:

```sh
python3 tools/reference/run_reference.py m3f1_reference.py --check
python3 tools/reference/run_reference.py m3f1_snapshot_goldens.py --check
cargo test -p doom-sim --test m3f1_reference --test m3f1_policies --locked
```

All platforms compare committed C observations and independent full bytes/digests
in Rust tests. Source-body reproduction is intentionally Linux-only in CI.
[Reference tooling](../tools/reference/README.md) documents provenance and adapters;
[STATE_FORMAT](STATE_FORMAT.md) specifies DSIM. DSIM is diagnostic state, not a
vanilla demo or savegame.

## Asserted headless examples

```sh
cargo run -p doom-map --example parse_map --locked
cargo run -p doom-sim --example demon_map_context --locked
python3 tools/reference/check_examples.py
python3 tools/reference/check_map_scale.py
python3 tools/reference/check_map_extent.py
```

The frozen historical helper and separate milestone helpers below compare
debug/release output for the committed simulation examples. They span
movement, collision, sectors, lifecycle, combat, weapons/projectiles, perception,
pursuit, selected attacks and Demon gameplay/map context. Assertions use committed
original-C evidence and independent snapshots where applicable. Earlier examples
and equality helpers retain their historical bytes.

Every headless example, each on synthetic public maps with no commercial IWAD, GPU,
audio device or network service, runs with one of these commands:

```sh
cargo run -p doom-map --example parse_map --locked
cargo run -p doom-sim --example actor_states --locked
cargo run -p doom-sim --example canonical_demon --locked
cargo run -p doom-sim --example chaingunner_attacks --locked
cargo run -p doom-sim --example combat_arena --locked
cargo run -p doom-sim --example context_ownership --locked
cargo run -p doom-sim --example demon_expanded_map --locked
cargo run -p doom-sim --example demon_lifecycle --locked
cargo run -p doom-sim --example demon_map_context --locked
cargo run -p doom-sim --example demon_map_spawn --locked
cargo run -p doom-sim --example demon_melee --locked
cargo run -p doom-sim --example demon_stand_run --locked
cargo run -p doom-sim --example diagnostic_free --locked
cargo run -p doom-sim --example game_instance --locked
cargo run -p doom-sim --example headless --locked
cargo run -p doom-sim --example map_extent --locked
cargo run -p doom-sim --example map_scale --locked
cargo run -p doom-sim --example monster_attacks --locked
cargo run -p doom-sim --example monster_perception --locked
cargo run -p doom-sim --example monster_pursuit --locked
cargo run -p doom-sim --example multisector_arena --locked
cargo run -p doom-sim --example original_identity --locked
cargo run -p doom-sim --example original_player --locked
cargo run -p doom-sim --example original_registry --locked
cargo run -p doom-sim --example player_arena --locked
cargo run -p doom-sim --example player_armor --locked
cargo run -p doom-sim --example player_bonuses --locked
cargo run -p doom-sim --example player_commands --locked
cargo run -p doom-sim --example player_demon --locked
cargo run -p doom-sim --example player_demon_melee --locked
cargo run -p doom-sim --example player_diagonal --locked
cargo run -p doom-sim --example player_e1m1 --locked
cargo run -p doom-sim --example player_heights --locked
cargo run -p doom-sim --example player_imps --locked
cargo run -p doom-sim --example player_knockback --locked
cargo run -p doom-sim --example player_map_scale --locked
cargo run -p doom-sim --example player_monsters --locked
cargo run -p doom-sim --example player_movers --locked
cargo run -p doom-sim --example player_pistol --locked
cargo run -p doom-sim --example player_real_map --locked
cargo run -p doom-sim --example player_specials --locked
cargo run -p doom-sim --example player_weapon_pack --locked
cargo run -p doom-sim --example player_weapons --locked
cargo run -p doom-sim --example player_world_pickups --locked
cargo run -p doom-sim --example projectiles --locked
cargo run -p doom-sim --example runtime_identity --locked
cargo run -p doom-sim --example shotgunner_attacks --locked
```

Each prints diagnostic results and asserts its expected behavior. A
`tools/reference/check_*.py` helper runs each example in debug and release and asserts
equal output (`check_headless.py` covers `headless`, `player_arena`,
`multisector_arena`, `actor_states`, `combat_arena` and `player_weapons`; the others
have a helper of their own name); the milestone sections below give each one's
reproduction commands. A few notes on the less obvious ones: `demon_expanded_map` runs
source-tested Demon gameplay on larger synthetic maps; `canonical_demon` drives that
behavior with original MT_SERGEANT/SARG identities and does not admit Spectre or
resurrection; `runtime_identity` checks the
[native identity boundary](ACTOR_IDENTITIES.md#native-runtime-identity-boundary) against
unchanged historical captures; `original_registry` checks
[actual original-ID storage](ACTOR_IDENTITIES.md#original-id-execution-registry) and
bounded v20/v21 diagnostic reading; `context_ownership` checks scoped target operations
and retirement against compact C observations (see the
[ownership contract](COMPATIBILITY.md#context-and-target-lifetime-contract));
`game_instance` demonstrates [qualified references](COMPATIBILITY.md#game-instance-provenance)
and reuses an existing C scenario without admitting new gameplay; `map_extent`
demonstrates expanded unbound coordinates and spatial work; `player_e1m1` runs the
synthetic M3h9 rooms, not the real E1M1.

## Fuzzing and sanitizers

Install cargo-fuzz and a supported nightly Rust toolchain, then:

```sh
cargo +nightly fuzz build --sanitizer address
cargo +nightly fuzz run map_scale --sanitizer address -- -runs=4096 -max_len=35 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The build covers all committed fuzz targets. The map-scale target mutates public
classic geometry, source BLOCKMAP references, malformed indexes and setup budgets,
then exercises bounded controls/queries/stepping, stale identities and complete-state
rejection in two independent instances. Actual corpus/model/execution results are
owned by [M3f1 evidence](validation/M3F1.md). Historical campaigns remain in their
original records, including [M3d3j](validation/M3D3J.md).
If your environment requires `ASAN_OPTIONS=detect_leaks=0`, disclose it; that
campaign has no LeakSanitizer coverage. Bounded campaigns are not exhaustive.

## Documentation and preservation

```sh
python3 tools/check_docs.py
```

Verify preserved upstream files against baseline
`a77dfb96cb91780ca334d0d4cfd86957558007e0`, historical reference/helper bytes against
the feature base, Git ancestry and license notices. The
[archive manifest](validation/archive/manifest.json) records exact historical
ledger bytes and rebased navigation copies. Private opt-in asset tests stay ignored;
see [ASSETS](ASSETS.md) without accessing such assets for public validation.

For earlier per-profile procedures and qualified historical sanitizer observations,
read the [complete testing archive](validation/archive/pre-m3d3j-testing.md).

## BLOCKMAP corner regression

```sh
python3 tools/reference/blockmap_corner_wad.py --check
python3 tools/reference/blockmap_corner_probe.py
cargo test --locked -p doom-sim --test blockmap_corner
cargo test --locked -p doom-sim --test blockmap_corner --release
```

The fixture check compares bytes in memory and returns nonzero on a difference
or a missing file. Run the generator without `--check` to rebuild the fixture,
or pass `--out /tmp/blockmap_corner.wad` to write a separate copy. The genuine
probe uses the GNU C host and M4b runner described above. The
[corner evidence](validation/BLOCKMAP_CORNER.md) records its cases and the
scope of the Rust rerun.

## Stacked documentation PRs

The existing [workflow](../.github/workflows/rust.yml) automatically validates PRs
targeting master; its branch filter does not run on a PR targeting a feature
branch. Keep such a documentation stack draft and report its exact-head CI as
unavailable. A passing prerequisite run is evidence for that base commit, not the
new docs head. After the prerequisite merges, retarget to master, recheck the
resulting diff/base and run fresh required local and exact-head platform checks.
Do not change workflow filters or create a duplicate PR to work around this.

## Map-scale measurements

On Linux with GNU C and a release Rust toolchain:

```sh
cargo run -p doom-sim --example map_scale --locked
python3 tools/reference/summarize_m3f1.py
python3 tools/measure_map_scale.py
```

The last command prints three-sample host measurements as JSON; it is not a CI
performance threshold or a golden file. It builds the release example and a small
native `wait4` launcher, then measures preparation excluding parsing, collision,
sight, snapshot-inclusive stepping, requested prepared capacities and process peak
RSS. Recorded results and method limitations live in [M3f1 evidence](validation/M3F1.md).

## Expanded map extents and spatial work

Run the new pair alongside every historical source/encoder pair above:

```sh
python3 tools/reference/run_reference.py m3f2_reference.py --check --overflow-check
python3 tools/reference/run_reference.py m3f2_snapshot_goldens.py --check
python3 tools/reference/summarize_m3f2.py
cargo run -p doom-sim --example map_extent --locked
python3 tools/reference/check_map_extent.py
```

The overflow check recompiles genuine C with signed-overflow, integer-division and
float-cast sanitization and requires identical outputs. It does not claim portable
ISO-C negative shifts or exhaustive undefined-behavior coverage; the GNU source
setup-shift contract is declared in [source findings](M3F2_REFERENCE.md).
To reproduce the separately excluded original C failure on GNU/Linux:

```sh
python3 tools/reference/run_reference.py m3f2_reference.py --check --probe-undefined-clip
```

The tool expects the original abs(INT_MIN) sanitizer failure and prints it. This
is evidence for profile 19's recoverable rejection, not a passing original-C full
sight comparison. Standard reference modes never enter the undefined continuation.

The source/encoder names deliberately stay outside historical `generate*.py`
discovery; frozen aggregators/helpers remain unchanged.

For the bounded synthetic campaign, after installing cargo-fuzz/nightly:

```sh
python3 tools/seed_map_extent_fuzz.py fuzz/corpus/map_extent
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run map_extent fuzz/corpus/map_extent --sanitizer address -- -runs=4096 -max_len=35 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The environment setting explicitly disables leak detection. Do not report that
campaign as LeakSanitizer coverage. Corpus and artifacts remain ignored development
outputs. Inputs exercise expanded geometry/origins, shared lists, repeated BSP
children, budgets, stale identities, controls, pause/RNG and complete-state errors
in two instances; no proprietary data is needed.

On Linux with GNU C, collect separate release-process samples:

```sh
python3 tools/measure_map_extent.py
```

This prints JSON, not a performance golden or acceptance threshold. The same small
fixture is measured under profiles18/19; larger fixtures have different operation
volumes and cannot establish like-for-like regressions. Parsing and full preparation
are timed separately; preparation includes geometry/sight validation, BLOCKMAP and
topology work and canonical-cache construction, not isolated subsystem timers.
[Measured observations](validation/M3F2.md) own results, memory scope and limitations.

## Expanded-map Demon gameplay composition

Run both M3f3 source/encoder adapters alongside all historical pairs:

```sh
python3 tools/reference/run_reference.py m3f3_reference.py --check --overflow-check --probe-undefined-angle
python3 tools/reference/run_reference.py m3f3_snapshot_goldens.py --check
python3 tools/reference/run_reference.py m3f3_clipping_reference.py --check --overflow-check
python3 tools/reference/run_reference.py m3f3_clipping_snapshot_goldens.py --check
cargo test -p doom-sim --test m3f3_reference --test m3f3_clipping_reference --locked
cargo run -p doom-sim --example demon_expanded_map --locked
python3 tools/reference/check_demon_expanded_map.py
python3 tools/measure_demon_expanded_map.py
```

Every capture compares complete ordered event-field bytes, in addition to full
snapshot fixtures and per-tic digests. Event projections come from the independent
C encoder and include named-field/first-byte divergence diagnostics.

The extra adapter explicitly executes non-cardinal AI rays through differing
openings and source slope clipping on larger maps. Both use the same genuine
source bodies. The strict audit excludes the original signed ANG45 overflow; the
optional expected-failure component extracts the exact original expression and
requires its positive-180-degree sanitizer diagnostic. That finding proves the
new angle rejection policy, not a successful full spawn after undefined behavior.
Signed-shift/GNU ABI assumptions remain declared. Rust ASan and C arithmetic
sanitization are separate evidence families.

```sh
python3 tools/seed_demon_expanded_map_fuzz.py fuzz/corpus/demon_expanded_map
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run demon_expanded_map fuzz/corpus/demon_expanded_map --sanitizer address -- -runs=4096 -max_len=35 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The bounded model uses two instances, at most eight four-byte operations and
64 tic calls per instance. It mutates public geometry/THING admission, roles,
identities, actions, damage, sight and setup/operation atomicity. Its exact MIN
seed exercises read-only rejection; nested AI rejection is separately injected in
Rust safety tests. The evolving corpus stays ignored. Disabled leak detection is
not LeakSanitizer coverage. Actual executions and timings belong in
[M3f3 observations](validation/M3F3.md).

Measurements use separate release processes and source-backed inputs; only the
same small input under profiles 17/20 is a direct gameplay comparison. Larger-map
unbound and typed workloads have different behavior and observation volumes.
The tool records construction/spawn/query/Chase/step/snapshot timings and native
peak RSS. It defines no performance threshold; raw samples and caveats belong in
the evidence record.

## Original actor/state identity metadata

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3f4_reference.py --check --sanitize
cargo test -p doom-sim --test m3f4_reference --locked
python3 tools/reference/check_original_identity.py
python3 tools/seed_original_identity_fuzz.py fuzz/corpus/original_identity
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run original_identity fuzz/corpus/original_identity --sanitizer address -- -runs=4096 -max_len=68 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The C observer compiles untouched original tables and evaluates actual enum
symbols. Aborting action stubs supply pointer-to-symbol metadata only. It never
reads Rust output. Its separate schema preserves every historical DSIM fixture.
Rust policy tests exhaust u16 candidates, signed extremes and altered registry
records; existing independent captures verify chronology and RNG retention.
The fuzz target bounds name input, checks descriptors/round trips and mutates
registry properties. It does not broaden gameplay. Both sanitizer commands disable
leak detection explicitly; they provide no LeakSanitizer coverage.
Run all historical commands above as well. Reuse the unmodified
`tools/measure_demon_expanded_map.py` on the base and feature with the same
release toolchain/workloads, three samples each; compare identical cases/profiles.
[M3f4 observations](validation/M3F4.md) own results and measurement limitations.

## Canonical Demon execution binding

These checks retain the original canonical C identities during gameplay, then
compare an explicitly projected native Profile20 representation. They add no
DSIM schema or gameplay profile. Run them alongside every historical check above:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3f5_reference.py --check --overflow-check --address-check
python3 tools/reference/run_reference.py m3f5_snapshot_goldens.py --check
cargo test -p doom-sim --test m3f5_reference --test m3f5_policies --locked -- --nocapture
python3 tools/reference/check_canonical_demon.py
cargo run -p doom-sim --example canonical_demon --locked
cargo run -p doom-sim --example canonical_demon --release --locked
```

The observer retains raw original type/state fields in DOOM-DEMON-EXEC-v1 files.
The independent encoder declares identity-only translation before reusing the
historical DSIM-v20 encoder. Rust compares C physical/statistics/RNG observations,
canonical definition queries, ordered event fields and independent snapshots.
Separate policy tests cover metadata mutation, wrong type/owner, raise, corpse,
stale-handle and atomicity boundaries. See [investigation](M3F5_REFERENCE.md) and
[observations](validation/M3F5.md) for exact coverage and limitations.

```sh
python3 tools/seed_canonical_demon_fuzz.py fuzz/corpus/canonical_demon
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run canonical_demon fuzz/corpus/canonical_demon --sanitizer address -- -runs=4096 -max_len=35 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_demon_expanded_map.py > /tmp/m3f5-legacy-performance.json
python3 tools/measure_canonical_demon.py > /tmp/m3f5-api-performance.json
```

Build-all and actual campaign execution are different results. The seed recipe
creates fresh synthetic requests; evolving data stays ignored. Leak detection is
disabled in the documented managed-host campaign; no leak coverage is claimed.
The legacy benchmark supplies matching before/after workloads. The new canonical
API workload includes explicit validation/query costs and is reported separately;
it has no pre-M3f5 API baseline. Timing output is observational, never an assertion.

To exercise cross-filesystem provenance reproducibility without changing source
observations, reverse file discovery in both new publishers. This checks whole
publisher output bytes, separately from gameplay comparisons:

```sh
python3 - <<'PY'
import sys
from pathlib import Path
sys.path.insert(0, 'tools/reference')
import m3f5_reference as source
import m3f5_snapshot_goldens as encoder
raw = source.capture(list(source.main.legacy.base.FLAGS))
source.capture = lambda flags: dict(reversed(list(raw.items())))
source.run(True, False)
original = Path.glob
try:
    Path.glob = lambda path, pattern: iter(reversed(list(original(path, pattern))))
    encoder.run(True)
finally:
    Path.glob = original
PY
```

## Native runtime identity boundary

Run the complete historical suite and M3f4/M3f5 C/encoder checks above, plus:

```sh
cargo test -p doom-sim --test m3f6_reference --test m3f6_policies --locked -- --nocapture
python3 tools/reference/check_runtime_identity.py
cargo run -p doom-sim --example runtime_identity --locked
python3 tools/seed_runtime_identity_fuzz.py fuzz/corpus/runtime_identity
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run runtime_identity fuzz/corpus/runtime_identity --sanitizer address -- -runs=4096 -max_len=35 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_demon_expanded_map.py
python3 tools/measure_canonical_demon.py
```

The new source comparison reuses immutable M3f5 raw C observations and independent
snapshots/event fields; it does not generate another capture or claim new C input
coverage. Policy tests exhaust all u16 native candidates, canonical/native
collisions, direct-entry restrictions, purity, replay and stale/reused ownership;
private corruption tests ensure canonical ordinals in native storage reject
atomically. The new example runs in debug/release against existing C-derived output.

The fuzz model uses two instances, bounded native/canonical requests and at most
eight operations; corpus seeds include valid native slots, source ordinals and
sentinels. Build-all and campaign counts are separate. Leak detection is disabled
by the documented setting; no leak coverage or exhaustive proof is implied.
Retain matched before/after three-sample legacy and canonical-API workloads and
all outliers. Those M3f6 measurements contain no canonical-storage workload;
the separate original-ID workload below belongs to M3f7.
[M3f6 observations](validation/M3F6.md) retain the historical results.

## Original-ID registry and diagnostic contract

```sh
cargo test -p doom-sim --test m3f7_reference --test m3f7_policies --test m3f7_diagnostics --locked
python3 tools/reference/run_reference.py m3f7_snapshot_goldens.py --check
python3 tools/reference/check_original_registry.py
python3 tools/check_docs.py
```

The C comparison reads immutable M3f5 raw source observations against actual
ActorLife source IDs; it checks every ordered event payload by reusing the old
EV20 bytes with identity fields supplied by C rows. New compact diagnostics retain
provenance and two complete fixtures; no new C-input/gameplay coverage is claimed.
Keep all existing generators/encoders, metadata/source sanitizer checks and all
historical debug/release examples in the full validation run. New decoder policy
checks are structural/identity safety, not physical-state restoration proof.

```sh
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_original_registry_fuzz.py
python3 tools/seed_diagnostic_fuzz.py
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run original_registry fuzz/corpus/original_registry --sanitizer address -- -runs=4096 -max_len=34 -seed=1 -timeout=20 -rss_limit_mb=2048
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run diagnostic_format fuzz/corpus/diagnostic_format --sanitizer address -- -runs=4096 -max_len=131072 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_demon_expanded_map.py
python3 tools/measure_canonical_demon.py
python3 tools/measure_original_registry.py
```

## Context and target-lifetime validation

Run the new pair alongside every historical pair above:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3f8_reference.py --check --overflow-check --address-check
python3 tools/reference/run_reference.py m3f8_snapshot_goldens.py --check
cargo test -p doom-sim --test m3f8_reference --test m3f8_policies --locked -- --nocapture
cargo test -p doom-sim --doc --locked -- --show-output
python3 tools/reference/check_context_ownership.py
```

The observer supplies new controlled C inputs, not another implementation of the
engine. Original source/body hashes are checked against inherited provenance.
Logical retirement uses the declared storage quarantine; no freed-pointer or
allocator-reuse gameplay equivalence is asserted. The independent encoder reads
C observations and declared v21 schema policy, not Rust output. Rust policy tests
separately cover lifetime brands, local admission, rejected mutations, retained
references and generation reuse. Rustdoc output confirms the expected lifetime
errors rather than an unrelated syntax error. Whole workspace, release and MSRV
checks also include the focused tests and generation-exhaustion unit case.

```sh
python3 tools/seed_context_ownership_fuzz.py
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run context_ownership fuzz/corpus/context_ownership --sanitizer address -- -runs=4096 -max_len=128 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The new target decodes bounded operation chunks and malicious handle/definition/
state/roster candidates, compares two executions and checks complete-state
rejection. Continue relevant original-registry/diagnostic campaigns above; building
all targets does not mean all targets were executed. Leak detection disabled by
the environment is not leak coverage. Corpus files remain ignored local inputs.

For matched measurements, retain all before/after JSON from the unchanged
`measure_demon_expanded_map.py`, `measure_canonical_demon.py` and
`measure_original_registry.py` workloads. Also run:

```sh
python3 tools/measure_context_ownership.py
```

This new microbenchmark compares raw and scoped access paths on the same inputs,
mutations and retirement cycles, with identical final digests. Scoped observations
perform additional validation; permissive raw query latency is not equivalent
work. It is not a historical before baseline or a whole-simulation benchmark.
The [record](validation/M3F8.md) owns actual counts, samples, RSS and limitations.

The first two measurement tools retain unchanged workloads for matched baseline/
feature comparisons. The third is separate: source-ID diagnostics omit immutable
metadata and have different volume, so its timings are not a like-for-like legacy
speedup. Keep all three process samples/outliers and RSS. Build-all and executed
campaign counts are separate; disabled leak detection supplies no leak coverage.
[Design](M3F7_REFERENCE.md) and [observations](validation/M3F7.md) define the scope.

## Game-instance provenance validation

```sh
cargo test -p doom-sim --test m3f9_policies --test m3f9_reference --locked -- --nocapture
cargo test -p doom-sim --doc --locked -- --show-output
python3 tools/reference/check_game_instance.py
python3 tools/check_docs.py
```

The new reference test and example drive the existing M3f8 replacement/clear input
through qualified imports. Expected fields, RNG and complete event payloads still
come from frozen C observations and the independent encoder. No new C scenarios,
raw traces or diagnostic version are introduced. Continue every historical/source/
metadata/sanitizer command above. Rust policy tests separately cover identical raw
IDs in different worlds, clone/move/extraction, successful/failed restart, current/
pending/corpse/retired/stale/vacant/invalid actors, missing/foreign provenance, wrong
definitions, pure rejection and exact replay/snapshot/RNG preservation. Inspect
compile-fail diagnostics for invariant token escape and a non-Clone owner.

```sh
python3 tools/seed_game_instance_fuzz.py
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run game_instance fuzz/corpus/game_instance --sanitizer address -- -runs=4096 -max_len=128 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The bounded harness selects local opaque keys; input bytes never become InstanceId.
It compares two instances, malformed candidates, missing/foreign/old routing,
generation/state/definition guards, restart/state-clone and rejected-operation
atomicity. Continue the original-registry and diagnostic campaigns above. Building
all targets is separate from executing them. Record seeds, actual executions,
limits, RSS, findings and leak configuration; leak0 is not leak coverage.

Retain all four existing measurement tools unchanged for matched before/after
three-process samples; then measure the separate boundary workload:

```sh
python3 tools/measure_game_instance.py
```

This separately times checked import/export and compares identical gameplay with
an already scoped actor versus explicit reimport on each update. Validation work
differs; import is never inserted automatically into ordinary tic execution.
Retain every sample/outlier/RSS and avoid universal or causal performance claims.
The [evidence record](validation/M3F9.md) owns actual results and limitations.

## Snapshot-free stepping validation

```sh
cargo test -p doom-sim --lib advance_tests --locked
cargo test -p doom-sim --test m3g1_reference --test m3g1_policies --locked
python3 tools/reference/check_diagnostic_free.py
```

The unit capture-boundary hook checks success, pause and rejection without timing
assumptions. Complete snapshots/bytes compare across every supported profile;
reference tests reuse frozen C observations on both APIs, not Rust-generated
expectations. Historical callers, encoders and all example checks remain required.
The new example requests its final diagnostic explicitly.

On Linux/glibc with a C compiler, run advisory optimized measurements:

```sh
python3 tools/measure_stepping.py --check
python3 tools/measure_stepping.py > stepping-observations.json
```

Full measurement uses three separate processes per mode/workload, separate
allocation-counter processes with mandatory zero/positive controls, and private
preflight/staging instrumentation in the release test binary. Host-only
[interposition](../tools/measurement/alloc_probe.c) adds no engine dependency or
unsafe Rust. Counts cover the marked thread's intercepted allocator requests;
requested bytes are cumulative, not peak live heap or leaks. Timing/RSS samples
run without the counter. Private frozen-state component timers are not additive
to evolving update timers, and test-harness RSS differs from example RSS.
Process RSS includes setup and final verification outside the measured interval.
No shared-host timing threshold, CPU isolation or universal speed claim applies.
CI executes a small advisory smoke plus deterministic structural tests.
[The record](validation/M3G1.md) retains all samples and remaining costs.

The bounded refinement fuzz model uses the existing profile setups, pause,
commands, RNG, removals and invalid handles:

```sh
python3 tools/seed_stepping_fuzz.py /tmp/doom-stepping-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run stepping_equivalence /tmp/doom-stepping-seeds --sanitizer address -- -runs=4096 -max_len=97 -seed=1 -timeout=20 -rss_limit_mb=2048
```

Build all targets as above, and report executed targets separately. This setting
provides no leak coverage; bounded executions are not exhaustive. Frozen helper
reuse needs a narrow duplicate-module Clippy allowance in the new host input
modules; no runtime lint, assertion or historical helper is relaxed.

## Original player body validation

Run the standard workspace/MSRV/release/format/Clippy/rustdoc and historical
commands above unchanged, then the new source/encoder/example checks:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3g2_reference.py --check --overflow-check --address-check
python3 tools/reference/run_reference.py m3g2_snapshot_goldens.py --check
cargo test -p doom-sim --test m3g2_reference --test m3g2_policy --locked
cargo test -p doom-sim --lib original_player --locked
python3 tools/reference/check_original_player.py
cargo run -p doom-sim --example original_player --locked
cargo +nightly fuzz build --fuzz-dir fuzz
python3 tools/seed_player_fuzz.py /tmp/doom-player-corpus
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run original_player /tmp/doom-player-corpus -- -runs=4096 -seed=1 -max_len=97 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_player_body.py > /tmp/player-measurements.json
```

The source observer uses actual MT_PLAYER0 and canonical Demon12, with source
association fragment and explicit presentation-service boundaries. The independent
encoder uses C rows/schema policy, never Rust output. C field/event comparisons,
full-byte/digest encoding, scoped/generation/provenance safety, rollback and API
refinement are separate tests. The new reader is explicitly decode_player; the
historical decoder must still reject22. Old fixtures/expected bytes are immutable.

For matched production measurements run unchanged `tools/measure_stepping.py`
before/after on the same toolchain and inputs; use advance rows for gameplay.
Retain old five-tool measurements as separate step/setup/sight/collision controls.
New player measurements have no historical before-player equivalent. Their full
world construction+spawn timer is separate from100-update timers. Keep all three
process samples/RSS and separate allocation-process samples, controls and outliers.
No hard shared-host time gates, zero-allocation inference or leak-coverage claim.
See [scope](COMPATIBILITY.md#original-player-body-lifecycle),
[source investigation](M3G2_REFERENCE.md) and [results](validation/M3G2.md).

## Grounded original-player knockback

Run the new observers alongside every historical pair and M3g2 command:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3g3_reference.py --check --overflow-check --address-check
python3 tools/reference/run_reference.py m3g3_snapshot_goldens.py --check
cargo test -p doom-sim --test m3g3_reference --test m3g3_policy --locked
cargo run -p doom-sim --example player_knockback --locked
python3 tools/reference/check_player_knockback.py
python3 tools/measure_player_knockback.py
```

The genuine source observer retains map-THING spawning for Demon origins,
source association for one player body, eligible thrust and actual subsequent
XY/friction/axis-slide execution. Its manifest separates compiled inventory from
actual entry counts and new inputs from inherited bodies. The independent encoder
compares selected complete states and every frame's ordered event payloads without
Rust-generated expectations. Rust-only policy tests cover handles/provenance,
geometry/arithmetic rejection, rollback and both stepping APIs. See
[source findings](M3G3_REFERENCE.md), [support](COMPATIBILITY.md#original-player-grounded-knockback)
and [observed results](validation/M3G3.md).

After building all fuzz targets, use a fresh development corpus:

```sh
python3 tools/seed_knockback_fuzz.py fuzz/corpus/player_knockback
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_knockback fuzz/corpus/player_knockback --sanitizer address -- -runs=4096 -max_len=129 -seed=1 -timeout=20 -rss_limit_mb=2048
```

Also execute relevant unchanged player/stepping/provenance targets; building them
is not an executed campaign. Leak detection is disabled in the command above and
provides no LeakSanitizer evidence. Keep corpus/artifacts outside tracked fixtures.
The host measurement tool uses three independent timing/RSS processes and separate
allocation probes per workload/mode. Construction is distinct from100 updates;
new physical workloads have no executable baseline equivalent. Retain unchanged
historical tools and matched old-player workloads for before/after comparisons.


## Original player command movement

Read [source assumptions](M3G4_REFERENCE.md) and the
[bounded input contract](COMPATIBILITY.md#original-player-command-movement).
Keep every historical check above; the new observer/encoder are additive:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3g4_reference.py --check --overflow-check --address-check
python3 tools/reference/run_reference.py m3g4_snapshot_goldens.py --check
cargo test -p doom-sim --test m3g4_reference --test m3g4_policy --locked
python3 tools/reference/check_player_commands.py
cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_commands_fuzz.py /tmp/doom-command-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_commands /tmp/doom-command-seeds --sanitizer address -- -runs=4096 -max_len=129 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_player_commands.py > /tmp/doom-command-measurements.json
```

The source check intentionally verifies a negative-shift UB diagnostic separately;
its normalized gameplay observer must pass strict/ASan/UBSan equality. Defined
literal source cases are checked independently of negative-turn bit policy. This
is a command-gate harness, not full PlayerThink or a player roster. Rust tests
separately exercise ownership/provenance, inactive/stale input, bounds, late rollback,
full step/advance refinement and zero implicit advance captures. The new fuzz
model compares outcomes and complete rejected state, decodes bounded diagnostics,
and mutates external inputs; it is not an original-C oracle or exhaustive proof.

Retain matched existing measure_stepping, measure_player_body and
measure_player_knockback controls; new command workload timing is separate. Tools
report three independent timing/RSS and three allocation processes, raw samples
and validated allocator controls. Source fields/capture counts, actually executed
campaigns and host limitations belong in [M3g4 evidence](validation/M3G4.md).

## Active original player and Demon acquisition

The [capability contract](COMPATIBILITY.md#active-original-player-and-demon-acquisition)
and [source matrix](M3G5_REFERENCE.md) define fixed active player0, genuine Demon
acquisition, nonmelee pursuit and explicit rollback at the melee boundary.
Historical helpers and expected bytes remain unchanged.

```sh
cargo test -p doom-sim --test m3g5_reference --test m3g5_policy --locked
cargo test -p doom-sim --lib player_roster --locked
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3g5_reference.py --check --overflow-check --address-check
python3 tools/reference/run_reference.py m3g5_snapshot_goldens.py --check
python3 tools/reference/check_player_demon.py
cargo run -p doom-sim --example player_demon --locked
cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_demon_fuzz.py /tmp/doom-player-demon-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_demon /tmp/doom-player-demon-seeds --sanitizer address -- -runs=4096 -max_len=161 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_player_demon.py
```

The new source command harness is a declared bounded movement gate before
P_RunThinkers, not full active-player P_Ticker/PlayerThink execution. Never execute
all-inactive original P_LookForPlayers as a terminating test. Source death/corpse
retention is distinct from Rust fixed-body removal and stale/foreign-handle
rejection. Leak detection remains disabled under the tracing environment;
bounded fuzzing is neither exhaustive proof nor leak coverage.

Run every existing workspace/MSRV/release, example, historical reference/encoder,
metadata, parser/generator, documentation, preservation and cross-platform gate.
The new helper supplements all existing checks; the two asset-gated tests stay
ignored. Measure new workloads separately from matched frozen historical tools.
`advance` measures production stepping; diagnostic `step` is a separate mode.
Raw three-process timing/RSS/allocation samples and limitations belong in
[the engineering evidence](validation/M3G5.md).

## Autonomous Demon-player melee

Run the new observer/encoder alongside every historical pair and all existing
checks. Expected source and diagnostic files are independent of Rust output:

```sh
cargo test -p doom-sim --test m3g6_policy --test m3g6_reference --locked
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3g6_reference.py --check --overflow-check --address-check
python3 tools/reference/run_reference.py m3g6_snapshot_goldens.py --check
python3 tools/reference/check_player_demon_melee.py
cargo run -p doom-sim --example player_demon_melee --locked
cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_demon_melee_fuzz.py /tmp/doom-player-demon-melee-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_demon_melee /tmp/doom-player-demon-melee-seeds --sanitizer address -- -runs=4096 -max_len=162 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_player_demon_melee.py
python3 tools/measure_player_demon_melee.py --smoke
```

The default example checks original-C fields, RNG, complete ordered events and
independent diagnostics, including pure autonomous acquisition through lethal
damage and a terminal corpse. It compares every scripted tic through step and
advance followed by explicit snapshot. Separate Rust policy tests cover invalid
roles/generations/provenance, pending/retired storage, conservative impulse limits,
late failure after real staged damage and zero implicit advance captures.

The source tool reuses the frozen bounded command gate before original thinkers;
it does not run full P_Ticker/PlayerThink/DeathThink. Current range is rechecked at
attack time. Range misses, absent targets and ignored dead-body damage have
different RNG consequences. New turn inputs are nonnegative literal defined cases;
historical negative-turn qualification remains reused evidence. Never execute an
unbounded all-inactive source scan or dereference freed pointers.

The new fuzz model warms only bounded committed timelines, then exercises individual
checked operations in two instances, complete rejection/equivalence and diagnostic
corruption. Built targets and actually executed campaigns are separate observations.
Leak detection is disabled above; bounded runs are not exhaustive or leak coverage.

Measure old controls with their unchanged tools. New melee intervals each contain
100 shared-core updates, with setup and final explicit diagnostics excluded.
Repeated melee warms64 tics; corpse continuation warms300 tics before its measured
interval. Other labels start at tic0. Near-lethal health reduction, prior impulse
and reaction delay are declared setup; no timed interval injects an attack or
damage. Advance and diagnostic step are separate modes. Retain all three timing/
RSS and separate allocation-process samples, outliers and workload fingerprints.
The smoke mode is advisory and has no timing threshold. See
[source boundaries](M3G6_REFERENCE.md), [support](COMPATIBILITY.md#autonomous-original-demon-player-melee)
and [measured observations](validation/M3G6.md).


## Genuine player pistol

Use synthetic inputs only. Run the complete standard suite and every historical
source/encoder family, then the new genuine-player weapon checks:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3g7_reference.py --check --overflow-check --address-check
python3 tools/reference/run_reference.py m3g7_snapshot_goldens.py --check
cargo test -p doom-sim --test m3g7_reference --test m3g7_policy --locked
cargo test -p doom-sim --lib player_pistol --locked
python3 tools/reference/check_player_pistol.py
cargo run -p doom-sim --example player_pistol --locked
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_pistol_fuzz.py /tmp/doom-player-pistol-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_pistol /tmp/doom-player-pistol-seeds --sanitizer address -- -runs=4096 -max_len=162 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_player_pistol.py
python3 tools/measure_player_pistol.py --smoke
python3 tools/measure_legacy_weapons.py
```

The compiled-C observer executes genuine reborn/setup, weapon/psprite, aim/trace,
impact, damage and Demon functions over the declared command/weapon stage before
original thinkers. It does not execute complete PlayerThink/Ticker/DeathThink.
The independent encoder reads only source rows and explicit DSIM23 policy. Full
byte fixtures precede matching Rust integration. Separate actual aim-query tests
compare cells, candidates, intercepts, windows, targets and source stopping.
Compiled bodies, linked fixed math, instrumented function entries, source fields,
new composition evidence and reused historical coverage are counted separately.

Rust policy tests cover unsupported inputs, missing/pending/stale/reused/foreign
ownership, invalid weapon state, capacity and late rollback, empty-ammo fallback,
schema corruption, deterministic replay and full step/advance equivalence. These
are safety claims, not fabricated C lifetime behavior. The source driver defaults
commands after death and never scans an all-inactive roster or freed C pointer.

Fuzz targets built are distinct from campaigns actually run. Record seeds,
execution and byte budgets, timeouts, memory limits and findings. Leak detection
is disabled by the recorded ASAN_OPTIONS because of the environment's tracing
limits; this is not leak coverage or exhaustive testing. Historical campaign
parameters and fixtures remain unchanged.

Pistol benchmarks use100 production advances or separately100 diagnostic steps,
with source inputs, warmup/setup and final diagnostics outside the measured
interval. Three separate timing/RSS processes and three allocation-instrumented
processes retain all samples/outliers. New labels are continuous source scenarios,
not isolated shot/pain/retaliation costs; some labels intentionally share an
interval. Compare old controls only with identical frozen workloads/tools and
compiler settings. Whole-process RSS includes setup; requested allocation bytes
are cumulative and do not establish peak live heap or leaks. No shared-runner
hard timing gate is used. [Observations](validation/M3G7.md) hold raw records.

CI preserves all required jobs and the native Intel macOS runner. Linux adds the
new source/encoder checks and all stable platforms run the asserted pistol
example. Its timeout has additional headroom because the unchanged M3g6
exact-master Linux job approached28 minutes; no validation is removed or bypassed.
See [support](COMPATIBILITY.md#genuine-player-pistol-combat) and
[source arithmetic/harness limits](M3G7_REFERENCE.md).

## Original player weapon expansion

```sh
python3 tools/reference/evidence_pack.py
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3g8_reference.py --check --overflow-check --address-check
python3 tools/reference/run_reference.py m3g8_snapshot_goldens.py --check
cargo test -p doom-sim --test player_weapon_pack --locked
python3 tools/reference/check_player_weapon_pack.py
cargo run -p doom-sim --example player_weapon_pack --locked
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_weapon_pack_fuzz.py /tmp/player-weapon-pack-seeds
# From fuzz/; bounded example, not exhaustive coverage or leak detection:
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_weapon_pack /tmp/player-weapon-pack-seeds -- -seed=381 -runs=4096 -max_len=162 -timeout=15 -rss_limit_mb=1024
python3 tools/measure_player_weapons.py
```

The asserted example executes source command streams, not injected damage.
New DREFLZ1 test storage is reproducible with standard-library Python and a bounded
safe Rust test decoder. For inspection:

```sh
python3 tools/reference/evidence_pack.py --unpack tests/reference/m3g8_trace.dref --output /tmp/m3g8-trace.csv
```

Expected source fields/events and independent DSIM24 fixtures precede runtime
integration. Historical tools and expectations remain unchanged. All historical
checks above still apply. See [source qualification](M3G8_REFERENCE_INVESTIGATION.md)
and [observed results](validation/M3G8.md); source observation totals and actual
Rust-admitted comparison totals are deliberately reported separately.


## Original world pickups

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/run_reference.py m3g9_reference.py --check --overflow-check --address-check
python3 tools/reference/run_reference.py m3g9_snapshot_goldens.py --check
cargo test -p doom-sim --lib world_pickup_tests --locked
python3 tools/reference/check_player_world_pickups.py
cargo run -p doom-sim --example player_world_pickups --locked
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_world_pickups_fuzz.py /tmp/player-world-pickup-seeds
# From fuzz/; bounded execution, leak detection disabled:
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_world_pickups /tmp/player-world-pickup-seeds -- -seed=391 -runs=4096 -max_len=162 -timeout=15 -rss_limit_mb=1024
python3 tools/measure_player_world_pickups.py
```

All historical commands remain required. The new observer compiles actual source
GiveAmmo/GiveBody/GiveWeapon/TouchSpecialThing and reuses existing original map,
movement, player, weapon, damage and AI bodies. Source observation rows and
independent DSIM25 bytes precede matching runtime integration. The asserted
example uses fixed source command streams, actual world contacts and autonomous
combat; it never injects a bullet/melee hit to stand in for gameplay. Controlled
health/loadout/dropped setup cases remain explicitly separate. Direct vertical
components do not admit airborne physics. Source-only item-respawn queue and
geometry trace rows remain qualified observations.

The focused fuzz target exercises commands, contacts, resource/lifetime/provenance
boundaries, invalid operations and DSIM25 with exact advance/step equality and
full-state rejection checks. Record built executables separately from campaigns
run. ASAN_OPTIONS disables leak detection; bounded campaigns do not prove
exhaustive coverage. Benchmarks retain three timing/RSS and separate allocation
processes, frozen control workloads and independently labeled pickup intervals.
See [source matrix](M3G9_REFERENCE_INVESTIGATION.md) and
[observed validation](validation/M3G9.md).

## Historical zstd evidence

Run ordinary tests from the compressed checkout, without Git history or network
access after dependencies are installed. [Storage contract](EVIDENCE_STORAGE.md)
owns framing, budgets, preservation boundaries and dependency rationale.

```sh
python3 tools/reference/evidence_storage.py verify
python3 tools/reference/run_reference.py --suite
python3 tools/reference/run_reference.py generate_m3d2.py --check
# Separate migration audit; fetch this pinned object explicitly in shallow CI.
git fetch --no-tags --depth=1 origin 138c66b6c6b6d57d0b18cf945d8661722741bd66
python3 tools/reference/evidence_storage.py verify-git
cargo test -p doom-test-evidence --locked
python3 tools/seed_evidence_fuzz.py /tmp/doom-evidence-corpus
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run evidence_zstd /tmp/doom-evidence-corpus -- -seed=401 -runs=4096 -max_len=131072 -timeout=15 -rss_limit_mb=1024
```

`source_suite.json` retains the exact previous 65 CI source commands, including
strict arithmetic and sanitizer options. `--jobs N` runs them N at a time in N private
workspaces, scheduled longest-first from the committed duration table
`source_suite_durations.json` (measured once on an idle host; a command without an
entry runs last); every command still only compares against committed bytes, so the
order changes no result, and `--jobs 1` is the historical sequential run. CI passes
`--jobs 4` on the four-vCPU Ubuntu runner. `--report <file>` writes each command's
exit status, seconds and stdout digest. The wrapper also supports individual
frozen `--check` scripts. Direct legacy generators expect raw paths and therefore
run inside this wrapper after migration. Historical archived commands describe
their old checkout; archived bytes remain untouched. The wrapper changes no
expected output, generator or encoder.

For deterministic recompression, extract an original file from the manifest's
pinned Git object into a temporary path, then use the locked `evidence encode`
CLI (`cargo run -p doom-test-evidence --bin evidence --locked -- encode RAW ZST`).
Compare the produced compressed SHA-256 with the manifest before replacing any
artifact. This command pins bundled encoder version/parameters and verifies its
own actual round trip. `migrate` is a one-time raw-tree operation; it deliberately
fails when original raw files are absent.

## Shared weapon formula validation

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/weapon_formulas_reference.py --check --sanitize
cargo test -p doom-sim --lib formula_tests --locked
cargo test -p doom-sim --lib legacy_weapon_stages --locked
# Optional private stage-cost observations, separate from whole advance/step.
DOOM_WEAPON_STAGE_COST=1 cargo test -p doom-sim --release --lib legacy_weapon_stages --locked -- --nocapture --test-threads=1
```

[Formula contract](WEAPON_FORMULAS.md) describes source arithmetic, shared callers
and the bounded component observer. Run the full historical suite above as well;
component hashes do not establish hitscan/combat or whole-weapon equivalence.
The optional stage harness prepares 100 independent source-fixture worlds outside
each interval and measures preview or already-preflighted weapon commit only.
It is not a second gameplay loop, a production API or a whole-tic benchmark.

## Maintenance measurements

Keep source/test/fuzz execution outside the quiet measurement window. Use the
same release compiler, map/command inputs and process counts for before/after.
Retain each established `tools/measure_*.py` control output, all samples and
allocation arrays; [engineering observations](validation/ZSTD_WEAPON_MAINTENANCE.md)
own actual results. These are advisory measurements, not CI timing thresholds.

```sh
# Raw root is a detached checkout of the manifest's exact baseline object.
python3 tools/measure_evidence.py RAW_BASELINE_ROOT
python3 tools/measure_weapon_stages.py --logs /tmp/doom-weapon-stage-samples
```

The loading microbenchmark uses the same release executable for raw files and
compressed decode, checks original lengths/hashes and measures separate allocation
processes. It differs from the original embedded include_str architecture. Measure
actual reference test binaries separately (three processes per debug/release mode)
to capture parser/comparison overhead. Whole-gameplay controls use advance/step
as already labeled by each preserved measurement driver.


## Original armor and sustained combat

Run the frozen historical suite through its existing wrapper. New armor evidence
has its own source/encoding contract and does not rewrite historical expectations:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3g10_reference.py --check --overflow-check --address-check
python3 tools/reference/m3g10_snapshot_goldens.py --check
cargo test -p doom-sim armor_
python3 tools/reference/check_player_armor.py
python3 tools/seed_player_armor_fuzz.py /tmp/doom-armor-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_armor /tmp/doom-armor-seeds -- -runs=4096 -seed=461 -max_len=290 -timeout=15 -rss_limit_mb=1024
python3 tools/measure_player_armor.py
```

The example asserts every source timeline and complete ordered event frame in
both debug and release. Its `--measure workload advance|step|capture` mode times
ordinary gameplay, deliberate step capture, or advance+explicit snapshot separately.
Initialization is separately labeled100 complete setups, not100 updates. All raw
three-process timing/RSS/allocation samples and toolchain limitations belong to
[the engineering record](validation/M3G10.md). Rust lifetime/rollback/malformed
input tests are safety policy; source component and world comparisons are separate.
Leak detection is disabled in the documented ASan environment, not verified.


## Original health and armor bonuses

Bonus evidence has its own source/encoding contract and leaves historical
expectations untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3g11_reference.py --check --overflow-check --address-check
python3 tools/reference/m3g11_snapshot_goldens.py --check
cargo test -p doom-sim bonus_
python3 tools/reference/check_player_bonuses.py
python3 tools/seed_player_bonuses_fuzz.py /tmp/doom-bonus-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_bonuses /tmp/doom-bonus-seeds -- -runs=4096 -seed=471 -max_len=290 -timeout=15 -rss_limit_mb=1024
```

The example asserts every source timeline and complete ordered event frame in
debug and release; `--measure workload advance|step|capture` times ordinary
gameplay, deliberate step capture, or advance plus explicit snapshot. Measurement
summaries and any toolchain limitations belong to
[the engineering record](validation/M3G11.md). Leak detection is disabled in the
documented ASan environment, not verified.


## Original former humans and mixed rosters

Former-human evidence has its own source/encoding contract and leaves historical
expectations untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3h1_reference.py --check --overflow-check --address-check
python3 tools/reference/m3h1_snapshot_goldens.py --check
cargo test -p doom-sim monster_
python3 tools/reference/check_player_monsters.py
python3 tools/seed_player_monsters_fuzz.py /tmp/doom-monster-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_monsters /tmp/doom-monster-seeds --sanitizer address -- -runs=4096 -seed=481 -max_len=290 -timeout=15 -rss_limit_mb=1024
```

`m3h1_reference.py` keeps the declared GNU wrapping build for its trace; the strict
signed-overflow and ASan/UBSan audits retype only the original attack-angle locals
and must reproduce the identical trace. The example asserts every scenario's
source fields and complete ordered event frames in debug and release;
`--measure <workload> advance|step|capture` times one full scripted timeline.
Measurement summaries and limitations belong to
[the engineering record](validation/M3H1.md). Leak detection is disabled in the
documented ASan environment, not verified.

## Original Imps and fireballs

Imp evidence has its own source/encoding contract and leaves historical expectations
untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3h2_reference.py --check --overflow-check --address-check
python3 tools/reference/m3h2_snapshot_goldens.py --check
cargo test -p doom-sim imp_tests
python3 tools/reference/check_player_imps.py
python3 tools/seed_player_imps_fuzz.py /tmp/doom-imp-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_imps /tmp/doom-imp-seeds --sanitizer address -- -runs=4096 -seed=491 -max_len=290 -timeout=15 -rss_limit_mb=1024
```

Add `--cases` to `m3h2_reference.py` to print launches, explosions, contact draws,
claws and kills per scenario. The example asserts every scenario's source fields and
complete ordered event frames in debug and release; `--measure <workload>
advance|step|capture` times one full scripted timeline. Run measurements alone, with
no fuzzing, builds or tests at the same time. Measurement summaries and limitations
belong to [the engineering record](validation/M3H2.md). Leak detection is disabled in
the documented ASan environment, not verified.

## Original monsters at real-map scale

Map-scale evidence has its own source/encoding contract and leaves historical
expectations untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3h3_reference.py --check --overflow-check --address-check
python3 tools/reference/m3h3_snapshot_goldens.py --check
cargo test -p doom-sim player_map_scale_tests
python3 tools/reference/check_player_map_scale.py
python3 tools/seed_player_map_scale_fuzz.py /tmp/doom-map-scale-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_map_scale /tmp/doom-map-scale-seeds --sanitizer address -- -runs=4096 -seed=501 -max_len=290 -timeout=15 -rss_limit_mb=1024
```

Add `--cases` to `m3h3_reference.py` to print launches, explosions, slides, blocked
moves, near-wall damage, peak live fireballs and the largest coordinate per scenario.
The `stall_*` tests reproduce the M3h2 stalls on the historical policies and must keep
passing; the `map_scale_*` tests show the same scenarios progressing. The fuzz target
asserts the progress property: a valid slot-One command to a live player is never
rejected and every accepted tic advances the counter by one unless paused. The example
asserts every scenario's source fields and complete ordered event frames in debug and
release; `--measure <workload> advance|step|capture` times one full scripted timeline.
Run measurements alone, with no fuzzing, builds or tests at the same time. Measurement
summaries and limitations belong to [the engineering record](validation/M3H3.md). Leak
detection is disabled in the documented ASan environment, not verified.

## Original monsters with static heights

Heights evidence has its own source/encoding contract and leaves historical
expectations untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3h4_reference.py --check --overflow-check --address-check
python3 tools/reference/m3h4_snapshot_goldens.py --check
cargo test -p doom-sim player_heights_tests
python3 tools/reference/check_player_heights.py
python3 tools/seed_player_heights_fuzz.py /tmp/doom-heights-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_heights /tmp/doom-heights-seeds --sanitizer address -- -runs=4096 -seed=501 -max_len=290 -timeout=15 -rss_limit_mb=1024
```

Add `--cases` to `m3h4_reference.py` to print, per scenario, the blocked moves by
`P_TryMove` reason, vertical steps, falls, floor and ceiling contacts, landings,
sight queries, hitscans, noise floods, launches and explosions. The
`heights_source_case_*` tests replay every committed scenario; the remaining
`player_heights_tests` are Rust-only: the room builder mirrors the committed geometry
byte for byte, every out-of-scope feature is rejected at load, spawns that do not fit
their sector are rejected with unchanged state, the vertical and `momz` witnesses roll
a tic back, and the stairs and ledge soaks assert the vertical invariant after every
tic. The fuzz target adds the vertical invariant to the map-scale progress property.
The example asserts every scenario's source fields and complete ordered event frames
in debug and release; `--measure <workload> advance|step|capture` times one full
scripted timeline (`stairs`, `ledge_drop`, `corpse_ledge`, `window_sight`,
`fence_slide`, `ceiling_fireball`). Run measurements alone, with no fuzzing, builds or
tests at the same time. Measurement summaries and limitations belong to
[the engineering record](validation/M3H4.md). Leak detection is disabled in the
documented ASan environment, not verified.

## Original monsters among diagonal walls

Diagonal-wall evidence has its own source/encoding contract and leaves historical
expectations untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3h5_reference.py --check --overflow-check --address-check
python3 tools/reference/m3h5_snapshot_goldens.py --check
cargo test -p doom-sim player_diagonal_tests
python3 tools/reference/check_player_diagonal.py
python3 tools/seed_player_diagonal_fuzz.py /tmp/doom-diagonal-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_diagonal /tmp/doom-diagonal-seeds --sanitizer address -- -runs=4096 -seed=501 -max_len=290 -timeout=15 -rss_limit_mb=1024
```

Add `--cases` to `m3h5_reference.py` to print, per scenario, the slides and blocked
moves (and how many were at diagonal lines), the blocked moves by `P_TryMove` reason,
sight queries, hitscans, noise floods, launches, explosions and the captured player
positions outside the room (`player_void_captures`, non-zero only for `corner_jump`).
The `diagonal_source_case_*` tests replay every committed scenario; `corner_jump` must
fail at tic 1 with `MapScaleError::Escape` and leave the state byte-identical, so only
its spawn capture is compared and its later committed rows are never reached. The
remaining `player_diagonal_tests` are Rust-only: rejection variants are built by
patching the committed WAD lumps (a reversed wall, a shortened wall, a moved vertex,
an unlisted line, forbidden flags, specials, sky, heights), spawns whose box meets a
wall or whose center has even parity are rejected with unchanged state, the sliver
quirks of `P_PointOnLineSide`/`P_BoxOnLineSide` are asserted on a `(1,1)` wall, the
escape witness rolls a tic back, and the wedge and pillar soaks assert Invariant Ω
(center parity and the open box) after every tic for 1,200 tics on four seeds. The
fuzz target adds Invariant Ω and a controlled push operation to the heights progress
property; a valid command may then be refused only by the escape witness. The example
asserts every scenario's source fields and complete ordered event frames in debug and
release; `--measure <workload> advance|step|capture` times one full scripted timeline
(`wedge_slide`, `pillar_chase`, `corridor_pursuit`). Run measurements alone, with no
fuzzing, builds or tests at the same time. Measurement summaries and limitations belong
to [the engineering record](validation/M3H5.md). Leak detection is disabled in the
documented ASan environment, not verified.

## Original monsters at classic-map scale

Real-map-scale evidence has its own source/encoding contract and leaves historical
expectations untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3h6_reference.py --check --overflow-check --address-check
python3 tools/reference/m3h6_snapshot_goldens.py --check
cargo test -p doom-sim player_real_map_tests
python3 tools/reference/check_player_real_map.py
python3 tools/seed_player_real_map_fuzz.py /tmp/doom-real-map-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_real_map /tmp/doom-real-map-seeds --sanitizer address -- -runs=4096 -seed=501 -max_len=290 -timeout=15 -rss_limit_mb=1024
python3 tools/measure_real_map.py --check
```

Add `--cases` to `m3h6_reference.py` to print, per scenario, the room's table sizes,
slides, blocked moves by `P_TryMove` reason, sight queries and the most per tic,
hitscans, noise floods, launches, explosions, pickups, the traces and the largest
intercept count of one trace by traverser kind (the `TI` observer rows). The
`real_map_source_case_*` tests replay every committed scenario; no scenario is refused.
The remaining `player_real_map_tests` are Rust-only: every committed M3h5 room loads
under the new policy, rejection variants are built by patching the committed lumps (a
3,854-norm wall, a stretched rectangle, a 32,000-unit span, heights beyond ±512, an
overfilled 2 × 2 window, the boundary, BLOCKMAP, special, sky and short-REJECT cases),
player starts are kept as records, a hitscan fires in the 516-line town (a regression
test that fails against the pre-fix line-indexed buffer), the intercept witness is
reached on the Rust-only witness hall (`tests/reference/m3h6_witness.bin`) through both
`advance` and `step` with the state byte-identical, a push to an item is refused, the
per-sector noise flood equals the linedef scan, and the town and wide soaks assert
Invariant Ω after every tic for 1,200 tics. An opt-in test gated on `DOOM_IWAD_DIR`
(`private_iwad_real_map_counts`, `--ignored`) prints pass/fail counts of the certificate,
boundary, BLOCKMAP and window rules over every map of the configured IWADs and the
admission time of E1M1's geometry with specials and sky stripped in memory; it never
prints a name beyond the slot or a byte of the map, and `tools/iwad_counts.py` (local
only, never run by CI, a no-op without `/mnt/d/DoomIWADs`) prints the same counts from
Python; its `--spawn-rule [SLOT]`, `--spawn-rule-markdown`, `--spawn-fit [SLOT]` and
`--things-budget [SLOT]` modes (M3h9b) print the closed-box and S′ spawn-rule counts, the
spawn height-fit counts and the per-cell BLOCKMAP budget facts, counts only. The fuzz target runs six rooms (town, gallery, extent, dense, wide, witness) under
the M3h5 properties plus the intercept witness; a valid command may be refused only by
the escape or intercept witness. The example asserts every scenario's source fields and
complete ordered event frames in debug and release; `--measure <workload>
advance|step|capture` times one full scripted timeline (`town_patrol`, `town_fight`,
`gallery_shot`, `wide_sight`) and `--admit town|gallery|wide` the admission of one
committed room with parsing excluded; `measure_real_map.py` drives both with a native
`wait4` launcher (`--check` is the CI smoke). The fuzz workspace is nightly-only and CI
only format-checks it, so the campaign is a local step. Run measurements alone, with no fuzzing,
builds or tests at the same time. Measurement summaries and limitations belong to
[the engineering record](validation/M3H6.md). Leak detection is disabled in the
documented ASan environment, not verified.

## Original monsters with sky and static specials

Sky and static-specials evidence has its own source/encoding contract and leaves
historical expectations untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3h7_reference.py --check --overflow-check --address-check
python3 tools/reference/m3h7_snapshot_goldens.py --check
cargo test -p doom-sim player_specials_tests
cargo run -p doom-sim --example player_specials --locked
python3 tools/reference/check_player_specials.py
python3 tools/measure_specials.py --check
python3 tools/seed_player_specials_fuzz.py /tmp/doom-specials-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_specials /tmp/doom-specials-corpus --sanitizer address -- -runs=8653 -seed=501 -max_len=290 -timeout=15 -rss_limit_mb=1024
```

Run the fuzz campaign from `fuzz/` on a fresh copy of the seeds in
`/tmp/doom-specials-corpus`, outside the working tree, and delete the corpus afterwards.
libFuzzer counts the initial-corpus runs against `-runs`, so `-runs` must exceed the
number of seeds plus one (the empty input) for any mutation to happen: the seed tool
writes 4,556 seeds, a `-runs=4096` campaign ends after the seeds without a single
mutation, and the recorded coverage-guided campaign used `-runs=8653` (the 4,557
initial runs plus 4,096 mutations). `-runs=0` replays the seeds only.

Add `--cases` to `m3h7_reference.py` to print the per-scenario inventory as JSON lines.
The `specials_source_case_*` tests replay every committed scenario of the `pools` and
`sky` rooms against the C rows, including every sector light and special (`SX`) and the
shared thinker order with every light thinker's fields (`TK`), the independent DSIM34
captures, the full-byte fixtures and the ordered events; no scenario is refused. The
remaining `player_specials_tests` are Rust-only: admission of every committed room and
rejection of every other special, a two-sided scroller, the 65th scroller, lights
outside the range and an out-of-range height; the `F_SKY1` lookup-name rule; the load
order of `spawn_level_specials`; `BT_USE` still refused; refused tics that keep the
lights, thinkers and an unfound secret; a secret found on landing, not while the body
is above the sector floor; the policy identity, the malformed-field rejections of
`decode_specials` and its cross-checks against the encoded map; the sky-shot boundary at exactly the front ceiling; the
sector-thinker slot and visit budget; and the `pools` and `sky` soaks, which assert
Invariant Ω, the vertical invariant, every light within its thinker's bounds, the shared
order and `secretcount ≤ totalsecret` after every tic for 1,200 tics on four seeds. The
example replays every scenario, asserts the committed headline counts of the C rows in
debug and release and prints one combined digest; `--measure <workload>
advance|step|capture` times one full scripted timeline (`pools_lights_soak`,
`pools_light_and_fireball`, `sky_hack_wall`, `sky_window_order`) and `--admit pools|sky`
the admission, THING and body spawns and `spawn_level_specials` of one committed room
with parsing excluded; `measure_specials.py` drives both with a native `wait4` launcher
(`--check` is the CI smoke). The fuzz target admits one of the committed scenario spawns,
runs `spawn_level_specials` and applies bounded operations under the M3h6 properties
plus the light envelopes, the shared order and the secret bound. The fuzz workspace is
nightly-only and CI only format-checks it, so the campaign is a local step. Run
measurements alone, with no fuzzing, builds or tests at the same time. Observations
belong to [the engineering record](validation/M3H7.md). Leak detection is disabled in
the documented ASan environment (`detect_leaks=0`), not verified.

## Original doors, lifts, floors, switches and the exit (C evidence)

The Stage B C evidence of the
[M3h8 investigation](M3H8_REFERENCE_INVESTIGATION.md#11-evidence-plan) has its own
source contract and leaves every historical expectation untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3h8_reference.py --check --overflow-check --address-check
python3 tools/reference/m3h8_snapshot_goldens.py --check
python3 tools/reference/check_player_movers.py
cargo test -p doom-sim player_movers
cargo run -p doom-sim --example player_movers --locked
python3 tools/measure_movers.py --check
python3 tools/seed_player_movers_fuzz.py /tmp/doom-movers-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_movers /tmp/doom-movers-corpus --sanitizer address -- -runs=0 -max_len=290 -timeout=15 -rss_limit_mb=1024
# Bounded campaigns, each on a fresh copy of the seeds:
cargo +nightly fuzz run player_movers /tmp/doom-movers-corpus --sanitizer none -- -seed=501 -max_total_time=1800 -max_len=290 -timeout=15 -rss_limit_mb=1024
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_movers /tmp/doom-movers-corpus --sanitizer address -- -seed=502 -max_total_time=1800 -max_len=290 -timeout=15 -rss_limit_mb=1024
```

The generator compiles the genuine door, plat, floor, switch, use, crossing and
`P_ChangeSector` bodies over the `doorway`, `hall`, `switchyard`, `lift` (three lift
ceilings), `floors` and `dense` rooms and checks `tests/reference/m3h8_trace.dref`,
`m3h8_cases.csv`, the `m3h8_geometry_*.bin` lumps, `m3h8_maps.rs`, the Rust-only
nine-line lane `m3h8_dense9.bin` and `m3h8_contract.json`. `--overflow-check` and
`--address-check` rebuild the harness under the sanitizers and assert that each raw trace
equals the strict one byte for byte; `spechit` keeps its source size of eight, so a ninth
write would stop the sanitizer build. Add `--cases` to print the per-scenario inventory
as JSON lines. `m3h8_snapshot_goldens.py` is the independent
[DSIM35](STATE_FORMAT.md#dsim-v35--original-doors-lifts-floors-switches-and-the-exit)
projection of those rows (never of Rust output): it checks
`tests/reference/m3h8_snapshots.csv`, the ordered events `m3h8_events.dref`, the
full-byte `m3h8_state_*.bin` fixtures and `m3h8_snapshot_contract.json`, and asserts that
the event stream without its tag-26 records equals the DSIM34 projection of the same
rows. `check_player_movers.py` runs the full Rust replay (the library test
`movers_replay::full::full_replay_matches_the_c_rows_and_the_independent_dsim35_goldens`)
in debug and release and asserts that both pass and print the same summary: all 33
scenarios through their last op, every C row the replay reads (heights, line specials,
use latch, exit and player state, crossings, uses, blocked moves, the thinker list
with every mover record, the button slots), every PBE1 event frame, and at all 1,559
captures the independent digest, size and field count, the 13 fixtures byte for byte
and the `Diagnostic::decode_movers` round trip. It is a library test because the
replay's declared damage input uses the crate-internal checked transaction. The
replay applies the declared between-tic inputs before each op's tic: the damage, the
attack entry and the see-state entry (`chase_at`: target assignment plus
`S_POSS_RUN1`, whose `A_Chase` runs at once). It also runs
the asserted example in debug and release and asserts equal output
(`--example-only` runs the example alone, the CI step on the hosts that skip the
replay). The example replays, through the public API alone, the 24 scenarios whose
declared inputs are public (all but the seven with the controlled 200 health and the
two whose controlled damage kills, both crate-internal inputs of the replay),
comparing every op's sector heights and mover work (`T_MovePlane` steps,
`P_ChangeSector` calls, `PIT_ChangeSector` clips, crossings, uses, blocked moves,
exits) with the C rows and the final runtime line specials, and asserts the C rows'
headline counts; `--measure doorway_patrol|lift_cycle|switch_cycle
advance|step|capture` times a 1,200-tic patrol with `BT_USE` in the doorway room with
monsters, the committed `lift_last_step_short` timeline or the committed
`switch_sr_door` timeline and prints the per-tic `P_ChangeSector` work recorded from an
unmeasured run, and `--admit doorway|switchyard|lift|floors|dense|pools` times the
movers admission (with the fixpoint and the window rule; for the M3h7 `pools` room the
specials-policy admission too); `measure_movers.py` drives both with the native
`wait4` launcher (`--check` is the CI smoke).

The `player_movers` tests (`player_movers_tests`, `player_movers_soak_tests`) are
Rust-only: rejection, with the map untouched and the unpatched room admitted, of
every deferred line family and every other special value on either side, the
two-sided scroller (N1 iii), a one-sided DR line, a shared family, an inverting door,
a reachable height of 513, 17 reusable switches, 9 special lines in a window, a sky
mover, a missing switch texture and a mover sector without a defined blockbox (rule
10, through `mover_bounds`) on the committed rooms; the turbo destination folded from
−500 (`pit` room, admission and runtime); the between-tic see-state entries that use a
DR line from a blocked move and cross a WR 88 line; and of a mover without a
two-sided neighbour, a self-referencing door line, 31 plat sectors and 65 scrollers
on synthetic strip rooms (`heights_room`); a tic refused, by a test-only fault site,
after a plane step's write and `P_ChangeSector` (doors, lifts, floors) or after a
use's line action, and at the `SpecialCross` witness, with every byte, the RNG, the
sector, line-special and button tables, the thinkers and the events unchanged and the
same tic then running as unfaulted; the exit and reborn refusals with any command; and
1,200-tic soaks with four seeds on the doorway, switchyard, lift and floors rooms with
`BT_USE`, asserting V′ and the item witness after every tic, `step == advance` every
10th tic and the DSIM35 `decode_movers` round trip every 50th. The fuzz target admits
one of 27 distinct scenario starts of the first 30 committed scenarios (the three
added by the independent review are not fuzz starts) and applies bounded operations
(commands with `BT_USE`, damage, retargeting, removal, pause, pushes) under the M3h6
properties plus V′ (from the public `mover_bounds`), the relaxed item witness, the
reach of every sector height, the transaction (a refused operation keeps every byte),
the exit and reborn refusals and the DSIM35 round trip. Run the fuzz commands from
`fuzz/` on a fresh copy of the seeds outside the working tree and delete the corpus
afterwards; `-runs=0` replays the seeds only, and as for the specials target a
`-runs` bound must exceed the seed count plus one (the seed tool writes 4,428 seeds)
before any mutation happens, so the campaigns above are bounded by
`-max_total_time` instead (libFuzzer counts the initial-corpus runs inside it too).
The campaign without a sanitizer explores faster; the AddressSanitizer campaign is the
sanitizer evidence. The fuzz workspace is nightly-only and CI only format-checks it, so
the campaigns are a local step. Run measurements alone, with no fuzzing, builds or tests
at the same time. Observations belong to [the engineering record](validation/M3H8.md).
Leak detection is disabled in the documented ASan environment (`detect_leaks=0`), not
verified.

## Original two-sided scrollers, static decorations and the spawn filter (C evidence)

The M3h9a C evidence of the
[M3h9 investigation](M3H9_REFERENCE_INVESTIGATION.md#7-evidence-plan-c-first-genuine-bodies)
has its own source contract and leaves every historical expectation untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3h9_reference.py --check --overflow-check --address-check
python3 tools/reference/m3h9_snapshot_goldens.py --check
python3 tools/reference/check_player_e1m1.py
cargo test -p doom-sim e1m1
cargo run -p doom-sim --example player_e1m1 --locked
python3 tools/measure_e1m1.py --check
python3 tools/seed_player_e1m1_fuzz.py /tmp/doom-e1m1-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_e1m1 /tmp/doom-e1m1-corpus --sanitizer address -- -runs=0 -max_len=290 -timeout=15 -rss_limit_mb=1024
# Bounded campaigns, each on a fresh copy of the seeds:
cargo +nightly fuzz run player_e1m1 /tmp/doom-e1m1-corpus --sanitizer none -- -seed=601 -max_total_time=1800 -max_len=290 -timeout=15 -rss_limit_mb=1024
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_e1m1 /tmp/doom-e1m1-corpus --sanitizer address -- -seed=602 -max_total_time=1800 -max_len=290 -timeout=15 -rss_limit_mb=1024
```

The generator reuses every compiled M3h8 body over the `scrollhall`, `scrollblock`,
`dense8`, `gallery`, `liftroom`, `census` and `doorstep` rooms and checks
`tests/reference/m3h9_trace.dref`, `m3h9_cases.csv`, the `m3h9_geometry_*.bin` lumps,
`m3h9_maps.rs`, the Rust-only nine-scroller lane `m3h9_dense9.bin` and
`m3h9_contract.json`; the sanitizer options and `--cases` work as for M3h8.
`m3h9_snapshot_goldens.py` is the independent
[DSIM36](STATE_FORMAT.md#dsim-v36--original-two-sided-scrollers-and-static-decorations)
projection of those rows (never of Rust output): it checks
`tests/reference/m3h9_snapshots.csv`, the ordered events `m3h9_events.dref`, the 15
full-byte `m3h9_state_*.bin` fixtures and `m3h9_snapshot_contract.json`.
`check_player_e1m1.py` runs the full Rust replay (the library tests in
`e1m1_replay`) in debug and release and asserts that both pass and print the same
summary: all 20 medium-skill scenarios through their last op, every C row the replay
reads (heights, line specials, use latch, exit and player state, crossings, uses, use
intercepts, blocked moves, the thinker list), every PBE1 event frame, and at every
capture the independent digest, size and field count, the 15 fixtures byte for byte
and the `Diagnostic::decode_e1m1` round trip (with `decode_movers` rejecting the
bytes). The comparison bodies are the M3h8 replay's. The two C-only census rows at
easy and hard skill are not replayed. A second, Rust-only test rebuilds the
`monster_hitscan_through_pillar` alcove in memory with its walls on the Zombieman's
closed box and asserts that the diagonal spawn rule still refuses it with the state
unchanged (`P_SpawnMapThing` checks no position; the committed alcove is wide enough).
Two more replay the review scenarios: the fireball's crossing record at the blocking
scroller, and the pillar whose floor stays above its ceiling while the door is shut on
it, with a shut-door capture decoding under `decode_e1m1`.
The example `player_e1m1` runs in the
synthetic M3h9 rooms, not the real E1M1. Through the public API alone it replays the
scenarios whose declared inputs are public (all but those with the controlled
200 health), comparing every op's sector heights and work with the C rows, every
capture digest with the independent index and the final runtime line specials, and
asserts the C rows' headline counts; `--example-only` runs it alone (the CI step on
the hosts that skip the replay). `--measure
gallery_patrol|liftroom_ride|dense8_cross|scrollhall_fight advance|step|capture` times
a 1,200-tic patrol in the gallery or the scroller hall, or the committed
`decorations_ride_lift` or `dense_spechit_8` timeline, and `measure_e1m1.py` drives it
with the native `wait4` launcher (`--check` is the CI smoke and asserts the same final
digest in every mode). Run measurements alone, with no fuzzing, builds or tests at the
same time.

The `player_e1m1_soak_tests` are Rust-only: every info.c type with a DoomEd number
that wire 20 does not admit (the barrel, the hanging corpses, the candle and the
other unlisted decorations among them) refused as `UnsupportedType` after the ten
gallery decorations; a bit-16 or skill-filtered record (a decoration or a barrel)
filtered before its type is checked, with no draw; the fixed medium single-player
options; 64 scrollers admitted and 65 refused; every crusher line on a two-sided or
one-sided line; each decoration type refused under wires 18 and 19; a spawn batch
with eight monsters, sixteen pickups and 32 decorations spawning all 56 while a 33rd
decoration, a barrel or hanging corpse after the decorations and a ninth monster each
leave the capture bytes, the RNG, the actor order, the events and the statistics
unchanged; every nonzero push, vertical momentum, target and removal of a decoration
refused, a direct spawn of each decoration type refused, a death request and damage
with an inflictor refused and damage with neither ignored with no gameplay change, and
nonzero vertical momentum on a census item refused; 2,400-tic
soaks with four seeds on the five gallery and the two liftroom rooms asserting V′,
every decoration's type, state, tics, health, flags, radius, height and `z == floorz`
after every tic, `step == advance` every 10th tic and the DSIM36 `decode_e1m1` round
trip every 100th; and the M3h9 note §5.3 budget formulas. The fuzz target
`player_e1m1` starts from the 20 replayed scenario starts and applies bounded
operations (commands with `BT_USE` and `BT_ATTACK`, damage, retargeting, removal,
pause, pushes, vertical momentum, each also aimed at a decoration, whose target,
removal, nonzero push and nonzero vertical momentum must be refused, and direct spawns
of a decoration type, which must be `DirectSpawn`) under the movers target's properties (`step == advance`, a refused
operation keeping the snapshot, V′, Ω, the item witness, the mover bounds, progress)
plus the decorations' static fields, `z == floorz` and count, the DSIM36 round trip
with `decode_movers` rejecting the bytes, and mutated decodes of two pinned DSIM36
fixtures; `seed_player_e1m1_fuzz.py` writes its byte seeds without reading Rust
output. Run the fuzz commands from `fuzz/` on a fresh copy of the seeds outside the
working tree and delete the corpus afterwards. `-runs=0` replays the seeds only; the seed
tool writes 5,360 seeds, so, as for the movers target, the campaigns are bounded by
`-max_total_time` rather than `-runs`. The campaign without a sanitizer explores faster;
the AddressSanitizer campaign is the sanitizer evidence. The fuzz workspace is
nightly-only and CI only format-checks it, so the campaigns are a local step. Run
measurements alone, with no fuzzing, builds or tests at the same time. Observations
belong to [the engineering record](validation/M3H9.md). Leak detection is disabled in
the documented ASan environment (`detect_leaks=0`), not verified.

## Original exploding barrels, pickup scale and every thing angle (C evidence)

The M3h9b C evidence of the
[M3h9b investigation](M3H9B_REFERENCE_INVESTIGATION.md#7-evidence-plan-c-first-genuine-bodies)
has its own source contract and leaves every historical expectation untouched:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3h9b_reference.py --check --overflow-check --address-check
python3 tools/reference/m3h9b_snapshot_goldens.py --check
python3 tools/reference/check_player_barrels.py
cargo test -p doom-sim barrel
cargo run -p doom-sim --example player_barrels --locked
python3 tools/measure_barrels.py --check
python3 tools/seed_player_barrels_fuzz.py /tmp/doom-barrels-seeds      # 3,172 seeds, max_len 362
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_barrels /tmp/doom-barrels-corpus --sanitizer address -- -runs=0 -max_len=362 -timeout=15 -rss_limit_mb=1024
# Bounded campaigns, each on a fresh copy of the seeds:
cargo +nightly fuzz run player_barrels /tmp/doom-barrels-corpus --sanitizer none -- -seed=701 -max_total_time=1800 -max_len=362 -timeout=15 -rss_limit_mb=1024
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_barrels /tmp/doom-barrels-corpus --sanitizer address -- -seed=702 -max_total_time=1800 -max_len=362 -timeout=15 -rss_limit_mb=1024
```

The generator layers on the M3h9 pack and adds three genuine bodies, `P_RadiusAttack`,
`PIT_RadiusAttack` (p_map.c) and `A_Explode` (p_enemy.c, replacing the abort stub), with
observer rows for every radius attack (`RA`) and every candidate of its block walk (`RC`,
in walk order with the outcome code). It runs 26 scenarios over the ten rooms `range`,
`cluster`, `player_range`, `monsters`, `walls`, `door`, `fireball`, `dense64`, `angles` and
`embedded`, and checks `tests/reference/m3h9b_trace.dref`, `m3h9b_cases.csv`, the
`m3h9b_geometry_*.bin` lumps, `m3h9b_maps.rs`, the two Rust-only lanes
`m3h9b_embedded_barrel.bin` and `m3h9b_intercepts129.bin` and `m3h9b_contract.json`. The
strict build compiles the genuine text under `-fwrapv`; the two sanitizer builds apply
two declared, cited adapters (the unsigned rewrites of the angle product at p_mobj.c:794,
which the sanitizers otherwise report, and of the radius shift at p_map.c:1221, which
under `-std=gnu89` they do not), and the three raw traces must be byte-identical.
`m3h9b_snapshot_goldens.py` is the independent
[DSIM37](STATE_FORMAT.md#dsim-v37--original-exploding-barrels-pickup-scale-and-every-thing-angle)
projection of those rows (never of Rust output): `tests/reference/m3h9b_snapshots.csv`,
the ordered events `m3h9b_events.dref`, the 16 full-byte `m3h9b_state_*.bin` fixtures and
`m3h9b_snapshot_contract.json`, with the assertion that every capture equals the DSIM36
projection of the same rows except the version, profile, context and `store.max_slots`.
`check_player_barrels.py` runs the full Rust replay (the library tests in
`barrels_replay`) in debug and release and asserts that both pass and print the same
summary: all 26 scenarios through their last op, every C row the M3h9 replay reads plus
the `RA` rows, every `RC` candidate in walk order with its outcome and damage, the
`A_Explode` action rows and the contract's death chains, every PBE1 event frame, and at
every capture the independent digest, size and field count, the 16 fixtures byte for byte
and the `Diagnostic::decode_barrels` round trip (with `decode_e1m1` rejecting the bytes);
the two Rust-only lanes assert the S′ refusal of an embedded barrel at load and the
intercept witness on the 129-intercept trace. The example `player_barrels` replays the
scenarios whose inputs are public through the public API alone, asserts the C headline
counts (27 explosions, the candidates by outcome, 28 barrel deaths, 1 crunched) and the
capture digests against the independent index; `--example-only` runs it alone on the hosts
that skip the replay. `--measure cluster_chain|dense64_fight|range_patrol|liftroom_ride|dense64_lift
advance|step|capture` times the chain-reaction timeline, a 1,200-tic fight in the dense
room, a patrol without barrel damage, the M3h9 lift timeline under the new policy and the
in-memory 121-THING lift room (the `P_ChangeSector` measurement at the 129 linked-thing
bound), and `measure_barrels.py` drives them (`--check` is the CI smoke and asserts the same
final digest in every mode). Run measurements alone.

The Rust-only tests (`player_barrels_tests`, `player_barrel_combat_tests`) cover the policy
plumbing (every M3h9 room under wires 20 and 21 with captures equal except the four
identity fields; `decode_barrels` reading 37 only and every older reader rejecting it), the
bounds (64 pickups and the 65th `WorkLimit`, 16 barrels and the 17th `BarrelLimit`, the
§5.3 formulas as asserted constants), the angles (17 values against the wrapped product;
wire 20 still `Angle`), the spawn rule S′ (an Imp embedded 4 units admitted, a deeper one
refused, an embedded player and barrel refused, edge contact and overlapping statics
admitted, even parity refused, wire 20 unchanged), every barrel path of the note's §2 with
expected values derived from the C formulas, the stale-source witness, the API refusals,
rollback of a 17th barrel, 2,400-tic soaks with four seeds and the DSIM37 round trip, and
26 forged-field negatives of `decode_barrels`. The fuzz target `player_barrels` starts from
the 26 scenario starts (an input is 2 header bytes and up to 40 nine-byte records, at most
1,024 tics) and applies bounded operations: commands with `BT_USE` and `BT_ATTACK` through
`advance` and `step`, damage with and without an inflictor aimed at the body, a monster, a
decoration, a pickup and a barrel, the refused barrel calls (target, removal, state entry,
death request, direct spawn), pushes and vertical momentum on a barrel, pause and runs of
up to 256 tics, under the properties `step == advance` on a fresh clone every fourth
operation, a refused operation leaving the snapshot and RNG unchanged, V′ and Ω (Ω's box
clause once an actor has left its spawn position, since S′ admits an embedded Imp), the
item witness, every live barrel passing the barrel witness with at most 16 barrels, a
dying barrel solid, non-shootable and 10.5 units high or gibbed, the DSIM37 round trip
after every operation and every 32nd tic of a run (with `decode_e1m1` rejecting it),
progress, and every refused tic being a declared witness (`Intercepts`, `WorkLimit`,
`SpecialCross`, `Vertical`, `Escape`, the exit, the reborn request, `Invariant` for a stale
source) and never a panic.

### Docs-only CI fast path

The workflow starts with a `Detect changes` job that classifies a push as "documentation
only since a green head": for a pull-request `synchronize` event or a push to `master` it
takes the push's previous head and new head from the event, requires that the previous
head has a successful run of this workflow (one `gh api` query of the workflow's runs,
which is why the job has `actions: read`), lists the paths changed between the two heads
with one `git diff --name-only --no-renames`, and sets `docs_only` when every path ends in
`.md` and none lies under `docs/validation/`. The four required jobs (`Stable /
ubuntu-latest`, `Stable / windows-latest`, `Stable / macos-latest`, `Minimum Rust /
1.85.0`) `need` it and read it into `DOCS_ONLY`; on a documentation-only push every
toolchain install, build, test, evidence, example and measurement step is skipped by a
step-level `if`, while the checkout, the documentation link check and a one-line report
always run, so every required check still runs and reports success without building or
testing. A newly opened or reopened pull request, a base change, a code or mixed change,
a push whose previous head cannot be found or whose run failed or was cancelled, and a
manual run keep every step. A change under `docs/validation/` is never documentation-only,
because the historical evidence records there are guarded by the migration check
(`evidence_storage.py verify-git`), which must keep running on them. If the detect-changes
job itself fails, the required jobs still run in full (`!cancelled()`), and renames are not
detected, so a file renamed out of a code suffix cannot pass as documentation.
`paths-ignore` is not used, because a skipped required check never reports. Job names, the
`macos-26-intel` runner and the timeouts are unchanged. The proof (one docs-only commit and
one code commit of the M3h9b pull request) is in the [evidence record](validation/M3H9B.md#ci).

## M3h9c whole-engine runner and the real-E1M1 comparison

The [M3h9c investigation](M3H9C_REFERENCE_INVESTIGATION.md) compares the Rust simulation
with the genuine linuxdoom-1.10 built whole (not extracted bodies) on the real E1M1, tic
for tic. **IWAD rules:** the real comparison is local only. It needs `DOOM_IWAD_DIR` naming a
directory with `DOOM.WAD`, is never run by CI, and without the variable the wrapper prints
one skip line and exits 0. The IWAD is read in place and never copied. Dumps, captures,
logs and binaries stay under a `--work` directory outside the repository (delete it once the
results are recorded). Only counts, yes/no results, RNG indices, times and digests may be
printed or committed; coordinates, angles, lump bytes and any other map-derived value stay
local (`--values` below is local-only for that reason). Committed inputs are the text
scripts and their demo lumps, which hold no IWAD data. Details of the C side are in the
[reference tools README](../tools/reference/README.md#m3h9c-whole-engine-runner).

```sh
# the genuine runner: three variants (strict, overflow-checked, address-checked)
python3 tools/reference/m3h9c_build.py --variant strict|overflow|address|all [--out DIR] [--cc CC]
python3 tools/reference/m3h9c_build.py --check          # regenerate the contract in memory and compare
python3 tools/reference/m3h9c_build.py --contract tests/reference/m3h9c_runner_contract.json

# demo scripts (text -> version-110 .lmp)
python3 tools/e1m1_scripts.py SCRIPT.txt [-o OUT.lmp]
python3 tools/e1m1_scripts.py --check                   # committed .lmp equals regenerated bytes

# the local real-E1M1 run (genuine engine, three variants, byte-identical dumps)
DOOM_IWAD_DIR=/path tools/local/e1m1_reference.py [--work DIR] [--script NAME]... [--selftest TIC]

# the Rust side: one demo, one level
cargo run -p doom-sim --release --example player_level_run -- --wad WAD --map E1M1 --demo LMP --out DIR [--captures]
cargo test -p doom-sim player_level                      # Rust-only wire-22 tests
DOOM_IWAD_DIR=/path cargo test -p doom-sim --test private_level -- --ignored --nocapture

# the independent projection and the per-tic comparator
python3 tools/reference/level_snapshot.py DUMP_DIR --wad WAD [--map E1M1] [--demo LMP] [--out DIR]
python3 tools/reference/level_compare.py DUMP_DIR RUST_DIR --wad WAD [--map E1M1] [--demo LMP] [--values] [--json OUT]
python3 tools/reference/level_compare.py DUMP_DIR --self-check --wad WAD [--map E1M1] [--demo LMP]
python3 tools/reference/m3h9c_dump.py diff DIR_A DIR_B [--values]
```

`--selftest TIC` makes the wrapper plant an extra `P_Random` draw and require that the first
difference is reported at that tic. `level_compare.py` reports the first differing tic and
field path, ignores the `map.*` and `events.*` groups (compared separately or Rust's own
records), writes counts-only JSON with `--json`, and `--self-check` needs no Rust captures.
`--values` prints values and is local-only. The ASan runs set `ASAN_OPTIONS=detect_leaks=0`;
that is disabled leak detection, not LeakSanitizer coverage.

The ignored `private_iwad_e1m1_level_policy` test (and the older opt-in
`private_iwad_real_map_counts`) read `DOOM_IWAD_DIR` and print counts and digests only.

Script details: a script may name its output with a `# output: NAME.lmp` comment line (same
directory), and `seed S chaos N nouse` draws the same stream with USE removed from every
buttons entry.

CI path (no IWAD): a synthetic game WAD (`tools/reference/m3h9c_wad.py`,
`tests/reference/m3h9c_synthetic.wad`), synthetic scripts (`tests/reference/m3h9c_scripts/`)
and the commands below. The `stable` job of `.github/workflows/rust.yml` runs, in order:
`m3h9c_build.py --check`, `m3h9c_wad.py --check` and `e1m1_scripts.py --check` (every OS);
`m3h9c_reference.py --check --keep-dumps` and `m3h9c_snapshot_goldens.py --dumps DIR --check`
(Linux only); and the Rust whole-level run of the six synthetic scripts followed by
`check_player_level.py` (every OS). A script whose golden ends `reborn` (SYNDIE: the dead
player presses USE) must leave Rust's `refused.txt` naming the next tic with
`PlayerReborn`. All are skipped for documentation-only pushes.

```sh
python3 tools/reference/m3h9c_wad.py [--out FILE] [--check]  # synthetic WAD (default tests/reference/m3h9c_synthetic.wad); --check compares the committed file
python3 tools/reference/m3h9c_reference.py [--work DIR] [--check] [--keep-dumps] [--cc CC] [--selftest TIC]
#   three-variant run of every tests/reference/m3h9c_scripts/*.lmp; --check compares
#   tests/reference/m3h9c_synthetic_results.json; address variant uses detect_leaks=0;
#   --selftest TIC plants an extra draw and a changed ticcmd on SYNIDLE (strict build)
#   and requires both first seen in the dump of tic TIC
python3 tools/reference/m3h9c_snapshot_goldens.py --dumps DIR/runs [--check]   # DSIM38 goldens and fixtures from kept dumps
cargo run -p doom-sim --release --example player_level_run -- --wad tests/reference/m3h9c_synthetic.wad --map E1M1 --demo tests/reference/m3h9c_scripts/SYNIDLE.lmp --out OUT/SYNIDLE --captures
python3 tools/reference/check_player_level.py OUT      # Rust runs of the six scripts against the goldens

# fuzzing (the 49th target); detect_leaks=0 means LeakSanitizer is disabled, not passed
python3 tools/seed_player_level_fuzz.py /tmp/doom-level-seeds      # 1,508 seeds, max_len 1404
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_level /tmp/doom-level-corpus --sanitizer address -- -runs=0 -max_len=1404 -timeout=15 -rss_limit_mb=1024
```

Observations of the matched runs belong to the [M3h9c evidence record](validation/M3H9C.md), not to this page.

## M4a synthetic mechanism WADs and the Episode 1 comparison

The M4a [reference investigation](M4A_REFERENCE_INVESTIGATION.md) extends the M3h9c
whole-engine runner to nine small synthetic mechanism WADs (doors, floors, platforms, stairs,
exits, weapons, items, a Spectre fight and a scale map) and compares the Rust simulation, wire
23 (`PlayerMonstersEpisode1V1`, DSIM39), with the genuine engine tic for tic on 30 scripts.
Everything on this page is synthetic: no IWAD is read, and every command below runs on a
clean checkout. The tools are described in the
[reference tools README](../tools/reference/README.md#m4a-synthetic-mechanism-wads-and-the-episode-1-comparison).
Dumps, captures and binaries go under a `--work` directory outside the repository.

```sh
# generators and invariants (no compiler)
python3 tools/reference/m4a_wad.py [--out DIR] [--check]   # 9 WADs in tests/reference/m4a_wads/; --check compares the committed bytes, then runs the verifier
python3 tools/reference/m4a_check_wad.py --check           # independent verifier of every manifest WAD
python3 tools/reference/m4a_check_wad.py WAD [--name NAME] [--dump-counts]
python3 tools/e1m1_scripts.py --check                      # also checks tests/reference/m4a_scripts (.txt -> .lmp)

# runner contracts and newline regression (no compiler; every OS in CI)
python3 tools/reference/m4a_build.py --check               # M4a driver-fork contract (m4a_driver.c), including its columns
python3 tools/reference/test_runner_newlines.py            # both frozen contracts from LF and CRLF source copies
# The contract hashes cover LF-normalised Latin-1 text, so they do not see a line-ending-only change
# (lone CR is untested); Git object identity is the byte-level guard for the sources.

# genuine runner execution (C; Linux only in CI)
python3 tools/reference/m4a_reference.py [--work DIR] [--check] [--keep-dumps] [--cc CC] [--only WAD] [--selftest TIC]
python3 tools/reference/m4a_snapshot_goldens.py [--dumps DIR/runs | --work DIR] [--check]

# the Rust side
cargo build -p doom-sim --release --locked --example player_level_run
python3 tools/reference/check_player_episode1.py --binary target/release/examples/player_level_run --example-args "--policy episode1" [--out DIR] [--require]
python3 tools/reference/check_player_episode1.py --rust-root DIR   # captures already written to DIR/<SCRIPT>/

# the independent projection and comparator, profile episode1 (DSIM39)
python3 tools/reference/level_snapshot.py DUMP_DIR --wad WAD [--map E1M1] [--demo LMP] --profile episode1 [--out DIR]
python3 tools/reference/level_compare.py DUMP_DIR RUST_DIR --wad WAD [--map E1M1] [--demo LMP] --profile episode1 [--values] [--json OUT]

# fuzzing (the 50th target); detect_leaks=0 means LeakSanitizer is disabled, not passed
python3 tools/seed_player_episode1_fuzz.py /tmp/doom-episode1-seeds      # 459 seeds, max_len 2404
# -timeout=120: a 600-tic input on the scale WAD (160 actors) takes 2-4 s plain and 26-33 s under ASan
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_episode1 /tmp/doom-episode1-corpus --sanitizer address -- -runs=0 -max_len=2404 -timeout=120 -rss_limit_mb=1024

# opt-in ILP32 variant (needs a 32-bit toolchain; otherwise one skip line, exit 0)
python3 tools/reference/m4a_build.py --variant strict32 [--out DIR]   # or m3h9c_build.py for the M3h9c driver
DOOM_IWAD_DIR=/path tools/local/e1m1_reference.py --strict32   # local only; see the M3h9c section
```

What each asserts:

- `m4a_wad.py --check` regenerates the nine WADs and requires the committed bytes, then runs
  `m4a_check_wad.py`, which imports nothing from the repository, re-parses every lump and
  re-derives the mechanism layouts (stair chains, donut sectors, plat flats, floor neighbours,
  switch textures, spawn counts on medium single player, thing heights, no teleport landing).
  `e1m1_scripts.py --check` requires every committed `.lmp` of `m3h9c_scripts`, `e1m1_scripts`
  and `m4a_scripts` to equal its regenerated bytes.
- `m4a_reference.py` is manifest-driven (`tests/reference/m4a_manifest.json`). It builds the
  strict, overflow-checked and address-checked runners, runs every script on its WAD in each,
  and requires byte-identical dump sets, the manifest's ending (`marker`, `exit` or `reborn`
  and its tic), the `secretexit` flag and every `must_show` event at its declared tic.
  `--check` compares the counts, digests and event tics with
  `tests/reference/m4a_synthetic_results.json` (the informational `timing` block is not
  compared); without it the file is rewritten. `--only WAD` runs one WAD and neither writes nor
  checks the file. The address variant runs with `ASAN_OPTIONS=detect_leaks=0`, which is
  disabled leak detection, not LeakSanitizer coverage. `--selftest TIC` plants an extra
  `P_Random` draw and a changed ticcmd in the manifest's self-test script and requires each
  to be first reported at that tic.
- `m4a_snapshot_goldens.py --check` projects the strict dumps with the independent encoder
  (policy 23, DSIM39) and requires `tests/reference/m4a_synthetic_goldens.json` (per-tic
  digests, endings, projection stops) and the 18 `tests/reference/m4a_episode1_*.bin`
  fixtures to equal the regenerated output. Without `--dumps` it runs the reference itself.
- `check_player_episode1.py` runs the Rust example on every script and compares each stripped
  capture (no `map.*`, no `events.*`) with the golden digest, then the fixtures field by field.
  The manifest's `rust` expectation decides the range and the ending: `match`, a
  `PlayerReborn` refusal, a witness refusal such as `PlatBlocked`, or `tbd` (informational; no script uses it now, so
  `--require` gates all 30 scripts and 18 fixtures).
  Without wire 23 in the example it prints "SKIP: example not available (no wire 23)" and
  exits 0; `--require` makes that a failure.
- Driver columns: the M4a runner is built from `tools/reference/m4a_driver.c`, the M4a fork of
  the merged `m3h9c_driver.c` (which is unchanged). In the fork the `H` row ends with
  `secretexit` (`g_game.c:999`); `R` rows carry the removed mover's kind and sector; the `P`
  message column holds 38..40 for the locked-door strings and 41 for the turbo message. `m4a_build.py --check` regenerates
  `tests/reference/m4a_runner_contract.json`, whose hash covers the fork, so it fails on any
  change to them; `m3h9c_build.py --check` still holds the merged M3h9c contract.

### The real Episode 1 maps (local only)

The M4a real-map comparison reads the commercial IWAD, so the rules of the
[M3h9c section](#m3h9c-whole-engine-runner-and-the-real-e1m1-comparison) apply unchanged: it needs
`DOOM_IWAD_DIR` naming a directory with `DOOM.WAD`, is never run by CI, prints one skip line and exits 0
without the variable, and reads the IWAD in place. Dumps, captures, logs and binaries live under a
`--work` directory outside the repository, and only counts, yes/no results, tic numbers, RNG indices,
times and digests may be printed or committed. The committed inputs are the text scripts and demo lumps
of `tests/reference/e1m{2,3,4,6,7}_scripts/` (and E1M1's); they hold no IWAD data.

```sh
# the genuine engine on the real maps: strict, overflow-checked and address-checked builds (plus -m32 with --strict32)
DOOM_IWAD_DIR=/path tools/local/episode1_reference.py [--work DIR] [--map E1Mn]... [--script NAME]... [--strict32] [--jobs 1|2] [--keep-dumps] [--lmp PATH]...

# the Rust side, one demo of one map (wire 23), then the per-tic comparison
DOOM_IWAD_DIR=/path cargo run -p doom-sim --release --example player_level_run -- --wad "$DOOM_IWAD_DIR/DOOM.WAD" --map E1M2 --demo tests/reference/e1m2_scripts/M2IDLE.lmp --out RUST_DIR --captures --policy episode1
python3 tools/reference/level_compare.py WORK/runs/strict_E1M2_M2IDLE RUST_DIR --wad "$DOOM_IWAD_DIR/DOOM.WAD" --map E1M2 --demo tests/reference/e1m2_scripts/M2IDLE.lmp --profile episode1 [--json OUT]
```

- `--map E1Mn` (repeatable; E1M1, E1M2, E1M3, E1M4, E1M6 or E1M7) selects maps; by default every map
  with a `tests/reference/e1mN_scripts/` directory runs. `--script NAME` narrows to demo-lump base
  names such as `M2IDLE`, `M2CHAOS`, `M2FIGHT`, `M2DIE` and `M2WALK`. A lump whose header names another
  map than its directory is refused. E1M2-E1M7 use the M4a driver fork (`m4a_build.py`); E1M1 stays on the
  merged M3h9c driver, the frozen wire-22 evidence, so its `secretexit` is reported as null.
- `--strict32` adds the opt-in `-m32` variant (no adapters; the wrapper fails, naming `m3h9c_build.py --probe-m32`, when the toolchain cannot build `-m32`).
  `--jobs 1|2` sets the C runs in parallel (at most 2; the default is 2). `--keep-dumps` keeps
  `WORK/runs/<variant>_<MAP>_<script>/`, which `level_compare.py` and `level_snapshot.py` read; without it the
  dumps are deleted once the digests and events are recorded.
- The wrapper requires byte-identical dump sets in every variant and writes
  `WORK/results/<MAP>_<LUMP>.json` (tics, end `marker`, `exit` or `reborn`, end tic, `prndindex` at load
  and at the end, thinkers and mobjs, mover starts, pickups, key cards, weapons, damage tics, blocked
  stair steps and raise-and-change plats, and the digests). It never records a coordinate, angle or lump
  byte. A blocked stair step or raise-and-change plat is the H7 situation: the C side runs on and Rust
  refuses the tic with `StairBlocked` or `PlatBlocked`.
- `player_level_run --policy episode1` loads the map under wire 23 and writes `digests.txt`,
  `stats.txt` and, on a refused tic, `refused.txt`; the comparator ends a complete run at the C side's
  `exit` or `reborn` tic, where Rust refuses the next tic (`LevelExited`, `PlayerReborn`). Under
  `--policy episode1` only, `stats.txt` also has a `store_high_water` line (one past the highest
  actor-store slot that `thinker.order` names on any capture); the wire-22 output is unchanged. The demo
  reader parses the version-110 header (13 bytes, with the episode, map and skill fields) through
  `parse_demo_episode1(bytes, map)`, so a demo of any of the five maps runs through the example.
- Timings are taken alone on an idle machine, with no concurrent build, test or fuzz job. `--values` of
  the comparator prints map-derived values and stays local.

CI path: the `stable` job of `.github/workflows/rust.yml` runs, in order, `m4a_build.py --check`,
`test_runner_newlines.py`, `m4a_wad.py --check`, `m4a_check_wad.py --check` and
`e1m1_scripts.py --check` (every OS, under Bash so every command's failure fails the step);
`m4a_reference.py --check --keep-dumps --selftest 300` and
`m4a_snapshot_goldens.py --dumps DIR/runs --check` (Linux only, one runner build); and a
release build of `player_level_run` followed by
`check_player_episode1.py --example-args "--policy episode1" --require` (every
OS). All are skipped for documentation-only pushes. Local only: the opt-in `strict32` variant,
which needs a 32-bit toolchain. Observations of the matched runs belong to the M4a evidence
record, not to this page.
## M4b synthetic mechanism WADs

The M4b [reference investigation](M4B_REFERENCE_INVESTIGATION.md) adds small synthetic
WADs (listed in `tests/reference/m4b_manifest.json` and `m4b_manifest2.json`) for teleportation, telefrags, monster and sliding teleports, the Baron of Hell,
`A_BossDeath` (one WAD in the E1M8 slot), the special-line (`spechit`) limit and the skill-3
THING filter, with their scripts run through the genuine engine. Everything here is synthetic and
runs on a clean checkout; the observations are recorded in
[the M4b evidence record](validation/M4B.md#synthetic-mechanism-wads-ci-evidence). The Rust side
is wire 24 (`--policy episode1v2`): it runs the 30 M4a scripts against their DSIM40 projection
now, and every M4b script (plan steps 4-6 admitted them all; the `PENDING` table of
`check_player_episode1v2.py` is empty).

```sh
# generators and invariants (no compiler; every OS in CI)
python3 tools/reference/m4b_build.py --check               # M4b runner contract: the M4a driver plus the telefrag adapter (p_inter.c:817, sanitizer builds only)
python3 tools/reference/m4b_wad.py [--out-dir DIR] [--check]  # every WAD of both manifests, in tests/reference/m4b_wads/; --check compares bytes and both manifests' fields, then runs the verifier
python3 tools/reference/m4b_check_wad.py --check           # independent verifier of every WAD of both manifests (teleport search order, special-line box, skill counts)
python3 tools/reference/m4b_scripts.py --check             # tests/reference/m4b_scripts (.txt -> .lmp; adds the "skill N" directive)
python3 tools/reference/test_level_snapshot_m4b.py          # DSIM40 tables against info.h/info.c, dump columns against m4a_driver.c, header rules

# genuine runner execution (C; Linux only in CI); ASAN_OPTIONS=detect_leaks=0 is set for the address variant (leak detection disabled)
python3 tools/reference/m4b_reference.py [--work DIR] [--check] [--keep-dumps] [--cc CC] [--only WAD] [--selftest TIC] [--update-events]
python3 tools/reference/m4b_reference.py --work DIR --strict32   # local opt-in: also the -m32 genuine-text build, dumps must be identical; writes nothing
python3 tools/reference/m4b_build.py --variant strict|overflow|address|all|strict32 [--out DIR]

# the independent DSIM40 projection (profile episode1v2) and its goldens, from the strict dumps
python3 tools/reference/m4b_reference.py --work /mnt/d/scratch/m4b/ref --check --keep-dumps
python3 tools/reference/m4b_snapshot_goldens.py --dumps /mnt/d/scratch/m4b/ref/runs --check [--selftest]
python3 tools/reference/m4b_snapshot_goldens.py [--work DIR] --check   # runs m4b_reference.py itself, deletes the dumps afterwards

# the second M4b set (step 5: TELE2X, two teleports in one tic; step 7: SAWBITE, a bite on an A_Saw-flagged body; step 9: BTUNNEL, a Baron fireball through an inner wall; step 14: TELEZ, a walking monster's teleport fog z), the same tools with --set 2
python3 tools/reference/m4b_reference.py --set 2 --work /mnt/d/scratch/m4b/ref2 --check --keep-dumps
python3 tools/reference/m4b_snapshot_goldens.py --set 2 --dumps /mnt/d/scratch/m4b/ref2/runs --check
python3 tools/reference/level_snapshot_m4b.py DUMP_DIR --wad WAD --demo LMP [--out DIR] [--compare39]
```

What each asserts:

- `m4b_reference.py --check` builds the strict, overflow-checked and address-checked runners,
  requires byte-identical dump sets for every script, the manifest's ending and `must_show`
  tics (a `null` tic means the event must never happen), and the committed
  `tests/reference/m4b_synthetic_results.json` (counts, digests, event tics and the per-script
  observed facts; the `timing` block is not compared). For `SPECH9` it also runs the overrun
  probe: the longer `SPECH8` demo on the nine-line WAD in the address variant must stop with
  the sanitizer report at `p_map.c:242` after dumping tic 74. `--selftest TIC` plants an extra
  `P_Random` draw and a changed command at tic TIC of `TELE1` and requires each to be first
  reported at that tic.
- `--set 2` selects the second M4b set: `tests/reference/m4b_manifest2.json`,
  `m4b_synthetic_results2.json`, `m4b_synthetic_goldens2.json` and the `m4b2_episode1v2_*.bin`
  fixtures. It exists so that every file of the first set stays byte-identical when scripts are
  added; it has no self-test script of its own (the first set's self-tests cover the tools).
- `m4b_snapshot_goldens.py --check` requires each strict dump set to equal the
  `dumps_sha256` of `m4b_synthetic_results.json`, projects every script to DSIM40 with the
  skill and map number of its demo header, checks each fixture's claim on its dump, and
  compares `tests/reference/m4b_synthetic_goldens.json` and the 19
  `tests/reference/m4b_episode1v2_*.bin` fixtures byte for byte. `--selftest` adds eight
  checks: a changed field (`reactiontime` at the `TELE1` teleport) moves the first differing
  digest to exactly that tic; an unknown row kind, an extra column, a landing out of `S_NULL`,
  a Baron in a raise state and a skill that differs from the dump are refused; another map
  number changes every digest; and every snapshot of every script projects to DSIM39 bytes
  except `profile`, `context.schema`, `actors.options` and `level.map`.
  `level_snapshot_m4b.py --compare39` runs that last check on any dump directory (for example
  the M4a runs, locally). `test_level_snapshot_m4b.py` needs no dumps.

The Rust side (wire 24, DSIM40):

```sh
# DSIM40 goldens of the 30 M4a scripts, from the genuine M4a dumps (C; Linux only in CI)
python3 tools/reference/m4a_reference.py --check --keep-dumps --work /mnt/d/scratch/m4b/m4a
python3 tools/reference/m4b_m4a_goldens.py --dumps /mnt/d/scratch/m4b/m4a/runs --check [--compare39]

# the Rust example under wire 24 against both goldens sets (every OS in CI)
cargo build -p doom-sim --release --locked --example player_level_run
python3 tools/reference/check_player_episode1v2.py --binary target/release/examples/player_level_run --out DIR --require
python3 tools/reference/check_player_episode1v2.py --rust-root DIR [--require]   # captures already made

# wire-24 soak (Rust-only, in the workspace tests) and fuzz target (M4b step 9 and 12)
cargo test -p doom-sim --release --locked --test episode1v2_soak
python3 tools/seed_player_episode1_v2_fuzz.py /tmp/doom-episode1v2-seeds      # seed corpus, max_len 2404
cd fuzz && cargo +nightly fuzz run player_episode1_v2 /tmp/doom-episode1v2-corpus --sanitizer address -- -runs=0 -max_len=2404 -timeout=180 -rss_limit_mb=2048   # replay; add -max_total_time=1800 -seed=N for a coverage-guided run

# one run against its dumps, field by field (any dump directory; DSIM40)
python3 tools/reference/level_compare_m4b.py DUMP_DIR RUST_DIR --wad WAD --demo LMP [--values] [--json OUT]
```

The README's animated recording (`docs/media/m4b-tele1-side-by-side.svg`) is generated, not
captured: `python3 tools/media/make_readme_recording.py [--work DIR] [--cargo-target DIR] [--check]`
builds the strict genuine runner, runs the synthetic TELE1 script through it and through
`player_level_run --policy episode1v2 --captures`, and draws the values read from the two engines'
per-tic snapshots (it needs a C compiler and cargo, and refuses to write if any tic differs). The
output is deterministic; `--check` compares it with the committed file.

- `m4b_m4a_goldens.py --check` requires each strict M4a dump set to equal the frozen
  `dumps_sha256` of `m4a_synthetic_results.json`, projects every M4a script to DSIM40 with the
  skill and map of its header and compares `tests/reference/m4b_m4a_goldens.json` byte for
  byte. `--compare39` also requires every snapshot's DSIM40 projection to equal its DSIM39
  projection outside the four DSIM40 fields.
- `check_player_episode1v2.py` runs every script of the M4a manifest and both M4b manifests with
  `--policy episode1v2`, except the scripts its `PENDING` table names with the step that
  admits them (empty since step 6), and compares each tic's stripped digest with the goldens. M4a scripts keep their
  M4a manifest expectations; an M4b `tbd` must match, and `SPECH9` must then refuse its declared
  tic with `SpecialCross`: since `SPECH9.lmp` ends just before that tic, the tool runs it on its
  manifest `overrun_probe` demo (`SPECH8.lmp`, whose commands begin with SPECH9's). With
  `--require` a missing wire 24 or a non-pending script without captures fails. It prints each
  pending script and the step it waits for.
- `cargo test -p doom-sim --test episode1v2_mechanism_goldens` (part of the workspace tests)
  replays the admitted M4b scripts through the public API against the same goldens, decodes
  every capture with the DSIM40 reader and compares each committed fixture byte for byte (with
  the capture's event lists emptied, the fixtures' form).

Real maps, local only (M4b step 4; never in CI): the local Episode 1 runner takes uncommitted
demos with `--lmp`, E1M5, E1M8 and E1M9 included (those use the M4b runner). For example, an
idle demo written into scratch, the genuine run and the comparison:

```sh
target/release/examples/player_level_run --write-idle 2100 --demo /mnt/d/scratch/m4b/M5IDLE.lmp --policy episode1v2 --map E1M5
DOOM_IWAD_DIR=/mnt/d/DoomIWADs python3 tools/local/episode1_reference.py --work /mnt/d/scratch/m4b/real --lmp /mnt/d/scratch/m4b/M5IDLE.lmp --keep-dumps
target/release/examples/player_level_run --wad /path/to/DOOM.WAD --map E1M5 --demo /mnt/d/scratch/m4b/M5IDLE.lmp --out /mnt/d/scratch/m4b/rust --captures --policy episode1v2
python3 tools/reference/level_compare_m4b.py /mnt/d/scratch/m4b/real/runs/strict_E1M5_M5IDLE /mnt/d/scratch/m4b/rust --wad /path/to/DOOM.WAD --demo /mnt/d/scratch/m4b/M5IDLE.lmp --json OUT
```

A demo's base name becomes a WAD lump name in the genuine runner, so it must have at most eight
characters. Delete the dumps once the counts and digests are recorded.

Skill 3 (step 6): `--write-idle` writes a skill-2 header, so for a hard-skill run patch the demo's
skill byte (byte 1 of the 13-byte header; 2 becomes 3) in scratch before both runs; the genuine
engine and wire 24 both read the skill from that header. The commands are otherwise those above
(for example `M5IDL3.lmp` in place of `M5IDLE.lmp`), and the dumps and the demo are deleted after
the digests are recorded.
- `level_compare_m4b.py` is `level_compare.py`'s method over the DSIM40 projection. It is a new
  tool, because `level_compare.py` is frozen and has no DSIM40 profile.

## README animations

The simulation recording is described under [M4b mechanism WADs](#m4b-synthetic-mechanism-wads). The renderer
animation (`docs/media/m5a-renderer-side-by-side.gif`) is generated the same way, never captured:

```sh
python3 tools/media/m5a_readme_visual.py [--work DIR] [--cargo-target DIR] [--check]   # --work defaults to ~/scratch/m5a-readme-visual
```

It builds the strict genuine M5a runner (`m5a_build.py`, a C compiler), runs it with `--frames-raw` on the
committed synthetic rooms SKY, SPRITES and FUZZ (their scripts and `tests/reference/m5a_gfx.wad`, no IWAD), and
renders the same tics with `m5a_view --raw` (screenblocks 11). Every shown frame must be byte-identical between
the two and equal to the golden digest in `tests/reference/m5a_frames/*.sb11.frames` (and not a non-comparable
frame), or the script exits with an error. It shows every 5th tic at real speed, draws the genuine frame, the
Rust frame and their per-pixel difference, and computes the caption's differing-pixel count from the frames; the
lettering is the script's own 5x7 bitmap font. The GIF encoder is a Python port of `crates/doom-render/src/gif.rs`
with frame deltas, and the script decodes its own output and compares every displayed frame. The file has no
timestamp or path, so a rerun writes the same bytes. `--check` regenerates the animation and compares it with the
committed file; it builds the C runner and runs cargo, so it is a local check and not part of CI.

The status bar animation (`docs/media/m5b-status-bar-side-by-side.gif`) is generated the same way:

```sh
python3 tools/media/m5b_readme_visual.py [--work DIR] [--cargo-target DIR] [--check]   # --work defaults to ~/scratch/m5b-readme-visual
```

It builds the strict genuine M5b runner (`m5b_build.py`, a C compiler) and the release examples `m5b_session`
and `m5b_animation_frames`. It plays the committed session `palred_damage` (synthetic `palred.wad` and
`m5b_gfx.wad`, screenblocks 10, no IWAD) through the headless app and replays the app's demo through the genuine
runner and through the app's composed `Display`. It requires the script and the app's demo to match
`tests/reference/m5b_session_results.json` (`script_sha256`, `driver_demo_sha256`), the genuine normalised frame
stream to hash to the committed `frames_sha256`, every raw genuine frame to equal its stream digest, and all 155
tics to be identical between the genuine and Rust frames (palette indices) and palette numbers. On the first
difference it exits with the tic and pixel and writes nothing. Each frame is drawn through the PLAYPAL palette of
its tic, so the pickup and damage flashes show; the difference panel and the differing-pixel count in the caption
are computed from the frames. The GIF has no timestamp or path, so a rerun writes the same bytes; `--check`
regenerates it and compares it with the committed file (a local check, not part of CI). The art is our own
generated test art on a synthetic room, not the game's, and the frames are identical to the genuine engine under
the declared sound-free, wipe-free contract, not to a vanilla session.

The sound picture (`docs/media/m5c-mixer-waveforms.svg`) and its audio (`docs/media/m5c-mixer-session.wav`) are
generated the same way:

```sh
python3 tools/media/m5c_readme_visual.py [--work DIR] [--cargo-target DIR] [--binary FILE] [--check] [--selftest]
```

It runs the genuine mixer (linuxdoom-1.10 built with its in-process sound code, one define removed; the strict
build of `m5c_mixer_build.py`) and the release example `m5c_mixer_raw` on the committed start list of the synthetic
session SNDDIST (our own generated test sounds in `m5c_sfx.wad`, no IWAD) under the declared schedule. It refuses to
draw unless the genuine rows equal the committed golden, and it stops on the first sample where the two streams
differ; the differing-sample count in the picture comes from that comparison. `--check` regenerates the SVG and the
WAV and compares them with the committed files byte for byte, `--selftest` plants four faults that must be caught,
and the CI job `M5c genuine sound` runs both. Nothing is played. An MP4 is written only if `ffmpeg` is
already installed, and it is never checked.

## M5a synthetic renderer rooms

The M5a [reference investigation](M5A_REFERENCE_INVESTIGATION.md) (decisions R1-R16, §5) compares
a Rust renderer with the genuine linuxdoom-1.10 `R_RenderPlayerView`. Phase 2 step 1 adds the
genuine side: a synthetic graphics PWAD (no id art), one synthetic room per renderer feature and
per vanilla limit, and the M5a runner, which compiles the genuine renderer whole and renders after
every dumped tic. Everything here is synthetic and runs on a clean checkout; the observations are
in [the M5a evidence record](validation/M5A.md).

```sh
# generators, verifiers and the contract (no compiler)
python3 tools/reference/m5a_gfx.py [--out PATH] [--list] [--check]   # tests/reference/m5a_gfx.wad; --check compares bytes, then runs the verifier
python3 tools/reference/m5a_check_gfx.py [PWAD]                        # independent verifier: colour maps, inherited texture/flat order and heights, switches, sky, sprite frames and rotations, patch bounds and padding
python3 tools/reference/m5a_wad.py [--only NAME] [--list] [--manifest] [--check]   # rooms in tests/reference/m5a_wads/, scripts in tests/reference/m5a_scripts/, generated manifest fields
python3 tools/reference/m5a_build.py --check                           # tests/reference/m5a_runner_contract.json (DOOM-M5A-RUNNER-v1)

# the genuine renderer (C; Linux); the address variant runs with ASAN_OPTIONS=detect_leaks=0 (leak detection disabled)
python3 tools/reference/m5a_reference.py --work DIR --check [--pngs] [--selftest]
python3 tools/reference/m5a_reference.py --work DIR --strict32          # local opt-in: also the -m32 genuine-text build; writes nothing
python3 tools/reference/m5a_reference.py --work DIR --only M5SHORT,BINF1 # a subset; writes nothing
python3 tools/reference/m5a_build.py --variant strict|overflow|address|all|strict32 [--out DIR]
```

What each asserts:

- `m5a_gfx.py --check` regenerates the PWAD and requires the committed bytes; the verifier
  re-parses it without importing the generator (rules in its docstring).
- `m5a_wad.py --check` regenerates every room WAD, script text and `.lmp`, and the manifest's
  generated fields (WAD digest and counts, script digest and tic count, `png` frames and the
  `must_show` expressions; a frame or view expression may end in `from N`, the first tic at or
  after N where it holds). The observed tics in the manifest come from `m5a_reference.py
  --update-events` and are held fixed by `--check`.
- `m5a_build.py --check` regenerates the runner contract: sources and their adapted digests, the
  inherited and the renderer adapters (each with file, line, original, adapted text and reason; the
  three line-keyed ones record their occurrence count), stubs, wrapped symbols, driver behaviour,
  view configurations, and the dump, frame and view row kinds.
- `m5a_reference.py --check` builds the strict, overflow and address runners and the frozen M4b
  runner. For every room and every M4b synthetic script: the never-render dump set must equal the
  committed M4b digest (M4b scripts) or the frozen M4b runner's dump set (rooms); for screenblocks
  11, 10 and 9 every variant renders every dumped tic, and the dump sets (with `ML_MAPPED` masked)
  must equal the never-render set, while the frame streams and view records must be byte-identical
  across the variants up to the earliest end (tracker-flagged frames compared on their masked
  digest). It then requires the manifest's endings and event tics, `tests/reference/m5a_results.json`,
  every `tests/reference/m5a_frames/*.frames` and `tests/reference/m5a_views/*.view`, and with
  `--pngs` the pixels of every `tests/reference/m5a_png/*.png`. `--selftest` plants a one-pixel change
  in M5VIEW's frame of tic 20 and requires the comparison to report that tic and that pixel.
- Low detail is refused (decision R2): `--detail 1` exits with status 8 and no comparison is built.

The Rust renderer (`crates/doom-render`, Phase 2 step 2) and its comparison with those goldens:

```sh
python3 tools/reference/m5a_tables.py --check     # crates/doom-render/src/tables.rs from linuxdoom-1.10/tables.c
cargo test -p doom-render                                   # unit tests, the view-shadow validation and the frame harness
cargo test -p doom-render --release --test view_shadow -- --nocapture   # per-field counts against tests/reference/m5a_views
cargo test -p doom-render --release --test frames -- --nocapture        # frames compared, matched, refused; the limit refusals
cargo test -p doom-render --release --test switches                     # the switch tracker against M5SWTCH's genuine switch textures
cargo test -p doom-render --release --test rewind -- --nocapture        # restore the render state at tic N-k next to a simulation clone, continue to N
# local first-difference reporter: counts, the first non-matching frame per item and configuration,
# and with --c-runner the pixel comparison of every frame against the genuine raw frames, naming the
# feature in play at the first differing pixel, with a PNG pair (genuine, Rust) in ~/scratch/m5a/diff
# timings (recorded, never gated; run alone on an idle machine, built and run from ext4: CARGO_TARGET_DIR=~/scratch/m5a/perf):
# per-tic simulation advance and render time, warm-up then N replays, median/p95/max per configuration
cargo run -p doom-render --release --example m5a_bench -- --room M5PLANE [--screenblocks 11,10] [--reps 5] [--warmup 1]
cargo run -p doom-render --release --example m5a_bench -- --iwad ~/scratch/m5a/iwad/DOOM.WAD --lmp tests/reference/e1m1_scripts/WALK.lmp
python3 tools/reference/m5a_build.py --variant strict --out ~/scratch/m5a/build
cargo run -p doom-render --release --example m5a_compare -- --c-runner ~/scratch/m5a/build/strict/crunner [--only M5PEG,TELE1] [--screenblocks 11,10,9] [--diff DIR]
```

- `m5a_tables.py --check` regenerates `finetangent` followed by `finesine` (one array, the
  genuine contiguous layout) and `tantoangle` from the preserved source text and requires the
  committed bytes.
- `tests/view_shadow.rs` replays every item through the simulation's public API and requires every
  field of every genuine view record (`viewx` .. `lightlevel` and the A rows) to equal the Rust
  renderer's view inputs: the view shadow (`viewz`, `viewheight`, `deltaviewheight`, `onground`,
  `fixedcolormap`) and the simulation values it reads.
- `tests/frames.rs` runs every item once with three renderers (screenblocks 11, 10, 9) observing
  after every committed `advance` and compares every frame with the committed SHA-256 (the masked
  digest where the genuine tracker flagged columns). It requires every frame to match, with the
  Rust flagged columns equal to the genuine U rows, except M5SWTCH's 35 parity-unknown frames per
  configuration (decision R7); on every frame the genuine F row's counters (mapped lines,
  visplanes, drawsegs, vissprites, openings, the solid-segment peak, dropped drawsegs, the
  vissprite overflow), `fuzzpos` and M row (newly mapped lines); no refused frame; and, where the
  genuine run ends at a renderer limit (`m5a_results.json`), the matching refusal at the next tic
  (VPLANES, OPENINGS, SOLIDSEGS). Its self-test plants a one-pixel change and requires a mismatch.
- `tests/rewind.rs` (owner ruling on R5-R7): for M5FUZZ, BINF1, M5SWTCH, M5VIEWZ and M5DSEG at
  the three configurations it renders straight through to tic N, and separately restores, at tic
  N-k, a clone of the simulation (doom-sim has no restore-from-snapshot API) and the render state
  through its `DRST` encoding into a fresh renderer, then continues to N; every rewound frame and
  report and the encoded render state at N must equal the uninterrupted run's, and every rewound
  frame must match its golden. Its self-test restores a state with a changed `fuzzpos` and
  requires a different frame.
- `m5a_compare --c-runner` (local) reruns the genuine runner with `--frames-raw` per item and
  configuration and compares every pixel of every comparable frame except the genuine flagged
  columns; the first difference names the stage and its object (seg, visplane, vissprite with
  sprite and frame, weapon sprite, fuzz owner). On the first compared frame of each item and
  configuration it plants a one-pixel change and requires the comparison to detect it.
- `tests/switches.rs` replays M5SWTCH and requires the switch tracker's side textures to equal the
  genuine `s0mid` of its four switch lines at every tic where the parity is known (the expected
  values are the strict runner's dump rows, recorded in the evidence record), and the parity to be
  unknown exactly at the tics after a re-press until the button expires (decision R7).

Real maps, local only (M5a step 4; never in CI, it reads the IWAD from a scratch copy). The tool runs
the 61 runs of `tests/reference/m4b_real_results.json` (the committed E1M1-E1M9 scripts and DEMO1,
extracted from the IWAD copy and patched 109 to 110 into `--work`, checked against the recorded
digest) through the genuine M5a runner in the strict, address and overflow variants (address with
`ASAN_OPTIONS=detect_leaks=0`, so leak detection is disabled) and through the `m5a_real` example
(the Rust simulation replays the demo, the Rust renderer renders every tic), at screenblocks 11 and
10, high detail; at most 2 C jobs at once:

```sh
python3 tools/local/m5a_real.py [--maps E1M1,E1M2] [--only WALK,M2DIE] [--diagnose] [--write | --check]
cargo run -p doom-render --release --example m5a_real -- --iwad ~/scratch/m5a/iwad/DOOM.WAD \
  --lmp tests/reference/e1m1_scripts/WALK.lmp --frames 10=FILE [--raw 10=DIR] --json OUT   # one run by hand
```

- Per run and configuration it requires the three variants' frame streams to be identical up to the
  earliest end (a tracker-flagged frame compared on its masked digest and its flagged columns), and
  reports the Rust frames matched, mismatched, non-comparable (switch parity unknown, R7; whole frame,
  R4), refused by kind and not reached, with the counters, `fuzzpos` and newly mapped lines of every
  frame. Any mismatch, refusal, counter difference or variant difference makes it exit 1.
- `--write` merges the selected runs into `tests/reference/m5a_real_results.json`
  (DOOM-M5A-REAL-RESULTS-v1: counts, pass/fail facts and the genuine and Rust stream digests, the rule
  in the tool's docstring); `--check` requires the selected runs' records to equal the committed ones.
- `--diagnose` reruns a failing run with raw frames, and `m5a_real --raw` names the feature at the
  first differing pixel and writes a PNG pair to `~/scratch/m5a/diff`. Those images, the genuine
  frame streams (they name IWAD textures), the per-frame digest lists (`--work/digests`) and the
  per-texture report (`~/scratch/m5a/reports/real_textures.json`) are IWAD-derived and stay in
  scratch; delete them once the counts are recorded.

Local viewer (M5a step 5, decision R16; a developer aid, not a comparison). `m5a_view` replays a demo
script through the Rust simulation and renderer and writes an animated GIF of the 3D view in palette 0 of
the WAD in use (palette flashes are M5b), using the hand-written encoder `doom_render::gif` (GIF89a, LZW,
no dependency; unit-tested by decoding its own output with an independent test decoder). Build and run it
from the Linux disk; IWAD-derived output must never be committed (write it to `/mnt/d/DoomRenders/`):

```sh
cargo run -p doom-render --release --example m5a_view -- --iwad ~/scratch/m5a/iwad/DOOM.WAD \
  --lmp tests/reference/e1m1_scripts/WALK.lmp --out /mnt/d/DoomRenders/e1m1_walk.gif \
  [--screenblocks 11] [--from TIC] [--to TIC] [--scale 2] [--fps 35]
cargo run -p doom-render --release --example m5a_view -- --room M5AMP --out ~/scratch/m5a/m5amp.gif   # a committed synthetic room, no IWAD
```

GIF delays are whole hundredths of a second, so 35 tics/second is written as 2, 3, 3, 3, 3, 3, 3 cs
repeating (20 cs per 7 tics; every `n * 100 / fps` cs total is exact), never as a uniform 3 cs.
`--scale` is an integer nearest-neighbour enlargement (1-8). The replay ends at the end of the script, at
`--to`, when the simulation stops advancing, or at a renderer refusal (reported).

CI (M5a step 6, decision R15). The Rust renderer's synthetic-room frames against the committed goldens
(`tests/frames.rs`, `view_shadow.rs`, `switches.rs`, `rewind.rs`) are ordinary workspace tests, so the
`Stable` job already runs them on Linux, Windows and macOS (`cargo test --workspace`); nothing further is
needed there. The genuine renderer (C, Linux only, as M4b's genuine runs) is the separate job
`M5a genuine renderer`, which runs on the runner's default gcc:

```sh
python3 tools/reference/m5a_gfx.py --check && python3 tools/reference/m5a_check_gfx.py
python3 tools/reference/m5a_wad.py --check && python3 tools/reference/m5a_build.py --check
python3 tools/reference/m5a_tables.py --check
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5a_reference.py --check --pngs --selftest --jobs 4 --work DIR
```

The last command builds the strict, overflow and address runners (leak detection disabled in the address
variant) and the frozen M4b runner; locally keep `--jobs` at 2 or below on the owner's machine. The real-map
tool stays local (it reads the IWAD).

Renderer guard tests and fuzz targets (M5a step 7a). The unit tests below the public API (`r_draw`,
`r_things`, `r_main`, `r_data`, `state`) name the guard they pin and why `Renderer::render` cannot reach it;
`crates/doom-render/src/test_support.rs` builds a frame context over the committed `amp.wad` room for them.
`tests/contract.rs` covers the public contract (configuration refusals, call order, the limit refusals are
typed, repeatable and change nothing, and rendering never changes the simulation, compared with a bare run
tic by tic). All run in `cargo test -p doom-render` and the workspace tests; no IWAD. Two fuzz targets
(no IWAD, no id art) and one seed tool:

```sh
python3 tools/seed_render_fuzz.py frames DIR   # render_frames seeds (max_len 516): the 45 scripts of the target's 41 WADs at three view sizes plus variants
python3 tools/seed_render_fuzz.py data DIR     # render_data seeds (max_len 2048): edits of the synthetic graphics WAD and raw tails
cargo +nightly fuzz build --sanitizer address render_frames   # also render_data; or build every target as above
# seed replay (not a campaign); ASAN_OPTIONS=detect_leaks=0 means LeakSanitizer is disabled, not passed
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run render_frames --sanitizer address CORPUS -- -runs=0 -max_len=516 -timeout=300 -rss_limit_mb=2048
cargo +nightly fuzz run render_data --sanitizer address CORPUS -- -runs=0 -max_len=2048 -timeout=60 -rss_limit_mb=2048
# bounded coverage-guided campaign (M5a; leak detection on works for both targets), binaries from a CARGO_TARGET_DIR on ext4:
# ASAN_OPTIONS=detect_leaks=1 BIN/render_data CORPUS -max_total_time=1800 -max_len=2048 -timeout=60 -rss_limit_mb=2048 -seed=N
```

- `render_frames` loads one of the 23 M5a rooms or 18 M4b WADs with the synthetic graphics set, runs the
  byte-derived commands (clamped into the simulation's admitted domain), and observes and renders every tic
  at screenblocks 9, 10 or 11: no panic; a refusal is a typed `RenderRefusal` that leaves the render state
  unchanged; a cloned renderer and simulation render the identical frame, report and state; the render state
  round-trips through `DRST` and a mutated or truncated copy never panics; the simulation digest is the same
  before and after every frame. It compares no pixels (the committed goldens do).
- `render_data` edits lumps of the synthetic graphics WAD in memory (counts, offsets, sizes, deletions) and
  loads the result with `Resources::load`, walks every column, flat and sprite of a successful load, and
  feeds the raw tail to `RenderState::decode` and the patch readers. Its seeds cannot reach a valid `DRST`
  (64,000-byte framebuffer): `render_frames` covers the deep decoder paths with real states.
- `seed_render_fuzz.py` is deterministic (fixed-seed generator, committed inputs only) and has no `--check`
  flag, as the other `seed_*_fuzz.py` tools have none.

## M5b playable: status bar, HUD, input and the app

The M5b [reference investigation](M5B_REFERENCE_INVESTIGATION.md) (decisions S1-S23, owner rulings
2026-11-17) adds three things to compare with the genuine linuxdoom-1.10: the status bar, HUD and palette
choice (the M5b runner), the event-to-command path (the M5b input driver) and whole app sessions. Everything
committed is synthetic and runs on a clean checkout; the observations are in
[the M5b evidence record](validation/M5B.md) and the support statement is
[Playable build (M5b)](COMPATIBILITY.md#playable-build-m5b). **Frames are identical to the genuine engine
under the declared sound-free, wipe-free contract (decision S15)**, never claimed identical to a vanilla
session without that qualifier.

```sh
# generators, verifiers and contracts (no compiler; every OS in CI where the job runs them)
python3 tools/reference/m5b_gfx.py [--out PATH] [--list] [--check]      # tests/reference/m5b_gfx.wad: the M5a set plus our own status bar, face and HUD font lumps
python3 tools/reference/m5b_check_gfx.py [PWAD]                           # independent verifier of that WAD (it re-parses it)
python3 tools/reference/m5b_wad.py [--only NAME] [--list] [--manifest] [--check]   # the 11 new rooms in tests/reference/m5b_wads/, scripts in tests/reference/m5b_scripts/
python3 tools/reference/m5b_build.py --check                              # tests/reference/m5b_runner_contract.json
python3 tools/reference/m5b_input_scripts.py --check                      # the 251 event scripts of tests/reference/m5b_input/scripts
python3 tools/reference/m5b_input_build.py --check                        # tests/reference/m5b_input_contract.json (DOOM-M5B-INPUT-v1)

# the genuine status bar, HUD and palette (C; Linux); the address variant runs with ASAN_OPTIONS=detect_leaks=0 (leak detection disabled)
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5b_reference.py --work DIR --check [--pngs] [--selftest] [--jobs N]
python3 tools/reference/m5b_build.py --variant strict32 --out DIR         # local opt-in: builds the -m32 genuine-text runner only; m5b_reference.py has no --strict32 option (this line listed one until 2026-11-18)
python3 tools/reference/m5b_reference.py --work DIR --only NAME[,NAME]    # a subset; writes nothing

# the genuine input path (C; Linux)
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5b_input_reference.py --work DIR --check [--selftest] [--jobs N]
python3 tools/reference/m5b_input_reference.py --work DIR --strict32      # local opt-in; never required
python3 tools/reference/m5b_input_reference.py --work DIR --summary       # which commands fall outside wire 24

# scripted app sessions replayed through the genuine input driver and engine (C and a release build of the headless app; Linux)
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5b_session.py --work DIR --check [--selftest] [--only NAME] [--jobs N]

# full-resolution commands injected into the genuine runner (the unrecorded mode's evidence; C and a release build of the headless app; Linux)
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5b_inject.py --work DIR --check [--selftest] [--jobs N]
python3 tools/reference/m5b_inject.py --contract                          # rewrites tests/reference/m5b_inject_contract.json (every other mode first requires it to equal the regenerated text)
```

What each asserts:

- `m5b_gfx.py --check` and `m5b_check_gfx.py` require the committed graphics PWAD bytes (every M5a lump keeps
  its bytes and number, so every M5a golden keeps its meaning). `m5b_wad.py --check` regenerates the rooms,
  scripts and manifest fields; `m5b_build.py --check` the runner contract. No id art anywhere.
- `m5b_reference.py --check` builds the strict, overflow and address M5b runners and the frozen M4b runner. For
  the 59 items (the 26 M5a rooms, the 22 M4b scripts and the 11 new rooms) at screenblocks 10 and 11: the masked
  never-render dump set equals the committed digest or the frozen runner's; the render-every dump sets equal it
  snapshot for snapshot; the frame streams (frame digests, palette, face, `M_Random` index, messages) are
  byte-identical across the variants; the committed `tests/reference/m5b_frames` goldens and
  `m5b_results.json` match; with `--pngs`, the PNG pixels. `--selftest` plants six faults (a status bar pixel, a
  palette number, an `M_Random` index, a message tic, and the two masks switched off) and requires each to be
  reported at the planted place.
- `m5b_input_reference.py --check` runs every script in each variant: no sanitizer report, byte-identical
  goldens, the recorded demo consistent with the built and executed commands, the witness scripts as stated, the
  chain (a recorded demo replayed by the genuine M5a runner, every command equal) and the committed goldens and
  results. `--selftest` plants four faults (a command bit, a dropped key-up, a changed binding and a changed
  turn-table entry).
- `m5b_session.py --check` plays the four scripts of `tests/reference/m5b_sessions` through the headless app
  (`crates/doom-game/examples/m5b_session.rs`) and compares every tic with the genuine engine as described in
  the evidence record; `--selftest` plants four faults. The scripts were written by hand, not played by a person.
- `m5b_inject.py --check` runs each script of `tests/reference/m5b_inject/*.txt` (a literal or golden-derived command
  per tic, with a nonzero low byte of the turn that no demo can carry) in `tools/reference/m5b_inject.c`, the M5b
  runner's unchanged driver with a declared command-injection mode, in the strict, overflow and address variants
  (frame streams, dumps and endings must be identical); compares the C-only goldens
  `tests/reference/m5b_inject/goldens`; requires the Rust replay (`m5b_session --commands`) to equal them tic for
  tic (command, frame, status fields, messages and the game dump through the independent DSIM40 encoder); checks
  that the control (zero low bytes) equals plain demo replay and that each script's demo-resolution twin diverges;
  and checks the two unrecorded app sessions in `tests/reference/m5b_inject/sessions` as described in the evidence
  record. `--selftest` plants two faults (a low byte and a frame digest). The tool takes the headless example's
  path from cargo's JSON output, so it honours `CARGO_TARGET_DIR`.

The Rust side (all in `cargo test --workspace`, so on every OS and in the Minimum Rust job):

```sh
cargo test -p doom-render --test m5b_display --test m5b_restore   # the Display against every golden frame, field and message; DDSP restore and refusal
cargo test -p doom-game                                            # input goldens (175 app-domain scripts), admission, layer, recorder, the app's headless scenarios
cargo test -p doom-play                                            # key table, argument parsing, UTC dates, letterbox fit, backend choice, the input log (no window opened)
cargo test -p doom-game --test stuckfire                           # 7 tests: orders of press, motion and release (below)
cargo run -p doom-game --release --example m5b_session -- --wad ROOM.wad --wad tests/reference/m5b_gfx.wad --script SCRIPT.txt --out DIR   # one headless session (also --demo FILE.lmp, --map, --skill, --screenblocks, --captures)
```

**Local: real maps and recorded sessions** (reads the IWAD, so never in CI; the IWAD is used in place or from a
scratch copy and nothing derived from it is committed). `tools/local/m5b_session_real.py` replays a script or
a session recorded by `doom-play` through the headless app and the genuine runner in the strict, overflow and
address variants, and ends with `VERDICT: PASS: ...` or `VERDICT: FAIL: first difference at tic T (kind)`:

```sh
python3 tools/local/m5b_session_real.py --iwad /path/to/DOOM.WAD --lmp /path/to/E1M1-skill2-YYYYMMDD-HHMMSS.lmp [--screenblocks 11] [--wad EXTRA.wad]... [--work DIR]
python3 tools/local/m5b_session_real.py --iwad /path/to/DOOM.WAD --script SESSION.txt
```

Add `--screenblocks 11` if the session was played at 11. With a script, the app's demo must also equal the one
the genuine input driver records (that header is E1M1 at skill 2; for other maps only commands are compared).
Any session saved by `doom-play` can be checked this way; the check is not a guarantee about other sessions.

The real-map display check compares the genuine status bar, HUD, palette and 3D view with the Rust display on all
61 runs of `tests/reference/m4b_real_results.json`, every tic at screenblocks 10 and 11 (a full run took about 45
minutes at 4 jobs; use at most 2 C jobs on the owner's machine unless told otherwise):

```sh
cargo build --release -p doom-render --example m5b_real
ASAN_OPTIONS=detect_leaks=0 python3 tools/local/m5b_real.py --work DIR --rust target/release/examples/m5b_real --jobs N [--iwad /path/to/DOOM.WAD] [--maps E1M1,E1M2] [--only NAME[,NAME]] [--screenblocks 10,11] [--selftest] [--check | --write]
```

`--check` compares the run with `tests/reference/m5b_real_results.json` (counts, pass/fail facts and digests only);
`--write` rewrites it; `--selftest` plants faults in a copy of one strict stream and requires them to be found. The
IWAD is read from a scratch copy, and nothing it contains is committed.

**Playing** (a window; a human judges the result):

```sh
cargo run --release -p doom-play -- --iwad /path/to/DOOM.WAD --map 1 --skill hmp --sessions /path/for/sessions
cargo run --release -p doom-play -- --help          # every flag and the stated limits
```

**The stuck-fire tests and the input log (M5c; Rust-only, no genuine run).** `crates/doom-game/tests/stuckfire.rs` has 7 tests of
the app's event layer and pump for the owner's Windows report (left-button fire sticking while the mouse moves): every order of
press, motion and release in one pump and in adjacent pumps (24 and 96 runs), the owner's case spelled out, an exhaustive run of
every 4-pump sequence of 0 to 3 inputs (2 x 50,625 runs), rapid clicks during continuous motion (12 runs), the same through
`App::mouse_button`, `mouse_motion` and `update` (6 runs), and the S6 burst rule (`clicks_faster_than_one_per_build_drain_at_one_per_build`).
The expectation is read from the C and from two existing genuine goldens. None reproduces the fault; a planted fault (a dropped
release) failed 6 of the 7. Results and the investigation: [record](validation/M5C.md#stuck-fire-on-windows-the-investigation). The input
log's tests are in `crates/doom-play/src/input_log.rs` (`cargo test -p doom-play`), among them one that runs 25 pumps with the log off and on
and requires the same frame digests and demo bytes. The option itself is a diagnostic, not a test:

```sh
doom-play.exe --iwad <path to your IWAD> --bindings modern --input-log input-log.txt   # PowerShell, on Windows; one line per raw input and per game event
```

Each line is `KIND PUMP TIC TEXT` (`raw`, `app`, `game`, `pump`, `summary`); the steps for reading it are in the record. The owner's log was read by the summary line's downs and ups (Left 18/15) and by cursor reports that stop while raw motion continues ([record](validation/M5C.md#the-owners-windows-retest-with-the-input-log-2026-11-19)). The file is replaced if it
exists. The tests carry no totals here: the counts are in the record.

`--skill 2` or `hmp` is Hurt Me Plenty and `--skill 3` or `uv` is Ultra-Violence (the demo-header values; vanilla's
own `-skill` counts from 1). The opt-in `--unrecorded-full-turning` writes no demo and cannot be replayed (see
[support](COMPATIBILITY.md#unrecorded-full-turning-mode-opt-in)).

For a measurement or a smoke run, `--quit-after-tics N` saves and quits after N tics; a session plays at 35 tics
a second, so 350 tics take about 10 seconds. Run any timing from a binary built on and run from the Linux
filesystem, never from `/mnt/*`. **Under WSLg an unattended run can hang:** the window occasionally reports a focus loss in its first moments and the
game opens paused ([support](COMPATIBILITY.md#the-app-with-sound-and-what-is-presentation), [record](validation/M5C.md#step-7-sound-in-doom-play)),
and then `--quit-after-tics` is never reached. Recognise it by a demo of 14 bytes (the header and the end marker) and no
tic; use a timeout and retry (a SIGTERM from `timeout` saves nothing).

**Fuzzing** (no IWAD; two targets, from `fuzz/`). Seed replay, a coverage-guided campaign and a sanitizer
campaign are different things; the observed runs, their options and findings are in
[the evidence record](validation/M5B.md#step-12-fuzz-targets), which also says where leak detection was disabled.

- [`game_input_events`](../fuzz/fuzz_targets/game_input_events.rs): layer and core events, controls, refusals,
  determinism. Seeds in `fuzz/corpus/game_input_events/`.

```sh
cargo +nightly fuzz build game_input_events                                  # add --sanitizer none for the plain build
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run game_input_events --sanitizer address CORPUS -- -runs=0 -max_len=1017   # seed replay; leak detection disabled
```

A coverage-guided campaign is the same binary with `-fork=4 -max_total_time=N -max_len=1017 -timeout=20
-rss_limit_mb=2048`.

- [`render_display_state`](../fuzz/fuzz_targets/render_display_state.rs): `DDSP` decode, canonical re-encode and
  restore refusals, and driven runs on the M5b rooms with `observe_with` perturbations, per-tic state round trips
  and save, restore and continue equality. Seeds in `fuzz/corpus/render_display_state/` (inputs are up to about
  64 KB, so executions are slow: a few per second).

```sh
cargo +nightly fuzz build render_display_state                               # add --sanitizer none for the plain build
cargo +nightly fuzz run render_display_state --sanitizer address CORPUS -- -runs=0 -max_len=70000 -timeout=60   # seed replay
cargo +nightly fuzz run render_display_state --sanitizer none CORPUS -- -fork=2 -max_total_time=N -max_len=70000 -timeout=60 -rss_limit_mb=2048
```

`cargo fuzz` defaults to the address sanitizer, so name `--sanitizer none` for a campaign without one. Set
`ASAN_OPTIONS=detect_leaks=0` or `=1` explicitly and say which; whether LeakSanitizer actually ran is a separate
fact to check.

**Mutation testing.** The `Mutants` workflow covers `-p doom-game` as well as `-p doom-sim -p doom-render`
(see [CI mutation workflow](#ci-mutation-workflow-m4b)); `doom-play` is deliberately out of scope (a window
shell with no logic a test could pin). The shard count is one value in the workflow's `Mutants plan` job. Each
test process runs under a memory and time cap (below), because one first-run mutant made `wrap()` allocate without
bound. The runs and the survivors' dispositions are recorded in
[the evidence record](validation/M5B.md#step-11-mutation-testing).

**CI.** The Rust frame, input and app tests run in the ordinary workspace tests (the `Stable` job on Linux,
Windows and macOS and `Minimum Rust / 1.85.0`), which also build `doom-game` and `doom-play`. The genuine runs
are the separate Linux job `M5b genuine status bar, HUD and input` (default gcc; `--strict32` is not used):

```sh
python3 tools/reference/m5b_gfx.py --check && python3 tools/reference/m5b_check_gfx.py
python3 tools/reference/m5b_wad.py --check && python3 tools/reference/m5b_build.py --check
python3 tools/reference/m5b_input_scripts.py --check && python3 tools/reference/m5b_input_build.py --check
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5b_reference.py --check --pngs --selftest --jobs 4 --work DIR
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5b_input_reference.py --check --selftest --work DIR
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5b_session.py --check --selftest --work DIR
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5b_inject.py --check --selftest --work DIR
```

The job reports on a documentation-only push through the same fast path as the M5a job. No required check was
added, renamed or removed for M5b, and `macos-26-intel` is unchanged. A second, label-triggered workflow
(`windows-artifact.yml`, job **`Windows build artifact (doom-play)`**) builds `doom-play.exe` for Windows with
a statically linked C runtime and uploads it as the 30-day artifact `doom-play-windows-x86_64` (with
`LICENSE.TXT` and a generated `README.txt`; no game data). It starts when the repository label `windows-artifact`
is put on a pull request (remove and re-add the label for a new head) or by `workflow_dispatch` once the file is
on `master`; it is informational and not a required check.

### Windows build artifact

To get a Windows build of a pull request: put the repository label `windows-artifact` on the pull request (remove
and re-add it for a new head). The `Windows artifact` workflow run appears under the pull request's checks and in
the Actions tab; when it succeeds, open that run and download the artifact `doom-play-windows-x86_64` from its
Artifacts section. It is a zip of `doom-play.exe` (statically linked C runtime), `LICENSE.TXT` and a generated
`README.txt` that names the commit; it contains no game data. Unzip it and, from a command prompt, run it with the
path to your own `DOOM.WAD` as an argument, for example `doom-play.exe --iwad C:\path\to\DOOM.WAD --map 1 --skill hmp`
(`doom-play.exe --help` lists every option; an OpenGL 2.1 or OpenGL ES 2 driver is required). The job ran
successfully on the M5b pull request on 2026-11-17. The owner ran a native build on Windows 11 on 2026-11-18 ([what he observed](validation/M5C.md#the-owners-playtest-2026-11-18)).

### M5b timings

Recorded, never gated; run alone on an idle machine (check `uptime` and `ps -eo pcpu,args --sort=-pcpu | head -5`
first), built and run from ext4 (`CARGO_TARGET_DIR=~/scratch/m5b/step13/target`), never from `/mnt/*`. The
[M5b record](validation/M5B.md#step-13-timings) owns the results.

```sh
# frame time per room and screenblocks: Display observe+draw, Display draw and the M5a Renderer on the same frames
# (frames, total, mean, median, p95, max in microseconds); run three times
cargo run -p doom-render --release --example m5b_bench -- [--rooms M5MSG,M5PALRD,M5PALBN,M5STDIG,M5EVIL] [--screenblocks 10,11] [--warmup 1]
# tic jitter of the real loop under WSLg: executed and dropped tics and the interval distribution on stderr at exit
# (X11: env -u WAYLAND_DISPLAY; Wayland: env -u DISPLAY); idle, no input, about 100 s
R=tests/reference
cargo run --release -p doom-play -- --iwad $R/m5b_wads/stdigit.wad --wad $R/m5b_gfx.wad \
    --sessions ~/scratch/m5b/step13/sessions --quit-after-tics 3500 --tic-stats
# a start-up pause under WSLg (above) makes this run hang until killed; a 14-byte demo and no tic identify it
```

## M5c sound effects: the genuine sound logic and mixer

The M5c [reference investigation](M5C_REFERENCE_INVESTIGATION.md) (decisions T1-T16, confirmed by the owner on
2026-11-18) is under way on the branch `feat/m5c-sound`. This section lists the commands of the pieces done so far:
the genuine-engine tools (Phase 2 steps 1 and 2), wire 25 and its comparisons, the crate `doom-sound`, the display
contract with sound and the game loop (`doom-render`, `doom-game`), the real-run and session tools, the device in
`doom-play`, the fuzz targets and the audio probe. Everything committed is synthetic and runs on a clean checkout, except the commands marked local
only (they read an IWAD). The observations are in [the M5c evidence record](validation/M5C.md). **The CI job
`M5c genuine sound` runs the genuine-engine commands, [below](#the-ci-job-and-the-mutation-workflow);
it is not a required check.** Every result is produced under the confirmed values of T2, T5 and T6 (the
`--handle integrated`, `--update-schedule tic` and `--wipe genuine` switches of `m5c_build.py`; the tools take the
other values as switches). "Identical to the genuine engine" means linuxdoom-1.10 built with its in-process sound code
(one define removed), under the declared schedule.

```sh
# generators, verifiers and contracts (Python only, no compiler)
python3 tools/reference/m5c_sfx.py [--out DIR] [--list] [--check]        # the three PWADs: tests/reference/m5c_sfx.wad, m5c_sfx_edge.wad, m5c_music.wad
python3 tools/reference/m5c_check_sfx.py [DIR]                           # their independent verifier (it takes a directory or none)
python3 tools/reference/m5c_check_sfx.py --self-test                      # plants 9 defects and requires each to be found
python3 tools/reference/m5c_wad.py [--only NAME] [--list] [--manifest] [--check]   # the 4 rooms in tests/reference/m5c_wads/ and 5 scripts in m5c_scripts/
python3 tools/reference/m5c_build.py --check                              # tests/reference/m5c_runner_contract.json (DOOM-M5C-RUNNER-v1)
python3 tools/reference/m5c_mixer_build.py --check                        # tests/reference/m5c_mixer_contract.json (DOOM-M5C-MIXER-v1)
python3 tools/reference/m5c_starts.py --selftest                          # the bridge from a sound stream to a start list
python3 tools/reference/m5c_starts.py STREAM --check LIST                 # one list against a fresh conversion of its stream; writes nothing

# the genuine sound logic (C; Linux; the address variant runs with leak detection off)
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5c_reference.py --work DIR --check [--selftest] [--jobs N]
python3 tools/reference/m5c_reference.py --work DIR [--jobs N]            # no --check: regenerates the goldens, tests/reference/m5c_results.json and m5c_coverage.json
python3 tools/reference/m5c_reference.py --work DIR --only NAME[,NAME]    # a subset; writes nothing
python3 tools/reference/m5c_reference.py --work DIR --strict32            # local opt-in (-m32): runs only the 32-bit comparison, not --check
python3 tools/reference/m5c_build.py --variant strict|overflow|address|all|strict32 --out DIR   # build the runner alone (also --handle, --update-schedule, --wipe, --sound for a run)

# the genuine mixer (C; Linux)
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5c_mixer_reference.py --work DIR --check [--selftest] [--strict32] [--jobs N]
python3 tools/reference/m5c_mixer_reference.py --work DIR                 # no --check: regenerates the 23 scenarios, the 64 session lists and goldens and both results files
python3 tools/reference/m5c_mixer_reference.py --work DIR --only NAME[,NAME]        # a subset; writes nothing
python3 tools/reference/m5c_mixer_reference.py --work DIR --sessions STREAMDIR [--strict32]   # sound-logic streams through the bridge and the mixer; writes nothing
python3 tools/reference/m5c_mixer_build.py --variant strict|overflow|address|all|strict32 --out DIR   # build the mixer driver alone
python3 tools/reference/m5c_mixer_model.py --wad WAD --starts LIST [--schedule ticbuffers|pertic]   # the independent model on one list

# the new rooms against the Rust wire-24 simulation (the caller builds the existing example first)
CARGO_TARGET_DIR=$HOME/scratch/m5c-target cargo build -p doom-sim --release --locked --example player_level_run
python3 tools/reference/m5c_rust_check.py --binary $HOME/scratch/m5c-target/release/examples/player_level_run [--work DIR] [--only LUMP[,LUMP]]
```

Notes:

- The C tools need a GNU C compiler on Linux (they build the genuine linuxdoom-1.10 files whole; the `-m32`
  variants also need a 32-bit toolchain, which `m5c_build.py --probe-m32` and `m5c_mixer_build.py --probe-m32`
  detect). `--work` and every `--out` must lie outside the repository. The sound-logic runner reads the M5a, M5b
  and M5c synthetic WADs and `tests/reference/m5b_gfx.wad`; no IWAD.
- `ASAN_OPTIONS=detect_leaks=0` means LeakSanitizer is disabled for the address variant: an address-checked run
  is not LeakSanitizer coverage, and every record must say so. The fuzz targets of `doom-sound` are
  [below](#m5c-fuzz-targets-of-doom-sound).
- `--strict32` is a local opt-in and never required. In `m5c_mixer_reference.py` it adds the 32-bit build to
  `--check` (and to `--sessions`). In `m5c_reference.py` it is a mode of its own: it builds the strict32 runner,
  compares its sound and frame streams with the strict ones at screenblocks 10, prints every differing row and
  exits, without `--check`; on 2026-11-18 it found 63 of 64 items identical, the difference being the switch
  release of decision T12.
- `m5c_mixer_reference.py --check` covers the 23 scenarios and the 64 session lists and goldens
  (`tests/reference/m5c_mixer/sessions`) and compares `m5c_mixer_results.json` and
  `m5c_mixer_sessions_results.json`; it first requires each stream's SHA-256 to equal the one in
  `tests/reference/m5c_results.json`, so regenerating the sound streams (for instance after a different ruling on
  T2, T5 or T6) changes the sessions files and not the scenario goldens.
- `m5c_rust_check.py` needs the release build of the existing `player_level_run` example, which the caller makes
  with a `CARGO_TARGET_DIR` outside the tree as shown; it writes nothing in the repository.
- Use `--jobs 2` for the C jobs on the owner's machine unless told it is idle.

**Local only: the real maps and the IWADs** (read an IWAD, so never in CI; use a scratch copy outside the
repository, delete it afterwards, and nothing derived from it is committed except counts and digests):

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/local/m5c_real.py --iwad SCRATCH/DOOM.WAD --work DIR --jobs N [--maps E1M1] [--only NAME] [--write | --check]   # the 61 real Episode 1 runs; tests/reference/m5c_real_results.json
ASAN_OPTIONS=detect_leaks=0 python3 tools/local/m5c_mixer_real.py [--iwad SCRATCH/DOOM.WAD] [--iwad SCRATCH/DOOM2.WAD] --work DIR [--write | --check]   # tests/reference/m5c_mixer_real_results.json
```

`m5c_mixer_real.py` prints one skip line and exits 0 without an IWAD. Both tools write counts, pass/fail facts and
digests only.

### M5c wire 25: the sound records, against the genuine engine

Wire 25 (`PlayerMonstersEpisode1V3`, DSIM41; [support](COMPATIBILITY.md#original-episode-1-part-3-canonical-sound-records-wire-25),
[layout](STATE_FORMAT.md#dsim-v41--wire-25-canonical-sound-records)) is compared with the genuine engine through an
independent Python encoder. The Rust side is the example `sound_records_run`, which the caller builds into a
`CARGO_TARGET_DIR` outside the tree:

```sh
# Rust only (no C): wire 25 against wire 24 on the 64 synthetic items (stripped captures equal; events equal except the scream), and the item table against the 64 committed streams
cargo test -p doom-sim --release --test wire25_projection -- --nocapture   # 3 tests: the item table, the projection to wire 24, and that every capture of the T4 map decodes with the DSIM41 reader (93 of 93)
# Rust only: the twenty unit tests of wire 25 in `wire25_tests.rs` (counted from the source: scream, sector origin arithmetic and overflow, release origin and slot 15, stop records, record order and the Unplaced refusals, THING truncation, the unloaded THING tail not counted, the DSIM41 round trip and its forged records, the forged frames of the reader: a surplus or unknown field, a stop without its removal, slot 15, and the mutation survivors' tests; four of them check the reader's cross-check of pending sound events with the records: `dsim41_reader_bounds_the_pending_event_count_by_the_field_count`, `dsim41_reader_matches_a_pending_sound_event_with_its_record`, `dsim41_reader_matches_sector_and_button_sound_events_with_their_records` and `dsim41_reader_does_not_count_a_malformed_stop_event_as_a_call`)
cargo test -p doom-sim --lib wire25
CARGO_TARGET_DIR=$HOME/scratch/m5c-target cargo build -p doom-sim --release --locked --example sound_records_run
# one run: wire 24 and wire 25 side by side with checks 1 and 2; --out writes the records, --captures DIR or --capture-stream PATH the DSIM41 captures, --alone runs wire 25 without wire 24
sound_records_run --wad WAD --map E1M1 --demo LMP [--out FILE] [--captures DIR | --capture-stream PATH] [--alone]

# every synthetic item, genuine engine against wire 25 over DSIM41 with every sound record, plus the self-test (C and Rust; Linux)
python3 tools/reference/m5c_wire25_check.py --binary $HOME/scratch/m5c-target/release/examples/sound_records_run --work DIR --check --selftest --jobs 4
python3 tools/reference/m5c_wire25_check.py --binary PATH --work DIR [--only LUMP[,LUMP]] [--write]   # --write regenerates tests/reference/m5c_wire25_results.json

# decision T4: the THING-loading truncation map, wire 25 (matches) against wire 24 (diverges at the load capture) (C and Rust; Linux; the caller builds both examples)
CARGO_TARGET_DIR=$HOME/scratch/m5c-target cargo build -p doom-sim --release --locked --example player_level_run
python3 tools/reference/m5c_t4.py --sound-records PATH --player-level PATH --work DIR --check      # without --check it writes the map, script and results

# the encoder and the comparator on one run (the caller has the dumps, the genuine stream and a capture directory or stream)
python3 tools/reference/level_snapshot_m5c.py DUMPDIR --wad WAD --demo LMP --sound STREAM [--out DIR] [--compare40]
python3 tools/reference/level_compare_m5c.py DUMPDIR RUST --wad WAD --demo LMP --sound STREAM [--values] [--json FILE]
```

- `m5c_wire25_check.py --check --selftest` took 29 seconds with `--jobs 4` once the binary was built. `--check`
  regenerates every item's genuine dumps and sound stream, requires the M4b scripts' dumps to equal their committed
  digests and each fresh genuine stream's rows to equal the committed golden's through its last tic, compares the Rust
  captures with the encoder, and compares `tests/reference/m5c_wire25_results.json` (counts and digests). M5CHEAT's
  stream is recorded, not required: its golden was made with cheat keys that the frozen runner and the simulation do
  not see.
- `--selftest` (75 checks) shows the comparison can fail: DSIM41 equals DSIM40 outside the three header fields and the
  sound group on every capture of all 64 items; a changed sound number moves the group at that tic only; an unknown
  actor, a start without positions, a stop of a sector origin, and a stream cut 10 tics short are each refused at the
  right tic; four planted faults on BARON1 (a position bit, a swapped pair, a dropped stop, an actor origin made
  NULL) are each reported at exactly that tic; and two planted faults in a BARON1 Rust frame's layout (the sound group
  moved to the front of the frame, and placed before the setup records of the load capture) are reported by the
  comparator's frame-layout check. That check (in `level_compare_m5c.py`, with the encoder placing the group after the
  last `events.pending` field) requires per capture that every event record follows its count at once and that the
  paths, event records set aside, are equal in order.
- `m5c_t4.py` writes `tests/reference/m5c_t4/truncate.wad`, `TRUNC.txt` and `TRUNC.lmp` and
  `tests/reference/m5c_t4_results.json`; `--check` regenerates the map and script in memory and compares all of them.

**Local only: the 61 real runs under wire 25** (reads an IWAD; use a scratch copy, delete it afterwards, and the
results file holds counts, pass/fail facts and digests only):

```sh
python3 tools/local/m5c_wire25_real.py --binary PATH --iwad SCRATCH/DOOM.WAD --work DIR --jobs 6 [--maps E1M1,..] [--only NAME,..] [--write | --check] [--report PATH]
```

It takes about 50 minutes with `--jobs 6` to write `tests/reference/m5c_wire25_real_results.json`; `--check` takes
about an hour and has not been run since the file was written. Per run it requires the dump set to equal the M4b
strict digest and the genuine stream to give the committed `m5c_real_results.json` sound digest, then compares every
capture. `--report PATH` (scratch, never committed) writes every disagreement with both values. The captures go
through a pipe, so none touches the disk.

### M5c sound in the game: the display contract, the loop, the real runs and the sessions

The Rust tests run in `cargo test --workspace` (every operating system in CI, and the Minimum Rust job); none needs a
device, an IWAD or a C compiler. The comparisons are against committed genuine goldens.

| Test | What it gates | Kind of evidence |
| --- | --- | --- |
| `doom-game` `tests/m5c_frames.rs` | **the frame gate:** the 64 items at screenblocks 10 and 11 through the app's own per-tic pipeline (45,540 frames: face index, status rows, counters, messages, endings on all; **45,170 compared on the whole-frame and face-zeroed digests, 300 compared on the masked digest only** (their face-zeroed digests are `undefined` in the goldens and are counted without a comparison), **70 not digest-compared** (switch parity unknown)), plus two planted faults (draw order, a wipe of 319) | source equivalence |
| `doom-game` `tests/m5c_restore.rs` (4) | `DDSP` version 2 and `DSND` saved at several tics of 7 items, restored and continued equal to the unbroken run and to the goldens (whole-frame and face-zeroed digests and the status row; for the 70 M5SWTCH frames, 35 per size, whose switch parity is unknown, only the status row; none of the seven items has a masked frame); a self-test that it can fail; forgeries refused, including a `DSND` saved on map 8 (`ConfigMismatch`); an M5b save is version 1 and restores as the M5b contract | Rust-only consistency; the continued rows are also equal to the genuine goldens |
| `doom-game` `tests/m5c_app.rs` (4) | a scripted session twice (identical, the demo replays), pause, resume and restart reach the audio sink as specified, a sound refusal keeps the session as before the tic and the end screen shows its plain message (`a_sound_refusal_ends_the_session_with_the_sound_state_preserved`), and a session whose detector fires at tic 1 gives the one layout-dependent line with its exact text (`a_layout_dependent_session_gives_one_plain_line`) | Rust-only |
| `doom-render` `tests/m5c_display.rs`, `tests/m5c_contract.rs` (4) | the same frames with the draws taken from the goldens (a fixture projection); the contract's rules, the `DDSP` version 2 encoding and its forgeries, a display-only restore, and that a version 2 wipe flag the original could not hold is refused (`a_version_2_wipe_flag_the_original_cannot_hold_is_refused`; the same flag is still accepted under version 1) | fixture projection; Rust-only |
| `doom-play` `cargo test -p doom-play` (21, no window, no device) | the sink and queue arithmetic (the start threshold of two buffers, 1,024 frames, and the cap of four), the gain, the options and help text (which carry the reference's qualifiers), the mute key, and `session_tests.rs`: the sound options change no tic, frame, index, sound report, stream or demo, and an unlabelled session leaves no layout-dependent line in the exit lines | Rust-only |

```sh
cargo test -p doom-game --release --test m5c_frames --test m5c_restore --test m5c_app   # about 22 s for the frame gate
# the unit tests in `cargo test -p doom-game --lib` include the frame source's two views of the M_Random index (the_two_views_of_the_random_index_must_agree) and `app::tests::the_app_counts_layout_dependent_sessions_across_restarts` (the app's count of layout-dependent sessions across restarts: a real `App` with a session whose detector fired, a restart into unlabelled and labelled sessions; written to kill three mutation survivors, no production line changed)
cargo test -p doom-render --test m5c_display --test m5c_contract
cargo test -p doom-play
# the 61 real runs, local only (reads an IWAD copy in scratch; only counts and digests are committed). First the example:
cargo build -p doom-game --release --example m5c_real_rows
python3 tools/local/m5c_real_full.py --rust TARGET/release/examples/m5c_real_rows --synthetic --selftest --work DIR --jobs 6   # no IWAD: the committed goldens through the same comparators (about 15 s)
ASAN_OPTIONS=detect_leaks=0 python3 tools/local/m5c_real_full.py --rust TARGET/release/examples/m5c_real_rows --iwad SCRATCH/DOOM.WAD --work DIR --jobs 6 --selftest [--write | --check]   # tests/reference/m5c_real_full_results.json; about 13 minutes
# the scripted app sessions against the genuine M5c runner, the input driver and the mixer (C and Rust; Linux; in CI)
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m5c_session.py --check --selftest --jobs 4 --work DIR   # tests/reference/m5c_session_results.json; writes nothing
python3 tools/reference/m5c_session.py --work DIR [--only NAME]   # without --check: regenerates the results file
# a session you played in doom-play with sound, local only (needs the example above; nothing is written or committed)
python3 tools/local/m5c_session_real.py --rust TARGET/release/examples/m5c_real_rows --synthetic --work DIR   # no IWAD: the self-check (under a minute)
python3 tools/local/m5c_session_real.py --rust TARGET/release/examples/m5c_real_rows --iwad SCRATCH/DOOM.WAD --lmp SESSION.lmp [--lmp SESSION2.lmp]... [--screenblocks 11] --work DIR
```

- `m5c_real_full.py` runs, per real run, the genuine runner at screenblocks 10 and 11, a 32-bit build for sound,
  the genuine mixer and the example, and compares rows, frames and buffers; `--selftest` plants frame, face, mixer-buffer,
  volume and serial faults and requires each to be caught. The address variant is not in it (`m5c_real.py` has it); the
  `ASAN_OPTIONS` shown applies to the genuine runs it starts.
- `m5c_session_real.py` checks demos that doom-play recorded with sound (pass each part of a restarted session): it
  replays each through the example and through the genuine M5c runner (strict, overflow and address identical; address
  with **leak detection disabled**) and the genuine mixer, and compares every frame at the session's screen size, every
  sound-logic row and every mixer buffer, printing one `VERDICT: PASS` or `VERDICT: FAIL` line per demo with counts and
  the first differing tic. `--synthetic` must pass on two committed synthetic demos and must catch three planted faults
  (a changed demo byte, sound row and buffer digest) at their tic.
- `m5c_session.py` plays the five scripts of `tests/reference/m5c_sessions` (its README cites the C) and the four M5b
  scripts through the example `m5c_session`, and checks the pause (no tic and no buffer between a pause and its resume),
  with `--selftest` planting seven faults. Address runs have **leak detection disabled**.
- **A lesson of the T4 check:** `.gitignore` excludes `*.wad` and `*.lmp`, so a generated PWAD or lump is skipped by a plain
  `git add` without a message, and a `--check` passes only where the file already lies. After adding a generator's
  outputs, list them against the tracked files: for example `git ls-files tests/reference/m5c_t4` must show every file
  the tool writes, and `git status --ignored` must not show one of the tool's outputs as ignored. CI found this on
  2026-11-18 ([record](validation/M5C.md#the-ci-record)).

### M5c sound: the doom-sound crate

`doom-sound` is platform-free (no device); its tests need no C compiler and no IWAD. They read the committed synthetic
goldens under `tests/reference` and run in `cargo test --workspace` on every operating system in CI:

```sh
cargo test -p doom-sound [--release]       # 59 tests (counted from the sources: 57 in the test files, 2 unit tests); the end-to-end gate takes about 7 seconds in a debug build
python3 tools/reference/m5c_sounds_table.py --check        # crates/doom-sound/src/sounds_table.rs and steptable.rs against sounds.c, sounds.h and the genuine digest 95f2209c...6dde; writes nothing
python3 tools/reference/m5c_sounds_table.py --self-test    # 8 cases that run the tool's real comparison (compare() and the digest check) on planted inputs: the committed text matches, a changed byte, a missing file and a changed steptable entry each differ; --check and --self-test may be given together
python3 tools/reference/m5c_sounds_table.py                # without a flag: writes both generated files
```

| Test file | What it gates | Kind of evidence |
| --- | --- | --- |
| `survivors.rs` (16) | tests written for the mutants the mutation runs missed in this crate (logic, state, mixer, detector, lumps, session, schedule; the last two, `every_sound_refusal_has_a_message_that_names_its_facts` and `every_session_refusal_has_a_message_that_names_its_facts`, pin the `Display` text of the two refusal types, exact wording, so they are Rust-only); each names the C line or documented rule it rests on, and three take an expectation from the loaded set, the simulation's reading or the `DSND` layout, so they are Rust-only | Rust-only pins, see [step 10](validation/M5C.md#step-10-mutation-testing) |
| `tables_and_lumps.rs` (6) | the sound names against the simulation's source list; one linked sound and the documented priorities; the step table as the exact integer floor of the genuine expression; both table digests; edge lumps padded as `getsfx` does; the load refusals | digests and goldens of the genuine tables; Rust-only for the refusals |
| `mixer_goldens.rs` (5) | the rows of the 23 scenario goldens and the 64 session goldens of the genuine mixer except the fake device's `d` and `q` rows; the refused scenarios stop where the genuine code ends in `I_Error`; the short-lump witnesses and the pitch-256 probe are refused | source equivalence against genuine traces |
| `adjust_streams.rs` (1) | `S_AdjustSoundParams` on all 404 positional start rows of the 64 genuine streams | source equivalence |
| `logic_rules.rs` (10) | channel allocation and priority and the draw rule as the genuine runs observed them, pitches from the drawn values, a centred origin on the listener, every refusal leaving state unchanged (including a sector or button start without a placement and a NULL origin with one), one lump lookup per sound, updates freeing finished and inaudible channels, level start and removal, the mixer refusing in the C's order (`the_mixer_refuses_in_the_c_order`), and a volume outside 0..=127 refused (`adjust_sound_params_refuses_a_volume_outside_the_range`) | genuine observations for the first two; Rust-only for the rest |
| `stream_replay.rs` (1) | the sound logic replayed on the 64 genuine streams, as far as each holds the positions it needs | source equivalence |
| `session_rules.rs` (7) | a session is a fresh process; records run at `gametic` t and the update at t+1; a removal's stop frees the channel within the tic; a refused record and a refused update each leave the session equal to its clone; a start of another actor without a placement, and of the listener's own body with one, is refused (`an_unplaced_start_of_another_actor_is_refused`); a tic's draw count of 65,535 is exact and 65,536 is refused (`the_draw_count_is_exact_or_refused`) | Rust-only (the C facts cited) |
| `detector_rules.rs` (4) | the T12 detector fires on an audibility flip at the clipping edge and on a separation-only change, does not fire on SNDBTN2's busy-slot release, and labels a session from the tic and continues | Rust-only, built from cited C facts |
| `session_gate.rs` (1) | **the gate:** every genuine row of the 64 items, end to end from the Rust wire-25 simulation | source equivalence |
| `state_restore.rs` (6) | saving and restoring at several tics of every item continues exactly as the unbroken run; forged states are each refused; restore checks the simulation; restore refuses a configuration other than the caller's (map 8, volume 0, 64 channels, textures 0); a linked sound saved before its frame is mixed restores; an encode of an out-of-range value never decodes as another (300 once decoded as 44) | Rust-only (consistency of the saved state; the continued rows are also equal to the genuine streams) |
| unit tests in `src/` (2) | the schedule rule, and the mixer's volume arithmetic far outside the game's range (a fuzz finding) | Rust-only |

`cargo test -p doom-sim --release --test wire25_projection` and the wire-25 unit tests above are also part of
`cargo test --workspace`.

#### What the sound gates do not compare

The gates above say "every genuine row" loosely; this is the exact list of what they leave out (the module documentation of
`session_gate.rs` and `state_restore.rs` repeats it). None of these is a known difference; each is a point where the
comparison is weaker than the words would suggest.

- **In the genuine streams:** the `#` header rows, the `W` rows (the wipe's draws and melt steps; the wipe's 320 draws are
  still checked through the next `R` row's index) and the `Z` rows (the totals). Every `S`, `T`, `K`, `C`, `L` and `R` row is
  compared.
- **The three listener columns (x, y, angle) of the starts with no placement** (a NULL origin or the listener's own body):
  381 of 785 `S` rows of the 64 items; a wire-25 record carries the listener only for a positional start, and the C does not
  read it for these starts (s_sound.c:304-324).
- **In the mixer session goldens:** the fake device's `d` and `q` rows. In the scenario goldens the three short-lump
  witnesses and the pitch-256 probe compare only the stop position and counts: the two buffers mixed before each short-lump
  refusal are not checked by digest.
- **Four items run only to the length of their genuine stream,** which is shorter than their demo: BOSS1 ends at the level
  exit (the next tic is checked to be refused as `LevelExited`); M5VPLAN, M5OPEN and M5SSEG (49, 40 and 15 tics of 82) end
  where the genuine process ends at a renderer `I_Error`, and nothing after that is checked.
- **Frames** (`m5c_frames.rs`): 45,540 in all; 45,170 compared on the whole-frame and face-zeroed digests, 300 on the masked
  digest only (their face-zeroed digests are `undefined` in the goldens), 70 not comparable (switch parity unknown; the
  status row and counters are still compared). `m5c_restore.rs` compares the same way over its seven items (70 M5SWTCH frames
  on the status row only).
- **The app's sink stream:** the required tests count the buffers handed to the sink (28,082, a number the Rust run
  produced) and the doom-sound gate compares the same `Sound::tic` buffers against the genuine mixer; the comparison of the
  app's own buffers with the genuine mixer, buffer by buffer, is in `tools/reference/m5c_session.py` (the non-required CI job,
  and the local real runs), not in `cargo test`.
- **The records of wire 25 against the genuine engine** are compared in the non-required job too
  (`m5c_wire25_check.py`, `m5c_t4.py`); in required CI the placements are checked against the genuine C only indirectly,
  through the doom-sound gate's full `S` rows.

### The audio probe

```sh
doom-play --audio-probe        # a diagnostic: opens a playback device, writes about one second of silence, never plays a sound
```

Exit 0 if a device opened (it prints the backend and device), 1 if none did (it prints every backend tried and each
one's error), 2 if mixed with any other argument. It is the only code that can open a device, and CI never runs it (no
runner has an audio device). On the development machine without the system's PulseAudio client library (`libpulse0`) it
exits 1; that is the expected and loud outcome there. With the library (`sudo apt install libpulse0` under WSLg) it opened
PulseAudio's "RDP Sink" and wrote about one second of silence; the owner reported game sound under WSLg and on Windows on 2026-11-18 ([record](validation/M5C.md#the-owners-playtest-2026-11-18)). Windows: `doom-play.exe
--audio-probe` (the probe itself was not reported as run). The unit tests in `crates/doom-play/src/audio.rs` (the backend list names only
real backends; an error names every backend tried) run in `cargo test -p doom-play` without opening a device.

### M5c fuzz targets of doom-sound

Five targets in the fuzz workspace (four for `doom-sound` and one for `DDSP` version 2, `render_display_state_v2`, below), each with a deterministic seed tool, no IWAD and no id data:

```sh
python3 tools/seed_sound_fuzz.py lumps DIR     # sound_lumps (max_len 4096): SoundSet::load against an independent model of I_InitSound's loop and getsfx
python3 tools/seed_sound_fuzz.py mixer DIR     # sound_mixer (max_len 2050): refusal order and mixing
python3 tools/seed_sound_fuzz.py logic DIR     # sound_logic (max_len 1060): the sound logic's invariants
python3 tools/seed_sound_fuzz.py state DIR     # sound_state (max_len 2048): the DSND decoder and the session
python3 tools/seed_sound_fuzz.py --check [DIR_LUMPS DIR_MIXER DIR_LOGIC DIR_STATE]   # writes nothing; with directories, requires them to hold exactly the seeds the tool writes
python3 tools/seed_sound_fuzz.py --self-test   # shows --check fails on a flipped byte, a missing file and an extra file
# build one target at a time, into a CARGO_TARGET_DIR on ext4 and with debug info off:
cargo +nightly fuzz build --fuzz-dir fuzz --target-dir S/target-asan --sanitizer address sound_mixer   # also --sanitizer none into S/target-none; BIN below is the built target binary
# seed replay (not a campaign); ASAN_OPTIONS=detect_leaks=0 means LeakSanitizer is disabled, not passed
ASAN_OPTIONS=detect_leaks=0 BIN SEEDS -runs=0 -max_len=2050 -timeout=20 -rss_limit_mb=2048
# bounded coverage-guided campaign on a fresh copy of the seeds
BIN CORPUS -max_total_time=300 -max_len=2050 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The first six commands above are the seed tool's; the build, replay and campaign lines show the form with one
target (`sound_mixer`); the others differ in the name and the `max_len`. The state target also has a second mode that
runs a session on arbitrary records; since the review it restores with the session's own configuration and requires any other to
be refused with `ConfigMismatch`. What was run and found is in the [evidence record](validation/M5C.md#step-11-part-1-the-fuzz-targets).

**`render_display_state_v2`** (the fifth target; `DDSP` version 2, which the existing `render_display_state` target does not
read; that target and its seeds are unchanged). Its input starts with a byte that picks one of three parts: arbitrary bytes
through `DisplayState::decode_any` (version 1 must equal the frozen decode, version 2 must be the version-1 decode with the
contract byte, accepted blocks re-encode to the same bytes); `Display::restore` into a fresh display (a refusal leaves the state
equal, a success gives the state back and a second restore agrees; the wipe-flag rule is an invariant here); and
`FrameSource::restore` of a display and a `DSND` together over the committed SNDBUSY item, with five genuine saves, edited or
raw candidates, any accepted candidate re-saving to identical bytes and running one more tic. Version 2 seeds are written from
the documented layout (version 1 bytes with the version changed and one contract byte inserted), never from the Rust encoder.

```sh
python3 tools/seed_display_state_v2_fuzz.py DIR        # writes the 379 seeds (max_len 70000)
python3 tools/seed_display_state_v2_fuzz.py --check [DIR]   # writes nothing; with a directory, requires exactly the seeds the tool writes
python3 tools/seed_display_state_v2_fuzz.py --self-test     # shows --check fails on a flipped byte, a missing file and an extra file
cargo +nightly fuzz build --fuzz-dir fuzz --target-dir S/target-asan --sanitizer address render_display_state_v2   # also --sanitizer none into S/target-none
ASAN_OPTIONS=detect_leaks=0 BIN SEEDS -runs=0 -max_len=70000 -timeout=60 -rss_limit_mb=2048                       # seed replay, leak detection disabled
BIN CORPUS -max_total_time=300 -max_len=70000 -seed=1 -timeout=60 -rss_limit_mb=2048                              # coverage-guided campaign on a fresh copy of the seeds
```

### The CI job and the mutation workflow

The Linux-only job `M5c genuine sound` (`m5c-genuine-sound` in `.github/workflows/rust.yml`; it keeps
the documentation-only fast path; it is not yet a required check, and its name lost the suffix "(not required)" in M5d so that
the owner can make it one) runs: the generator and contract checks (`m5c_sfx.py`,
`m5c_check_sfx.py` and its self-test, `m5c_wad.py`, `m5c_build.py`, `m5c_mixer_build.py` and `m5c_sounds_table.py`
each with `--check`, and the table's `--self-test`); `m5c_starts.py --selftest` and each of the 64 committed start lists
against a fresh conversion of its stream; `m5c_reference.py --check --selftest --jobs 4` and
`m5c_mixer_reference.py --check --selftest --jobs 4` (the address variants run with leak detection disabled, named in
the step names); a release build of `player_level_run` and `m5c_rust_check.py`; a release build of `sound_records_run`,
`m5c_wire25_check.py --check --selftest --jobs 4`, `m5c_t4.py --check` and `m5c_session.py --check --selftest --jobs 4`
(the scripted app sessions; leak detection disabled in its address runs). It runs no IWAD tool, no `--strict32` and no
device command. The Rust tests of `doom-sound` run in the existing Stable job on all three operating systems and in the
Minimum Rust job as part of `--workspace`. The [mutation workflow](#ci-mutation-workflow-m4b) now also mutates
`doom-sound` (`-p doom-sound` beside `doom-sim`, `doom-render` and `doom-game`; `doom-play`, the window and device code,
is left out), over **24 shards dealt round-robin** (`--sharding round-robin`: mutant i runs on shard i mod 24; the
consecutive slices of the first M5c run left two shards cut off by the 120-minute job limit while eleven finished in 4 to 23
minutes). The runs' figures, the survivors' dispositions with their independent check, the second round of survivors and the confirming run
at `937cf9d` (whose only missed mutants are the 14 judged equivalent or unreachable) are in the
[evidence record](validation/M5C.md#step-10-mutation-testing); the tables are not repeated here. The job failed at the T4 check at `216f976` for two uncommitted
generated files, fixed in `a6f3b3b` ([record](validation/M5C.md#the-ci-record)).

### M5c timings

Recorded, never gated; run alone on an idle machine (check `uptime` and `ps -eo pcpu,comm --sort=-pcpu | head -5`
first), built and run from ext4 (`CARGO_TARGET_DIR=~/scratch/m5c/step12/target`), never from `/mnt/*`, with the
repository's release profile (set no `CARGO_PROFILE_RELEASE_*` variable). No IWAD, no audio device and no sound. The
[M5c record](validation/M5C.md#step-12-timings) owns the results.

```sh
# per room and screenblocks: the sound session's tic, Mixer::mix_buffer, the frame step with sound and the same room
# under the M5b contract (count, total, mean, median, p95, max in microseconds, and a workload line); run three times
cargo run -p doom-game --release --example m5c_bench -- [--rooms SNDBUSY,SNDMONST,SNDBTN2,SLIDEB,M5AMP,M5STDIG] [--screenblocks 10,11] [--warmup 1]
```

## M5d kill tests and trial options

Observations are in the [M5d record](validation/M5D.md); this section is the commands only.

### The kill test (Unix; runs in the Linux and macOS jobs of the Stable workflow)

```sh
cargo test --release -p doom-play kill_tests::sigkill_at_random_points_leaves_valid_demos -- --nocapture   # 50 kills
DOOM_PLAY_KILLS=2000 DOOM_PLAY_KILL_SEED=1000 TMPDIR=~/scratch/kills cargo test --release -p doom-play \
  kill_tests::sigkill_at_random_points_leaves_valid_demos -- --nocapture
DOOM_PLAY_KILL_AT_TIC=1021 DOOM_PLAY_KILLS=200 cargo test --release -p doom-play kill_tests::sigkill -- --nocapture
```

The test spawns its own executable (the ignored `kill_tests::child_session`, which `cargo test` does not run by itself),
plays the real `App` with the real `WriterHost` (the writer thread `doom-play` runs; `FsHost` underneath) on the synthetic session set,
SIGKILLs it at a random moment and checks every surviving `.lmp`: it parses, replays with no refusal, its digest matches what the
child printed at its tic count, and its tic count lies between **W** (the tics the file held after the writer's last completed write,
printed by the child on each `@tic` line, `-` before the claim completed) and **M** (the tics the session had run); exactly M once a
later session's claim completed; a missing file only before its claim completed; and the session running at the kill has lost at
most the **lag bound** (M minus the tics in its file; default 525 tics, three times the writer's queue bound of 175). Environment
variables, all optional (the module comment in `crates/doom-play/src/kill_tests.rs` is the full list):
`DOOM_PLAY_KILLS` (default 50), `DOOM_PLAY_KILL_SEED` (first seed, default 1), `DOOM_PLAY_KILL_MAX_MS` (default 400) and
`DOOM_PLAY_KILL_LONG_MS` (default 1600) for the delays, `DOOM_PLAY_KILL_AT_TIC` with `DOOM_PLAY_KILL_AT_US` (default
3000) to aim at a tic (1021 is the first page-boundary rewrite, 1050 the first periodic one),
`DOOM_PLAY_KILL_WADS` (newline-separated WAD paths; local use only, never in CI) with `DOOM_PLAY_KILL_MAP`, and
`DOOM_PLAY_KILL_KEEP` (a directory that keeps each kill's files), `DOOM_PLAY_KILL_DIR` (a directory, for example on another disk:
the kills' directories `kNNNN/` are made there and kept, nothing is deleted, and an existing `kNNNN/` fails the run, so give a new,
empty directory each time; it takes precedence over `DOOM_PLAY_KILL_KEEP`; filter on the test name, as in the example below, so the
self-tests, which use the system temporary directory, do not run there) and `DOOM_PLAY_KILL_MAX_LOST` (the lag bound in tics, or
`none` to lift it; the summary line says which applied; give it for a slow disk such as `/mnt/d`, where the lag was observed up to
hundreds of tics). `DOOM_PLAY_KILL_CHILD_DIR` is set by the parent for
the child; do not set it. `DOOM_PLAY_KILL_PLANT` is for the self-tests only (`torn`: the child's appends are two writes 1 ms
apart; `early`: every file operation is 5 ms slow and the printed W is 3 too high, capped at M - 1; `stall`: the writer stops for good
before the first file operation that would leave the file holding more than 200 tics; any other value makes the child panic); do not set it for a real run. The parent requires that the child died by
`SIGKILL` and reports the tail of its standard error when it did not. The test writes under the system temporary directory (`TMPDIR`); a real disk there makes more
kills land before the session file's first write than tmpfs does. The one summary line (seen with `--nocapture`) gives the files and
tics checked, where the kills landed, the distribution of tics lost by the session running at each kill, the kills that left no
`.lmp`, the leftover `.tmp` files, the aim and the lag bound applied. The child runs far faster than 35 tics a second, so the lag it
shows is worse than play would give. On a disk outside the system temporary directory:

```sh
DOOM_PLAY_KILL_DIR=/mnt/d/new-empty-dir DOOM_PLAY_KILL_MAX_LOST=none DOOM_PLAY_KILLS=300 cargo test --release -p doom-play sigkill_at_random -- --nocapture
```

Three self-tests run by default in `cargo test -p doom-play`, each planting a fault in the child and requiring the kill test to fail
at its first failing kill: `kill_tests::a_torn_append_fails_the_kill_test` (the torn append; "is not a valid demo"),
`kill_tests::a_w_printed_too_high_fails_the_kill_test` (the `early` plant; the W rule) and
`kill_tests::a_stopped_writer_fails_the_lag_bound` (the `stall` plant; the lag bound). The other new tests are unit tests in
`crates/doom-play/src/session_file.rs` (one five-byte write call, a rewrite that replaces the file, the claimed name,
a failed removal of the claim's temporary; the clean-up of leftover temporary files), `crates/doom-play/src/session_writer_tests.rs` and
`closing_tests.rs` (the writer thread's order, bound, errors and drain, and the settle at a session's end, with a real `App` over a test
disk), `crates/doom-game/tests/app_settle.rs` (`App::settle_save`) and `crates/doom-game/tests/app_retry.rs` (`cargo test -p doom-game --test app_retry`:
the retry cadence after a failed write, `FAILED_WRITE_RETRY_TICS`); `window.rs` has a test of the window configuration's DPI
flag.

### The cursor-left detector's replay test (local only)

The detector that infers on Windows that the cursor left the window (`crates/doom-play/src/cursor_leave.rs`) has unit tests that run
by default, and one **ignored** test that replays `--input-log` files and prints, per file, the reports and the pumps of left
presses without a release. It reads the owner's logs, which never enter the repository, so it runs locally only:

```sh
DOOM_PLAY_INPUT_LOGS=/path/one.txt:/path/two.txt cargo test -q -p doom-play replay_local_input_logs -- --ignored --nocapture
```

### The session check on the survivors (`tools/reference/m5d_kill_check.py`)

```sh
python3 tools/reference/m5d_kill_check.py --work ~/scratch/m5d-kills --kills 200 --seed 100 --jobs 2 --selftest   # synthetic
python3 tools/reference/m5d_kill_check.py --work ~/scratch/m5d-kills --iwad /path/to/DOOM.WAD --kills 50 --selftest   # local only
```

It builds the `doom-play` test executable in release, runs the kill test with the files kept, and puts every surviving
`.lmp` through the M5c session check (the genuine engine in its strict, overflow and address variants and the genuine
mixer, against the Rust replay; the address runs have leak detection disabled). Options: `--work DIR` (required, outside
the repository), `--kills K` (default 200), `--seed S`, `--iwad PATH` (local only: plays E1M1 of a scratch copy; the
original is never touched and nothing from it is committed), `--jobs N` (demos checked at once, default 2), `--selftest`
(plants four faults that must each fail: three malformed demos, refused by the header check, and one changed command byte that passes the header and must fail the genuine comparison), `--keep`, `--cc`, `--cargo`. Output: one VERDICT line per demo and a
summary; counts and verdicts only. The synthetic run is not yet a CI step.

### The lost-display check (`tools/local/m5d_lost_display.py`, local only)

```sh
python3 tools/local/m5d_lost_display.py --iwad /path/to/DOOM.WAD --work ~/scratch/m5d-display --mode display --runs 20
```

Runs the real `doom-play` windowed (an X server is needed, WSLg works) with `--no-sound` and ends it without its own quit:
`--mode display` shuts an X proxy (the lost display connection), `--mode int` sends SIGINT, `--mode kill` SIGKILL. Options:
`--runs N` (default 20), `--min-ms` and `--max-ms` (the delay after the session file appears, default 500 to 8000),
`--seed`, `--jobs`, `--cc`, `--cargo`. Every surviving `.lmp` must PASS the session check. It opens a window and reads the
IWAD, so it never runs in CI.

### The sound trial options and exit lines (`doom-play`)

```sh
doom-play --audio-probe                                   # silence for about 1 s; prints the device lines (see the audio probe above)
doom-play --iwad DOOM.WAD --experimental-audio-start-buffers 1 --experimental-audio-period-frames 256
```

`--experimental-audio-start-buffers N` takes 1 or 2 (default 2) and `--experimental-audio-period-frames N` takes 128, 256
or 512 (default 128 since 2026-11-21; it was 512 until then); any other value exits 2 with a message. They change only how mixed sound reaches the device, and do
nothing with `--no-sound`. With a device open, four lines (`audio device`, `audio settings`, `audio callbacks`, `audio
queue`) are printed on exit, and since 2026-11-21 a fifth (`audio underruns`) after them when at least one underrun was counted; a setting away from its default is marked `EXPERIMENTAL` on the `audio settings` line (so a period of 256 or 512 is marked, 128 is not;
`--audio-probe` asks for the default period, 128). The
unit tests for the lines and the options run in `cargo test -p doom-play` and open no device. What the options are
expected to change is arithmetic in the [M5d record](validation/M5D.md#b-the-trial-options); the owner's Windows runs are in the record's
[A7](validation/M5D.md#a7-the-owners-windows-session-of-2026-11-20-and-the-second-round) and [A9](validation/M5D.md#a9-the-owners-retest-of-2026-11-20-his-rulings-and-the-close).

### Local mutation run on `doom-play` (CI's workflow does not mutate it)

```sh
git diff master...HEAD -- crates/doom-play > ~/scratch/mutants/diff.patch
TMPDIR=~/scratch/mutants/tmp CARGO_PROFILE_DEV_DEBUG=0 \
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=~/scratch/mutants/runner.sh \
  cargo mutants -p doom-play --in-diff ~/scratch/mutants/diff.patch -j 2 --timeout 300 --build-timeout 900 \
  --exclude crates/doom-play/src/kill_tests.rs -o ~/scratch/mutants/out
```

`runner.sh` is the memory-cap wrapper described under [mutation testing](#mutation-testing) (`ulimit -v 4000000`, then `exec timeout 600 "$@"`).
Run it from a clean checkout on the Linux disk with `TMPDIR` on ext4, and **do not set `CARGO_TARGET_DIR` with more than one job**: the jobs
would share one target directory and a mutant could be tested with another mutant's binary. `kill_tests.rs` is excluded because it is a
test harness. The mutants in the window's event handlers and in `audio::probe` cannot be caught without a display or a real audio device;
the dispositions are in the [M5d record](validation/M5D.md#the-local-mutation-run-on-doom-play).

## Mutation testing

M4a used cargo-mutants 27.1.0 with cargo-nextest 0.9.151. These are separate
developer tools, not production or MSRV dependencies. Run one campaign at a
time, keep build/output directories outside the repository, and check free
space first (`df -h /mnt/c` on this WSL host; retain at least 20 GB). Historical
scope, outcomes, equivalence proofs and limitations are in the
[M4a evidence record](validation/M4A.md#mutation-testing). Counts depend on the
selected source revision; a listed mutation is not evidence that it was tested.

Create the diff before switching to a scratch copy of the source:

```sh
git diff --no-ext-diff master...HEAD -- crates/doom-sim > /tmp/m4a-mutants.patch
```

Run 1 selects that diff in two content shards. Set the output and temporary
paths to distinct scratch locations and use the same optimization settings for
both commands:

```sh
export CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=0
export M4A_MUTANTS_SCRATCH="$HOME/scratch/doom-m4a-mutants"
export TMPDIR="$M4A_MUTANTS_SCRATCH/tmp"
unset CARGO_TARGET_DIR
mkdir -p "$TMPDIR"
cargo mutants -p doom-sim --test-tool nextest --jobs 3 \
  --timeout 300 --build-timeout 900 --output "$M4A_MUTANTS_SCRATCH/behaviour" \
  --in-diff /tmp/m4a-mutants.patch \
  --exclude-re '^crates/doom-sim/src/(diagnostic|snapshot)\.rs:'
cargo mutants -p doom-sim --test-tool nextest --jobs 3 \
  --timeout 300 --build-timeout 900 --output "$M4A_MUTANTS_SCRATCH/reader" \
  --in-diff /tmp/m4a-mutants.patch \
  --re '^crates/doom-sim/src/(diagnostic|snapshot)\.rs:'
```

Choose concurrency for the available memory. The owner lifted M4a's temporary
job/thread caps on 2026-11-08; earlier runs retain the settings reported in the
evidence record. `CARGO_BUILD_JOBS` controls builds and `NEXTEST_TEST_THREADS`
controls tests within each mutant job.

For a targeted rerun, copy the full description from `missed.txt`, regex-escape
it and anchor both ends. Use one `--re` per mutation and omit `--in-diff` so a
later source diff does not silently omit the target. Limit files explicitly;
the recorded 27.1.0 runs also included struct-field deletions outside their
name filters. Check the actual listed and executed selections.

```sh
cargo mutants -p doom-sim --test-tool nextest --jobs 1 \
  --timeout 1800 --build-timeout 900 --output "$M4A_MUTANTS_SCRATCH/rerun" \
  -f crates/doom-sim/src/mover_rules.rs \
  --re '^crates/doom-sim/src/mover_rules\.rs:786:12: delete ! in reachable$'
```

A timeout alone is unresolved: rerun it individually with a generous timeout
and explain the loop from the code before counting nontermination as caught.
For survivors, add a test with source- or rule-derived expectations or give a
proof of equivalence. Identify internal refusal-witness tests explicitly;
they do not establish reachability through valid public operations.


The run-2 rejection/rollback selection used these Bash arguments (historical
source positions can change; list and inspect the selection before running):

```bash
RUN2_ARGS=(
 --re '^crates/doom-sim/src/episode1\.rs:'
 --re '^crates/doom-sim/src/mover_rules\.rs:.*[ :](validate_episode1|validate_movers_with|donut_geometry|stair_bounds|donut_triples|reachable|widen|later_episode_switch|switch_match|line_switch_match)( |$)'
 --re '^crates/doom-sim/src/movers\.rs:.*[ :](vertical_door|do_door|do_donut|do_plat|do_floor|build_stairs|plat_raise|move_floor|episode1)( |$)'
 --re '^crates/doom-sim/src/lib\.rs:.*[ :]advance( |$)'
 --re '^crates/doom-sim/src/player_commands\.rs:.*[ :](preflight_player_commands|max_command_move)( |$)'
 --re '^crates/doom-sim/src/original_player\.rs:.*[ :]preflight_player_body( |$)'
 --re '^crates/doom-sim/src/player_map_scale\.rs:.*[ :](derive_episode1|spawn_admitted|live_projectile_bound|actor_slots)( |$)'
 --re '^crates/doom-sim/src/original_registry\.rs:.*[ :](with_original_registry|require_role_for|is_policy_monster|has_episode1)( |$)'
 --re '^crates/doom-sim/src/player_monsters\.rs:.*[ :]original_player_episode1_map( |$)'
 --re '^crates/doom-sim/src/store\.rs:.*[ :](claim|reserve)( |$)'
 --re '^crates/doom-sim/src/spawning\.rs:.*[ :](validate_entry|reserve_actor_spawns)( |$)'
 --re '^crates/doom-sim/src/world_pickups\.rs:.*[ :](bonus_bound|pickup_bound|world_item_bound)( |$)'
 --re '^crates/doom-sim/src/demon_expanded_map\.rs:.*[ :](preflight_expanded_demons_for|validate_sector_specials_for)( |$)'
)
RUN2_ARGS+=( --exclude-re '^crates/doom-sim/src/lifecycle\.rs:' )
cargo mutants -p doom-sim --test-tool nextest --jobs 2 \
  --timeout 300 --build-timeout 900 \
  --output "$M4A_MUTANTS_SCRATCH/refusals" "${RUN2_ARGS[@]}"
```

### CI mutation workflow (M4b)

The [`Mutants` workflow](../.github/workflows/mutants.yml) runs cargo-mutants
27.1.0 with cargo-nextest 0.9.151 (the M4a versions, installed pinned) over the
diff of `crates/` against `master`, `-p doom-sim -p doom-render -p doom-sound -p doom-game --test-tool nextest --in-diff`
(`-p doom-game` since M5b, `-p doom-sound` since M5c),
in matrix shards (`--shard k/N`; N is 8 for the M4b and M5a runs and, since M5b, one value set in the
`Mutants plan` job, 24 since M5c, with `--sharding round-robin`). It is informational and not a required
check. Start it by adding the `run-mutants` label to the pull request (the
trigger that works before merge; a push does not re-run it, so remove and
re-add the label for a new head), or with `workflow_dispatch` from the Actions
tab once the file is on `master`. Each shard uploads `outcomes.json` and the
`missed`, `caught`, `timeout` and `unviable` lists as artifacts
`mutants-shard-k`; the `Mutants summary` job prints combined counts and the
missed and timed-out lists in the run's step summary. A shard fails only for a
usage error or a failing baseline; missed mutants and timeouts are findings,
triaged as above (the local procedure and the M4a rules for timeouts and
equivalent mutants apply unchanged). Since M5a the run covers the `doom-render` lines of the diff too
(`-p doom-render`), the crate whose diff M5a adds.

Since M5b each shard caps its test processes: the step writes a runner script and exports it as
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER`, so cargo starts every test executable (not `rustc`) through it,
and passes `--cargo-test-arg=--test-threads=2` to cargo-mutants:

```sh
cat > "$RUNNER_TEMP/capped-runner.sh" <<'EOS'
#!/bin/bash
ulimit -v 1500000        # per process, in KiB: about 1.5 GB of address space
exec timeout 330 "$@"       # a backstop above cargo-mutants' --timeout (300 s), see below
EOS
chmod +x "$RUNNER_TEMP/capped-runner.sh"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$RUNNER_TEMP/capped-runner.sh"
```

The memory cap is what protects the runner: a mutant that loops and allocates aborts its test process (reported as
caught) instead of exhausting the runner's memory before any timeout fires. The time limit is only a backstop for a
test that outlives cargo-mutants, and it must stay above cargo-mutants' own `--timeout` (300 s): at 240 s (the
first value, used in the second and third runs) the wrapper killed a looping test before cargo-mutants' timeout, so a
mutant that makes a loop endless was reported as caught, not as a timeout. The cap is per process, so the bound
is jobs x test threads x cap (2 x 2 x 1.5 GB); keep that product under the runner's memory, and note that a 3 GB cap with default parallelism
did not protect a 7 GB host locally. Use the same runner and `--test-threads` when re-running a shard locally.

#### Re-running one shard or the survivors locally (M5a)

A shard that the hosted runner killed (its job log ends "The runner has received a shutdown signal")
is re-run from the same patch and shard number, at the head the workflow tested, in a scratch worktree
on the Linux disk (never `/mnt/*`), with `TMPDIR` in scratch (cargo-mutants copies the tree there for each
job) and **no** `CARGO_TARGET_DIR`: a shared target directory makes concurrent jobs build into one
directory, and outcomes come out wrong (M5a, [evidence](validation/M5A.md#mutation-testing)):

```sh
git worktree add --detach ~/scratch/<milestone>/wt <head-sha>
git diff --no-ext-diff origin/master...<head-sha> -- crates/ > ~/scratch/<milestone>/mutants.patch
cd ~/scratch/<milestone>/wt
unset CARGO_TARGET_DIR
export CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=~/scratch/<milestone>/tmp
mkdir -p "$TMPDIR"
cargo mutants -p doom-sim -p doom-render --test-tool nextest --jobs 2 --timeout 300 --build-timeout 900 \
  --in-diff ~/scratch/<milestone>/mutants.patch --shard 0/8 --output ~/scratch/<milestone>/out
```

Run it detached and wait on its PID with a timeout. To check the new tests against the survivors, list
every `missed.txt` and `timeout.txt` line, give each a `--re '^<regex-escaped line>$'` (no `--in-diff`, so a
later source diff cannot drop a target) and run `cargo mutants -p doom-render --test-tool nextest` with
those arguments from a scratch worktree of the branch head, with the same `TMPDIR` and no `CARGO_TARGET_DIR`.
First run the same command with `--list`: cargo-mutants 27.1.0 also matches some struct-field deletions
outside the name filters (two expressions matched six mutants in M5a), so report which mutants ran. A
mutant that runs a loop forever (`+=` to `*=` on its counter) ends as `TIMEOUT`, which counts as killed
once the loop is explained.

## Whole-level simulation-only timing

The M4a H12 developer caller `player_level_advance` uses the same public setup
and demo commands as `player_level_run`, with no per-tic capture, hashing or
output. Build first, then stop other builds/tests/fuzzing and verify the machine
is idle before measuring. Keep the target directory and local results in scratch:

```sh
cargo build -p doom-sim --examples --release --locked
python3 -I tools/reference/check_player_level_advance.py \
  --binary target/release/examples/player_level_advance
target/release/examples/player_level_advance --wad "$IWAD" --map E1M1 \
  --demo tests/reference/e1m1_scripts/WALK.lmp --policy episode1
```

`IWAD` is a local read-only path; never upload its bytes or captures. With a
scratch `CARGO_TARGET_DIR`, use that directory's `release/examples` path instead.
The check uses six DSIM38 and thirty DSIM39 synthetic cases. Successful call
counts, refusal boundaries and final stripped hashes come from frozen genuine-C
projections (the pre-refusal fixtures supply the two H7 prefixes), not Rust output.
It checks final states, not the complete per-tic sequences covered by the existing
comparators. No existing fixture changes.

The JSON reports `advance_ns`, the sum around successful `Simulation::advance`
calls, and `admitted`, their denominator. `refused_ns` is separate; the refused
call is excluded from the successful-tic mean. `loop_ns` also includes input
assembly, timer/counter bookkeeping and a possible refused call. Setup, parsing,
spawning and the final snapshot/hash/output are outside both intervals. The
caller reports the floor and median of 1,001 empty clock pairs in
`clock_floor_ns`/`clock_median_ns`; overhead is not subtracted. Simulation runs
without capture work between tics, so its cache behaviour can differ from the
full capture pipeline. Report both boundaries rather than calling process wall
time simulation-only. The final FNV and comparator-stripped SHA-256 permit
checking that observation changes did not change state. No timing threshold gates CI.

## Advance benchmarks (M4p)

`tools/bench_advance.py` (standard library only, run with `python3 -I`) has two modes. Both use the release
examples; build them into a target directory on the Linux disk (ext4, for example `~/scratch`), never on
`/mnt/*`, and use the pinned toolchain for instruction counts:

```sh
CARGO_TARGET_DIR=~/scratch/m4p/target cargo +1.85.0 build -p doom-sim --examples --release --locked
```

**`ir`** (CI-safe, no IWAD, needs `valgrind`). Runs nine fixed workloads under
`valgrind --tool=callgrind --toggle-collect` on `Simulation::advance` and compares them with
`tests/reference/m4p_ir_baseline.json`: instructions inside `advance` outside libc and ld.so, plus the
`malloc`, `realloc` and `memcpy` call counts (total `advance` instructions are printed for information).
A final-state digest or outcome that differs from the baseline also fails, so a behaviour change is caught. A gated
count that falls to 0 from a nonzero baseline fails too (a renamed glibc symbol would otherwise stop the guard).
It refuses examples built by a different rustc than the baseline's, also when rewriting the baseline
(`--allow-toolchain-mismatch` overrides). These are instruction counts, not time.

```sh
export CARGO_TARGET_DIR=~/scratch/m4p/target
python3 -I tools/bench_advance.py ir --check                  # fail above the threshold (default 3%)
python3 -I tools/bench_advance.py ir --update-baseline        # rewrite the baseline after a reviewed change
python3 -I tools/bench_advance.py ir --only TFRAG1_wire24     # one workload; --jobs N, --keep, --json-out PATH
```

CI runs `ir --check` in the Linux job **Simulation instruction counts** (`simulation-ir`): it installs the
runner's `valgrind` package and Rust 1.85.0, builds the release examples with `cargo +1.85.0`, and uploads the
measured counts as the `simulation-ir` artifact. It is not a required check unless the owner makes it one. After a
reviewed change that lowers the counts, rewrite the baseline from a pinned build (`ir --update-baseline` refuses a
build by another rustc) and commit it with the change. A failure of this check can also come from code generation alone: a
change in code the case never runs can shift how rustc splits the crate into code-generation units and add `memcpy` calls
(M5c, [record](validation/M5C.md#the-instruction-count-check)); bisect the case by commit before changing the baseline.

**`timing`** (local only, needs an IWAD copy in scratch). Times `player_level_advance --policy episode1v2`
over the 61 real-map cases of `tests/reference/m4b_real_results.json` (DEMO1 is extracted from the IWAD copy
and patched as M4b recorded, then checked against the recorded digest), in rotated order, behind an idle gate
(no stray build/test/fuzz/Valgrind process, a quiet CPU, 1-minute load below a limit). Every final digest and
admitted-tic count must equal the recorded one, or the exit status is non-zero. It refuses a binary, IWAD or
output directory under `/mnt/*` or on a non-ext4 filesystem unless `--allow-slow-fs` is given, which marks the
result as not performance evidence. Output: `bench_timing.json` and `bench_timing.md` (counts, microseconds
per tic, digests) in `--out-dir`.

```sh
python3 -I tools/bench_advance.py timing --binary ~/scratch/m4p/target/release/examples/player_level_advance \
  --iwad ~/scratch/m4p/iwad/DOOM.WAD --out-dir ~/scratch/m4p/harness --reps 5
```

Run timings alone on an idle machine. The recorded numbers are in [M4P validation](validation/M4P.md).
