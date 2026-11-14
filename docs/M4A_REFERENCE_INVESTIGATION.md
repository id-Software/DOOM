# M4a reference investigation: Episode 1, map by map, against the genuine engine

Phase 1 note for milestone M4a, the first of the Episode 1 milestones that follow the E1M1
work ([M3h9c note](M3H9C_REFERENCE_INVESTIGATION.md)): the milestone that takes the
whole-engine harness of M3h9c from one real map to the rest of Episode 1 (E1M2–E1M9 of the
registered `DOOM.WAD`) on medium skill, single player. It records what Episode 1 needs
beyond the wire-22 policy (a census of counts only), the original C facts for every
mechanism the census names (function, file and line), the Rust wire-22 inventory the new
policy must extend, the command domain, the original demos, the synthetic-WAD CI path, the
proposed order of work and policy, the new owner decisions H1–H12 with recommendations, a
size estimate with the unattended-rule verdict, and the implementation plan.

Source authority is `linuxdoom-1.10/` at the repository's upstream commit; line numbers are
of those files, and `crates/` and `tools/` line numbers are of master at the start of this
milestone (`0ecfe4b`, the merge of PR #56). Milestone status lives in [ROADMAP](ROADMAP.md),
support in [COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md),
commands in [TESTING](TESTING.md), the asset rules in [ASSETS](ASSETS.md) and the earlier
census tables in [validation/M3H9_IWAD_COUNTS.md](validation/M3H9_IWAD_COUNTS.md) (see the
[documentation authority map](README.md#documentation-authority)). The observations of this
milestone will go to `docs/validation/M4A.md`, created in Phase 2. This note owns the source
facts, the design, the decisions and the plan; it never contains an IWAD-derived coordinate,
angle, lump byte or per-object datum: only counts, special numbers, doomednums with their
`info.c` names, tic counts and digests.

## 1. Starting point and the strategy change

Master contains M3h9c (`PlayerMonstersLevelV1`, wire 22, DSIM38, context `PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1`, the whole-engine harness: the genuine
linuxdoom-1.10 engine compiled headless with only I/O replaced, an independent DSIM encoder, and a Rust runner, all fed from one demo-format input; PR
#56, merged 2026-11-05). The owner confirmed G1–G13 on 2026-11-05. Real E1M1 on medium matches the genuine engine tic for tic on all five local
scripts; the evidence is in [validation/M3H9C.md](validation/M3H9C.md).

**The strategy change.** Earlier mover, monster and weapon milestones (M3h2–M3h9b) proved each mechanism with bespoke C scenarios, one small mechanism
at a time. With the harness in place that is no longer the efficient route to Episode 1, so M4a changes the evidence strategy (the rules below are the
brief's, restated; the decisions that carry them are [H1–H12](#10-new-owner-decisions-h1h12-with-recommendations)):

1. **Primary evidence is the whole-engine harness on the real maps.** The genuine engine and Rust run the real `DOOM.WAD` maps from one input and are
   compared after every tic. The runs are local-only (the IWAD never enters Git, CI or logs); only counts, pass/fail and digests are committed.
2. **CI evidence comes from synthetic WADs through the same harness.** A generated WAD per mechanism group runs through the same genuine runner,
   encoder and Rust runner, with committed goldens, so CI covers every mechanic without an IWAD ([§7](#7-the-synthetic-wad-ci-path)).
3. **Bespoke C scenarios only for quirks the maps do not reach.** A mechanism the real maps never exercise (for example a blocked stair step) is still
   tested, but by a synthetic map or a witness, not by a free-standing scenario.
4. **Whole systems at once.** A door, floor or plat family is ported as the whole system the original has (every Episode 1 kind), not as the
   census-minimal subset.
5. **Every earlier rule stays.** Older policies and wires stay frozen byte for byte; every real-map divergence gets a synthetic reproduction (the
   witnesses rule of G9); the sanitizer builds are identical to the strict build apart from the declared adapters; independent review against the C
   source; matched timings against master; precise fuzz reporting (target compiled, seed replay, coverage-guided campaign and sanitizer campaign are
   distinct claims, and disabled leak detection is disclosed).

### 1.1 Phase 1 facts recorded here

**The census tooling.** Two commits on this branch added counts-only modes to `tools/iwad_counts.py`: `a70263f` (the episode census
`--episode-markdown` and the demo-header census `--demos-markdown`) and `1a12461` (the demo-lump command-domain counts). They parse raw lumps only,
run in about a second, and exit 0 with a one-line notice when `DOOM.WAD` is absent. The five older modes (`--things-markdown`, `--specials-markdown`,
`--movers-markdown`, `--spawn-rule`, `--things-budget`) produce byte-identical output to the committed tool at HEAD; every row of `--things-markdown`
appears verbatim in [validation/M3H9_IWAD_COUNTS.md](validation/M3H9_IWAD_COUNTS.md) (182 of 182), and every row of `--specials-markdown` and
`--movers-markdown` appears in [validation/M3H8_IWAD_COUNTS.md](validation/M3H8_IWAD_COUNTS.md) (70 of 70 each). The timing of the tool was not
recorded as a performance measurement (machine not idle).

**The 32-bit build (G3 closed).** M3h9c left open (G3) whether the five LP64 adapters of the genuine build change behavior relative to a true 32-bit
(ILP32) build. Commit `79a485d` added an opt-in `strict32` variant to `tools/reference/m3h9c_build.py` and an opt-in `--strict32` option to
`tools/local/e1m1_reference.py`: the genuine text, **no adapter**, compiled with `-m32` and the same flags as the strict build (`-std=gnu89 -O0
-fwrapv -fno-strict-aliasing -ffp-contract=off -fexcess-precision=standard -fsigned-char -DNORMALUNIX -DLINUX`), plus the per-file `-include stdint.h`
and the `--wrap` link flags. It is not part of `all`, not in the runner contract, and prints a skip line when the toolchain has no 32-bit support.
Result (host WSL2 x86-64, gcc 15.2.0, **not idle**: other agents were active and the runs were concurrent, so the timings are indicative only):

- The strict32 dumps are **byte-identical** to the LP64 strict dumps on all five real E1M1 scripts and all six synthetic scripts. The 34 whole source
  files in the strict32 build tree were compared with `cmp` against `linuxdoom-1.10/`: 34 identical.
- strict32 applies none of the 8 `ABI_ADAPTERS` entries (the "five adapters" of the M3h9c note are five sites; `r_data.c:482–485` is four entries) and
  none of the 6 sanitizer adapters. The binary is `ELF 32-bit LSB`; the driver's runtime check that `maptexture_t` `patchcount`/`patches` sit at
  offsets 20/22 passed on the unadapted `r_data.c`. Two warnings per variant, all in `p_setup.c` (631:36 and 633:35, `-Wformat-overflow`), identical
  in every variant; the driver and stubs compile warning-free on 32 bits with no change. One build-script fix was needed (the compiler-generated
  `__x86.get_pc_thunk.*` symbols are filtered from the stub check).
- The overflow and address variants were also byte-identical on every script.
- `m3h9c_build.py --check` passes ("34 whole files, 8 ABI + 6 sanitizer adapters, 6 extracted bodies, 43 function + 17 variable stubs"); the committed
  runner contract was not regenerated.

| Script (real E1M1) | Tics | End | End tic | `prndindex` end | Dump-set SHA-256 (LP64 strict = strict32) |
| --- | ---: | --- | ---: | ---: | --- |
| idle | 2100 | marker | 2100 | 253 | `8a0aa289fc32faec67e36ba002807ee995b34e216be3136c7ac5b5a93730700f` |
| WALK | 766 | exit | 765 | 127 | `97fe9afde14a474037c1db40b643cd3291d07cd55e2eeb1547fb7c8d585c7d85` |
| FIGHT | 1281 | marker | 1281 | 89 | `c52b502cceac29473e4c3dd9f6cd1306d38756ad38019fa01213322713533360` |
| BARRELS | 882 | marker | 882 | 187 | `9da617a77e780809a15bed6c31e8427b78b072fb92ea54ba140d5edb0658c1dc` |
| CHAOS | 5000 | marker | 5000 | 227 | `a490ce1a3305f6b315d4d15561bbb4b2ba7597c3fff81cf098b1e89fc48d2d83` |

At load every script has `prndindex` 144, 96 thinkers and 92 mobjs. The real-map run took 1 m 34.9 s wall time including the builds.

| Script (synthetic) | Tics | End | End tic | `prndindex` end | Dump-set SHA-256 (strict = strict32 = committed) |
| --- | ---: | --- | ---: | ---: | --- |
| SYNIDLE | 2100 | marker | 2100 | 5 | `c774d7f42c337096d368a0fb81e36ded493fc8ccfd375aee9d40382be15b04cb` |
| SYNWALK | 1468 | exit | 1467 | 102 | `7297a677c897da48abcc720ad1b3557d604e882923870f4c7c1d7c24c6d89bb5` |
| SYNFIGHT | 273 | marker | 273 | 164 | `165d85be5ea06c0c39ffeb093fd20392422e43e64dfe89bc0993746682903aac` |
| SYNBARRL | 581 | marker | 581 | 176 | `c94f21f685537bca3f19b833ff590c46a534d595d44e921732874192132e85c2` |
| SYNCHAOS | 5000 | marker | 5000 | 48 | `1accfcf39650fe1aaaa72cf8ebab39a7e6a7d9be18905b45262c878158e9834b` |
| SYNDIE | 1001 | reborn | 1000 | 156 | `66af9268e6366ef328269a5111e4b6b433318c54e2a0ff9bb876c2cad8a7fc36` |

The synthetic comparison used a scratch wrapper that imports `m3h9c_reference.py`'s own `run()` and compares every script's full result record with
the committed `tests/reference/m3h9c_synthetic_results.json` (all pass); that wrapper is not committed. The M3h9c evidence record holds per-tic DSIM
sequence and final digests, not the raw C dump-set digests, so the LP64 digests above cannot be compared with a recorded value; the tics, endings and
end tics agree with that record (idle 2,101 snapshots, walk_to_exit exit at 766 commands, fighting 1,282, barrels 883, chaos 5,001). **Limitations:**
the timing is not an idle-machine measurement; binary hashes depend on the build path; there is no `-m32` sanitizer build; the synthetic comparison is
not a committed tool option. Decision H11 records that strict32 stays a local, opt-in check, not a CI job.

## 2. Episode 1 census (medium skill)

Produced by `python3 tools/iwad_counts.py --episode-markdown` against the registered `DOOM.WAD` (read in place, never copied). Counts only: no
coordinate, angle or per-object datum. "Medium" means the record is handled by `P_SpawnMapThing` on medium skill (p_mobj.c:717–751): starts (types 1–4
and 11) on every skill, any other type only when `options & 2` is set and `options & 16` (multiplayer only) is clear. Names are the `info.c`
`mobjinfo` entries of the doomednum (info.h enum order). Table rows for the other IWAD facts are in the earlier census records linked above.

**Tooling and the wire-22 set.** The census modes are `episode_census`, `episode_markdown_report` and `demo_markdown_report` in `tools/iwad_counts.py`
(wired in `main` before the heavy `MapCounts` pass; constants `WIRE22_LINE_SPECIALS` and `WIRE22_THINGS` and the special-number groups, derived from
`P_CrossSpecialLine`/`P_ShootSpecialLine` and `P_UseSpecialLine` and cited in comments). The tool reuses `read_wad`, `map_slots`, `map_lumps`,
`records`, `mobj_table`, `thing_kind`, `LINE_KINDS`, `STAGE_A_SECTOR_SPECIALS` and the `MTF_*` constants; it passes `py_compile`, and the repository
has no Python lint or format convention to apply. Wire 22 admits line specials {0, 1, 11, 36, 38, 48, 63, 88, 109} and sector specials {0, 1, 2, 3, 4,
5, 7, 8, 9, 12, 13, 16, 17}.

### 2.1 (a) THING doomednums on medium

| doomednum | info.c name | class | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | player 1 start | player start | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| 2 | player 2 start | player start | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| 3 | player 3 start | player start | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| 4 | player 4 start | player start | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| 5 | MT_MISC4 | pickup | 0 | 0 | 1 | 1 | 1 | 1 | 1 | 0 | 1 |
| 6 | MT_MISC6 | pickup | 0 | 0 | 1 | 1 | 1 | 1 | 1 | 0 | 1 |
| 8 | MT_MISC24 | pickup | 0 | 2 | 1 | 1 | 1 | 3 | 1 | 0 | 1 |
| 9 | MT_SHOTGUY | monster | 0 | 0 | 19 | 9 | 24 | 35 | 19 | 6 | 21 |
| 10 | MT_MISC68 | corpse | 2 | 9 | 4 | 5 | 0 | 0 | 0 | 0 | 0 |
| 11 | deathmatch start | dm start | 5 | 10 | 9 | 9 | 8 | 11 | 10 | 9 | 10 |
| 12 | MT_MISC69 | corpse | 2 | 3 | 2 | 5 | 0 | 0 | 0 | 0 | 0 |
| 13 | MT_MISC5 | pickup | 0 | 1 | 0 | 0 | 0 | 1 | 1 | 0 | 1 |
| 14 | MT_TELEPORTMAN | decoration | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 1 | 1 |
| 15 | MT_MISC62 | corpse | 4 | 8 | 5 | 4 | 0 | 0 | 0 | 0 | 0 |
| 24 | MT_MISC71 | corpse | 7 | 5 | 4 | 7 | 0 | 0 | 0 | 0 | 0 |
| 34 | MT_MISC49 | decoration | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 7 |
| 35 | MT_MISC50 | obstacle | 2 | 0 | 0 | 0 | 2 | 2 | 1 | 0 | 1 |
| 46 | MT_MISC43 | obstacle | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 6 | 4 |
| 48 | MT_MISC48 | obstacle | 2 | 2 | 11 | 0 | 0 | 3 | 2 | 0 | 2 |
| 58 | MT_SHADOWS | monster | 0 | 0 | 0 | 0 | 9 | 9 | 1 | 0 | 5 |
| 2001 | MT_SHOTGUN | pickup | 1 | 1 | 1 | 0 | 1 | 1 | 2 | 2 | 1 |
| 2002 | MT_CHAINGUN | pickup | 0 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| 2003 | MT_MISC27 | pickup | 0 | 0 | 1 | 1 | 1 | 1 | 1 | 0 | 1 |
| 2005 | MT_MISC26 | pickup | 0 | 1 | 1 | 0 | 1 | 0 | 1 | 0 | 1 |
| 2007 | MT_CLIP | pickup | 2 | 12 | 10 | 11 | 2 | 9 | 5 | 0 | 4 |
| 2008 | MT_MISC22 | pickup | 2 | 13 | 17 | 4 | 16 | 26 | 12 | 0 | 1 |
| 2010 | MT_MISC18 | pickup | 0 | 0 | 0 | 1 | 3 | 2 | 2 | 0 | 0 |
| 2011 | MT_MISC10 | pickup | 1 | 6 | 14 | 9 | 14 | 21 | 10 | 11 | 6 |
| 2012 | MT_MISC11 | pickup | 3 | 11 | 15 | 5 | 12 | 22 | 10 | 3 | 6 |
| 2013 | MT_MISC12 | pickup | 0 | 1 | 2 | 1 | 1 | 2 | 1 | 1 | 0 |
| 2014 | MT_MISC2 | pickup | 12 | 21 | 48 | 30 | 11 | 39 | 33 | 0 | 16 |
| 2015 | MT_MISC3 | pickup | 25 | 20 | 44 | 14 | 14 | 54 | 48 | 0 | 27 |
| 2018 | MT_MISC0 | pickup | 1 | 3 | 3 | 1 | 2 | 3 | 2 | 1 | 1 |
| 2019 | MT_MISC1 | pickup | 1 | 0 | 1 | 1 | 2 | 1 | 1 | 0 | 0 |
| 2024 | MT_INS | pickup | 0 | 0 | 1 | 0 | 1 | 2 | 1 | 1 | 0 |
| 2025 | MT_MISC14 | pickup | 0 | 0 | 0 | 1 | 1 | 3 | 1 | 0 | 0 |
| 2026 | MT_MISC15 | pickup | 0 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| 2028 | MT_MISC31 | obstacle | 8 | 1 | 6 | 4 | 0 | 10 | 17 | 0 | 5 |
| 2035 | MT_BARREL | barrel | 6 | 24 | 28 | 32 | 28 | 22 | 16 | 28 | 20 |
| 2045 | MT_MISC16 | pickup | 0 | 0 | 0 | 0 | 1 | 1 | 0 | 0 | 0 |
| 2046 | MT_MISC19 | pickup | 0 | 0 | 3 | 2 | 1 | 4 | 2 | 0 | 2 |
| 2048 | MT_MISC17 | pickup | 1 | 12 | 5 | 3 | 2 | 7 | 5 | 11 | 6 |
| 2049 | MT_MISC23 | pickup | 3 | 1 | 5 | 1 | 2 | 8 | 3 | 4 | 2 |
| 3001 | MT_TROOP | monster | 2 | 8 | 29 | 20 | 26 | 43 | 30 | 5 | 30 |
| 3002 | MT_SERGEANT | monster | 0 | 0 | 2 | 3 | 4 | 9 | 3 | 14 | 11 |
| 3003 | MT_BRUISER | monster | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 |
| 3004 | MT_POSSESSED | monster | 4 | 33 | 24 | 22 | 28 | 22 | 31 | 0 | 5 |
| | **handled on medium** | | 100 | 213 | 322 | 213 | 227 | 384 | 280 | 111 | 205 |
| | **records in lump (all skills)** | | 143 | 262 | 380 | 254 | 293 | 463 | 358 | 126 | 237 |
| | **distinct doomednums on medium** | | 26 | 29 | 36 | 34 | 37 | 38 | 38 | 22 | 35 |

### 2.2 (b) LINEDEF specials (count of lines)

| special | kind, trigger | wire 22 | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | - | admitted | 464 | 991 | 974 | 789 | 752 | 1299 | 891 | 310 | 596 |
| 1 | manual door DR | admitted | 8 | 15 | 12 | 6 | 22 | 12 | 7 | 2 | 3 |
| 2 | door W1 | missing | 0 | 0 | 9 | 1 | 2 | 8 | 3 | 3 | 17 |
| 5 | floor W1 | missing | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| 7 | stairs S1 | missing | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 |
| 8 | stairs W1 | missing | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 |
| 9 | donut S1 | missing | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 11 | exit S1 | admitted | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 0 | 1 |
| 16 | door W1 | missing | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 |
| 18 | floor S1 | missing | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 |
| 20 | plat S1 | missing | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 1 |
| 22 | plat W1 | missing | 0 | 0 | 0 | 0 | 1 | 0 | 1 | 0 | 0 |
| 23 | floor S1 | missing | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 0 |
| 26 | manual door DR | missing | 0 | 0 | 2 | 2 | 3 | 3 | 2 | 0 | 0 |
| 27 | manual door DR | missing | 0 | 0 | 2 | 2 | 2 | 6 | 6 | 0 | 0 |
| 28 | manual door DR | missing | 0 | 2 | 0 | 0 | 0 | 4 | 2 | 0 | 0 |
| 31 | manual door D1 | missing | 0 | 2 | 4 | 2 | 6 | 0 | 3 | 4 | 2 |
| 32 | manual door D1 | missing | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |
| 33 | manual door D1 | missing | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 4 |
| 34 | manual door D1 | missing | 0 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 4 |
| 35 | light W1 | missing | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 0 |
| 36 | floor W1 | admitted | 1 | 0 | 0 | 2 | 0 | 1 | 0 | 0 | 1 |
| 38 | floor W1 | admitted | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 46 | door GR | missing | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 48 | scroll - | admitted | 8 | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 0 |
| 51 | exit S1 | missing | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 |
| 62 | plat SR | missing | 0 | 6 | 4 | 3 | 3 | 4 | 7 | 2 | 6 |
| 63 | door SR | admitted | 1 | 0 | 1 | 0 | 0 | 0 | 2 | 0 | 0 |
| 70 | floor SR | missing | 0 | 0 | 0 | 0 | 2 | 0 | 1 | 0 | 0 |
| 76 | door WR | missing | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 |
| 82 | floor WR | missing | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 |
| 86 | door WR | missing | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | 0 |
| 88 | plat WR | admitted | 1 | 11 | 7 | 3 | 5 | 8 | 12 | 0 | 11 |
| 90 | door WR | missing | 0 | 0 | 2 | 12 | 1 | 0 | 3 | 0 | 2 |
| 91 | floor WR | missing | 0 | 0 | 0 | 0 | 10 | 0 | 7 | 1 | 0 |
| 97 | teleport WR | missing | 0 | 0 | 0 | 0 | 10 | 0 | 0 | 8 | 2 |
| 98 | floor WR | missing | 0 | 0 | 0 | 0 | 3 | 0 | 1 | 0 | 0 |
| 103 | door S1 | missing | 0 | 3 | 1 | 0 | 2 | 3 | 1 | 0 | 1 |
| 109 | door W1 | admitted | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| | **linedefs** | | 486 | 1033 | 1026 | 830 | 825 | 1352 | 958 | 333 | 653 |

### 2.3 (c) SECTOR specials (count of sectors)

| special | wire 22 | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | admitted | 77 | 163 | 145 | 116 | 113 | 204 | 142 | 63 | 130 |
| 1 | admitted | 1 | 3 | 8 | 4 | 2 | 5 | 2 | 4 | 0 |
| 2 | admitted | 0 | 1 | 0 | 1 | 4 | 1 | 1 | 0 | 0 |
| 3 | admitted | 0 | 5 | 0 | 0 | 0 | 1 | 3 | 0 | 0 |
| 7 | admitted | 4 | 5 | 8 | 8 | 7 | 30 | 8 | 1 | 10 |
| 8 | admitted | 2 | 13 | 4 | 4 | 8 | 2 | 7 | 4 | 4 |
| 9 | admitted | 3 | 6 | 7 | 3 | 9 | 4 | 4 | 1 | 2 |
| 11 | missing | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 |
| 12 | admitted | 1 | 1 | 0 | 1 | 0 | 3 | 3 | 0 | 0 |
| 13 | admitted | 0 | 3 | 0 | 2 | 0 | 0 | 0 | 0 | 0 |
| 16 | admitted | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 1 |
| | **sectors** | 88 | 200 | 177 | 139 | 143 | 250 | 170 | 74 | 147 |
| | **sectors with a nonzero tag** | 5 | 13 | 28 | 12 | 12 | 20 | 16 | 17 | 17 |

### 2.4 (d) Tags

| count | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| linedefs with a nonzero tag | 5 | 22 | 30 | 30 | 39 | 27 | 39 | 17 | 41 |
| linedefs with a special and a nonzero tag | 5 | 22 | 30 | 28 | 39 | 27 | 39 | 17 | 41 |
| linedefs with a nonzero special and tag 0 | 17 | 20 | 22 | 13 | 34 | 26 | 28 | 6 | 16 |
| sectors with a nonzero tag | 5 | 13 | 28 | 12 | 12 | 20 | 16 | 17 | 17 |
| distinct nonzero line tags | 5 | 10 | 19 | 9 | 9 | 14 | 14 | 6 | 12 |
| distinct nonzero sector tags | 5 | 11 | 20 | 8 | 9 | 14 | 14 | 10 | 12 |
| distinct nonzero tags (lines and sectors) | 5 | 11 | 20 | 9 | 9 | 14 | 14 | 10 | 12 |
| line tags matching no sector tag | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |

### 2.5 (e) Wire 22 classification

Wire 22 admits: line specials {0, 1, 11, 36, 38, 48, 63, 88, 109}; sector specials {0, 1, 2, 3, 4, 5, 7, 8, 9, 12, 13, 16, 17}; THING doomednums:
player 1 start, plus 9 MT_SHOTGUY, 10 MT_MISC68, 12 MT_MISC69, 15 MT_MISC62, 24 MT_MISC71, 35 MT_MISC50, 48 MT_MISC48, 65 MT_CHAINGUY, 2001
MT_SHOTGUN, 2002 MT_CHAINGUN, 2007 MT_CLIP, 2008 MT_MISC22, 2011 MT_MISC10, 2012 MT_MISC11, 2014 MT_MISC2, 2015 MT_MISC3, 2018 MT_MISC0, 2019
MT_MISC1, 2028 MT_MISC31, 2035 MT_BARREL, 2048 MT_MISC17, 2049 MT_MISC23, 3001 MT_TROOP, 3002 MT_SERGEANT, 3004 MT_POSSESSED. Non-spawning on this
path (stored, not spawned): deathmatch starts 11 and player starts 2-4.

The wire-22 admitted thing set, line specials and sector specials are listed with file and line in [§4.1](#41-admitted-sets-under-wire-22); the census tool mirrors them (`WIRE22_THINGS`, `WIRE22_LINE_SPECIALS`).

| map | missing thing types | missing line specials | missing sector specials | non-spawning starts (records) |
| --- | --- | --- | --- | --- |
| E1M1 | 0: none | 0: none | 0: none | 2 x1, 3 x1, 4 x1, 11 x5 |
| E1M2 | 4: 8, 13, 2005, 2013 | 6: 9, 28, 31, 46, 62, 103 | 0: none | 2 x1, 3 x1, 4 x1, 11 x10 |
| E1M3 | 8: 5, 6, 8, 2003, 2005, 2013, 2024, 2046 | 11: 2, 8, 20, 26, 27, 31, 35, 51, 62, 90, 103 | 0: none | 2 x1, 3 x1, 4 x1, 11 x9 |
| E1M4 | 8: 5, 6, 8, 2003, 2010, 2013, 2025, 2046 | 9: 2, 5, 18, 26, 27, 31, 62, 86, 90 | 0: none | 2 x1, 3 x1, 4 x1, 11 x9 |
| E1M5 | 14: 5, 6, 8, 14, 58, 2003, 2005, 2010, 2013, 2024, 2025, 2026, 2045, 2046 | 12: 2, 22, 26, 27, 31, 62, 70, 90, 91, 97, 98, 103 | 0: none | 2 x1, 3 x1, 4 x1, 11 x8 |
| E1M6 | 13: 5, 6, 8, 13, 58, 2003, 2010, 2013, 2024, 2025, 2026, 2045, 2046 | 8: 2, 16, 26, 27, 28, 62, 76, 103 | 0: none | 2 x1, 3 x1, 4 x1, 11 x11 |
| E1M7 | 13: 5, 6, 8, 13, 58, 2003, 2005, 2010, 2013, 2024, 2025, 2026, 2046 | 14: 2, 22, 23, 26, 27, 28, 31, 34, 62, 70, 90, 91, 98, 103 | 0: none | 2 x1, 3 x1, 4 x1, 11 x10 |
| E1M8 | 6: 14, 46, 2013, 2024, 2026, 3003 | 8: 2, 7, 23, 31, 62, 82, 91, 97 | 1: 11 | 2 x1, 3 x1, 4 x1, 11 x9 |
| E1M9 | 11: 5, 6, 8, 13, 14, 34, 46, 58, 2003, 2005, 2046 | 10: 2, 20, 31, 32, 33, 34, 62, 90, 97, 103 | 0: none | 2 x1, 3 x1, 4 x1, 11 x10 |

Missing thing types with names and counts on medium:

| map | missing thing types (doomednum name x count) |
| --- | --- |
| E1M1 | none |
| E1M2 | 8 MT_MISC24 x2, 13 MT_MISC5 x1, 2005 MT_MISC26 x1, 2013 MT_MISC12 x1 |
| E1M3 | 5 MT_MISC4 x1, 6 MT_MISC6 x1, 8 MT_MISC24 x1, 2003 MT_MISC27 x1, 2005 MT_MISC26 x1, 2013 MT_MISC12 x2, 2024 MT_INS x1, 2046 MT_MISC19 x3 |
| E1M4 | 5 MT_MISC4 x1, 6 MT_MISC6 x1, 8 MT_MISC24 x1, 2003 MT_MISC27 x1, 2010 MT_MISC18 x1, 2013 MT_MISC12 x1, 2025 MT_MISC14 x1, 2046 MT_MISC19 x2 |
| E1M5 | 5 MT_MISC4 x1, 6 MT_MISC6 x1, 8 MT_MISC24 x1, 14 MT_TELEPORTMAN x1, 58 MT_SHADOWS x9, 2003 MT_MISC27 x1, 2005 MT_MISC26 x1, 2010 MT_MISC18 x3, 2013 MT_MISC12 x1, 2024 MT_INS x1, 2025 MT_MISC14 x1, 2026 MT_MISC15 x1, 2045 MT_MISC16 x1, 2046 MT_MISC19 x1 |
| E1M6 | 5 MT_MISC4 x1, 6 MT_MISC6 x1, 8 MT_MISC24 x3, 13 MT_MISC5 x1, 58 MT_SHADOWS x9, 2003 MT_MISC27 x1, 2010 MT_MISC18 x2, 2013 MT_MISC12 x2, 2024 MT_INS x2, 2025 MT_MISC14 x3, 2026 MT_MISC15 x1, 2045 MT_MISC16 x1, 2046 MT_MISC19 x4 |
| E1M7 | 5 MT_MISC4 x1, 6 MT_MISC6 x1, 8 MT_MISC24 x1, 13 MT_MISC5 x1, 58 MT_SHADOWS x1, 2003 MT_MISC27 x1, 2005 MT_MISC26 x1, 2010 MT_MISC18 x2, 2013 MT_MISC12 x1, 2024 MT_INS x1, 2025 MT_MISC14 x1, 2026 MT_MISC15 x1, 2046 MT_MISC19 x2 |
| E1M8 | 14 MT_TELEPORTMAN x1, 46 MT_MISC43 x6, 2013 MT_MISC12 x1, 2024 MT_INS x1, 2026 MT_MISC15 x1, 3003 MT_BRUISER x2 |
| E1M9 | 5 MT_MISC4 x1, 6 MT_MISC6 x1, 8 MT_MISC24 x1, 13 MT_MISC5 x1, 14 MT_TELEPORTMAN x1, 34 MT_MISC49 x7, 46 MT_MISC43 x4, 58 MT_SHADOWS x5, 2003 MT_MISC27 x1, 2005 MT_MISC26 x1, 2046 MT_MISC19 x2 |

Missing line specials with line counts:

| map | missing line specials (special x lines) |
| --- | --- |
| E1M1 | none |
| E1M2 | 9 x1, 28 x2, 31 x2, 46 x1, 62 x6, 103 x3 |
| E1M3 | 2 x9, 8 x1, 20 x2, 26 x2, 27 x2, 31 x4, 35 x3, 51 x1, 62 x4, 90 x2, 103 x1 |
| E1M4 | 2 x1, 5 x1, 18 x2, 26 x2, 27 x2, 31 x2, 62 x3, 86 x4, 90 x12 |
| E1M5 | 2 x2, 22 x1, 26 x3, 27 x2, 31 x6, 62 x3, 70 x2, 90 x1, 91 x10, 97 x10, 98 x3, 103 x2 |
| E1M6 | 2 x8, 16 x1, 26 x3, 27 x6, 28 x4, 62 x4, 76 x2, 103 x3 |
| E1M7 | 2 x3, 22 x1, 23 x1, 26 x2, 27 x6, 28 x2, 31 x3, 34 x3, 62 x7, 70 x1, 90 x3, 91 x7, 98 x1, 103 x1 |
| E1M8 | 2 x3, 7 x1, 23 x1, 31 x4, 62 x2, 82 x1, 91 x1, 97 x8 |
| E1M9 | 2 x17, 20 x1, 31 x2, 32 x2, 33 x4, 34 x4, 62 x6, 90 x2, 97 x2, 103 x1 |

Missing sector specials: only E1M8 (special 11, one sector); every other map has none.

Unknown doomednums (would be `I_Error` in P_SpawnMapThing, p_mobj.c:758-761) after the filter: none

### 2.6 (f) Feature groups

Line groups are counts of lines; thing groups are counts of medium records.

| group | members | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| line: locked doors | 26, 27, 28, 32, 33, 34, 99, 133, 134, 135, 136, 137 | 0 | 2 | 4 | 4 | 5 | 13 | 13 | 0 | 10 |
| line: doors, all (manual and tagged) | 1, 2, 3, 4, 16, 26, 27, 28, 29, 31, 32, 33, 34, 42, 46, 50, 61, 63, 75, 76, 86, 90, 99, 103, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 133, 134, 135, 136, 137 | 10 | 23 | 33 | 29 | 38 | 39 | 32 | 9 | 35 |
| line: lifts and platforms (plat, plat stop) | 10, 14, 15, 20, 21, 22, 47, 53, 54, 62, 66, 67, 68, 87, 88, 89, 95, 120, 121, 122, 123 | 1 | 17 | 13 | 6 | 9 | 12 | 20 | 2 | 18 |
| line: floor movers | 5, 18, 19, 23, 24, 30, 36, 37, 38, 40, 45, 55, 56, 58, 59, 60, 64, 65, 69, 70, 71, 82, 83, 84, 91, 92, 93, 94, 96, 98, 101, 102, 119, 128, 129, 130, 131, 132, 140 | 2 | 0 | 0 | 5 | 15 | 1 | 10 | 3 | 1 |
| line: crusher ceilings | 6, 25, 44, 49, 72, 73, 77, 141 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| line: other ceiling movers | 40, 41, 43 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| line: crusher stops | 57, 74 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| line: stairs | 7, 8, 100, 127 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 1 | 0 |
| line: donut | 9 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| line: light line specials | 12, 13, 17, 35, 79, 80, 81, 104, 138, 139 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 0 |
| line: teleport lines | 39, 97, 125, 126 | 0 | 0 | 0 | 0 | 10 | 0 | 0 | 8 | 2 |
| line: exit lines | 11, 51, 52, 124 | 1 | 1 | 2 | 1 | 1 | 1 | 1 | 0 | 1 |
| line: scroller 48 | 48 | 8 | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 0 |

| thing group | members | E1M1 | E1M2 | E1M3 | E1M4 | E1M5 | E1M6 | E1M7 | E1M8 | E1M9 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| keys | 5 MT_MISC4, 6 MT_MISC6, 13 MT_MISC5, 38 MT_MISC8, 39 MT_MISC7, 40 MT_MISC9 | 0 | 1 | 2 | 2 | 2 | 3 | 3 | 0 | 3 |
| teleport destination | 14 MT_TELEPORTMAN | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 1 | 1 |
| weapons | 2001 MT_SHOTGUN, 2002 MT_CHAINGUN, 2003 MT_MISC27, 2004 MT_MISC28, 2005 MT_MISC26, 2006 MT_MISC25, 82 MT_SUPERSHOTGUN | 1 | 3 | 4 | 2 | 4 | 3 | 5 | 3 | 4 |
| powerups | 2013 MT_MISC12, 2022 MT_INV, 2023 MT_MISC13, 2024 MT_INS, 2025 MT_MISC14, 2026 MT_MISC15, 2045 MT_MISC16, 83 MT_MEGA | 0 | 1 | 3 | 2 | 5 | 9 | 4 | 3 | 0 |
| admitted monsters | 3004 MT_POSSESSED, 9 MT_SHOTGUY, 65 MT_CHAINGUY, 3001 MT_TROOP, 3002 MT_SERGEANT | 6 | 41 | 74 | 54 | 82 | 109 | 83 | 25 | 67 |
| monsters beyond the admitted set | 7 MT_SPIDER, 16 MT_CYBORG, 58 MT_SHADOWS, 64 MT_VILE, 66 MT_UNDEAD, 67 MT_FATSO, 68 MT_BABY, 69 MT_KNIGHT, 71 MT_PAIN, 72 MT_KEEN, 84 MT_WOLFSS, 3003 MT_BRUISER, 3005 MT_HEAD, 3006 MT_SKULL | 0 | 0 | 0 | 0 | 9 | 9 | 1 | 2 | 5 |
| pickups beyond the admitted set | 5 MT_MISC4, 6 MT_MISC6, 8 MT_MISC24, 13 MT_MISC5, 17 MT_MISC21, 38 MT_MISC8, 39 MT_MISC7, 40 MT_MISC9, 82 MT_SUPERSHOTGUN, 83 MT_MEGA, 2003 MT_MISC27, 2004 MT_MISC28, 2005 MT_MISC26, 2006 MT_MISC25, 2010 MT_MISC18, 2013 MT_MISC12, 2022 MT_INV, 2023 MT_MISC13, 2024 MT_INS, 2025 MT_MISC14, 2026 MT_MISC15, 2045 MT_MISC16, 2046 MT_MISC19, 2047 MT_MISC20 | 0 | 5 | 11 | 9 | 14 | 22 | 14 | 3 | 8 |
| barrel | 2035 MT_BARREL | 6 | 24 | 28 | 32 | 28 | 22 | 16 | 28 | 20 |

### 2.7 Reading the census

- **E1M1 is fully admitted** (0 missing thing types, line specials or sector specials); every other map needs more. No doomednum is unknown to
  `info.c` on any E1 map and no Doom II type appears, so `P_LoadThings` would not abort on any of them.
- **Union of line specials needed beyond wire 22: 30 types** (2, 5, 7, 8, 9, 16, 18, 20, 22, 23, 26, 27, 28, 31, 32, 33, 34, 35, 46, 51, 62, 70, 76,
  82, 86, 90, 91, 97, 98, 103; with the teleport 97 split off, the 29 of the [order of work](#8-the-order-of-work)). The commonest are 31 (D1 door),
  62 (SR lift), 2 (W1 door) and 90 (WR door). Special 46 occurs once (E1M2) and 51 once (E1M3).
- **Sector special 11** (end-level damage) occurs only on E1M8, one sector. Specials 10 and 14 do not occur on Episode 1.
- **E1M8 has no exit line** (11, 51, 52 and 124 are all absent); it ends on the death of both barons (3003 `MT_BRUISER` x2) through `A_BossDeath` (tag
  666) plus the special-11 sector ([§3.4](#34-new-monsters-spectre-and-baron-of-hell)).
- **Monsters beyond the admitted set** are only 58 `MT_SHADOWS` (the Spectre: E1M5 x9, E1M6 x9, E1M7 x1, E1M9 x5) and 3003 `MT_BRUISER` (E1M8 x2).
- **Absent on medium on every map:** the Lost Soul, the Cacodemon, the Cyberdemon, the Spider Mastermind, the plasma rifle, the BFG, cells as pickups,
  berserk and invulnerability, skull keys, hanging decorations, the super shotgun and megasphere; and every crusher, ceiling mover, crusher stop and
  platform stop line (the table (f) rows are all zero). These mechanics are out of scope for the Episode 1 milestones.
- **Teleportation** is confined to E1M5 (10 lines), E1M8 (8) and E1M9 (2), with the landing thing 14 once on each of those maps; **stairs** to E1M3
  (special 8) and E1M8 (special 7); the **donut** (special 9) to E1M2; the **light trigger** (35) to E1M3; **locked doors** appear from E1M2.
- **Tags:** every line tag matches a sector tag except one on E1M4 (the "line tags matching no sector tag" row), which a load rule must account for.
- The medium counts of starts treat player starts 2–4 and deathmatch starts 11 as records handled on every skill; they are stored and never spawned on
  the wire-22 path.

### 2.8 Geometry, bounds and mover rules: what refuses each map today

Beyond the specials and things above, each map must also pass the wire-22 load admission ([§4.9](#49-load-admission-what-refuses-e1m2e1m9-today) lists
the constants): the fixed caps, the M3h6 arithmetic certificate, the M3h6 boundary rules and the mover rules. The table below is built from the
committed M3h6, M3h8 and M3h9 census records and the Rust constants (medium skill; "✗" means over the wire-22 bound).

Sources: the committed M3H9, M3H8 and M3H6 IWAD count records ("Spawned" counts the THINGs that spawn an actor, excluding the starts).

| Map | Spawned | Monsters (>8) | Pickups (>64) | Barrels (>16) | Deco (≤32, list) | Geometry (M3h6) | Line specials outside set | Other mover rules failing | Sector specials |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| E1M2 | 199 | 41 ✗ | 106 ✗ | 24 ✗ | 28 (types?) | **(ii) FAIL** | 9 28 31 46 62 103 (+1 88) | — | 3,13 ok |
| E1M3 | 309 | 74 ✗ | 175 ✗ | 28 ✗ | 32 (types?) | **(vii) FAIL, boundary C1/C2 FAIL**; max height 528 (>512 load check too) | 2 8 20 26 27 31 35 51 62 90 103 | rule 4 reach > ±512; 1 sky mover | 16 ok |
| E1M4 | 200 | 54 ✗ | 89 ✗ | 32 ✗ | 25 | pass | 2 5 18 26 27 31 62 86 90 | 1 multi-family sector | ok |
| E1M5 | 215 | 91 ✗ | 93 ✗ | 28 ✗ | 3 | pass | 2 22 26 27 31 62 70 90 91 97 98 103 | spechit window 14 > 8 | ok |
| E1M6 | 369 | 118 ✗ | 214 ✗ | 22 ✗ | 15 | pass | 2 16 26 27 28 62 76 103 | 2 sky movers | ok |
| E1M7 | 266 | 84 ✗ | 146 ✗ | 16 ok | 20 | pass | 2 22 23 26 27 28 31 34 62 70 90 91 98 103 | — (scroller two-sided ok) | ok |
| E1M8 | 98 | 27 ✗ | 36 ok | 28 ✗ | 7 | **(i)+(ii) FAIL** | 2 7 23 31 62 82 91 97 | — | **11** ✗ |
| E1M9 | 191 | 72 ✗ | 79 ✗ | 20 ✗ | 20 | pass | 2 20 31 32 33 34 62 90 97 103 | spechit window 9 > 8; 4 sky movers | ok |

Union of E1 line specials needed beyond wire 22: **2, 5, 7, 8, 9, 16, 18, 20, 22, 23, 26, 27, 28, 31, 32, 33, 34, 35, 46, 51, 62, 70, 76, 82, 86, 90,
91, 97, 98, 103** (30 types; no crushers/ceilings). Sector special beyond: **11** (E1M8 only; 10 and 14 do not occur in E1).

Actor slots: E1M6 medium spawns 369 THINGs (+ body, + missiles, puffs, blood, drops, fog) against 384 slots — refused in practice; even E1M3 (309)
leaves 75. A 1,024-slot store is the existing hard cap (spawning.rs:50). The live-actor proof in e1m1_rules.rs:58–118 (`1 + K + P + 26k + …`) has to
be re-derived with K ≈ 120, P ≈ 220, B ≈ 32, D ≈ 32 and per-type missile bounds; at those values `26k` per Imp alone exceeds 1,024, so the "store
never fills" proof becomes a runtime witness (refuse the tic) instead.

Geometry certificate failures on E1M2, E1M3, E1M8 are arithmetic-overflow proofs (sight `P_InterceptVector2`, hitscan `P_InterceptVector`, heights)
and need an owner decision (re-prove with wider intermediates mirroring C, or exclude those maps).

## 3. Original C facts

Every citation is `function (file:lines)` against `linuxdoom-1.10/` at the behavioral-authority commit; "draw" means one `P_Random ()` call
(m_random.c:57–61). `M_Random` is a separate index used only by sound pitch, the status bar and the intermission's animated background
(wi_stuff.c:523, 525, 561). Where an earlier note already records a fact, this note links to it and adds only what is new for Episode 1. Subsection
numbers 3.N follow the topic order of the source investigation; the mechanics Episode 1 does not use are summarised in one sentence each.

Already recorded and not re-derived here:

| Topic | Authoritative earlier record |
| --- | --- |
| `spechit`, `P_TryMove` crossing loop, `P_Move` failure branch, `P_UseLines`/`PTR_UseTraverse`, `P_UseSpecialLine` prologue | [M3h8 §2.2–2.3](M3H8_REFERENCE_INVESTIGATION.md) |
| `vldoor_t`, `EV_VerticalDoor` (line 1), `EV_DoDoor` (normal, blazeOpen), `T_VerticalDoor` normal/blazeOpen paths | M3h8 §2.4 |
| `plat_t`, `EV_DoPlat(downWaitUpStay)`, `T_PlatRaise`, `activeplats` | M3h8 §2.5 |
| `EV_DoFloor` lowerFloorToLowest/turboLower, `T_MoveFloor` | M3h8 §2.6 |
| `T_MovePlane`, `P_ChangeSector`, `PIT_ChangeSector`, `P_ThingHeightClip` | M3h8 §2.7 |
| `P_ChangeSwitchTexture`, `P_StartButton`, buttons, exit 11 | M3h8 §2.8 |
| Missile lifecycle (`P_SpawnMissile`, `P_CheckMissileSpawn`, `P_ExplodeMissile`, `PIT_CheckThing` missile branch, `P_XYMovement`/`P_ZMovement` for missiles) | [M3h2](M3H2_REFERENCE_INVESTIGATION.md) "Missile lifecycle" |
| `A_Explode`, `P_RadiusAttack`, `PIT_RadiusAttack`, `P_DamageMobj`, `P_KillMobj` | [M3h9b §2.2–2.5](M3H9B_REFERENCE_INVESTIGATION.md) |
| `P_SpawnMapThing`, spawn draws, decorations of E1M1 | [M3h9 §2.1, §2.5](M3H9_REFERENCE_INVESTIGATION.md) |
| `-playdemo`, `G_DoPlayDemo`, `G_InitNew`, `G_DoLoadLevel`, `P_SetupLevel`, `G_Ticker`, `P_Ticker`, `P_PlayerThink` table, exit and death | M3h9c §2.1–2.8 |
| Light thinkers and their draws | [M3h7 §2.4](M3H7_REFERENCE_INVESTIGATION.md) |
| Intercepts, `MAXINTERCEPTS` | [M3h6 §2.2, §4.4](M3H6_REFERENCE_INVESTIGATION.md) |

### 3.0 What Episode 1 adds (census, counts only)

Sources: `python3 tools/iwad_counts.py --specials-markdown`, `--movers-markdown`, `--things-markdown`, and the per-slot census `--things E1M2` …
`--things E1M9` (the same read-only count report documented in docs/validation/M3H9_IWAD_COUNTS.md for E1M1; it prints doomednums, info.c facts and
counts only). "Handled" counts are the records `P_SpawnMapThing` spawns on medium in single player (skill bit, `MTF_NOTSINGLE` applied).

#### 3.0.1 Line specials that E1M2–E1M9 use and wire 22 lacks

Wire 22 admits {1, 11, 36, 38, 48, 63, 88, 109}. E1M2–E1M9 add **30 line specials**:

| Special | Trigger | Handler (dispatch site) | Slots (count) |
| --- | --- | --- | --- |
| 2 | W1 | `EV_DoDoor(open)` (p_spec.c:543–547) | E1M3 9, E1M4 1, E1M5 2, E1M6 8, E1M7 3, E1M8 3, E1M9 17 |
| 5 | W1 | `EV_DoFloor(raiseFloor)` (p_spec.c:561–565) | E1M4 1 |
| 7 | S1 | `EV_BuildStairs(build8)` + switch (p_switch.c:347–351) | E1M8 1 |
| 8 | W1 | `EV_BuildStairs(build8)` (p_spec.c:573–577) | E1M3 1 |
| 9 | S1 | `EV_DoDonut` + switch (p_switch.c:353–357) | E1M2 1 |
| 16 | W1 | `EV_DoDoor(close30ThenOpen)` (p_spec.c:597–601) | E1M6 1 |
| 18 | S1 | `EV_DoFloor(raiseFloorToNearest)` + switch (p_switch.c:377–381) | E1M4 2 |
| 20 | S1 | `EV_DoPlat(raiseToNearestAndChange,0)` + switch (p_switch.c:383–387) | E1M3 2, E1M9 1 |
| 22 | W1 | `EV_DoPlat(raiseToNearestAndChange,0)` (p_spec.c:615–619) | E1M5 1, E1M7 1 |
| 23 | S1 | `EV_DoFloor(lowerFloorToLowest)` + switch (p_switch.c:395–399) | E1M7 1, E1M8 1 |
| 26, 27, 28 | DR (blue, yellow, red lock) | `EV_VerticalDoor` (p_switch.c:326–338) | 26: E1M3 2, E1M4 2, E1M5 3, E1M6 3, E1M7 2; 27: E1M3 2, E1M4 2, E1M5 2, E1M6 6, E1M7 6; 28: E1M2 2, E1M6 4, E1M7 2 |
| 31 | D1 | `EV_VerticalDoor` → `open` (p_switch.c:331, p_doors.c:477–483) | E1M2 2, E1M3 4, E1M4 2, E1M5 6, E1M7 3, E1M8 4, E1M9 2 |
| 32, 33, 34 | D1 (blue, red, yellow lock) | `EV_VerticalDoor` → `open` | 32: E1M9 2; 33: E1M9 4; 34: E1M7 3, E1M9 4 |
| 35 | W1 | `EV_LightTurnOn(line,35)` (p_spec.c:634–638) | E1M3 3 |
| 46 | GR (shot) | `EV_DoDoor(open)` + `P_ChangeSwitchTexture(line,1)` (p_spec.c:988–992) | E1M2 1 |
| 51 | S1 | `P_ChangeSwitchTexture(line,0); G_SecretExitLevel ()` (p_switch.c:431–435) | E1M3 1 |
| 62 | SR | `EV_DoPlat(downWaitUpStay,1)` + switch (p_switch.c:544–548) | E1M2 6, E1M3 4, E1M4 3, E1M5 3, E1M6 4, E1M7 7, E1M8 2, E1M9 6 |
| 70 | SR | `EV_DoFloor(turboLower)` + switch (p_switch.c:592–596) | E1M5 2, E1M7 1 |
| 76 | WR | `EV_DoDoor(close30ThenOpen)` (p_spec.c:807–810) | E1M6 2 |
| 82 | WR | `EV_DoFloor(lowerFloorToLowest)` (p_spec.c:832–835) | E1M8 1 |
| 86 | WR | `EV_DoDoor(open)` (p_spec.c:847–850) | E1M4 4 |
| 90 | WR | `EV_DoDoor(normal)` (p_spec.c:867–870) | E1M3 2, E1M4 12, E1M5 1, E1M7 3, E1M9 2 |
| 91 | WR | `EV_DoFloor(raiseFloor)` (p_spec.c:872–875) | E1M5 10, E1M7 7, E1M8 1 |
| 97 | WR, monsters too | `EV_Teleport (line, side, thing)` (p_spec.c:904–907) | E1M5 10, E1M8 8, E1M9 2 |
| 98 | WR | `EV_DoFloor(turboLower)` (p_spec.c:909–912) | E1M5 3, E1M7 1 |
| 103 | S1 | `EV_DoDoor(open)` + switch (p_switch.c:455–459) | E1M2 3, E1M3 1, E1M5 2, E1M6 3, E1M7 1, E1M9 1 |

The admitted 1, 11, 36, 48, 63, 88 also recur (11 ends every map but E1M8; 48 recurs only on E1M7, four two-sided scrollers). **Absent from Episode 1
entirely**: every ceiling and crusher special (6, 25, 40, 41, 43, 44, 49, 72, 73, 77, 141), platform stop/perpetual (53, 54, 87, 89),
`EV_DoLockedDoor` (99, 133–137), `EV_StartLightStrobing` (17), `EV_TurnTagLightsOff` (104), lowerAndChange (37, 84), raiseToTexture (30, 96), the
crushing floor (55, 56, 65, 94), the monster-only teleports (125, 126), exit 52 and secret exit 124. So `EV_DoCeiling`, `T_MoveCeiling`,
`activeceilings`, `EV_CeilingCrushStop`, `EV_StopPlat`, `P_ActivateInStasis*` and the monster-only teleport rule are **not needed** for Episode 1
(recorded briefly in §3.1.9 for completeness).

#### 3.0.2 Sector specials

E1M2–E1M9 use {1, 2, 3, 7, 8, 9, 11, 12, 13, 16}. All but **11** (E1M8, one sector) are admitted. 10 (door close in 30 s), 14 (door raise in 5 min),
4, 5 and 17 do not occur in Episode 1.

**Absent on medium on every Episode 1 map (out of scope):** the Lost Soul, the Cacodemon, the Cyberdemon and the Spider Mastermind, the plasma rifle,
the BFG and cells as pickups, invulnerability, berserk (only as a multiplayer record, `MTF_NOTSINGLE`, p_mobj.c:740–741), skull keys, hanging
decorations and every `MF_SPAWNCEILING` type. The medium totals over E1M2–E1M9 and the per-map counts are in the census
([§3.2](#2-episode-1-census-medium-skill)). Lost Soul and Cacodemon mechanics (`A_SkullAttack`, `A_HeadAttack`, the skull slam and bounce, the floater
branches) and the Cyberdemon and Mastermind `A_BossDeath` cases are not recorded here.

#### 3.0.4 Mover load facts (from `--movers-markdown`)

Plat sectors per map ≤ 9 (E1M3), reusable switch lines ≤ 10 (E1M7), two-sided scrollers ≤ 4 (E1M7), one-sided manual doors 0, tag-0 tag-using lines 0,
self-referencing door lines 0. **Facts that break current rules**: multi-family sectors 1 on E1M4; sky mover sectors 3 (E1M3), 2 (E1M6), 2 (E1M8), 4
(E1M9); the 2×2-window bound on two-sided special lines is 14 on E1M5 and 9 on E1M9 (> `MAXSPECIALCROSS` 8, rule 6 fails); E1M3's reachable height
span leaves ±512; E1M2, E1M3 and E1M8 still fail the M3h6 geometry rules. These are Rust admission facts; the C consequences are in §3.13.

### 3.1 Line specials

#### 3.1.1 Dispatch sites

- **`P_CrossSpecialLine`** (p_spec.c:492–950), called from the `P_TryMove` crossing loop (p_map.c:500–514) with `side = oldside` (p_map.c:507, 511). A
  non-player returns for `MT_ROCKET`, `MT_PLASMA`, `MT_BFG`, `MT_TROOPSHOT`, `MT_HEADSHOT`, `MT_BRUISERSHOT` (506–518), then acts only on 39, 97, 125,
  126, 4, 10, 88 (520–534). So in Episode 1 a monster crossing does something only for **97** (teleport) and 88 (lift); the rocket and the baron's
  ball cross nothing. W1 cases set `line->special = 0` after the call **whatever the call returned** (e.g. 545–546); WR cases never clear (787–948).
- **`P_UseSpecialLine`** (p_switch.c:276–653), from `PTR_UseTraverse` (M3h8 §2.3) and from the `P_Move` failure branch with `side = 0`
  (p_enemy.c:313–324). Back side returns false except 124 (284–297). A non-player returns false on `ML_SECRET` lines and on anything but 1, 32, 33, 34
  (301–319); for 32–34 `EV_VerticalDoor` then returns because `thing->player` is null (p_doors.c:373–374, 386–387, 400–401), so **monsters open only
  special 1**. S1 cases call `P_ChangeSwitchTexture(line,0)` (special cleared, p_switch.c:211–212) only when the `EV_*` returned nonzero (e.g.
  349–350); SR cases call it with 1 (button, no clear). 11 and 51 change the texture unconditionally before exiting (359–363, 431–435). It returns
  true for every front-side use (652).
- **`P_ShootSpecialLine`** (p_spec.c:959–1000), from `PTR_ShootTraverse` for **every** special line the hitscan trace reaches, before the two-sided
  and opening tests, so a shot that passes through a two-sided special line still triggers it (p_map.c:915–920). A non-player acts only on 46
  (966–978), so **monsters' hitscans open 46 doors too**. Missiles never reach it (it is called only from `PTR_ShootTraverse`, i.e. `P_LineAttack`).

#### 3.1.2 Tag search and sector ownership

- **`P_FindSectorFromLineTag`** (p_spec.c:434–445): linear scan `for (i=start+1; i<numsectors; i++)` returning the first `sectors[i].tag ==
  line->tag`, −1 at the end. Every `EV_*` loops `while ((secnum = P_FindSectorFromLineTag(line,secnum)) >= 0)`, so sectors are visited in index order.
  `EV_BuildStairs` overwrites `secnum` inside its loop (§3.1.5), changing where the next search starts.
- **Ownership**: `sector->specialdata` is the single mover slot. `EV_DoDoor` (p_doors.c:278–279), `EV_DoFloor` (p_floor.c:277–278), `EV_DoPlat`
  (p_plats.c:168–169), `EV_BuildStairs` (p_floor.c:480–481, and per stair 533–534) and `EV_DoDonut` for `s1` only (p_spec.c:1180–1181) skip a sector
  whose `specialdata` is set. **Exceptions**: `EV_VerticalDoor` reads `specialdata` as a `vldoor_t` without a type check (p_doors.c:416–418) and, for
  31–34, creates a second door over a busy sector (§3.1.3); `EV_DoDonut` overwrites `s2->specialdata` without a test (p_spec.c:1195).
- **Release**: a thinker clears `specialdata` when it ends: doors (p_doors.c:130, 138, 188), floors (p_floor.c:224), plats via `P_RemoveActivePlat`
  (p_plats.c:307). A thinker that ends after another thinker took the slot clears the newer owner's slot (possible only through the two exceptions
  above).
- **Thinker order**: every new mover is appended with `P_AddThinker` before its fields are filled (e.g. p_doors.c:284–285), so movers run in creation
  order relative to mobjs; two `EV_*` calls in one tic append in call order.

#### 3.1.3 Doors (2, 16, 26–28, 31–34, 46, 76, 86, 90, 103)

`vldoor_e` is `normal 0, close30ThenOpen 1, close 2, open 3, raiseIn5Mins 4, blazeRaise 5, blazeOpen 6, blazeClose 7` (p_spec.h:328–339). New types
for Episode 1: **open** and **close30ThenOpen** (normal and blazeOpen are M3h8 §2.4).

- **`EV_DoDoor`** (p_doors.c:264–347), per tagged sector without `specialdata`: appended, `specialdata = door`, `T_VerticalDoor`, `type`, `topwait =
  VDOORWAIT` (150), `speed = VDOORSPEED` (2 units) (283–292); then
  - `open` (and `normal`, 331–339): `direction = 1`, `topheight = P_FindLowestCeilingSurrounding(sec) − 4 units`, `sfx_doropn` only if `topheight !=
    ceilingheight`. `topcountdown` uninitialised (first written at 182).
  - `close30ThenOpen` (313–318): **`topheight = sec->ceilingheight`** (the current ceiling, not the neighbours), `direction = −1`, `sfx_dorcls`
    always.
  - Returns 1 if any door was created (283, 346).
- **`T_VerticalDoor`** (p_doors.c:63–198), plane 1 (ceiling), `crush = false` (120–123, 170–173):
  - `open` going up: `pastdest` clears `specialdata` and removes the thinker (185–190). An `open` door already at its top reaches `pastdest` on its
    first tic (`T_MovePlane` up: `ceiling + speed > dest` sets the ceiling to `dest`, p_floor.c:169–180) and is removed after one tic.
  - `close30ThenOpen`: going down, `pastdest` sets `direction = 0`, `topcountdown = 35*30` (142–145); `crushed` goes back up with `sfx_doropn`
    (159–162, the `default` case); waiting, at `topcountdown` 0 `direction = 1`, `sfx_doropn` (87–91); going up, `pastdest` removes it (185–190). A
    close30 door obstructed while closing therefore reopens and is removed: it does not retry the close.
  - No draws anywhere.
- **`EV_VerticalDoor`** (p_doors.c:354–499), new branches:
  - Locks (369–410): 26/32 need `it_bluecard` or `it_blueskull`, 27/34 yellow, 28/33 red; a player without the key gets `player->message` (`PD_BLUEK`,
    `PD_YELLOWK`, `PD_REDK`) and `S_StartSound(NULL,sfx_oof)` and the function returns (presentation only; no state change). A non-player returns
    (373–374, 386–387, 400–401).
  - `sec = sides[line->sidenum[side^1]].sector` with `side = 0` (364, 413): the back sector, no two-sided check (M3h8 §2.4).
  - Busy sector (416–437): for **1, 26, 27, 28, 117** the existing `specialdata` is cast to `vldoor_t`; going down → up; otherwise a monster returns
    ("bad guys never close doors", 430–431) and a player sets `direction = −1`; return. For **31–34** there is no case, so control falls through and a
    **second door thinker** is created and `specialdata` overwritten (459–461).
  - Sound (440–455): `sfx_doropn` for 1, 31 and every locked door, unconditionally.
  - New thinker (459–498): `direction 1`, speed 2, wait 150; 1/26/27/28 → `normal` (470–475); **31/32/33/34 → `open` and `line->special = 0`**
    (477–483: the D1 line is cleared only when a thinker is created, so a locked D1 used without the key stays usable); `topheight =
    P_FindLowestCeilingSurrounding(sec) − 4` (497–498).
- **46 (gun)**: `EV_DoDoor(line,open)` then `P_ChangeSwitchTexture(line,1)` **without a success test** (p_spec.c:988–992): every hit restarts a button
  if the texture is in the switch list, even when no door started.

#### 3.1.4 Floors (5, 18, 23, 70, 82, 91, 98)

`floor_e` (p_spec.h:537–570) indices: lowerFloor 0, lowerFloorToLowest 1, turboLower 2, raiseFloor 3, raiseFloorToNearest 4, raiseToTexture 5,
lowerAndChange 6, raiseFloor24 7, raiseFloor24AndChange 8, raiseFloorCrush 9, raiseFloorTurbo 10, **donutRaise 11**, raiseFloor512 12. `EV_DoFloor`
(p_floor.c:260–444) for each tagged free sector: append, `specialdata`, `T_MoveFloor`, `type`, **`crush = false`** (282–287); `newspecial` and
`texture` are left uninitialised for every type below and are read at `pastdest` only for `donutRaise` (up) and `lowerAndChange` (down)
(p_floor.c:226–247).

| Type (specials) | Direction, speed | Destination | Lines |
| --- | --- | --- | --- |
| lowerFloorToLowest (23 S1, 82 WR; 38 admitted) | −1, 1 | `P_FindLowestFloorSurrounding` (own floor or lower) | 299–305 |
| turboLower (70 SR, 98 WR; 36 admitted) | −1, 4 | `P_FindHighestFloorSurrounding` (starts −500) **+ 8 unless equal to the current floor** | 307–315 |
| raiseFloor (5 W1, 91 WR) | +1, 1 | `P_FindLowestCeilingSurrounding` (starts `MAXINT`, p_spec.c:387), clamped to the own ceiling, minus `8*(type == raiseFloorCrush)` = 0 | 319–329 |
| raiseFloorToNearest (18 S1) | +1, 1 | `P_FindNextHighestFloor(sec, floorheight)`; the current floor if no higher neighbour | 339–345 |

- **`P_FindNextHighestFloor`** (p_spec.c:329–375): walks `sec->lines` in order, collects neighbour floors strictly above `currentheight` into
  `heightlist[20]` (`MAX_ADJOINING_SECTORS`, 326), and **after each line** (also lines that added nothing) tests `h >= 20` and breaks with a stderr
  message (354–360); returns the minimum of the list or `currentheight` if empty (364–374). The write is bounded (index ≤ 19), but the search is
  **truncated** after the 20th qualifying neighbour: later, possibly lower, candidates are ignored. A Rust witness or load rule must bound qualifying
  neighbours < 20 per target sector, or replicate the truncation.
- **`T_MoveFloor`** (p_floor.c:209–254): `T_MovePlane(sector, speed, dest, crush, 0, direction)`; `sfx_stnmov` when `!(leveltime&7)`; at `pastdest`:
  `specialdata = NULL`, the type-dependent special/floorpic write, `P_RemoveThinker`, `sfx_pstop`. A **raising floor that is blocked** (`T_MovePlane`
  floor-up with `crush` false restores and returns `crushed`, p_floor.c:112–123) is not handled by `T_MoveFloor`: it retries every tic until the
  obstruction leaves. A raising floor whose destination equals its height is removed after one tic.
- A retriggered WR raise/lower (91, 82, 98) on a sector already at its destination creates a thinker that ends on its first tic (no state change
  beyond the thinker's existence for one tic and the `sfx_pstop`).

#### 3.1.5 Stairs: `EV_BuildStairs` (7 S1 on E1M8, 8 W1 on E1M3)

`EV_BuildStairs` (p_floor.c:453–554):

1. For each tagged free sector (475–481): a floor thinker appended, `specialdata`, `T_MoveFloor`, `direction 1`, `sector` (484–490); `build8`: `speed
   = FLOORSPEED/4` (16384, a quarter unit per tic), `stairsize = 8 units` (493–496); `floordestheight = floor + stairsize` (502–504); `texture =
   sec->floorpic` (506).
2. Repeat (511–551): scan `sec->lines` in order (514); skip one-sided by **flag** (`ML_TWOSIDED`, 516–517); require `frontsector == sec` **compared by
   index against the variable `secnum`** (519–523); take `tsec = backsector` (525–526); require `tsec->floorpic == texture` (528–529); **`height +=
   stairsize` before the busy test** (531); skip a busy `tsec` and keep scanning the same `sec` (533–534); otherwise `sec = tsec; secnum = newsecnum`,
   append a thinker with `direction 1`, `speed`, `floordestheight = height`, `ok = 1` and restart from the new sector (536–549).
3. **Quirks a reimplementation must copy**:
   - `secnum` is the outer tag-search cursor, so after a staircase the next `P_FindSectorFromLineTag(line, secnum)` resumes **after the last stair
     sector's index**, skipping tagged sectors with a lower index.
   - A busy candidate bumps `height` without being raised, so the next stair built from the same `sec` is one `stairsize` higher.
   - **`floor->type` and `floor->crush` are never written for stair thinkers** (only `thinker`, `direction`, `sector`, `speed`, `floordestheight`:
     485–504, 538–547), and `Z_Malloc` does not clear (z_zone.c:183–289 has no `memset`). `T_MoveFloor` passes the garbage `crush` to `T_MovePlane`
     and `P_ChangeSector` (`crushchange`), and reads the garbage `type` at `pastdest` (`case donutRaise`, p_floor.c:228–232). Consequences: (a)
     `crush` matters only when a stair step does not fit: `T_MovePlane` floor-up returns early without restoring only for `crush == true` exactly
     (p_floor.c:118–119), and `PIT_ChangeSector` damages 10 every 4 tics and sprays blood (5 draws) only when `crushchange` is nonzero
     (p_map.c:1298–1309); (b) a garbage `type == 11` would copy garbage `newspecial`/`texture` into the sector. **This is the only route in Episode 1
     to the crush-damage code** (§3.1.8). A Rust model needs either a declared C adapter (zero-filled `Z_Malloc` for `PU_LEVSPEC`, making `type =
     lowerFloor`, `crush = false`) or a witness that refuses any stair step with `nofit` (then `crush` is unobservable) plus the adapter for `type`.
   - A two-sided-flag line whose `backsector` is null (flag without a second side) would crash at 528; a load rule must refuse it.

#### 3.1.6 Donut: `EV_DoDonut` (9 S1 on E1M2)

`EV_DoDonut` (p_spec.c:1163–1221), for each tagged sector `s1` without `specialdata`:

- `rtn = 1` before anything is spawned (1183): the switch changes even if no thinker is created.
- `s2 = getNextSector(s1->lines[0], s1)` (1184): the other side of **s1's first line**; null (crash at 1185) if that line is not two-sided.
- Loop over `s2->lines` (1185): the skip test is `(!s2->lines[i]->flags & ML_TWOSIDED) || (s2->lines[i]->backsector == s1)` (1187–1189). By C
  precedence `!flags & 4` is `(!flags) & 4`, always 0, so **one-sided lines are not skipped**; only a line whose back sector is `s1` is. `s3 =
  s2->lines[i]->backsector` (1190): **null for a one-sided line (crash at 1202)**, and **`s2` itself** for a line with `s2` on its back side (e.g. an
  `s1`/`s2` line drawn with `s1` in front).
- First qualifying line: a `donutRaise` thinker on `s2` (1193–1205): `crush false`, `direction 1`, speed `FLOORSPEED/2`, `texture = s3->floorpic`,
  `newspecial = 0`, dest `s3->floorheight`, **`s2->specialdata` overwritten without a busy test**; then a `lowerFloor` thinker on `s1` (1207–1216):
  `direction −1`, speed 1/2, dest `s3->floorheight` (no `P_FindHighestFloorSurrounding`; the type only matters at `pastdest`, where lowerFloor does
  nothing extra); `break` (1217).
- At `pastdest` the `s2` thinker writes `special = 0` and `floorpic = s3's` (p_floor.c:230–232).
- No draws. Rust needs a load rule for each donut line: `s1->lines[0]` two-sided, the first `s2` line whose back sector is not `s1` has a back sector,
  and `s2` either free or a declared double-owner behavior.

#### 3.1.7 Plats (20 S1, 22 W1, 62 SR)

- **62** is `EV_DoPlat(line, downWaitUpStay, 1)` (p_switch.c:546): identical to 88 (M3h8 §2.5); `amount` is unused for this type (p_plats.c:207–218).
  SR: the texture changes only when a plat started.
- **`raiseToNearestAndChange`** (20, 22; p_plats.c:185–195): `speed = PLATSPEED/2` (half a unit); **`sec->floorpic =
  sides[line->sidenum[0]].sector->floorpic`** (the line's front sector's floor flat); `high = P_FindNextHighestFloor(sec, floorheight)` (§3.1.4);
  `wait = 0`; `status = up`; **`sec->special = 0`**; `sfx_stnmov`. `low` and `oldstatus` are not written. `P_AddActivePlat` (251).
- **`T_PlatRaise`** for this type (p_plats.c:52–131): `up`: `T_MovePlane(floor, speed, high, crush = false, 0, 1)`, `sfx_stnmov` every 8 tics (64–70);
  `pastdest` → `count = wait` (0), `waiting`, `sfx_pstop`, **`P_RemoveActivePlat`** (82–99): the plat ends at the top. If **blocked while rising**
  (`res == crushed && !crush`, 73–79): `count = wait = 0`, `status = down`, `sfx_pstart`, and the next tics run `T_MovePlane(floor, speed, plat->low,
  …, −1)` (109) with the **uninitialised `low`**; at that `pastdest` `waiting` with `count = 0`, and `--count` (120) goes to −1 and keeps decrementing
  (no zero for 2³² tics). A Rust model must refuse (witness) a raiseToNearestAndChange plat being blocked while rising.
- `floorpic` is gameplay state only through `EV_BuildStairs` (§3.1.5) and `EV_DoDonut` (`s3->floorpic`, §3.1.6) comparisons/copies; otherwise
  presentation.
- `P_AddActivePlat` (p_plats.c:288–299) fills the first null of `activeplats[MAXPLATS]` (30, p_spec.h:306) and `I_Error`s when full (checked).
  `P_RemoveActivePlat` (301–314) clears `specialdata`, removes the thinker, nulls the slot, `I_Error` if absent. Episode 1 has at most 9 plat sectors
  per map (§3.0.4) and `specialdata` allows one plat per sector, so the bound is unreachable.

#### 3.1.8 Crushing and `P_ChangeSector` in Episode 1

- `P_ChangeSector` (p_map.c:1321–1338), `PIT_ChangeSector` (1257–1313): M3h8 §2.7. The damage branch `if (crushchange && !(leveltime&3))`:
  `P_DamageMobj(thing, NULL, NULL, 10)` then `P_SpawnMobj(x, y, z + height/2, MT_BLOOD)` and `momx = (P_Random() − P_Random())<<12`, `momy` likewise
  (1298–1309).
- Draws when it fires, in call order: the `P_DamageMobj` draws (with `inflictor == NULL` there is no thrust and no fall-forward draw, p_inter.c:806;
  one pain draw if the victim survives, 894), then `lastlook` of the blood (p_mobj.c:506), then four draws in two differences of unspecified
  evaluation order (M3h7 §2.10).
- Every Episode 1 mover passes `crush = false` (doors p_doors.c:123, 173; plats p_plats.c:62, 109 with `plat->crush = false`, 180; floors
  p_floor.c:287; donut p_spec.c:1198, 1212) **except the uninitialised stair `crush`** (§3.1.5). The "gib state" (`S_GIBS`, info.c:1031, tics −1, no
  action) is reachable by any mover through `PIT_ChangeSector`'s `health <= 0` branch (1269–1279).

#### 3.1.9 Ceilings, stasis and stop (absent from Episode 1)

No Episode 1 line is a ceiling, crusher, crusher-stop or platform-stop special (census table (f)); `EV_DoCeiling`, `T_MoveCeiling`, `activeceilings`,
`EV_CeilingCrushStop`, `EV_StopPlat` and `P_ActivateInStasis*` are not needed, and out of scope. (For the record, `P_AddActiveCeiling` at
p_ceilng.c:251–262 silently drops the 31st ceiling and `perpetualRaise` draws `status = P_Random()&1`, p_plats.c:246.)

#### 3.1.10 Light (35 W1 on E1M3)

`EV_LightTurnOn(line, 35)` (p_lights.c:271–306): for every sector with the line's tag, `lightlevel = 35` (bright nonzero skips the neighbour search,
290–303). No thinker, no draw. `lightlevel` is read in game code only by the light thinkers themselves (their `minlight`/`maxlight` were fixed at
spawn), so a light thinker on the same sector keeps overwriting it. Gameplay-neutral, but sector state.

#### 3.1.11 Exits: secret exit 51

`G_SecretExitLevel` (g_game.c:1009–1018): outside commercial mode `secretexit = true; gameaction = ga_completed`. The rest of the tic runs (as for 11,
M3h9c §2.8). §3.11 has the level transition.

#### 3.1.12 Retrigger classes in Episode 1

| Class | Specials | Clearing rule |
| --- | --- | --- |
| W1 | 2, 5, 8, 16, 22, 35 (+36, 38, 109) | `line->special = 0` after the call, even if nothing started (p_spec.c:546 etc.) |
| WR | 76, 82, 86, 90, 91, 97, 98 (+88) | never cleared |
| S1 | 7, 9, 18, 20, 23, 103 | cleared through `P_ChangeSwitchTexture(line,0)` only when the `EV_*` returned nonzero; 51 (+11) unconditionally |
| SR | 62, 70 (+63) | texture swapped and a button started (if the texture is a switch) only when the `EV_*` returned nonzero |
| GR | 46 | button started on every hit (no success test) |
| D1 | 31–34 | cleared in `EV_VerticalDoor` only when a door thinker is created (p_doors.c:482) |
| DR | 26–28 (+1) | never cleared |

Button facts (dedup per line, `MAXBUTTONS` 16 with `I_Error`, `BUTTONTIME` 35, sound at `buttonlist->soundorg`): M3h8 §2.8. Episode 1 has ≤ 10
reusable switch lines per map (§3.0.4), so the 16-slot bound is unreachable.

#### 3.1.13 Every draw on the §3.1 paths

None: `EV_DoDoor`, `EV_VerticalDoor`, `T_VerticalDoor`, `EV_DoFloor`, `T_MoveFloor`, `EV_BuildStairs`, `EV_DoDonut`, `EV_DoPlat` (all Episode 1
types), `T_PlatRaise`, `EV_LightTurnOn`, `G_SecretExitLevel`, the switch and button code. Draws arise only from (a) the crush branch (§3.1.8, stair
garbage only), (b) `P_DamageMobj`/`P_KillMobj` of things killed or hurt during a clip, (c) monster door use through `P_TryWalk` (M3h8 §2.11), and (d)
the teleport's fog spawns (§3.3).

### 3.2 Sector specials

#### 3.2.1 Which run where

- `P_SpawnSpecials` (p_spec.c:1239–1362) has cases 1, 2, 3, 4, 8, 9, 10, 12, 13, 14, 17 (1276–1331); **11 and 16 have no spawn case** and stay in
  `sector->special`. Every light case zeroes the special (M3h9c §2.5), 4 restores 4 (1296), 9 keeps 9 until entered.
- `P_PlayerInSpecialSector` (p_spec.c:1009–1071) runs from `P_PlayerThink` only if the player's centre sector has a nonzero special
  (p_user.c:274–275), and returns at once unless `player->mo->z == sector->floorheight` (1016–1017). Cases 5, 7, 16/4, 9, 11; **any other nonzero
  special is `I_Error`** (1065–1069). In Episode 1 the sectors that keep a nonzero special during play are 7, 9 (until entered), 16 and 11, plus any
  special a mover writes: donut writes 0 (p_floor.c:231 with `newspecial = 0`), raiseToNearestAndChange writes 0 (p_plats.c:192); no Episode 1 mover
  writes a nonzero special (raiseFloor24AndChange and lowerAndChange, which copy one, are absent).

#### 3.2.2 Special 16 (E1M3 ×5, E1M9 ×1; admitted) and the radiation-suit draw

Case 16 falls into case 4 (1036–1046):

```c
if (!player->powers[pw_ironfeet] || (P_Random()<5) )
{   if (!(leveltime&0x1f)) P_DamageMobj (player->mo, NULL, NULL, 20); }
```

With **no suit** the `||` short-circuits: no draw (the admitted behavior). **With a suit** the draw happens **every tic the player stands on the floor
of a 16 sector**, whether or not `leveltime&0x1f` is zero, and on the 1-in-32 tics a draw below 5 lets 20 damage through. Cases 5 and 7 test the suit
with no draw (1022–1034). **Reachability in Episode 1 (medium): none.** Suits exist only on E1M4–E1M7 ([§2](#2-episode-1-census-medium-skill)),
special 16 only on E1M3 and E1M9, and `G_PlayerFinishLevel` clears all powers at every exit (g_game.c:785), so a suit never reaches a 16 sector. The
draw is still part of the admitted special's body once suits exist; a model must implement it (or refuse the combination by witness).

#### 3.2.3 Special 11 (E1M8, one sector): end-of-episode damage

Case 11 (p_spec.c:1054–1063), on every tic the player's centre sector is 11 and `z == floorheight`:

1. `player->cheats &= ~CF_GODMODE` (1056; cheats never set in playback);
2. `if (!(leveltime&0x1f)) P_DamageMobj (player->mo, NULL, NULL, 20)` (1058–1059): no suit test;
3. `if (player->health <= 10) G_ExitLevel ()` (1061–1062), tested every such tic, after the damage.

Coupled rule in **`P_DamageMobj`** (p_inter.c:837–842): for a player target, `if (target->subsector->sector->special == 11 && damage >=
target->health) damage = target->health − 1;`, applied **before** the invulnerability return and armor (847–869) and **regardless of z** (any damage
source: monster, missile, barrel, floor). So in an 11 sector a player cannot be killed by a single blow while its pre-armor damage is capped to health
− 1; armor then reduces the capped damage further. Exiting needs health ≤ 10 at a tic where the player is on the floor. Draws: the 20-point damage
uses `P_DamageMobj` with `inflictor` null (no thrust, no fall-forward draw) and **one pain draw** if the player survives (894: `P_Random () <
painchance`; the player's `painchance` is 255, info.c:1117), i.e. one draw per 32 tics. `G_ExitLevel` on E1M8 leads to the finale, not an intermission
(§3.11.2).

### 3.3 Teleportation (97 WR; E1M5 ×10, E1M8 ×8, E1M9 ×2)

#### 3.3.1 `EV_Teleport` (p_telept.c:47–131)

1. `if (thing->flags & MF_MISSILE) return 0` (65–66). Redundant for the six missile types, which `P_CrossSpecialLine` already rejects
   (p_spec.c:506–518).
2. **`if (side == 1) return 0`** (70–71): crossing from the back side does nothing (the line stays, being WR).
3. `tag = line->tag`; **for each sector index `i` in order** with `sectors[i].tag == tag` (74–78), **walk the whole thinker list from
   `thinkercap.next`** (79–83), skipping non-mobj thinkers (`function.acp1 != P_MobjThinker`, 85–86; removed mobjs carry `(actionf_v)(−1)` and are
   skipped too), non-`MT_TELEPORTMAN` mobjs (91–92) and teleporters whose `subsector->sector` is not sector `i` (94–97). The first match wins: **the
   lowest-indexed tagged sector that holds a landing, and within it the oldest landing in thinker order**. Cost is `O(tagged sectors × thinkers)` per
   crossing: no array, but an unbounded scan that a Rust budget must cover.
4. `oldx/oldy/oldz` saved (99–101); **`if (!P_TeleportMove (thing, m->x, m->y)) return 0`** (103–104): a failed move aborts the whole teleport (no
   other landing is tried, no fog).
5. `thing->z = thing->floorz` (106); for a player `viewz = z + viewheight` (107–108).
6. **Fog 1**: `P_SpawnMobj (oldx, oldy, oldz, MT_TFOG)` and `sfx_telept` (111–112). **Fog 2**: `an = m->angle >> ANGLETOFINESHIFT`; `P_SpawnMobj (m->x
   + 20*finecosine[an], m->y + 20*finesine[an], thing->z, MT_TFOG)` and `sfx_telept` (113–118). The second fog is 20 units in front of the landing and
   may be in another sector or beyond a wall; it is positioned with no check.
7. Player only: **`thing->reactiontime = 18`** (121–122).
8. `thing->angle = m->angle`; **`momx = momy = momz = 0`** (124–125); return 1.

#### 3.3.2 `P_TeleportMove` and `PIT_StompThing` (p_map.c:81–177)

- Sets `tmthing`, `tmflags`, `tmx/tmy`, the bbox (130–139); **floor and ceiling from the destination's centre subsector only** (`R_PointInSubsector`,
  141, 148–149): `tmfloorz = tmdropoffz = floorheight`, `tmceilingz = ceilingheight`; `ceilingline = NULL` (142); `validcount++`; **`numspechit = 0`**
  (151–152).
- **No line pass and no fit test**: the thing's new `floorz`/`ceilingz` ignore nearby lines, and nothing checks that it fits (`tmceilingz − tmfloorz`
  or the step).
- Things: cells `tmbbox ± MAXRADIUS`, **`bx` outer, `by` inner**, each cell's `bnext` list (155–163). `PIT_StompThing` (81–108): skip
  non-`MF_SHOOTABLE` (85–86), skip if outside the summed radii on either axis (88–95; **no z test**), skip itself (98–99); **a non-player mover
  returns false** unless `gamemap == 30` (102–103), so a monster teleport fails if any shootable thing overlaps the landing; a player **telefrags**:
  `P_DamageMobj (thing, tmthing, tmthing, 10000)` (105) and continues. Barrels, monsters and corpses-in-pain (any shootable) are stomped.
- Then unlink, set `floorz`, `ceilingz`, `x`, `y`, relink (167–174). `z` is set by the caller.

#### 3.3.3 Interaction with the movement loops

- **The crossing loop stops after a teleport.** EV_Teleport runs inside `P_TryMove`'s `while (numspechit--)` (p_map.c:502–513); `P_TeleportMove`
  zeroes `numspechit`, so the next test `numspechit--` reads 0, ends the loop and leaves `numspechit = −1`. Entries not yet visited are never crossed.
  (`numspechit` is reset by the next `P_CheckPosition`, p_map.c:410, and read by `P_Move`'s failure branch only after a fresh one.)
- **A split move continues from the landing.** `P_XYMovement` keeps its local `xmove/ymove` (p_mobj.c:147–191): if the teleport happened in the first
  half of a split step (`xmove > MAXMOVE/2 || ymove > MAXMOVE/2`, 152), the second half is tried **from the landing** with the remaining half-move,
  although `momx/momy` are now 0; afterwards the friction code sees zero momentum. A player moving faster than 15 units per axis (positive direction
  only, §3.10.4) can thus step up to half a move past the landing.
- `P_TryMove` itself returns true; `P_Move` then sets `z = floorz` for a non-floater (p_enemy.c:332–333) and `P_TryWalk` draws its `movecount`
  (p_enemy.c:356).

#### 3.3.4 The fog actor `MT_TFOG`

`MT_TFOG` (info.c:2122–2146): spawn state `S_TFOG`, radius 20, height 16, flags `MF_NOBLOCKMAP|MF_NOGRAVITY`, not shootable. States `S_TFOG`,
`S_TFOG01`, `S_TFOG02`, `S_TFOG2` … `S_TFOG10`, twelve states of 6 tics, no actions, then `S_NULL` (info.c:266–277): each fog lives **72 tics** and is
removed by `P_SetMobjState(S_NULL)`. `P_SpawnMobj` draws `lastlook` once per fog (p_mobj.c:506) and appends it at the thinker tail; there is no
spawn-tics draw (that is `P_SpawnMapThing` only). `P_MobjThinker` runs `P_ZMovement` for a fog whose `z != floorz` (p_mobj.c:428–431): with
`MF_NOGRAVITY`, `z` only clamps to `floorz` / `ceilingz − 16` (p_mobj.c:284–311, 328–335). Fog 1 inherits the teleported thing's old z; fog 2 the new
z.

#### 3.3.5 The landing `MT_TELEPORTMAN`

`MT_TELEPORTMAN` (info.c:2174–2198): doomednum 14, **spawn state `S_NULL`** (state 0: `SPR_TROO`, tics −1, no action, info.c:136), flags
`MF_NOBLOCKMAP|MF_NOSECTOR`, radius 20, height 16. It is a mobj with a `P_MobjThinker` thinker that never changes state; it is in no sector list and
no block list (p_maputl.c:409, 425) but `subsector` is set (p_maputl.c:406–407), which is what `EV_Teleport` reads. Spawn costs one `lastlook` draw
and no spawn-tics draw (tics −1). Not counted (no `MF_COUNTKILL`/`MF_COUNTITEM`).

#### 3.3.6 Draws, in order, for one successful teleport

1. `PIT_StompThing` victims (player only), in `bx`-outer/`by`-inner cell order and `bnext` order: each `P_DamageMobj(…, 10000)` → thrust from the
   player (inflictor), the fall-forward condition `damage < 40` is false (no draw), the victim dies → `P_KillMobj` → **one `tics` draw**
   (p_inter.c:726) and, for zombieman/shotgun guy, a drop `P_SpawnMobj` (**one `lastlook` draw**, 756). A stomped barrel starts its explosion chain
   later (M3h9b §2.4–2.5). **The knock-back overflows**: `thrust = damage*(FRACUNIT>>3)*100/target->info->mass` (p_inter.c:817) is evaluated in `int`;
   `10000*8192*100` = 8,192,000,000 exceeds 2³¹ − 1 (any damage ≥ 2622 overflows). Signed overflow is undefined; a wrapping build gives −397,934,592
   before the division by the mass (toward zero), i.e. a large thrust **toward** the stomping player, applied to the victim's momentum before it dies
   (the corpse then slides, clamped to `MAXMOVE` per axis). No thrust at all if the player's ready weapon is the chainsaw (p_inter.c:806–811). This
   needs a declared sanitizer/arithmetic adapter like the existing p_map.c:1221 one (M3h9c §2.10), and the genuine build's value must be observed.
2. Fog 1 `lastlook` (one draw).
3. Fog 2 `lastlook` (one draw).

A failed monster teleport draws nothing. `S_StartSound` draws `M_Random` only.

#### 3.3.7 Freeze, angle, momentum, alert

- **18-tic freeze**: `P_PlayerThink` decrements `reactiontime` instead of calling `P_MovePlayer` (p_user.c:267–270), so for the next 18 player thinks
  neither thrust **nor turning** (`angleturn` is applied inside `P_MovePlayer`, 154) happens; `P_CalcHeight`, specials, weapon change, use, psprites
  (attack) and power timers still run. The teleport itself happens in `P_MobjThinker` after that tic's `P_PlayerThink`.
- Monsters get no freeze (121–122 test `thing->player`); their `reactiontime` field is unchanged.
- Angle = the landing's map angle (`P_SpawnMapThing` sets `ANG45*(angle/45)`, p_mobj.c:794; the octant adapter of M3h9c §2.10 applies).
- Sound alert: nothing new. `P_NoiseAlert` is driven by weapon fire only; `soundtarget` stays per sector, a monster keeps its `target` across a
  teleport, and `A_Look`'s `MF_AMBUSH` handling (p_enemy.c:604–627) is unchanged (§3.9).

### 3.4 New monsters: Spectre and Baron of Hell

#### 3.4.1 Spectre (`MT_SHADOWS`, doomednum 58)

- **info.c:1446–1470** is field-for-field the demon `MT_SERGEANT` (info.c:1420–1444): states `S_SARG_*`, health 150, reaction 8, painchance 180, speed
  10, radius 30, height 56, mass 400, sounds `sgtsit`/`sgtatk`/`dmpain`/`sgtdth`/`dmact`, raise state — **except the flags**, which add `MF_SHADOW`:
  `MF_SOLID|MF_SHOOTABLE|MF_SHADOW|MF_COUNTKILL`. Its states and actions are therefore the admitted demon's (`S_SARG_STND` 10 tics A_Look, info.c:611;
  `S_SARG_ATK1/2` A_FaceTarget, `S_SARG_ATK3` A_SargAttack, 621–623); its spawn costs a `lastlook` draw and a spawn-tics draw (tics 10).
- **Where `MF_SHADOW` acts in game code** (complete list of readers in `p_*.c` and g_game.c): `A_FaceTarget` (p_enemy.c:794–795) and `P_SpawnMissile`
  (p_mobj.c:909–910), both testing the **target's** flag; `P_GivePower` sets it on the player (p_inter.c:303), `P_PlayerThink` clears it when
  invisibility expires (p_user.c:345–347), `G_PlayerFinishLevel` clears it (g_game.c:787). The renderer reads it (presentation). Hence the spectre's
  own behavior equals the demon's; the flag matters only when **another monster targets the spectre** (infighting): the attacker's `A_FaceTarget`
  draws two more and fuzzes its angle; a missile attacker's `P_SpawnMissile` draws two more. Player hitscans, autoaim (`P_AimLineAttack`), sight
  (`P_CheckSight`) and `P_LookForPlayers` do not read it.
- `A_FaceTarget` (p_enemy.c:782–796): `if (!target) return`; clears `MF_AMBUSH` (787); `angle = R_PointToAngle2(actor, target)` (789–792); **`if
  (target->flags & MF_SHADOW) angle += (P_Random()−P_Random())<<21`** (794–795). The two draws' evaluation order is unspecified C; the left-shift of a
  negative difference is undefined. The project's convention (docs/M3D1_REFERENCE.md "explicit ordered RNG calls", docs/M3D2B_REFERENCE.md "explicit
  left then right draws and multiplication") is the declared adapter; this site (`<<21`) and the `P_SpawnMissile` site (`<<20`) need the same
  treatment and a C observation of the genuine build's order.

#### 3.4.2 Baron of Hell (`MT_BRUISER`, doomednum 3003; two on E1M8)

- **info.c:1498–1522**: spawn `S_BOSS_STND`, health 1000, see `S_BOSS_RUN1`, seesound `brssit`, reaction 8, attacksound 0, pain `S_BOSS_PAIN`,
  **painchance 50**, painsound `dmpain`, **meleestate = missilestate = `S_BOSS_ATK1`**, death `S_BOSS_DIE1`, no xdeath, deathsound `brsdth`, **speed
  8, radius 24, height 64, mass 1000**, damage 0, activesound `dmact`, flags `MF_SOLID|MF_SHOOTABLE|MF_COUNTKILL`, raise `S_BOSS_RAISE1`.
- **States** (info.c:663–691): `STND`/`STND2` 10 tics A_Look; `RUN1`–`RUN8` 3 tics A_Chase; **`ATK1` 8 A_FaceTarget → `ATK2` 8 A_FaceTarget → `ATK3` 8
  A_BruisAttack → `RUN1`**; `PAIN` 2 (no action) → `PAIN2` 2 A_Pain → `RUN1`; `DIE1` 8 → `DIE2` 8 A_Scream → `DIE3` 8 → `DIE4` 8 A_Fall → `DIE5` 8 →
  `DIE6` 8 → **`DIE7` tics −1 A_BossDeath**; `RAISE1`–`RAISE7` (archvile only). Spawn: `lastlook` + spawn-tics draw (10 tics).
- **`A_Chase`** has no type-specific code (p_enemy.c:672–776). The melee branch fires when `P_CheckMeleeRange` holds (725–733; `dist < MELEERANGE − 20
  + target radius`, i.e. 60 units against a player, plus sight, p_enemy.c:174–195); the missile branch (736–750) on medium needs `movecount == 0` and
  `P_CheckMissileRange`. Both lead to `S_BOSS_ATK1`; the choice between claw and fireball is made later by `A_BruisAttack`.
- **`P_CheckMissileRange`** (p_enemy.c:197–256): `P_CheckSight`; `MF_JUSTHIT` → clear and true; `reactiontime` → false; `dist = P_AproxDistance − 64`;
  **no −128** for the baron (it has a melee state, 219–220); `>>16`; type cases for `MT_VILE`, `MT_UNDEAD`, `MT_CYBORG/MT_SPIDER/MT_SKULL` only
  (224–244); cap 200 (246–247); **one draw** `P_Random () < dist` → false (252–253). Spectre: no missile state, never called.
- **`A_BruisAttack`** (p_enemy.c:979–996): `if (!target) return`; **no `A_FaceTarget`** (the two preceding states did it); if `P_CheckMeleeRange`:
  `sfx_claw`, **damage `(P_Random()%8+1)*10`** (10–80, one draw), `P_DamageMobj(target, actor, actor, damage)`, return; else `P_SpawnMissile (actor,
  target, MT_BRUISERSHOT)`.
- **`MT_BRUISERSHOT`** (info.c:1524–1548): spawn `S_BRBALL1`, seesound `firsht`, death `S_BRBALLX1`, deathsound `firxpl`, **speed 15 units, radius 6,
  height 8, damage 8**, flags `MF_NOBLOCKMAP|MF_MISSILE|MF_DROPOFF|MF_NOGRAVITY`. States (info.c:658–662): `BRBALL1`/`BRBALL2` 4 tics loop; `BRBALLX1`
  6 → `X2` 6 → `X3` 6 → `S_NULL` (no `A_Explode`: no splash damage). Hit damage `((P_Random()%8)+1)*8` = 8–64 (p_map.c:323). Speed 15 equals
  `MAXMOVE/2` exactly and the split test is strict (`xmove > MAXMOVE/2`, p_mobj.c:152; `FixedMul(15·FRACUNIT, finecosine)` ≤ 15·FRACUNIT), so **a
  baron ball never splits its move**. It is filtered from line crossings (p_spec.c:513).
- **Species rule** (p_map.c:299–313): a missile whose owner's type equals the victim's type, or Knight/Baron pairs (301–302), explodes on contact
  without damage unless the victim is a player; the owner itself is passed. So baron balls do not hurt barons, but hurt every other monster type
  (infighting through `P_DamageMobj`'s retarget, p_inter.c:904–915).
- **`A_Scream`** (p_enemy.c:1535–1569): `brsdth` and `sgtdth` take the default branch (no draw); full volume only for spider/cyborg. **`A_Pain`**
  (1577–1581) and **`A_Fall`** (1585–1592, clears `MF_SOLID`) draw nothing. **`A_Look`** (604–664): the see sounds `brssit`/`sgtsit` take the default
  branch (no draw).
- **`P_KillMobj`** (p_inter.c:668–758): no drop for either type (737–754 drop only for `MT_WOLFSS`, `MT_POSSESSED`, `MT_SHOTGUY`, `MT_CHAINGUY`); one
  `tics` draw (726). Barons have no xdeath state, so they always pass `S_BOSS_DIE7`.

#### 3.4.3 `A_BossDeath` and the E1M8 ending (p_enemy.c:1609–1756)

On entry to `S_BOSS_DIE7`, six 8-tic states after the kill, the first shortened by the `P_KillMobj` tics draw (`tics -= P_Random()&3`, minimum 1),
i.e. 45–48 tics after death:

1. Not commercial: `gameepisode == 1` requires **`gamemap == 8`** (1630–1631) and **`mo->type == MT_BRUISER`** (1633–1634), else return.
2. **A live player must exist**: the first `i` with `playeringame[i] && players[i].health > 0`; none → return (1683–1688). `players[i].health` is the
   player's, not the mobj's.
3. **Thinker scan** (1692–1705): any other mobj of the same type with `health > 0` → return. A baron that is dead but has not yet reached `DIE7`
   counts as dead, so when both die in quick succession **each** reaches the victory branch on entering `DIE7`.
4. Episode 1 victory (1731–1734): `junk.tag = 666; EV_DoFloor (&junk, lowerFloorToLowest); return;` — **no `G_ExitLevel`** on E1M8 (the exit is the
   special-11 sector, §3.2.3). `junk` is an uninitialised stack `line_t` (1613) whose only field read by `EV_DoFloor(lowerFloorToLowest)` is `tag`
   (through `P_FindSectorFromLineTag`, p_spec.c:441), so the garbage is harmless. The second baron's call finds the tag-666 sectors busy
   (`specialdata`) if the first floor is still moving, or starts a floor that ends at once if they are already at their lowest neighbour.
5. A baron corpse **gibbed by a mover before `DIE7`** (`PIT_ChangeSector`, p_map.c:1269–1279 sets `S_GIBS`) never runs `A_BossDeath`; if it was the
   last baron, the tag-666 floor never lowers. (Movers that can close on a corpse exist on E1M8: doors and lifts.)

#### 3.4.4 Every draw on the §3.4 paths, in call order

| Path | Draws |
| --- | --- |
| Spawn (spectre, baron) | `lastlook`; spawn-tics (tics 10) |
| `A_Look`, `A_Pain`, `A_Fall`, `A_Scream` (these types) | none |
| `A_Chase` | the existing `P_NewChaseDir`/`P_TryWalk` draws; `P_Random () < 3` for the active sound (771–772); baron missile branch: one in `P_CheckMissileRange` |
| `A_FaceTarget` vs an `MF_SHADOW` target | two (fuzz) |
| `A_SargAttack` (spectre) | `A_FaceTarget` (two if target shadowed) → if in melee range one damage draw `((P_Random()%10)+1)*4` (p_enemy.c:945) → `P_DamageMobj` draws |
| Baron attack cycle | `ATK1` `A_FaceTarget` (two if shadowed) → `ATK2` `A_FaceTarget` (two if shadowed) → `ATK3`: melee: one damage draw → `P_DamageMobj` draws; missile: `P_SpawnMissile` = `lastlook` → (two if shadowed) → `P_CheckMissileSpawn` `tics` draw (p_mobj.c:870) → if the spawn move hits a thing: hit-damage draw → `P_DamageMobj` draws; if it fails: `P_ExplodeMissile` `tics` draw (p_mobj.c:96) |
| `P_DamageMobj` on a monster/player (p_inter.c:775–917) | fall-forward draw only if `inflictor && damage < 40 && damage > health && target z − inflictor z > 64` (820–823, `&&` short-circuit); pain draw if the target survives and is not skull-flying (894) |
| `P_KillMobj` | `tics` draw (726); drop `lastlook` for the three dropping types |

#### 3.4.5 Lost Soul and Cacodemon (absent from Episode 1)

Not spawned on any skill in E1M2–E1M9; the Lost Soul and Cacodemon mechanics are out of scope and not recorded here.

### 3.5 Weapons: chainsaw and rocket launcher

Plasma rifle, BFG and cells are absent ([§2](#2-episode-1-census-medium-skill)), but the backpack gives cells (§3.6.6), so `am_cell` becomes nonzero
state without any cell weapon.

#### 3.5.1 Weapon table and slots

- `weapontype_t` (doomdef.h:180–195): `wp_fist 0, wp_pistol 1, wp_shotgun 2, wp_chaingun 3, wp_missile 4, wp_plasma 5, wp_bfg 6, wp_chainsaw 7,
  wp_supershotgun 8, wp_nochange`. `ammotype_t` (201–209): `am_clip 0, am_shell 1, am_cell 2, am_misl 3, am_noammo`.
- `weaponinfo[]` (d_items.c): missile launcher `am_misl`, `S_MISSILEUP`, `S_MISSILEDOWN`, `S_MISSILE`, `S_MISSILE1`, flash `S_MISSILEFLASH1`; chainsaw
  `am_noammo`, `S_SAWUP`, `S_SAWDOWN`, `S_SAW`, `S_SAW1`, flash `S_NULL`.
- `maxammo = {200, 50, 300, 50}`, `clipammo = {10, 4, 20, 1}` (p_inter.c:58–59): a rocket is one "clip".

#### 3.5.2 Chainsaw (pickup 2005, `MT_MISC26`)

- **States** (info.c:203–209): `S_SAW` 4 tics A_WeaponReady → `S_SAWB` 4 tics A_WeaponReady → `S_SAW`; `S_SAWDOWN` 1 A_Lower; `S_SAWUP` 1 A_Raise;
  **`S_SAW1` 4 A_Saw → `S_SAW2` 4 A_Saw → `S_SAW3` 0 A_ReFire → `S_SAW`**. Unlike every admitted weapon (ready states of 1 tic), the chainsaw's ready
  loop runs `A_WeaponReady` **only every 4 tics**, so fire and weapon-change requests are sampled every 4th tic while it is ready.
- **`A_WeaponReady`** (p_pspr.c:282–334): the saw idle sound when `readyweapon == wp_chainsaw && psp->state == &states[S_SAW]` (296–300),
  presentation.
- **`A_Saw`** (p_pspr.c:502–543), draws in order: **`damage = 2*(P_Random()%10+1)`** (510, 2–20), **`angle += (P_Random()−P_Random())<<18`** (511–512,
  unspecified order, negative shift); `slope = P_AimLineAttack(mo, angle, MELEERANGE+1)`; `P_LineAttack(mo, angle, MELEERANGE+1, slope, damage)`
  (515–516). Because `attackrange == MELEERANGE+1 != MELEERANGE`, a wall hit spawns a **normal puff** (p_mobj.c:829–830 skips `S_PUFF3`): two `z`
  draws, `lastlook`, one `tics` draw (p_mobj.c:819–823). No `linetarget` → `sfx_sawful`, return (518–522). Otherwise `sfx_sawhit`, then the turn
  toward the target (526–541): with `d = angle − mo->angle` as **unsigned** `angle_t`, `if (d > ANG180)`: `if (d < −ANG90/20)` (the int
  `−0x40000000/20` converted to unsigned) `mo->angle = angle + ANG90/21` else `mo->angle −= ANG90/20`; else `if (d > ANG90/20)` `mo->angle = angle −
  ANG90/21` else `mo->angle += ANG90/20`. Finally **`mo->flags |= MF_JUSTATTACKED`** (542).
- **The lunge**: at the next `P_PlayerThink`, `MF_JUSTATTACKED` rewrites the player's command: `angleturn = 0`, **`forwardmove = 0xc800/512` (=
  100)**, `sidemove = 0`, flag cleared (p_user.c:249–255). Buttons are kept. So a chainsaw hit always produces a forward thrust of `100*2048` on the
  next tic (if on the ground and not frozen), **outside the ±50 command domain Rust admits today**.
- **No thrust on the victim**: `P_DamageMobj` skips the knock-back when `source->player-> readyweapon == wp_chainsaw` (p_inter.c:806–811).
  `pw_strength` does **not** multiply saw damage (only `A_Punch`, p_pspr.c:478–479).
- **Pickup**: `P_GiveWeapon(player, wp_chainsaw, false)` (p_inter.c:616–621): no ammo (`am_noammo`, 198–208); **if already owned it returns false and
  the item stays** (210–211, 219); otherwise owned and `pendingweapon = wp_chainsaw` (214–216). `P_BringUpWeapon` plays `sfx_sawup`
  (p_pspr.c:145–146).
- **Selection rules** (p_user.c:283–319): `newweapon = (buttons & BT_WEAPONMASK) >> 3` (288); **slot 0 (key 1) becomes `wp_chainsaw` when the chainsaw
  is owned, unless the chainsaw is ready and `pw_strength` is set** (290–296), so with a chainsaw and no berserk the fist cannot be selected by key 1.
  Slot 7 (key 8, g_game.c:342–348) selects the chainsaw directly. The supershotgun substitution needs commercial mode (298–304). Owned and not ready →
  `pendingweapon` (307–318); plasma/BFG are refused only in shareware.
- **`P_CheckAmmo`** fallback order (p_pspr.c:161–240) when the ready weapon lacks ammo: plasma (cells, not shareware) → supershotgun (commercial,
  shells > 2) → chaingun (clip) → shotgun (shells) → pistol (clip) → **chainsaw (owned)** → **missile (owned, rockets)** → BFG (cells > 40) → fist;
  then the ready weapon's down state. With a chainsaw the fist is never chosen while out of ammo.

#### 3.5.3 Rocket launcher (pickup 2003, `MT_MISC27`) and `MT_ROCKET`

- **States** (info.c:193–202): `S_MISSILE` 1 A_WeaponReady; `S_MISSILEDOWN` 1 A_Lower; `S_MISSILEUP` 1 A_Raise; **`S_MISSILE1` 8 A_GunFlash →
  `S_MISSILE2` 12 A_FireMissile → `S_MISSILE3` 0 A_ReFire → `S_MISSILE`**; flash `S_MISSILEFLASH1` 3 A_Light1 → `FLASH2` 4 → `FLASH3` 4 A_Light2 →
  `FLASH4` 4 A_Light2 → `S_LIGHTDONE`. The rocket leaves **8 tics after** `P_FireWeapon`; a held trigger refires through `A_ReFire` every 20 tics
  (p_pspr.c:343–362, no `attackdown` test there).
- **`attackdown` rule** (p_pspr.c:315–327): from the ready state, a missile launcher (or BFG) fires only if `!attackdown`; `attackdown` is set when
  firing and cleared on a ready tic without `BT_ATTACK`. So switching to the launcher with the trigger already held, or re-entering the ready state
  with it held, does not fire until it is released for one ready tic. `G_PlayerReborn` starts with `attackdown = true` (g_game.c:822).
- **`A_FireMissile`** (p_pspr.c:551–557): `ammo[am_misl]--` **then** `P_SpawnPlayerMissile (mo, MT_ROCKET)`.
- **`P_SpawnPlayerMissile`** (p_mobj.c:935–987): autoaim `P_AimLineAttack(source, an, 16*64 units)` at `an = angle`, then `an + (1<<26)`, then `an −
  (1<<26)` (net), falling back to `an = angle, slope = 0` (948–967); spawn at `z + 32` (969–973; **`lastlook` draw**); seesound; `target = source`;
  `angle = an`; `momx/momy = FixedMul(speed, cos/sin)`; **`momz = FixedMul(speed, slope)`** (980–984); `P_CheckMissileSpawn` (986: **`tics` draw**,
  half-step, `P_TryMove`, explode on failure). No shadow fuzz (players' missiles never read `MF_SHADOW`).
- **`MT_ROCKET`** (info.c:1966–1990): spawn `S_ROCKET` (1 tic, loops, info.c:250), seesound `rlaunc`, death `S_EXPLODE1`, deathsound `barexp`, **speed
  20, radius 11, height 8, damage 20**, flags `MF_NOBLOCKMAP|MF_MISSILE|MF_DROPOFF|MF_NOGRAVITY`. Death states (info.c:263–265): **`S_EXPLODE1` 8 tics
  A_Explode** → `S_EXPLODE2` 6 → `S_EXPLODE3` 4 → `S_NULL`.
- **Speed 20 splits moves**: `P_XYMovement` halves the step when a component is **positive** and above 15 (p_mobj.c:152; a negative component never
  triggers the split, `xmove/2` truncates toward zero while `xmove >>= 1` floors, 154–157). The admitted fireballs (10, 15) never split; rockets do
  for headings within about ±41° of +x or +y.
- **Explosion order** (p_mobj.c:90–105 with info.c:263): `P_ExplodeMissile` zeroes momentum, `P_SetMobjState(S_EXPLODE1)`, **which runs `A_Explode`
  immediately** (p_enemy.c:1598–1601: `P_RadiusAttack(thingy, thingy->target, 128)`), and only then draws `tics -= P_Random()&3` (min 1) and clears
  `MF_MISSILE`. So the splash damage and all its draws precede the explosion's tics draw. (The barrel's `A_Explode` sits in a later state, M3h9b
  §2.1.)
- **Direct hit then splash**: a rocket that hits a thing first damages it in `PIT_CheckThing` (`((P_Random()%8)+1)*20` = 20–160, p_map.c:323–324,
  source = the player), then `P_TryMove` fails and the explosion's `P_RadiusAttack` damages the same thing again.
- **Self damage**: `bombsource` is the player (`thingy->target`); `PIT_RadiusAttack` excludes only non-shootables and `MT_CYBORG`/`MT_SPIDER`
  (p_map.c:1170–1177), so the shooter takes `128 − dist` (dist = max(|dx|,|dy|) − radius, ≥ 0, in whole units, 1179–1189) if `P_CheckSight(thing,
  bombspot)` (1191), with `P_DamageMobj(player, rocket, player, …)`: knock-back from the rocket's position, `player->attacker = player`
  (p_inter.c:874), so `P_DeathThink` does not turn (M3h9c §2.7). Iteration is **`y` outer, `x` inner** over blocks (p_map.c:1230–1232), `bnext` order
  within a block.
- **Post-explosion half step** (p_mobj.c:150–191): when the rocket explodes in the **first half of a split move**, the `do … while (xmove || ymove)`
  loop continues: the exploded rocket (no longer `MF_MISSILE`) tries the second half-step with `P_TryMove`. As a non-missile it is now blocked by
  `ML_BLOCKING`/`ML_BLOCKMONSTERS` lines (p_map.c:214–221) and by solid things (342); on failure `momx = momy = 0` (189); on success it relinks half a
  step further on. `A_Explode` already ran, so only the exploded rocket's position changes; crossings return at once (`MT_ROCKET` filter,
  p_spec.c:508). The sky hack (176–184) removes the rocket instead of exploding it.
- **Pickups**: launcher `P_GiveWeapon(wp_missile)` gives `2 × clipammo` = 2 rockets (p_inter.c:205) and the weapon; a second launcher is picked up
  only if the rockets were below max (210–219). Rocket 2010 `P_GiveAmmo(am_misl,1)` = 1; rocket box 2046 = 5 (p_inter.c:553–563). `P_GiveAmmo`
  (73–160) doubles on baby/nightmare only; when the previous count was 0 it switches **to the launcher only from the fist** (149–154; the `case
  am_misl` falls through into `default`).
- **Noise**: `P_FireWeapon` calls `P_NoiseAlert(mo, mo)` on every shot, chainsaw and launcher included (p_pspr.c:256).

#### 3.5.4 Draws per shot

| Weapon | Draws in call order |
| --- | --- |
| Chainsaw (`A_Saw`, twice per cycle) | damage 1; angle 2; then a wall puff (2 + `lastlook` + 1 = 4) or, on a thing, blood (2 + `lastlook` + 1 = 4, p_mobj.c:847–850) and `P_DamageMobj` (no thrust → no fall-forward draw; pain draw if it survives; `P_KillMobj` tics draw if it dies) |
| Rocket launcher (`A_FireMissile`) | `lastlook` 1; spawn `tics` 1; if the spawn half-step hits a thing: hit damage 1 + `P_DamageMobj` draws; explosion (at spawn, in flight or on floor/ceiling): `A_Explode` victims' `P_DamageMobj` draws in block order, then the explosion `tics` draw 1 |

Every rocket-in-flight tic has no draw of its own.

### 3.6 Keys, powerups and other items

Power and card enums: `pw_invulnerability 0, pw_strength 1, pw_invisibility 2, pw_ironfeet 3, pw_allmap 4, pw_infrared 5` (doomdef.h:214–222);
`it_bluecard 0, it_yellowcard 1, it_redcard 2, it_blueskull 3, it_yellowskull 4, it_redskull 5` (162–171); `INVULNTICS 30*35, INVISTICS 60*35,
INFRATICS 120*35, IRONTICS 60*35` (233–239).

#### 3.6.1 Keycards (5 blue, 6 yellow, 13 red)

- `MT_MISC4`/`MT_MISC5`/`MT_MISC6` (info.c:2330–2406): radius 20, height 16, `MF_SPECIAL|MF_NOTDMATCH`, not `MF_COUNTITEM`; states
  `S_BKEY`/`S_RKEY`/`S_YKEY` 10 tics alternating (info.c:964–969), so each spawn has a **spawn-tics draw**.
- `P_TouchSpecialThing` (p_inter.c:421–443): set the message if new, `P_GiveCard`, and **in single player `break`** (removal and bonus below); in a
  netgame `return` (keys stay).
- **`P_GiveCard`** (p_inter.c:274–283): if already held, return; else **`bonuscount = BONUSADD`** (assignment, 6) and `cards[card] = 1`. Then the
  common tail adds `bonuscount += BONUSADD` (658): a new key leaves `bonuscount` 12 (or the old value + 6 if the card was already held).
- Readers: `EV_VerticalDoor` (p_doors.c:369–410) and `EV_DoLockedDoor` (219–257, absent in Episode 1). Cleared at level end (`G_PlayerFinishLevel`,
  g_game.c:786).

#### 3.6.2 Soulsphere (2013, `MT_MISC12`)

`MF_SPECIAL|MF_COUNTITEM` (info.c:2538–2562), `S_SOUL` 6-tic loop (info.c:978–983, spawn draw). Pickup (p_inter.c:400–407): `health += 100`, cap 200,
`mo->health = health`, `sfx_getpow`; **always picked up** (no `MAXHEALTH` test, unlike `P_GiveBody`).

#### 3.6.3 Partial invisibility (2024, `MT_INS`)

`MF_SPECIAL|MF_COUNTITEM` (info.c:2616–2640), `S_PINS` 6-tic loop (info.c:989–992, spawn draw). `P_GivePower(pw_invisibility)` (p_inter.c:300–305):
`powers = INVISTICS` (2100) and **`mo->flags |= MF_SHADOW`**; always succeeds (re-pickup resets the timer). Countdown in `P_PlayerThink`: `if
(powers[pw_invisibility]) if (!--powers[...]) mo->flags &= ~MF_SHADOW` (p_user.c:345–347). Gameplay effect: every monster attack that targets the
player draws extra (§3.4.4): `A_FaceTarget` +2 per call; `P_SpawnMissile` +2. Per attack cycle, counting the `A_FaceTarget` states and the calls
inside the attack actions: zombieman and shotgun guy +4 (`S_POSS_ATK1`/`S_SPOS_ATK1`, info.c:320, 353, plus the call inside
`A_PosAttack`/`A_SPosAttack`, p_enemy.c:811, 833); chaingunner +2 for `S_CPOS_ATK1` (info.c:552), +2 per `A_CPosAttack` shot (856) and +2 per
`A_CPosRefire` (868); imp +6 claw, +8 fireball (`S_TROO_ATK1`/`ATK2`, info.c:588–589, `A_TroopAttack`'s call, p_enemy.c:920, and `P_SpawnMissile`);
demon/spectre bite +6 (`S_SARG_ATK1`/`ATK2`, `A_SargAttack`'s call, 942); baron +4 claw, +6 fireball (`A_BruisAttack` has no `A_FaceTarget`). Monster
sight and target selection are unaffected.

#### 3.6.4 Radiation suit (2025, `MT_MISC14`)

`MF_SPECIAL` (not counted), `S_SUIT` tics −1 (info.c:997, no spawn draw). `P_GivePower(pw_ironfeet)`: `powers = IRONTICS` (2100), always succeeds
(p_inter.c:313–317). Readers: `P_PlayerInSpecialSector` cases 5, 7 (no draw) and 16/4 (the draw of §3.2.2). Not read by `P_DamageMobj`. Counts down
(p_user.c:352–353).

#### 3.6.5 Computer map (2026, `MT_MISC15`) and light amplification (2045, `MT_MISC16`)

- Both `MF_SPECIAL|MF_COUNTITEM`, 6-tic loops (`S_PMAP` info.c:998–1003, `S_PVIS` 1004–1005): spawn draws.
- `pw_allmap` takes `P_GivePower`'s generic tail (p_inter.c:326–330): **if already held it returns false and the item stays**; else `powers[pw_allmap]
  = 1` (never counts down; cleared at level end). No game-code reader (only am_map.c:1160).
- `pw_infrared` = `INFRATICS` (4200), always succeeds (307–311); counts down (p_user.c:349–350); read only for `fixedcolormap` (371–381,
  presentation).

#### 3.6.6 Backpack (8, `MT_MISC24`)

`MF_SPECIAL`, `S_BPAK` tics −1 (no spawn draw). Pickup (p_inter.c:589–599): **first backpack only** doubles `maxammo[0..3]` and sets `backpack`
(590–595); every backpack then calls `P_GiveAmmo(player, i, 1)` for `i = 0..3` in order: +10 clip, +4 shells, **+20 cells**, +1 rocket, each capped,
each with the zero-ammo switch rules (§3.5.3). Always picked up (the return values are ignored). `backpack` and the doubled `maxammo` persist across
levels (not cleared by `G_PlayerFinishLevel`).

#### 3.6.7 Berserk and invulnerability (not spawned in Episode 1 single player)

Neither is reachable in E1M2–E1M9 on medium single player (E1M1's berserk is `MTF_NOTSINGLE`); out of scope and not recorded here.

#### 3.6.8 Power timers and level end

`P_PlayerThink` order after `P_MovePsprites` (p_user.c:339–359): strength++, invulnerability−−, invisibility−− (clearing `MF_SHADOW` at 0),
infrared−−, ironfeet−−, `damagecount−−`, `bonuscount−−`; then `fixedcolormap` (363–383, presentation). No draw. `G_PlayerFinishLevel`
(g_game.c:779–792) zeroes `powers` and `cards`, clears `MF_SHADOW`, `extralight`, `fixedcolormap`, `damagecount`, `bonuscount` at every exit.

### 3.7 `P_TouchSpecialThing` for the new sprites

`P_TouchSpecialThing` (p_inter.c:339–661) is reached from `PIT_CheckThing` when the moving thing has `MF_PICKUP` (p_map.c:331–340: only the player;
the call happens for every overlapping `MF_SPECIAL` thing during `P_CheckPosition`, including the failed and height-clip cases of M3h8 §2.7) and
returns `!solid`.

- **Reach test** (348–355): `delta = special->z − toucher->z`; out of reach if `delta > toucher->height` or `delta < −8 units`. Dead toucher returns
  (363–364).
- Dispatch by **sprite** (367): new cases for Episode 1: `SPR_SOUL` (400), `SPR_BKEY` (421), `SPR_YKEY` (429), `SPR_RKEY` (437), `SPR_PINS` (504),
  `SPR_SUIT` (511), `SPR_PMAP` (518), `SPR_PVIS` (525), `SPR_ROCK` (553), `SPR_BROK` (559), `SPR_BPAK` (589), `SPR_CSAW` (616), `SPR_LAUN` (623).
  Unknown sprite → `I_Error` (651–652).
- Refusals (item stays, no tail): chainsaw already owned; launcher owned with rockets at max; rocket/rocket box at max; computer map already held.
- **Common tail** (655–660): `if (special->flags & MF_COUNTITEM) player->itemcount++` (soulsphere, invisibility, computer map, light amp, plus the
  admitted bonuses); `P_RemoveMobj (special)`; `bonuscount += BONUSADD`; the sound for the console player. The `message` assignments and sounds are
  presentation.
- No draw on any pickup path (`P_RemoveMobj` draws nothing).

### 3.8 Decorations and other new spawned types

| Doomednum | Type | info.c | Flags | Radius / height | Spawn state, tics | Spawn draws |
| --- | --- | --- | --- | --- | --- | --- |
| 14 | `MT_TELEPORTMAN` | 2174–2198 | `NOBLOCKMAP`, `NOSECTOR` | 20 / 16 | `S_NULL`, −1 | `lastlook` |
| 34 | `MT_MISC49` candle | 3682–3706 | none | 20 / 16 | `S_CANDLESTIK`, −1 (info.c:1047) | `lastlook` |
| 46 | `MT_MISC43` tall red firestick | 3526–3550 | `SOLID` | 16 / 16 | `S_REDTORCH`, 4-tic loop (info.c:1070–1073) | `lastlook`, spawn-tics |

- No Episode 1 decoration has `MF_SPAWNCEILING` (`P_SpawnMapThing`'s `ONCEILINGZ` branch, p_mobj.c:779–782, stays unused) or `MF_NOGRAVITY`. The
  firestick, like E1M1's solid decorations, has height 16 in info.c (the rendered sprite is taller); collision uses info.c (M3h9 §2.5–2.6).
- The candle is non-solid and non-special: `PIT_CheckThing` ignores it (p_map.c:258–259); it is still linked in its block list and re-clipped by
  `P_ChangeSector`.
- New pickups' spawn draws: keys, soulsphere, invisibility, computer map and light amp have a spawn-tics draw (tics 6 or 10); suit, rocket, rocket
  box, backpack, chainsaw and launcher do not (tics −1). Spectre and baron: spawn-tics draw. These enter each map's level-start draw count (the "144"
  of M3h9c §2.5).

### 3.9 Sound propagation and alerts

- No new `P_NoiseAlert` caller: its only caller is `P_FireWeapon` (p_pspr.c:256), so chainsaw and launcher shots alert like every weapon (also on a
  refire through `A_ReFire`). Teleport fog sounds and switch sounds are `S_StartSound` (no alert).
- `A_Look` (p_enemy.c:604–664) is unchanged: `threshold = 0`, `soundtarget` of the monster's current sector, `MF_AMBUSH` requires `P_CheckSight`. A
  teleported monster keeps `target`; its new sector's `soundtarget` applies only while it is still in a look state. `A_FaceTarget` clears `MF_AMBUSH`
  (787).
- `P_RecursiveSound` crosses two-sided lines with an opening (M3h4 §2.8); doors closing or opening change the opening and therefore the flood (a
  timing fact, no new code).

### 3.10 The command domain (vanilla `ticcmd_t`)

#### 3.10.1 Layout and sources

`ticcmd_t` (d_ticcmd.h:36–44): `char forwardmove; char sidemove; short angleturn; short consistancy; byte chatchar; byte buttons;` (`char` is signed
on the x86 targets). Playback reads `forwardmove`/`sidemove` as signed bytes, `angleturn = byte << 8`, buttons as a byte (g_game.c:1491–1503, M3h9c
§2.1). A demo byte can encode **−128..127** for `sidemove` and **−127..127** for `forwardmove` (0x80 there is the end marker), any multiple of 256 for
`angleturn`, and any byte for `buttons`.

#### 3.10.2 `G_BuildTiccmd` ranges (live input; g_game.c:237–437)

- `forwardmove[2] = {0x19, 0x32}` (25, 50), `sidemove[2] = {0x18, 0x28}` (24, 40), `angleturn[3] = {640, 1280, 320}` (slow turn for the first
  `SLOWTURNTICS` 6 tics) (g_game.c:175–179). `MAXPLMOVE` is `forwardmove[1]` (171).
- Keys add `±forwardmove[speed]`/`±sidemove[speed]` (278–325); mouse adds `mousey` to forward and `mousex*2` to side (strafe) or `−mousex*8` to
  `angleturn` (405–409); then **both `forward` and `side` are clamped to ±`MAXPLMOVE`** (413–420, side uses the forward maximum) and added to the base
  command (422–423). Joystick likewise.
- **`-turbo`** (d_main.c:885–902): scale 10–400 % (default 200), `forwardmove[i] = forwardmove[i]*scale/100`, likewise `sidemove`; `MAXPLMOVE` scales
  with it. At scale ≥ 255 the clamp exceeds 127 and the store into `char` wraps (scale 400: 200 → −56). Live keyboard input therefore produces |move|
  ≤ 50 without turbo, ≤ 127 with turbo ≤ 254 %, wrapped values above.
- Weapon keys `'1'..'8'` → `BT_CHANGE | i << 3` for `i = 0..7` (342–348): **slot 7 (key 8) is `wp_chainsaw`**.
- Pause and save inject `BT_SPECIAL | BTS_PAUSE` or `BT_SPECIAL | BTS_SAVEGAME | slot << 2`, replacing all buttons (426–436).

#### 3.10.3 `G_Ticker`: turbo check, `BT_SPECIAL`, pause

- **Turbo message** (g_game.c:672–679): `forwardmove > TURBOTHRESHOLD (0x32)` and `!(gametic&31)` and `((gametic>>5)&3) == i` →
  `players[consoleplayer].message = "<name> is turbo!"`. Keyed to `gametic`, not `leveltime`. It runs during demo playback too (the check follows `G_ReadDemoTiccmd`), and it writes the message field that a DSIM tic captures, so it is **not** presentation-only for the comparison: see the correction in [§14](#14-corrections-and-findings-from-phase-2).
- **`BT_SPECIAL`** (698–724), processed **before** `P_Ticker` (730): `buttons & BT_SPECIALMASK` (3): `BTS_PAUSE` (1) toggles `paused` with
  `S_PauseSound`/ `S_ResumeSound`; `BTS_SAVEGAME` (2) sets `savegameslot = (buttons & 28) >> 2` and `gameaction = ga_savegame`; values 0 and 3 do
  nothing.
- **Pause**: `P_Ticker` returns at once when `paused` (p_tick.c:135–136): no `P_PlayerThink`, thinkers, specials or `leveltime++`. `G_Ticker` still
  reads one ticcmd per tic (playback consumes the demo), `ST_Ticker`/`AM_Ticker`/`HU_Ticker` still run, and `gametic` still advances
  (D_DoomLoop/TryRunTics). So the **pause tic itself does not simulate** (the toggle precedes `P_Ticker`), each paused tic consumes a command, and the
  **unpause tic simulates** with its buttons cleared by `P_PlayerThink` (p_user.c:280–281) but its `forwardmove`, `sidemove` and `angleturn` applied.
  `gametic` is read by gameplay only in `A_Tracer` (M3h9c §2.6), so pause has no other gameplay effect. `G_InitNew` and `G_DoLoadLevel` clear `paused`
  (M3h9c §2.2).
- **Save during playback**: the next `G_Ticker`'s gameaction loop runs `G_DoSaveGame` (g_game.c:1270–1320), which writes `doomsav<slot>.dsg` and sets
  the `GGSAVED` message; the archive functions copy players, world, thinkers and specials into the buffer before rewriting pointers **in the copy**
  (p_saveg.c:60–68, 245–250, 381–461), so the simulation state is unchanged. The save tic itself still simulates with buttons cleared. Under LP64 the
  archive's pointer casts truncate (M3h9c §2.10); a harness must stub the file write.

#### 3.10.4 `P_PlayerThink` and `P_MovePlayer` with the full range

- `P_MovePlayer` (p_user.c:148–171): `mo->angle += angleturn << 16` (154; any 16-bit value, negative shift as in M3h9c §2.10); `onground = (z <=
  floorz)` (158); **forward and side thrusts are both applied in the same tic** if nonzero and on ground: `P_Thrust(angle, forwardmove*2048)` then
  `P_Thrust(angle − ANG90, sidemove*2048)` (160–164); the run state if any move is nonzero and the state is `S_PLAY` (166–170). Turning is applied
  before the thrusts, so the thrust uses the new angle.
- `P_Thrust` (p_user.c:59–68): `momx += FixedMul(move, finecosine[angle>>19])`, `momy` likewise. With |move| ≤ 128·2048 = 262144 (4 units), `FixedMul`
  cannot overflow, and the sum with |mom| ≤ `MAXMOVE` stays far inside 32 bits: **the whole −128..127 range is overflow-free**.
- Speeds: friction `FRICTION` 0xe800 (p_mobj.c:112) gives a terminal speed `v = t·f/(1−f)` for thrust `t`: ≈ 14.75 units/tic at forward 50, ≈ 37.5 at
  127, so **turbo or demo values ≥ ~51 per axis reach `MAXMOVE` 30**, which `P_XYMovement` clamps per axis (p_mobj.c:137–145) and splits above 15
  (positive direction only, 152). The chainsaw lunge (forward 100) is in this range (§3.5.2).
- `BT_SPECIAL` clears all buttons before weapon change and use (p_user.c:280–281).
- `BT_CHANGE` (283–319): weapon bits 0–7 as in §3.5.2; ignored if not owned or already ready; the change happens at the next `A_WeaponReady` (or
  immediately through `A_ReFire`/`P_CheckAmmo` paths).
- **Use** (322–331): `P_UseLines` only on a press edge (`!usedown`), `usedown` cleared on a tic without `BT_USE`. **Attack**: `attackdown` handled in
  `A_WeaponReady` (§3.5.3).
- Button bit 64 is read nowhere in game code (`BT_WEAPONMASK` is 8+16+32, d_event.h:87): ignored in vanilla.

#### 3.10.5 What the Rust domain would have to lift (from M3h9c §4.2)

| Field | Wire 22 (`player_commands.rs`) | Vanilla, as reachable from an original demo |
| --- | --- | --- |
| `forward` | −50..50 | −127..127 (−128 impossible: marker) |
| `side` | −50..50 | −128..127 |
| internal forward | — | 100 for the chainsaw lunge, independent of the command |
| `angle_turn` | any | any (demo: multiples of 256) |
| `BT_CHANGE` weapon 4–7 | `UnsupportedWeapon` | 4 launcher, 7 chainsaw selectable when owned; 5, 6 ignored (never owned in Episode 1) |
| weapon bits without `BT_CHANGE`, bit 64 | `UnsupportedButtons` | ignored |
| `BT_SPECIAL` | `UnsupportedButtons` | pause toggle / save request; buttons cleared in `P_PlayerThink` |

### 3.11 Level transitions, intermission and finale (what original demos need beyond the maps)

#### 3.11.1 Normal and secret exits (E1M1–E1M7, E1M9)

1. Exit tic: `G_ExitLevel` / `G_SecretExitLevel` set `gameaction = ga_completed` (and `secretexit`); the rest of that `P_Ticker` runs (M3h9c §2.8). A
   player killed later in the same tic is `PST_DEAD` at transition time.
2. Next `G_Ticker`: reborn loop (g_game.c:612–614; single player: `G_DoReborn` → reload only for `PST_REBORN`), then **`G_DoCompleted`** (1020–1141):
   `G_PlayerFinishLevel` per player (§3.6.8); `AM_Stop`; **map 8 (non-commercial) → `gameaction = ga_victory` and return** (1033–1038, §3.11.2); map 9
   sets `didsecret` (1039–1042, 1054–1060); `wminfo.next` = 8 after a secret exit (E1M9), **3 when leaving map 9 in episode 1 (i.e. E1M4)**, else
   `gamemap` (1087–1110); par time `pars[1][map]`; per-player kills/items/ secrets/`leveltime`; `gamestate = GS_INTERMISSION`; `WI_Start` (1140). The
   same `G_Ticker` then reads a ticcmd and runs `WI_Ticker`.
3. **Intermission** (wi_stuff.c): `WI_Ticker` (1503–1536) runs `WI_checkForAccelerate` (1470–1498), which **writes `player->attackdown` and
   `player->usedown`** from the command (edge-detected: a press while not down sets `acceleratestage`), then the single-player state machine
   `WI_updateStats` (1330–1434; counts advance by 2 % per tic with 35-tic pauses, `acceleratestage` jumps to the end), `WI_updateShowNextLoc`
   (761–769; 4·35 tics or acceleration) and `WI_updateNoState` (737–747; 10 tics, then `WI_End` and **`G_WorldDone`**). The number of intermission
   tics is a deterministic function of kills/items/secrets percentages (`maxkills` etc. forced ≥ 1, wi_stuff.c:1825–1831), `leveltime`, par time and
   the BT_ATTACK/BT_USE edges. Only `M_Random` is drawn (animated backgrounds, 523–561); **no `P_Random`**.
4. **`G_WorldDone`** (g_game.c:1147–1170): `gameaction = ga_worlddone`, `didsecret` after a secret exit. Next `G_Ticker`: **`G_DoWorldDone`**
   (1172–1179): `gamestate = GS_LEVEL`, `gamemap = wminfo.next + 1`, **`G_DoLoadLevel ()`**, then `P_Ticker` in the same `G_Ticker`.
5. **What carries over** (`G_DoLoadLevel` → `P_SetupLevel` → `P_SpawnPlayer`): `G_DoLoadLevel` turns only a `PST_DEAD` player into `PST_REBORN`
   (g_game.c:479–480) and clears `frags` (481); `P_SetupLevel` zeroes `killcount`, `secretcount`, `itemcount`, `totalkills/items/secret` and
   `leveltime` (p_setup.c:594–600, 646); `P_SpawnPlayer` calls `G_PlayerReborn` **only for `PST_REBORN`** (p_mobj.c:659–660). So a living player keeps
   `health`, `armorpoints`, `armortype`, weapons, ammo, `maxammo`, `backpack`, `readyweapon`, `usedown` and `attackdown` (as left by the
   intermission); `refire`, `damagecount`, `bonuscount` and the view height are reset (p_mobj.c:675–683), and gets a new mobj with `health =
   p->health` (673), a `lastlook` draw, `P_SetupPsprites` (the weapon rises from the bottom). Powers, cards and `MF_SHADOW` were cleared at exit. A
   player who died in the exit tic restarts the next map with a pistol (`G_PlayerReborn`). `player->attacker` is left pointing at a freed mobj of the
   previous level, but it is rewritten by every `P_DamageMobj` before `P_DeathThink` could read it.
6. **The RNG is not reseeded** between levels (`M_ClearRandom` only in `G_InitNew`, M3h9c §2.2): the next level's spawn draws continue the sequence.

#### 3.11.2 E1M8: finale, no next level

`G_DoCompleted` returns with `gameaction = ga_victory` before any intermission (g_game.c:1033–1038); the `while (gameaction)` loop of the same
`G_Ticker` calls `F_StartFinale` (639–641), which sets `gameaction = ga_nothing`, `gamestate = GS_FINALE` (f_finale.c:98–101). `F_Ticker`
(f_finale.c:207–249) in non-commercial modes only advances `finalecount`/`finalestage`; **nothing ever sets `ga_worlddone`**, so an episode ends at
E1M8 and E1M9 is reachable only through E1M3's secret exit (51). (This refines M3h9c §2.8, which describes `G_DoCompleted` reaching `WI_Start`: true
for every map but 8.)

#### 3.11.3 Demo-version dependence

`G_DoPlayDemo` (g_game.c:1582–1620) reads the version byte once and compares it with `VERSION` (1589); no other statement in `G_DoPlayDemo`,
`G_ReadDemoTiccmd` (1491–1503) or `G_CheckDemoStatus` reads the version. The 13-byte header and 4-byte ticcmds are the only format. (Gameplay
differences between the 1.9 executables and the 1.10 source, if any, are outside the source and not observable here.)

### 3.12 Size estimate per group

The calibration and the per-group estimates are in [§3.11](#11-size-estimate-and-the-unattended-rule-verdict).

### 3.13 Unchecked bounds, undefined reads and hazards a Rust witness must guard

| Item | C behaviour | Episode 1 status |
| --- | --- | --- |
| `spechit[MAXSPECIALCROSS 8]` (p_map.c:67–70, write 240–244) | **unchecked**; overflow writes past the array into `numspechit` and following globals | 2×2-window bound 14 on E1M5, 9 on E1M9 (§3.0.4): a per-call witness (`numspechit < 8` before each append) or a sharper load rule is needed |
| `intercepts[MAXINTERCEPTS 128]` (p_local.h:154, p_maputl.c:544, 603–606, 668–671) | **unchecked** | M3h6 §4.4 bound; rockets add `P_AimLineAttack` traces (range 1024) and the saw adds 65-unit traces — same bound argument |
| `linespeciallist[MAXLINEANIMS 64]` (p_spec.c:141, 1343–1344) | **unchecked** at load | ≤ 8 scrollers per Episode 1 map: unreachable |
| `activeplats[MAXPLATS 30]` (p_plats.c:288–299) | checked (`I_Error`) | ≤ 9 plat sectors: unreachable |
| `activeceilings[MAXCEILINGS 30]` (p_ceilng.c:251–262) | **silently drops** the 31st (unremovable thinker) | no ceilings in Episode 1 |
| `buttonlist[MAXBUTTONS 16]` (p_switch.c:155–190) | checked (`I_Error`), deduplicated per line | ≤ 10 reusable switch lines: unreachable |
| `heightlist[20]` in `P_FindNextHighestFloor` (p_spec.c:326–375) | bounded write, **truncated search** after the 20th qualifying neighbour | witness or load rule: < 20 qualifying neighbours per target of 18, 20, 22 |
| Teleport thinker scan (p_telept.c:75–128) | no array; `O(tagged sectors × thinkers)` per crossing | a work budget per crossing |
| Stair thinkers' `type`, `crush` (p_floor.c:485–504, 538–547) | **uninitialised** `Z_Malloc` fields read every tic (`crush`) and at `pastdest` (`type`) | declared C adapter (zero-filled `PU_LEVSPEC`) plus a "no stair step with `nofit`" witness (§3.1.5) |
| raiseToNearestAndChange `low` (p_plats.c:185–195, 109) | **uninitialised**, read only after the plat is blocked while rising | witness: refuse a blocked rise for that type (§3.1.7) |
| `EV_DoDonut` (p_spec.c:1184–1190) | null dereference for a one-sided first line of `s1` or a one-sided `s2` line reached first; precedence bug in the two-sided test; `s2` overwritten without a busy test | load rule per donut line; witness for a busy `s2` (§3.1.6) |
| `EV_VerticalDoor` busy sector (p_doors.c:416–437, 459–461) | casts any `specialdata` to `vldoor_t` (1, 26–28); creates a second door over a busy sector (31–34) | witness: refuse DR use on a non-door owner and D1 use on any busy sector; E1M4 has one multi-family sector (§3.0.4) |
| `EV_VerticalDoor` back side (p_doors.c:413) | `sides[-1]` for a one-sided line | Episode 1 has no one-sided manual door (§3.0.4); keep the load rule |
| `ML_TWOSIDED` without a back sector | `getNextSector` and `EV_BuildStairs` trust the flag (p_spec.c:255–261; p_floor.c:516–528) | load rule |
| `(P_Random()−P_Random())<<k` sites (p_enemy.c:795; p_mobj.c:910; p_pspr.c:512; p_map.c:1307–1308) | unspecified operand order, negative left shift | the declared ordered-draw adapter (docs/M3D1_REFERENCE.md, docs/M3D2B_REFERENCE.md), with each new site observed in the genuine build |
| Telefrag knock-back `damage*(FRACUNIT>>3)*100` (p_inter.c:817) | **signed overflow** for damage ≥ 2622 (telefrag 10000) | declared wrapping adapter; observe the genuine value (§3.3.6) |
| `P_GiveAmmo` bound test `ammo > NUMAMMO` (p_inter.c:84) | off by one (admits `NUMAMMO`) | unreachable (callers pass 0–3) |
| `G_DoSaveGame` during playback (g_game.c:1270–1320) | writes a file; LP64 pointer truncation in the archive | harness stub; no simulation effect |

### 3.14 Refinements to earlier notes

1. **M3h9c §2.8**: `G_DoCompleted` reaches `WI_Start` for every map except 8; on E1M8 it sets `ga_victory` and the finale never returns to a level
   (§3.11.2).
2. **M3h8 §2.4**: `EV_VerticalDoor`'s busy-sector branch returns only for 1, 26–28, 117; D1 specials 31–34 fall through and create a second thinker
   (§3.1.3). The D1 special is cleared only when a thinker is created.
3. **M3h8 §2.6**: `newspecial`/`texture` are uninitialised for all `EV_DoFloor` types used in Episode 1, and stair thinkers also leave `type` and
   `crush` uninitialised; the latter are read (§3.1.5).
4. **M3h8 §2.3**: `P_ShootSpecialLine` is also reached by monsters' hitscans; for 46 it acts (§3.1.1).
5. **`numspechit` after a teleport** is −1 until the next `P_CheckPosition` (§3.3.3).
6. The rocket's `A_Explode` runs **before** its explosion `tics` draw, unlike the barrel (§3.5.3).
7. Counts in §3.0 come from the per-slot `--things SLOT` census as well as the three markdown reports.

## 4. Rust inventory (wire 22 baseline; what wire 23 and DSIM39 must touch)

Read-only survey of the tree at `0ecfe4b`; every citation is `path:line`. The IWAD was not read; per-map counts come only from the committed census
records.

### 4.1 Admitted sets under wire 22

Wire 22 is `PlayerMonstersLevelV1`, defined by predicate membership in `crates/doom-sim/src/original_registry.rs` (enum variant :96, `wire_id` :123,
`context_schema` :150, predicates :160–458); it is in every `has_*` predicate and `has_level` (:334–336) is wire 22 alone. THING admission is
`Simulation::spawn_map_things` (crates/doom-sim/src/spawning.rs:425–640).

| Class | DoomEd → MT_ | Gate (file:line) | Bound | Over-bound error |
| --- | --- | --- | --- | --- |
| Player start | 1 (the body is spawned at its lump position) | spawning.rs:450–475 | exactly one type-1; a second → `PlayerBody(Binding)` :463; none → :628–630 | `PlayerBodyError::Binding` |
| Co-op / DM starts | 2, 3, 4 stored (:450–452); 11, first 10 stored (:444–449) | spawning.rs | never spawned | — |
| Former humans | 3004 `MT_POSSESSED`, 9 `MT_SHOTGUY`, 65 `MT_CHAINGUY` | `FORMER_HUMANS` original_registry.rs:610–614; `is_policy_monster` :730–734 | roster `MAX_PLAYER_MONSTERS` = **8** (player_monsters.rs:12; `map_demon_limit` player_roster.rs:82–90) | `MapSpawnError::WorkLimit` spawning.rs:569 |
| Imp | 3001 `MT_TROOP` | `is_imp` original_registry.rs:714 | inside the 8 | same |
| Demon | 3002 `MT_SERGEANT` (owner slot, original_registry.rs:748–750; other listed types rewritten to the owner at spawning.rs:589–591) | spawning.rs:516–521 | inside the 8 | same |
| Pickups (12 kinds) | 2007, 2048, 2008, 2049, 2011, 2012, 2001, 2002, 2018, 2019, 2014, 2015 | `PickupKind` world_pickups.rs:37–80; `kind_for` :130–139 | `BARRELS_MAX_PICKUPS` = **64** (world_pickups.rs:18, `max_pickups` :22–28) plus 8 drops (`max_world_items` :927–933) | `WorkLimit` spawning.rs:569 |
| Static decorations (7) | 2028, 48, 35, 24, 15, 10, 12 | `E1M1_DECORATIONS` e1m1_rules.rs:44–52; `is_decoration` :124–126 | `MAX_DECORATIONS` = **32** (e1m1_rules.rs:56) | `DecorationLimit` spawning.rs:541 |
| Barrel | 2035 `MT_BARREL` | `is_barrel` barrels.rs:73–79 | `MAX_BARRELS` = **16** (e1m1_rules.rs:111) | `BarrelLimit` spawning.rs:550 |
| Anything else | — | spawning.rs:509–523 | — | `UnsupportedType(kind)` :522; unknown doomednum `ActorError::InvalidType` :488–490 |

Runtime-only types: `MT_TROOPSHOT` (projectiles.rs:10), `MT_PUFF`, `MT_BLOOD` (`is_impact` original_registry.rs:722–724), the three drops
(`P_KillMobj`) and the `S_GIBS` state. Every angle is admitted (`has_barrels`, spawning.rs:528–533); skill is fixed medium (`ActorOptions::default()`
required, original_registry.rs:1083–1091); nightmare is refused (demon_expanded_map.rs:75–77). Multiplayer-only records (rocket launcher 2003, rocket
box 2046 and berserk 2023 on E1M1) are filtered at spawning.rs:482–485 before type admission. The census tool's rendering of the same set is
`WIRE22_THINGS` in `tools/iwad_counts.py`.

**Line specials** are admitted by `mover_rules::validate_movers_with` (mover_rules.rs:267–358, through `validate_e1m1` :263 from
demon_expanded_map.rs:83–91): 0, 1 (DR door, use_lines.rs → movers.rs:334), 11 (S1 exit, use_lines.rs:182–197, event `MoverEvent::Exit{secret:false}`
:196), 36 and 38 (W1 floors, line_triggers.rs:149–189 → movers.rs:423), 48 (scroller, mover_rules.rs:292; at most 64, `MAX_LINE_ANIMS`
sector_thinkers.rs:22), 63 (SR door → movers.rs:289), 88 (WR lift → movers.rs:383) and 109 (W1 door `blazeOpen` → movers.rs:289). Any other special is
`PursuitError::Specials` (mover_rules.rs:306). The other load rules, each a field-less `MapScaleError`: DR without a back side `DoorLine` (:294), two
families in one sector `MoverFamily` (:318), a sky-ceilinged mover `SkyMover` (:331), no neighbour `MoverNeighbor` (:334), a self-referencing door
`SelfReferencingDoor` (:337), `MoverBlockBox` (:347), more than 30 plats or 16 SR switch lines `MoverCount` (:351; `MAX_PLATS` :24, `MAX_BUTTONS`
:26), reachable heights beyond ±512 `MoverHeights` (:458) and `InvertingDoor` (:475), a 2×2 special-line window above 8 `SpecialWindow` (:355;
`MAX_SPECIAL_CROSS` :28), and a switch texture that is not an Episode 1 `switchlist` name `SwitchTexture` (:283; `EPISODE1_SWITCHES` :72–92). **No
`P_ShootSpecialLine` exists anywhere** (only a doc mention, mover_rules.rs:254).

**Sector specials:** `ADMITTED_SECTOR_SPECIALS = [1, 2, 3, 4, 5, 7, 8, 9, 12, 13, 16, 17]` (sector_thinkers.rs:25), checked by
`validate_sector_specials` (demon_expanded_map.rs:279–289). Light thinkers are spawned in `spawn_level_specials` (sector_thinkers.rs:~329–431; 10, 11
and 14 refused again at :420–421). `P_PlayerInSpecialSector` (sector_thinkers.rs:473–546): 5 → 10 damage, 7 → 5, 4 and 16 → 20, 9 secret; **11 is
refused at :528**. `ironfeet` is the constant `false` (:467–469); a radiation suit would need the `P_Random() < 5` draw and is refused at :506–508.

**Weapons:** the `Weapon` enum (weapons.rs:18–37) names all nine but only 0–3 execute: `Weapon::from_number` (weapons.rs:39–47) →
`WeaponError::UnsupportedWeapon` for 4–8, `Weapon::states` is `unreachable!` for the others (:86), `Weapon::ammo` knows only clip and shell (:48–54).
`WeaponInventory::validate` (weapons.rs:148–165): `owned & !15 == 0`, cells and rockets must be 0. `PlayerWeaponState` (player_weapons.rs:11–32)
stores only `bullets` and `shells` (u16); `max_ammo` is the constant `AMMO_CAPACITIES = [200, 50, 300, 50]` (weapons.rs:5, player_weapons.rs:59).
Implemented fire actions: `A_Punch`, `A_FirePistol`, `A_FireShotgun`, `A_FireCGun`, `A_Light0..2`, `A_WeaponReady`, `A_ReFire`, `A_Raise`, `A_Lower`
(actions.rs:20–47).

### 4.2 Version-ladder sites

M3h9c's commit `4e64d08` (`feat: PlayerMonstersLevelV1 (wire 22, DSIM38)`, 14 files, +916/−98) is the pattern: (1) registry: a new enum variant,
`wire_id` and `context_schema` arms, the variant added to every inherited `has_*` list (18) and the two registry `match` arms (`new` :804, `validate`
:859); (2) a context constant and constructor (player_monsters.rs: `PLAYER_LEVEL_CONTEXT_SCHEMA` :39, `original_player_level_map` :248–261 using
`player_map_scale::actor_slots(policy)`), re-exported in lib.rs:47–50; (3) roster lists (player_roster.rs:54, 144); (4) behavior behind `has_level()`
at 7 call sites (player_commands.rs:118, demon_map_spawn.rs:144, original_player.rs:308, demon_expanded_map.rs:792, snapshot.rs:718,
world_pickups.rs:424, spawning.rs:443); (5) the snapshot version selector (snapshot.rs:716–720, newest predicate first) and 14 `matches!(version,
a..=37)` ranges widened to `..=38`; (6) the diagnostic reader (`decode_level` via `decode_selected(.., Some(38))` diagnostic.rs:283–288, the
`policy()` arm :470, the event validators' policy arms :2591 and :2639, `slot_bound_for`, `pickup_bound_for` and `bonus_bound_for` :2254–2280, ~45
range widenings and the new per-capture witness `validate_level_body`); (7) tests.

| Site group | Where | Lines |
| --- | --- | ---: |
| Enum variant, `wire_id`, `context_schema`, 18 predicate lists, 2 registry arms | original_registry.rs:96, 123, 150; 168–454; 804, 859 | 23 |
| Roster, context constant and constructor, re-export | player_roster.rs:54, 144; player_monsters.rs:39, 248; lib.rs:47–50 | 7 |
| Snapshot selector and ranges | snapshot.rs:719 (+716), 760, 768, 775, 813, 837, 975, 1043, 1058, 1081, 1140, 1147, 1172, 1186, 1195 | 15 |
| Diagnostic reader, policy mapping, ranges, bounds | diagnostic.rs:270–288 … 2639 | 53 |
| **Total Rust ladder lines** | | **95** (plus 7 `has_level()` gates and 9 `has_barrels()`-keyed bounds) |

The nine `has_barrels()`-keyed bounds (world_pickups.rs:23 `max_pickups`, player_map_scale.rs:24 `actor_slots`, spawning.rs:529, barrels.rs:73 and
five more) are bounds wire 23 must **override**, not inherit. Tools and CI sites: level_snapshot.py:8–9, 115 (VERSION 38, PROFILE 22, CONTEXT);
level_compare.py:174 (`--map`); check_player_level.py; tools/seed_player_level_fuzz.py:24 (`DEMO_HEADER`); fuzz/Cargo.toml:362–363;
.github/workflows/rust.yml:246–269; docs (STATE_FORMAT §DSIM v38 at :2032, COMPATIBILITY, TESTING :1606–1665, ARCHITECTURE).

**Proposed names** (the decision is [H2](#10-new-owner-decisions-h1h12-with-recommendations)): `OriginalExecutionProfile::PlayerMonstersEpisode1V1`,
wire 23, DSIM 39, context `PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1` (the digit is fused because `EPISODE-1-` would read as "version 1" by analogy with
`LEVEL-1-`), predicate `has_episode1()`, constructor `Simulation::original_player_episode1_map`, reader `Diagnostic::decode_episode1`, fuzz target
`player_episode1.rs`, seeder `tools/seed_player_episode1_fuzz.py`, encoder `tools/reference/episode1_snapshot.py`. `has_level()` becomes true for wire
23 (a strict superset), and every new bound gets its own `const fn` keyed on `has_episode1()` like `max_pickups` and `actor_slots`.

### 4.3 Actor and state machine

- **Tables:** generated `SOURCE_STATES`, `SOURCE_TYPES`, `SOURCE_ACTIONS`, `SOURCE_SOUNDS` in crates/doom-sim/src/source_actors.rs (included at
  lifecycle.rs:8); typed identities in identity.rs and identity/names.rs; `ActorRegistry::vanilla()` (lifecycle.rs:~266–290) compiles them into
  `ActorStateDef { tics, action, next, misc }` (:45–61) with `ActorAction::{None, Original(ActionId), Probe}` (:36–43). `actions::OriginalAction`
  (actions.rs:10–47) is a compile-time name-to-ordinal table; naming a variant grants nothing.
- **Admission, "unsupported actions must reject":** (1) per-type closure policies in original_registry.rs (`ActorPolicy::source` :545–575,
  `demon_policy` :578–608, `former_human_policy` :624–661, `imp_policy` :663–693, `fireball_policy` :695–709, `player_policy` :1268): every reachable
  state's action must be in an explicit list or `OriginalRegistryError::Graph`; (2) `ActorRegistry::validate_state_with` (lifecycle.rs:403–478): an
  unlisted action is `ActorError::UnsupportedAction(id)` (:463), then `immediate_cycle` (:480–503, `ENTRY_LIMIT` 128 :262) and `flow` (:504ff);
  `SpecialActions` (:253–263) selects Demon/Chaingunner/OriginalMonsters/Barrel; (3) dispatch `LifeHost::action` (lifecycle.rs:1516–1631) ends in
  `unreachable!("unsupported action rejected before mutation")` (:1629).
- **Flags:** `ALLOWED_FLAGS = 2|4|8|16|32|256|512|1024|2048|262144|33554432` (SOLID, SHOOTABLE, NOSECTOR, NOBLOCKMAP, AMBUSH, SPAWNCEILING, NOGRAVITY,
  DROPOFF, PICKUP, SHADOW, NOTDMATCH) at lifecycle.rs:224, extended per profile at :570, 657, 680, 710, 738, 764, 794, 824, 883, 922. `MF_FLOAT`,
  `MF_SKULLFLY`, `MF_NOCLIP` and `MF_TELEPORT` are absent, so `MT_HEAD` and `MT_SKULL` are refused at type admission (both are absent from Episode 1
  on medium, so no work is needed for them).

**Actions present and missing for Episode 1.** Present: `A_Look`, `A_Chase`, `A_FaceTarget`, `A_Pain`, `A_Scream`, `A_XScream`, `A_Fall`,
`A_PlayerScream` (lifecycle.rs:1531–1627), `A_SargAttack` (demon.rs `strike`; the Spectre has the same states), `A_PosAttack`, `A_SPosAttack`,
`A_CPosAttack`, `A_CPosRefire`, `A_TroopAttack`, the admitted weapon actions, and `A_Explode`, which is **barrel-only** (admission at
lifecycle.rs:446–448 for `S_BEXP4`, runtime check barrels.rs:299–305) and so must serve `MT_ROCKET`'s `S_EXPLODE1`. Missing and needed: `A_Saw`
(chainsaw), `A_FireMissile` and `A_GunFlash` (rocket launcher), `A_BruisAttack` (Baron) and `A_BossDeath` (E1M8, tag 666). Missing and not needed on
medium: `A_SkullAttack`, `A_HeadAttack`, `A_FirePlasma`, `A_BFGsound`, `A_FireBFG`, `A_BFGSpray` and the Doom II actions. Decorations with animation
(tics above 0, no action, cycling) are not on the admitted list: `static_decoration` (e1m1_rules.rs:138–160) requires tics −1; the tall red firestick
(46) has a 4-tic loop and so needs the list extended.

**Movement paths.** `P_Move` ignores `MF_FLOAT` and sets `z = floor` unconditionally (pursuit.rs:164–211, :197; comment :206 "no admitted floater");
`P_TryMove` computes `floatok` and never consumes it (multisector.rs:106–107, 412, 464); non-missile `P_XYMovement`/`P_ZMovement` have no
float-toward-target z, no `MF_INFLOAT` and no skull slam (thinkers.rs:153ff, multisector.rs, player_map_scale.rs), and `P_DamageMobj` has no
`MF_SKULLFLY` stop (combat.rs:208–240, 863–960). None of that is needed on medium Episode 1 (no floaters, no skulls). **Shadow is already honored
where the Spectre needs it:** `P_SpawnMissile` aim reads the target's shadow (projectiles.rs:103–118, :116, :140–144) and `A_FaceTarget` does too
(`flags & 262144`, monsters.rs:334–346). `P_SpawnMapThing` height honors `MF_SPAWNCEILING` (spawning.rs:770–774) and `P_KillMobj` clears
`FLOAT`/`SKULLFLY` (combat.rs:950–962). The player body witness refuses NOCLIP/MISSILE/SKULLFLY/FLOAT/NOGRAVITY on the body (original_player.rs:363).

### 4.4 Missiles

- **Imp fireball** (`MT_TROOPSHOT`): `A_TroopAttack` → `player_monsters::troop_attack` → `projectiles::launch_fireball` (projectiles.rs:547–600: live
  bound check `live_projectile_bound` player_map_scale.rs:934, slot reservation, `actor_aim`, momz witness `momz_bound` :694, `Launch` event, spawn at
  `z + 32`, `finish_launch` :120–177 including shadow draws, the `P_Random()&3` tics, the half step and `P_CheckMissileSpawn`). Flight `fly`
  (:346–352) → `move_xy` (:253–288: MAXMOVE clamp, half steps, sky-hack removal) and `move_z` (:290–340). Contact `try_move` (:197–251) with damage
  `((P_Random()%8)+1)*damage` (:200–209) and missile spechit/crossings; `explode` (:179–195). The algorithms are generic over `MissileWorld` (:35–56)
  and read speed, damage, sounds and death state from the type definition.
- **Generalisation needed:** `FIREBALL` is hard-wired in `launch_fireball` (:556, :561, :577, :582), in the thinker dispatch (`is_fireball`,
  thinkers.rs:123–125, 121–149 where **only `MT_TROOPSHOT`** reaches `projectiles::fly`; original_registry.rs:717–719), in the live bound (counts only
  `FIREBALL`), in the policy closure (`fireball_policy`) and in the diagnostic witnesses. `MT_BRUISERSHOT` is a plain no-action missile and reuses
  everything once the type is a parameter; `MT_ROCKET` adds `A_Explode` in its death state. The Knight/Baron species rule is already present
  (projectiles.rs:410–412, `PIT_CheckThing` :355–438).
- **`P_SpawnPlayerMissile`** exists only in the frozen profile-7 plasma host as `ProjectileAim::PlayerAutoAim` (projectiles.rs:617–622, 871–880,
  `PLASMA` = type 34 :9); it is not reachable from the player weapon world. The player autoaim slope code (`P_AimLineAttack` three times with
  ±(1<<26)) lives in player_pistol.rs and is reusable.
- **`P_RadiusAttack`:** barrels.rs:296–390 with `pit_radius_attack` :434–520 (BLOCKMAP window, Cyborg/Spider exemption :455–456, `P_CheckSight`,
  `P_DamageMobj(thing, spot, source, 128 − dist)`), gated to `is_barrel_type` (:299–305); the damage constant `BOMB_DAMAGE` 128 (:29) equals the
  rocket's. It is bounded by `EXPLOSION_SIGHT_QUERIES` = 24 × (1 + 8) = 216 (:51–69), derived from `MAX_PLAYER_MONSTERS` and `MAX_BARRELS`, and must
  be re-derived.

### 4.5 Thinkers

- **Store:** `ThinkerStore<T, S = SectorThinker>` (store.rs:268ff) has actor slots and a separate sector-thinker slot class of fixed capacity
  (`with_sector_capacity` :297), one shared run order `shared_order()` (:439), the actor subsequence `order()` (:452), `visit_bound = max_slots +
  max_sector_slots` (:316–318) and lowest-free-slot spawn with a generation bump on unlink. Captured as `thinker.order` and `thinker.shared_order`
  (snapshot.rs:830–851, kind byte 0 actor / 1 sector). Sector slots = `L + 2M` (lights + 2 × mover sectors; demon_expanded_map.rs:188–203, 221).
- **Sector-thinker payload** `SectorThinker` (sector_thinkers.rs:92–108): `Flash`, `Strobe`, `Glow`, `Flicker`, `Door`, `Plat`, `Floor`; `kind_code`
  1–7 (:133–143); DSIM kind 8 is a removed mover (snapshot.rs:1319–1330), so new kind codes must avoid 8.
- **Movers** (movers.rs): `Door{sector, kind: Normal|BlazeOpen, direction, topheight, topcountdown}` (:36–55, speed and topwait implied by the kind),
  `Plat{sector, status, count, low, high}` (:77–89; type, speed, wait, crush, tag and oldstatus implied), `FloorMove{sector, kind:
  LowerFloorToLowest|TurboLower, floordestheight}` (:91–107; direction, speed, crush, newspecial and texture implied). Starters `do_door` :289,
  `vertical_door` :334, `do_plat` :383, `do_floor` :423; thinkers :462–625. `specialdata` is derived (`special_data` :155–161), `activeplats` derived
  (`active_plats` :163–170). There is no ceiling family; `T_MovePlane` runs only with `crush == false` (planes.rs:7, 134–200; the crush branch of
  `PIT_ChangeSector` is omitted :244).
- **Buttons:** `Buttons = [Option<Button>; 16]` (use_lines.rs:34–44); `change_switch_texture` (:74–95) records only the sound and the special clear
  (side textures are not modelled); `start_button` (:97–120); `update_buttons` (:122).

What each Episode 1 addition touches:

| Addition | Touches |
| --- | --- |
| More door kinds (open 2/31/86/103, close30ThenOpen 16/76, locked 26–28/32–34, gun 46) | `DoorKind` plus explicit speed/topwait or a kind table; `vertical_door` card check, `sfx_oof` and message; `do_door`; `cross`/`use_special_line` arms; a new `shoot_special_line` called from the hitscan (`PTR_ShootTraverse` in combat_trace.rs); mover_rules family and reachable rules; DSIM door fields |
| More floor kinds (raiseFloor 5/91, raiseFloorToNearest 18, lowerFloorToLowest S1/WR 23/82, turbo SR/WR 70/98, donut 9) | `FloorKind`; `direction`, `speed`, `crush`, `newspecial`, `texture` become state; the `T_MoveFloor` change-texture branch; runtime **sector floorpic** (not state today); the donut spawns two movers |
| Stairs (7/8) | a multi-sector loop over `world.sector_lines` (player_map_scale.rs:1163) comparing runtime floorpic, the `specialdata` skip, floor movers created in loop order (thinker order) |
| Plats 20/22 (raiseToNearestAndChange) and SR lift 62 | `Plat` gains type, speed, wait, crush, tag; floorpic and special change; `activeplats` order semantics only if stasis is ever needed (no Episode 1 line needs it) |
| Teleport (97) | new `EV_Teleport`: `MT_TELEPORTMAN` (14) as an admitted invisible thing, the thinker-list scan, `P_TeleportMove` with telefrag over the BLOCKMAP, two `MT_TFOG` spawns, `reactiontime = 18`, momentum zeroed, monster crossing rules (M4b) |
| Light line 35 | no thinker: write `LevelSpecials.sectors[i].light` (sector_thinkers.rs:147–152) for tagged sectors; already captured as `sector.N.light` |
| Secret exit 51 | a `use_special_line` arm; `MoverEvent::Exit{secret:true}` exists in the enum; `level.exit` needs a secret bit in DSIM39 |
| Sector 11 (E1M8) | a `P_PlayerInSpecialSector` arm: clear `CF_GODMODE`, 20 damage on the gate, `health <= 10` → `G_ExitLevel`; plus the `P_DamageMobj` cap |

### 4.6 Player model

- `OriginalPlayer` (original_player.rs:75–89): `mo, health, state, attacker, damage_count, frags`. `PlayerRuntime` (:104–122): `policy, room, weapon,
  pickups, armor, secret_count, use_down, reborn`. **Absent:** `powers[NUMPOWERS]`, `cards[6]`, `backpack`, `maxammo[4]`, `ammo[2..3]` (cells,
  rockets), `cheats`, `fixedcolormap`, and `killcount`/`itemcount` beyond `pickups.item_count`, `didsecret`.
- **Weapons:** add rockets (and cells for the backpack) to `PlayerWeaponState` (player_weapons.rs:11–32), owned bits 4 and 7, the `weapon_state`
  (:137–146) and `Weapon::states` (weapons.rs:55–88) arms, the `P_CheckAmmo` fallback order, and the slot-1 fist/chainsaw toggle (needs
  `powers[pw_strength]`, which stays 0). `validate_command` (player_weapons.rs:207–214) admits weapon numbers via `Weapon::from_number`.
- **Damage:** `DamagePlan::start` (combat.rs:209–240) has no `pw_invulnerability`/ `CF_GODMODE` test, no chainsaw thrust exclusion and no
  `MF_SKULLFLY` stop; armor is handled (`ArmorEvent::plan_for`). The sector-11 cap is also absent.
- **Radiation suit:** `ironfeet` (sector_thinkers.rs:467) becomes `powers[pw_ironfeet] != 0` with the `P_Random() < 5` draw for 4 and 16. **Player
  `MF_SHADOW`:** monster aim already reads it (monsters.rs:339, projectiles.rs:116); `P_PlayerThink` must set and clear the flag with the power timer.
- **Pickups:** `PickupKind` (world_pickups.rs:37–62) has 12 kinds; add keys (5, 6, 13), soulsphere 2013, invisibility 2024, radiation suit 2025,
  computer map 2026, light amplifier 2045, backpack 8, rocket 2010, rocket box 2046, rocket launcher 2003 and chainsaw 2005, with messages
  (`PickupMessage` :156ff), sounds, `P_GivePower` and the backpack doubling of `maxammo`.

### 4.7 Snapshot surface and schema versioning

Format: named fields (`path`, bytes), framing `DSIM`, a u32 version, a u64 count, then per field a u32 path length, the path, a u64 value length and
the value (snapshot.rs `canonical_bytes`); `capture` (snapshot.rs:687ff) emits fields gated by `matches!(version, a..=b)` ranges. **Each DSIM version
is a whole field list**; there is no prefix reading: every reader is explicit (`decode_selected(bytes, limits, Some(N))`, diagnostic.rs:317–332) and
rejects any other version (`DiagnosticError::Version`). DSIM39 must **add** (proposal):

- `original_player.0.powers` (6 × i32), `.cards` (6 × u8), `.backpack` (u8), `.weapons.max_ammo` (4 × i32) or derived from the backpack,
  `.weapons.rockets` and `.cells`, and the wider `.weapons.owned` domain; `cheats` stays unencoded (always 0, no demo input sets it, stated
  explicitly) and `fixedcolormap` is presentation (documented).
- Mover fields: door `speed`/`topwait` (or kind enumerations), plat `type, speed, wait, crush, tag, oldstatus`, floor `direction, speed, crush,
  newspecial, texture`.
- `sector.N.floorpic` (runtime; read by stairs, the donut and raise-and-change) and `level.exit` with a secret bit (or `level.secret_exit`).
- Bounds: `slot_bound_for`, `pickup_bound_for`, `bonus_bound_for` (diagnostic.rs:2254–2280) and the per-type witnesses (`gibs_admitted` :462–468,
  event validators :2585–2650, actor-type validators ~:1340–1570, :1929). `DiagnosticLimits::default` (diagnostic.rs: 17–25: 262,144 fields, 64 MiB)
  is sufficient for about 1,024 slots × 35 fields.
- Encoder side: see [§7](#7-the-synthetic-wad-ci-path).

### 4.8 Runner and demo reader

`crates/doom-sim/examples/player_level_run.rs` already takes `--map` (default `E1M1`, :79–90; `Map::from_wad(&wad, &a.map, ..)` :113); it always uses
`original_player_level_map` (:115) and the default inventory (`WeaponInventory::default`, :117), and stops at the first refused tic (:131–138); there
is no map-versus-header check and no level transition. `crates/doom-sim/src/demo.rs` has `DEMO_HEADER = [110, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0]`
(:22), matched byte for byte by `parse_demo` (:79–87 → `DemoError::Header{index,value}`), so **a demo recorded on E1M2 (map byte 2) is refused at
header byte 3**, and `write_demo` (:116) always writes E1M1. To run E1M2–E1M9, parse the header into a `DemoHeader { skill, episode, map, … }` (skill
2, episode 1, map 1–9, deathmatch/respawn/fast/nomonsters 0, consoleplayer 0, `playeringame [1,0,0,0]`), derive the map name `E{episode}M{map}` (or
check `--map` against it), make `write_demo` take the header, and keep the old constant and reader for the wire-22 fixtures. A pistol start per map is
what `G_DoPlayDemo` → `G_InitNew` gives, so per-map runs need no carried inventory. A continuous episode run would need
`G_PlayerFinishLevel`/`G_DoWorldDone` ([§3.11](#311-level-transitions-intermission-and-finale-what-original-demos-need-beyond-the-maps)): out of scope
unless the owner asks. Exit ends the run (`LevelExited`, lib.rs:1276).

### 4.9 Load admission: what refuses E1M2–E1M9 today

Constants: monsters `MAX_PLAYER_MONSTERS` 8 (player_monsters.rs:12); pickups 64 (world_pickups.rs:18); barrels 16 (e1m1_rules.rs:111); decorations 32
and the 7-type list (e1m1_rules.rs:44–56); actor slots 384 (`BARRELS_ACTOR_SLOTS` player_map_scale.rs:19; the constructor must pass exactly this,
original_registry.rs:1101–1108); store hard cap `max_actors > 1024` refused (spawning.rs:50–52) and `things > 4096` (:47); live fireballs 208
(player_map_scale.rs:35); explosion sight queries 216 (barrels.rs:68); sector-thinker slots `L + 2M` (demon_expanded_map.rs:188–203); static heights
±512 refused at load (demon_expanded_map.rs:99–105, `PerceptionError::Arithmetic`) and certificate clause (vii) (player_map_scale.rs:1080);
certificate (i)–(viii) (`ArithmeticCertificate::derive` player_map_scale.rs:996–1004, called at original_registry.rs:1113); slide window of at most
128 lines (player_map_scale.rs:1155–1160); real-map format bounds, BLOCKMAP of at most 32,768 words and work budgets (map_extent.rs:162–240); the
mover rules above; intercepts 128 and spechit 8 are per-tic witnesses, not load rules.

## 5. The command domain

The original facts (the `ticcmd_t` layout, `G_BuildTiccmd` ranges, the `G_Ticker` turbo, `BT_SPECIAL` and pause paths, and `P_MovePlayer` over the
full byte range) are in [§3.10](#310-the-command-domain-vanilla-ticcmd_t); the Rust rules are the preflight of `preflight_player_commands`
(player_commands.rs:60–131):

| Rule | Site | Error |
| --- | --- | --- |
| policy must have commands | :67–69 | `PlayerBody(Profile)` |
| slots 1–3 must be `TicCommand::default()` | :70–77 | `PlayerBody(Slot)` |
| buttons within `BT_ATTACK\|BT_USE\|BT_CHANGE\|BT_WEAPONMASK` (1\|2\|4\|56; weapons.rs:8–14): bit 64 and `BT_SPECIAL` 128 refused | :79–93 | `SimError::UnsupportedButtons` |
| `forward`, `side` within −50..=50 (`MAX_COMMAND_MOVE` :9) | :94–99 | `PlayerBody(Unsupported)` |
| `BT_CHANGE` weapon must be 0–3; weapon bits without `BT_CHANGE` refused | `player_weapons::validate_command` player_weapons.rs:207–214 | `Weapon(UnsupportedWeapon)` / `UnsupportedButtons` |
| no current body: only the zero command | :104–110 | `PlayerBody(Binding)` |
| dead (wire 22): any admitted command accepted | :116–121 (`has_level`) | — |
| `angle_turn` unrestricted | — | — |
| next tic after exit or reborn | lib.rs:1276–1281 | `SimError::LevelExited` / `PlayerReborn` |

Live vanilla input clamps forward and side to `MAXPLMOVE` (50) as well, so ±127 arises only from `-turbo` or hand-made demos. Lifting wire 23 to ±127
is a one-constant change, but the travel and arithmetic witnesses (`attempt_bounds` player_commands.rs:133–160, `travel_admitted` player_map_scale.rs)
must be re-derived for the larger momentum, which [§3.10.4](#3104-p_playerthink-and-p_moveplayer-with-the-full-range) shows is overflow-free over the
whole range. The script tool `tools/e1m1_scripts.py` enforces the same limits in `checked` (:75–84): forward and side within −50..50 (−128 is the
marker), turn −128..255, buttons 0..255 with 0x80, 0x40 and 0x20 (weapon numbers 4–7) refused, weapon bits without CHANGE refused.

| Field | Vanilla (from a demo) | Wire 22 | Original demos (counts, [§6](#6-original-demos)) | Wire 23 proposal ([H5](#10-new-owner-decisions-h1h12-with-recommendations)) |
| --- | --- | --- | --- | --- |
| `forward` | −127..127 (−128 is the end marker) | −50..50 | no demo exceeds 50 | −127..127 |
| `side` | −128..127 | −50..50 | no demo exceeds 50 | −127..127 (−128 refused by the tool and reader) |
| internal forward | 100 for the chainsaw lunge, independent of the command | — | — | handled in the movement code |
| `angle_turn` | any (demo: multiples of 256) | any | max absolute 7680 | unchanged |
| `BT_CHANGE` weapon 4–7 | 4 launcher and 7 chainsaw selectable when owned; 5 and 6 ignored (never owned in Episode 1) | `UnsupportedWeapon` | index 0–2 only (DOOM2 demos), max index 2 | indices 0–7 under vanilla's rules |
| bit 64 | ignored | `UnsupportedButtons` | 0 occurrences | still refused |
| `BT_SPECIAL` (0x80) | pause toggle or save request | `UnsupportedButtons` | 0 occurrences | still refused (M4c) |

Demo census command-domain counts (ticcmds over all players; counts and maxima only):

| WAD | lump | ticcmds | abs(forwardmove) > 50 | abs(sidemove) > 50 | forwardmove = -128 | buttons & 0x80 | buttons & 0x40 | BT_CHANGE | BT_CHANGE weapon > 3 | max weapon index | distinct weapon indices | BT_ATTACK | BT_USE | angleturn != 0 | max abs(angleturn) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| DOOM.WAD | DEMO1 | 1710 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 412 | 4 | 612 | 6144 |
| DOOM.WAD | DEMO2 | 2347 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 438 | 8 | 1215 | 6144 |
| DOOM.WAD | DEMO3 | 3863 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 994 | 34 | 1584 | 5632 |
| DOOM.WAD | DEMO4 | 818 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 198 | 5 | 303 | 4352 |
| DOOM2.WAD | DEMO1 | 1205 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 369 | 2 | 360 | 4608 |
| DOOM2.WAD | DEMO2 | 2001 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 2 | 1 | 378 | 8 | 728 | 7680 |
| DOOM2.WAD | DEMO3 | 4471 | 0 | 0 | 0 | 0 | 0 | 7 | 0 | 2 | 1 | 1183 | 27 | 1372 | 5376 |

The same counts are the evidence for H5: no demo leaves the proposed domain, and none leaves the present one except by the weapon index in two Doom II
demos.

## 6. Original demos

Header facts only (the `G_ReadDemoTiccmd` layout, g_game.c: version, skill, episode, map, deathmatch, respawn, fast, nomonsters, consoleplayer,
`playeringame[4]`; then 4 bytes per tic per player; then the 0x80 end marker). Skill is 0-based (0 = I'm too young to die, 2 = hurt me plenty, 3 =
ultra-violence). Medium, the skill the wire admits, is 2. In all seven lumps deathmatch, respawn, fast and nomonsters are 0, consoleplayer is 0,
`playeringame` is 1/0/0/0 (one player), the 0x80 marker is at the expected end, and there are no stray bytes.

| WAD | lump | bytes | version | skill | episode | map | players | ticcmds | seconds (35/s) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| DOOM.WAD | DEMO1 | 6854 | 109 | 3 | 1 | 5 | 1 | 1710 | 48.9 |
| DOOM.WAD | DEMO2 | 9402 | 109 | 3 | 2 | 2 | 1 | 2347 | 67.1 |
| DOOM.WAD | DEMO3 | 15466 | 109 | 3 | 3 | 5 | 1 | 3863 | 110.4 |
| DOOM.WAD | DEMO4 | 3286 | 109 | 3 | 4 | 2 | 1 | 818 | 23.4 |
| DOOM2.WAD | DEMO1 | 4834 | 109 | 3 | 1 | 11 | 1 | 1205 | 34.4 |
| DOOM2.WAD | DEMO2 | 8018 | 109 | 3 | 1 | 5 | 1 | 2001 | 57.2 |
| DOOM2.WAD | DEMO3 | 17898 | 109 | 3 | 1 | 26 | 1 | 4471 | 127.7 |

The command-domain counts of the same lumps are in the table of [§5](#5-the-command-domain).

**Which could run once Episode 1 is admitted.**

- **All seven demos are skill 3** (ultra-violence), and the wire is medium-only, so none runs until hard skill is admitted. On skill 3 only the THING
  filter differs from medium in single player (`options & 4`), which is why the order of work puts hard skill in M4b.
- **DOOM.WAD DEMO1 is E1M5**, 1,710 ticcmds, entirely inside today's command domain (no absolute move above 50, no `BT_CHANGE`, no button bit 0x40 or
  0x80). It needs E1M5 (the teleports, the Spectre and the other [order-of-work](#8-the-order-of-work) items) and hard skill, so it becomes runnable
  in M4b. The Rust reader checks the version byte against 110 while the shipped lumps carry 109, so a run needs the version byte patched 109 → 110 in
  a **scratch copy** (never in Git); `G_DoPlayDemo` reads the version once and compares it with `VERSION` (g_game.c:1589) and nothing else reads it
  ([§3.11.3](#3113-demo-version-dependence)).
- **DOOM.WAD DEMO2, DEMO3 and DEMO4 are E2M2, E3M5 and E4M2**: other episodes, out of scope.
- **The three DOOM2.WAD demos** (maps 11, 5 and 26 of Doom II) need Doom II (commercial game mode, other monsters and rules). Two of them select
  weapon 2 through `BT_CHANGE` (3 and 7 commands), inside the domain.
- **What else the demos need beyond the map** ([§3.11](#311-level-transitions-intermission-and-finale-what-original-demos-need-beyond-the-maps)): for
  a single-map run that ends at the exit tic or the demo marker, nothing: the intermission, `G_WorldDone` and the carry-over of health, ammo and
  weapons matter only when a demo continues into the next level, and E1M8 ends in a finale, not an intermission. The harness stops at the exit
  (`LevelExited`, lib.rs:1276) as in M3h9c. Whether each demo's recorded ending is an exit or a marker is not known without running it and is not
  recorded here.

## 7. The synthetic-WAD CI path

Read-only survey of what it takes to cover every part-1 mechanic with generated WADs through the M3h9c harness without an IWAD. Paths are relative to
the repository root.

### 7.1 The generator (`tools/reference/m3h9c_wad.py`, 269 lines)

- **Structure.** Game data (:46–60): `SWITCHES` (:49–53, the 29 pairs of every `alphSwitchList` entry with episode ≤ 2; registered mode uses episode
  2, `P_InitSwitchList` p_switch.c:107–148), `PLAIN` textures (:55: TESTWALL, STARTAN2, BIGDOOR2, DOORTRAK, PLAT1, SUPPORT2), `SKIES` (:56), `FLATS`
  (:58: FLOOR7_1, CEIL3_5, TESTFLR0, TESTCEIL, F_SKY1, NUKAGE1–3), all textures 64×128 with one patch `BLANKP` (:59–60). The map (:100–171) is a
  hard-coded E1M1-scale layout; `build_map()` (:118–171) only lists sectors as `(rings, floor, ceiling, attrs)` (attrs: light, special, tag, floor,
  ceiling) and a `lines` dict `{((x0,y0),(x1,y1)): (special, tag)}` and calls `G.room(..., lines=lines)` (:171). All BSP, SEGS, NODES, BLOCKMAP and
  REJECT code is in the imported chain m3h9b_geometry → m3h9_geometry → m3h8_geometry → m3h7_geometry → m3h6_geometry (208 lines: an automatic node
  builder with exact-clip BLOCKMAP, bounds 4096 lines / 8192 sides / 1024 sectors, `room()` at m3h6_geometry.py:60) → generate_m3b. `fix_sides`
  (:200–223) rewrites sidedef textures by special.
- **Things** (`make_things()` :174–197) are raw tuples `(x, y, angle, doomednum, options)`; only barrel, pickups,
  lamp/pillar/candelabra/gibs/deadplayer/mess, zombieman and imp exist in the constants. `build()` (:226–239) orders PLAYPAL, COLORMAP, flats, PNAMES,
  TEXTURE1, BLANKP, then the map named **`E1M1` (hard-coded at :234)**; `--check` compares the committed bytes then runs the independent verifier
  `tools/reference/m3h9c_check_wad.py` (357 lines, E1M1-specific: `MAXMAP` :43, the kind tables :30–41, "at most 8 special two-sided lines per 2×2
  BLOCKMAP window", "clear straight line from the start to the exit switch").
- **Required changes:** (a) a registry of map specs and a `build()` taking a spec list (registered mode caps episodes at 3, so slots E1M1–E1M9 are
  available and the slot name at :234 must change); (b) tags and specials already work (the value of the `lines` dict is `(special, tag)`), needing a
  wider `fix_sides` (S1/SR specials need a `SW1xxx` middle that `P_ChangeSwitchTexture` finds in the 29-pair list; locked doors need no texture); (c)
  differing heights are already supported per sector, and stair steps need adjacent same-texture floors (one flat for all steps); (d) teleport
  destinations are only THING 14 records inside the destination sector with the same tag; (e) keys are THINGs 5, 6, 13; (f) a new flat or texture must
  be added to `FLATS`/`PLAIN`, animated flats need complete ranges (the checker enforces `ANIM_FLATS`); (g) the demo header's nomonsters is 0, so any
  monster THING with options 7 spawns, and new constants and verifier entries are needed for the Spectre 58 and Baron 3003; (h) `A_BossDeath` requires
  slot E1M8 with tag-666 sectors and all barons dead (the thinker scan), so that mechanic needs the map in slot E1M8 (M4b). Hanging decorations need
  only the doomednum (out of scope). The geometry chain is about 900 lines and the checker 357; M3h9c's WAD stays frozen.

### 7.2 The script tool (`tools/e1m1_scripts.py`, 157 lines)

Text format (docstring :4–9, parser `ticcmds` :86–105): `idle N`, `cmd N FORWARD SIDE TURN BUTTONS`, `seed S chaos N [nouse]`, `#` comments and `#
output: NAME.lmp`; BUTTONS is an integer or `ATTACK|USE|CHANGE|W0..W3` (`NAMES` :51). `checked` (:75–84) enforces the command-domain limits of
[§5](#5-the-command-domain), so the weapons 4–7 refusal (:82) and the chaos table (:52) must be lifted to three weapon bits (0x38) with 0x40 still
refused. The header is hard-coded: `HEADER = bytes([110, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0])` (:49; `lump()` :112–114 prepends it), so a `map` (and
perhaps `skill`) directive or CLI option is needed; `SCRIPT_DIRS` (:48) and `--check` (:134–146) must include the new directories. Lump names are the
upper-cased base name of 1–8 characters (`lump_name` :116–119). Scripts are authored by hand as closed loops against the strict runner (the synthetic
walk is 368 lines; the others 3–61), not generated, so each mechanic map needs an iterative authoring session (run C, read the dump, adjust).

### 7.3 The C reference runner and goldens (`tools/reference/m3h9c_reference.py`, 117 lines)

- **Discovery** is hard-coded: `WAD` (:31, one committed WAD), `SCRIPTS = tests/reference/m3h9c_scripts` (:32), `sorted(SCRIPTS.glob('*.lmp'))` (:69),
  results `m3h9c_synthetic_results.json` (:33). `run()` (:38–54) invokes `crunner --wad WAD --wad LMP --demo LMP.stem.upper() --out DIR`. **There is
  no map argument:** the demo header's episode and map select the level (m3h9c_driver.c:489–490, 516; driver comment :14–17), so the C side is not
  tied to E1M1, only the lump name must exist in the WAD. A (WAD, script) pairing registry is needed.
- **Hard-coded E1M1 elsewhere:** m3h9c_snapshot_goldens.py:35–37, 71 (`WAD`, `SCRIPTS`, `L.MapData(str(WAD), 'E1M1')`); level_snapshot.py:881 (`--map`
  default) and :158 (`MapData(wad, slot)`); level_compare.py:174 (`--map` default); check_player_level.py:31 (`GOLDENS`) and the CI loop
  (rust.yml:266–267, `--map E1M1`); `player_level_run.rs` default `map: "E1M1"` (:80; accepts `--map`).
- **Goldens layout** (all under tests/reference/): `m3h9c_synthetic_results.json` (2.8 KB: schema, `wad_sha256`, `runner_contract_sha256`, variants,
  per script `lmp_sha256`, tics, end (marker | exit | reborn), `end_tic`, `prndindex_after_load`/`end`, `thinkers_at_load`, `mobjs_at_load`,
  `unknown_messages`, snapshots, `dumps_sha256`); `m3h9c_synthetic_goldens.json` (255 KB: per script the lmp hash, snapshots, end, `projection_stop`,
  `tics_projected`, `sequence_sha256`, `final_sha256`, the per-tic 16-hex digests, and a `fixtures` list `{path, script, tic, why, sha256}`); 10 full
  DSIM38 captures `m3h9c_level_<SCRIPT>_<tic>.bin` (125–139 KB each, about 1.3 MB) picked by `chosen()` (m3h9c_snapshot_goldens.py:43–67);
  `m3h9c_runner_contract.json` (45 KB); the WAD (60 KB).
- **Run time and scale** (docs/validation/M3H9C.md:90, 293–302): `m3h9c_reference.py --check` takes 19.7 s for 6 scripts and about 10.4k tics × 3 variants; the Rust loop with `--captures` 17.3 s (writes 1.4 GB, about 130 KB per tic); `check_player_level.py` 37.4 s; per tic the genuine `G_Ticker` costs 3.5–5.2 µs (strict), 5–8 µs (overflow), 13–39 µs (address), the dump about 0.25 ms and Rust 1.7–1.9 ms. Scaled to 50 scripts of about 600 tics (30k tics): C side about 34 s, Rust loop about 57 s plus about 60 ms fixed per run, `check_player_level.py` about 110 s. Storage is the constraint: 30k × 130 KB is about 4 GB of captures, too much for the macOS and Windows hosted disks, so check and delete per script or add a digest-only Rust mode. The 5000-tic chaos script is half of all tics, so keep new scripts at 200–800 tics and one or two chaos runs on the big map. Estimated added CI time: Linux about +3.5 min (30 of 45 min used at `5e711be`); macOS Intel is the risk (17 of 30 min used, timeout 40), where the Rust loop plus check (+2.5 min) is acceptable but captures are not. The encoder has a hard `MAX_SLOTS = 384` (level_snapshot.py:117, 709), so a synthetic big map stays near 250 spawned things or the DSIM slot count is raised.
- **One WAD per mechanic group versus multi-map:** the driver takes `--wad` any number of times, so a multi-map WAD fits (the map comes from the
  header), but every golden then depends on one `wad_sha256` and any edit invalidates all of them; per-group WADs invalidate only their own goldens.

### 7.4 The CI workflow (`.github/workflows/rust.yml`, 381 lines)

Jobs: `changes` ("Detect changes", ubuntu-latest, 5 min; classifies `docs_only` at :49–73); `stable` (name `Stable / ${{ matrix.os }}`, matrix
ubuntu-latest, windows-latest, macos-latest, the last running on `macos-26-intel` (:83); timeouts 45 / 40 (macOS) / 30 (Windows), :93; `if:
!cancelled()`); `msrv` (name `Minimum Rust / 1.85.0`, ubuntu, 10 min, `cargo +1.85.0 test --workspace --locked`). **Required check names to
preserve:** `Stable / ubuntu-latest`, `Stable / windows-latest`, `Stable / macos-latest`, `Minimum Rust / 1.85.0` and presumably `Detect changes`. The
harness steps in the `stable` job (all skipped when `DOCS_ONLY == 'true'`):

- :243–248 "M3h9c whole-engine runner contract, synthetic game WAD and demo scripts" (every OS, pure Python): `m3h9c_build.py --check`, `m3h9c_wad.py
  --check`, `e1m1_scripts.py --check`.
- :249–251 genuine whole-engine run of the synthetic E1M1 (Linux only): `m3h9c_reference.py --check --keep-dumps --work "$RUNNER_TEMP/m3h9c"`;
  :252–258 failure-only upload of `runs/*.log`; :259–261 independent DSIM38 projection (Linux): `m3h9c_snapshot_goldens.py --dumps ... --check`.
- :262–269 Rust whole-level run (every OS): a loop over SYNIDLE SYNWALK SYNFIGHT SYNBARRL SYNCHAOS SYNDIE running `cargo +stable run -p doom-sim
  --release --locked --example player_level_run -- --wad ... --map E1M1 --demo ...`, then `check_player_level.py`. The script, WAD and map names are
  hard-coded in the YAML, so new maps need a manifest-driven loop. The genuine C runner is Linux-only (gcc, sanitizers, `-Wl,--wrap` GNU linker flags,
  `setjmp`); Windows and macOS run the Rust goldens only.
- :352–353 "Documentation links and archive integrity" (`python tools/check_docs.py`, always runs, even for docs-only changes) and :354–356 `cargo
  doc`.

New synthetic maps go after step :261 and :262–269, reusing the same job with a loop over a registry; no new job is needed.

### 7.5 Dump rows and the encoder (m3h9c_driver.c 556 lines; level_snapshot.py 906; level_compare.py 197)

The driver dump (`dump()` m3h9c_driver.c:378–474, `COLUMNS` :313) has row kinds H (tic, leveltime, gametic, `prndindex`, `rndindex`, totals,
`gameaction`, `gamestate`, `numth`, `numlinespecials`, `gameskill`), M and RM (full mobj including `tracer`), R/RR (pending removal), thinker rows DR
(vldoor), PL (plat), FL (floormove), CE (ceiling), LF/LL/LS/LG (lights), X (any other thinker), T, K, S (sector), L (special lines), B (buttons) and P
(player: including `powers[NUMPOWERS]`, `cards[NUMCARDS]`, `backpack`, `ammo`, `maxammo`, `weaponowned`, `cheats`, `didsecret`, `message`). So the C
dump already has powers, cards, backpack and the ceiling rows. **What the DSIM38 encoder (level_snapshot.py) does not project** (docstring :28–90):

- `Unadmitted` for CE rows, stasis plats and X rows (:473, 501, 585, 602, 757), two live movers in one sector (:772), a sector-thinker record outside
  the DSIM35 domain (:780), lights created after load (:500, 601), level reload (:459), pickup messages outside the 13-entry `DRIVER_MESSAGES` table
  (:143, 554), more than 384 actor slots (:709), weapon states outside the DSIM37 domain (:816), and a mobj `sp_type` set on spawned things (:502,
  605).
- Player fields projected: only the `original_player.0` subset (docstring :66–73). **Not projected today:** `powers[]`, `cards[]`, `backpack`,
  `maxammo`, ammo 2–3 (cells, rockets), weapon-owned bits beyond those of the admitted weapons, `cheats`, `colormap`/`fixedcolormap`, messages beyond
  the table, `tracer`, `didsecret`, `player.1..3`, `frags`.
- **The secret exit is invisible in the dump:** the H row carries `gameaction` but not `secretexit`, so a secret-exit line (51) and a normal exit are
  indistinguishable today. The driver therefore gains a `secretexit` column ([§12](#12-implementation-plan) step 1).
- The mobj registry is data-driven (`STATES = v4.metadata(0)`); `PLAYER_HITSCAN` and `MONSTER_HITSCAN` (:128–129) enumerate the hitscan actions used
  for removal causes (`A_Saw` is already there; rockets need explicit handling in the removed-mobj logic :348–375, otherwise "unprojected cause"
  refusals).
- The frozen chain imported unchanged: movers_snapshot.Store, specials_snapshot, weapon_snapshot, m3f3 `map_fields`; `VERSION = 38`, `POLICY = 22`,
  `CONTEXT b'PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1'` (:113–116). DSIM39 therefore needs the new mover kinds and multiplicity,
  `powers`/`cards`/`backpack`/`maxammo`/ammo 2–3/all weapon-owned, the secret-exit flag, a wider message table, and the teleport fog actors as
  ordinary M rows. (Ceiling and stasis thinkers are not needed for Episode 1.) `level_compare.py` (197 lines) is the per-tic comparator of C dump
  against Rust capture (first differing field path; `--map` default :174, `--self-check`, `--json`, `stripped()`).

### 7.6 Docs pattern

`docs/TESTING.md` "M3h9c whole-engine runner" (from :1594) holds the purpose and IWAD rules, two command blocks and the "CI path (no IWAD)" paragraph (:1647–1654); observations go to the evidence record `docs/validation/M3H9C.md` (385 lines). The committed result files are under tests/reference/ (the synthetic WAD, scripts, results and goldens JSON, 10 `.bin` fixtures, the runner contract); the real-IWAD E1M1 scripts are `tests/reference/e1m1_scripts/{*.txt,*.lmp}` (local only, no data).

### 7.7 Recommendation

- **One generator, a registry of map specs, one small WAD per mechanism group** (for example doors and locks, floors and plats, stairs and donut,
  weapons, powers, scale) with the map in the slot the mechanic needs (E1M1 for most; E1M8 only for tag-666 boss death in M4b), so per-WAD goldens
  stay independent, each WAD load is cheap, and review diffs stay small. A manifest `tests/reference/m4a_manifest.json` (`{wad, map, scripts[]}`) is
  consumed by the reference run, the goldens, the check and the CI steps, so no script or WAD name is hard-coded in YAML.
- Script names are upper-case lump base names of at most 8 characters with a mechanic prefix (for example `LKDRRED`, `STAIR1`, `SPECTRE`, `POWER1`),
  text sources `m4a_<name>.txt` with `# output: NAME.lmp`, directory `tests/reference/m4a_scripts/`; 200–800 tics per script and one `chaos nouse` run
  per big map only.
- Estimated sizes: generator 600–900 lines in a new `tools/reference/m4a_wad.py` (keep `m3h9c_wad.py` unchanged to preserve historical fixtures);
  verifier generalisation +120; `e1m1_scripts.py` map/skill directive and weapon-bit extension about 25; the reference, goldens and check scripts
  parametrised by the manifest about 80 changed lines each (about 250); workflow +15; the DSIM39 encoder additions (a separate task) 300+; TESTING
  +40; a new M4A.md about 400.
- **Risks:** (a) the encoder's admission limits (CE rows, 384 slots, the message table) gate what a golden can cover: tics after a `projection_stop`
  are covered only by the three-variant C runs; (b) Rust-side disk (4 GB) and macOS time; (c) `m3h9c_check_wad.py` is E1M1-specific; (d) the secret
  exit is not distinguishable in the dump.

## 8. The order of work

(part 1 = M4a; the rest proposed as M4b and M4c)

What each map needs beyond wire 22 (from the census, medium skill), grouped by system:

- Movers and triggers (group 1): 29 line specials that reuse the door, floor and plat machinery of wire 19 plus three new mechanisms (stairs 7/8, the donut 9, the light trigger 35) and the shot trigger (46, `P_ShootSpecialLine`) and the secret exit (51): 2, 5, 7, 8, 9, 16, 18, 20, 22, 23, 26, 27, 28, 31, 32, 33, 34, 35, 46, 51, 62, 70, 76, 82, 86, 90, 91, 98, 103. Needed by every map E1M2–E1M9.
- Teleportation (group 3): line 97 and the landing thing 14. Needed only by E1M5, E1M8 and E1M9.
- Sector special 11: E1M8 only.
- Monsters: the Spectre (E1M5, E1M6, E1M7, E1M9) and the Baron with `A_BossDeath` (E1M8 only).
- Weapons: chainsaw (E1M2, E1M3, E1M5, E1M7, E1M9) and rocket launcher with rockets (E1M3–E1M7, E1M9).
- Items: keys 5/6/13 and the locked-door rules (E1M2–E1M7, E1M9), backpack 8, soulsphere 2013, partial invisibility 2024, radiation suit 2025, computer map 2026, light amplifier 2045.
- Static things: candle 34 and tall red firestick 46 (E1M8, E1M9), the teleport landing 14 as a static spawn.
- Scale: every map from E1M2 exceeds wire 22's load bounds (8 monsters, 64 pickups, 16 barrels, 384 actor slots); E1M2, E1M3 and E1M8 fail clauses of the M3h6 arithmetic certificate; E1M3 fails boundary rules C1/C2 and the ±512 height check; E1M4 has one sector in two mover families; E1M3, E1M6 and E1M9 have movers in sky-ceilinged sectors; E1M5 and E1M9 exceed the 2×2 special-line window of 8.
- The command domain: the chainsaw's `MF_JUSTATTACKED` lunge sets `forwardmove` 100 inside the engine, so the movement code must handle the full byte range whatever the demo holds; the rocket launcher is weapon slot 4.

**Part 1 (this milestone, M4a).** Everything above except teleportation, the Baron and `A_BossDeath`: the 29 line specials as whole systems (all E1 door kinds including locked and D1 and the shot-activated door, all E1 floor kinds, the three plat kinds, stairs, the donut, the light trigger, the secret exit), sector special 11, the Spectre (a Demon with `MF_SHADOW`; the shadow aim fuzz is needed anyway for partial invisibility), the chainsaw and the rocket launcher with the generalised missile and explosion paths, every E1 item and key, the three static types, the lifted bounds and the proofs-to-witnesses rule (H3), and the command-domain lift (H5). After part 1, **E1M2, E1M3 and E1M4 load and are scripted in full** (primary targets). **E1M6 and E1M7** need nothing outside part 1 either and are secondary targets: they are admitted if they load and every script authored for them matches; their walk-to-exit routes are long and are authored only as far as the milestone allows, which is reported per map. Estimated size: 14–18k `crates/` lines (wire plumbing and DSIM39 1.8k, movers 5–6k, weapons and missiles 3k, items and powers 1.8k, sector 11 0.5k, Spectre and shadow 0.4k, statics 0.3k, bounds and witnesses 1.1k, command domain 0.3k, runner 0.25k) plus about 1k fuzz, 3–4k tools (generator, scripts, encoder, checks), 3k docs and 1–2k golden JSON: **about 23–27k changed lines**, under the brief's 30k cap. If Phase 2 measurements show the diff heading past 30k, the fallback is to move E1M6/E1M7 scripts and the Spectre to M4b; E1M2–E1M4 stay.

**M4b (proposed).** Teleportation (line 97, `EV_Teleport`, `P_TeleportMove`, `PIT_StompThing`, the two fog spawns and the 18-tic freeze, the crossing-loop abort, the split-move continuation, the telefrag thrust overflow as a declared wrapping adapter), the Baron (`A_BruisAttack`, `MT_BRUISERSHOT`) and `A_BossDeath` (E1M8's tag 666), hard skill (skill byte 3: only the THING filter differs from medium in single player), and with them E1M5, E1M8 and E1M9. DOOM.WAD `DEMO1` (E1M5, skill 3, 1,710 ticcmds, inside today's command domain) becomes runnable here, its version byte patched 109 → 110 in a scratch copy. Estimate 8–12k lines.

**M4c (proposed).** Easy skill, `BT_SPECIAL` pause (the pause tic consumes a ticcmd without `P_Ticker`; the unpause tic runs with cleared buttons), the reborn reload as an admitted transition, and the revisit of the IWAD maps' remaining load rules; level transitions (intermission, `G_WorldDone`, state carry-over; the E1M8 finale) stay a later direction (the brief's section 4). The other six IWAD demos need Episodes 2–4 or Doom II.

## 9. Policy proposal

`OriginalExecutionProfile::PlayerMonstersEpisode1V1`, wire **23**, DSIM**39**, context **`PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1`**, constructor `Simulation::original_player_episode1_map (map, options)`: a superset of wire 22 admitting part 1. Wires ≤ 22 stay frozen byte for byte (unchanged goldens, matched digests). DSIM39 is the DSIM38 layout plus the fields part 1 needs: `original_player.0.powers.*` (the six `powers[]` counters), `cards.*` (six), `backpack`, `maxammo.*` and `ammo.*` for rockets, the new door, floor and plat kinds (the same records with new kind values), the stairs and donut floor thinkers (ordinary floor records in thinker order), the shot-trigger and light-trigger side effects on `line.N.special` and `sector.N.light`, `level.secret_exit`, the `MF_SHADOW` flag on the player body and on Spectres (already in `flags`), and the new actor types (rocket, chainsaw puff is the ordinary puff). The synthetic-WAD CI path covers every part 1 mechanic: one small generated WAD per mechanism group (doors and locks, floors and plats, stairs and donut, light and secret exit, weapons, items and powers, Spectre and shadow, scale) through the same genuine runner, encoder and Rust runner, with committed goldens and a self-test that plants a difference.

## 10. New owner decisions (H1–H12) with recommendations

Each: (a) consistent with the brief's section 4 and E1–E9, F1–F11, G1–G13? (b) no silent divergence? (c) no new kind of fidelity gap beyond refuse-the-tic witnesses? (d) bounded stores?

**H1. Scope split.** Part 1 as above (29 line specials, sector 11, Spectre, chainsaw and rocket launcher, every E1 item and key, three statics, bounds, command lift), with E1M2–E1M4 as primary and E1M6–E1M7 as secondary targets; teleportation, the Baron, `A_BossDeath` and hard skill in M4b; pause, easy skill, the reborn reload in M4c. Rejected: (i) all of Episode 1 in one milestone (every estimate puts it past 30k lines); (ii) the strict census-minimal set for E1M2–E1M4 alone (17 line specials), which would leave door, floor and plat kinds half ported against the brief's whole-systems rule. (a)–(d) yes. **Recommendation:** part 1 as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H2. Policy identity.** `PlayerMonstersEpisode1V1`, wire 23, DSIM39, context `PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1`, constructor `original_player_episode1_map`; `has_level()`-style predicates gain the new wire where behaviour is inherited and every new bound gets its own predicate so wire 22 bytes cannot change. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H3. Load-time proofs that an IWAD map fails become per-tic witnesses under wire 23.** The maps part 1 targets fail load rules whose purpose is to prove, for the whole map, that an arithmetic bound or a structural invariant can never be hit: the M3h6 certificate clauses (i), (ii) and (vii) and the ±512 height check (E1M2: (ii); E1M3: (vii), 528 units), the boundary rules C1/C2 (E1M3), the mover rules "one family per sector" (E1M4), "no mover in a sky-ceilinged sector" (E1M3, E1M6, E1M9) and "reach within ±512" (E1M3), the 2×2 special-line window of 8 (E1M5, E1M9: M4b), and the "store never fills" live-actor proof. Under wire 23 each such map loads, and the quantity the proof bounded is checked where the original computes it, refusing the tic with complete canonical state when it would overflow or leave the modelled domain: checked arithmetic at `P_InterceptVector`/`P_InterceptVector2`/`SlopeDiv`/height sites (`MapScaleError::Arithmetic` at the tic), the existing `Escape`/`Position`/`Vertical` witnesses, the ninth `spechit`, a use of a DR/D1 line on a sector whose special data is not a door (vanilla casts a plat to a door there, and the aliased field differs between the 32-bit original and LP64), the blocked stair step and the blocked raise-and-change plat (H7), and a full store. The three genuine C builds (strict `-fwrapv`, overflow-checked, address-checked) stay the independent check that no overflow actually occurs on any matched run. Older wires keep their load proofs. This changes, for the new wire only, the M3h6 standard "proven at load, not merely caught" (M3h6 owner decision 8) and is **the judgment flagged for the owner**, as G3 was: without it part 1 admits E1M7 alone. (a) yes, the brief keeps "refuse-the-tic witnesses for fidelity gaps"; (b) yes, every witness refuses the tic; (c) no new kind; (d) yes. **Recommendation:** adopt for wire 23. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H4. Bounds.** Under wire 23 the fixed caps of wires 12–22 (8 monsters, 64 pickups, 16 barrels, 32 decorations, 384 actor slots) are replaced by per-map values derived at load from the THINGS lump on the admitted skill (monsters, pickups, barrels, statics as counted), an actor-slot count of spawned things + 1 + a transient allowance within the store's 1,024 hard cap, and the store-full witness of H3; the per-type live bounds (208 fireballs, 216 explosion sight queries) are re-derived for the new counts or made witnesses with the same refusal contract. Older wires keep their caps. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H5. Command domain.** Wire 23 admits `forwardmove` and `sidemove` in −127..127 (−128 is the demo marker and is refused by the script tool and the reader), `angleturn` unchanged, `BT_CHANGE` with weapon indices 0–7 under vanilla's rules (ignored when the weapon is not owned; index 0 selects the chainsaw once owned unless berserk is active with the chainsaw ready; index 7 is the chainsaw), and still refuses button bit 0x40 and `BT_SPECIAL` (pause and save: M4c). The movement code handles the full range, which the chainsaw lunge needs regardless. The census shows no IWAD demo leaves this domain: none has |moves| > 50, −128, bit 0x40 or 0x80, or a weapon index above 3. The script tool lifts the same limits. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H6. CI evidence.** A new generator `tools/reference/m4a_wad.py` built on the M3h6–M3h9b geometry chain (BSP, SEGS, NODES, BLOCKMAP, REJECT; `m3h9c_wad.py` is frozen for its WAD) writes one small WAD per mechanism group in the slot it needs (E1M1 for all of part 1), each with 1–3 scripts named as upper-case lump names of at most 8 characters under `tests/reference/m4a_scripts/`; a manifest `tests/reference/m4a_manifest.json` drives the C reference run (three variants, identical dumps), the encoder goldens, the Rust check and the CI steps; every real-map divergence fixed in Phase 2 gets a synthetic map that reproduces it (G9). The self-test (planted extra draw, perturbed command) runs on at least one M4a map. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H7. Uninitialised zone reads reached by part 1.** `EV_BuildStairs` never writes `floor->type` or `floor->crush` (p_floor.c:485–547) and `EV_DoPlat`'s raise-to-nearest-and-change never writes `plat->low` (p_plats.c:109–120); vanilla reads zone garbage there when a stair step or such a plat is blocked. The M3h9c driver zero-fills `Z_Malloc` (a declared driver behaviour, decision N6's projection), so the genuine runner reads 0; Rust models 0 for the unblocked path (where the fields are never read) and **refuses the tic** when a stair step or a raise-and-change plat is blocked, because vanilla's behaviour there depends on zone contents no model can know. (a) yes; (b) yes; (c) no new kind; (d) yes. **Recommendation:** zero projection plus the two witnesses. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H8. The donut's precedence bug.** `EV_DoDonut` tests `!flags & ML_TWOSIDED` (p_spec.c:1184–1195), which never skips a one-sided line, dereferences a null back sector in that case and overwrites `s2->specialdata` of a busy sector. A load rule admits a donut only when every line of its inner sector is two-sided and the outer sector is found as vanilla finds it, and a witness refuses the tic when the donut's sectors are busy. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H9. Real-map scripts.** For every map part 1 admits: `idle` (2,100 tics), `walk_to_exit` (authored iteratively against the genuine build; turns and durations only), `fighting`, `chaos` (fixed seed, inside the H5 domain) and `death` (the player dies; the reborn request tic compared, the next tic refused). The real E1M1 scripts are re-run under wire 23 and must still match. Scripts are committed as sources and `.lmp` files under `tests/reference/e1mN_scripts/` (one directory per map) with the header's map byte set by the tool. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H10. Demo header.** The Rust demo reader accepts version 110, skill 2, episode 1, map 1–9, single player, and checks the map against the runner's `--map`; the writer takes the map. Skill 3 is M4b. The wire-22 fixtures keep their reader. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H11. The 32-bit confirmation.** The opt-in `strict32` variant (genuine text, no adapter, `-m32`) stays a local, opt-in check recorded in the evidence record, not a CI job (the runners' toolchains are not under our control). G3 is closed by the observed byte-identical dumps. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

**H12. Timings and fuzz.** As G12 and the M3h9c fuzz pattern: matched timings against master for every older-policy workload (digests equal) and the per-tic cost of each runner on the real maps, alone on the idle machine, callgrind on any slowdown over 3%; a `player_episode1` fuzz target with a seeder; seed replay, one coverage-guided and one AddressSanitizer campaign with `detect_leaks=0` disclosed. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

## 11. Size estimate and the unattended-rule verdict

**The proposed sizes** (quoted from [§8](#8-the-order-of-work); part 1 is this milestone):

| Part | Changed lines (estimate) | Breakdown |
| --- | --- | --- |
| M4a, part 1 | **about 23–27k**, under the brief's 30k cap | `crates/` 14–18k (wire plumbing and DSIM39 1.8k, movers 5–6k, weapons and missiles 3k, items and powers 1.8k, sector 11 0.5k, Spectre and shadow 0.4k, statics 0.3k, bounds and witnesses 1.1k, command domain 0.3k, runner 0.25k), fuzz about 1k, tools 3–4k (generator, scripts, encoder, checks), docs 3k, golden JSON 1–2k |
| M4b (proposed) | 8–12k | teleportation, the Baron, `A_BossDeath`, hard skill |
| M4c (proposed) | not estimated | easy skill, pause, the reborn reload |

Named fallback: if Phase 2 measurements show the diff heading past 30k, E1M6/E1M7 scripts
and the Spectre move to M4b; E1M2–E1M4 stay.

**C-side calibration** (merge commits, `git diff --shortstat <merge>^1 <merge>`):

| Milestone (merge) | C functions newly modelled (approx.) | `crates/` lines | Whole PR |
| --- | --- | --- | --- |
| M3h2 Imp fireball (2da111d) | ~5 (A_TroopAttack, missile tail reuse, species rule, explosion, states) | +2,077 / −467 | +5,703 / −492 |
| M3h8 movers (3b4fbbb) | ~22 (doors, plat, two floors, `T_MovePlane`, `P_ChangeSector` family, use/cross dispatch, switches, buttons) | +9,119 / −282 | +20,488 / −408 |
| M3h9b barrels (59a397e) | ~7 (barrel states, `A_Explode`, `P_RadiusAttack`, `PIT_RadiusAttack`, inflictor damage, kill chain) | +5,143 / −135 | +15,910 / −218 |
| M3h9c E1M1 run (0ecfe4b) | harness, little new gameplay | +1,680 / −99 | +21,669 / −114 |

Ratio used: **about 400 (300–700) `crates/` lines per C function or major branch** (code,
tests, fuzz target, schema), and **whole PR about 2.2–3 times the `crates/` lines** (C
fixtures and generators, independent encoder, docs). The C-side agent's per-group estimates for all of Episode 1 (gameplay only) came to **about 19–28k `crates/` lines, about 45–70k PR lines**, far above the 25k split threshold the M3h9 and M3h9b notes use, which is why the order of work splits it: line specials 5–7k `crates/` (12–18k PR); sector specials 0.5–1k; teleport 2–3k; Spectre and Baron 2–3k; chainsaw and launcher 3.5–5k; keys and powerups 2–3k; pickup dispatch 0.8–1.2k; decorations 0.2–0.4k; command domain 1–2k; level transitions 2–3k. The order in [§8](#8-the-order-of-work) differs from the agent's proposed split because the real maps need whole systems and the Spectre and Baron are tied to specific maps.

**Rust-side calibration** (non-test `crates/doom-sim/src` lines added and tests added, from
`git diff --numstat` of the merges):

| Merge | Scope | src + | tests + | whole PR |
| --- | --- | --- | --- | --- |
| 3b4fbbb M3h8 movers | 7 line types, 3 mover kinds, planes, buttons | 5,260 | 3,496 | +20,488 |
| 37d4e5c M3h7 specials | lights, sky, damage floors, secrets | 2,351 | 1,095 | +10,188 |
| 43460e2 M3h9 E1M1 | two-sided scrollers, 7 decorations | 1,142 | 2,607 | +13,192 |
| 59a397e M3h9b barrels | one actor type, radius attack, bounds | 2,288 | 2,193 | +15,910 |
| 0ecfe4b M3h9c level | 3 behaviours, runner, demo | 749 | 747 | +21,669 (tools 3,730, fuzz 1,150) |

The Rust-side agent estimated wire 23 at about 11,000 non-test source lines and about 9,600 Rust test lines for its full scope (which also priced the Lost Soul, the Cacodemon, plasma, berserk, invulnerability and hanging decorations, all absent on medium and out of scope), plus fuzz about 1,500, tools and encoder about 5,000–7,000 and docs about 3,000: a PR of roughly 35–45k lines, about twice M3h8, M3h9b and M3h9c combined, with staged commits advised, which [§8](#8-the-order-of-work) does. Its largest lines: wire plumbing, DSIM39 fields and reader witnesses 1,200 / 600; line specials (30 types) and load-rule generalisation 3,500 / 3,000; weapons 1,400 / 1,200; keys and powers 1,200 / 1,000; bounds re-derivation 600 / 500; generic missile type 400 / 400; Baron 400 / 400; sector 11 250 / 300 (source / tests).

**The unattended-rule verdict.**

Every decision H1–H12 has a recommendation consistent with section 4 of the brief and with E1–E9, F1–F11 and G1–G13; none admits a silent divergence (H3, H7 and H8 add refuse-the-tic witnesses, the only kind of fidelity gap allowed); the genuine engine still needs only the I/O stubs and the declared adapters of M3h9c (part 1 adds no C adapter; the driver gains a `secretexit` column and the manifest-driven runner; the telefrag overflow adapter belongs to M4b); and part 1 is estimated at 23–27k changed lines, under the 30k cap, with a named fallback. **Phase 2 therefore proceeds with H1–H12 provisionally adopted.** The judgment the owner should look at first is H3. The owner confirmed H1–H12 as written, H3 included (wire 23 only), on 2026-11-05.

## 12. Implementation plan

Phase 2, part 1; one commit per step, pushed.

1. **Synthetic WADs and scripts**: `m4a_wad.py` with the manifest and one map per mechanism group; `e1m1_scripts.py` lifted to H5 and H10 (map byte, ±127, weapons 0–7); the driver's `secretexit` column; C reference run of every synthetic script in the three variants, dumps identical; goldens committed from the encoder (step 2) before the Rust writer where possible.
2. **Encoder**: DSIM39 projection in `level_snapshot.py` (powers, cards, backpack, ammo, new kinds, secret exit), `level_compare.py` unchanged in method; the goldens and fixtures; `--check` modes.
3. **Wire 23**: the variant, context, predicates, version ladder (39), every range site; load admission per H3/H4; the thinker order unchanged (append in source order).
4. **Movers**: all E1 door kinds (open, close-30-then-open, D1, locked DR/D1 with the card checks, the shot trigger), floor kinds, the three plat kinds, stairs, the donut (H8), the light trigger, the secret exit; the H7 witnesses.
5. **Items and powers**: keys, backpack, soulsphere, invisibility (player `MF_SHADOW`, the aim fuzz), radiation suit (special 16 draw; nukage immunity), computer map, light amplifier, rockets; power timers in the player think; sector special 11 with the damage cap.
6. **Weapons and monsters**: chainsaw (`A_Saw`, the lunge, slot rules, `P_CheckAmmo` order), rocket launcher (`A_FireMissile`, `P_SpawnPlayerMissile`, `MT_ROCKET`, the explosion order and the post-explosion half step), the generalised missile tail; the Spectre.
7. **Command domain** (H5) and the demo reader/writer (H10); the runner's per-map stats.
8. **Real runs**: E1M2, E1M3, E1M4, then E1M6, E1M7: the H9 scripts, every divergence explained and fixed under wire 23 with a synthetic reproduction; the real E1M1 scripts re-run.
9. **Original demos**: none is inside part 1's domain (all DOOM.WAD demos are skill 3; DEMO1 needs E1M5); reported with the exact blockers.
10. **Tests, rejections, rollback, soaks, example, fuzz target and seeds**.
11. **Docs**: COMPATIBILITY (wire 23), STATE_FORMAT (DSIM39), ROADMAP (the strategy change; M4a current, M4b/M4c proposed), TESTING, this note's corrections section, `docs/validation/M4A.md`, README ("Where it stands", one policy row), the owner-approved AGENTS.md update as its own commit.
12. **Matched timings** (H12), **fuzz** campaigns, **independent Opus review** of the full diff against the C source, **full gates** from clean, the draft PR.

## 13. Follow-ups, risks, status

Risks and follow-ups from the two inventories (the hazards table in
[§3.13](#313-unchecked-bounds-undefined-reads-and-hazards-a-rust-witness-must-guard) is the per-site list):

- **Thinker order at new spawn sites.** Stairs (a loop of floor thinkers), the donut (a pair), rocket explosions and, in M4b, the teleport fogs append
  at the tail; they must follow source order ([§3.1.2](#312-tag-search-and-sector-ownership)), and `secnum` after a staircase resumes the tag search
  after the last stair sector's index.
- **Flag-gating so wire 22 bytes stay identical.** `has_level()` and `has_barrels()`-keyed bounds are hard-wired single-variant predicates; widening
  them silently changes wire 22 if done wrong. Add wire 23 to the inherited lists and override bounds through new `const fn`s
  ([§4.2](#42-version-ladder-sites)). Any change to the shared movement and missile paths (the half-step, the generalised missile dispatch, the
  `A_Explode` gate) must be gated the same way.
- **The spechit window on E1M5 and E1M9 (for M4b).** The 2×2-window bound on special lines is 14 on E1M5 and 9 on E1M9 against `MAXSPECIALCROSS` 8
  (p_map.c:67–70, write 240–244, unchecked in C); this is why those maps are not part-1 targets and why H3 turns the ninth `spechit` into a per-tic
  witness.
- **Intercepts.** `intercepts[128]` is unchecked in C (p_maputl.c:544, 603–606, 668–671); with 100 or more monsters, overflow is more likely than on
  E1M1, and rockets add `P_AimLineAttack` traces (range 1024) and the saw 65-unit traces.
- **Telefrag overflow (M4b).** `damage*(FRACUNIT>>3)*100` overflows for the telefrag's 10000 (p_inter.c:817); a declared wrapping adapter and an
  observation in the genuine build are needed.
- **`P_FindNextHighestFloor`'s 20-neighbour cut-off** (p_spec.c:326–375): the search is truncated after the 20th qualifying neighbour; a witness or a
  load rule must bound the qualifying neighbours of every target of 18, 20 and 22.
- **`MAXPLATS` and `MAXBUTTONS`.** `P_AddActivePlat` and the button list `I_Error` when full (30 plats, 16 buttons); Episode 1 has at most 9 plat
  sectors and 10 reusable switch lines per map, so both are unreachable at Episode 1 counts.
- **`activeceilings` is unused**; `P_AddActiveCeiling` silently drops the 31st ceiling, but Episode 1 has no ceiling or crusher special, so none of
  that is needed.
- **Store and bounds.** With 100 or more monsters on E1M6-scale maps the "store never fills" proofs become runtime witnesses
  ([H3](#10-new-owner-decisions-h1h12-with-recommendations), [H4](#10-new-owner-decisions-h1h12-with-recommendations)); the encoder's 384-slot limit
  caps the synthetic big map ([§7.3](#73-the-c-reference-runner-and-goldens-toolsreferencem3h9c_referencepy-117-lines)).
- **Determinism.** The survey found no determinism hazard in the Rust source: no `HashMap`/`HashSet` in doom-sim, doom-map or doom-types sources
  (collections are `Vec`, `BTreeMap`, `BTreeSet`, 65 uses); floats appear only in `Fixed::div2_checked` (doom-types fixed.rs:78–87, unused by
  doom-sim) and a `#[cfg(test)]` JSON parser (barrels_replay.rs:161); no `Instant` or `SystemTime`; `thread_local!`s are test hooks or run statistics
  (level_stats.rs:21, mover_rules.rs:367 test-only bypass); the `AtomicU64` in demon_lifecycle.rs:10 is an admitted-type cache, not gameplay.
- **Not covered by Phase 1:** the real-map divergences (found in Phase 2), the performance record (measured alone on the idle machine in Phase 2,
  H12), and the continuous-episode and finale behavior (a later direction).

**Status (2026-11-10 PST).** M4a is complete and merged into master as [PR #57](https://github.com/Copgunn/DOOM/pull/57) on 2026-11-10. The closing review, gates and dispositions are in the [evidence record](validation/M4A.md); exact CI and history stay in GitHub. Work continues in M4b.

**Historical preparation commands (superseded by the completed 2026-11-09 campaign).**
These describe the earlier unexecuted preparation, not remaining work; the completed
run used a fresh scratch root and builds as recorded in the evidence link above.
The scratch scripts are prepared and syntax-checked, not campaign-tested. The
matched harness retains all 246 workloads and eleven paired AB/BA rounds;
Callgrind selects every >3% median slowdown from its analysis. The separate
per-tic driver uses five repetitions of thirty Episode 1 cases plus five
wire-22 E1M1 cases, separating full Rust wall time, successful advance time and
thirty genuine C strict32 ticker timings. IWADs remain read-only/local-only.
The supplied release binaries were built from 9aa7d6c; the following diff guard
permits a later documentation-only handoff head. If Rust, fixture helpers or
build settings change, rebuild/relink the two trees and legacy callers before
measuring, using a fresh output root; do not overwrite historical results.
First verify no other builds/tests/fuzzing/benchmarks or gaming are active and
that /mnt/c has at least 20 GB free. The old orphan process was absent at the
last precheck; recheck rather than assume the machine is idle.

```bash
set -e
cd /home/copgunn/src/DOOM
df -h /mnt/c
ps -eo pid,ppid,etimes,pcpu,comm --sort=-pcpu
vmstat 1 6
git diff --exit-code 9aa7d6c -- crates tests/support Cargo.toml Cargo.lock

# One simulation-only case; JSON contains counts, digests and timings, no captures.
/home/copgunn/scratch/m4a/perf-idle-codex/target-branch/release/examples/player_level_advance \
  --wad /mnt/d/DoomIWADs/DOOM.WAD --map E1M1 \
  --demo tests/reference/e1m1_scripts/WALK.lmp --policy episode1

# Fresh independent final-state validation, if desired, before retaining timings.
python3 -I tools/reference/check_player_level_advance.py \
  --binary /home/copgunn/scratch/m4a/perf-idle-codex/target-branch/release/examples/player_level_advance

python3 /home/copgunn/scratch/m4a/perf-idle-codex/harness.py run
python3 /home/copgunn/scratch/m4a/perf-idle-codex/harness.py analyze
python3 /home/copgunn/scratch/m4a/perf-idle-codex/pertic.py run
python3 /home/copgunn/scratch/m4a/perf-idle-codex/pertic.py analyze
python3 /home/copgunn/scratch/m4a/perf-idle-codex/cg.py
```

The prepared scripts refuse existing raw campaign results and check stable
non-timing outputs; they do not establish host idleness automatically. Keep
raw observations and signatures, check every slowdown, and update the
[M4a evidence record](validation/M4A.md#matched-performance) with actual results
and limitations. Preparation/build logs and exact compiled heads are in
`~/scratch/m4a/perf-idle-codex/builds.json`; no `raw.json`, `pertic.json` or
`cg/results.json` existed at this stop. Poll long runs no more often than every
30 minutes. Before stopping a later session, stop its jobs and verify ps again.


**Pre-budget-change CI slow steps (seconds, descending; 2e33c35).**

- **macOS**: Test — 759s; Release deterministic checks — 489s; Rust whole-level run of the synthetic scripts against the encoder goldens — 199s; Headless debug/release equality — 113s; Clippy — 93s; Rust Episode 1 run of the M4a synthetic scripts against the encoder goldens — 87s; Original real-map-scale room debug/release comparison — 23s; Original Imp encounter debug/release comparison — 20s; Original armor sustained combat debug/release comparison — 19s; Original former-human encounter debug/release comparison — 17s.
- **Ubuntu**: Reproduce isolated reference vectors and snapshot goldens — 425s; Test — 323s; Release deterministic checks — 147s; Independent original real-map-scale room evidence — 142s; Independent original Imp, fireball and mixed-roster evidence — 84s; Independent original real-map-scale evidence — 82s; Independent DSIM38 whole-level projection — 73s; Independent original former-human, mixed-roster and infighting evidence — 68s; Independent original armor, animation, pickup and absorption evidence — 45s; Independent original sky and static-specials evidence — 45s.
- **Windows**: Test — 280s; Release deterministic checks — 186s; Rust whole-level run of the synthetic scripts against the encoder goldens — 44s; Clippy — 38s; Headless debug/release equality — 37s; Rust Episode 1 run of the M4a synthetic scripts against the encoder goldens — 21s; Run actions/checkout@11d5960a326750d5838078e36cf38b85af677262 — 9s; Format — 8s; Build — 8s; Original former-human encounter debug/release comparison — 8s.


**Mutation-testing handoff context.** Phase 2 steps 1–8 are done and recorded (30 real-map runs match tic for tic, strict32 identical; fuzz re-run clean; matched timings recorded, four explained slowdowns). Owner-requested mutation testing (cargo-mutants 27.1.0, nextest, opt-level 1), run 1 against master's diff in two shards: shard 1 (behaviour files, 1,336 mutants) 1,119 of 1,212 viable caught (89 by new tests), 90 equivalent, 2 harmless, 1 open (`barrels.rs:362:50`); shard 2 (reader and encoder, 444) 435 of 439 viable caught (105 by new tests), 4 equivalent; both shards' tests are on the milestone branch (20 commits cherry-picked, last 38a41b4) with the one reader-code change (v39 `store.max_slots` exactly 1,024). **Run 2** (the scoped rejection/rollback/refusal set, `~/scratch/m4a/mutants/run2_args.sh`, 693 mutants + 12 struct-field deletions; uncapped 08:51–12:06 for 387, then capped for 318): **637 caught, 49 missed (19 run 1 equivalents already proven in `run1_shard1_dispositions.json`, 30 new), 12 unviable, 7 timed out.** **Run 2 survivor state at the original pause (superseded by step 1 above):** no disposition recorded yet (`run2_dispositions.json` does not exist); WIP tests for the 30 new misses (player-body and command preflights, `reachable`, `do_plat`, registry and store bounds, expanded-demon preflight) are committed on the work branch `feat/m4a-rust` as **6873aaa, "WIP, not yet compiled"** (nine test files, 393 lines, never compiled or run); every earlier `feat/m4a-rust` commit up to c3d2192 is compiled, gated and cherry-picked. Timeout re-run (`~/scratch/m4a/mutants/run2_timeouts.sh`: `JOBS=1 TIMEOUT=1800 run.sh`, caps `CARGO_BUILD_JOBS=3 NEXTEST_TEST_THREADS=2`, ended 17:47 after 8,809 s): 4 of the 7 re-run, all timed out again at 1,800 s (`mover_rules.rs:607:5 widen -> true`, `610:15 != -> == in widen`, `636:19 += -> *= in stair_bounds`, `782:25 != -> == in reachable`); not re-run: `mover_rules.rs:782:36`, `786:12 delete ! in reachable`, `movers.rs:755:28`. The four known loops (636:19, 782:25, 782:36, 755:28) are the infinite loops explained in the scratch report; the three `widen`/`786:12` ones need their loop reason read from the code before they count as caught. **Exact re-check commands** (from `~/scratch/m4a/mutants`, after `rsync -a --delete --exclude target --exclude .git ~/scratch/m4a/wt-rust/ src-head/`): survivors `JOBS=2 TIMEOUT=300 ./run.sh <name> --re '^<re.escape(description)>$' …` with one `--re` per mutant from `run2_uncapped_part1/mutants.out/missed.txt` and `run2b/mutants.out/missed.txt` (no `--in-diff`; `run2b.sh` shows the form); timeouts `JOBS=1 TIMEOUT=1800 ./run.sh run2_timeouts …` as in `run2_timeouts.sh`; the wrapper exports the caps and `--jobs ${JOBS}` and logs them in `stages.txt`. **Remaining order:** finish run 2 survivors (compile the WIP tests, disposition the 30 with proofs or tests, resolve the 7 timeouts, targeted re-run, gates, `run2_dispositions.json` and the report's run 2 section) → cherry-pick `feat/m4a-rust` into `feat/m4a-episode-1` → evidence-record mutation section and TESTING entry (both runs' counts, commands, caps, the grouped equivalence tables, the 23 crate-internal witnesses, the `store.max_slots` gap) → per-commit Clippy over the cherry-picked commits → [Claude after reset: independent review, idle-machine timings re-run and simulation-only per-tic figure, full gates, draft PR]. Scratch: `~/scratch/m4a` (reports in `phase2/`, mutation outcomes in `mutants/*/mutants.out`).

## 14. Corrections and findings from Phase 2

- **Barrel packing argument (mutation follow-up, 2026-11-08).** BLOCKMAP membership uses the actor center (p_maputl.c:428–442), so a collision box need not fit inside its cell. The earlier scratch argument limiting a cell to nine radius-20 actors was not a proof: a sixteen-monster synthetic placement is admitted. Non-shootable items can amplify revisits without adding sight queries or damage (p_map.c:1170–1194). The new public test uses a nonlethal first wound before killing the barrel, because a lethal first hit returns before P_DamageMobj records its source (p_inter.c:888–915). This is a Rust-only refusal-bound regression with source-derived expectations, not an independent C trace.
- **CI policy selection (2026-11-08).** The M4a Rust-script step omitted
  `--example-args "--policy episode1"`, although the local CI-budget measurement
  included it. The example therefore used its default older policy, all 30 scripts
  exited 1 and the comparator reported missing captures on Linux, Windows and macOS.
  The workflow now passes the Episode 1 policy explicitly. The exact corrected
  build-and-comparison command passed locally in a scratch checkout: all 30 scripts
  and all 18 fixture comparisons passed. This repairs the CI invocation; it does not
  change simulation behavior or any golden expectation.
- **Windows source newlines and masked command failures (2026-11-08).** The runner
  builder decoded raw bytes without translating Windows checkout CRLF endings, so
  `r_main_extract` could not find its exact include line; adapter matching, stub
  discovery and raw checkout-file hashes had the same newline dependency. The shared
  M3h9c/M4a reader now reads Latin-1 text with universal newlines and hashes that LF
  source text. Both frozen contracts still match on Linux, including all six extracted
  bodies, and a new regression test compares both complete frozen contracts against
  LF and CRLF scratch source copies. Its CRLF cases fail when the old reader is restored.
  No original C file or reference fixture changed. The M4a multi-command generator
  step now uses `shell: bash`, so an earlier failure cannot be hidden by the last
  command's success under PowerShell. The other M4a multi-command step already used
  Bash. The affected M3h9c contract batch and the pre-existing Format and headless
  comparison batches now use Bash too; Linux-only batches already have a fail-fast
  Bash default. Commands are in
  [TESTING](TESTING.md#m4a-synthetic-mechanism-wads-and-the-episode-1-comparison).
- **Matched timings (H12).** The idle-machine replacement and simulation-only real-map costs are complete; explained shared-validation costs remain for owner review. See the authoritative [performance evidence](validation/M4A.md#matched-performance), including the discarded-run archive.
- **Owner confirmation.** H1–H12 confirmed as written on 2026-11-05 (owner review of the branch at d6d715c), H3 included for wire 23 only.
- **Rust plan decisions D1–D10 adopted** (the Phase 2 plan in scratch; recorded here because scratch is deleted): `store.max_slots` is the constant 1,024 under wire 23; `sector.N.floorpic` is the normalized 8-byte flat name; the switch-texture rule is generalised (a line whose front texture matches no switch is a silent no-op as in `P_ChangeSwitchTexture`, a match only in another episode's list is refused); the one-sided boundary is optional under wire 23 (a C1/C2 failure such as E1M3's leaves the Escape witness inactive; the Position witness stays); DSIM39 `player.0.*` is the command after `P_PlayerThink`'s chainsaw rewrite (p_user.c:249–255); **`MT_TELEPORTMAN` (14) is deferred to M4b** with teleportation (its spawn state is `S_NULL`), so H1's "three statics 34/46/14" is two, 34 and 46; the locked-door messages `PD_BLUEK`/`PD_YELLOWK`/`PD_REDK` are state in the DSIM message field; the per-tic monster sight budget becomes 8·(K+1) for K monsters (the fixed 64 would refuse legal tics on E1 maps); `side` −128 is refused like `forward` −128; the per-type live missile caps do not apply under wire 23 (the full store is the witness).
- **Corrections to §3.** `MELEERANGE+1` is 64 units plus one raw fixed-point unit, not 65 units; linuxdoom-1.10 clears `MF_SHADOW` only when invisibility expires (the `&8` blink is the colormap, presentation); the red card 13 is `MT_MISC5` and the yellow card 6 `MT_MISC6`; the chainsaw no-thrust rule in `P_DamageMobj` keys on the source player's **ready weapon**, so a rocket's damage also thrusts nothing once the player has switched to the chainsaw.
- **Squashed non-shootable things (ruling, step 5).** A pickup or static under a mover can end with `floorz > ceilingz`; `PIT_ChangeSector` (p_map.c) returns true for `!(thing->flags & MF_SHOOTABLE)` and leaves it. Under wire 23 that state is admitted for non-shootable things; wires ≤ 22 and shootable things keep the `Ownership` refusal.
- **Uninitialised zone reads (H7) confirmed in the dumps.** The synthetic PLATS2 run shows the blocked raise-and-change plat reversing at tic 91, descending to `low` = 0 and waiting with `count` running below zero; STAIRS2 shows the blocked step stalling at tic 153. Rust refuses the tic after each (`PlatBlocked`, `StairBlocked`), matching every earlier tic.
- **Engine facts observed in the synthetic runs (step 1).** `P_FindNextHighestFloor`'s 20-neighbour cut-off ignores the 21st neighbour (the comb rises to 64, not 8); a close-30 door shutting onto a barrel reverses at once and is removed at the top; a raise-and-change plat copies the flat of the trigger line's front sector; `EV_LightTurnOn` to 35 is rewritten by a glow thinker in the same sector from the next tic; a hitscan hits a thing only if the trace enters the BLOCKMAP cell holding the thing's centre.
- **Shadow aim fuzz order (step 5).** `A_FaceTarget` adds (first − second) · 2²¹ and `P_SpawnMissile` (first − second) · 2²⁰, the convention of `A_PosAttack`; confirmed by the INVIS1 goldens.
- **Reachable-height fixpoint.** Without the ±512 refusal the M3h8 fixpoint need not terminate; under wire 23 it is clamped to ±32,767 and a round counts as a change only when the clamped intervals differ (a stair line in the floors room hung before the fix).
- **The driver fork (owner review, 2026-11-05).** Commits 2a27e58 and 9e323ca changed the merged M3h9c driver (`m3h9c_driver.c`: `H.secretexit`, the PD_* message codes, `--wrap=P_RemoveThinker`, the R-row `fn,sector` columns) and re-hashed two merged result files. Existing fixtures are new-files-only and wire-22 evidence is frozen, so the M4a additions now live in the fork `tools/reference/m4a_driver.c` (built by `tools/reference/m4a_build.py`, schema `DOOM-M4A-RUNNER-v1`, contract `tests/reference/m4a_runner_contract.json`; `m3h9c_build.py` gained only a `Runner` parameter whose defaults are the M3h9c values, and keeps the opt-in `strict32` variant). `m3h9c_driver.c`, `m3h9c_runner_contract.json` and `m3h9c_synthetic_results.json` are byte-identical to master again, and `m3h9c_reference.py --check` reproduces master's six `dumps_sha256` values; the M4a results and goldens are unchanged apart from the contract hash. `tools/local/e1m1_reference.py` stays on the M3h9c driver (frozen wire-22 evidence); `tools/local/episode1_reference.py` uses the M4a driver for E1M2–E1M7 and the M3h9c driver for E1M1. The `episode1` encoder profile projects a missing `H.secretexit` as 0, exact for any map without a special 51/124 line (E1M1 has none), and a missing R-row `fn,sector` as the M3h9c encoder did.
- **Size check (owner review).** After step 6 the branch adds 51,983 lines (`git diff master --numstat`): 6,872 in `crates/`, 3,584 in `tools/`, 2,312 in docs, 33 elsewhere, and **39,182 under `tests/reference/`**, of which the DSIM39 goldens JSON is 14,746 (the same per-tic digest format as the M3h9c goldens, 10,592 lines), the five walk_to_exit scripts about 14,000, the M4a runner contract 1,338 and the manifest 1,234. Code, tools and docs together are 12.8k and are projected at about 16k after step 7, the fuzz target, the remaining tests and the docs; the named fallback (E1M6/E1M7 scripts and the Spectre to M4b) would remove about 10k lines of script data and 400 of code, which cannot bring the all-inclusive count under 30k either (the goldens, contract and manifest alone are 17k before any map). The data files are generated, checked by `--check` tools and not reviewed line by line, so **E1M6, E1M7 and the Spectre are kept**, and this judgment is flagged for the owner, who can still direct the fallback at review.
- **SCALE WAD (step 6).** `scale.wad`'s tower ceiling of 1,100 exceeded the ±1,024 sector-height rule that wire 23 keeps (`map_extent::validate_real_map`; not an H3 rule), so Rust refused the WAD at load with `Geometry(CoordinateOverflow)`. The generator now sets it to 1,000 and the SCALE goldens were regenerated from the genuine runs; the C side observed nothing else change.
- **The turbo message is not presentation (step 8; corrects §3.10.3).** `G_Ticker` (g_game.c:671-679) sets `players[i].message = "<name> is turbo!"` when `forwardmove > TURBOTHRESHOLD` (50), `!(gametic&31)` and `((gametic>>5)&3) == i`, so every 128th gametic for player 0, during demo playback as well. §3.10.3 called it message-only presentation and the Rust model did not assign it, but the message is a captured DSIM field. The first real run showed the difference at DSIM tic 1 of M4FIGHT and M4WALK (also M6FIGHT, M6WALK and M7WALK). Fix (wire 23 only): `PickupMessage::Turbo` is code 30, set before the ticker from the command as read with the simulation tic as `gametic`; the v39 diagnostic reader admits 30; the driver builds message index 41 with `sprintf("%s is turbo!", player_names[0])` and the encoder maps 41 to code 30. The earlier real-walk counts of 20 (M6WALK) and 23 (M7WALK) unknown messages were this message; all 30 C runs now report none. Reproduction: the synthetic script `TURBO1`.
- **Encoder deficiency: `target` and `tracer` positions (step 8).** The driver prints `target` and `tracer` as thinker-list positions of its own dump. A barrel marked for removal whose first damager is listed after it leaves the list before the damager (`P_RunThinkers`), so the damager's position falls by one between consecutive dumps, and the E1M3 M3WALK projection stopped at DSIM tic 5216 with "the RR record differs from the next RM row" (column `target`). `level_snapshot.py` now compares those two columns as the serials they name. Rust was unchanged and matched all 5,794 snapshots. Reproduction: `TARGET1` (a pending imp fireball whose imp shifts down the list); a barrel version could not be authored because every synthetic WAD lists the player first and about 40 attempts on WEAPONS never got an imp to hit a barrel, so the real E1M3 run is the barrel evidence.
- **Original demos (step 8).** None of DOOM.WAD's demos can run under wire 23: all are skill 3 (M4b) and DEMO1 is E1M5 with teleporters (M4b). None was attempted.
- **Runtime sector heights are unbounded under wire 23 (independent review A1, 2026-11-10).** In C nothing bounds a sector height a mover writes. `SR 70` is repeatable (p_switch.c:592–596; `WR 98`, p_spec.c:909–911). `EV_DoFloor(turboLower)` sets the destination to `P_FindHighestFloorSurrounding` (folded from −500, p_spec.c:297–315) plus 8 unless that equals the floor (p_floor.c:307–315). `T_MovePlane`'s floor-down branch moves a floor to a destination *above* it in one step (`floorheight - speed < dest` then `floorheight = dest`, p_floor.c:66–78), in the same tic as the use (`P_PlayerThink`, then `P_RunThinkers`, p_tick.c:150–152). So two adjacent sectors tagged by one such line ratchet up by 8 per activation: from 992 and 1,008 the destinations are (1,016, 1,000), then (1,008, 1,024), then 1,032. The door top `P_FindLowestCeilingSurrounding − 4` (p_doors.c:324, 335, 498) ratchets ceilings down the same way through `T_MovePlane`'s ceiling-up branch, and stair heights add 8 per step (p_floor.c:503, 531). Older wires prove at load that every reachable height stays within ±512 (`MoverHeights`), and they admit only the single-use `W1 36` turbo. Wire 23 dropped that proof (H3), and its fixpoint is clamped at ±32,767, so it no longer bounds anything. The ±1,024 sector-height rule it keeps (`map_extent::validate_real_map`) reads only the static heights, and V′'s ±2,048 `floorz`/`ceilingz` witness reads only actors, so an empty sector is not checked. In Rust every height write goes through `PlaneHost::set` (crates/doom-sim/src/planes.rs), and the sums `dest + 8` (`movers.rs` `do_floor`), `lowest_ceiling − 4` (`do_door`, `vertical_door`) and `before ± speed` (`move_plane_witnessed`) wrap (`Fixed` Add/Sub, crates/doom-types/src/fixed.rs). A Rust run of the 992/1,008 strip reached a floor of 1,032 on the third press without any refusal: outside the ±1,024 domain that docs/COMPATIBILITY.md states for wire 23 ("Sector heights"). Repeated about 4,000 times, the ratchet would wrap the fixed-point value silently. Every other admitted write stays inside the hull of existing heights: plats (lowest floor, own floor, next-highest floor), the donut (`s3`'s floor), `raiseFloor` (the minimum of ceilings), `lowerFloorToLowest`, `raiseFloorToNearest`, the close-30 top (own ceiling) and every door closing to its floor. A plane step moves only between `before` and `dest`. **Rule (H3, wire 23 only):** sector heights stay inside the domain the arithmetic proofs assume. That domain is the ±1,024 map units of the wire-23 bound, which is also `map_extent`'s M3f2 height domain. A write past it refuses the tic with `MapScaleError::Arithmetic`, the variant H3 names for height sites. The sums above are computed with checked `i32` arithmetic, so a wrapped value can never reach the domain check. The test is in `PlaneHost::set`, before the staged write. Every wire-23 mutation runs on the staged plan, so the refused tic keeps the complete canonical state. The check is gated by `StaticWorld::checked_arithmetic`, so older wires, whose load proof keeps heights within ±512, are unchanged. Its one strict-side departure from C: the genuine engine runs on past 1,024 without overflowing until about 32,767.
- **`strength()` read the invulnerability counter (independent review A2, 2026-11-10).** `powertype_t` is `pw_invulnerability` = 0, `pw_strength` = 1 (doomdef.h:214–224). The weapon change keeps the fist only `if (player->readyweapon == wp_chainsaw && player->powers[pw_strength])` (p_user.c:290–296), but the Rust weapons host's `strength()` (crates/doom-sim/src/player_pistol.rs) read `powers[0]`. This is latent: under wire 23 `episode1::validate_powers` refuses any nonzero `powers[0]` or `powers[1]` at every tic, and so does the DSIM39 reader. So no run can observe it, and no test can construct the state through the simulation. The other numeric indices are correct: `episode1::powers_tick` (strength 1 incremented, then 0, 2, 5, 3 decremented, p_user.c:339–353), `validate_powers` (0–5) and the `world_pickups` `POWER_*` constants (2–5). The fix names the two missing indices.
- **Review notes verified (2026-11-10).** None of these is a silent divergence:
  - **A3:** `raiseFloor` can jump down to a neighbour ceiling below its own floor (p_floor.c:318–327 with `T_MovePlane`'s up branch, p_floor.c:95–106), which `reachable` does not model. That makes `R_min` too high. `R_min` is read only by V′ case (e) (player_map_scale.rs `v_prime_case`), where a higher `R_min` can only refuse more.
  - **A4:** `stair_bounds` reads only the static floors, while `EV_BuildStairs` adds to the runtime floor (p_floor.c:503). `R_max` is not read at run time. The public `mover_bounds` and `e1m1_mover_bounds` run the older rule sets, which refuse the stair specials 7 and 8, so no public value is wrong. With the domain rule above, runtime heights are bounded by the witness, not by the fixpoint.
  - **A5:** the m4a driver calls `G_DoPlayDemo` and then `{ G_Ticker (); gametic++; }` from `gametic` 0 (tools/reference/m4a_driver.c:582–596, mirroring d_main.c:1135–1140 and d_net.c:742–746). The first played tic therefore runs `G_Ticker`'s turbo test (g_game.c:671–679) at `gametic` 0. Rust passes `Simulation::tic`, which is 0 on the first `advance` and grows only after each committed tic (demon_expanded_map.rs:761–765, 998). Every admitted wire-23 run starts at load, and a level exit ends it.
  - **A6:** without a boundary (decision D4), the remaining certificate clauses are stated over the vertex rectangle and its 2,177-unit margin, not over the boundary. Clauses (i), (ii) and (vii) are checked arithmetic. Every live actor's centre is checked against the rectangle before each tic (`preflight_monster_owner`, monsters.rs:719–745). After the tic the player is checked by `validate_result`, and monsters and barrels by `travel_admitted` (demon_expanded_map.rs:959–974). One tic's motion (at most 30 units per axis) stays inside that margin.
- **`A_Saw`'s turn at a delta of exactly `ANG180` (independent review B5, 2026-11-10).** The mutation record called a delta of exactly `ANG180` impossible. It is reachable when `linetarget` stands on the player's own centre. `PIT_AddThingIntercepts` (p_maputl.c:616–671) crosses the trace with the thing's corner-to-corner diagonal. When the trace starts at the thing's centre, that diagonal passes through the trace origin, so `P_InterceptVector` gives frac 0. Only `frac < 0` is skipped (p_maputl.c:665–666). `PTR_AimTraverse` then divides by a distance of 0, and `FixedDiv` saturates to `MAXINT`/`MININT` for the top and bottom slopes. So the thing is accepted and becomes `linetarget`. `R_PointToAngle2` of the two equal centres returns 0 (`if ( (!x) && (!y) ) return 0;`, r_main.c:299–300). With the player facing 180, `player->mo->angle` is `ANG180` = 0x80000000 (tables.h:70). In `A_Saw` (p_pspr.c:529–541) `angle - player->mo->angle` is the unsigned `0 - 0x80000000` = 0x80000000, which is not above `ANG180`. The else branch then tests `> ANG90/20` (0x40000000/20 = 53,687,091), which holds, so `player->mo->angle = angle - ANG90/21` = `0 - 51,130,563` = **0xFCF3CF3D**. A `>=` comparison takes the first branch: 0x80000000 is below `-ANG90/20` (0xFCCCCCCD as `angle_t`), so it gives `angle + ANG90/21` = 0x030C30C3. A public input reaches it: THINGs in order are the player at (−32, 0) facing 180, a barrel and the chainsaw, both on the same spot (spawning admits coincident things; S′ applies to monsters). A forward move runs `PIT_CheckThing` (p_map.c:252–343) over the cell's things newest first. It touches the chainsaw (a special returns `!solid`, after `P_TouchSpecialThing`), then the solid barrel blocks the move. The player never leaves (−32, 0), and the chainsaw is picked up and raised. Rust gives 0xFCF3CF3D, so there is no divergence. The test is `a_saw_target_on_the_players_own_centre_turns_by_minus_ang90_21`.
- **The per-tic monster sight allowance is reachable (independent review B6, 2026-11-10).** `action_sight` charges every monster `P_CheckSight` of one tic (or one between-tic operation) to one control. `PIT_RadiusAttack`'s sight test has its own per-explosion control. Refusal starts at query `limit + 1`: `max(64, 8·(K+1))` under wire 23 (decision D8) and 64 under the older wires. The mutation record argued that the count never reaches the limit (at most 4K). That proof is wrong in two places.
  - With one player, `P_LookForPlayers` makes at most 2 sight queries (p_enemy.c:499–553). Its `lastlook` ends at 0, so every later call makes exactly 2. `A_Look` adds the ambush query first (p_enemy.c:611–627), so it can make 3.
  - A damage-entered seestate is not limited to one per monster per tic. `P_DamageMobj` (p_inter.c:893–913) draws for pain. Without pain, with `threshold` 0, a non-NULL source and the target in its spawn state, it sets `target = source` and enters the seestate. Its `A_Chase` (p_enemy.c:672–713) finds the source not shootable when the source is a dead monster. That happens when the source is the stored `target` of a barrel (`A_Explode`, p_enemy.c:1598–1601) or the shooter of a missile. `A_Chase` sets `threshold` to 0 and calls `P_LookForPlayers(allaround)` (2 queries). If that fails, it re-enters the spawn state, whose `A_Look` makes 2 or 3 more queries (the 3 needs `MF_AMBUSH` and the player as the sector's sound target). The monster is then back in its spawn state with `threshold` 0, so the next such hit in the same tic does the same again.
  - The true per-tic count is therefore at most 5 per monster thinker, plus 2 for each live-source seestate entry, plus up to 5 for each dead-source hit that finds its victim asleep without pain. The last term grows with the number of corpse-sourced explosions and missiles in the tic, not with K. So no bound in K alone holds. The mechanism is the same under the older barrels wires.
  - Hitting the limit refuses a C-legal tic. That is the documented refuse-the-tic witness of a Rust-only work bound, not a divergence. It is possible in principle on a real map and was not observed in the 30 real runs.
  - The public boundary test `corpse_sourced_explosions_reach_the_sight_allowance_exactly` uses a two-sector wire-23 room with REJECT (0, 1) set, so `P_CheckSight` across it fails at the REJECT test (p_sight.c:313–324). The player is in sector 1. Sector 0 holds, in THINGS order, the "early" Shotgun Guys, five barrels on P = (−300, 0), a Demon `M` and the remaining Shotgun Guys. All seven monsters have `MTF_AMBUSH`, so K = 7 and the limit is 64. The Shotgun Guys stand at Chebyshev distance 145 from P. That is inside the 3×3 BLOCKMAP cells the explosion visits, because `MAXRADIUS` shifts out of `(damage+MAXRADIUS)<<FRACBITS` (p_map.c:1221–1225). Each explosion hits each of them for 128 − (145 − 20) = 3 (p_map.c:1179–1194).
  - Gameplay draws (p_random index from 16): spawning makes 1 for the player's `P_SpawnMobj` and 2 for each of the 12 other THINGs (p_mobj.c:506, 788), up to index 41. Between tics 0, 1 and 2 (counted from the first wound), the setup wounds four barrels by 1 with `M` as the source (one pain draw each, p_inter.c:894; `target = M`, `threshold = BASETHRESHOLD`; the barrel's seestate is `S_NULL`) and kills one, then kills one, then kills two, wounds the fifth and kills it. The five `P_KillMobj` draws (p_inter.c:726) are `rndtable` 68, 145, 62, 70 and 190 (m_random.c), giving `S_BEXP` 5, 4, 3, 3 and 3 tics. With `S_BEXP2` and `S_BEXP3` at 5 tics each (info.c:944–947), all five barrels reach `A_Explode` in tic 15. Tic 3 presses fire. `P_FireWeapon`'s `P_NoiseAlert` (p_pspr.c:256) makes the player the sound target of both sectors through the open two-sided line. Between tics 3 and 4 `M` is killed (1 draw). Between tics 5 and 6 the early Shotgun Guys enter `S_SPOS_STND2` and the others `S_SPOS_STND` (10 tics each, info.c:343–344). `A_FirePistol` runs in tic 7, after `S_PISTOL1`'s 4 tics (info.c:149–150). It makes 5 draws: `P_GunShot`'s damage (p_pspr.c:626–639), then `P_SpawnPuff`'s 2, the puff's `P_SpawnMobj` and its tics (p_mobj.c:812–823). Nothing else draws, so the explosions start at index 57.
  - In tic 15 each early Shotgun Guy returns to its spawn state, and its `A_Look` makes 3 queries (ambush query plus 2). Then the five explosions make 30 pain draws, `rndtable[58..87]`. A Shotgun Guy is asleep for a hit until its first draw below 170 (info.c MT_SHOTGUY). Grouping the draws by visit position (`58 + 6e + p`) gives runs of 0, 2, 1, 1, 2 and 5 hits without pain: 11 in all, each making 5 queries. The order of the visits does not matter, because nothing relinks during the explosions.
  - With three early Shotgun Guys the tic makes 9 + 55 = 64 queries, exactly the limit, and is admitted. With six early it makes 18 + 55 = 73, and the 65th query refuses the tic with `PerceptionError::WorkLimit` and the canonical state unchanged. Rust agrees, so there is no divergence. The mutants `count == limit`, `count >= limit` and `work >= budget` refuse the 64-query tic. The work charge per query is constant (map_extent.rs), so `work >= budget` is the same as `count >= limit`.
- **Melee on a dead target is not always equivalent under the flat policies (independent review B10, 2026-11-10).** The mutation record treats `combat.rs` `live && thrust` → `live || thrust` (both branches of `DamagePlan::current_original_in`) as equivalent, on the grounds that the gated code is a pure preflight that cannot refuse. That holds for the room policies, wire 23 included. It is false for `PlayerMonstersV1` and `PlayerMonstersImpV1`, which have no room.
  - **A dead target is reachable.** `P_CheckMeleeRange` has no `MF_SHOOTABLE` test (p_enemy.c:174–190). So an `A_SargAttack` (p_enemy.c:935) or `A_TroopAttack` (p_enemy.c:913) whose attack states began while the target lived still draws its damage and calls `P_DamageMobj` on the corpse. `P_KillMobj` cleared `MF_SHOOTABLE` (p_inter.c:675), and `P_DamageMobj` returns at its first test (p_inter.c:787–788), so nothing changes.
  - **What the mutants add.** They also run the thrust preflight (`preflight_impulse`, original_player.rs) for a dead target, or for a live target the chainsaw rule spares from thrust. Under a room it checks the room rectangle and the 4,096-unit momentum bound. C momentum stays far below that: about 30 units after `P_XYMovement`'s clamp, plus thrust from the damage a target can take before it dies. One more hypothetical thrust (at most 160 · 12.5/mass units) cannot refuse there.
  - **Without a room it refuses.** The preflight applies the historical envelope `|position| + ⌈160/3 · max|momentum|⌉ ≤ 994` units (`travel_bounds`, a Rust-only proof, not a C limit). A corpse at rest within 160/3 · thrust of that edge is refused, although C leaves it unchanged. Under `PlayerMonstersV1` a Zombieman killed at (960, 0) by a pistol shot (its momentum is zeroed when the Demon beside it blocks the slide) is then bitten by the Demon whose `S_SARG_ATK1` began before the death. Under `PlayerMonstersImpV1` the player is killed at (976, 0) by environment damage, then clawed by an Imp in `S_TROO_ATK1`–`ATK3`. Both tics are admitted, and the target's position, momentum, health and flags are unchanged. Each mutant refuses its tic with `PlayerBodyError::Unsupported`. The original code matches C, so there is no fix. The tests are `a_demon_bite_on_a_corpse_at_the_travel_edge_changes_nothing` and `an_imp_claw_on_the_dead_player_at_the_travel_edge_changes_nothing`.
- **The explosion visit bound, corrected for drops (independent review B7, 2026-11-10).** The recorded reason "order·(1+K) is the most visits one explosion can make" leaves out things linked during the walk. `P_RadiusAttack` walks the 3×3 cells around the spot (p_map.c:1221–1233) with `P_BlockThingsIterator` (p_maputl.c:529–535), reading `bnext` after each callback. A visit can relink or spawn a thing:
  - **Relinks.** A victim relinks when its seestate `A_Chase` moves it (`P_TryMove` → `P_SetThingPosition`, which puts it at the head of its cell, p_maputl.c:436–442). The walk then restarts from that cell's old head. Seestate entry needs the spawn state (p_inter.c:911–913), and a monster that moved is left in its seestate. A corpse-sourced chase that fails re-enters the spawn state without moving. So each monster relinks at most once per explosion: L ≤ K.
  - **Drops.** A killed former human's drop is spawned at the head of its cell (`P_SpawnMobj` → `P_SetThingPosition`, p_inter.c:745–769). Later passes over that cell visit it, so D ≤ K.
  - **The bound.** Each traversal visits a thing at most once and covers one cell list of at most order + D things. Traversals are the 9 cell walks plus one per relink, so visits ≤ (order + D)(1 + L). Every monster stays linked (corpses keep their links) and the exploding barrel is linked, so K ≤ order − 1. That gives visits ≤ (2·order − 1)(1 + R), with R = `visit_roster` = max(K, 8) ≥ K.
  - **The original limit never refuses a legal explosion.** It is 9·order·(R + 2) ≥ 2·order·(1 + R). Nor does either 362:60 mutant: 9·order·(R − 2) ≥ 2·order·(1 + R) needs 7R ≥ 20, and 9·order·2R ≥ 2·order·(1 + R) always holds.
  - **The 362:19 mutant (limit 9 + order·(R + 2)) is not proven equivalent.** It is safe whenever no dropper dies in the explosion (D = 0), since then visits ≤ order·(1 + R). It is also safe whenever D·(1 + L) ≤ order·(R + 1 − L) + 9. Inside one cell, a relink revisits only the things ahead of the relinked monster's old position. That gives visits ≤ order + I·L + K², with I the things linked ahead of the monsters, still below 9 + order·(R + 2). Cross-cell relinks restart a whole other cell, and no proof or counterexample for them was found. A counterexample would need many former humans in one explosion's window that are hit without pain while asleep, move across a cell boundary and then die on a later pass, with their drops revisited by the remaining relinks. Status: untested boundary, unresolved.

### CI time budget and slow-step inventory (2026-11-08)

The first full `2e33c35` macOS job was cancelled after 40m21s during
headless debug/release equality, matching its 40-minute job limit. Its retry
passed in 33m33s; Ubuntu passed in 32m33s and Windows in 11m57s. Thus the
retry is green, but the original limit had insufficient margin. At the owner's
request, Stable now allows 60 minutes on macOS and Ubuntu and 45 on Windows,
with every existing validation step and runner name retained.

The inventories below use GitHub API step `started_at`/`completed_at`
timestamps from the last full executions on this exact Rust tree, with the
corresponding job logs retrieved. Durations are integer wall seconds, sorted
descending; ties retain step order. They include build and command overhead
and are CI observations, not idle-machine simulation benchmarks. macOS uses
the successful retry; the cancelled first attempt spent 964s in Test, 610s in
Release deterministic checks, 237s in the Rust whole-level script comparison,
144s in Clippy and 108s in the M4a Rust script comparison. Its cancelled
headless comparison ran for 70s, a partial duration, not a completed timing.
The largest costs are workspace tests, release checks and Ubuntu reference
reproduction; the M4a checks are included rather than presumed dominant.

#### macOS: ten slowest steps

[Full job log](https://github.com/Copgunn/DOOM/actions/runs/42537174727/job/128920697787)

| Rank | Step | Seconds |
| ---: | --- | ---: |
| 1 | Test | 759 |
| 2 | Release deterministic checks | 489 |
| 3 | Rust whole-level run of the synthetic scripts against the encoder goldens | 199 |
| 4 | Headless debug/release equality | 113 |
| 5 | Clippy | 93 |
| 6 | Rust Episode 1 run of the M4a synthetic scripts against the encoder goldens | 87 |
| 7 | Original real-map-scale room debug/release comparison | 23 |
| 8 | Original Imp encounter debug/release comparison | 20 |
| 9 | Original armor sustained combat debug/release comparison | 19 |
| 10 | Original former-human encounter debug/release comparison | 17 |

#### Ubuntu: ten slowest steps

[Full job log](https://github.com/Copgunn/DOOM/actions/runs/42537174727/job/128903994549)

| Rank | Step | Seconds |
| ---: | --- | ---: |
| 1 | Reproduce isolated reference vectors and snapshot goldens | 425 |
| 2 | Test | 323 |
| 3 | Release deterministic checks | 147 |
| 4 | Independent original real-map-scale room evidence | 142 |
| 5 | Independent original Imp, fireball and mixed-roster evidence | 84 |
| 6 | Independent original real-map-scale evidence | 82 |
| 7 | Independent DSIM38 whole-level projection | 73 |
| 8 | Independent original former-human, mixed-roster and infighting evidence | 68 |
| 9 | Independent original armor, animation, pickup and absorption evidence | 45 |
| 10 | Independent original sky and static-specials evidence | 45 |

#### Windows: ten slowest steps

[Full job log](https://github.com/Copgunn/DOOM/actions/runs/42537174727/job/128903994560)

| Rank | Step | Seconds |
| ---: | --- | ---: |
| 1 | Test | 280 |
| 2 | Release deterministic checks | 186 |
| 3 | Rust whole-level run of the synthetic scripts against the encoder goldens | 44 |
| 4 | Clippy | 38 |
| 5 | Headless debug/release equality | 37 |
| 6 | Rust Episode 1 run of the M4a synthetic scripts against the encoder goldens | 21 |
| 7 | Run actions/checkout@11d5960a326750d5838078e36cf38b85af677262 | 9 |
| 8 | Format | 8 |
| 9 | Build | 8 |
| 10 | Original former-human encounter debug/release comparison | 8 |
