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
target/release/examples/player_level_run --wad /mnt/d/DoomIWADs/DOOM.WAD --map E1M5 --demo /mnt/d/scratch/m4b/M5IDLE.lmp --out /mnt/d/scratch/m4b/rust --captures --policy episode1v2
python3 tools/reference/level_compare_m4b.py /mnt/d/scratch/m4b/real/runs/strict_E1M5_M5IDLE /mnt/d/scratch/m4b/rust --wad /mnt/d/DoomIWADs/DOOM.WAD --demo /mnt/d/scratch/m4b/M5IDLE.lmp --json OUT
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
diff of `crates/` against `master`, `-p doom-sim --test-tool nextest --in-diff`,
in 8 matrix shards (`--shard k/8`). It is informational and not a required
check. Start it by adding the `run-mutants` label to the pull request (the
trigger that works before merge; a push does not re-run it, so remove and
re-add the label for a new head), or with `workflow_dispatch` from the Actions
tab once the file is on `master`. Each shard uploads `outcomes.json` and the
`missed`, `caught`, `timeout` and `unviable` lists as artifacts
`mutants-shard-k`; the `Mutants summary` job prints combined counts and the
missed and timed-out lists in the run's step summary. A shard fails only for a
usage error or a failing baseline; missed mutants and timeouts are findings,
triaged as above (the local procedure and the M4a rules for timeouts and
equivalent mutants apply unchanged).

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
